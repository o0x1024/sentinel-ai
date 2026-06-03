//! QuickJS-backed plugin runtime
//!
//! Wraps `rquickjs` to provide the same plugin execution semantics as the
//! previous One Engine integration: host-function bridge, bootstrap polyfill,
//! TypeScript stripping, and memory-based limits.

use rquickjs::{Context, Ctx, Function, IntoJs, Runtime, Value, object::Object};
use std::cell::RefCell;

use crate::error::{PluginError, Result};
use crate::plugin_context::PluginContext;

thread_local! {
    pub(crate) static QJS_PLUGIN_CTX: RefCell<Option<PluginContext>> = const { RefCell::new(None) };
}

pub(crate) fn with_plugin_ctx<R>(f: impl FnOnce(&PluginContext) -> R) -> R {
    QJS_PLUGIN_CTX.with(|ctx| {
        let ctx_ref = ctx.borrow();
        f(ctx_ref.as_ref().expect("PluginContext not set"))
    })
}

pub(crate) fn set_plugin_ctx(ctx: &PluginContext) {
    QJS_PLUGIN_CTX.with(|cell| {
        *cell.borrow_mut() = Some(ctx.clone());
    });
}

pub(crate) fn clear_plugin_ctx() {
    QJS_PLUGIN_CTX.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

pub(crate) fn block_on_async<F: std::future::Future>(future: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}

/// A QuickJS-backed engine for running a single plugin.
pub struct QjsPluginRuntime {
    runtime: Runtime,
    context: Context,
}

impl QjsPluginRuntime {
    pub fn new() -> Result<Self> {
        let runtime = Runtime::new()
            .map_err(|e| PluginError::Load(format!("Failed to create QuickJS runtime: {e}")))?;

        runtime.set_max_stack_size(512 * 1024);
        runtime.set_memory_limit(256 * 1024 * 1024);

        let context = Context::full(&runtime)
            .map_err(|e| PluginError::Load(format!("Failed to create QuickJS context: {e}")))?;

        Ok(Self { runtime, context })
    }

    pub fn register_host_functions(
        &self,
        permissions: &crate::permissions::PluginPermissions,
    ) -> Result<()> {
        self.context.with(|ctx| {
            register_core_functions(&ctx)?;

            if permissions.network != crate::permissions::NetworkPermission::None {
                register_fetch_functions(&ctx)?;
                register_network_functions(&ctx)?;
            }

            if permissions.filesystem != crate::permissions::FsPermission::None {
                register_filesystem_functions(&ctx)?;
            }

            if permissions.dictionary {
                register_dictionary_functions(&ctx)?;
            }

            if permissions.tls_inspect {
                register_tls_functions(&ctx)?;
            }

            if permissions.monitor_events {
                register_monitor_functions(&ctx)?;
            }

            if permissions.ast_parse {
                register_ast_functions(&ctx)?;
            }

            Ok::<_, PluginError>(())
        })?;
        Ok(())
    }

    pub fn eval_bootstrap(&self, _plugin_ctx: &PluginContext) -> Result<()> {
        // PluginContext is already installed in the thread-local by PluginEngine::new().
        self.context.with(|ctx| -> std::result::Result<(), PluginError> {
            ctx.eval::<(), _>(include_str!("plugin_bootstrap.js"))
                .map_err(|e| {
                    let detail = if let rquickjs::Error::Exception = e {
                        let caught = ctx.catch();
                        if let Some(exc) = caught.as_exception() {
                            format!("{}: {}", exc.message().unwrap_or_default(),
                                exc.stack().unwrap_or_default())
                        } else {
                            format!("{caught:?}")
                        }
                    } else {
                        format!("{e}")
                    };
                    PluginError::Load(format!("Bootstrap eval failed: {detail}"))
                })
        })
    }

    pub fn eval(&self, code: &str) -> Result<()> {
        self.context
            .with(|ctx| -> std::result::Result<(), PluginError> {
                ctx.eval::<(), _>(code).map_err(|e| {
                    let detail = if let rquickjs::Error::Exception = e {
                        let caught = ctx.catch();
                        if let Some(exc) = caught.as_exception() {
                            format!(
                                "{}: {}",
                                exc.message().unwrap_or_default(),
                                exc.stack().unwrap_or_default()
                            )
                        } else {
                            format!("{caught:?}")
                        }
                    } else {
                        format!("{e}")
                    };
                    PluginError::Execution(detail)
                })
            })
    }

    pub fn load_plugin(&self, code: &str, _is_module: bool) -> Result<()> {
        // QuickJS handles both scripts and modules via eval.
        // ESM `export` statements in plugin code are stripped by ts_strip or
        // ignored — plugins bind to globalThis, not via ESM exports.
        self.context
            .with(|ctx| -> std::result::Result<(), PluginError> {
                ctx.eval::<(), _>(code).map_err(|e| {
                    let detail = if let rquickjs::Error::Exception = e {
                        let caught = ctx.catch();
                        if let Some(exc) = caught.as_exception() {
                            format!(
                                "{}: {}",
                                exc.message().unwrap_or_default(),
                                exc.stack().unwrap_or_default()
                            )
                        } else {
                            format!("{caught:?}")
                        }
                    } else {
                        format!("{e}")
                    };
                    // Log the problematic line for debugging
                    if let Some(line_num) = detail.find("eval_script:").and_then(|pos| {
                        let after = &detail[pos + 12..];
                        after.split(':').next().and_then(|n| n.parse::<usize>().ok())
                    }) {
                        if let Some(line) = code.lines().nth(line_num.saturating_sub(1)) {
                            tracing::error!("Failed at line {}: {}", line_num, line.trim());
                        }
                    }
                    PluginError::Load(format!("Script load failed: {detail}"))
                })
            })?;
        self.execute_pending_jobs()?;
        Ok(())
    }

    pub fn set_json_global(&self, name: &str, value: &serde_json::Value) -> Result<()> {
        let name = name.to_string();
        let value = value.clone();
        self.context
            .with(|ctx| -> std::result::Result<(), PluginError> {
                let js_val = json_to_qjs(&ctx, &value)
                    .map_err(|e| PluginError::Execution(format!("JSON→JS: {e}")))?;
                ctx.globals()
                    .set(&*name, js_val)
                    .map_err(|e| PluginError::Execution(format!("set global '{name}': {e}")))
            })
    }

    pub fn execute_pending_jobs(&self) -> Result<()> {
        loop {
            match self.runtime.execute_pending_job() {
                Ok(false) => break,
                Ok(true) => continue,
                Err(e) => {
                    return Err(PluginError::Execution(format!("Pending job failed: {e}")));
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Standalone host functions with proper lifetime annotations
// ---------------------------------------------------------------------------

fn host_emit_finding<'js>(ctx: Ctx<'js>, val: Value<'js>) -> rquickjs::Result<bool> {
    let json = qjs_value_to_json(&ctx, val).unwrap_or(serde_json::Value::Null);
    let js_finding: crate::plugin_ops::JsFinding =
        serde_json::from_value(json).unwrap_or_default();
    with_plugin_ctx(|pctx| {
        let mut finding: crate::types::Finding = js_finding.into();
        if finding.plugin_id.is_empty() {
            finding.plugin_id = pctx
                .plugin_id()
                .unwrap_or_else(|| "unknown-plugin".to_string());
        }
        let _ = pctx.emit_finding(finding);
    });
    Ok(true)
}

fn host_return<'js>(ctx: Ctx<'js>, val: Value<'js>) -> rquickjs::Result<bool> {
    let json = qjs_value_to_json(&ctx, val).unwrap_or(serde_json::Value::Null);
    with_plugin_ctx(|pctx| {
        let mut last = pctx.last_result.lock().unwrap();
        *last = Some(json);
    });
    Ok(true)
}

fn host_emit_active_probe_event<'js>(
    ctx: Ctx<'js>,
    val: Value<'js>,
) -> rquickjs::Result<bool> {
    let json = qjs_value_to_json(&ctx, val).unwrap_or(serde_json::Value::Null);
    let update: crate::plugin_ops::ActiveProbeRuntimeUpdate = match serde_json::from_value(json) {
        Ok(v) => v,
        Err(_) => return Ok(false),
    };
    if update.request_id.trim().is_empty()
        || update.phase.trim().is_empty()
        || update.url.trim().is_empty()
    {
        return Ok(false);
    }
    with_plugin_ctx(|pctx| {
        let payload = crate::plugin_ops::ActiveProbeEvent {
            plugin_id: pctx.plugin_id(),
            traffic_request_id: pctx.traffic_request_id(),
            request_id: update.request_id,
            phase: update.phase,
            method: update.method,
            url: update.url,
            probe_label: update.probe_label,
            target_name: update.target_name,
            target_path: update.target_path,
            target_location: update.target_location,
            probe_value: update.probe_value,
            technique: update.technique,
            probe_class: update.probe_class,
            probe_priority: update.probe_priority,
            cooldown_key: update.cooldown_key,
            cooldown_wait_ms: update.cooldown_wait_ms,
            jitter_wait_ms: update.jitter_wait_ms,
            total_wait_ms: update.total_wait_ms,
            adaptive_penalty_ms: update.adaptive_penalty_ms,
            status: update.status,
            error: update.error,
            reason: update.reason,
            target_count: update.target_count,
            active_slots: update.active_slots,
            max_concurrent_per_host: update.max_concurrent_per_host,
            queue_depth: update.queue_depth,
            response_elapsed_ms: update.response_elapsed_ms,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        crate::runtime_events::emit_active_probe_event(&payload);
    });
    Ok(true)
}

fn host_get_plugin_runtime_settings<'js>(ctx: Ctx<'js>) -> rquickjs::Result<Value<'js>> {
    let settings = crate::runtime_config::get_plugin_runtime_settings();
    let json = serde_json::to_value(settings).unwrap_or(serde_json::Value::Null);
    json_to_qjs(&ctx, &json)
}

fn host_fetch<'js>(
    ctx: Ctx<'js>,
    url_val: Value<'js>,
    opts_val: Value<'js>,
) -> rquickjs::Result<Value<'js>> {
    let url = if url_val.is_object() {
        let json = qjs_value_to_json(&ctx, url_val).unwrap_or(serde_json::Value::Null);
        json.get("url")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    } else {
        url_val
            .as_string()
            .map(|s| s.to_string().unwrap_or_default())
            .unwrap_or_default()
    };

    let opts_json = qjs_value_to_json(&ctx, opts_val).unwrap_or(serde_json::Value::Null);
    let options: crate::plugin_fetch_types::FetchOptions =
        serde_json::from_value(opts_json).unwrap_or_else(|_| {
            crate::plugin_fetch_types::FetchOptions {
                method: "GET".to_string(),
                headers: std::collections::HashMap::new(),
                body: None,
                timeout: Some(3000),
                redirect: Some("follow".to_string()),
                max_redirects: Some(10),
                max_body_bytes: None,
                request_id: None,
                active_probe: None,
            }
        });

    let mut response =
        block_on_async(async { crate::plugin_engine::plugin_fetch(url, options).await });
    response.body_bytes = Vec::new();
    let json = serde_json::to_value(&response).unwrap_or(serde_json::Value::Null);
    json_to_qjs(&ctx, &json)
}

fn host_scan_ports<'js>(ctx: Ctx<'js>, val: Value<'js>) -> rquickjs::Result<Value<'js>> {
    let json = qjs_value_to_json(&ctx, val).unwrap_or(serde_json::Value::Null);
    let request: crate::network_scan::PortScanRequest =
        serde_json::from_value(json).unwrap_or_default();
    let response = block_on_async(crate::network_scan::op_scan_ports(request));
    let result_json = serde_json::to_value(response).unwrap_or(serde_json::Value::Null);
    json_to_qjs(&ctx, &result_json)
}

fn host_probe_services<'js>(ctx: Ctx<'js>, val: Value<'js>) -> rquickjs::Result<Value<'js>> {
    let json = qjs_value_to_json(&ctx, val).unwrap_or(serde_json::Value::Null);
    let request: crate::service_probe_runtime::ServiceProbeRequest =
        serde_json::from_value(json).unwrap_or_default();
    let response = block_on_async(crate::service_probe_runtime::op_probe_services(request));
    let result_json = serde_json::to_value(response).unwrap_or(serde_json::Value::Null);
    json_to_qjs(&ctx, &result_json)
}

fn host_get_service_probe_capabilities<'js>(ctx: Ctx<'js>) -> rquickjs::Result<Value<'js>> {
    let response = crate::service_probe::op_get_service_probe_capabilities();
    let json = serde_json::to_value(response).unwrap_or(serde_json::Value::Null);
    json_to_qjs(&ctx, &json)
}

fn host_read_dir<'js>(ctx: Ctx<'js>, path: String) -> rquickjs::Result<Value<'js>> {
    let mut entries = Vec::new();
    let mut read_dir = block_on_async(tokio::fs::read_dir(&path)).map_err(|e| {
        ctx.throw(
            rquickjs::String::from_str(ctx.clone(), &format!("readdir '{path}': {e}"))
                .unwrap()
                .into(),
        )
    })?;
    while let Some(entry) = block_on_async(read_dir.next_entry()).map_err(|e| {
        ctx.throw(
            rquickjs::String::from_str(ctx.clone(), &format!("readdir entry: {e}"))
                .unwrap()
                .into(),
        )
    })? {
        let meta = block_on_async(entry.metadata()).ok();
        entries.push(serde_json::json!({
            "name": entry.file_name().to_string_lossy(),
            "isFile": meta.as_ref().map(|m| m.is_file()).unwrap_or(false),
            "isDirectory": meta.as_ref().map(|m| m.is_dir()).unwrap_or(false),
            "isSymlink": meta.as_ref().map(|m| m.is_symlink()).unwrap_or(false),
        }));
    }
    json_to_qjs(&ctx, &serde_json::Value::Array(entries))
}

fn host_stat<'js>(ctx: Ctx<'js>, path: String) -> rquickjs::Result<Value<'js>> {
    let meta = block_on_async(tokio::fs::metadata(&path)).map_err(|e| {
        ctx.throw(
            rquickjs::String::from_str(ctx.clone(), &format!("stat '{path}': {e}"))
                .unwrap()
                .into(),
        )
    })?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);
    json_to_qjs(
        &ctx,
        &serde_json::json!({
            "size": meta.len(),
            "isFile": meta.is_file(),
            "isDirectory": meta.is_dir(),
            "isSymlink": meta.is_symlink(),
            "mtime": mtime,
        }),
    )
}

fn host_read_file<'js>(ctx: Ctx<'js>, path: String) -> rquickjs::Result<Value<'js>> {
    let bytes = block_on_async(tokio::fs::read(&path)).map_err(|e| {
        ctx.throw(
            rquickjs::String::from_str(ctx.clone(), &format!("read '{path}': {e}"))
                .unwrap()
                .into(),
        )
    })?;
    let arr = rquickjs::Array::new(ctx.clone())?;
    for (i, byte) in bytes.iter().enumerate() {
        arr.set(i, *byte as i32)?;
    }
    arr.into_js(&ctx)
}

fn host_get_tls_certificate<'js>(
    ctx: Ctx<'js>,
    hostname: String,
    port: f64,
    _timeout_ms: f64,
) -> rquickjs::Result<Value<'js>> {
    let response = crate::plugin_ops::TlsCertificateResponse {
        success: false,
        cert: None,
        error: Some(format!(
            "TLS certificate retrieval for {hostname}:{} not implemented",
            port as u16
        )),
    };
    let json = serde_json::to_value(response).unwrap_or(serde_json::Value::Null);
    json_to_qjs(&ctx, &json)
}

fn host_report_monitor_progress<'js>(
    ctx: Ctx<'js>,
    val: Value<'js>,
) -> rquickjs::Result<bool> {
    let json = qjs_value_to_json(&ctx, val).unwrap_or(serde_json::Value::Null);
    let request: crate::monitor_progress::PluginMonitorProgressRequest =
        serde_json::from_value(json).unwrap_or_default();
    if let Some(context) = request.monitor_progress {
        crate::monitor_progress::emit_plugin_monitor_progress(&context, request.update);
        Ok(true)
    } else {
        Ok(false)
    }
}

fn host_parse_js<'js>(
    ctx: Ctx<'js>,
    code: String,
    filename: rquickjs::Result<String>,
) -> rquickjs::Result<Value<'js>> {
    let fname = filename.ok();
    let result = crate::plugin_engine::parse_js_literals(code, fname);
    let json = serde_json::to_value(result).unwrap_or(serde_json::Value::Null);
    json_to_qjs(&ctx, &json)
}

fn host_get_dictionary<'js>(ctx: Ctx<'js>, id_or_name: Value<'js>) -> rquickjs::Result<Value<'js>> {
    let id = id_or_name
        .as_string()
        .map(|s| s.to_string().unwrap_or_default())
        .unwrap_or_default();
    if id.is_empty() {
        return Ok(Value::new_null(ctx));
    }
    match block_on_async(crate::dictionary_runtime::get_dictionary(id)) {
        Ok(Some(dict)) => {
            let json = serde_json::to_value(dict).unwrap_or(serde_json::Value::Null);
            json_to_qjs(&ctx, &json)
        }
        _ => Ok(Value::new_null(ctx)),
    }
}

fn host_get_default_dictionary_id<'js>(
    ctx: Ctx<'js>,
    dict_type: Value<'js>,
) -> rquickjs::Result<Value<'js>> {
    let dt = dict_type
        .as_string()
        .map(|s| s.to_string().unwrap_or_default())
        .unwrap_or_default();
    if dt.is_empty() {
        return rquickjs::String::from_str(ctx.clone(), "")?.into_js(&ctx);
    }
    match block_on_async(crate::dictionary_runtime::get_default_dictionary_id(dt)) {
        Ok(id) => rquickjs::String::from_str(ctx.clone(), &id)?.into_js(&ctx),
        Err(_) => rquickjs::String::from_str(ctx.clone(), "")?.into_js(&ctx),
    }
}

fn host_get_dictionary_words<'js>(
    ctx: Ctx<'js>,
    id_or_name: Value<'js>,
    limit: Value<'js>,
) -> rquickjs::Result<Value<'js>> {
    let id = id_or_name
        .as_string()
        .map(|s| s.to_string().unwrap_or_default())
        .unwrap_or_default();
    if id.is_empty() {
        return rquickjs::Array::new(ctx.clone())?.into_js(&ctx);
    }
    let lim = limit.as_int().map(|v| v as i32);
    match block_on_async(crate::dictionary_runtime::get_dictionary_words(id, lim)) {
        Ok(words) => {
            let json = serde_json::to_value(words).unwrap_or(serde_json::Value::Array(vec![]));
            json_to_qjs(&ctx, &json)
        }
        Err(_) => rquickjs::Array::new(ctx.clone())?.into_js(&ctx),
    }
}

fn host_get_dictionary_entries<'js>(
    ctx: Ctx<'js>,
    id_or_name: Value<'js>,
    limit: Value<'js>,
) -> rquickjs::Result<Value<'js>> {
    let id = id_or_name
        .as_string()
        .map(|s| s.to_string().unwrap_or_default())
        .unwrap_or_default();
    if id.is_empty() {
        return rquickjs::Array::new(ctx.clone())?.into_js(&ctx);
    }
    let lim = limit.as_int().map(|v| v as i32);
    match block_on_async(crate::dictionary_runtime::get_dictionary_entries(id, lim)) {
        Ok(entries) => {
            let json = serde_json::to_value(entries).unwrap_or(serde_json::Value::Array(vec![]));
            json_to_qjs(&ctx, &json)
        }
        Err(_) => rquickjs::Array::new(ctx.clone())?.into_js(&ctx),
    }
}

fn host_list_dictionaries<'js>(
    ctx: Ctx<'js>,
    filter: Value<'js>,
) -> rquickjs::Result<Value<'js>> {
    let filter_json = qjs_value_to_json(&ctx, filter).unwrap_or(serde_json::Value::Null);
    let dict_type = filter_json.get("dictType").or(filter_json.get("dict_type"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let category = filter_json.get("category")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    match block_on_async(crate::dictionary_runtime::list_dictionaries(dict_type, category)) {
        Ok(dicts) => {
            let json = serde_json::to_value(dicts).unwrap_or(serde_json::Value::Array(vec![]));
            json_to_qjs(&ctx, &json)
        }
        Err(_) => rquickjs::Array::new(ctx.clone())?.into_js(&ctx),
    }
}

// ---------------------------------------------------------------------------
// Host function registration
// ---------------------------------------------------------------------------

fn register_core_functions(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    globals
        .set(
            "__sentinel_log",
            Function::new(ctx.clone(), |level: String, message: String| {
                use tracing::{debug, error, info, warn};
                match level.to_lowercase().as_str() {
                    "error" => error!("[Plugin] {}", message),
                    "warn" => warn!("[Plugin] {}", message),
                    "info" => info!("[Plugin] {}", message),
                    "debug" => debug!("[Plugin] {}", message),
                    _ => debug!("[Plugin] {}", message),
                }
            }),
        )
        .map_err(|e| PluginError::Load(format!("register __sentinel_log: {e}")))?;

    globals
        .set(
            "__sentinel_emit_finding",
            Function::new(ctx.clone(), host_emit_finding),
        )
        .map_err(|e| PluginError::Load(format!("register __sentinel_emit_finding: {e}")))?;

    globals
        .set(
            "__sentinel_return",
            Function::new(ctx.clone(), host_return),
        )
        .map_err(|e| PluginError::Load(format!("register __sentinel_return: {e}")))?;

    globals
        .set(
            "__sentinel_emit_active_probe_event",
            Function::new(ctx.clone(), host_emit_active_probe_event),
        )
        .map_err(|e| PluginError::Load(format!("register active_probe_event: {e}")))?;

    globals
        .set(
            "__sentinel_get_plugin_runtime_settings",
            Function::new(ctx.clone(), host_get_plugin_runtime_settings),
        )
        .map_err(|e| PluginError::Load(format!("register runtime_settings: {e}")))?;

    Ok(())
}

fn host_fetch_batch<'js>(ctx: Ctx<'js>, val: Value<'js>) -> rquickjs::Result<Value<'js>> {
    #[derive(serde::Deserialize)]
    struct BatchEntry {
        url: String,
        #[serde(default)]
        options: crate::plugin_fetch_types::FetchOptions,
    }

    let json = qjs_value_to_json(&ctx, val).unwrap_or(serde_json::Value::Null);
    let entries: Vec<BatchEntry> =
        serde_json::from_value(json).unwrap_or_default();

    if entries.is_empty() {
        return json_to_qjs(&ctx, &serde_json::json!([]));
    }

    let plugin_ctx = with_plugin_ctx(|pctx| pctx.clone());

    let results = block_on_async(async {
        let mut handles = Vec::with_capacity(entries.len());
        for entry in entries {
            let ctx = plugin_ctx.clone();
            handles.push(tokio::spawn(async move {
                QJS_PLUGIN_CTX.with(|cell| {
                    *cell.borrow_mut() = Some(ctx);
                });
                let mut resp =
                    crate::plugin_engine::plugin_fetch(entry.url, entry.options).await;
                resp.body_bytes = Vec::new();
                resp
            }));
        }
        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(resp) => results.push(resp),
                Err(_) => results.push(crate::plugin_fetch_types::FetchResponse {
                    success: false,
                    status: 0,
                    headers: std::collections::HashMap::new(),
                    body: String::new(),
                    body_bytes: Vec::new(),
                    ok: false,
                    redirected: false,
                    final_url: String::new(),
                    error: Some("batch fetch task panicked".to_string()),
                }),
            }
        }
        results
    });

    let json = serde_json::to_value(&results).unwrap_or(serde_json::Value::Null);
    json_to_qjs(&ctx, &json)
}

fn register_fetch_functions(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    globals
        .set(
            "__sentinel_fetch",
            Function::new(ctx.clone(), host_fetch),
        )
        .map_err(|e| PluginError::Load(format!("register __sentinel_fetch: {e}")))?;

    globals
        .set(
            "__sentinel_abort_fetch",
            Function::new(ctx.clone(), || -> bool { false }),
        )
        .map_err(|e| PluginError::Load(format!("register __sentinel_abort_fetch: {e}")))?;

    globals
        .set(
            "__sentinel_fetch_batch",
            Function::new(ctx.clone(), host_fetch_batch),
        )
        .map_err(|e| PluginError::Load(format!("register __sentinel_fetch_batch: {e}")))?;

    Ok(())
}

fn register_network_functions(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    globals
        .set("__sentinel_scan_ports", Function::new(ctx.clone(), host_scan_ports))
        .map_err(|e| PluginError::Load(format!("register scan_ports: {e}")))?;

    globals
        .set(
            "__sentinel_probe_services",
            Function::new(ctx.clone(), host_probe_services),
        )
        .map_err(|e| PluginError::Load(format!("register probe_services: {e}")))?;

    globals
        .set(
            "__sentinel_get_service_probe_capabilities",
            Function::new(ctx.clone(), host_get_service_probe_capabilities),
        )
        .map_err(|e| PluginError::Load(format!("register service_probe_capabilities: {e}")))?;

    Ok(())
}

fn register_filesystem_functions(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    globals
        .set(
            "__sentinel_read_text_file",
            Function::new(ctx.clone(), |ctx: Ctx, path: String| -> rquickjs::Result<String> {
                block_on_async(tokio::fs::read_to_string(&path)).map_err(|e| {
                    ctx.throw(
                        rquickjs::String::from_str(ctx.clone(), &format!("read '{path}': {e}"))
                            .unwrap()
                            .into(),
                    )
                })
            }),
        )
        .map_err(|e| PluginError::Load(format!("register read_text_file: {e}")))?;

    globals
        .set(
            "__sentinel_write_text_file",
            Function::new(
                ctx.clone(),
                |ctx: Ctx, path: String, content: String| -> rquickjs::Result<()> {
                    block_on_async(tokio::fs::write(&path, content)).map_err(|e| {
                        ctx.throw(
                            rquickjs::String::from_str(
                                ctx.clone(),
                                &format!("write '{path}': {e}"),
                            )
                            .unwrap()
                            .into(),
                        )
                    })
                },
            ),
        )
        .map_err(|e| PluginError::Load(format!("register write_text_file: {e}")))?;

    globals
        .set("__sentinel_read_file", Function::new(ctx.clone(), host_read_file))
        .map_err(|e| PluginError::Load(format!("register read_file: {e}")))?;

    globals
        .set(
            "__sentinel_write_file",
            Function::new(
                ctx.clone(),
                |ctx: Ctx, path: String, data: Value| -> rquickjs::Result<()> {
                    let mut bytes = Vec::new();
                    if let Some(arr) = data.as_array() {
                        for i in 0..arr.len() {
                            if let Ok(v) = arr.get::<f64>(i) {
                                bytes.push(v as u8);
                            }
                        }
                    }
                    block_on_async(tokio::fs::write(&path, bytes)).map_err(|e| {
                        ctx.throw(
                            rquickjs::String::from_str(
                                ctx.clone(),
                                &format!("write '{path}': {e}"),
                            )
                            .unwrap()
                            .into(),
                        )
                    })
                },
            ),
        )
        .map_err(|e| PluginError::Load(format!("register write_file: {e}")))?;

    globals
        .set(
            "__sentinel_mkdir",
            Function::new(
                ctx.clone(),
                |ctx: Ctx, path: String, recursive: bool| -> rquickjs::Result<()> {
                    let result = if recursive {
                        block_on_async(tokio::fs::create_dir_all(&path))
                    } else {
                        block_on_async(tokio::fs::create_dir(&path))
                    };
                    result.map_err(|e| {
                        ctx.throw(
                            rquickjs::String::from_str(
                                ctx.clone(),
                                &format!("mkdir '{path}': {e}"),
                            )
                            .unwrap()
                            .into(),
                        )
                    })
                },
            ),
        )
        .map_err(|e| PluginError::Load(format!("register mkdir: {e}")))?;

    globals
        .set("__sentinel_read_dir", Function::new(ctx.clone(), host_read_dir))
        .map_err(|e| PluginError::Load(format!("register read_dir: {e}")))?;

    globals
        .set("__sentinel_stat", Function::new(ctx.clone(), host_stat))
        .map_err(|e| PluginError::Load(format!("register stat: {e}")))?;

    globals
        .set(
            "__sentinel_copy_file",
            Function::new(
                ctx.clone(),
                |ctx: Ctx, from: String, to: String| -> rquickjs::Result<()> {
                    block_on_async(tokio::fs::copy(&from, &to))
                        .map(|_| ())
                        .map_err(|e| {
                            ctx.throw(
                                rquickjs::String::from_str(
                                    ctx.clone(),
                                    &format!("copy: {e}"),
                                )
                                .unwrap()
                                .into(),
                            )
                        })
                },
            ),
        )
        .map_err(|e| PluginError::Load(format!("register copy_file: {e}")))?;

    globals
        .set(
            "__sentinel_remove",
            Function::new(
                ctx.clone(),
                |ctx: Ctx, path: String, recursive: bool| -> rquickjs::Result<()> {
                    let meta = block_on_async(tokio::fs::metadata(&path)).map_err(|e| {
                        ctx.throw(
                            rquickjs::String::from_str(
                                ctx.clone(),
                                &format!("remove '{path}': {e}"),
                            )
                            .unwrap()
                            .into(),
                        )
                    })?;
                    let result = if meta.is_dir() {
                        if recursive {
                            block_on_async(tokio::fs::remove_dir_all(&path))
                        } else {
                            block_on_async(tokio::fs::remove_dir(&path))
                        }
                    } else {
                        block_on_async(tokio::fs::remove_file(&path))
                    };
                    result.map_err(|e| {
                        ctx.throw(
                            rquickjs::String::from_str(
                                ctx.clone(),
                                &format!("remove '{path}': {e}"),
                            )
                            .unwrap()
                            .into(),
                        )
                    })
                },
            ),
        )
        .map_err(|e| PluginError::Load(format!("register remove: {e}")))?;

    globals
        .set(
            "__sentinel_make_temp_file",
            Function::new(
                ctx.clone(),
                |ctx: Ctx, prefix: String, suffix: String| -> rquickjs::Result<String> {
                    let temp_dir = std::env::temp_dir();
                    let name = format!("{prefix}{}{suffix}", uuid::Uuid::new_v4());
                    let path = temp_dir.join(name);
                    std::fs::File::create(&path).map_err(|e| {
                        ctx.throw(
                            rquickjs::String::from_str(
                                ctx.clone(),
                                &format!("make_temp_file: {e}"),
                            )
                            .unwrap()
                            .into(),
                        )
                    })?;
                    Ok(path.to_string_lossy().to_string())
                },
            ),
        )
        .map_err(|e| PluginError::Load(format!("register make_temp_file: {e}")))?;

    Ok(())
}

fn register_dictionary_functions(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    globals
        .set(
            "__sentinel_get_dictionary",
            Function::new(ctx.clone(), host_get_dictionary),
        )
        .map_err(|e| PluginError::Load(format!("register dict: {e}")))?;

    globals
        .set(
            "__sentinel_get_default_dictionary_id",
            Function::new(ctx.clone(), host_get_default_dictionary_id),
        )
        .map_err(|e| PluginError::Load(format!("register dict: {e}")))?;

    globals
        .set(
            "__sentinel_get_dictionary_words",
            Function::new(ctx.clone(), host_get_dictionary_words),
        )
        .map_err(|e| PluginError::Load(format!("register dict: {e}")))?;

    globals
        .set(
            "__sentinel_get_dictionary_entries",
            Function::new(ctx.clone(), host_get_dictionary_entries),
        )
        .map_err(|e| PluginError::Load(format!("register dict: {e}")))?;

    globals
        .set(
            "__sentinel_list_dictionaries",
            Function::new(ctx.clone(), host_list_dictionaries),
        )
        .map_err(|e| PluginError::Load(format!("register dict: {e}")))?;

    Ok(())
}

fn register_tls_functions(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    globals
        .set(
            "__sentinel_get_tls_certificate",
            Function::new(ctx.clone(), host_get_tls_certificate),
        )
        .map_err(|e| PluginError::Load(format!("register tls: {e}")))?;

    Ok(())
}

fn register_monitor_functions(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    globals
        .set(
            "__sentinel_report_monitor_progress",
            Function::new(ctx.clone(), host_report_monitor_progress),
        )
        .map_err(|e| PluginError::Load(format!("register monitor: {e}")))?;

    Ok(())
}

fn register_ast_functions(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    globals
        .set(
            "__sentinel_parse_js",
            Function::new(ctx.clone(), host_parse_js),
        )
        .map_err(|e| PluginError::Load(format!("register ast: {e}")))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// JSON ↔ QuickJS value conversion
// ---------------------------------------------------------------------------

pub(crate) fn json_to_qjs<'js>(
    ctx: &Ctx<'js>,
    value: &serde_json::Value,
) -> rquickjs::Result<Value<'js>> {
    match value {
        serde_json::Value::Null => Ok(Value::new_null(ctx.clone())),
        serde_json::Value::Bool(b) => (*b).into_js(ctx),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    (i as i32).into_js(ctx)
                } else {
                    (i as f64).into_js(ctx)
                }
            } else {
                n.as_f64().unwrap_or(0.0).into_js(ctx)
            }
        }
        serde_json::Value::String(s) => {
            rquickjs::String::from_str(ctx.clone(), s).map(|s| s.into())
        }
        serde_json::Value::Array(arr) => {
            let js_arr = rquickjs::Array::new(ctx.clone())?;
            for (i, item) in arr.iter().enumerate() {
                js_arr.set(i, json_to_qjs(ctx, item)?)?;
            }
            js_arr.into_js(ctx)
        }
        serde_json::Value::Object(map) => {
            let obj = Object::new(ctx.clone())?;
            for (key, val) in map {
                obj.set(key.as_str(), json_to_qjs(ctx, val)?)?;
            }
            obj.into_js(ctx)
        }
    }
}

pub(crate) fn qjs_value_to_json(ctx: &Ctx, val: Value) -> rquickjs::Result<serde_json::Value> {
    qjs_to_json(ctx, val)
}

fn qjs_to_json(ctx: &Ctx, val: Value) -> rquickjs::Result<serde_json::Value> {
    if val.is_null() || val.is_undefined() {
        return Ok(serde_json::Value::Null);
    }
    if let Some(b) = val.as_bool() {
        return Ok(serde_json::Value::Bool(b));
    }
    if let Some(i) = val.as_int() {
        return Ok(serde_json::json!(i));
    }
    if let Some(f) = val.as_float() {
        if f.is_finite() {
            return Ok(serde_json::json!(f));
        } else {
            return Ok(serde_json::Value::Null);
        }
    }
    if let Some(s) = val.as_string() {
        return Ok(serde_json::Value::String(
            s.to_string().unwrap_or_default(),
        ));
    }
    if let Some(arr) = val.as_array() {
        let mut result = Vec::with_capacity(arr.len());
        for i in 0..arr.len() {
            let item: Value = arr.get(i)?;
            result.push(qjs_to_json(ctx, item)?);
        }
        return Ok(serde_json::Value::Array(result));
    }
    if let Some(obj) = val.as_object() {
        let mut map = serde_json::Map::new();
        for key in obj.keys::<String>() {
            let key = key?;
            let v: Value = obj.get(&key)?;
            map.insert(key, qjs_to_json(ctx, v)?);
        }
        return Ok(serde_json::Value::Object(map));
    }
    Ok(serde_json::Value::Null)
}

//! Sentinel-JS backed plugin runtime.
//!
//! Drop-in replacement for `one_plugin_runtime.rs`, using the `sentinel-js`
//! crate for JavaScript execution. Provides all the same host function bindings
//! but with proper Web/Node API support built into the engine.

use std::cell::RefCell;

use sentinel_js::{HostBindingsExt, PluginRuntime, RuntimeConfig};

use crate::error::{PluginError, Result};
use crate::permissions::PluginPermissions;
use crate::plugin_context::PluginContext;

thread_local! {
    pub(crate) static PLUGIN_CTX: RefCell<Option<PluginContext>> = const { RefCell::new(None) };
}

pub(crate) fn with_plugin_ctx<R>(f: impl FnOnce(&PluginContext) -> R) -> R {
    PLUGIN_CTX.with(|ctx| {
        let ctx_ref = ctx.borrow();
        f(ctx_ref.as_ref().expect("PluginContext not set"))
    })
}

pub(crate) fn set_plugin_ctx(ctx: &PluginContext) {
    PLUGIN_CTX.with(|cell| {
        *cell.borrow_mut() = Some(ctx.clone());
    });
}

pub(crate) fn clear_plugin_ctx() {
    PLUGIN_CTX.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

pub(crate) fn block_on_async<F: std::future::Future>(future: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}

/// Sentinel-JS backed plugin runtime, replacing OnePluginRuntime.
pub struct SentinelJsRuntime {
    runtime: PluginRuntime,
}

impl SentinelJsRuntime {
    pub fn new() -> Result<Self> {
        let config = RuntimeConfig {
            max_memory: 256 * 1024 * 1024,
            max_execution_ms: 30_000,
            max_stack_depth: 512,
            node_compat: true,
            web_apis: true,
            ..Default::default()
        };

        let runtime = PluginRuntime::new(config)
            .map_err(|e| PluginError::Load(format!("Failed to create sentinel-js runtime: {e}")))?;

        Ok(Self { runtime })
    }

    pub fn register_host_functions(&mut self, permissions: &PluginPermissions) -> Result<()> {
        let mut bindings = HostBindingsExt::new();

        // Core functions — always registered
        register_core_bindings(&mut bindings);

        if permissions.network != crate::permissions::NetworkPermission::None {
            register_fetch_bindings(&mut bindings);
            register_network_bindings(&mut bindings);
        }

        if permissions.filesystem != crate::permissions::FsPermission::None {
            register_filesystem_bindings(&mut bindings);
        }

        if permissions.dictionary {
            register_dictionary_bindings(&mut bindings);
        }

        if permissions.tls_inspect {
            register_tls_bindings(&mut bindings);
        }

        if permissions.monitor_events {
            register_monitor_bindings(&mut bindings);
        }

        if permissions.ast_parse {
            register_ast_bindings(&mut bindings);
        }

        self.runtime
            .install_host_bindings(&bindings)
            .map_err(|e| PluginError::Load(format!("register host functions: {e}")))?;

        Ok(())
    }

    /// Bootstrap is no longer needed — sentinel-js provides all APIs natively.
    /// This method exists for API compatibility; it installs a minimal compatibility shim.
    pub fn eval_bootstrap(&self, _plugin_ctx: &PluginContext) -> Result<()> {
        // Install __host_fetch which is called by sentinel-js's fetch polyfill
        let fetch_bridge = r#"
            globalThis.__host_fetch = function(url, opts) {
                var urlStr = (typeof url === "string") ? url : String(url);
                var result = globalThis.__sentinel_fetch(urlStr, opts || {});
                if (result && result.success !== undefined) {
                    return result;
                }
                return { success: false, error: "fetch bridge: unexpected result" };
            };
        "#;
        self.runtime.eval_void(fetch_bridge)
            .map_err(|e| PluginError::Load(format!("Bootstrap eval failed: {e}")))?;

        // Compatibility aliases for plugins that reference Sentinel/Deno globals
        let compat = r#"
            if (typeof globalThis.Sentinel === "undefined") {
                globalThis.Sentinel = {
                    log: function(level, msg) { console[level] ? console[level](msg) : console.log(msg); },
                    emit: function(finding) { if (typeof __sentinel_emit_finding === "function") __sentinel_emit_finding(finding); },
                    resolve: function(result) { if (typeof __sentinel_return === "function") __sentinel_return(result); },
                };
            }
            if (!globalThis.Sentinel.Dictionary) {
                globalThis.Sentinel.Dictionary = {
                    get: function(idOrName) {
                        return (typeof __sentinel_get_dictionary === "function")
                            ? __sentinel_get_dictionary(idOrName) : null;
                    },
                    getDefaultId: function(dictType) {
                        return (typeof __sentinel_get_default_dictionary_id === "function")
                            ? __sentinel_get_default_dictionary_id(dictType) : "";
                    },
                    getWords: function(idOrName, limit) {
                        return (typeof __sentinel_get_dictionary_words === "function")
                            ? __sentinel_get_dictionary_words(idOrName, limit) : [];
                    },
                    getEntries: function(idOrName, limit) {
                        return (typeof __sentinel_get_dictionary_entries === "function")
                            ? __sentinel_get_dictionary_entries(idOrName, limit) : [];
                    },
                    list: function(filter) {
                        if (typeof __sentinel_list_dictionaries !== "function") return [];
                        var dictType = null, category = null;
                        if (filter) {
                            if (filter.dictType) dictType = filter.dictType;
                            if (filter.category) category = filter.category;
                        }
                        return __sentinel_list_dictionaries(dictType, category);
                    }
                };
            }
            if (typeof globalThis.Deno === "undefined") {
                globalThis.Deno = { build: { os: process.platform, arch: process.arch } };
            }
        "#;
        self.runtime.eval_void(compat)
            .map_err(|e| PluginError::Load(format!("Compat shim failed: {e}")))?;

        Ok(())
    }

    pub fn eval(&self, code: &str) -> Result<()> {
        self.runtime
            .eval_void(code)
            .map_err(|e| PluginError::Execution(format!("{e}")))
    }

    pub fn load_plugin(&self, code: &str, is_module: bool) -> Result<()> {
        self.runtime
            .load_plugin(code, is_module)
            .map_err(|e| PluginError::Load(format!("Script load failed: {e}")))
    }

    pub fn set_json_global(&self, name: &str, value: &serde_json::Value) -> Result<()> {
        self.runtime
            .set_global(name, value)
            .map_err(|e| PluginError::Execution(format!("set global '{name}': {e}")))
    }

    pub fn execute_pending_jobs(&self) -> Result<()> {
        self.runtime
            .execute_pending_jobs()
            .map_err(|e| PluginError::Execution(format!("Pending job failed: {e}")))
    }

    pub fn call_function(
        &self,
        fn_name: &str,
        input: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.runtime
            .call_function(fn_name, input)
            .map_err(|e| PluginError::Execution(format!("call {fn_name}: {e}")))
    }
}

// ---------------------------------------------------------------------------
// Host function binding registrations
// ---------------------------------------------------------------------------

fn register_core_bindings(bindings: &mut HostBindingsExt) {
    // __sentinel_log(level, message)
    bindings.register_void("__sentinel_log", |json_str| {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
            // The wrapper stringifies the first argument only;
            // but the JS wrapper calls with a single JSON-ified object.
            // We handle both cases.
            let msg = val.as_str().unwrap_or("").to_string();
            tracing::info!(target: "plugin", "{}", msg);
        }
    });

    // __sentinel_emit_finding
    bindings.register_fn1("__sentinel_emit_finding", |json_str| {
        let json: serde_json::Value =
            serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null);
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
        "true".to_string()
    });

    // __sentinel_return
    bindings.register_fn1("__sentinel_return", |json_str| {
        let json: serde_json::Value =
            serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null);
        with_plugin_ctx(|pctx| {
            let mut last = pctx.last_result.lock().unwrap();
            *last = Some(json);
        });
        "true".to_string()
    });

    // __sentinel_emit_active_probe_event
    bindings.register_fn1("__sentinel_emit_active_probe_event", |json_str| {
        let json: serde_json::Value =
            serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null);
        let update: crate::plugin_ops::ActiveProbeRuntimeUpdate = match serde_json::from_value(json)
        {
            Ok(v) => v,
            Err(_) => return "false".to_string(),
        };
        if update.request_id.trim().is_empty()
            || update.phase.trim().is_empty()
            || update.url.trim().is_empty()
        {
            return "false".to_string();
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
        "true".to_string()
    });

    // __sentinel_get_plugin_runtime_settings
    bindings.register_fn0("__sentinel_get_plugin_runtime_settings", || {
        let settings = crate::runtime_config::get_plugin_runtime_settings();
        serde_json::to_string(&settings).unwrap_or_else(|_| "null".to_string())
    });
}

fn register_fetch_bindings(bindings: &mut HostBindingsExt) {
    // __sentinel_fetch(url, options) -> response
    bindings.register_fn2("__sentinel_fetch", |url_json, opts_json| {
        let url: String = serde_json::from_str(url_json).unwrap_or_default();
        let options: crate::plugin_fetch_types::FetchOptions =
            serde_json::from_str(opts_json).unwrap_or_else(|_| {
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
        serde_json::to_string(&response).unwrap_or_else(|_| "null".to_string())
    });

    bindings.register_fn0("__sentinel_abort_fetch", || "false".to_string());
}

fn register_network_bindings(bindings: &mut HostBindingsExt) {
    bindings.register_fn1("__sentinel_scan_ports", |json_str| {
        let request: crate::network_scan::PortScanRequest =
            serde_json::from_str(json_str).unwrap_or_default();
        let response = block_on_async(crate::network_scan::op_scan_ports(request));
        serde_json::to_string(&response).unwrap_or_else(|_| "null".to_string())
    });

    bindings.register_fn1("__sentinel_probe_services", |json_str| {
        let request: crate::service_probe_runtime::ServiceProbeRequest =
            serde_json::from_str(json_str).unwrap_or_default();
        let response = block_on_async(crate::service_probe_runtime::op_probe_services(request));
        serde_json::to_string(&response).unwrap_or_else(|_| "null".to_string())
    });

    bindings.register_fn0("__sentinel_get_service_probe_capabilities", || {
        let response = crate::service_probe::op_get_service_probe_capabilities();
        serde_json::to_string(&response).unwrap_or_else(|_| "null".to_string())
    });
}

fn register_filesystem_bindings(bindings: &mut HostBindingsExt) {
    bindings.register_fn1("__sentinel_read_text_file", |json_str| {
        let path: String = serde_json::from_str(json_str).unwrap_or_default();
        match block_on_async(tokio::fs::read_to_string(&path)) {
            Ok(content) => serde_json::to_string(&content).unwrap_or_else(|_| "null".to_string()),
            Err(e) => serde_json::to_string(&format!("ERROR: {e}"))
                .unwrap_or_else(|_| "null".to_string()),
        }
    });

    bindings.register_fn2("__sentinel_write_text_file", |path_json, content_json| {
        let path: String = serde_json::from_str(path_json).unwrap_or_default();
        let content: String = serde_json::from_str(content_json).unwrap_or_default();
        match block_on_async(tokio::fs::write(&path, &content)) {
            Ok(_) => "true".to_string(),
            Err(e) => format!("\"ERROR: {e}\""),
        }
    });

    bindings.register_fn1("__sentinel_read_file", |json_str| {
        let path: String = serde_json::from_str(json_str).unwrap_or_default();
        match block_on_async(tokio::fs::read(&path)) {
            Ok(bytes) => {
                let arr: Vec<u8> = bytes;
                serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
            }
            Err(e) => format!("\"ERROR: {e}\""),
        }
    });

    bindings.register_fn1("__sentinel_read_dir", |json_str| {
        let path: String = serde_json::from_str(json_str).unwrap_or_default();
        let mut entries = Vec::new();
        if let Ok(mut read_dir) = block_on_async(tokio::fs::read_dir(&path)) {
            while let Ok(Some(entry)) = block_on_async(read_dir.next_entry()) {
                let meta = block_on_async(entry.metadata()).ok();
                entries.push(serde_json::json!({
                    "name": entry.file_name().to_string_lossy(),
                    "isFile": meta.as_ref().map(|m| m.is_file()).unwrap_or(false),
                    "isDirectory": meta.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                    "isSymlink": meta.as_ref().map(|m| m.is_symlink()).unwrap_or(false),
                }));
            }
        }
        serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
    });

    bindings.register_fn1("__sentinel_stat", |json_str| {
        let path: String = serde_json::from_str(json_str).unwrap_or_default();
        match block_on_async(tokio::fs::metadata(&path)) {
            Ok(meta) => {
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64);
                serde_json::to_string(&serde_json::json!({
                    "size": meta.len(),
                    "isFile": meta.is_file(),
                    "isDirectory": meta.is_dir(),
                    "isSymlink": meta.is_symlink(),
                    "mtime": mtime,
                }))
                .unwrap_or_else(|_| "null".to_string())
            }
            Err(e) => format!("\"ERROR: {e}\""),
        }
    });

    bindings.register_fn2("__sentinel_mkdir", |path_json, recursive_json| {
        let path: String = serde_json::from_str(path_json).unwrap_or_default();
        let recursive: bool = serde_json::from_str(recursive_json).unwrap_or(false);
        let result = if recursive {
            block_on_async(tokio::fs::create_dir_all(&path))
        } else {
            block_on_async(tokio::fs::create_dir(&path))
        };
        match result {
            Ok(_) => "true".to_string(),
            Err(e) => format!("\"ERROR: {e}\""),
        }
    });

    bindings.register_fn2("__sentinel_make_temp_file", |prefix_json, suffix_json| {
        let prefix: String = serde_json::from_str(prefix_json).unwrap_or_default();
        let suffix: String = serde_json::from_str(suffix_json).unwrap_or_default();
        let temp_dir = std::env::temp_dir();
        let name = format!("{prefix}{}{suffix}", uuid::Uuid::new_v4());
        let path = temp_dir.join(name);
        match std::fs::File::create(&path) {
            Ok(_) => serde_json::to_string(&path.to_string_lossy().to_string())
                .unwrap_or_else(|_| "null".to_string()),
            Err(e) => format!("\"ERROR: {e}\""),
        }
    });
}

fn register_dictionary_bindings(bindings: &mut HostBindingsExt) {
    bindings.register_fn1("__sentinel_get_dictionary", |id_json| {
        let id: String = serde_json::from_str(id_json).unwrap_or_default();
        if id.is_empty() {
            return "null".to_string();
        }
        match block_on_async(crate::dictionary_runtime::get_dictionary(id)) {
            Ok(Some(dict)) => {
                serde_json::to_string(&dict).unwrap_or_else(|_| "null".to_string())
            }
            _ => "null".to_string(),
        }
    });
    bindings.register_fn1("__sentinel_get_default_dictionary_id", |type_json| {
        let dt: String = serde_json::from_str(type_json).unwrap_or_default();
        if dt.is_empty() {
            return "\"\"".to_string();
        }
        match block_on_async(crate::dictionary_runtime::get_default_dictionary_id(dt)) {
            Ok(id) => serde_json::to_string(&id).unwrap_or_else(|_| "\"\"".to_string()),
            Err(_) => "\"\"".to_string(),
        }
    });
    bindings.register_fn2("__sentinel_get_dictionary_words", |id_json, limit_json| {
        let id: String = serde_json::from_str(id_json).unwrap_or_default();
        if id.is_empty() {
            return "[]".to_string();
        }
        let lim: Option<i32> = serde_json::from_str(limit_json).ok();
        match block_on_async(crate::dictionary_runtime::get_dictionary_words(id, lim)) {
            Ok(words) => serde_json::to_string(&words).unwrap_or_else(|_| "[]".to_string()),
            Err(_) => "[]".to_string(),
        }
    });
    bindings.register_fn2("__sentinel_get_dictionary_entries", |id_json, limit_json| {
        let id: String = serde_json::from_str(id_json).unwrap_or_default();
        if id.is_empty() {
            return "[]".to_string();
        }
        let lim: Option<i32> = serde_json::from_str(limit_json).ok();
        match block_on_async(crate::dictionary_runtime::get_dictionary_entries(id, lim)) {
            Ok(entries) => serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string()),
            Err(_) => "[]".to_string(),
        }
    });
    bindings.register_fn1("__sentinel_list_dictionaries", |filter_json| {
        let filter: serde_json::Value =
            serde_json::from_str(filter_json).unwrap_or(serde_json::Value::Null);
        let dict_type = filter.get("dictType").or(filter.get("dict_type"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let category = filter.get("category")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        match block_on_async(crate::dictionary_runtime::list_dictionaries(dict_type, category)) {
            Ok(dicts) => serde_json::to_string(&dicts).unwrap_or_else(|_| "[]".to_string()),
            Err(_) => "[]".to_string(),
        }
    });
}

fn register_tls_bindings(bindings: &mut HostBindingsExt) {
    bindings.register_fn1("__sentinel_get_tls_certificate", |json_str| {
        let response = crate::plugin_ops::TlsCertificateResponse {
            success: false,
            cert: None,
            error: Some("TLS certificate retrieval not implemented".to_string()),
        };
        serde_json::to_string(&response).unwrap_or_else(|_| "null".to_string())
    });
}

fn register_monitor_bindings(bindings: &mut HostBindingsExt) {
    bindings.register_fn1("__sentinel_report_monitor_progress", |json_str| {
        let request: crate::monitor_progress::PluginMonitorProgressRequest =
            serde_json::from_str(json_str).unwrap_or_default();
        if let Some(context) = request.monitor_progress {
            crate::monitor_progress::emit_plugin_monitor_progress(&context, request.update);
            "true".to_string()
        } else {
            "false".to_string()
        }
    });
}

fn register_ast_bindings(bindings: &mut HostBindingsExt) {
    bindings.register_fn1("__sentinel_parse_js", |json_str| {
        // The JS side sends {code: "...", filename: "..."} but our wrapper stringifies
        // the full argument. Parse it.
        let val: serde_json::Value =
            serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null);
        let code = val
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let filename = val.get("filename").and_then(|v| v.as_str()).map(String::from);
        let result = crate::plugin_engine::parse_js_literals(code, filename);
        serde_json::to_string(&result).unwrap_or_else(|_| "null".to_string())
    });
}

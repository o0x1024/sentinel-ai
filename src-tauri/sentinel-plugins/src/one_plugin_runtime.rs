//! One Engine-backed plugin runtime
//!
//! Replaces the QuickJS (`rquickjs`) backend with the One Engine for plugin
//! execution. Provides: host-function bridge to Rust plugin ops, bootstrap
//! polyfill, TypeScript stripping, fuel-based limits and wall-clock timeout.

use std::cell::RefCell;

use one_core::{JsValue, OneResult};
use one_engine::{Engine, EngineBuilder, RuntimeLimits};
use one_vm::Vm;

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

const GLOBAL_THIS_SHIM: &str = r#"
var globalThis = {};
"#;

const POST_BOOTSTRAP_GLOBALS: &str = r#"
var Sentinel = globalThis.Sentinel;
var Deno = globalThis.Deno;
if (typeof globalThis.fetch === "function") {
    var fetch = globalThis.fetch;
}
if (typeof URL === "undefined" && typeof globalThis.URL !== "undefined") {
    var URL = globalThis.URL;
}
if (typeof URLSearchParams === "undefined" && typeof globalThis.URLSearchParams !== "undefined") {
    var URLSearchParams = globalThis.URLSearchParams;
}
"#;

pub struct OnePluginRuntime {
    engine: Engine<()>,
}

impl OnePluginRuntime {
    pub fn new() -> Result<Self> {
        let engine = EngineBuilder::<()>::new()
            .limits(RuntimeLimits {
                max_operations: Some(50_000_000),
                max_call_depth: Some(128),
                ..Default::default()
            })
            .build_with_store(());

        Ok(Self { engine })
    }

    pub fn register_host_functions(&mut self, permissions: &PluginPermissions) -> Result<()> {
        register_core_functions(self.engine.vm_mut());

        if permissions.network != crate::permissions::NetworkPermission::None {
            register_fetch_functions(self.engine.vm_mut());
            register_network_functions(self.engine.vm_mut());
        }

        if permissions.filesystem != crate::permissions::FsPermission::None {
            register_filesystem_functions(self.engine.vm_mut());
        }

        if permissions.dictionary {
            register_dictionary_functions(self.engine.vm_mut());
        }

        if permissions.tls_inspect {
            register_tls_functions(self.engine.vm_mut());
        }

        if permissions.monitor_events {
            register_monitor_functions(self.engine.vm_mut());
        }

        if permissions.ast_parse {
            register_ast_functions(self.engine.vm_mut());
        }

        Ok(())
    }

    pub fn eval_bootstrap(&mut self, _plugin_ctx: &PluginContext) -> Result<()> {
        self.engine
            .eval(GLOBAL_THIS_SHIM)
            .map_err(|e| PluginError::Load(format!("globalThis shim failed: {e}")))?;
        self.engine
            .eval(include_str!("plugin_bootstrap.js"))
            .map_err(|e| PluginError::Load(format!("Bootstrap eval failed: {e}")))?;
        self.engine
            .eval(POST_BOOTSTRAP_GLOBALS)
            .map_err(|e| PluginError::Load(format!("Post-bootstrap globals failed: {e}")))?;
        Ok(())
    }

    pub fn eval(&mut self, code: &str) -> Result<()> {
        self.engine
            .eval(code)
            .map_err(|e| PluginError::Execution(format!("{e}")))?;
        Ok(())
    }

    pub fn load_plugin(&mut self, code: &str, is_module: bool) -> Result<()> {
        if is_module {
            self.engine
                .eval_module(code, "plugin.js")
                .map_err(|e| PluginError::Load(format!("Module load failed: {e}")))?;
        } else {
            self.engine
                .eval(code)
                .map_err(|e| PluginError::Load(format!("Script load failed: {e}")))?;
        }
        self.engine
            .run_event_loop()
            .map_err(|e| PluginError::Load(format!("Event loop failed: {e}")))?;
        Ok(())
    }

    pub fn set_json_global(&mut self, name: &str, value: &serde_json::Value) -> Result<()> {
        self.engine.set_json_global(name, value);
        Ok(())
    }

    pub fn execute_pending_jobs(&mut self) -> Result<()> {
        self.engine
            .run_event_loop()
            .map_err(|e| PluginError::Execution(format!("Event loop failed: {e}")))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Host function registration
// ---------------------------------------------------------------------------

fn register_core_functions(vm: &mut Vm) {
    vm.register_host_fn("__sentinel_log", |vm, args| {
        let level = args
            .first()
            .map(|v| vm.value_to_string(*v))
            .unwrap_or_default();
        let message = args
            .get(1)
            .map(|v| vm.value_to_string(*v))
            .unwrap_or_default();

        match level.as_str() {
            "error" => tracing::error!("[Plugin] {}", message),
            "warn" => tracing::warn!("[Plugin] {}", message),
            "info" => tracing::info!("[Plugin] {}", message),
            "debug" => tracing::debug!("[Plugin] {}", message),
            _ => tracing::debug!("[Plugin] {}", message),
        }
        Ok(JsValue::undefined())
    });

    vm.register_host_fn("__sentinel_emit_finding", host_emit_finding);
    vm.register_host_fn("__sentinel_return", host_return);
    vm.register_host_fn(
        "__sentinel_emit_active_probe_event",
        host_emit_active_probe_event,
    );
    vm.register_host_fn(
        "__sentinel_get_plugin_runtime_settings",
        host_get_plugin_runtime_settings,
    );
}

fn register_fetch_functions(vm: &mut Vm) {
    vm.register_host_fn("__sentinel_fetch", host_fetch);
    vm.register_host_fn("__sentinel_abort_fetch", |_vm, _args| {
        Ok(JsValue::from_bool(false))
    });
}

fn register_network_functions(vm: &mut Vm) {
    vm.register_host_fn("__sentinel_scan_ports", host_scan_ports);
    vm.register_host_fn("__sentinel_probe_services", host_probe_services);
    vm.register_host_fn(
        "__sentinel_get_service_probe_capabilities",
        host_get_service_probe_capabilities,
    );
}

fn register_filesystem_functions(vm: &mut Vm) {
    vm.register_host_fn("__sentinel_read_text_file", host_read_text_file);
    vm.register_host_fn("__sentinel_write_text_file", host_write_text_file);
    vm.register_host_fn("__sentinel_read_file", host_read_file);
    vm.register_host_fn("__sentinel_write_file", host_write_file);
    vm.register_host_fn("__sentinel_mkdir", host_mkdir);
    vm.register_host_fn("__sentinel_read_dir", host_read_dir);
    vm.register_host_fn("__sentinel_stat", host_stat);
    vm.register_host_fn("__sentinel_copy_file", host_copy_file);
    vm.register_host_fn("__sentinel_remove", host_remove);
    vm.register_host_fn("__sentinel_make_temp_file", host_make_temp_file);
}

fn register_dictionary_functions(vm: &mut Vm) {
    vm.register_host_fn("__sentinel_get_dictionary", |vm, args| {
        let id = args
            .first()
            .map(|v| vm.value_to_string(*v).to_string())
            .unwrap_or_default();
        if id.is_empty() {
            return Ok(JsValue::undefined());
        }
        match block_on_async(crate::dictionary_runtime::get_dictionary(id)) {
            Ok(Some(dict)) => {
                let json = serde_json::to_value(dict).unwrap_or(serde_json::Value::Null);
                Ok(one_engine::json_to_js(vm, &json))
            }
            _ => Ok(JsValue::undefined()),
        }
    });
    vm.register_host_fn("__sentinel_get_default_dictionary_id", |vm, args| {
        let dt = args
            .first()
            .map(|v| vm.value_to_string(*v).to_string())
            .unwrap_or_default();
        if dt.is_empty() {
            return Ok(vm.alloc_string(""));
        }
        match block_on_async(crate::dictionary_runtime::get_default_dictionary_id(dt)) {
            Ok(id) => Ok(vm.alloc_string(&id)),
            Err(_) => Ok(vm.alloc_string("")),
        }
    });
    vm.register_host_fn("__sentinel_get_dictionary_words", |vm, args| {
        let id = args
            .first()
            .map(|v| vm.value_to_string(*v).to_string())
            .unwrap_or_default();
        if id.is_empty() {
            return Ok(vm.new_array_from_elements(Vec::new()));
        }
        let lim = args.get(1).and_then(|v| {
            let n = v.to_number();
            if n.is_finite() && n > 0.0 { Some(n as i32) } else { None }
        });
        match block_on_async(crate::dictionary_runtime::get_dictionary_words(id, lim)) {
            Ok(words) => {
                let json = serde_json::to_value(words).unwrap_or(serde_json::Value::Array(vec![]));
                Ok(one_engine::json_to_js(vm, &json))
            }
            Err(_) => Ok(vm.new_array_from_elements(Vec::new())),
        }
    });
    vm.register_host_fn("__sentinel_get_dictionary_entries", |vm, args| {
        let id = args
            .first()
            .map(|v| vm.value_to_string(*v).to_string())
            .unwrap_or_default();
        if id.is_empty() {
            return Ok(vm.new_array_from_elements(Vec::new()));
        }
        let lim = args.get(1).and_then(|v| {
            let n = v.to_number();
            if n.is_finite() && n > 0.0 { Some(n as i32) } else { None }
        });
        match block_on_async(crate::dictionary_runtime::get_dictionary_entries(id, lim)) {
            Ok(entries) => {
                let json = serde_json::to_value(entries).unwrap_or(serde_json::Value::Array(vec![]));
                Ok(one_engine::json_to_js(vm, &json))
            }
            Err(_) => Ok(vm.new_array_from_elements(Vec::new())),
        }
    });
    vm.register_host_fn("__sentinel_list_dictionaries", |vm, args| {
        let filter_json = args
            .first()
            .map(|v| one_engine::js_to_json(vm, *v))
            .unwrap_or(serde_json::Value::Null);
        let dict_type = filter_json.get("dictType").or(filter_json.get("dict_type"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let category = filter_json.get("category")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        match block_on_async(crate::dictionary_runtime::list_dictionaries(dict_type, category)) {
            Ok(dicts) => {
                let json = serde_json::to_value(dicts).unwrap_or(serde_json::Value::Array(vec![]));
                Ok(one_engine::json_to_js(vm, &json))
            }
            Err(_) => Ok(vm.new_array_from_elements(Vec::new())),
        }
    });
}

fn register_tls_functions(vm: &mut Vm) {
    vm.register_host_fn("__sentinel_get_tls_certificate", host_get_tls_certificate);
}

fn register_monitor_functions(vm: &mut Vm) {
    vm.register_host_fn(
        "__sentinel_report_monitor_progress",
        host_report_monitor_progress,
    );
}

fn register_ast_functions(vm: &mut Vm) {
    vm.register_host_fn("__sentinel_parse_js", host_parse_js);
}

// ---------------------------------------------------------------------------
// Host function implementations
// ---------------------------------------------------------------------------

fn host_emit_finding(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let val = args.first().copied().unwrap_or(JsValue::undefined());
    let json = one_engine::js_to_json(vm, val);
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
    Ok(JsValue::from_bool(true))
}

fn host_return(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let val = args.first().copied().unwrap_or(JsValue::undefined());
    let json = one_engine::js_to_json(vm, val);
    with_plugin_ctx(|pctx| {
        let mut last = pctx.last_result.lock().unwrap();
        *last = Some(json);
    });
    Ok(JsValue::from_bool(true))
}

fn host_emit_active_probe_event(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let val = args.first().copied().unwrap_or(JsValue::undefined());
    let json = one_engine::js_to_json(vm, val);
    let update: crate::plugin_ops::ActiveProbeRuntimeUpdate = match serde_json::from_value(json) {
        Ok(v) => v,
        Err(_) => return Ok(JsValue::from_bool(false)),
    };
    if update.request_id.trim().is_empty()
        || update.phase.trim().is_empty()
        || update.url.trim().is_empty()
    {
        return Ok(JsValue::from_bool(false));
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
    Ok(JsValue::from_bool(true))
}

fn host_get_plugin_runtime_settings(vm: &mut Vm, _args: &[JsValue]) -> OneResult<JsValue> {
    let settings = crate::runtime_config::get_plugin_runtime_settings();
    let json = serde_json::to_value(settings).unwrap_or(serde_json::Value::Null);
    Ok(one_engine::json_to_js(vm, &json))
}

fn host_fetch(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let url_val = args.first().copied().unwrap_or(JsValue::undefined());
    let opts_val = args.get(1).copied();

    let url = if url_val.is_string() {
        vm.value_to_string(url_val).to_string()
    } else {
        let json = one_engine::js_to_json(vm, url_val);
        json.get("url")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };

    let opts_json = opts_val
        .map(|v| one_engine::js_to_json(vm, v))
        .unwrap_or(serde_json::Value::Null);

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

    let response =
        block_on_async(async { crate::plugin_engine::plugin_fetch(url, options).await });
    let json = serde_json::to_value(response).unwrap_or(serde_json::Value::Null);
    Ok(one_engine::json_to_js(vm, &json))
}

fn host_scan_ports(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let val = args.first().copied().unwrap_or(JsValue::undefined());
    let json = one_engine::js_to_json(vm, val);
    let request: crate::network_scan::PortScanRequest =
        serde_json::from_value(json).unwrap_or_default();
    let response = block_on_async(crate::network_scan::op_scan_ports(request));
    let result_json = serde_json::to_value(response).unwrap_or(serde_json::Value::Null);
    Ok(one_engine::json_to_js(vm, &result_json))
}

fn host_probe_services(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let val = args.first().copied().unwrap_or(JsValue::undefined());
    let json = one_engine::js_to_json(vm, val);
    let request: crate::service_probe_runtime::ServiceProbeRequest =
        serde_json::from_value(json).unwrap_or_default();
    let response = block_on_async(crate::service_probe_runtime::op_probe_services(request));
    let result_json = serde_json::to_value(response).unwrap_or(serde_json::Value::Null);
    Ok(one_engine::json_to_js(vm, &result_json))
}

fn host_get_service_probe_capabilities(vm: &mut Vm, _args: &[JsValue]) -> OneResult<JsValue> {
    let response = crate::service_probe::op_get_service_probe_capabilities();
    let json = serde_json::to_value(response).unwrap_or(serde_json::Value::Null);
    Ok(one_engine::json_to_js(vm, &json))
}

fn host_read_text_file(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let path = args
        .first()
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    match block_on_async(tokio::fs::read_to_string(&path)) {
        Ok(content) => Ok(vm.alloc_string(content.as_str())),
        Err(e) => Err(one_core::OneError::js_exception(
            "Error",
            &format!("read '{path}': {e}"),
        )),
    }
}

fn host_write_text_file(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let path = args
        .first()
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    let content = args
        .get(1)
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    match block_on_async(tokio::fs::write(&path, &content)) {
        Ok(()) => Ok(JsValue::undefined()),
        Err(e) => Err(one_core::OneError::js_exception(
            "Error",
            &format!("write '{path}': {e}"),
        )),
    }
}

fn host_read_file(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let path = args
        .first()
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    match block_on_async(tokio::fs::read(&path)) {
        Ok(bytes) => {
            let elements: Vec<JsValue> = bytes.iter().map(|&b| JsValue::from_i32(b as i32)).collect();
            Ok(vm.new_array_from_elements(elements))
        }
        Err(e) => Err(one_core::OneError::js_exception(
            "Error",
            &format!("read '{path}': {e}"),
        )),
    }
}

fn host_write_file(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let path = args
        .first()
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    let data_val = args.get(1).copied().unwrap_or(JsValue::undefined());
    let mut bytes = Vec::new();
    if let Some(obj) = vm.get_object(data_val) {
        if let Some(len_val) = obj.get_property("length") {
            let len = len_val.to_number() as usize;
            for i in 0..len {
                if let Some(v) = obj.get_property(&i.to_string()) {
                    bytes.push(v.to_number() as u8);
                }
            }
        }
    }
    match block_on_async(tokio::fs::write(&path, &bytes)) {
        Ok(()) => Ok(JsValue::undefined()),
        Err(e) => Err(one_core::OneError::js_exception(
            "Error",
            &format!("write '{path}': {e}"),
        )),
    }
}

fn host_mkdir(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let path = args
        .first()
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    let recursive = args
        .get(1)
        .map(|v| v.to_number() != 0.0)
        .unwrap_or(false);
    let result = if recursive {
        block_on_async(tokio::fs::create_dir_all(&path))
    } else {
        block_on_async(tokio::fs::create_dir(&path))
    };
    match result {
        Ok(()) => Ok(JsValue::undefined()),
        Err(e) => Err(one_core::OneError::js_exception(
            "Error",
            &format!("mkdir '{path}': {e}"),
        )),
    }
}

fn host_read_dir(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let path = args
        .first()
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    let mut entries = Vec::new();
    let mut read_dir = block_on_async(tokio::fs::read_dir(&path)).map_err(|e| {
        one_core::OneError::js_exception("Error", &format!("readdir '{path}': {e}"))
    })?;
    while let Some(entry) = block_on_async(read_dir.next_entry())
        .map_err(|e| one_core::OneError::js_exception("Error", &format!("readdir entry: {e}")))?
    {
        let meta = block_on_async(entry.metadata()).ok();
        let json = serde_json::json!({
            "name": entry.file_name().to_string_lossy(),
            "isFile": meta.as_ref().map(|m| m.is_file()).unwrap_or(false),
            "isDirectory": meta.as_ref().map(|m| m.is_dir()).unwrap_or(false),
            "isSymlink": meta.as_ref().map(|m| m.is_symlink()).unwrap_or(false),
        });
        entries.push(one_engine::json_to_js(vm, &json));
    }
    Ok(vm.new_array_from_elements(entries))
}

fn host_stat(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let path = args
        .first()
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    let meta = block_on_async(tokio::fs::metadata(&path)).map_err(|e| {
        one_core::OneError::js_exception("Error", &format!("stat '{path}': {e}"))
    })?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);
    let json = serde_json::json!({
        "size": meta.len(),
        "isFile": meta.is_file(),
        "isDirectory": meta.is_dir(),
        "isSymlink": meta.is_symlink(),
        "mtime": mtime,
    });
    Ok(one_engine::json_to_js(vm, &json))
}

fn host_copy_file(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let from = args
        .first()
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    let to = args
        .get(1)
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    block_on_async(tokio::fs::copy(&from, &to))
        .map(|_| JsValue::undefined())
        .map_err(|e| one_core::OneError::js_exception("Error", &format!("copy: {e}")))
}

fn host_remove(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let path = args
        .first()
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    let recursive = args
        .get(1)
        .map(|v| v.to_number() != 0.0)
        .unwrap_or(false);
    let meta = block_on_async(tokio::fs::metadata(&path)).map_err(|e| {
        one_core::OneError::js_exception("Error", &format!("remove '{path}': {e}"))
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
    result
        .map(|_| JsValue::undefined())
        .map_err(|e| one_core::OneError::js_exception("Error", &format!("remove '{path}': {e}")))
}

fn host_make_temp_file(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let prefix = args
        .first()
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    let suffix = args
        .get(1)
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    let temp_dir = std::env::temp_dir();
    let name = format!("{prefix}{}{suffix}", uuid::Uuid::new_v4());
    let path = temp_dir.join(name);
    std::fs::File::create(&path).map_err(|e| {
        one_core::OneError::js_exception("Error", &format!("make_temp_file: {e}"))
    })?;
    Ok(vm.alloc_string(path.to_string_lossy().as_ref()))
}

fn host_get_tls_certificate(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let hostname = args
        .first()
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    let port = args.get(1).map(|v| v.to_number() as u16).unwrap_or(443);
    let response = crate::plugin_ops::TlsCertificateResponse {
        success: false,
        cert: None,
        error: Some(format!(
            "TLS certificate retrieval for {hostname}:{port} not implemented"
        )),
    };
    let json = serde_json::to_value(response).unwrap_or(serde_json::Value::Null);
    Ok(one_engine::json_to_js(vm, &json))
}

fn host_report_monitor_progress(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let val = args.first().copied().unwrap_or(JsValue::undefined());
    let json = one_engine::js_to_json(vm, val);
    let request: crate::monitor_progress::PluginMonitorProgressRequest =
        serde_json::from_value(json).unwrap_or_default();
    if let Some(context) = request.monitor_progress {
        crate::monitor_progress::emit_plugin_monitor_progress(&context, request.update);
        Ok(JsValue::from_bool(true))
    } else {
        Ok(JsValue::from_bool(false))
    }
}

fn host_parse_js(vm: &mut Vm, args: &[JsValue]) -> OneResult<JsValue> {
    let code = args
        .first()
        .map(|v| vm.value_to_string(*v).to_string())
        .unwrap_or_default();
    let filename = args.get(1).map(|v| vm.value_to_string(*v).to_string());
    let result = crate::plugin_engine::parse_js_literals(code, filename);
    let json = serde_json::to_value(result).unwrap_or(serde_json::Value::Null);
    Ok(one_engine::json_to_js(vm, &json))
}

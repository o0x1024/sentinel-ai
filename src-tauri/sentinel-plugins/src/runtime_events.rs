use serde::Serialize;
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter, Runtime};

pub const MONITOR_TASK_PROGRESS_EVENT: &str = "monitor:task-progress";
pub const ACTIVE_PROBE_EVENT: &str = "plugin:active-probe";

type RuntimeEmitter = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;

static APP_HANDLE: OnceLock<RuntimeEmitter> = OnceLock::new();

pub fn register_app_handle<R: Runtime>(handle: AppHandle<R>) {
    let emitter: RuntimeEmitter = Arc::new(move |event, payload| {
        let _ = handle.emit(event, payload.clone());
    });
    let _ = APP_HANDLE.set(emitter);
}

pub fn emit_monitor_task_progress<T>(payload: &T)
where
    T: Serialize,
{
    if let Some(handle) = APP_HANDLE.get() {
        if let Ok(payload) = serde_json::to_value(payload) {
            handle(MONITOR_TASK_PROGRESS_EVENT, payload);
        }
    }
}

pub fn emit_active_probe_event<T>(payload: &T)
where
    T: Serialize,
{
    if let Some(handle) = APP_HANDLE.get() {
        if let Ok(payload) = serde_json::to_value(payload) {
            handle(ACTIVE_PROBE_EVENT, payload);
        }
    }
}

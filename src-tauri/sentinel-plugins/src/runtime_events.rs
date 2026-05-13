use serde::Serialize;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Runtime};

pub const MONITOR_TASK_PROGRESS_EVENT: &str = "monitor:task-progress";
pub const ACTIVE_PROBE_EVENT: &str = "plugin:active-probe";
pub const ACTIVE_PROBE_QUEUE_EVENT: &str = "traffic:active-probe-queue-updated";

type RuntimeEmitter = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;

static APP_HANDLE: OnceLock<RuntimeEmitter> = OnceLock::new();
static SUPPRESSED_MONITOR_PROGRESS_RUNS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn suppressed_monitor_progress_runs() -> &'static Mutex<HashSet<String>> {
    SUPPRESSED_MONITOR_PROGRESS_RUNS.get_or_init(|| Mutex::new(HashSet::new()))
}

pub(crate) fn suppress_monitor_progress_for_run(run_id: &str) {
    if !run_id.starts_with("monitor:") {
        return;
    }

    let mut suppressed_runs = suppressed_monitor_progress_runs()
        .lock()
        .expect("suppressed monitor progress registry poisoned");
    suppressed_runs.insert(run_id.to_string());
}

pub(crate) fn clear_suppressed_monitor_progress_for_run(run_id: &str) {
    let mut suppressed_runs = suppressed_monitor_progress_runs()
        .lock()
        .expect("suppressed monitor progress registry poisoned");
    suppressed_runs.remove(run_id);
}

fn monitor_run_id_from_payload(payload: &serde_json::Value) -> Option<String> {
    let task_id = payload.get("task_id").and_then(|value| value.as_str())?;
    let started_at = payload.get("started_at").and_then(|value| value.as_str())?;
    Some(format!("monitor:{task_id}:{started_at}"))
}

fn is_suppressed_monitor_progress(payload: &serde_json::Value) -> bool {
    if payload.get("status").and_then(|value| value.as_str()) != Some("running") {
        return false;
    }

    let Some(run_id) = monitor_run_id_from_payload(payload) else {
        return false;
    };

    suppressed_monitor_progress_runs()
        .lock()
        .expect("suppressed monitor progress registry poisoned")
        .contains(&run_id)
}

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
            if is_suppressed_monitor_progress(&payload) {
                return;
            }
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

pub fn emit_active_probe_queue_event<T>(payload: &T)
where
    T: Serialize,
{
    if let Some(handle) = APP_HANDLE.get() {
        if let Ok(payload) = serde_json::to_value(payload) {
            handle(ACTIVE_PROBE_QUEUE_EVENT, payload);
        }
    }
}

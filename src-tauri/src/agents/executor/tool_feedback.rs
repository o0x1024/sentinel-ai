use std::sync::{Arc, Mutex};

use serde_json::json;
use tauri::{AppHandle, Emitter};

use crate::agents::executor::types::ToolCallRecord;
use crate::agents::tenth_man::{InterventionContext, TenthMan};
use crate::agents::AgentExecuteParams;

pub(super) fn spawn_tenth_man_warning(
    app: AppHandle,
    params: AgentExecuteParams,
    context: InterventionContext,
    trigger: &'static str,
    require_confirmation: bool,
    extra_payload: serde_json::Value,
) {
    tauri::async_runtime::spawn(async move {
        let tenth_man = TenthMan::new(&params);
        match tenth_man.quick_review(&context).await {
            Ok(Some(critique)) => {
                let mut payload = serde_json::Map::from_iter([
                    (
                        "execution_id".to_string(),
                        serde_json::Value::String(context.execution_id.clone()),
                    ),
                    (
                        "generation".to_string(),
                        params
                            .cancellation_generation
                            .map(serde_json::Value::from)
                            .unwrap_or(serde_json::Value::Null),
                    ),
                    (
                        "trigger".to_string(),
                        serde_json::Value::String(trigger.to_string()),
                    ),
                    ("critique".to_string(), serde_json::Value::String(critique)),
                    (
                        "requires_confirmation".to_string(),
                        serde_json::Value::Bool(require_confirmation),
                    ),
                ]);

                if let Some(extra_obj) = extra_payload.as_object() {
                    for (key, value) in extra_obj {
                        payload.insert(key.clone(), value.clone());
                    }
                }

                let _ = app.emit(
                    "agent:tenth_man_warning",
                    &serde_json::Value::Object(payload),
                );
            }
            Ok(None) => {
                tracing::debug!(
                    "Tenth Man: No significant risk detected for trigger {}",
                    trigger
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Tenth Man quick review failed for trigger {}: {}",
                    trigger,
                    e
                );
            }
        }
    });
}

pub(super) fn emit_retry_event(
    app_handle: &AppHandle,
    execution_id: &str,
    generation: Option<u64>,
    retries: usize,
    max_retries: usize,
    last_error: Option<&anyhow::Error>,
    accumulated_tool_calls: &Arc<Mutex<Vec<ToolCallRecord>>>,
    accumulated_assistant_output: &Arc<Mutex<String>>,
) {
    let _ = app_handle.emit(
        "agent:retry",
        &json!({
            "execution_id": execution_id,
            "generation": generation,
            "retry_count": retries,
            "max_retries": max_retries,
            "error": last_error.map(|e| e.to_string()),
            "accumulated_progress": {
                "tool_calls": accumulated_tool_calls.lock().map(|c| c.len()).unwrap_or(0),
                "output_chars": accumulated_assistant_output.lock().map(|s| s.len()).unwrap_or(0),
            }
        }),
    );
}

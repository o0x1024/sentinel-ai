use chrono::Utc;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub const MONITOR_PLUGIN_FAILURE_EVENT: &str = "monitor:plugin-failed";
pub const MONITOR_TASK_SUMMARY_EVENT: &str = "monitor:task-summary";

#[derive(Debug, Clone, Serialize)]
pub struct MonitorPluginFailureEvent {
    pub task_id: String,
    pub task_name: String,
    pub program_id: String,
    pub execution_mode: String,
    pub plugin_id: String,
    pub plugin_label: String,
    pub error: String,
    pub started_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonitorTaskSummaryEvent {
    pub task_id: String,
    pub task_name: String,
    pub program_id: String,
    pub execution_mode: String,
    pub status: String,
    pub imported_assets: u32,
    pub findings_created: u32,
    pub findings_updated: u32,
    pub started_at: String,
    pub completed_at: String,
    pub message: Option<String>,
}

pub fn normalize_monitor_error_message(message: &str) -> String {
    let mut current = message.trim();

    while let Some((prefix, remainder)) =
        current.split_once(':').or_else(|| current.split_once('：'))
    {
        let normalized_prefix = prefix.trim().to_ascii_lowercase();
        let normalized_remainder = remainder.trim();
        let is_wrapper = normalized_prefix.ends_with("failed")
            || normalized_prefix.ends_with("error")
            || normalized_prefix.ends_with("exception");

        if !is_wrapper || normalized_remainder.is_empty() {
            break;
        }

        current = normalized_remainder;
    }

    if current.is_empty() {
        "monitor plugin execution failed".to_string()
    } else {
        current.to_string()
    }
}

pub fn build_monitor_plugin_failure_event(
    task_id: &str,
    task_name: &str,
    program_id: &str,
    execution_mode: &str,
    plugin_id: &str,
    plugin_label: &str,
    error: &str,
    started_at: &str,
) -> MonitorPluginFailureEvent {
    MonitorPluginFailureEvent {
        task_id: task_id.to_string(),
        task_name: task_name.to_string(),
        program_id: program_id.to_string(),
        execution_mode: execution_mode.to_string(),
        plugin_id: plugin_id.to_string(),
        plugin_label: plugin_label.to_string(),
        error: normalize_monitor_error_message(error),
        started_at: started_at.to_string(),
        created_at: Utc::now().to_rfc3339(),
    }
}

pub fn build_monitor_task_summary_event(
    task_id: &str,
    task_name: &str,
    program_id: &str,
    execution_mode: &str,
    status: &str,
    imported_assets: usize,
    findings_created: usize,
    findings_updated: usize,
    started_at: &str,
    message: Option<String>,
) -> MonitorTaskSummaryEvent {
    MonitorTaskSummaryEvent {
        task_id: task_id.to_string(),
        task_name: task_name.to_string(),
        program_id: program_id.to_string(),
        execution_mode: execution_mode.to_string(),
        status: status.to_string(),
        imported_assets: imported_assets as u32,
        findings_created: findings_created as u32,
        findings_updated: findings_updated as u32,
        started_at: started_at.to_string(),
        completed_at: Utc::now().to_rfc3339(),
        message,
    }
}

pub fn emit_monitor_plugin_failure(app: &AppHandle, payload: &MonitorPluginFailureEvent) {
    let _ = app.emit(MONITOR_PLUGIN_FAILURE_EVENT, payload);
}

pub fn emit_monitor_task_summary(app: &AppHandle, payload: &MonitorTaskSummaryEvent) {
    let _ = app.emit(MONITOR_TASK_SUMMARY_EVENT, payload);
}

#[cfg(test)]
mod tests {
    use super::normalize_monitor_error_message;

    #[test]
    fn normalize_monitor_error_message_strips_nested_failed_wrappers() {
        let message = "Tool execution failed: Subdomain brute failed: subdomain brute force failed: 无法解析下一跳 198.18.0.1 在接口 utun6 上的MAC地址";

        assert_eq!(
            normalize_monitor_error_message(message),
            "无法解析下一跳 198.18.0.1 在接口 utun6 上的MAC地址"
        );
    }

    #[test]
    fn normalize_monitor_error_message_keeps_plain_reason() {
        assert_eq!(
            normalize_monitor_error_message("network unreachable"),
            "network unreachable"
        );
    }
}

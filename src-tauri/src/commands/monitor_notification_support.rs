use chrono::Utc;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub const MONITOR_TASK_SUMMARY_EVENT: &str = "monitor:task-summary";

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

pub fn emit_monitor_task_summary(app: &AppHandle, payload: &MonitorTaskSummaryEvent) {
    let _ = app.emit(MONITOR_TASK_SUMMARY_EVENT, payload);
}

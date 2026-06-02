use chrono::Utc;
use sentinel_bounty::services::{MonitorPluginConfig, MonitorTask};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub const MONITOR_TASK_PROGRESS_EVENT: &str = "monitor:task-progress";
pub const MONITOR_TASK_LOG_EVENT: &str = "monitor:task-log";

#[derive(Debug, Clone, Serialize)]
pub struct MonitorTaskProgressEvent {
    pub task_id: String,
    pub task_name: String,
    pub program_id: String,
    pub execution_mode: String,
    pub status: String,
    pub progress: u32,
    pub completed_steps: u32,
    pub total_steps: u32,
    pub current_plugin: Option<String>,
    pub current_plugin_index: Option<u32>,
    pub target_count: u32,
    pub imported_assets: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_breakdown_label: Option<String>,
    pub indeterminate: bool,
    pub message: Option<String>,
    pub started_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_completed_units: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_total_units: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_phase_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_target: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonitorTaskLogEvent {
    pub task_id: String,
    pub task_name: String,
    pub plugin_id: String,
    pub execution_mode: String,
    pub status: String,
    pub step_index: u32,
    pub total_steps: u32,
    pub duration_ms: Option<u64>,
    pub imported_assets_delta: u32,
    pub imported_assets_total: u32,
    pub message: String,
    pub occurred_at: String,
}

pub fn collect_monitor_plugins(task: &MonitorTask) -> Vec<MonitorPluginConfig> {
    let mut plugins = Vec::new();

    if task.config.enable_dns_monitoring {
        plugins.extend(task.config.dns_plugins.clone());
    }
    if task.config.enable_ip_monitoring {
        plugins.extend(task.config.ip_plugins.clone());
    }
    if task.config.enable_port_monitoring {
        plugins.extend(task.config.port_plugins.clone());
    }
    if task.config.enable_service_monitoring {
        plugins.extend(task.config.service_plugins.clone());
    }
    if task.config.enable_web_monitoring {
        plugins.extend(task.config.web_plugins.clone());
    }
    if task.config.enable_cert_monitoring {
        plugins.extend(task.config.cert_plugins.clone());
    }
    if task.config.enable_api_monitoring {
        plugins.extend(task.config.api_plugins.clone());
    }
    if task.config.enable_content_monitoring {
        plugins.extend(task.config.content_plugins.clone());
    }
    if task.config.enable_risk_monitoring {
        plugins.extend(task.config.risk_plugins.clone());
    }

    plugins
}

pub fn build_monitor_task_progress_event(
    task: &MonitorTask,
    execution_mode: &str,
    status: &str,
    completed_steps: usize,
    total_steps: usize,
    current_plugin: Option<&str>,
    current_plugin_index: Option<usize>,
    target_count: usize,
    imported_assets: usize,
    message: Option<String>,
    started_at: &str,
) -> MonitorTaskProgressEvent {
    let bounded_completed = completed_steps.min(total_steps);
    let progress = if total_steps == 0 {
        if matches!(status, "completed" | "stopped") {
            100
        } else {
            0
        }
    } else {
        ((bounded_completed as f64 / total_steps as f64) * 100.0).round() as u32
    };

    MonitorTaskProgressEvent {
        task_id: task.id.clone(),
        task_name: task.name.clone(),
        program_id: task.program_id.clone(),
        execution_mode: execution_mode.to_string(),
        status: status.to_string(),
        progress,
        completed_steps: bounded_completed as u32,
        total_steps: total_steps as u32,
        current_plugin: current_plugin.map(|value| value.to_string()),
        current_plugin_index: current_plugin_index.map(|value| value as u32),
        target_count: target_count as u32,
        imported_assets: imported_assets as u32,
        target_breakdown_label: None,
        indeterminate: false,
        message,
        started_at: started_at.to_string(),
        updated_at: Utc::now().to_rfc3339(),
        plugin_completed_units: None,
        plugin_total_units: None,
        plugin_phase: None,
        plugin_phase_label: None,
        current_target: None,
    }
}

pub fn with_monitor_target_breakdown(
    mut payload: MonitorTaskProgressEvent,
    target_breakdown_label: Option<String>,
) -> MonitorTaskProgressEvent {
    payload.target_breakdown_label = target_breakdown_label;
    payload
}

pub fn emit_monitor_task_progress(app: &AppHandle, payload: &MonitorTaskProgressEvent) {
    let _ = app.emit(MONITOR_TASK_PROGRESS_EVENT, payload);
}

pub fn build_monitor_task_log_event(
    task: &MonitorTask,
    plugin_id: &str,
    execution_mode: &str,
    status: &str,
    step_index: usize,
    total_steps: usize,
    duration_ms: Option<u64>,
    imported_assets_delta: usize,
    imported_assets_total: usize,
    message: String,
) -> MonitorTaskLogEvent {
    MonitorTaskLogEvent {
        task_id: task.id.clone(),
        task_name: task.name.clone(),
        plugin_id: plugin_id.to_string(),
        execution_mode: execution_mode.to_string(),
        status: status.to_string(),
        step_index: step_index as u32,
        total_steps: total_steps as u32,
        duration_ms,
        imported_assets_delta: imported_assets_delta as u32,
        imported_assets_total: imported_assets_total as u32,
        message,
        occurred_at: Utc::now().to_rfc3339(),
    }
}

pub fn emit_monitor_task_log(app: &AppHandle, payload: &MonitorTaskLogEvent) {
    let _ = app.emit(MONITOR_TASK_LOG_EVENT, payload);
}

use crate::runtime_events::emit_monitor_task_progress;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorProgressContext {
    #[serde(rename = "taskId", alias = "task_id")]
    pub task_id: String,
    #[serde(rename = "taskName", alias = "task_name")]
    pub task_name: String,
    #[serde(rename = "programId", alias = "program_id")]
    pub program_id: String,
    #[serde(rename = "executionMode", alias = "execution_mode")]
    pub execution_mode: String,
    #[serde(rename = "startedAt", alias = "started_at")]
    pub started_at: String,
    #[serde(rename = "currentPlugin", alias = "current_plugin")]
    pub current_plugin: String,
    #[serde(rename = "currentPluginIndex", alias = "current_plugin_index")]
    pub current_plugin_index: u32,
    #[serde(rename = "completedSteps", alias = "completed_steps")]
    pub completed_steps: u32,
    #[serde(rename = "totalSteps", alias = "total_steps")]
    pub total_steps: u32,
    #[serde(rename = "importedAssets", alias = "imported_assets", default)]
    pub imported_assets: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginMonitorProgressUpdate {
    #[serde(default)]
    pub current: Option<u32>,
    #[serde(default)]
    pub total: Option<u32>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub current_target: Option<String>,
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(default)]
    pub phase_label: Option<String>,
    #[serde(default)]
    pub indeterminate: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PluginMonitorProgressRequest {
    #[serde(default)]
    pub monitor_progress: Option<MonitorProgressContext>,
    #[serde(flatten)]
    pub update: PluginMonitorProgressUpdate,
}

#[derive(Debug, Clone, Serialize)]
struct MonitorTaskPluginProgressEvent {
    task_id: String,
    task_name: String,
    program_id: String,
    execution_mode: String,
    status: String,
    progress: u32,
    completed_steps: u32,
    total_steps: u32,
    current_plugin: Option<String>,
    current_plugin_index: Option<u32>,
    target_count: u32,
    imported_assets: u32,
    indeterminate: bool,
    message: Option<String>,
    started_at: String,
    updated_at: String,
    current_target: Option<String>,
    plugin_completed_units: Option<u32>,
    plugin_total_units: Option<u32>,
    plugin_phase: Option<String>,
    plugin_phase_label: Option<String>,
}

pub fn emit_plugin_monitor_progress(
    context: &MonitorProgressContext,
    update: PluginMonitorProgressUpdate,
) {
    let total_steps = context.total_steps.max(1);
    let completed_steps = context.completed_steps.min(total_steps);
    let progress_fraction = match (update.current, update.total) {
        (Some(current), Some(total)) if total > 0 => current.min(total) as f64 / total as f64,
        _ => 0.0,
    };
    let overall_progress = (((completed_steps as f64 + progress_fraction) / total_steps as f64)
        * 100.0)
        .round() as u32;

    emit_monitor_task_progress(&MonitorTaskPluginProgressEvent {
        task_id: context.task_id.clone(),
        task_name: context.task_name.clone(),
        program_id: context.program_id.clone(),
        execution_mode: context.execution_mode.clone(),
        status: "running".to_string(),
        progress: overall_progress.min(99),
        completed_steps,
        total_steps,
        current_plugin: Some(context.current_plugin.clone()),
        current_plugin_index: Some(context.current_plugin_index),
        target_count: 0,
        imported_assets: context.imported_assets,
        indeterminate: update.indeterminate.unwrap_or(false),
        message: update.message,
        started_at: context.started_at.clone(),
        updated_at: Utc::now().to_rfc3339(),
        current_target: update.current_target,
        plugin_completed_units: update.current,
        plugin_total_units: update.total,
        plugin_phase: update.phase,
        plugin_phase_label: update.phase_label,
    });
}

use std::sync::Arc;

use sentinel_db::{DatabaseService, SurfaceDiscoveryRunRow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorRunHistoryItem {
    pub run_id: String,
    pub program_id: String,
    pub task_id: Option<String>,
    pub task_name: Option<String>,
    pub trigger_source: String,
    pub execution_mode: Option<String>,
    pub plugin_id: Option<String>,
    pub status: String,
    pub observation_count: i32,
    pub imported_asset_count: i32,
    pub changed_asset_count: i32,
    pub error_message: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub summary: Option<Value>,
}

fn parse_run_metadata(run: &SurfaceDiscoveryRunRow) -> Value {
    run.metadata_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<Value>(value).ok())
        .unwrap_or(Value::Null)
}

fn string_value(value: &Value, key: &str) -> Option<String> {
    value.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn run_to_history_item(run: SurfaceDiscoveryRunRow) -> Option<MonitorRunHistoryItem> {
    let metadata = parse_run_metadata(&run);
    let task_id = string_value(&metadata, "task_id");
    let task_name = string_value(&metadata, "task_name");
    if task_id.is_none() && task_name.is_none() {
        return None;
    }

    Some(MonitorRunHistoryItem {
        run_id: run.id,
        program_id: run.program_id,
        task_id,
        task_name,
        trigger_source: run.trigger_source,
        execution_mode: string_value(&metadata, "execution_mode"),
        plugin_id: run.plugin_id,
        status: run.status,
        observation_count: run.observation_count.unwrap_or(0),
        imported_asset_count: run.imported_asset_count.unwrap_or(0),
        changed_asset_count: run.changed_asset_count.unwrap_or(0),
        error_message: run.error_message,
        started_at: run.started_at,
        completed_at: run.completed_at,
        summary: metadata.get("summary").cloned(),
    })
}

#[tauri::command]
pub async fn monitor_list_run_history(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<MonitorRunHistoryItem>, String> {
    let safe_limit = limit.unwrap_or(100).clamp(1, 500);
    let runs = db_service
        .list_surface_discovery_runs(program_id.as_deref(), Some(safe_limit))
        .await
        .map_err(|error| error.to_string())?;

    Ok(runs.into_iter().filter_map(run_to_history_item).collect())
}

use sentinel_db::models::task_tool::*;
use sentinel_db::DatabaseService;
use std::sync::Arc;
use tauri::State;

/// Get active tools for a scan task
#[tauri::command]
pub async fn get_task_active_tools(
    task_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<ActiveToolInfo>, String> {
    db.get_task_active_tools(task_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get tool statistics for a scan task
#[tauri::command]
pub async fn get_task_tool_statistics(
    task_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<ToolStatistics, String> {
    db.get_task_tool_statistics(task_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get tool execution history for a scan task
#[tauri::command]
pub async fn get_tool_execution_history(
    task_id: String,
    tool_id: Option<String>,
    limit: Option<i64>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<ExecutionRecord>, String> {
    db.get_tool_execution_history(task_id, tool_id, limit)
        .await
        .map_err(|e| e.to_string())
}

/// Manually record tool execution start (for testing/debugging)
#[tauri::command]
pub async fn record_tool_execution_start(
    task_id: String,
    tool_id: String,
    tool_name: String,
    tool_type: String,
    input_params: Option<serde_json::Value>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let tool_type_enum = tool_type
        .parse::<ToolType>()
        .map_err(|e| format!("Invalid tool type: {}", e))?;

    db.record_tool_execution_start(task_id, tool_id, tool_name, tool_type_enum, input_params)
        .await
        .map_err(|e| e.to_string())
}

/// Manually record tool execution completion (for testing/debugging)
#[tauri::command]
pub async fn record_tool_execution_complete(
    log_id: String,
    success: bool,
    output_result: Option<serde_json::Value>,
    error_message: Option<String>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db.record_tool_execution_complete(log_id, success, output_result, error_message)
        .await
        .map_err(|e| e.to_string())
}

/// Get all active tools across all tasks
#[tauri::command]
pub async fn get_all_active_tools(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<ActiveToolInfo>, String> {
    db.get_all_active_tools()
        .await
        .map_err(|e| e.to_string())
}

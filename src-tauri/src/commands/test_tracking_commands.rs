use sentinel_db::models::task_tool::ToolType;
use sentinel_db::DatabaseService;
use std::sync::Arc;
use tauri::State;

/// Test tool execution tracking - simulates a plugin execution
#[tauri::command]
pub async fn test_plugin_tracking(
    task_id: String,
    plugin_id: String,
    plugin_name: String,
    _db: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let tracker = crate::trackers::get_tracker();
    
    if tracker.is_none() {
        return Err("Tracker not initialized".to_string());
    }
    
    let tracker = tracker.unwrap();
    
    // Start tracking
    let log_id = tracker
        .track_start(
            task_id.clone(),
            plugin_id.clone(),
            plugin_name.clone(),
            ToolType::Plugin,
            Some(serde_json::json!({
                "test": true,
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        )
        .await
        .map_err(|e| e.to_string())?;
    
    // Simulate execution
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Complete tracking
    tracker
        .track_complete(
            log_id.clone(),
            task_id,
            plugin_id,
            true,
            Some(serde_json::json!({
                "result": "success",
                "findings_count": 3
            })),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(log_id)
}

/// Test MCP tool execution tracking
#[tauri::command]
pub async fn test_mcp_tracking(
    task_id: String,
    connection_id: String,
    tool_name: String,
    _db: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let tracker = crate::trackers::get_tracker();
    
    if tracker.is_none() {
        return Err("Tracker not initialized".to_string());
    }
    
    let tracker = tracker.unwrap();
    
    let tool_id = format!("mcp:{}:{}", connection_id, tool_name);
    
    // Start tracking
    let log_id = tracker
        .track_start(
            task_id.clone(),
            tool_id.clone(),
            tool_name.clone(),
            ToolType::McpServer,
            Some(serde_json::json!({
                "connection_id": connection_id,
                "test": true
            })),
        )
        .await
        .map_err(|e| e.to_string())?;
    
    // Simulate execution
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    
    // Complete tracking
    tracker
        .track_complete(
            log_id.clone(),
            task_id,
            tool_id,
            true,
            Some(serde_json::json!({
                "result": "Tool executed successfully"
            })),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(log_id)
}

/// Test builtin tool execution tracking
#[tauri::command]
pub async fn test_builtin_tracking(
    task_id: String,
    tool_name: String,
    _db: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let tracker = crate::trackers::get_tracker();
    
    if tracker.is_none() {
        return Err("Tracker not initialized".to_string());
    }
    
    let tracker = tracker.unwrap();
    
    let tool_id = format!("builtin:{}", tool_name);
    
    // Start tracking
    let log_id = tracker
        .track_start(
            task_id.clone(),
            tool_id.clone(),
            tool_name.clone(),
            ToolType::Builtin,
            Some(serde_json::json!({
                "target": "example.com",
                "test": true
            })),
        )
        .await
        .map_err(|e| e.to_string())?;
    
    // Simulate execution
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // Complete tracking
    tracker
        .track_complete(
            log_id.clone(),
            task_id,
            tool_id,
            true,
            Some(serde_json::json!({
                "ports_found": [80, 443, 8080]
            })),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(log_id)
}

/// Test error tracking
#[tauri::command]
pub async fn test_error_tracking(
    task_id: String,
    tool_id: String,
    tool_name: String,
    _db: State<'_, Arc<DatabaseService>>,
) -> Result<String, String> {
    let tracker = crate::trackers::get_tracker();
    
    if tracker.is_none() {
        return Err("Tracker not initialized".to_string());
    }
    
    let tracker = tracker.unwrap();
    
    // Start tracking
    let log_id = tracker
        .track_start(
            task_id.clone(),
            tool_id.clone(),
            tool_name.clone(),
            ToolType::Plugin,
            Some(serde_json::json!({
                "test": "error_case"
            })),
        )
        .await
        .map_err(|e| e.to_string())?;
    
    // Simulate execution failure
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Track error
    tracker
        .track_error(
            log_id.clone(),
            task_id,
            tool_id,
            "Simulated error: Connection timeout".to_string(),
        )
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(log_id)
}

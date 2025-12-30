use anyhow::Result;
use sentinel_db::models::task_tool::ToolType;
use sentinel_tools::ToolResult;
use tracing::{debug, error};

use crate::trackers::get_tracker;

/// Execute a builtin tool with tracking
pub async fn execute_builtin_tool_with_tracking(
    tool_name: String,
    inputs: serde_json::Value,
    task_id: Option<String>,
) -> Result<ToolResult> {
    let tracker = get_tracker();
    let mut log_id: Option<String> = None;

    // Start tracking if task_id is provided
    if let (Some(ref t), Some(ref tid)) = (&tracker, &task_id) {
        match t
            .track_start(
                tid.clone(),
                format!("builtin:{}", tool_name),
                tool_name.clone(),
                ToolType::Builtin,
                Some(inputs.clone()),
            )
            .await
        {
            Ok(id) => {
                log_id = Some(id);
                debug!("Started tracking builtin tool execution: {}", tool_name);
            }
            Err(e) => {
                error!("Failed to track builtin tool start: {}", e);
            }
        }
    }

    // Execute builtin tool
    let tool_server = sentinel_tools::get_tool_server();
    tool_server.init_builtin_tools().await;
    let result = tool_server.execute(&tool_name, inputs).await;

    // Complete tracking
    if let (Some(ref t), Some(ref tid), Some(ref lid)) = (&tracker, &task_id, &log_id) {
        let tool_id = format!("builtin:{}", tool_name);
        
        let _ = t
            .track_complete(
                lid.clone(),
                tid.clone(),
                tool_id,
                result.success,
                result.output.clone(),
                result.error.clone(),
            )
            .await;
    }

    Ok(result)
}

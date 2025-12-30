use anyhow::Result;
use sentinel_db::models::task_tool::ToolType;
use sentinel_plugins::{Finding, HttpTransaction, PluginManager};
use tracing::{debug, error};

use crate::trackers::get_tracker;

/// Execute a traffic analysis plugin with tracking
pub async fn execute_plugin_with_tracking(
    plugin_manager: &PluginManager,
    plugin_id: &str,
    plugin_name: &str,
    transaction: &HttpTransaction,
    task_id: Option<String>,
) -> Result<Vec<Finding>> {
    let tracker = get_tracker();
    let mut log_id: Option<String> = None;

    // Start tracking if task_id is provided
    if let (Some(ref t), Some(ref tid)) = (&tracker, &task_id) {
        match t
            .track_start(
                tid.clone(),
                plugin_id.to_string(),
                plugin_name.to_string(),
                ToolType::Plugin,
                Some(serde_json::json!({
                    "request_id": transaction.request.id,
                    "url": transaction.request.url,
                    "method": transaction.request.method,
                })),
            )
            .await
        {
            Ok(id) => {
                log_id = Some(id);
                debug!("Started tracking plugin execution: {}", plugin_id);
            }
            Err(e) => {
                error!("Failed to track plugin start: {}", e);
            }
        }
    }

    // Execute plugin
    let result = plugin_manager
        .scan_transaction(plugin_id, transaction)
        .await;

    // Complete tracking
    if let (Some(ref t), Some(ref tid), Some(ref lid)) = (&tracker, &task_id, &log_id) {
        match &result {
            Ok(findings) => {
                let _ = t
                    .track_complete(
                        lid.clone(),
                        tid.clone(),
                        plugin_id.to_string(),
                        true,
                        Some(serde_json::json!({
                            "findings_count": findings.len(),
                        })),
                        None,
                    )
                    .await;
            }
            Err(e) => {
                let _ = t
                    .track_error(
                        lid.clone(),
                        tid.clone(),
                        plugin_id.to_string(),
                        e.to_string(),
                    )
                    .await;
            }
        }
    }

    result.map_err(|e| anyhow::anyhow!(e))
}

/// Execute an agent plugin with tracking
pub async fn execute_agent_plugin_with_tracking(
    plugin_manager: &PluginManager,
    plugin_id: &str,
    plugin_name: &str,
    input: &serde_json::Value,
    task_id: Option<String>,
) -> Result<(Vec<Finding>, Option<serde_json::Value>)> {
    let tracker = get_tracker();
    let mut log_id: Option<String> = None;

    // Start tracking if task_id is provided
    if let (Some(ref t), Some(ref tid)) = (&tracker, &task_id) {
        match t
            .track_start(
                tid.clone(),
                plugin_id.to_string(),
                plugin_name.to_string(),
                ToolType::Plugin,
                Some(input.clone()),
            )
            .await
        {
            Ok(id) => {
                log_id = Some(id);
                debug!("Started tracking agent plugin execution: {}", plugin_id);
            }
            Err(e) => {
                error!("Failed to track agent plugin start: {}", e);
            }
        }
    }

    // Execute plugin
    let result = plugin_manager.execute_agent(plugin_id, input).await;

    // Complete tracking
    if let (Some(ref t), Some(ref tid), Some(ref lid)) = (&tracker, &task_id, &log_id) {
        match &result {
            Ok((findings, output)) => {
                let _ = t
                    .track_complete(
                        lid.clone(),
                        tid.clone(),
                        plugin_id.to_string(),
                        true,
                        Some(serde_json::json!({
                            "findings_count": findings.len(),
                            "has_output": output.is_some(),
                        })),
                        None,
                    )
                    .await;
            }
            Err(e) => {
                let _ = t
                    .track_error(
                        lid.clone(),
                        tid.clone(),
                        plugin_id.to_string(),
                        e.to_string(),
                    )
                    .await;
            }
        }
    }

    result.map_err(|e| anyhow::anyhow!(e))
}

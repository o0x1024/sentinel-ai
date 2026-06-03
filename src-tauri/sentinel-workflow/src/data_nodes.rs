use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use sentinel_db::Database;
use sentinel_db::DatabaseService;
use serde_json::Value;

use crate::engine::WorkflowEngine;

pub async fn execute_raw_node(inputs: &HashMap<String, Value>) -> Result<Value, String> {
    let raw_type = inputs
        .get("raw_type")
        .and_then(|v| v.as_str())
        .unwrap_or("json");
    let value = inputs
        .get("value")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    match raw_type {
        "text" => Ok(Value::String(value)),
        _ => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Ok(Value::Null)
            } else {
                serde_json::from_str::<Value>(trimmed)
                    .map_err(|e| format!("Invalid raw JSON value: {}", e))
            }
        }
    }
}

pub async fn execute_data_node_action(
    action: &str,
    inputs: &HashMap<String, Value>,
    execution_id: &str,
    node_id: &str,
    engine: &Arc<WorkflowEngine>,
    db: &Arc<DatabaseService>,
) -> bool {
    let result = match action {
        "raw" => execute_raw_node(inputs).await,
        _ => return false,
    };

    match result {
        Ok(result_json) => {
            engine
                .mark_step_completed_with_result(execution_id, node_id, result_json.clone())
                .await;
            if let Err(e) = db
                .update_workflow_run_step_status(
                    execution_id,
                    node_id,
                    "completed",
                    Utc::now(),
                    Some(result_json.to_string()),
                    None,
                )
                .await
            {
                tracing::warn!("failed to update step status: {}", e);
            }
        }
        Err(err) => {
            let error_val = serde_json::json!({ "error": err });
            engine
                .mark_step_completed_with_result(execution_id, node_id, error_val.clone())
                .await;
            if let Err(e) = db
                .update_workflow_run_step_status(
                    execution_id,
                    node_id,
                    "failed",
                    Utc::now(),
                    Some(error_val.to_string()),
                    error_val.get("error").and_then(|v| v.as_str()),
                )
                .await
            {
                tracing::warn!("failed to update step status: {}", e);
            }
        }
    }

    true
}

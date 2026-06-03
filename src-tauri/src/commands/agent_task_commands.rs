use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

use crate::services::database::DatabaseService;

#[derive(Debug, Clone, Serialize)]
pub struct AgentTaskHistoryItem {
    pub id: String,
    pub execution_id: String,
    pub conversation_id: Option<String>,
    pub item_index: i32,
    pub content: String,
    pub status: String,
    pub result: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

fn parse_rfc3339_millis(value: &str) -> i64 {
    DateTime::parse_from_rfc3339(value)
        .map(|ts| ts.with_timezone(&Utc).timestamp_millis())
        .unwrap_or(0)
}

#[tauri::command]
pub async fn get_agent_tasks(
    execution_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<AgentTaskHistoryItem>, String> {
    db_service
        .get_execution_tasks(&execution_id)
        .await
        .map(|items| {
            items
                .into_iter()
                .map(|item| AgentTaskHistoryItem {
                    id: item.id,
                    execution_id: item.execution_id,
                    conversation_id: None,
                    item_index: item.item_index,
                    content: item.description,
                    status: item.status,
                    result: item.result,
                    created_at_ms: parse_rfc3339_millis(&item.created_at),
                    updated_at_ms: parse_rfc3339_millis(&item.updated_at),
                })
                .collect()
        })
        .map_err(|e| format!("Failed to load task history for {}: {}", execution_id, e))
}

#[tauri::command]
pub async fn get_agent_tasks_for_conversation(
    conversation_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<AgentTaskHistoryItem>, String> {
    let turns = db_service
        .list_agent_execution_turns_for_conversation(&conversation_id)
        .await
        .map_err(|e| {
            format!(
                "Failed to load execution turns for conversation {}: {}",
                conversation_id, e
            )
        })?;

    let mut rows = Vec::new();
    for turn in turns {
        let tasks = db_service
            .get_execution_tasks(&turn.turn_id)
            .await
            .map_err(|e| {
                format!(
                    "Failed to load task history for turn {} in conversation {}: {}",
                    turn.turn_id, conversation_id, e
                )
            })?;

        rows.extend(tasks.into_iter().map(|item| AgentTaskHistoryItem {
            id: item.id,
            execution_id: item.execution_id,
            conversation_id: Some(turn.conversation_id.clone()),
            item_index: item.item_index,
            content: item.description,
            status: item.status,
            result: item.result,
            created_at_ms: parse_rfc3339_millis(&item.created_at),
            updated_at_ms: parse_rfc3339_millis(&item.updated_at),
        }));
    }

    Ok(rows)
}

#[tauri::command]
pub async fn prune_agent_tasks_after(
    execution_id: String,
    after_timestamp_ms: i64,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<AgentTaskHistoryItem>, String> {
    db_service
        .delete_execution_tasks_after(&execution_id, after_timestamp_ms)
        .await
        .map_err(|e| {
            format!(
                "Failed to prune task history for {} after {}: {}",
                execution_id, after_timestamp_ms, e
            )
        })?;

    sentinel_tools::buildin_tools::tasks::invalidate_execution_tasks(&execution_id).await;

    get_agent_tasks(execution_id, db_service).await
}

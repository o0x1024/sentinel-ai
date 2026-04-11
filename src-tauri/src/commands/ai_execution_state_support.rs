use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use sentinel_db::Database;

use crate::commands::ai::{AgentExecutionFinishedEvent, AgentExecutionOutcome};
use crate::models::database::AiConversation;
use crate::services::ai::AiServiceManager;

#[derive(Debug, Clone, Serialize)]
pub struct AiConversationListItem {
    #[serde(flatten)]
    pub conversation: AiConversation,
    pub execution_state: Option<PersistedAgentExecutionState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedAgentExecutionState {
    pub outcome: AgentExecutionOutcome,
    pub success: bool,
    pub error: Option<String>,
    pub message: Option<String>,
    pub response: Option<String>,
    pub completed_at: String,
}

pub fn extract_execution_state(existing: Option<&str>) -> Option<PersistedAgentExecutionState> {
    existing
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|value| value.get("execution_state").cloned())
        .and_then(|value| serde_json::from_value::<PersistedAgentExecutionState>(value).ok())
}

fn merge_execution_state_json(
    existing: Option<&str>,
    state: &PersistedAgentExecutionState,
) -> Option<String> {
    let mut root = existing
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .filter(|value| value.is_object())
        .unwrap_or_else(|| serde_json::json!({}));

    if let Some(obj) = root.as_object_mut() {
        obj.insert(
            "execution_state".to_string(),
            serde_json::to_value(state).unwrap_or_else(|_| serde_json::json!({})),
        );
        serde_json::to_string(obj).ok()
    } else {
        None
    }
}

fn clear_execution_state_json(existing: Option<&str>) -> Option<String> {
    let mut root = existing
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .filter(|value| value.is_object())
        .unwrap_or_else(|| serde_json::json!({}));

    if let Some(obj) = root.as_object_mut() {
        obj.remove("execution_state");
        serde_json::to_string(obj).ok()
    } else {
        None
    }
}

pub fn decorate_conversation(conversation: AiConversation) -> AiConversationListItem {
    let execution_state = extract_execution_state(conversation.conversation_data.as_deref());
    AiConversationListItem {
        conversation,
        execution_state,
    }
}

pub async fn clear_persisted_agent_execution_state(
    app_handle: &AppHandle,
    conversation_id: &str,
) -> Result<()> {
    let Some(db) = app_handle.try_state::<std::sync::Arc<sentinel_db::DatabaseService>>() else {
        return Ok(());
    };

    let Some(mut conversation) = db.get_ai_conversation(conversation_id).await? else {
        return Ok(());
    };

    if extract_execution_state(conversation.conversation_data.as_deref()).is_none() {
        return Ok(());
    }

    conversation.conversation_data =
        clear_execution_state_json(conversation.conversation_data.as_deref());
    conversation.updated_at = Utc::now();
    db.update_ai_conversation(&conversation).await?;
    Ok(())
}

pub async fn persist_agent_execution_state(
    app_handle: &AppHandle,
    event: &AgentExecutionFinishedEvent,
) -> Result<()> {
    let Some(db) = app_handle.try_state::<std::sync::Arc<sentinel_db::DatabaseService>>() else {
        return Ok(());
    };

    let Some(mut conversation) = db.get_ai_conversation(&event.execution_id).await? else {
        return Ok(());
    };

    let state = PersistedAgentExecutionState {
        outcome: event.outcome,
        success: event.success,
        error: event.error.clone(),
        message: event.message.clone(),
        response: event.response.clone(),
        completed_at: Utc::now().to_rfc3339(),
    };

    conversation.conversation_data =
        merge_execution_state_json(conversation.conversation_data.as_deref(), &state);
    conversation.updated_at = Utc::now();

    db.update_ai_conversation(&conversation).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_ai_conversations(
    ai_manager: State<'_, std::sync::Arc<AiServiceManager>>,
) -> Result<Vec<AiConversationListItem>, String> {
    let services = ai_manager.list_services();
    if let Some(service_name) = services.first() {
        if let Some(service) = ai_manager.get_service(service_name) {
            return service
                .list_conversations()
                .await
                .map(|items| items.into_iter().map(decorate_conversation).collect())
                .map_err(|e| e.to_string());
        }
    }
    Ok(vec![])
}

#[tauri::command]
pub async fn get_ai_conversations_paginated(
    limit: i64,
    offset: i64,
    ai_manager: State<'_, std::sync::Arc<AiServiceManager>>,
) -> Result<Vec<AiConversationListItem>, String> {
    let services = ai_manager.list_services();
    if let Some(service_name) = services.first() {
        if let Some(service) = ai_manager.get_service(service_name) {
            return service
                .list_conversations_paginated(limit, offset)
                .await
                .map(|items| items.into_iter().map(decorate_conversation).collect())
                .map_err(|e| e.to_string());
        }
    }
    Ok(vec![])
}

#[tauri::command]
pub async fn get_ai_conversations_count(
    ai_manager: State<'_, std::sync::Arc<AiServiceManager>>,
) -> Result<i64, String> {
    let services = ai_manager.list_services();
    if let Some(service_name) = services.first() {
        if let Some(service) = ai_manager.get_service(service_name) {
            return service
                .get_conversations_count()
                .await
                .map_err(|e| e.to_string());
        }
    }
    Ok(0)
}

#[tauri::command]
pub async fn get_ai_conversation(
    conversation_id: String,
    db_service: State<'_, std::sync::Arc<crate::services::database::DatabaseService>>,
) -> Result<Option<AiConversationListItem>, String> {
    let conversation = db_service
        .get_ai_conversation_detail_internal(&conversation_id)
        .await
        .map_err(|e| format!("Failed to load conversation {}: {}", conversation_id, e))?;

    let Some(conversation) = conversation else {
        return Ok(None);
    };

    Ok(Some(decorate_conversation(conversation)))
}

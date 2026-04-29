use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use sentinel_db::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantConversationBinding {
    pub schema_version: u32,
    pub profile_id: String,
    pub context_mode: String,
    pub run_mode: String,
    pub rag_enabled: bool,
    pub web_search_enabled: bool,
    pub tenth_man_enabled: bool,
    pub browser_shell_direct_write_enabled: Option<bool>,
    pub browser_shell_session_id: Option<String>,
    pub selected_model: Option<String>,
    pub tools_enabled: Option<bool>,
    pub tool_config: Option<serde_json::Value>,
}

const BINDING_KEY: &str = "assistant_conversation_binding";

fn parse_root_json(existing: Option<&str>) -> serde_json::Value {
    existing
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .filter(|value| value.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

pub fn extract_conversation_binding(
    existing: Option<&str>,
) -> Option<AssistantConversationBinding> {
    parse_root_json(existing)
        .get(BINDING_KEY)
        .cloned()
        .and_then(|value| serde_json::from_value::<AssistantConversationBinding>(value).ok())
}

fn merge_conversation_binding_json(
    existing: Option<&str>,
    binding: &AssistantConversationBinding,
) -> Option<String> {
    let mut root = parse_root_json(existing);
    if let Some(obj) = root.as_object_mut() {
        obj.insert(
            BINDING_KEY.to_string(),
            serde_json::to_value(binding).unwrap_or_else(|_| serde_json::json!({})),
        );
        serde_json::to_string(obj).ok()
    } else {
        None
    }
}

pub async fn persist_conversation_binding(
    app_handle: &AppHandle,
    conversation_id: &str,
    binding: &AssistantConversationBinding,
) -> Result<()> {
    let Some(db) = app_handle.try_state::<std::sync::Arc<sentinel_db::DatabaseService>>() else {
        return Ok(());
    };

    let Some(mut conversation) = db.get_ai_conversation(conversation_id).await? else {
        return Ok(());
    };

    conversation.conversation_data =
        merge_conversation_binding_json(conversation.conversation_data.as_deref(), binding);
    conversation.updated_at = Utc::now();
    db.update_ai_conversation(&conversation).await?;
    Ok(())
}

#[tauri::command]
pub async fn save_ai_conversation_binding(
    conversation_id: String,
    binding: AssistantConversationBinding,
    app_handle: AppHandle,
) -> Result<(), String> {
    persist_conversation_binding(&app_handle, &conversation_id, &binding)
        .await
        .map_err(|e| {
            format!(
                "Failed to save conversation binding for {}: {}",
                conversation_id, e
            )
        })
}

#[tauri::command]
pub async fn get_ai_conversation_binding(
    conversation_id: String,
    db_service: State<'_, std::sync::Arc<crate::services::database::DatabaseService>>,
) -> Result<Option<AssistantConversationBinding>, String> {
    let conversation = db_service
        .get_ai_conversation(&conversation_id)
        .await
        .map_err(|e| format!("Failed to load conversation {}: {}", conversation_id, e))?;

    Ok(conversation
        .as_ref()
        .and_then(|item| extract_conversation_binding(item.conversation_data.as_deref())))
}

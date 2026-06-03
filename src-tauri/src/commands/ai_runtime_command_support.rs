use crate::commands::ai::{
    emit_agent_execution_finished_for_generation_with_conversation, AgentExecutionOutcome,
};
use crate::commands::ai_runtime_harness::AgentHarnessSuccessAssessment;
use crate::services::database::DatabaseService;
use sentinel_db::Database;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EffectiveImageAttachmentMode {
    Auto,
    LocalOcr,
    ModelVision,
}

pub(crate) const TENTH_MAN_REVIEW_TOOL_ID: &str = "tenth_man_review";

pub(crate) fn tool_config_allows_tool(config: &crate::agents::ToolConfig, tool_id: &str) -> bool {
    if !config.enabled || config.disabled_tools.iter().any(|id| id == tool_id) {
        return false;
    }

    if !config.allowed_tools.is_empty() {
        return config.allowed_tools.iter().any(|id| id == tool_id);
    }

    if let crate::agents::ToolSelectionStrategy::Manual(tools) = &config.selection_strategy {
        return tools.iter().any(|id| id == tool_id)
            || config.preselected_tools.iter().any(|id| id == tool_id);
    }

    true
}

pub(crate) fn build_vision_unsupported_message(provider: &str, model_name: &str) -> String {
    format!(
        "Current model does not support image understanding: {}/{}. Switch to a vision-capable model in the conversation work config, or explicitly change image handling to local OCR.",
        provider, model_name
    )
}

pub(crate) async fn settle_running_tool_messages(
    app_handle: &AppHandle,
    execution_id: &str,
    terminal_status: &str,
    reason: &str,
) {
    let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() else {
        return;
    };
    if let Err(error) = db
        .settle_running_ai_tool_messages(execution_id, terminal_status, reason)
        .await
    {
        tracing::warn!(
            "Failed to settle running tool messages for {} as {}: {}",
            execution_id,
            terminal_status,
            error
        );
    }
}

pub(crate) async fn emit_agent_harness_outcome(
    app_handle: &AppHandle,
    execution_id: &str,
    conversation_id: &str,
    generation: u64,
    assessment: &AgentHarnessSuccessAssessment,
    response: String,
) {
    let outcome = if assessment.succeeded() {
        AgentExecutionOutcome::Succeeded
    } else {
        AgentExecutionOutcome::Failed
    };
    emit_agent_execution_finished_for_generation_with_conversation(
        app_handle,
        execution_id,
        conversation_id,
        generation,
        outcome,
        assessment.error.clone(),
        Some(response),
        None,
    );
}

pub(crate) fn sanitize_image_attachments(attachments: &serde_json::Value) -> serde_json::Value {
    fn sanitize_one(value: &mut serde_json::Value) {
        let image = if value.get("type").and_then(|item| item.as_str()) == Some("image") {
            Some(value)
        } else {
            value.get_mut("image")
        };
        let Some(image) = image else { return };
        if let Some(object) = image.as_object_mut() {
            object.remove("source_path");
        }
    }

    let mut cloned = attachments.clone();
    if let Some(items) = cloned.as_array_mut() {
        for item in items.iter_mut() {
            sanitize_one(item);
        }
    } else if cloned.is_object() {
        sanitize_one(&mut cloned);
    }
    cloned
}

pub(crate) async fn load_tool_config_from_db(
    app_handle: &AppHandle,
) -> Option<crate::agents::ToolConfig> {
    if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
        if let Ok(Some(config_str)) = db.get_config("agent", "tool_config").await {
            if let Ok(config) = crate::agents::ToolConfig::from_json_str(&config_str) {
                tracing::info!("Loaded global tool config from database");
                return Some(config);
            }
        }
    }

    tracing::info!("No global tool config found, using default");
    None
}

pub(crate) async fn save_tool_config_to_db(
    app_handle: &AppHandle,
    config: &crate::agents::ToolConfig,
) -> Result<(), String> {
    let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() else {
        return Err("Database service not available".to_string());
    };
    let config_str = serde_json::to_string(config)
        .map_err(|error| format!("Failed to serialize tool config: {}", error))?;

    db.set_config(
        "agent",
        "tool_config",
        &config_str,
        Some("Global tool configuration"),
    )
    .await
    .map_err(|error| format!("Failed to save tool config: {}", error))?;

    tracing::info!("Saved global tool config to database");
    Ok(())
}

pub(crate) async fn persist_agent_execution_turn_start(
    db: &Arc<DatabaseService>,
    turn_id: &str,
    conversation_id: &str,
    user_message_id: &str,
    task: &str,
    harness_mode: &str,
) {
    if let Err(error) = db
        .upsert_agent_execution_turn_started(sentinel_db::AgentExecutionTurnStartInput {
            turn_id: turn_id.to_string(),
            conversation_id: conversation_id.to_string(),
            user_message_id: Some(user_message_id.to_string()),
            parent_turn_id: None,
            task: task.to_string(),
            harness_mode: Some(harness_mode.to_string()),
        })
        .await
    {
        tracing::warn!(
            "Failed to persist agent execution turn start for {}: {}",
            turn_id,
            error
        );
    }
}

pub(crate) async fn load_image_attachment_settings(
    db: &DatabaseService,
) -> (EffectiveImageAttachmentMode, bool) {
    let mode_str = db
        .get_config("agent", "image_attachment_mode")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "auto".to_string());
    let mode = match mode_str.as_str() {
        "auto" => EffectiveImageAttachmentMode::Auto,
        "model_vision" => EffectiveImageAttachmentMode::ModelVision,
        _ => EffectiveImageAttachmentMode::LocalOcr,
    };

    let allow_upload = db
        .get_config("agent", "allow_image_upload_to_model")
        .await
        .ok()
        .flatten()
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    (mode, allow_upload)
}

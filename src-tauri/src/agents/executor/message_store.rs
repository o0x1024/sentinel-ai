//! Message persistence helpers.

use serde_json::{json, Value};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

use sentinel_db::Database;

use crate::agents::executor::types::ToolCallRecord;

pub fn mark_first_response_ms(
    first_response_ms: &std::sync::Mutex<Option<i64>>,
    execution_started_at_ms: i64,
) {
    if let Ok(mut guard) = first_response_ms.lock() {
        if guard.is_none() {
            *guard = Some(chrono::Utc::now().timestamp_millis() - execution_started_at_ms);
        }
    }
}

pub fn build_assistant_session_stats_metadata(
    duration_ms: Option<i64>,
    first_response_ms: Option<i64>,
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
) -> Option<Value> {
    let duration_ms = duration_ms?;
    let input_tokens = input_tokens?;
    let output_tokens = output_tokens?;
    if duration_ms <= 0 {
        return None;
    }

    let duration_secs = duration_ms as f64 / 1000.0;
    let total_tokens = input_tokens + output_tokens;

    Some(json!({
        "session_stats": {
            "duration_ms": duration_ms,
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "total_tokens": total_tokens,
            "tokens_per_second": output_tokens as f64 / duration_secs,
            "first_response_ms": first_response_ms.filter(|ms| *ms > 0),
        }
    }))
}

pub async fn save_assistant_message(
    app_handle: &AppHandle,
    execution_id: &str,
    conversation_id: &str,
    generation: Option<u64>,
    content: &str,
    tool_calls: Option<&[ToolCallRecord]>,
    reasoning_content: Option<String>,
    metadata: Option<Value>,
    persist_messages: bool,
    subagent_run_id: Option<&str>,
) {
    if !persist_messages {
        if let Some(run_id) = subagent_run_id {
            save_subagent_message(
                app_handle,
                run_id,
                "assistant",
                content,
                tool_calls,
                reasoning_content,
            )
            .await;
        }
        return;
    }
    if content.trim().is_empty() && tool_calls.is_none_or(|tc| tc.is_empty()) {
        return;
    }

    if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        use sentinel_core::models::database as core_db;

        let message_id = uuid::Uuid::new_v4().to_string();
        let tool_calls_json = tool_calls.map(|tc| serde_json::to_string(tc).unwrap_or_default());

        let msg = core_db::AiMessage {
            id: message_id.clone(),
            conversation_id: conversation_id.to_string(),
            role: "assistant".to_string(),
            content: content.to_string(),
            metadata: metadata
                .as_ref()
                .and_then(|value| serde_json::to_string(value).ok()),
            token_count: Some(content.len() as i32),
            cost: None,
            tool_calls: tool_calls_json,
            attachments: None,
            reasoning_content,
            timestamp: chrono::Utc::now(),
            architecture_type: None,
            architecture_meta: None,
            structured_data: None,
        };

        if let Err(e) = db.create_ai_message(&msg).await {
            tracing::warn!("Failed to save assistant message: {}", e);
        } else {
            tracing::info!(
                "Saved assistant message: {} for conversation: {} with {} tool calls",
                message_id,
                conversation_id,
                tool_calls.map_or(0, |tc| tc.len())
            );

            let _ = app_handle.emit(
                "agent:assistant_message_saved",
                &serde_json::json!({
                        "execution_id": execution_id,
                        "conversation_id": conversation_id,
                        "generation": generation,
                    "message_id": message_id,
                    "content": content,
                    "metadata": metadata,
                    "reasoning_content": msg.reasoning_content,
                    "timestamp": msg.timestamp.timestamp_millis(),
                    "tool_calls": tool_calls,
                }),
            );
        }
    }
}

async fn save_subagent_message(
    app_handle: &AppHandle,
    subagent_run_id: &str,
    role: &str,
    content: &str,
    tool_calls: Option<&[ToolCallRecord]>,
    reasoning_content: Option<String>,
) {
    if content.trim().is_empty() && tool_calls.is_none_or(|tc| tc.is_empty()) {
        return;
    }
    if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        use sentinel_core::models::database as core_db;
        let message_id = uuid::Uuid::new_v4().to_string();
        let tool_calls_json = tool_calls.map(|tc| serde_json::to_string(tc).unwrap_or_default());
        let timestamp = chrono::Utc::now();
        let msg = core_db::SubagentMessage {
            id: message_id.clone(),
            subagent_run_id: subagent_run_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            metadata: None,
            tool_calls: tool_calls_json.clone(),
            attachments: None,
            reasoning_content: reasoning_content.clone(),
            timestamp,
            structured_data: None,
        };
        if let Err(e) = db.create_subagent_message_internal(&msg).await {
            tracing::warn!("Failed to save subagent message: {}", e);
        } else {
            // Emit event for real-time update
            let _ = app_handle.emit(
                "subagent:message",
                &serde_json::json!({
                    "subagent_run_id": subagent_run_id,
                    "message_id": message_id,
                    "role": role,
                    "content": content,
                    "tool_calls": tool_calls_json,
                    "reasoning_content": reasoning_content,
                    "timestamp": timestamp.to_rfc3339(),
                }),
            );
        }
    }
}

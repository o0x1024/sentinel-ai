use sentinel_db::Database;
use std::sync::Arc;

use serde_json::json;
use tauri::{AppHandle, Emitter};

fn build_tool_selection_payload(
    execution_id: &str,
    selected_tool_ids: &[String],
) -> serde_json::Value {
    json!({
        "execution_id": execution_id,
        "tools": selected_tool_ids,
    })
}

fn build_tool_activation_payload(
    execution_id: &str,
    requested_tools: &[String],
    activation_query: Option<&str>,
    runtime_hint: Option<&str>,
    current_tool_ids: &[String],
) -> serde_json::Value {
    json!({
        "execution_id": execution_id,
        "tool_ids": requested_tools,
        "query": activation_query,
        "runtime_hint": runtime_hint,
        "tools": current_tool_ids,
    })
}

fn build_tools_preview(current_tool_ids: &[String]) -> String {
    let preview = current_tool_ids
        .iter()
        .take(6)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let suffix = if current_tool_ids.len() > 6 {
        format!(" +{}", current_tool_ids.len() - 6)
    } else {
        String::new()
    };
    format!("{}{}", preview, suffix)
}

fn build_tool_activation_metadata(
    requested_tools: &[String],
    activation_query: Option<&str>,
    runtime_hint: Option<&str>,
    current_tool_ids: &[String],
) -> serde_json::Value {
    json!({
        "kind": "tools_activated",
        "tool_ids": requested_tools,
        "query": activation_query,
        "runtime_hint": runtime_hint,
        "tools": current_tool_ids,
        "tools_preview": build_tools_preview(current_tool_ids),
    })
}

pub(super) fn emit_initial_tool_selection(
    app_handle: &AppHandle,
    execution_id: &str,
    selected_skill: Option<(&str, &str)>,
    selected_tool_ids: &[String],
) {
    if let Some((skill_id, skill_name)) = selected_skill {
        let _ = app_handle.emit(
            "agent:skill_selected",
            &json!({
                "execution_id": execution_id,
                "skill_id": skill_id,
                "skill_name": skill_name,
            }),
        );
    }

    let _ = app_handle.emit(
        "agent:tools_selected",
        &build_tool_selection_payload(execution_id, selected_tool_ids),
    );
}

pub(super) fn emit_and_persist_tool_activation(
    app_handle: &AppHandle,
    execution_id: &str,
    requested_tools: &[String],
    activation_query: Option<String>,
    runtime_hint: Option<String>,
    current_tool_ids: &[String],
    db_for_stream: Option<Arc<sentinel_db::DatabaseService>>,
) {
    let _ = app_handle.emit(
        "agent:tools_selected",
        &build_tool_selection_payload(execution_id, current_tool_ids),
    );
    let _ = app_handle.emit(
        "agent:tools_activated",
        &build_tool_activation_payload(
            execution_id,
            requested_tools,
            activation_query.as_deref(),
            runtime_hint.as_deref(),
            current_tool_ids,
        ),
    );

    if let Some(db) = db_for_stream {
        use sentinel_core::models::database as core_db;

        let meta = build_tool_activation_metadata(
            requested_tools,
            activation_query.as_deref(),
            runtime_hint.as_deref(),
            current_tool_ids,
        );
        let msg = core_db::AiMessage {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: execution_id.to_string(),
            role: "system".to_string(),
            content: "Deferred tools activated".to_string(),
            metadata: Some(meta.to_string()),
            token_count: None,
            cost: None,
            tool_calls: None,
            attachments: None,
            reasoning_content: None,
            timestamp: chrono::Utc::now(),
            architecture_type: None,
            architecture_meta: None,
            structured_data: None,
        };
        tauri::async_runtime::spawn(async move {
            if let Err(e) = db.upsert_ai_message_append(&msg).await {
                tracing::warn!("Failed to persist tools_activated message: {}", e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_tool_activation_metadata, build_tool_activation_payload, build_tools_preview,
    };

    #[test]
    fn tool_activation_payload_includes_runtime_hint() {
        let requested = vec!["file_read".to_string()];
        let current = vec!["tool_search".to_string(), "file_read".to_string()];
        let payload = build_tool_activation_payload(
            "exec-1",
            &requested,
            Some("read the changed file"),
            Some("recent file changes detected; prefer readback"),
            &current,
        );

        assert_eq!(payload["execution_id"], "exec-1");
        assert_eq!(payload["tool_ids"][0], "file_read");
        assert_eq!(
            payload["runtime_hint"],
            "recent file changes detected; prefer readback"
        );
        assert_eq!(payload["tools"][1], "file_read");
    }

    #[test]
    fn tool_activation_metadata_includes_runtime_hint_and_preview() {
        let requested = vec!["file_read".to_string()];
        let current = vec![
            "tool_search".to_string(),
            "file_read".to_string(),
            "grep".to_string(),
        ];
        let metadata = build_tool_activation_metadata(
            &requested,
            Some("read the changed file"),
            Some("recent file changes detected; prefer readback"),
            &current,
        );

        assert_eq!(metadata["kind"], "tools_activated");
        assert_eq!(
            metadata["runtime_hint"],
            "recent file changes detected; prefer readback"
        );
        assert_eq!(metadata["tools_preview"], "tool_search, file_read, grep");
    }

    #[test]
    fn tools_preview_truncates_after_six_tools() {
        let current = vec![
            "tool_search".to_string(),
            "file_read".to_string(),
            "grep".to_string(),
            "glob".to_string(),
            "lsp".to_string(),
            "file_edit".to_string(),
            "file_write".to_string(),
        ];

        assert_eq!(
            build_tools_preview(&current),
            "tool_search, file_read, grep, glob, lsp, file_edit +1"
        );
    }
}

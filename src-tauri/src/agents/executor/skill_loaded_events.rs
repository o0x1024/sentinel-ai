use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use sentinel_db::Database;

fn build_skill_loaded_payload(
    execution_id: &str,
    skill_id: &str,
    skill_name: &str,
) -> serde_json::Value {
    json!({
        "execution_id": execution_id,
        "skill_id": skill_id,
        "skill_name": skill_name,
    })
}

fn build_skill_loaded_metadata(skill_id: &str, skill_name: &str) -> serde_json::Value {
    json!({
        "kind": "skill_loaded",
        "skill_id": skill_id,
        "skill_name": skill_name,
    })
}

fn build_skill_loaded_content(skill_id: &str, skill_name: &str) -> String {
    format!("Skill loaded: {} ({})", skill_name, skill_id)
}

pub(super) fn emit_and_persist_skill_loaded(
    app_handle: &AppHandle,
    execution_id: &str,
    skill_id: &str,
    skill_name: &str,
    db_for_stream: Option<Arc<sentinel_db::DatabaseService>>,
) {
    let _ = app_handle.emit(
        "agent:skill_loaded",
        &build_skill_loaded_payload(execution_id, skill_id, skill_name),
    );

    if let Some(db) = db_for_stream {
        use sentinel_core::models::database as core_db;

        let msg = core_db::AiMessage {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: execution_id.to_string(),
            role: "system".to_string(),
            content: build_skill_loaded_content(skill_id, skill_name),
            metadata: Some(build_skill_loaded_metadata(skill_id, skill_name).to_string()),
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
                tracing::warn!("Failed to persist skill_loaded message: {}", e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_skill_loaded_content, build_skill_loaded_metadata, build_skill_loaded_payload,
    };

    #[test]
    fn skill_loaded_payload_contains_only_skill_identity() {
        let payload = build_skill_loaded_payload("exec-1", "agent-browser", "agent-browser");

        assert_eq!(payload["execution_id"], "exec-1");
        assert_eq!(payload["skill_id"], "agent-browser");
        assert_eq!(payload["skill_name"], "agent-browser");
        assert!(payload.get("tools").is_none());
    }

    #[test]
    fn skill_loaded_metadata_contains_only_skill_identity() {
        let metadata = build_skill_loaded_metadata("agent-browser", "agent-browser");

        assert_eq!(metadata["kind"], "skill_loaded");
        assert_eq!(metadata["skill_id"], "agent-browser");
        assert_eq!(metadata["skill_name"], "agent-browser");
        assert!(metadata.get("tools").is_none());
        assert!(metadata.get("tools_preview").is_none());
    }

    #[test]
    fn skill_loaded_content_uses_skill_identity() {
        assert_eq!(
            build_skill_loaded_content("agent-browser", "agent-browser"),
            "Skill loaded: agent-browser (agent-browser)"
        );
    }
}

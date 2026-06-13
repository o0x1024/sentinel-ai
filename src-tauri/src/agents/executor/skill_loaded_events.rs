use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use sentinel_db::Database;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SkillActivityMode {
    Invoke,
    Fork,
}

impl SkillActivityMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Invoke => "invoke",
            Self::Fork => "fork",
        }
    }
}

fn build_skill_activity_payload(
    execution_id: &str,
    generation: Option<u64>,
    skill_id: &str,
    skill_name: &str,
    mode: SkillActivityMode,
    referenced_files: &[String],
    result_preview: Option<&str>,
) -> serde_json::Value {
    json!({
        "execution_id": execution_id,
        "generation": generation,
        "skill_id": skill_id,
        "skill_name": skill_name,
        "mode": mode.as_str(),
        "referenced_files": referenced_files,
        "result_preview": result_preview,
    })
}

fn build_skill_activity_metadata(
    kind: &str,
    generation: Option<u64>,
    skill_id: &str,
    skill_name: &str,
    mode: SkillActivityMode,
    referenced_files: &[String],
    result_preview: Option<&str>,
) -> serde_json::Value {
    json!({
        "kind": kind,
        "generation": generation,
        "skill_id": skill_id,
        "skill_name": skill_name,
        "mode": mode.as_str(),
        "referenced_files": referenced_files,
        "result_preview": result_preview,
    })
}

fn build_skill_loaded_content(skill_name: &str, skill_id: &str, referenced_files: &[String]) -> String {
    if referenced_files.is_empty() {
        return format!("Skill loaded: {skill_name} ({skill_id})");
    }
    format!(
        "Skill loaded: {skill_name} ({skill_id}) with {} helper file(s)",
        referenced_files.len()
    )
}

fn build_skill_forked_content(skill_name: &str, skill_id: &str) -> String {
    format!("Skill forked: {skill_name} ({skill_id})")
}

fn emit_and_persist_skill_activity(
    app_handle: &AppHandle,
    event_name: &str,
    metadata_kind: &str,
    execution_id: &str,
    conversation_id: &str,
    generation: Option<u64>,
    skill_id: &str,
    skill_name: &str,
    mode: SkillActivityMode,
    referenced_files: &[String],
    result_preview: Option<&str>,
    content: &str,
    db_for_stream: Option<Arc<sentinel_db::DatabaseService>>,
) {
    let payload = build_skill_activity_payload(
        execution_id,
        generation,
        skill_id,
        skill_name,
        mode,
        referenced_files,
        result_preview,
    );
    let _ = app_handle.emit(event_name, &payload);

    if let Some(db) = db_for_stream {
        use sentinel_core::models::database as core_db;

        let msg = core_db::AiMessage {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            role: "system".to_string(),
            content: content.to_string(),
            metadata: Some(
                build_skill_activity_metadata(
                    metadata_kind,
                    generation,
                    skill_id,
                    skill_name,
                    mode,
                    referenced_files,
                    result_preview,
                )
                .to_string(),
            ),
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
        let persist_kind = metadata_kind.to_string();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = db.upsert_ai_message_append(&msg).await {
                tracing::warn!("Failed to persist {persist_kind} message: {e}");
            }
        });
    }
}

pub(super) fn emit_and_persist_skill_loaded(
    app_handle: &AppHandle,
    execution_id: &str,
    conversation_id: &str,
    generation: Option<u64>,
    skill_id: &str,
    skill_name: &str,
    referenced_files: &[String],
    skill_content: Option<&str>,
    db_for_stream: Option<Arc<sentinel_db::DatabaseService>>,
) {
    emit_and_persist_skill_activity(
        app_handle,
        "agent:skill_loaded",
        "skill_loaded",
        execution_id,
        conversation_id,
        generation,
        skill_id,
        skill_name,
        SkillActivityMode::Invoke,
        referenced_files,
        skill_content,
        &build_skill_loaded_content(skill_name, skill_id, referenced_files),
        db_for_stream,
    );
}

pub(super) fn emit_and_persist_skill_forked(
    app_handle: &AppHandle,
    execution_id: &str,
    conversation_id: &str,
    generation: Option<u64>,
    skill_id: &str,
    skill_name: &str,
    referenced_files: &[String],
    result_preview: &str,
    db_for_stream: Option<Arc<sentinel_db::DatabaseService>>,
) {
    emit_and_persist_skill_activity(
        app_handle,
        "agent:skill_forked",
        "skill_forked",
        execution_id,
        conversation_id,
        generation,
        skill_id,
        skill_name,
        SkillActivityMode::Fork,
        referenced_files,
        Some(result_preview),
        &build_skill_forked_content(skill_name, skill_id),
        db_for_stream,
    );
}

#[cfg(test)]
mod tests {
    use super::{
        build_skill_activity_metadata, build_skill_activity_payload, build_skill_forked_content,
        build_skill_loaded_content, SkillActivityMode,
    };

    #[test]
    fn skill_loaded_payload_contains_mode_and_referenced_files() {
        let payload = build_skill_activity_payload(
            "exec-1",
            Some(7),
            "agent-browser",
            "agent-browser",
            SkillActivityMode::Invoke,
            &["references/guide.md".to_string()],
            None,
        );

        assert_eq!(payload["execution_id"], "exec-1");
        assert_eq!(payload["mode"], "invoke");
        assert_eq!(payload["referenced_files"][0], "references/guide.md");
    }

    #[test]
    fn skill_forked_metadata_contains_result_preview() {
        let metadata = build_skill_activity_metadata(
            "skill_forked",
            Some(3),
            "review",
            "Code Review",
            SkillActivityMode::Fork,
            &[],
            Some("Review complete."),
        );

        assert_eq!(metadata["kind"], "skill_forked");
        assert_eq!(metadata["mode"], "fork");
        assert_eq!(metadata["result_preview"], "Review complete.");
    }

    #[test]
    fn skill_loaded_content_mentions_helper_file_count() {
        assert_eq!(
            build_skill_loaded_content("audit", "code-audit", &["a.md".to_string()]),
            "Skill loaded: audit (code-audit) with 1 helper file(s)"
        );
        assert_eq!(
            build_skill_forked_content("audit", "code-audit"),
            "Skill forked: audit (code-audit)"
        );
    }
}

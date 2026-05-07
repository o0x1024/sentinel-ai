use anyhow::Result;
use serde_json::json;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

use sentinel_db::Database;
use sentinel_db::DatabaseService;
use sentinel_llm::{normalize_tool_call_arguments_str, ChatMessage};
use sentinel_tools::buildin_tools::{
    AskUserQuestionTool, FileEditTool, FileReadTool, FileWriteTool, GlobTool, GrepTool,
    HttpRequestTool, LspTool, ShellTool, SkillsTool, TasksTool, ToolSearchTool,
};
use sentinel_tools::dynamic_tool::{
    DynamicTool, DynamicToolDef, ToolCategory, ToolExecutionPolicy, ToolExecutor, ToolSource,
};
use sentinel_tools::terminal::server::TerminalServer;
use sentinel_tools::terminal::{EXEC_COMMAND_TOOL_NAME, WRITE_STDIN_TOOL_NAME};
use sentinel_tools::ToolServer;

use crate::agents::executor::file_tool_state::{
    ensure_file_snapshot_is_editable, record_file_read_snapshot, record_file_revision_snapshot,
};
use crate::agents::executor::http_request_override::build_http_override_def;
use crate::agents::executor::question_override::build_ask_user_question_override_def;
use crate::agents::executor::shell_override::build_shell_override_def;
use crate::agents::executor::terminal_override::{
    build_exec_command_override_def, build_interactive_shell_override_def,
    build_write_stdin_override_def,
};
use crate::agents::executor::tool_search_override::build_tool_search_override_def;
use crate::agents::executor::traffic_response_read_tool::build_traffic_response_read_tool;
use crate::agents::executor::types::ToolCallRecord;

type PendingToolCalls = std::collections::HashMap<String, (String, String, i64, u32)>;

async fn is_skills_enabled_in_db(db: &DatabaseService) -> bool {
    match db.get_config("agent", "skills_enabled").await {
        Ok(Some(val)) => {
            let v = val.trim().to_lowercase();
            matches!(v.as_str(), "true" | "1" | "yes" | "on")
        }
        _ => true,
    }
}

async fn is_skill_enabled_in_db(db: &DatabaseService, skill_id: &str) -> bool {
    let key = format!("enabled::{}", skill_id);
    match db.get_config("skills", &key).await {
        Ok(Some(val)) => {
            let v = val.trim().to_lowercase();
            matches!(v.as_str(), "true" | "1" | "yes" | "on")
        }
        _ => true,
    }
}

pub(super) async fn register_skills_tool_guard(
    tool_server: &ToolServer,
    db: Arc<DatabaseService>,
) -> Result<()> {
    let Some(info) = tool_server.get_tool(SkillsTool::NAME).await else {
        return Ok(());
    };

    let input_schema = info.input_schema.clone();
    let description = info.description.clone();

    tool_server.unregister_tool(SkillsTool::NAME).await;

    let executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let db = db.clone();
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::skills::{SkillsAction, SkillsTool, SkillsToolArgs};

            let tool_args: SkillsToolArgs =
                serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

            if !is_skills_enabled_in_db(&db).await {
                return Err("Skills tool is disabled".to_string());
            }

            let skill_id = tool_args.skill_id.as_deref();
            let requires_skill = matches!(
                tool_args.action,
                SkillsAction::Load | SkillsAction::ReadFile
            );
            if requires_skill {
                if let Some(id) = skill_id {
                    if !is_skill_enabled_in_db(&db, id).await {
                        return Err(format!("Skill '{}' is disabled", id));
                    }
                }
            }

            let tool = SkillsTool;
            let mut result = tool
                .call(tool_args)
                .await
                .map_err(|e| format!("Skills operation failed: {}", e))?;

            if matches!(result.action.as_str(), "list") {
                if let Some(skills) = result.skills.take() {
                    let mut filtered = Vec::new();
                    for skill in skills {
                        if is_skill_enabled_in_db(&db, &skill.id).await {
                            filtered.push(skill);
                        }
                    }
                    result.skills = Some(filtered);
                }
            }

            serde_json::to_value(result).map_err(|e| format!("Failed to serialize result: {}", e))
        })
    });

    let def = DynamicToolDef {
        name: SkillsTool::NAME.to_string(),
        description,
        input_schema,
        output_schema: None,
        source: ToolSource::Builtin,
        category: ToolCategory::System,
        tags: info.tags.clone(),
        search_hint: info.search_hint.clone(),
        exposure: info.exposure.clone(),
        execution_policy: ToolExecutionPolicy::default(),
        executor,
    };

    tool_server.register_tool(def).await;
    Ok(())
}

pub(super) fn apply_allowed_tools_policy(
    mut tool_ids: Vec<String>,
    allowed_tools: &[String],
) -> Vec<String> {
    if allowed_tools.is_empty() {
        return tool_ids;
    }
    let allowed = allowed_tools
        .iter()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
        .map(|id| id.to_string())
        .collect::<std::collections::HashSet<_>>();
    tool_ids.retain(|id| allowed.contains(id));
    tool_ids
}

pub(super) fn infer_tool_result_success(raw: &str) -> bool {
    fn is_structured_http_response(map: &serde_json::Map<String, serde_json::Value>) -> bool {
        map.get("status_code").and_then(|v| v.as_u64()).is_some()
            && map.get("headers").and_then(|v| v.as_object()).is_some()
            && (map.get("url").and_then(|v| v.as_str()).is_some()
                || map.get("status_text").and_then(|v| v.as_str()).is_some())
    }

    fn has_hard_error(text: &str) -> bool {
        let lower = text.trim().to_lowercase();
        if lower.is_empty() {
            return false;
        }
        if lower.contains("toolset error")
            || lower.contains("tool execution failed")
            || lower.contains("shell execution failed")
            || lower.contains("command timeout after")
            || lower.contains("llm request timeout")
            || lower.contains("traceback (most recent call last)")
            || lower.contains("fatal error:")
        {
            return true;
        }
        (lower.contains("timed out") || lower.contains("timeout after"))
            && (lower.contains("error") || lower.contains("failed"))
    }

    fn visit(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Null => true,
            serde_json::Value::Bool(v) => *v,
            serde_json::Value::Number(n) => n.as_i64().map(|v| v == 0).unwrap_or(true),
            serde_json::Value::String(s) => {
                let lower = s.trim().to_lowercase();
                if has_hard_error(&lower) {
                    return false;
                }
                if lower.starts_with("error:") || lower.starts_with("failed:") {
                    return false;
                }
                !lower.contains(" no such file or directory")
            }
            serde_json::Value::Array(arr) => arr.iter().all(visit),
            serde_json::Value::Object(map) => {
                // For http_request, any structured HTTP response means the tool call itself succeeded.
                // Application-layer status (4xx/5xx) and response payload fields must not be treated as
                // tool-execution failure.
                if is_structured_http_response(map) {
                    return true;
                }
                if let Some(v) = map.get("success").and_then(|v| v.as_bool()) {
                    return v;
                }
                if let Some(v) = map.get("ok").and_then(|v| v.as_bool()) {
                    return v;
                }
                if let Some(v) = map.get("completed").and_then(|v| v.as_bool()) {
                    if !v {
                        return false;
                    }
                }
                if let Some(v) = map.get("exit_code").and_then(|v| v.as_i64()) {
                    return v == 0;
                }
                if let Some(v) = map.get("code").and_then(|v| v.as_i64()) {
                    return v == 0;
                }
                if let Some(v) = map.get("error").and_then(|v| v.as_str()) {
                    if !v.trim().is_empty() {
                        return false;
                    }
                }
                map.values().all(visit)
            }
        }
    }

    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(v) => visit(&v),
        Err(_) => {
            let lower = raw.trim().to_lowercase();
            if has_hard_error(&lower) {
                return false;
            }
            if lower.starts_with("error:") || lower.starts_with("failed:") {
                return false;
            }
            !lower.contains(" no such file or directory")
        }
    }
}

fn shorten_for_fingerprint(raw: &str, max_chars: usize) -> String {
    if raw.chars().count() <= max_chars {
        return raw.trim().to_string();
    }
    raw.chars()
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_string()
}

pub(super) fn tool_loop_fingerprint(tool_name: &str, arguments: &str, result: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    tool_name.hash(&mut hasher);
    shorten_for_fingerprint(arguments, 320).hash(&mut hasher);
    shorten_for_fingerprint(result, 320).hash(&mut hasher);
    hasher.finish()
}

fn lower_tool_argument(arguments: &str, keys: &[&str]) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(arguments).ok()?;
    for key in keys {
        if let Some(text) = value.get(*key).and_then(|v| v.as_str()) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_lowercase());
            }
        }
    }
    None
}

fn is_shell_like_side_effect_command(command: &str) -> bool {
    let write_markers = [
        "rm ",
        "mv ",
        "cp ",
        "chmod ",
        "chown ",
        "touch ",
        "mkdir ",
        "sed -i",
        "tee ",
        "git apply",
        "git add",
        "git commit",
        "git push",
        "cargo fmt",
        "cargo fix",
        "npm install",
        "pnpm install",
        "yarn install",
        "pip install",
        "terraform apply",
        "kubectl apply",
        "kubectl delete",
        "docker run",
        "docker exec",
        "docker compose up",
    ];

    write_markers.iter().any(|marker| command.contains(marker))
        || command.contains(" >")
        || command.contains(">>")
}

fn is_shell_like_high_risk_command(command: &str) -> bool {
    let high_risk_markers = [
        "rm -rf",
        "mkfs",
        "dd ",
        "chmod ",
        "chown ",
        "git push",
        "git commit",
        "terraform apply",
        "terraform destroy",
        "kubectl apply",
        "kubectl delete",
        "docker run",
        "docker exec",
        "docker compose up",
        "npm publish",
        "cargo publish",
        "psql ",
        "mysql ",
        "sqlite3 ",
        "sed -i",
        "tee ",
    ];

    high_risk_markers
        .iter()
        .any(|marker| command.contains(marker))
        || command.contains(" >")
        || command.contains(">>")
}

fn http_method(arguments: &str) -> Option<String> {
    lower_tool_argument(arguments, &["method"]).map(|m| m.to_ascii_uppercase())
}

pub(super) fn is_high_risk_tool_call(tool_name: &str, arguments: &str) -> bool {
    match tool_name {
        "shell" | "interactive_shell" => {
            lower_tool_argument(arguments, &["command", "initial_command"])
                .map(|cmd| is_shell_like_high_risk_command(&cmd))
                .unwrap_or(false)
        }
        "http_request" => matches!(
            http_method(arguments).as_deref(),
            Some("POST" | "PUT" | "PATCH" | "DELETE")
        ),
        "file_edit" | "file_write" => true,
        "sops" => true,
        _ => false,
    }
}

pub(super) fn is_side_effectful_tool_call(tool_name: &str, arguments: &str) -> bool {
    match tool_name {
        "shell" | "interactive_shell" => {
            lower_tool_argument(arguments, &["command", "initial_command"])
                .map(|cmd| is_shell_like_side_effect_command(&cmd))
                .unwrap_or(false)
        }
        "http_request" => matches!(
            http_method(arguments).as_deref(),
            Some("POST" | "PUT" | "PATCH" | "DELETE")
        ),
        "file_edit" | "file_write" => true,
        "sops" => true,
        _ => false,
    }
}

pub(super) fn trailing_failed_tool_calls(records: &[ToolCallRecord]) -> usize {
    records
        .iter()
        .rev()
        .take_while(|record| !record.success)
        .count()
}

pub(super) fn looks_like_verification_tool_call(
    tool_name: &str,
    arguments: &str,
    success: bool,
) -> bool {
    if !success {
        return false;
    }

    match tool_name {
        "shell" | "interactive_shell" => {
            lower_tool_argument(arguments, &["command", "initial_command"])
                .map(|command| {
                    let markers = [
                        "cargo test",
                        "cargo check",
                        "cargo clippy",
                        "go test",
                        "pytest",
                        "npm test",
                        "pnpm test",
                        "yarn test",
                        "vitest",
                        "jest",
                        "ruff",
                        "mypy",
                        "eslint",
                        "npm run lint",
                        "pnpm lint",
                        "git diff",
                        "git status",
                        "rg ",
                        "grep ",
                        "cat ",
                        "sed -n",
                        "head ",
                        "tail ",
                        "ls ",
                        "find ",
                        "wc ",
                    ];
                    markers.iter().any(|marker| command.contains(marker))
                })
                .unwrap_or(false)
        }
        "http_request" => matches!(http_method(arguments).as_deref(), Some("GET" | "HEAD")),
        "file_read" | "grep" | "glob" => true,
        "browser" => true,
        _ => false,
    }
}

fn final_response_makes_completion_claim(final_response: &str) -> bool {
    let lower = final_response.trim().to_lowercase();
    if lower.is_empty() {
        return false;
    }

    let markers = [
        "已修复",
        "已经修复",
        "已完成",
        "完成了",
        "已经完成",
        "修好了",
        "fixed",
        "resolved",
        "completed",
        "done",
        "implemented",
    ];

    markers.iter().any(|marker| lower.contains(marker))
}

fn final_response_has_high_confidence_claim(final_response: &str) -> bool {
    let lower = final_response.trim().to_lowercase();
    if lower.is_empty() {
        return false;
    }

    let markers = [
        "可以确定",
        "明确",
        "根因就是",
        "一定是",
        "显然",
        "最佳方案",
        "唯一方案",
        "一定",
        "must",
        "definitely",
        "clearly",
        "root cause is",
        "best approach",
        "the answer is",
    ];

    markers.iter().any(|marker| lower.contains(marker))
        || final_response_makes_completion_claim(final_response)
}

fn shell_evidence_score(command: &str) -> u32 {
    let direct_content_markers = [
        "cat ", "sed -n", "head ", "tail ", "git show", "jq ", "read ",
    ];
    let broad_scan_markers = ["rg ", "grep ", "find ", "ls ", "tree ", "git log", "wc "];

    if direct_content_markers
        .iter()
        .any(|marker| command.contains(marker))
    {
        2
    } else if broad_scan_markers
        .iter()
        .any(|marker| command.contains(marker))
    {
        1
    } else {
        0
    }
}

fn is_focus_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':')
}

fn extract_focus_tokens(text: &str) -> HashSet<String> {
    const STOPWORDS: &[&str] = &[
        "therefore",
        "because",
        "should",
        "must",
        "clearly",
        "definitely",
        "best",
        "approach",
        "answer",
        "issue",
        "problem",
        "solution",
        "resolved",
        "completed",
        "implemented",
        "fixed",
        "done",
    ];

    let lowered = text.to_lowercase();
    let mut tokens = HashSet::new();
    let mut current = String::new();

    for ch in lowered.chars() {
        if is_focus_token_char(ch) {
            current.push(ch);
        } else if !current.is_empty() {
            if current.len() >= 3 && !STOPWORDS.contains(&current.as_str()) {
                tokens.insert(current.clone());
            }
            current.clear();
        }
    }

    if current.len() >= 3 && !STOPWORDS.contains(&current.as_str()) {
        tokens.insert(current);
    }

    tokens
}

fn tool_call_relevance_text(record: &ToolCallRecord) -> String {
    let mut text = format!("{} {}", record.name, record.arguments).to_lowercase();
    if let Some(result) = &record.result {
        let preview = result.chars().take(400).collect::<String>().to_lowercase();
        if !preview.is_empty() {
            text.push(' ');
            text.push_str(&preview);
        }
    }
    text
}

pub(super) fn evidence_score_for_tool_call(tool_name: &str, arguments: &str, success: bool) -> u32 {
    if !success {
        return 0;
    }

    if looks_like_verification_tool_call(tool_name, arguments, success) {
        return 3;
    }

    match tool_name {
        "shell" | "interactive_shell" => {
            lower_tool_argument(arguments, &["command", "initial_command"])
                .map(|command| {
                    if is_side_effectful_tool_call(tool_name, arguments) {
                        0
                    } else {
                        shell_evidence_score(&command)
                    }
                })
                .unwrap_or(0)
        }
        "http_request" => match http_method(arguments).as_deref() {
            Some("GET" | "HEAD") => 2,
            _ => 0,
        },
        "file_read" | "grep" => 2,
        "glob" => 1,
        "browser" | "web_search" | "ocr" => 2,
        "memory" | "tool_search" => 1,
        _ => 0,
    }
}

fn evidence_score_for_record_against_focus(
    record: &ToolCallRecord,
    focus_tokens: &HashSet<String>,
) -> u32 {
    let base = evidence_score_for_tool_call(&record.name, &record.arguments, record.success);
    if base == 0 {
        return 0;
    }
    if focus_tokens.is_empty()
        || looks_like_verification_tool_call(&record.name, &record.arguments, record.success)
    {
        return base;
    }

    let relevance_text = tool_call_relevance_text(record);
    if focus_tokens
        .iter()
        .any(|token| relevance_text.contains(token.as_str()))
    {
        base
    } else {
        0
    }
}

#[cfg(test)]
pub(super) fn total_evidence_score(records: &[ToolCallRecord]) -> u32 {
    records
        .iter()
        .map(|record| evidence_score_for_tool_call(&record.name, &record.arguments, record.success))
        .sum()
}

fn relevant_evidence_score(claim_text: &str, records: &[ToolCallRecord]) -> u32 {
    let focus_tokens = extract_focus_tokens(claim_text);
    records
        .iter()
        .map(|record| evidence_score_for_record_against_focus(record, &focus_tokens))
        .sum()
}

pub(super) fn final_response_needs_verification_review(
    final_response: &str,
    records: &[ToolCallRecord],
) -> bool {
    if !final_response_makes_completion_claim(final_response) {
        return false;
    }

    let Some(last_side_effect_seq) = records
        .iter()
        .rev()
        .find(|record| is_side_effectful_tool_call(&record.name, &record.arguments))
        .map(|record| record.sequence)
    else {
        return false;
    };

    !records.iter().any(|record| {
        record.sequence > last_side_effect_seq
            && looks_like_verification_tool_call(&record.name, &record.arguments, record.success)
    })
}

pub(super) fn final_response_needs_evidence_review(
    final_response: &str,
    focus_hint: Option<&str>,
    records: &[ToolCallRecord],
    minimum_evidence_score: u32,
) -> bool {
    if !final_response_has_high_confidence_claim(final_response) {
        return false;
    }

    let claim_text = focus_hint.unwrap_or(final_response);
    relevant_evidence_score(claim_text, records) < minimum_evidence_score.max(1)
}

pub(super) fn streaming_content_needs_evidence_review(
    content: &str,
    focus_hint: Option<&str>,
    records: &[ToolCallRecord],
    minimum_evidence_score: u32,
) -> bool {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return false;
    }

    let has_high_confidence_claim = final_response_has_high_confidence_claim(trimmed);
    let has_conclusion_markers =
        crate::agents::tenth_man::TenthMan::contains_conclusion_markers(trimmed);

    if !has_high_confidence_claim && !(has_conclusion_markers && trimmed.chars().count() >= 80) {
        return false;
    }

    let claim_text = focus_hint.unwrap_or(trimmed);
    relevant_evidence_score(claim_text, records) < minimum_evidence_score.max(1)
}

#[derive(Debug, Clone)]
pub(super) struct TeamStreamContext {
    pub session_id: String,
    pub stream_id: String,
    pub member_id: Option<String>,
    pub phase: String,
}

pub(super) fn parse_team_stream_context(execution_id: &str) -> Option<TeamStreamContext> {
    if !execution_id.starts_with("team-v3:") {
        return None;
    }
    let parts = execution_id.split(':').collect::<Vec<_>>();
    if parts.len() < 4 {
        return None;
    }
    let session_id = parts.get(1)?.trim().to_string();
    if session_id.is_empty() {
        return None;
    }
    let member_id = if parts.len() >= 5 {
        parts
            .get(3)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    } else {
        None
    };
    Some(TeamStreamContext {
        session_id,
        stream_id: execution_id.to_string(),
        member_id,
        phase: "task_execution".to_string(),
    })
}

pub(super) async fn persist_ai_message_with_retry(
    db: Arc<sentinel_db::DatabaseService>,
    msg: sentinel_core::models::database::AiMessage,
    log_label: &str,
) {
    const MAX_RETRIES: usize = 3;
    for attempt in 0..=MAX_RETRIES {
        match db.upsert_ai_message_append(&msg).await {
            Ok(_) => return,
            Err(e) => {
                let err = e.to_string().to_lowercase();
                let locked = err.contains("database is locked") || err.contains("(code: 5)");
                if locked && attempt < MAX_RETRIES {
                    let backoff_ms = 30u64 * (1u64 << attempt);
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                    continue;
                }
                tracing::warn!("Failed to persist {}: {}", log_label, e);
                return;
            }
        }
    }
}

pub(super) async fn ensure_ai_conversation_exists_for_persistence(
    db: &DatabaseService,
    execution_id: &str,
    model: &str,
    provider: &str,
) {
    match db.get_ai_conversation(execution_id).await {
        Ok(Some(_)) => return,
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(
                "Failed to check ai_conversation before persistence (execution_id={}): {}",
                execution_id,
                e
            );
            return;
        }
    }

    use sentinel_core::models::database as core_db;
    let now = chrono::Utc::now();
    let conv = core_db::AiConversation {
        id: execution_id.to_string(),
        title: None,
        service_name: if provider.trim().is_empty() {
            "default".to_string()
        } else {
            provider.to_string()
        },
        model_name: if model.trim().is_empty() {
            "default".to_string()
        } else {
            model.to_string()
        },
        model_provider: Some(provider.to_string()),
        context_type: None,
        project_id: None,
        vulnerability_id: None,
        scan_task_id: None,
        conversation_data: None,
        summary: None,
        total_messages: 0,
        total_tokens: 0,
        cost: 0.0,
        tags: None,
        tool_config: None,
        is_archived: false,
        created_at: now,
        updated_at: now,
    };

    if let Err(e) = db.create_ai_conversation(&conv).await {
        let err = e.to_string().to_lowercase();
        let already_exists = err.contains("unique")
            || err.contains("duplicate")
            || err.contains("already exists")
            || err.contains("constraint failed");
        if !already_exists {
            tracing::warn!(
                "Failed to create ai_conversation for persistence (execution_id={}): {}",
                execution_id,
                e
            );
        }
    }
}

pub(super) fn build_retry_history(
    base_history_messages: &[ChatMessage],
    tool_calls_snapshot: Vec<ToolCallRecord>,
    output_snapshot: String,
    attempt: u32,
    include_accumulated: bool,
) -> Vec<ChatMessage> {
    let mut history = base_history_messages.to_vec();
    if attempt == 0 && !include_accumulated {
        return history;
    }

    let mut unique_calls = std::collections::HashMap::new();
    for call in tool_calls_snapshot {
        unique_calls.entry(call.id.clone()).or_insert(call);
    }
    let mut ordered_calls = unique_calls.into_values().collect::<Vec<_>>();
    ordered_calls.sort_by_key(|c| c.sequence);

    if !ordered_calls.is_empty() {
        let tool_calls_json = serde_json::to_string(
            &ordered_calls
                .iter()
                .map(|c| {
                    let normalized_arguments =
                        normalize_tool_call_arguments_str(&c.name, &c.arguments);
                    json!({
                        "id": c.id,
                        "type": "function",
                        "function": {
                            "name": c.name,
                            "arguments": normalized_arguments,
                        }
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default();

        tracing::info!(
            "Building assistant tool_calls message: tool_calls={}, content_empty=true",
            ordered_calls.len()
        );
        let mut tool_calls_msg = ChatMessage::assistant(".");
        tool_calls_msg.tool_calls = Some(tool_calls_json);
        tool_calls_msg.reasoning_content = Some(String::new());
        history.push(tool_calls_msg);

        for call in &ordered_calls {
            if let Some(result) = &call.result {
                history.push(ChatMessage::tool(result.clone(), call.id.clone()));
            }
        }
    }

    if !output_snapshot.trim().is_empty() {
        history.push(ChatMessage::assistant(output_snapshot));
    }

    history
}

pub(super) fn accumulate_retry_progress(
    tool_calls_collector: &Arc<Mutex<Vec<ToolCallRecord>>>,
    accumulated_tool_calls: &Arc<Mutex<Vec<ToolCallRecord>>>,
    assistant_segment_buf: &Arc<Mutex<String>>,
    accumulated_assistant_output: &Arc<Mutex<String>>,
) {
    if let Ok(current_calls) = tool_calls_collector.lock() {
        if let Ok(mut acc) = accumulated_tool_calls.lock() {
            acc.extend(current_calls.clone());
        }
    }

    if let Ok(current_output) = assistant_segment_buf.lock() {
        if !current_output.is_empty() {
            if let Ok(mut acc) = accumulated_assistant_output.lock() {
                if !acc.is_empty() {
                    acc.push_str("\n\n");
                }
                acc.push_str(current_output.as_str());
            }
        }
    }
}

pub(super) fn clear_retry_turn_state(
    assistant_segment_buf: &Arc<Mutex<String>>,
    reasoning_content_buf: &Arc<Mutex<String>>,
    pending_calls: &Arc<Mutex<PendingToolCalls>>,
    tool_calls_collector: &Arc<Mutex<Vec<ToolCallRecord>>>,
    last_tool_fingerprint: Option<&Arc<Mutex<Option<u64>>>>,
    repeated_tool_fingerprint_count: Option<&AtomicUsize>,
) {
    if let Ok(mut buf) = assistant_segment_buf.lock() {
        buf.clear();
    }
    if let Ok(mut buf) = reasoning_content_buf.lock() {
        buf.clear();
    }
    if let Ok(mut pending) = pending_calls.lock() {
        pending.clear();
    }
    if let Ok(mut tool_calls) = tool_calls_collector.lock() {
        tool_calls.clear();
    }
    if let Some(fp) = last_tool_fingerprint {
        if let Ok(mut slot) = fp.lock() {
            *slot = None;
        }
    }
    if let Some(counter) = repeated_tool_fingerprint_count {
        counter.store(0, Ordering::SeqCst);
    }
}

pub(super) fn finalize_response_state(
    response: &str,
    accumulated_assistant_output: &Arc<Mutex<String>>,
    persisted_segment_count: &AtomicUsize,
) -> (String, String, usize) {
    let full_response = if let Ok(acc) = accumulated_assistant_output.lock() {
        if !acc.is_empty() && !response.is_empty() {
            format!("{}\n\n{}", acc, response)
        } else if !acc.is_empty() {
            acc.clone()
        } else {
            response.to_string()
        }
    } else {
        response.to_string()
    };

    let seg_count = persisted_segment_count.load(Ordering::SeqCst);
    let final_response = if seg_count > 0 && !response.is_empty() {
        tracing::info!(
            "Segments already persisted: {}, saving only last turn response ({} chars) instead of full ({} chars)",
            seg_count,
            response.len(),
            full_response.len()
        );
        response.to_string()
    } else {
        full_response.clone()
    };

    (full_response, final_response, seg_count)
}

pub(super) fn collect_all_tool_calls(
    accumulated_tool_calls: &Arc<Mutex<Vec<ToolCallRecord>>>,
    tool_calls_collector: &Arc<Mutex<Vec<ToolCallRecord>>>,
) -> Vec<ToolCallRecord> {
    let mut all_tool_calls = Vec::new();
    if let Ok(acc_calls) = accumulated_tool_calls.lock() {
        all_tool_calls.extend(acc_calls.clone());
    }
    if let Ok(current_calls) = tool_calls_collector.lock() {
        all_tool_calls.extend(current_calls.clone());
    }
    all_tool_calls
}

fn replace_dynamic_tool(dynamic_tools: Vec<DynamicTool>, def: DynamicToolDef) -> Vec<DynamicTool> {
    let mut replaced = false;
    let mut tools: Vec<DynamicTool> = dynamic_tools
        .into_iter()
        .map(|tool| {
            if tool.name() == def.name {
                replaced = true;
                DynamicTool::new(def.clone())
            } else {
                tool
            }
        })
        .collect();
    if !replaced {
        tools.push(DynamicTool::new(def));
    }
    tools
}

async fn build_glob_override_def(
    tool_server: &ToolServer,
    active_terminal_session_id: Option<&str>,
    host_working_directory: Option<&str>,
) -> Option<DynamicToolDef> {
    let info = tool_server.get_tool(GlobTool::NAME).await?;
    let active_terminal_session_id = active_terminal_session_id.map(str::to_string);
    let host_working_directory = host_working_directory.map(str::to_string);
    let input_schema = info.input_schema.clone();
    let description = info.description.clone();
    let execution_policy = info.execution_policy.clone();
    let executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let active_terminal_session_id = active_terminal_session_id.clone();
        let host_working_directory = host_working_directory.clone();
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::file_runtime::{
                build_file_runtime_context, with_file_runtime_context,
            };
            use sentinel_tools::buildin_tools::glob::{GlobArgs, GlobTool};

            let tool_args: GlobArgs =
                serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;
            let runtime_context = build_file_runtime_context(
                active_terminal_session_id.as_deref(),
                host_working_directory.as_deref(),
            )
            .await;
            let result = with_file_runtime_context(runtime_context, GlobTool.call(tool_args))
                .await
                .map_err(|e| format!("Glob failed: {}", e))?;

            serde_json::to_value(result)
                .map_err(|e| format!("Failed to serialize glob result: {}", e))
        })
    });

    Some(DynamicToolDef {
        name: GlobTool::NAME.to_string(),
        description,
        input_schema,
        output_schema: info.output_schema.clone(),
        source: ToolSource::Builtin,
        category: ToolCategory::Utility,
        tags: info.tags.clone(),
        search_hint: info.search_hint.clone(),
        exposure: info.exposure.clone(),
        execution_policy,
        executor,
    })
}

async fn build_grep_override_def(
    tool_server: &ToolServer,
    active_terminal_session_id: Option<&str>,
    host_working_directory: Option<&str>,
) -> Option<DynamicToolDef> {
    let info = tool_server.get_tool(GrepTool::NAME).await?;
    let active_terminal_session_id = active_terminal_session_id.map(str::to_string);
    let host_working_directory = host_working_directory.map(str::to_string);
    let input_schema = info.input_schema.clone();
    let description = info.description.clone();
    let execution_policy = info.execution_policy.clone();
    let executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let active_terminal_session_id = active_terminal_session_id.clone();
        let host_working_directory = host_working_directory.clone();
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::file_runtime::{
                build_file_runtime_context, with_file_runtime_context,
            };
            use sentinel_tools::buildin_tools::grep::{GrepArgs, GrepTool};

            let tool_args: GrepArgs =
                serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;
            let runtime_context = build_file_runtime_context(
                active_terminal_session_id.as_deref(),
                host_working_directory.as_deref(),
            )
            .await;
            let result = with_file_runtime_context(runtime_context, GrepTool.call(tool_args))
                .await
                .map_err(|e| format!("Grep failed: {}", e))?;

            serde_json::to_value(result)
                .map_err(|e| format!("Failed to serialize grep result: {}", e))
        })
    });

    Some(DynamicToolDef {
        name: GrepTool::NAME.to_string(),
        description,
        input_schema,
        output_schema: info.output_schema.clone(),
        source: ToolSource::Builtin,
        category: ToolCategory::Utility,
        tags: info.tags.clone(),
        search_hint: info.search_hint.clone(),
        exposure: info.exposure.clone(),
        execution_policy,
        executor,
    })
}

async fn build_lsp_override_def(
    tool_server: &ToolServer,
    active_terminal_session_id: Option<&str>,
    host_working_directory: Option<&str>,
) -> Option<DynamicToolDef> {
    let info = tool_server.get_tool(LspTool::NAME).await?;
    let active_terminal_session_id = active_terminal_session_id.map(str::to_string);
    let host_working_directory = host_working_directory.map(str::to_string);
    let input_schema = info.input_schema.clone();
    let description = info.description.clone();
    let execution_policy = info.execution_policy.clone();
    let executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let active_terminal_session_id = active_terminal_session_id.clone();
        let host_working_directory = host_working_directory.clone();
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::file_runtime::{
                build_file_runtime_context, with_file_runtime_context,
            };
            use sentinel_tools::buildin_tools::lsp::{LspArgs, LspTool};

            let tool_args: LspArgs =
                serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;
            let runtime_context = build_file_runtime_context(
                active_terminal_session_id.as_deref(),
                host_working_directory.as_deref(),
            )
            .await;
            let result = with_file_runtime_context(runtime_context, LspTool.call(tool_args))
                .await
                .map_err(|e| format!("LSP navigation failed: {}", e))?;

            serde_json::to_value(result)
                .map_err(|e| format!("Failed to serialize lsp result: {}", e))
        })
    });

    Some(DynamicToolDef {
        name: LspTool::NAME.to_string(),
        description,
        input_schema,
        output_schema: info.output_schema.clone(),
        source: ToolSource::Builtin,
        category: ToolCategory::Utility,
        tags: info.tags.clone(),
        search_hint: info.search_hint.clone(),
        exposure: info.exposure.clone(),
        execution_policy,
        executor,
    })
}

async fn build_file_read_override_def(
    tool_server: &ToolServer,
    execution_id: &str,
    active_terminal_session_id: Option<&str>,
    host_working_directory: Option<&str>,
) -> Option<DynamicToolDef> {
    let info = tool_server.get_tool(FileReadTool::NAME).await?;
    let execution_id_for_file_read = execution_id.to_string();
    let active_terminal_session_id = active_terminal_session_id.map(str::to_string);
    let host_working_directory = host_working_directory.map(str::to_string);
    let input_schema = info.input_schema.clone();
    let description = info.description.clone();
    let execution_policy = info.execution_policy.clone();
    let executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let execution_id_for_file_read = execution_id_for_file_read.clone();
        let active_terminal_session_id = active_terminal_session_id.clone();
        let host_working_directory = host_working_directory.clone();
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::file_read::{FileReadArgs, FileReadTool};
            use sentinel_tools::buildin_tools::file_runtime::{
                build_file_runtime_context, with_file_runtime_context,
            };

            let tool_args: FileReadArgs =
                serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;
            let runtime_context = build_file_runtime_context(
                active_terminal_session_id.as_deref(),
                host_working_directory.as_deref(),
            )
            .await;
            let result = with_file_runtime_context(
                runtime_context.clone(),
                FileReadTool.call(tool_args.clone()),
            )
            .await
            .map_err(|e| format!("File read failed: {}", e))?;

            with_file_runtime_context(
                runtime_context,
                record_file_read_snapshot(
                    &execution_id_for_file_read,
                    &tool_args.file_path,
                    &result.content_hash,
                    result.start_line,
                    result.end_line,
                    result.total_lines,
                    result.truncated,
                ),
            )
            .await
            .map_err(|e| format!("Failed to record file read state: {}", e))?;

            serde_json::to_value(result)
                .map_err(|e| format!("Failed to serialize file_read result: {}", e))
        })
    });

    Some(DynamicToolDef {
        name: FileReadTool::NAME.to_string(),
        description,
        input_schema,
        output_schema: info.output_schema.clone(),
        source: ToolSource::Builtin,
        category: ToolCategory::Utility,
        tags: info.tags.clone(),
        search_hint: info.search_hint.clone(),
        exposure: info.exposure.clone(),
        execution_policy,
        executor,
    })
}

async fn build_file_edit_override_def(
    tool_server: &ToolServer,
    execution_id: &str,
    active_terminal_session_id: Option<&str>,
    host_working_directory: Option<&str>,
) -> Option<DynamicToolDef> {
    let info = tool_server.get_tool(FileEditTool::NAME).await?;
    let execution_id_for_file_edit = execution_id.to_string();
    let active_terminal_session_id = active_terminal_session_id.map(str::to_string);
    let host_working_directory = host_working_directory.map(str::to_string);
    let input_schema = info.input_schema.clone();
    let description = info.description.clone();
    let execution_policy = info.execution_policy.clone();
    let executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let execution_id_for_file_edit = execution_id_for_file_edit.clone();
        let active_terminal_session_id = active_terminal_session_id.clone();
        let host_working_directory = host_working_directory.clone();
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::file_edit::{FileEditArgs, FileEditTool};
            use sentinel_tools::buildin_tools::file_runtime::{
                build_file_runtime_context, with_file_runtime_context,
            };

            let tool_args: FileEditArgs =
                serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;
            let runtime_context = build_file_runtime_context(
                active_terminal_session_id.as_deref(),
                host_working_directory.as_deref(),
            )
            .await;
            with_file_runtime_context(
                runtime_context.clone(),
                ensure_file_snapshot_is_editable(&execution_id_for_file_edit, &tool_args.file_path),
            )
            .await
            .map_err(|e| e.to_string())?;

            let result = with_file_runtime_context(
                runtime_context.clone(),
                FileEditTool.call(tool_args.clone()),
            )
            .await
            .map_err(|e| format!("File edit failed: {}", e))?;
            with_file_runtime_context(
                runtime_context,
                record_file_revision_snapshot(
                    &execution_id_for_file_edit,
                    &tool_args.file_path,
                    &result.content_hash,
                ),
            )
            .await
            .map_err(|e| format!("Failed to update file edit state: {}", e))?;

            serde_json::to_value(result)
                .map_err(|e| format!("Failed to serialize file_edit result: {}", e))
        })
    });

    Some(DynamicToolDef {
        name: FileEditTool::NAME.to_string(),
        description,
        input_schema,
        output_schema: info.output_schema.clone(),
        source: ToolSource::Builtin,
        category: ToolCategory::Utility,
        tags: info.tags.clone(),
        search_hint: info.search_hint.clone(),
        exposure: info.exposure.clone(),
        execution_policy,
        executor,
    })
}

async fn build_file_write_override_def(
    tool_server: &ToolServer,
    execution_id: &str,
    active_terminal_session_id: Option<&str>,
    host_working_directory: Option<&str>,
) -> Option<DynamicToolDef> {
    let info = tool_server.get_tool(FileWriteTool::NAME).await?;
    let execution_id_for_file_write = execution_id.to_string();
    let active_terminal_session_id = active_terminal_session_id.map(str::to_string);
    let host_working_directory = host_working_directory.map(str::to_string);
    let input_schema = info.input_schema.clone();
    let description = info.description.clone();
    let execution_policy = info.execution_policy.clone();
    let executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let execution_id_for_file_write = execution_id_for_file_write.clone();
        let active_terminal_session_id = active_terminal_session_id.clone();
        let host_working_directory = host_working_directory.clone();
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::file_runtime::{
                build_file_runtime_context, path_exists, with_file_runtime_context,
            };
            use sentinel_tools::buildin_tools::file_write::{FileWriteArgs, FileWriteTool};

            let tool_args: FileWriteArgs =
                serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;
            let runtime_context = build_file_runtime_context(
                active_terminal_session_id.as_deref(),
                host_working_directory.as_deref(),
            )
            .await;
            let exists = with_file_runtime_context(
                runtime_context.clone(),
                path_exists(&tool_args.file_path),
            )
            .await
            .map_err(|e| e.to_string())?;

            if exists && tool_args.overwrite {
                with_file_runtime_context(
                    runtime_context.clone(),
                    ensure_file_snapshot_is_editable(
                        &execution_id_for_file_write,
                        &tool_args.file_path,
                    ),
                )
                .await
                .map_err(|e| e.to_string())?;
            }

            let result = with_file_runtime_context(
                runtime_context.clone(),
                FileWriteTool.call(tool_args.clone()),
            )
            .await
            .map_err(|e| format!("File write failed: {}", e))?;
            with_file_runtime_context(
                runtime_context,
                record_file_revision_snapshot(
                    &execution_id_for_file_write,
                    &tool_args.file_path,
                    &result.content_hash,
                ),
            )
            .await
            .map_err(|e| format!("Failed to update file write state: {}", e))?;

            serde_json::to_value(result)
                .map_err(|e| format!("Failed to serialize file_write result: {}", e))
        })
    });

    Some(DynamicToolDef {
        name: FileWriteTool::NAME.to_string(),
        description,
        input_schema,
        output_schema: info.output_schema.clone(),
        source: ToolSource::Builtin,
        category: ToolCategory::Utility,
        tags: info.tags.clone(),
        search_hint: info.search_hint.clone(),
        exposure: info.exposure.clone(),
        execution_policy,
        executor,
    })
}

async fn build_tasks_override_def(
    tool_server: &ToolServer,
    execution_id: &str,
) -> Option<DynamicToolDef> {
    let tasks_info = tool_server.get_tool(TasksTool::NAME).await?;
    let execution_id_for_tasks = execution_id.to_string();
    let mut tasks_input_schema = tasks_info.input_schema.clone();
    if let Some(obj) = tasks_input_schema.as_object_mut() {
        if let Some(properties) = obj
            .get_mut("properties")
            .and_then(|value| value.as_object_mut())
        {
            properties.remove("execution_id");
        }
        if let Some(required) = obj
            .get_mut("required")
            .and_then(|value| value.as_array_mut())
        {
            required.retain(|value| value.as_str() != Some("execution_id"));
        }
    }
    let tasks_description = tasks_info.description.clone();
    let tasks_executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let execution_id_for_tasks = execution_id_for_tasks.clone();
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::tasks::{TasksArgs, TasksTool};

            let mut patched_args = args;
            if let Some(obj) = patched_args.as_object_mut() {
                obj.insert(
                    "execution_id".to_string(),
                    serde_json::Value::String(execution_id_for_tasks.clone()),
                );
            }

            let tool_args: TasksArgs = serde_json::from_value(patched_args)
                .map_err(|e| format!("Invalid arguments: {}", e))?;

            let tool = TasksTool::new();
            let result = tool
                .call(tool_args)
                .await
                .map_err(|e| format!("Tasks operation failed: {}", e))?;

            serde_json::to_value(result)
                .map_err(|e| format!("Failed to serialize tasks result: {}", e))
        })
    });

    Some(DynamicToolDef {
        name: TasksTool::NAME.to_string(),
        description: tasks_description,
        input_schema: tasks_input_schema,
        output_schema: None,
        source: ToolSource::Builtin,
        category: ToolCategory::System,
        tags: tasks_info.tags.clone(),
        search_hint: tasks_info.search_hint.clone(),
        exposure: tasks_info.exposure.clone(),
        execution_policy: tasks_info.execution_policy.clone(),
        executor: tasks_executor,
    })
}

pub(super) async fn patch_builtin_dynamic_tools(
    mut dynamic_tools: Vec<DynamicTool>,
    current_tool_ids: &[String],
    app_handle: &AppHandle,
    tool_server: &ToolServer,
    execution_id: &str,
    active_terminal_session_id: Option<&str>,
    host_working_directory: Option<&str>,
    referenced_traffic: &[serde_json::Value],
) -> Vec<DynamicTool> {
    if let Some(def) = build_traffic_response_read_tool(app_handle.clone(), referenced_traffic) {
        dynamic_tools.push(DynamicTool::new(def));
    }

    if current_tool_ids.iter().any(|id| id == ShellTool::NAME) {
        if let Some(def) = build_shell_override_def(tool_server, execution_id).await {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids
        .iter()
        .any(|id| id == HttpRequestTool::NAME)
    {
        if let Some(def) = build_http_override_def(tool_server, referenced_traffic).await {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids.iter().any(|id| id == GlobTool::NAME) {
        if let Some(def) = build_glob_override_def(
            tool_server,
            active_terminal_session_id,
            host_working_directory,
        )
        .await
        {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids.iter().any(|id| id == GrepTool::NAME) {
        if let Some(def) = build_grep_override_def(
            tool_server,
            active_terminal_session_id,
            host_working_directory,
        )
        .await
        {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids.iter().any(|id| id == LspTool::NAME) {
        if let Some(def) = build_lsp_override_def(
            tool_server,
            active_terminal_session_id,
            host_working_directory,
        )
        .await
        {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids.iter().any(|id| id == FileReadTool::NAME) {
        if let Some(def) = build_file_read_override_def(
            tool_server,
            execution_id,
            active_terminal_session_id,
            host_working_directory,
        )
        .await
        {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids.iter().any(|id| id == FileEditTool::NAME) {
        if let Some(def) = build_file_edit_override_def(
            tool_server,
            execution_id,
            active_terminal_session_id,
            host_working_directory,
        )
        .await
        {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids.iter().any(|id| id == FileWriteTool::NAME) {
        if let Some(def) = build_file_write_override_def(
            tool_server,
            execution_id,
            active_terminal_session_id,
            host_working_directory,
        )
        .await
        {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids.iter().any(|id| id == TasksTool::NAME) {
        if let Some(def) = build_tasks_override_def(tool_server, execution_id).await {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids
        .iter()
        .any(|id| id == AskUserQuestionTool::NAME)
    {
        if let Some(def) = build_ask_user_question_override_def(tool_server, execution_id).await {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids.iter().any(|id| id == ToolSearchTool::NAME) {
        if let Some(def) = build_tool_search_override_def(tool_server, execution_id).await {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids.iter().any(|id| id == TerminalServer::NAME) {
        if let Some(def) = build_interactive_shell_override_def(
            tool_server,
            execution_id,
            active_terminal_session_id,
            host_working_directory,
        )
        .await
        {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
        if let Some(def) =
            build_exec_command_override_def(tool_server, execution_id, host_working_directory).await
        {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
        if let Some(def) = build_write_stdin_override_def(tool_server, execution_id).await {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids
        .iter()
        .any(|id| id == EXEC_COMMAND_TOOL_NAME)
    {
        if let Some(def) =
            build_exec_command_override_def(tool_server, execution_id, host_working_directory).await
        {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
        if let Some(def) = build_write_stdin_override_def(tool_server, execution_id).await {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids
        .iter()
        .any(|id| id == WRITE_STDIN_TOOL_NAME)
    {
        if let Some(def) = build_write_stdin_override_def(tool_server, execution_id).await {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    dynamic_tools
}

pub(super) fn is_retryable_error(err_msg: &str) -> bool {
    let err_lower = err_msg.to_lowercase();
    if err_lower.contains("empty response") || err_lower.contains("without textual response") {
        return true;
    }
    if err_lower.contains("error decoding response body") {
        return true;
    }
    if err_lower.contains("unexpected eof") || err_lower.contains("connection closed") {
        return true;
    }
    if err_lower.contains("timed out") || err_lower.contains("timeout") {
        return true;
    }
    if err_lower.contains("connection reset") || err_lower.contains("network") {
        return true;
    }
    false
}

pub(super) fn is_empty_response_error(err_msg: &str) -> bool {
    let err_lower = err_msg.to_lowercase();
    err_lower.contains("empty response") || err_lower.contains("without textual response")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn infer_tool_result_success_detects_timeout_failure() {
        assert!(!infer_tool_result_success(
            "Tool execution failed: command timeout after 180000ms"
        ));
    }

    #[test]
    fn infer_tool_result_success_respects_success_field() {
        assert!(infer_tool_result_success(
            r#"{"success":true,"output":"ok"}"#
        ));
        assert!(!infer_tool_result_success(
            r#"{"success":false,"error":"boom"}"#
        ));
    }

    #[test]
    fn infer_tool_result_success_treats_http_response_body_as_payload() {
        assert!(infer_tool_result_success(
            r#"{"url":"http://example.com","status_code":200,"status_text":"200 OK","headers":{"content-type":"text/html"},"body":"Warning: file_get_contents(/home/flag): failed to open stream: No such file or directory","body_length":123}"#
        ));
    }

    #[test]
    fn infer_tool_result_success_treats_http_404_as_tool_success() {
        assert!(infer_tool_result_success(
            r#"{"url":"http://example.com/missing","status_code":404,"status_text":"404 Not Found","headers":{"content-type":"application/json"},"body":"{\"error\":\"resource not found\",\"code\":404}","body_length":43}"#
        ));
    }

    #[test]
    fn infer_tool_result_success_prefers_http_shape_over_generic_error_field() {
        assert!(infer_tool_result_success(
            r#"{"url":"http://example.com","status_code":500,"status_text":"500 Internal Server Error","headers":{"content-type":"application/json"},"error":"upstream application error","body":"{\"message\":\"boom\"}","body_length":18}"#
        ));
    }

    #[test]
    fn detects_high_risk_mutating_tool_calls() {
        assert!(is_high_risk_tool_call(
            "shell",
            r#"{"command":"sed -i 's/a/b/' src/app.rs"}"#
        ));
        assert!(is_high_risk_tool_call(
            "http_request",
            r#"{"method":"POST","url":"https://example.com/api"}"#
        ));
        assert!(!is_high_risk_tool_call(
            "shell",
            r#"{"command":"rg tenth_man src-tauri/src"}"#
        ));
    }

    #[test]
    fn counts_trailing_failed_tool_calls() {
        let records = vec![
            ToolCallRecord {
                id: "1".to_string(),
                name: "shell".to_string(),
                arguments: r#"{"command":"rg foo ."}"#.to_string(),
                result: Some("ok".to_string()),
                success: true,
                sequence: 0,
                started_at_ms: 0,
                completed_at_ms: 1,
                duration_ms: 1,
            },
            ToolCallRecord {
                id: "2".to_string(),
                name: "shell".to_string(),
                arguments: r#"{"command":"cargo test"}"#.to_string(),
                result: Some("failed".to_string()),
                success: false,
                sequence: 1,
                started_at_ms: 2,
                completed_at_ms: 3,
                duration_ms: 1,
            },
            ToolCallRecord {
                id: "3".to_string(),
                name: "shell".to_string(),
                arguments: r#"{"command":"cargo test"}"#.to_string(),
                result: Some("failed".to_string()),
                success: false,
                sequence: 2,
                started_at_ms: 4,
                completed_at_ms: 5,
                duration_ms: 1,
            },
        ];

        assert_eq!(trailing_failed_tool_calls(&records), 2);
    }

    #[test]
    fn final_response_review_required_when_changes_lack_verification() {
        let records = vec![ToolCallRecord {
            id: "1".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"sed -i 's/a/b/' src/app.rs"}"#.to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 0,
            started_at_ms: 0,
            completed_at_ms: 1,
            duration_ms: 1,
        }];

        assert!(final_response_needs_verification_review(
            "问题已修复，已经完成。",
            &records
        ));
    }

    #[test]
    fn final_response_review_skipped_when_verification_exists() {
        let records = vec![
            ToolCallRecord {
                id: "1".to_string(),
                name: "shell".to_string(),
                arguments: r#"{"command":"sed -i 's/a/b/' src/app.rs"}"#.to_string(),
                result: Some("ok".to_string()),
                success: true,
                sequence: 0,
                started_at_ms: 0,
                completed_at_ms: 1,
                duration_ms: 1,
            },
            ToolCallRecord {
                id: "2".to_string(),
                name: "shell".to_string(),
                arguments: r#"{"command":"cargo check"}"#.to_string(),
                result: Some("ok".to_string()),
                success: true,
                sequence: 1,
                started_at_ms: 2,
                completed_at_ms: 3,
                duration_ms: 1,
            },
        ];

        assert!(!final_response_needs_verification_review(
            "问题已修复，已经完成。",
            &records
        ));
    }

    #[test]
    fn final_response_review_required_for_low_evidence_high_confidence() {
        let records = vec![ToolCallRecord {
            id: "1".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"ls src"}"#.to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 0,
            started_at_ms: 0,
            completed_at_ms: 1,
            duration_ms: 1,
        }];

        assert!(final_response_needs_evidence_review(
            "可以确定根因就是这里，最佳方案已经明确。",
            None,
            &records,
            3,
        ));
    }

    #[test]
    fn final_response_review_skipped_when_evidence_is_sufficient() {
        let records = vec![
            ToolCallRecord {
                id: "1".to_string(),
                name: "shell".to_string(),
                arguments: r#"{"command":"rg tenth_man src-tauri/src"}"#.to_string(),
                result: Some("ok".to_string()),
                success: true,
                sequence: 0,
                started_at_ms: 0,
                completed_at_ms: 1,
                duration_ms: 1,
            },
            ToolCallRecord {
                id: "2".to_string(),
                name: "shell".to_string(),
                arguments: r#"{"command":"sed -n '1,120p' src-tauri/src/agents/tenth_man.rs"}"#
                    .to_string(),
                result: Some("ok".to_string()),
                success: true,
                sequence: 1,
                started_at_ms: 2,
                completed_at_ms: 3,
                duration_ms: 1,
            },
        ];

        assert!(!final_response_needs_evidence_review(
            "可以确定根因就是这里，最佳方案已经明确。",
            None,
            &records,
            3,
        ));
    }

    #[test]
    fn streaming_review_required_for_low_evidence_conclusion() {
        let records = vec![ToolCallRecord {
            id: "1".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"ls src"}"#.to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 0,
            started_at_ms: 0,
            completed_at_ms: 1,
            duration_ms: 1,
        }];

        assert!(streaming_content_needs_evidence_review(
            "基于当前情况，可以确定根因就是这里，因此应该直接按这个方向修改。",
            None,
            &records,
            3,
        ));
    }

    #[test]
    fn streaming_review_skipped_for_short_conclusion_fragment() {
        let records = vec![ToolCallRecord {
            id: "1".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"ls src"}"#.to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 0,
            started_at_ms: 0,
            completed_at_ms: 1,
            duration_ms: 1,
        }];

        assert!(!streaming_content_needs_evidence_review(
            "因此可以先看这里。",
            None,
            &records,
            3,
        ));
    }

    #[test]
    fn evidence_score_distinguishes_broad_scan_from_direct_inspection() {
        let broad_scan = evidence_score_for_tool_call("shell", r#"{"command":"ls src"}"#, true);
        let direct_read = evidence_score_for_tool_call(
            "shell",
            r#"{"command":"sed -n '1,120p' src/main.rs"}"#,
            true,
        );
        let verification =
            evidence_score_for_tool_call("shell", r#"{"command":"cargo check"}"#, true);

        assert_eq!(broad_scan, 1);
        assert_eq!(direct_read, 2);
        assert_eq!(verification, 3);
    }

    #[test]
    fn total_evidence_score_adds_weighted_evidence() {
        let records = vec![
            ToolCallRecord {
                id: "1".to_string(),
                name: "shell".to_string(),
                arguments: r#"{"command":"ls src"}"#.to_string(),
                result: Some("ok".to_string()),
                success: true,
                sequence: 0,
                started_at_ms: 0,
                completed_at_ms: 1,
                duration_ms: 1,
            },
            ToolCallRecord {
                id: "2".to_string(),
                name: "shell".to_string(),
                arguments: r#"{"command":"sed -n '1,120p' src/main.rs"}"#.to_string(),
                result: Some("ok".to_string()),
                success: true,
                sequence: 1,
                started_at_ms: 2,
                completed_at_ms: 3,
                duration_ms: 1,
            },
        ];

        assert_eq!(total_evidence_score(&records), 3);
    }

    #[test]
    fn relevant_evidence_score_ignores_unrelated_evidence() {
        let records = vec![
            ToolCallRecord {
                id: "1".to_string(),
                name: "shell".to_string(),
                arguments: r#"{"command":"rg metrics src-tauri/src"}"#.to_string(),
                result: Some("ok".to_string()),
                success: true,
                sequence: 0,
                started_at_ms: 0,
                completed_at_ms: 1,
                duration_ms: 1,
            },
            ToolCallRecord {
                id: "2".to_string(),
                name: "shell".to_string(),
                arguments: r#"{"command":"sed -n '1,120p' src-tauri/src/agents/metrics.rs"}"#
                    .to_string(),
                result: Some("ok".to_string()),
                success: true,
                sequence: 1,
                started_at_ms: 2,
                completed_at_ms: 3,
                duration_ms: 1,
            },
        ];

        assert!(final_response_needs_evidence_review(
            "可以确定问题就在 src-tauri/src/agents/tenth_man.rs 这里。",
            None,
            &records,
            3,
        ));
    }

    #[test]
    fn relevant_evidence_score_counts_matching_evidence() {
        let records = vec![
            ToolCallRecord {
                id: "1".to_string(),
                name: "shell".to_string(),
                arguments: r#"{"command":"rg tenth_man src-tauri/src"}"#.to_string(),
                result: Some("ok".to_string()),
                success: true,
                sequence: 0,
                started_at_ms: 0,
                completed_at_ms: 1,
                duration_ms: 1,
            },
            ToolCallRecord {
                id: "2".to_string(),
                name: "shell".to_string(),
                arguments: r#"{"command":"sed -n '1,120p' src-tauri/src/agents/tenth_man.rs"}"#
                    .to_string(),
                result: Some("ok".to_string()),
                success: true,
                sequence: 1,
                started_at_ms: 2,
                completed_at_ms: 3,
                duration_ms: 1,
            },
        ];

        assert!(!final_response_needs_evidence_review(
            "可以确定问题就在 src-tauri/src/agents/tenth_man.rs 这里。",
            None,
            &records,
            3,
        ));
    }

    #[tokio::test]
    async fn file_snapshot_requires_prior_read_and_fresh_mtime() {
        let execution_id = format!("exec-{}", uuid::Uuid::new_v4());
        let temp_dir = std::env::temp_dir().join(format!("file-state-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let file_path: PathBuf = temp_dir.join("sample.txt");
        tokio::fs::write(&file_path, "a\nb\nc\n").await.unwrap();
        let revision_token = sentinel_tools::buildin_tools::file_runtime::revision_token(
            &file_path.to_string_lossy(),
        )
        .await
        .unwrap();

        record_file_read_snapshot(
            &execution_id,
            &file_path.to_string_lossy(),
            &revision_token,
            1,
            2,
            3,
            true,
        )
        .await
        .unwrap();
        ensure_file_snapshot_is_editable(&execution_id, &file_path.to_string_lossy())
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        tokio::fs::write(&file_path, "changed\n").await.unwrap();
        let stale_err =
            ensure_file_snapshot_is_editable(&execution_id, &file_path.to_string_lossy())
                .await
                .unwrap_err()
                .to_string();
        assert!(stale_err.contains("changed since it was read"));

        crate::agents::executor::file_tool_state::clear_file_tool_state(&execution_id).await;
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn file_read_edit_override_allows_consecutive_edits_after_snapshot_update() {
        use rig::tool::Tool;
        use sentinel_tools::buildin_tools::shell::{
            get_shell_config, set_shell_config, ShellExecutionMode,
        };

        let execution_id = format!("exec-{}", uuid::Uuid::new_v4());
        let temp_dir =
            std::env::temp_dir().join(format!("file-override-state-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        let file_path: PathBuf = temp_dir.join("sample.txt");
        tokio::fs::write(&file_path, "alpha\nbeta\ngamma\n")
            .await
            .unwrap();
        let original_shell_config = get_shell_config().await;
        let mut host_shell_config = original_shell_config.clone();
        host_shell_config.default_execution_mode = ShellExecutionMode::Host;
        set_shell_config(host_shell_config).await;

        let tool_server = ToolServer::new();
        tool_server.init_builtin_tools().await;
        let working_dir = temp_dir.to_string_lossy().to_string();
        let read_def =
            build_file_read_override_def(&tool_server, &execution_id, None, Some(&working_dir))
                .await
                .expect("file_read override should be available");
        let edit_def =
            build_file_edit_override_def(&tool_server, &execution_id, None, Some(&working_dir))
                .await
                .expect("file_edit override should be available");
        let read_tool = DynamicTool::new(read_def);
        let edit_tool = DynamicTool::new(edit_def);

        read_tool
            .call(json!({
                "file_path": file_path.to_string_lossy(),
                "offset": 1,
                "limit": 200,
            }))
            .await
            .expect("file_read should record an editable snapshot");

        edit_tool
            .call(json!({
                "file_path": file_path.to_string_lossy(),
                "old_string": "beta",
                "new_string": "delta",
            }))
            .await
            .expect("first edit should update the snapshot to the new revision");

        edit_tool
            .call(json!({
                "file_path": file_path.to_string_lossy(),
                "old_string": "gamma",
                "new_string": "epsilon",
            }))
            .await
            .expect("second edit should not require another file_read");

        let updated = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(updated, "alpha\ndelta\nepsilon\n");

        crate::agents::executor::file_tool_state::clear_file_tool_state(&execution_id).await;
        set_shell_config(original_shell_config).await;
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}

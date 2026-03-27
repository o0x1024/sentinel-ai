use anyhow::Result;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use sentinel_db::Database;
use sentinel_db::DatabaseService;
use sentinel_llm::ChatMessage;
use sentinel_tools::buildin_tools::{HttpRequestTool, ShellTool, SkillsTool, TodosTool};
use sentinel_tools::dynamic_tool::{DynamicTool, DynamicToolDef, ToolExecutor, ToolSource};
use sentinel_tools::ToolServer;

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
        category: "system".to_string(),
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
                    json!({
                        "id": c.id,
                        "type": "function",
                        "function": {
                            "name": c.name,
                            "arguments": c.arguments,
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
    dynamic_tools
        .into_iter()
        .map(|tool| {
            if tool.name() == def.name {
                DynamicTool::new(def.clone())
            } else {
                tool
            }
        })
        .collect()
}

async fn build_shell_override_def(
    tool_server: &ToolServer,
    execution_id: &str,
) -> Option<DynamicToolDef> {
    let shell_info = tool_server.get_tool(ShellTool::NAME).await?;
    let execution_id_for_shell = execution_id.to_string();
    let shell_input_schema = shell_info.input_schema.clone();
    let shell_description = shell_info.description.clone();
    let shell_executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let execution_id_for_shell = execution_id_for_shell.clone();
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::shell::{ShellArgs, ShellTool};

            let mut patched_args = args;
            if let Some(obj) = patched_args.as_object_mut() {
                obj.insert(
                    "execution_id".to_string(),
                    serde_json::Value::String(execution_id_for_shell.clone()),
                );
                obj.insert(
                    "enable_large_output_storage".to_string(),
                    serde_json::Value::Bool(true),
                );
            }

            let tool_args: ShellArgs = serde_json::from_value(patched_args)
                .map_err(|e| format!("Invalid arguments: {}", e))?;

            let tool = ShellTool::new();
            let result = tool
                .call(tool_args)
                .await
                .map_err(|e| format!("Shell execution failed: {}", e))?;

            serde_json::to_value(result)
                .map_err(|e| format!("Failed to serialize shell result: {}", e))
        })
    });

    Some(DynamicToolDef {
        name: ShellTool::NAME.to_string(),
        description: shell_description,
        input_schema: shell_input_schema,
        output_schema: None,
        source: ToolSource::Builtin,
        category: "system".to_string(),
        executor: shell_executor,
    })
}

async fn build_http_override_def(tool_server: &ToolServer) -> Option<DynamicToolDef> {
    let http_info = tool_server.get_tool(HttpRequestTool::NAME).await?;
    let http_input_schema = http_info.input_schema.clone();
    let http_description = http_info.description.clone();
    let http_executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::http_request::{HttpRequestArgs, HttpRequestTool};

            let mut patched_args = args;
            if let Some(obj) = patched_args.as_object_mut() {
                obj.insert(
                    "enable_large_output_storage".to_string(),
                    serde_json::Value::Bool(true),
                );
            }

            let tool_args: HttpRequestArgs = serde_json::from_value(patched_args)
                .map_err(|e| format!("Invalid arguments: {}", e))?;

            let tool = HttpRequestTool::default();
            let result = tool
                .call(tool_args)
                .await
                .map_err(|e| format!("HTTP request failed: {}", e))?;

            serde_json::to_value(result)
                .map_err(|e| format!("Failed to serialize HTTP result: {}", e))
        })
    });

    Some(DynamicToolDef {
        name: HttpRequestTool::NAME.to_string(),
        description: http_description,
        input_schema: http_input_schema,
        output_schema: None,
        source: ToolSource::Builtin,
        category: "network".to_string(),
        executor: http_executor,
    })
}

async fn build_todos_override_def(
    tool_server: &ToolServer,
    execution_id: &str,
) -> Option<DynamicToolDef> {
    let todos_info = tool_server.get_tool(TodosTool::NAME).await?;
    let execution_id_for_todos = execution_id.to_string();
    let todos_input_schema = todos_info.input_schema.clone();
    let todos_description = todos_info.description.clone();
    let todos_executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let execution_id_for_todos = execution_id_for_todos.clone();
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::todos::{TodosArgs, TodosTool};

            let mut patched_args = args;
            if let Some(obj) = patched_args.as_object_mut() {
                obj.insert(
                    "execution_id".to_string(),
                    serde_json::Value::String(execution_id_for_todos.clone()),
                );
            }

            let tool_args: TodosArgs = serde_json::from_value(patched_args)
                .map_err(|e| format!("Invalid arguments: {}", e))?;

            let tool = TodosTool::new();
            let result = tool
                .call(tool_args)
                .await
                .map_err(|e| format!("Todos operation failed: {}", e))?;

            serde_json::to_value(result)
                .map_err(|e| format!("Failed to serialize todos result: {}", e))
        })
    });

    Some(DynamicToolDef {
        name: TodosTool::NAME.to_string(),
        description: todos_description,
        input_schema: todos_input_schema,
        output_schema: None,
        source: ToolSource::Builtin,
        category: "system".to_string(),
        executor: todos_executor,
    })
}

pub(super) async fn patch_builtin_dynamic_tools(
    mut dynamic_tools: Vec<DynamicTool>,
    current_tool_ids: &[String],
    tool_server: &ToolServer,
    execution_id: &str,
) -> Vec<DynamicTool> {
    if current_tool_ids.iter().any(|id| id == ShellTool::NAME) {
        if let Some(def) = build_shell_override_def(tool_server, execution_id).await {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids
        .iter()
        .any(|id| id == HttpRequestTool::NAME)
    {
        if let Some(def) = build_http_override_def(tool_server).await {
            dynamic_tools = replace_dynamic_tool(dynamic_tools, def);
        }
    }

    if current_tool_ids.iter().any(|id| id == TodosTool::NAME) {
        if let Some(def) = build_todos_override_def(tool_server, execution_id).await {
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
}

//! Tool-enabled execution path.

use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use sentinel_db::Database;
use sentinel_db::DatabaseService;
use sentinel_llm::{parse_image_from_json, ChatMessage, StreamContent, StreamingLlmClient};
use sentinel_memory::{get_global_memory, ExecutionRecord, ToolCallSummary};
use sentinel_tools::buildin_tools::{ShellTool, SkillsTool, TodosTool};
use sentinel_tools::dynamic_tool::{DynamicTool, DynamicToolDef, ToolExecutor, ToolSource};
use sentinel_tools::ToolServer;

use super::AgentExecuteParams;
use crate::agents::context_engineering::reflection::{
    record_execution_reflection, ExecutionOutcome,
};
use crate::agents::executor::message_store::save_assistant_message;
use crate::agents::executor::types::ToolCallRecord;
use crate::agents::executor::utils::{cleanup_container_context_async, truncate_for_memory};
use crate::agents::tenth_man::{InterventionContext, InterventionMode, TenthMan, TriggerReason};
use crate::agents::tool_router::ToolRouter;
use crate::agents::{append_tool_digests, build_context, build_tool_digest, ContextBuildInput};
use crate::utils::ai_generation_settings::apply_generation_settings_from_db;

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

async fn register_skills_tool_guard(
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

fn apply_allowed_tools_policy(mut tool_ids: Vec<String>, allowed_tools: &[String]) -> Vec<String> {
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

fn infer_tool_result_success(raw: &str) -> bool {
    fn visit(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Null => true,
            serde_json::Value::Bool(v) => *v,
            serde_json::Value::Number(n) => n.as_i64().map(|v| v == 0).unwrap_or(true),
            serde_json::Value::String(s) => {
                let lower = s.trim().to_lowercase();
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
    raw.chars().take(max_chars).collect::<String>().trim().to_string()
}

fn tool_loop_fingerprint(tool_name: &str, arguments: &str, result: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    tool_name.hash(&mut hasher);
    shorten_for_fingerprint(arguments, 320).hash(&mut hasher);
    shorten_for_fingerprint(result, 320).hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, Clone)]
struct TeamStreamContext {
    session_id: String,
    stream_id: String,
    member_id: Option<String>,
    phase: String,
}

fn parse_team_stream_context(execution_id: &str) -> Option<TeamStreamContext> {
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
    // New format: team-v3:{session_id}:{task_id}:{member_id}:{uuid}
    // Legacy format: team-v3:{session_id}:{task_id}:{uuid}
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

async fn persist_ai_message_with_retry(
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

async fn ensure_ai_conversation_exists_for_persistence(
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

pub async fn execute_agent_with_tools(
    app_handle: &AppHandle,
    params: AgentExecuteParams,
    tool_server: &ToolServer,
) -> Result<String> {
    let tool_config = params.tool_config.clone().unwrap_or_default();

    // 1. 创建工具路由器（加载所有动态工具：工作流、MCP、插件）
    use tauri::Manager;
    let db_service = app_handle.state::<std::sync::Arc<sentinel_db::DatabaseService>>();

    let tool_router = ToolRouter::new_with_all_tools(Some(db_service.inner())).await;

    // 2. 工具选择（传入 LLM 配置用于智能选择）
    let rig_provider = params.rig_provider.to_lowercase();
    let mut llm_config = sentinel_llm::LlmConfig::new(&rig_provider, &params.model)
        .with_timeout(params.timeout_secs)
        .with_max_turns(params.max_iterations)
        .with_rig_provider(&rig_provider)
        .with_conversation_id(&params.execution_id);

    if let Some(ref api_key) = params.api_key {
        llm_config = llm_config.with_api_key(api_key);
    }

    if let Some(ref api_base) = params.api_base {
        llm_config = llm_config.with_base_url(api_base);
    }

    if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        llm_config = apply_generation_settings_from_db(db.as_ref(), llm_config).await;
    }

    // Use plan_tools to support Skills mode with injected context
    let selection_plan = tool_router
        .plan_tools(&params.task, &tool_config, Some(&llm_config))
        .await?;

    let mut selected_tool_ids =
        apply_allowed_tools_policy(selection_plan.tool_ids.clone(), &tool_config.allowed_tools);

    tracing::info!(
        "Selected {} tools for execution_id {}: {:?} (strategy={:?})",
        selected_tool_ids.len(),
        params.execution_id,
        selected_tool_ids,
        tool_config.selection_strategy
    );

    // Emit skill_selected event if applicable
    if let Some(ref skill) = selection_plan.selected_skill {
        let _ = app_handle.emit(
            "agent:skill_selected",
            &json!({
                "execution_id": params.execution_id,
                "skill_id": skill.id,
                "skill_name": skill.name,
            }),
        );
    }

    // 发送工具选择事件到前端
    let _ = app_handle.emit(
        "agent:tools_selected",
        &json!({
            "execution_id": params.execution_id,
            "tools": selected_tool_ids,
        }),
    );

    // 3. 获取 DynamicTool 实例（用于 rig-core 原生工具调用）
    let mut current_tool_ids = selected_tool_ids.clone();

    // 4. Build context via Context Engineering
    let context_policy = params
        .context_policy
        .clone()
        .unwrap_or_else(crate::agents::ContextPolicy::default);
    let context_result = build_context(ContextBuildInput {
        app_handle: app_handle.clone(),
        execution_id: params.execution_id.clone(),
        base_system_prompt: params.system_prompt.clone(),
        injected_skill_prompt: selection_plan.injected_system_prompt.clone(),
        task: params.task.clone(),
        rig_provider: rig_provider.clone(),
        llm_config: llm_config.clone(),
        selected_tool_ids: selected_tool_ids.clone(),
        document_attachments: params.document_attachments.clone(),
        policy: context_policy.clone(),
    })
    .await?;

    let final_system_prompt_content = Some(context_result.system_prompt);
    let mut history_chat_messages = context_result.history_messages;

    // 移除历史记录中最后一条用户消息，避免与当前任务重复发送
    // 因为 stream_chat_with_dynamic_tools 会自动将 user_prompt 添加到对话末尾
    if let Some(last) = history_chat_messages.last() {
        if last.role == "user" {
            history_chat_messages.pop();
        }
    }

    // 解析图片附件
    let image_attachment = parse_image_from_json(params.image_attachments.as_ref());

    // 6. 使用 rig-core 原生工具调用
    // rig 的 multi_turn() 会自动处理工具调用循环
    let client = StreamingLlmClient::new(llm_config);
    let execution_id = params.execution_id.clone();
    let team_stream_context = parse_team_stream_context(&execution_id);
    let app = app_handle.clone();
    let db_for_stream: Option<std::sync::Arc<sentinel_db::DatabaseService>> =
        if params.persist_messages {
            app_handle
                .try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
                .map(|s| s.inner().clone())
        } else {
            None
        };
    if let Some(db) = db_for_stream.as_ref() {
        ensure_ai_conversation_exists_for_persistence(
            db.as_ref(),
            &execution_id,
            &params.model,
            &params.rig_provider,
        )
        .await;
    }

    // 用于收集工具调用信息
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
    use std::sync::Mutex;
    let tool_calls_collector: Arc<Mutex<Vec<ToolCallRecord>>> = Arc::new(Mutex::new(Vec::new()));
    let pending_calls: Arc<Mutex<std::collections::HashMap<String, (String, String, i64, u32)>>> =
        Arc::new(Mutex::new(std::collections::HashMap::new()));
    let tool_seq: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));
    let tool_call_counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let loop_break_requested: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let loop_guard_prompt_needed: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let last_tool_fingerprint: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));
    let repeated_tool_fingerprint_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let assistant_segment_buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    // Track how many assistant text segments have been flushed (persisted) to the database
    // at tool-call boundaries. When > 0, the final save_assistant_message should only save
    // the last turn's response to avoid duplicating earlier segments.
    let persisted_segment_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let reasoning_content_buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let pending_tool_digests: Arc<Mutex<Vec<crate::agents::ToolDigest>>> =
        Arc::new(Mutex::new(Vec::new()));
    let context_policy_for_stream = context_policy.clone();

    let collector = tool_calls_collector.clone();
    let pending = pending_calls.clone();
    let seq_counter = tool_seq.clone();
    let tool_counter = tool_call_counter.clone();
    let loop_break_flag = loop_break_requested.clone();
    let loop_prompt_flag = loop_guard_prompt_needed.clone();
    let last_tool_fp = last_tool_fingerprint.clone();
    let repeated_tool_fp_count = repeated_tool_fingerprint_count.clone();
    let segment_buf = assistant_segment_buf.clone();
    let reasoning_buf = reasoning_content_buf.clone();
    let pending_digests = pending_tool_digests.clone();
    let persisted_seg_count = persisted_segment_count.clone();

    // Ensure skills tool enforces per-skill enable flags at execution time.
    if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        register_skills_tool_guard(tool_server, db.inner().clone()).await?;
    }

    // 7. 调用带动态工具的流式方法，增加重试机制以应对模型抖动或解析错误
    let mut retries = 0;
    let max_retries = 2; // 最多重试 2 次
    let mut last_error: Option<anyhow::Error> = None;
    let mut skill_reload_count = 0;
    let max_skill_reload = 3;

    // 累积的工具调用记录（跨重试保留）
    let accumulated_tool_calls: Arc<Mutex<Vec<ToolCallRecord>>> = Arc::new(Mutex::new(Vec::new()));
    // 累积的助手输出（跨重试保留）
    let accumulated_assistant_output: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let base_history_messages = history_chat_messages.clone();
    let build_retry_history = |attempt: u32, include_accumulated: bool| -> Vec<ChatMessage> {
        let mut history = base_history_messages.clone();
        if attempt == 0 && !include_accumulated {
            return history;
        }

        let tool_calls_snapshot = accumulated_tool_calls
            .lock()
            .map(|calls| calls.clone())
            .unwrap_or_default();
        let output_snapshot = accumulated_assistant_output
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();

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

            for call in ordered_calls.iter() {
                if let Some(result) = &call.result {
                    history.push(ChatMessage::tool(result.clone(), call.id.clone()));
                }
            }
        }

        if !output_snapshot.trim().is_empty() {
            history.push(ChatMessage::assistant(output_snapshot));
        }

        history
    };

    let skill_reload_requested = Arc::new(AtomicBool::new(false));
    let loaded_skill_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let team_stream_started = Arc::new(AtomicBool::new(false));
    let team_stream_had_delta = Arc::new(AtomicBool::new(false));

    let mut force_history_with_tools = false;
    while retries <= max_retries {
        // Early exit if cancelled before starting a new stream turn
        if crate::commands::ai::is_conversation_cancelled(&params.execution_id) {
            tracing::info!(
                "Execution cancelled before new stream turn: {}",
                params.execution_id
            );
            let _ = app_handle.emit(
                "agent:complete",
                &serde_json::json!({
                    "execution_id": params.execution_id,
                    "cancelled": true,
                }),
            );
            return Ok(String::new());
        }

        let mut dynamic_tools = tool_server.get_dynamic_tools(&current_tool_ids).await;

        if current_tool_ids.iter().any(|id| id == ShellTool::NAME) {
            if let Some(shell_info) = tool_server.get_tool(ShellTool::NAME).await {
                let execution_id_for_shell = params.execution_id.clone();
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

                let shell_def = DynamicToolDef {
                    name: ShellTool::NAME.to_string(),
                    description: shell_description,
                    input_schema: shell_input_schema,
                    output_schema: None,
                    source: ToolSource::Builtin,
                    category: "system".to_string(),
                    executor: shell_executor,
                };

                dynamic_tools = dynamic_tools
                    .into_iter()
                    .map(|tool| {
                        if tool.name() == ShellTool::NAME {
                            DynamicTool::new(shell_def.clone())
                        } else {
                            tool
                        }
                    })
                    .collect();
            }
        }

        if current_tool_ids.iter().any(|id| id == TodosTool::NAME) {
            if let Some(todos_info) = tool_server.get_tool(TodosTool::NAME).await {
                let execution_id_for_todos = params.execution_id.clone();
                let todos_input_schema = todos_info.input_schema.clone();
                let todos_description = todos_info.description.clone();
                let todos_executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
                    let execution_id_for_todos = execution_id_for_todos.clone();
                    Box::pin(async move {
                        use rig::tool::Tool;
                        use sentinel_tools::buildin_tools::todos::{TodosArgs, TodosTool};

                        let mut patched_args = args;
                        if let Some(obj) = patched_args.as_object_mut() {
                            // Align todos writes with current execution context to avoid
                            // cross-run leakage when model-provided execution_id is stale.
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

                let todos_def = DynamicToolDef {
                    name: TodosTool::NAME.to_string(),
                    description: todos_description,
                    input_schema: todos_input_schema,
                    output_schema: None,
                    source: ToolSource::Builtin,
                    category: "system".to_string(),
                    executor: todos_executor,
                };

                dynamic_tools = dynamic_tools
                    .into_iter()
                    .map(|tool| {
                        if tool.name() == TodosTool::NAME {
                            DynamicTool::new(todos_def.clone())
                        } else {
                            tool
                        }
                    })
                    .collect();
            }
        }

        tracing::info!(
            "Got {} dynamic tool instances for rig-core native tool calling",
            dynamic_tools.len()
        );

        if retries > 0 {
            // 保存当前已完成的工具调用到累积记录
            if let Ok(current_calls) = tool_calls_collector.lock() {
                if let Ok(mut acc) = accumulated_tool_calls.lock() {
                    acc.extend(current_calls.clone());
                }
            }

            // 保存当前已输出的内容到累积输出
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

            tracing::warn!(
                "Retrying agent execution (attempt {}/{}) due to error: {}. Accumulated {} tool calls and {} chars output.",
                retries,
                max_retries,
                last_error.as_ref().map(|e| e.to_string()).unwrap_or_default(),
                accumulated_tool_calls.lock().map(|c| c.len()).unwrap_or(0),
                accumulated_assistant_output.lock().map(|s| s.len()).unwrap_or(0)
            );

            // 发送重试事件给前端（包含已完成的进度信息）
            let _ = app_handle.emit(
                "agent:retry",
                &json!({
                    "execution_id": params.execution_id,
                    "retry_count": retries,
                    "max_retries": max_retries,
                    "error": last_error.as_ref().map(|e| e.to_string()),
                    "accumulated_progress": {
                        "tool_calls": accumulated_tool_calls.lock().map(|c| c.len()).unwrap_or(0),
                        "output_chars": accumulated_assistant_output.lock().map(|s| s.len()).unwrap_or(0),
                    }
                }),
            );

            // 重试前稍作延迟
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }

        let include_accumulated = retries > 0 || force_history_with_tools;
        let mut history_for_retry = build_retry_history(retries, include_accumulated);
        if loop_guard_prompt_needed.swap(false, Ordering::SeqCst) {
            history_for_retry.push(ChatMessage::user(
                "[LoopGuard] You are repeating the same tool call arguments and getting the same result. Do not repeat identical probes. First summarize what has been learned, then change strategy (different endpoint/input/query) or explicitly conclude insufficient evidence.",
            ));
        }
        force_history_with_tools = false;
        let result = client
            .stream_chat_with_dynamic_tools(
                final_system_prompt_content.as_deref(),
                &params.task,
                &history_for_retry,
                image_attachment.as_ref(), // 传递图片附件
                dynamic_tools.clone(),
                |content| {
                    if crate::commands::ai::is_conversation_cancelled(&execution_id) {
                        return false;
                    }
                    match content {
                        StreamContent::Text(text) => {
                            if let Some(ctx) = team_stream_context.as_ref() {
                                if !team_stream_started.swap(true, Ordering::SeqCst) {
                                    let _ = app.emit(
                                        "agent_team:message_stream_start",
                                        &json!({
                                            "session_id": ctx.session_id.clone(),
                                            "stream_id": ctx.stream_id.clone(),
                                            "member_id": ctx.member_id.clone(),
                                            "member_name": ctx.member_id.clone(),
                                            "phase": ctx.phase.clone(),
                                        }),
                                    );
                                }
                                team_stream_had_delta.store(true, Ordering::SeqCst);
                                let _ = app.emit(
                                    "agent_team:message_stream_delta",
                                    &json!({
                                        "session_id": ctx.session_id.clone(),
                                        "stream_id": ctx.stream_id.clone(),
                                        "member_id": ctx.member_id.clone(),
                                        "member_name": ctx.member_id.clone(),
                                        "phase": ctx.phase.clone(),
                                        "delta": text.clone(),
                                    }),
                                );
                            }
                            // Accumulate assistant text into a segment buffer.
                            let _ = segment_buf.lock().map(|mut buf| buf.push_str(&text));

                            let _ = app.emit(
                                "agent:chunk",
                                &json!({
                                    "execution_id": execution_id,
                                    "chunk_type": "text",
                                    "content": text,
                                }),
                            );
                        }
                        StreamContent::Reasoning(reasoning) => {
                            // Accumulate reasoning content
                            if let Ok(mut buf) = reasoning_buf.lock() {
                                buf.push_str(&reasoning);
                            }
                            let _ = app.emit(
                                "agent:chunk",
                                &json!({
                                    "execution_id": execution_id,
                                    "chunk_type": "reasoning",
                                    "content": reasoning,
                                }),
                            );
                        }
                        StreamContent::ToolCallStart { id, name } => {
                            tracing::debug!("Tool call started via rig-core: {} ({})", name, id);
                            
                            // Increment tool call counter
                            tool_counter.fetch_add(1, Ordering::SeqCst);
                            
                            // Tenth Man Intervention Point 1: Before Tool Execution
                            if params.enable_tenth_man_rule {
                                if let Some(ref tm_config) = params.tenth_man_config {
                                    let tenth_man = TenthMan::new(&params);
                                    let current_count = tool_counter.load(Ordering::SeqCst) as usize;
                                    
                                    let context = InterventionContext {
                                        execution_id: execution_id.clone(),
                                        task: params.task.clone(),
                                        tool_call_count: current_count,
                                        current_content: Some(format!("Preparing to call tool: {}", name)),
                                        trigger_reason: TriggerReason::ToolCallThreshold,
                                    };
                                    
                                    if tenth_man.should_trigger(&context) {
                                        let app_clone = app.clone();
                                        let exec_id = execution_id.clone();
                                        let tool_name = name.clone();
                                        let require_confirmation = tm_config.require_user_confirmation;
                                        
                                        tauri::async_runtime::spawn(async move {
                                            match tenth_man.quick_review(&context).await {
                                                Ok(Some(critique)) => {
                                                    tracing::info!("Tenth Man warning before tool call: {}", tool_name);
                                                    let _ = app_clone.emit(
                                                        "agent:tenth_man_warning",
                                                        &json!({
                                                            "execution_id": exec_id,
                                                            "trigger": "before_tool_call",
                                                            "tool_name": tool_name,
                                                            "critique": critique,
                                                            "requires_confirmation": require_confirmation,
                                                        })
                                                    );
                                                }
                                                Ok(None) => {
                                                    tracing::debug!("Tenth Man: No significant risk detected");
                                                }
                                                Err(e) => {
                                                    tracing::warn!("Tenth Man quick review failed: {}", e);
                                                }
                                            }
                                        });
                                    }
                                }
                            }
                            
                            let _ = app.emit(
                                "agent:tool_call_start",
                                &json!({
                                    "execution_id": execution_id,
                                    "tool_call_id": id,
                                    "tool_name": name,
                                }),
                            );
                        }
                        StreamContent::ToolCallDelta { id, delta } => {
                            let _ = app.emit(
                                "agent:tool_call_delta",
                                &json!({
                                    "execution_id": execution_id,
                                    "tool_call_id": id,
                                    "delta": delta,
                                }),
                            );
                        }
                        StreamContent::ToolCallComplete {
                            id,
                            name,
                            arguments,
                        } => {
                            tracing::debug!("Tool call complete via rig-core: {} ({})", name, id);
                            sentinel_llm::log::log_tool_call(
                                &execution_id,
                                Some(&execution_id),
                                &params.rig_provider,
                                &params.model,
                                &name,
                                &id,
                                &arguments,
                            );

                            // 记录 pending 的工具调用，等待结果
                            if let Ok(mut pending_map) = pending.lock() {
                                let seq = seq_counter.fetch_add(1, Ordering::Relaxed);
                                let started_at_ms = chrono::Utc::now().timestamp_millis() + seq as i64;
                                pending_map.insert(
                                    id.clone(),
                                    (name.clone(), arguments.clone(), started_at_ms, seq),
                                );
                            }

                            // Flush assistant segment BEFORE inserting tool call message (preserve ordering on reload).
                            if let Some(db) = db_for_stream.clone() {
                                use sentinel_core::models::database as core_db;
                                use chrono::TimeZone;
                                let seg = segment_buf
                                    .lock()
                                    .map(|mut g| std::mem::take(&mut *g))
                                    .unwrap_or_default();
                                let seg_trimmed = seg.trim().to_string();
                                if !seg_trimmed.trim().is_empty() {
                                    persisted_seg_count.fetch_add(1, Ordering::SeqCst);
                                    // Ensure segment timestamp is slightly before tool call timestamp.
                                    let seg_ts_ms = chrono::Utc::now().timestamp_millis() - 1;
                                    let seg_ts = chrono::Utc
                                        .timestamp_millis_opt(seg_ts_ms)
                                        .single()
                                        .unwrap_or_else(chrono::Utc::now);

                                    // Get reasoning content (for deepseek-reasoner with tool calls, always include it)
                                    // 参考：https://api-docs.deepseek.com/zh-cn/guides/thinking_mode#tool-calls
                                    let reasoning = reasoning_buf
                                        .lock()
                                        .map(|g| {
                                            let r = g.clone();
                                            // 即使为空也返回 Some("")，因为 deepseek-reasoner 要求必须有此字段
                                            Some(if r.trim().is_empty() {
                                                String::new()
                                            } else {
                                                r
                                            })
                                        })
                                        .ok()
                                        .flatten();

                                    let seg_msg = core_db::AiMessage {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        conversation_id: execution_id.clone(),
                                        role: "assistant".to_string(),
                                        content: seg_trimmed.clone(),
                                        metadata: None,
                                        token_count: Some(seg_trimmed.len() as i32),
                                        cost: None,
                                        tool_calls: None,
                                        attachments: None,
                                        reasoning_content: reasoning,
                                        timestamp: seg_ts,
                                        architecture_type: None,
                                        architecture_meta: None,
                                        structured_data: None,
                                    };
                                    tauri::async_runtime::spawn(async move {
                                        persist_ai_message_with_retry(
                                            db,
                                            seg_msg,
                                            "assistant segment",
                                        )
                                        .await;
                                    });
                                }
                            }

                            // Persist tool call as a standalone message (role=tool) so history ordering is correct.
                            if let Some(db) = db_for_stream.clone() {
                                use sentinel_core::models::database as core_db;
                                use chrono::TimeZone;
                                let (started_at_ms, seq) = pending
                                    .lock()
                                    .ok()
                                    .and_then(|m| m.get(&id).map(|(_, _, ms, s)| (*ms, *s)))
                                    .unwrap_or((chrono::Utc::now().timestamp_millis(), 0));

                                let started_at = chrono::Utc
                                    .timestamp_millis_opt(started_at_ms)
                                    .single()
                                    .unwrap_or_else(chrono::Utc::now);

                                let tool_args_val: serde_json::Value =
                                    serde_json::from_str(&arguments)
                                        .unwrap_or_else(|_| json!({ "raw": arguments }));
                                let meta = json!({
                                    "kind": "tool_call",
                                    "tool_name": name,
                                    "tool_args": tool_args_val,
                                    "tool_call_id": id,
                                    "status": "running",
                                    "sequence": seq,
                                    "started_at_ms": started_at_ms,
                                });

                                let tool_msg = core_db::AiMessage {
                                    id: id.clone(),
                                    conversation_id: execution_id.clone(),
                                    role: "tool".to_string(),
                                    content: String::new(),
                                    metadata: Some(meta.to_string()),
                                    token_count: None,
                                    cost: None,
                                    tool_calls: None,
                                    attachments: None,
                                    reasoning_content: None,
                                    timestamp: started_at,
                                    architecture_type: None,
                                    architecture_meta: None,
                                    structured_data: None,
                                };
                                tauri::async_runtime::spawn(async move {
                                    persist_ai_message_with_retry(db, tool_msg, "tool call message")
                                        .await;
                                });
                            }

                            let team_tool_call_id = id.clone();
                            let team_tool_name = name.clone();
                            let team_tool_arguments = arguments.clone();
                            let _ = app.emit(
                                "agent:tool_call_complete",
                                &json!({
                                    "execution_id": execution_id,
                                    "tool_call_id": id,
                                    "tool_name": name,
                                    "arguments": arguments,
                                }),
                            );
                            if let Some(ctx) = team_stream_context.as_ref() {
                                let _ = app.emit(
                                    "agent_team:tool_call",
                                    &json!({
                                        "session_id": ctx.session_id.clone(),
                                        "stream_id": ctx.stream_id.clone(),
                                        "member_id": ctx.member_id.clone(),
                                        "member_name": ctx.member_id.clone(),
                                        "phase": ctx.phase.clone(),
                                        "tool_call_id": team_tool_call_id,
                                        "name": team_tool_name,
                                        "arguments": team_tool_arguments,
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                    }),
                                );
                            }
                        }
                        StreamContent::ToolResult { id, result } => {
                            // tracing::info!(
                            //     "Tool result via rig-core: id={}, result_preview={}",
                //     id,
                //     &result.chars().take(500).collect::<String>()
                // );

                            // 将工具调用完整信息添加到收集器
                            if let Ok(mut pending_map) = pending.lock() {
                                if let Some((name, arguments, started_at_ms, seq)) =
                                    pending_map.remove(&id)
                                {
                                    let completed_at_ms = chrono::Utc::now().timestamp_millis();
                                    let duration_ms = completed_at_ms.saturating_sub(started_at_ms);
                                    let name_for_meta = name.clone();
                                    let args_for_meta = arguments.clone();
                                    sentinel_llm::log::log_tool_result(
                                        &execution_id,
                                        Some(&execution_id),
                                        &params.rig_provider,
                                        &params.model,
                                        &name_for_meta,
                                        &id,
                                        Some(duration_ms),
                                        !result.to_lowercase().contains("error"),
                                        &result,
                                    );
                                    if let Ok(mut records) = collector.lock() {
                                        records.push(ToolCallRecord {
                                            id: id.clone(),
                                            name,
                                            arguments,
                                            result: Some(result.clone()),
                                            success: !result.to_lowercase().contains("error"),
                                            sequence: seq,
                                            started_at_ms,
                                            completed_at_ms,
                                            duration_ms,
                                        });
                                    }

                                    // Update persisted tool message with result (keep timestamp as started_at to avoid reordering).
                                    if let Some(db) = db_for_stream.clone() {
                                        use sentinel_core::models::database as core_db;
                                        use chrono::TimeZone;
                                        let started_at = chrono::Utc
                                            .timestamp_millis_opt(started_at_ms)
                                            .single()
                                            .unwrap_or_else(chrono::Utc::now);

                                        let tool_args_val: serde_json::Value =
                                            serde_json::from_str(&args_for_meta)
                                                .unwrap_or_else(|_| json!({ "raw": args_for_meta }));
                                        let tool_success = infer_tool_result_success(&result);
                                        let meta = json!({
                                            "kind": "tool_call",
                                            "tool_name": name_for_meta,
                                            "tool_args": tool_args_val,
                                            "tool_call_id": id,
                                            "status": if tool_success { "completed" } else { "failed" },
                                            "sequence": seq,
                                            "started_at_ms": started_at_ms,
                                            "completed_at_ms": completed_at_ms,
                                            "duration_ms": duration_ms,
                                            "tool_result": result,
                                            "success": tool_success,
                                        });
                                        let tool_msg = core_db::AiMessage {
                                            id: id.clone(),
                                            conversation_id: execution_id.clone(),
                                            role: "tool".to_string(),
                                            content: String::new(),
                                            metadata: Some(meta.to_string()),
                                            token_count: None,
                                            cost: None,
                                            tool_calls: None,
                                            attachments: None,
                                            reasoning_content: None,
                                            timestamp: started_at,
                                            architecture_type: None,
                                            architecture_meta: None,
                                            structured_data: None,
                                        };
                                        tauri::async_runtime::spawn(async move {
                                            persist_ai_message_with_retry(
                                                db,
                                                tool_msg,
                                                "tool result update",
                                            )
                                            .await;
                                        });
                                    }

                                    if name_for_meta == "skills" {
                                        if let Ok(args_json) =
                                            serde_json::from_str::<serde_json::Value>(&args_for_meta)
                                        {
                                            if args_json
                                                .get("action")
                                                .and_then(|v| v.as_str())
                                                .map(|a| a == "load")
                                                .unwrap_or(false)
                                            {
                                                if let Some(skill_id) = args_json
                                                    .get("skill_id")
                                                    .and_then(|v| v.as_str())
                                                {
                                                    if let Ok(mut slot) = loaded_skill_id.lock() {
                                                        *slot = Some(skill_id.to_string());
                                                    }
                                                    skill_reload_requested
                                                        .store(true, Ordering::SeqCst);
                                                }
                                            }
                                        }
                                    }

                                    let args_value: serde_json::Value = serde_json::from_str(&args_for_meta)
                                        .unwrap_or_else(|_| json!({ "raw": args_for_meta }));
                                    let digest = build_tool_digest(&name_for_meta, &args_value, &result);
                                    if let Ok(mut queue) = pending_digests.lock() {
                                        queue.push(digest);
                                    }

                                    let fingerprint =
                                        tool_loop_fingerprint(&name_for_meta, &args_for_meta, &result);
                                    let mut loop_triggered = false;
                                    let mut repeat_hits = 0usize;
                                    if let Ok(mut last_slot) = last_tool_fp.lock() {
                                        let same_as_last = last_slot
                                            .as_ref()
                                            .map(|prev| *prev == fingerprint)
                                            .unwrap_or(false);
                                        if same_as_last {
                                            repeat_hits =
                                                repeated_tool_fp_count.fetch_add(1, Ordering::SeqCst) + 1;
                                            if repeat_hits >= 2 {
                                                loop_triggered = true;
                                            }
                                        } else {
                                            *last_slot = Some(fingerprint);
                                            repeated_tool_fp_count.store(0, Ordering::SeqCst);
                                        }
                                    }

                                    if loop_triggered {
                                        loop_break_flag.store(true, Ordering::SeqCst);
                                        loop_prompt_flag.store(true, Ordering::SeqCst);
                                        tracing::warn!(
                                            "Detected repeated tool loop - execution_id: {}, tool: {}, repeats: {}",
                                            execution_id,
                                            name_for_meta,
                                            repeat_hits
                                        );
                                        let _ = app.emit(
                                            "agent:loop_detected",
                                            &json!({
                                                "execution_id": execution_id,
                                                "tool_name": name_for_meta,
                                                "repeat_count": repeat_hits,
                                                "reason": "repeated identical tool arguments and result"
                                            }),
                                        );
                                    }
                                }
                            }

                            let team_tool_call_id = id.clone();
                            let team_result = result.clone();
                            let team_success = infer_tool_result_success(&result);
                            let _ = app.emit(
                                "agent:tool_result",
                                &json!({
                                    "execution_id": execution_id,
                                    "tool_call_id": id,
                                    "result": result,
                                    "success": team_success,
                                }),
                            );
                            if let Some(ctx) = team_stream_context.as_ref() {
                                let _ = app.emit(
                                    "agent_team:tool_result",
                                    &json!({
                                        "session_id": ctx.session_id.clone(),
                                        "stream_id": ctx.stream_id.clone(),
                                        "member_id": ctx.member_id.clone(),
                                        "member_name": ctx.member_id.clone(),
                                        "phase": ctx.phase.clone(),
                                        "tool_call_id": team_tool_call_id,
                                        "result": team_result,
                                        "success": team_success,
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                    }),
                                );
                            }
                        }
                StreamContent::Usage {
                    input_tokens,
                    output_tokens,
                } => {
                    tracing::info!(
                        "Token usage report - execution_id: {}, input: {}, output: {}, total: {}",
                        execution_id,
                        input_tokens,
                        output_tokens,
                        input_tokens + output_tokens
                    );
                    let _ = app.emit(
                                "agent:chunk",
                                &json!({
                                    "execution_id": execution_id,
                                    "chunk_type": "usage",
                                    "input_tokens": input_tokens,
                                    "output_tokens": output_tokens,
                                }),
                            );

                            // 记录 token 使用统计到数据库
                            if input_tokens > 0 || output_tokens > 0 {
                                use tauri::Manager;
                                if let Some(db) =
                                    app.try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
                                {
                                    let provider = params.rig_provider.clone();
                                    let model = params.model.clone();
                                    let cost = sentinel_llm::calculate_cost(
                                        &provider,
                                        &model,
                                        input_tokens,
                                        output_tokens,
                                    );

                                    let db_clone = db.inner().clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = db_clone
                                            .update_ai_usage(
                                                &provider,
                                                &model,
                                                input_tokens as i32,
                                                output_tokens as i32,
                                                cost,
                                            )
                                            .await
                                        {
                                            tracing::warn!("Failed to update AI usage stats: {}", e);
                                        } else {
                                            tracing::info!(
                                                "Updated AI usage: provider={}, model={}, input={}, output={}, cost=${:.4}",
                                                provider, model, input_tokens, output_tokens, cost
                                            );
                                        }
                                    });
                                }
                            }
                        }
                        StreamContent::Done => {
                            tracing::info!("Stream completed - execution_id: {}", execution_id);
                            // Always clear the segment buffer. Persisting the final segment here will duplicate the
                            // assistant message because we also persist the final response in `save_assistant_message`.
                            // The final response includes tool_calls metadata and is the canonical persisted message.
                            let _ = segment_buf
                                .lock()
                                .map(|mut g| std::mem::take(&mut *g))
                                .unwrap_or_default();
                        }
                    }
                    if loop_break_flag.load(Ordering::SeqCst) {
                        return false;
                    }
                    if skill_reload_requested.load(Ordering::SeqCst) {
                        return false;
                    }
                    true
                },
            )
            .await;

        let digests_to_flush = pending_tool_digests
            .lock()
            .map(|mut queue| std::mem::take(&mut *queue))
            .unwrap_or_default();
        if let Err(e) = append_tool_digests(
            app_handle,
            &params.execution_id,
            digests_to_flush,
            &context_policy_for_stream,
        )
        .await
        {
            tracing::warn!("Failed to flush tool digests: {}", e);
        }

        if skill_reload_requested.load(Ordering::SeqCst) {
            if skill_reload_count >= max_skill_reload {
                skill_reload_requested.store(false, Ordering::SeqCst);
            } else {
                skill_reload_count += 1;

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

                if let Ok(mut pending_map) = pending.lock() {
                    pending_map.clear();
                }

                let skill_id = if let Ok(mut slot) = loaded_skill_id.lock() {
                    slot.take()
                } else {
                    None
                };
                if let Some(skill_id) = skill_id {
                    if let Some(db) =
                        app_handle.try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
                    {
                        if let Ok(Some(skill)) = db.get_skill(&skill_id).await {
                            let mut next_tools = vec![
                                "skills".to_string(),
                                "todos".to_string(),
                                "http_request".to_string(),
                                "subagent_execute".to_string(),
                                "subagent_await".to_string(),
                                "subagent_channel".to_string(),
                                "tenth_man_review".to_string(),
                            ];
                            if !tool_config
                                .disabled_tools
                                .contains(&ShellTool::NAME.to_string())
                            {
                                next_tools.push(ShellTool::NAME.to_string());
                            }
                            next_tools.extend(skill.allowed_tools.clone());
                            next_tools.extend(tool_config.fixed_tools.clone());
                            let available_tools = tool_server
                                .list_tools()
                                .await
                                .into_iter()
                                .map(|t| t.name)
                                .collect::<std::collections::HashSet<_>>();
                            let mut seen = std::collections::HashSet::new();
                            next_tools.retain(|id| seen.insert(id.clone()));
                            next_tools.retain(|id| available_tools.contains(id));
                            next_tools.retain(|id| !tool_config.disabled_tools.contains(id));
                            current_tool_ids = apply_allowed_tools_policy(
                                next_tools.clone(),
                                &tool_config.allowed_tools,
                            );
                            let _ = app_handle.emit(
                                "agent:tools_selected",
                                &json!({
                                    "execution_id": params.execution_id,
                                    "tools": current_tool_ids,
                                }),
                            );
                            let _ = app_handle.emit(
                                "agent:skill_loaded",
                                &json!({
                                    "execution_id": params.execution_id,
                                    "skill_id": skill.id,
                                    "skill_name": skill.name,
                                    "tools": current_tool_ids,
                                }),
                            );

                            if let Some(db) = db_for_stream.clone() {
                                use sentinel_core::models::database as core_db;
                                let tools_preview = {
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
                                };
                                let meta = json!({
                                    "kind": "skill_loaded",
                                    "skill_id": skill.id,
                                    "skill_name": skill.name,
                                    "tools": current_tool_ids,
                                    "tools_preview": tools_preview,
                                });
                                let msg = core_db::AiMessage {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    conversation_id: params.execution_id.clone(),
                                    role: "system".to_string(),
                                    content: format!("Skill loaded: {} ({})", skill.name, skill.id),
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
                                let db_clone = db;
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) = db_clone.upsert_ai_message_append(&msg).await {
                                        tracing::warn!(
                                            "Failed to persist skill_loaded message: {}",
                                            e
                                        );
                                    }
                                });
                            }
                        }
                    }
                }

                skill_reload_requested.store(false, Ordering::SeqCst);
                force_history_with_tools = true;
                continue;
            }
        }

        if loop_break_requested.load(Ordering::SeqCst) {
            loop_break_requested.store(false, Ordering::SeqCst);
            let loop_err = anyhow::anyhow!(
                "Detected repeated identical tool loop; retrying with loop-guard context"
            );
            if retries < max_retries {
                retries += 1;
                last_error = Some(loop_err);

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
                if let Ok(mut p) = pending.lock() {
                    p.clear();
                }
                if let Ok(mut tc) = tool_calls_collector.lock() {
                    tc.clear();
                }
                if let Ok(mut fp) = last_tool_fingerprint.lock() {
                    *fp = None;
                }
                repeated_tool_fingerprint_count.store(0, Ordering::SeqCst);
                force_history_with_tools = true;
                continue;
            }
            return Err(loop_err);
        }

        match result {
            Ok(response) => {
                // 合并最终输出和累积的输出
                let full_response = if let Ok(acc) = accumulated_assistant_output.lock() {
                    if !acc.is_empty() && !response.is_empty() {
                        format!("{}\n\n{}", acc, response)
                    } else if !acc.is_empty() {
                        acc.clone()
                    } else {
                        response.clone()
                    }
                } else {
                    response.clone()
                };

                // If assistant text segments were already persisted at tool-call boundaries,
                // only save the last turn's response text to avoid duplicating earlier segments
                // in the database. The full_response is still used for memory/logging.
                let seg_count = persisted_segment_count.load(Ordering::SeqCst);
                let final_response = if seg_count > 0 && !response.is_empty() {
                    tracing::info!(
                        "Segments already persisted: {}, saving only last turn response ({} chars) instead of full ({} chars)",
                        seg_count, response.len(), full_response.len()
                    );
                    response.clone()
                } else {
                    full_response.clone()
                };

                tracing::info!(
                    "Agent with tools completed - execution_id: {}, final_save_length: {}, full_response_length: {}, persisted_segments: {}",
                    params.execution_id,
                    final_response.len(),
                    full_response.len(),
                    seg_count
                );

                // 合并所有工具调用记录（包括累积的和当前的）
                let mut all_tool_calls = Vec::new();
                if let Ok(acc_calls) = accumulated_tool_calls.lock() {
                    all_tool_calls.extend(acc_calls.clone());
                }
                if let Ok(current_calls) = tool_calls_collector.lock() {
                    all_tool_calls.extend(current_calls.clone());
                }

                let tool_summaries = all_tool_calls
                    .iter()
                    .map(|call| ToolCallSummary {
                        name: call.name.clone(),
                        success: call.success,
                        duration_ms: Some(call.duration_ms),
                    })
                    .collect::<Vec<_>>();

                tracing::info!(
                    "Total tool calls completed: {} (accumulated: {}, current: {})",
                    tool_summaries.len(),
                    accumulated_tool_calls.lock().map(|c| c.len()).unwrap_or(0),
                    tool_calls_collector.lock().map(|c| c.len()).unwrap_or(0)
                );

                let tool_names_for_reflection: Vec<String> = all_tool_calls
                    .iter()
                    .map(|c| c.name.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();

                if let Err(e) = get_global_memory()
                    .record_execution(ExecutionRecord {
                        id: params.execution_id.clone(),
                        task: params.task.clone(),
                        environment: Some(rig_provider.clone()),
                        tool_calls: tool_summaries,
                        success: true,
                        error: None,
                        response_excerpt: Some(truncate_for_memory(&full_response, 400)),
                        created_at: chrono::Utc::now().timestamp(),
                    })
                    .await
                {
                    tracing::warn!("Failed to store memory record: {}", e);
                }

                record_execution_reflection(
                    app_handle,
                    &ExecutionOutcome {
                        execution_id: params.execution_id.clone(),
                        task: params.task.clone(),
                        success: true,
                        error: None,
                        tool_names_used: tool_names_for_reflection,
                        response_excerpt: Some(truncate_for_memory(&full_response, 200)),
                    },
                )
                .await;

                let reasoning_content = reasoning_content_buf
                    .lock()
                    .ok()
                    .map(|s| s.clone())
                    .filter(|s| !s.trim().is_empty());

                let tool_calls_slice = if all_tool_calls.is_empty() {
                    None
                } else {
                    Some(all_tool_calls.as_slice())
                };

                save_assistant_message(
                    app_handle,
                    &params.execution_id,
                    &final_response,
                    tool_calls_slice,
                    reasoning_content,
                    params.persist_messages,
                    params.subagent_run_id.as_deref(),
                )
                .await;

                // Tenth Man Rule: Adversarial Review (System-enforced final check)
                if params.enable_tenth_man_rule {
                    let tenth_man = TenthMan::new(&params);

                    // Check if we should run final review based on mode
                    let should_run_final = if let Some(ref config) = params.tenth_man_config {
                        match &config.mode {
                            InterventionMode::SystemOnly => true,
                            InterventionMode::Hybrid {
                                force_final_review, ..
                            } => *force_final_review,
                            InterventionMode::ToolOnly => false,
                            _ => true, // Legacy modes default to true
                        }
                    } else {
                        true // Default: run final review
                    };

                    if should_run_final {
                        tracing::info!(
                            "Running Tenth Man final review with full history for execution_id: {}",
                            params.execution_id
                        );

                        match tenth_man.review_with_history(&params.execution_id).await {
                            Ok(critique) => {
                                tracing::info!(
                                    "Tenth Man Critique generated ({} chars)",
                                    critique.len()
                                );

                                if params.persist_messages {
                                    if let Some(db) = db_for_stream.clone() {
                                        use sentinel_core::models::database as core_db;
                                        let review_msg = core_db::AiMessage {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            conversation_id: params.execution_id.clone(),
                                            role: "system".to_string(),
                                            content: critique.clone(),
                                            metadata: Some(
                                                json!({
                                                    "kind": "tenth_man_critique",
                                                    "trigger": "final_review",
                                                    "mode": "system_enforced"
                                                })
                                                .to_string(),
                                            ),
                                            token_count: Some(critique.len() as i32),
                                            cost: None,
                                            tool_calls: None,
                                            attachments: None,
                                            reasoning_content: None,
                                            timestamp: chrono::Utc::now(),
                                            architecture_type: None,
                                            architecture_meta: None,
                                            structured_data: None,
                                        };

                                        if let Err(e) = db.create_ai_message(&review_msg).await {
                                            tracing::warn!(
                                                "Failed to save Tenth Man critique: {}",
                                                e
                                            );
                                        }

                                        // Emit event to frontend
                                        let _ = app.emit(
                                            "agent:tenth_man_critique",
                                            &json!({
                                                "execution_id": params.execution_id,
                                                "critique": critique,
                                                "message_id": review_msg.id,
                                                "trigger": "final_review",
                                                "mode": "system_enforced"
                                            }),
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Tenth Man Review failed: {}", e);
                            }
                        }
                    } else {
                        tracing::info!("Skipping final Tenth Man review (mode: ToolOnly)");
                    }
                }

                // Cleanup Tenth Man execution context
                if params.enable_tenth_man_rule {
                    use crate::agents::tenth_man_executor;
                    tenth_man_executor::clear_tenth_man_execution(&params.execution_id).await;
                }

                // Cleanup container context files (keep history.txt)
                cleanup_container_context_async(&app, &params.execution_id).await;

                if let Some(ctx) = team_stream_context.as_ref() {
                    let _ = app.emit(
                        "agent_team:message_stream_done",
                        &json!({
                            "session_id": ctx.session_id.clone(),
                            "stream_id": ctx.stream_id.clone(),
                            "member_id": ctx.member_id.clone(),
                            "member_name": ctx.member_id.clone(),
                            "phase": ctx.phase.clone(),
                            "content": final_response.clone(),
                            "had_delta": team_stream_had_delta.load(Ordering::SeqCst),
                        }),
                    );
                }

                return Ok(final_response);
            }
            Err(e) => {
                let err_msg = e.to_string();

                // Cleanup container context files even on error
                cleanup_container_context_async(&app, &params.execution_id).await;

                // 优化错误消息
                let friendly_err = if err_msg.contains("error decoding response body") {
                    if err_msg.contains("UnexpectedEof") || err_msg.contains("unexpected EOF") {
                        anyhow::anyhow!("LLM provider connection closed unexpectedly. This might be a temporary issue with the provider or proxy. (Original error: {})", err_msg)
                    } else {
                        anyhow::anyhow!("Failed to decode LLM response. The provider may have returned an invalid format. (Original error: {})", err_msg)
                    }
                } else {
                    e
                };

                // 检查是否是可重试的错误（主要是解析错误和网络抖动）
                let is_retryable = is_retryable_error(&err_msg);
                let has_tool_activity = tool_calls_collector
                    .lock()
                    .map(|calls| !calls.is_empty())
                    .unwrap_or(false)
                    || pending_calls
                        .lock()
                        .map(|pending| !pending.is_empty())
                        .unwrap_or(false);

                if is_retryable && !has_tool_activity && retries < max_retries {
                    retries += 1;
                    last_error = Some(friendly_err);

                    // 保存当前工作到累积记录中（在清理之前）
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

                    // 清理重试前的临时状态（但不清理累积记录）
                    if let Ok(mut buf) = assistant_segment_buf.lock() {
                        buf.clear();
                    }
                    if let Ok(mut buf) = reasoning_content_buf.lock() {
                        buf.clear();
                    }
                    if let Ok(mut p) = pending_calls.lock() {
                        p.clear();
                    }
                    if let Ok(mut tc) = tool_calls_collector.lock() {
                        tc.clear();
                    }

                    continue;
                } else {
                    // Final failure recording
                    let tool_summaries = tool_calls_collector
                        .lock()
                        .map(|calls| {
                            calls
                                .iter()
                                .map(|call| ToolCallSummary {
                                    name: call.name.clone(),
                                    success: call.success,
                                    duration_ms: Some(call.duration_ms),
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    let fail_tool_names: Vec<String> = tool_summaries
                        .iter()
                        .map(|t| t.name.clone())
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .collect();
                    let err_msg_clone = err_msg.clone();

                    if let Err(err) = get_global_memory()
                        .record_execution(ExecutionRecord {
                            id: params.execution_id.clone(),
                            task: params.task.clone(),
                            environment: Some(rig_provider.clone()),
                            tool_calls: tool_summaries,
                            success: false,
                            error: Some(err_msg),
                            response_excerpt: None,
                            created_at: chrono::Utc::now().timestamp(),
                        })
                        .await
                    {
                        tracing::warn!("Failed to store memory record: {}", err);
                    }

                    record_execution_reflection(
                        app_handle,
                        &ExecutionOutcome {
                            execution_id: params.execution_id.clone(),
                            task: params.task.clone(),
                            success: false,
                            error: Some(err_msg_clone),
                            tool_names_used: fail_tool_names,
                            response_excerpt: None,
                        },
                    )
                    .await;

                    if let Some(ctx) = team_stream_context.as_ref() {
                        let _ = app.emit(
                            "agent_team:message_stream_done",
                            &json!({
                                "session_id": ctx.session_id.clone(),
                                "stream_id": ctx.stream_id.clone(),
                                "member_id": ctx.member_id.clone(),
                                "member_name": ctx.member_id.clone(),
                                "phase": ctx.phase.clone(),
                                "error": friendly_err.to_string(),
                                "had_delta": team_stream_had_delta.load(Ordering::SeqCst),
                            }),
                        );
                    }

                    return Err(friendly_err);
                }
            }
        }
    }

    let final_error = last_error.unwrap_or_else(|| anyhow::anyhow!("Max retries reached"));
    if let Some(ctx) = team_stream_context.as_ref() {
        let _ = app.emit(
            "agent_team:message_stream_done",
            &json!({
                "session_id": ctx.session_id.clone(),
                "stream_id": ctx.stream_id.clone(),
                "member_id": ctx.member_id.clone(),
                "member_name": ctx.member_id.clone(),
                "phase": ctx.phase.clone(),
                "error": final_error.to_string(),
                "had_delta": team_stream_had_delta.load(Ordering::SeqCst),
            }),
        );
    }
    Err(final_error)
}

fn is_retryable_error(err_msg: &str) -> bool {
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

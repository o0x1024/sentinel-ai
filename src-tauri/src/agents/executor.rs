//! Agent Executor - 使用 sentinel-llm 和 ToolServer 执行 agent 任务
//!
//! 支持工具调用、流式输出、多轮对话。

use anyhow::Result;
use sentinel_db::Database;
use sentinel_llm::{LlmConfig, StreamContent, StreamingLlmClient, parse_image_from_json};
use sentinel_memory::{get_global_memory, ExecutionRecord, ToolCallSummary};
use sentinel_tools::{get_tool_server, mcp_adapter, ToolServer};
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

use super::tool_router::{ToolConfig, ToolRouter};
use super::tenth_man::{TenthMan, TenthManConfig, InterventionContext, TriggerReason};
use super::sliding_window::{SlidingWindowManager, SlidingWindowConfig};

/// Document attachment info for prompt injection
#[derive(Debug, Clone)]
pub struct DocumentAttachmentInfo {
    pub id: String,
    pub original_filename: String,
    pub file_size: u64,
    pub mime_type: String,
    pub processing_mode: String,
    pub extracted_text: Option<String>,
    pub container_path: Option<String>,
}

/// Agent 执行配置
#[derive(Debug, Clone)]
pub struct AgentExecuteParams {
    pub execution_id: String,
    pub model: String,
    pub system_prompt: String,
    pub task: String,
    pub rig_provider: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub max_iterations: usize,
    pub timeout_secs: u64,
    pub tool_config: Option<ToolConfig>,
    pub enable_tenth_man_rule: bool,
    pub tenth_man_config: Option<TenthManConfig>,
    pub document_attachments: Option<Vec<DocumentAttachmentInfo>>,
    pub image_attachments: Option<serde_json::Value>,
}

/// 执行 agent 任务
///
/// 使用 sentinel-llm 的 StreamingLlmClient 处理所有 provider，
/// 通过 Tauri 事件将流式响应发送给前端。
pub async fn execute_agent(app_handle: &AppHandle, params: AgentExecuteParams) -> Result<String> {
    let rig_provider = params.rig_provider.to_lowercase();

    tracing::info!(
        "Executing agent - rig_provider: {}, model: {}, execution_id: {}, tools_enabled: {}",
        rig_provider,
        params.model,
        params.execution_id,
        params
            .tool_config
            .as_ref()
            .map(|c| c.enabled)
            .unwrap_or(false)
    );

    if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        if let Ok(client) = db.get_db() {
            get_global_memory().set_database_client(client).await;
        }
        
        // Load Tavily API key from database and set it globally
        if let Ok(api_key) = db.get_config("ai", "tavily_api_key").await {
            sentinel_tools::tool_server::set_tavily_api_key(api_key).await;
        }
    }

    // 初始化全局工具服务器
    let tool_server = get_tool_server();
    tool_server.init_builtin_tools().await;

    // 设置 todos 的 AppHandle 以便发射事件
    use sentinel_tools::buildin_tools::todos::set_todos_app_handle;
    set_todos_app_handle(app_handle.clone()).await;
    
    // 始终初始化 Tenth Man 配置（工具默认开启，可能随时被调用）
    use crate::agents::tenth_man_executor;
    
    // Build LLM config for Tenth Man
    let mut tenth_man_llm_config = LlmConfig::new(&rig_provider, &params.model)
        .with_timeout(params.timeout_secs)
        .with_rig_provider(&rig_provider);
    
    if let Some(ref api_key) = params.api_key {
        tenth_man_llm_config = tenth_man_llm_config.with_api_key(api_key);
    }
    if let Some(ref api_base) = params.api_base {
        tenth_man_llm_config = tenth_man_llm_config.with_base_url(api_base);
    }
    
    tenth_man_executor::set_tenth_man_config(params.execution_id.clone(), tenth_man_llm_config).await;
    tenth_man_executor::set_task_context(params.execution_id.clone(), params.task.clone()).await;
    
    tracing::info!(
        "Tenth Man initialized for execution_id: {} (rule_enabled: {})", 
        params.execution_id,
        params.enable_tenth_man_rule
    );

    // 检查是否启用工具
    let tool_config = params.tool_config.clone().unwrap_or_default();

    if tool_config.enabled {
        // 刷新 MCP 工具以确保它们已注册到 ToolServer
        tracing::info!("Refreshing MCP tools before execution...");
        mcp_adapter::refresh_mcp_tools(&tool_server).await;

        // Register VisionExplorerV2Tool if enabled
        if tool_config.enabled && !tool_config.disabled_tools.contains(&"vision_explorer".to_string()) {
           if let Some(_mcp_service) = app_handle.try_state::<std::sync::Arc<crate::services::mcp::McpService>>() {
                use crate::engines::vision_explorer_v2::VisionExplorerV2Tool;
                use sentinel_tools::dynamic_tool::{DynamicToolBuilder, ToolSource};
                use rig::tool::Tool;

                let rig_provider = params.rig_provider.to_lowercase();
                let mut llm_config = sentinel_llm::LlmConfig::new(&rig_provider, &params.model)
                   .with_timeout(params.timeout_secs)
                   .with_rig_provider(&rig_provider);
                
                // Set api_key and base_url for VisionExplorer V2
                if let Some(ref api_key) = params.api_key {
                    llm_config = llm_config.with_api_key(api_key);
                }
                if let Some(ref api_base) = params.api_base {
                    llm_config = llm_config.with_base_url(api_base);
                }

                let ve_tool = VisionExplorerV2Tool::new(llm_config)
                    .with_app_handle(app_handle.clone())
                    .with_execution_id(params.execution_id.clone());
                
                // Get definition
                let def = ve_tool.definition(String::new()).await;
                
                let tool_def = DynamicToolBuilder::new(def.name)
                   .description(def.description)
                   .input_schema(def.parameters)
                   .source(ToolSource::Builtin)
                   .executor(move |args| {
                       let tool = ve_tool.clone();
                       async move {
                           // Deserialize args
                           let tool_args: crate::engines::vision_explorer_v2::tool::VisionExplorerV2Args = 
                               serde_json::from_value(args).map_err(|e| e.to_string())?;
                           
                           let result = tool.call(tool_args).await
                               .map_err(|e| e.to_string())?;
                           
                           Ok(serde_json::Value::String(result))
                       }
                   })
                   .build();
                
                if let Ok(tool_def) = tool_def {
                    tool_server.register_tool(tool_def).await;
                    tracing::info!("Registered VisionExplorerV2Tool");
                } else if let Err(e) = tool_def {
                     tracing::warn!("Failed to build VisionExplorerV2Tool definition: {}", e);
                }
           } else {
               tracing::warn!("McpService not found, skipping VisionExplorerV2Tool registration");
           }
        }

        // 打印当前注册的工具列表
        let registered_tools = tool_server.list_tools().await;
        tracing::info!(
            "ToolServer has {} registered tools: {:?}",
            registered_tools.len(),
            registered_tools.iter().map(|t| &t.name).collect::<Vec<_>>()
        );

        // 使用工具增强的 Agent
        execute_agent_with_tools(app_handle, params, &tool_server).await
    } else {
        // 简单的 LLM 调用（无工具）
        execute_agent_simple(app_handle, params).await
    }
}

/// 简单的 Agent 执行（无工具调用）
async fn execute_agent_simple(
    app_handle: &AppHandle,
    params: AgentExecuteParams,
) -> Result<String> {
    let rig_provider = params.rig_provider.to_lowercase();

    // 构建 LlmConfig
    let mut config = LlmConfig::new(&rig_provider, &params.model)
        .with_timeout(params.timeout_secs)
        .with_rig_provider(&rig_provider);

    if let Some(ref api_key) = params.api_key {
        config = config.with_api_key(api_key);
    }

    if let Some(ref api_base) = params.api_base {
        config = config.with_base_url(api_base);
    }

    let system_prompt = params.system_prompt.clone();

    // 创建流式客户端
    let client = StreamingLlmClient::new(config);

    // 准备事件发送
    let execution_id = params.execution_id.clone();
    let app = app_handle.clone();

    // 执行流式调用
    let result = client
        .stream_completion(
            Some(&system_prompt),
            &params.task,
            |content| {
                if crate::commands::ai::is_conversation_cancelled(&execution_id) {
                    return false;
                }
                match content {
                    StreamContent::Text(text) => {
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
                        let _ = app.emit(
                            "agent:tool_call_complete",
                            &json!({
                                "execution_id": execution_id,
                                "tool_call_id": id,
                                "tool_name": name,
                                "arguments": arguments,
                            }),
                        );
                    }
                    StreamContent::ToolResult { id, result } => {
                        let _ = app.emit(
                            "agent:tool_result",
                            &json!({
                                "execution_id": execution_id,
                                "tool_call_id": id,
                                "result": result,
                            }),
                        );
                    }
                    StreamContent::Usage { input_tokens, output_tokens } => {
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
                            if let Some(db) = app.try_state::<std::sync::Arc<sentinel_db::DatabaseService>>() {
                                let provider = params.rig_provider.clone();
                                let model = params.model.clone();
                                let cost = sentinel_llm::calculate_cost(&provider, &model, input_tokens, output_tokens);
                                
                                let db_clone = db.inner().clone();
                                tokio::spawn(async move {
                                    if let Err(e) = db_clone.update_ai_usage(&provider, &model, input_tokens as i32, output_tokens as i32, cost).await {
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
                        tracing::info!("Agent completed - execution_id: {}", execution_id);
                    }
                }
                true
            },
        )
        .await;

    match result {
        Ok(response) => {
            tracing::info!(
                "Agent execution successful - execution_id: {}, response_length: {}",
                params.execution_id,
                response.len()
            );

            // 保存助手消息到数据库（无工具调用）
            save_assistant_message(app_handle, &params.execution_id, &response, None, None).await;

            // Legacy memory recording removed. Agent now consciously stores memories via tools.

            // Cleanup container context files
            cleanup_container_context_async(&params.execution_id).await;

            Ok(response)
        }
        Err(e) => {
            tracing::error!(
                "Agent execution failed - execution_id: {}, error: {}",
                params.execution_id,
                e
            );
            
            // Cleanup container context files even on error
            cleanup_container_context_async(&params.execution_id).await;
            
            // Legacy memory recording removed.
            Err(e)
        }
    }
}

/// 保存助手消息到数据库并发送事件
async fn save_assistant_message(
    app_handle: &AppHandle, 
    conversation_id: &str, 
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
        
        // 将工具调用转换为 JSON 字符串
        let tool_calls_json = tool_calls.map(|tc| serde_json::to_string(tc).unwrap_or_default());
        
        let msg = core_db::AiMessage {
            id: message_id.clone(),
            conversation_id: conversation_id.to_string(),
            role: "assistant".to_string(),
            content: content.to_string(),
            metadata: None,
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

            // 发送助手消息保存成功事件到前端
            let _ = app_handle.emit(
                "agent:assistant_message_saved",
                &json!({
                    "execution_id": conversation_id,
                    "message_id": message_id,
                    "content": content,
                    "timestamp": msg.timestamp.timestamp_millis(),
                    "tool_calls": tool_calls,
                }),
            );
        }
    }
}

/// 工具调用记录（用于持久化）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCallRecord {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub result: Option<String>,
    pub success: bool,
    /// Sequence number within one agent execution (0-based).
    pub sequence: u32,
    /// Tool call started timestamp (UTC millis).
    pub started_at_ms: i64,
    /// Tool call completed timestamp (UTC millis).
    pub completed_at_ms: i64,
    /// Tool call duration in millis.
    pub duration_ms: i64,
}

/// 带工具调用的 Agent 执行（使用 rig-core 原生工具调用）
async fn execute_agent_with_tools(
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
        .with_rig_provider(&rig_provider);

    if let Some(ref api_key) = params.api_key {
        llm_config = llm_config.with_api_key(api_key);
    }

    if let Some(ref api_base) = params.api_base {
        llm_config = llm_config.with_base_url(api_base);
    }

    // Use plan_tools to support Ability mode with injected context
    let db_pool = db_service.get_pool().ok();
    let selection_plan = tool_router
        .plan_tools(&params.task, &tool_config, Some(&llm_config), db_pool)
        .await?;

    let selected_tool_ids = selection_plan.tool_ids.clone();
    
    tracing::info!(
        "Selected {} tools for execution_id {}: {:?}",
        selected_tool_ids.len(),
        params.execution_id,
        selected_tool_ids
    );

    // Emit ability_selected event if applicable
    if let Some(ref group) = selection_plan.selected_ability_group {
        let _ = app_handle.emit(
            "agent:ability_selected",
            &json!({
                "execution_id": params.execution_id,
                "group_id": group.id,
                "group_name": group.name,
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
    let dynamic_tools = tool_server.get_dynamic_tools(&selected_tool_ids).await;

    tracing::info!(
        "Got {} dynamic tool instances for rig-core native tool calling",
        dynamic_tools.len()
    );

    // 4. Build final system prompt (with injected Ability instructions if any)
    let mut final_system_prompt = if let Some(ref injected) = selection_plan.injected_system_prompt {
        format!("{}{}", params.system_prompt, injected)
    } else {
        params.system_prompt.clone()
    };

    // Inject execution_id for todos tool
    final_system_prompt.push_str(&format!(
        "\n\n[SystemContext: Current Execution ID is '{}'. Use this for todos tool calls.]",
        params.execution_id
    ));

    // Inject working directory if configured
    if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        if let Ok(Some(working_dir)) = db.get_config("ai", "working_directory").await {
            if !working_dir.is_empty() {
                final_system_prompt.push_str(&format!(
                    "\n\n[Working Directory: Your working directory is '{}'. When performing file operations, executing scripts, or any file system related tasks, use this directory as your base path unless explicitly specified otherwise by the user.]",
                    working_dir
                ));
                tracing::info!("Injected working directory into system prompt: {}", working_dir);
            }
        }
    }

    // Inject Docker container context directory information
    final_system_prompt.push_str(&format!(
        "\n\n[Context Storage]: All large tool outputs are automatically stored in Docker container at '{}'.\n\
        - Tool outputs exceeding threshold are saved as files (not truncated)\n\
        - Applies to: shell commands, HTTP responses, and other tools\n\
        - Your conversation history is at '{}/history.txt'\n\
        - Use shell tool with grep/tail/head/cat to access these files\n\
        \n\
        Examples:\n\
        • shell(command=\"ls -lh {}\")  (list all stored files)\n\
        • shell(command=\"grep -i 'pattern' {}/*.txt\")  (search across files)\n\
        • shell(command=\"tail -n 100 {}/http_response_*.txt\")  (view HTTP output)\n\
        • shell(command=\"cat {}/history.txt | grep 'keyword'\")  (search history)\n\
        \n\
        All files are centralized in one directory for easy management and search.",
        sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR,
        sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR,
        sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR,
        sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR,
        sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR,
        sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR
    ));

    // Inject document attachments context
    if let Some(ref doc_attachments) = params.document_attachments {
        if !doc_attachments.is_empty() {
            let doc_context = build_document_attachments_context(doc_attachments);
            final_system_prompt.push_str(&doc_context);
            tracing::info!("Injected {} document attachment(s) into system prompt", doc_attachments.len());
        }
    }

    // 5. 使用 SlidingWindowManager 管理上下文和历史
    let max_context_length = get_provider_max_context_length(app_handle, &rig_provider).await.unwrap_or(128000) as usize;
    
    let sw_config = SlidingWindowConfig {
        max_context_tokens: max_context_length,
        ..Default::default()
    };
    
    let mut sliding_window = match SlidingWindowManager::new(
        app_handle, 
        &params.execution_id, 
        Some(sw_config)
    ).await {
        Ok(sw) => sw,
        Err(e) => {
            tracing::warn!("Failed to init SlidingWindowManager: {}", e);
            return Err(e.into());
        }
    };
    
    // 尝试压缩历史
    if let Err(e) = sliding_window.compress_if_needed(&llm_config).await {
        tracing::warn!("Sliding window compression failed: {}", e);
    }
    
    // Export history to container (for方案3: 对话历史按需检索)
    if let Ok(history_content) = sliding_window.export_history().await {
        // Get Docker sandbox config and create sandbox to store history
        use sentinel_tools::shell::get_shell_config;
        let shell_config = get_shell_config().await;
        if let Some(docker_config) = shell_config.docker_config {
            let sandbox = sentinel_tools::DockerSandbox::new(docker_config);
            if let Err(e) = sentinel_tools::store_history_in_container(&sandbox, &history_content).await {
                tracing::warn!("Failed to store history in container: {}", e);
            } else {
                tracing::info!("Conversation history exported to container: {}/history.txt", 
                    sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR);
            }
        }
    }
    
    // 构建上下文
    let context_messages = sliding_window.build_context(&final_system_prompt);
    
    // 分离 System Prompt 和 历史消息
    // SlidingWindow 将全局摘要和段落摘要放在第一个 System 消息中
    let (final_system_prompt_content, mut history_chat_messages) = if !context_messages.is_empty() && context_messages[0].role == "system" {
        (Some(context_messages[0].content.clone()), context_messages[1..].to_vec())
    } else {
        (Some(final_system_prompt), context_messages)
    };

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
    let app = app_handle.clone();
    let db_for_stream: Option<std::sync::Arc<sentinel_db::DatabaseService>> = app_handle
        .try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
        .map(|s| s.inner().clone());

    // 用于收集工具调用信息
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
    let tool_calls_collector: Arc<Mutex<Vec<ToolCallRecord>>> = Arc::new(Mutex::new(Vec::new()));
    let pending_calls: Arc<Mutex<std::collections::HashMap<String, (String, String, i64, u32)>>> =
        Arc::new(Mutex::new(std::collections::HashMap::new()));
    let tool_seq: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));
    let tool_call_counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let assistant_segment_buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let reasoning_content_buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    
    let collector = tool_calls_collector.clone();
    let pending = pending_calls.clone();
    let seq_counter = tool_seq.clone();
    let tool_counter = tool_call_counter.clone();
    let segment_buf = assistant_segment_buf.clone();
    let reasoning_buf = reasoning_content_buf.clone();

    // 7. 调用带动态工具的流式方法，增加重试机制以应对模型抖动或解析错误
    let mut retries = 0;
    let max_retries = 2; // 最多重试 2 次
    let mut last_error: Option<anyhow::Error> = None;
    
    // 累积的工具调用记录（跨重试保留）
    let accumulated_tool_calls: Arc<Mutex<Vec<ToolCallRecord>>> = Arc::new(Mutex::new(Vec::new()));
    // 累积的助手输出（跨重试保留）
    let accumulated_assistant_output: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));

    while retries <= max_retries {
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

        let result = client
            .stream_chat_with_dynamic_tools(
                final_system_prompt_content.as_deref(),
                &params.task,
                &history_chat_messages,
                image_attachment.as_ref(), // 传递图片附件
                dynamic_tools.clone(),
                |content| {
                    if crate::commands::ai::is_conversation_cancelled(&execution_id) {
                        return false;
                    }
                    match content {
                        StreamContent::Text(text) => {
                            // Accumulate assistant text into a segment buffer.
                            let full_content = if let Ok(mut buf) = segment_buf.lock() {
                                buf.push_str(&text);
                                buf.clone()
                            } else {
                                String::new()
                            };
                            
                            // Tenth Man Intervention Point 2: Conclusion Detection
                            if params.enable_tenth_man_rule && !full_content.is_empty() {
                                if let Some(ref _tm_config) = params.tenth_man_config {
                                    // Check if content contains conclusion markers
                                    if TenthMan::contains_conclusion_markers(&full_content) {
                                        let tenth_man = TenthMan::new(&params);
                                        let current_count = tool_counter.load(Ordering::SeqCst) as usize;
                                        
                                        let context = InterventionContext {
                                            execution_id: execution_id.clone(),
                                            task: params.task.clone(),
                                            tool_call_count: current_count,
                                            current_content: Some(full_content.clone()),
                                            trigger_reason: TriggerReason::ConclusionDetected,
                                        };
                                        
                                        if tenth_man.should_trigger(&context) {
                                            let app_clone = app.clone();
                                            let exec_id = execution_id.clone();
                                            let task_clone = params.task.clone();
                                            
                                            tauri::async_runtime::spawn(async move {
                                                match tenth_man.review(
                                                    &task_clone,
                                                    "Conclusion detected in assistant response",
                                                    &full_content
                                                ).await {
                                                    Ok(critique) => {
                                                        tracing::info!("Tenth Man intervention on conclusion detection");
                                                        let _ = app_clone.emit(
                                                            "agent:tenth_man_intervention",
                                                            &json!({
                                                                "execution_id": exec_id,
                                                                "trigger": "conclusion_detected",
                                                                "critique": critique,
                                                                "timestamp": chrono::Utc::now().timestamp_millis(),
                                                            })
                                                        );
                                                    }
                                                    Err(e) => {
                                                        tracing::warn!("Tenth Man review failed: {}", e);
                                                    }
                                                }
                                            });
                                        }
                                    }
                                }
                            }
                            
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
                                        if let Err(e) = db.upsert_ai_message_append(&seg_msg).await {
                                            tracing::warn!(
                                                "Failed to persist assistant segment: {}",
                                                e
                                            );
                                        }
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
                                    if let Err(e) = db.upsert_ai_message_append(&tool_msg).await {
                                        tracing::warn!("Failed to persist tool call message: {}", e);
                                    }
                                });
                            }

                            let _ = app.emit(
                                "agent:tool_call_complete",
                                &json!({
                                    "execution_id": execution_id,
                                    "tool_call_id": id,
                                    "tool_name": name,
                                    "arguments": arguments,
                                }),
                            );
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
                                        let meta = json!({
                                            "kind": "tool_call",
                                            "tool_name": name_for_meta,
                                            "tool_args": tool_args_val,
                                            "tool_call_id": id,
                                            "status": "completed",
                                            "sequence": seq,
                                            "started_at_ms": started_at_ms,
                                            "completed_at_ms": completed_at_ms,
                                            "duration_ms": duration_ms,
                                            "tool_result": result,
                                            "success": !result.to_lowercase().contains("error"),
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
                                            if let Err(e) =
                                                db.upsert_ai_message_append(&tool_msg).await
                                            {
                                                tracing::warn!(
                                                    "Failed to persist tool result update: {}",
                                                    e
                                                );
                                            }
                                        });
                                    }
                                }
                            }

                            let _ = app.emit(
                                "agent:tool_result",
                                &json!({
                                    "execution_id": execution_id,
                                    "tool_call_id": id,
                                    "result": result,
                                }),
                            );
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
                            // Flush any remaining assistant segment at the end (so history shows it after the last tool call).
                            if let Some(db) = db_for_stream.clone() {
                                use sentinel_core::models::database as core_db;
                                let seg = segment_buf
                                    .lock()
                                    .map(|mut g| std::mem::take(&mut *g))
                                    .unwrap_or_default();
                                let seg_trimmed = seg.trim().to_string();
                                if !seg_trimmed.trim().is_empty() {
                                    // Get reasoning content for final segment
                                    // 参考：https://api-docs.deepseek.com/zh-cn/guides/thinking_mode#tool-calls
                                    let reasoning = reasoning_buf
                                        .lock()
                                        .map(|mut g| {
                                            let r = std::mem::take(&mut *g);
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
                                        timestamp: chrono::Utc::now(),
                                        architecture_type: None,
                                        architecture_meta: None,
                                        structured_data: None,
                                    };
                                    tauri::async_runtime::spawn(async move {
                                        if let Err(e) = db.upsert_ai_message_append(&seg_msg).await {
                                            tracing::warn!(
                                                "Failed to persist final assistant segment: {}",
                                                e
                                            );
                                        }
                                    });
                                }
                            }
                        }
                    }
                    true
                },
            )
            .await;

        match result {
            Ok(response) => {
                // 合并最终输出和累积的输出
                let final_response = if let Ok(acc) = accumulated_assistant_output.lock() {
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
                
                tracing::info!(
                    "Agent with tools completed - execution_id: {}, response_length: {}, accumulated_length: {}",
                    params.execution_id,
                    final_response.len(),
                    accumulated_assistant_output.lock().map(|s| s.len()).unwrap_or(0)
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

                if let Err(e) = get_global_memory()
                    .record_execution(ExecutionRecord {
                        id: params.execution_id.clone(),
                        task: params.task.clone(),
                        environment: Some(rig_provider.clone()),
                        tool_calls: tool_summaries,
                        success: true,
                        error: None,
                        response_excerpt: Some(truncate_for_memory(&response, 400)),
                        created_at: chrono::Utc::now().timestamp(),
                    })
                    .await
                {
                    tracing::warn!("Failed to store memory record: {}", e);
                }

                // Tenth Man Rule: Adversarial Review (System-enforced final check)
                if params.enable_tenth_man_rule {
                    let tenth_man = TenthMan::new(&params);
                    
                    // Check if we should run final review based on mode
                    let should_run_final = if let Some(ref config) = params.tenth_man_config {
                        match &config.mode {
                            super::tenth_man::InterventionMode::SystemOnly => true,
                            super::tenth_man::InterventionMode::Hybrid { force_final_review, .. } => {
                                *force_final_review
                            }
                            super::tenth_man::InterventionMode::ToolOnly => false,
                            _ => true, // Legacy modes default to true
                        }
                    } else {
                        true // Default: run final review
                    };
                    
                    if should_run_final {
                        tracing::info!("Running Tenth Man final review with full history for execution_id: {}", params.execution_id);
                        
                        match tenth_man.review_with_history(&params.execution_id).await {
                            Ok(critique) => {
                                tracing::info!("Tenth Man Critique generated ({} chars)", critique.len());
                                
                                if let Some(db) = db_for_stream.clone() {
                                    use sentinel_core::models::database as core_db;
                                    let review_msg = core_db::AiMessage {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        conversation_id: params.execution_id.clone(),
                                        role: "system".to_string(), 
                                        content: critique.clone(),
                                        metadata: Some(json!({
                                            "kind": "tenth_man_critique",
                                            "trigger": "final_review",
                                            "mode": "system_enforced"
                                        }).to_string()),
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
                                        tracing::warn!("Failed to save Tenth Man critique: {}", e);
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
                                        })
                                    );
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
                cleanup_container_context_async(&params.execution_id).await;

                return Ok(final_response);
            }
            Err(e) => {
                let err_msg = e.to_string();
                
                // Cleanup container context files even on error
                cleanup_container_context_async(&params.execution_id).await;
                
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
                let is_retryable = !err_msg.is_empty();

                if is_retryable && retries < max_retries {
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
                    return Err(friendly_err);
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Max retries reached")))
}

fn truncate_for_memory(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    let mut out = text.chars().take(max_len).collect::<String>();
    out.push_str("...");
    out
}

/// Build document attachments context for system prompt injection
fn build_document_attachments_context(attachments: &[DocumentAttachmentInfo]) -> String {
    let mut context = String::new();
    
    // Separate content and security mode attachments
    let content_docs: Vec<_> = attachments.iter()
        .filter(|a| a.processing_mode == "content" && a.extracted_text.is_some())
        .collect();
    let security_docs: Vec<_> = attachments.iter()
        .filter(|a| a.processing_mode == "security" && a.container_path.is_some())
        .collect();
    
    // Content mode documents: include extracted text
    if !content_docs.is_empty() {
        context.push_str("\n\n[Document Content]\nThe user has attached the following document(s). The content has been extracted for your analysis:\n\n");
        
        for (i, doc) in content_docs.iter().enumerate() {
            context.push_str(&format!(
                "--- Document {} ---\n\
                Filename: {}\n\
                Size: {} bytes\n\
                Type: {}\n\n\
                Content:\n{}\n\n",
                i + 1,
                doc.original_filename,
                doc.file_size,
                doc.mime_type,
                doc.extracted_text.as_ref().unwrap_or(&String::new())
            ));
        }
        
        context.push_str("Please help the user with their request regarding the above document content.\n");
    }
    
    // Security mode documents: provide file paths and analysis instructions
    if !security_docs.is_empty() {
        context.push_str("\n\n[Security Analysis Task]\nThe user wants you to perform a security analysis on the following file(s):\n\n");
        
        for (i, doc) in security_docs.iter().enumerate() {
            context.push_str(&format!(
                "{}. {} ({})\n\
                   Path: {}\n\
                   Size: {} bytes\n",
                i + 1,
                doc.original_filename,
                doc.mime_type,
                doc.container_path.as_ref().unwrap_or(&String::new()),
                doc.file_size
            ));
        }
        
        context.push_str("\n\
Available security analysis tools in the Docker container:\n\
- olevba: Analyze VBA macros for malicious code (Office documents)\n\
- oleobj: Extract embedded objects from Office documents\n\
- exiftool: Examine metadata for suspicious information\n\
- file: Detect file type and verify file signatures\n\
- binwalk: Analyze binary files for embedded data\n\
- strings: Extract printable strings from binary files\n\
- xxd: Hex dump for binary analysis\n\
- pdftotext/pdfinfo: Analyze PDF documents\n\
- unzip/7z: Extract and inspect archive contents\n\n\
Recommended analysis workflow:\n\
1. Verify file type with `file <path>`\n\
2. Check for macros with `olevba <path>` (Office files)\n\
3. Examine metadata with `exiftool <path>`\n\
4. Look for embedded objects with `oleobj <path>`\n\
5. Check for hidden data with `binwalk <path>`\n\
6. Generate a comprehensive security risk report\n\n\
Use the shell tool to execute commands. Be thorough and systematic in your analysis.\n");
    }
    
    context
}

/// Get provider's max context length from database config
async fn get_provider_max_context_length(app_handle: &AppHandle, provider: &str) -> Result<u32> {
    if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        // Get providers config from database
        if let Ok(Some(config_str)) = db.get_config_internal("ai", "providers_config").await {
            if let Ok(providers) = serde_json::from_str::<std::collections::HashMap<String, serde_json::Value>>(&config_str) {
                // Find provider config (case-insensitive)
                for (key, value) in providers.iter() {
                    if key.to_lowercase() == provider.to_lowercase() {
                        if let Some(max_ctx) = value.get("max_context_length").and_then(|v| v.as_u64()) {
                            return Ok(max_ctx as u32);
                        }
                    }
                }
            }
        }
    }
    
    // Default fallback values based on provider
    let default = match provider.to_lowercase().as_str() {
        "openai" => 128000,      // GPT-4 Turbo
        "anthropic" => 200000,   // Claude 3
        "gemini" => 1000000,     // Gemini 1.5 Pro
        "deepseek" => 128000,    // DeepSeek (V3/R1 support 128k)
        "moonshot" => 128000,    // Moonshot
        "groq" => 32000,         // Groq
        "ollama" => 8192,        // Ollama (varies by model)
        "openrouter" => 128000,  // Varies by model
        _ => 128000,             // Safe default
    };
    
    Ok(default)
}


/// 工具调用结构
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ToolCall {
    tool: String,
    arguments: serde_json::Value,
}

/// 从响应中提取工具调用
#[allow(dead_code)]
fn extract_tool_call(response: &str) -> Option<ToolCall> {
    // 查找 JSON 代码块
    let json_pattern = regex::Regex::new(r"```json\s*(\{[^`]+\})\s*```").ok()?;

    if let Some(captures) = json_pattern.captures(response) {
        if let Some(json_str) = captures.get(1) {
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str.as_str()) {
                if let (Some(tool), Some(args)) = (
                    json_value.get("tool").and_then(|v| v.as_str()),
                    json_value.get("arguments"),
                ) {
                    return Some(ToolCall {
                        tool: tool.to_string(),
                        arguments: args.clone(),
                    });
                }
            }
        }
    }

    None
}

/// 从工具定义构建工具描述
#[allow(dead_code)]
fn build_tools_description_from_definitions(
    definitions: &[rig::completion::ToolDefinition],
) -> String {
    let mut descriptions = Vec::new();

    for def in definitions {
        descriptions.push(format!(
            "### {}\n{}\n\nParameters:\n```json\n{}\n```",
            def.name,
            def.description,
            serde_json::to_string_pretty(&def.parameters).unwrap_or_default()
        ));
    }

    descriptions.join("\n\n")
}

/// 执行内置工具（兼容旧代码）
pub async fn execute_builtin_tool(
    tool_name: &str,
    arguments: &serde_json::Value,
) -> Result<String> {
    let tool_server = get_tool_server();
    tool_server.init_builtin_tools().await;

    let result = tool_server.execute(tool_name, arguments.clone()).await;

    if result.success {
        Ok(result
            .output
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
            .unwrap_or_else(|| "Tool executed successfully".to_string()))
    } else {
        Err(anyhow::anyhow!(
            "Tool execution failed: {}",
            result.error.unwrap_or_else(|| "Unknown error".to_string())
        ))
    }
}

/// 执行工作流工具
pub async fn execute_workflow_tool(
    workflow_id: &str,
    arguments: &serde_json::Value,
) -> Result<String> {
    let tool_name = format!("workflow::{}", workflow_id);
    let tool_server = get_tool_server();

    let result = tool_server.execute(&tool_name, arguments.clone()).await;

    if result.success {
        Ok(result
            .output
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
            .unwrap_or_else(|| "Workflow executed successfully".to_string()))
    } else {
        Err(anyhow::anyhow!(
            "Workflow execution failed: {}",
            result.error.unwrap_or_else(|| "Unknown error".to_string())
        ))
    }
}

/// 执行 MCP 工具
pub async fn execute_mcp_tool(
    server_name: &str,
    tool_name: &str,
    arguments: &serde_json::Value,
) -> Result<String> {
    let full_name = format!("mcp::{}::{}", server_name, tool_name);
    let tool_server = get_tool_server();

    let result = tool_server.execute(&full_name, arguments.clone()).await;

    if result.success {
        Ok(result
            .output
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
            .unwrap_or_else(|| "MCP tool executed successfully".to_string()))
    } else {
        Err(anyhow::anyhow!(
            "MCP tool execution failed: {}",
            result.error.unwrap_or_else(|| "Unknown error".to_string())
        ))
    }
}

/// 执行插件工具
pub async fn execute_plugin_tool(plugin_id: &str, arguments: &serde_json::Value) -> Result<String> {
    let tool_name = format!("plugin::{}", plugin_id);
    let tool_server = get_tool_server();

    let result = tool_server.execute(&tool_name, arguments.clone()).await;

    if result.success {
        Ok(result
            .output
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_default())
            .unwrap_or_else(|| "Plugin executed successfully".to_string()))
    } else {
        Err(anyhow::anyhow!(
            "Plugin execution failed: {}",
            result.error.unwrap_or_else(|| "Unknown error".to_string())
        ))
    }
}

/// Cleanup container workspace files asynchronously (non-blocking)
/// Removes temporary files created during task execution in /workspace
/// Preserves conversation history at /workspace/context/history.txt
async fn cleanup_container_context_async(execution_id: &str) {
    tracing::info!("Starting container workspace cleanup for execution: {}", execution_id);
    
    // Spawn cleanup task in background to avoid blocking response
    let execution_id = execution_id.to_string();
    tokio::spawn(async move {
        use sentinel_tools::shell::get_shell_config;
        
        match get_shell_config().await.docker_config {
            Some(docker_config) => {
                let sandbox = sentinel_tools::DockerSandbox::new(docker_config);
                
                match sentinel_tools::cleanup_container_context(&sandbox).await {
                    Ok(_) => {
                        tracing::info!("Container workspace cleanup completed for execution: {}", execution_id);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to cleanup container workspace for execution {}: {}", execution_id, e);
                    }
                }
            }
            None => {
                tracing::debug!("No Docker config, skipping container workspace cleanup");
            }
        }
    });
}

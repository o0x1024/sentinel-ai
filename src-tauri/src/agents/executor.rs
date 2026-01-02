//! Agent Executor - 使用 sentinel-llm 和 ToolServer 执行 agent 任务
//!
//! 支持工具调用、流式输出、多轮对话。

use anyhow::Result;
use sentinel_db::Database;
use sentinel_llm::{LlmConfig, StreamContent, StreamingLlmClient};
use sentinel_memory::{get_global_memory, ExecutionRecord, MemoryContextRequest, ToolCallSummary};
use sentinel_tools::{get_tool_server, mcp_adapter, ToolServer};
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

use super::tool_router::{ToolConfig, ToolRouter};
use crate::commands::ai::reconstruct_chat_history;

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

    // 设置 task_planner 的 AppHandle 以便发射事件
    use sentinel_tools::buildin_tools::task_planner::set_planner_app_handle;
    set_planner_app_handle(app_handle.clone()).await;

    // 检查是否启用工具
    let tool_config = params.tool_config.clone().unwrap_or_default();

    if tool_config.enabled {
        // 刷新 MCP 工具以确保它们已注册到 ToolServer
        tracing::info!("Refreshing MCP tools before execution...");
        mcp_adapter::refresh_mcp_tools(&tool_server).await;

        // Register VisionExplorerV2Tool if enabled
        if tool_config.enabled && !tool_config.disabled_tools.contains(&"vision_explorer".to_string()) {
           if let Some(mcp_service) = app_handle.try_state::<std::sync::Arc<crate::services::mcp::McpService>>() {
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

                let ve_tool = VisionExplorerV2Tool::new(mcp_service.inner().clone(), llm_config)
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

        // Register TestExplorerTool if enabled (unified tool for text-based web exploration)
        if tool_config.enabled && !tool_config.disabled_tools.contains(&"test_explorer".to_string()) {
            use sentinel_tools::dynamic_tool::{DynamicToolBuilder, ToolSource};
            use crate::engines::test_explorer_v1::{TestExplorerV1Engine, TestExplorerV1Config};

            let app = app_handle.clone();
            let exec_id = params.execution_id.clone();

            let tool_def = DynamicToolBuilder::new("test_explorer")
                .description("Explore a website using text-based automation with LLM. Automatically navigates, interacts with elements, captures API requests, and completes exploration tasks. Best for discovering web APIs and endpoints.")
                .input_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "target_url": {
                            "type": "string",
                            "description": "The target URL to explore and discover APIs from"
                        },
                        "task": {
                            "type": "string",
                            "description": "Optional task instruction (e.g., 'collect all API endpoints'). If not provided, defaults to API discovery."
                        },
                        "max_steps": {
                            "type": "integer",
                            "description": "Maximum number of exploration steps (default: 20)",
                            "default": 20
                        },
                        "headless": {
                            "type": "boolean",
                            "description": "Run browser in headless mode (default: true)",
                            "default": true
                        }
                    },
                    "required": ["target_url"]
                }))
                .source(ToolSource::Builtin)
                .executor(move |args| {
                    let app = app.clone();
                    let exec_id = exec_id.clone();
                    async move {
                        let target_url = args["target_url"].as_str()
                            .ok_or_else(|| "Missing target_url parameter".to_string())?;
                        let task = args["task"].as_str()
                            .unwrap_or("Explore the website and discover all API endpoints, forms, and interactive elements.");
                        let max_steps = args["max_steps"].as_u64().unwrap_or(20) as u32;
                        let headless = args["headless"].as_bool().unwrap_or(true);

                        // Create TestExplorerV1Config with correct fields
                        let config = TestExplorerV1Config {
                            target_url: target_url.to_string(),
                            max_steps,
                            headless,
                            capture_network: true,
                            viewport_width: 1280,
                            viewport_height: 720,
                            page_load_timeout_ms: 30000,
                            user_agent: None,
                        };

                        // Create engine (async new)
                        let mut engine = TestExplorerV1Engine::new(config, None).await
                            .map_err(|e| format!("Failed to initialize browser: {}", e))?;
                        
                        // Execute exploration with the task
                        let result = engine.execute_direct(task).await
                            .map_err(|e| format!("Exploration failed: {}", e))?;
                        
                        // Format result
                        let output = serde_json::json!({
                            "success": result.success,
                            "target_url": target_url,
                            "captured_apis": result.captured_apis.len(),
                            "steps_taken": result.steps_taken.len(),
                            "final_url": result.final_state.url,
                            "final_title": result.final_state.title,
                            "interactive_elements": result.final_state.interactive_elements.len(),
                            "total_duration_ms": result.total_duration_ms,
                            "apis": result.captured_apis,
                        });
                        
                        // Emit event to frontend
                        let _ = app.emit(
                            "test_explorer:complete",
                            &serde_json::json!({
                                "execution_id": exec_id,
                                "result": output,
                            }),
                        );
                        
                        Ok(output)
                    }
                })
                .build();

            if let Ok(tool_def) = tool_def {
                tool_server.register_tool(tool_def).await;
                tracing::info!("Registered TestExplorerTool");
            } else if let Err(e) = tool_def {
                tracing::warn!("Failed to build TestExplorerTool definition: {}", e);
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

    // Build memory context from RAG (Semantic Memory)
    let memory_context = if let Ok(rag_service) = crate::commands::rag_commands::get_global_rag_service().await {
        let mut filters = std::collections::HashMap::new();
        filters.insert("type".to_string(), "agent_memory".to_string());
        
        // Use the current task as query
        let request = sentinel_rag::models::RagQueryRequest {
            query: params.task.clone(),
            collection_id: None,
            top_k: Some(5), // Retrieve top 5 relevant memories
            use_mmr: Some(true),
            mmr_lambda: None,
            filters: Some(filters),
            use_embedding: Some(true),
            reranking_enabled: Some(true),
            similarity_threshold: Some(0.45), // Strict threshold to avoid noise
        };
        
        match rag_service.query(request).await {
            Ok(response) => {
                if response.results.is_empty() {
                    None
                } else {
                    let mut lines = Vec::new();
                    lines.push("[Memory Context (Recall from past experiences):]".to_string());
                    for (i, res) in response.results.iter().enumerate() {
                         lines.push(format!("{}. {}", i + 1, res.chunk.content.trim()));
                    }
                    tracing::info!("Injected {} semantic memories into context", response.results.len());
                    Some(lines.join("\n"))
                }
            }
            Err(e) => {
                tracing::warn!("Failed to query RAG memory: {}", e);
                None
            }
        }
    } else {
        None
    };

    let mut system_prompt = params.system_prompt.clone();
    if let Some(context) = memory_context {
        if !system_prompt.trim().is_empty() {
            system_prompt.push_str("\n\n");
        }
        system_prompt.push_str(&context);
    }

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

            Ok(response)
        }
        Err(e) => {
            tracing::error!(
                "Agent execution failed - execution_id: {}, error: {}",
                params.execution_id,
                e
            );
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

    // Build memory context from RAG (Semantic Memory)
    let memory_context = if let Ok(rag_service) = crate::commands::rag_commands::get_global_rag_service().await {
        let mut filters = std::collections::HashMap::new();
        filters.insert("type".to_string(), "agent_memory".to_string());
        
        let request = sentinel_rag::models::RagQueryRequest {
            query: params.task.clone(),
            collection_id: None,
            top_k: Some(5),
            use_mmr: Some(true),
            mmr_lambda: None,
            filters: Some(filters),
            use_embedding: Some(true),
            reranking_enabled: Some(true),
            similarity_threshold: Some(0.45),
        };
        
        match rag_service.query(request).await {
            Ok(response) => {
                if response.results.is_empty() {
                    None
                } else {
                    let mut lines = Vec::new();
                    lines.push("[Memory Context (Recall from past experiences):]".to_string());
                    for (i, res) in response.results.iter().enumerate() {
                         lines.push(format!("{}. {}", i + 1, res.chunk.content.trim()));
                    }
                    tracing::info!("Injected {} semantic memories into context", response.results.len());
                    Some(lines.join("\n"))
                }
            }
            Err(e) => {
                tracing::warn!("Failed to query RAG memory: {}", e);
                None
            }
        }
    } else {
        None
    };

    if let Some(context) = memory_context {
        if !final_system_prompt.trim().is_empty() {
            final_system_prompt.push_str("\n\n");
        }
        final_system_prompt.push_str(&context);
    }

    // Inject execution_id for task_planner tool
    final_system_prompt.push_str(&format!(
        "\n\n[SystemContext: Current Execution ID is '{}'. Use this for task_planner calls.]",
        params.execution_id
    ));

    // 5. Reuse the same history reconstruction logic as stream_chat_with_llm.
    let history_messages = match db_service
        .inner()
        .get_ai_messages_by_conversation(&params.execution_id)
        .await
    {
        Ok(msgs) => msgs,
        Err(e) => {
            tracing::warn!("Failed to get conversation history: {}", e);
            Vec::new()
        }
    };
    let mut history = reconstruct_chat_history(&history_messages);

    // 移除历史记录中最后一条用户消息，避免与当前任务重复发送
    // 因为 stream_chat_with_dynamic_tools 会自动将 user_prompt 添加到对话末尾
    if let Some(last) = history.last() {
        if last.role == "user" {
            history.pop();
        }
    }

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
    use std::sync::atomic::{AtomicU32, Ordering};
    let tool_calls_collector: Arc<Mutex<Vec<ToolCallRecord>>> = Arc::new(Mutex::new(Vec::new()));
    let pending_calls: Arc<Mutex<std::collections::HashMap<String, (String, String, i64, u32)>>> =
        Arc::new(Mutex::new(std::collections::HashMap::new()));
    let tool_seq: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));
    let assistant_segment_buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let reasoning_content_buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    
    let collector = tool_calls_collector.clone();
    let pending = pending_calls.clone();
    let seq_counter = tool_seq.clone();
    let segment_buf = assistant_segment_buf.clone();
    let reasoning_buf = reasoning_content_buf.clone();

    // 7. 调用带动态工具的流式方法
    let result = client
        .stream_chat_with_dynamic_tools(
            Some(&final_system_prompt),
            &params.task,
            &history,
            None, // 无图片
            dynamic_tools,
            |content| {
                if crate::commands::ai::is_conversation_cancelled(&execution_id) {
                    return false;
                }
                match content {
                StreamContent::Text(text) => {
                    // Accumulate assistant text into a segment buffer.
                    if let Ok(mut buf) = segment_buf.lock() {
                        buf.push_str(&text);
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
                    tracing::info!("Tool call started via rig-core: {} ({})", name, id);
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
                    tracing::info!("Tool call complete via rig-core: {} ({})", name, id);
                    
                    // 记录 pending 的工具调用，等待结果
                    if let Ok(mut pending_map) = pending.lock() {
                        let seq = seq_counter.fetch_add(1, Ordering::Relaxed);
                        let started_at_ms = chrono::Utc::now().timestamp_millis() + seq as i64;
                        pending_map.insert(id.clone(), (name.clone(), arguments.clone(), started_at_ms, seq));
                    }

                    // Flush assistant segment BEFORE inserting tool call message (preserve ordering on reload).
                    if let Some(db) = db_for_stream.clone() {
                        use sentinel_core::models::database as core_db;
                        use chrono::TimeZone;
                        let seg = segment_buf.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
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
                            let reasoning = reasoning_buf.lock().map(|g| {
                                let r = g.clone();
                                // 即使为空也返回 Some("")，因为 deepseek-reasoner 要求必须有此字段
                                Some(if r.trim().is_empty() { String::new() } else { r })
                            }).ok().flatten();
                            
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
                                    tracing::warn!("Failed to persist assistant segment: {}", e);
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

                        let tool_args_val: serde_json::Value = serde_json::from_str(&arguments)
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
                    tracing::info!("Tool result via rig-core: id={}, result_preview={}", id, &result.chars().take(500).collect::<String>());
                    
                    // 将工具调用完整信息添加到收集器
                    if let Ok(mut pending_map) = pending.lock() {
                        if let Some((name, arguments, started_at_ms, seq)) = pending_map.remove(&id) {
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

                                let tool_args_val: serde_json::Value = serde_json::from_str(&args_for_meta)
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
                                    if let Err(e) = db.upsert_ai_message_append(&tool_msg).await {
                                        tracing::warn!("Failed to persist tool result update: {}", e);
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
                    tracing::info!("Stream completed - execution_id: {}", execution_id);
                    // Flush any remaining assistant segment at the end (so history shows it after the last tool call).
                    if let Some(db) = db_for_stream.clone() {
                        use sentinel_core::models::database as core_db;
                        let seg = segment_buf.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
                        let seg_trimmed = seg.trim().to_string();
                        if !seg_trimmed.trim().is_empty() {
                            // Get reasoning content for final segment
                            // 参考：https://api-docs.deepseek.com/zh-cn/guides/thinking_mode#tool-calls
                            let reasoning = reasoning_buf.lock().map(|mut g| {
                                let r = std::mem::take(&mut *g);
                                // 即使为空也返回 Some("")，因为 deepseek-reasoner 要求必须有此字段
                                Some(if r.trim().is_empty() { String::new() } else { r })
                            }).ok().flatten();
                            
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
                                    tracing::warn!("Failed to persist final assistant segment: {}", e);
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
            tracing::info!(
                "Agent with tools completed - execution_id: {}, response_length: {}",
                params.execution_id,
                response.len()
            );

            // NOTE:
            // We persist assistant output as multiple "assistant" segments and tool calls as "tool" messages
            // during streaming, to preserve strict event ordering when reloading history.
            // So we intentionally do NOT save a single aggregated assistant message here.

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

            // 记录工具使用（用于智能选择学习）
            // FIXME: record_tool_usage signature mismatch
            // for tool_id in &selected_tool_ids {
            //     record_tool_usage(tool_id, &params.task);
            // }

            Ok(response)
        }
        Err(e) => {
            tracing::error!(
                "Agent with tools failed - execution_id: {}, error: {}",
                params.execution_id,
                e
            );
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
                    error: Some(e.to_string()),
                    response_excerpt: None,
                    created_at: chrono::Utc::now().timestamp(),
                })
                .await
            {
                tracing::warn!("Failed to store memory record: {}", err);
            }
            Err(e)
        }
    }
}

fn truncate_for_memory(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    let mut out = text.chars().take(max_len).collect::<String>();
    out.push_str("...");
    out
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

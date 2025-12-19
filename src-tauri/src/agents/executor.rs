//! Agent Executor - 使用 sentinel-llm 和 ToolServer 执行 agent 任务
//!
//! 支持工具调用、流式输出、多轮对话。

use anyhow::Result;
use sentinel_db::Database;
use sentinel_llm::{LlmConfig, StreamContent, StreamingLlmClient};
use sentinel_tools::{get_tool_server, mcp_adapter, ToolServer};
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

use super::tool_router::{record_tool_usage, ToolConfig, ToolRouter};

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

    // 初始化全局工具服务器
    let tool_server = get_tool_server();
    tool_server.init_builtin_tools().await;

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

    // 创建流式客户端
    let client = StreamingLlmClient::new(config);

    // 准备事件发送
    let execution_id = params.execution_id.clone();
    let app = app_handle.clone();

    // 执行流式调用
    let result = client
        .stream_completion(
            Some(&params.system_prompt),
            &params.task,
            |content| {
                if crate::commands::ai::is_conversation_cancelled(&execution_id) {
                    return;
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
                    StreamContent::Done => {
                        tracing::info!("Agent completed - execution_id: {}", execution_id);
                    }
                }
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
            save_assistant_message(app_handle, &params.execution_id, &response, None).await;

            Ok(response)
        }
        Err(e) => {
            tracing::error!(
                "Agent execution failed - execution_id: {}, error: {}",
                params.execution_id,
                e
            );
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
) {
    if content.trim().is_empty() && tool_calls.map_or(true, |tc| tc.is_empty()) {
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

    let selected_tool_ids = tool_router
        .select_tools(&params.task, &tool_config, Some(&llm_config))
        .await?;

    tracing::info!(
        "Selected {} tools for execution_id {}: {:?}",
        selected_tool_ids.len(),
        params.execution_id,
        selected_tool_ids
    );

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

    // 4. 使用 rig-core 原生工具调用
    // rig 的 multi_turn() 会自动处理工具调用循环
    let client = StreamingLlmClient::new(llm_config);
    let execution_id = params.execution_id.clone();
    let app = app_handle.clone();

    // 用于收集工具调用信息
    use std::sync::Mutex;
    let tool_calls_collector: Arc<Mutex<Vec<ToolCallRecord>>> = Arc::new(Mutex::new(Vec::new()));
    let pending_calls: Arc<Mutex<std::collections::HashMap<String, (String, String)>>> = 
        Arc::new(Mutex::new(std::collections::HashMap::new()));
    
    let collector = tool_calls_collector.clone();
    let pending = pending_calls.clone();

    // 5. 调用带动态工具的流式方法
    let result = client
        .stream_chat_with_dynamic_tools(
            Some(&params.system_prompt),
            &params.task,
            &[],  // 空的历史记录
            None, // 无图片
            dynamic_tools,
            |content| {
                if crate::commands::ai::is_conversation_cancelled(&execution_id) {
                    return;
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
                        pending_map.insert(id.clone(), (name.clone(), arguments.clone()));
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
                        if let Some((name, arguments)) = pending_map.remove(&id) {
                            if let Ok(mut records) = collector.lock() {
                                records.push(ToolCallRecord {
                                    id: id.clone(),
                                    name,
                                    arguments,
                                    result: Some(result.clone()),
                                    success: !result.to_lowercase().contains("error"),
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
                StreamContent::Done => {
                    tracing::info!("Stream completed - execution_id: {}", execution_id);
                }
            }},
        )
        .await;

    match result {
        Ok(response) => {
            tracing::info!(
                "Agent with tools completed - execution_id: {}, response_length: {}",
                params.execution_id,
                response.len()
            );

            // 获取收集到的工具调用记录
            let tool_call_records: Vec<ToolCallRecord> = tool_calls_collector
                .lock()
                .map(|guard| guard.clone())
                .unwrap_or_default();
            
            tracing::info!(
                "Collected {} tool call records for execution_id {}",
                tool_call_records.len(),
                params.execution_id
            );

            // 保存助手消息到数据库（带工具调用信息）
            let records_ref = if tool_call_records.is_empty() {
                None
            } else {
                Some(tool_call_records.as_slice())
            };
            save_assistant_message(app_handle, &params.execution_id, &response, records_ref).await;

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
            Err(e)
        }
    }
}

/// 工具调用结构
#[derive(Debug, Clone)]
struct ToolCall {
    tool: String,
    arguments: serde_json::Value,
}

/// 从响应中提取工具调用
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

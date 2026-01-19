use sentinel_db::Database;
use crate::commands::traffic_analysis_commands::TrafficAnalysisState;
use crate::commands::tool_commands;
use crate::models::database::{AiConversation, AiMessage, SubagentMessage, SubagentRun};
use crate::services::ai::{AiConfig, AiServiceManager, AiServiceWrapper, AiToolCall};
use crate::services::database::DatabaseService;
use crate::utils::ordered_message::ChunkType;
use anyhow::Result;
use chrono::Utc;
use sentinel_llm::{
    parse_image_from_json, ChatMessage as LlmChatMessage, StreamContent, StreamingLlmClient,
};
use sentinel_rag;
use sentinel_workflow::WorkflowGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

// Re-export AI settings related types for backward compatibility
pub use crate::commands::aisettings::{
    TestConnectionRequest, TestConnectionResponse, SaveAiConfigRequest,
    SetDefaultProviderRequest, AddCustomProviderRequest, AiProviderConfig,
};

// DTO for Tauri command argument to avoid CommandArg bound issues
#[derive(Debug, Clone, Deserialize)]
pub struct CommandAiConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub organization: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl From<CommandAiConfig> for AiConfig {
    fn from(c: CommandAiConfig) -> Self {
        AiConfig {
            provider: c.provider,
            model: c.model,
            api_key: c.api_key,
            api_base: c.api_base,
            organization: c.organization,
            temperature: c.temperature,
            max_tokens: c.max_tokens,
            rig_provider: None,
            max_turns: None,
        }
    }
}

// 全局取消令牌管理器
static CANCELLATION_TOKENS: std::sync::LazyLock<Mutex<HashMap<String, CancellationToken>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

// 辅助函数：创建取消令牌
fn create_cancellation_token(conversation_id: &str) -> CancellationToken {
    let token = CancellationToken::new();
    if let Ok(mut tokens) = CANCELLATION_TOKENS.lock() {
        if let Some(old) = tokens.remove(conversation_id) {
            old.cancel();
        }
        tokens.insert(conversation_id.to_string(), token.clone());
    }
    token
}

// 辅助函数：获取取消令牌
fn get_cancellation_token(conversation_id: &str) -> Option<CancellationToken> {
    if let Ok(tokens) = CANCELLATION_TOKENS.lock() {
        tokens.get(conversation_id).cloned()
    } else {
        None
    }
}

// 辅助函数：移除取消令牌
fn remove_cancellation_token(conversation_id: &str) {
    if let Ok(mut tokens) = CANCELLATION_TOKENS.lock() {
        tokens.remove(conversation_id);
    }
}

// 辅助函数：取消对话流（公开以便其他模块调用）
pub fn cancel_conversation_stream(conversation_id: &str) {
    if let Some(token) = get_cancellation_token(conversation_id) {
        token.cancel();
        tracing::info!("Cancelled conversation stream: {}", conversation_id);
    }
}

/// 检查某个会话是否已被取消（用于停止后续事件发送）
pub fn is_conversation_cancelled(conversation_id: &str) -> bool {
    get_cancellation_token(conversation_id)
        .map(|t| t.is_cancelled())
        .unwrap_or(false)
}

/// 辅助结构：用于在任务结束时自动移除取消令牌
pub struct CancellationGuard(pub String);

impl Drop for CancellationGuard {
    fn drop(&mut self) {
        remove_cancellation_token(&self.0);
    }
}

/// 执行 RAG 增强：包含查询重写、多集合检索和配置透传
async fn perform_rag_enhancement(
    app_handle: &AppHandle,
    conversation_id: &str,
    user_message: &str,
    history_messages: &[AiMessage],
    rag_config: Option<sentinel_rag::config::RagConfig>,
) -> Result<(String, Vec<sentinel_rag::models::Citation>), String> {
    // 1. 获取数据库服务
    let db = app_handle
        .try_state::<Arc<DatabaseService>>()
        .ok_or("Database service not initialized")?;

    // 2. 查询重写 (Query Rewriting)
    let mut search_query = user_message.to_string();
    if !history_messages.is_empty() {
        // 使用默认模型重写
        if let Ok(Some((provider, model))) = app_handle.state::<Arc<AiServiceManager>>().get_default_llm_model().await {
            // 获取 provider 配置
            if let Ok(Some(provider_cfg)) = app_handle.state::<Arc<AiServiceManager>>().get_provider_config(&provider).await {
                let llm_config = sentinel_llm::LlmConfig::new(&provider, &model)
                    .with_api_key(provider_cfg.api_key.as_deref().unwrap_or_default())
                    .with_base_url(provider_cfg.api_base.as_deref().unwrap_or_default());
                
                let client = sentinel_llm::LlmClient::new(llm_config);
                
                let rewrite_prompt = "you are a search query rewrite expert. Please rewrite the user's latest question into a independent and complete search query for retrieval in the vector database. If the user's question is already independent, return it as is. Only return the rewritten query text, no additional explanation.";
                let mut history_text = String::new();
                // 取最近几条历史
                for msg in history_messages.iter().rev().take(6).rev() {
                    history_text.push_str(&format!("{}: {}\n", msg.role, msg.content));
                }
                let input = format!("conversation history: \n{}\nuser question: {}", history_text, user_message);
                
                if let Ok(rewritten) = client.completion(Some(rewrite_prompt), &input).await {
                    if !rewritten.trim().is_empty() {
                        search_query = rewritten.trim().to_string();
                        tracing::info!("Query rewritten: {} -> {}", user_message, search_query);
                    }
                }
            }
        }
    }

    // 3. 获取所有激活的集合
    let active_collections = match db.get_rag_collections().await {
        Ok(cols) => cols.into_iter().filter(|c| c.is_active).collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };

    if active_collections.is_empty() {
        return Ok((String::new(), Vec::new()));
    }

    // 4. 执行多集合检索
    let rag_service = crate::commands::rag_commands::get_global_rag_service().await?;

    // 如果没有传入配置，尝试从数据库获取
    let effective_config = if let Some(cfg) = rag_config {
        cfg
    } else {
        match db.get_rag_config().await {
            Ok(Some(cfg_core)) => crate::commands::rag_commands::convert_core_to_rag(cfg_core),
            _ => sentinel_rag::config::RagConfig::default(),
        }
    };

        let rag_req = sentinel_rag::models::AssistantRagRequest {
            query: search_query.clone(),
            conversation_id: Some(conversation_id.to_string()),
            collection_id: None,
            collection_ids: Some(active_collections.into_iter().map(|c| c.id).collect()),
            conversation_history: None, // 我们已经重写了查询
            top_k: Some(effective_config.top_k),
            use_mmr: Some(effective_config.mmr_lambda < 1.0),
            mmr_lambda: Some(effective_config.mmr_lambda),
            similarity_threshold: Some(effective_config.similarity_threshold),
            reranking_enabled: Some(effective_config.reranking_enabled),
            model_provider: None,
            model_name: None,
            max_tokens: None,
            temperature: None,
            system_prompt: None,
        };

    // 设置 5 秒超时支持多集合检索
    let (all_context, all_citations) = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        rag_service.query_for_assistant(&rag_req),
    ).await {
        Ok(Ok(res)) => res,
        Ok(Err(e)) => {
            tracing::warn!("RAG search error: {}", e);
            (String::new(), Vec::new())
        }
        Err(_) => {
            tracing::warn!("RAG search timeout");
            (String::new(), Vec::new())
        }
    };

    Ok((all_context, all_citations))
}

/// 流式调用 LLM 并处理事件发送、消息保存
///
/// 重新组合历史消息，将 role=tool 的消息转换为符合 DeepSeek API 的格式
/// 
/// 当前数据库存储格式：
/// - role=assistant: 文本片段（有 reasoning_content，无 tool_calls）
/// - role=tool: 工具调用信息（metadata 中包含 tool_name, tool_args, tool_result）
///
/// DeepSeek API 期望格式：
/// - role=assistant: 包含 reasoning_content 和 tool_calls
/// - role=tool: 工具执行结果
pub(crate) fn reconstruct_chat_history(
    messages: &[sentinel_core::models::database::AiMessage],
) -> Vec<LlmChatMessage> {
    use serde_json::Value;
    use std::collections::HashSet;
    
    let mut result = Vec::new();
    let mut i = 0;
    
    // Track seen tool_call_ids to prevent duplicate tool_results
    // Anthropic API requires each tool_use to have exactly one tool_result
    let mut seen_tool_call_ids: HashSet<String> = HashSet::new();
    
    while i < messages.len() {
        let msg = &messages[i];
        
        match msg.role.as_str() {
            "user" => {
                if !msg.content.trim().is_empty() {
                    result.push(LlmChatMessage::user(&msg.content));
                }
                i += 1;
            }
            "assistant" => {
                // 检查后面是否有 tool 消息
                let mut tool_calls_json = Vec::new();
                let mut tool_results = Vec::new();
                let mut j = i + 1;
                let reasoning_content = msg.reasoning_content.clone();
                
                // 收集所有连续的 tool 消息
                while j < messages.len() && messages[j].role == "tool" {
                    if let Some(ref metadata_str) = messages[j].metadata {
                        if let Ok(metadata) = serde_json::from_str::<Value>(metadata_str) {
                            if metadata.get("kind").and_then(|v| v.as_str()) == Some("tool_call") {
                                let tool_result = metadata.get("tool_result")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                // 只在存在工具结果时才加入 tool_calls，避免 DeepSeek 要求的 tool_result 不足
                                if let Some(result_str) = tool_result {
                                    // 这是一个完成的工具调用
                                    let tool_call_id = metadata.get("tool_call_id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    
                                    // Skip duplicate tool_call_id
                                    if !tool_call_id.is_empty() && !seen_tool_call_ids.contains(&tool_call_id) {
                                        seen_tool_call_ids.insert(tool_call_id.clone());
                                        
                                        let tool_name = metadata.get("tool_name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let tool_args = metadata.get("tool_args")
                                            .cloned()
                                            .unwrap_or(Value::Object(serde_json::Map::new()));

                                        // 构建 tool_call JSON
                                        tool_calls_json.push(serde_json::json!({
                                            "id": tool_call_id,
                                            "type": "function",
                                            "function": {
                                                "name": tool_name,
                                                "arguments": tool_args
                                            }
                                        }));

                                        // 保存为 tool result 消息
                                        tool_results.push((tool_call_id, result_str));
                                    }
                                }
                            }
                        }
                    }
                    j += 1;
                }
                
                // 创建 assistant 消息
                let mut chat_msg = LlmChatMessage::new("assistant", &msg.content);
                
                // 如果有工具调用，添加 tool_calls 和 reasoning_content
                if !tool_calls_json.is_empty() {
                    chat_msg.tool_calls = Some(serde_json::to_string(&tool_calls_json).unwrap_or_default());
                    // 确保有 reasoning_content（即使为空）
                    chat_msg.reasoning_content = Some(reasoning_content.unwrap_or_default());
                } else if reasoning_content.is_some() {
                    chat_msg.reasoning_content = reasoning_content;
                }
                
                result.push(chat_msg);
                
                // 添加 tool result 消息
                for (tool_call_id, tool_result) in tool_results {
                    let mut tool_msg = LlmChatMessage::new("tool", &tool_result);
                    tool_msg.tool_call_id = Some(tool_call_id);
                    result.push(tool_msg);
                }
                
                i = j;
            }
            "tool" => {
                // 独立的 tool 消息（不跟在 assistant 后面的）
                // 这种情况不应该发生，但为了安全起见还是处理一下
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    
    result
}

/// 使用 sentinel_llm::StreamingLlmClient 处理 LLM 调用
async fn stream_chat_with_llm(
    service: &AiServiceWrapper,
    app_handle: &AppHandle,
    conversation_id: &str,
    message_id: &str,
    user_message: &str,
    system_prompt: Option<&str>,
    attachments: Option<serde_json::Value>,
    is_final: bool,
) -> Result<String, String> {
    let db = app_handle
        .try_state::<Arc<DatabaseService>>()
        .ok_or("Database service not initialized")?;

    // 获取对话历史
    let history_messages = match db.get_ai_messages_by_conversation(conversation_id).await {
        Ok(msgs) => msgs,
        Err(e) => {
            tracing::warn!("Failed to get conversation history: {}", e);
            Vec::new()
        }
    };

    // 检查对话是否存在
    let has_conversation = db
        .get_ai_conversation(conversation_id)
        .await
        .map(|c| c.is_some())
        .unwrap_or(false);

    // 注意：用户消息已经在 agent_execute 中保存，这里不需要重复保存

    // 解析图片附件
    let image = parse_image_from_json(attachments.as_ref());

    // 转换历史消息，重新组合 assistant + tool 消息以符合 DeepSeek API 要求
    let mut history: Vec<LlmChatMessage> = reconstruct_chat_history(&history_messages);

    // 移除历史记录中最后一条用户消息，避免与当前消息重复发送
    // 因为 stream_chat 会自动将 user_message 添加到对话末尾
    if let Some(last) = history.last() {
        if last.role == "user" {
            history.pop();
        }
    }

    // 创建 LLM 客户端
    let llm_config = service.service.to_llm_config();
    let streaming_client = StreamingLlmClient::new(llm_config);

    // -------------------
    
    // 准备系统提示词
    let final_system_prompt = system_prompt.unwrap_or("").to_string();
    
    // 注意：RAG 增强逻辑已移至 agent_execute 中统一处理，通过 system_prompt 传入。
    // 这里保持 stream_chat_with_llm 职责单一，仅负责流式输出。

    // 流式调用
    let execution_id = conversation_id.to_string();
    let msg_id = message_id.to_string();
    let conv_id = conversation_id.to_string();
    let app = app_handle.clone();

    // 记录用量
    let usage_data = Arc::new(std::sync::Mutex::new(None::<(u32, u32)>));
    let usage_data_clone = usage_data.clone();

    let content = streaming_client
        .stream_chat(
            Some(&final_system_prompt),
            user_message,
            &history,
            image.as_ref(),
            move |chunk| {
                if is_conversation_cancelled(&conv_id) {
                    return false;
                }
                match chunk {
                    StreamContent::Text(text) => {
                        tracing::debug!("Stream chunk received: {} chars", text.len());
                        crate::utils::ordered_message::emit_message_chunk_with_arch(
                            &app,
                            &execution_id,
                            &msg_id,
                            Some(&conv_id),
                            ChunkType::Content,
                            &text,
                            false,
                            None,
                            None,
                            None,
                            None,
                        );
                    }
                    StreamContent::Reasoning(text) => {
                        tracing::debug!("Stream reasoning received: {} chars", text.len());
                        crate::utils::ordered_message::emit_message_chunk_with_arch(
                            &app,
                            &execution_id,
                            &msg_id,
                            Some(&conv_id),
                            ChunkType::Thinking,
                            &text,
                            false,
                            None,
                            None,
                            None,
                            None,
                        );
                    }
                    StreamContent::Usage { input_tokens, output_tokens } => {
                        tracing::info!("Stream usage received: input={}, output={}", input_tokens, output_tokens);
                        if let Ok(mut guard) = usage_data_clone.lock() {
                            *guard = Some((input_tokens, output_tokens));
                        }
                    }
                    StreamContent::ToolCallStart { id, name } => {
                        tracing::info!("Tool call started: id={}, name={}", id, name);
                        // 发送工具调用开始事件
                        let _ = app.emit(
                            "agent:tool_call_start",
                            serde_json::json!({
                                "execution_id": &execution_id,
                                "tool_call_id": id,
                                "tool_name": name,
                            }),
                        );
                    }
                    StreamContent::ToolCallDelta { id, delta } => {
                        tracing::debug!("Tool call delta: id={}, delta_len={}", id, delta.len());
                        // 发送工具调用参数增量
                        let _ = app.emit(
                            "agent:tool_call_delta",
                            serde_json::json!({
                                "execution_id": &execution_id,
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
                        tracing::info!("Tool call complete: id={}, name={}", id, name);
                        // 发送工具调用完成事件
                        let _ = app.emit(
                            "agent:tool_call_complete",
                            serde_json::json!({
                                "execution_id": &execution_id,
                                "tool_call_id": id,
                                "tool_name": name,
                                "arguments": arguments,
                            }),
                        );
                    }
                    StreamContent::ToolResult { id, result } => {
                        tracing::info!("Tool result: id={}, result_len={}", id, result.len());
                        // 发送工具执行结果事件
                        let _ = app.emit(
                            "agent:tool_result",
                            serde_json::json!({
                                "execution_id": &execution_id,
                                "tool_call_id": id,
                                "result": result,
                            }),
                        );
                    }
                    StreamContent::Done => {
                        tracing::debug!("Stream done received");
                    }
                }
                true
            },
        )
        .await
        .map_err(|e| format!("LLM stream error: {}", e))?;

    // 发送完成标记
    if is_final && has_conversation {
        crate::utils::ordered_message::emit_message_chunk_with_arch(
            app_handle,
            conversation_id,
            message_id,
            Some(conversation_id),
            ChunkType::Meta,
            "",
            true,
            None,
            None,
            None,
            None,
        );
    }

    // 保存助手消息
    if has_conversation && !content.is_empty() {
        use sentinel_core::models::database as core_db;
        
        let (input_tokens, output_tokens) = if let Ok(guard) = usage_data.lock() {
            guard.unwrap_or((0, 0))
        } else {
            (0, 0)
        };

        let msg = core_db::AiMessage {
            id: message_id.to_string(),
            conversation_id: conversation_id.to_string(),
            role: "assistant".to_string(),
            content: content.clone(),
            metadata: None,
            token_count: Some(output_tokens as i32),
            cost: None,
            tool_calls: None,
            attachments: None,
            reasoning_content: None,
            timestamp: chrono::Utc::now(),
            architecture_type: None,
            architecture_meta: None,
            structured_data: None,
        };
        if let Err(e) = db.upsert_ai_message_append(&msg).await {
            tracing::warn!("Failed to save assistant message: {}", e);
        } else {
            // 更新用量统计
            if input_tokens > 0 || output_tokens > 0 {
                let provider = &service.config.provider;
                let model = &service.config.model;
                
                // 计算成本
                let cost = sentinel_llm::calculate_cost(provider, model, input_tokens, output_tokens);
                
                if let Err(e) = db.update_ai_usage(provider, model, input_tokens as i32, output_tokens as i32, cost).await {
                    tracing::warn!("Failed to update AI usage stats: {}", e);
                } else {
                    tracing::debug!(
                        "Updated AI usage: provider={}, model={}, input={}, output={}, cost=${:.4}",
                        provider, model, input_tokens, output_tokens, cost
                    );
                }
            }

            // 发送助手消息保存成功事件到前端
            let _ = app_handle.emit(
                "agent:assistant_message_saved",
                &serde_json::json!({
                    "execution_id": conversation_id,
                    "message_id": message_id,
                    "content": content,
                    "timestamp": msg.timestamp.timestamp_millis(),
                }),
            );
        }
    }

    Ok(content)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
    pub service_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub conversation_id: String,
    pub message: String,
    pub service_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendStreamMessageRequest {
    pub conversation_id: String,
    pub message: String,
    pub service_name: Option<String>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveMessageRequest {
    pub id: Option<String>,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub architecture_type: Option<String>,
    pub architecture_meta: Option<String>,
    pub structured_data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteToolCallRequest {
    pub conversation_id: String,
    pub service_name: String,
    pub tool_call: AiToolCall,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiServiceInfo {
    pub name: String,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiModelInfo {
    pub provider: String,
    pub models: Vec<String>,
}



#[derive(Debug, Serialize, Deserialize)]
pub struct AiServiceStatusResponse {
    pub provider: String,
    pub is_available: bool,
    pub models_count: usize,
    pub active_conversations: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub is_chat: bool,
    pub is_embedding: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    pub name: String,
    pub provider: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StopStreamRequest {
    pub conversation_id: String,
}

// 列出所有AI服务
#[tauri::command]
pub async fn list_ai_services(
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<Vec<String>, String> {
    Ok(ai_manager.list_services())
}

// 取消流式聊天
#[tauri::command]
pub async fn cancel_ai_stream(
    conversation_id: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    tracing::info!("Cancelling stream for conversation: {}", conversation_id);
    cancel_conversation_stream(&conversation_id);

    // Also cancel long-running tool executions (e.g. VisionExplorer) that use the global cancellation manager.
    // conversation_id is used as execution_id across the app.
    let _ = crate::managers::cancellation_manager::cancel_execution(&conversation_id).await;

    // 发送取消事件通知前端
    let _ = app_handle.emit(
        "agent:cancelled",
        &serde_json::json!({
            "execution_id": conversation_id,
            "message": "Execution cancelled by user"
        }),
    );

    Ok(())
}

// 轻量级流式生成请求（不保存消息到数据库）
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateStreamRequest {
    pub stream_id: String,
    pub message: String,
    pub system_prompt: Option<String>,
    pub service_name: Option<String>,
    pub history: Option<Vec<LlmChatMessage>>,
}

// 轻量级流式生成（插件生成专用，不保存消息）
#[tauri::command]
pub async fn generate_plugin_stream(
    request: GenerateStreamRequest,
    app_handle: AppHandle,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<String, String> {
    // Get actual default LLM provider and model from database config
    let mut service_name = request.service_name.clone().unwrap_or_else(|| "default".to_string());
    let mut override_model: Option<String> = None;
    
    // If using default, try to get the actual provider name and model from config
    if service_name == "default" {
        if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
            // Get default model (format: "provider/model")
            if let Ok(Some(default_llm_model)) = db.get_config("ai", "default_llm_model").await {
                if let Some((provider, model)) = default_llm_model.split_once('/') {
                    let provider_lc = provider.to_lowercase();
                    if ai_manager.get_service(&provider_lc).is_some() {
                        service_name = provider_lc;
                        override_model = Some(model.to_string());
                        tracing::debug!("Using default LLM model from config: {}/{}", service_name, model);
                    }
                }
            }
            
            // Fallback to default_llm_provider if model not set
            if override_model.is_none() {
                if let Ok(Some(default_llm_provider)) = db.get_config("ai", "default_llm_provider").await {
                    let provider_lc = default_llm_provider.to_lowercase();
                    if ai_manager.get_service(&provider_lc).is_some() {
                        service_name = provider_lc;
                        tracing::debug!("Using default LLM provider from config: {}", service_name);
                    }
                }
            }
        }
    }

    let service = ai_manager
        .get_service(&service_name)
        .or_else(|| ai_manager.get_service("default"))
        .ok_or_else(|| format!("AI service '{}' not found", service_name))?;
    
    // Use override model if available, otherwise use service's configured model
    let model_to_use = override_model.unwrap_or_else(|| service.get_config().model.clone());
    
    tracing::info!("Plugin generation using provider: {}, model: {}", 
        service.get_config().provider, model_to_use);

    let stream_id = request.stream_id.clone();
    let user_message = request.message.clone();
    let system_prompt = request.system_prompt.clone();

    let _cancellation_token = create_cancellation_token(&stream_id);
    let app_clone = app_handle.clone();
    let sid = stream_id.clone();
    let history = request.history.unwrap_or_default();

    // Build LLM config with correct model
    let mut llm_config = service.service.to_llm_config();
    llm_config = llm_config.with_model(&model_to_use);

    tokio::spawn(async move {
        let _guard = CancellationGuard(sid.clone());
        // Start event
        let _ = app_clone.emit("plugin_gen_start", &serde_json::json!({ "stream_id": sid }));

        // Create LLM client with configured model
        let streaming_client = StreamingLlmClient::new(llm_config);
        let app_for_callback = app_clone.clone();
        let sid_for_callback = sid.clone();

        let result = streaming_client
            .stream_chat(
                system_prompt.as_deref(),
                &user_message,
                &history,
                None,
                move |chunk| {
                    if is_conversation_cancelled(&sid_for_callback) {
                        return false;
                    }
                    match chunk {
                        StreamContent::Text(text) => {
                            let _ = app_for_callback.emit(
                                "plugin_gen_delta",
                                serde_json::json!({
                                    "stream_id": sid_for_callback,
                                    "delta": text
                                }),
                            );
                        }
                        StreamContent::Reasoning(text) => {
                            let _ = app_for_callback.emit(
                                "plugin_gen_thinking",
                                serde_json::json!({
                                    "stream_id": sid_for_callback,
                                    "delta": text
                                }),
                            );
                        }
                        StreamContent::Done => {}
                        _ => {}
                    }
                    true
                },
            )
            .await;

        match result {
            Ok(content) => {
                let _ = app_clone.emit(
                    "plugin_gen_complete",
                    serde_json::json!({
                        "stream_id": sid,
                        "content": content
                    }),
                );
            }
            Err(e) => {
                let _ = app_clone.emit(
                    "plugin_gen_error",
                    serde_json::json!({
                        "stream_id": sid,
                        "error": e.to_string()
                    }),
                );
            }
        }
    });

    Ok(stream_id)
}

#[tauri::command]
pub async fn generate_ai_role(
    prompt: String,
    app_handle: AppHandle,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<serde_json::Value, String> {
    // Get actual default LLM provider from database config
    let mut service_name = "default".to_string();
    if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
        if let Ok(Some(default_llm_provider)) = db.get_config("ai", "default_llm_provider").await {
            let provider_lc = default_llm_provider.to_lowercase();
            if ai_manager.get_service(&provider_lc).is_some() {
                service_name = provider_lc;
            }
        }
    }

    let service = ai_manager
        .get_service(&service_name)
        .or_else(|| ai_manager.get_service("default"))
        .ok_or_else(|| format!("AI service '{}' not found", service_name))?;

    let llm_config = service.service.to_llm_config();
    let client = sentinel_llm::LlmClient::new(llm_config);

    let system_prompt = r#"You are a professional AI Assistant Role Creator. 
Your task is to create a specific AI role based on user's description.
Output MUST be in JSON format with the following fields:
- title: A short, professional title for the role.
- description: A brief summary of what the role does.
- prompt: A comprehensive system prompt that defines the role's persona, expertise, tone, and specific instructions.

ONLY return the JSON object, no other text."#;

    let user_input = format!("Create an AI role for: {}", prompt);

    match client.completion(Some(system_prompt), &user_input).await {
        Ok(response) => {
            // Try to parse JSON from the response
            let cleaned = response.trim();
            let json_start = cleaned.find('{').unwrap_or(0);
            let json_end = cleaned.rfind('}').map(|e| e + 1).unwrap_or(cleaned.len());
            let json_str = &cleaned[json_start..json_end];
            
            serde_json::from_str(json_str).map_err(|e| format!("Failed to parse generated role JSON: {}. Original response: {}", e, response))
        }
        Err(e) => Err(format!("Failed to generate AI role: {}", e)),
    }
}

// 取消插件生成
#[tauri::command]
pub async fn cancel_plugin_generation(
    stream_id: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    cancel_conversation_stream(&stream_id);
    let _ = app_handle.emit(
        "plugin_gen_cancelled",
        &serde_json::json!({ "stream_id": stream_id }),
    );
    Ok(())
}

// AI 助手对话请求（专门用于编辑器内的 AI 助手面板）
#[derive(Debug, Clone, Deserialize)]
pub struct PluginAssistantRequest {
    pub stream_id: String,
    pub message: String,
    pub system_prompt: Option<String>,
    pub service_name: Option<String>,
    pub history: Option<Vec<LlmChatMessage>>,
    pub current_code: Option<String>,  // 当前编辑的代码
    pub code_context: Option<String>,  // 代码上下文（选中的代码片段）
}

// AI 助手对话流式响应（专门用于编辑器 AI 助手）
#[tauri::command]
pub async fn plugin_assistant_chat_stream(
    request: PluginAssistantRequest,
    app_handle: AppHandle,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<String, String> {
    // Get actual default LLM provider from database config
    let mut service_name = request.service_name.clone().unwrap_or_else(|| "default".to_string());
    
    // If using default, try to get the actual provider name from config
    if service_name == "default" {
        if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
            if let Ok(Some(default_llm_provider)) = db.get_config("ai", "default_llm_provider").await {
                let provider_lc = default_llm_provider.to_lowercase();
                if ai_manager.get_service(&provider_lc).is_some() {
                    service_name = provider_lc;
                    tracing::debug!("Using default LLM provider from config: {}", service_name);
                }
            }
        }
    }

    let service = ai_manager
        .get_service(&service_name)
        .or_else(|| ai_manager.get_service("default"))
        .ok_or_else(|| format!("AI service '{}' not found", service_name))?;
    
    tracing::info!("Plugin assistant chat using provider: {}, model: {}", 
        service.get_config().provider, service.get_config().model);

    let stream_id = request.stream_id.clone();
    let user_message = request.message.clone();
    let system_prompt = request.system_prompt.clone();
    let service_clone = service.clone();

    let _cancellation_token = create_cancellation_token(&stream_id);
    let app_clone = app_handle.clone();
    let sid = stream_id.clone();
    let history = request.history.unwrap_or_default();

    tokio::spawn(async move {
        let _guard = CancellationGuard(sid.clone());
        // Start event
        let _ = app_clone.emit("plugin_assistant_start", &serde_json::json!({ "stream_id": sid }));

        // Create LLM client and stream
        let llm_config = service_clone.service.to_llm_config();
        let streaming_client = StreamingLlmClient::new(llm_config);
        let app_for_callback = app_clone.clone();
        let sid_for_callback = sid.clone();

        let result = streaming_client
            .stream_chat(
                system_prompt.as_deref(),
                &user_message,
                &history,
                None,
                move |chunk| {
                    if is_conversation_cancelled(&sid_for_callback) {
                        return false;
                    }
                    match chunk {
                        StreamContent::Text(text) => {
                            let _ = app_for_callback.emit(
                                "plugin_assistant_delta",
                                serde_json::json!({
                                    "stream_id": sid_for_callback,
                                    "delta": text
                                }),
                            );
                        }
                        StreamContent::Reasoning(text) => {
                            let _ = app_for_callback.emit(
                                "plugin_assistant_thinking",
                                serde_json::json!({
                                    "stream_id": sid_for_callback,
                                    "delta": text
                                }),
                            );
                        }
                        StreamContent::Done => {}
                        _ => {}
                    }
                    true
                },
            )
            .await;

        match result {
            Ok(content) => {
                let _ = app_clone.emit(
                    "plugin_assistant_complete",
                    serde_json::json!({
                        "stream_id": sid,
                        "content": content
                    }),
                );
            }
            Err(e) => {
                let _ = app_clone.emit(
                    "plugin_assistant_error",
                    serde_json::json!({
                        "stream_id": sid,
                        "error": e.to_string()
                    }),
                );
            }
        }
    });

    Ok(stream_id)
}

// 取消 AI 助手对话
#[tauri::command]
pub async fn cancel_plugin_assistant_chat(
    stream_id: String,
    app_handle: AppHandle,
) -> Result<(), String> {
    cancel_conversation_stream(&stream_id);
    let _ = app_handle.emit(
        "plugin_assistant_cancelled",
        &serde_json::json!({ "stream_id": stream_id }),
    );
    Ok(())
}

// 打印所有AI对话消息
#[tauri::command]
pub async fn print_ai_conversations(app: AppHandle) -> Result<String, String> {
    // 获取数据库服务
    let db = match app.try_state::<Arc<crate::services::database::DatabaseService>>() {
        Some(db) => db,
        None => return Err("Database service not initialized".to_string()),
    };

    // 获取所有对话
    let conversations = match db.get_ai_conversations().await {
        Ok(convs) => convs,
        Err(e) => return Err(format!("Failed to get conversation list: {}", e)),
    };

    // 如果没有对话，返回提示信息
    if conversations.is_empty() {
        return Ok("No AI conversation records found".to_string());
    }

    // 构建输出字符串
    let mut output = String::new();
    output.push_str(&format!(
        "Found {} AI conversations\n\n",
        conversations.len()
    ));

    // 遍历每个对话
    for (idx, conv) in conversations.iter().enumerate() {
        output.push_str(&format!(
            "对话 {}/{}: {} (ID: {})\n",
            idx + 1,
            conversations.len(),
            conv.title.as_deref().unwrap_or("No title"),
            conv.id
        ));
        output.push_str(&format!(
            "Created time: {}\n",
            conv.created_at.format("%Y-%m-%d %H:%M:%S")
        ));
        output.push_str(&format!(
            "Model: {} ({})\n",
            conv.model_name,
            conv.model_provider.as_deref().unwrap_or("Unknown")
        ));
        output.push_str(&format!("Message count: {}\n", conv.total_messages));
        output.push_str("------------------------------------\n");

        // 获取此对话的所有消息
        let messages = match db.get_ai_messages_by_conversation(&conv.id).await {
            Ok(msgs) => msgs,
            Err(e) => {
                output.push_str(&format!("Failed to get messages: {}\n\n", e));
                continue;
            }
        };

        // 打印每条消息
        for (msg_idx, msg) in messages.iter().enumerate() {
            let role_str = match msg.role.as_str() {
                "user" => "User",
                "assistant" => "Assistant",
                "system" => "System",
                "tool" => "Tool",
                _ => msg.role.as_str(),
            };

            output.push_str(&format!(
                "Message {}/{} - {} ({})\n",
                msg_idx + 1,
                messages.len(),
                role_str,
                msg.timestamp.format("%H:%M:%S")
            ));

            // 打印消息内容
            output.push_str(&format!("{}\n", msg.content));

            // 如果有Token使用情况，打印出来
            if let Some(token_count) = msg.token_count {
                output.push_str(&format!("Token usage: {}\n", token_count));

                if let Some(cost) = msg.cost {
                    output.push_str(&format!("Cost: ${:.6}\n", cost));
                }
            }

            // 如果有工具调用情况，打印出来
            if let Some(tool_calls_json) = &msg.tool_calls {
                // 尝试解析JSON
                if let Ok(tool_calls) =
                    serde_json::from_str::<Vec<serde_json::Value>>(tool_calls_json)
                {
                    if !tool_calls.is_empty() {
                        let tool_names: Vec<String> = tool_calls
                            .iter()
                            .filter_map(|t| {
                                t.get("name").and_then(|n| n.as_str()).map(String::from)
                            })
                            .collect();

                        if !tool_names.is_empty() {
                            output.push_str(&format!("Used tools: {}\n", tool_names.join(", ")));
                        }
                    }
                }
            }

            output.push_str("------------------------------------\n");
        }

        output.push_str("\n\n");
    }

    Ok(output)
}

#[derive(Debug, Serialize)]
pub struct ProviderUsageStats {
    pub input_tokens: f64,
    pub output_tokens: f64,
    pub total_tokens: f64,
    pub cost: f64,
}

/// 聚合全局 AI 用量统计（按 provider 分组）
#[tauri::command]
pub async fn get_ai_usage_stats(
    db: tauri::State<'_, Arc<DatabaseService>>,
) -> Result<std::collections::HashMap<String, ProviderUsageStats>, String> {
    let aggregated = db.get_aggregated_ai_usage()
        .await
        .map_err(|e| format!("Failed to get aggregated usage stats: {}", e))?;

    let mut map = std::collections::HashMap::new();
    for (provider, stats) in aggregated {
        map.insert(
            provider,
            ProviderUsageStats {
                input_tokens: stats.input_tokens as f64,
                output_tokens: stats.output_tokens as f64,
                total_tokens: stats.total_tokens as f64,
                cost: stats.cost,
            },
        );
    }

    Ok(map)
}

#[tauri::command]
pub async fn get_detailed_ai_usage_stats(
    db: tauri::State<'_, Arc<DatabaseService>>,
) -> Result<Vec<sentinel_core::models::database::AiUsageStats>, String> {
    db.get_ai_usage_stats()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_ai_usage_stats(
    db: tauri::State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM ai_usage_stats")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// 添加AI服务
#[tauri::command]
pub async fn add_ai_service(
    name: String,
    config: CommandAiConfig,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<(), String> {
    ai_manager
        .add_service(name, config.into())
        .await
        .map_err(|e| e.to_string())
}

// 移除AI服务
#[tauri::command]
pub async fn remove_ai_service(
    name: String,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<bool, String> {
    Ok(ai_manager.remove_service(&name))
}

// 创建AI对话
#[tauri::command]
pub async fn create_ai_conversation(
    request: CreateConversationRequest,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<String, String> {
    if let Some(service) = ai_manager.get_service(&request.service_name) {
        service
            .create_conversation(request.title)
            .await
            .map_err(|e| e.to_string())
    } else {
        // Fallback or default service creation
        if let Some(default_service) = ai_manager.get_service("default") {
            return default_service
                .create_conversation(request.title)
                .await
                .map_err(|e| e.to_string());
        }
        Err(format!(
            "AI service '{}' not found and no default service is available.",
            request.service_name
        ))
    }
}

// 仅保存AI消息到对话（不触发模型回复）
#[tauri::command]
pub async fn save_ai_message(
    request: SaveMessageRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    use sentinel_core::models::database as core_db;
    let message = core_db::AiMessage {
        id: request.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        conversation_id: request.conversation_id,
        role: request.role,
        content: request.content,
        metadata: request
            .metadata
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok()),
        token_count: None,
        cost: None,
        tool_calls: None,
        attachments: None,
        reasoning_content: None,
        timestamp: Utc::now(),
        architecture_type: request.architecture_type,
        architecture_meta: request.architecture_meta,
        structured_data: request.structured_data,
    };

    db.create_ai_message(&message)
        .await
        .map_err(|e| e.to_string())
}

// 删除AI对话
#[tauri::command]
pub async fn delete_ai_conversation(
    conversation_id: String,
    service_name: String,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<(), String> {
    if let Some(service) = ai_manager.get_service(&service_name) {
        return service
            .delete_conversation(&conversation_id)
            .await
            .map_err(|e| e.to_string());
    }
    Err(format!(
        "AI service '{}' not found for deleting conversation.",
        service_name
    ))
}

// 更新对话标题
#[tauri::command]
pub async fn update_ai_conversation_title(
    conversation_id: String,
    title: String,
    service_name: String,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<(), String> {
    if let Some(service) = ai_manager.get_service(&service_name) {
        return service
            .update_conversation_title(&conversation_id, &title)
            .await
            .map_err(|e| e.to_string());
    }
    Err("AI service not found".to_string())
}

// 归档对话
#[tauri::command]
pub async fn archive_ai_conversation(
    conversation_id: String,
    service_name: String,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<(), String> {
    if let Some(service) = ai_manager.get_service(&service_name) {
        return service
            .archive_conversation(&conversation_id)
            .await
            .map_err(|e| e.to_string());
    }
    Err("AI service not found".to_string())
}

// 获取AI对话列表
#[tauri::command]
pub async fn get_ai_conversations(
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<Vec<AiConversation>, String> {
    // 获取默认服务的对话列表
    let services = ai_manager.list_services();
    if let Some(service_name) = services.first() {
        if let Some(service) = ai_manager.get_service(service_name) {
            return service
                .list_conversations()
                .await
                .map_err(|e| e.to_string());
        }
    }
    Ok(vec![])
}

// 分页获取AI对话列表
#[tauri::command]
pub async fn get_ai_conversations_paginated(
    limit: i64,
    offset: i64,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<Vec<AiConversation>, String> {
    let services = ai_manager.list_services();
    if let Some(service_name) = services.first() {
        if let Some(service) = ai_manager.get_service(service_name) {
            return service
                .list_conversations_paginated(limit, offset)
                .await
                .map_err(|e| e.to_string());
        }
    }
    Ok(vec![])
}

// 获取AI对话总数
#[tauri::command]
pub async fn get_ai_conversations_count(
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<i64, String> {
    let services = ai_manager.list_services();
    if let Some(service_name) = services.first() {
        if let Some(service) = ai_manager.get_service(service_name) {
            return service
                .get_conversations_count()
                .await
                .map_err(|e| e.to_string());
        }
    }
    Ok(0)
}

// 获取对话历史
#[tauri::command(rename_all = "snake_case")]
pub async fn get_ai_conversation_history(
    conversation_id: String,
    service_name: String,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<Vec<AiMessage>, String> {
    if let Some(service) = ai_manager.get_service(&service_name) {
        return service
            .get_conversation_history(&conversation_id)
            .await
            .map_err(|e| e.to_string());
    }
    Err(format!(
        "AI service '{}' not found for getting conversation history.",
        service_name
    ))
}

// 删除单条AI消息（按消息ID）
#[tauri::command]
pub async fn delete_ai_message(
    message_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db_service
        .delete_ai_message(&message_id)
        .await
        .map_err(|e: anyhow::Error| format!("Failed to delete AI message {}: {}", message_id, e))
}

// Delete all messages after a specific message
#[tauri::command]
pub async fn delete_ai_messages_after(
    conversation_id: String,
    message_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<u64, String> {
    let deleted = db_service
        .delete_ai_messages_after(&conversation_id, &message_id)
        .await
        .map_err(|e: anyhow::Error| format!("Failed to delete messages after {}: {}", message_id, e))?;

    // Resend semantics delete tail messages; clear run_state to avoid stale "Recent Tool Digests".
    if let Ok(pool) = db_service.get_pool() {
        // Keep schema in sync with checkpoint storage.
        if let Err(e) = sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_run_states (
                execution_id TEXT PRIMARY KEY,
                state_json TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            )"#,
        )
        .execute(pool)
        .await
        {
            tracing::warn!("Failed to ensure agent_run_states table exists: {}", e);
        } else if let Err(e) = sqlx::query("DELETE FROM agent_run_states WHERE execution_id = ?")
            .bind(&conversation_id)
            .execute(pool)
            .await
        {
            tracing::warn!("Failed to clear agent run_state for {}: {}", conversation_id, e);
        }
    }

    Ok(deleted)
}

// 获取会话的所有消息
#[tauri::command]
pub async fn get_ai_messages_by_conversation(
    conversation_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<AiMessage>, String> {
    db_service
        .get_ai_messages_by_conversation(&conversation_id)
        .await
        .map_err(|e| {
            format!(
                "Failed to get messages for conversation {}: {}",
                conversation_id, e
            )
        })
}

#[tauri::command]
pub async fn get_subagent_runs(
    parent_execution_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<SubagentRun>, String> {
    db_service
        .get_subagent_runs_by_parent_internal(&parent_execution_id)
        .await
        .map_err(|e| format!("Failed to get subagent runs for {}: {}", parent_execution_id, e))
}

#[tauri::command]
pub async fn get_subagent_messages(
    subagent_run_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<SubagentMessage>, String> {
    db_service
        .get_subagent_messages_by_run_internal(&subagent_run_id)
        .await
        .map_err(|e| format!("Failed to get subagent messages for {}: {}", subagent_run_id, e))
}

// 清空会话的所有消息
#[tauri::command]
pub async fn clear_conversation_messages(
    conversation_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db_service
        .delete_ai_messages_by_conversation(&conversation_id)
        .await
        .map_err(|e: anyhow::Error| {
            format!(
                "Failed to clear messages for conversation {}: {}",
                conversation_id, e
            )
        })?;

    // Also clear agent run state to prevent cross-run leakage (tool digests, etc).
    if let Ok(pool) = db_service.get_pool() {
        if let Err(e) = sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_run_states (
                execution_id TEXT PRIMARY KEY,
                state_json TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            )"#,
        )
        .execute(pool)
        .await
        {
            tracing::warn!("Failed to ensure agent_run_states table exists: {}", e);
        } else if let Err(e) = sqlx::query("DELETE FROM agent_run_states WHERE execution_id = ?")
            .bind(&conversation_id)
            .execute(pool)
            .await
        {
            tracing::warn!("Failed to clear agent run_state for {}: {}", conversation_id, e);
        }
    }

    Ok(())
}

/// 保存全局工具配置（不与会话绑定）
#[tauri::command]
pub async fn save_tool_config(
    tool_config: crate::agents::ToolConfig,
    app_handle: AppHandle,
) -> Result<(), String> {
    save_tool_config_to_db(&app_handle, &tool_config).await
}

/// 获取全局工具配置
#[tauri::command]
pub async fn get_tool_config(
    app_handle: AppHandle,
) -> Result<Option<crate::agents::ToolConfig>, String> {
    Ok(load_tool_config_from_db(&app_handle).await)
}



/// 通过自然语言描述生成工作流图
#[tauri::command(rename_all = "snake_case")]
pub async fn generate_workflow_from_nl(
    description: String,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    traffic_state: State<'_, TrafficAnalysisState>,
) -> Result<WorkflowGraph, String> {
    let desc = description.trim();
    if desc.is_empty() {
        return Err("description is empty".to_string());
    }

    // Use default LLM AI configuration
    let ai_config = if let Some((provider, model)) = ai_manager
        .get_default_llm_model()
        .await
        .map_err(|e| e.to_string())?
    {
        let mut config = ai_manager
            .get_provider_config(&provider)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Provider '{}' not configured", provider))?;
        config.model = model;
        config
    } else {
        return Err("No default LLM model configured".to_string());
    };

    // 将 AiConfig 转换为 AiService，再获取 LlmConfig
    let ai_service = sentinel_llm::AiService::new(ai_config);
    let llm_config = ai_service.to_llm_config();
    let llm_client = sentinel_llm::LlmClient::new(llm_config);

    // 可用工具与节点摘要（提高生成命中率）
    let tools_summary = match tool_commands::list_unified_tools().await {
        Ok(tools) => {
            let mut lines = Vec::new();
            for t in tools.into_iter().take(60) {
                let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                let cat = t.get("category").and_then(|v| v.as_str()).unwrap_or("misc");
                let desc = t.get("description").and_then(|v| v.as_str()).unwrap_or("");
                lines.push(format!("- {} ({}): {}", name, cat, desc));
            }
            lines.join("\n")
        }
        Err(_) => "".to_string(),
    };

    let catalog_summary = match tool_commands::build_node_catalog(traffic_state.inner()).await {
        Ok(catalog) => {
            let mut lines = Vec::new();
            for item in catalog.into_iter().take(80) {
                lines.push(format!(
                    "- {} [{}]: {}",
                    item.node_type, item.category, item.label
                ));
            }
            lines.join("\n")
        }
        Err(_) => "".to_string(),
    };

    let system_prompt = format!(
        r#"you are a workflow design assistant for Sentinel AI.
Based on the user's natural language description, output a WorkflowGraph that strictly conforms to the following JSON Schema.
Only output JSON, do not explain, do not include Markdown.

Available unified tool list (tool::<name> for tool nodes):
{}

Available node type list (node_type must be chosen from or based on its name):
{}

Schema:
{{
  "id": "string",
  "name": "string",
  "version": "string",
  "nodes": [
    {{
      "id": "string",
      "node_type": "string",
      "node_name": "string",
      "x": number,
      "y": number,
      "params": object,
      "input_ports": [{{"id":"string","name":"string","port_type":"String|Integer|Float|Boolean|Json|Array|Object|Artifact","required":boolean}}],
      "output_ports": [{{"id":"string","name":"string","port_type":"String|Integer|Float|Boolean|Json|Array|Object|Artifact","required":boolean}}]
    }}
  ],
  "edges": [
    {{"id":"string","from_node":"string","from_port":"string","to_node":"string","to_port":"string"}}
  ],
  "variables": [],
  "credentials": []
}}

规则：
1) Use a concise node_type, use "tool::<name>" if it corresponds to a built-in tool, use "llm::completion" if it needs AI reasoning.
2) For each node, give node_name and a brief params.description.
3) At least one entry node (node_type can be "start").
4) Give a reasonable x/y layout (from left to right).
"#,
        tools_summary, catalog_summary
    );

    let user_prompt = format!("用户描述：{}\n请生成 WorkflowGraph JSON。", desc);

    let raw = llm_client
        .completion(Some(&system_prompt), &user_prompt)
        .await
        .map_err(|e| e.to_string())?;

    let json_str = raw.trim();
    let parsed_value: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e0) => {
            if let (Some(s), Some(e)) = (json_str.find('{'), json_str.rfind('}')) {
                serde_json::from_str(&json_str[s..=e])
                    .map_err(|e| format!("Failed to parse extracted JSON: {}", e))?
            } else {
                return Err(format!("Failed to parse LLM output as JSON: {}", e0));
            }
        }
    };

    let mut graph: WorkflowGraph = serde_json::from_value(parsed_value)
        .map_err(|e| format!("Failed to parse workflow graph: {}", e))?;

    if graph.id.trim().is_empty() {
        graph.id = format!("wf_{}", Utc::now().timestamp_millis());
    }
    if graph.name.trim().is_empty() {
        graph.name = "AI生成工作流".to_string();
    }
    if graph.version.trim().is_empty() {
        graph.version = "0.1.0".to_string();
    }
    // 确保变量/凭据字段存在
    if graph.variables.is_empty() {
        graph.variables = vec![];
    }
    if graph.credentials.is_empty() {
        graph.credentials = vec![];
    }

    Ok(graph)
}

/// 生成默认的AI提供商配置（与前端 `AiProviderConfig` 结构兼容）



// 保存模型配置配置
#[tauri::command]
pub async fn save_scheduler_config(
    config: crate::services::ai::SchedulerConfig,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    tracing::info!("Saving scheduler configuration");

    // 保存各个阶段的模型配置
    db.set_config(
        "scheduler",
        "intent_analysis_model",
        &config.intent_analysis_model,
        Some("Intent analysis model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "intent_analysis_provider",
        &config.intent_analysis_provider,
        Some("Intent analysis provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "planner_model",
        &config.planner_model,
        Some("Planner model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "planner_provider",
        &config.planner_provider,
        Some("Planner provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "replanner_model",
        &config.replanner_model,
        Some("Replanner model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "replanner_provider",
        &config.replanner_provider,
        Some("Replanner provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "executor_model",
        &config.executor_model,
        Some("Executor model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "executor_provider",
        &config.executor_provider,
        Some("Executor provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "evaluator_model",
        &config.evaluator_model,
        Some("Evaluator model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "evaluator_provider",
        &config.evaluator_provider,
        Some("Evaluator provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    // 保存默认重规划策略
    db.set_config(
        "scheduler",
        "default_strategy",
        &config.default_strategy,
        Some("Default replanning strategy"),
    )
    .await
    .map_err(|e| e.to_string())?;

    // 保存启用状态
    db.set_config(
        "scheduler",
        "enabled",
        &config.enabled.to_string(),
        Some("Scheduler enabled status"),
    )
    .await
    .map_err(|e| e.to_string())?;

    // 保存最大重试次数
    db.set_config(
        "scheduler",
        "max_retries",
        &config.max_retries.to_string(),
        Some("Maximum retry attempts"),
    )
    .await
    .map_err(|e| e.to_string())?;

    // 保存超时时间
    db.set_config(
        "scheduler",
        "timeout_seconds",
        &config.timeout_seconds.to_string(),
        Some("Timeout in seconds"),
    )
    .await
    .map_err(|e| e.to_string())?;

    // 保存场景配置
    let scenarios_str = serde_json::to_string(&config.scenarios)
        .map_err(|e| format!("Failed to serialize scenarios: {}", e))?;
    db.set_config(
        "scheduler",
        "scenarios",
        &scenarios_str,
        Some("Scenario configurations"),
    )
    .await
    .map_err(|e| e.to_string())?;

    tracing::info!("Successfully saved scheduler configuration");
    Ok(())
}


#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HandleTaskExecutionStreamRequest {
    pub user_input: String,
    pub conversation_id: String,
    pub message_id: String,
    pub execution_id: String,
}

// LM Studio相关的命令

/// 刷新LM Studio模型列表
/// DISABLED (ai_adapter removed)
#[tauri::command]
pub async fn refresh_lm_studio_models(
    _api_base: Option<String>,
    _api_key: Option<String>,
) -> Result<Vec<String>, String> {
    Err("LM Studio model refresh disabled - ai_adapter removed, use Rig instead".to_string())
}

/// 获取LM Studio服务器状态 - DISABLED (ai_adapter removed)
#[tauri::command]
pub async fn get_lm_studio_status(
    _api_base: Option<String>,
    _api_key: Option<String>,
) -> Result<serde_json::Value, String> {
    Err("LM Studio status check disabled - ai_adapter removed, use Rig instead".to_string())
}

/// 测试LM Studio提供商连接 - DISABLED (ai_adapter removed)
#[tauri::command]
pub async fn test_lm_studio_provider_connection(
    _api_base: Option<String>,
    _api_key: Option<String>,
) -> Result<TestConnectionResponse, String> {
    Err("LM Studio provider test disabled - ai_adapter removed, use Rig instead".to_string())
}

/// 上传图片文件并转换为 base64
#[tauri::command]
pub async fn upload_image_attachment(file_path: String) -> Result<serde_json::Value, String> {
    use crate::models::attachment::{load_image_from_path, MessageAttachment};

    tracing::info!("上传图片附件: {}", file_path);

    match load_image_from_path(&file_path).await {
        Ok(image_attachment) => {
            let attachment = MessageAttachment::Image(image_attachment);
            serde_json::to_value(&attachment).map_err(|e| format!("序列化图片附件失败: {}", e))
        }
        Err(e) => {
            tracing::error!("加载图片失败: {}", e);
            Err(format!("加载图片失败: {}", e))
        }
    }
}

/// 批量上传图片文件
#[tauri::command]
pub async fn upload_multiple_images(
    file_paths: Vec<String>,
) -> Result<Vec<serde_json::Value>, String> {
    use crate::models::attachment::{load_image_from_path, MessageAttachment};

    tracing::info!("批量上传 {} 个图片", file_paths.len());

    let mut attachments = Vec::new();
    let mut errors = Vec::new();

    for file_path in file_paths {
        match load_image_from_path(&file_path).await {
            Ok(image_attachment) => {
                let attachment = MessageAttachment::Image(image_attachment);
                if let Ok(value) = serde_json::to_value(&attachment) {
                    attachments.push(value);
                } else {
                    errors.push(format!("序列化失败: {}", file_path));
                }
            }
            Err(e) => {
                errors.push(format!("{}: {}", file_path, e));
            }
        }
    }

    if !errors.is_empty() {
        tracing::warn!("部分图片上传失败: {:?}", errors);
    }

    if attachments.is_empty() {
        Err(format!("所有图片上传失败: {:?}", errors))
    } else {
        Ok(attachments)
    }
}

// ============== Agent Execute Command ==============

/// Agent执行配置
#[derive(Debug, Clone, Deserialize)]
pub struct AgentExecuteConfig {
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
    pub enable_rag: Option<bool>,
    /// Image attachments (base64 for LLM vision)
    pub attachments: Option<serde_json::Value>,
    /// Document attachments (content or security mode)
    #[serde(default)]
    pub document_attachments: Option<Vec<crate::commands::document_commands::ProcessedDocumentResult>>,
    #[serde(default)]
    pub tool_config: Option<crate::agents::ToolConfig>,
    /// Traffic context to prepend to the task (not shown in user message display)
    #[serde(default)]
    pub traffic_context: Option<String>,
    /// Display content for user message (if different from full task)
    #[serde(default)]
    pub display_content: Option<String>,
    // 以下字段用于兼容前端，但在此函数中不使用
    #[serde(default)]
    pub max_iterations: Option<usize>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub force_todos: Option<bool>,
    #[serde(default)]
    pub enable_tenth_man_rule: Option<bool>,
    #[serde(default)]
    pub tenth_man_config: Option<crate::agents::tenth_man::TenthManConfig>,
}

/// Agent执行请求
#[derive(Debug, Clone, Deserialize)]
pub struct AgentExecuteRequest {
    pub task: String,
    pub config: Option<AgentExecuteConfig>,
}

fn sanitize_image_attachments(attachments: &serde_json::Value) -> serde_json::Value {
    fn sanitize_one(v: &mut serde_json::Value) {
        // Expected: { type: "image", ... } or legacy { image: { ... } }
        let img = if v.get("type").and_then(|t| t.as_str()) == Some("image") {
            Some(v)
        } else {
            v.get_mut("image")
        };
        let Some(img) = img else { return };
        if let Some(obj) = img.as_object_mut() {
            obj.remove("source_path");
        }
    }

    let mut cloned = attachments.clone();
    if let Some(arr) = cloned.as_array_mut() {
        for item in arr.iter_mut() {
            sanitize_one(item);
        }
    } else if cloned.is_object() {
        sanitize_one(&mut cloned);
    }
    cloned
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectiveImageAttachmentMode {
    LocalOcr,
    ModelVision,
}

async fn load_image_attachment_settings(
    db: &crate::services::database::DatabaseService,
) -> (EffectiveImageAttachmentMode, bool) {
    let mode_str = db
        .get_config("agent", "image_attachment_mode")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "local_ocr".to_string());
    let mode = if mode_str == "model_vision" {
        EffectiveImageAttachmentMode::ModelVision
    } else {
        EffectiveImageAttachmentMode::LocalOcr
    };

    let allow_upload = db
        .get_config("agent", "allow_image_upload_to_model")
        .await
        .ok()
        .flatten()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    (mode, allow_upload)
}

/// Agent执行 - 统一的聊天入口，支持流式输出、联网搜索、RAG知识检索
#[tauri::command]
pub async fn agent_execute(
    task: String,
    config: Option<AgentExecuteConfig>,
    app_handle: AppHandle,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<String, String> {
    // Multi-point license verification
    #[cfg(not(debug_assertions))]
    if !sentinel_license::is_licensed() {
        return Err("License required for this feature".to_string());
    }

    let config = config.unwrap_or(AgentExecuteConfig {
        conversation_id: None,
        message_id: None,
        enable_rag: Some(false),
        attachments: None,
        document_attachments: None,
        tool_config: None,
        traffic_context: None,
        display_content: None,
        max_iterations: None,
        timeout_secs: None,
        force_todos: None,
        enable_tenth_man_rule: None,
        tenth_man_config: None,
    });

    let conversation_id = config
        .conversation_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let message_id = config
        .message_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let enable_rag = config.enable_rag.unwrap_or(false);
    let raw_attachments = config.attachments.clone();
    let attachments_for_save = raw_attachments
        .as_ref()
        .map(|v| sanitize_image_attachments(v));
    let document_attachments_for_save = config.document_attachments.clone();

    // 获取工具配置：优先使用前端传递的配置，否则从数据库加载
    let effective_tool_config = if config.tool_config.is_some() {
        tracing::info!("Using tool config from frontend request");
        config.tool_config.clone()
    } else {
        load_tool_config_from_db(&app_handle).await
    };

    tracing::info!(
        "Agent execute: conv={}, msg={}, rag={}, tools={}",
        conversation_id,
        message_id,
        enable_rag,
        effective_tool_config
            .as_ref()
            .map(|c| c.enabled)
            .unwrap_or(false)
    );

    // 获取默认模型配置
    let (provider, model_name) = match ai_manager.get_default_llm_model().await {
        Ok(Some((p, m))) => {
            tracing::info!("Using default chat model: {}/{}", p, m);
            (p, m)
        }
        Ok(None) => return Err("Default chat model is not configured".to_string()),
        Err(e) => return Err(format!("Failed to read default chat model: {}", e)),
    };

    // 获取provider配置
    let provider_config = ai_manager
        .get_provider_config(&provider)
        .await
        .map_err(|e| format!("Failed to load provider config '{}': {}", provider, e))?
        .ok_or_else(|| format!("Provider '{}' configuration not found", provider))?;

    let mut dynamic_config = provider_config.clone();
    dynamic_config.model = model_name.clone();

    let db_service = app_handle.state::<Arc<crate::services::database::DatabaseService>>();
    let (image_mode, allow_image_upload_to_model) =
        load_image_attachment_settings(db_service.inner()).await;
    let mut service = crate::services::ai::AiService::new(
        dynamic_config,
        db_service.inner().clone(),
        Some(app_handle.clone()),
    );
    service.set_app_handle(app_handle.clone());

    // 从数据库读取并应用输出存储阈值配置（动态上下文发现）
    if let Ok(threshold_str_opt) = db_service.get_config_internal("ai", "output_storage_threshold").await {
        if let Some(threshold_str) = threshold_str_opt {
             if let Ok(threshold) = threshold_str.parse::<usize>() {
                 tracing::info!("Setting output storage threshold to {} bytes (Dynamic Context Discovery)", threshold);
                 sentinel_tools::set_storage_threshold(threshold);
             }
        }
    }

    // 创建取消令牌
    let _cancellation_token = create_cancellation_token(&conversation_id);

    let service_clone = service.clone();
    let conv_id = conversation_id.clone();
    let msg_id = message_id.clone();
    let task_clone = task.clone();
    let display_content_clone = config.display_content.clone();
    // 从数据库直接读取系统提示词，不依赖前端传递
    let mut base_system_prompt: Option<String> = None;

    // 为了在闭包中使用，需要克隆这些值
    let provider_for_closure = if provider_config.rig_provider.is_some() {
        provider_config.rig_provider.clone().unwrap()
    } else {
        provider_config.provider.clone()
    };

    let model_name_for_closure = model_name.clone();
    let provider_config_for_closure = provider_config.clone();

    tokio::spawn(async move {
        let _guard = CancellationGuard(conv_id.clone());
        // 确保会话存在并保存用户消息
        if let Some(db) = app_handle.try_state::<Arc<crate::services::database::DatabaseService>>()
        {
            // 检查会话是否存在
            let conversation_exists = db
                .get_ai_conversation(&conv_id)
                .await
                .map(|c| c.is_some())
                .unwrap_or(false);

            if !conversation_exists {
                // 创建新会话
                use sentinel_core::models::database as core_db;
                let new_conv = core_db::AiConversation {
                    id: conv_id.clone(),
                    title: Some(task_clone.chars().take(50).collect::<String>()),
                    service_name: "default".to_string(),
                    model_name: "default".to_string(),
                    model_provider: None,
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
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };
                if let Err(e) = db.create_ai_conversation(&new_conv).await {
                    tracing::warn!("Failed to create conversation: {}", e);
                }
            }

            // 保存用户消息
            // Use display_content for UI display, but save full task to database
            use sentinel_core::models::database as core_db;
            let user_msg_id = Uuid::new_v4().to_string();
            let display_text = display_content_clone.as_ref().unwrap_or(&task_clone);
            
            // Build structured_data with display_content and document_attachments
            let structured_data = {
                let mut data = serde_json::json!({});
                if let Some(ref content) = display_content_clone {
                    data["display_content"] = serde_json::json!(content);
                }
                if let Some(ref doc_atts) = document_attachments_for_save {
                    if !doc_atts.is_empty() {
                        data["document_attachments"] = serde_json::to_value(doc_atts).unwrap_or_default();
                    }
                }
                if data.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                    None
                } else {
                    Some(data.to_string())
                }
            };
            
            // Build metadata with image attachments and document_attachments
            let metadata = {
                let mut meta = serde_json::json!({});
                if let Some(ref atts) = attachments_for_save {
                    meta["image_attachments"] = atts.clone();
                }
                if let Some(ref doc_atts) = document_attachments_for_save {
                    if !doc_atts.is_empty() {
                        meta["document_attachments"] = serde_json::to_value(doc_atts).unwrap_or_default();
                    }
                }
                if meta.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                    None
                } else {
                    Some(meta.to_string())
                }
            };
            
            let user_msg = core_db::AiMessage {
                id: user_msg_id.clone(),
                conversation_id: conv_id.clone(),
                role: "user".to_string(),
                content: task_clone.clone(),
                metadata,
                token_count: Some(task_clone.len() as i32),
                cost: None,
                tool_calls: None,
                attachments: attachments_for_save
                    .as_ref()
                    .and_then(|v| serde_json::to_string(v).ok()),
                reasoning_content: None,
                timestamp: chrono::Utc::now(),
                architecture_type: None,
                architecture_meta: None,
                structured_data,
            };
            if let Err(e) = db.create_ai_message(&user_msg).await {
                tracing::warn!("Failed to save user message: {}", e);
            } else {
                // 发送用户消息到前端 (use display_text for UI)
                let _ = app_handle.emit(
                    "agent:user_message",
                    &serde_json::json!({
                        "execution_id": conv_id,
                        "message_id": user_msg_id,
                        "content": display_text,
                        "timestamp": user_msg.timestamp.timestamp_millis(),
                        "document_attachments": document_attachments_for_save,
                        "image_attachments": attachments_for_save,
                    }),
                );
            }
        }

        // 获取当前角色提示
        let mut role_prompt = String::new();
        if let Some(db) = app_handle.try_state::<Arc<crate::services::database::DatabaseService>>()
        {
            if let Ok(Some(current_role)) = db.get_current_ai_role().await {
                if !current_role.prompt.trim().is_empty() {
                    role_prompt = current_role.prompt;
                    tracing::info!("Using role prompt: {}", current_role.title);
                }
            }
        }

        // 合并角色提示和系统提示
        if !role_prompt.is_empty() {
            base_system_prompt = match base_system_prompt {
                Some(existing) if !existing.trim().is_empty() => {
                    Some(format!("{}\n\n{}", role_prompt, existing))
                }
                _ => Some(role_prompt),
            };
        }

        // Image attachments processing (default: local OCR, do not upload images)
        let mut augmented_task = task_clone.clone();

        // If shell runs in Docker, stage images into container and expose paths to the LLM.
        // This enables the model to access images via shell tool in container (e.g. /workspace/context/attachments/...).
        if let Some(ref raw) = raw_attachments {
            let shell_cfg = sentinel_tools::buildin_tools::shell::get_shell_config().await;
            if shell_cfg.default_execution_mode == sentinel_tools::buildin_tools::shell::ShellExecutionMode::Docker {
                match crate::utils::image_ocr::stage_images_to_docker_context(raw).await {
                    Ok(paths) => {
                        if !paths.is_empty() {
                            let mut lines = Vec::new();
                            for p in paths {
                                let name = p
                                    .filename
                                    .as_deref()
                                    .filter(|s| !s.trim().is_empty())
                                    .unwrap_or("image");
                                lines.push(format!("- {}: {}", name, p.container_path));
                            }
                            augmented_task = format!(
                                "[Image Files in Docker]\n{}\n\n{}",
                                lines.join("\n"),
                                augmented_task
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to stage images to docker context: {}", e);
                    }
                }
            }
        }

        let effective_mode = if image_mode == EffectiveImageAttachmentMode::ModelVision && allow_image_upload_to_model {
            EffectiveImageAttachmentMode::ModelVision
        } else {
            EffectiveImageAttachmentMode::LocalOcr
        };

        let image_attachments_for_execution: Option<serde_json::Value> = match effective_mode {
            EffectiveImageAttachmentMode::LocalOcr => {
                if let Some(ref raw) = raw_attachments {
                    match crate::utils::image_ocr::ocr_images_from_attachments(raw).await {
                        Ok(results) => {
                            let ctx = crate::utils::image_ocr::format_ocr_context(&results, 8000);
                            if !ctx.trim().is_empty() {
                                augmented_task = format!("[Image OCR]\n{}\n\n{}", ctx, augmented_task);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Image OCR failed: {}", e);
                        }
                    }
                }
                None
            }
            EffectiveImageAttachmentMode::ModelVision => {
                // Only send sanitized attachments to model (no local path)
                attachments_for_save.clone()
            }
        };

        // RAG 知识检索增强
        if enable_rag {
            if let Some(db) =
                app_handle.try_state::<Arc<crate::services::database::DatabaseService>>()
            {
                // 获取对话历史用于查询重写
                let history_messages = match db.get_ai_messages_by_conversation(&conv_id).await {
                    Ok(msgs) => msgs,
                    Err(_) => Vec::new(),
                };

                // 使用显示文本（原始查询）进行 RAG，而不是包含流量上下文的完整任务
                let rag_query = display_content_clone.as_ref().unwrap_or(&task_clone);

                // 执行统一的 RAG 增强逻辑
                match perform_rag_enhancement(&app_handle, &conv_id, rag_query, &history_messages, None).await {
                    Ok((context, citations)) => {
                        if !context.trim().is_empty() {
                            let base = base_system_prompt.unwrap_or_default();
                            let policy = "you must strictly answer the question based on the evidence. When citing evidence in your response, use the [SOURCE n] format. If the evidence is insufficient, please answer directly and avoid fabricating. ";
                            let augmented = if base.trim().is_empty() {
                                format!("[rule of knowledge]\n{}\n\n[Source Evidence Block]\n{}", policy, context)
                            } else {
                                format!(
                                    "{}\n\n[rule of knowledge]\n{}\n\n[Source Evidence Block]\n{}",
                                    base, policy, context
                                )
                            };
                            base_system_prompt = Some(augmented);

                            let _ = app_handle.emit(
                                "ai_meta_info",
                                &serde_json::json!({
                                    "conversation_id": conv_id,
                                    "message_id": msg_id,
                                    "rag_applied": true,
                                    "rag_sources_used": !citations.is_empty(),
                                    "source_count": citations.len(),
                                    "citations": citations // 发送完整引用信息供前端交互
                                }),
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!("RAG enhancement failed in agent_execute: {}", e);
                    }
                }
            }
        }

        // 发送开始事件
        if let Err(e) = app_handle.emit(
            "ai_stream_start",
            &serde_json::json!({
                "conversation_id": conv_id,
                "message_id": msg_id
            }),
        ) {
            tracing::error!("Failed to emit stream start event: {}", e);
        }

        // 检查是否启用工具调用
        if let Some(ref tool_cfg) = effective_tool_config {
            if tool_cfg.enabled {
                // 使用工具支持的代理执行器
                tracing::info!(
                    "Using tool-enabled agent executor for conversation: {}",
                    conv_id
                );

                // 构建代理执行参数
                // Convert document attachments to executor format
                let doc_attachments = config.document_attachments.as_ref().map(|docs| {
                    docs.iter().map(|d| crate::agents::DocumentAttachmentInfo {
                        id: d.id.clone(),
                        original_filename: d.original_filename.clone(),
                        file_size: d.file_size,
                        mime_type: d.mime_type.clone(),
                        processing_mode: d.processing_mode.clone(),
                        extracted_text: d.extracted_text.clone(),
                        container_path: d.container_path.clone(),
                    }).collect()
                });
                
                let executor_params = crate::agents::executor::AgentExecuteParams {
                    execution_id: conv_id.clone(),
                    model: model_name_for_closure.clone(),
                    system_prompt: base_system_prompt.unwrap_or_default(),
                    task: augmented_task.clone(),
                    rig_provider: provider_for_closure.clone(),
                    api_key: provider_config_for_closure.api_key.clone(),
                    api_base: provider_config_for_closure.api_base.clone(),
                    max_iterations: config.max_iterations.unwrap_or(10),
                    timeout_secs: config.timeout_secs.unwrap_or(300),
                    tool_config: effective_tool_config.clone(),
                    enable_tenth_man_rule: config.enable_tenth_man_rule.unwrap_or(false),
                    tenth_man_config: config.tenth_man_config.clone(),
                    document_attachments: doc_attachments,
                    image_attachments: image_attachments_for_execution.clone(),
                    persist_messages: true,
                    subagent_run_id: None,
                    context_policy: None,
                };

                // 调用工具支持的代理执行器
                match crate::agents::executor::execute_agent(&app_handle, executor_params).await {
                    Ok(_) => {
                        tracing::info!("Agent with tools completed for conversation: {}", conv_id);
                        let _ = app_handle.emit(
                            "agent:complete",
                            &serde_json::json!({
                                "execution_id": conv_id,
                                "success": true
                            }),
                        );
                        
                        // Cleanup todos for this execution
                        sentinel_tools::buildin_tools::todos::cleanup_execution_todos(&conv_id).await;
                    }
                    Err(e) => {
                        tracing::error!("Agent with tools execution failed: {}", e);
                        let _ = app_handle.emit(
                            "agent:error",
                            &serde_json::json!({
                                "execution_id": conv_id,
                                "error": e.to_string()
                            }),
                        );
                        
                        // Cleanup todos for this execution even on error
                        sentinel_tools::buildin_tools::todos::cleanup_execution_todos(&conv_id).await;
                    }
                }

                remove_cancellation_token(&conv_id);
                return;
            }
        }

        // 流式调用 LLM（不使用工具）
        match stream_chat_with_llm(
            &service_clone,
            &app_handle,
            &conv_id,
            &msg_id,
            &augmented_task,
            base_system_prompt.as_deref(),
            image_attachments_for_execution,
            true,
        )
        .await
        {
            Ok(_) => {
                tracing::info!("Stream chat completed for conversation: {}", conv_id);
            }
            Err(e) => {
                tracing::error!("Stream chat failed: {}", e);
            }
        }

        remove_cancellation_token(&conv_id);
    });

    Ok(message_id)
}


/// 从数据库加载全局工具配置
async fn load_tool_config_from_db(app_handle: &AppHandle) -> Option<crate::agents::ToolConfig> {
    if let Some(db) = app_handle.try_state::<Arc<crate::services::database::DatabaseService>>() {
        // 从 config 表中读取工具配置
        if let Ok(Some(config_str)) = db.get_config("agent", "tool_config").await {
            if let Ok(config) = serde_json::from_str::<crate::agents::ToolConfig>(&config_str) {
                tracing::info!("Loaded global tool config from database");
                return Some(config);
            }
        }
    }

    // 如果没有配置，返回默认值
    tracing::info!("No global tool config found, using default");
    None
}

/// 保存全局工具配置到数据库
async fn save_tool_config_to_db(
    app_handle: &AppHandle,
    config: &crate::agents::ToolConfig,
) -> Result<(), String> {
    if let Some(db) = app_handle.try_state::<Arc<crate::services::database::DatabaseService>>() {
        let config_str = serde_json::to_string(config)
            .map_err(|e| format!("Failed to serialize tool config: {}", e))?;

        db.set_config(
            "agent",
            "tool_config",
            &config_str,
            Some("Global tool configuration"),
        )
        .await
        .map_err(|e| format!("Failed to save tool config: {}", e))?;

        tracing::info!("Saved global tool config to database");
        Ok(())
    } else {
        Err("Database service not available".to_string())
    }
}

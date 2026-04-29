use crate::agents::executor::message_store::{
    build_assistant_session_stats_metadata, mark_first_response_ms,
};
use crate::commands::ai_task_support::{
    build_virtual_tool_context, complete_external_profile_run_failure,
    complete_external_profile_run_success, load_external_profile_context,
    merge_external_profile_prompt, run_external_chat_task, start_external_profile_run,
};
use crate::commands::traffic::TrafficAnalysisState;
use crate::models::database::{AiMessage, SubagentMessage, SubagentRun};
use crate::services::ai::{AiConfig, AiServiceManager, AiServiceWrapper, AiToolCall};
use crate::services::database::DatabaseService;
use crate::services::SystemAgentRuntime;
use crate::utils::ai_generation_settings::apply_generation_settings_from_db;
use crate::utils::ordered_message::ChunkType;
use anyhow::Result;
use chrono::Utc;
use sentinel_db::Database;
use sentinel_llm::{
    normalize_tool_call_arguments_value, parse_images_from_json, ChatMessage as LlmChatMessage,
    StreamContent, StreamingLlmClient,
};
use sentinel_rag;
use sentinel_workflow::WorkflowGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Hard safety ceiling for output storage threshold.
/// UI recommends 8K-32K, but we still allow up to 50K for compatibility.
pub(crate) const MAX_SAFE_OUTPUT_STORAGE_THRESHOLD: usize = 50_000;
pub(crate) const USER_FORCED_RULES_CONFIG_CATEGORY: &str = "agent";
pub(crate) const USER_FORCED_RULES_CONFIG_KEY: &str = "user_forced_rules";

pub use crate::commands::ai_runtime_commands::{
    AgentExecuteConfig, AgentExecuteRequest, HandleTaskExecutionStreamRequest,
};

// Re-export AI settings related types for backward compatibility
pub use crate::commands::aisettings::{
    AddCustomProviderRequest, AiProviderConfig, SaveAiConfigRequest, SetDefaultProviderRequest,
    TestConnectionRequest, TestConnectionResponse,
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
    #[serde(default)]
    pub extra_headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub extra_body: Option<serde_json::Value>,
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
            extra_headers: c.extra_headers,
            extra_body: c.extra_body,
        }
    }
}

// 全局取消令牌管理器: maps conversation_id -> (token, generation counter)
static CANCELLATION_TOKENS: std::sync::LazyLock<Mutex<HashMap<String, (CancellationToken, u64)>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

static CANCELLATION_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

// 辅助函数：创建取消令牌，返回 (token, generation)
pub(crate) fn create_cancellation_token(conversation_id: &str) -> (CancellationToken, u64) {
    let token = CancellationToken::new();
    let gen = CANCELLATION_GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    if let Ok(mut tokens) = CANCELLATION_TOKENS.lock() {
        if let Some((old_token, _)) = tokens.remove(conversation_id) {
            old_token.cancel();
        }
        tokens.insert(conversation_id.to_string(), (token.clone(), gen));
    }
    (token, gen)
}

// 辅助函数：获取取消令牌
fn get_cancellation_token(conversation_id: &str) -> Option<CancellationToken> {
    if let Ok(tokens) = CANCELLATION_TOKENS.lock() {
        tokens.get(conversation_id).map(|(t, _)| t.clone())
    } else {
        None
    }
}

// 辅助函数：移除取消令牌（仅当 generation 匹配时才移除，避免竞态条件）
fn remove_cancellation_token(conversation_id: &str, generation: u64) {
    if let Ok(mut tokens) = CANCELLATION_TOKENS.lock() {
        if let Some((_, gen)) = tokens.get(conversation_id) {
            if *gen == generation {
                tokens.remove(conversation_id);
            }
        }
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

/// Guard that auto-removes its cancellation token on drop, only if the generation matches.
/// Prevents a stale task from removing a newer task's token.
pub(crate) struct CancellationGuard(pub String, pub u64);

impl Drop for CancellationGuard {
    fn drop(&mut self) {
        remove_cancellation_token(&self.0, self.1);
    }
}

/// 执行 RAG 增强：包含查询重写、多集合检索和配置透传
pub(crate) async fn perform_rag_enhancement(
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
        if let Ok(Some((provider, model))) = app_handle
            .state::<Arc<AiServiceManager>>()
            .get_default_llm_model()
            .await
        {
            // 获取 provider 配置
            if let Ok(Some(provider_cfg)) = app_handle
                .state::<Arc<AiServiceManager>>()
                .get_provider_config(&provider)
                .await
            {
                let llm_config = sentinel_llm::LlmConfig::new(&provider, &model)
                    .with_api_key(provider_cfg.api_key.as_deref().unwrap_or_default())
                    .with_base_url(provider_cfg.api_base.as_deref().unwrap_or_default());

                let llm_config = apply_generation_settings_from_db(db.as_ref(), llm_config).await;
                let client = sentinel_llm::LlmClient::new(llm_config);

                let rewrite_prompt = "you are a search query rewrite expert. Please rewrite the user's latest question into a independent and complete search query for retrieval in the vector database. If the user's question is already independent, return it as is. Only return the rewritten query text, no additional explanation.";
                let mut history_text = String::new();
                // 取最近几条历史
                for msg in history_messages.iter().rev().take(6).rev() {
                    history_text.push_str(&format!("{}: {}\n", msg.role, msg.content));
                }
                let input = format!(
                    "conversation history: \n{}\nuser question: {}",
                    history_text, user_message
                );

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
    let rag_service =
        crate::commands::rag_commands::get_or_init_rag_service(db.inner().clone()).await?;

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
        mmr_lambda: Some(effective_config.mmr_lambda as f64),
        similarity_threshold: Some(effective_config.similarity_threshold as f64),
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
    )
    .await
    {
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
                                let tool_result = metadata
                                    .get("tool_result")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                // 只在存在工具结果时才加入 tool_calls，避免 DeepSeek 要求的 tool_result 不足
                                if let Some(result_str) = tool_result {
                                    // 这是一个完成的工具调用
                                    let tool_call_id = metadata
                                        .get("tool_call_id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();

                                    // Skip duplicate tool_call_id
                                    if !tool_call_id.is_empty()
                                        && !seen_tool_call_ids.contains(&tool_call_id)
                                    {
                                        seen_tool_call_ids.insert(tool_call_id.clone());

                                        let tool_name = metadata
                                            .get("tool_name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let tool_args_raw = metadata
                                            .get("tool_args")
                                            .cloned()
                                            .unwrap_or(Value::Object(serde_json::Map::new()));
                                        // Normalize tool_args: some paths persist args as a JSON string.
                                        let tool_args = normalize_tool_call_arguments_value(
                                            &tool_name,
                                            tool_args_raw,
                                        );

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

                let has_content = !msg.content.trim().is_empty();
                let has_tool_calls = !tool_calls_json.is_empty();
                let has_reasoning = reasoning_content
                    .as_ref()
                    .is_some_and(|r| !r.trim().is_empty());

                if has_content || has_tool_calls || has_reasoning {
                    // 创建 assistant 消息
                    let mut chat_msg = LlmChatMessage::new("assistant", msg.content.as_str());

                    // 如果有工具调用，添加 tool_calls 和 reasoning_content
                    if has_tool_calls {
                        chat_msg.tool_calls =
                            Some(serde_json::to_string(&tool_calls_json).unwrap_or_default());
                        // Keep historical behavior: preserve field but don't force non-empty placeholder.
                        chat_msg.reasoning_content = Some(reasoning_content.unwrap_or_default());
                    } else if has_reasoning {
                        chat_msg.reasoning_content = reasoning_content;
                    }

                    result.push(chat_msg);
                }

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
pub(crate) async fn stream_chat_with_llm(
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
    let images = parse_images_from_json(attachments.as_ref());

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
    let llm_config =
        apply_generation_settings_from_db(db.as_ref(), service.service.to_llm_config()).await;
    let streaming_client = StreamingLlmClient::new(llm_config);

    // -------------------

    // 过滤空系统提示词，避免下游组装出空 text 触发 400/1214。
    let final_system_prompt = system_prompt
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    // 注意：RAG 增强逻辑已移至 agent_execute 中统一处理，通过 system_prompt 传入。
    // 这里保持 stream_chat_with_llm 职责单一，仅负责流式输出。

    // 流式调用
    let execution_id = conversation_id.to_string();
    let msg_id = message_id.to_string();
    let conv_id = conversation_id.to_string();
    let app = app_handle.clone();
    let execution_started_at_ms = chrono::Utc::now().timestamp_millis();

    // 记录用量
    let usage_data = Arc::new(std::sync::Mutex::new(None::<(u32, u32)>));
    let usage_data_clone = usage_data.clone();
    let first_response_ms = Arc::new(std::sync::Mutex::new(None::<i64>));
    let first_response_ms_clone = first_response_ms.clone();

    let content = streaming_client
        .stream_chat(
            final_system_prompt.as_deref(),
            user_message,
            &history,
            images.as_slice(),
            move |chunk| {
                if is_conversation_cancelled(&conv_id) {
                    return false;
                }
                match chunk {
                    StreamContent::Text(text) => {
                        mark_first_response_ms(
                            first_response_ms_clone.as_ref(),
                            execution_started_at_ms,
                        );
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
                        mark_first_response_ms(
                            first_response_ms_clone.as_ref(),
                            execution_started_at_ms,
                        );
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
                    StreamContent::Usage {
                        input_tokens,
                        output_tokens,
                    } => {
                        tracing::info!(
                            "Stream usage received: input={}, output={}",
                            input_tokens,
                            output_tokens
                        );
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
            metadata: build_assistant_session_stats_metadata(
                Some(chrono::Utc::now().timestamp_millis() - execution_started_at_ms),
                first_response_ms.lock().ok().and_then(|guard| *guard),
                Some(input_tokens),
                Some(output_tokens),
            )
            .as_ref()
            .and_then(|value| serde_json::to_string(value).ok()),
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
                let cost =
                    sentinel_llm::calculate_cost(provider, model, input_tokens, output_tokens);

                if let Err(e) = db
                    .update_ai_usage(
                        provider,
                        model,
                        input_tokens as i32,
                        output_tokens as i32,
                        cost,
                    )
                    .await
                {
                    tracing::warn!("Failed to update AI usage stats: {}", e);
                } else {
                    tracing::debug!(
                        "Updated AI usage: provider={}, model={}, input={}, output={}, cost=${:.4}",
                        provider,
                        model,
                        input_tokens,
                        output_tokens,
                        cost
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
                    "metadata": msg.metadata.as_ref().and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok()),
                    "reasoning_content": msg.reasoning_content,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecutionOutcome {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecutionFinishedEvent {
    pub execution_id: String,
    pub outcome: AgentExecutionOutcome,
    pub success: bool,
    pub error: Option<String>,
    pub response: Option<String>,
    pub message: Option<String>,
}

pub(crate) fn emit_agent_execution_finished(
    app_handle: &AppHandle,
    execution_id: &str,
    outcome: AgentExecutionOutcome,
    error: Option<String>,
    response: Option<String>,
    message: Option<String>,
) {
    let success = matches!(outcome, AgentExecutionOutcome::Succeeded);
    let finished_payload = AgentExecutionFinishedEvent {
        execution_id: execution_id.to_string(),
        outcome,
        success,
        error: error.clone(),
        response: response.clone(),
        message: message.clone(),
    };

    let persist_payload = finished_payload.clone();
    let persist_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = crate::commands::ai_execution_state_support::persist_agent_execution_state(
            &persist_handle,
            &persist_payload,
        )
        .await
        {
            tracing::warn!(
                "Failed to persist execution state for {}: {}",
                persist_payload.execution_id,
                e
            );
        }
    });

    let _ = app_handle.emit("agent:execution_finished", &finished_payload);

    match outcome {
        AgentExecutionOutcome::Succeeded => {
            let _ = app_handle.emit(
                "agent:complete",
                &serde_json::json!({
                    "execution_id": execution_id,
                    "success": true,
                    "response": response,
                }),
            );
        }
        AgentExecutionOutcome::Failed => {
            let _ = app_handle.emit(
                "agent:error",
                &serde_json::json!({
                    "execution_id": execution_id,
                    "error": error.unwrap_or_else(|| "Agent execution failed".to_string()),
                }),
            );
        }
        AgentExecutionOutcome::Cancelled => {
            let _ = app_handle.emit(
                "agent:cancelled",
                &serde_json::json!({
                    "execution_id": execution_id,
                    "message": message.unwrap_or_else(|| "Execution cancelled by user".to_string()),
                }),
            );
        }
    }
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
    let _ = sentinel_tools::buildin_tools::shell::cancel_shell_execution(&conversation_id).await;
    if let Err(err) =
        sentinel_tools::buildin_tools::shell_background::stop_background_shell_tasks_for_execution(
            &conversation_id,
        )
        .await
    {
        tracing::warn!(
            "Failed to stop background shell tasks for {}: {}",
            conversation_id,
            err
        );
    }
    if let Err(err) =
        crate::agents::suspend_sentinel_active_intent(&app_handle, &conversation_id).await
    {
        tracing::warn!(
            "Failed to suspend sentinel active intent for {}: {}",
            conversation_id,
            err
        );
    }

    emit_agent_execution_finished(
        &app_handle,
        &conversation_id,
        AgentExecutionOutcome::Cancelled,
        None,
        None,
        Some("Execution cancelled by user".to_string()),
    );

    Ok(())
}

/// Cancel only current shell execution for an execution id (without cancelling the whole conversation).
#[tauri::command]
pub async fn cancel_shell_execution(execution_id: String) -> Result<(), String> {
    let cancelled =
        sentinel_tools::buildin_tools::shell::cancel_shell_execution(&execution_id).await;
    if cancelled {
        tracing::info!(
            "Cancelled shell execution for execution_id: {}",
            execution_id
        );
    } else {
        tracing::warn!(
            "No active shell execution found for execution_id: {}",
            execution_id
        );
    }
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
    system_agent_runtime: State<'_, Arc<SystemAgentRuntime>>,
) -> Result<String, String> {
    let agent_context = load_external_profile_context(
        system_agent_runtime.inner(),
        "traffic_plugin_generator_agent",
        &[],
    )
    .await
    .map_err(|e| e.to_string())?;

    let stream_id = request.stream_id.clone();
    let user_message = request.message.clone();
    let virtual_tool_sections = build_virtual_tool_context(&agent_context, None)
        .await
        .map_err(|e| e.to_string())?;
    let external_system_prompt = if virtual_tool_sections.is_empty() {
        request.system_prompt.clone()
    } else {
        let virtual_context = format!(
            "Virtual tool context:\n{}",
            virtual_tool_sections.join("\n\n")
        );
        match request.system_prompt.clone() {
            Some(prompt) if !prompt.trim().is_empty() => {
                Some(format!("{}\n\n{}", prompt, virtual_context))
            }
            _ => Some(virtual_context),
        }
    };
    let system_prompt = merge_external_profile_prompt(external_system_prompt, &agent_context);
    let tracked_run = start_external_profile_run(
        system_agent_runtime.inner(),
        "traffic_plugin_generator_agent",
        serde_json::json!({
            "streamId": stream_id,
            "messagePreview": user_message.chars().take(400).collect::<String>(),
            "hasCustomSystemPrompt": system_prompt.is_some(),
            "historyLength": request.history.as_ref().map(|history| history.len()).unwrap_or(0),
            "declaredTools": agent_context
                .tool_policy
                .as_ref()
                .map(|policy| policy.declared_tools())
                .unwrap_or_default(),
        }),
    )
    .await;

    let (_cancellation_token, cancel_gen) = create_cancellation_token(&stream_id);
    let app_clone = app_handle.clone();
    let sid = stream_id.clone();
    let history = request.history.unwrap_or_default();
    let runtime_for_tracking = system_agent_runtime.inner().clone();
    let ai_manager_arc = ai_manager.inner().clone();

    tokio::spawn(async move {
        let _guard = CancellationGuard(sid.clone(), cancel_gen);
        // Start event
        let _ = app_clone.emit("plugin_gen_start", &serde_json::json!({ "stream_id": sid }));

        let run_id = tracked_run
            .as_ref()
            .map(|run| run.run_id.as_str())
            .unwrap_or("plugin-generator-ad-hoc");
        let result = run_external_chat_task(
            &app_clone,
            &ai_manager_arc,
            "traffic_plugin_generator_agent",
            run_id,
            &agent_context,
            system_prompt.clone(),
            &history,
            user_message.clone(),
        )
        .await;

        match result {
            Ok(content) => {
                complete_external_profile_run_success(
                    &runtime_for_tracking,
                    &tracked_run,
                    serde_json::json!({
                        "content": content,
                        "streamId": sid,
                    }),
                )
                .await;
                let _ = app_clone.emit(
                    "plugin_gen_complete",
                    serde_json::json!({
                        "stream_id": sid,
                        "content": content
                    }),
                );
            }
            Err(e) => {
                complete_external_profile_run_failure(
                    &runtime_for_tracking,
                    &tracked_run,
                    e.to_string(),
                )
                .await;
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

    let llm_config = if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
        apply_generation_settings_from_db(db.as_ref(), service.service.to_llm_config()).await
    } else {
        service.service.to_llm_config()
    };
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

            serde_json::from_str(json_str).map_err(|e| {
                format!(
                    "Failed to parse generated role JSON: {}. Original response: {}",
                    e, response
                )
            })
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
    pub current_code: Option<String>, // 当前编辑的代码
    pub code_context: Option<String>, // 代码上下文（选中的代码片段）
}

// AI 助手对话流式响应（专门用于编辑器 AI 助手）
#[tauri::command]
pub async fn plugin_assistant_chat_stream(
    request: PluginAssistantRequest,
    app_handle: AppHandle,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<String, String> {
    // Get actual default LLM provider from database config
    let mut service_name = request
        .service_name
        .clone()
        .unwrap_or_else(|| "default".to_string());

    // If using default, try to get the actual provider name from config
    if service_name == "default" {
        if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
            if let Ok(Some(default_llm_provider)) =
                db.get_config("ai", "default_llm_provider").await
            {
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

    tracing::info!(
        "Plugin assistant chat using provider: {}, model: {}",
        service.get_config().provider,
        service.get_config().model
    );

    let stream_id = request.stream_id.clone();
    let user_message = request.message.clone();
    let system_prompt = request.system_prompt.clone();

    let (_cancellation_token, cancel_gen) = create_cancellation_token(&stream_id);
    let app_clone = app_handle.clone();
    let sid = stream_id.clone();
    let history = request.history.unwrap_or_default();

    let llm_config = if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
        apply_generation_settings_from_db(db.as_ref(), service.service.to_llm_config()).await
    } else {
        service.service.to_llm_config()
    };

    tokio::spawn(async move {
        let _guard = CancellationGuard(sid.clone(), cancel_gen);
        // Start event
        let _ = app_clone.emit(
            "plugin_assistant_start",
            &serde_json::json!({ "stream_id": sid }),
        );

        // Create LLM client and stream
        let streaming_client = StreamingLlmClient::new(llm_config);
        let app_for_callback = app_clone.clone();
        let sid_for_callback = sid.clone();

        let result = streaming_client
            .stream_chat(
                system_prompt.as_deref(),
                &user_message,
                &history,
                &[],
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
    let aggregated = db
        .get_aggregated_ai_usage()
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
    db.get_ai_usage_stats().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_ai_usage_stats(
    db: tauri::State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db.clear_ai_usage_stats().await.map_err(|e| e.to_string())?;
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
        .map_err(|e: anyhow::Error| {
            format!("Failed to delete messages after {}: {}", message_id, e)
        })?;

    // Resend semantics delete tail messages; clear run_state to avoid stale "Recent Tool Digests".
    if let Err(e) = db_service.delete_agent_run_state(&conversation_id).await {
        tracing::warn!(
            "Failed to clear agent run_state for {}: {}",
            conversation_id,
            e
        );
    }

    // Clear sliding window summaries to prevent stale LONG-TERM MEMORY / RECENT ACTIVITY SUMMARY
    // from being injected into the system prompt after message editing/resending.
    if let Err(e) = db_service
        .delete_sliding_window_summaries(&conversation_id)
        .await
    {
        tracing::warn!(
            "Failed to clear sliding window summaries for {}: {}",
            conversation_id,
            e
        );
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
    crate::commands::ai_runtime_commands::get_subagent_runs(
        parent_execution_id,
        db_service.inner().clone(),
    )
    .await
}

#[tauri::command]
pub async fn get_subagent_messages(
    subagent_run_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<SubagentMessage>, String> {
    crate::commands::ai_runtime_commands::get_subagent_messages(
        subagent_run_id,
        db_service.inner().clone(),
    )
    .await
}

#[tauri::command]
pub async fn delete_subagent_runs_after(
    parent_execution_id: String,
    after_timestamp_ms: i64,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<u64, String> {
    crate::commands::ai_runtime_commands::delete_subagent_runs_after(
        parent_execution_id,
        after_timestamp_ms,
        db_service.inner().clone(),
    )
    .await
}

// 清空会话的所有消息
#[tauri::command]
pub async fn clear_conversation_messages(
    conversation_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    crate::commands::ai_runtime_commands::clear_conversation_messages(
        conversation_id,
        db_service.inner().clone(),
    )
    .await
}

/// 保存全局工具配置（不与会话绑定）
#[tauri::command]
pub async fn save_tool_config(
    tool_config: crate::agents::ToolConfig,
    app_handle: AppHandle,
) -> Result<(), String> {
    crate::commands::ai_runtime_commands::save_tool_config(tool_config, app_handle).await
}

/// 获取全局工具配置
#[tauri::command]
pub async fn get_tool_config(
    app_handle: AppHandle,
) -> Result<Option<crate::agents::ToolConfig>, String> {
    crate::commands::ai_runtime_commands::get_tool_config(app_handle).await
}

/// 通过自然语言描述生成工作流图
#[tauri::command(rename_all = "snake_case")]
pub async fn generate_workflow_from_nl(
    description: String,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    traffic_state: State<'_, TrafficAnalysisState>,
    system_agent_runtime: State<'_, Arc<SystemAgentRuntime>>,
) -> Result<WorkflowGraph, String> {
    crate::commands::ai_runtime_commands::generate_workflow_from_nl(
        description,
        ai_manager.inner().clone(),
        traffic_state.inner(),
        system_agent_runtime.inner().clone(),
    )
    .await
}

/// 生成默认的AI提供商配置（与前端 `AiProviderConfig` 结构兼容）

// 保存模型配置配置
#[tauri::command]
pub async fn save_scheduler_config(
    config: crate::services::ai::SchedulerConfig,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    crate::commands::ai_runtime_commands::save_scheduler_config(config, db.inner().clone()).await
}

// LM Studio相关的命令

/// 刷新LM Studio模型列表
/// DISABLED (ai_adapter removed)
#[tauri::command]
pub async fn refresh_lm_studio_models(
    _api_base: Option<String>,
    _api_key: Option<String>,
) -> Result<Vec<String>, String> {
    crate::commands::ai_runtime_commands::refresh_lm_studio_models(_api_base, _api_key).await
}

/// 获取LM Studio服务器状态 - DISABLED (ai_adapter removed)
#[tauri::command]
pub async fn get_lm_studio_status(
    _api_base: Option<String>,
    _api_key: Option<String>,
) -> Result<serde_json::Value, String> {
    crate::commands::ai_runtime_commands::get_lm_studio_status(_api_base, _api_key).await
}

/// 测试LM Studio提供商连接 - DISABLED (ai_adapter removed)
#[tauri::command]
pub async fn test_lm_studio_provider_connection(
    _api_base: Option<String>,
    _api_key: Option<String>,
) -> Result<TestConnectionResponse, String> {
    crate::commands::ai_runtime_commands::test_lm_studio_provider_connection(_api_base, _api_key)
        .await
}

/// 上传图片文件并转换为 base64
#[tauri::command]
pub async fn upload_image_attachment(file_path: String) -> Result<serde_json::Value, String> {
    crate::commands::ai_runtime_commands::upload_image_attachment(file_path).await
}

/// 批量上传图片文件
#[tauri::command]
pub async fn upload_multiple_images(
    file_paths: Vec<String>,
) -> Result<Vec<serde_json::Value>, String> {
    crate::commands::ai_runtime_commands::upload_multiple_images(file_paths).await
}

/// Agent执行 - 统一的聊天入口，支持流式输出、联网搜索、RAG知识检索
#[tauri::command]
pub async fn agent_execute(
    task: String,
    config: Option<AgentExecuteConfig>,
    app_handle: AppHandle,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<String, String> {
    crate::commands::ai_runtime_commands::agent_execute(
        task,
        config,
        app_handle,
        ai_manager.inner().clone(),
    )
    .await
}

use crate::models::database::{AiConversation, AiMessage};
use crate::services::ai::{AiConfig, AiServiceManager, AiToolCall};
use crate::services::database::{Database, DatabaseService};
use crate::utils::ordered_message::ChunkType;
use crate::utils::global_proxy::create_client_with_proxy;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use sqlx::Row;
use crate::services::prompt_db::PromptRepository;
use crate::utils::prompt_resolver::{PromptResolver, AgentPromptConfig, CanonicalStage};
use crate::models::prompt::ArchitectureType;

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
        remove_cancellation_token(conversation_id);
        tracing::info!("Cancelled conversation stream: {}", conversation_id);
    }
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SendStreamMessageRequest {
    pub conversation_id: String,
    pub message: String,
    pub service_name: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
    pub message_id: Option<String>, // 前端传递的消息ID
    pub attachments: Option<serde_json::Value>, // 前端传递的附件数组(JSON)
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
pub struct AiProviderConfig {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub organization: Option<String>,
    pub enabled: bool,
    pub default_model: String,
    pub models: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestConnectionRequest {
    pub provider: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub organization: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
    pub models: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveAiConfigRequest {
    pub providers: HashMap<String, AiProviderConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetDefaultProviderRequest {
    pub provider: String,
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

// 发送流式聊天消息
#[tauri::command]
pub async fn send_ai_stream_message(
    request: SendStreamMessageRequest,
    app_handle: AppHandle,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<String, String> {
    tracing::info!(
        "Starting stream message for conversation: {}",
        request.conversation_id
    );

    // 普通聊天模式：强制使用默认 Provider/Model，不接受前端覆盖，也不做任何回退
    let (provider, model_name) = match ai_manager.get_default_chat_model().await {
        Ok(Some((p, m))) => {
            tracing::info!("Using default chat model: {}/{}", p, m);
            (p, m)
        }
        Ok(None) => {
            return Err("Default chat model is not configured. Please set it in Settings > AI.".to_string())
        }
        Err(e) => {
            return Err(format!("Failed to read default chat model: {}", e))
        }
    };

    // 读取 provider 配置并构建一次性服务实例（内部用 Rig 直连）
    let provider_config = ai_manager
        .get_provider_config(&provider)
        .await
        .map_err(|e| format!("Failed to load provider config '{}': {}", provider, e))?
        .ok_or_else(|| format!("Provider '{}' configuration not found", provider))?;

    let mut dynamic_config = provider_config;
    dynamic_config.model = model_name.clone();
    if let Some(temp) = request.temperature { dynamic_config.temperature = Some(temp); }
    if let Some(max_tokens) = request.max_tokens { dynamic_config.max_tokens = Some(max_tokens); }

    let db_service = app_handle.state::<Arc<crate::services::database::DatabaseService>>();
    let mcp_service = ai_manager.get_mcp_service();
    let mut service = crate::services::ai::AiService::new(
        dynamic_config,
        db_service.inner().clone(),
        Some(app_handle.clone()),
        mcp_service,
    );
    service.set_app_handle(app_handle.clone());

    // 使用前端传递的消息ID，如果没有则生成新的
    let message_id = request
        .message_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // 创建取消令牌
    let _cancellation_token = create_cancellation_token(&request.conversation_id);

    // 在后台执行流式聊天，直接使用AI服务的流式响应
    let conversation_id = request.conversation_id.clone();
    let message = request.message.clone();
    let attachments = request.attachments.clone();
    let service_clone = service.clone();
    let mut system_prompt = request.system_prompt.clone();
    let message_id_clone = message_id.clone();

    tokio::spawn(async move {
        // First, get current role prompt if available
        let mut role_prompt = String::new();
        if let Some(db) = app_handle.try_state::<Arc<crate::services::database::DatabaseService>>() {
            if let Ok(Some(current_role)) = db.get_current_ai_role().await {
                if !current_role.prompt.trim().is_empty() {
                    role_prompt = current_role.prompt;
                    tracing::info!("Using role prompt from: {}", current_role.title);
                }
            }
        }

        // If no system prompt provided, resolve from unified prompt system (System stage)
        if system_prompt.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) {
            if let Some(db) = app_handle.try_state::<Arc<crate::services::database::DatabaseService>>() {
                if let Ok(pool) = db.get_pool() {
                    let repo = PromptRepository::new(pool.clone());
                    let resolver = PromptResolver::new(repo);
                    let cfg = AgentPromptConfig::default();
                    match resolver
                        .resolve_prompt(&cfg, ArchitectureType::PlanExecute, CanonicalStage::System, None)
                        .await {
                        Ok(content) if !content.trim().is_empty() => {
                            system_prompt = Some(content);
                        }
                        _ => {
                            if let Ok(Some(db_prompt)) = db.get_config("ai", "system_prompt").await {
                                if !db_prompt.trim().is_empty() {
                                    system_prompt = Some(db_prompt);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Combine role prompt with system prompt
        if !role_prompt.is_empty() {
            system_prompt = match system_prompt {
                Some(existing) if !existing.trim().is_empty() => {
                    Some(format!("{}\n\n{}", role_prompt, existing))
                }
                _ => Some(role_prompt)
            };
        }

        // Conditionally augment with RAG evidence blocks when enabled
        if let Some(db) = app_handle.try_state::<Arc<crate::services::database::DatabaseService>>() {
            // Read RAG config to check augmentation toggle
            let mut rag_enabled = false;
            if let Ok(Some(cfg)) = db.get_rag_config().await { rag_enabled = cfg.augmentation_enabled; }

            if rag_enabled {
                // Determine an active collection (fallback to default if none active)
                let active_collection_id: Option<String> = match db.get_rag_collections().await {
                    Ok(cols) => cols.into_iter().find(|c| c.is_active).map(|c| c.id),
                    Err(_) => None,
                };

                // Try to get global RAG service
                if let Ok(rag_service) = crate::commands::rag_commands::get_global_rag_service().await {
                    use tokio::time::{timeout, Duration};

                    // Build brief conversation history for query context (last 3 user/assistant messages)
                    let mut history_snippets: Vec<String> = Vec::new();
                    if let Ok(msgs) = db.get_ai_messages_by_conversation(&conversation_id).await {
                        for msg in msgs.iter().rev().take(6) { // last ~3 rounds
                            let prefix = match msg.role.as_str() { "user" => "U:", "assistant" => "A:", _ => "" };
                            let snippet: String = msg.content.chars().take(200).collect();
                            history_snippets.push(format!("{} {}", prefix, snippet));
                        }
                    }

                    let rag_req = sentinel_rag::models::AssistantRagRequest {
                        query: message.clone(),
                        collection_id: active_collection_id.clone(),
                        conversation_history: if history_snippets.is_empty() { None } else { Some(history_snippets) },
                        top_k: Some(5),
                        use_mmr: Some(true),
                        mmr_lambda: Some(0.7),
                        similarity_threshold: Some(0.65),
                        reranking_enabled: Some(false),
                        model_provider: None,
                        model_name: None,
                        max_tokens: None,
                        temperature: None,
                        system_prompt: None, // 不需要传递，后端会自动获取当前角色
                    };

                    // Short timeout to avoid delaying stream start
                    if let Ok(Ok((context, _citations))) = timeout(Duration::from_millis(1200), rag_service.query_for_assistant(&rag_req)).await {
                        if !context.trim().is_empty() {
                            let base = system_prompt.unwrap_or_else(|| "".to_string());
                            let policy = "你必须严格基于证据回答问题。在回答中引用证据时，使用 [SOURCE n] 格式。如果证据不足，请直接回答并避免编造。";
                            // Evidence blocks are already formatted with === SOURCE n ... ===
                            let augmented = if base.trim().is_empty() {
                                format!(
                                    "[知识溯源规范]\n{}\n\n[证据块]\n{}",
                                    policy, context
                                )
                            } else {
                                format!(
                                    "{}\n\n[知识溯源规范]\n{}\n\n[证据块]\n{}",
                                    base, policy, context
                                )
                            };
                            system_prompt = Some(augmented);

                            // Optionally notify frontend RAG was applied
                            let _ = app_handle.emit(
                                "ai_meta_info",
                                &serde_json::json!({
                                    "conversation_id": conversation_id,
                                    "message_id": message_id_clone,
                                    "rag_applied": true
                                })
                            );
                        }
                    }
                }
            }
        }
        // 发送开始事件（为了与前端配合）
        if let Err(e) = app_handle.emit(
            "ai_stream_start",
            &serde_json::json!({
                "conversation_id": conversation_id,
                "message_id": message_id_clone
            }),
        ) {
            tracing::error!("Failed to emit stream start event: {}", e);
        }

        // 直接调用AI服务的流式方法，它内部已经处理所有的事件发送
        match service_clone
            .send_message_stream(
                Some(&message),
                system_prompt.as_deref(),
                Some(conversation_id.clone()),
                Some(message_id_clone.clone()), // 传递消息ID作为后端的assistant_message_id
                true,
                true,
                Some(ChunkType::Content),
                attachments,
            )
            .await
        {
            Ok(_response_content) => {
                tracing::info!(
                    "Stream chat completed successfully for conversation: {}",
                    conversation_id
                );
            }
            Err(e) => {
                tracing::error!("Stream chat failed: {}", e);
                // AI服务内部已经处理了错误事件发送，这里不需要重复发送
            }
        }

        // 清理取消令牌
        remove_cancellation_token(&conversation_id);
    });

    Ok(message_id)
}


#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SendStreamWithSearchRequest {
    pub conversation_id: String,
    pub message: String,
    pub service_name: String,
    pub max_results: Option<u32>,   // Tavily: 0..=20
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub message_id: Option<String>,
}

/// 发送消息前先根据用户输入执行联网搜索，并将搜索结果注入到提示中再进行流式总结
#[tauri::command]
pub async fn send_ai_stream_with_search(
    request: SendStreamWithSearchRequest,
    app_handle: AppHandle,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<String, String> {
    use std::time::Duration;
    let max_results = request.max_results.unwrap_or(5).min(20);

    // 读取 Tavily API Key（优先环境变量，其次数据库配置 ai/tavily_api_key）
    let tavily_api_key = std::env::var("TAVILY_API_KEY").ok()
        .or_else(|| {
            if let Some(db) = app_handle.try_state::<Arc<crate::services::database::DatabaseService>>() {
                futures::executor::block_on(db.get_config("ai", "tavily_api_key")).ok().flatten()
            } else { None }
        })
        .ok_or_else(|| "TAVILY_API_KEY not configured".to_string())?;

    // 使用全局代理构建客户端
    // 重要：reqwest 不会自动读取环境变量代理，必须手动应用
    let client = {
        let builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(30));
        let builder = crate::utils::global_proxy::apply_proxy_to_client(builder).await;
        builder.build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?
    };
    let tavily_url = "https://api.tavily.com/search";
    let payload = serde_json::json!({
        "query": request.message,
        "max_results": max_results,
        "include_answer": false,
        "include_raw_content": false,
        "search_depth": "basic"
    });
    let tavily_resp = client
        .post(tavily_url)
        .bearer_auth(tavily_api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to call Tavily: {}", e))?;
    if !tavily_resp.status().is_success() {
        let err_txt = tavily_resp.text().await.unwrap_or_default();
        return Err(format!("Tavily error: {}", err_txt));
    }
    let tavily_json: serde_json::Value = tavily_resp.json().await
        .map_err(|e| format!("Failed to parse Tavily response: {}", e))?;

    // 整理 Tavily 结果给 LLM
    let mut lines: Vec<String> = Vec::new();
    if let Some(results) = tavily_json.get("results").and_then(|r| r.as_array()) {
        for (idx, item) in results.iter().enumerate() {
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let url = item.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let content = item.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let line = format!("{}. {}\n{}\n{}", idx + 1, title, url, content);
            lines.push(line);
        }
    }
    let search_block = if lines.is_empty() { String::new() } else { format!(
        "[Web Search]\nsource: Tavily\nresults:\n{}\n\n",
        lines.join("\n\n")
    )};

    // 为LLM增加系统提示，并组装用户内容
    let system_prompt = Some("你是一个AI总结助手，你擅长对信息进行总结，请对用户输入的信息进行总结".to_string());
    let augmented_user_content = if search_block.is_empty() {
        request.message.clone()
    } else {
        format!(
            "{}\n请基于上面的最新搜索结果，对下述用户问题进行客观、结构化总结，最后附上 Sources 列表（列出最相关链接）。用户问题：{}",
            search_block, request.message
        )
    };

    // 复用 send_ai_stream_message 的服务创建逻辑
    let service = if let (Some(provider), Some(model)) = (&request.provider, &request.model) {
        if let Ok(Some(provider_config)) = ai_manager.get_provider_config(provider).await {
            let mut dynamic_config = provider_config;
            dynamic_config.model = model.clone();
            if let Some(temp) = request.temperature { dynamic_config.temperature = Some(temp); }
            if let Some(max_tokens) = request.max_tokens { dynamic_config.max_tokens = Some(max_tokens); }
            let db_service = app_handle.state::<Arc<crate::services::database::DatabaseService>>();
            let mcp_service = ai_manager.get_mcp_service();
            let mut temp_service = crate::services::ai::AiService::new(
                dynamic_config,
                db_service.inner().clone(),
                Some(app_handle.clone()),
                mcp_service,
            );
            temp_service.set_app_handle(app_handle.clone());
            temp_service
        } else {
            let mut default_service = ai_manager
                .get_service(&request.service_name)
                .ok_or_else(|| format!("AI service not found: {}", request.service_name))?;
            default_service.set_app_handle(app_handle.clone());
            default_service
        }
    } else {
        match ai_manager.get_default_chat_model().await {
            Ok(Some((provider, model_name))) => {
                // 直接基于默认配置构建服务
                if let Ok(Some(provider_config)) = ai_manager.get_provider_config(&provider).await {
                    let mut dynamic_config = provider_config;
                    dynamic_config.model = model_name;
                    let db_service = app_handle.state::<Arc<crate::services::database::DatabaseService>>();
                    let mcp_service = ai_manager.get_mcp_service();
                    let mut temp_service = crate::services::ai::AiService::new(
                        dynamic_config,
                        db_service.inner().clone(),
                        Some(app_handle.clone()),
                        mcp_service,
                    );
                    temp_service.set_app_handle(app_handle.clone());
                    temp_service
                } else {
                    let mut default_service = ai_manager
                        .get_service(&request.service_name)
                        .ok_or_else(|| format!("AI service not found: {}", request.service_name))?;
                    default_service.set_app_handle(app_handle.clone());
                    default_service
                }
            }
            _ => {
                let mut service = ai_manager
                    .get_service(&request.service_name)
                    .ok_or_else(|| format!("AI service not found: {}", request.service_name))?;
                service.set_app_handle(app_handle.clone());
                service
            }
        }
    };

    // 消息ID与取消令牌
    let message_id = request
        .message_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let _cancellation_token = create_cancellation_token(&request.conversation_id);

    // 后台流式
    let conversation_id = request.conversation_id.clone();
    let service_clone = service.clone();
    let message_id_clone = message_id.clone();
    let content_to_send = augmented_user_content.clone();
    let mut resolved_system_prompt = system_prompt.clone();

    tokio::spawn(async move {
        // First, get current role prompt if available
        let mut role_prompt = String::new();
        if let Some(db) = app_handle.try_state::<Arc<crate::services::database::DatabaseService>>() {
            if let Ok(Some(current_role)) = db.get_current_ai_role().await {
                if !current_role.prompt.trim().is_empty() {
                    role_prompt = current_role.prompt;
                    tracing::info!("Using role prompt from: {}", current_role.title);
                }
            }
        }

        // Resolve system prompt if empty via unified prompt system
        if resolved_system_prompt.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true) {
            if let Some(db) = app_handle.try_state::<Arc<crate::services::database::DatabaseService>>() {
                if let Ok(pool) = db.get_pool() {
                    let repo = PromptRepository::new(pool.clone());
                    let resolver = PromptResolver::new(repo);
                    let cfg = AgentPromptConfig::default();
                    match resolver
                        .resolve_prompt(&cfg, ArchitectureType::PlanExecute, CanonicalStage::System, None)
                        .await {
                        Ok(content) if !content.trim().is_empty() => {
                            resolved_system_prompt = Some(content);
                        }
                        _ => {
                            if let Ok(Some(db_prompt)) = db.get_config("ai", "system_prompt").await {
                                if !db_prompt.trim().is_empty() {
                                    resolved_system_prompt = Some(db_prompt);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Combine role prompt with system prompt
        if !role_prompt.is_empty() {
            resolved_system_prompt = match resolved_system_prompt {
                Some(existing) if !existing.trim().is_empty() => {
                    Some(format!("{}\n\n{}", role_prompt, existing))
                }
                _ => Some(role_prompt)
            };
        }
        if let Err(e) = app_handle.emit(
            "ai_stream_start",
            &serde_json::json!({
                "conversation_id": conversation_id,
                "message_id": message_id_clone
            }),
        ) {
            tracing::error!("Failed to emit stream start event: {}", e);
        }

        if let Err(e) = service_clone
            .send_message_stream(
                Some(&content_to_send),
                resolved_system_prompt.as_deref(),
                Some(conversation_id.clone()),
                Some(message_id_clone.clone()),
                true,
                false,
                Some(ChunkType::Content),
                None, // attachments not supported in search mode yet
            )
            .await
        {
            tracing::error!("Stream chat with search failed: {}", e);
        }

        remove_cancellation_token(&conversation_id);
    });

    Ok(message_id)
}

// 取消流式聊天
#[tauri::command]
pub async fn cancel_ai_stream(conversation_id: String) -> Result<(), String> {
    tracing::info!("Cancelling stream for conversation: {}", conversation_id);
    cancel_conversation_stream(&conversation_id);
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
    db: State<'_, Arc<DatabaseService>>,
) -> Result<std::collections::HashMap<String, ProviderUsageStats>, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?;
    let rows = sqlx::query(
        r#"
        SELECT 
            c.model_provider as provider,
            SUM(COALESCE(m.token_count, 0)) as total_tokens,
            SUM(CASE WHEN m.role = 'user' THEN COALESCE(m.token_count, 0) ELSE 0 END) as input_tokens,
            SUM(CASE WHEN m.role = 'assistant' THEN COALESCE(m.token_count, 0) ELSE 0 END) as output_tokens,
            SUM(COALESCE(CAST(m.cost AS REAL), 0.0)) as cost
        FROM ai_messages m
        JOIN ai_conversations c ON m.conversation_id = c.id
        GROUP BY c.model_provider
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to query usage stats: {}", e))?;

    let mut map = std::collections::HashMap::new();
    for row in rows {
        let provider: String = row.get::<String, _>("provider");
        let total_tokens: i64 = row.get::<i64, _>("total_tokens");
        let input_tokens: i64 = row.get::<i64, _>("input_tokens");
        let output_tokens: i64 = row.get::<i64, _>("output_tokens");
        let cost: f64 = row.get::<f64, _>("cost");
        map.insert(
            provider,
            ProviderUsageStats {
                input_tokens: input_tokens.max(0) as f64,
                output_tokens: output_tokens.max(0) as f64,
                total_tokens: total_tokens.max(0) as f64,
                cost,
            },
        );
    }

    Ok(map)
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
    let message = AiMessage {
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
        .delete_message(&message_id)
        .await
        .map_err(|e| format!("Failed to delete AI message {}: {}", message_id, e))
}


// 获取AI提供商的实时模型列表
#[tauri::command]
pub async fn get_provider_models(
    provider: String,
    api_key: Option<String>,
    api_base: Option<String>,
    organization: Option<String>,
) -> Result<Vec<String>, String> {
    let request = TestConnectionRequest {
        provider: provider.clone(),
        api_key,
        api_base,
        organization,
        model: None,
    };

    let response = match provider.to_lowercase().as_str() {
        "openai" => test_openai_connection(request).await?,
        "anthropic" => test_anthropic_connection(request).await?,
        "gemini" => test_gemini_connection(request).await?,
        "deepseek" => test_deepseek_connection(request).await?,
        "moonshot" => test_moonshot_connection(request).await?,
        "ollama" => test_ollama_connection(request).await?,
        "openrouter" => test_openrouter_connection(request).await?,
        "modelscope" => test_modelscope_connection(request).await?,
        "lm studio" | "lmstudio" | "lm_studio" => test_lm_studio_connection(request).await?,
        _ => return Err(format!("Unsupported AI provider: {}", provider)),
    };

    if response.success {
        Ok(response.models.unwrap_or_default())
    } else {
        Err(response.message)
    }
}

// 测试AI提供商连接
#[tauri::command]
pub async fn test_ai_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    // 根据提供商类型选择不同的测试方法
    match request.provider.to_lowercase().as_str() {
        "openai" => test_openai_connection(request).await,
        "anthropic" => test_anthropic_connection(request).await,
        "gemini" => test_gemini_connection(request).await,
        "deepseek" => test_deepseek_connection(request).await,
        "moonshot" => test_moonshot_connection(request).await,
        "ollama" => test_ollama_connection(request).await,
        "openrouter" => test_openrouter_connection(request).await,
        "modelscope" => test_modelscope_connection(request).await,
        "lm studio" | "lmstudio" | "lm_studio" => test_lm_studio_connection(request).await,
        _ => Ok(TestConnectionResponse {
            success: false,
            message: format!("Unsupported AI provider: {}", request.provider),
            models: None,
        }),
    }
}

// 测试ModelScope连接
async fn test_modelscope_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    if request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "ModelScope API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy().await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_base = request
        .api_base
        .unwrap_or_else(|| "https://api-inference.modelscope.cn/v1/models".to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {}", request.api_key.unwrap())
            .parse()
            .map_err(|e| format!("无效的API密钥: {}", e))?,
    );

    if let Some(org) = &request.organization {
        if !org.is_empty() {
            headers.insert(
                "x-title",
                org.parse().map_err(|e| format!("无效的组织ID: {}", e))?,
            );
        }
    }

    // 测试连接 - 获取模型列表
    let response = client
        .get(format!("{}/models", api_base))
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to ModelScope: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // 提取模型ID列表
        if let Some(models_array) = models_response.get("data").and_then(|d| d.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect();

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to ModelScope, found {} models",
                    models.len()
                ),
                models: Some(models),
            })
        } else {
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to ModelScope, but failed to get model list"
                    .to_string(),
                models: None,
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to ModelScope: {}", error_text),
            models: None,
        })
    }
}

// 测试OpenRouter连接
pub async fn test_openrouter_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    if request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "OpenRouter API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy().await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_base = request
        .api_base
        .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {}", request.api_key.unwrap())
            .parse()
            .map_err(|e| format!("无效的API密钥: {}", e))?,
    );

    if let Some(org) = &request.organization {
        if !org.is_empty() {
            headers.insert(
                "x-title",
                org.parse().map_err(|e| format!("无效的组织ID: {}", e))?,
            );
        }
    }

    // 测试连接 - 获取模型列表
    let response = client
        .get(format!("{}/models", api_base))
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to OpenRouter: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // 提取模型ID列表
        if let Some(models_array) = models_response.get("data").and_then(|d| d.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect();

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to OpenRouter, found {} models",
                    models.len()
                ),
                models: Some(models),
            })
        } else {
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to ModelScope, but failed to get model list"
                    .to_string(),
                models: None,
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to ModelScope: {}", error_text),
            models: None,
        })
    }
}

// 测试OpenAI连接
async fn test_openai_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    if request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "OpenAI API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy().await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_base = request
        .api_base
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {}", request.api_key.unwrap())
            .parse()
            .map_err(|e| format!("无效的API密钥: {}", e))?,
    );

    if let Some(org) = &request.organization {
        if !org.is_empty() {
            headers.insert(
                "OpenAI-Organization",
                org.parse().map_err(|e| format!("无效的组织ID: {}", e))?,
            );
        }
    }

    // 测试连接 - 获取模型列表
    let response = client
        .get(format!("{}/models", api_base))
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to OpenAI: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // 提取模型ID列表
        if let Some(models_array) = models_response.get("data").and_then(|d| d.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect();

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to OpenAI, found {} models",
                    models.len()
                ),
                models: Some(models),
            })
        } else {
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to OpenAI, but failed to get model list"
                    .to_string(),
                models: None,
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to OpenAI: {}", error_text),
            models: None,
        })
    }
}

// 测试Anthropic连接
async fn test_anthropic_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    if request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "Anthropic API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy().await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_base = request
        .api_base
        .unwrap_or_else(|| "https://api.anthropic.com".to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "x-api-key",
        request
            .api_key
            .unwrap()
            .parse()
            .map_err(|e| format!("无效的API密钥: {}", e))?,
    );
    headers.insert("anthropic-version", "2023-06-01".parse().unwrap());

    // Anthropic没有列出模型的公开API，我们只能测试连接
    // 使用简单的消息请求来测试连接
    let test_payload = serde_json::json!({
        "model": "claude-3-haiku-20240307",
        "max_tokens": 1,
        "messages": [
            {
                "role": "user",
                "content": "Hello"
            }
        ]
    });

    let response = client
        .post(format!("{}/v1/messages", api_base))
        .headers(headers)
        .json(&test_payload)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Anthropic: {}", e))?;

    if response.status().is_success() {
        // 预定义Anthropic模型列表
        let models = vec![
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
            "claude-2.1".to_string(),
            "claude-2.0".to_string(),
            "claude-instant-1.2".to_string(),
        ];

        Ok(TestConnectionResponse {
            success: true,
            message: "Successfully connected to Anthropic Claude API".to_string(),
            models: Some(models),
        })
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "未知错误".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to Anthropic: {}", error_text),
            models: None,
        })
    }
}

// 测试Gemini连接
async fn test_gemini_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    if request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "Gemini API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy().await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_key = request.api_key.unwrap();

    // 使用Gemini API测试连接
    let response = client
        .get(format!(
            "https://generativelanguage.googleapis.com/v1/models?key={}",
            api_key
        ))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Gemini: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // 提取模型列表
        if let Some(models_array) = models_response.get("models").and_then(|m| m.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| {
                    m.get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.replace("models/", ""))
                })
                .collect();

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to Gemini, found {} models",
                    models.len()
                ),
                models: Some(models),
            })
        } else {
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to Gemini, but failed to get model list"
                    .to_string(),
                models: None,
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "未知错误".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to Gemini: {}", error_text),
            models: None,
        })
    }
}

// 测试DeepSeek连接
async fn test_deepseek_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    if request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "DeepSeek API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy().await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_base = request
        .api_base
        .unwrap_or_else(|| "https://api.deepseek.com/v1".to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {}", request.api_key.unwrap())
            .parse()
            .map_err(|e| format!("无效的API密钥: {}", e))?,
    );

    // 测试连接 - 获取模型列表
    let response = client
        .get(format!("{}/models", api_base))
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to DeepSeek: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // 提取模型ID列表
        if let Some(models_array) = models_response.get("data").and_then(|d| d.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect();

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to DeepSeek, found {} models",
                    models.len()
                ),
                models: Some(models),
            })
        } else {
            // 如果无法解析模型列表，使用默认模型
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to DeepSeek, using default model list".to_string(),
                models: Some(vec![
                    "deepseek-reasoner".to_string(),
                    "deepseek-chat".to_string(),
                ]),
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "未知错误".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to DeepSeek: {}", error_text),
            models: None,
        })
    }
}

// 测试LM Studio连接(本地)
async fn test_lm_studio_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    let client = create_client_with_proxy().await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_base = request
        .api_base
        .unwrap_or_else(|| "http://localhost:1234".to_string());

    // 构建请求头
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("Accept", "application/json".parse().unwrap());
    
    // 如果提供了API密钥，添加Authorization头
    if let Some(api_key) = &request.api_key {
        if !api_key.is_empty() && api_key != "lm-studio" {
            headers.insert(
                "Authorization",
                format!("Bearer {}", api_key).parse().unwrap(),
            );
        }
    }

    // 获取可用模型列表
    let response = client
        .get(format!("{}/v1/models", api_base))
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to LM Studio: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // 提取模型列表
        if let Some(models_array) = models_response.get("data").and_then(|m| m.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| m.get("id").and_then(|n| n.as_str()).map(String::from))
                .collect();

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to LM Studio, found {} local models",
                    models.len()
                ),
                models: Some(models),
            })
        } else {
            // 如果无法解析模型列表，返回成功但没有模型
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to LM Studio, but no models found".to_string(),
                models: Some(vec![]),
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to LM Studio: {}", error_text),
            models: None,
        })
    }
}

// 测试Ollama连接(本地) - 使用rig crate增强
async fn test_ollama_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    let api_base = request
        .api_base
        .unwrap_or_else(|| "http://localhost:11434".to_string());

    // 首先使用原始HTTP方式获取模型列表（更可靠）
    let client = create_client_with_proxy().await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let response = client
        .get(format!("{}/api/tags", api_base))
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // 提取模型列表
        if let Some(models_array) = models_response.get("models").and_then(|m| m.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect();

            // 使用rig crate进行额外的连接验证（如果有可用模型）
            let mut rig_test_result = None;
            if !models.is_empty() {
                // 选择第一个可用模型进行rig连接测试
                let test_model = &models[0];
                match test_ollama_with_rig(test_model).await {
                    Ok(rig_msg) => {
                        rig_test_result = Some(format!(" (Rig test: {})", rig_msg));
                    }
                    Err(e) => {
                        log::warn!("Rig connection test failed: {}", e);
                        rig_test_result = Some(format!(" (Rig test failed: {})", e));
                    }
                }
            }

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to Ollama, found {} local models{}",
                    models.len(),
                    rig_test_result.unwrap_or_default()
                ),
                models: Some(models),
            })
        } else {
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to Ollama, but failed to get model list"
                    .to_string(),
                models: None,
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "未知错误".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to Ollama: {}", error_text),
            models: None,
        })
    }
}

// 使用rig crate测试Ollama连接
async fn test_ollama_with_rig(model: &str) -> Result<String, String> {
    use rig::completion::Prompt;
    use rig::client::builder::DynClientBuilder;

    // 通过动态客户端创建Agent（提供稳定的 prompt 接口）
    let agent = DynClientBuilder::new()
        .agent("ollama", model)
        .map_err(|e| format!("Rig agent build failed: {}", e))?
        .build();

    // 发送简单的测试消息
    match agent.prompt("Hello").await {
        Ok(response) => {
            let response_text = response.trim();
            if response_text.is_empty() {
                Ok("Connected but got empty response".to_string())
            } else {
                Ok(format!("Connected and got response ({} chars)", response_text.len()))
            }
        }
        Err(e) => Err(format!("Rig connection failed: {}", e))
    }
}

// 测试Moonshot连接
async fn test_moonshot_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    if request.api_key.is_none() {
        return Ok(TestConnectionResponse {
            success: false,
            message: "Moonshot API key cannot be empty".to_string(),
            models: None,
        });
    }

    let client = create_client_with_proxy().await
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let api_base = request
        .api_base
        .unwrap_or_else(|| "https://api.moonshot.cn/v1".to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {}", request.api_key.unwrap())
            .parse()
            .map_err(|e| format!("无效的API密钥: {}", e))?,
    );

    // 测试连接 - 获取模型列表
    let response = client
        .get(format!("{}/models", api_base))
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Moonshot: {}", e))?;

    if response.status().is_success() {
        let models_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // 提取模型ID列表
        if let Some(models_array) = models_response.get("data").and_then(|d| d.as_array()) {
            let models: Vec<String> = models_array
                .iter()
                .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect();

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to Moonshot, found {} models",
                    models.len()
                ),
                models: Some(models),
            })
        } else {
            // 如果无法解析模型列表，使用默认模型
            Ok(TestConnectionResponse {
                success: true,
                message: "Successfully connected to Moonshot, using default model list".to_string(),
                models: Some(vec![
                    "moonshot-v1-8k".to_string(),
                    "moonshot-v1-32k".to_string(),
                    "moonshot-v1-128k".to_string(),
                ]),
            })
        }
    } else {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "未知错误".to_string());
        Ok(TestConnectionResponse {
            success: false,
            message: format!("Failed to connect to Moonshot: {}", error_text),
            models: None,
        })
    }
}

// 保存AI配置
#[tauri::command]
pub async fn save_ai_config(
    config: SaveAiConfigRequest,
    db: State<'_, Arc<DatabaseService>>,
    ai_manager_state: State<'_, Arc<AiServiceManager>>,
    app: AppHandle,
) -> Result<(), String> {
    tracing::info!("Starting to save AI configuration...");

    // 直接使用注入的数据库服务
    let db_service = db.inner().clone();

    // 保存providers配置为JSON
    let config_str = serde_json::to_string(&config.providers)
        .map_err(|e| format!("Failed to serialize providers config: {}", e))?;


    db_service
        .set_config(
            "ai",
            "providers_config",
            &config_str,
            Some("AI providers configuration"),
        )
        .await
        .map_err(|e| format!("Failed to save providers config to DB: {}", e))?;

    // 分别保存每个提供商的API密钥
    for (_id, provider) in &config.providers {
        if provider.enabled {
            if let Some(api_key) = &provider.api_key {
                if !api_key.is_empty() {
                    let key_name = format!("api_key_{}", provider.provider.to_lowercase());
                    let description = format!("{} API密钥", provider.provider);
                    if let Err(e) = db_service
                        .set_config("ai", &key_name, api_key, Some(&description))
                        .await
                    {
                        tracing::error!("Failed to save API key for {}: {}", provider.provider, e);
                    } else {
                        tracing::info!("Saved API key for {}", provider.provider);
                    }
                }
            }
        }
    }

    tracing::info!("AI configuration saved successfully");

    // 重新加载运行态 AI 服务，确保启用/禁用即时生效
    if let Some(ai_manager) = app.try_state::<Arc<AiServiceManager>>() {
        if let Err(e) = ai_manager.reload_services().await {
            tracing::error!("Failed to reload AI services after saving config: {}", e);
        } else {
            tracing::info!("AI services reloaded after saving config");
        }
        // 尝试应用数据库中的默认 provider 到 default 别名
        if let Ok(Some(default_provider)) = db
            .inner()
            .get_config("ai", "default_provider")
            .await
        {
            if let Err(e) = ai_manager.set_default_alias_to(&default_provider).await {
                tracing::warn!(
                    "Failed to set default alias to '{}': {}",
                    default_provider,
                    e
                );
            } else {
                tracing::info!("Default provider alias updated to '{}'", default_provider);
            }
        }
    }


    // 验证配置是否正确加载
    if let Some(ai_manager) = app.try_state::<Arc<AiServiceManager>>() {
        match ai_manager.get_chat_models().await {
            Ok(models) => {
                if models.is_empty() {
                    tracing::warn!("No chat models found after reloading AI services");
                } else {
                    // tracing::info!("Successfully loaded {} chat models", models.len());
                    // for model in &models {
                    //     tracing::info!("  - {}/{}", model.provider, model.name);
                    // }
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to get chat models after reloading AI services: {}",
                    e
                );
            }
        }
    }

    // 发送AI配置更新事件，通知前端重新加载模型列表
    if let Err(e) = app.emit("ai_config_updated", ()) {
        tracing::warn!("Failed to emit ai_config_updated event: {}", e);
    } else {
        tracing::info!("Emitted ai_config_updated event to frontend");
    }

    Ok(())
}

/// 设置全局默认 AI Provider（保存到DB并更新运行态别名与全局适配器默认）
#[tauri::command]
pub async fn set_default_provider(
    request: SetDefaultProviderRequest,
    db: State<'_, Arc<DatabaseService>>,
    app: AppHandle,
) -> Result<(), String> {
    let provider = request.provider.to_lowercase();

    // 保存到数据库
    db.set_config(
        "ai",
        "default_provider",
        &provider,
        Some("Global default AI provider"),
    )
    .await
    .map_err(|e| e.to_string())?;

    // 应用到运行时：调整 default 别名并设置全局适配器默认 provider
    if let Some(ai_manager) = app.try_state::<Arc<AiServiceManager>>() {
        if let Err(e) = ai_manager.set_default_alias_to(&provider).await {
            tracing::warn!("Failed to update default alias: {}", e);
        }
    }



    // 通知前端
    if let Err(e) = app.emit("ai_default_provider_updated", &provider) {
        tracing::warn!("Failed to emit ai_default_provider_updated event: {}", e);
    }

    Ok(())
}



/// 设置默认Chat模型（UI专用）
#[tauri::command]
pub async fn set_default_chat_model(
    model: String,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    // 保存完整的模型ID到数据库
    db.set_config(
        "ai",
        "default_chat_model",
        &model,
        Some("Default chat model"),
    )
    .await
    .map_err(|e| e.to_string())?;

    // 如果模型格式为 provider/model_name，解析并更新AI管理器
    if let Some((provider, model_name)) = model.split_once('/') {
        if let Err(e) = ai_manager
            .set_default_chat_model(provider, model_name)
            .await
        {
            tracing::warn!("Failed to update AI manager default chat model: {}", e);
        }
    }

    tracing::info!("Set default chat model to: {}", model);
    Ok(())
}





// 获取AI配置
#[tauri::command]
pub async fn get_ai_config(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<serde_json::Value, String> {
    tracing::info!("Getting AI configuration");

    // 构建AI配置对象
    let mut ai_config = serde_json::json!({
        "providers": {}
    });

    // 从数据库获取AI提供商配置
    match db.get_config("ai", "providers_config").await {
        Ok(Some(providers_json)) => {
            if let Ok(providers) = serde_json::from_str::<serde_json::Value>(&providers_json) {
                ai_config["providers"] = providers;
            }
        }
        Ok(None) => {
            tracing::info!("No AI providers configuration found, using Rig defaults");
            // 返回默认提供商配置（未启用，便于前端展示并填写）
            ai_config["providers"] = default_providers_config();
        }
        Err(e) => {
            tracing::warn!("Failed to load AI providers configuration: {}", e);
            // 发生错误时也提供默认配置，避免前端空列表
            ai_config["providers"] = default_providers_config();
        }
    }

    // 获取其他AI配置项
    if let Ok(Some(default_provider)) = db.get_config("ai", "default_provider").await {
        ai_config["default_provider"] = serde_json::Value::String(default_provider);
    }

    if let Ok(Some(default_model)) = db.get_config("ai", "default_model").await {
        ai_config["default_model"] = serde_json::Value::String(default_model);
    }

    if let Ok(Some(default_chat_model)) = db.get_config("ai", "default_chat_model").await {
        ai_config["default_chat_model"] = serde_json::Value::String(default_chat_model);
    }

    if let Ok(Some(system_prompt)) = db.get_config("ai", "system_prompt").await {
        ai_config["system_prompt"] = serde_json::Value::String(system_prompt);
    }

    if let Ok(Some(temperature_str)) = db.get_config("ai", "temperature").await {
        if let Ok(temperature) = temperature_str.parse::<f64>() {
            ai_config["temperature"] = serde_json::Value::Number(
                serde_json::Number::from_f64(temperature)
                    .unwrap_or(serde_json::Number::from_f64(0.7).unwrap()),
            );
        }
    }

    if let Ok(Some(max_tokens_str)) = db.get_config("ai", "max_tokens").await {
        if let Ok(max_tokens) = max_tokens_str.parse::<u32>() {
            ai_config["max_tokens"] =
                serde_json::Value::Number(serde_json::Number::from(max_tokens));
        }
    }

    if let Ok(Some(stream_response_str)) = db.get_config("ai", "stream_response").await {
        if let Ok(stream_response) = stream_response_str.parse::<bool>() {
            ai_config["stream_response"] = serde_json::Value::Bool(stream_response);
        }
    }

    tracing::info!("Successfully retrieved AI configuration");
    Ok(ai_config)
}

/// 生成默认的AI提供商配置（与前端 `AiProviderConfig` 结构兼容）
fn default_providers_config() -> serde_json::Value {
    use serde_json::json;
    // 提供商清单与后端支持的 provider 名称保持一致
    // rig_provider 字段指定后端使用的 rig 库提供商类型
    let providers: Vec<(&'static str, serde_json::Value)> = vec![
        (
            "OpenAI",
            json!({
                "id": "openai",
                "provider": "openai",
                "rig_provider": "openai",
                "name": "OpenAI",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.openai.com/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Anthropic",
            json!({
                "id": "anthropic",
                "provider": "anthropic",
                "rig_provider": "anthropic",
                "name": "Anthropic",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.anthropic.com",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Gemini",
            json!({
                "id": "gemini",
                "provider": "gemini",
                "rig_provider": "gemini",
                "name": "Gemini",
                "enabled": false,
                "api_key": null,
                "api_base": null,
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "DeepSeek",
            json!({
                "id": "deepseek",
                "provider": "deepseek",
                "rig_provider": "deepseek",
                "name": "DeepSeek",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.deepseek.com/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Ollama",
            json!({
                "id": "ollama",
                "provider": "ollama",
                "rig_provider": "ollama",
                "name": "Ollama",
                "enabled": false,
                "api_key": null,
                "api_base": "http://localhost:11434",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Groq",
            json!({
                "id": "groq",
                "provider": "groq",
                "rig_provider": "groq",
                "name": "Groq",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.groq.com/openai/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Cohere",
            json!({
                "id": "cohere",
                "provider": "cohere",
                "rig_provider": "cohere",
                "name": "Cohere",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.cohere.ai",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Moonshot",
            json!({
                "id": "moonshot",
                "provider": "moonshot",
                "rig_provider": "moonshot",
                "name": "Moonshot",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.moonshot.cn/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "xAI",
            json!({
                "id": "xai",
                "provider": "xai",
                "rig_provider": "xai",
                "name": "xAI",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.x.ai/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Perplexity",
            json!({
                "id": "perplexity",
                "provider": "perplexity",
                "rig_provider": "perplexity",
                "name": "Perplexity",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.perplexity.ai",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "TogetherAI",
            json!({
                "id": "togetherai",
                "provider": "togetherai",
                "rig_provider": "togetherai",
                "name": "TogetherAI",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.together.xyz/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "OpenRouter",
            json!({
                "id": "openrouter",
                "provider": "openrouter",
                "rig_provider": "openrouter",
                "name": "OpenRouter",
                "enabled": false,
                "api_key": null,
                "api_base": "https://openrouter.ai/api/v1",
                "organization": null,
                "default_model": "",
                "models": [],
                "http_referer": null,
                "x_title": null
            }),
        ),
        (
            "ModelScope",
            json!({
                "id": "modelscope",
                "provider": "modelscope",
                "rig_provider": "openai",
                "name": "ModelScope",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api-inference.modelscope.cn/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
        (
            "Hyperbolic",
            json!({
                "id": "hyperbolic",
                "provider": "hyperbolic",
                "rig_provider": "hyperbolic",
                "name": "Hyperbolic",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api.hyperbolic.xyz/v1",
                "organization": null,
                "default_model": "",
                "models": []
            }),
        ),
    ];

    let mut map = serde_json::Map::new();
    for (key, value) in providers {
        map.insert(key.to_string(), value);
    }
    serde_json::Value::Object(map)
}

// 模型配置相关命令

// 获取模型配置配置
#[tauri::command]
pub async fn get_scheduler_config(
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<crate::services::ai::SchedulerConfig, String> {
    tracing::info!("Getting scheduler configuration");

    match ai_manager.get_scheduler_config().await {
        Ok(config) => {
            tracing::info!("Successfully retrieved scheduler configuration");
            Ok(config)
        }
        Err(e) => {
            let error_msg = format!("Failed to get scheduler configuration: {}", e);
            tracing::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

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

// 根据阶段获取对应的AI服务
#[tauri::command]
pub async fn get_service_for_stage(
    stage: crate::services::ai::SchedulerStage,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<Option<AiServiceInfo>, String> {
    tracing::info!("Getting AI service for stage: {:?}", stage);

    match ai_manager.get_service_for_stage(stage.clone()).await {
        Ok(Some(service)) => {
            let config = service.get_config();
            let info = AiServiceInfo {
                name: format!("stage_{:?}", stage).to_lowercase(),
                provider: config.provider.clone(),
                model: config.model.clone(),
            };
            tracing::info!(
                "Found service for stage {:?}: {}/{}",
                stage,
                info.provider,
                info.model
            );
            Ok(Some(info))
        }
        Ok(None) => {
            tracing::warn!("No service found for stage {:?}", stage);
            Ok(None)
        }
        Err(e) => {
            let error_msg = format!("Failed to get service for stage {:?}: {}", stage, e);
            tracing::error!("{}", error_msg);
            Err(error_msg)
        }
    }
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
            serde_json::to_value(&attachment)
                .map_err(|e| format!("序列化图片附件失败: {}", e))
        }
        Err(e) => {
            tracing::error!("加载图片失败: {}", e);
            Err(format!("加载图片失败: {}", e))
        }
    }
}

/// 批量上传图片文件
#[tauri::command]
pub async fn upload_multiple_images(file_paths: Vec<String>) -> Result<Vec<serde_json::Value>, String> {
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

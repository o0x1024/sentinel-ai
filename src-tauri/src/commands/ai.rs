use crate::models::database::{AiConversation, AiMessage};
use crate::services::ai::{AiConfig, AiServiceManager, AiToolCall};
use crate::services::database::{Database, DatabaseService};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

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

// 辅助函数：取消对话流
fn cancel_conversation_stream(conversation_id: &str) {
    if let Some(token) = get_cancellation_token(conversation_id) {
        token.cancel();
        remove_cancellation_token(conversation_id);
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
    pub conversation_id: String,
    pub role: String,
    pub content: String,
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

    // 获取AI服务，如果指定了provider和model，则动态创建服务配置
    let service = if let (Some(provider), Some(model)) = (&request.provider, &request.model) {
        tracing::info!("Creating dynamic AI service with provider: {}, model: {}", provider, model);
        
        // 从AI管理器获取提供商配置
        if let Ok(Some(provider_config)) = ai_manager.get_provider_config(provider).await {
            // 创建临时服务配置，使用指定的模型
            let mut dynamic_config = provider_config;
            dynamic_config.model = model.clone();
            
            if let Some(temp) = request.temperature {
                dynamic_config.temperature = Some(temp);
            }
            if let Some(max_tokens) = request.max_tokens {
                dynamic_config.max_tokens = Some(max_tokens);
            }
            
            // 创建临时AI服务
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
            tracing::warn!("Provider {} not found, falling back to default service", provider);
            let mut default_service = ai_manager
                .get_service(&request.service_name)
                .ok_or_else(|| format!("AI service not found: {}", request.service_name))?;
            default_service.set_app_handle(app_handle.clone());
            default_service
        }
    } else {
        // 使用默认服务
        let mut service = ai_manager
            .get_service(&request.service_name)
            .ok_or_else(|| format!("AI service not found: {}", request.service_name))?;
        service.set_app_handle(app_handle.clone());
        service
    };

    // 使用前端传递的消息ID，如果没有则生成新的
    let message_id = request
        .message_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // 创建取消令牌
    let _cancellation_token = create_cancellation_token(&request.conversation_id);

    // 在后台执行流式聊天，直接使用AI服务的流式响应
    let conversation_id = request.conversation_id.clone();
    let message = request.message.clone();
    let service_clone = service.clone();
    let system_prompt = request.system_prompt.clone();
    let message_id_clone = message_id.clone();

    tokio::spawn(async move {
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
                &message,
                system_prompt.as_deref(),
                Some(conversation_id.clone()),
                true,                           // enable_events
                Some(message_id_clone.clone()), // 传递消息ID
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

// 发送AI消息
#[tauri::command]
pub async fn send_ai_message(
    request: SendMessageRequest,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<String, String> {
    tracing::info!("Sending AI message: {:?}", request);
    if let Some(service) = ai_manager.get_service(&request.service_name) {
        service
            .send_message(&request.message, Some(request.conversation_id))
            .await
            .map_err(|e| e.to_string())
    } else {
        Err("AI service not found".to_string())
    }
}

// 仅保存AI消息到对话（不触发模型回复）
#[tauri::command]
pub async fn save_ai_message(
    request: SaveMessageRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    let message = AiMessage {
        id: Uuid::new_v4().to_string(),
        conversation_id: request.conversation_id,
        role: request.role,
        content: request.content,
        metadata: None,
        token_count: None,
        cost: None,
        tool_calls: None,
        attachments: None,
        timestamp: Utc::now(),
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

// 获取AI服务信息
#[tauri::command]
pub async fn get_ai_service_info(
    service_name: String,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<Option<AiServiceInfo>, String> {
    if let Some(service) = ai_manager.get_service(&service_name) {
        let config = service.get_config();
        let info = AiServiceInfo {
            name: service_name,
            provider: config.provider.clone(),
            model: config.model.clone(),
        };
        return Ok(Some(info));
    }
    Ok(None)
}

// 配置AI服务
#[tauri::command]
pub async fn configure_ai_service(
    service_name: String,
    config: CommandAiConfig,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    app: AppHandle,
) -> Result<(), String> {
    // convert early for downstream usage
    let mut config: AiConfig = config.into();
    // 如果配置中没有API密钥，尝试从数据库获取
    if config.api_key.is_none() {
        if let Some(database) = app.try_state::<Arc<crate::services::database::DatabaseService>>() {
            let api_key_name = format!("api_key_{}", config.provider.to_lowercase());
            match database.get_config("ai", &api_key_name).await {
                Ok(Some(api_key)) => {
                    config.api_key = Some(api_key);
                    tracing::info!("Loading API key for {} from database", config.provider);
                }
                Ok(None) => {
                    tracing::warn!("API key for {} not found in database", config.provider);
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to load API key for {} from database: {}",
                        config.provider,
                        e
                    );
                }
            }
        }
    }

    // 如果有API密钥，设置为环境变量
    if let Some(api_key) = &config.api_key {
        let env_var_name = match config.provider.as_str() {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "gemini" => "GEMINI_API_KEY",
            "deepseek" => "DEEPSEEK_API_KEY",
            "moonshot" => "MOONSHOT_API_KEY",
            "groq" => "GROQ_API_KEY",
            "cohere" => "COHERE_API_KEY",
            "xai" => "XAI_API_KEY",
            "openrouter" => "OPENROUTER_API_KEY",
            "modelscope" => "MODEL_SCOPE_API_KEY",
            _ => {
                tracing::warn!("Unknown AI provider: {}", config.provider);
                return Err(format!("Unsupported AI provider: {}", config.provider));
            }
        };
        std::env::set_var(env_var_name, api_key);
        tracing::info!(
            "Set API key environment variable for {}: {}",
            config.provider,
            env_var_name
        );
    } else {
        tracing::warn!(
            "No API key found for {}, requests may fail",
            config.provider
        );
    }

    // 先移除旧配置
    ai_manager.remove_service(&service_name);
    // 添加新配置
    ai_manager
        .add_service(service_name, config)
        .await
        .map_err(|e| e.to_string())
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

// 获取所有可用的AI模型
#[tauri::command]
pub async fn get_available_ai_models() -> Result<Vec<AiModelInfo>, String> {
    let models = Vec::new();

    Ok(models)
}

// 更新AI模型列表（由前端调用，传递实际测试获取的模型）
#[tauri::command]
pub async fn update_ai_models(models: Vec<AiModelInfo>, app: AppHandle) -> Result<(), String> {
    tracing::info!("Updating AI model list: {:?}", models);

    // 将模型列表保存到应用状态中
    if let Some(_ai_manager) = app.try_state::<Arc<AiServiceManager>>() {
        // 这里可以将模型信息保存到AI服务管理器中
        // 目前先记录日志
        for model_info in &models {
            tracing::info!(
                "Provider {}: {} models",
                model_info.provider,
                model_info.models.len()
            );
        }
    }

    Ok(())
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

    let client = reqwest::Client::new();
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

    let client = reqwest::Client::new();
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

    let client = reqwest::Client::new();
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

    let client = reqwest::Client::new();
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

    let client = reqwest::Client::new();
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

    let client = reqwest::Client::new();
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

// 测试Ollama连接(本地)
async fn test_ollama_connection(
    request: TestConnectionRequest,
) -> Result<TestConnectionResponse, String> {
    let client = reqwest::Client::new();
    let api_base = request
        .api_base
        .unwrap_or_else(|| "http://localhost:11434".to_string());

    // 获取可用模型列表
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

            Ok(TestConnectionResponse {
                success: true,
                message: format!(
                    "Successfully connected to Ollama, found {} local models",
                    models.len()
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

    let client = reqwest::Client::new();
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
    _ai_manager: State<'_, Arc<AiServiceManager>>,
    app: AppHandle,
) -> Result<(), String> {
    tracing::info!("Starting to save AI configuration...");

    // 直接使用注入的数据库服务
    let db_service = db.inner().clone();

    // 保存providers配置为JSON
    let config_str = serde_json::to_string(&config.providers)
        .map_err(|e| format!("Failed to serialize providers config: {}", e))?;

    tracing::info!("Saving providers config: {}", config_str);

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

    // 触发AI服务重新加载
    if let Err(e) = reload_ai_services(app.clone()).await {
        tracing::warn!("Failed to reload AI services: {}", e);
        // 不返回错误，因为配置已保存成功
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

    if let Ok(adapter_manager) = crate::ai_adapter::core::AiAdapterManager::global().get_client() {
        if let Ok(mut client) = adapter_manager.write() {
            if let Err(e) = client.set_default_provider(&provider) {
                tracing::warn!("Failed to set global default provider in adapter: {}", e);
            }
        }
    }

    // 通知前端
    if let Err(e) = app.emit("ai_default_provider_updated", &provider) {
        tracing::warn!("Failed to emit ai_default_provider_updated event: {}", e);
    }

    Ok(())
}

// 获取AI使用统计
#[tauri::command]
pub async fn get_ai_usage_stats(
    _ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<serde_json::Value, String> {
    // 这里应该从数据库获取使用统计数据
    // 目前返回模拟数据
    let stats = serde_json::json!({
        "openai": {
            "input_tokens": 1250,
            "output_tokens": 890,
            "total_tokens": 2140,
            "cost": 0.0125
        },
        "anthropic": {
            "input_tokens": 850,
            "output_tokens": 650,
            "total_tokens": 1500,
            "cost": 0.0088
        }
    });

    Ok(stats)
}

// 重新加载AI服务
#[tauri::command]
pub async fn reload_ai_services(app: AppHandle) -> Result<(), String> {
    tracing::info!("Starting to reload AI services...");

    match app.try_state::<Arc<AiServiceManager>>() {
        Some(ai_manager) => {
            // 使用新的reload_services方法
            match ai_manager.reload_services().await {
                Ok(_) => {
                    tracing::info!("AI services reloaded successfully");
                    // 验证模型是否正确加载
                    match ai_manager.get_chat_models().await {
                        Ok(models) => {
                            if models.is_empty() {
                                tracing::warn!("No chat models found after reloading AI services");
                            } else {
                                tracing::info!("Successfully loaded {} chat models", models.len());
                                for model in &models {
                                    tracing::info!("  - {}/{}", model.provider, model.name);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to get chat models after reloading AI services: {}",
                                e
                            );
                        }
                    }
                    Ok(())
                }
                Err(e) => {
                    let error_msg = format!("Failed to reload AI services: {}", e);
                    tracing::error!("{}", error_msg);
                    Err(error_msg)
                }
            }
        }
        None => {
            tracing::warn!("AI service manager not found, trying to recreate...");

            let database = match app.try_state::<Arc<crate::services::database::DatabaseService>>()
            {
                Some(db) => db.inner().clone(),
                None => {
                    let error_msg =
                        "Database service not initialized, cannot reload AI services.".to_string();
                    tracing::error!("{}", error_msg);
                    return Err(error_msg);
                }
            };

            // 创建新的AI服务管理器
            let mut new_ai_manager = AiServiceManager::new(database);

            // 设置MCP服务（如果可用）
            if let Some(mcp_service) = app.try_state::<Arc<crate::services::McpService>>() {
                new_ai_manager.set_mcp_service(mcp_service.inner().clone());
            }

            // 设置应用句柄
            new_ai_manager.set_app_handle(app.clone());

            // 初始化默认服务
            match new_ai_manager.init_default_services().await {
                Ok(_) => {
                    // 更新应用状态
                    app.manage(Arc::new(new_ai_manager.clone()));
                    tracing::info!("AI service manager recreated and initialized successfully");

                    // 验证模型是否正确加载
                    match new_ai_manager.get_chat_models().await {
                        Ok(models) => {
                            if models.is_empty() {
                                tracing::warn!(
                                    "No chat models found after recreating AI service manager"
                                );
                            } else {
                                tracing::info!("Successfully loaded {} chat models", models.len());
                                for model in &models {
                                    tracing::info!("  - {}/{}", model.provider, model.name);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to get chat models after recreating AI service manager: {}",
                                e
                            );
                        }
                    }

                    Ok(())
                }
                Err(e) => {
                    let error_msg = format!("Failed to recreate AI service manager: {}", e);
                    tracing::error!("{}", error_msg);
                    Err(error_msg)
                }
            }
        }
    }
}

// 获取AI服务状态
#[tauri::command]
pub async fn get_ai_service_status() -> Result<Vec<AiServiceStatusResponse>, String> {
    // 示例实现
    Ok(vec![])
}

#[tauri::command]
pub async fn get_ai_chat_models(
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<Vec<ModelInfo>, String> {
    tracing::info!("Getting available AI chat models...");

    match ai_manager.get_chat_models().await {
        Ok(models) => {
            if models.is_empty() {
                tracing::warn!("No chat models found");
            } else {
                // tracing::info!("Found {} chat models", models.len());
                // for model in &models {
                //     tracing::info!("  - {}/{}", model.provider, model.name);
                // }
            }
            Ok(models)
        }
        Err(e) => {
            let error_msg = format!("Failed to get chat models: {}", e);
            tracing::error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

#[tauri::command]
pub async fn get_ai_embedding_models(
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<Vec<ModelInfo>, String> {
    ai_manager
        .get_embedding_models()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_default_ai_model(
    model_type: String,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<Option<ModelInfo>, String> {
    tracing::info!("Getting default AI model for type: {}", model_type);

    // 尝试从AI服务管理器获取默认模型
    match ai_manager.get_default_model(&model_type).await {
        Ok(Some(model)) => {
            tracing::info!("Found default model: {}/{}", model.provider, model.name);
            return Ok(Some(model));
        }
        Ok(None) => {
            tracing::info!("No default model found, trying to find first available model");

            // 如果没有设置默认模型，尝试获取第一个可用的模型
            match ai_manager.get_chat_models().await {
                Ok(models) if !models.is_empty() => {
                    let first_model = &models[0];
                    tracing::info!(
                        "Using first available model as default: {}/{}",
                        first_model.provider,
                        first_model.name
                    );
                    return Ok(Some(first_model.clone()));
                }
                Ok(_) => {
                    tracing::warn!("No models available to use as default");
                    return Ok(None);
                }
                Err(e) => {
                    let error_msg = format!("Failed to get models: {}", e);
                    tracing::error!("{}", error_msg);
                    return Err(error_msg);
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to get default model: {}", e);
            tracing::error!("{}", error_msg);
            return Err(error_msg);
        }
    }
}

#[tauri::command]
pub async fn set_default_ai_model(
    ai_manager: State<'_, Arc<AiServiceManager>>,
    model_type: String,
    provider: String,
    model_name: String,
) -> Result<(), String> {
    ai_manager
        .set_default_model(&model_type, &provider, &model_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_ai_model_config(
    ai_manager: State<'_, Arc<AiServiceManager>>,
    provider: String,
    model_name: String,
) -> Result<Option<ModelConfig>, String> {
    ai_manager
        .get_model_config(&provider, &model_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_ai_model_config(
    ai_manager: State<'_, Arc<AiServiceManager>>,
    config: ModelConfig,
) -> Result<(), String> {
    ai_manager
        .update_model_config(config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_ai_providers_config(config: String, app: AppHandle) -> Result<(), String> {
    let db = app
        .try_state::<Arc<dyn Database + Send + Sync>>()
        .ok_or_else(|| "Database service not initialized".to_string())?;

    // 这里 config 已经是 JSON string，直接保存
    db.set_config(
        "ai",
        "providers_config",
        &config,
        Some("AI providers configuration"),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

// 停止AI流式响应
#[tauri::command]
pub async fn stop_ai_stream(request: StopStreamRequest, app: AppHandle) -> Result<(), String> {
    tracing::info!(
        "Stopping AI stream for conversation: {}",
        request.conversation_id
    );

    // 取消对话流
    cancel_conversation_stream(&request.conversation_id);

    // 尝试通过AI服务管理器停止流
    if let Some(ai_manager) = app.try_state::<Arc<AiServiceManager>>() {
        // 遍历所有服务，尝试停止对应的对话流
        for service_name in ai_manager.list_services() {
            if let Some(_service) = ai_manager.get_service(&service_name) {
                // 这里可以添加服务级别的停止逻辑
                tracing::debug!("Attempting to stop stream in service: {}", service_name);
            }
        }
    }

    // 发送停止事件到前端
    if let Err(e) = app.emit("ai_stream_stopped", &request.conversation_id) {
        tracing::error!("Failed to emit ai_stream_stopped event: {}", e);
        return Err(format!("Failed to emit stop event: {}", e));
    }

    tracing::info!(
        "AI stream stop signal sent for conversation: {}",
        request.conversation_id
    );
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
            tracing::info!("No AI providers configuration found, using defaults from @ai_adapter");
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
    let providers: Vec<(&'static str, serde_json::Value)> = vec![
        (
            "OpenAI",
            json!({
                "id": "openai",
                "provider": "openai",
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
                "name": "Gemini",
                "enabled": false,
                "api_key": null,
                "api_base": null, // Gemini 主要通过 key 查询，无必须 base
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
                "name": "Moonshot AI",
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
            "OpenRouter",
            json!({
                "id": "openrouter",
                "provider": "openrouter",
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
                "name": "ModelScope",
                "enabled": false,
                "api_key": null,
                "api_base": "https://api-inference.modelscope.cn/v1",
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

// 调度策略相关命令

// 获取调度策略配置
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

// 保存调度策略配置
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
        "planner_model",
        &config.planner_model,
        Some("Planner model for scheduler"),
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
        "executor_model",
        &config.executor_model,
        Some("Executor model for scheduler"),
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

    // 保存默认重规划策略
    db.set_config(
        "scheduler",
        "default_strategy",
        &config.default_strategy,
        Some("Default replanning strategy"),
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

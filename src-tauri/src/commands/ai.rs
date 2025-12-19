use crate::commands::passive_scan_commands::PassiveScanState;
use crate::commands::tool_commands;
use crate::models::database::{AiConversation, AiMessage};
use crate::services::ai::{AiConfig, AiServiceManager, AiServiceWrapper, AiToolCall};
use crate::services::database::{Database, DatabaseService};
use crate::services::prompt_db::PromptRepository;
use crate::services::SchedulerStage;
use crate::utils::global_proxy::create_client_with_proxy;
use crate::utils::ordered_message::ChunkType;
use crate::utils::prompt_resolver::{AgentPromptConfig, CanonicalStage, PromptResolver};
use anyhow::Result;
use chrono::Utc;
use sentinel_llm::{
    parse_image_from_json, ChatMessage as LlmChatMessage, StreamContent, StreamingLlmClient,
};
use sentinel_workflow::WorkflowGraph;
use serde::{Deserialize, Serialize};
use sqlx::Row;
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
            rig_provider: None,
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
        remove_cancellation_token(conversation_id);
        tracing::info!("Cancelled conversation stream: {}", conversation_id);
    }
}

/// 检查某个会话是否已被取消（用于停止后续事件发送）
pub fn is_conversation_cancelled(conversation_id: &str) -> bool {
    get_cancellation_token(conversation_id)
        .map(|t| t.is_cancelled())
        .unwrap_or(false)
}

/// 流式调用 LLM 并处理事件发送、消息保存
///
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

    // 转换历史消息
    let history: Vec<LlmChatMessage> = history_messages
        .iter()
        .filter(|msg| !msg.content.trim().is_empty())
        .map(|msg| LlmChatMessage::new(&msg.role, &msg.content))
        .collect();

    // 创建 LLM 客户端
    let llm_config = service.service.to_llm_config();
    let streaming_client = StreamingLlmClient::new(llm_config);

    let provider = service.get_config().provider.to_lowercase();
    let model = service.get_config().model.clone();

    // 流式调用
    let execution_id = message_id.to_string();
    let msg_id = message_id.to_string();
    let conv_id = conversation_id.to_string();
    let app = app_handle.clone();

    let content = streaming_client
        .stream_chat(
            system_prompt,
            user_message,
            &history,
            image.as_ref(),
            move |chunk| {
                if is_conversation_cancelled(&conv_id) {
                    return;
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
            },
        )
        .await
        .map_err(|e| format!("LLM stream error: {}", e))?;

    // 发送完成标记
    if is_final && has_conversation {
        crate::utils::ordered_message::emit_message_chunk_with_arch(
            app_handle,
            message_id,
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
        let msg = core_db::AiMessage {
            id: message_id.to_string(),
            conversation_id: conversation_id.to_string(),
            role: "assistant".to_string(),
            content: content.clone(),
            metadata: None,
            token_count: Some(content.len() as i32),
            cost: None,
            tool_calls: None,
            attachments: None,
            timestamp: chrono::Utc::now(),
            architecture_type: None,
            architecture_meta: None,
            structured_data: None,
        };
        if let Err(e) = db.upsert_ai_message_append(&msg).await {
            tracing::warn!("Failed to save assistant message: {}", e);
        } else {
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
    /// rig 库使用的提供商类型，决定后端 API 调用方式
    pub rig_provider: Option<String>,
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

// 清空会话的所有消息
#[tauri::command]
pub async fn clear_conversation_messages(
    conversation_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    db_service
        .delete_messages_by_conversation(&conversation_id)
        .await
        .map_err(|e| {
            format!(
                "Failed to clear messages for conversation {}: {}",
                conversation_id, e
            )
        })
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

    let client = create_client_with_proxy()
        .await
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

    let client = create_client_with_proxy()
        .await
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

    let client = create_client_with_proxy()
        .await
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

    let client = create_client_with_proxy()
        .await
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

    let client = create_client_with_proxy()
        .await
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

    let client = create_client_with_proxy()
        .await
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
    let client = create_client_with_proxy()
        .await
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
    let client = create_client_with_proxy()
        .await
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
    use rig::client::{CompletionClient, ProviderClient};
    use rig::completion::Prompt;
    use rig::providers::ollama;

    // 直接创建 Ollama 客户端
    let client = ollama::Client::from_env();
    let agent = client.agent(model).build();

    // 发送简单的测试消息
    match agent.prompt("Hello").await {
        Ok(response) => {
            let response_text = response.trim();
            if response_text.is_empty() {
                Ok("Connected but got empty response".to_string())
            } else {
                Ok(format!(
                    "Connected and got response ({} chars)",
                    response_text.len()
                ))
            }
        }
        Err(e) => Err(format!("Rig connection failed: {}", e)),
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

    let client = create_client_with_proxy()
        .await
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
        if let Ok(Some(default_provider)) = db.inner().get_config("ai", "default_provider").await {
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

/// 设置默认Vision模型
#[tauri::command]
pub async fn set_default_vision_model(
    model: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    // 保存完整的模型ID到数据库
    db.set_config(
        "ai",
        "default_vision_model",
        &model,
        Some("Default vision model for Vision Explorer"),
    )
    .await
    .map_err(|e| e.to_string())?;

    tracing::info!("Set default vision model to: {}", model);
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

    if let Ok(Some(default_vlm_provider)) = db.get_config("ai", "default_vlm_provider").await {
        ai_config["default_vlm_provider"] = serde_json::Value::String(default_vlm_provider);
    }

    if let Ok(Some(default_vision_model)) = db.get_config("ai", "default_vision_model").await {
        ai_config["default_vision_model"] = serde_json::Value::String(default_vision_model);
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

    // 读取多模态模式配置
    if let Ok(Some(enable_multimodal_str)) = db.get_config("ai", "enable_multimodal").await {
        if let Ok(enable_multimodal) = enable_multimodal_str.parse::<bool>() {
            ai_config["enable_multimodal"] = serde_json::Value::Bool(enable_multimodal);
        }
    }

    tracing::info!("Successfully retrieved AI configuration");
    Ok(ai_config)
}

/// 通过自然语言描述生成工作流图
#[tauri::command(rename_all = "snake_case")]
pub async fn generate_workflow_from_nl(
    description: String,
    ai_manager: State<'_, Arc<AiServiceManager>>,
    passive_state: State<'_, PassiveScanState>,
) -> Result<WorkflowGraph, String> {
    let desc = description.trim();
    if desc.is_empty() {
        return Err("description is empty".to_string());
    }

    // 选择一个用于规划的 AI 配置
    let mut ai_config = ai_manager
        .get_ai_config_for_stage(SchedulerStage::Planning)
        .await
        .map_err(|e| e.to_string())?;
    if ai_config.is_none() {
        ai_config = ai_manager
            .get_ai_config_for_stage(SchedulerStage::Execution)
            .await
            .map_err(|e| e.to_string())?;
    }
    let ai_config = ai_config.ok_or("No AI model configured for planning/execution")?;

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

    let catalog_summary = match tool_commands::build_node_catalog(passive_state.inner()).await {
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
        r#"你是 Sentinel AI 的工作流设计助手。
根据用户的自然语言描述，输出一个严格符合下面 JSON Schema 的 WorkflowGraph。
只输出 JSON，不要解释，不要包含 Markdown。

可用统一工具列表（tool::<name> 用于工具节点）：
{}

可用节点类型列表（node_type 必须从中选择或基于其命名）：
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
1) 用简洁的 node_type，能对应内置工具则用 "tool::<name>"，需要 AI 推理则用 "llm::completion"。
2) 每个节点给出 node_name 与简短 params.description。
3) 至少包含一个入口节点（node_type 可以为 "start"）。
4) 给出合理的 x/y 布局（从左到右）。
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
    pub enable_web_search: Option<bool>,
    pub attachments: Option<serde_json::Value>,
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
}

/// Agent执行请求
#[derive(Debug, Clone, Deserialize)]
pub struct AgentExecuteRequest {
    pub task: String,
    pub config: Option<AgentExecuteConfig>,
}

/// Agent执行 - 统一的聊天入口，支持流式输出、联网搜索、RAG知识检索
#[tauri::command]
pub async fn agent_execute(
    task: String,
    config: Option<AgentExecuteConfig>,
    app_handle: AppHandle,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<String, String> {
    use std::time::Duration;

    // Multi-point license verification
    #[cfg(not(debug_assertions))]
    if !sentinel_license::is_licensed() {
        return Err("License required for this feature".to_string());
    }

    let config = config.unwrap_or(AgentExecuteConfig {
        conversation_id: None,
        message_id: None,
        enable_rag: Some(false),
        enable_web_search: Some(false),
        attachments: None,
        tool_config: None,
        traffic_context: None,
        display_content: None,
        max_iterations: None,
        timeout_secs: None,
        force_todos: None,
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
    let enable_web_search = config.enable_web_search.unwrap_or(false);
    let attachments = config.attachments.clone();

    // 获取工具配置：优先使用前端传递的配置，否则从数据库加载
    let mut effective_tool_config = if config.tool_config.is_some() {
        tracing::info!("Using tool config from frontend request");
        config.tool_config.clone()
    } else {
        load_tool_config_from_db(&app_handle).await
    };

    tracing::info!(
        "Agent execute: conv={}, msg={}, rag={}, search={}, tools={}",
        conversation_id,
        message_id,
        enable_rag,
        enable_web_search,
        effective_tool_config
            .as_ref()
            .map(|c| c.enabled)
            .unwrap_or(false)
    );

    // 获取默认模型配置
    let (provider, model_name) = match ai_manager.get_default_chat_model().await {
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
    let mut service = crate::services::ai::AiService::new(
        dynamic_config,
        db_service.inner().clone(),
        Some(app_handle.clone()),
    );
    service.set_app_handle(app_handle.clone());

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
            let user_msg = core_db::AiMessage {
                id: user_msg_id.clone(),
                conversation_id: conv_id.clone(),
                role: "user".to_string(),
                content: display_text.clone(),
                metadata: attachments
                    .as_ref()
                    .and_then(|v| serde_json::to_string(v).ok()),
                token_count: Some(display_text.len() as i32),
                cost: None,
                tool_calls: None,
                attachments: attachments
                    .as_ref()
                    .and_then(|v| serde_json::to_string(v).ok()),
                timestamp: chrono::Utc::now(),
                architecture_type: None,
                architecture_meta: None,
                structured_data: None,
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

        // 解析系统提示
        // 只有当 system_prompt 是 None 时才自动加载默认模板
        // 如果是 Some("")（空字符串），说明用户明确不想使用系统提示
        if base_system_prompt.is_none() {
            if let Some(db) =
                app_handle.try_state::<Arc<crate::services::database::DatabaseService>>()
            {
                if let Ok(pool) = db.get_pool() {
                    let repo = PromptRepository::new(pool.clone());
                    let resolver = PromptResolver::new(repo);
                    let cfg = AgentPromptConfig::default();
                    match resolver
                        .resolve_prompt(&cfg, CanonicalStage::System, None)
                        .await
                    {
                        Ok(content) if !content.trim().is_empty() => {
                            base_system_prompt = Some(content);
                        }
                        _ => {}
                    }
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

        let mut augmented_task = task_clone.clone();

        // 联网搜索增强
        if enable_web_search {
            match perform_web_search(&app_handle, &task_clone).await {
                Ok(search_block) if !search_block.is_empty() => {
                    augmented_task = format!(
                        "{}\n\n请基于上面的搜索结果回答用户问题：{}",
                        search_block, task_clone
                    );
                    let _ = app_handle.emit(
                        "ai_meta_info",
                        &serde_json::json!({
                            "conversation_id": conv_id,
                            "message_id": msg_id,
                            "web_search_applied": true
                        }),
                    );
                }
                Err(e) => {
                    tracing::warn!("Web search failed: {}", e);
                }
                _ => {}
            }
        }

        // RAG知识检索增强
        if enable_rag {
            if let Some(db) =
                app_handle.try_state::<Arc<crate::services::database::DatabaseService>>()
            {
                // 获取已激活的集合
                let active_collection_id: Option<String> = match db.get_rag_collections().await {
                    Ok(cols) => cols.into_iter().find(|c| c.is_active).map(|c| c.id),
                    Err(_) => None,
                };

                if let Ok(rag_service) =
                    crate::commands::rag_commands::get_global_rag_service().await
                {
                    let mut history_snippets: Vec<String> = Vec::new();
                    if let Ok(msgs) = db.get_ai_messages_by_conversation(&conv_id).await {
                        for msg in msgs.iter().rev().take(6) {
                            let prefix = match msg.role.as_str() {
                                "user" => "U:",
                                "assistant" => "A:",
                                _ => "",
                            };
                            let snippet: String = msg.content.chars().take(200).collect();
                            history_snippets.push(format!("{} {}", prefix, snippet));
                        }
                    }

                    let rag_req = sentinel_rag::models::AssistantRagRequest {
                        query: task_clone.clone(),
                        collection_id: active_collection_id,
                        conversation_history: if history_snippets.is_empty() {
                            None
                        } else {
                            Some(history_snippets)
                        },
                        top_k: Some(5),
                        use_mmr: Some(true),
                        mmr_lambda: Some(0.7),
                        similarity_threshold: Some(0.65),
                        reranking_enabled: Some(false),
                        model_provider: None,
                        model_name: None,
                        max_tokens: None,
                        temperature: None,
                        system_prompt: None,
                    };

                    if let Ok(Ok((context, citations))) = tokio::time::timeout(
                        Duration::from_millis(1200),
                        rag_service.query_for_assistant(&rag_req),
                    )
                    .await
                    {
                        if !context.trim().is_empty() {
                            let base = base_system_prompt.unwrap_or_default();
                            let policy = "你必须严格基于证据回答问题。在回答中引用证据时，使用 [SOURCE n] 格式。如果证据不足，请直接回答并避免编造。";
                            let augmented = if base.trim().is_empty() {
                                format!("[知识溯源规范]\n{}\n\n[证据块]\n{}", policy, context)
                            } else {
                                format!(
                                    "{}\n\n[知识溯源规范]\n{}\n\n[证据块]\n{}",
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
                                    "source_count": citations.len()
                                }),
                            );
                        }
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
            attachments,
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

/// 执行联网搜索
async fn perform_web_search(app_handle: &AppHandle, query: &str) -> Result<String, String> {
    use std::time::Duration;

    // 读取 Tavily API Key
    let tavily_api_key = std::env::var("TAVILY_API_KEY")
        .ok()
        .or_else(|| {
            if let Some(db) =
                app_handle.try_state::<Arc<crate::services::database::DatabaseService>>()
            {
                futures::executor::block_on(db.get_config("ai", "tavily_api_key"))
                    .ok()
                    .flatten()
            } else {
                None
            }
        })
        .ok_or_else(|| "TAVILY_API_KEY not configured".to_string())?;

    let client = {
        let builder = reqwest::Client::builder().timeout(Duration::from_secs(30));
        let builder = crate::utils::global_proxy::apply_proxy_to_client(builder).await;
        builder
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?
    };

    let payload = serde_json::json!({
        "query": query,
        "max_results": 5,
        "include_answer": false,
        "include_raw_content": false,
        "search_depth": "basic"
    });

    let resp = client
        .post("https://api.tavily.com/search")
        .bearer_auth(tavily_api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to call Tavily: {}", e))?;

    if !resp.status().is_success() {
        let err_txt = resp.text().await.unwrap_or_default();
        return Err(format!("Tavily error: {}", err_txt));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Tavily response: {}", e))?;

    let mut lines: Vec<String> = Vec::new();
    if let Some(results) = json.get("results").and_then(|r| r.as_array()) {
        for (idx, item) in results.iter().enumerate() {
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let url = item.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let content = item.get("content").and_then(|v| v.as_str()).unwrap_or("");
            lines.push(format!("{}. {}\n{}\n{}", idx + 1, title, url, content));
        }
    }

    if lines.is_empty() {
        return Ok(String::new());
    }

    Ok(format!(
        "[Web Search]\nsource: Tavily\nresults:\n{}\n",
        lines.join("\n\n")
    ))
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

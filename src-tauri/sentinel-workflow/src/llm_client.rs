//! Workflow LLM 客户端
//!
//! 直接调用 LLM API，不经过 ai_service

use anyhow::{anyhow, Result};
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::builder::DynClientBuilder;
use rig::completion::{message::Image, Message};
use rig::message::{DocumentSourceKind, ImageDetail, ImageMediaType, UserContent};
use rig::one_or_many::OneOrMany;
use rig::message::AssistantContent;
use rig::streaming::{StreamedAssistantContent, StreamingChat, StreamingPrompt};
use tracing::{debug, error, info};

/// LLM 配置
#[derive(Debug, Clone)]
pub struct WorkflowLlmConfig {
    pub provider: String,
    pub model: String,
    pub timeout_secs: u64,
}

impl Default for WorkflowLlmConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            timeout_secs: 120,
        }
    }
}

// ============================================================================
// 图片附件
// ============================================================================

/// 图片附件信息
#[derive(Debug, Clone)]
pub struct ImageAttachment {
    /// base64 编码的图片数据
    pub base64_data: String,
    /// 媒体类型：png, jpeg, webp, gif
    pub media_type: String,
}

impl ImageAttachment {
    pub fn new(base64_data: String, media_type: String) -> Self {
        Self { base64_data, media_type }
    }

    /// 从 base64 数据和媒体类型创建
    pub fn from_base64(base64_data: impl Into<String>, media_type: impl Into<String>) -> Self {
        Self {
            base64_data: base64_data.into(),
            media_type: media_type.into(),
        }
    }

    /// 解析媒体类型为 rig 的 ImageMediaType
    fn to_image_media_type(&self) -> ImageMediaType {
        match self.media_type.to_lowercase().as_str() {
            "png" => ImageMediaType::PNG,
            "webp" => ImageMediaType::WEBP,
            "gif" => ImageMediaType::GIF,
            "jpeg" | "jpg" => ImageMediaType::JPEG,
            _ => ImageMediaType::JPEG,
        }
    }

    /// 转换为 rig 的 Image 对象
    fn to_rig_image(&self) -> Image {
        Image {
            data: DocumentSourceKind::base64(&self.base64_data),
            media_type: Some(self.to_image_media_type()),
            detail: Some(ImageDetail::Auto),
            additional_params: None,
        }
    }
}

// ============================================================================
// 聊天消息（用于多轮对话）
// ============================================================================

/// 聊天消息（简化版，用于传递历史）
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".to_string(), content: content.into() }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: "assistant".to_string(), content: content.into() }
    }
}

/// 将 ChatMessage 列表转换为 rig Message 列表
fn convert_chat_history(history: &[ChatMessage]) -> Vec<Message> {
    history
        .iter()
        .filter_map(|msg| {
            let content = msg.content.trim();
            if content.is_empty() {
                return None;
            }
            match msg.role.to_lowercase().as_str() {
                "user" => Some(Message::User {
                    content: OneOrMany::one(UserContent::text(content.to_string())),
                }),
                "assistant" => Some(Message::Assistant {
                    id: None,
                    content: OneOrMany::one(AssistantContent::Text(
                        rig::message::Text::from(content.to_string()),
                    )),
                }),
                _ => None,
            }
        })
        .collect()
}

/// Workflow LLM 客户端
pub struct WorkflowLlmClient {
    config: WorkflowLlmConfig,
}

impl WorkflowLlmClient {
    pub fn new(config: WorkflowLlmConfig) -> Self {
        Self { config }
    }

    /// 调用 LLM 获取响应
    pub async fn completion(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
    ) -> Result<String> {
        // rig 使用小写 provider 名称
        let provider = self.config.provider.to_lowercase();
        let model = &self.config.model;

        info!(
            "Workflow LLM request - Provider: {}, Model: {}",
            provider, model
        );

        // 创建 agent
        let agent = {
            let client = DynClientBuilder::new();
            let agent_builder = match client.agent(&provider, model) {
                Ok(builder) => builder,
                Err(e) => {
                    error!(
                        "Failed to create agent for provider '{}' with model '{}': {}",
                        provider, model, e
                    );
                    return Err(anyhow!(
                        "Failed to create AI agent: Provider '{}' may not be supported or model '{}' is invalid. Error: {}",
                        provider, model, e
                    ));
                }
            };

            let preamble = system_prompt.unwrap_or("You are a helpful AI assistant.");
            agent_builder.preamble(preamble).build()
        };

        // 构建用户消息
        let user_message = Message::User {
            content: OneOrMany::one(UserContent::text(user_prompt.to_string())),
        };

        // 流式请求（带超时）
        let stream_result = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
            agent.stream_prompt(user_message).multi_turn(100),
        )
        .await;

        let mut stream_iter = match stream_result {
            Ok(iter) => iter,
            Err(_) => {
                error!(
                    "LLM request timeout after {} seconds for provider '{}' model '{}'",
                    self.config.timeout_secs, provider, model
                );
                return Err(anyhow!(
                    "LLM request timeout: The AI service did not respond within {} seconds.",
                    self.config.timeout_secs
                ));
            }
        };

        // 处理流式响应
        let mut content = String::new();

        while let Some(item) = stream_iter.next().await {
            match item {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t))) => {
                    let piece = t.text;
                    if !piece.is_empty() {
                        content.push_str(&piece);
                    }
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    debug!("LLM stream completed");
                    break;
                }
                Ok(_) => { /* ignore other stream items */ }
                Err(e) => {
                    error!("LLM stream error: {}", e);
                    return Err(anyhow!("LLM stream error: {}", e));
                }
            }
        }

        info!(
            "Workflow LLM response - Length: {} chars",
            content.len()
        );

        Ok(content)
    }

    /// 调用 LLM（支持图片），返回完整响应
    pub async fn completion_with_image(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        image: Option<&ImageAttachment>,
    ) -> Result<String> {
        let provider = self.config.provider.to_lowercase();
        let model = &self.config.model;

        info!(
            "Workflow LLM request (with image: {}) - Provider: {}, Model: {}",
            image.is_some(), provider, model
        );

        let agent = {
            let client = DynClientBuilder::new();
            let agent_builder = match client.agent(&provider, model) {
                Ok(builder) => builder,
                Err(e) => {
                    error!(
                        "Failed to create agent for provider '{}' with model '{}': {}",
                        provider, model, e
                    );
                    return Err(anyhow!(
                        "Failed to create AI agent: Provider '{}' may not be supported or model '{}' is invalid. Error: {}",
                        provider, model, e
                    ));
                }
            };
            let preamble = system_prompt.unwrap_or("You are a helpful AI assistant.");
            agent_builder.preamble(preamble).build()
        };

        // 构建用户消息（可能包含图片）
        let user_message = if let Some(img) = image {
            Message::User {
                content: OneOrMany::many(vec![
                    UserContent::Image(img.to_rig_image()),
                    UserContent::text(user_prompt.to_string()),
                ]).expect("Failed to create multi-content message"),
            }
        } else {
            Message::User {
                content: OneOrMany::one(UserContent::text(user_prompt.to_string())),
            }
        };

        let stream_result = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
            agent.stream_prompt(user_message).multi_turn(100),
        )
        .await;

        let mut stream_iter = match stream_result {
            Ok(iter) => iter,
            Err(_) => {
                error!(
                    "LLM request timeout after {} seconds for provider '{}' model '{}'",
                    self.config.timeout_secs, provider, model
                );
                return Err(anyhow!(
                    "LLM request timeout: The AI service did not respond within {} seconds.",
                    self.config.timeout_secs
                ));
            }
        };

        let mut content = String::new();
        while let Some(item) = stream_iter.next().await {
            match item {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t))) => {
                    let piece = t.text;
                    if !piece.is_empty() {
                        content.push_str(&piece);
                    }
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    debug!("LLM stream completed");
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    error!("LLM stream error: {}", e);
                    return Err(anyhow!("LLM stream error: {}", e));
                }
            }
        }

        info!(
            "Workflow LLM response - Length: {} chars",
            content.len()
        );

        Ok(content)
    }

    /// 多轮对话调用 LLM（支持历史消息和图片）
    pub async fn chat(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        history: &[ChatMessage],
        image: Option<&ImageAttachment>,
    ) -> Result<String> {
        let provider = self.config.provider.to_lowercase();
        let model = &self.config.model;

        info!(
            "Workflow LLM chat - Provider: {}, Model: {}, History: {} messages, Image: {}",
            provider, model, history.len(), image.is_some()
        );

        let agent = {
            let client = DynClientBuilder::new();
            let agent_builder = match client.agent(&provider, model) {
                Ok(builder) => builder,
                Err(e) => {
                    error!(
                        "Failed to create agent for provider '{}' with model '{}': {}",
                        provider, model, e
                    );
                    return Err(anyhow!(
                        "Failed to create AI agent: Provider '{}' may not be supported or model '{}' is invalid. Error: {}",
                        provider, model, e
                    ));
                }
            };
            let preamble = system_prompt.unwrap_or("You are a helpful AI assistant.");
            agent_builder.preamble(preamble).build()
        };

        // 构建用户消息
        let user_message = if let Some(img) = image {
            Message::User {
                content: OneOrMany::many(vec![
                    UserContent::Image(img.to_rig_image()),
                    UserContent::text(user_prompt.to_string()),
                ]).expect("Failed to create multi-content message"),
            }
        } else {
            Message::User {
                content: OneOrMany::one(UserContent::text(user_prompt.to_string())),
            }
        };

        // 转换历史消息
        let chat_history = convert_chat_history(history);

        // 使用 stream_chat 支持多轮对话
        let stream_result = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
            agent.stream_chat(user_message, chat_history).multi_turn(100),
        )
        .await;

        let mut stream_iter = match stream_result {
            Ok(iter) => iter,
            Err(_) => {
                error!(
                    "LLM request timeout after {} seconds for provider '{}' model '{}'",
                    self.config.timeout_secs, provider, model
                );
                return Err(anyhow!(
                    "LLM request timeout: The AI service did not respond within {} seconds.",
                    self.config.timeout_secs
                ));
            }
        };

        let mut content = String::new();
        while let Some(item) = stream_iter.next().await {
            match item {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t))) => {
                    let piece = t.text;
                    if !piece.is_empty() {
                        content.push_str(&piece);
                    }
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    debug!("LLM stream completed");
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    error!("LLM stream error: {}", e);
                    return Err(anyhow!("LLM stream error: {}", e));
                }
            }
        }

        info!(
            "Workflow LLM chat response - Length: {} chars",
            content.len()
        );

        Ok(content)
    }
}


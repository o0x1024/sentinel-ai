//! 统一 LLM 客户端模块
//!
//! 提供各架构共用的 LLM 调用能力：
//! - LlmClient: 基础调用，返回完整响应（用于规划、分析等）
//! - StreamingLlmClient: 流式调用，支持回调处理每个 token
//!
//! 各架构可以使用这些基础客户端，配合自己的消息发送器实现特定的流式展示

use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::builder::DynClientBuilder;
use rig::completion::{message::Image, Message};
use rig::message::{DocumentSourceKind, ImageDetail, ImageMediaType, UserContent};
use rig::one_or_many::OneOrMany;
use rig::message::AssistantContent;
use rig::streaming::{StreamedAssistantContent, StreamingChat, StreamingPrompt};
use std::fs::OpenOptions;
use std::io::Write;
use tracing::{debug, error, info};

// ============================================================================
// LLM 配置
// ============================================================================

/// LLM 配置
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub timeout_secs: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            api_key: None,
            base_url: None,
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
            _ => ImageMediaType::JPEG, // 默认 JPEG
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

/// 从 AiService 配置创建 LlmConfig
pub fn create_llm_config(ai_service: &crate::services::ai::AiService) -> LlmConfig {
    let config = ai_service.get_config();
    LlmConfig {
        provider: config.provider.clone(),
        model: config.model.clone(),
        api_key: config.api_key.clone(),
        base_url: config.api_base.clone(),
        timeout_secs: 120,
    }
}

// ============================================================================
// LlmClient - 基础 LLM 调用（无流式前端显示）
// ============================================================================

/// 基础 LLM 客户端（用于规划、分析等不需要流式显示的场景）
#[derive(Clone)]
pub struct LlmClient {
    config: LlmConfig,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    /// 从 AiService 创建
    pub fn from_ai_service(ai_service: &crate::services::ai::AiService) -> Self {
        Self::new(create_llm_config(ai_service))
    }

    /// 获取配置
    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// 设置 rig 库所需的环境变量
    fn setup_env_vars(&self) {
        let provider = self.config.provider.to_lowercase();
        
        // 设置 API Key
        if let Some(api_key) = &self.config.api_key {
            match provider.as_str() {
                "gemini" | "google" => {
                    std::env::set_var("GEMINI_API_KEY", api_key);
                }
                "openai" => {
                    std::env::set_var("OPENAI_API_KEY", api_key);
                }
                "anthropic" => {
                    std::env::set_var("ANTHROPIC_API_KEY", api_key);
                }
                _ => {
                    // For other providers, try OpenAI-compatible env vars
                    std::env::set_var("OPENAI_API_KEY", api_key);
                }
            }
        }
        
        // 设置 Base URL
        if let Some(base_url) = &self.config.base_url {
            match provider.as_str() {
                "gemini" | "google" => {
                    std::env::set_var("GEMINI_API_BASE", base_url);
                }
                "anthropic" => {
                    std::env::set_var("ANTHROPIC_API_BASE", base_url);
                }
                _ => {
                    // OpenAI and compatible providers
                    std::env::set_var("OPENAI_API_BASE", base_url);
                    std::env::set_var("OPENAI_BASE_URL", base_url);
                    std::env::set_var("OPENAI_BASE", base_url);
                }
            }
            debug!("LlmClient: Set base URL env vars for provider '{}': {}", provider, base_url);
        }
    }

    /// 调用 LLM，返回完整响应
    pub async fn completion(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
    ) -> Result<String> {
        let provider = &self.config.provider;
        let model = &self.config.model;

        info!(
            "LlmClient request - Provider: {}, Model: {}",
            provider, model
        );
        
        // 记录 prompt 到日志
        log_prompts("LlmClient", system_prompt, user_prompt);

        // 设置 rig 库所需的环境变量
        self.setup_env_vars();

        // 创建 agent
        let agent = {
            let client = DynClientBuilder::new();
            let agent_builder = match client.agent(provider, model) {
                Ok(builder) => builder,
                Err(e) => {
                    error!(
                        "LlmClient: Failed to create agent for provider '{}' model '{}': {}",
                        provider, model, e
                    );
                    return Err(anyhow!(
                        "LLM client unavailable: Provider '{}' model '{}' error: {}",
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
                    "LlmClient: Request timeout after {} seconds",
                    self.config.timeout_secs
                );
                return Err(anyhow!(
                    "LLM request timeout after {} seconds",
                    self.config.timeout_secs
                ));
            }
        };

        // 收集响应
        let mut content = String::new();
        while let Some(item) = stream_iter.next().await {
            match item {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t))) => {
                    content.push_str(&t.text);
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    debug!("LlmClient: Stream completed");
                    break;
                }
                Ok(_) => { /* ignore other stream items */ }
                Err(e) => {
                    error!("LlmClient: Stream error: {}", e);
                    return Err(anyhow!("LLM stream error: {}", e));
                }
            }
        }

        info!("LlmClient: Response length: {} chars", content.len());
        
        // 记录响应到日志文件
        log_response("LlmClient", &content);
        
        Ok(content)
    }

    /// 调用 LLM（支持图片），返回完整响应
    pub async fn completion_with_image(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        image: Option<&ImageAttachment>,
    ) -> Result<String> {
        let provider = &self.config.provider;
        let model = &self.config.model;

        info!(
            "LlmClient request (with image: {}) - Provider: {}, Model: {}",
            image.is_some(), provider, model
        );
        
        log_prompts("LlmClient", system_prompt, user_prompt);
        self.setup_env_vars();

        let agent = {
            let client = DynClientBuilder::new();
            let agent_builder = match client.agent(provider, model) {
                Ok(builder) => builder,
                Err(e) => {
                    error!(
                        "LlmClient: Failed to create agent for provider '{}' model '{}': {}",
                        provider, model, e
                    );
                    return Err(anyhow!(
                        "LLM client unavailable: Provider '{}' model '{}' error: {}",
                        provider, model, e
                    ));
                }
            };
            let preamble = system_prompt.unwrap_or("You are a helpful AI assistant.");
            agent_builder.preamble(preamble).build()
        };

        // 构建用户消息（可能包含图片）
        let user_message = if let Some(img) = image {
            // 多模态消息：图片 + 文本
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
                    "LlmClient: Request timeout after {} seconds",
                    self.config.timeout_secs
                );
                return Err(anyhow!(
                    "LLM request timeout after {} seconds",
                    self.config.timeout_secs
                ));
            }
        };

        let mut content = String::new();
        while let Some(item) = stream_iter.next().await {
            match item {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t))) => {
                    content.push_str(&t.text);
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    debug!("LlmClient: Stream completed");
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    error!("LlmClient: Stream error: {}", e);
                    return Err(anyhow!("LLM stream error: {}", e));
                }
            }
        }

        info!("LlmClient: Response length: {} chars", content.len());
        log_response("LlmClient", &content);
        
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
        let provider = &self.config.provider;
        let model = &self.config.model;

        info!(
            "LlmClient chat - Provider: {}, Model: {}, History: {} messages, Image: {}",
            provider, model, history.len(), image.is_some()
        );
        
        log_prompts("LlmClient", system_prompt, user_prompt);
        self.setup_env_vars();

        let agent = {
            let client = DynClientBuilder::new();
            let agent_builder = match client.agent(provider, model) {
                Ok(builder) => builder,
                Err(e) => {
                    error!(
                        "LlmClient: Failed to create agent for provider '{}' model '{}': {}",
                        provider, model, e
                    );
                    return Err(anyhow!(
                        "LLM client unavailable: Provider '{}' model '{}' error: {}",
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
                    "LlmClient: Request timeout after {} seconds",
                    self.config.timeout_secs
                );
                return Err(anyhow!(
                    "LLM request timeout after {} seconds",
                    self.config.timeout_secs
                ));
            }
        };

        let mut content = String::new();
        while let Some(item) = stream_iter.next().await {
            match item {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t))) => {
                    content.push_str(&t.text);
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    debug!("LlmClient: Stream completed");
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    error!("LlmClient: Stream error: {}", e);
                    return Err(anyhow!("LLM stream error: {}", e));
                }
            }
        }

        info!("LlmClient: Response length: {} chars", content.len());
        log_response("LlmClient", &content);
        
        Ok(content)
    }
}

/// 类型别名以保持向后兼容
pub type SimpleLlmClient = LlmClient;

// ============================================================================
// StreamingLlmClient - 流式调用（支持回调）
// ============================================================================

/// 流式内容类型
#[derive(Debug, Clone)]
pub enum StreamContent {
    /// 文本内容
    Text(String),
    /// 推理内容（思考过程）
    Reasoning(String),
    /// 流完成
    Done,
}

/// 流式 LLM 客户端（支持回调处理每个 token）
pub struct StreamingLlmClient {
    config: LlmConfig,
}

impl StreamingLlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    /// 从 AiService 创建
    pub fn from_ai_service(ai_service: &crate::services::ai::AiService) -> Self {
        Self::new(create_llm_config(ai_service))
    }

    /// 获取配置
    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// 设置 rig 库所需的环境变量
    fn setup_env_vars(&self) {
        let provider = self.config.provider.to_lowercase();
        
        // 设置 API Key
        if let Some(api_key) = &self.config.api_key {
            match provider.as_str() {
                "gemini" | "google" => {
                    std::env::set_var("GEMINI_API_KEY", api_key);
                }
                "openai" => {
                    std::env::set_var("OPENAI_API_KEY", api_key);
                }
                "anthropic" => {
                    std::env::set_var("ANTHROPIC_API_KEY", api_key);
                }
                _ => {
                    std::env::set_var("OPENAI_API_KEY", api_key);
                }
            }
        }
        
        // 设置 Base URL
        if let Some(base_url) = &self.config.base_url {
            match provider.as_str() {
                "gemini" | "google" => {
                    std::env::set_var("GEMINI_API_BASE", base_url);
                }
                "anthropic" => {
                    std::env::set_var("ANTHROPIC_API_BASE", base_url);
                }
                _ => {
                    std::env::set_var("OPENAI_API_BASE", base_url);
                    std::env::set_var("OPENAI_BASE_URL", base_url);
                    std::env::set_var("OPENAI_BASE", base_url);
                }
            }
            debug!("StreamingLlmClient: Set base URL env vars for provider '{}': {}", provider, base_url);
        }
    }

    /// 流式调用 LLM，通过回调处理每个内容块
    /// 返回完整的响应内容
    pub async fn stream_completion<F>(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        mut on_content: F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        let provider = &self.config.provider;
        let model = &self.config.model;

        info!(
            "StreamingLlmClient request - Provider: {}, Model: {}",
            provider, model
        );
        
        // 记录 prompt 到日志
        log_prompts("StreamingLlmClient", system_prompt, user_prompt);

        // 设置 rig 库所需的环境变量
        self.setup_env_vars();

        // 创建 agent
        let agent = {
            let client = DynClientBuilder::new();
            let agent_builder = match client.agent(provider, model) {
                Ok(builder) => builder,
                Err(e) => {
                    error!(
                        "StreamingLlmClient: Failed to create agent for provider '{}' model '{}': {}",
                        provider, model, e
                    );
                    return Err(anyhow!(
                        "LLM client unavailable: Provider '{}' model '{}' error: {}",
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
                    "StreamingLlmClient: Request timeout after {} seconds",
                    self.config.timeout_secs
                );
                return Err(anyhow!(
                    "LLM request timeout after {} seconds",
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
                        on_content(StreamContent::Text(piece));
                    }
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Reasoning(r),
                )) => {
                    let piece = r.reasoning.join("");
                    if !piece.is_empty() {
                        on_content(StreamContent::Reasoning(piece));
                    }
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    debug!("StreamingLlmClient: Stream completed");
                    on_content(StreamContent::Done);
                    break;
                }
                Ok(_) => { /* ignore other stream items */ }
                Err(e) => {
                    error!("StreamingLlmClient: Stream error: {}", e);
                    return Err(anyhow!("LLM stream error: {}", e));
                }
            }
        }

        info!(
            "StreamingLlmClient: Response length: {} chars",
            content.len()
        );
        
        // 记录响应到日志文件
        log_response("StreamingLlmClient", &content);
        
        Ok(content)
    }

    /// 流式调用 LLM（支持图片），通过回调处理每个内容块
    pub async fn stream_completion_with_image<F>(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        image: Option<&ImageAttachment>,
        mut on_content: F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        let provider = &self.config.provider;
        let model = &self.config.model;

        info!(
            "StreamingLlmClient request (with image: {}) - Provider: {}, Model: {}",
            image.is_some(), provider, model
        );
        
        log_prompts("StreamingLlmClient", system_prompt, user_prompt);
        self.setup_env_vars();

        let agent = {
            let client = DynClientBuilder::new();
            let agent_builder = match client.agent(provider, model) {
                Ok(builder) => builder,
                Err(e) => {
                    error!(
                        "StreamingLlmClient: Failed to create agent for provider '{}' model '{}': {}",
                        provider, model, e
                    );
                    return Err(anyhow!(
                        "LLM client unavailable: Provider '{}' model '{}' error: {}",
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
                    "StreamingLlmClient: Request timeout after {} seconds",
                    self.config.timeout_secs
                );
                return Err(anyhow!(
                    "LLM request timeout after {} seconds",
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
                        on_content(StreamContent::Text(piece));
                    }
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Reasoning(r),
                )) => {
                    let piece = r.reasoning.join("");
                    if !piece.is_empty() {
                        on_content(StreamContent::Reasoning(piece));
                    }
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    debug!("StreamingLlmClient: Stream completed");
                    on_content(StreamContent::Done);
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    error!("StreamingLlmClient: Stream error: {}", e);
                    return Err(anyhow!("LLM stream error: {}", e));
                }
            }
        }

        info!(
            "StreamingLlmClient: Response length: {} chars",
            content.len()
        );
        log_response("StreamingLlmClient", &content);
        
        Ok(content)
    }

    /// 流式多轮对话（支持历史消息和图片）
    pub async fn stream_chat<F>(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        history: &[ChatMessage],
        image: Option<&ImageAttachment>,
        mut on_content: F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        let provider = &self.config.provider;
        let model = &self.config.model;

        info!(
            "StreamingLlmClient chat - Provider: {}, Model: {}, History: {} messages, Image: {}",
            provider, model, history.len(), image.is_some()
        );
        
        log_prompts("StreamingLlmClient", system_prompt, user_prompt);
        self.setup_env_vars();

        let agent = {
            let client = DynClientBuilder::new();
            let agent_builder = match client.agent(provider, model) {
                Ok(builder) => builder,
                Err(e) => {
                    error!(
                        "StreamingLlmClient: Failed to create agent for provider '{}' model '{}': {}",
                        provider, model, e
                    );
                    return Err(anyhow!(
                        "LLM client unavailable: Provider '{}' model '{}' error: {}",
                        provider, model, e
                    ));
                }
            };
            let preamble = system_prompt.unwrap_or("You are a helpful AI assistant.");
            agent_builder.preamble(preamble).build()
        };

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

        let chat_history = convert_chat_history(history);

        let stream_result = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
            agent.stream_chat(user_message, chat_history).multi_turn(100),
        )
        .await;

        let mut stream_iter = match stream_result {
            Ok(iter) => iter,
            Err(_) => {
                error!(
                    "StreamingLlmClient: Request timeout after {} seconds",
                    self.config.timeout_secs
                );
                return Err(anyhow!(
                    "LLM request timeout after {} seconds",
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
                        on_content(StreamContent::Text(piece));
                    }
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Reasoning(r),
                )) => {
                    let piece = r.reasoning.join("");
                    if !piece.is_empty() {
                        on_content(StreamContent::Reasoning(piece));
                    }
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    debug!("StreamingLlmClient: Stream completed");
                    on_content(StreamContent::Done);
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    error!("StreamingLlmClient: Stream error: {}", e);
                    return Err(anyhow!("LLM stream error: {}", e));
                }
            }
        }

        info!(
            "StreamingLlmClient: Response length: {} chars",
            content.len()
        );
        log_response("StreamingLlmClient", &content);
        
        Ok(content)
    }
}

// ============================================================================
// Prompt 日志记录
// ============================================================================

/// 记录 prompts 到 LLM 日志文件
fn log_prompts(client_name: &str, system_prompt: Option<&str>, user_prompt: &str) {
    write_llm_log(client_name, "REQUEST", system_prompt, user_prompt, None);
}

/// 记录 LLM 响应到日志文件
fn log_response(client_name: &str, response: &str) {
    write_llm_log(client_name, "RESPONSE", None, "", Some(response));
}

/// 写入 LLM 日志到文件
fn write_llm_log(
    client_name: &str,
    log_type: &str,
    system_prompt: Option<&str>,
    user_prompt: &str,
    response: Option<&str>,
) {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
    
    let content = if let Some(resp) = response {
        // 安全截断，确保不在 UTF-8 字符中间切断
        let truncated = if resp.len() > 2000 {
            let mut end = 2000;
            while end > 0 && !resp.is_char_boundary(end) {
                end -= 1;
            }
            &resp[..end]
        } else {
            resp
        };
        format!(
            "Response ({} chars):\n{}\n",
            resp.len(),
            truncated
        )
    } else {
        // format!(
        //     "System Prompt:\n{}\n\nUser Prompt:\n{}\n",
        //     system_prompt.unwrap_or("(none)"),
        //     user_prompt
        // )
        format!(
            "User Prompt:\n{}\n",
            user_prompt
        )
    };
    
    let separator = "=".repeat(80);
    let log_entry = format!(
        "\n{}\n[{}] [{}] [Client: {}]\n{}\n{}\n",
        separator, timestamp, log_type, client_name, separator, content
    );

    // 确保日志目录存在
    if let Err(e) = std::fs::create_dir_all("logs") {
        error!("Failed to create logs directory: {}", e);
        return;
    }

    // 写入专门的 LLM 请求日志文件
    let log_file_path = format!(
        "logs/llm-http-requests-{}.log",
        Utc::now().format("%Y-%m-%d")
    );

    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
    {
        Ok(mut file) => {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                error!("Failed to write to LLM log file {}: {}", log_file_path, e);
            } else {
                let _ = file.flush();
            }
        }
        Err(e) => {
            error!("Failed to open LLM log file {}: {}", log_file_path, e);
        }
    }
}

// ============================================================================
// 便捷工厂函数
// ============================================================================

/// 创建 LLM 客户端
pub fn create_client(ai_service: &crate::services::ai::AiService) -> LlmClient {
    LlmClient::from_ai_service(ai_service)
}

/// 创建流式 LLM 客户端
pub fn create_streaming_client(ai_service: &crate::services::ai::AiService) -> StreamingLlmClient {
    StreamingLlmClient::from_ai_service(ai_service)
}

/// 向后兼容：创建简单 LLM 客户端
pub fn create_simple_client(ai_service: &crate::services::ai::AiService) -> LlmClient {
    create_client(ai_service)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.timeout_secs, 120);
    }
}


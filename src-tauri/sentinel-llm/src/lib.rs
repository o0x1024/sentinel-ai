//! Sentinel LLM 统一客户端库
//!
//! 提供统一的 LLM 调用能力，包括：
//! - 基础调用（completion）
//! - 流式调用（stream_completion）
//! - 多轮对话（chat / stream_chat）
//! - 图片支持（多模态）
//! - Agent 创建（支持多提供商）
//! - AI 服务（AiService）
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use sentinel_llm::{LlmClient, LlmConfig, ChatMessage, AiConfig, AiService};
//!
//! // 使用 LlmClient（简单场景）
//! let config = LlmConfig::new("openai", "gpt-4")
//!     .with_api_key("sk-xxx")
//!     .with_timeout(120);
//! let client = LlmClient::new(config);
//! let response = client.completion(Some("You are helpful"), "Hello").await?;
//!
//! // 使用 AiService（完整场景）
//! let ai_config = AiConfig {
//!     provider: "openai".to_string(),
//!     model: "gpt-4".to_string(),
//!     api_key: Some("sk-xxx".to_string()),
//!     ..Default::default()
//! };
//! let service = AiService::new(ai_config);
//! let response = service.completion(Some("You are helpful"), "Hello").await?;
//! ```

pub mod agent;
mod client;
mod config;
pub mod custom_provider;
pub mod log;
mod message;
pub mod service;
mod streaming;
pub mod types;
pub mod usage;

pub use agent::{get_rig_provider, needs_gemini_config, validate_config};
pub use client::LlmClient;
pub use config::LlmConfig;
pub use log::{log_request, log_request_with_image, log_response, write_llm_log};
pub use message::ImageAttachment;
pub use message::{build_user_message, convert_chat_history, parse_image_from_json, ChatMessage};
pub use service::{AiService, StreamChunk};
pub use streaming::{StreamContent, StreamingLlmClient};
pub use types::{
    AiConfig, AiToolCall, SchedulerConfig, SchedulerStage, StreamError, StreamMessage,
    TaskProgressMessage, TaskStreamMessage, ToolCallResultMessage,
};
pub use usage::{calculate_cost, TokenUsage};

// Re-export rig types for convenience
pub use rig::completion::Message;
pub use rig::message::{AssistantContent, UserContent};
pub use rig::one_or_many::OneOrMany;

// ============== 便利函数 ==============

/// 创建 LLM 配置（便利函数）
pub fn create_llm_config(
    provider: &str,
    model: &str,
    api_key: Option<&str>,
    base_url: Option<&str>,
) -> LlmConfig {
    let mut config = LlmConfig::new(provider, model);
    if let Some(key) = api_key {
        config = config.with_api_key(key);
    }
    if let Some(url) = base_url {
        config = config.with_base_url(url);
    }
    config
}

/// 创建 LLM 客户端（便利函数）
pub fn create_client(config: LlmConfig) -> LlmClient {
    LlmClient::new(config)
}

/// 创建流式 LLM 客户端（便利函数）
pub fn create_streaming_client(config: LlmConfig) -> StreamingLlmClient {
    StreamingLlmClient::new(config)
}

// 向后兼容的类型别名
pub type SimpleLlmClient = LlmClient;

/// 创建简单客户端（向后兼容）
pub fn create_simple_client(config: LlmConfig) -> LlmClient {
    LlmClient::new(config)
}

impl StreamingLlmClient {
    /// 简单的流式完成（无工具支持）
    pub async fn stream_completion<F>(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        on_content: F,
    ) -> anyhow::Result<String>
    where
        F: FnMut(StreamContent) -> bool,
    {
        self.stream_chat(system_prompt, user_prompt, &[], None, on_content)
            .await
    }
}

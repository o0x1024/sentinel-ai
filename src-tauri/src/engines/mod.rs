//! Engines Module
//!
//! Provides LLM client utilities.
//! Agent execution is now handled by rig-core.

pub mod types;
pub mod vision_explorer_v2;

pub use types::*;

// Re-export sentinel_llm LLM client
pub use sentinel_llm::{
    create_client as create_client_raw, create_llm_config as create_llm_config_raw,
    create_streaming_client as create_streaming_client_raw, ChatMessage, ImageAttachment,
    LlmClient, LlmConfig, Message, StreamContent, StreamingLlmClient,
};

use crate::services::ai::AiService;

/// Create LlmConfig from AiService
pub fn create_llm_config(ai_service: &AiService) -> LlmConfig {
    let config = ai_service.get_config();
    let mut llm_config = LlmConfig::new(&config.provider, &config.model)
        .with_api_key(config.api_key.clone().unwrap_or_default())
        .with_base_url(config.api_base.clone().unwrap_or_default())
        .with_timeout(120);

    // 传递 rig_provider（用于决定使用哪个 client）
    if let Some(rig_provider) = &config.rig_provider {
        llm_config = llm_config.with_rig_provider(rig_provider.clone());
    }

    // 传递 max_tokens（用于 Anthropic 等需要显式设置的提供商）
    if let Some(max_tokens) = config.max_tokens {
        llm_config = llm_config.with_max_tokens(max_tokens);
    }

    llm_config
}

/// Create LlmClient from AiService
pub fn create_client(ai_service: &AiService) -> LlmClient {
    LlmClient::new(create_llm_config(ai_service))
}

/// Create StreamingLlmClient from AiService
pub fn create_streaming_client(ai_service: &AiService) -> StreamingLlmClient {
    StreamingLlmClient::new(create_llm_config(ai_service))
}

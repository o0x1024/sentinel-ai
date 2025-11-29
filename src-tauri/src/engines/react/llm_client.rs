//! ReAct 专用 LLM 客户端
//!
//! 直接调用 LLM API，不经过 ai_service，实现对消息流的精确控制

use anyhow::{anyhow, Result};
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::builder::DynClientBuilder;
use rig::completion::Message;
use rig::message::UserContent;
use rig::one_or_many::OneOrMany;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
use std::sync::Arc;
use tauri::AppHandle;
use tracing::{debug, error, info};

use super::message_emitter::ReactMessageEmitter;

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

/// ReAct LLM 客户端
pub struct ReactLlmClient {
    config: LlmConfig,
    emitter: Arc<ReactMessageEmitter>,
}

impl ReactLlmClient {
    pub fn new(config: LlmConfig, emitter: Arc<ReactMessageEmitter>) -> Self {
        Self { config, emitter }
    }

    /// 流式调用 LLM，每个 token 通过 emitter 发送
    /// 返回完整的响应内容
    pub async fn stream_completion(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        iteration: u32,
    ) -> Result<String> {
        let provider = &self.config.provider;
        let model = &self.config.model;

        info!(
            "ReAct LLM stream request - Provider: {}, Model: {}, Iteration: {}",
            provider, model, iteration
        );

        // 创建 agent
        let agent = {
            let client = DynClientBuilder::new();
            let agent_builder = match client.agent(provider, model) {
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
                    if piece.is_empty() {
                        continue;
                    }
                    content.push_str(&piece);
                    // 通过 emitter 发送每个 token
                    self.emitter.emit_content(&piece, false);
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Reasoning(r),
                )) => {
                    let piece = r.reasoning.join("");
                    if !piece.is_empty() {
                        // Reasoning 内容也发送
                        self.emitter.emit_thinking(&piece);
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
            "ReAct LLM response - Iteration: {}, Length: {} chars",
            iteration,
            content.len()
        );

        Ok(content)
    }
}

/// 从 AiService 配置创建 LlmConfig
pub fn create_llm_config_from_ai_service(
    ai_service: &crate::services::ai::AiService,
) -> LlmConfig {
    let config = ai_service.get_config();
    LlmConfig {
        provider: config.provider.clone(),
        model: config.model.clone(),
        api_key: config.api_key.clone(),
        base_url: config.api_base.clone(),
        timeout_secs: 120,
    }
}


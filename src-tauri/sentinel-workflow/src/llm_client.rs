//! Workflow LLM 客户端
//!
//! 直接调用 LLM API，不经过 ai_service

use anyhow::{anyhow, Result};
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::builder::DynClientBuilder;
use rig::completion::Message;
use rig::message::UserContent;
use rig::one_or_many::OneOrMany;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
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
}


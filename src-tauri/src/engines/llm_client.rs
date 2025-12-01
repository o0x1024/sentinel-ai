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
use rig::completion::Message;
use rig::message::UserContent;
use rig::one_or_many::OneOrMany;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
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
        format!(
            "Response ({} chars):\n{}\n",
            resp.len(),
            if resp.len() > 2000 { &resp[..2000] } else { resp }
        )
    } else {
        format!(
            "System Prompt:\n{}\n\nUser Prompt:\n{}\n",
            system_prompt.unwrap_or("(none)"),
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


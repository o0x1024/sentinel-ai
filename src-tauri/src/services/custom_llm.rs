//! 自定义 LLM 客户端模块
//!
//! 为非 rig-core 原生支持的 AI 提供商提供统一的 HTTP 请求接口。
//! 支持 OpenAI 兼容和 Anthropic 兼容的 API 格式。

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info};

/// 自定义 LLM 客户端
#[derive(Clone)]
pub struct CustomLlmClient {
    api_base: String,
    api_key: Option<String>,
    model: String,
    compat_mode: String,
    extra_headers: Option<HashMap<String, String>>,
    timeout_secs: u64,
    client: Client,
}

/// OpenAI 兼容的聊天请求
#[derive(Debug, Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
}

/// Anthropic 兼容的聊天请求
#[derive(Debug, Serialize)]
struct AnthropicChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    max_tokens: u32,
    stream: bool,
}

/// 聊天消息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI 兼容的响应
#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
    #[allow(unused)]
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    #[allow(unused)]
    prompt_tokens: u32,
    #[allow(unused)]
    completion_tokens: u32,
    #[allow(unused)]
    total_tokens: u32,
}

/// Anthropic 兼容的响应
#[derive(Debug, Deserialize)]
struct AnthropicChatResponse {
    content: Vec<AnthropicContent>,
    #[allow(unused)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[allow(unused)]
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    #[allow(unused)]
    input_tokens: u32,
    #[allow(unused)]
    output_tokens: u32,
}

impl CustomLlmClient {
    /// 创建新的自定义 LLM 客户端
    pub fn new(
        api_base: String,
        api_key: Option<String>,
        model: String,
        compat_mode: String,
        extra_headers: Option<HashMap<String, String>>,
        timeout_secs: u64,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_default();

        Self {
            api_base,
            api_key,
            model,
            compat_mode,
            extra_headers,
            timeout_secs,
            client,
        }
    }

    /// 测试连接
    pub async fn test_connection(&self) -> Result<String> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: "Hello, respond with 'OK' if you receive this.".to_string(),
        }];

        match self.completion(None, &messages, None, None).await {
            Ok(response) => Ok(format!(
                "Connection successful! Response: {}",
                response.chars().take(100).collect::<String>()
            )),
            Err(e) => Err(e),
        }
    }

    /// 执行聊天补全请求
    pub async fn completion(
        &self,
        system_prompt: Option<&str>,
        messages: &[ChatMessage],
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<String> {
        match self.compat_mode.as_str() {
            "anthropic" => {
                self.anthropic_completion(system_prompt, messages, temperature, max_tokens)
                    .await
            }
            _ => {
                // 默认使用 OpenAI 兼容格式
                self.openai_completion(system_prompt, messages, temperature, max_tokens)
                    .await
            }
        }
    }

    /// OpenAI 兼容的补全请求
    async fn openai_completion(
        &self,
        system_prompt: Option<&str>,
        messages: &[ChatMessage],
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<String> {
        let url = format!("{}/chat/completions", self.api_base.trim_end_matches('/'));

        // 构建消息列表
        let mut all_messages = Vec::new();
        if let Some(system) = system_prompt {
            all_messages.push(ChatMessage {
                role: "system".to_string(),
                content: system.to_string(),
            });
        }
        all_messages.extend_from_slice(messages);

        let request_body = OpenAiChatRequest {
            model: self.model.clone(),
            messages: all_messages,
            temperature,
            max_tokens,
            stream: false,
        };

        debug!(
            "CustomLlmClient OpenAI request - URL: {}, Model: {}",
            url, self.model
        );

        let mut request = self.client.post(&url).json(&request_body);

        // 添加认证头
        if let Some(api_key) = &self.api_key {
            request = request.bearer_auth(api_key);
        }

        // 添加额外的请求头
        if let Some(headers) = &self.extra_headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        let response = request.send().await.map_err(|e| {
            error!("CustomLlmClient request failed: {}", e);
            anyhow!("Request failed: {}", e)
        })?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| {
            error!("Failed to read response body: {}", e);
            anyhow!("Failed to read response: {}", e)
        })?;

        if !status.is_success() {
            error!(
                "CustomLlmClient error response - Status: {}, Body: {}",
                status, response_text
            );
            return Err(anyhow!(
                "API error ({}): {}",
                status,
                response_text.chars().take(500).collect::<String>()
            ));
        }

        let parsed: OpenAiChatResponse = serde_json::from_str(&response_text).map_err(|e| {
            error!(
                "Failed to parse OpenAI response: {} - Body: {}",
                e, response_text
            );
            anyhow!("Failed to parse response: {}", e)
        })?;

        parsed
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("No response content"))
    }

    /// Anthropic 兼容的补全请求
    async fn anthropic_completion(
        &self,
        system_prompt: Option<&str>,
        messages: &[ChatMessage],
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<String> {
        let url = format!("{}/messages", self.api_base.trim_end_matches('/'));

        // Anthropic 格式：system 是单独的字段，messages 只包含 user/assistant
        let filtered_messages: Vec<ChatMessage> = messages
            .iter()
            .filter(|m| m.role != "system")
            .cloned()
            .collect();

        let request_body = AnthropicChatRequest {
            model: self.model.clone(),
            messages: filtered_messages,
            system: system_prompt.map(|s| s.to_string()),
            temperature,
            max_tokens: max_tokens.unwrap_or(4096),
            stream: false,
        };

        debug!(
            "CustomLlmClient Anthropic request - URL: {}, Model: {}",
            url, self.model
        );

        let mut request = self.client.post(&url).json(&request_body);

        // Anthropic 使用 x-api-key 头
        if let Some(api_key) = &self.api_key {
            request = request.header("x-api-key", api_key);
            request = request.header("anthropic-version", "2023-06-01");
        }

        // 添加额外的请求头
        if let Some(headers) = &self.extra_headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        let response = request.send().await.map_err(|e| {
            error!("CustomLlmClient Anthropic request failed: {}", e);
            anyhow!("Request failed: {}", e)
        })?;

        let status = response.status();
        let response_text = response.text().await.map_err(|e| {
            error!("Failed to read Anthropic response body: {}", e);
            anyhow!("Failed to read response: {}", e)
        })?;

        if !status.is_success() {
            error!(
                "CustomLlmClient Anthropic error - Status: {}, Body: {}",
                status, response_text
            );
            return Err(anyhow!(
                "API error ({}): {}",
                status,
                response_text.chars().take(500).collect::<String>()
            ));
        }

        let parsed: AnthropicChatResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                error!(
                    "Failed to parse Anthropic response: {} - Body: {}",
                    e, response_text
                );
                anyhow!("Failed to parse response: {}", e)
            })?;

        parsed
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| anyhow!("No response content"))
    }

    /// 执行简单的补全（系统提示 + 用户输入）
    pub async fn simple_completion(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
    ) -> Result<String> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: user_prompt.to_string(),
        }];
        self.completion(system_prompt, &messages, None, None).await
    }

    /// 获取模型名称
    pub fn model(&self) -> &str {
        &self.model
    }

    /// 获取 API Base URL
    pub fn api_base(&self) -> &str {
        &self.api_base
    }

    /// 获取兼容模式
    pub fn compat_mode(&self) -> &str {
        &self.compat_mode
    }
}

/// 从提供商配置创建自定义 LLM 客户端
pub fn create_custom_llm_client(
    api_base: &str,
    api_key: Option<&str>,
    model: &str,
    compat_mode: &str,
    extra_headers: Option<HashMap<String, String>>,
    timeout_secs: u64,
) -> CustomLlmClient {
    CustomLlmClient::new(
        api_base.to_string(),
        api_key.map(|s| s.to_string()),
        model.to_string(),
        compat_mode.to_string(),
        extra_headers,
        timeout_secs,
    )
}

/// 判断提供商是否为自定义提供商（需要使用 CustomLlmClient）
pub fn is_custom_provider(provider: &str, compat_mode: Option<&str>) -> bool {
    // 如果明确指定了兼容模式且不是 rig 原生模式，则使用自定义客户端
    if let Some(mode) = compat_mode {
        return !mode.starts_with("rig_");
    }
    
    // rig-core 原生支持的提供商
    let rig_native_providers = [
        "openai",
        "anthropic",
        "gemini",
        "google",
        "ollama",
        "groq",
        "cohere",
        "deepseek",
        "moonshot",
        "xai",
        "openrouter",
        "modelscope",
    ];

    !rig_native_providers
        .iter()
        .any(|p| provider.to_lowercase() == *p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_custom_provider() {
        assert!(!is_custom_provider("openai", None));
        assert!(!is_custom_provider("anthropic", None));
        assert!(is_custom_provider("my_custom_provider", None));
        
        // 测试带兼容模式的情况
        assert!(is_custom_provider("openai", Some("openai")));
        assert!(!is_custom_provider("openai", Some("rig_openai")));
    }
}


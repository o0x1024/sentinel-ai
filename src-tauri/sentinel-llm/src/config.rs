//! LLM 配置模块

use anyhow::{anyhow, Result};
use http::{HeaderMap, HeaderName, HeaderValue};
use rig::{agent::AgentBuilder, client::ClientBuilder, completion::CompletionModel};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// LLM 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// 提供商名称（如 openai, anthropic, gemini 等）
    pub provider: String,
    /// 模型名称
    pub model: String,
    /// API Key
    pub api_key: Option<String>,
    /// API Base URL
    pub base_url: Option<String>,
    /// 请求超时（秒）
    pub timeout_secs: u64,
    /// rig 提供商类型（决定使用哪个 client）
    pub rig_provider: Option<String>,
    /// 对话/执行标识（用于日志与上下文关联）
    pub conversation_id: Option<String>,
    /// 温度参数（控制随机性）
    pub temperature: Option<f32>,
    /// 最大 token 数（用于 Anthropic 等需要显式设置 max_tokens 的提供商）
    pub max_tokens: Option<u32>,
    /// 最大对话轮数（工具调用循环次数）
    pub max_turns: Option<usize>,
    /// 额外 HTTP 请求头
    pub extra_headers: Option<HashMap<String, String>>,
    /// 额外请求体字段
    pub extra_body: Option<Value>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            api_key: None,
            base_url: None,
            timeout_secs: 120,
            rig_provider: None,
            conversation_id: None,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            max_turns: Some(100),
            extra_headers: None,
            extra_body: None,
        }
    }
}

impl LlmConfig {
    /// 创建新配置
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            ..Default::default()
        }
    }

    /// 设置模型
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// 设置 API Key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// 设置 Base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// 设置超时
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// 设置 rig_provider
    pub fn with_rig_provider(mut self, rig_provider: impl Into<String>) -> Self {
        self.rig_provider = Some(rig_provider.into());
        self
    }

    /// 设置 conversation_id
    pub fn with_conversation_id(mut self, conversation_id: impl Into<String>) -> Self {
        self.conversation_id = Some(conversation_id.into());
        self
    }

    /// 设置温度
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// 设置最大 token 数
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// 获取温度（默认 0.7）
    pub fn get_temperature(&self) -> f32 {
        self.temperature.unwrap_or(0.7)
    }

    /// 获取最大 token 数（默认 4096）
    pub fn get_max_tokens(&self) -> u32 {
        self.max_tokens.unwrap_or(4096)
    }

    /// 设置最大对话轮数
    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = Some(max_turns);
        self
    }

    /// 设置额外 HTTP 请求头
    pub fn with_extra_headers(mut self, extra_headers: HashMap<String, String>) -> Self {
        self.extra_headers = Some(extra_headers);
        self
    }

    /// 设置额外请求体字段
    pub fn with_extra_body(mut self, extra_body: Value) -> Self {
        self.extra_body = Some(extra_body);
        self
    }

    /// 获取最大对话轮数（默认 100）
    pub fn get_max_turns(&self) -> usize {
        self.max_turns.unwrap_or(100)
    }

    pub fn apply_extra_headers<Ext, Key, H>(
        &self,
        builder: ClientBuilder<Ext, Key, H>,
    ) -> Result<ClientBuilder<Ext, Key, H>>
    where
        Ext: Clone,
    {
        let Some(extra_headers) = self.extra_headers.as_ref() else {
            return Ok(builder);
        };
        if extra_headers.is_empty() {
            return Ok(builder);
        }

        let mut headers = HeaderMap::new();
        for (key, value) in extra_headers {
            let header_name = HeaderName::from_bytes(key.as_bytes())
                .map_err(|err| anyhow!("Invalid extra header name '{}': {}", key, err))?;
            let header_value = HeaderValue::from_str(value)
                .map_err(|err| anyhow!("Invalid extra header value for '{}': {}", key, err))?;
            headers.insert(header_name, header_value);
        }

        Ok(builder.http_headers(headers))
    }

    pub fn apply_extra_body<M>(
        &self,
        builder: AgentBuilder<M>,
        base_params: Option<Value>,
    ) -> Result<AgentBuilder<M>>
    where
        M: CompletionModel,
    {
        let params = match (base_params, self.extra_body.as_ref()) {
            (None, None) => return Ok(builder),
            (Some(params), None) => params,
            (None, Some(extra)) => extra_body_object(extra)?,
            (Some(base), Some(extra)) => merge_extra_body(base, extra_body_object(extra)?),
        };

        Ok(builder.additional_params(params))
    }

    /// 获取实际使用的 rig provider（优先使用 rig_provider，否则使用 provider）
    pub fn get_effective_rig_provider(&self) -> String {
        normalize_rig_provider_name(
            self.rig_provider
                .as_deref()
                .unwrap_or(self.provider.as_str()),
        )
    }

    /// 设置 rig 库所需的环境变量
    pub fn setup_env_vars(&self) {
        // 使用 rig_provider 来设置环境变量（如果设置了的话）
        let rig_provider = self.get_effective_rig_provider();
        let provider = rig_provider.to_lowercase();

        // 设置 API Key
        if let Some(api_key) = &self.api_key {
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
                "deepseek" => {
                    // DeepSeek 使用 OpenAI 兼容 API
                    std::env::set_var("OPENAI_API_KEY", api_key);
                    std::env::set_var("DEEPSEEK_API_KEY", api_key);
                }
                "groq" => {
                    std::env::set_var("GROQ_API_KEY", api_key);
                }
                "perplexity" => {
                    std::env::set_var("PERPLEXITY_API_KEY", api_key);
                }
                "xai" => {
                    std::env::set_var("XAI_API_KEY", api_key);
                }
                "cohere" => {
                    std::env::set_var("COHERE_API_KEY", api_key);
                }
                "openrouter" => {
                    std::env::set_var("OPENROUTER_API_KEY", api_key);
                }
                "moonshot" => {
                    std::env::set_var("MOONSHOT_API_KEY", api_key);
                }
                _ => {
                    // 默认使用 OpenAI 兼容
                    std::env::set_var("OPENAI_API_KEY", api_key);
                }
            }
        }

        // 设置 Base URL
        if let Some(base_url) = &self.base_url {
            match provider.as_str() {
                "gemini" | "google" => {
                    std::env::set_var("GEMINI_API_BASE", base_url);
                }
                "anthropic" => {
                    std::env::set_var("ANTHROPIC_API_BASE", base_url);
                }
                "moonshot" => {
                    std::env::set_var("MOONSHOT_API_BASE", base_url);
                }
                _ => {
                    // OpenAI 及兼容提供商
                    std::env::set_var("OPENAI_API_BASE", base_url);
                    std::env::set_var("OPENAI_BASE_URL", base_url);
                    std::env::set_var("OPENAI_BASE", base_url);
                }
            }
            tracing::debug!(
                "Set base URL env vars for provider '{}': {}",
                provider,
                base_url
            );
        } else {
            // 为特定提供商设置默认 base URL
            if provider.as_str() == "deepseek" {
                // DeepSeek 使用 OpenAI 兼容模式，需要设置正确的 base URL
                let deepseek_base = "https://api.deepseek.com";
                std::env::set_var("OPENAI_API_BASE", deepseek_base);
                std::env::set_var("OPENAI_BASE_URL", deepseek_base);
                std::env::set_var("OPENAI_BASE", deepseek_base);
                tracing::debug!("Set DeepSeek default base URL: {}", deepseek_base);
            }
        }
    }
}

fn extra_body_object(value: &Value) -> Result<Value> {
    match value {
        Value::Object(map) => Ok(Value::Object(map.clone())),
        _ => Err(anyhow!("extra_body must be a JSON object")),
    }
}

fn merge_extra_body(base: Value, extra: Value) -> Value {
    match (base, extra) {
        (Value::Object(mut base_map), Value::Object(extra_map)) => {
            for (key, value) in extra_map {
                base_map.insert(key, value);
            }
            Value::Object(base_map)
        }
        (base, _) => base,
    }
}

fn normalize_rig_provider_name(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "openapi" => "openai".to_string(),
        "lm studio" | "lmstudio" | "lm_studio" => "openai".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::LlmConfig;

    #[test]
    fn normalizes_openapi_provider_to_openai_rig() {
        let config = LlmConfig::new("openapi", "ark-code-latest");
        assert_eq!(config.get_effective_rig_provider(), "openai");
    }

    #[test]
    fn normalizes_openapi_rig_provider_override_to_openai() {
        let config = LlmConfig::new("openapi", "ark-code-latest").with_rig_provider("openapi");
        assert_eq!(config.get_effective_rig_provider(), "openai");
    }
}

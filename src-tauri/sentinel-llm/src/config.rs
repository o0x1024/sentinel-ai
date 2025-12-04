//! LLM 配置模块

use serde::{Deserialize, Serialize};

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

impl LlmConfig {
    /// 创建新配置
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            ..Default::default()
        }
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

    /// 设置 rig 库所需的环境变量
    pub fn setup_env_vars(&self) {
        let provider = self.provider.to_lowercase();

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
                    std::env::set_var("OPENAI_API_KEY", api_key);
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
        }
    }
}


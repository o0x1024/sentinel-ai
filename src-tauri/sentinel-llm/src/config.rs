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

    /// 获取最大对话轮数（默认 100）
    pub fn get_max_turns(&self) -> usize {
        self.max_turns.unwrap_or(100)
    }

    /// 获取实际使用的 rig provider（优先使用 rig_provider，否则使用 provider）
    pub fn get_effective_rig_provider(&self) -> String {
        self.rig_provider
            .clone()
            .unwrap_or_else(|| self.provider.to_lowercase())
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

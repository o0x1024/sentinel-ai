//! 基础 LLM 客户端

use anyhow::{anyhow, Result};
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::{Chat, Message, Prompt};
use rig::providers::gemini::completion::gemini_api_types::{
    AdditionalParameters, GenerationConfig,
};
use serde_json::json;
use tracing::{error, info};

use crate::config::LlmConfig;
use crate::log::{build_log_session_id, log_error_response, log_request_with_image, log_response};
use crate::message::{build_user_message, convert_chat_history, ChatMessage, ImageAttachment};

/// 基础 LLM 客户端
///
/// 用于非流式调用场景，如规划、分析等。
#[derive(Clone)]
pub struct LlmClient {
    config: LlmConfig,
}

impl LlmClient {
    /// 创建新的客户端
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    /// 获取配置
    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    fn moonshot_thinking_params(&self, model: &str) -> Option<serde_json::Value> {
        let model_lower = model.to_lowercase();
        if !model_lower.contains("kimi-k2.5") {
            return None;
        }
        let provider = self.config.provider.to_lowercase();
        let base = self
            .config
            .base_url
            .as_ref()
            .map(|u| u.to_lowercase())
            .unwrap_or_default();
        if provider.contains("moonshot")
            || provider.contains("moonshut")
            || base.contains("moonshot")
        {
            Some(json!({ "thinking": { "type": "disabled" } }))
        } else {
            None
        }
    }

    fn validate_moonshot_temperature(&self) -> Result<()> {
        let provider = self.config.provider.to_lowercase();
        let base = self
            .config
            .base_url
            .as_ref()
            .map(|u| u.to_lowercase())
            .unwrap_or_default();
        let model = self.config.model.to_lowercase();
        if !(provider.contains("moonshot") || base.contains("moonshot")) {
            return Ok(());
        }
        if !model.contains("kimi-k2.5") {
            return Ok(());
        }

        let temp = self.config.temperature.unwrap_or(0.7);
        if (temp - 0.6).abs() > f32::EPSILON {
            return Err(anyhow!(
                "Moonshot kimi-k2.5 requires temperature=0.6. Set it in AI Settings."
            ));
        }
        Ok(())
    }

    fn apply_generation_settings<M>(
        &self,
        builder: rig::agent::AgentBuilder<M>,
    ) -> Result<rig::agent::AgentBuilder<M>>
    where
        M: rig::completion::CompletionModel,
    {
        self.apply_generation_settings_with_params(builder, None)
    }

    fn apply_generation_settings_with_params<M>(
        &self,
        mut builder: rig::agent::AgentBuilder<M>,
        base_params: Option<serde_json::Value>,
    ) -> Result<rig::agent::AgentBuilder<M>>
    where
        M: rig::completion::CompletionModel,
    {
        if let Some(temp) = self.config.temperature {
            builder = builder.temperature(temp as f64);
        }
        if let Some(max_tokens) = self.config.max_tokens {
            builder = builder.max_tokens(max_tokens as u64);
        }
        self.config.apply_extra_body(builder, base_params)
    }

    /// 简单调用 LLM，返回完整响应
    pub async fn completion(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
    ) -> Result<String> {
        self.chat(system_prompt, user_prompt, &[], None).await
    }

    /// 带图片的调用
    pub async fn completion_with_image(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        image: Option<&ImageAttachment>,
    ) -> Result<String> {
        self.chat(system_prompt, user_prompt, &[], image).await
    }

    /// 多轮对话调用（核心方法）
    pub async fn chat(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        history: &[ChatMessage],
        image: Option<&ImageAttachment>,
    ) -> Result<String> {
        let provider = self.config.provider.to_lowercase();
        // 使用 rig_provider（如果设置了）来选择正确的 client
        let provider_for_agent = self.config.get_effective_rig_provider();
        let model = &self.config.model;
        let conversation_id = self.config.conversation_id.as_deref();
        let session_id = build_log_session_id(conversation_id);

        info!(
            "LlmClient chat - Provider: {}, rig_provider: {}, Model: {}, History: {} messages, Image: {}",
            provider,
            provider_for_agent,
            model,
            history.len(),
            image.is_some()
        );

        let mut system_prompt_with_hack = system_prompt
            .unwrap_or("You are a helpful AI assistant.")
            .to_string();

        // CRITICAL FIX: Moonshot/DeepSeek and other picky providers REQUIRE non-empty assistant messages.
        let provider_lower = provider_for_agent.to_lowercase();
        if (provider_lower.contains("moonshot")
            || provider_lower.contains("deepseek")
            || provider_lower.contains("kimi"))
            && !system_prompt_with_hack.contains("text response")
        {
            system_prompt_with_hack.push_str("\n\nIMPORTANT: You must always provide a brief text response alongside any tool calls. Do not output empty text messages.");
        }
        let preamble = &system_prompt_with_hack;

        // 记录请求日志（含图片标记）
        log_request_with_image(
            &session_id,
            conversation_id,
            &provider,
            model,
            Some(preamble),
            user_prompt,
            image.is_some(),
        );

        // 设置环境变量
        self.config.setup_env_vars();

        // 构建用户消息
        let user_message = build_user_message(user_prompt, image);

        // 转换历史消息
        let chat_history = convert_chat_history(history);

        let timeout = std::time::Duration::from_secs(self.config.timeout_secs);

        // 根据 rig_provider 创建 agent 并执行
        let content_result: Result<String> = match provider_for_agent.as_str() {
            "openai" | "lm studio" | "lmstudio" | "lm_studio" => {
                self.chat_with_openai(model, preamble, user_message, chat_history, timeout)
                    .await
            }
            "moonshot" => {
                self.chat_with_moonshot(model, preamble, user_message, chat_history, timeout)
                    .await
            }
            "anthropic" => {
                self.chat_with_anthropic(model, preamble, user_message, chat_history, timeout)
                    .await
            }
            "gemini" | "google" => {
                self.chat_with_gemini(model, preamble, user_message, chat_history, timeout)
                    .await
            }
            "ollama" => {
                self.chat_with_ollama(model, preamble, user_message, chat_history, timeout)
                    .await
            }
            "deepseek" => {
                self.chat_with_deepseek(model, preamble, user_message, chat_history, timeout)
                    .await
            }
            "openrouter" => {
                self.chat_with_openrouter(model, preamble, user_message, chat_history, timeout)
                    .await
            }
            "xai" => {
                self.chat_with_xai(model, preamble, user_message, chat_history, timeout)
                    .await
            }
            "groq" => {
                self.chat_with_groq(model, preamble, user_message, chat_history, timeout)
                    .await
            }
            _ => {
                // 未知 provider 尝试使用 openai 兼容方式
                info!(
                    "Unknown rig_provider '{}', trying OpenAI compatible mode (via Generic Client)",
                    provider_for_agent
                );
                self.chat_with_generic_openai(model, preamble, user_message, chat_history, timeout)
                    .await
            }
        };
        let content = match content_result {
            Ok(content) => content,
            Err(err) => {
                log_error_response(
                    &session_id,
                    conversation_id,
                    &provider_for_agent,
                    model,
                    "provider_error",
                    &err.to_string(),
                );
                return Err(err);
            }
        };

        if content.trim().is_empty() {
            let err = anyhow!(
                "LLM returned empty response (provider={}, model={})",
                provider_for_agent,
                model
            );
            log_error_response(
                &session_id,
                conversation_id,
                &provider_for_agent,
                model,
                "empty_response",
                &err.to_string(),
            );
            return Err(err);
        }

        // 记录响应日志
        log_response(
            &session_id,
            conversation_id,
            &provider_for_agent,
            model,
            &content,
        );

        info!("LlmClient: Response length: {} chars", content.len());
        Ok(content)
    }

    async fn chat_with_generic_openai(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
    ) -> Result<String> {
        // Use DeepSeek client as a generic OpenAI compatible client
        // because rig's OpenAI client forces the new /v1/responses API
        use rig::providers::deepseek;

        let api_key = self.config.api_key.clone().unwrap_or_default();

        let mut builder =
            deepseek::Client::<rig::http_client::ReqwestClient>::builder().api_key(api_key);
        builder = self.config.apply_extra_headers(builder)?;

        if let Some(base_url) = &self.config.base_url {
            builder = builder.base_url(base_url);
        }

        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build generic client: {}", e))?;

        let builder = client.agent(model).preamble(preamble);
        let agent = self.apply_generation_settings(builder)?.build();
        self.execute_chat(agent, user_message, chat_history, timeout)
            .await
    }

    async fn chat_with_openai(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::openai;

        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;

        // If custom base_url is set, use Chat Completions API (for third-party providers)
        // Otherwise use Responses API (for official OpenAI)
        if let Some(base_url) = &self.config.base_url {
            info!(
                "Using Chat Completions API with custom base URL: {}",
                base_url
            );
            let builder = openai::Client::builder()
                .api_key(api_key)
                .base_url(base_url);
            let client: openai::CompletionsClient = self.config.apply_extra_headers(builder)?
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build OpenAI client: {:?}", e))?
                .completions_api();

            let builder = client.agent(model).preamble(preamble);
            let agent = self.apply_generation_settings(builder)?.build();
            self.execute_chat(agent, user_message, chat_history, timeout)
                .await
        } else {
            info!("Using Responses API for official OpenAI");
            let builder = openai::Client::builder().api_key(api_key);
            let client: openai::Client = self.config.apply_extra_headers(builder)?
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build OpenAI client: {:?}", e))?;

            let builder = client.agent(model).preamble(preamble);
            let agent = self.apply_generation_settings(builder)?.build();
            self.execute_chat(agent, user_message, chat_history, timeout)
                .await
        }
    }

    async fn chat_with_moonshot(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::moonshot;

        let api_key = self
            .config
            .api_key
            .clone()
            .or_else(|| std::env::var("MOONSHOT_API_KEY").ok())
            .ok_or_else(|| anyhow::anyhow!("MOONSHOT_API_KEY not set"))?;

        let mut builder =
            moonshot::Client::<rig::http_client::ReqwestClient>::builder().api_key(api_key);
        builder = self.config.apply_extra_headers(builder)?;

        if let Some(base_url) = &self.config.base_url {
            builder = builder.base_url(base_url);
        }

        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Moonshot client: {:?}", e))?;

        let builder = client.agent(model).preamble(preamble);
        let agent = self
            .apply_generation_settings_with_params(builder, self.moonshot_thinking_params(model))?
            .build();
        self.execute_chat(agent, user_message, chat_history, timeout)
            .await
    }

    async fn chat_with_anthropic(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::anthropic;

        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

        let mut builder =
            anthropic::Client::<rig::http_client::ReqwestClient>::builder().api_key(api_key);
        builder = self.config.apply_extra_headers(builder)?;

        // 检查是否设置了自定义 base_url
        if let Ok(base_url) = std::env::var("ANTHROPIC_API_BASE") {
            if !base_url.is_empty() {
                info!("Using custom Anthropic base URL: {}", base_url);
                builder = builder.base_url(&base_url);
            }
        }

        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Anthropic client: {:?}", e))?;

        let builder = client
            .agent(model)
            .preamble(preamble)
            .max_tokens(self.config.get_max_tokens() as u64);
        let agent = self.apply_generation_settings(builder)?.build();
        self.execute_chat(agent, user_message, chat_history, timeout)
            .await
    }

    async fn chat_with_gemini(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::gemini;
        let client = gemini::Client::from_env();
        let gen_cfg = GenerationConfig::default();
        let cfg = AdditionalParameters::default().with_config(gen_cfg);
        let builder = client.agent(model).preamble(preamble);
        let agent = self
            .apply_generation_settings_with_params(
                builder,
                Some(serde_json::to_value(cfg).unwrap()),
            )?
            .build();
        self.execute_chat(agent, user_message, chat_history, timeout)
            .await
    }

    async fn chat_with_ollama(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::ollama;
        let client = ollama::Client::from_env();
        let builder = client.agent(model).preamble(preamble);
        let agent = self.apply_generation_settings(builder)?.build();
        self.execute_chat(agent, user_message, chat_history, timeout)
            .await
    }

    async fn chat_with_deepseek(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::deepseek;

        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .map_err(|_| anyhow::anyhow!("DEEPSEEK_API_KEY not set"))?;

        let mut builder =
            deepseek::Client::<rig::http_client::ReqwestClient>::builder().api_key(api_key);
        builder = self.config.apply_extra_headers(builder)?;

        // Use custom base_url if configured
        if let Some(base_url) = &self.config.base_url {
            info!("Using custom DeepSeek base URL: {}", base_url);
            builder = builder.base_url(base_url);
        }

        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build DeepSeek client: {}", e))?;

        let builder = client.agent(model).preamble(preamble);
        let agent = self.apply_generation_settings(builder)?.build();
        self.execute_chat(agent, user_message, chat_history, timeout)
            .await
    }

    async fn chat_with_openrouter(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::openrouter;
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENROUTER_API_KEY not set"))?;
        let builder = openrouter::Client::builder().api_key(api_key);
        let client = self
            .config
            .apply_extra_headers(builder)?
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build OpenRouter client: {:?}", e))?;
        let builder = client.agent(model).preamble(preamble);
        let agent = self.apply_generation_settings(builder)?.build();
        self.execute_chat(agent, user_message, chat_history, timeout)
            .await
    }

    async fn chat_with_xai(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::xai;
        let api_key =
            std::env::var("XAI_API_KEY").map_err(|_| anyhow::anyhow!("XAI_API_KEY not set"))?;
        let builder = xai::Client::builder().api_key(api_key);
        let client = self
            .config
            .apply_extra_headers(builder)?
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build xAI client: {:?}", e))?;
        let builder = client.agent(model).preamble(preamble);
        let agent = self.apply_generation_settings(builder)?.build();
        self.execute_chat(agent, user_message, chat_history, timeout)
            .await
    }

    async fn chat_with_groq(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::groq;
        let api_key =
            std::env::var("GROQ_API_KEY").map_err(|_| anyhow::anyhow!("GROQ_API_KEY not set"))?;
        let builder = groq::Client::builder().api_key(api_key);
        let client = self
            .config
            .apply_extra_headers(builder)?
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Groq client: {:?}", e))?;
        let builder = client.agent(model).preamble(preamble);
        let agent = self.apply_generation_settings(builder)?.build();
        self.execute_chat(agent, user_message, chat_history, timeout)
            .await
    }

    async fn execute_chat<M>(
        &self,
        agent: rig::agent::Agent<M>,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
    ) -> Result<String>
    where
        M: rig::completion::CompletionModel + 'static,
    {
        self.validate_moonshot_temperature()?;
        // 非助手场景统一走真正的非流式 completion，避免 SSE/chunked 中途断流。
        let content = if chat_history.is_empty() {
            match tokio::time::timeout(timeout, agent.prompt(user_message)).await {
                Ok(result) => result.map_err(|e| {
                    error!("LLM prompt error: {}", e);
                    anyhow!("LLM prompt error: {}", e)
                })?,
                Err(_) => {
                    error!("LLM request timeout");
                    return Err(anyhow!("LLM request timeout"));
                }
            }
        } else {
            match tokio::time::timeout(timeout, agent.chat(user_message, chat_history)).await {
                Ok(result) => result.map_err(|e| {
                    error!("LLM chat error: {}", e);
                    anyhow!("LLM chat error: {}", e)
                })?,
                Err(_) => {
                    error!("LLM request timeout");
                    return Err(anyhow!("LLM request timeout"));
                }
            }
        };

        if content.trim().is_empty() {
            return Err(anyhow!("LLM finished without textual response"));
        }

        Ok(content)
    }
}

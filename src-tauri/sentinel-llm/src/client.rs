//! 基础 LLM 客户端

use anyhow::{anyhow, Result};
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Message;
use serde_json::json;
use rig::providers::gemini::completion::gemini_api_types::{AdditionalParameters, GenerationConfig};
use rig::streaming::{StreamedAssistantContent, StreamingChat, StreamingPrompt};
use tracing::{debug, error, info};

use crate::config::LlmConfig;
use crate::log::{log_request_with_image, log_response};
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
        if provider.contains("moonshot") || provider.contains("moonshut") || base.contains("moonshot") {
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
        mut builder: rig::agent::AgentBuilder<M>,
    ) -> rig::agent::AgentBuilder<M>
    where
        M: rig::completion::CompletionModel,
    {
        if let Some(temp) = self.config.temperature {
            builder = builder.temperature(temp as f64);
        }
        if let Some(max_tokens) = self.config.max_tokens {
            builder = builder.max_tokens(max_tokens as u64);
        }
        builder
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
        let session_id = uuid::Uuid::new_v4().to_string();

        info!(
            "LlmClient chat - Provider: {}, rig_provider: {}, Model: {}, History: {} messages, Image: {}",
            provider,
            provider_for_agent,
            model,
            history.len(),
            image.is_some()
        );

        let mut system_prompt_with_hack = system_prompt.unwrap_or("You are a helpful AI assistant.").to_string();
        
        // CRITICAL FIX: Moonshot/DeepSeek and other picky providers REQUIRE non-empty assistant messages.
        let provider_lower = provider_for_agent.to_lowercase();
        if provider_lower.contains("moonshot") || provider_lower.contains("deepseek") || provider_lower.contains("kimi") {
            if !system_prompt_with_hack.contains("text response") {
                system_prompt_with_hack.push_str("\n\nIMPORTANT: You must always provide a brief text response alongside any tool calls. Do not output empty text messages.");
            }
        }
        let preamble = &system_prompt_with_hack;

        // 记录请求日志（含图片标记）
        log_request_with_image(
            &session_id,
            self.config.conversation_id.as_deref(),
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
        let content = match provider_for_agent.as_str() {
            "openai" | "lm studio" | "lmstudio" | "lm_studio" => {
                self.chat_with_openai(model, preamble, user_message, chat_history, timeout).await?
            }
            "moonshot" => {
                self.chat_with_moonshot(model, preamble, user_message, chat_history, timeout).await?
            }
            "anthropic" => {
                self.chat_with_anthropic(model, preamble, user_message, chat_history, timeout).await?
            }
            "gemini" | "google" => {
                self.chat_with_gemini(model, preamble, user_message, chat_history, timeout).await?
            }
            "ollama" => {
                self.chat_with_ollama(model, preamble, user_message, chat_history, timeout).await?
            }
            "deepseek" => {
                self.chat_with_deepseek(model, preamble, user_message, chat_history, timeout).await?
            }
            "openrouter" => {
                self.chat_with_openrouter(model, preamble, user_message, chat_history, timeout).await?
            }
            "xai" => {
                self.chat_with_xai(model, preamble, user_message, chat_history, timeout).await?
            }
            "groq" => {
                self.chat_with_groq(model, preamble, user_message, chat_history, timeout).await?
            }
            _ => {
                // 未知 provider 尝试使用 openai 兼容方式
                info!("Unknown rig_provider '{}', trying OpenAI compatible mode (via Generic Client)", provider_for_agent);
                self.chat_with_generic_openai(model, preamble, user_message, chat_history, timeout).await?
            }
        };

        // 记录响应日志
        log_response(
            &session_id,
            self.config.conversation_id.as_deref(),
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
        
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        
        // Apply global proxy configuration
        let builder = reqwest::Client::builder().default_headers(headers);
        let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
        let http_client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;
        
        let mut builder = deepseek::Client::<reqwest::Client>::builder()
            .api_key(api_key)
            .http_client(http_client);

        if let Some(base_url) = &self.config.base_url {
             builder = builder.base_url(base_url);
        }
        
        let client = builder.build()
            .map_err(|e| anyhow::anyhow!("Failed to build generic client: {}", e))?;
        
        let builder = client.agent(model).preamble(preamble);
        let agent = self.apply_generation_settings(builder).build();
        self.execute_chat(agent, user_message, chat_history, timeout).await
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
            info!("Using Chat Completions API with custom base URL: {}", base_url);
            let client: openai::CompletionsClient = openai::Client::builder()
                .api_key(api_key)
                .base_url(base_url)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build OpenAI client: {:?}", e))?
                .completions_api();
            
            let builder = client.agent(model).preamble(preamble);
            let agent = self.apply_generation_settings(builder).build();
            self.execute_chat(agent, user_message, chat_history, timeout).await
        } else {
            info!("Using Responses API for official OpenAI");
            let client: openai::Client = openai::Client::builder()
                .api_key(api_key)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build OpenAI client: {:?}", e))?;
            
            let builder = client.agent(model).preamble(preamble);
            let agent = self.apply_generation_settings(builder).build();
            self.execute_chat(agent, user_message, chat_history, timeout).await
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

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        let builder_req = reqwest::Client::builder().default_headers(headers);
        let builder_req = sentinel_core::global_proxy::apply_proxy_to_client(builder_req).await;
        let http_client = builder_req
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;

        let mut builder = moonshot::Client::<reqwest::Client>::builder()
            .api_key(api_key)
            .http_client(http_client);

        if let Some(base_url) = &self.config.base_url {
            builder = builder.base_url(base_url);
        }

        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Moonshot client: {:?}", e))?;

        let mut builder = client.agent(model).preamble(preamble);
        if let Some(params) = self.moonshot_thinking_params(model) {
            builder = builder.additional_params(params);
        }
        let agent = self.apply_generation_settings(builder).build();
        self.execute_chat(agent, user_message, chat_history, timeout).await
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
        
        // 创建带有正确 Content-Type 的 HTTP 客户端
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        
        // Apply global proxy configuration
        let builder = reqwest::Client::builder().default_headers(headers);
        let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
        let http_client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;
        
        let mut builder = anthropic::Client::<reqwest::Client>::builder()
            .api_key(api_key)
            .http_client(http_client);
        
        // 检查是否设置了自定义 base_url
        if let Ok(base_url) = std::env::var("ANTHROPIC_API_BASE") {
            if !base_url.is_empty() {
                info!("Using custom Anthropic base URL: {}", base_url);
                builder = builder.base_url(&base_url);
            }
        }
        
        let client = builder.build()
            .map_err(|e| anyhow::anyhow!("Failed to build Anthropic client: {:?}", e))?;
        
        let builder = client
            .agent(model)
            .preamble(preamble)
            .max_tokens(self.config.get_max_tokens() as u64);
        let agent = self.apply_generation_settings(builder).build();
        self.execute_chat(agent, user_message, chat_history, timeout).await
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
        let builder = client.agent(model)
            .preamble(preamble)
            .additional_params(serde_json::to_value(cfg).unwrap());
        let agent = self.apply_generation_settings(builder).build();
        self.execute_chat(agent, user_message, chat_history, timeout).await
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
        let agent = self.apply_generation_settings(builder).build();
        self.execute_chat(agent, user_message, chat_history, timeout).await
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
        
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        
        // Apply global proxy configuration
        let builder = reqwest::Client::builder().default_headers(headers);
        let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
        let http_client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;
        
        let mut builder = deepseek::Client::<reqwest::Client>::builder()
            .api_key(api_key)
            .http_client(http_client);
        
        // Use custom base_url if configured
        if let Some(base_url) = &self.config.base_url {
            info!("Using custom DeepSeek base URL: {}", base_url);
            builder = builder.base_url(base_url);
        }
        
        let client = builder.build()
            .map_err(|e| anyhow::anyhow!("Failed to build DeepSeek client: {}", e))?;
        
        let builder = client.agent(model).preamble(preamble);
        let agent = self.apply_generation_settings(builder).build();
        self.execute_chat(agent, user_message, chat_history, timeout).await
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
        let client = openrouter::Client::from_env();
        let builder = client.agent(model).preamble(preamble);
        let agent = self.apply_generation_settings(builder).build();
        self.execute_chat(agent, user_message, chat_history, timeout).await
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
        let client = xai::Client::from_env();
        let builder = client.agent(model).preamble(preamble);
        let agent = self.apply_generation_settings(builder).build();
        self.execute_chat(agent, user_message, chat_history, timeout).await
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
        let client = groq::Client::from_env();
        let builder = client.agent(model).preamble(preamble);
        let agent = self.apply_generation_settings(builder).build();
        self.execute_chat(agent, user_message, chat_history, timeout).await
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
        M::StreamingResponse: Clone + Unpin + rig::completion::GetTokenUsage,
    {
        self.validate_moonshot_temperature()?;
        // Get max_turns from config
        let max_turns = self.config.get_max_turns();
        info!("Using max_turns: {}", max_turns);

        // 根据是否有历史消息选择调用方式
        let stream_result = if chat_history.is_empty() {
            tokio::time::timeout(
                timeout,
                agent.stream_prompt(user_message).multi_turn(max_turns),
            )
            .await
        } else {
            tokio::time::timeout(
                timeout,
                agent.stream_chat(user_message, chat_history).multi_turn(max_turns),
            )
            .await
        };

        let mut stream_iter = match stream_result {
            Ok(iter) => iter,
            Err(_) => {
                error!("LLM request timeout");
                return Err(anyhow!("LLM request timeout"));
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
                    debug!("LLM stream completed");
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    error!("LLM stream error: {}", e);
                    return Err(anyhow!("LLM stream error: {}", e));
                }
            }
        }

        Ok(content)
    }
}

//! AI 服务核心实现
//!
//! 提供统一的 AI 服务接口，不依赖应用特定的类型。

use anyhow::{anyhow, Result};
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::{message::Image, Message};
use rig::message::{
    AssistantContent, DocumentSourceKind, ImageDetail, UserContent,
};
use rig::one_or_many::OneOrMany;
use rig::providers::gemini::completion::gemini_api_types::{
    AdditionalParameters, GenerationConfig,
};
use rig::streaming::{StreamedAssistantContent, StreamingChat};
use tracing::{debug, error, info};

use crate::agent::validate_config;
use crate::config::LlmConfig;
use crate::log::{log_request, log_response};
use crate::message::{ChatMessage, ImageAttachment};
use crate::types::AiConfig;
use crate::usage::TokenUsage;

/// AI 服务 - 无应用依赖版本
#[derive(Clone)]
pub struct AiService {
    config: AiConfig,
}

impl AiService {
    /// 创建新的 AI 服务
    pub fn new(config: AiConfig) -> Self {
        Self { config }
    }

    /// 获取配置
    pub fn get_config(&self) -> &AiConfig {
        &self.config
    }

    /// 转换为 LlmConfig
    pub fn to_llm_config(&self) -> LlmConfig {
        let mut config = LlmConfig::new(&self.config.provider, &self.config.model);
        if let Some(ref key) = self.config.api_key {
            config = config.with_api_key(key);
        }
        if let Some(ref url) = self.config.api_base {
            config = config.with_base_url(url);
        }
        if let Some(ref rig_provider) = self.config.rig_provider {
            config = config.with_rig_provider(rig_provider);
        }
        if let Some(max_turns) = self.config.max_turns {
            config = config.with_max_turns(max_turns);
        }
        config.with_timeout(120)
    }

    /// 流式发送消息（带 token 统计）
    ///
    /// 返回完整响应和 token 使用信息
    pub async fn send_message_stream_with_usage<F>(
        &self,
        user_prompt: &str,
        system_prompt: Option<&str>,
        history: &[ChatMessage],
        image_attachment: Option<ImageAttachment>,
        execution_id: &str,
        conversation_id: Option<&str>,
        mut on_chunk: F,
    ) -> Result<CompletionResponse>
    where
        F: FnMut(StreamChunk) -> bool,
    {
        let mut usage = TokenUsage::default();
        let content = self.send_message_stream_internal(
            user_prompt,
            system_prompt,
            history,
            image_attachment,
            execution_id,
            conversation_id,
            &mut |chunk| {
                if let StreamChunk::Usage { input_tokens, output_tokens } = chunk {
                    usage = TokenUsage::new(input_tokens, output_tokens);
                    usage.estimate_cost(&self.config.provider, &self.config.model);
                }
                on_chunk(chunk)
            },
        ).await?;

        Ok(CompletionResponse { content, usage })
    }

    /// 流式发送消息
    ///
    /// 返回 (完整响应, 流式回调)
    pub async fn send_message_stream<F>(
        &self,
        user_prompt: &str,
        system_prompt: Option<&str>,
        history: &[ChatMessage],
        image_attachment: Option<ImageAttachment>,
        execution_id: &str,
        conversation_id: Option<&str>,
        on_chunk: F,
    ) -> Result<String>
    where
        F: FnMut(StreamChunk) -> bool,
    {
        self.send_message_stream_internal(
            user_prompt,
            system_prompt,
            history,
            image_attachment,
            execution_id,
            conversation_id,
            on_chunk,
        ).await
    }

    /// 内部流式发送消息实现
    async fn send_message_stream_internal<F>(
        &self,
        user_prompt: &str,
        system_prompt: Option<&str>,
        history: &[ChatMessage],
        image_attachment: Option<ImageAttachment>,
        execution_id: &str,
        conversation_id: Option<&str>,
        mut on_chunk: F,
    ) -> Result<String>
    where
        F: FnMut(StreamChunk) -> bool,
    {
        info!("发送流式消息请求 - 模型: {}", self.config.model);

        // 验证配置
        let llm_config = self.to_llm_config();
        if let Err(e) = validate_config(&llm_config) {
            error!("{}", e);
            return Err(e);
        }

        let provider = self.config.provider.to_lowercase();
        // 使用 rig_provider（如果设置了）来选择正确的 client
        let provider_for_agent = llm_config.get_effective_rig_provider();
        let model = self.config.model.clone();

        // 设置环境变量
        llm_config.setup_env_vars();

        // 处理 LM Studio
        if matches!(provider.as_str(), "lm studio" | "lmstudio" | "lm_studio") {
            let mut base = self
                .config
                .api_base
                .clone()
                .unwrap_or_else(|| "http://localhost:1234".to_string());
            if !base.ends_with("/v1") {
                base = format!("{}/v1", base.trim_end_matches('/'));
            }
            std::env::set_var("OPENAI_API_BASE", base.clone());
            std::env::set_var("OPENAI_BASE_URL", base.clone());
            std::env::set_var("OPENAI_BASE", base);

            let needs_set_key = std::env::var("OPENAI_API_KEY")
                .map(|v| v.trim().is_empty())
                .unwrap_or(true);
            if needs_set_key {
                let key = self
                    .config
                    .api_key
                    .clone()
                    .unwrap_or_else(|| "lm-studio".to_string());
                std::env::set_var("OPENAI_API_KEY", key);
            }
        }

        // 构造用户消息
        let user_message: Message = if let Some(img) = image_attachment {
            let image = Image {
                data: DocumentSourceKind::base64(&img.base64_data),
                media_type: Some(img.to_image_media_type()),
                detail: Some(ImageDetail::Auto),
                additional_params: None,
            };
            Message::User {
                content: OneOrMany::one(UserContent::Image(image)),
            }
        } else {
            Message::User {
                content: OneOrMany::one(UserContent::text(user_prompt.to_string())),
            }
        };

        // 转换历史消息
        let chat_history = Self::convert_history(history);
        debug!("Chat history: {} messages converted", chat_history.len());

        let preamble = system_prompt.unwrap_or("You are a helpful AI assistant.");
        let timeout = std::time::Duration::from_secs(120);

        // 记录请求日志
        info!(
            "LLM Request - Provider: {}, Model: {}, Input length: {} chars",
            provider,
            model,
            user_prompt.len()
        );
        log_request(
            execution_id,
            conversation_id,
            &provider,
            &model,
            system_prompt,
            user_prompt,
        );

        // 根据 provider 创建 agent 并执行流式调用
        let content = match provider_for_agent.as_str() {
            "openai" => {
                self.stream_with_openai(
                    &model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    &mut on_chunk,
                )
                .await?
            }
            "anthropic" => {
                self.stream_with_anthropic(
                    &model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    &mut on_chunk,
                )
                .await?
            }
            "gemini" | "google" => {
                self.stream_with_gemini(
                    &model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    &mut on_chunk,
                )
                .await?
            }
            "ollama" => {
                self.stream_with_ollama(
                    &model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    &mut on_chunk,
                )
                .await?
            }
            "deepseek" => {
                self.stream_with_deepseek(
                    &model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    &mut on_chunk,
                )
                .await?
            }
            "openrouter" => {
                self.stream_with_openrouter(
                    &model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    &mut on_chunk,
                )
                .await?
            }
            "xai" => {
                self.stream_with_xai(
                    &model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    &mut on_chunk,
                )
                .await?
            }
            "groq" => {
                self.stream_with_groq(
                    &model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    &mut on_chunk,
                )
                .await?
            }
            _ => {
                info!(
                    "Unknown provider '{}', trying OpenAI compatible mode",
                    provider_for_agent
                );
                self.stream_with_openai(
                    &model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    &mut on_chunk,
                )
                .await?
            }
        };

        // 记录响应日志
        info!(
            "LLM Response - Provider: {}, Model: {}, Output length: {} chars",
            provider,
            model,
            content.len()
        );
        log_response(execution_id, conversation_id, &provider, &model, &content);

        Ok(content)
    }

    async fn stream_with_openai<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_chunk: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamChunk) -> bool,
    {
        use rig::providers::openai;

        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;

        let llm_config = self.to_llm_config();

        // If custom base_url is set, use Chat Completions API (for third-party providers)
        // Otherwise use Responses API (for official OpenAI)
        if let Some(base_url) = &llm_config.base_url {
            info!(
                "Using Chat Completions API with custom base URL: {}",
                base_url
            );
            let client: openai::CompletionsClient = openai::Client::builder()
                .api_key(api_key)
                .base_url(base_url)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build OpenAI client: {:?}", e))?
                .completions_api();

            let agent = client.agent(model).preamble(preamble).build();
            self.execute_stream(agent, user_message, chat_history, timeout, on_chunk)
                .await
        } else {
            info!("Using Responses API for official OpenAI");
            let client: openai::Client = openai::Client::builder()
                .api_key(api_key)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build OpenAI client: {:?}", e))?;

            let agent = client.agent(model).preamble(preamble).build();
            self.execute_stream(agent, user_message, chat_history, timeout, on_chunk)
                .await
        }
    }

    async fn stream_with_anthropic<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_chunk: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamChunk) -> bool,
    {
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
        let builder_req = reqwest::Client::builder().default_headers(headers);
        let builder_req = sentinel_core::global_proxy::apply_proxy_to_client(builder_req).await;
        let http_client = builder_req
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

        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Anthropic client: {:?}", e))?;

        let agent = client
            .agent(model)
            .preamble(preamble)
            .max_tokens(4096)
            .build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_chunk)
            .await
    }

    async fn stream_with_gemini<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_chunk: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamChunk) -> bool,
    {
        use rig::providers::gemini;
        let client = gemini::Client::from_env();
        let gen_cfg = GenerationConfig::default();
        let cfg = AdditionalParameters::default().with_config(gen_cfg);
        let agent = client
            .agent(model)
            .preamble(preamble)
            .additional_params(serde_json::to_value(cfg).unwrap())
            .build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_chunk)
            .await
    }

    async fn stream_with_ollama<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_chunk: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamChunk) -> bool,
    {
        use rig::providers::ollama;
        let client = ollama::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_chunk)
            .await
    }

    async fn stream_with_deepseek<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_chunk: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamChunk) -> bool,
    {
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
        let builder_req = reqwest::Client::builder().default_headers(headers);
        let builder_req = sentinel_core::global_proxy::apply_proxy_to_client(builder_req).await;
        let http_client = builder_req
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;

        let mut builder = deepseek::Client::<reqwest::Client>::builder()
            .api_key(api_key)
            .http_client(http_client);

        // Use custom base_url if configured
        if let Some(ref base_url) = self.config.api_base {
            info!("Using custom DeepSeek base URL: {}", base_url);
            builder = builder.base_url(base_url);
        }

        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build DeepSeek client: {}", e))?;

        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_chunk)
            .await
    }

    async fn stream_with_openrouter<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_chunk: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamChunk) -> bool,
    {
        use rig::providers::openrouter;
        let client = openrouter::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_chunk)
            .await
    }

    async fn stream_with_xai<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_chunk: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamChunk) -> bool,
    {
        use rig::providers::xai;
        let client = xai::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_chunk)
            .await
    }

    async fn stream_with_groq<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_chunk: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamChunk) -> bool,
    {
        use rig::providers::groq;
        let client = groq::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_chunk)
            .await
    }

    async fn execute_stream<M, F>(
        &self,
        agent: rig::agent::Agent<M>,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_chunk: &mut F,
    ) -> Result<String>
    where
        M: rig::completion::CompletionModel + 'static,
        M::StreamingResponse: Clone + Unpin + rig::completion::GetTokenUsage,
        F: FnMut(StreamChunk) -> bool,
    {
        // Get max_turns from config
        let max_turns = self.config.max_turns.unwrap_or(100);
        info!("Using max_turns: {}", max_turns);

        // 流式调用
        let stream_result = tokio::time::timeout(
            timeout,
            agent
                .stream_chat(user_message, chat_history)
                .multi_turn(max_turns),
        )
        .await;

        let mut stream_iter = match stream_result {
            Ok(iter) => iter,
            Err(_) => {
                error!("LLM request timeout after 120 seconds");
                return Err(anyhow!(
                    "LLM request timeout: The AI service did not respond within 120 seconds."
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
                        if !on_chunk(StreamChunk::Text(piece)) {
                            break;
                        }
                    }
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Reasoning(r),
                )) => {
                    let piece = r.reasoning.join("");
                    if !piece.is_empty()
                        && !on_chunk(StreamChunk::Reasoning(piece)) {
                            break;
                        }
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::ToolCall(_),
                )) => {}
                Ok(MultiTurnStreamItem::FinalResponse(resp)) => {
                    let usage = resp.usage();
                    let _ = on_chunk(StreamChunk::Usage {
                        input_tokens: usage.input_tokens as u32,
                        output_tokens: usage.output_tokens as u32,
                    });
                    let _ = on_chunk(StreamChunk::Done);
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    error!("Stream error: {}", e);
                    return Err(anyhow!("Stream error: {}", e));
                }
            }
        }

        Ok(content)
    }

    /// 简单调用（非流式）
    pub async fn completion(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
    ) -> Result<String> {
        let mut result = String::new();
        self.send_message_stream(
            user_prompt,
            system_prompt,
            &[],
            None,
            &uuid::Uuid::new_v4().to_string(),
            None,
            |chunk| {
                if let StreamChunk::Text(text) = chunk {
                    result.push_str(&text);
                }
                true
            },
        )
        .await?;
        Ok(result)
    }

    /// 转换历史消息
    fn convert_history(history: &[ChatMessage]) -> Vec<Message> {
        history
            .iter()
            .filter_map(|msg| {
                let content = msg.content.trim();
                if content.is_empty() {
                    return None;
                }
                match msg.role.to_lowercase().as_str() {
                    "user" => Some(Message::User {
                        content: OneOrMany::one(UserContent::text(content.to_string())),
                    }),
                    "assistant" => Some(Message::Assistant {
                        id: None,
                        content: OneOrMany::one(AssistantContent::Text(rig::message::Text::from(
                            content.to_string(),
                        ))),
                    }),
                    _ => None,
                }
            })
            .collect()
    }
}

/// 流式响应块
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// 文本内容
    Text(String),
    /// 推理内容
    Reasoning(String),
    /// 用量统计
    Usage {
        input_tokens: u32,
        output_tokens: u32,
    },
    /// 完成
    Done,
}

/// 带 token 使用信息的响应
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    /// 响应内容
    pub content: String,
    /// Token 使用统计
    pub usage: TokenUsage,
}

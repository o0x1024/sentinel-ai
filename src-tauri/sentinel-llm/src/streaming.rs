//! 流式 LLM 客户端

use anyhow::{anyhow, Result};
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Message;
use rig::providers::gemini::completion::gemini_api_types::{AdditionalParameters, GenerationConfig};
use rig::streaming::{StreamedAssistantContent, StreamingChat, StreamingPrompt};
use tracing::{debug, error, info};

use crate::agent::get_rig_provider;
use crate::config::LlmConfig;
use crate::log::{log_request, log_response};
use crate::message::{build_user_message, convert_chat_history, ChatMessage, ImageAttachment};

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

/// 流式 LLM 客户端
///
/// 支持回调处理每个 token，适用于需要实时展示的场景。
pub struct StreamingLlmClient {
    config: LlmConfig,
}

impl StreamingLlmClient {
    /// 创建新的流式客户端
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    /// 获取配置
    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// 流式调用 LLM
    pub async fn stream_completion<F>(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        on_content: F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        self.stream_chat(system_prompt, user_prompt, &[], None, on_content)
            .await
    }

    /// 流式调用（带图片）
    pub async fn stream_completion_with_image<F>(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        image: Option<&ImageAttachment>,
        on_content: F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        self.stream_chat(system_prompt, user_prompt, &[], image, on_content)
            .await
    }

    /// 流式多轮对话（核心方法）
    pub async fn stream_chat<F>(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        history: &[ChatMessage],
        image: Option<&ImageAttachment>,
        mut on_content: F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        let provider = self.config.provider.to_lowercase();
        let provider_for_agent = get_rig_provider(&provider);
        let model = &self.config.model;
        let session_id = uuid::Uuid::new_v4().to_string();

        info!(
            "StreamingLlmClient chat - Provider: {}, Model: {}, History: {} messages, Image: {}",
            provider,
            model,
            history.len(),
            image.is_some()
        );

        // 记录请求日志
        log_request(&session_id, None, &provider, model, system_prompt, user_prompt);

        // 设置环境变量
        self.config.setup_env_vars();

        // LM Studio 特殊处理
        if matches!(provider.as_str(), "lm studio" | "lmstudio" | "lm_studio") {
            let mut base = self
                .config
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:1234".to_string());
            if !base.ends_with("/v1") {
                base = format!("{}/v1", base.trim_end_matches('/'));
            }
            std::env::set_var("OPENAI_API_BASE", base.clone());
            std::env::set_var("OPENAI_BASE_URL", base.clone());
            std::env::set_var("OPENAI_BASE", base);

            if std::env::var("OPENAI_API_KEY")
                .map(|v| v.trim().is_empty())
                .unwrap_or(true)
            {
                let key = self
                    .config
                    .api_key
                    .clone()
                    .unwrap_or_else(|| "lm-studio".to_string());
                std::env::set_var("OPENAI_API_KEY", key);
            }
        }

        // 构建用户消息
        let user_message = build_user_message(user_prompt, image);

        // 转换历史消息
        let chat_history = convert_chat_history(history);

        let preamble = system_prompt.unwrap_or("You are a helpful AI assistant.");
        let timeout = std::time::Duration::from_secs(self.config.timeout_secs);

        // 根据 provider 创建 agent 并执行流式调用
        let content = match provider_for_agent.as_str() {
            "openai" => {
                self.stream_with_openai(model, preamble, user_message, chat_history, timeout, &mut on_content).await?
            }
            "anthropic" => {
                self.stream_with_anthropic(model, preamble, user_message, chat_history, timeout, &mut on_content).await?
            }
            "gemini" | "google" => {
                self.stream_with_gemini(model, preamble, user_message, chat_history, timeout, &mut on_content).await?
            }
            "ollama" => {
                self.stream_with_ollama(model, preamble, user_message, chat_history, timeout, &mut on_content).await?
            }
            "deepseek" => {
                self.stream_with_deepseek(model, preamble, user_message, chat_history, timeout, &mut on_content).await?
            }
            "openrouter" => {
                self.stream_with_openrouter(model, preamble, user_message, chat_history, timeout, &mut on_content).await?
            }
            "xai" => {
                self.stream_with_xai(model, preamble, user_message, chat_history, timeout, &mut on_content).await?
            }
            "groq" => {
                self.stream_with_groq(model, preamble, user_message, chat_history, timeout, &mut on_content).await?
            }
            _ => {
                info!("Unknown provider '{}', trying OpenAI compatible mode", provider_for_agent);
                self.stream_with_openai(model, preamble, user_message, chat_history, timeout, &mut on_content).await?
            }
        };

        // 记录响应日志
        log_response(&session_id, None, &provider, model, &content);

        info!(
            "StreamingLlmClient: Response length: {} chars",
            content.len()
        );
        Ok(content)
    }

    async fn stream_with_openai<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::openai;
        let client = openai::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    async fn stream_with_anthropic<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::anthropic;
        let client = anthropic::Client::from_env();
        let agent = client.agent(model).preamble(preamble).max_tokens(4096).build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    async fn stream_with_gemini<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::gemini;
        let client = gemini::Client::from_env();
        let gen_cfg = GenerationConfig::default();
        let cfg = AdditionalParameters::default().with_config(gen_cfg);
        let agent = client.agent(model)
            .preamble(preamble)
            .additional_params(serde_json::to_value(cfg).unwrap())
            .build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    async fn stream_with_ollama<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::ollama;
        let client = ollama::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    async fn stream_with_deepseek<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::deepseek;
        let client = deepseek::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    async fn stream_with_openrouter<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::openrouter;
        let client = openrouter::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    async fn stream_with_xai<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::xai;
        let client = xai::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    async fn stream_with_groq<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::groq;
        let client = groq::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    async fn execute_stream<M, F>(
        &self,
        agent: rig::agent::Agent<M>,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_content: &mut F,
    ) -> Result<String>
    where
        M: rig::completion::CompletionModel + 'static,
        M::StreamingResponse: Clone + Unpin + rig::completion::GetTokenUsage,
        F: FnMut(StreamContent),
    {
        // 根据是否有历史消息选择调用方式
        let stream_result = if chat_history.is_empty() {
            tokio::time::timeout(
                timeout,
                agent.stream_prompt(user_message).multi_turn(100),
            )
            .await
        } else {
            tokio::time::timeout(
                timeout,
                agent.stream_chat(user_message, chat_history).multi_turn(100),
            )
            .await
        };

        let mut stream_iter = match stream_result {
            Ok(iter) => iter,
            Err(_) => {
                error!("LLM request timeout after {} seconds", self.config.timeout_secs);
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
                    debug!("LLM stream completed");
                    on_content(StreamContent::Done);
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

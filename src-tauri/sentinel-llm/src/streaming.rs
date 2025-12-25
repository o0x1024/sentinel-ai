//! 流式 LLM 客户端
use anyhow::{anyhow, Result};
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Message;
use rig::providers::gemini::completion::gemini_api_types::{
    AdditionalParameters, GenerationConfig,
};
use rig::streaming::{StreamedAssistantContent, StreamingChat, StreamingPrompt};
use rig::tool::server::ToolServer;
use tracing::{error, info};
use std::collections::HashMap;

use crate::config::LlmConfig;
use crate::log::{log_request, log_response};
use crate::message::{build_user_message, convert_chat_history, ChatMessage, ImageAttachment};
use sentinel_tools::DynamicTool;

/// 流式内容类型
#[derive(Debug, Clone)]
pub enum StreamContent {
    /// 文本内容
    Text(String),
    /// 推理内容（思考过程）
    Reasoning(String),
    /// 工具调用开始（tool_call_id, tool_name）
    ToolCallStart { id: String, name: String },
    /// 工具调用参数增量（tool_call_id, arguments_delta）
    ToolCallDelta { id: String, delta: String },
    /// 工具调用完成（tool_call_id, tool_name, arguments）
    ToolCallComplete {
        id: String,
        name: String,
        arguments: String,
    },
    /// 工具执行结果（tool_call_id, result）
    ToolResult { id: String, result: String },
    /// 用量统计
    Usage {
        input_tokens: u32,
        output_tokens: u32,
    },
    /// 流完成
    Done,
}

/// 流式 LLM 客户端
pub struct StreamingLlmClient {
    config: LlmConfig,
}

impl StreamingLlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    /// 流式对话（无工具）
    pub async fn stream_chat<F>(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        history: &[ChatMessage],
        image: Option<&ImageAttachment>,
        on_content: F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        self.stream_chat_with_dynamic_tools(
            system_prompt,
            user_prompt,
            history,
            image,
            vec![],
            on_content,
        )
        .await
    }

    /// 流式多轮对话（带动态工具支持 - 使用 rig-core 原生工具调用）
    /// 支持所有 rig-core 提供商: openai, anthropic, gemini, deepseek, ollama, openrouter, xai, groq
    pub async fn stream_chat_with_dynamic_tools<F>(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        history: &[ChatMessage],
        image: Option<&ImageAttachment>,
        dynamic_tools: Vec<DynamicTool>,
        mut on_content: F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        let provider = self.config.provider.to_lowercase();
        let provider_for_agent = self.config.get_effective_rig_provider();
        let model = &self.config.model;
        let session_id = uuid::Uuid::new_v4().to_string();
        let tool_names: Vec<String> = dynamic_tools.iter().map(|t| t.name().to_string()).collect();

        info!(
            "StreamingLlmClient - Provider: {}, Model: {}, Tools: {:?}, History: {} messages",
            provider,
            model,
            tool_names,
            history.len()
        );

        let preamble = system_prompt.unwrap_or("You are a helpful AI assistant.");

        log_request(
            &session_id,
            None,
            &provider,
            model,
            Some(preamble),
            user_prompt,
        );

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

            unsafe {
                std::env::set_var("OPENAI_API_BASE", base.clone());
                std::env::set_var("OPENAI_BASE_URL", base.clone());
                std::env::set_var("OPENAI_BASE", base);
            }
            if std::env::var("OPENAI_API_KEY")
                .map(|v| v.trim().is_empty())
                .unwrap_or(true)
            {
                let key = self
                    .config
                    .api_key
                    .clone()
                    .unwrap_or_else(|| "lm-studio".to_string());
                unsafe {
                    std::env::set_var("OPENAI_API_KEY", key);
                }
            }
        }

        let user_message = build_user_message(user_prompt, image);
        let chat_history = convert_chat_history(history);
        let timeout = std::time::Duration::from_secs(self.config.timeout_secs);

        // 根据 provider 创建带动态工具的 agent
        let content = match provider_for_agent.as_str() {
            "openai" => {
                self.stream_with_openai(model, preamble, user_message, chat_history, timeout, dynamic_tools, &mut on_content).await?
            }
            "anthropic" => {
                self.stream_with_anthropic(model, preamble, user_message, chat_history, timeout, dynamic_tools, &mut on_content).await?
            }
            "gemini" | "google" => {
                self.stream_with_gemini(model, preamble, user_message, chat_history, timeout, dynamic_tools, &mut on_content).await?
            }
            "deepseek" => {
                self.stream_with_deepseek(model, preamble, user_message, chat_history, timeout, dynamic_tools, &mut on_content).await?
            }
            "ollama" => {
                self.stream_with_ollama(model, preamble, user_message, chat_history, timeout, dynamic_tools, &mut on_content).await?
            }
            "openrouter" => {
                self.stream_with_openrouter(model, preamble, user_message, chat_history, timeout, dynamic_tools, &mut on_content).await?
            }
            "xai" => {
                self.stream_with_xai(model, preamble, user_message, chat_history, timeout, dynamic_tools, &mut on_content).await?
            }
            "groq" => {
                self.stream_with_groq(model, preamble, user_message, chat_history, timeout, dynamic_tools, &mut on_content).await?
            }
            _ => {
                info!(
                    "Unknown provider '{}', trying OpenAI compatible mode (via Generic Client)",
                    provider_for_agent
                );
                self.stream_with_generic_openai(model, preamble, user_message, chat_history, timeout, dynamic_tools, &mut on_content).await?
            }
        };

        log_response(&session_id, None, &provider, model, &content);
        info!(
            "StreamingLlmClient: Response length: {} chars",
            content.len()
        );
        Ok(content)
    }

    // ==================== Provider 实现 ====================

    async fn stream_with_generic_openai<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        dynamic_tools: Vec<DynamicTool>,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::deepseek;
        
        let api_key = self.config.api_key.clone().unwrap_or_default();
        
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

        if let Some(base_url) = &self.config.base_url {
             builder = builder.base_url(base_url);
        }
        
        let client = builder.build()
            .map_err(|e| anyhow::anyhow!("Failed to build generic client: {}", e))?;
        
        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let agent = client
            .agent(model)
            .preamble(preamble)
            .tool_server_handle(tool_server_handle)
            .build();
            
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    async fn stream_with_openai<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        dynamic_tools: Vec<DynamicTool>,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::openai;
        
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
        
        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        
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
            
            let agent = client
                .agent(model)
                .preamble(preamble)
                .tool_server_handle(tool_server_handle)
                .build();
            self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
        } else {
            info!("Using Responses API for official OpenAI");
            let client: openai::Client = openai::Client::builder()
                .api_key(api_key)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build OpenAI client: {:?}", e))?;
            
            let agent = client
                .agent(model)
                .preamble(preamble)
                .tool_server_handle(tool_server_handle)
                .build();
            self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
        }
    }

    async fn stream_with_anthropic<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        dynamic_tools: Vec<DynamicTool>,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::anthropic;
        
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;
        
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
            
        if let Ok(base_url) = std::env::var("ANTHROPIC_API_BASE") {
            if !base_url.is_empty() {
                builder = builder.base_url(&base_url);
            }
        }
        
        let client = builder.build()
            .map_err(|e| anyhow::anyhow!("Failed to build Anthropic client: {:?}", e))?;
        
        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let agent = client
            .agent(model)
            .preamble(preamble)
            .max_tokens(self.config.get_max_tokens() as u64)
            .tool_server_handle(tool_server_handle)
            .build();
            
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    async fn stream_with_gemini<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        dynamic_tools: Vec<DynamicTool>,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::gemini;
        let client = gemini::Client::from_env();
        let gen_cfg = GenerationConfig::default();
        let cfg = AdditionalParameters::default().with_config(gen_cfg);
        
        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let agent = client
            .agent(model)
            .preamble(preamble)
            .additional_params(serde_json::to_value(cfg).unwrap())
            .tool_server_handle(tool_server_handle)
            .build();
            
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    async fn stream_with_deepseek<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        dynamic_tools: Vec<DynamicTool>,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
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
        if let Some(base_url) = &self.config.base_url {
            info!("Using custom DeepSeek base URL: {}", base_url);
            builder = builder.base_url(base_url);
        }
        
        let client = builder.build()
            .map_err(|e| anyhow::anyhow!("Failed to build DeepSeek client: {}", e))?;
        
        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let agent = client
            .agent(model)
            .preamble(preamble)
            .tool_server_handle(tool_server_handle)
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
        dynamic_tools: Vec<DynamicTool>,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::ollama;
        let client = ollama::Client::from_env();
        
        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let agent = client
            .agent(model)
            .preamble(preamble)
            .tool_server_handle(tool_server_handle)
            .build();
            
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    async fn stream_with_openrouter<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        dynamic_tools: Vec<DynamicTool>,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::openrouter;
        let client = openrouter::Client::from_env();
        
        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let agent = client
            .agent(model)
            .preamble(preamble)
            .tool_server_handle(tool_server_handle)
            .build();
            
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    async fn stream_with_xai<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        dynamic_tools: Vec<DynamicTool>,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::xai;
        let client = xai::Client::from_env();
        
        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let agent = client
            .agent(model)
            .preamble(preamble)
            .tool_server_handle(tool_server_handle)
            .build();
            
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    async fn stream_with_groq<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        dynamic_tools: Vec<DynamicTool>,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent),
    {
        use rig::providers::groq;
        let client = groq::Client::from_env();
        
        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let agent = client
            .agent(model)
            .preamble(preamble)
            .tool_server_handle(tool_server_handle)
            .build();
            
        self.execute_stream(agent, user_message, chat_history, timeout, on_content).await
    }

    // ==================== 辅助方法 ====================

    fn build_tool_server(dynamic_tools: Vec<DynamicTool>) -> rig::tool::server::ToolServerHandle {
        let mut tool_server = ToolServer::new();
        for tool in dynamic_tools {
            info!("Adding dynamic tool to agent: {}", tool.name());
            tool_server = tool_server.tool(tool);
        }
        tool_server.run()
    }

    async fn execute_stream<M, F>(
        &self,
        agent: rig::agent::Agent<M>,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        mut on_content: F,
    ) -> Result<String>
    where
        M: rig::completion::CompletionModel + 'static,
        M::StreamingResponse: Clone + Unpin + rig::completion::GetTokenUsage,
        F: FnMut(StreamContent),
    {
        let mut tool_call_args: HashMap<String, String> = HashMap::new();
        let mut tool_call_names: HashMap<String, String> = HashMap::new();
        info!("Starting stream iteration...");

        let stream_result = if chat_history.is_empty() {
            info!("Using stream_prompt for empty chat history");
            tokio::time::timeout(
                timeout,
                agent.stream_prompt(user_message).multi_turn(100),
            )
            .await
        } else {
            info!("Using stream_chat with {} history messages", chat_history.len());
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

        let mut content = String::new();
        let mut chunk_count = 0;

        while let Some(item) = stream_iter.next().await {
            chunk_count += 1;
            match item {
                // 文本内容
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t))) => {
                    let piece = t.text;
                    if !piece.is_empty() {
                        content.push_str(&piece);
                        on_content(StreamContent::Text(piece));
                    }
                }
                // 推理内容
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Reasoning(r),
                )) => {
                    let piece = r.reasoning.join("");
                    if !piece.is_empty() {
                        on_content(StreamContent::Reasoning(piece));
                    }
                }
                // 完整的工具调用
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::ToolCall(tool_call),
                )) => {
                    info!(
                        "Tool call received: id={}, name={}, args={}",
                        tool_call.id, tool_call.function.name, tool_call.function.arguments
                    );
                    on_content(StreamContent::ToolCallComplete {
                        id: tool_call.id.clone(),
                        name: tool_call.function.name.clone(),
                        arguments: tool_call.function.arguments.to_string(),
                    });
                    tool_call_names.insert(tool_call.id.clone(), tool_call.function.name.clone());
                }
                // 工具调用增量
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::ToolCallDelta { id, delta },
                )) => {
                    tool_call_args
                        .entry(id.clone())
                        .or_default()
                        .push_str(&delta);
                    on_content(StreamContent::ToolCallDelta { id, delta });
                }
                // 工具执行结果
                Ok(MultiTurnStreamItem::StreamUserItem(user_content)) => {
                    let rig::streaming::StreamedUserContent::ToolResult(tool_result) = user_content;
                    let result_str = serde_json::to_string(&tool_result.content).unwrap_or_default();
                    info!(
                        "Tool result received: id={}, result_len={}, content_preview={}",
                        tool_result.id,
                        result_str.len(),
                        &result_str.chars().take(300).collect::<String>()
                    );
                    on_content(StreamContent::ToolResult {
                        id: tool_result.id,
                        result: result_str,
                    });
                }
                // 最终响应
                Ok(MultiTurnStreamItem::FinalResponse(final_resp)) => {
                    info!(
                        "Stream completed after {} chunks, total content: {} chars",
                        chunk_count,
                        content.len()
                    );
                    let final_text = final_resp.response();
                    if !final_text.is_empty() && !content.ends_with(final_text) {
                        content.push_str(final_text);
                    }
                    
                    let usage = final_resp.usage();
                    on_content(StreamContent::Usage {
                        input_tokens: usage.input_tokens as u32,
                        output_tokens: usage.output_tokens as u32,
                    });
                    
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
        info!(
            "Stream iteration ended, total chunks: {}, content length: {}",
            chunk_count,
            content.len()
        );
        Ok(content)
    }
}

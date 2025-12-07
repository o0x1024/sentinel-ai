//! ReAct 消息发送器和专用 LLM 客户端
//!
//! 简化版：直接发送流式内容到前端，并收集完整内容用于保存

use crate::engines::LlmConfig;
use crate::utils::ordered_message::{emit_message_chunk_with_arch, ArchitectureType, ChunkType};
use anyhow::{anyhow, Result};
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::Message;
use rig::message::UserContent;
use rig::one_or_many::OneOrMany;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use tauri::AppHandle;
use tracing::{debug, error, info};

/// ReAct 消息发送器
pub struct ReactMessageEmitter {
    app_handle: Arc<AppHandle>,
    execution_id: String,
    message_id: String,
    conversation_id: Option<String>,
    /// 收集所有发送的内容，用于保存到数据库
    content_collector: Arc<Mutex<String>>,
}

/// 执行统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactExecutionStats {
    pub total_iterations: u32,
    pub tool_calls_count: u32,
    pub successful_tool_calls: u32,
    pub failed_tool_calls: u32,
    pub total_duration_ms: u64,
    pub status: String,
}

impl ReactMessageEmitter {
    pub fn new(
        app_handle: Arc<AppHandle>,
        execution_id: String,
        message_id: String,
        conversation_id: Option<String>,
    ) -> Self {
        Self {
            app_handle,
            execution_id,
            message_id,
            conversation_id,
            content_collector: Arc::new(Mutex::new(String::new())),
        }
    }

    /// 获取收集的完整内容（用于保存到数据库）
    pub fn get_full_content(&self) -> String {
        self.content_collector.lock().unwrap().clone()
    }

    /// 发送执行开始信号
    pub fn emit_start(&self, config: Option<serde_json::Value>) {
        self.emit_meta("start", serde_json::json!({
            "type": "start",
            "config": config
        }));
    }

    /// 发送执行完成信号
    pub fn emit_complete(&self, stats: ReactExecutionStats) {
        // 发送完成信号（is_final = true）
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            "",
            true, // is_final
            Some("complete"),
            None,
            Some(ArchitectureType::ReAct),
            Some(serde_json::json!({
                "type": "complete",
                "statistics": stats
            })),
        );
    }

    /// 发送流式内容 chunk（LLM 输出的每个 token）
    pub fn emit_content(&self, content: &str, is_final: bool) {
        // 收集内容用于保存到数据库
        if let Ok(mut collector) = self.content_collector.lock() {
            collector.push_str(content);
        }
        
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Content,
            content,
            is_final,
            None,
            None,
            Some(ArchitectureType::ReAct),
            None,
        );
    }

    /// 发送思考内容 chunk（用于显示 LLM 的 reasoning 过程）
    pub fn emit_thinking(&self, content: &str) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Thinking,
            content,
            false,
            None,
            None,
            Some(ArchitectureType::ReAct),
            None,
        );
    }

    /// 发送错误消息
    pub fn emit_error(&self, error_message: &str) {
        let content = format!(
            "\n\n---\n❌ **执行错误**\n\n{}\n",
            error_message
        );
        
        // 发送内容
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Error,
            &content,
            true, // is_final
            Some("error"),
            None,
            Some(ArchitectureType::ReAct),
            None,
        );
        
        // 收集到完整内容
        if let Ok(mut collector) = self.content_collector.lock() {
            collector.push_str(&content);
        }
    }

    /// 发送步骤进度更新
    pub fn emit_step_progress(
        &self,
        step_id: &str,
        step_description: &str,
        status: &str,  // "running", "completed", "failed", "skipped"
        completed_count: usize,
        total_count: usize,
    ) {
        let status_icon = match status {
            "completed" => "✅",
            "failed" => "❌",
            "running" => "🔄",
            "skipped" => "⏭️",
            _ => "⏳",
        };
        
        let progress_percent = if total_count > 0 {
            (completed_count * 100) / total_count
        } else {
            0
        };
        
        let content = format!(
            "\n📊 **进度更新**: [{}] {} {} ({}/{}，{}%)\n",
            step_id, status_icon, step_description, completed_count, total_count, progress_percent
        );
        
        self.emit_content(&content, false);
        
        // 发送结构化进度数据
        self.emit_step("progress", serde_json::json!({
            "type": "progress",
            "step_id": step_id,
            "step_description": step_description,
            "status": status,
            "completed": completed_count,
            "total": total_count,
            "percent": progress_percent
        }));
    }

    /// 发送工具调用信息（内联 markdown 格式 + 结构化数据）
    pub fn emit_tool_call(&self, iteration: u32, tool_name: &str, args: &serde_json::Value) {
        let args_str = serde_json::to_string_pretty(args).unwrap_or_default();
        let content = format!(
            "\n\n---\n🔧 **调用工具: `{}`**\n<details>\n<summary>📥 参数</summary>\n\n```json\n{}\n```\n</details>\n",
            tool_name, args_str
        );
        self.emit_content(&content, false);

        // 同时发送结构化数据（用于状态追踪）
        self.emit_step("action", serde_json::json!({
            "type": "step",
            "step": {
                "index": iteration.saturating_sub(1),
                "action": {
                    "tool": tool_name,
                    "args": args,
                    "status": "running"
                }
            }
        }));
    }

    /// 发送工具执行结果（内联 markdown 格式 + 结构化数据）
    pub fn emit_tool_result(&self, iteration: u32, tool_name: &str, args: &serde_json::Value, result: &serde_json::Value, success: bool, duration_ms: u64) {
        let status_icon = if success { "✅" } else { "❌" };
        let result_str = serde_json::to_string_pretty(result).unwrap_or_default();
        let content = format!(
            "<details>\n<summary>{} 结果 ({}ms)</summary>\n\n```json\n{}\n```\n</details>\n---\n\n",
            status_icon, duration_ms, result_str
        );
        self.emit_content(&content, false);

        // 同时发送结构化数据（用于状态追踪）
        let status = if success { "completed" } else { "failed" };
        self.emit_step("observation", serde_json::json!({
            "type": "step",
            "step": {
                "index": iteration.saturating_sub(1),
                "action": {
                    "tool": tool_name,
                    "args": args,
                    "status": status
                },
                "observation": result
            }
        }));
    }

    /// 发送步骤数据
    fn emit_step(&self, stage: &str, data: serde_json::Value) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            "",
            false,
            Some(stage),
            None,
            Some(ArchitectureType::ReAct),
            Some(data),
        );
    }

    // === 内部方法 ===

    fn emit_meta(&self, stage: &str, data: serde_json::Value) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            "",
            false,
            Some(stage),
            None,
            Some(ArchitectureType::ReAct),
            Some(data),
        );
    }
}

// ============================================================================
// ReactLlmClient - 流式 LLM 调用（每个 token 发送到前端）
// ============================================================================

/// ReAct LLM 客户端
pub struct ReactLlmClient {
    config: LlmConfig,
    emitter: Arc<ReactMessageEmitter>,
}

impl ReactLlmClient {
    pub fn new(config: LlmConfig, emitter: Arc<ReactMessageEmitter>) -> Self {
        Self { config, emitter }
    }

    /// 设置 rig 库所需的环境变量
    fn setup_env_vars(&self) {
        let provider = self.config.provider.to_lowercase();
        
        if let Some(api_key) = &self.config.api_key {
            match provider.as_str() {
                "gemini" | "google" => std::env::set_var("GEMINI_API_KEY", api_key),
                "openai" => std::env::set_var("OPENAI_API_KEY", api_key),
                "anthropic" => std::env::set_var("ANTHROPIC_API_KEY", api_key),
                _ => std::env::set_var("OPENAI_API_KEY", api_key),
            }
        }
        
        if let Some(base_url) = &self.config.base_url {
            match provider.as_str() {
                "gemini" | "google" => std::env::set_var("GEMINI_API_BASE", base_url),
                "anthropic" => std::env::set_var("ANTHROPIC_API_BASE", base_url),
                _ => {
                    std::env::set_var("OPENAI_API_BASE", base_url);
                    std::env::set_var("OPENAI_BASE_URL", base_url);
                    std::env::set_var("OPENAI_BASE", base_url);
                }
            }
            tracing::debug!("ReactLlmClient: Set base URL for '{}': {}", provider, base_url);
        }
    }

    /// 流式调用 LLM，每个 token 通过 emitter 发送
    pub async fn stream_completion(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        iteration: u32,
    ) -> Result<String> {
        let provider = self.config.provider.to_lowercase();
        let model = &self.config.model;

        info!(
            "ReAct LLM stream request - Provider: {}, Model: {}, Iteration: {}",
            provider, model, iteration
        );
        
        // 记录 prompt 到日志
        log_prompts_react("ReactLlmClient", system_prompt, user_prompt);

        // 设置 rig 库所需的环境变量
        self.setup_env_vars();

        // 构建用户消息
        let user_message = Message::User {
            content: OneOrMany::one(UserContent::text(user_prompt.to_string())),
        };

        let preamble = system_prompt.unwrap_or("You are a helpful AI assistant.");
        let timeout = std::time::Duration::from_secs(self.config.timeout_secs);

        // 根据 provider 创建 agent 并执行
        let content = match provider.as_str() {
            "openai" | "lm studio" | "lmstudio" | "lm_studio" => {
                self.stream_with_openai(model, preamble, user_message, timeout).await?
            }
            "anthropic" => {
                self.stream_with_anthropic(model, preamble, user_message, timeout).await?
            }
            "gemini" | "google" => {
                self.stream_with_gemini(model, preamble, user_message, timeout).await?
            }
            "ollama" => {
                self.stream_with_ollama(model, preamble, user_message, timeout).await?
            }
            "deepseek" => {
                self.stream_with_deepseek(model, preamble, user_message, timeout).await?
            }
            "openrouter" => {
                self.stream_with_openrouter(model, preamble, user_message, timeout).await?
            }
            "xai" => {
                self.stream_with_xai(model, preamble, user_message, timeout).await?
            }
            "groq" => {
                self.stream_with_groq(model, preamble, user_message, timeout).await?
            }
            _ => {
                info!("Unknown provider '{}', trying OpenAI compatible mode", provider);
                self.stream_with_openai(model, preamble, user_message, timeout).await?
            }
        };

        info!(
            "ReactLlmClient: Response length: {} chars, Iteration: {}",
            content.len(), iteration
        );
        
        // 记录响应到日志文件
        log_response_react("ReactLlmClient", &content);

        Ok(content)
    }

    async fn stream_with_openai(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::openai;
        let client = openai::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, timeout).await
    }

    async fn stream_with_anthropic(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::anthropic;
        let client = anthropic::Client::from_env();
        let agent = client.agent(model).preamble(preamble).max_tokens(4096).build();
        self.execute_stream(agent, user_message, timeout).await
    }

    async fn stream_with_gemini(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::gemini;
        use rig::providers::gemini::completion::gemini_api_types::{AdditionalParameters, GenerationConfig};
        let client = gemini::Client::from_env();
        let gen_cfg = GenerationConfig::default();
        let cfg = AdditionalParameters::default().with_config(gen_cfg);
        let agent = client.agent(model)
            .preamble(preamble)
            .additional_params(serde_json::to_value(cfg).unwrap())
            .build();
        self.execute_stream(agent, user_message, timeout).await
    }

    async fn stream_with_ollama(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::ollama;
        let client = ollama::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, timeout).await
    }

    async fn stream_with_deepseek(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::deepseek;
        
        // 获取 API Key
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .map_err(|_| anyhow::anyhow!("DEEPSEEK_API_KEY not set"))?;
        
        // 创建带有正确 Content-Type 的 HTTP 客户端（DeepSeek API 要求）
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        
        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;
        
        let client = deepseek::Client::<reqwest::Client>::builder()
            .api_key(api_key)
            .http_client(http_client)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build DeepSeek client: {}", e))?;
        
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, timeout).await
    }

    async fn stream_with_openrouter(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::openrouter;
        let client = openrouter::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, timeout).await
    }

    async fn stream_with_xai(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::xai;
        let client = xai::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, timeout).await
    }

    async fn stream_with_groq(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        timeout: std::time::Duration,
    ) -> Result<String> {
        use rig::providers::groq;
        let client = groq::Client::from_env();
        let agent = client.agent(model).preamble(preamble).build();
        self.execute_stream(agent, user_message, timeout).await
    }

    async fn execute_stream<M>(
        &self,
        agent: rig::agent::Agent<M>,
        user_message: Message,
        timeout: std::time::Duration,
    ) -> Result<String>
    where
        M: rig::completion::CompletionModel + 'static,
        M::StreamingResponse: Clone + Unpin + rig::completion::GetTokenUsage,
    {
        // 流式请求（带超时）
        let stream_result = tokio::time::timeout(
            timeout,
            agent.stream_prompt(user_message).multi_turn(100),
        )
        .await;

        let mut stream_iter = match stream_result {
            Ok(iter) => iter,
            Err(_) => {
                error!(
                    "ReactLlmClient: Request timeout after {} seconds",
                    self.config.timeout_secs
                );
                return Err(anyhow!(
                    "ReAct LLM request timeout after {} seconds",
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
                        // 通过 emitter 发送每个 token
                        self.emitter.emit_content(&piece, false);
                    }
                }
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Reasoning(r),
                )) => {
                    let piece = r.reasoning.join("");
                    if !piece.is_empty() {
                        self.emitter.emit_thinking(&piece);
                    }
                }
                Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                    debug!("ReactLlmClient: Stream completed");
                    break;
                }
                Ok(_) => { /* ignore other stream items */ }
                Err(e) => {
                    error!("ReactLlmClient: Stream error: {}", e);
                    return Err(anyhow!("ReAct LLM stream error: {}", e));
                }
            }
        }

        Ok(content)
    }
}

/// 记录 prompts 到 LLM 日志文件
fn log_prompts_react(client_name: &str, system_prompt: Option<&str>, user_prompt: &str) {
    write_llm_log_react(client_name, "REQUEST", system_prompt, user_prompt, None);
}

/// 记录 LLM 响应到日志文件
fn log_response_react(client_name: &str, response: &str) {
    write_llm_log_react(client_name, "RESPONSE", None, "", Some(response));
}

/// 写入 LLM 日志到文件
fn write_llm_log_react(
    client_name: &str,
    log_type: &str,
    system_prompt: Option<&str>,
    user_prompt: &str,
    response: Option<&str>,
) {
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC");
    
    let content = if let Some(resp) = response {
        // 安全截断，确保不在 UTF-8 字符中间切断
        let truncated = if resp.len() > 2000 {
            let mut end = 2000;
            while end > 0 && !resp.is_char_boundary(end) {
                end -= 1;
            }
            &resp[..end]
        } else {
            resp
        };
        format!(
            "Response ({} chars):\n{}\n",
            resp.len(),
            truncated
        )
    } else {
        format!(
            "\nUser Prompt:\n{}\n",
            // system_prompt.unwrap_or("(none)"),
            user_prompt
        )
    };
    
    let log_entry = format!(
        "\n{}\n[{}] [{}] [Client: {}]\n{}\n{}\n",
        "=".repeat(80), timestamp, log_type, client_name, "=".repeat(80), content
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

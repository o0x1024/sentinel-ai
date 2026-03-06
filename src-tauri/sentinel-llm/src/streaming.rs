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
use serde_json::json;
use std::collections::HashMap;
use tracing::{error, info, warn};

use crate::config::LlmConfig;
use crate::log::{
    build_log_session_id, log_error_response, log_request, log_response, log_stream_event,
    log_turn_summary,
};
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

fn parse_embedded_json_value(value: serde_json::Value, depth: usize) -> serde_json::Value {
    if depth >= 4 {
        return value;
    }

    match value {
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return serde_json::Value::String(s);
            }

            let looks_like_json =
                matches!(trimmed.chars().next(), Some('{') | Some('[') | Some('"'));
            if !looks_like_json {
                return serde_json::Value::String(s);
            }

            match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(parsed) => parse_embedded_json_value(parsed, depth + 1),
                Err(_) => serde_json::Value::String(s),
            }
        }
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .into_iter()
                .map(|item| parse_embedded_json_value(item, depth + 1))
                .collect(),
        ),
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, parse_embedded_json_value(v, depth + 1)))
                .collect(),
        ),
        other => other,
    }
}

fn normalize_jsonish_string(raw: &str) -> serde_json::Value {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(parsed) => parse_embedded_json_value(parsed, 0),
        Err(_) => serde_json::Value::String(raw.to_string()),
    }
}

fn infer_tool_result_success_for_turn(value: &serde_json::Value) -> bool {
    fn has_hard_error(text: &str) -> bool {
        let lower = text.trim().to_lowercase();
        if lower.is_empty() {
            return false;
        }
        if lower.contains("toolset error")
            || lower.contains("tool execution failed")
            || lower.contains("shell execution failed")
            || lower.contains("command timeout after")
            || lower.contains("llm request timeout")
            || lower.contains("traceback (most recent call last)")
            || lower.contains("fatal error:")
        {
            return true;
        }
        (lower.contains("timed out") || lower.contains("timeout after"))
            && (lower.contains("error") || lower.contains("failed"))
    }

    fn visit(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Null => true,
            serde_json::Value::Bool(v) => *v,
            serde_json::Value::Number(n) => n.as_i64().map(|v| v == 0).unwrap_or(true),
            serde_json::Value::String(s) => {
                let lower = s.trim().to_lowercase();
                if has_hard_error(&lower) {
                    return false;
                }
                if lower.starts_with("error:") || lower.starts_with("failed:") {
                    return false;
                }
                !lower.contains(" no such file or directory")
            }
            serde_json::Value::Array(arr) => arr.iter().all(visit),
            serde_json::Value::Object(map) => {
                if let Some(v) = map.get("success").and_then(|v| v.as_bool()) {
                    return v;
                }
                if let Some(v) = map.get("ok").and_then(|v| v.as_bool()) {
                    return v;
                }
                if let Some(v) = map.get("completed").and_then(|v| v.as_bool()) {
                    if !v {
                        return false;
                    }
                }
                if let Some(v) = map.get("exit_code").and_then(|v| v.as_i64()) {
                    return v == 0;
                }
                if let Some(v) = map.get("code").and_then(|v| v.as_i64()) {
                    return v == 0;
                }
                if let Some(v) = map.get("error").and_then(|v| v.as_str()) {
                    if !v.trim().is_empty() {
                        return false;
                    }
                }
                map.values().all(visit)
            }
        }
    }

    visit(value)
}

impl StreamingLlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
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
        F: FnMut(StreamContent) -> bool,
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
        F: FnMut(StreamContent) -> bool,
    {
        let provider = self.config.provider.to_lowercase();
        let provider_for_agent = self.config.get_effective_rig_provider();
        let model = &self.config.model;
        let conversation_id = self.config.conversation_id.as_deref();
        let session_id = build_log_session_id(conversation_id);
        let tool_names: Vec<String> = dynamic_tools.iter().map(|t| t.name().to_string()).collect();
        let tool_count = dynamic_tools.len();
        let history_count = history.len();

        info!(
            "StreamingLlmClient - Provider: {}, Model: {}, Tools: {:?}, History: {} messages",
            provider,
            model,
            tool_names,
            history.len()
        );

        let mut system_prompt_with_hack = system_prompt
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("You are a helpful AI assistant.")
            .to_string();

        // CRITICAL FIX: Moonshot/DeepSeek and other picky providers REQUIRE non-empty assistant messages.
        // We add a system-level instruction to help them comply, and we'll also use placeholders in history.
        let provider_lower = provider_for_agent.to_lowercase();
        if (provider_lower.contains("moonshot")
            || provider_lower.contains("deepseek")
            || provider_lower.contains("kimi"))
            && !system_prompt_with_hack.contains("text response")
        {
            system_prompt_with_hack.push_str("\n\nIMPORTANT: You must always provide a brief text response alongside any tool calls. Do not output empty text messages.");
        }
        let preamble = &system_prompt_with_hack;

        log_request(
            &session_id,
            conversation_id,
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
        let is_bigmodel_compat =
            Self::is_bigmodel_compatible_base_url(self.config.base_url.as_deref());
        let provider_for_stream = provider_for_agent.clone();
        let model_for_stream = model.to_string();
        let conversation_id_for_stream = conversation_id.map(str::to_string);
        let session_id_for_stream = session_id.clone();
        let turn_started_at = chrono::Utc::now();
        let mut assistant_text_full = String::new();
        let mut reasoning_text_full = String::new();
        let mut last_usage: Option<(u32, u32)> = None;
        let mut stream_completed = false;
        let mut tool_calls_by_id: HashMap<String, serde_json::Map<String, serde_json::Value>> =
            HashMap::new();

        let mut emit_content = |chunk: StreamContent| {
            let (event_type, payload) = match &chunk {
                StreamContent::Text(text) => (
                    "assistant_text_delta",
                    json!({
                        "content": text,
                        "content_length": text.len(),
                    }),
                ),
                StreamContent::Reasoning(text) => (
                    "reasoning_delta",
                    json!({
                        "content": text,
                        "content_length": text.len(),
                    }),
                ),
                StreamContent::ToolCallStart { id, name } => (
                    "tool_call_start",
                    json!({
                        "tool_call_id": id,
                        "tool_name": name,
                    }),
                ),
                StreamContent::ToolCallDelta { id, delta } => (
                    "tool_call_delta",
                    json!({
                        "tool_call_id": id,
                        "delta": delta,
                        "delta_length": delta.len(),
                    }),
                ),
                StreamContent::ToolCallComplete {
                    id,
                    name,
                    arguments,
                } => (
                    "tool_call_complete",
                    json!({
                        "tool_call_id": id,
                        "tool_name": name,
                        "arguments": arguments,
                    }),
                ),
                StreamContent::ToolResult { id, result } => (
                    "tool_result",
                    json!({
                        "tool_call_id": id,
                        "result": result,
                    }),
                ),
                StreamContent::Usage {
                    input_tokens,
                    output_tokens,
                } => (
                    "usage",
                    json!({
                        "input_tokens": input_tokens,
                        "output_tokens": output_tokens,
                    }),
                ),
                StreamContent::Done => ("done", json!({})),
            };
            match &chunk {
                StreamContent::Text(text) => assistant_text_full.push_str(text),
                StreamContent::Reasoning(text) => reasoning_text_full.push_str(text),
                StreamContent::ToolCallStart { id, name } => {
                    let entry = tool_calls_by_id.entry(id.clone()).or_default();
                    entry.insert("tool_call_id".to_string(), json!(id));
                    entry.insert("tool_name".to_string(), json!(name));
                    entry.insert("status".to_string(), json!("started"));
                }
                StreamContent::ToolCallDelta { id, delta } => {
                    let entry = tool_calls_by_id.entry(id.clone()).or_default();
                    entry.insert("tool_call_id".to_string(), json!(id));
                    let current = entry
                        .get("arguments_delta")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    entry.insert(
                        "arguments_delta".to_string(),
                        json!(format!("{}{}", current, delta)),
                    );
                    entry.insert("status".to_string(), json!("streaming"));
                }
                StreamContent::ToolCallComplete {
                    id,
                    name,
                    arguments,
                } => {
                    let entry = tool_calls_by_id.entry(id.clone()).or_default();
                    entry.insert("tool_call_id".to_string(), json!(id));
                    entry.insert("tool_name".to_string(), json!(name));
                    entry.insert("arguments".to_string(), normalize_jsonish_string(arguments));
                    entry.insert("arguments_raw".to_string(), json!(arguments));
                    entry.insert("status".to_string(), json!("called"));
                }
                StreamContent::ToolResult { id, result } => {
                    let entry = tool_calls_by_id.entry(id.clone()).or_default();
                    entry.insert("tool_call_id".to_string(), json!(id));
                    let normalized_result = normalize_jsonish_string(result);
                    let success = infer_tool_result_success_for_turn(&normalized_result);
                    entry.insert("result".to_string(), normalized_result);
                    entry.insert("result_raw".to_string(), json!(result));
                    entry.insert("success".to_string(), json!(success));
                    entry.insert("status".to_string(), json!("completed"));
                }
                StreamContent::Usage {
                    input_tokens,
                    output_tokens,
                } => {
                    last_usage = Some((*input_tokens, *output_tokens));
                }
                StreamContent::Done => {
                    stream_completed = true;
                }
            }
            log_stream_event(
                &session_id_for_stream,
                conversation_id_for_stream.as_deref(),
                &provider_for_stream,
                &model_for_stream,
                event_type,
                &payload,
            );
            on_content(chunk)
        };

        // 根据 provider 创建带动态工具的 agent
        let content_result: Result<String> = match provider_for_agent.as_str() {
            "openai" => {
                let tool_count = dynamic_tools.len();
                info!(
                    "OpenAI-compatible call context: base_url={:?}, bigmodel_compat={}, temperature={:?}, max_tokens={:?}, tools={}",
                    self.config.base_url,
                    is_bigmodel_compat,
                    self.config.temperature,
                    self.config.max_tokens,
                    tool_count
                );

                let retry_user_message = user_message.clone();
                let retry_chat_history = chat_history.clone();
                match self
                    .stream_with_openai(
                        model,
                        preamble,
                        user_message,
                        chat_history,
                        timeout,
                        dynamic_tools,
                        &mut emit_content,
                    )
                    .await
                {
                    Ok(content) => Ok(content),
                    Err(e) if is_bigmodel_compat && Self::is_bigmodel_1210_error(&e) => {
                        warn!(
                            "BigModel returned 1210 (parameter error). Retrying once in minimal compatibility mode: no tools, no generation overrides."
                        );
                        self.stream_with_openai_minimal_compat(
                            model,
                            preamble,
                            retry_user_message,
                            retry_chat_history,
                            timeout,
                            &mut emit_content,
                        )
                        .await
                    }
                    Err(e) => Err(e),
                }
            }
            "moonshot" => {
                self.stream_with_moonshot(
                    model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    dynamic_tools,
                    &mut emit_content,
                )
                .await
            }
            "anthropic" => {
                self.stream_with_anthropic(
                    model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    dynamic_tools,
                    &mut emit_content,
                )
                .await
            }
            "gemini" | "google" => {
                self.stream_with_gemini(
                    model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    dynamic_tools,
                    &mut emit_content,
                )
                .await
            }
            "deepseek" => {
                self.stream_with_deepseek(
                    model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    dynamic_tools,
                    &mut emit_content,
                )
                .await
            }
            "ollama" => {
                self.stream_with_ollama(
                    model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    dynamic_tools,
                    &mut emit_content,
                )
                .await
            }
            "openrouter" => {
                self.stream_with_openrouter(
                    model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    dynamic_tools,
                    &mut emit_content,
                )
                .await
            }
            "xai" => {
                self.stream_with_xai(
                    model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    dynamic_tools,
                    &mut emit_content,
                )
                .await
            }
            "groq" => {
                self.stream_with_groq(
                    model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    dynamic_tools,
                    &mut emit_content,
                )
                .await
            }
            _ => {
                info!(
                    "Unknown provider '{}', trying OpenAI compatible mode (via Generic Client)",
                    provider_for_agent
                );
                self.stream_with_generic_openai(
                    model,
                    preamble,
                    user_message,
                    chat_history,
                    timeout,
                    dynamic_tools,
                    &mut emit_content,
                )
                .await
            }
        };
        let content = match content_result {
            Ok(content) => content,
            Err(err) => {
                let tool_calls: Vec<serde_json::Value> = tool_calls_by_id
                    .into_values()
                    .map(serde_json::Value::Object)
                    .collect();
                let (input_tokens, output_tokens) = last_usage.unwrap_or((0, 0));
                log_turn_summary(
                    &session_id,
                    conversation_id,
                    &provider_for_agent,
                    model,
                    &json!({
                        "status": "error",
                        "started_at": turn_started_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                        "completed_at": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                        "duration_ms": (chrono::Utc::now() - turn_started_at).num_milliseconds(),
                        "user_request": user_prompt,
                        "system_prompt": preamble,
                        "assistant_response": assistant_text_full,
                        "reasoning": reasoning_text_full,
                        "tool_calls": tool_calls,
                        "usage": {
                            "input_tokens": input_tokens,
                            "output_tokens": output_tokens,
                        },
                        "stream_completed": stream_completed,
                        "error": err.to_string(),
                    }),
                );
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
                "LLM stream returned empty response (provider={}, model={})",
                provider_for_agent,
                model
            );
            let tool_calls: Vec<serde_json::Value> = tool_calls_by_id
                .into_values()
                .map(serde_json::Value::Object)
                .collect();
            let (input_tokens, output_tokens) = last_usage.unwrap_or((0, 0));
            log_turn_summary(
                &session_id,
                conversation_id,
                &provider_for_agent,
                model,
                &json!({
                    "status": "empty_response",
                    "started_at": turn_started_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    "completed_at": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    "duration_ms": (chrono::Utc::now() - turn_started_at).num_milliseconds(),
                    "user_request": user_prompt,
                    "system_prompt": preamble,
                    "assistant_response": assistant_text_full,
                    "reasoning": reasoning_text_full,
                    "tool_calls": tool_calls,
                    "usage": {
                        "input_tokens": input_tokens,
                        "output_tokens": output_tokens,
                    },
                    "stream_completed": stream_completed,
                    "error": err.to_string(),
                }),
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

        log_response(&session_id, conversation_id, &provider, model, &content);
        let tool_calls: Vec<serde_json::Value> = tool_calls_by_id
            .into_values()
            .map(serde_json::Value::Object)
            .collect();
        let (input_tokens, output_tokens) = last_usage.unwrap_or((0, 0));
        log_turn_summary(
            &session_id,
            conversation_id,
            &provider_for_agent,
            model,
            &json!({
                "status": if stream_completed { "completed" } else { "interrupted" },
                "started_at": turn_started_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                "completed_at": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                "duration_ms": (chrono::Utc::now() - turn_started_at).num_milliseconds(),
                "user_request": user_prompt,
                "system_prompt": preamble,
                "assistant_response": content.clone(),
                "reasoning": reasoning_text_full,
                "tool_calls": tool_calls,
                "usage": {
                    "input_tokens": input_tokens,
                    "output_tokens": output_tokens,
                },
                "stream_completed": stream_completed,
                "tool_count": tool_count,
                "history_count": history_count,
            }),
        );
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
        F: FnMut(StreamContent) -> bool,
    {
        use rig::providers::deepseek;

        let api_key = self.config.api_key.clone().unwrap_or_default();

        let mut builder =
            deepseek::Client::<rig::http_client::ReqwestClient>::builder().api_key(api_key);

        if let Some(base_url) = &self.config.base_url {
            builder = builder.base_url(base_url);
        }

        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build generic client: {}", e))?;

        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let builder = client.agent(model).preamble(preamble);
        let agent = self
            .apply_generation_settings(builder)
            .tool_server_handle(tool_server_handle)
            .build();

        self.execute_stream(agent, user_message, chat_history, timeout, on_content)
            .await
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
        F: FnMut(StreamContent) -> bool,
    {
        use rig::providers::openai;

        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;

        let tool_server_handle = Self::build_tool_server(dynamic_tools);

        // If custom base_url is set, use Chat Completions API (for third-party providers)
        // Otherwise use Responses API (for official OpenAI)
        if let Some(base_url) = &self.config.base_url {
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

            let builder = client.agent(model).preamble(preamble);
            let agent = self
                .apply_generation_settings(builder)
                .tool_server_handle(tool_server_handle)
                .build();
            self.execute_stream(agent, user_message, chat_history, timeout, on_content)
                .await
        } else {
            info!("Using Responses API for official OpenAI");
            let client: openai::Client = openai::Client::builder()
                .api_key(api_key)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build OpenAI client: {:?}", e))?;

            let builder = client.agent(model).preamble(preamble);
            let agent = self
                .apply_generation_settings(builder)
                .tool_server_handle(tool_server_handle)
                .build();
            self.execute_stream(agent, user_message, chat_history, timeout, on_content)
                .await
        }
    }

    async fn stream_with_openai_minimal_compat<F>(
        &self,
        model: &str,
        preamble: &str,
        user_message: Message,
        chat_history: Vec<Message>,
        timeout: std::time::Duration,
        on_content: &mut F,
    ) -> Result<String>
    where
        F: FnMut(StreamContent) -> bool,
    {
        use rig::providers::openai;

        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
        let base_url = self
            .config
            .base_url
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("base_url is required for minimal compat mode"))?;

        let client: openai::CompletionsClient = openai::Client::builder()
            .api_key(api_key)
            .base_url(base_url)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build OpenAI client: {:?}", e))?
            .completions_api();

        let agent = client.agent(model).preamble(preamble).build();

        self.execute_stream(agent, user_message, chat_history, timeout, on_content)
            .await
    }

    async fn stream_with_moonshot<F>(
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
        F: FnMut(StreamContent) -> bool,
    {
        use rig::providers::moonshot;

        let api_key = self
            .config
            .api_key
            .clone()
            .or_else(|| std::env::var("MOONSHOT_API_KEY").ok())
            .ok_or_else(|| anyhow::anyhow!("MOONSHOT_API_KEY not set"))?;

        let mut builder =
            moonshot::Client::<rig::http_client::ReqwestClient>::builder().api_key(api_key);

        if let Some(base_url) = &self.config.base_url {
            builder = builder.base_url(base_url);
        }

        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Moonshot client: {:?}", e))?;

        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let mut builder = client.agent(model).preamble(preamble);
        if let Some(params) = self.moonshot_thinking_params(model) {
            builder = builder.additional_params(params);
        }
        let agent = self
            .apply_generation_settings(builder)
            .tool_server_handle(tool_server_handle)
            .build();

        self.execute_stream(agent, user_message, chat_history, timeout, on_content)
            .await
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
        F: FnMut(StreamContent) -> bool,
    {
        use rig::providers::anthropic;

        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

        let mut builder =
            anthropic::Client::<rig::http_client::ReqwestClient>::builder().api_key(api_key);

        if let Ok(base_url) = std::env::var("ANTHROPIC_API_BASE") {
            if !base_url.is_empty() {
                builder = builder.base_url(&base_url);
            }
        }

        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build Anthropic client: {:?}", e))?;

        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let builder = client
            .agent(model)
            .preamble(preamble)
            .max_tokens(self.config.get_max_tokens() as u64);
        let agent = self
            .apply_generation_settings(builder)
            .tool_server_handle(tool_server_handle)
            .build();

        self.execute_stream(agent, user_message, chat_history, timeout, on_content)
            .await
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
        F: FnMut(StreamContent) -> bool,
    {
        use rig::providers::gemini;
        let client = gemini::Client::from_env();
        let gen_cfg = GenerationConfig::default();
        let cfg = AdditionalParameters::default().with_config(gen_cfg);

        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let builder = client
            .agent(model)
            .preamble(preamble)
            .additional_params(serde_json::to_value(cfg).unwrap());
        let agent = self
            .apply_generation_settings(builder)
            .tool_server_handle(tool_server_handle)
            .build();

        self.execute_stream(agent, user_message, chat_history, timeout, on_content)
            .await
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
        F: FnMut(StreamContent) -> bool,
    {
        use rig::providers::deepseek;

        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .map_err(|_| anyhow::anyhow!("DEEPSEEK_API_KEY not set"))?;

        let mut builder =
            deepseek::Client::<rig::http_client::ReqwestClient>::builder().api_key(api_key);

        if let Some(base_url) = &self.config.base_url {
            builder = builder.base_url(base_url);
        }

        let client = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build DeepSeek client: {}", e))?;

        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let builder = client.agent(model).preamble(preamble);
        let agent = self
            .apply_generation_settings(builder)
            .tool_server_handle(tool_server_handle)
            .build();

        self.execute_stream(agent, user_message, chat_history, timeout, on_content)
            .await
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
        F: FnMut(StreamContent) -> bool,
    {
        use rig::providers::ollama;
        let client = ollama::Client::from_env();

        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let builder = client.agent(model).preamble(preamble);
        let agent = self
            .apply_generation_settings(builder)
            .tool_server_handle(tool_server_handle)
            .build();

        self.execute_stream(agent, user_message, chat_history, timeout, on_content)
            .await
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
        F: FnMut(StreamContent) -> bool,
    {
        use rig::providers::openrouter;
        let client = openrouter::Client::from_env();

        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let builder = client.agent(model).preamble(preamble);
        let agent = self
            .apply_generation_settings(builder)
            .tool_server_handle(tool_server_handle)
            .build();

        self.execute_stream(agent, user_message, chat_history, timeout, on_content)
            .await
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
        F: FnMut(StreamContent) -> bool,
    {
        use rig::providers::xai;
        let client = xai::Client::from_env();

        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let builder = client.agent(model).preamble(preamble);
        let agent = self
            .apply_generation_settings(builder)
            .tool_server_handle(tool_server_handle)
            .build();

        self.execute_stream(agent, user_message, chat_history, timeout, on_content)
            .await
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
        F: FnMut(StreamContent) -> bool,
    {
        use rig::providers::groq;
        let client = groq::Client::from_env();

        let tool_server_handle = Self::build_tool_server(dynamic_tools);
        let builder = client.agent(model).preamble(preamble);
        let agent = self
            .apply_generation_settings(builder)
            .tool_server_handle(tool_server_handle)
            .build();

        self.execute_stream(agent, user_message, chat_history, timeout, on_content)
            .await
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
        F: FnMut(StreamContent) -> bool,
    {
        let mut tool_call_args: HashMap<String, String> = HashMap::new();
        let mut tool_call_names: HashMap<String, String> = HashMap::new();
        info!("Starting stream iteration...");

        self.validate_moonshot_temperature()?;
        let max_turns = self.config.get_max_turns();
        info!("Using max_turns: {}", max_turns);

        let chat_history = chat_history;

        tracing::info!("Final chat_history has {} messages", chat_history.len());

        let stream_result = if chat_history.is_empty() {
            info!("Using stream_prompt for empty chat history");
            tokio::time::timeout(
                timeout,
                agent.stream_prompt(user_message).multi_turn(max_turns),
            )
            .await
        } else {
            info!(
                "Using stream_chat with {} history messages",
                chat_history.len()
            );
            tokio::time::timeout(
                timeout,
                agent
                    .stream_chat(user_message, chat_history)
                    .multi_turn(max_turns),
            )
            .await
        };

        let mut stream_iter = match stream_result {
            Ok(iter) => iter,
            Err(_) => {
                error!(
                    "LLM request timeout after {} seconds",
                    self.config.timeout_secs
                );
                return Err(anyhow!(
                    "LLM request timeout after {} seconds",
                    self.config.timeout_secs
                ));
            }
        };

        let mut content = String::new();
        let mut chunk_count = 0;

        loop {
            let item = match stream_iter.next().await {
                Some(item) => item,
                None => break,
            };

            chunk_count += 1;
            match item {
                // 文本内容
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t))) => {
                    let piece = t.text;
                    if !piece.is_empty() {
                        content.push_str(&piece);
                        if !on_content(StreamContent::Text(piece)) {
                            info!("Stream cancelled by callback");
                            break;
                        }
                    }
                }
                // 推理内容
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Reasoning(r),
                )) => {
                    let piece = r.display_text();
                    if !piece.is_empty() && !on_content(StreamContent::Reasoning(piece)) {
                        info!("Stream cancelled by callback");
                        break;
                    }
                }
                // 完整的工具调用
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::ToolCall { tool_call, .. },
                )) => {
                    // info!(
                    //     "Tool call received: id={}, name={}, args={}",
                    //     tool_call.id, tool_call.function.name, tool_call.function.arguments
                    // );
                    if !on_content(StreamContent::ToolCallComplete {
                        id: tool_call.id.clone(),
                        name: tool_call.function.name.clone(),
                        arguments: tool_call.function.arguments.to_string(),
                    }) {
                        info!("Stream cancelled by callback");
                        break;
                    }
                    tool_call_names.insert(tool_call.id.clone(), tool_call.function.name.clone());
                }
                // 工具调用增量
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::ToolCallDelta { id, content, .. },
                )) => {
                    use rig::streaming::ToolCallDeltaContent;
                    let delta_str = match &content {
                        ToolCallDeltaContent::Name(n) => n.clone(),
                        ToolCallDeltaContent::Delta(d) => d.clone(),
                    };
                    tool_call_args
                        .entry(id.clone())
                        .or_default()
                        .push_str(&delta_str);
                    if !on_content(StreamContent::ToolCallDelta {
                        id,
                        delta: delta_str,
                    }) {
                        info!("Stream cancelled by callback");
                        break;
                    }
                }
                // 工具执行结果
                Ok(MultiTurnStreamItem::StreamUserItem(user_content)) => {
                    let rig::streaming::StreamedUserContent::ToolResult { tool_result, .. } =
                        user_content;
                    let result_str =
                        serde_json::to_string(&tool_result.content).unwrap_or_default();
                    // info!(
                    //     "Tool result received: id={}, result_len={}, content_preview={}",
                    //     tool_result.id,
                    //     result_str.len(),
                    //     &result_str.chars().take(300).collect::<String>()
                    // );
                    if !on_content(StreamContent::ToolResult {
                        id: tool_result.id,
                        result: result_str,
                    }) {
                        info!("Stream cancelled by callback");
                        break;
                    }
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
                    let _ = on_content(StreamContent::Usage {
                        input_tokens: usage.input_tokens as u32,
                        output_tokens: usage.output_tokens as u32,
                    });

                    let _ = on_content(StreamContent::Done);
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

    fn is_bigmodel_compatible_base_url(base_url: Option<&str>) -> bool {
        base_url
            .map(|u| {
                let u = u.to_lowercase();
                u.contains("open.bigmodel.cn") || u.contains("bigmodel.cn/api/paas")
            })
            .unwrap_or(false)
    }

    fn is_bigmodel_1210_error(err: &anyhow::Error) -> bool {
        let msg = err.to_string();
        msg.contains("\"code\":\"1210\"")
            || msg.contains("\"code\": \"1210\"")
            || (msg.contains("1210") && msg.contains("API 调用参数有误"))
    }
}

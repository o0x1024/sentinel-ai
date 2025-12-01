//! Travel 消息发送器和专用 LLM 客户端
//!
//! 专门用于 Travel 架构的流式消息发送
//! 发送 ooda_step 格式以与前端 TravelMessageProcessor 兼容

use crate::engines::llm_client::LlmConfig;
use crate::utils::ordered_message::{emit_message_chunk_with_arch, ArchitectureType, ChunkType};
use anyhow::{anyhow, Result};
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::client::builder::DynClientBuilder;
use rig::completion::Message;
use rig::message::UserContent;
use rig::one_or_many::OneOrMany;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use chrono::Utc;
use tracing::{debug, error, info};

/// Travel 消息发送器
pub struct TravelMessageEmitter {
    app_handle: Arc<AppHandle>,
    execution_id: String,
    message_id: String,
    conversation_id: Option<String>,
    /// 收集所有发送的内容，用于保存到数据库
    content_collector: Arc<Mutex<String>>,
    /// 当前循环号（用于 OODA step 消息）
    current_cycle: Arc<Mutex<u32>>,
}

/// 执行统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TravelExecutionStats {
    pub total_iterations: u32,
    pub tool_calls_count: u32,
    pub successful_tool_calls: u32,
    pub failed_tool_calls: u32,
    pub total_duration_ms: u64,
    pub status: String,
}

/// OODA 步骤（与前端 TravelMessageProcessor 对齐）
#[derive(Debug, Clone, Serialize)]
struct OodaStep {
    cycle: u32,
    phase: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    thought: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<OodaAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// OODA 工具调用
#[derive(Debug, Clone, Serialize)]
struct OodaAction {
    tool: String,
    args: serde_json::Value,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
}

impl TravelMessageEmitter {
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
            current_cycle: Arc::new(Mutex::new(1)),
        }
    }

    /// 获取收集的完整内容（用于保存到数据库）
    pub fn get_full_content(&self) -> String {
        self.content_collector.lock().unwrap().clone()
    }

    /// 获取当前循环号
    fn get_cycle(&self) -> u32 {
        *self.current_cycle.lock().unwrap()
    }

    /// 设置当前循环号
    fn set_cycle(&self, cycle: u32) {
        *self.current_cycle.lock().unwrap() = cycle;
    }

    /// 发送执行开始信号
    pub fn emit_start(&self, iteration: u32) {
        self.set_cycle(iteration.max(1));
        self.emit_meta("start", serde_json::json!({
            "type": "start",
            "iteration": iteration
        }));
    }

    /// 发送执行完成信号
    pub fn emit_complete(&self, stats: TravelExecutionStats) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::StreamComplete,
            "",
            true, // is_final
            Some("complete"),
            None,
            Some(ArchitectureType::Travel),
            Some(serde_json::json!({
                "type": "complete",
                "statistics": stats
            })),
        );
    }

    /// 发送 Thought (思考) 内容 - 使用 ooda_step 格式
    pub fn emit_thought(&self, content: &str, iteration: u32) {
        // 更新循环号
        self.set_cycle(iteration.max(1));

        // 收集内容 (JSON 格式)
        if let Ok(mut collector) = self.content_collector.lock() {
            let entry = serde_json::json!({"thought": content, "iteration": iteration});
            collector.push_str(&format!("{}\n", entry));
        }

        // 发送流式内容
        let formatted = format!("\n**Thought (Iteration {})**\n{}\n", iteration, content);
        self.emit_content(&formatted, false);

        // 发送 ooda_step 格式的结构化数据（与前端 TravelMessageProcessor 兼容）
        let step = OodaStep {
            cycle: self.get_cycle(),
            phase: "Act".to_string(),  // ReAct 在 OODA 的 Act 阶段执行
            status: "running".to_string(),
            thought: Some(content.to_string()),
            action: None,
            output: None,
            error: None,
        };
        self.emit_ooda_step(&step);
    }

    /// 发送工具调用信息 - 使用 ooda_step 格式
    pub fn emit_tool_call(&self, iteration: u32, tool_name: &str, args: &serde_json::Value) {
        self.set_cycle(iteration.max(1));
        let args_str = serde_json::to_string_pretty(args).unwrap_or_default();

        // 收集内容 (JSON 格式)
        if let Ok(mut collector) = self.content_collector.lock() {
            let entry = serde_json::json!({
                "action": {
                    "name": tool_name,
                    "input": args
                },
                "iteration": iteration
            });
            collector.push_str(&format!("{}\n", entry));
        }

        // 发送 markdown 格式内容
        let content = format!(
            "\n---\n**Action: `{}`**\n<details>\n<summary>Parameters</summary>\n\n```json\n{}\n```\n</details>\n",
            tool_name, args_str
        );
        self.emit_content(&content, false);

        // 发送 ooda_step 格式的结构化数据
        let step = OodaStep {
            cycle: self.get_cycle(),
            phase: "Act".to_string(),
            status: "running".to_string(),
            thought: None,
            action: Some(OodaAction {
                tool: tool_name.to_string(),
                args: args.clone(),
                status: "running".to_string(),
                result: None,
            }),
            output: None,
            error: None,
        };
        self.emit_ooda_step(&step);
    }

    /// 发送工具执行结果 - 使用 ooda_step 格式
    pub fn emit_tool_result(&self, iteration: u32, tool_name: &str, result: &serde_json::Value, success: bool, duration_ms: u64) {
        self.set_cycle(iteration.max(1));
        let result_str = serde_json::to_string_pretty(result).unwrap_or_default();

        // 收集内容 (JSON 格式)
        if let Ok(mut collector) = self.content_collector.lock() {
            let entry = serde_json::json!({
                "observation": result,
                "tool": tool_name,
                "success": success,
                "duration_ms": duration_ms
            });
            collector.push_str(&format!("{}\n", entry));
        }

        // 截断过长的结果用于显示（安全处理 UTF-8 边界）
        let display_result = if result_str.len() > 500 {
            let mut end = 500;
            while end > 0 && !result_str.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...(truncated)", &result_str[..end])
        } else {
            result_str.clone()
        };

        // 发送 markdown 格式内容
        let content = format!(
            "<details>\n<summary>**Observation** ({}ms)</summary>\n\n```json\n{}\n```\n</details>\n---\n\n",
            duration_ms, display_result
        );
        self.emit_content(&content, false);

        // 发送 ooda_step 格式的结构化数据
        let status = if success { "completed" } else { "failed" };
        let step = OodaStep {
            cycle: self.get_cycle(),
            phase: "Act".to_string(),
            status: "running".to_string(),
            thought: None,
            action: Some(OodaAction {
                tool: tool_name.to_string(),
                args: serde_json::json!({}),  // args 在 tool_call 阶段已发送
                status: status.to_string(),
                result: Some(result.clone()),
            }),
            output: None,
            error: if !success { Some(format!("Tool execution failed")) } else { None },
        };
        self.emit_ooda_step(&step);
    }

    /// 发送 Final Answer - 使用 ooda_step 格式
    pub fn emit_final_answer(&self, content: &str, iteration: u32) {
        self.set_cycle(iteration.max(1));

        // 收集内容
        if let Ok(mut collector) = self.content_collector.lock() {
            collector.push_str(&format!("\nFinal Answer: {}\n", content));
        }

        let formatted = format!("\n**Final Answer**\n{}\n", content);
        self.emit_content(&formatted, false);

        // 发送 ooda_step 格式的结构化数据，标记阶段完成
        let step = OodaStep {
            cycle: self.get_cycle(),
            phase: "Act".to_string(),
            status: "completed".to_string(),
            thought: Some(format!("Final Answer: {}", content)),
            action: None,
            output: Some(serde_json::json!({ "final_answer": content })),
            error: None,
        };
        self.emit_ooda_step(&step);
    }

    /// 发送流式内容 chunk
    pub fn emit_content(&self, content: &str, is_final: bool) {
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
            Some(ArchitectureType::Travel),
            None,
        );
    }

    /// 发送思考内容（LLM reasoning）
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
            Some(ArchitectureType::Travel),
            None,
        );
    }

    /// 发送错误信息
    pub fn emit_error(&self, error: &str) {
        let content = format!("\n**Error**: {}\n", error);
        self.emit_content(&content, false);

        // 发送 ooda_step 格式的错误
        let step = OodaStep {
            cycle: self.get_cycle(),
            phase: "Act".to_string(),
            status: "failed".to_string(),
            thought: None,
            action: None,
            output: None,
            error: Some(error.to_string()),
        };
        self.emit_ooda_step(&step);
    }

    /// 发送 OODA 步骤数据（与前端 TravelMessageProcessor 兼容的格式）
    fn emit_ooda_step(&self, step: &OodaStep) {
        let meta_data = serde_json::json!({
            "type": "ooda_step",
            "step": step,
        });

        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            "",
            false,
            Some(&step.phase),
            None,
            Some(ArchitectureType::Travel),
            Some(meta_data),
        );

        log::debug!(
            "Travel ooda_step emitted: cycle={}, phase={}, status={}",
            step.cycle, step.phase, step.status
        );
    }

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
            Some(ArchitectureType::Travel),
            Some(data),
        );
    }
}

// ============================================================================
// TravelLlmClient - 流式 LLM 调用（带 ReAct 解析和前端消息发送）
// ============================================================================

/// Travel LLM 客户端（带 ReAct 解析）
pub struct TravelLlmClient {
    config: LlmConfig,
    emitter: Arc<TravelMessageEmitter>,
}

impl TravelLlmClient {
    pub fn new(config: LlmConfig, emitter: Arc<TravelMessageEmitter>) -> Self {
        Self { config, emitter }
    }

    /// 流式调用 LLM，解析 Thought/Action/Observation 并发送到前端
    pub async fn stream_completion(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        iteration: u32,
    ) -> Result<String> {
        let provider = &self.config.provider;
        let model = &self.config.model;

        info!(
            "Travel LLM stream request - Provider: {}, Model: {}, Iteration: {}",
            provider, model, iteration
        );
        
        // 记录 prompt 到日志
        log_prompts_travel("TravelLlmClient", system_prompt, user_prompt);

        // 创建 agent
        let agent = {
            let client = DynClientBuilder::new();
            let agent_builder = match client.agent(provider, model) {
                Ok(builder) => builder,
                Err(e) => {
                    error!(
                        "TravelLlmClient: Failed to create agent for provider '{}' model '{}': {}",
                        provider, model, e
                    );
                    return Err(anyhow!(
                        "Travel LLM client unavailable: Provider '{}' model '{}' error: {}",
                        provider, model, e
                    ));
                }
            };
            let preamble = system_prompt.unwrap_or("You are a helpful AI assistant.");
            agent_builder.preamble(preamble).build()
        };

        // 构建用户消息
        let user_message = Message::User {
            content: OneOrMany::one(UserContent::text(user_prompt.to_string())),
        };

        // 流式请求（带超时）
        let stream_result = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
            agent.stream_prompt(user_message).multi_turn(100),
        )
        .await;

        let mut stream_iter = match stream_result {
            Ok(iter) => iter,
            Err(_) => {
                error!(
                    "TravelLlmClient: Request timeout after {} seconds",
                    self.config.timeout_secs
                );
                return Err(anyhow!(
                    "Travel LLM request timeout after {} seconds",
                    self.config.timeout_secs
                ));
            }
        };

        // 处理流式响应，实时解析并发送
        let mut content = String::new();
        let mut current_line = String::new();
        let mut parser = ReActParser::new(iteration, self.emitter.clone());

        while let Some(item) = stream_iter.next().await {
            match item {
                Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(t))) => {
                    let piece = t.text;
                    if piece.is_empty() {
                        continue;
                    }
                    content.push_str(&piece);
                    current_line.push_str(&piece);

                    // 检查是否有完整行可以解析
                    while let Some(pos) = current_line.find('\n') {
                        let line = current_line[..pos].to_string();
                        current_line = current_line[pos + 1..].to_string();
                        parser.process_line(&line);
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
                    debug!("TravelLlmClient: Stream completed");
                    break;
                }
                Ok(_) => { /* ignore other stream items */ }
                Err(e) => {
                    error!("TravelLlmClient: Stream error: {}", e);
                    return Err(anyhow!("Travel LLM stream error: {}", e));
                }
            }
        }

        // 处理最后一行（可能没有换行符）
        if !current_line.trim().is_empty() {
            parser.process_line(&current_line);
        }

        // 完成解析
        parser.finalize();

        info!(
            "TravelLlmClient: Response length: {} chars, Iteration: {}",
            content.len(), iteration
        );
        
        // 记录响应到日志文件
        log_response_travel("TravelLlmClient", &content);

        Ok(content)
    }
}

// ============================================================================
// ReAct 格式解析器（Travel 特定）
// ============================================================================

/// ReAct 格式解析器
struct ReActParser {
    iteration: u32,
    emitter: Arc<TravelMessageEmitter>,
    state: ParserState,
    thought_buffer: String,
    action_name: Option<String>,
    action_input_buffer: String,
    final_answer_buffer: String,
}

#[derive(Debug, Clone, PartialEq)]
enum ParserState {
    Initial,
    InJson,      // 解析 JSON 格式
    InThought,   // 兼容旧文本格式
    InAction,
    InActionInput,
    InFinalAnswer,
}

impl ReActParser {
    fn new(iteration: u32, emitter: Arc<TravelMessageEmitter>) -> Self {
        Self {
            iteration,
            emitter,
            state: ParserState::Initial,
            thought_buffer: String::new(),
            action_name: None,
            action_input_buffer: String::new(),
            final_answer_buffer: String::new(),
        }
    }

    fn process_line(&mut self, line: &str) {
        let trimmed = line.trim();
        
        // 跳过空行和 markdown 代码块标记
        if trimmed.is_empty() || trimmed == "```json" || trimmed == "```" {
            return;
        }

        // 优先尝试 JSON 格式解析
        if trimmed.starts_with('{') || self.state == ParserState::InJson {
            self.process_json_line(trimmed);
            return;
        }

        // 兼容旧的文本格式
        if trimmed.starts_with("Thought:") {
            self.flush_current();
            self.state = ParserState::InThought;
            let content = trimmed.strip_prefix("Thought:").unwrap_or("").trim();
            self.thought_buffer = content.to_string();
        } else if trimmed.starts_with("Action:") {
            self.flush_current();
            self.state = ParserState::InAction;
            let name = trimmed.strip_prefix("Action:").unwrap_or("").trim();
            self.action_name = Some(name.to_string());
        } else if trimmed.starts_with("Action Input:") {
            self.state = ParserState::InActionInput;
            let input = trimmed.strip_prefix("Action Input:").unwrap_or("").trim();
            self.action_input_buffer = input.to_string();
        } else if trimmed.starts_with("Final Answer:") {
            self.flush_current();
            self.state = ParserState::InFinalAnswer;
            let content = trimmed.strip_prefix("Final Answer:").unwrap_or("").trim();
            self.final_answer_buffer = content.to_string();
        } else {
            // 继续当前状态的内容
            match self.state {
                ParserState::InThought => {
                    if !self.thought_buffer.is_empty() {
                        self.thought_buffer.push(' ');
                    }
                    self.thought_buffer.push_str(trimmed);
                }
                ParserState::InActionInput => {
                    self.action_input_buffer.push_str(trimmed);
                }
                ParserState::InFinalAnswer => {
                    if !self.final_answer_buffer.is_empty() {
                        self.final_answer_buffer.push(' ');
                    }
                    self.final_answer_buffer.push_str(trimmed);
                }
                _ => {}
            }
        }
    }
    
    /// 处理 JSON 格式行
    fn process_json_line(&mut self, line: &str) {
        // 累积 JSON 内容
        if self.state != ParserState::InJson {
            self.state = ParserState::InJson;
            self.thought_buffer.clear();
        }
        self.thought_buffer.push_str(line);
        
        // 尝试解析完整的 JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&self.thought_buffer) {
            self.process_json_object(&json);
            self.thought_buffer.clear();
            self.state = ParserState::Initial;
        }
    }
    
    /// 处理解析后的 JSON 对象
    fn process_json_object(&mut self, json: &serde_json::Value) {
        // 提取 thought
        if let Some(thought) = json.get("thought").and_then(|v| v.as_str()) {
            self.emitter.emit_thought(thought, self.iteration);
        }
        
        // 检查 action
        if let Some(action) = json.get("action") {
            let tool_name = action.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let args = action.get("input")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            
            if !tool_name.is_empty() {
                self.emitter.emit_tool_call(self.iteration, tool_name, &args);
            }
        }
        
        // 检查 final_answer
        if let Some(answer) = json.get("final_answer").and_then(|v| v.as_str()) {
            self.final_answer_buffer = answer.to_string();
        }
    }

    fn flush_current(&mut self) {
        match self.state {
            ParserState::InThought => {
                if !self.thought_buffer.is_empty() {
                    self.emitter.emit_thought(&self.thought_buffer, self.iteration);
                    self.thought_buffer.clear();
                }
            }
            ParserState::InActionInput => {
                if let Some(name) = &self.action_name {
                    let args = serde_json::from_str(&self.action_input_buffer)
                        .unwrap_or(serde_json::json!({"raw": self.action_input_buffer}));
                    self.emitter.emit_tool_call(self.iteration, name, &args);
                }
                self.action_name = None;
                self.action_input_buffer.clear();
            }
            ParserState::InFinalAnswer => {
                if !self.final_answer_buffer.is_empty() {
                    self.emitter.emit_final_answer(&self.final_answer_buffer, self.iteration);
                    self.final_answer_buffer.clear();
                }
            }
            _ => {}
        }
    }

    fn finalize(&mut self) {
        self.flush_current();
    }
}

/// 记录 prompts 到 LLM 日志文件
fn log_prompts_travel(client_name: &str, system_prompt: Option<&str>, user_prompt: &str) {
    write_llm_log_travel(client_name, "REQUEST", system_prompt, user_prompt, None);
}

/// 记录 LLM 响应到日志文件
fn log_response_travel(client_name: &str, response: &str) {
    write_llm_log_travel(client_name, "RESPONSE", None, "", Some(response));
}

/// 写入 LLM 日志到文件
fn write_llm_log_travel(
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
        // format!(
        //     "System Prompt:\n{}\n\nUser Prompt:\n{}\n",
        //     system_prompt.unwrap_or("(none)"),
        //     user_prompt
        // )
        format!(
            "User Prompt:\n{}\n",
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


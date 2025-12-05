//! Travel 消息发送器和专用 LLM 客户端
//!
//! 专门用于 Travel 架构的流式消息发送
//! 发送 ooda_step 格式以与前端 TravelMessageProcessor 兼容

use crate::engines::LlmConfig;
use crate::utils::ordered_message::{emit_message_chunk_with_arch, ArchitectureType, ChunkType};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sentinel_llm::{StreamingLlmClient, StreamContent};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tracing::{debug, error, info};

/// Travel 消息发送器
pub struct TravelMessageEmitter {
    app_handle: Arc<AppHandle>,
    execution_id: String,
    message_id: String,
    conversation_id: Option<String>,
    /// 收集所有发送的内容，用于保存到数据库
    content_collector: Arc<Mutex<String>>,
    /// 当前OODA循环号（在整个ReAct执行期间保持不变）
    current_cycle: Arc<Mutex<u32>>,
    /// 当前ReAct迭代号（每次工具调用递增）
    current_iteration: Arc<Mutex<u32>>,
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
    /// ReAct 迭代号（仅在 Act 阶段的 ReAct 执行中使用）
    #[serde(skip_serializing_if = "Option::is_none")]
    react_iteration: Option<u32>,
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
            current_iteration: Arc::new(Mutex::new(0)),
        }
    }

    /// 获取收集的完整内容（用于保存到数据库）
    pub fn get_full_content(&self) -> String {
        self.content_collector.lock().unwrap().clone()
    }

    /// 获取当前OODA循环号
    fn get_cycle(&self) -> u32 {
        *self.current_cycle.lock().unwrap()
    }

    /// 设置当前OODA循环号（整个ReAct执行期间保持不变）
    pub fn set_ooda_cycle(&self, cycle: u32) {
        *self.current_cycle.lock().unwrap() = cycle;
    }

    /// 设置当前循环号（已弃用，请使用 set_ooda_cycle）
    fn set_cycle(&self, _cycle: u32) {
        // 不再由 iteration 设置 cycle，保持 OODA 循环号不变
    }

    /// 获取当前ReAct迭代号
    fn get_iteration(&self) -> u32 {
        *self.current_iteration.lock().unwrap()
    }

    /// 设置当前ReAct迭代号
    fn set_iteration(&self, iteration: u32) {
        *self.current_iteration.lock().unwrap() = iteration;
    }

    /// 发送执行开始信号
    /// iteration 参数实际上是 OODA 循环号，在 ReAct 开始时设置
    pub fn emit_start(&self, ooda_cycle: u32) {
        self.set_ooda_cycle(ooda_cycle.max(1));
        self.set_iteration(0);  // 重置迭代号
        self.emit_meta("start", serde_json::json!({
            "type": "start",
            "cycle": ooda_cycle
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
        // 更新迭代号（不再改变OODA循环号）
        self.set_iteration(iteration);

        // 收集内容 (JSON 格式)
        if let Ok(mut collector) = self.content_collector.lock() {
            let entry = serde_json::json!({"thought": content, "iteration": iteration});
            collector.push_str(&format!("{}\n", entry));
        }

        // 发送流式内容
        let formatted = format!("\n**Thought (Iteration {})**\n{}\n", iteration, content);
        self.emit_content(&formatted, false);

        // 发送 ooda_step 格式的结构化数据（与前端 TravelMessageProcessor 兼容）
        // cycle 保持为 OODA 循环号，react_iteration 用于标识 ReAct 迭代
        let step = OodaStep {
            cycle: self.get_cycle(),
            phase: "Act".to_string(),  // ReAct 在 OODA 的 Act 阶段执行
            status: "running".to_string(),
            react_iteration: Some(iteration),
            thought: Some(content.to_string()),
            action: None,
            output: None,
            error: None,
        };
        self.emit_ooda_step(&step);
    }

    /// 发送工具调用信息 - 使用 ooda_step 格式
    pub fn emit_tool_call(&self, iteration: u32, tool_name: &str, args: &serde_json::Value) {
            self.set_iteration(iteration);
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
            react_iteration: Some(iteration),
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
        self.set_iteration(iteration);
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
            react_iteration: Some(iteration),
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
        self.set_iteration(iteration);

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
            react_iteration: Some(iteration),
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
            react_iteration: Some(self.get_iteration()),
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
    /// 底层使用 sentinel_llm 的流式客户端
    streaming_client: StreamingLlmClient,
}

impl TravelLlmClient {
    pub fn new(config: LlmConfig, emitter: Arc<TravelMessageEmitter>) -> Self {
        // 将 crate::engines::LlmConfig 转换为 sentinel_llm::LlmConfig
        let sentinel_config = sentinel_llm::LlmConfig::new(&config.provider, &config.model)
            .with_timeout(config.timeout_secs);
        let sentinel_config = if let Some(api_key) = &config.api_key {
            sentinel_config.with_api_key(api_key)
        } else {
            sentinel_config
        };
        let sentinel_config = if let Some(base_url) = &config.base_url {
            sentinel_config.with_base_url(base_url)
        } else {
            sentinel_config
        };
        
        let streaming_client = StreamingLlmClient::new(sentinel_config);
        
        Self { config, emitter, streaming_client }
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
        
        // 注意：请求日志由 sentinel_llm 的 streaming_client 自动记录
        // 不需要在这里重复记录

        // 使用 sentinel_llm 流式客户端
        // 创建 ReAct 解析器和状态容器
        let emitter = self.emitter.clone();
        let mut current_line = String::new();
        let mut parser = ReActParser::new(iteration, emitter.clone());

        // 使用 sentinel_llm 的流式调用
        let result = self.streaming_client.stream_completion(
            system_prompt,
            user_prompt,
            |stream_content| {
                match stream_content {
                    StreamContent::Text(piece) => {
                        if !piece.is_empty() {
                            current_line.push_str(&piece);
                            
                            // 检查是否有完整行可以解析
                            while let Some(pos) = current_line.find('\n') {
                                let line = current_line[..pos].to_string();
                                current_line = current_line[pos + 1..].to_string();
                                parser.process_line(&line);
                            }
                        }
                    }
                    StreamContent::Reasoning(piece) => {
                        if !piece.is_empty() {
                            emitter.emit_thinking(&piece);
                        }
                    }
                    StreamContent::Done => {
                        debug!("TravelLlmClient: Stream completed");
                    }
                }
            },
        ).await;

        match result {
            Ok(content) => {
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
                
                // 注意：响应日志由 sentinel_llm 的 streaming_client 自动记录
                // 不需要在这里重复记录

                Ok(content)
            }
            Err(e) => {
                error!("TravelLlmClient: Stream error: {}", e);
                Err(anyhow!("Travel LLM stream error: {}", e))
            }
        }
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


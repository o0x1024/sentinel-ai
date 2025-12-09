//! Agent 类型定义
//!
//! 定义 Agent 系统所需的核心类型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent 消息类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Thinking,    // 思考过程
    Planning,    // 任务规划
    ToolCall,    // 工具调用
    ToolResult,  // 工具结果
    Progress,    // 进度更新
    Final,       // 最终答案
    Error,       // 错误信息
}

/// Agent 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    pub content: String,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MessageMetadata>,
}

impl AgentMessage {
    pub fn new(msg_type: MessageType, content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            msg_type,
            content: content.into(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: MessageMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn thinking(content: impl Into<String>) -> Self {
        Self::new(MessageType::Thinking, content)
    }

    pub fn planning(content: impl Into<String>) -> Self {
        Self::new(MessageType::Planning, content)
    }

    pub fn tool_call(tool_name: &str, args: &serde_json::Value) -> Self {
        Self::new(MessageType::ToolCall, format!("Calling tool: {}", tool_name))
            .with_metadata(MessageMetadata {
                tool_name: Some(tool_name.to_string()),
                tool_args: Some(args.clone()),
                ..Default::default()
            })
    }

    pub fn tool_result(tool_name: &str, result: impl Into<String>, success: bool, duration_ms: u64) -> Self {
        Self::new(MessageType::ToolResult, result)
            .with_metadata(MessageMetadata {
                tool_name: Some(tool_name.to_string()),
                success: Some(success),
                duration_ms: Some(duration_ms),
                ..Default::default()
            })
    }

    pub fn progress(step_index: usize, total_steps: usize, content: impl Into<String>) -> Self {
        Self::new(MessageType::Progress, content)
            .with_metadata(MessageMetadata {
                step_index: Some(step_index),
                total_steps: Some(total_steps),
                ..Default::default()
            })
    }

    pub fn final_answer(content: impl Into<String>) -> Self {
        Self::new(MessageType::Final, content)
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self::new(MessageType::Error, content)
    }
}

/// 消息元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_args: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_steps: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub args: serde_json::Value,
}

impl ToolCall {
    pub fn new(name: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            args,
        }
    }
}

/// 规划步骤（扩展版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolCall>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>,
}

/// 计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub description: String,
    pub steps: Vec<PlanStep>,
    pub expected_outcome: String,
}

/// 步骤执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub success: bool,
    pub output: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl StepResult {
    pub fn success(step_id: impl Into<String>, output: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            step_id: step_id.into(),
            success: true,
            output,
            error: None,
            duration_ms,
        }
    }

    pub fn failed(step_id: impl Into<String>, error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            step_id: step_id.into(),
            success: false,
            output: serde_json::Value::Null,
            error: Some(error.into()),
            duration_ms,
        }
    }

    pub fn no_tool(step_id: impl Into<String>) -> Self {
        Self {
            step_id: step_id.into(),
            success: true,
            output: serde_json::json!({"message": "No tool execution required"}),
            error: None,
            duration_ms: 0,
        }
    }
}

/// 反思决策
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Decision {
    Complete { answer: String },
    Continue,
    Replan { reason: String },
}

/// 反思结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub decision: Decision,
    pub reasoning: String,
    #[serde(default)]
    pub improvements: Vec<String>,
}

/// Agent 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub execution_id: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub iterations: usize,
    pub duration_ms: u64,
    #[serde(default)]
    pub step_results: Vec<StepResult>,
}

impl AgentResult {
    pub fn completed(execution_id: impl Into<String>, answer: String, iterations: usize, duration_ms: u64) -> Self {
        Self {
            execution_id: execution_id.into(),
            success: true,
            answer: Some(answer),
            error: None,
            iterations,
            duration_ms,
            step_results: vec![],
        }
    }

    pub fn failed(execution_id: impl Into<String>, error: String, iterations: usize, duration_ms: u64) -> Self {
        Self {
            execution_id: execution_id.into(),
            success: false,
            answer: None,
            error: Some(error),
            iterations,
            duration_ms,
            step_results: vec![],
        }
    }

    pub fn cancelled(execution_id: impl Into<String>) -> Self {
        Self {
            execution_id: execution_id.into(),
            success: false,
            answer: None,
            error: Some("Execution cancelled".to_string()),
            iterations: 0,
            duration_ms: 0,
            step_results: vec![],
        }
    }

    pub fn max_iterations_reached(execution_id: impl Into<String>, iterations: usize, duration_ms: u64) -> Self {
        Self {
            execution_id: execution_id.into(),
            success: false,
            answer: None,
            error: Some("Max iterations reached".to_string()),
            iterations,
            duration_ms,
            step_results: vec![],
        }
    }

    pub fn with_step_results(mut self, results: Vec<StepResult>) -> Self {
        self.step_results = results;
        self
    }
}

/// 执行上下文
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub execution_id: String,
    pub task: String,
    pub iteration: usize,
    pub max_iterations: usize,
    pub results: HashMap<String, StepResult>,
    pub messages: Vec<AgentMessage>,
    pub cancelled: bool,
    pub start_time: std::time::Instant,
}

impl ExecutionContext {
    pub fn new(execution_id: impl Into<String>, task: impl Into<String>, max_iterations: usize) -> Self {
        Self {
            execution_id: execution_id.into(),
            task: task.into(),
            iteration: 0,
            max_iterations,
            results: HashMap::new(),
            messages: Vec::new(),
            cancelled: false,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn add_result(&mut self, step_id: String, result: StepResult) {
        self.results.insert(step_id, result);
    }

    pub fn add_message(&mut self, message: AgentMessage) {
        self.messages.push(message);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    pub fn increment_iteration(&mut self) {
        self.iteration += 1;
    }

    pub fn has_reached_max_iterations(&self) -> bool {
        self.iteration >= self.max_iterations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_message_creation() {
        let msg = AgentMessage::thinking("Analyzing the task...");
        assert_eq!(msg.msg_type, MessageType::Thinking);
        assert!(msg.content.contains("Analyzing"));
    }

    #[test]
    fn test_step_result() {
        let result = StepResult::success("step-1", serde_json::json!({"data": "test"}), 100);
        assert!(result.success);
        assert_eq!(result.duration_ms, 100);
    }

    #[test]
    fn test_execution_context() {
        let mut ctx = ExecutionContext::new("exec-1", "Test task", 10);
        assert_eq!(ctx.iteration, 0);
        
        ctx.increment_iteration();
        assert_eq!(ctx.iteration, 1);
        
        ctx.cancel();
        assert!(ctx.is_cancelled());
    }
}


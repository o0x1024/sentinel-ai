//! AI 服务相关类型定义

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// AI 服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub organization: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            api_key: None,
            api_base: None,
            organization: None,
            temperature: Some(0.7),
            max_tokens: Some(4096),
        }
    }
}

/// 模型调度器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub enabled: bool,
    pub intent_analysis_model: String,
    pub intent_analysis_provider: String,
    pub planner_model: String,
    pub planner_provider: String,
    pub replanner_model: String,
    pub replanner_provider: String,
    pub executor_model: String,
    pub executor_provider: String,
    pub evaluator_model: String,
    pub evaluator_provider: String,
    pub default_strategy: String,
    pub max_retries: i32,
    pub timeout_seconds: i32,
    pub scenarios: Value,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            intent_analysis_model: String::new(),
            intent_analysis_provider: String::new(),
            planner_model: String::new(),
            planner_provider: String::new(),
            replanner_model: String::new(),
            replanner_provider: String::new(),
            executor_model: String::new(),
            executor_provider: String::new(),
            evaluator_model: String::new(),
            evaluator_provider: String::new(),
            default_strategy: "adaptive".to_string(),
            max_retries: 3,
            timeout_seconds: 120,
            scenarios: Value::Object(serde_json::Map::new()),
        }
    }
}

/// 调度器阶段
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SchedulerStage {
    IntentAnalysis,
    Planning,
    Replanning,
    Execution,
    Evaluation,
}

/// AI 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
    pub result: Option<Value>,
    pub error: Option<String>,
}

/// 流式消息
#[derive(Debug, Clone, Serialize)]
pub struct StreamMessage {
    pub conversation_id: String,
    pub message_id: String,
    pub content: String,
    pub is_complete: bool,
    pub token_count: Option<u32>,
    pub total_tokens: Option<u32>,
    pub tool_calls: Option<Vec<AiToolCall>>,
    pub is_incremental: bool,
    pub content_delta: Option<String>,
    pub total_content_length: Option<usize>,
    pub intent_type: Option<String>,
    pub stream_phase: Option<String>,
}

/// 任务流式消息
#[derive(Debug, Clone, Serialize)]
pub struct TaskStreamMessage {
    pub conversation_id: String,
    pub message_id: String,
    pub execution_id: String,
    pub phase: String,
    pub content: String,
    pub execution_plan: Option<Value>,
    pub progress: Option<f32>,
    pub current_step: Option<String>,
    pub completed_steps: Option<u32>,
    pub total_steps: Option<u32>,
    pub is_complete: bool,
}

/// 任务进度消息
#[derive(Debug, Clone, Serialize)]
pub struct TaskProgressMessage {
    pub conversation_id: String,
    pub execution_id: String,
    pub step_name: String,
    pub step_index: u32,
    pub total_steps: u32,
    pub progress: f32,
    pub status: String,
    pub result: Option<Value>,
    pub error: Option<String>,
}

/// 流式错误
#[derive(Debug, Clone, Serialize)]
pub struct StreamError {
    pub conversation_id: String,
    pub message_id: Option<String>,
    pub execution_id: Option<String>,
    pub error: String,
    pub error_type: String,
}

/// 工具调用结果消息
#[derive(Debug, Clone, Serialize)]
pub struct ToolCallResultMessage {
    pub conversation_id: String,
    pub message_id: String,
    pub tool_call_id: String,
    pub result: Value,
    pub is_error: bool,
}


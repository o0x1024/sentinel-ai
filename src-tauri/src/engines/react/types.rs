//! ReAct 架构核心类型定义
//!
//! 实现基于 Reasoning + Acting 的循环执行模式，支持：
//! - Thought（思考）→ Action（行动）→ Observation（观察）循环
//! - 强 JSON 优先 + 自然语言解析兜底
//! - 工具调用与 RAG 集成
//! - 流式输出与追踪

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

/// ReAct 执行配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactConfig {
    /// 最大循环步数（防止无限循环）
    pub max_iterations: u32,
    /// 最大思考时间（秒）
    pub max_thinking_time: Option<f64>,
    /// LLM 温度参数
    pub temperature: Option<f32>,
    /// 最大生成 tokens
    pub max_tokens: Option<u32>,
    /// 工具调用超时（秒）
    pub tool_timeout: Option<f64>,
    /// 工具白名单（None 表示允许所有）
    pub allowed_tools: Option<Vec<String>>,
    /// 是否启用 RAG 注入
    pub enable_rag: bool,
    /// RAG 配置
    pub rag_config: Option<RagConfig>,
    /// 停止词列表
    pub stop_sequences: Vec<String>,
    /// 解析策略
    pub parse_strategy: ParseStrategy,
    /// 重试配置
    pub retry_config: RetryConfig,
    /// 是否启用详细日志
    pub verbose: bool,
}

/// RAG 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// 检索 top_k
    pub top_k: usize,
    /// 相似度阈值
    pub similarity_threshold: f32,
    /// 是否使用 MMR
    pub use_mmr: bool,
    /// MMR lambda 参数
    pub mmr_lambda: f32,
    /// 注入时机
    pub injection_point: RagInjectionPoint,
}

/// RAG 注入时机
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RagInjectionPoint {
    /// 仅在首次思考前注入
    Initial,
    /// 每次思考前都注入
    EveryThought,
    /// 作为工具按需调用
    OnDemand,
}

/// 解析策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParseStrategy {
    /// 仅强 JSON
    StrictJson,
    /// JSON 优先，失败时尝试自然语言解析
    JsonWithFallback,
    /// 仅自然语言解析
    NaturalLanguage,
}

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔（秒）
    pub retry_delay: f64,
    /// 退避策略
    pub backoff_strategy: BackoffStrategy,
}

/// 退避策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// 固定间隔
    Fixed,
    /// 指数退避
    Exponential { multiplier: f32 },
    /// 线性增长
    Linear { increment: f64 },
}

/// ReAct 步骤（单次循环的一个环节）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactStep {
    /// 步骤 ID
    pub id: String,
    /// 步骤类型
    pub step_type: ReactStepType,
    /// 时间戳
    pub timestamp: SystemTime,
    /// 执行耗时（毫秒）
    pub duration_ms: Option<u64>,
    /// Token 使用（可选）
    pub token_usage: Option<TokenUsage>,
    /// 错误信息（如有）
    pub error: Option<String>,
}

/// ReAct 步骤类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ReactStepType {
    /// 思考（LLM 推理）
    Thought {
        content: String,
        /// 是否包含 RAG 证据
        has_rag_context: bool,
    },
    /// 行动（工具调用）
    Action {
        tool_call: ReactToolCall,
    },
    /// 观察（工具返回）
    Observation {
        tool_name: String,
        result: serde_json::Value,
        success: bool,
    },
    /// 最终答案
    Final {
        answer: String,
        citations: Vec<String>,
    },
    /// 错误/重试
    Error {
        error_type: String,
        message: String,
        retryable: bool,
    },
}

/// ReAct 工具调用（统一结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactToolCall {
    /// 工具名称
    pub tool: String,
    /// 工具参数（JSON）
    pub args: serde_json::Value,
    /// 调用 ID（用于追踪）
    pub call_id: String,
    /// 是否为并行调用批次的一部分
    pub is_parallel: bool,
}

/// Token 使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

/// ReAct 执行轨迹（完整的循环历史）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactTrace {
    /// 轨迹 ID
    pub trace_id: String,
    /// 任务描述
    pub task: String,
    /// 步骤序列
    pub steps: Vec<ReactStep>,
    /// 开始时间
    pub started_at: SystemTime,
    /// 结束时间
    pub completed_at: Option<SystemTime>,
    /// 最终状态
    pub status: ReactStatus,
    /// 聚合指标
    pub metrics: ReactMetrics,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// ReAct 执行状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReactStatus {
    /// 运行中
    Running,
    /// 成功完成
    Completed,
    /// 失败
    Failed,
    /// 达到上限停止
    MaxIterationsReached,
    /// 用户取消
    Cancelled,
}

/// ReAct 聚合指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactMetrics {
    /// 总迭代次数
    pub total_iterations: u32,
    /// 工具调用次数
    pub tool_calls_count: u32,
    /// 总耗时（毫秒）
    pub total_duration_ms: u64,
    /// Token 总量
    pub total_tokens: u32,
    /// 成功的工具调用
    pub successful_tool_calls: u32,
    /// 失败的工具调用
    pub failed_tool_calls: u32,
    /// 重试次数
    pub retry_count: u32,
}

/// Action 指令（LLM 输出的 JSON 结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ActionInstruction {
    /// 工具调用
    ToolCall {
        action: ReactToolCall,
        #[serde(default)]
        final_answer: bool,
    },
    /// 最终答案
    FinalAnswer {
        #[serde(rename = "final")]
        final_answer: FinalAnswer,
    },
}

/// 最终答案结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalAnswer {
    pub answer: String,
    #[serde(default)]
    pub citations: Vec<String>,
}

/// 自然语言解析结果
#[derive(Debug, Clone)]
pub struct ParsedAction {
    pub tool_name: String,
    pub args: serde_json::Value,
}

// ========== 默认实现 ==========

impl Default for ReactConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            max_thinking_time: Some(300.0), // 5 分钟
            temperature: Some(0.7),
            max_tokens: Some(2000),
            tool_timeout: Some(30.0),
            allowed_tools: None,
            enable_rag: true,
            rag_config: Some(RagConfig::default()),
            stop_sequences: vec![
                "Observation:".to_string(),
                "\nObservation".to_string(),
            ],
            parse_strategy: ParseStrategy::JsonWithFallback,
            retry_config: RetryConfig::default(),
            verbose: false,
        }
    }
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            top_k: 5,
            similarity_threshold: 0.65,
            use_mmr: true,
            mmr_lambda: 0.7,
            injection_point: RagInjectionPoint::Initial,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            retry_delay: 1.0,
            backoff_strategy: BackoffStrategy::Exponential { multiplier: 2.0 },
        }
    }
}

impl Default for ReactMetrics {
    fn default() -> Self {
        Self {
            total_iterations: 0,
            tool_calls_count: 0,
            total_duration_ms: 0,
            total_tokens: 0,
            successful_tool_calls: 0,
            failed_tool_calls: 0,
            retry_count: 0,
        }
    }
}

impl ReactTrace {
    pub fn new(task: String) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            task,
            steps: Vec::new(),
            started_at: SystemTime::now(),
            completed_at: None,
            status: ReactStatus::Running,
            metrics: ReactMetrics::default(),
            metadata: HashMap::new(),
        }
    }

    pub fn add_step(&mut self, step: ReactStep) {
        self.steps.push(step);
    }

    pub fn complete(&mut self, status: ReactStatus) {
        self.status = status;
        self.completed_at = Some(SystemTime::now());
    }
}

impl ReactToolCall {
    pub fn new(tool: String, args: serde_json::Value) -> Self {
        Self {
            tool,
            args,
            call_id: Uuid::new_v4().to_string(),
            is_parallel: false,
        }
    }
}

// 确保线程安全
unsafe impl Send for ReactTrace {}
unsafe impl Sync for ReactTrace {}
unsafe impl Send for ReactStep {}
unsafe impl Sync for ReactStep {}

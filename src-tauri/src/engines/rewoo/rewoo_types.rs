//! ReWOO (Reasoning without Observation) 核心类型定义
//! 
//! 基于 LangGraph ReWOO 标准实现，包含 Planner、Worker、Solver 三个核心模块
//! 参考：https://github.com/langchain-ai/langgraph/blob/main/docs/docs/tutorials/rewoo/rewoo.ipynb

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// ReWOO 状态 - 核心状态结构，在整个执行过程中共享
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReWOOState {
    /// 用户任务描述
    pub task: String,
    /// 计划字符串（由 Planner 生成）
    pub plan_string: String,
    /// 解析后的执行步骤列表
    pub steps: Vec<String>,
    /// 工具执行结果映射 (#E1 -> result)
    pub results: HashMap<String, String>,
    /// 最终结果（由 Solver 生成）
    pub result: String,
}

impl Default for ReWOOState {
    fn default() -> Self {
        Self {
            task: String::new(),
            plan_string: String::new(),
            steps: Vec::new(),
            results: HashMap::new(),
            result: String::new(),
        }
    }
}

/// 计划步骤 - 从计划字符串中解析出的单个步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// 步骤变量名 (如 #E1, #E2)
    pub variable: String,
    /// 工具名称
    pub tool: String,
    /// 工具参数（可能包含变量引用）
    pub args: String,
    /// 推理描述
    pub reasoning: String,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 是否成功
    pub success: bool,
    /// 结果内容
    pub content: String,
    /// 错误信息（如果失败）
    pub error: Option<String>,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
}

/// ReWOO 执行配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReWOOConfig {
    /// Planner 配置
    pub planner: PlannerConfig,
    /// Worker 配置
    pub worker: WorkerConfig,
    /// Solver 配置
    pub solver: SolverConfig,
}

/// Planner 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerConfig {
    /// 使用的模型名称
    pub model_name: String,
    /// 温度参数
    pub temperature: f32,
    /// 最大 token 数
    pub max_tokens: u32,
    /// 最大计划步骤数
    pub max_steps: u32,
}

/// Worker 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// 工具执行超时时间（秒）
    pub timeout_seconds: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 是否启用并行执行
    pub enable_parallel: bool,
}

/// Solver 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverConfig {
    /// 使用的模型名称
    pub model_name: String,
    /// 温度参数
    pub temperature: f32,
    /// 最大 token 数
    pub max_tokens: u32,
}

impl Default for ReWOOConfig {
    fn default() -> Self {
        Self {
            planner: PlannerConfig {
                model_name: "gpt-4".to_string(),
                temperature: 0.0,
                max_tokens: 4000,
                max_steps: 10,
            },
            worker: WorkerConfig {
                timeout_seconds: 300,
                max_retries: 3,
                enable_parallel: false, // 默认串行执行以保证稳定性
            },
            solver: SolverConfig {
                model_name: "gpt-4".to_string(),
                temperature: 0.0,
                max_tokens: 2000,
            },
        }
    }
}

/// ReWOO 错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReWOOError {
    /// 计划生成错误
    PlanningError(String),
    /// 计划解析错误
    PlanParsingError(String),
    /// 工具执行错误
    ToolExecutionError(String),
    /// 变量替换错误
    VariableSubstitutionError(String),
    /// 求解错误
    SolvingError(String),
    /// 配置错误
    ConfigurationError(String),
    /// AI 提供商错误
    AiProviderError(String),
    /// 工具系统错误
    ToolSystemError(String),
}

impl std::fmt::Display for ReWOOError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReWOOError::PlanningError(msg) => write!(f, "Planning error: {}", msg),
            ReWOOError::PlanParsingError(msg) => write!(f, "Plan parsing error: {}", msg),
            ReWOOError::ToolExecutionError(msg) => write!(f, "Tool execution error: {}", msg),
            ReWOOError::VariableSubstitutionError(msg) => write!(f, "Variable substitution error: {}", msg),
            ReWOOError::SolvingError(msg) => write!(f, "Solving error: {}", msg),
            ReWOOError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            ReWOOError::AiProviderError(msg) => write!(f, "AI provider error: {}", msg),
            ReWOOError::ToolSystemError(msg) => write!(f, "Tool system error: {}", msg),
        }
    }
}

impl std::error::Error for ReWOOError {}

/// 节点路由结果
#[derive(Debug, Clone, PartialEq)]
pub enum NodeRoute {
    /// 继续执行工具
    Tool,
    /// 进入求解阶段
    Solve,
    /// 结束执行
    End,
}

/// ReWOO 执行统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReWOOMetrics {
    /// 总执行时间（毫秒）
    pub total_time_ms: u64,
    /// 计划生成时间（毫秒）
    pub planning_time_ms: u64,
    /// 工具执行时间（毫秒）
    pub working_time_ms: u64,
    /// 求解时间（毫秒）
    pub solving_time_ms: u64,
    /// 工具调用次数
    pub tool_calls: u32,
    /// 成功的工具调用次数
    pub successful_tool_calls: u32,
    /// 总 token 消耗
    pub total_tokens: u32,
}

impl Default for ReWOOMetrics {
    fn default() -> Self {
        Self {
            total_time_ms: 0,
            planning_time_ms: 0,
            working_time_ms: 0,
            solving_time_ms: 0,
            tool_calls: 0,
            successful_tool_calls: 0,
            total_tokens: 0,
        }
    }
}

/// ReWOO 执行会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReWOOSession {
    /// 会话 ID
    pub id: String,
    /// 当前状态
    pub state: ReWOOState,
    /// 执行统计
    pub metrics: ReWOOMetrics,
    /// 开始时间
    pub started_at: SystemTime,
    /// 结束时间
    pub completed_at: Option<SystemTime>,
    /// 是否完成
    pub is_completed: bool,
    /// 错误信息
    pub error: Option<String>,
}

impl ReWOOSession {
    /// 创建新会话
    pub fn new(id: String, task: String) -> Self {
        let mut state = ReWOOState::default();
        state.task = task;
        
        Self {
            id,
            state,
            metrics: ReWOOMetrics::default(),
            started_at: SystemTime::now(),
            completed_at: None,
            is_completed: false,
            error: None,
        }
    }
    
    /// 标记会话完成
    pub fn complete(&mut self) {
        self.completed_at = Some(SystemTime::now());
        self.is_completed = true;
        
        if let Ok(duration) = self.completed_at.unwrap().duration_since(self.started_at) {
            self.metrics.total_time_ms = duration.as_millis() as u64;
        }
    }
    
    /// 标记会话失败
    pub fn fail(&mut self, error: String) {
        self.completed_at = Some(SystemTime::now());
        self.is_completed = true;
        self.error = Some(error);
        
        if let Ok(duration) = self.completed_at.unwrap().duration_since(self.started_at) {
            self.metrics.total_time_ms = duration.as_millis() as u64;
        }
    }
}
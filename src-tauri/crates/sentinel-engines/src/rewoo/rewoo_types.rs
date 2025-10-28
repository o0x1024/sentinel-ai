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
    /// 工具执行结果映射 (#E1 -> result) - 升级为 JSON 以支持结构化数据
    pub results: HashMap<String, serde_json::Value>,
    /// 最终结果（由 Solver 生成）
    pub result: String,
    /// 执行轨迹（新增）
    pub execution_trace: Option<ExecutionTrace>,
}

impl Default for ReWOOState {
    fn default() -> Self {
        Self {
            task: String::new(),
            plan_string: String::new(),
            steps: Vec::new(),
            results: HashMap::new(),
            result: String::new(),
            execution_trace: None,
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
    /// 结果内容（字符串格式，向后兼容）
    pub content: String,
    /// 结构化 JSON 内容（新增）
    pub json_content: Option<serde_json::Value>,
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

/// 步骤依赖信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepDependency {
    /// 步骤变量名
    pub variable: String,
    /// 依赖的变量列表
    pub dependencies: Vec<String>,
    /// 是否已准备执行
    pub ready: bool,
}

/// 执行批次
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionBatch {
    /// 批次 ID
    pub batch_id: usize,
    /// 包含的步骤变量
    pub steps: Vec<String>,
    /// 是否完成
    pub completed: bool,
}

/// 步骤纠错类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CorrectionType {
    /// 替换工具
    ToolReplacement { original_tool: String, replacement_tool: String },
    /// 修正参数
    ParameterFix { original_args: String, corrected_args: String },
    /// 添加缺失参数
    ParameterAddition { missing_params: std::collections::HashMap<String, serde_json::Value> },
    /// 移除无效步骤
    StepRemoval { reason: String },
}

/// 纠错建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionSuggestion {
    /// 问题步骤变量
    pub step_variable: String,
    /// 纠错类型
    pub correction_type: CorrectionType,
    /// 置信度 (0-100)
    pub confidence: f32,
    /// 纠错原因
    pub reason: String,
    /// 修正后的步骤字符串
    pub corrected_step: Option<String>,
}

/// 任务域类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskDomain {
    /// 网络安全
    CyberSecurity,
    /// 数据分析
    DataAnalysis,
    /// 问答咨询
    QuestionAnswering,
    /// API 集成
    ApiIntegration,
    /// 信息检索
    InformationRetrieval,
    /// 内容生成
    ContentGeneration,
    /// 通用任务
    General,
}

impl TaskDomain {
    /// 从任务描述和工具类型推断任务域
    pub fn infer_from_task(task_description: &str, tool_names: &[String]) -> Self {
        let task_lower = task_description.to_lowercase();
        
        // 网络安全关键词
        let security_keywords = [
            "scan", "vulnerability", "security", "penetration", "exploit", 
            "nmap", "nuclei", "subdomain", "port", "attack", "threat"
        ];
        
        // 数据分析关键词
        let data_keywords = [
            "analyze", "statistics", "chart", "graph", "data", "metrics",
            "report", "dashboard", "visualization", "trend"
        ];
        
        // API 集成关键词
        let api_keywords = [
            "api", "endpoint", "request", "response", "webhook", "integration",
            "service", "microservice", "rest", "graphql"
        ];
        
        // 检查工具名称
        let security_tools = ["nmap", "nuclei", "portscan", "vulnerability_scan", "rsubdomain"];
        let has_security_tools = tool_names.iter().any(|tool| {
            security_tools.iter().any(|sec_tool| tool.to_lowercase().contains(sec_tool))
        });
        
        // 检查任务描述
        if security_keywords.iter().any(|keyword| task_lower.contains(keyword)) || has_security_tools {
            TaskDomain::CyberSecurity
        } else if data_keywords.iter().any(|keyword| task_lower.contains(keyword)) {
            TaskDomain::DataAnalysis
        } else if api_keywords.iter().any(|keyword| task_lower.contains(keyword)) {
            TaskDomain::ApiIntegration
        } else if task_lower.contains("question") || task_lower.contains("what") || task_lower.contains("how") {
            TaskDomain::QuestionAnswering
        } else if task_lower.contains("search") || task_lower.contains("find") || task_lower.contains("retrieve") {
            TaskDomain::InformationRetrieval
        } else if task_lower.contains("generate") || task_lower.contains("create") || task_lower.contains("write") {
            TaskDomain::ContentGeneration
        } else {
            TaskDomain::General
        }
    }
}

/// 参数验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterValidationResult {
    /// 是否通过验证
    pub is_valid: bool,
    /// 规范化后的参数
    pub normalized_params: std::collections::HashMap<String, serde_json::Value>,
    /// 验证错误信息
    pub errors: Vec<String>,
    /// 警告信息
    pub warnings: Vec<String>,
    /// 应用的默认值
    pub applied_defaults: Vec<String>,
}

/// 参数转换规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterConversionRule {
    /// 参数名
    pub parameter_name: String,
    /// 源类型
    pub from_type: String,
    /// 目标类型
    pub to_type: String,
    /// 转换函数名
    pub converter: String,
}

impl Default for ParameterValidationResult {
    fn default() -> Self {
        Self {
            is_valid: true,
            normalized_params: std::collections::HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            applied_defaults: Vec::new(),
        }
    }
}

/// 错误类型分类
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// 网络错误（可重试）
    Network,
    /// 超时错误（可重试）
    Timeout,
    /// 参数错误（一般不可重试）
    Parameter,
    /// 工具不可用（可尝试替代）
    ToolUnavailable,
    /// 资源不足（可重试，延长间隔）
    ResourceExhaustion,
    /// 权限错误（不可重试）
    Permission,
    /// 服务器内部错误（可重试）
    ServerError,
    /// 未知错误（可重试，谨慎）
    Unknown,
}

/// 重试策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStrategy {
    /// 错误类型
    pub error_category: ErrorCategory,
    /// 最大重试次数
    pub max_retries: u32,
    /// 基础延迟（毫秒）
    pub base_delay_ms: u64,
    /// 是否使用指数退避
    pub exponential_backoff: bool,
    /// 退避倍数
    pub backoff_multiplier: f32,
    /// 最大延迟（毫秒）
    pub max_delay_ms: u64,
    /// 是否添加随机抖动
    pub jitter: bool,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            error_category: ErrorCategory::Unknown,
            max_retries: 3,
            base_delay_ms: 1000,
            exponential_backoff: true,
            backoff_multiplier: 2.0,
            max_delay_ms: 30000,
            jitter: true,
        }
    }
}

/// 错误分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorAnalysis {
    /// 错误类型
    pub category: ErrorCategory,
    /// 是否建议重试
    pub should_retry: bool,
    /// 建议的重试策略
    pub retry_strategy: RetryStrategy,
    /// 错误原因分析
    pub analysis: String,
    /// 可能的修复建议
    pub suggestions: Vec<String>,
}

/// 步骤执行轨迹
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepTrace {
    /// 步骤变量名
    pub step_variable: String,
    /// 步骤推理
    pub reasoning: String,
    /// 工具名称
    pub tool_name: String,
    /// 原始参数
    pub original_args: String,
    /// 替换后参数
    pub substituted_args: String,
    /// 规范化后参数
    pub normalized_params: std::collections::HashMap<String, serde_json::Value>,
    /// 开始时间
    pub start_time: std::time::SystemTime,
    /// 结束时间
    pub end_time: Option<std::time::SystemTime>,
    /// 执行时长（毫秒）
    pub duration_ms: Option<f64>,
    /// 重试次数
    pub retry_count: u32,
    /// 是否成功
    pub success: bool,
    /// 输出结果
    pub output: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 错误分析
    pub error_analysis: Option<ErrorAnalysis>,
    /// 应用的纠错建议
    pub applied_corrections: Vec<String>,
}

/// 执行会话轨迹
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    /// 会话 ID
    pub session_id: String,
    /// 原始任务
    pub original_task: String,
    /// 推断的任务域
    pub inferred_domain: TaskDomain,
    /// 计划生成轨迹
    pub planning_trace: PlanningTrace,
    /// 步骤执行轨迹
    pub step_traces: Vec<StepTrace>,
    /// 求解轨迹
    pub solving_trace: SolvingTrace,
    /// 总执行时间
    pub total_duration_ms: f64,
    /// 成功率
    pub success_rate: f32,
    /// 资源使用情况
    pub resource_usage: std::collections::HashMap<String, f64>,
}

/// 计划生成轨迹
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningTrace {
    /// 生成的原始计划
    pub raw_plan: String,
    /// 清洗后的计划
    pub cleaned_plan: String,
    /// 解析出的步骤数
    pub parsed_steps_count: usize,
    /// 应用的防护措施
    pub applied_guards: Vec<String>,
    /// 生成时间（毫秒）
    pub generation_time_ms: f64,
    /// 使用的模型
    pub model_used: String,
    /// Token 消耗
    pub tokens_used: Option<u32>,
}

/// 求解轨迹
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolvingTrace {
    /// 选择的模板类型
    pub template_type: String,
    /// 最终答案长度
    pub answer_length: usize,
    /// 求解时间（毫秒）
    pub solving_time_ms: f64,
    /// 置信度评分
    pub confidence_score: f32,
    /// 证据来源数量
    pub evidence_sources_count: usize,
}

impl StepTrace {
    /// 创建新的步骤轨迹
    pub fn new(step_variable: String, tool_name: String, reasoning: String) -> Self {
        Self {
            step_variable,
            reasoning,
            tool_name,
            original_args: String::new(),
            substituted_args: String::new(),
            normalized_params: std::collections::HashMap::new(),
            start_time: std::time::SystemTime::now(),
            end_time: None,
            duration_ms: None,
            retry_count: 0,
            success: false,
            output: None,
            error: None,
            error_analysis: None,
            applied_corrections: Vec::new(),
        }
    }
    
    /// 标记步骤完成
    pub fn complete_with_success(&mut self, output: serde_json::Value) {
        self.end_time = Some(std::time::SystemTime::now());
        self.duration_ms = self.end_time
            .and_then(|end| end.duration_since(self.start_time).ok())
            .map(|d| d.as_millis() as f64);
        self.success = true;
        self.output = Some(output);
    }
    
    /// 标记步骤失败
    pub fn complete_with_error(&mut self, error: String, error_analysis: Option<ErrorAnalysis>) {
        self.end_time = Some(std::time::SystemTime::now());
        self.duration_ms = self.end_time
            .and_then(|end| end.duration_since(self.start_time).ok())
            .map(|d| d.as_millis() as f64);
        self.success = false;
        self.error = Some(error);
        self.error_analysis = error_analysis;
    }
}
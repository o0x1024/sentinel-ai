//! Plan-and-Execute 架构的类型定义
//! 
//! 定义了Plan-and-Execute架构中使用的核心数据结构和错误类型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

/// Plan-and-Execute 引擎整体配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAndExecuteConfig {
    /// 规划器配置
    pub planner: crate::engines::plan_and_execute::planner::PlannerConfig,
    /// 执行器配置
    pub executor: crate::engines::plan_and_execute::executor::ExecutorConfig,
    /// 重新规划器配置
    pub replanner: crate::engines::plan_and_execute::replanner::ReplannerConfig,
    /// 内存管理器配置
    pub memory_manager: crate::engines::plan_and_execute::memory_manager::MemoryManagerConfig,
    /// 引擎名称
    pub engine_name: String,
    /// 引擎版本
    pub engine_version: String,
    /// 是否启用调试模式
    pub debug_mode: bool,
    /// 性能指标收集配置
    pub metrics_config: MetricsConfig,
}

/// 性能指标收集配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// 是否启用指标收集
    pub enabled: bool,
    /// 指标收集间隔（秒）
    pub collection_interval: u64,
    /// 是否收集详细步骤指标
    pub detailed_step_metrics: bool,
    /// 是否收集资源使用指标
    pub resource_metrics: bool,
}

impl Default for PlanAndExecuteConfig {
    fn default() -> Self {
        Self {
            planner: crate::engines::plan_and_execute::planner::PlannerConfig::default(),
            executor: crate::engines::plan_and_execute::executor::ExecutorConfig::default(),
            replanner: crate::engines::plan_and_execute::replanner::ReplannerConfig::default(),
            memory_manager: crate::engines::plan_and_execute::memory_manager::MemoryManagerConfig::default(),
            engine_name: "Plan-and-Execute".to_string(),
            engine_version: "1.0.0".to_string(),
            debug_mode: false,
            metrics_config: MetricsConfig::default(),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval: 60,
            detailed_step_metrics: false,
            resource_metrics: false,
        }
    }
}

/// 通用执行状态 - 参考 LangGraph 设计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecuteState {
    /// 用户输入
    pub input: String,
    /// 生成的计划
    pub plan: Vec<String>,
    /// 执行历史 (步骤, 结果)
    pub past_steps: Vec<(String, String)>,
    /// 最终响应
    pub response: Option<String>,
    /// 当前执行步骤索引
    pub current_step: usize,
    /// 是否需要重新规划
    pub should_replan: bool,
    /// 上下文信息
    pub context: HashMap<String, serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
}

impl Default for PlanExecuteState {
    fn default() -> Self {
        Self {
            input: String::new(),
            plan: Vec::new(),
            past_steps: Vec::new(),
            response: None,
            current_step: 0,
            should_replan: false,
            context: HashMap::new(),
            error: None,
        }
    }
}

/// Plan-and-Execute 错误类型
#[derive(Debug, thiserror::Error)]
pub enum PlanAndExecuteError {
    #[error("Initialization failed: {0}")]
    InitializationError(String),

    #[error("Planning failed: {0}")]
    PlanningFailed(String),

    #[error("AI error: {0}")]
    AiError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Replanning failed: {0}")]
    ReplanningFailed(String),
    
    #[error("Memory operation failed: {0}")]
    MemoryFailed(String),
    
    #[error("Tool operation failed: {0}")]
    ToolFailed(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("AI adapter error: {0}")]
    AiAdapterError(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Status invalid error: {0}")]
    InvalidState(String),
    
    #[error("Resource limit exceeded error: {0}")]
    ResourceLimitExceeded(String),
    
    #[error("Session not found error: {0}")]
    SessionNotFound(String),
    
    #[error("Invalid step configuration: {step_name} - {reason}")]
    InvalidStepConfiguration { step_name: String, reason: String },
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// 任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    /// 任务ID
    pub id: String,
    /// 任务名称
    pub name: String,
    /// 任务描述
    pub description: String,
    /// 任务类型
    pub task_type: TaskType,
    /// 目标信息（可选，某些任务可能不需要特定目标）
    pub target: Option<TargetInfo>,
    /// 任务参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 优先级
    pub priority: Priority,
    /// 约束条件
    pub constraints: HashMap<String, serde_json::Value>,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
    /// 创建时间
    pub created_at: SystemTime,
}

/// 任务类型 - 通用化设计
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskType {
    /// 研究分析
    Research,
    /// 问题解决
    ProblemSolving,
    /// 数据处理
    DataProcessing,
    /// 创作生成
    ContentGeneration,
    /// 信息检索
    InformationRetrieval,
    /// 任务自动化
    Automation,
    /// 推理分析
    Reasoning,
    /// 文档处理
    DocumentProcessing,
    /// 代码相关
    CodeRelated,
    /// 通信交互
    Communication,
    /// 自定义任务
    Custom(String),
}

/// 目标信息 - 通用化设计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetInfo {
    /// 目标类型
    pub target_type: TargetType,
    /// 主要标识符（URL、文件路径、文本内容等）
    pub identifier: String,
    /// 辅助参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 认证信息（如API密钥等）
    pub credentials: Option<HashMap<String, String>>,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 目标类型 - 通用化设计
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TargetType {
    /// 文本内容
    Text,
    /// 文件资源
    File,
    /// 网络资源
    Url,
    /// 数据集
    Dataset,
    /// API端点
    Api,
    /// 数据库
    Database,
    /// 服务
    Service,
    /// 应用程序
    Application,
    /// 通用输入
    GenericInput,
    /// 上下文信息
    Context,
}

/// 优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Normal,
    Medium,
    High,
    Critical,
}

/// 任务结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// 任务ID
    pub task_id: String,
    /// 执行状态
    pub status: TaskStatus,
    /// 开始时间
    pub started_at: SystemTime,
    /// 结束时间
    pub completed_at: Option<SystemTime>,
    /// 执行结果数据
    pub result_data: serde_json::Value,
    /// 错误信息
    pub error: Option<String>,
    /// 执行指标
    pub metrics: TaskMetrics,
    /// 生成的报告
    pub reports: Vec<TaskReport>,
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    /// 待执行
    Pending,
    /// 规划中
    Planning,
    /// 执行中
    Executing,
    /// 重规划中
    Replanning,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 已取消
    Cancelled,
    /// 需要人工干预
    RequiresIntervention,
}

/// 任务指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    /// 开始时间
    pub start_time: SystemTime,
    /// 结束时间
    pub end_time: Option<SystemTime>,
    /// 总执行时间(毫秒)
    pub total_duration_ms: u64,
    /// 步骤数量
    pub total_steps: usize,
    /// 成功步骤数
    pub successful_steps: usize,
    /// 失败步骤数
    pub failed_steps: usize,
    /// 重试次数
    pub retry_count: usize,
    /// 内存使用量(字节)
    pub memory_usage_bytes: u64,
    /// CPU使用时间(毫秒)
    pub cpu_time_ms: u64,
    /// 网络请求数
    pub network_requests: usize,
    /// 工具调用数
    pub tool_calls: usize,
}

/// 任务报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskReport {
    /// 报告ID
    pub id: String,
    /// 报告类型
    pub report_type: ReportType,
    /// 报告标题
    pub title: String,
    /// 报告内容
    pub content: String,
    /// 报告格式
    pub format: ReportFormat,
    /// 生成时间
    pub generated_at: SystemTime,
    /// 附件
    pub attachments: Vec<ReportAttachment>,
}

/// 报告类型 - 通用化设计
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReportType {
    /// 执行摘要
    ExecutionSummary,
    /// 详细报告
    DetailedReport,
    /// 分析结果
    AnalysisResult,
    /// 处理结果
    ProcessingResult,
    /// 自定义报告
    Custom(String),
}

/// 报告格式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReportFormat {
    /// 纯文本
    Text,
    /// Markdown
    Markdown,
    /// HTML
    Html,
    /// JSON
    Json,
    /// PDF
    Pdf,
}

/// 报告附件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportAttachment {
    /// 附件名称
    pub name: String,
    /// 附件类型
    pub content_type: String,
    /// 附件大小（字节）
    pub size: f64,
    /// 附件路径或URL
    pub path: String,
}

/// 执行计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// 计划ID
    pub id: String,
    /// 任务ID
    pub task_id: String,
    /// 计划名称
    pub name: String,
    /// 计划描述
    pub description: String,
    /// 执行步骤
    pub steps: Vec<ExecutionStep>,
    /// 预估执行时间（秒）
    pub estimated_duration: u64,
    /// 创建时间
    pub created_at: SystemTime,
    /// 依赖关系
    pub dependencies: HashMap<String, Vec<String>>,
    /// 计划元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 执行步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    /// 步骤ID
    pub id: String,
    /// 步骤名称
    pub name: String,
    /// 步骤描述
    pub description: String,
    /// 步骤类型
    pub step_type: StepType,
    /// 工具配置
    pub tool_config: Option<ToolConfig>,
    /// 步骤参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 预估执行时间（秒）
    pub estimated_duration: u64,
    /// 重试配置
    pub retry_config: RetryConfig,
    /// 前置条件
    pub preconditions: Vec<String>,
    /// 后置条件
    pub postconditions: Vec<String>,
}

/// 步骤类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StepType {
    /// 工具调用
    ToolCall,
    /// AI推理
    AiReasoning,
    /// 数据处理
    DataProcessing,
    /// 条件判断
    Conditional,
    /// 并行执行
    Parallel,
    /// 等待
    Wait,
    /// 人工确认
    ManualConfirmation,
}

/// 工具配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// 工具名称
    pub tool_name: String,
    /// 工具版本
    pub tool_version: Option<String>,
    /// 工具参数
    pub tool_args: HashMap<String, serde_json::Value>,
    /// 超时设置（秒）
    pub timeout: Option<u64>,
    /// 环境变量
    pub env_vars: HashMap<String, String>,
}

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔（秒）
    pub retry_interval: u64,
    /// 退避策略
    pub backoff_strategy: BackoffStrategy,
}

/// 退避策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackoffStrategy {
    /// 固定间隔
    Fixed,
    /// 线性增长
    Linear,
    /// 指数增长
    Exponential,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_interval: 2,
            backoff_strategy: BackoffStrategy::Exponential,
        }
    }
}

impl Default for TaskMetrics {
    fn default() -> Self {
        Self {
            start_time: SystemTime::now(),
            end_time: None,
            total_duration_ms: 0,
            total_steps: 0,
            successful_steps: 0,
            failed_steps: 0,
            retry_count: 0,
            memory_usage_bytes: 0,
            cpu_time_ms: 0,
            network_requests: 0,
            tool_calls: 0,
        }
    }
}

impl Default for TaskRequest {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "Default Task".to_string(),
            description: "Default task description".to_string(),
            task_type: TaskType::Custom("default".to_string()),
            target: None,
            parameters: HashMap::new(),
            priority: Priority::Medium,
            constraints: HashMap::new(),
            metadata: HashMap::new(),
            created_at: SystemTime::now(),
        }
    }
}

impl Default for TargetInfo {
    fn default() -> Self {
        Self {
            target_type: TargetType::GenericInput,
            identifier: "".to_string(),
            parameters: HashMap::new(),
            credentials: None,
            metadata: HashMap::new(),
        }
    }
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            tool_name: "default".to_string(),
            tool_version: None,
            tool_args: HashMap::new(),
            timeout: Some(300), // 5分钟默认超时
            env_vars: HashMap::new(),
        }
    }
}
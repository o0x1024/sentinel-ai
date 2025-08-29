//! Plan-and-Execute 引擎的核心数据类型定义
//! 
//! 这个模块定义了Plan-and-Execute架构中使用的所有核心数据结构，
//! 包括执行计划、步骤、会话、监控指标等。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

/// 执行计划 - 包含完整的任务分解和执行策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// 计划唯一标识符
    pub id: String,
    /// 计划名称
    pub name: String,
    /// 计划描述
    pub description: String,
    /// 执行步骤列表
    pub steps: Vec<PlanStep>,
    /// 预估执行时间（秒）
    pub estimated_duration: u64,
    /// 计划创建时间
    pub created_at: SystemTime,
    /// 计划元数据
    pub metadata: PlanMetadata,
    /// 依赖关系图
    pub dependencies: HashMap<String, Vec<String>>,
}

/// 计划元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetadata {
    /// 计划类型
    pub plan_type: PlanType,
    /// 优先级
    pub priority: Priority,
    /// 复杂度评估
    pub complexity: Complexity,
    /// 风险等级
    pub risk_level: RiskLevel,
    /// 标签
    pub tags: Vec<String>,
}

/// 计划类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanType {
    /// 安全扫描
    SecurityScan,
    /// 漏洞评估
    VulnerabilityAssessment,
    /// 渗透测试
    PenetrationTest,
    /// 资产发现
    AssetDiscovery,
    /// 自定义任务
    Custom(String),
}

/// 优先级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// 复杂度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Complexity {
    Simple,
    Medium,
    Complex,
    VeryComplex,
}

/// 风险等级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// 计划步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// 步骤唯一标识符
    pub id: String,
    /// 步骤名称
    pub name: String,
    /// 步骤描述
    pub description: String,
    /// 步骤类型
    pub step_type: StepType,
    /// 工具调用配置
    pub tool_config: ToolConfig,
    /// 预估执行时间（秒）
    pub estimated_duration: u64,
    /// 重试配置
    pub retry_config: RetryConfig,
    /// 步骤参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 前置条件
    pub preconditions: Vec<String>,
    /// 后置条件
    pub postconditions: Vec<String>,
}

/// 步骤类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    /// 工具调用
    ToolCall,
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


/// 流式消息类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StreamMessageType {
    /// AI生成的内容块
    Content,
    /// 工具执行状态更新
    ToolUpdate,
    /// 计划更新
    PlanUpdate,
    /// 最终结果
    FinalResult,
    /// 错误信息
    Error,
}

/// 统一流式消息结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedStreamMessage {
    /// 执行ID
    pub execution_id: String,
    /// 消息ID（可选）
    pub message_id: Option<String>,
    /// 对话ID（可选）
    pub conversation_id: Option<String>,
    /// 消息类型
    pub message_type: StreamMessageType,
    
    /// 内容块 (用于 Content 类型)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_delta: Option<String>,
    
    /// 工具执行信息 (用于 ToolUpdate 类型)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_execution: Option<serde_json::Value>,
    
    /// 执行计划 (用于 PlanUpdate 类型)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_plan: Option<serde_json::Value>,
    
    /// 最终内容 (用于 FinalResult 类型)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_content: Option<String>,
    
    /// 错误信息 (用于 Error 类型)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// 是否为流的最后一个消息
    pub is_complete: bool,
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
    /// 重试条件
    pub retry_conditions: Vec<RetryCondition>,
}

/// 退避策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// 固定间隔
    Fixed,
    /// 线性增长
    Linear,
    /// 指数增长
    Exponential,
    /// 自定义
    Custom(Vec<u64>),
}

/// 重试条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryCondition {
    /// 网络错误
    NetworkError,
    /// 超时
    Timeout,
    /// 特定错误码
    ErrorCode(i32),
    /// 自定义条件
    Custom(String),
}

/// 执行会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSession {
    /// 会话唯一标识符
    pub id: String,
    /// 关联的计划ID
    pub plan_id: String,
    /// 会话状态
    pub status: ExecutionStatus,
    /// 开始时间
    pub started_at: SystemTime,
    /// 结束时间
    pub completed_at: Option<SystemTime>,
    /// 当前执行步骤
    pub current_step: Option<i32>,
    /// 执行进度（0-100）
    pub progress: f32,
    /// 执行上下文
    pub context: ExecutionContext,
    /// 步骤执行结果
    pub step_results: HashMap<String, StepExecutionResult>,
    /// 会话元数据
    pub metadata: SessionMetadata,
}

/// 执行状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 已取消
    Cancelled,
    /// 需要人工干预
    RequiresIntervention,
}

/// 执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// 用户ID
    pub user_id: Option<String>,
    /// 目标信息
    pub target_info: TargetInfo,
    /// 环境变量
    pub environment: HashMap<String, String>,
    /// 共享数据
    pub shared_data: HashMap<String, serde_json::Value>,
    /// 配置参数
    pub config: HashMap<String, serde_json::Value>,
}

/// 目标信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetInfo {
    /// 目标类型
    pub target_type: TargetType,
    /// 目标地址
    pub address: String,
    /// 目标值
    pub target_value: String,
    /// 端口范围
    pub port_range: Option<String>,
    /// 协议
    pub protocols: Vec<String>,
    /// 认证信息
    pub credentials: Option<HashMap<String, String>>,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 目标类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetType {
    /// 单个主机
    Host,
    /// 网络段
    Network,
    /// 域名
    Domain,
    /// URL
    Url,
    /// IP地址
    IpAddress,
}

/// 步骤执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionResult {
    /// 步骤ID
    pub step_id: String,
    /// 执行状态
    pub status: StepExecutionStatus,
    /// 开始时间
    pub started_at: SystemTime,
    /// 结束时间
    pub completed_at: Option<SystemTime>,
    /// 执行结果数据
    pub result_data: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<ExecutionError>,
    /// 重试次数
    pub retry_count: u32,
    /// 执行日志
    pub logs: Vec<ExecutionLog>,
    /// 性能指标
    pub metrics: ExecutionMetrics,
}

/// 步骤执行状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepExecutionStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 已跳过
    Skipped,
    /// 重试中
    Retrying,
}

/// 执行错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    /// 错误类型
    pub error_type: ErrorType,
    /// 错误消息
    pub message: String,
    /// 错误详情
    pub details: Option<String>,
    /// 错误码
    pub error_code: Option<i32>,
    /// 是否可重试
    pub retryable: bool,
    /// 发生时间
    pub timestamp: SystemTime,
}

/// 错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    /// 网络错误
    Network,
    /// 超时
    Timeout,
    /// 认证失败
    Authentication,
    /// 权限不足
    Permission,
    /// 配置错误
    Configuration,
    /// 工具错误
    Tool,
    /// 系统错误
    System,
    /// 用户错误
    User,
    /// 未知错误
    Unknown,
}

/// 执行日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
    /// 日志级别
    pub level: LogLevel,
    /// 日志消息
    pub message: String,
    /// 时间戳
    pub timestamp: SystemTime,
    /// 来源
    pub source: String,
    /// 额外数据
    pub data: Option<serde_json::Value>,
}

/// 日志级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// 执行指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
    /// 内存使用（字节）
    pub memory_usage_bytes: u64,
    /// CPU使用率（百分比）
    pub cpu_usage_percent: f32,
    /// 网络IO（字节）
    pub network_io_bytes: u64,
    /// 磁盘IO（字节）
    pub disk_io_bytes: u64,
    /// 自定义指标
    pub custom_metrics: HashMap<String, f64>,
}

/// 会话元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// 创建者
    pub created_by: Option<String>,
    /// 标签
    pub tags: Vec<String>,
    /// 注释
    pub notes: Option<String>,
    /// 版本
    pub version: String,
    /// 自定义属性
    pub custom_attributes: HashMap<String, serde_json::Value>,
}

/// 异常检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    /// 异常ID
    pub id: String,
    /// 异常类型
    pub anomaly_type: AnomalyType,
    /// 严重程度
    pub severity: Severity,
    /// 异常描述
    pub description: String,
    /// 检测时间
    pub detected_at: SystemTime,
    /// 相关步骤ID
    pub step_id: Option<String>,
    /// 异常数据
    pub data: serde_json::Value,
    /// 建议操作
    pub suggested_actions: Vec<String>,
}

/// 异常类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    /// 性能异常
    Performance,
    /// 错误率异常
    ErrorRate,
    /// 资源使用异常
    ResourceUsage,
    /// 执行时间异常
    ExecutionTime,
    /// 数据异常
    Data,
    /// 安全异常
    Security,
}

/// 严重程度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// 执行反馈
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionFeedback {
    /// 反馈ID
    pub id: String,
    /// 会话ID
    pub session_id: String,
    /// 反馈类型
    pub feedback_type: FeedbackType,
    /// 反馈内容
    pub content: String,
    /// 评分（1-5）
    pub rating: Option<u8>,
    /// 创建时间
    pub created_at: SystemTime,
    /// 反馈来源
    pub source: FeedbackSource,
    /// 相关数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 反馈类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackType {
    /// 成功反馈
    Success,
    /// 错误反馈
    Error,
    /// 改进建议
    Improvement,
    /// 用户评价
    UserRating,
    /// 系统建议
    SystemSuggestion,
}

/// 反馈来源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackSource {
    /// 用户
    User,
    /// 系统
    System,
    /// 监控
    Monitor,
    /// 外部工具
    ExternalTool,
}

/// 计划约束
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningConstraints {
    /// 最大执行时间（秒）
    pub max_execution_time: Option<u64>,
    /// 最大步骤数
    pub max_steps: Option<u32>,
    /// 资源限制
    pub resource_limits: ResourceLimits,
    /// 允许的工具列表
    pub allowed_tools: Option<Vec<String>>,
    /// 禁止的工具列表
    pub forbidden_tools: Vec<String>,
    /// 并发限制
    pub concurrency_limit: Option<u32>,
    /// 自定义约束
    pub custom_constraints: HashMap<String, serde_json::Value>,
}

/// 资源限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// 最大内存使用（字节）
    pub max_memory_bytes: Option<u64>,
    /// 最大CPU使用率（百分比）
    pub max_cpu_percent: Option<f32>,
    /// 最大网络带宽（字节/秒）
    pub max_network_bps: Option<u64>,
    /// 最大磁盘使用（字节）
    pub max_disk_bytes: Option<u64>,
}

/// 执行状态详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    /// 当前状态
    pub status: ExecutionStatus,
    /// 状态描述
    pub description: String,
    /// 状态变更时间
    pub timestamp: SystemTime,
    /// 相关数据
    pub data: HashMap<String, serde_json::Value>,
    /// 下一个可能的状态
    pub next_possible_states: Vec<ExecutionStatus>,
}

/// 计划修订记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRevision {
    /// 修订ID
    pub id: String,
    /// 原计划ID
    pub original_plan_id: String,
    /// 修订后的计划
    pub revised_plan: ExecutionPlan,
    /// 修订原因
    pub revision_reason: String,
    /// 修订时间
    pub revised_at: SystemTime,
    /// 修订者
    pub revised_by: RevisionSource,
    /// 变更摘要
    pub change_summary: Vec<String>,
}

/// 修订来源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevisionSource {
    /// 用户手动修订
    User(String),
    /// 系统自动修订
    System,
    /// AI建议修订
    AI,
    /// 监控触发修订
    Monitor,
}

// 实现默认值和辅助方法
impl Default for ExecutionPlan {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "Untitled Plan".to_string(),
            description: String::new(),
            steps: Vec::new(),
            estimated_duration: 0,
            created_at: SystemTime::now(),
            metadata: PlanMetadata::default(),
            dependencies: HashMap::new(),
        }
    }
}

impl Default for PlanMetadata {
    fn default() -> Self {
        Self {
            plan_type: PlanType::Custom("default".to_string()),
            priority: Priority::Medium,
            complexity: Complexity::Medium,
            risk_level: RiskLevel::Medium,
            tags: Vec::new(),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_interval: 5,
            backoff_strategy: BackoffStrategy::Exponential,
            retry_conditions: vec![
                RetryCondition::NetworkError,
                RetryCondition::Timeout,
            ],
        }
    }
}

// 确保关键类型是Send + Sync的
unsafe impl Send for ExecutionSession {}
unsafe impl Sync for ExecutionSession {}
unsafe impl Send for ExecutionPlan {}
unsafe impl Sync for ExecutionPlan {}
unsafe impl Send for PlanStep {}
unsafe impl Sync for PlanStep {}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self {
            execution_time_ms: 0,
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
            network_io_bytes: 0,
            disk_io_bytes: 0,
            custom_metrics: HashMap::new(),
        }
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            user_id: None,
            target_info: TargetInfo {
                target_type: TargetType::Host,
                address: "localhost".to_string(),
                target_value: "localhost".to_string(),
                port_range: None,
                protocols: vec!["tcp".to_string()],
                credentials: None,
                metadata: HashMap::new(),
            },
            environment: HashMap::new(),
            shared_data: HashMap::new(),
            config: HashMap::new(),
        }
    }
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self {
            created_by: None,
            tags: Vec::new(),
            notes: None,
            version: "1.0".to_string(),
            custom_attributes: HashMap::new(),
        }
    }
}

impl Default for PlanningConstraints {
    fn default() -> Self {
        Self {
            max_execution_time: Some(3600), // 1小时
            max_steps: Some(100),
            resource_limits: ResourceLimits::default(),
            allowed_tools: None,
            forbidden_tools: Vec::new(),
            concurrency_limit: Some(10),
            custom_constraints: HashMap::new(),
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(1024 * 1024 * 1024), // 1GB
            max_cpu_percent: Some(80.0),
            max_network_bps: Some(100 * 1024 * 1024), // 100MB/s
            max_disk_bytes: Some(10 * 1024 * 1024 * 1024), // 10GB
        }
    }
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self {
            status: ExecutionStatus::Pending,
            description: "等待执行".to_string(),
            timestamp: SystemTime::now(),
            data: HashMap::new(),
            next_possible_states: vec![ExecutionStatus::Running, ExecutionStatus::Cancelled],
        }
    }
}

// 辅助函数
impl ExecutionPlan {
    /// 创建新的执行计划
    pub fn new(name: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            ..Default::default()
        }
    }
    
    /// 添加步骤
    pub fn add_step(&mut self, step: PlanStep) {
        self.steps.push(step);
    }
    
    /// 验证计划的有效性
    pub fn validate(&self) -> Result<(), String> {
        if self.steps.is_empty() {
            return Err("计划必须包含至少一个步骤".to_string());
        }
        
        // 验证依赖关系
        for (step_id, deps) in &self.dependencies {
            if !self.steps.iter().any(|s| s.id == *step_id) {
                return Err(format!("依赖关系中的步骤ID {} 不存在", step_id));
            }
            
            for dep_id in deps {
                if !self.steps.iter().any(|s| s.id == *dep_id) {
                    return Err(format!("依赖的步骤ID {} 不存在", dep_id));
                }
            }
        }
        
        Ok(())
    }
}

impl PlanStep {
    /// 创建新的计划步骤
    pub fn new(name: String, description: String, step_type: StepType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            step_type,
            tool_config: ToolConfig {
                tool_name: String::new(),
                tool_version: None,
                tool_args: HashMap::new(),
                timeout: None,
                env_vars: HashMap::new(),
            },
            estimated_duration: 60, // 默认1分钟
            retry_config: RetryConfig::default(),
            parameters: HashMap::new(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
        }
    }
}

impl ExecutionSession {
    /// 创建新的执行会话
    pub fn new(plan_id: String, context: ExecutionContext) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            plan_id,
            status: ExecutionStatus::Pending,
            started_at: SystemTime::now(),
            completed_at: None,
            current_step: None,
            progress: 0.0,
            context,
            step_results: HashMap::new(),
            metadata: SessionMetadata {
                created_by: None,
                tags: Vec::new(),
                notes: None,
                version: "1.0".to_string(),
                custom_attributes: HashMap::new(),
            },
        }
    }
    
    /// 更新执行进度
    pub fn update_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 100.0);
    }
    
    /// 标记会话完成
    pub fn mark_completed(&mut self) {
        self.status = ExecutionStatus::Completed;
        self.completed_at = Some(SystemTime::now());
        self.progress = 100.0;
    }
    
    /// 标记会话失败
    pub fn mark_failed(&mut self, error: ExecutionError) {
        self.status = ExecutionStatus::Failed;
        self.completed_at = Some(SystemTime::now());
        
        // 将错误信息添加到当前步骤的结果中
        if let Some(current_step) = &self.current_step {
            let step_key = current_step.to_string();
            if let Some(result) = self.step_results.get_mut(&step_key) {
                result.error = Some(error);
                result.status = StepExecutionStatus::Failed;
            }
        }
    }
}
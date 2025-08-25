//! 智能调度器相关类型定义

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use super::workflow_engine::WorkflowDefinition;

/// 调度决策
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchDecision {
    /// 选择的架构
    pub architecture: AgentArchitecture,
    /// 任务类型
    pub task_type: TaskType,
    /// 复杂度
    pub complexity: TaskComplexity,
    /// 选择理由
    pub reasoning: String,
    /// 置信度
    pub confidence: f32,
    /// 预估执行时长（秒）
    pub estimated_duration: Option<u64>,
    /// 建议的工作流
    pub suggested_workflow: Option<WorkflowDefinition>,
}

/// 调度结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchResult {
    /// 请求ID
    pub request_id: String,
    /// 执行ID
    pub execution_id: String,
    /// 调度决策
    pub decision: DispatchDecision,
    /// 工作流状态
    pub status: WorkflowStatus,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 执行结果
    pub result: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
}

/// Agent架构类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentArchitecture {
    /// Plan-and-Execute架构
    PlanAndExecute,
    /// ReWOO架构
    ReWoo,
    /// LLMCompiler架构
    LlmCompiler,
    /// 智能调度器
    IntelligentDispatcher,
}

/// 任务类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    /// 扫描任务
    Scanning,
    /// 分析任务
    Analysis,
    /// 查询任务
    Query,
    /// 配置任务
    Configuration,
    /// 监控任务
    Monitoring,
    /// 其他任务
    Other,
}

/// 任务复杂度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskComplexity {
    /// 简单任务
    Simple,
    /// 中等复杂度
    Medium,
    /// 复杂任务
    Complex,
}

/// 工作流状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 已取消
    Cancelled,
    /// 暂停
    Paused,
}

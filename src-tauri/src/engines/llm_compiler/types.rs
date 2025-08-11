//! LLMCompiler类型定义
//!
//! 包含LLMCompiler架构中使用的所有数据结构和类型定义

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// LLMCompiler配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCompilerConfig {
    /// 最大并发数
    pub max_concurrency: usize,
    /// 任务超时时间（秒）
    pub task_timeout: u64,
    /// 是否启用动态重规划
    pub enable_replanning: bool,
    /// 是否启用性能监控
    pub enable_monitoring: bool,
    /// Joiner决策阈值
    pub joiner_threshold: f32,
    /// 最大重规划次数
    pub max_replanning_iterations: usize,
    /// 任务重试次数
    pub max_task_retries: u32,
    /// 最大执行轮次
    pub max_iterations: usize,
}

impl Default for LlmCompilerConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 10,
            task_timeout: 300,
            enable_replanning: true,
            enable_monitoring: true,
            joiner_threshold: 0.8,
            max_replanning_iterations: 5,
            max_task_retries: 3,
            max_iterations: 10,
        }
    }
}

/// 任务状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    /// 等待依赖
    Pending,
    /// 就绪（可执行）
    Ready,
    /// 正在执行
    Running,
    /// 执行成功
    Completed,
    /// 执行失败
    Failed,
    /// 已取消
    Cancelled,
    /// 重试中
    Retrying,
}

impl TaskStatus {
    /// 是否为终态
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled)
    }

    /// 是否为成功状态
    pub fn is_successful(&self) -> bool {
        matches!(self, TaskStatus::Completed)
    }

    /// 是否为失败状态
    pub fn is_failed(&self) -> bool {
        matches!(self, TaskStatus::Failed)
    }
}

/// DAG任务节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagTaskNode {
    /// 任务ID
    pub id: String,
    /// 任务名称
    pub name: String,
    /// 任务描述
    pub description: String,
    /// 工具名称
    pub tool_name: String,
    /// 输入参数
    pub inputs: HashMap<String, Value>,
    /// 依赖的任务ID列表
    pub dependencies: Vec<String>,
    /// 变量引用（如$1, $2等）
    pub variable_refs: Vec<String>,
    /// 任务状态
    pub status: TaskStatus,
    /// 优先级（数字越小优先级越高）
    pub priority: i32,
    /// 预估执行时间（秒）
    pub estimated_duration: Option<u64>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最大重试次数
    pub max_retries: u32,
    /// 当前重试次数
    pub retry_count: u32,
    /// 任务标签（用于分类和过滤）
    pub tags: Vec<String>,
}

impl DagTaskNode {
    /// 创建新的任务节点
    pub fn new(
        id: String,
        name: String,
        tool_name: String,
        inputs: HashMap<String, Value>,
    ) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            tool_name,
            inputs,
            dependencies: Vec::new(),
            variable_refs: Vec::new(),
            status: TaskStatus::Pending,
            priority: 1,
            estimated_duration: None,
            created_at: Utc::now(),
            max_retries: 3,
            retry_count: 0,
            tags: Vec::new(),
        }
    }

    /// 是否可以重试
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries && self.status == TaskStatus::Failed
    }

    /// 增加重试次数
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
        self.status = TaskStatus::Retrying;
    }

    /// 检查是否有指定标签
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.to_string())
    }

    /// 添加标签
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }
}

/// DAG执行计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagExecutionPlan {
    /// 计划ID
    pub id: String,
    /// 计划名称
    pub name: String,
    /// 任务节点列表
    pub nodes: Vec<DagTaskNode>,
    /// 依赖关系图 (task_id -> [dependency_ids])
    pub dependency_graph: HashMap<String, Vec<String>>,
    /// 变量映射 (variable_ref -> task_output_path)
    pub variable_mappings: HashMap<String, String>,
    /// 全局配置
    pub global_config: HashMap<String, Value>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 计划版本（用于重规划）
    pub version: u32,
    /// 父计划ID（重规划时指向原计划）
    pub parent_plan_id: Option<String>,
}

impl DagExecutionPlan {
    /// 获取所有根任务（无依赖的任务）
    pub fn get_root_tasks(&self) -> Vec<&DagTaskNode> {
        self.nodes.iter()
            .filter(|node| node.dependencies.is_empty())
            .collect()
    }

    /// 获取指定任务的直接依赖
    pub fn get_task_dependencies(&self, task_id: &str) -> Vec<&DagTaskNode> {
        if let Some(deps) = self.dependency_graph.get(task_id) {
            deps.iter()
                .filter_map(|dep_id| self.nodes.iter().find(|n| &n.id == dep_id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 获取依赖指定任务的任务列表
    pub fn get_dependent_tasks(&self, task_id: &str) -> Vec<&DagTaskNode> {
        self.dependency_graph.iter()
            .filter(|(_, deps)| deps.contains(&task_id.to_string()))
            .filter_map(|(dependent_id, _)| {
                self.nodes.iter().find(|n| &n.id == dependent_id)
            })
            .collect()
    }

    /// 获取任务总数
    pub fn task_count(&self) -> usize {
        self.nodes.len()
    }

    /// 获取依赖关系总数
    pub fn dependency_count(&self) -> usize {
        self.dependency_graph.values().map(|deps| deps.len()).sum()
    }
}

/// 任务执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionResult {
    /// 任务ID
    pub task_id: String,
    /// 执行状态
    pub status: TaskStatus,
    /// 输出数据
    pub outputs: HashMap<String, Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 执行时间（毫秒）
    pub duration_ms: u64,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 结束时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 重试次数
    pub retry_count: u32,
    /// 执行元数据
    pub metadata: HashMap<String, Value>,
}

impl TaskExecutionResult {
    /// 创建成功的执行结果
    pub fn success(
        task_id: String,
        outputs: HashMap<String, Value>,
        duration_ms: u64,
    ) -> Self {
        Self {
            task_id,
            status: TaskStatus::Completed,
            outputs,
            error: None,
            duration_ms,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            retry_count: 0,
            metadata: HashMap::new(),
        }
    }

    /// 创建失败的执行结果
    pub fn failure(
        task_id: String,
        error: String,
        duration_ms: u64,
        retry_count: u32,
    ) -> Self {
        Self {
            task_id,
            status: TaskStatus::Failed,
            outputs: HashMap::new(),
            error: Some(error),
            duration_ms,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            retry_count,
            metadata: HashMap::new(),
        }
    }

    /// 是否执行成功
    pub fn is_success(&self) -> bool {
        self.status == TaskStatus::Completed
    }

    /// 是否执行失败
    pub fn is_failure(&self) -> bool {
        self.status == TaskStatus::Failed
    }

    /// 获取指定输出值
    pub fn get_output(&self, key: &str) -> Option<&Value> {
        self.outputs.get(key)
    }

    /// 添加元数据
    pub fn add_metadata(&mut self, key: String, value: Value) {
        self.metadata.insert(key, value);
    }
}

/// Joiner决策类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinerDecision {
    /// 继续执行（需要更多信息）
    Continue {
        /// 反馈信息
        feedback: String,
        /// 建议的新任务
        suggested_tasks: Vec<DagTaskNode>,
        /// 置信度
        confidence: f32,
    },
    /// 完成执行（给出最终答案）
    Complete {
        /// 最终响应
        response: String,
        /// 置信度
        confidence: f32,
        /// 执行摘要
        summary: ExecutionSummary,
    },
}

impl JoinerDecision {
    /// 获取置信度
    pub fn confidence(&self) -> f32 {
        match self {
            JoinerDecision::Continue { confidence, .. } => *confidence,
            JoinerDecision::Complete { confidence, .. } => *confidence,
        }
    }

    /// 是否为完成决策
    pub fn is_complete(&self) -> bool {
        matches!(self, JoinerDecision::Complete { .. })
    }

    /// 是否为继续决策
    pub fn is_continue(&self) -> bool {
        matches!(self, JoinerDecision::Continue { .. })
    }
}

/// 执行摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    /// 总任务数
    pub total_tasks: usize,
    /// 成功任务数
    pub successful_tasks: usize,
    /// 失败任务数
    pub failed_tasks: usize,
    /// 总执行时间（毫秒）
    pub total_duration_ms: u64,
    /// 重规划次数
    pub replanning_count: usize,
    /// 发现的关键信息
    pub key_findings: Vec<String>,
    /// 执行效率指标
    pub efficiency_metrics: EfficiencyMetrics,
}

/// 效率指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfficiencyMetrics {
    /// 并行度（平均同时执行的任务数）
    pub average_parallelism: f32,
    /// 资源利用率
    pub resource_utilization: f32,
    /// 任务成功率
    pub task_success_rate: f32,
    /// 平均任务执行时间（毫秒）
    pub average_task_duration_ms: f32,
}

/// 执行统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    /// 等待任务数
    pub waiting_tasks: usize,
    /// 就绪任务数
    pub ready_tasks: usize,
    /// 执行中任务数
    pub executing_tasks: usize,
    /// 已完成任务数
    pub completed_tasks: usize,
    /// 失败任务数
    pub failed_tasks: usize,
    /// 总任务数
    pub total_tasks: usize,
}

impl ExecutionStats {
    /// 计算完成率
    pub fn completion_rate(&self) -> f32 {
        if self.total_tasks == 0 {
            0.0
        } else {
            self.completed_tasks as f32 / self.total_tasks as f32
        }
    }

    /// 计算成功率
    pub fn success_rate(&self) -> f32 {
        let finished_tasks = self.completed_tasks + self.failed_tasks;
        if finished_tasks == 0 {
            0.0
        } else {
            self.completed_tasks as f32 / finished_tasks as f32
        }
    }

    /// 是否全部完成
    pub fn is_all_completed(&self) -> bool {
        self.waiting_tasks == 0 && self.ready_tasks == 0 && self.executing_tasks == 0
    }
}

/// 任务执行事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskExecutionEvent {
    /// 任务开始
    TaskStarted {
        task_id: String,
        timestamp: DateTime<Utc>,
    },
    /// 任务完成
    TaskCompleted {
        task_id: String,
        result: TaskExecutionResult,
        timestamp: DateTime<Utc>,
    },
    /// 任务失败
    TaskFailed {
        task_id: String,
        error: String,
        retry_count: u32,
        timestamp: DateTime<Utc>,
    },
    /// 依赖满足
    DependencySatisfied {
        task_id: String,
        dependency_id: String,
        timestamp: DateTime<Utc>,
    },
    /// 计划重规划
    PlanReplanned {
        old_plan_id: String,
        new_plan_id: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },
}

/// 变量解析上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableResolutionContext {
    /// 已完成任务的结果
    pub completed_results: HashMap<String, TaskExecutionResult>,
    /// 变量映射
    pub variable_mappings: HashMap<String, String>,
    /// 全局变量
    pub global_variables: HashMap<String, Value>,
}

impl VariableResolutionContext {
    /// 创建新的变量解析上下文
    pub fn new() -> Self {
        Self {
            completed_results: HashMap::new(),
            variable_mappings: HashMap::new(),
            global_variables: HashMap::new(),
        }
    }

    /// 添加任务结果
    pub fn add_task_result(&mut self, result: TaskExecutionResult) {
        self.completed_results.insert(result.task_id.clone(), result);
    }

    /// 解析变量引用
    pub fn resolve_variable(&self, var_ref: &str) -> Option<Value> {
        // 首先检查全局变量
        if let Some(value) = self.global_variables.get(var_ref) {
            return Some(value.clone());
        }

        // 然后检查变量映射
        if let Some(mapping) = self.variable_mappings.get(var_ref) {
            return self.resolve_mapping_path(mapping);
        }

        None
    }

    /// 解析映射路径（如 "task_1.outputs.ip_address"）
    fn resolve_mapping_path(&self, path: &str) -> Option<Value> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.len() < 3 {
            return None;
        }

        let task_id = parts[0];
        let section = parts[1]; // 通常是 "outputs"
        let key = parts[2];

        if let Some(result) = self.completed_results.get(task_id) {
            match section {
                "outputs" => result.outputs.get(key).cloned(),
                "metadata" => result.metadata.get(key).cloned(),
                _ => None,
            }
        } else {
            None
        }
    }
}

impl Default for VariableResolutionContext {
    fn default() -> Self {
        Self::new()
    }
}
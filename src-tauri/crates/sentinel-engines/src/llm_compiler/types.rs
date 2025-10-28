//! LLMCompiler类型定义
//!
//! 包含LLMCompiler架构中使用的所有数据结构和类型定义

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use std::str::FromStr;

/// 失败传播策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FailurePropagationStrategy {
    /// 快速失败：依赖失败的任务立即取消
    FailFast,
    /// 尽力而为：跳过失败的输入，尝试用其他输入执行
    BestEffort,
    /// 回退策略：尝试使用替代工具或默认值
    Fallback,
    /// 继续执行：忽略失败，继续执行其他任务
    Continue,
}

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
    /// 失败传播策略
    pub failure_strategy: FailurePropagationStrategy,
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
            failure_strategy: FailurePropagationStrategy::FailFast,
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

/// 工具参数 Schema 定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolParameterSchema {
    /// 参数名
    pub name: String,
    /// 参数类型
    pub param_type: ParameterType,
    /// 是否必需
    pub required: bool,
    /// 参数描述
    pub description: Option<String>,
    /// 默认值
    pub default_value: Option<Value>,
    /// 值约束
    pub constraints: Option<ParameterConstraints>,
}

/// 参数类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
    Array(Box<ParameterType>),
    Object(HashMap<String, ToolParameterSchema>),
    Enum(Vec<String>),
}

/// 参数约束
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ParameterConstraints {
    /// 最小值（数字类型）
    pub min_value: Option<f64>,
    /// 最大值（数字类型）
    pub max_value: Option<f64>,
    /// 最小长度（字符串/数组类型）
    pub min_length: Option<usize>,
    /// 最大长度（字符串/数组类型）
    pub max_length: Option<usize>,
    /// 正则表达式模式（字符串类型）
    pub pattern: Option<String>,
    /// 枚举值（枚举类型）
    pub enum_values: Option<Vec<String>>,
}

/// 工具 Schema 定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolSchema {
    /// 工具名
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 参数定义
    pub parameters: Vec<ToolParameterSchema>,
    /// 输出 Schema
    pub output_schema: Option<Value>,
}

/// Schema 验证结果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// 是否有效
    pub is_valid: bool,
    /// 验证错误
    pub errors: Vec<String>,
    /// 修正后的参数（如果可以自动修正）
    pub corrected_params: Option<HashMap<String, Value>>,
}

/// 缓存键（用于工具调用去重）
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    /// 工具名
    pub tool_name: String,
    /// 标准化的参数（按键排序）
    pub normalized_params: String,
}

/// 缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// 缓存键
    pub key: CacheKey,
    /// 缓存的执行结果
    pub result: TaskExecutionResult,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后访问时间
    pub last_accessed: DateTime<Utc>,
    /// 访问次数
    pub access_count: u32,
    /// 生存时间（秒）
    pub ttl_seconds: Option<u64>,
}

/// 工具调用缓存管理器
#[derive(Debug, Clone)]
pub struct ToolCallCache {
    /// 缓存条目
    cache: HashMap<CacheKey, CacheEntry>,
    /// 最大缓存大小
    max_size: usize,
    /// 默认TTL（秒）
    default_ttl: u64,
    /// 缓存统计
    stats: CacheStats,
}

/// 缓存统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// 命中次数
    pub hits: u64,
    /// 未命中次数
    pub misses: u64,
    /// 驱逐次数
    pub evictions: u64,
    /// 过期清理次数
    pub expirations: u64,
}

impl ToolCallCache {
    /// 创建新的缓存管理器
    pub fn new(max_size: usize, default_ttl: u64) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            default_ttl,
            stats: CacheStats::default(),
        }
    }

    /// 生成缓存键
    pub fn generate_cache_key(&self, tool_name: &str, params: &HashMap<String, Value>) -> CacheKey {
        // 标准化参数：按键排序并序列化
        let mut sorted_params: Vec<_> = params.iter().collect();
        sorted_params.sort_by_key(|(k, _)| *k);
        
        let normalized_params = serde_json::to_string(&sorted_params)
            .unwrap_or_else(|_| "{}".to_string());

        CacheKey {
            tool_name: tool_name.to_string(),
            normalized_params,
        }
    }

    /// 检查缓存
    pub fn get(&mut self, key: &CacheKey) -> Option<TaskExecutionResult> {
        // 先检查是否存在和是否过期
        let should_remove = if let Some(entry) = self.cache.get(key) {
            self.is_expired(entry)
        } else {
            false
        };

        if should_remove {
            self.cache.remove(key);
            self.stats.expirations += 1;
            self.stats.misses += 1;
            return None;
        }

        if let Some(entry) = self.cache.get_mut(key) {
            // 更新访问信息
            entry.last_accessed = Utc::now();
            entry.access_count += 1;
            self.stats.hits += 1;
            
            Some(entry.result.clone())
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// 添加到缓存
    pub fn put(&mut self, key: CacheKey, result: TaskExecutionResult, ttl: Option<u64>) {
        // 如果缓存已满，执行LRU驱逐
        if self.cache.len() >= self.max_size {
            self.evict_lru();
        }

        let entry = CacheEntry {
            key: key.clone(),
            result,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            ttl_seconds: ttl.or(Some(self.default_ttl)),
        };

        self.cache.insert(key, entry);
    }

    /// 检查缓存条目是否过期
    fn is_expired(&self, entry: &CacheEntry) -> bool {
        if let Some(ttl) = entry.ttl_seconds {
            let elapsed = Utc::now().signed_duration_since(entry.created_at);
            elapsed.num_seconds() > ttl as i64
        } else {
            false // 无TTL，永不过期
        }
    }

    /// LRU驱逐策略
    fn evict_lru(&mut self) {
        if self.cache.is_empty() {
            return;
        }

        // 找到最久未访问的条目
        let mut oldest_key = None;
        let mut oldest_time = Utc::now();

        for (key, entry) in &self.cache {
            if entry.last_accessed < oldest_time {
                oldest_time = entry.last_accessed;
                oldest_key = Some(key.clone());
            }
        }

        if let Some(key) = oldest_key {
            self.cache.remove(&key);
            self.stats.evictions += 1;
        }
    }

    /// 清理过期条目
    pub fn cleanup_expired(&mut self) {
        let expired_keys: Vec<_> = self.cache
            .iter()
            .filter(|(_, entry)| self.is_expired(entry))
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            self.cache.remove(&key);
            self.stats.expirations += 1;
        }
    }

    /// 获取缓存统计
    pub fn get_stats(&self) -> &CacheStats {
        &self.stats
    }

    /// 获取缓存命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.stats.hits + self.stats.misses;
        if total > 0 {
            self.stats.hits as f64 / total as f64
        } else {
            0.0
        }
    }

    /// 清空缓存
    pub fn clear(&mut self) {
        self.cache.clear();
        self.stats = CacheStats::default();
    }

    /// 获取缓存大小
    pub fn size(&self) -> usize {
        self.cache.len()
    }

    /// 检查是否值得缓存（基于工具类型和参数）
    pub fn should_cache(&self, tool_name: &str, _params: &HashMap<String, Value>) -> bool {
        // 某些工具的结果更适合缓存
        match tool_name {
            // 网络扫描类工具，结果相对稳定
            "dns_scanner" | "port_scanner" | "port_scan" => true,
            // 子域名枚举，结果可能变化但短期内稳定
            "rsubdomain" => true,
            // 一般工具也可以缓存，但TTL较短
            _ => true,
        }
    }

    /// 获取工具特定的TTL
    pub fn get_tool_ttl(&self, tool_name: &str) -> u64 {
        match tool_name {
            // 网络基础信息相对稳定，较长TTL
            "dns_scanner" => 3600, // 1小时
            // 端口扫描结果相对稳定
            "port_scanner" | "port_scan" => 1800, // 30分钟
            // 子域名可能变化
            "rsubdomain" => 900, // 15分钟
            // 默认TTL
            _ => self.default_ttl,
        }
    }
}

/// 路径部分（用于变量解析）
#[derive(Debug, Clone)]
struct PathPart {
    /// 字段名
    name: String,
    /// 数组索引（如果有）
    index: Option<usize>,
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
    /// 支持多种格式：
    /// - $1, $2 等简单引用
    /// - task_id.outputs.field 等点分路径
    /// - /task_id/outputs/field 等 JSON Pointer 风格
    /// - task_id.outputs[0].field 等数组索引
    pub fn resolve_variable(&self, var_ref: &str) -> Option<Value> {
        // 首先检查全局变量
        if let Some(value) = self.global_variables.get(var_ref) {
            return Some(value.clone());
        }

        // 检查变量映射
        if let Some(mapping) = self.variable_mappings.get(var_ref) {
            return self.resolve_mapping_path(mapping);
        }

        // 直接解析路径（支持多种格式）
        self.resolve_path_directly(var_ref)
    }

    /// 解析映射路径（支持多种格式）
    fn resolve_mapping_path(&self, path: &str) -> Option<Value> {
        self.resolve_path_directly(path)
    }

    /// 直接解析路径，支持多种格式
    fn resolve_path_directly(&self, path: &str) -> Option<Value> {
        // 处理 JSON Pointer 格式 (/task_id/outputs/field)
        if path.starts_with('/') {
            return self.resolve_json_pointer(path);
        }

        // 处理点分路径格式 (task_id.outputs.field 或 task_id.outputs[0].field)
        self.resolve_dot_notation(path)
    }

    /// 解析 JSON Pointer 格式路径
    fn resolve_json_pointer(&self, pointer: &str) -> Option<Value> {
        let parts: Vec<&str> = pointer.trim_start_matches('/').split('/').collect();
        if parts.is_empty() {
            return None;
        }

        let task_id = parts[0];
        if let Some(result) = self.completed_results.get(task_id) {
            let mut current_value = serde_json::to_value(result).ok()?;
            
            // 遍历剩余路径部分
            for part in &parts[1..] {
                current_value = self.navigate_value(current_value, part)?;
            }
            
            Some(current_value)
        } else {
            None
        }
    }

    /// 解析点分路径格式
    fn resolve_dot_notation(&self, path: &str) -> Option<Value> {
        // 分割路径，但保留数组索引
        let path_parts = self.parse_dot_notation(path);
        if path_parts.is_empty() {
            return None;
        }

        let task_id = &path_parts[0].name;
        if let Some(result) = self.completed_results.get(task_id) {
            let mut current_value = serde_json::to_value(result).ok()?;
            
            // 遍历路径部分
            for part in &path_parts[1..] {
                current_value = self.navigate_value_with_index(current_value, &part.name, part.index)?;
            }
            
            Some(current_value)
        } else {
            None
        }
    }

    /// 解析点分路径为结构化部分
    fn parse_dot_notation(&self, path: &str) -> Vec<PathPart> {
        let mut parts = Vec::new();
        let segments: Vec<&str> = path.split('.').collect();
        
        for segment in segments {
            if let Some(bracket_start) = segment.find('[') {
                if let Some(bracket_end) = segment.find(']') {
                    // 包含数组索引: field[0]
                    let field_name = &segment[..bracket_start];
                    let index_str = &segment[bracket_start + 1..bracket_end];
                    
                    if let Ok(index) = usize::from_str(index_str) {
                        parts.push(PathPart {
                            name: field_name.to_string(),
                            index: Some(index),
                        });
                    } else {
                        // 索引解析失败，作为普通字段处理
                        parts.push(PathPart {
                            name: segment.to_string(),
                            index: None,
                        });
                    }
                } else {
                    // 括号不匹配，作为普通字段处理
                    parts.push(PathPart {
                        name: segment.to_string(),
                        index: None,
                    });
                }
            } else {
                // 普通字段
                parts.push(PathPart {
                    name: segment.to_string(),
                    index: None,
                });
            }
        }
        
        parts
    }

    /// 在值中导航（简单字段）
    fn navigate_value(&self, value: Value, field: &str) -> Option<Value> {
        match value {
            Value::Object(obj) => obj.get(field).cloned(),
            Value::Array(arr) => {
                if let Ok(index) = usize::from_str(field) {
                    arr.get(index).cloned()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 在值中导航（支持数组索引）
    fn navigate_value_with_index(&self, value: Value, field: &str, index: Option<usize>) -> Option<Value> {
        match value {
            Value::Object(obj) => {
                let field_value = obj.get(field).cloned()?;
                if let Some(idx) = index {
                    // 需要进一步索引
                    match field_value {
                        Value::Array(arr) => arr.get(idx).cloned(),
                        _ => None,
                    }
                } else {
                    Some(field_value)
                }
            }
            _ => None,
        }
    }

    /// 验证变量路径格式是否有效
    pub fn validate_variable_path(&self, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }

        // JSON Pointer 格式验证
        if path.starts_with('/') {
            let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
            return !parts.is_empty() && !parts[0].is_empty();
        }

        // 点分路径格式验证
        let path_parts = self.parse_dot_notation(path);
        !path_parts.is_empty() && !path_parts[0].name.is_empty()
    }

    /// 获取路径中引用的所有任务ID
    pub fn extract_task_ids_from_path(&self, path: &str) -> Vec<String> {
        let mut task_ids = Vec::new();

        if path.starts_with('/') {
            // JSON Pointer 格式
            let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
            if !parts.is_empty() && !parts[0].is_empty() {
                task_ids.push(parts[0].to_string());
            }
        } else {
            // 点分路径格式
            let path_parts = self.parse_dot_notation(path);
            if !path_parts.is_empty() && !path_parts[0].name.is_empty() {
                task_ids.push(path_parts[0].name.clone());
            }
        }

        task_ids
    }

    /// 检查路径是否可以解析（所需的任务结果是否可用）
    pub fn can_resolve_path(&self, path: &str) -> bool {
        let task_ids = self.extract_task_ids_from_path(path);
        for task_id in task_ids {
            if !self.completed_results.contains_key(&task_id) {
                return false;
            }
        }
        true
    }

    /// 添加变量映射的便捷方法
    pub fn add_variable_mapping(&mut self, var_ref: String, path: String) {
        if self.validate_variable_path(&path) {
            self.variable_mappings.insert(var_ref, path);
        }
    }

    /// 批量添加变量映射
    pub fn add_variable_mappings(&mut self, mappings: HashMap<String, String>) {
        for (var_ref, path) in mappings {
            self.add_variable_mapping(var_ref, path);
        }
    }

    /// 验证工具参数是否符合 Schema
    pub fn validate_tool_parameters(
        &self,
        tool_schema: &ToolSchema,
        parameters: &HashMap<String, Value>,
    ) -> ValidationResult {
        let mut errors = Vec::new();
        let mut corrected_params = HashMap::new();

        // 检查必需参数
        for param_schema in &tool_schema.parameters {
            if param_schema.required && !parameters.contains_key(&param_schema.name) {
                // 尝试使用默认值
                if let Some(default_value) = &param_schema.default_value {
                    corrected_params.insert(param_schema.name.clone(), default_value.clone());
                } else {
                    errors.push(format!("Missing required parameter: {}", param_schema.name));
                }
            }
        }

        // 验证每个提供的参数
        for (param_name, param_value) in parameters {
            if let Some(param_schema) = tool_schema.parameters.iter().find(|p| p.name == *param_name) {
                // 验证参数类型和约束
                match self.validate_parameter_value(param_schema, param_value) {
                    Ok(corrected_value) => {
                        corrected_params.insert(param_name.clone(), corrected_value);
                    }
                    Err(error) => {
                        errors.push(format!("Parameter '{}': {}", param_name, error));
                    }
                }
            } else {
                // 未知参数 - 可以选择忽略或报错
                errors.push(format!("Unknown parameter: {}", param_name));
            }
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            corrected_params: if corrected_params.is_empty() { None } else { Some(corrected_params) },
        }
    }

    /// 验证单个参数值
    fn validate_parameter_value(
        &self,
        param_schema: &ToolParameterSchema,
        value: &Value,
    ) -> Result<Value, String> {
        // 类型验证
        let validated_value = self.validate_parameter_type(&param_schema.param_type, value)?;

        // 约束验证
        if let Some(constraints) = &param_schema.constraints {
            self.validate_parameter_constraints(constraints, &validated_value, &param_schema.param_type)?;
        }

        Ok(validated_value)
    }

    /// 验证参数类型
    fn validate_parameter_type(&self, param_type: &ParameterType, value: &Value) -> Result<Value, String> {
        match (param_type, value) {
            (ParameterType::String, Value::String(_)) => Ok(value.clone()),
            (ParameterType::String, v) => {
                // 尝试转换为字符串
                match v {
                    Value::Number(n) => Ok(Value::String(n.to_string())),
                    Value::Bool(b) => Ok(Value::String(b.to_string())),
                    _ => Err("Cannot convert to string".to_string()),
                }
            }
            (ParameterType::Integer, Value::Number(n)) => {
                if n.is_i64() {
                    Ok(value.clone())
                } else {
                    Err("Not a valid integer".to_string())
                }
            }
            (ParameterType::Integer, Value::String(s)) => {
                match s.parse::<i64>() {
                    Ok(i) => Ok(Value::Number(serde_json::Number::from(i))),
                    Err(_) => Err("Cannot parse as integer".to_string()),
                }
            }
            (ParameterType::Float, Value::Number(_)) => Ok(value.clone()),
            (ParameterType::Float, Value::String(s)) => {
                match s.parse::<f64>() {
                    Ok(f) => Ok(Value::Number(serde_json::Number::from_f64(f)
                        .ok_or("Invalid float value")?)),
                    Err(_) => Err("Cannot parse as float".to_string()),
                }
            }
            (ParameterType::Boolean, Value::Bool(_)) => Ok(value.clone()),
            (ParameterType::Boolean, Value::String(s)) => {
                match s.to_lowercase().as_str() {
                    "true" | "1" | "yes" => Ok(Value::Bool(true)),
                    "false" | "0" | "no" => Ok(Value::Bool(false)),
                    _ => Err("Cannot parse as boolean".to_string()),
                }
            }
            (ParameterType::Array(inner_type), Value::Array(arr)) => {
                let mut validated_array = Vec::new();
                for (i, item) in arr.iter().enumerate() {
                    match self.validate_parameter_type(inner_type, item) {
                        Ok(validated_item) => validated_array.push(validated_item),
                        Err(e) => return Err(format!("Array item [{}]: {}", i, e)),
                    }
                }
                Ok(Value::Array(validated_array))
            }
            (ParameterType::Enum(allowed_values), Value::String(s)) => {
                if allowed_values.contains(s) {
                    Ok(value.clone())
                } else {
                    Err(format!("Value '{}' not in allowed enum values: {:?}", s, allowed_values))
                }
            }
            _ => Err(format!("Type mismatch: expected {:?}", param_type)),
        }
    }

    /// 验证参数约束
    fn validate_parameter_constraints(
        &self,
        constraints: &ParameterConstraints,
        value: &Value,
        param_type: &ParameterType,
    ) -> Result<(), String> {
        match param_type {
            ParameterType::String => {
                if let Value::String(s) = value {
                    if let Some(min_len) = constraints.min_length {
                        if s.len() < min_len {
                            return Err(format!("String too short (min: {})", min_len));
                        }
                    }
                    if let Some(max_len) = constraints.max_length {
                        if s.len() > max_len {
                            return Err(format!("String too long (max: {})", max_len));
                        }
                    }
                    if let Some(pattern) = &constraints.pattern {
                        if !self.matches_pattern(s, pattern) {
                            return Err(format!("String does not match pattern: {}", pattern));
                        }
                    }
                }
            }
            ParameterType::Integer | ParameterType::Float => {
                if let Value::Number(n) = value {
                    if let Some(n_f64) = n.as_f64() {
                        if let Some(min_val) = constraints.min_value {
                            if n_f64 < min_val {
                                return Err(format!("Value too small (min: {})", min_val));
                            }
                        }
                        if let Some(max_val) = constraints.max_value {
                            if n_f64 > max_val {
                                return Err(format!("Value too large (max: {})", max_val));
                            }
                        }
                    }
                }
            }
            ParameterType::Array(_) => {
                if let Value::Array(arr) = value {
                    if let Some(min_len) = constraints.min_length {
                        if arr.len() < min_len {
                            return Err(format!("Array too short (min: {})", min_len));
                        }
                    }
                    if let Some(max_len) = constraints.max_length {
                        if arr.len() > max_len {
                            return Err(format!("Array too long (max: {})", max_len));
                        }
                    }
                }
            }
            _ => {} // 其他类型暂不处理约束
        }

        Ok(())
    }

    /// 简单的模式匹配（不使用正则表达式库）
    fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        // 简化的模式匹配，支持基本的通配符
        if pattern == "*" {
            return true;
        }
        
        // 更复杂的正则表达式匹配需要外部库
        // 这里只做简单的前缀、后缀和包含匹配
        if pattern.starts_with('*') && pattern.ends_with('*') {
            let middle = &pattern[1..pattern.len()-1];
            text.contains(middle)
        } else if pattern.starts_with('*') {
            let suffix = &pattern[1..];
            text.ends_with(suffix)
        } else if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len()-1];
            text.starts_with(prefix)
        } else {
            text == pattern
        }
    }
}

impl Default for VariableResolutionContext {
    fn default() -> Self {
        Self::new()
    }
}
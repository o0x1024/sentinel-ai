//! Travel架构核心类型定义
//!
//! 基于OODA(Observe-Orient-Decide-Act)循环的安全测试Agent架构

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

/// Travel引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TravelConfig {
    /// 最大OODA循环次数
    pub max_ooda_cycles: u32,
    /// 单个循环最大执行时间(秒)
    pub max_cycle_duration: Option<f64>,
    /// 是否启用护栏检查
    pub enable_guardrails: bool,
    /// 护栏配置
    pub guardrail_config: GuardrailConfig,
    /// 是否启用威胁情报
    pub enable_threat_intel: bool,
    /// 威胁情报配置
    pub threat_intel_config: ThreatIntelConfig,
    /// 任务复杂度判断配置
    pub complexity_config: ComplexityConfig,
    /// 是否启用详细日志
    pub verbose: bool,
    /// 错误回退策略
    pub rollback_strategy: RollbackStrategy,
    /// 精简模式配置 (Token优化)
    pub lite_mode: LiteModeConfig,
    /// 并行执行配置
    pub parallel_config: ParallelExecutionConfig,
    /// 上下文管理配置
    pub context_config: ContextManagerConfig,
    /// 是否启用多模态模式（截图）
    /// true: 多模态模式，VisionExplorer 发送截图给 VLM
    /// false: 文本模式，发送元素列表给 LLM
    #[serde(default = "default_true")]
    pub enable_multimodal: bool,
}

/// 返回 true 的辅助函数 (用于 serde default)
fn default_true() -> bool {
    true
}

/// 精简模式配置 - 用于节省Token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteModeConfig {
    /// 是否启用精简模式
    pub enabled: bool,
    /// 精简模式适用的复杂度级别
    pub applicable_complexity: Vec<TaskComplexity>,
    /// 精简模式最大步骤数
    pub max_steps: u32,
    /// 是否跳过OODA循环(直接DAG执行)
    pub skip_ooda_for_simple: bool,
    /// 是否启用规划缓存
    pub enable_plan_cache: bool,
    /// 规划缓存TTL(秒)
    pub plan_cache_ttl: u64,
}

/// 并行执行配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelExecutionConfig {
    /// 是否启用并行执行
    pub enabled: bool,
    /// 最大并发任务数
    pub max_concurrency: usize,
    /// 任务超时(秒)
    pub task_timeout: u64,
    /// 是否启用资源追踪
    pub enable_resource_tracking: bool,
}

/// 上下文管理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextManagerConfig {
    /// 是否启用上下文压缩
    pub enable_compression: bool,
    /// 最大上下文Token数
    pub max_context_tokens: usize,
    /// 历史记录最大条数
    pub max_history_entries: usize,
    /// 工具结果最大长度
    pub max_tool_result_length: usize,
    /// 保留字段白名单(不压缩)
    pub preserve_fields: Vec<String>,
}

/// 护栏配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailConfig {
    /// Observe阶段检查规则
    pub observe_rules: Vec<GuardrailRule>,
    /// Orient阶段检查规则
    pub orient_rules: Vec<GuardrailRule>,
    /// Decide阶段检查规则
    pub decide_rules: Vec<GuardrailRule>,
    /// Act阶段检查规则
    pub act_rules: Vec<GuardrailRule>,
    /// 严格模式(任何违规立即停止)
    pub strict_mode: bool,
}

/// 护栏规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailRule {
    /// 规则ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 规则描述
    pub description: String,
    /// 规则类型
    pub rule_type: GuardrailRuleType,
    /// 严重程度
    pub severity: GuardrailSeverity,
    /// 是否启用
    pub enabled: bool,
}

/// 护栏规则类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuardrailRuleType {
    /// 目标合法性检查
    TargetLegality,
    /// 授权验证
    Authorization,
    /// 漏洞利用风险
    ExploitRisk,
    /// Payload安全性
    PayloadSafety,
    /// 操作风险
    OperationRisk,
    /// 资源限制
    ResourceLimit,
    /// 自定义规则
    Custom(String),
}

/// 护栏严重程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuardrailSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// 威胁情报配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatIntelConfig {
    /// 是否启用RAG查询
    pub enable_rag: bool,
    /// RAG top_k
    pub rag_top_k: usize,
    /// RAG相似度阈值
    pub rag_threshold: f32,
    /// 是否启用CVE工具查询
    pub enable_cve_tool: bool,
    /// CVE查询超时(秒)
    pub cve_timeout: Option<f64>,
    /// 威胁情报缓存时间(秒)
    pub cache_duration: u64,
}

/// 任务复杂度判断配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityConfig {
    /// 是否启用规则判断
    pub enable_rule_based: bool,
    /// 是否启用LLM判断
    pub enable_llm_based: bool,
    /// 规则判断关键词
    pub rule_keywords: HashMap<TaskComplexity, Vec<String>>,
    /// LLM判断温度
    pub llm_temperature: f32,
}

/// 错误回退策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackStrategy {
    /// 不回退,直接失败
    NoRollback,
    /// 回退到上一个阶段
    PreviousPhase,
    /// 回退到指定阶段
    SpecificPhase(OodaPhase),
    /// 智能回退(根据错误类型决定)
    Intelligent,
}

/// OODA循环
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OodaCycle {
    /// 循环ID
    pub id: String,
    /// 循环序号
    pub cycle_number: u32,
    /// 当前阶段
    pub current_phase: OodaPhase,
    /// 阶段历史
    pub phase_history: Vec<OodaPhaseExecution>,
    /// 循环状态
    pub status: OodaCycleStatus,
    /// 开始时间
    pub started_at: SystemTime,
    /// 结束时间
    pub completed_at: Option<SystemTime>,
    /// 循环结果
    pub result: Option<OodaCycleResult>,
    /// 错误信息
    pub error: Option<String>,
}

/// OODA阶段
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OodaPhase {
    /// Observe - 侦察
    Observe,
    /// Orient - 分析定位
    Orient,
    /// Decide - 决策
    Decide,
    /// Act - 执行
    Act,
}

/// OODA阶段执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OodaPhaseExecution {
    /// 阶段
    pub phase: OodaPhase,
    /// 开始时间
    pub started_at: SystemTime,
    /// 结束时间
    pub completed_at: Option<SystemTime>,
    /// 执行状态
    pub status: PhaseExecutionStatus,
    /// 阶段输入
    pub input: serde_json::Value,
    /// 阶段输出
    pub output: Option<serde_json::Value>,
    /// 护栏检查结果
    pub guardrail_checks: Vec<GuardrailCheckResult>,
    /// 工具调用记录
    pub tool_calls: Vec<ToolCallRecord>,
    /// 错误信息
    pub error: Option<String>,
}

/// 阶段执行状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PhaseExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    RolledBack,
}

/// OODA循环状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OodaCycleStatus {
    Running,
    Completed,
    Failed,
    RolledBack,
    Cancelled,
}

/// OODA循环结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OodaCycleResult {
    /// 是否成功
    pub success: bool,
    /// 收集的信息(Observe阶段)
    pub observations: HashMap<String, serde_json::Value>,
    /// 分析结果(Orient阶段)
    pub analysis: Option<ThreatAnalysis>,
    /// 决策计划(Decide阶段)
    pub decision: Option<ActionPlan>,
    /// 执行结果(Act阶段)
    pub execution_result: Option<serde_json::Value>,
}

/// 威胁分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatAnalysis {
    /// 识别的威胁
    pub threats: Vec<ThreatInfo>,
    /// 漏洞信息
    pub vulnerabilities: Vec<VulnerabilityInfo>,
    /// 威胁等级
    pub threat_level: ThreatLevel,
    /// 推荐行动
    pub recommendations: Vec<String>,
}

/// 威胁信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatInfo {
    /// 威胁ID
    pub id: String,
    /// 威胁名称
    pub name: String,
    /// 威胁描述
    pub description: String,
    /// 威胁等级
    pub level: ThreatLevel,
    /// 相关CVE
    pub cves: Vec<String>,
    /// 来源
    pub source: ThreatSource,
}

/// 漏洞信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityInfo {
    /// 漏洞ID
    pub id: String,
    /// 漏洞名称
    pub name: String,
    /// 漏洞描述
    pub description: String,
    /// CVSS评分
    pub cvss_score: Option<f32>,
    /// CVE编号
    pub cve_id: Option<String>,
    /// 影响组件
    pub affected_component: String,
    /// 修复建议
    pub remediation: Option<String>,
}

/// 威胁等级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatLevel {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// 威胁来源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreatSource {
    /// RAG知识库
    RAG,
    /// CVE数据库
    CVE,
    /// 威胁情报API
    ThreatIntelAPI,
    /// LLM分析
    LLMAnalysis,
}

/// 行动计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    /// 计划ID
    pub id: String,
    /// 计划名称
    pub name: String,
    /// 计划描述
    pub description: String,
    /// 计划步骤
    pub steps: Vec<ActionStep>,
    /// 预估执行时间(秒)
    pub estimated_duration: u64,
    /// 风险评估
    pub risk_assessment: RiskAssessment,
}

/// 行动步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStep {
    /// 步骤ID
    pub id: String,
    /// 步骤名称
    pub name: String,
    /// 步骤描述
    pub description: String,
    /// 步骤类型
    pub step_type: ActionStepType,
    /// 工具名称
    pub tool_name: Option<String>,
    /// 工具参数
    pub tool_args: HashMap<String, serde_json::Value>,
    /// 预估执行时间(秒)
    pub estimated_duration: u64,
}

/// 行动步骤类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionStepType {
    /// 直接工具调用
    DirectToolCall,
    /// 使用ReAct引擎
    ReactEngine,
    /// 使用其他引擎
    OtherEngine(String),
    /// 人工确认
    ManualConfirmation,
}

/// 风险评估
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// 风险等级
    pub risk_level: RiskLevel,
    /// 风险因素
    pub risk_factors: Vec<String>,
    /// 缓解措施
    pub mitigations: Vec<String>,
    /// 是否需要人工确认
    pub requires_manual_approval: bool,
}

/// 风险等级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// 护栏检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailCheckResult {
    /// 检查ID
    pub check_id: String,
    /// 规则ID
    pub rule_id: String,
    /// 规则名称
    pub rule_name: String,
    /// 检查结果
    pub result: GuardrailCheckStatus,
    /// 严重程度
    pub severity: GuardrailSeverity,
    /// 消息
    pub message: String,
    /// 详细信息
    pub details: Option<serde_json::Value>,
    /// 检查时间
    pub checked_at: SystemTime,
}

/// 护栏检查状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GuardrailCheckStatus {
    /// 通过
    Passed,
    /// 警告
    Warning,
    /// 失败
    Failed,
    /// 跳过
    Skipped,
}

/// 工具调用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    /// 调用ID
    pub call_id: String,
    /// 工具名称
    pub tool_name: String,
    /// 工具参数
    pub args: HashMap<String, serde_json::Value>,
    /// 调用时间
    pub called_at: SystemTime,
    /// 完成时间
    pub completed_at: Option<SystemTime>,
    /// 执行状态
    pub status: ToolCallStatus,
    /// 执行结果
    pub result: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
}

/// 工具调用状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolCallStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Timeout,
}

/// 任务复杂度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaskComplexity {
    /// 简单任务(单工具调用)
    Simple,
    /// 中等任务(多工具顺序调用)
    Medium,
    /// 复杂任务(需要推理和规划)
    Complex,
}

/// Travel执行轨迹
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TravelTrace {
    /// 轨迹ID
    pub trace_id: String,
    /// 任务描述
    pub task: String,
    /// 任务复杂度
    pub task_complexity: TaskComplexity,
    /// OODA循环历史
    pub ooda_cycles: Vec<OodaCycle>,
    /// 开始时间
    pub started_at: SystemTime,
    /// 结束时间
    pub completed_at: Option<SystemTime>,
    /// 最终状态
    pub status: TravelStatus,
    /// 最终结果
    pub final_result: Option<serde_json::Value>,
    /// 聚合指标
    pub metrics: TravelMetrics,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Travel执行状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TravelStatus {
    Running,
    Completed,
    Failed,
    MaxCyclesReached,
    Cancelled,
}

/// Travel聚合指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TravelMetrics {
    /// 总OODA循环次数
    pub total_cycles: u32,
    /// 总工具调用次数
    pub total_tool_calls: u32,
    /// 总执行时间(毫秒)
    pub total_duration_ms: u64,
    /// 护栏检查次数
    pub guardrail_checks: u32,
    /// 护栏失败次数
    pub guardrail_failures: u32,
    /// 回退次数
    pub rollback_count: u32,
    /// 威胁情报查询次数
    pub threat_intel_queries: u32,
}

// ========== 默认实现 ==========

impl Default for TravelConfig {
    fn default() -> Self {
        Self {
            max_ooda_cycles: 10,
            max_cycle_duration: Some(300.0), // 5分钟
            enable_guardrails: true,
            guardrail_config: GuardrailConfig::default(),
            enable_threat_intel: true,
            threat_intel_config: ThreatIntelConfig::default(),
            complexity_config: ComplexityConfig::default(),
            verbose: false,
            rollback_strategy: RollbackStrategy::Intelligent,
            lite_mode: LiteModeConfig::default(),
            parallel_config: ParallelExecutionConfig::default(),
            context_config: ContextManagerConfig::default(),
            enable_multimodal: true, // 默认启用多模态
        }
    }
}

impl Default for GuardrailConfig {
    fn default() -> Self {
        Self {
            observe_rules: vec![
                GuardrailRule {
                    id: "target_legality".to_string(),
                    name: "Target Legality Check".to_string(),
                    description: "Verify target is authorized for testing".to_string(),
                    rule_type: GuardrailRuleType::TargetLegality,
                    severity: GuardrailSeverity::Critical,
                    enabled: true,
                },
            ],
            orient_rules: vec![
                GuardrailRule {
                    id: "exploit_risk".to_string(),
                    name: "Exploit Risk Assessment".to_string(),
                    description: "Assess risk of exploit techniques".to_string(),
                    rule_type: GuardrailRuleType::ExploitRisk,
                    severity: GuardrailSeverity::Error,
                    enabled: true,
                },
            ],
            decide_rules: vec![
                GuardrailRule {
                    id: "payload_safety".to_string(),
                    name: "Payload Safety Check".to_string(),
                    description: "Verify attack payloads are safe".to_string(),
                    rule_type: GuardrailRuleType::PayloadSafety,
                    severity: GuardrailSeverity::Critical,
                    enabled: true,
                },
            ],
            act_rules: vec![
                GuardrailRule {
                    id: "operation_risk".to_string(),
                    name: "Operation Risk Check".to_string(),
                    description: "Final check before execution".to_string(),
                    rule_type: GuardrailRuleType::OperationRisk,
                    severity: GuardrailSeverity::Error,
                    enabled: true,
                },
            ],
            strict_mode: true,
        }
    }
}

impl Default for ThreatIntelConfig {
    fn default() -> Self {
        Self {
            enable_rag: true,
            rag_top_k: 5,
            rag_threshold: 0.7,
            enable_cve_tool: true,
            cve_timeout: Some(30.0),
            cache_duration: 3600, // 1小时
        }
    }
}

impl Default for ComplexityConfig {
    fn default() -> Self {
        let mut rule_keywords = HashMap::new();
        rule_keywords.insert(
            TaskComplexity::Simple,
            vec!["scan".to_string(), "check".to_string(), "查询".to_string()],
        );
        rule_keywords.insert(
            TaskComplexity::Medium,
            vec!["test".to_string(), "analyze".to_string(), "测试".to_string()],
        );
        rule_keywords.insert(
            TaskComplexity::Complex,
            vec![
                "penetration".to_string(),
                "exploit".to_string(),
                "渗透".to_string(),
                "攻击链".to_string(),
            ],
        );

        Self {
            enable_rule_based: true,
            enable_llm_based: true,
            rule_keywords,
            llm_temperature: 0.3,
        }
    }
}

impl Default for TravelMetrics {
    fn default() -> Self {
        Self {
            total_cycles: 0,
            total_tool_calls: 0,
            total_duration_ms: 0,
            guardrail_checks: 0,
            guardrail_failures: 0,
            rollback_count: 0,
            threat_intel_queries: 0,
        }
    }
}

impl OodaCycle {
    pub fn new(cycle_number: u32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            cycle_number,
            current_phase: OodaPhase::Observe,
            phase_history: Vec::new(),
            status: OodaCycleStatus::Running,
            started_at: SystemTime::now(),
            completed_at: None,
            result: None,
            error: None,
        }
    }

    pub fn add_phase_execution(&mut self, execution: OodaPhaseExecution) {
        self.phase_history.push(execution);
    }

    pub fn complete(&mut self, result: OodaCycleResult) {
        self.status = OodaCycleStatus::Completed;
        self.completed_at = Some(SystemTime::now());
        self.result = Some(result);
    }

    pub fn fail(&mut self, error: String) {
        self.status = OodaCycleStatus::Failed;
        self.completed_at = Some(SystemTime::now());
        self.error = Some(error);
    }
}

impl TravelTrace {
    pub fn new(task: String, task_complexity: TaskComplexity) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            task,
            task_complexity,
            ooda_cycles: Vec::new(),
            started_at: SystemTime::now(),
            completed_at: None,
            status: TravelStatus::Running,
            final_result: None,
            metrics: TravelMetrics::default(),
            metadata: HashMap::new(),
        }
    }

    pub fn add_cycle(&mut self, cycle: OodaCycle) {
        self.ooda_cycles.push(cycle);
        self.metrics.total_cycles += 1;
    }

    pub fn complete(&mut self, result: serde_json::Value) {
        self.status = TravelStatus::Completed;
        self.completed_at = Some(SystemTime::now());
        self.final_result = Some(result);
    }

    pub fn fail(&mut self, error: String) {
        self.status = TravelStatus::Failed;
        self.completed_at = Some(SystemTime::now());
        self.final_result = Some(serde_json::json!({
            "error": error
        }));
    }
}

// 确保线程安全
unsafe impl Send for TravelTrace {}
unsafe impl Sync for TravelTrace {}
unsafe impl Send for OodaCycle {}
unsafe impl Sync for OodaCycle {}

// ========== DAG 任务类型 (Token优化核心) ==========

/// DAG执行计划 - 一次LLM调用生成完整计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagPlan {
    /// 计划ID
    pub id: String,
    /// 任务描述
    pub task_description: String,
    /// DAG任务节点
    pub tasks: Vec<DagTask>,
    /// 任务依赖关系 (task_id -> 依赖的task_ids)
    pub dependencies: HashMap<String, Vec<String>>,
    /// 创建时间
    pub created_at: SystemTime,
    /// 预估总Token消耗
    pub estimated_tokens: u32,
}

/// DAG任务节点 - 增强版支持条件分支和循环
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagTask {
    /// 任务ID (如 "1", "2", "3")
    pub id: String,
    /// 工具名称 (特殊值: "_condition", "_loop", "_join")
    pub tool_name: String,
    /// 工具参数 (支持 $1, $2 等变量引用)
    pub arguments: HashMap<String, serde_json::Value>,
    /// 依赖的任务ID列表
    pub depends_on: Vec<String>,
    /// 任务描述
    pub description: Option<String>,
    /// 执行状态
    pub status: DagTaskStatus,
    /// 执行结果
    pub result: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 开始时间
    pub started_at: Option<SystemTime>,
    /// 完成时间
    pub completed_at: Option<SystemTime>,
    
    // ========== 增强功能 ==========
    /// 条件表达式 (如 "$1.status == 'success'")
    #[serde(default)]
    pub condition: Option<ConditionExpr>,
    /// 循环配置 (如 "for each $1.urls")
    #[serde(default)]
    pub loop_config: Option<LoopConfig>,
    /// 错误处理策略
    #[serde(default)]
    pub on_error: ErrorStrategy,
    /// 重试配置
    #[serde(default)]
    pub retry_config: Option<RetryConfig>,
    /// 任务优先级 (用于调度)
    #[serde(default)]
    pub priority: TaskPriority,
    /// 是否为动态生成的任务
    #[serde(default)]
    pub is_dynamic: bool,
    /// 父任务ID (如果是循环展开生成的)
    #[serde(default)]
    pub parent_task_id: Option<String>,
}

/// 条件表达式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionExpr {
    /// 条件表达式字符串 (如 "$1.status == 'success'")
    pub expr: String,
    /// 条件为真时执行的分支任务ID
    pub then_branch: Option<String>,
    /// 条件为假时执行的分支任务ID
    pub else_branch: Option<String>,
}

/// 循环配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    /// 循环类型
    pub loop_type: LoopType,
    /// 迭代变量名 (在循环体内可用 $item 引用)
    pub item_var: String,
    /// 循环体任务模板
    pub body_template: Box<DagTask>,
    /// 最大迭代次数 (防止无限循环)
    pub max_iterations: u32,
    /// 并行执行循环迭代
    pub parallel: bool,
}

/// 循环类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopType {
    /// 遍历数组: for each item in $1.items
    ForEach { source: String },
    /// 范围循环: for i in 0..10
    Range { start: i32, end: i32 },
    /// While 循环: while $1.has_more
    While { condition: String },
}

/// 错误处理策略
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ErrorStrategy {
    /// 失败时中止整个计划
    #[default]
    Abort,
    /// 跳过当前任务，继续执行
    Skip,
    /// 重试指定次数后跳过
    RetryThenSkip,
    /// 重试指定次数后中止
    RetryThenAbort,
    /// 执行回退任务
    Fallback { task_id: String },
    /// 触发重规划
    Replan,
}

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试延迟 (毫秒)
    pub delay_ms: u64,
    /// 延迟倍增因子
    pub backoff_factor: f32,
}

/// 任务优先级
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

/// DAG任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DagTaskStatus {
    /// 等待依赖
    Pending,
    /// 可执行(依赖已满足)
    Ready,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 执行失败
    Failed,
    /// 已跳过
    Skipped,
}

/// DAG执行结果 - 增强版支持重规划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagExecutionResult {
    /// 计划ID
    pub plan_id: String,
    /// 是否成功
    pub success: bool,
    /// 任务结果 (task_id -> result)
    pub task_results: HashMap<String, serde_json::Value>,
    /// 失败的任务
    pub failed_tasks: Vec<String>,
    /// 执行指标
    pub metrics: DagExecutionMetrics,
    /// 最终输出
    pub final_output: Option<serde_json::Value>,
    
    // ========== 重规划支持 ==========
    /// 是否需要重规划
    #[serde(default)]
    pub needs_replanning: bool,
    /// 重规划原因
    #[serde(default)]
    pub replan_reason: Option<ReplanReason>,
    /// 执行状态快照 (用于重规划时恢复)
    #[serde(default)]
    pub execution_snapshot: Option<ExecutionSnapshot>,
}

/// 重规划原因
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplanReason {
    /// 任务失败且策略为 Replan
    TaskFailed { task_id: String, error: String },
    /// 发现新的目标/信息需要调整计划
    NewDiscovery { description: String, data: serde_json::Value },
    /// 目标不可达，需要替代方案
    TargetUnreachable { original_target: String },
    /// 资源不足，需要调整策略
    ResourceExhausted { resource: String },
    /// 用户请求重规划
    UserRequested { reason: String },
    /// 循环检测到无效操作
    IneffectiveLoop { iterations: u32 },
}

/// 执行快照 (用于重规划时恢复上下文)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionSnapshot {
    /// 已完成的任务及其结果
    pub completed_tasks: HashMap<String, serde_json::Value>,
    /// 收集到的信息
    pub gathered_info: HashMap<String, serde_json::Value>,
    /// 当前目标状态
    pub target_state: Option<serde_json::Value>,
    /// 已尝试的方法
    pub attempted_approaches: Vec<String>,
    /// 错误历史
    pub error_history: Vec<ErrorRecord>,
}

/// 错误记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub task_id: String,
    pub tool_name: String,
    pub error: String,
    pub timestamp: SystemTime,
    pub context: Option<serde_json::Value>,
}

/// DAG执行指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DagExecutionMetrics {
    /// 总任务数
    pub total_tasks: u32,
    /// 成功任务数
    pub completed_tasks: u32,
    /// 失败任务数
    pub failed_tasks: u32,
    /// 跳过任务数
    pub skipped_tasks: u32,
    /// 并行执行的最大数量
    pub max_parallel: u32,
    /// 总执行时间(ms)
    pub total_duration_ms: u64,
    /// LLM调用次数
    pub llm_calls: u32,
    /// 估算节省的Token数
    pub tokens_saved: u32,
}

/// 执行模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionMode {
    /// 完整OODA模式
    FullOoda,
    /// 精简DAG模式
    LiteDag,
    /// 混合模式(根据复杂度自动切换)
    Hybrid,
    /// 流式DAG模式 (边规划边执行)
    StreamingDag,
    /// 自适应重规划模式
    AdaptiveReplan,
}

// ========== 自主 Observe 类型 ==========

/// 自主 Observe 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousObserveConfig {
    /// 最大探索深度
    pub max_depth: u32,
    /// 最大工具调用次数
    pub max_tool_calls: u32,
    /// 信息充分性阈值 (0-1)
    pub sufficiency_threshold: f32,
    /// 启用自适应策略选择
    pub adaptive_strategy: bool,
    /// 优先探索的信息类型
    pub priority_info_types: Vec<InfoType>,
}

/// 信息类型 (用于自主 Observe)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum InfoType {
    /// 目标结构信息
    TargetStructure,
    /// API 端点
    ApiEndpoints,
    /// 表单和输入
    FormsAndInputs,
    /// 技术栈
    TechStack,
    /// 认证机制
    Authentication,
    /// 错误信息
    ErrorMessages,
    /// 配置信息
    Configuration,
    /// 自定义类型
    Custom(String),
}

/// 观察策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserveStrategy {
    /// 策略名称
    pub name: String,
    /// 需要收集的信息类型
    pub required_info: Vec<InfoType>,
    /// 建议使用的工具
    pub suggested_tools: Vec<String>,
    /// 收集顺序
    pub collection_order: Vec<ObserveStep>,
    /// 成功条件
    pub success_criteria: String,
}

/// 观察步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserveStep {
    /// 步骤ID
    pub id: String,
    /// 步骤目标
    pub objective: String,
    /// 使用的工具
    pub tool: String,
    /// 参数模板
    pub args_template: HashMap<String, serde_json::Value>,
    /// 依赖的步骤
    pub depends_on: Vec<String>,
    /// 是否可选
    pub optional: bool,
}

/// 观察结果评估
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserveAssessment {
    /// 信息充分性得分 (0-1)
    pub sufficiency_score: f32,
    /// 已收集的信息类型
    pub collected_info: Vec<InfoType>,
    /// 缺失的信息类型
    pub missing_info: Vec<InfoType>,
    /// 建议的下一步
    pub suggested_next_steps: Vec<String>,
    /// 是否可以进入 Orient 阶段
    pub ready_for_orient: bool,
}

impl Default for AutonomousObserveConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            max_tool_calls: 10,
            sufficiency_threshold: 0.7,
            adaptive_strategy: true,
            priority_info_types: vec![
                InfoType::TargetStructure,
                InfoType::ApiEndpoints,
                InfoType::TechStack,
            ],
        }
    }
}

// ========== 流式执行类型 ==========

/// 流式任务事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamingTaskEvent {
    /// 新任务被规划
    TaskPlanned { task: DagTask },
    /// 任务开始执行
    TaskStarted { task_id: String },
    /// 任务执行中 (进度更新)
    TaskProgress { task_id: String, progress: f32, message: String },
    /// 任务完成
    TaskCompleted { task_id: String, result: serde_json::Value },
    /// 任务失败
    TaskFailed { task_id: String, error: String },
    /// 触发重规划
    ReplanTriggered { reason: ReplanReason },
    /// 新计划生成
    PlanUpdated { new_tasks: Vec<DagTask> },
    /// 执行完成
    ExecutionComplete { result: DagExecutionResult },
}

/// 流式执行状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingExecutionState {
    /// 当前计划版本
    pub plan_version: u32,
    /// 已完成任务
    pub completed: Vec<String>,
    /// 正在执行任务
    pub running: Vec<String>,
    /// 待执行任务
    pub pending: Vec<String>,
    /// 累积结果
    pub results: HashMap<String, serde_json::Value>,
    /// 重规划次数
    pub replan_count: u32,
}

/// 资源信息(用于追踪)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedResource {
    /// 资源ID
    pub id: String,
    /// 资源类型
    pub resource_type: TrackedResourceType,
    /// 创建任务
    pub created_by: Option<String>,
    /// 创建时间
    pub created_at: SystemTime,
    /// 是否已清理
    pub cleaned: bool,
}

/// 追踪的资源类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TrackedResourceType {
    /// 浏览器会话
    Browser,
    /// 代理服务
    Proxy,
    /// 临时文件
    TempFile,
    /// 网络连接
    NetworkConnection,
}

// ========== 新类型默认实现 ==========

impl Default for LiteModeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            applicable_complexity: vec![TaskComplexity::Simple, TaskComplexity::Medium],
            max_steps: 10,
            skip_ooda_for_simple: true,
            enable_plan_cache: true,
            plan_cache_ttl: 3600,
        }
    }
}

impl Default for ParallelExecutionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrency: 5,
            task_timeout: 60,
            enable_resource_tracking: true,
        }
    }
}

impl Default for ContextManagerConfig {
    fn default() -> Self {
        Self {
            enable_compression: true,
            max_context_tokens: 4000,
            max_history_entries: 10,
            max_tool_result_length: 2000,
            preserve_fields: vec![
                "target".to_string(),
                "url".to_string(),
                "domain".to_string(),
                "status".to_string(),
                "error".to_string(),
            ],
        }
    }
}

impl DagPlan {
    pub fn new(task_description: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            task_description,
            tasks: Vec::new(),
            dependencies: HashMap::new(),
            created_at: SystemTime::now(),
            estimated_tokens: 0,
        }
    }

    /// 添加任务
    pub fn add_task(&mut self, task: DagTask) {
        let task_id = task.id.clone();
        let deps = task.depends_on.clone();
        self.tasks.push(task);
        if !deps.is_empty() {
            self.dependencies.insert(task_id, deps);
        }
    }

    /// 获取可执行的任务(依赖已满足)
    pub fn get_ready_tasks(&self, completed: &[String]) -> Vec<&DagTask> {
        self.tasks
            .iter()
            .filter(|t| {
                t.status == DagTaskStatus::Pending
                    && t.depends_on.iter().all(|dep| completed.contains(dep))
            })
            .collect()
    }
}

impl DagTask {
    pub fn new(id: String, tool_name: String, arguments: HashMap<String, serde_json::Value>) -> Self {
        Self {
            id,
            tool_name,
            arguments,
            depends_on: Vec::new(),
            description: None,
            status: DagTaskStatus::Pending,
            result: None,
            error: None,
            started_at: None,
            completed_at: None,
            // 增强功能默认值
            condition: None,
            loop_config: None,
            on_error: ErrorStrategy::default(),
            retry_config: None,
            priority: TaskPriority::default(),
            is_dynamic: false,
            parent_task_id: None,
        }
    }

    pub fn with_depends(mut self, deps: Vec<String>) -> Self {
        self.depends_on = deps;
        self
    }

    pub fn with_description(mut self, desc: String) -> Self {
        self.description = Some(desc);
        self
    }
    
    pub fn with_condition(mut self, condition: ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
    
    pub fn with_loop(mut self, loop_config: LoopConfig) -> Self {
        self.loop_config = Some(loop_config);
        self
    }
    
    pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
        self.on_error = strategy;
        self
    }
    
    pub fn with_retry(mut self, max_retries: u32, delay_ms: u64) -> Self {
        self.retry_config = Some(RetryConfig {
            max_retries,
            delay_ms,
            backoff_factor: 2.0,
        });
        self
    }
    
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// 创建条件任务
    pub fn condition_task(id: String, expr: String, then_branch: Option<String>, else_branch: Option<String>) -> Self {
        Self::new(id, "_condition".to_string(), HashMap::new())
            .with_condition(ConditionExpr {
                expr,
                then_branch,
                else_branch,
            })
    }
    
    /// 创建循环任务
    pub fn loop_task(id: String, loop_type: LoopType, item_var: String, body_template: DagTask) -> Self {
        let mut task = Self::new(id, "_loop".to_string(), HashMap::new());
        task.loop_config = Some(LoopConfig {
            loop_type,
            item_var,
            body_template: Box::new(body_template),
            max_iterations: 100,
            parallel: false,
        });
        task
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            delay_ms: 1000,
            backoff_factor: 2.0,
        }
    }
}

impl DagExecutionResult {
    /// 检查是否需要重规划
    pub fn should_replan(&self) -> bool {
        self.needs_replanning
    }
    
    /// 创建需要重规划的结果
    pub fn with_replan_needed(mut self, reason: ReplanReason) -> Self {
        self.needs_replanning = true;
        self.replan_reason = Some(reason);
        self
    }
}

// DagExecutionMetrics 使用 derive(Default)


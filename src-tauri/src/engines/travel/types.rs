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


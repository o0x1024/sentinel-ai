//! Agent Team 数据结构定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ==================== 状态机枚举 ====================

/// Team 会话状态机
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TeamSessionState {
    Pending,
    Initializing,
    Proposing,
    Challenging,
    ConvergenceCheck,
    Revising,
    Deciding,
    ArtifactGeneration,
    Completed,
    Failed,
    SuspendedForHuman,
}

impl Default for TeamSessionState {
    fn default() -> Self {
        TeamSessionState::Pending
    }
}

impl std::fmt::Display for TeamSessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "PENDING".to_string());
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for TeamSessionState {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PENDING" => Ok(Self::Pending),
            "INITIALIZING" => Ok(Self::Initializing),
            "PROPOSING" => Ok(Self::Proposing),
            "CHALLENGING" => Ok(Self::Challenging),
            "CONVERGENCE_CHECK" => Ok(Self::ConvergenceCheck),
            "REVISING" => Ok(Self::Revising),
            "DECIDING" => Ok(Self::Deciding),
            "ARTIFACT_GENERATION" => Ok(Self::ArtifactGeneration),
            "COMPLETED" => Ok(Self::Completed),
            "FAILED" => Ok(Self::Failed),
            "SUSPENDED_FOR_HUMAN" => Ok(Self::SuspendedForHuman),
            _ => Err(anyhow::anyhow!("Unknown TeamSessionState: {}", s)),
        }
    }
}

/// 白板条目类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlackboardEntryType {
    Consensus,
    Dispute,
    ActionItem,
}

impl std::fmt::Display for BlackboardEntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Consensus => write!(f, "consensus"),
            Self::Dispute => write!(f, "dispute"),
            Self::ActionItem => write!(f, "action_item"),
        }
    }
}

impl std::str::FromStr for BlackboardEntryType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "consensus" => Ok(Self::Consensus),
            "dispute" => Ok(Self::Dispute),
            "action_item" => Ok(Self::ActionItem),
            _ => Err(anyhow::anyhow!("Unknown BlackboardEntryType: {}", s)),
        }
    }
}

// ==================== 模板结构 ====================

/// Agent Team 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeamTemplate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// 领域：product / security / ops / audit / ...
    pub domain: String,
    /// JSON: 默认轮次配置
    pub default_rounds_config: Option<serde_json::Value>,
    /// JSON: 默认工具策略
    pub default_tool_policy: Option<serde_json::Value>,
    pub is_system: bool,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// 关联的角色（查询时填充）
    #[serde(default)]
    pub members: Vec<AgentTeamTemplateMember>,
}

/// Agent Team 模板角色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeamTemplateMember {
    pub id: String,
    pub template_id: String,
    pub name: String,
    pub responsibility: Option<String>,
    pub system_prompt: Option<String>,
    /// 决策风格: conservative / balanced / aggressive
    pub decision_style: Option<String>,
    /// 风险偏好: low / medium / high
    pub risk_preference: Option<String>,
    /// 角色权重（用于最终决策）
    pub weight: f64,
    /// JSON: 工具策略（白名单/黑名单/调用上限）
    pub tool_policy: Option<serde_json::Value>,
    /// JSON: 输出格式模板
    pub output_schema: Option<serde_json::Value>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ==================== 会话结构 ====================

/// Agent Team 会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeamSession {
    pub id: String,
    pub conversation_id: Option<String>,
    pub template_id: Option<String>,
    pub name: String,
    pub goal: Option<String>,
    pub state: String,
    /// JSON: 状态机追踪
    pub state_machine: Option<serde_json::Value>,
    pub current_round: i32,
    pub max_rounds: i32,
    /// JSON: 白板共识快照
    pub blackboard_state: Option<serde_json::Value>,
    /// JSON: 按轮次记录的分歧度历史
    pub divergence_scores: Option<serde_json::Value>,
    /// FinOps
    pub total_tokens: i64,
    pub estimated_cost: f64,
    /// JSON: 挂起原因
    pub suspended_reason: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// 关联的成员（查询时填充）
    #[serde(default)]
    pub members: Vec<AgentTeamMember>,
}

/// Agent Team 会话成员（模板角色快照）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeamMember {
    pub id: String,
    pub session_id: String,
    pub name: String,
    pub responsibility: Option<String>,
    pub system_prompt: Option<String>,
    pub decision_style: Option<String>,
    pub risk_preference: Option<String>,
    pub weight: f64,
    /// JSON: 工具策略
    pub tool_policy: Option<serde_json::Value>,
    /// JSON: 输出格式模板
    pub output_schema: Option<serde_json::Value>,
    pub sort_order: i32,
    /// FinOps: 角色独立资源追踪
    pub token_usage: i64,
    pub tool_calls_count: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ==================== 讨论轮次 ====================

/// 轮次记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeamRound {
    pub id: String,
    pub session_id: String,
    pub round_number: i32,
    pub phase: String,
    pub status: String,
    /// JSON: 分歧度
    pub divergence_score: Option<f64>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 消息记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeamMessage {
    pub id: String,
    pub session_id: String,
    pub round_id: Option<String>,
    pub member_id: Option<String>,
    pub member_name: Option<String>,
    pub role: String,
    pub content: String,
    /// JSON: 工具调用
    pub tool_calls: Option<serde_json::Value>,
    pub token_count: Option<i32>,
    pub timestamp: DateTime<Utc>,
}

// ==================== 白板 ====================

/// 白板条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeamBlackboardEntry {
    pub id: String,
    pub session_id: String,
    pub round_id: Option<String>,
    pub entry_type: String,
    pub title: String,
    pub content: String,
    pub contributed_by: Option<String>,
    pub is_resolved: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ==================== 决策与产物 ====================

/// 最终决策
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeamDecision {
    pub id: String,
    pub session_id: String,
    pub round_id: Option<String>,
    pub decision_type: String,
    pub content: String,
    pub decided_by: Option<String>,
    pub confidence_score: Option<f64>,
    pub created_at: DateTime<Utc>,
}

/// 产物文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeamArtifact {
    pub id: String,
    pub session_id: String,
    pub artifact_type: String,
    pub title: String,
    pub content: String,
    pub version: i32,
    pub parent_artifact_id: Option<String>,
    /// JSON: 变更摘要
    pub diff_summary: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ==================== API 请求/响应 ====================

/// 创建模板请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentTeamTemplateRequest {
    pub name: String,
    pub description: Option<String>,
    pub domain: String,
    pub default_rounds_config: Option<serde_json::Value>,
    pub default_tool_policy: Option<serde_json::Value>,
    pub members: Vec<CreateAgentTeamTemplateMemberRequest>,
}

/// 创建模板角色请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentTeamTemplateMemberRequest {
    pub name: String,
    pub responsibility: Option<String>,
    pub system_prompt: Option<String>,
    pub decision_style: Option<String>,
    pub risk_preference: Option<String>,
    pub weight: Option<f64>,
    pub tool_policy: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub sort_order: Option<i32>,
}

/// 更新模板请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAgentTeamTemplateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub domain: Option<String>,
    pub default_rounds_config: Option<serde_json::Value>,
    pub default_tool_policy: Option<serde_json::Value>,
    /// 可选：若提供则整体替换模板角色列表
    pub members: Option<Vec<CreateAgentTeamTemplateMemberRequest>>,
}

/// 创建会话请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentTeamSessionRequest {
    pub name: String,
    pub goal: Option<String>,
    pub template_id: Option<String>,
    pub conversation_id: Option<String>,
    pub max_rounds: Option<i32>,
    /// JSON: 会话级状态（含 Team 全局工具配置）
    pub state_machine: Option<serde_json::Value>,
    /// 若不使用模板，手动指定成员列表
    pub members: Option<Vec<CreateAgentTeamTemplateMemberRequest>>,
}

/// 更新会话请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAgentTeamSessionRequest {
    pub name: Option<String>,
    pub goal: Option<String>,
    pub state: Option<String>,
    pub max_rounds: Option<i32>,
    /// JSON: 会话级状态（含 Team 全局工具配置）
    pub state_machine: Option<serde_json::Value>,
    pub error_message: Option<String>,
}

/// 提交人工介入消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitAgentTeamMessageRequest {
    pub session_id: String,
    pub content: String,
    pub resume: bool,
}

/// 保存中断时的流式消息片段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendAgentTeamPartialMessageRequest {
    pub session_id: String,
    pub member_id: Option<String>,
    pub member_name: Option<String>,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<serde_json::Value>,
}

/// 更新白板状态请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBlackboardRequest {
    pub session_id: String,
    pub entry_type: String,
    pub title: String,
    pub content: String,
    pub contributed_by: Option<String>,
    pub round_id: Option<String>,
}

/// Team 运行状态（实时状态）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeamRunStatus {
    pub session_id: String,
    pub state: String,
    pub current_round: i32,
    pub blackboard_snapshot: Option<serde_json::Value>,
    pub latest_message: Option<String>,
    pub divergence_score: Option<f64>,
    pub is_suspended: bool,
}

// ==================== 事件定义 ====================

/// Agent Team 事件载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTeamEvent {
    pub event_type: String,
    pub session_id: String,
    pub payload: serde_json::Value,
}

impl AgentTeamEvent {
    pub fn new(event_type: &str, session_id: &str, payload: serde_json::Value) -> Self {
        Self {
            event_type: event_type.to_string(),
            session_id: session_id.to_string(),
            payload,
        }
    }
}

// ==================== MVP 4角色模板种子数据 ====================
pub struct BuiltinTeamRoles;

impl BuiltinTeamRoles {
    pub fn product_dev_roles() -> Vec<CreateAgentTeamTemplateMemberRequest> {
        vec![
            CreateAgentTeamTemplateMemberRequest {
                name: "产品经理".to_string(),
                responsibility: Some("负责需求分析、PRD 撰写与用户价值评估".to_string()),
                system_prompt: Some(
                    r#"你是一位资深产品经理。你的职责是：
1. 深度理解并挖掘用户真实需求
2. 撰写清晰的产品需求文档（PRD）
3. 平衡技术可行性与商业价值
4. 为讨论中的功能优先级提供量化依据
在讨论中，请从用户需求与商业价值角度出发，提出有据可查的观点。"#
                        .to_string(),
                ),
                decision_style: Some("balanced".to_string()),
                risk_preference: Some("medium".to_string()),
                weight: Some(1.0),
                tool_policy: None,
                output_schema: None,
                sort_order: Some(1),
            },
            CreateAgentTeamTemplateMemberRequest {
                name: "架构师".to_string(),
                responsibility: Some("负责系统架构设计、技术选型与可扩展性评估".to_string()),
                system_prompt: Some(
                    r#"你是一位资深软件架构师。你的职责是：
1. 设计模块化、可扩展的系统架构
2. 评估技术方案的可行性与性能影响
3. 识别潜在的技术债务与架构风险
4. 提供数据库设计、API 设计规范建议
在讨论中，请从技术可行性与系统质量角度出发，对方案提出建设性修改意见。"#
                        .to_string(),
                ),
                decision_style: Some("conservative".to_string()),
                risk_preference: Some("low".to_string()),
                weight: Some(1.2),
                tool_policy: None,
                output_schema: None,
                sort_order: Some(2),
            },
            CreateAgentTeamTemplateMemberRequest {
                name: "安全专家".to_string(),
                responsibility: Some("负责安全威胁建模、漏洞识别与合规审查".to_string()),
                system_prompt: Some(
                    r#"你是一位资深安全专家。你的职责是：
1. 对系统设计进行威胁建模（STRIDE/PASTA）
2. 识别 OWASP Top 10 等常见安全漏洞
3. 确保方案符合行业合规要求（SOC2/ISO27001 等）
4. 提出具体的安全加固与数据隐私保护建议
在讨论中，请以"安全左移"原则，对每个功能模块进行安全审查并给出量化风险等级。"#
                        .to_string(),
                ),
                decision_style: Some("conservative".to_string()),
                risk_preference: Some("low".to_string()),
                weight: Some(1.1),
                tool_policy: None,
                output_schema: None,
                sort_order: Some(3),
            },
            CreateAgentTeamTemplateMemberRequest {
                name: "测试工程师".to_string(),
                responsibility: Some("负责测试策略制定、边界条件识别与质量保障".to_string()),
                system_prompt: Some(
                    r#"你是一位资深测试工程师。你的职责是：
1. 制定全面的测试策略（单元/集成/E2E/性能）
2. 识别功能边界条件与异常路径
3. 评估方案的可测试性与可观测性
4. 提出具体的测试用例与自动化建议
在讨论中，请从质量保障角度出发，挑战方案中的不确定性与缺失的错误处理。"#
                        .to_string(),
                ),
                decision_style: Some("aggressive".to_string()),
                risk_preference: Some("medium".to_string()),
                weight: Some(0.9),
                tool_policy: None,
                output_schema: None,
                sort_order: Some(4),
            },
        ]
    }
}

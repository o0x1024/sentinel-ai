use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub connection_timeout: u64,
    pub auto_migrate: bool,
    pub backup_enabled: bool,
    pub backup_interval_hours: u32,
    pub backup_retention_days: u32,
}

/// 数据库统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub projects_count: u64,
    pub scan_tasks_count: u64,
    pub vulnerabilities_count: u64,
    pub assets_count: u64,
    pub submissions_count: u64,
    pub conversations_count: u64,
    pub db_size_bytes: u64,
    pub last_backup: Option<DateTime<Utc>>,
}

/// 表统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStats {
    pub table_name: String,
    pub record_count: u64,
    pub size_mb: f64,
    pub last_updated: Option<DateTime<Utc>>,
}

/// 备份信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: String,
    pub file_path: String,
    pub size_mb: f64,
    pub created_at: DateTime<Utc>,
    pub tables_included: Vec<String>,
    pub compression: bool,
    pub checksum: String,
}

/// 查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: u64,
    pub execution_time_ms: u64,
    pub query: String,
}

/// 数据库迁移
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    pub version: String,
    pub name: String,
    pub up_sql: String,
    pub down_sql: String,
    pub applied_at: Option<DateTime<Utc>>,
    pub checksum: String,
}

/// 赏金项目
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BountyProject {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub url: Option<String>,
    pub scope_domains: Option<String>, // JSON数组
    pub scope_ips: Option<String>, // JSON数组
    pub out_of_scope: Option<String>, // JSON数组
    pub reward_range: Option<String>, // JSON对象
    pub difficulty_level: i32,
    pub priority: i32,
    pub status: String,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub roi_score: f64,
    pub success_rate: f64,
    pub competition_level: i32,
    pub tags: Option<String>, // JSON数组
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl BountyProject {
    pub fn new(name: String, platform: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            platform,
            url: None,
            scope_domains: None,
            scope_ips: None,
            out_of_scope: None,
            reward_range: None,
            difficulty_level: 1,
            priority: 1,
            status: "active".to_string(),
            last_activity_at: None,
            roi_score: 0.0,
            success_rate: 0.0,
            competition_level: 1,
            tags: None,
            notes: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// 扫描任务
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScanTask {
    pub id: String,
    pub project_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub target_type: String,
    pub targets: String, // JSON数组
    pub scan_type: String,
    pub tools_config: Option<String>, // JSON对象
    pub status: String,
    pub progress: f64,
    pub priority: i32,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub execution_time: Option<i32>,
    pub results_summary: Option<String>, // JSON对象
    pub error_message: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ScanTask {
    pub fn new(name: String, target_type: String, targets: Vec<String>, scan_type: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            project_id: None,
            name,
            description: None,
            target_type,
            targets: serde_json::to_string(&targets).unwrap_or_default(),
            scan_type,
            tools_config: None,
            status: "pending".to_string(),
            progress: 0.0,
            priority: 1,
            scheduled_at: None,
            started_at: None,
            completed_at: None,
            execution_time: None,
            results_summary: None,
            error_message: None,
            created_by: "user".to_string(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// 资产
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Asset {
    pub id: String,
    pub project_id: Option<String>,
    pub scan_task_id: Option<String>,
    pub asset_type: String,
    pub value: String,
    pub parent_id: Option<String>,
    pub metadata: Option<String>, // JSON对象
    pub status: String,
    pub confidence_score: f64,
    pub risk_level: String,
    pub tags: Option<String>, // JSON数组
    pub notes: Option<String>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Asset {
    pub fn new(asset_type: String, value: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            project_id: None,
            scan_task_id: None,
            asset_type,
            value,
            parent_id: None,
            metadata: None,
            status: "active".to_string(),
            confidence_score: 1.0,
            risk_level: "info".to_string(),
            tags: None,
            notes: None,
            first_seen: now,
            last_seen: now,
            created_at: now,
            updated_at: now,
        }
    }
}

/// 漏洞
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Vulnerability {
    pub id: String,
    pub project_id: Option<String>,
    pub asset_id: Option<String>,
    pub scan_task_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub vulnerability_type: Option<String>,
    pub severity: String,
    pub cvss_score: Option<f64>,
    pub cvss_vector: Option<String>,
    pub cwe_id: Option<String>,
    pub owasp_category: Option<String>,
    pub proof_of_concept: Option<String>,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub references: Option<String>, // JSON数组
    pub status: String,
    pub verification_status: String,
    pub submission_status: String,
    pub reward_amount: Option<f64>,
    pub submission_date: Option<DateTime<Utc>>,
    pub resolution_date: Option<DateTime<Utc>>,
    pub tags: Option<String>, // JSON数组
    pub attachments: Option<String>, // JSON数组
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Vulnerability {
    pub fn new(title: String, severity: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            project_id: None,
            asset_id: None,
            scan_task_id: None,
            title,
            description: None,
            vulnerability_type: None,
            severity,
            cvss_score: None,
            cvss_vector: None,
            cwe_id: None,
            owasp_category: None,
            proof_of_concept: None,
            impact: None,
            remediation: None,
            references: None,
            status: "open".to_string(),
            verification_status: "unverified".to_string(),
            submission_status: "not_submitted".to_string(),
            reward_amount: None,
            submission_date: None,
            resolution_date: None,
            tags: None,
            attachments: None,
            notes: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// 提交记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Submission {
    pub id: String,
    pub vulnerability_id: String,
    pub project_id: String,
    pub platform: String,
    pub submission_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub severity: String,
    pub status: String,
    pub reward_amount: Option<f64>,
    pub bonus_amount: Option<f64>,
    pub currency: String,
    pub submitted_at: DateTime<Utc>,
    pub triaged_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub feedback: Option<String>,
    pub response_time: Option<i32>,
    pub resolution_time: Option<i32>,
    pub collaborators: Option<String>, // JSON数组
    pub attachments: Option<String>, // JSON数组
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// MCP工具
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct McpTool {
    pub id: String,
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub category: Option<String>,
    pub tool_type: String,
    pub executable_path: Option<String>,
    pub install_command: Option<String>,
    pub config_schema: Option<String>, // JSON Schema
    pub default_config: Option<String>, // JSON对象
    pub capabilities: Option<String>, // JSON数组
    pub supported_platforms: Option<String>, // JSON数组
    pub requirements: Option<String>, // JSON数组
    pub status: String,
    pub installation_status: Option<String>,
    pub last_used: Option<DateTime<Utc>>,
    pub usage_count: i32,
    pub success_rate: f64,
    pub average_execution_time: Option<i32>,
    pub tags: Option<String>, // JSON数组
    pub author: Option<String>,
    pub license: Option<String>,
    pub documentation_url: Option<String>,
    pub source_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// MCP连接
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct McpConnection {
    pub id: String,
    pub name: String,
    pub connection_type: String,
    pub endpoint: Option<String>,
    pub config: Option<String>, // JSON对象
    pub status: String,
    pub capabilities: Option<String>, // JSON数组
    pub server_info: Option<String>, // JSON对象
    pub tools_count: i32,
    pub last_ping: Option<DateTime<Utc>>,
    pub connected_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    pub auto_reconnect: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 工具执行记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ToolExecution {
    pub id: String,
    pub tool_id: String,
    pub scan_task_id: Option<String>,
    pub command: String,
    pub arguments: Option<String>, // JSON对象
    pub status: String,
    pub progress: f64,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub execution_time: Option<i32>,
    pub output: Option<String>,
    pub error_output: Option<String>,
    pub exit_code: Option<i32>,
    pub resource_usage: Option<String>, // JSON对象
    pub artifacts: Option<String>, // JSON数组
    pub metadata: Option<String>, // JSON对象
    pub created_at: DateTime<Utc>,
}

/// AI对话
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AiConversation {
    pub id: String,
    pub title: Option<String>,
    pub service_name: String,
    pub model_name: String,
    pub model_provider: Option<String>,
    pub context_type: Option<String>,
    pub project_id: Option<String>,
    pub vulnerability_id: Option<String>,
    pub scan_task_id: Option<String>,
    pub conversation_data: Option<String>, // JSON数组
    pub summary: Option<String>,
    pub total_messages: i32,
    pub total_tokens: i32,
    pub cost: f64,
    pub tags: Option<String>, // JSON数组
    pub is_archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AiConversation {
    pub fn new(model_name: String, service_name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title: None,
            service_name,
            model_name,
            model_provider: None,
            context_type: None,
            project_id: None,
            vulnerability_id: None,
            scan_task_id: None,
            conversation_data: None,
            summary: None,
            total_messages: 0,
            total_tokens: 0,
            cost: 0.0,
            tags: None,
            is_archived: false,
            created_at: now,
            updated_at: now,
        }
    }
}

/// AI消息
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AiMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub metadata: Option<String>, // JSON对象
    pub token_count: Option<i32>,
    pub cost: Option<f64>,
    pub tool_calls: Option<String>, // JSON数组
    pub attachments: Option<String>, // JSON数组
    pub timestamp: DateTime<Utc>,
}

/// 收益记录
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Earning {
    pub id: String,
    pub submission_id: String,
    pub project_id: String,
    pub amount: f64,
    pub currency: String,
    pub earning_type: Option<String>,
    pub payment_status: String,
    pub payment_date: Option<DateTime<Utc>>,
    pub payment_method: Option<String>,
    pub tax_info: Option<String>, // JSON对象
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 配置
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Configuration {
    pub id: String,
    pub category: String,
    pub key: String,
    pub value: Option<String>,
    pub description: Option<String>,
    pub is_encrypted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建项目请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub platform: String,
    pub url: Option<String>,
    pub scope_domains: Option<Vec<String>>,
    pub scope_ips: Option<Vec<String>>,
    pub out_of_scope: Option<Vec<String>>,
    pub reward_range: Option<serde_json::Value>,
    pub difficulty_level: Option<i32>,
    pub priority: Option<i32>,
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}

impl From<CreateProjectRequest> for BountyProject {
    fn from(req: CreateProjectRequest) -> Self {
        let mut project = BountyProject::new(req.name, req.platform);
        project.url = req.url;
        project.difficulty_level = req.difficulty_level.unwrap_or(1);
        project.priority = req.priority.unwrap_or(1);
        project.notes = req.notes;
        
        if let Some(domains) = req.scope_domains {
            project.scope_domains = Some(serde_json::to_string(&domains).unwrap_or_default());
        }
        
        if let Some(ips) = req.scope_ips {
            project.scope_ips = Some(serde_json::to_string(&ips).unwrap_or_default());
        }
        
        if let Some(out_of_scope) = req.out_of_scope {
            project.out_of_scope = Some(serde_json::to_string(&out_of_scope).unwrap_or_default());
        }
        
        if let Some(reward_range) = req.reward_range {
            project.reward_range = Some(serde_json::to_string(&reward_range).unwrap_or_default());
        }
        
        if let Some(tags) = req.tags {
            project.tags = Some(serde_json::to_string(&tags).unwrap_or_default());
        }
        
        project
    }
}

/// 创建扫描任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScanTaskRequest {
    pub name: String,
    pub description: Option<String>,
    pub project_id: Option<String>,
    pub target_type: String,
    pub targets: Vec<String>,
    pub scan_type: String,
    pub tools_config: Option<serde_json::Value>,
    pub priority: Option<i32>,
    pub scheduled_at: Option<DateTime<Utc>>,
}

impl From<CreateScanTaskRequest> for ScanTask {
    fn from(req: CreateScanTaskRequest) -> Self {
        let mut task = ScanTask::new(req.name, req.target_type, req.targets, req.scan_type);
        task.description = req.description;
        task.project_id = req.project_id;
        task.priority = req.priority.unwrap_or(1);
        task.scheduled_at = req.scheduled_at;
        
        if let Some(config) = req.tools_config {
            task.tools_config = Some(serde_json::to_string(&config).unwrap_or_default());
        }
        
        task
    }
}

/// 创建漏洞请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVulnerabilityRequest {
    pub title: String,
    pub description: Option<String>,
    pub project_id: Option<String>,
    pub asset_id: Option<String>,
    pub scan_task_id: Option<String>,
    pub vulnerability_type: Option<String>,
    pub severity: String,
    pub cvss_score: Option<f64>,
    pub cvss_vector: Option<String>,
    pub cwe_id: Option<String>,
    pub owasp_category: Option<String>,
    pub proof_of_concept: Option<String>,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub references: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}

impl From<CreateVulnerabilityRequest> for Vulnerability {
    fn from(req: CreateVulnerabilityRequest) -> Self {
        let mut vuln = Vulnerability::new(req.title, req.severity);
        vuln.description = req.description;
        vuln.project_id = req.project_id;
        vuln.asset_id = req.asset_id;
        vuln.scan_task_id = req.scan_task_id;
        vuln.vulnerability_type = req.vulnerability_type;
        vuln.cvss_score = req.cvss_score;
        vuln.cvss_vector = req.cvss_vector;
        vuln.cwe_id = req.cwe_id;
        vuln.owasp_category = req.owasp_category;
        vuln.proof_of_concept = req.proof_of_concept;
        vuln.impact = req.impact;
        vuln.remediation = req.remediation;
        vuln.notes = req.notes;
        
        if let Some(references) = req.references {
            vuln.references = Some(serde_json::to_string(&references).unwrap_or_default());
        }
        
        if let Some(tags) = req.tags {
            vuln.tags = Some(serde_json::to_string(&tags).unwrap_or_default());
        }
        
        vuln
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub connection_type: String,
    pub command: String,
    pub args: String,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
} 
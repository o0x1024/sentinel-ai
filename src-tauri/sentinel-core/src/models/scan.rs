use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 扫描任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanTask {
    pub id: Uuid,
    pub name: String,
    pub target: String,
    pub config: ScanConfig,
    pub status: ScanStatus,
    pub progress: f32, // 0.0 - 1.0
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub results_count: u32,
    pub vulnerabilities_found: u32,
    pub error_message: Option<String>,
    pub current_step: Option<String>,
    pub steps: Vec<ScanStep>,
}

/// 扫描步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStep {
    pub name: String,
    pub description: String,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// 扫描配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub name: Option<String>,
    pub tools: Vec<String>,
    pub depth: u32,
    pub timeout: u64, // 秒
    pub concurrent_scans: u32,
    pub include_subdomains: bool,
    pub port_range: Option<String>,
    pub custom_wordlists: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub notification_webhook: Option<String>,
}

/// 扫描状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanStatus {
    Pending,   // 等待中
    Running,   // 运行中
    Paused,    // 暂停
    Completed, // 完成
    Failed,    // 失败
    Cancelled, // 取消
}

/// 扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub id: Uuid,
    pub task_id: Uuid,
    pub tool_name: String,
    pub target: String,
    pub result_type: ResultType,
    pub data: serde_json::Value,
    pub severity: Severity,
    pub confidence: f32, // 0.0 - 1.0
    pub discovered_at: DateTime<Utc>,
    pub verified: bool,
    pub false_positive: bool,
}

/// 结果类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResultType {
    Subdomain,
    Port,
    Service,
    Vulnerability,
    Technology,
    Certificate,
    Directory,
    File,
    Header,
    Cookie,
}

/// 严重性等级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Severity::Info => "info",
            Severity::Low => "low",
            Severity::Medium => "medium",
            Severity::High => "high",
            Severity::Critical => "critical",
        };
        write!(f, "{}", s)
    }
}

impl Severity {
    pub fn to_lowercase(&self) -> String {
        self.to_string()
    }

    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "info" => Severity::Info,
            "low" => Severity::Low,
            "medium" => Severity::Medium,
            "high" => Severity::High,
            "critical" => Severity::Critical,
            _ => Severity::Info,
        }
    }
}

/// 资产信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: Uuid,
    pub domain: String,
    pub ip_address: Option<String>,
    pub asset_type: AssetType,
    pub technologies: Vec<String>,
    pub ports: Vec<u16>,
    pub services: Vec<Service>,
    pub last_scanned: Option<DateTime<Utc>>,
    pub active: bool,
}

/// 资产类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetType {
    Domain,
    Subdomain,
    IpAddress,
    Url,
    Application,
}

/// 服务信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub port: u16,
    pub protocol: String,
    pub service_name: String,
    pub version: Option<String>,
    pub banner: Option<String>,
}

/// 任务统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStats {
    pub total: usize,
    pub running: usize,
    pub pending: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}

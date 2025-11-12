//! 被动扫描核心类型定义
pub use sentinel_plugins::{Finding, RequestContext, ResponseContext, Severity, Confidence, PluginMetadata};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStatus { pub running: bool, pub port: u16, pub mitm_enabled: bool, pub stats: ProxyStats }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyStats { pub http_requests: u64, pub https_requests: u64, pub errors: u64, pub qps: f64 }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSession { pub id: String, pub started_at: DateTime<Utc>, pub ended_at: Option<DateTime<Utc>>, pub enabled_plugins: Vec<String>, pub http_total: u64, pub https_total: u64, pub findings_total: u64 }

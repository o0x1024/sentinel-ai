//! 插件系统类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// 插件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// 插件 ID（唯一标识）
    pub id: String,
    /// 插件名称
    pub name: String,
    /// 版本号
    pub version: String,
    /// 作者
    pub author: Option<String>,
    /// 主分类 (traffic/agent)
    #[serde(default = "default_main_category")]
    pub main_category: String,
    /// 子分类 (vulnerability/injection/xss/scanner/analyzer/reporter)
    pub category: String,
    /// 默认严重等级
    pub default_severity: Severity,
    /// 标签
    pub tags: Vec<String>,
    /// 描述
    pub description: Option<String>,
}

fn default_main_category() -> String {
    "traffic".to_string()
}

/// 严重等级
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Critical => write!(f, "critical"),
            Severity::High => write!(f, "high"),
            Severity::Medium => write!(f, "medium"),
            Severity::Low => write!(f, "low"),
            Severity::Info => write!(f, "info"),
        }
    }
}

/// 置信度
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::High => write!(f, "high"),
            Confidence::Medium => write!(f, "medium"),
            Confidence::Low => write!(f, "low"),
        }
    }
}

/// 请求上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub id: String,
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub content_type: Option<String>,
    pub query_params: HashMap<String, String>,
    pub is_https: bool,
    pub timestamp: DateTime<Utc>,
    /// 是否经过拦截修改
    #[serde(default)]
    pub was_edited: bool,
    /// 修改后的方法（如果经过拦截修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_method: Option<String>,
    /// 修改后的 URL（如果经过拦截修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_url: Option<String>,
    /// 修改后的请求头（如果经过拦截修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_headers: Option<HashMap<String, String>>,
    /// 修改后的请求体（如果经过拦截修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_body: Option<Vec<u8>>,
}

/// 响应上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseContext {
    pub request_id: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub content_type: Option<String>,
    pub timestamp: DateTime<Utc>,
    /// 是否经过拦截修改
    #[serde(default)]
    pub was_edited: bool,
    /// 修改后的状态码（如果经过拦截修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_status: Option<u16>,
    /// 修改后的响应头（如果经过拦截修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_headers: Option<HashMap<String, String>>,
    /// 修改后的响应体（如果经过拦截修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_body: Option<Vec<u8>>,
}

/// HTTP 事务（包含请求和响应）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpTransaction {
    pub request: RequestContext,
    pub response: Option<ResponseContext>,
}

/// 漏洞发现
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub plugin_id: String,
    pub vuln_type: String,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub evidence: String,
    pub location: String,
    pub confidence: Confidence,
    pub cwe: Option<String>,
    pub owasp: Option<String>,
    pub remediation: Option<String>,
    pub url: String,
    pub method: String,
    pub created_at: DateTime<Utc>,
    // 完整请求/响应证据（用于展示）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_headers: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_status: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_headers: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
}

impl Finding {
    /// 计算 Finding 签名（用于去重）
    /// 签名基于: plugin_id + vuln_type + url + location + title
    /// title 包含了注入类型等详细信息，确保同一参数的不同漏洞类型不会被去重
    /// 注意：必须与 TrafficFinding::calculate_signature 保持一致
    pub fn calculate_signature(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.plugin_id.as_bytes());
        hasher.update(self.vuln_type.as_bytes());
        hasher.update(self.url.as_bytes());
        hasher.update(self.location.as_bytes());
        hasher.update(self.title.as_bytes()); // 添加 title 以区分同一参数的不同漏洞类型
        format!("{:x}", hasher.finalize())
    }
}

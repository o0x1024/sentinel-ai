//! 被动扫描 Tauri 事件
//!
//! 定义并发射被动扫描相关的事件：
//! - proxy:status - 代理状态变化
//! - scan:finding - 新漏洞发现
//! - scan:stats - 扫描统计更新
//! - plugin:changed - 插件状态变化

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use sentinel_passive::{Finding, ProxyStats};

/// 代理状态事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStatusEvent {
    pub running: bool,
    pub port: u16,
    pub mitm: bool,
    pub stats: ProxyStats,
}

/// 漏洞发现事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingEvent {
    pub vuln_id: String,
    pub vuln_type: String,
    pub severity: String,
    pub url: String,
    pub summary: String,
    pub timestamp: String,
}

impl From<Finding> for FindingEvent {
    fn from(finding: Finding) -> Self {
        let vuln_type = finding.vuln_type.clone();
        let description = finding.description.clone();
        
        Self {
            vuln_id: finding.id,
            vuln_type: vuln_type.clone(),
            severity: finding.severity.to_string(),
            url: finding.url,
            summary: format!("{} - {}", vuln_type, description),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// 扫描统计事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStatsEvent {
    pub requests: u64,
    pub responses: u64,
    pub qps: f64,
    pub findings: u64,
}

/// 插件变化事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginChangedEvent {
    pub plugin_id: String,
    pub enabled: bool,
    pub name: String,
}

/// 拦截请求事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptRequestEvent {
    pub id: String,
    pub method: String,
    pub url: String,
    pub path: String,
    pub protocol: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: i64,
}

/// 拦截响应事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptResponseEvent {
    pub id: String,
    pub request_id: String,
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: i64,
}

/// 发射代理状态事件
pub fn emit_proxy_status(app: &AppHandle, event: ProxyStatusEvent) {
    if let Err(e) = app.emit("proxy:status", event) {
        tracing::error!("Failed to emit proxy:status event: {}", e);
    }
}

/// 发射漏洞发现事件
pub fn emit_finding(app: &AppHandle, event: FindingEvent) {
    if let Err(e) = app.emit("scan:finding", event) {
        tracing::error!("Failed to emit scan:finding event: {}", e);
    }
}

/// 发射扫描统计事件
pub fn emit_scan_stats(app: &AppHandle, event: ScanStatsEvent) {
    if let Err(e) = app.emit("scan:stats", event) {
        tracing::error!("Failed to emit scan:stats event: {}", e);
    }
}

/// 发射插件变化事件
pub fn emit_plugin_changed(app: &AppHandle, event: PluginChangedEvent) {
    if let Err(e) = app.emit("plugin:changed", event) {
        tracing::error!("Failed to emit plugin:changed event: {}", e);
    }
}

/// 发射拦截请求事件
pub fn emit_intercept_request(app: &AppHandle, event: InterceptRequestEvent) {
    if let Err(e) = app.emit("intercept:request", event) {
        tracing::error!("Failed to emit intercept:request event: {}", e);
    }
}

/// 发射拦截响应事件
pub fn emit_intercept_response(app: &AppHandle, event: InterceptResponseEvent) {
    if let Err(e) = app.emit("intercept:response", event) {
        tracing::error!("Failed to emit intercept:response event: {}", e);
    }
}

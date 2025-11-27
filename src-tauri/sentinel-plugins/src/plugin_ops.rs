//! Deno Core Operations (Ops) for Plugin System
//!
//! 提供 JavaScript 插件可以调用的 Rust 函数，用于：
//! - 发送漏洞发现 (emit_finding)
//! - 日志输出 (log)

use deno_core::{extension, op2, OpState};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

use crate::types::{Confidence, Finding, Severity};

/// 插件执行上下文（用于收集插件发现的漏洞）
#[derive(Clone, Default)]
pub struct PluginContext {
    pub findings: Arc<Mutex<Vec<Finding>>>,
    pub last_result: Arc<Mutex<Option<serde_json::Value>>>,
}

impl PluginContext {
    pub fn new() -> Self {
        Self {
            findings: Arc::new(Mutex::new(Vec::new())),
            last_result: Arc::new(Mutex::new(None)),
        }
    }

    pub fn take_findings(&self) -> Vec<Finding> {
        let mut findings = self.findings.lock().unwrap();
        std::mem::take(&mut *findings)
    }

    pub fn take_last_result(&self) -> Option<serde_json::Value> {
        let mut last = self.last_result.lock().unwrap();
        std::mem::take(&mut *last)
    }
}

/// Finding 的 JavaScript 表示（用于序列化）
/// 插件调用 op_emit_finding 时使用的简化结构
/// 所有字段都是可选的，以支持不同格式的插件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsFinding {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub vuln_type: String,
    #[serde(default)]
    pub confidence: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub param_name: String,
    #[serde(default)]
    pub param_value: String,
    #[serde(default)]
    pub evidence: String,
    // 支持嵌套的 request/response 对象
    pub request: Option<JsRequest>,
    pub response: Option<JsResponse>,
    #[serde(default)]
    pub cwe: String,
    #[serde(default)]
    pub owasp: String,
    #[serde(default)]
    pub remediation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsRequest {
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsResponse {
    #[serde(default)]
    pub status: u16,
}

impl From<JsFinding> for Finding {
    fn from(js: JsFinding) -> Self {
        // 从 request 对象或顶层获取 url 和 method
        let url = if let Some(ref req) = js.request {
            if !req.url.is_empty() {
                req.url.clone()
            } else {
                js.url.clone()
            }
        } else {
            js.url.clone()
        };
        
        let method = if let Some(ref req) = js.request {
            if !req.method.is_empty() {
                req.method.clone()
            } else {
                js.method.clone()
            }
        } else {
            js.method.clone()
        };
        
        // 根据 param_name 和 param_value 构造 location
        let location = if !js.param_name.is_empty() {
            format!("param:{}", js.param_name)
        } else {
            String::from("unknown")
        };
        
        // 使用提供的 title 或构造默认 title
        let title = if !js.title.is_empty() {
            js.title.clone()
        } else if !js.description.is_empty() {
            js.description.lines().next().unwrap_or("Vulnerability detected").to_string()
        } else {
            format!("{} detected", if js.vuln_type.is_empty() { "Vulnerability" } else { &js.vuln_type })
        };
        
        // 构造 evidence（包含 param_value 如果存在）
        let evidence = if !js.evidence.is_empty() {
            js.evidence.clone()
        } else if !js.param_value.is_empty() {
            format!("Parameter value: {}", js.param_value)
        } else {
            String::new()
        };
        
        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            plugin_id: String::new(), // 将在 PluginEngine 中设置
            vuln_type: if js.vuln_type.is_empty() { "unknown".to_string() } else { js.vuln_type },
            severity: parse_severity(&js.severity),
            confidence: parse_confidence(&js.confidence),
            title,
            description: js.description,
            evidence,
            location,
            url,
            method,
            cwe: if js.cwe.is_empty() { None } else { Some(js.cwe) },
            owasp: if js.owasp.is_empty() { None } else { Some(js.owasp) },
            remediation: if js.remediation.is_empty() { None } else { Some(js.remediation) },
            created_at: chrono::Utc::now(),
            // 这些字段将在扫描流水线中填充（从 RequestContext/ResponseContext）
            request_headers: None,
            request_body: None,
            response_status: None,
            response_headers: None,
            response_body: None,
        }
    }
}

fn parse_severity(s: &str) -> Severity {
    match s.to_lowercase().as_str() {
        "critical" => Severity::Critical,
        "high" => Severity::High,
        "medium" => Severity::Medium,
        "low" => Severity::Low,
        "info" => Severity::Info,
        _ => Severity::Medium,
    }
}

fn parse_confidence(s: &str) -> Confidence {
    match s.to_lowercase().as_str() {
        "high" => Confidence::High,
        "medium" => Confidence::Medium,
        "low" => Confidence::Low,
        _ => Confidence::Medium,
    }
}

// ============================================================
// Deno Core Extension
// ============================================================

extension!(
    sentinel_plugin_ext,
    ops = [op_emit_finding, op_plugin_log, op_fetch, op_plugin_return],
    state = |state| {
        state.put(PluginContext::new());
    }
);

// ============================================================
// Operations
// ============================================================

/// Op: 发送漏洞发现
#[op2]
fn op_emit_finding(
    state: &mut OpState,
    #[serde] finding: JsFinding,
) -> bool {
    let ctx = state.borrow::<PluginContext>().clone();
    let rust_finding = Finding::from(finding.clone());

    debug!(
        "Plugin emitted finding: {:?}",
        rust_finding
    );

    ctx.findings.lock().unwrap().push(rust_finding);

    true
}

/// Op: 插件日志输出
#[op2(fast)]
fn op_plugin_log(
    #[string] level: String,
    #[string] message: String,
) {
    match level.to_lowercase().as_str() {
        "error" => error!("[Plugin] {}", message),
        "warn" => warn!("[Plugin] {}", message),
        "info" => info!("[Plugin] {}", message),
        "debug" => debug!("[Plugin] {}", message),
        _ => debug!("[Plugin] {}", message),
    }
}

    #[op2]
    fn op_plugin_return(state: &mut OpState, #[serde] value: serde_json::Value) -> bool {
        let ctx = state.borrow::<PluginContext>().clone();
        let mut last = ctx.last_result.lock().unwrap();
        *last = Some(value);
        true
    }

/// Fetch request options from JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchOptions {
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub timeout: Option<u64>, // timeout in milliseconds
}

/// Fetch response to JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResponse {
    pub success: bool,
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
    pub ok: bool,
    pub error: Option<String>,
}

/// Op: HTTP fetch (网络请求)
#[op2(async)]
#[serde]
async fn op_fetch(
    #[string] url: String,
    #[serde] options: Option<FetchOptions>,
) -> FetchResponse {
    debug!("[Plugin] Fetching URL: {}", url);
    
    let opts = options.unwrap_or_else(|| FetchOptions {
        method: "GET".to_string(),
        headers: std::collections::HashMap::new(),
        body: None,
        timeout: Some(30000), // 30s default
    });
    
    let method = opts.method.to_uppercase();
    let timeout_ms = opts.timeout.unwrap_or(30000);
    
    // Build reqwest client
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return FetchResponse {
                success: false,
                status: 0,
                headers: std::collections::HashMap::new(),
                body: String::new(),
                ok: false,
                error: Some(format!("Failed to build HTTP client: {}", e)),
            };
        }
    };
    
    // Build request
    let mut req_builder = match method.as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url),
        "HEAD" => client.head(&url),
        _ => client.get(&url),
    };
    
    // Add headers
    for (key, value) in opts.headers {
        req_builder = req_builder.header(&key, &value);
    }
    
    // Add body if present
    if let Some(body) = opts.body {
        req_builder = req_builder.body(body);
    }
    
    // Execute request
    let response = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            return FetchResponse {
                success: false,
                status: 0,
                headers: std::collections::HashMap::new(),
                body: String::new(),
                ok: false,
                error: Some(format!("HTTP request failed: {}", e)),
            };
        }
    };
    
    let status = response.status().as_u16();
    let ok = response.status().is_success();
    
    // Extract headers
    let mut headers = std::collections::HashMap::new();
    for (key, value) in response.headers() {
        if let Ok(v) = value.to_str() {
            headers.insert(key.to_string(), v.to_string());
        }
    }
    
    // Read body
    let body = match response.text().await {
        Ok(b) => b,
        Err(e) => {
            return FetchResponse {
                success: false,
                status,
                headers,
                body: String::new(),
                ok: false,
                error: Some(format!("Failed to read response body: {}", e)),
            };
        }
    };
    
    debug!("[Plugin] Fetch completed: {} (status: {})", url, status);
    
    FetchResponse {
        success: true,
        status,
        headers,
        body,
        ok,
        error: None,
    }
}

// ============================================================
// 测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_severity() {
        assert!(matches!(parse_severity("critical"), Severity::Critical));
        assert!(matches!(parse_severity("HIGH"), Severity::High));
        assert!(matches!(parse_severity("medium"), Severity::Medium));
        assert!(matches!(parse_severity("low"), Severity::Low));
        assert!(matches!(parse_severity("info"), Severity::Info));
        assert!(matches!(parse_severity("unknown"), Severity::Medium));
    }

    #[test]
    fn test_parse_confidence() {
        assert!(matches!(parse_confidence("HIGH"), Confidence::High));
        assert!(matches!(parse_confidence("medium"), Confidence::Medium));
        assert!(matches!(parse_confidence("low"), Confidence::Low));
        assert!(matches!(parse_confidence("unknown"), Confidence::Medium));
    }
}

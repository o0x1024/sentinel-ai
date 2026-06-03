//! Shared types and helpers for the plugin system.
//!
//! This module holds cross-cutting plugin data structures (findings, active-probe
//! events, TLS responses) and run-level fetch cancellation utilities used by
//! extensions and the plugin engine.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tokio::sync::oneshot;
use tracing::info;

use crate::plugin_finding_sanitizer::sanitize_response_body_for_evidence;
use crate::request_scheduler::PluginFetchPolicyKind;
use crate::types::{Confidence, Finding, Severity};

/// Finding 的 JavaScript 表示（用于序列化）
/// 插件调用 op_emit_finding 时使用的简化结构
/// 所有字段都是可选的，以支持不同格式的插件
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    #[serde(default)]
    pub headers: String,
    #[serde(default)]
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsResponse {
    #[serde(default)]
    pub status: u16,
    #[serde(default)]
    pub headers: String,
    #[serde(default)]
    pub body: String,
}

impl From<JsFinding> for Finding {
    fn from(js: JsFinding) -> Self {
        let request_ref = js.request.as_ref();
        let response_ref = js.response.as_ref();

        // 从 request 对象或顶层获取 url 和 method
        let url = if let Some(req) = request_ref {
            if !req.url.is_empty() {
                req.url.clone()
            } else {
                js.url.clone()
            }
        } else {
            js.url.clone()
        };

        let method = if let Some(req) = request_ref {
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
            js.description
                .lines()
                .next()
                .unwrap_or("Vulnerability detected")
                .to_string()
        } else {
            format!(
                "{} detected",
                if js.vuln_type.is_empty() {
                    "Vulnerability"
                } else {
                    &js.vuln_type
                }
            )
        };

        // 构造 evidence（包含 param_value 如果存在）
        let evidence = if !js.evidence.is_empty() {
            js.evidence.clone()
        } else if !js.param_value.is_empty() {
            format!("Parameter value: {}", js.param_value)
        } else {
            String::new()
        };

        let request_headers =
            request_ref.and_then(|req| normalize_optional_string(req.headers.clone()));
        let request_body = request_ref.and_then(|req| normalize_optional_string(req.body.clone()));
        let response_status = response_ref
            .map(|resp| resp.status)
            .filter(|status| *status > 0)
            .map(i32::from);
        let response_headers =
            response_ref.and_then(|resp| normalize_optional_string(resp.headers.clone()));
        let response_body = response_ref
            .and_then(|resp| normalize_optional_string(resp.body.clone()))
            .and_then(|body| {
                sanitize_response_body_for_evidence(response_headers.as_deref(), body)
            });

        Finding {
            id: uuid::Uuid::new_v4().to_string(),
            plugin_id: String::new(), // 将在 PluginEngine 中设置
            vuln_type: if js.vuln_type.is_empty() {
                "unknown".to_string()
            } else {
                js.vuln_type
            },
            severity: parse_severity(&js.severity),
            confidence: parse_confidence(&js.confidence),
            title,
            description: js.description,
            evidence,
            location,
            url,
            method,
            cwe: if js.cwe.is_empty() {
                None
            } else {
                Some(js.cwe)
            },
            owasp: if js.owasp.is_empty() {
                None
            } else {
                Some(js.owasp)
            },
            remediation: if js.remediation.is_empty() {
                None
            } else {
                Some(js.remediation)
            },
            created_at: chrono::Utc::now(),
            request_headers,
            request_body,
            response_status,
            response_headers,
            response_body,
        }
    }
}

fn normalize_optional_string(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveProbeRuntimeUpdate {
    #[serde(default)]
    pub request_id: String,
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub probe_label: Option<String>,
    #[serde(default)]
    pub target_name: Option<String>,
    #[serde(default)]
    pub target_path: Option<String>,
    #[serde(default)]
    pub target_location: Option<String>,
    #[serde(default)]
    pub probe_value: Option<String>,
    #[serde(default)]
    pub technique: Option<String>,
    #[serde(default)]
    pub probe_class: Option<String>,
    #[serde(default)]
    pub probe_priority: Option<i32>,
    #[serde(default)]
    pub cooldown_key: Option<String>,
    #[serde(default)]
    pub cooldown_wait_ms: Option<u64>,
    #[serde(default)]
    pub jitter_wait_ms: Option<u64>,
    #[serde(default)]
    pub total_wait_ms: Option<u64>,
    #[serde(default)]
    pub adaptive_penalty_ms: Option<u64>,
    #[serde(default)]
    pub status: Option<u16>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub target_count: Option<u32>,
    #[serde(default)]
    pub active_slots: Option<u32>,
    #[serde(default)]
    pub max_concurrent_per_host: Option<u32>,
    #[serde(default)]
    pub queue_depth: Option<u32>,
    #[serde(default)]
    pub response_elapsed_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveProbeEvent {
    pub plugin_id: Option<String>,
    pub traffic_request_id: Option<String>,
    pub request_id: String,
    pub phase: String,
    pub method: String,
    pub url: String,
    pub probe_label: Option<String>,
    pub target_name: Option<String>,
    pub target_path: Option<String>,
    pub target_location: Option<String>,
    pub probe_value: Option<String>,
    pub technique: Option<String>,
    pub probe_class: Option<String>,
    pub probe_priority: Option<i32>,
    pub cooldown_key: Option<String>,
    pub cooldown_wait_ms: Option<u64>,
    pub jitter_wait_ms: Option<u64>,
    pub total_wait_ms: Option<u64>,
    pub adaptive_penalty_ms: Option<u64>,
    pub status: Option<u16>,
    pub error: Option<String>,
    pub reason: Option<String>,
    pub target_count: Option<u32>,
    pub active_slots: Option<u32>,
    pub max_concurrent_per_host: Option<u32>,
    pub queue_depth: Option<u32>,
    pub response_elapsed_ms: Option<u64>,
    pub timestamp: String,
}

static FETCH_ABORTS: OnceLock<Mutex<HashMap<String, oneshot::Sender<()>>>> = OnceLock::new();

fn fetch_abort_cache() -> &'static Mutex<HashMap<String, oneshot::Sender<()>>> {
    FETCH_ABORTS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn cancel_plugin_fetch_requests_by_run(run_id: &str, reason: &str) -> usize {
    crate::runtime_events::suppress_monitor_progress_for_run(run_id);

    let terminated = crate::terminate_plugin_executions_by_run(run_id);
    if terminated > 0 {
        info!(
            "Requested termination for {} active plugin execution(s) in run {}",
            terminated, run_id
        );
    }

    let kinds = [
        PluginFetchPolicyKind::BountyFetch,
        PluginFetchPolicyKind::MonitorFetch,
        PluginFetchPolicyKind::AgentFetch,
        PluginFetchPolicyKind::TrafficActiveProbe,
        PluginFetchPolicyKind::PluginTestFetch,
    ];
    let mut cancelled = 0usize;

    for kind in kinds {
        let request_ids =
            crate::cancel_plugin_requests_by_run(kind, run_id, Some(reason.to_string()));
        cancelled += request_ids.len();
        for request_id in request_ids {
            let sender = fetch_abort_cache().lock().unwrap().remove(&request_id);
            if let Some(sender) = sender {
                let _ = sender.send(());
            }
        }
    }

    cancelled
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsCertificateResponse {
    pub success: bool,
    pub cert: Option<serde_json::Value>,
    pub error: Option<String>,
}

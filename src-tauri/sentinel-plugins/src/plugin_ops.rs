//! Deno Core Operations (Ops) for Plugin System
//!
//! 提供 JavaScript 插件可以调用的 Rust 函数，用于：
//! - 发送漏洞发现 (emit_finding)
//! - 日志输出 (log)
//! - HTTP 请求 (fetch)

use deno_core::{extension, op2, OpState};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;
use tracing::{debug, error, info, warn};
use x509_parser::extensions::GeneralName;
use x509_parser::parse_x509_certificate;

use crate::dictionary_runtime;
use crate::monitor_progress::{emit_plugin_monitor_progress, PluginMonitorProgressRequest};
use crate::network_scan::op_scan_ports;
use crate::runtime_config::get_plugin_runtime_settings;
use crate::runtime_events::emit_active_probe_event;
use crate::service_probe::op_get_service_probe_capabilities;
use crate::service_probe_runtime::op_probe_services;
use crate::types::{Confidence, Finding, Severity};
use crate::{
    cancel_active_probe, complete_active_probe, enqueue_active_probe, fail_active_probe,
    mark_active_probe_running, ActiveProbeRequest,
};

/// 插件执行上下文（用于收集插件发现的漏洞）
#[derive(Clone, Default)]
pub struct PluginContext {
    pub findings: Arc<Mutex<Vec<Finding>>>,
    pub last_result: Arc<Mutex<Option<serde_json::Value>>>,
    pub plugin_id: Arc<Mutex<Option<String>>>,
    pub traffic_request_id: Arc<Mutex<Option<String>>>,
    pub finding_sink: Arc<Mutex<Option<mpsc::UnboundedSender<Finding>>>>,
}

impl PluginContext {
    pub fn new() -> Self {
        Self {
            findings: Arc::new(Mutex::new(Vec::new())),
            last_result: Arc::new(Mutex::new(None)),
            plugin_id: Arc::new(Mutex::new(None)),
            traffic_request_id: Arc::new(Mutex::new(None)),
            finding_sink: Arc::new(Mutex::new(None)),
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

    pub fn set_plugin_id(&self, plugin_id: Option<String>) {
        let mut current = self.plugin_id.lock().unwrap();
        *current = plugin_id;
    }

    pub fn plugin_id(&self) -> Option<String> {
        self.plugin_id.lock().unwrap().clone()
    }

    pub fn set_traffic_request_id(&self, request_id: Option<String>) {
        let mut current = self.traffic_request_id.lock().unwrap();
        *current = request_id;
    }

    pub fn traffic_request_id(&self) -> Option<String> {
        self.traffic_request_id.lock().unwrap().clone()
    }

    pub fn set_finding_sink(&self, finding_sink: Option<mpsc::UnboundedSender<Finding>>) {
        let mut current = self.finding_sink.lock().unwrap();
        *current = finding_sink;
    }

    pub fn emit_finding(&self, finding: Finding) -> bool {
        if let Some(sink) = self.finding_sink.lock().unwrap().clone() {
            match sink.send(finding.clone()) {
                Ok(_) => {
                    let mut findings = self.findings.lock().unwrap();
                    findings.push(finding);
                    return true;
                }
                Err(error) => {
                    warn!("Failed to stream finding to traffic pipeline: {}", error);
                    return false;
                }
            }
        }

        false
    }
}

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
        let response_body =
            response_ref.and_then(|resp| normalize_optional_string(resp.body.clone()));

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

// ============================================================
// Deno Core Extension
// ============================================================

extension!(
    sentinel_plugin_ext,
    ops = [
        op_plugin_log,
        op_emit_finding,
        op_plugin_return,
        op_fetch,
        op_abort_fetch,
        op_tls_peer_certificate,
        op_get_tls_certificate,
        op_scan_ports,
        op_probe_services,
        op_get_service_probe_capabilities,
        op_report_monitor_progress,
        op_emit_active_probe_event,
        op_get_plugin_runtime_settings,
        // File system operations
        op_read_text_file,
        op_write_text_file,
        op_read_file,
        op_write_file,
        op_mkdir,
        op_read_dir,
        op_stat,
        op_copy_file,
        op_remove,
        op_make_temp_file,
        // Dictionary operations
        op_get_dictionary,
        op_get_default_dictionary_id,
        op_get_dictionary_words,
        op_get_dictionary_entries,
        op_list_dictionaries,
        // JavaScript AST parsing
        op_parse_js,
    ],
    esm_entry_point = "ext:sentinel_plugin_ext/plugin_bootstrap.js",
    esm = [ dir "src", "plugin_bootstrap.js", "active_probe_runtime.js" ],
    state = |state| {
        state.put(PluginContext::new());
    }
);

// ============================================================
// Operations
// ============================================================

/// Op: 插件日志输出
#[op2(fast)]
fn op_plugin_log(#[string] level: String, #[string] message: String) {
    match level.to_lowercase().as_str() {
        "error" => error!("[Plugin] {}", message),
        "warn" => warn!("[Plugin] {}", message),
        "info" => info!("[Plugin] {}", message),
        "debug" => debug!("[Plugin] {}", message),
        _ => debug!("[Plugin] {}", message),
    }
}

#[op2]
fn op_emit_finding(state: &mut OpState, #[serde] finding: JsFinding) -> bool {
    let ctx = state.borrow::<PluginContext>().clone();
    let mut finding: Finding = finding.into();

    if finding.plugin_id.is_empty() {
        finding.plugin_id = ctx
            .plugin_id()
            .unwrap_or_else(|| "unknown-plugin".to_string());
    }

    ctx.emit_finding(finding)
}

#[op2]
fn op_plugin_return(state: &mut OpState, #[serde] value: serde_json::Value) -> bool {
    let ctx = state.borrow::<PluginContext>().clone();
    let mut last = ctx.last_result.lock().unwrap();
    *last = Some(value);
    true
}

#[op2]
fn op_report_monitor_progress(#[serde] request: PluginMonitorProgressRequest) -> bool {
    let Some(context) = request.monitor_progress else {
        return false;
    };

    emit_plugin_monitor_progress(&context, request.update);
    true
}

#[op2]
fn op_emit_active_probe_event(
    state: &mut OpState,
    #[serde] update: ActiveProbeRuntimeUpdate,
) -> bool {
    if update.request_id.trim().is_empty()
        || update.phase.trim().is_empty()
        || update.url.trim().is_empty()
    {
        return false;
    }

    let ctx = state.borrow::<PluginContext>().clone();
    let payload = ActiveProbeEvent {
        plugin_id: ctx.plugin_id(),
        traffic_request_id: ctx.traffic_request_id(),
        request_id: update.request_id,
        phase: update.phase,
        method: update.method,
        url: update.url,
        probe_label: update.probe_label,
        target_name: update.target_name,
        target_path: update.target_path,
        target_location: update.target_location,
        probe_value: update.probe_value,
        technique: update.technique,
        probe_class: update.probe_class,
        probe_priority: update.probe_priority,
        cooldown_key: update.cooldown_key,
        cooldown_wait_ms: update.cooldown_wait_ms,
        jitter_wait_ms: update.jitter_wait_ms,
        total_wait_ms: update.total_wait_ms,
        adaptive_penalty_ms: update.adaptive_penalty_ms,
        status: update.status,
        error: update.error,
        reason: update.reason,
        target_count: update.target_count,
        active_slots: update.active_slots,
        max_concurrent_per_host: update.max_concurrent_per_host,
        queue_depth: update.queue_depth,
        response_elapsed_ms: update.response_elapsed_ms,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    emit_active_probe_event(&payload);
    true
}

#[op2]
#[serde]
fn op_get_plugin_runtime_settings() -> crate::runtime_config::PluginRuntimeSettings {
    get_plugin_runtime_settings()
}

/// Fetch request options from JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FetchBody {
    Text { text: String },
    Bytes { bytes: Vec<u8> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchOptions {
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub body: Option<FetchBody>,
    #[serde(default)]
    pub timeout: Option<u64>, // timeout in milliseconds
    #[serde(default)]
    pub redirect: Option<String>,
    #[serde(default)]
    pub max_redirects: Option<usize>,
    #[serde(default)]
    pub max_body_bytes: Option<usize>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub active_probe: Option<ActiveProbeFetchOptions>,
}

/// Fetch response to JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResponse {
    pub success: bool,
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
    pub ok: bool,
    pub redirected: bool,
    pub final_url: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveProbeFetchOptions {
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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct FetchClientKey {
    proxy_url: Option<String>,
    follow_redirects: bool,
    max_redirects: usize,
}

static FETCH_CLIENTS: OnceLock<Mutex<HashMap<FetchClientKey, reqwest::Client>>> = OnceLock::new();
static FETCH_ABORTS: OnceLock<Mutex<HashMap<String, oneshot::Sender<()>>>> = OnceLock::new();

fn fetch_client_cache() -> &'static Mutex<HashMap<FetchClientKey, reqwest::Client>> {
    FETCH_CLIENTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn fetch_abort_cache() -> &'static Mutex<HashMap<String, oneshot::Sender<()>>> {
    FETCH_ABORTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn build_active_probe_cooldown_key(url: &str) -> String {
    let Ok(parsed) = deno_core::url::Url::parse(url) else {
        return "global".to_string();
    };

    let Some(host) = parsed.host_str().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }) else {
        return "global".to_string();
    };

    let path = if parsed.path().is_empty() {
        "/"
    } else {
        parsed.path()
    };

    match parsed.port() {
        Some(port) => format!("{host}:{port}{path}"),
        None => format!("{host}{path}"),
    }
}

async fn read_response_body(
    mut response: reqwest::Response,
    max_body_bytes: Option<usize>,
) -> Result<String, reqwest::Error> {
    if let Some(limit) = max_body_bytes.filter(|value| *value > 0) {
        let mut body = Vec::with_capacity(limit.min(8192));
        while let Some(chunk) = response.chunk().await? {
            let remaining = limit.saturating_sub(body.len());
            if remaining == 0 {
                break;
            }
            if chunk.len() <= remaining {
                body.extend_from_slice(&chunk);
                continue;
            }
            body.extend_from_slice(&chunk[..remaining]);
            break;
        }
        return Ok(String::from_utf8_lossy(&body).into_owned());
    }

    response.text().await
}

async fn get_fetch_client(
    follow_redirects: bool,
    max_redirects: usize,
) -> Result<reqwest::Client, reqwest::Error> {
    let proxy_config = sentinel_core::global_proxy::get_global_proxy().await;
    let key = FetchClientKey {
        proxy_url: if proxy_config.enabled {
            proxy_config.build_proxy_url()
        } else {
            None
        },
        follow_redirects,
        max_redirects,
    };

    if let Some(client) = fetch_client_cache().lock().unwrap().get(&key).cloned() {
        return Ok(client);
    }

    let mut default_headers = reqwest::header::HeaderMap::new();
    default_headers.insert(
        reqwest::header::HeaderName::from_static("x-sentinel-internal"),
        reqwest::header::HeaderValue::from_static("true"),
    );

    let redirect_policy = if follow_redirects {
        reqwest::redirect::Policy::limited(max_redirects)
    } else {
        reqwest::redirect::Policy::none()
    };

    let mut builder = reqwest::Client::builder()
        .default_headers(default_headers)
        .redirect(redirect_policy);

    if let Some(proxy_url) = &key.proxy_url {
        match reqwest::Proxy::all(proxy_url) {
            Ok(proxy) => {
                builder = builder.proxy(proxy);
            }
            Err(error) => {
                warn!(
                    "Failed to create proxy for plugin fetch client: {}, using direct connection",
                    error
                );
            }
        }
    }

    let client = builder.build()?;
    fetch_client_cache()
        .lock()
        .unwrap()
        .insert(key, client.clone());
    Ok(client)
}

#[op2(fast)]
fn op_abort_fetch(#[string] request_id: String) -> bool {
    let request_id = request_id.trim();
    if request_id.is_empty() {
        return false;
    }

    let sender = fetch_abort_cache().lock().unwrap().remove(request_id);
    if let Some(sender) = sender {
        let _ = sender.send(());
        return true;
    }

    false
}

/// Op: HTTP fetch (网络请求)
#[op2(async)]
#[serde]
async fn op_fetch(
    state: Rc<RefCell<OpState>>,
    #[string] url: String,
    #[serde] options: Option<FetchOptions>,
) -> FetchResponse {
    // info!("[Plugin] Fetching URL: {}", url);

    let opts = options.unwrap_or_else(|| FetchOptions {
        method: "GET".to_string(),
        headers: std::collections::HashMap::new(),
        body: None,
        timeout: Some(3000), // 5s default
        redirect: Some("follow".to_string()),
        max_redirects: Some(10),
        max_body_bytes: None,
        request_id: None,
        active_probe: None,
    });

    let method = opts.method.to_uppercase();
    let follow_redirects = !matches!(opts.redirect.as_deref(), Some("manual"));
    let max_redirects = opts.max_redirects.unwrap_or(1);
    let max_body_bytes = opts.max_body_bytes.filter(|value| *value > 0);
    let request_id = opts
        .request_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let active_probe = opts.active_probe.clone();
    let runtime_settings = get_plugin_runtime_settings();
    let active_probe_defaults = runtime_settings.active_probe.clone();
    let timeout_ms = if active_probe.is_some() {
        active_probe_defaults.timeout_ms
    } else {
        opts.timeout.unwrap_or(3000)
    };
    let plugin_ctx = {
        let op_state = state.borrow();
        op_state.borrow::<PluginContext>().clone()
    };
    let client = match get_fetch_client(follow_redirects, max_redirects).await {
        Ok(c) => c,
        Err(e) => {
            return FetchResponse {
                success: false,
                status: 0,
                headers: std::collections::HashMap::new(),
                body: String::new(),
                ok: false,
                redirected: false,
                final_url: url.clone(),
                error: Some(format!("Failed to build HTTP client: {}", e)),
            };
        }
    };

    let (abort_tx, mut abort_rx) = oneshot::channel::<()>();
    let mut local_abort_tx = Some(abort_tx);
    if let Some(previous) = fetch_abort_cache()
        .lock()
        .unwrap()
        .insert(request_id.clone(), local_abort_tx.take().unwrap())
    {
        let _ = previous.send(());
    }

    let response = async {
        let active_probe_request = if let Some(active_probe) = active_probe {
            let Some(traffic_request_id) = plugin_ctx.traffic_request_id() else {
                return FetchResponse {
                    success: false,
                    status: 0,
                    headers: std::collections::HashMap::new(),
                    body: String::new(),
                    ok: false,
                    redirected: false,
                    final_url: url.clone(),
                    error: Some(
                        "activeProbe is only available in traffic scan context".to_string(),
                    ),
                };
            };

            Some(ActiveProbeRequest {
                plugin_id: plugin_ctx
                    .plugin_id()
                    .unwrap_or_else(|| "unknown-plugin".to_string()),
                traffic_request_id,
                request_id: request_id.clone(),
                method: method.clone(),
                url: url.clone(),
                probe_label: active_probe.probe_label,
                target_name: active_probe.target_name,
                target_path: active_probe.target_path,
                target_location: active_probe.target_location,
                probe_value: active_probe.probe_value,
                technique: active_probe.technique,
                probe_class: "fast".to_string(),
                probe_priority: 0,
                cooldown_key: build_active_probe_cooldown_key(&url),
                jitter_range: active_probe_defaults.jitter_range,
                min_host_cooldown_ms: active_probe_defaults.min_host_cooldown_ms,
                max_concurrent_per_host: active_probe_defaults.max_concurrent_per_host as u32,
            })
        } else {
            None
        };

        if let Some(active_probe_request) = active_probe_request.as_ref() {
            let dispatch_rx = enqueue_active_probe(active_probe_request.clone());
            let grant = tokio::select! {
                dispatch_result = dispatch_rx => match dispatch_result {
                    Ok(grant) => grant,
                    Err(_) => {
                        return FetchResponse {
                            success: false,
                            status: 0,
                            headers: std::collections::HashMap::new(),
                            body: String::new(),
                            ok: false,
                            redirected: false,
                            final_url: url.clone(),
                            error: Some("Active probe scheduler dropped dispatch grant".to_string()),
                        };
                    }
                },
                _ = &mut abort_rx => {
                    cancel_active_probe(&request_id, Some("aborted while queued".to_string()));
                    return FetchResponse {
                        success: false,
                        status: 0,
                        headers: std::collections::HashMap::new(),
                        body: String::new(),
                        ok: false,
                        redirected: false,
                        final_url: url.clone(),
                        error: Some("HTTP request aborted".to_string()),
                    };
                }
            };

            if grant.total_wait_ms > 0 {
                let wait = tokio::time::sleep(Duration::from_millis(grant.total_wait_ms));
                tokio::pin!(wait);
                tokio::select! {
                    _ = &mut wait => {}
                    _ = &mut abort_rx => {
                        cancel_active_probe(&request_id, Some("aborted while scheduled".to_string()));
                        return FetchResponse {
                            success: false,
                            status: 0,
                            headers: std::collections::HashMap::new(),
                            body: String::new(),
                            ok: false,
                            redirected: false,
                            final_url: url.clone(),
                            error: Some("HTTP request aborted".to_string()),
                        };
                    }
                }
            }

            mark_active_probe_running(&request_id);
        }

        let request_started_at = Instant::now();
        let request_future = async {
            let mut req_builder = match method.as_str() {
                "GET" => client.get(&url),
                "POST" => client.post(&url),
                "PUT" => client.put(&url),
                "DELETE" => client.delete(&url),
                "PATCH" => client.patch(&url),
                "HEAD" => client.head(&url),
                _ => client.get(&url),
            };

            for (key, value) in opts.headers {
                req_builder = req_builder.header(&key, &value);
            }

            req_builder = req_builder.timeout(Duration::from_millis(timeout_ms));

            if let Some(body) = opts.body {
                req_builder = match body {
                    FetchBody::Text { text } => req_builder.body(text),
                    FetchBody::Bytes { bytes } => req_builder.body(bytes),
                };
            }

            let response = match req_builder.send().await {
                Ok(r) => r,
                Err(e) => {
                    return FetchResponse {
                        success: false,
                        status: 0,
                        headers: std::collections::HashMap::new(),
                        body: String::new(),
                        ok: false,
                        redirected: false,
                        final_url: url.clone(),
                        error: Some(format!("HTTP request failed: {}", e)),
                    };
                }
            };

            let status = response.status().as_u16();
            let ok = response.status().is_success();
            let final_url = response.url().to_string();
            let redirected = final_url != url;

            let mut headers = std::collections::HashMap::new();
            for (key, value) in response.headers() {
                if let Ok(v) = value.to_str() {
                    headers.insert(key.to_string(), v.to_string());
                }
            }

            let body = match read_response_body(response, max_body_bytes).await {
                Ok(b) => b,
                Err(e) => {
                    return FetchResponse {
                        success: false,
                        status,
                        headers,
                        body: String::new(),
                        ok: false,
                        redirected,
                        final_url,
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
                redirected,
                final_url,
                error: None,
            }
        };

        let response = tokio::select! {
            result = request_future => result,
            _ = &mut abort_rx => FetchResponse {
                success: false,
                status: 0,
                headers: std::collections::HashMap::new(),
                body: String::new(),
                ok: false,
                redirected: false,
                final_url: url.clone(),
                error: Some("HTTP request aborted".to_string()),
            }
        };

        if active_probe_request.is_some() {
            let response_elapsed_ms = Some(request_started_at.elapsed().as_millis() as u64);
            if response.success {
                complete_active_probe(&request_id, Some(response.status), response_elapsed_ms);
            } else if response
                .error
                .as_deref()
                .map(|error| error.eq_ignore_ascii_case("HTTP request aborted"))
                .unwrap_or(false)
            {
                cancel_active_probe(&request_id, Some("aborted while running".to_string()));
            } else {
                fail_active_probe(
                    &request_id,
                    if response.status > 0 {
                        Some(response.status)
                    } else {
                        None
                    },
                    response.error.clone(),
                    response_elapsed_ms,
                );
            }
        }

        response
    }
    .await;

    fetch_abort_cache().lock().unwrap().remove(&request_id);
    drop(local_abort_tx);

    response
}

/// Stub op for TLS peer certificate (required by deno_net 02_tls.js)
/// Returns null as we don't support peer certificate retrieval
#[op2]
#[serde]
fn op_tls_peer_certificate(#[smi] _rid: u32, _detailed: bool) -> Option<serde_json::Value> {
    // Stub: return null for peer certificate
    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsCertificateResponse {
    pub success: bool,
    pub cert: Option<serde_json::Value>,
    pub error: Option<String>,
}

fn fingerprint_sha256(der: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(der);
    let digest = hasher.finalize();
    digest
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<String>>()
        .join(":")
}

#[op2(async)]
#[serde]
async fn op_get_tls_certificate(
    #[string] hostname: String,
    #[smi] port: u16,
    #[smi] timeout_ms: u32,
) -> TlsCertificateResponse {
    let timeout_ms = if timeout_ms == 0 {
        10_000
    } else {
        timeout_ms as u64
    };
    let port = if port == 0 { 443 } else { port };

    let addr = format!("{}:{}", hostname, port);

    let tcp_stream =
        match timeout(Duration::from_millis(timeout_ms), TcpStream::connect(&addr)).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => {
                return TlsCertificateResponse {
                    success: false,
                    cert: None,
                    error: Some(format!("TCP connect failed: {}", e)),
                };
            }
            Err(_) => {
                return TlsCertificateResponse {
                    success: false,
                    cert: None,
                    error: Some(format!("TCP connect timeout after {} ms", timeout_ms)),
                };
            }
        };

    let root_store = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));

    let server_name = match rustls::pki_types::ServerName::try_from(hostname.clone()) {
        Ok(name) => name,
        Err(_) => {
            return TlsCertificateResponse {
                success: false,
                cert: None,
                error: Some("Invalid hostname for TLS SNI".to_string()),
            };
        }
    };

    let tls_stream = match timeout(
        Duration::from_millis(timeout_ms),
        connector.connect(server_name, tcp_stream),
    )
    .await
    {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => {
            return TlsCertificateResponse {
                success: false,
                cert: None,
                error: Some(format!("TLS handshake failed: {}", e)),
            };
        }
        Err(_) => {
            return TlsCertificateResponse {
                success: false,
                cert: None,
                error: Some(format!("TLS handshake timeout after {} ms", timeout_ms)),
            };
        }
    };

    let (_socket, session) = tls_stream.get_ref();
    let certs = match session.peer_certificates() {
        Some(certs) if !certs.is_empty() => certs,
        _ => {
            return TlsCertificateResponse {
                success: false,
                cert: None,
                error: Some("No peer certificates presented".to_string()),
            };
        }
    };

    let leaf_der = certs[0].as_ref();
    let parsed = match parse_x509_certificate(leaf_der) {
        Ok((_, cert)) => cert,
        Err(e) => {
            return TlsCertificateResponse {
                success: false,
                cert: None,
                error: Some(format!("Failed to parse X509 certificate: {}", e)),
            };
        }
    };

    let alt_names = parsed
        .subject_alternative_name()
        .ok()
        .flatten()
        .map(|ext| {
            ext.value
                .general_names
                .iter()
                .filter_map(|name| match name {
                    GeneralName::DNSName(dns) => Some(dns.to_string()),
                    _ => None,
                })
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    let protocol = session
        .protocol_version()
        .map(|v| format!("{:?}", v))
        .unwrap_or_else(|| "Unknown".to_string());

    let cipher = session
        .negotiated_cipher_suite()
        .map(|s| format!("{:?}", s.suite()))
        .unwrap_or_else(|| "Unknown".to_string());

    let valid_from = chrono::DateTime::<chrono::Utc>::from_timestamp(
        parsed.validity().not_before.timestamp(),
        0,
    )
    .map(|dt| dt.to_rfc3339())
    .unwrap_or_else(|| parsed.validity().not_before.to_string());

    let valid_to =
        chrono::DateTime::<chrono::Utc>::from_timestamp(parsed.validity().not_after.timestamp(), 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| parsed.validity().not_after.to_string());

    let cert_json = serde_json::json!({
        "subject": parsed.subject().to_string(),
        "issuer": parsed.issuer().to_string(),
        "validFrom": valid_from,
        "validTo": valid_to,
        "fingerprint": fingerprint_sha256(leaf_der),
        "serialNumber": parsed.raw_serial_as_string(),
        "altNames": alt_names,
        "protocol": protocol,
        "cipher": cipher,
    });

    TlsCertificateResponse {
        success: true,
        cert: Some(cert_json),
        error: None,
    }
}

// ============================================================
// File System Operations (Custom Implementation)
// ============================================================

/// Read text file
#[op2(async)]
#[string]
async fn op_read_text_file(#[string] path: String) -> Result<String, std::io::Error> {
    tokio::fs::read_to_string(&path).await
}

/// Write text file
#[op2(async)]
async fn op_write_text_file(
    #[string] path: String,
    #[string] content: String,
) -> Result<(), std::io::Error> {
    tokio::fs::write(&path, content).await
}

/// Read binary file
#[op2(async)]
#[buffer]
async fn op_read_file(#[string] path: String) -> Result<Vec<u8>, std::io::Error> {
    tokio::fs::read(&path).await
}

/// Write binary file
#[op2(async)]
async fn op_write_file(
    #[string] path: String,
    #[buffer(copy)] data: Vec<u8>,
) -> Result<(), std::io::Error> {
    tokio::fs::write(&path, data).await
}

/// Create directory
#[op2(async)]
async fn op_mkdir(#[string] path: String, recursive: bool) -> Result<(), std::io::Error> {
    if recursive {
        tokio::fs::create_dir_all(&path).await
    } else {
        tokio::fs::create_dir(&path).await
    }
}

/// Directory entry
#[derive(Serialize)]
struct DirEntry {
    name: String,
    is_file: bool,
    is_directory: bool,
    is_symlink: bool,
}

/// Read directory contents
#[op2(async)]
#[serde]
async fn op_read_dir(#[string] path: String) -> Result<Vec<DirEntry>, std::io::Error> {
    let mut entries = Vec::new();
    let mut read_dir = tokio::fs::read_dir(&path).await?;

    while let Some(entry) = read_dir.next_entry().await? {
        let metadata = entry.metadata().await?;
        entries.push(DirEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            is_file: metadata.is_file(),
            is_directory: metadata.is_dir(),
            is_symlink: metadata.is_symlink(),
        });
    }

    Ok(entries)
}

/// File information
#[derive(Serialize)]
struct FileInfo {
    size: u64,
    is_file: bool,
    is_directory: bool,
    is_symlink: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    mtime: Option<u64>,
}

/// Get file information
#[op2(async)]
#[serde]
async fn op_stat(#[string] path: String) -> Result<FileInfo, std::io::Error> {
    let metadata = tokio::fs::metadata(&path).await?;

    let mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64);

    Ok(FileInfo {
        size: metadata.len(),
        is_file: metadata.is_file(),
        is_directory: metadata.is_dir(),
        is_symlink: metadata.is_symlink(),
        mtime,
    })
}

/// Copy file
#[op2(async)]
async fn op_copy_file(#[string] from: String, #[string] to: String) -> Result<(), std::io::Error> {
    tokio::fs::copy(&from, &to).await.map(|_| ())
}

/// Remove file or directory
#[op2(async)]
async fn op_remove(#[string] path: String, recursive: bool) -> Result<(), std::io::Error> {
    let metadata = tokio::fs::metadata(&path).await?;

    if metadata.is_dir() {
        if recursive {
            tokio::fs::remove_dir_all(&path).await
        } else {
            tokio::fs::remove_dir(&path).await
        }
    } else {
        tokio::fs::remove_file(&path).await
    }
}

/// Create temporary file
#[op2(async)]
#[string]
async fn op_make_temp_file(
    #[string] prefix: String,
    #[string] suffix: String,
) -> Result<String, std::io::Error> {
    let temp_dir = std::env::temp_dir();
    let file_name = format!("{}{}{}", prefix, uuid::Uuid::new_v4(), suffix);
    let temp_path = temp_dir.join(file_name);

    // Create the file
    std::fs::File::create(&temp_path)?;

    Ok(temp_path.to_string_lossy().to_string())
}

// ============================================================
// JavaScript AST Parsing Operations
// ============================================================

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::SourceType;

/// String literal extracted from JavaScript AST
#[derive(Debug, Clone, Serialize)]
struct JsStringLiteral {
    value: String,
    line: u32,
    column: u32,
    #[serde(rename = "type")]
    literal_type: String, // "string", "template", "regex"
}

/// AST parsing result
#[derive(Debug, Clone, Serialize)]
struct JsParseResult {
    success: bool,
    literals: Vec<JsStringLiteral>,
    errors: Vec<String>,
}

/// Op: Parse JavaScript code and extract all string literals using oxc_parser
#[op2]
#[serde]
fn op_parse_js(#[string] code: String, #[string] filename: Option<String>) -> JsParseResult {
    let allocator = Allocator::default();
    let source_type =
        SourceType::from_path(filename.as_deref().unwrap_or("script.js")).unwrap_or_default();

    let parser_ret = Parser::new(&allocator, &code, source_type).parse();

    let mut literals = Vec::new();
    let mut errors = Vec::new();

    // Collect parse errors
    for error in parser_ret.errors.iter() {
        errors.push(format!("{}", error));
    }

    // Extract literals from AST
    extract_literals_from_program(&parser_ret.program, &code, &mut literals);

    JsParseResult {
        success: parser_ret.errors.is_empty(),
        literals,
        errors,
    }
}

/// Extract all string literals from the AST program
fn extract_literals_from_program(
    program: &Program,
    source: &str,
    literals: &mut Vec<JsStringLiteral>,
) {
    // Walk through all statements
    for stmt in &program.body {
        extract_literals_from_statement(stmt, source, literals);
    }
}

/// Extract literals from a statement
fn extract_literals_from_statement(
    stmt: &Statement,
    source: &str,
    literals: &mut Vec<JsStringLiteral>,
) {
    match stmt {
        Statement::ExpressionStatement(expr_stmt) => {
            extract_literals_from_expression(&expr_stmt.expression, source, literals);
        }
        Statement::BlockStatement(block) => {
            for s in &block.body {
                extract_literals_from_statement(s, source, literals);
            }
        }
        Statement::IfStatement(if_stmt) => {
            extract_literals_from_expression(&if_stmt.test, source, literals);
            extract_literals_from_statement(&if_stmt.consequent, source, literals);
            if let Some(alt) = &if_stmt.alternate {
                extract_literals_from_statement(alt, source, literals);
            }
        }
        Statement::WhileStatement(while_stmt) => {
            extract_literals_from_expression(&while_stmt.test, source, literals);
            extract_literals_from_statement(&while_stmt.body, source, literals);
        }
        Statement::DoWhileStatement(do_while) => {
            extract_literals_from_statement(&do_while.body, source, literals);
            extract_literals_from_expression(&do_while.test, source, literals);
        }
        Statement::ForStatement(for_stmt) => {
            if let Some(init) = &for_stmt.init {
                if let ForStatementInit::VariableDeclaration(decl) = init {
                    extract_literals_from_var_decl(decl, source, literals);
                }
            }
            if let Some(test) = &for_stmt.test {
                extract_literals_from_expression(test, source, literals);
            }
            if let Some(update) = &for_stmt.update {
                extract_literals_from_expression(update, source, literals);
            }
            extract_literals_from_statement(&for_stmt.body, source, literals);
        }
        Statement::ForInStatement(for_in) => {
            extract_literals_from_expression(&for_in.right, source, literals);
            extract_literals_from_statement(&for_in.body, source, literals);
        }
        Statement::ForOfStatement(for_of) => {
            extract_literals_from_expression(&for_of.right, source, literals);
            extract_literals_from_statement(&for_of.body, source, literals);
        }
        Statement::ReturnStatement(ret) => {
            if let Some(arg) = &ret.argument {
                extract_literals_from_expression(arg, source, literals);
            }
        }
        Statement::ThrowStatement(throw) => {
            extract_literals_from_expression(&throw.argument, source, literals);
        }
        Statement::TryStatement(try_stmt) => {
            for s in &try_stmt.block.body {
                extract_literals_from_statement(s, source, literals);
            }
            if let Some(handler) = &try_stmt.handler {
                for s in &handler.body.body {
                    extract_literals_from_statement(s, source, literals);
                }
            }
            if let Some(finalizer) = &try_stmt.finalizer {
                for s in &finalizer.body {
                    extract_literals_from_statement(s, source, literals);
                }
            }
        }
        Statement::SwitchStatement(switch) => {
            extract_literals_from_expression(&switch.discriminant, source, literals);
            for case in &switch.cases {
                if let Some(test) = &case.test {
                    extract_literals_from_expression(test, source, literals);
                }
                for s in &case.consequent {
                    extract_literals_from_statement(s, source, literals);
                }
            }
        }
        Statement::VariableDeclaration(decl) => {
            extract_literals_from_var_decl(decl, source, literals);
        }
        Statement::FunctionDeclaration(func) => {
            if let Some(body) = &func.body {
                for s in &body.statements {
                    extract_literals_from_statement(s, source, literals);
                }
            }
        }
        Statement::ClassDeclaration(class) => {
            extract_literals_from_class(&class.body, source, literals);
        }
        Statement::ExportDefaultDeclaration(export) => match &export.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(func) => {
                if let Some(body) = &func.body {
                    for s in &body.statements {
                        extract_literals_from_statement(s, source, literals);
                    }
                }
            }
            ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                extract_literals_from_class(&class.body, source, literals);
            }
            _ => {}
        },
        Statement::ExportNamedDeclaration(export) => {
            if let Some(decl) = &export.declaration {
                extract_literals_from_declaration(decl, source, literals);
            }
        }
        Statement::LabeledStatement(labeled) => {
            extract_literals_from_statement(&labeled.body, source, literals);
        }
        Statement::WithStatement(with_stmt) => {
            extract_literals_from_expression(&with_stmt.object, source, literals);
            extract_literals_from_statement(&with_stmt.body, source, literals);
        }
        _ => {}
    }
}

/// Extract literals from a declaration
fn extract_literals_from_declaration(
    decl: &Declaration,
    source: &str,
    literals: &mut Vec<JsStringLiteral>,
) {
    match decl {
        Declaration::VariableDeclaration(var_decl) => {
            extract_literals_from_var_decl(var_decl, source, literals);
        }
        Declaration::FunctionDeclaration(func) => {
            if let Some(body) = &func.body {
                for s in &body.statements {
                    extract_literals_from_statement(s, source, literals);
                }
            }
        }
        Declaration::ClassDeclaration(class) => {
            extract_literals_from_class(&class.body, source, literals);
        }
        _ => {}
    }
}

/// Extract literals from variable declaration
fn extract_literals_from_var_decl(
    decl: &VariableDeclaration,
    source: &str,
    literals: &mut Vec<JsStringLiteral>,
) {
    for declarator in &decl.declarations {
        if let Some(init) = &declarator.init {
            extract_literals_from_expression(init, source, literals);
        }
    }
}

/// Extract literals from class body
fn extract_literals_from_class(
    body: &ClassBody,
    source: &str,
    literals: &mut Vec<JsStringLiteral>,
) {
    for element in &body.body {
        match element {
            ClassElement::MethodDefinition(method) => {
                if let Some(func_body) = &method.value.body {
                    for s in &func_body.statements {
                        extract_literals_from_statement(s, source, literals);
                    }
                }
            }
            ClassElement::PropertyDefinition(prop) => {
                if let Some(value) = &prop.value {
                    extract_literals_from_expression(value, source, literals);
                }
            }
            ClassElement::StaticBlock(block) => {
                for s in &block.body {
                    extract_literals_from_statement(s, source, literals);
                }
            }
            _ => {}
        }
    }
}

/// Extract literals from an expression
fn extract_literals_from_expression(
    expr: &Expression,
    source: &str,
    literals: &mut Vec<JsStringLiteral>,
) {
    match expr {
        Expression::StringLiteral(lit) => {
            let (line, col) = get_line_col(source, lit.span.start as usize);
            if lit.value.len() >= 3 {
                literals.push(JsStringLiteral {
                    value: lit.value.to_string(),
                    line,
                    column: col,
                    literal_type: "string".to_string(),
                });
            }
        }
        Expression::TemplateLiteral(template) => {
            for quasi in &template.quasis {
                let value = quasi
                    .value
                    .cooked
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("");
                if value.len() >= 3 {
                    let (line, col) = get_line_col(source, quasi.span.start as usize);
                    literals.push(JsStringLiteral {
                        value: value.to_string(),
                        line,
                        column: col,
                        literal_type: "template".to_string(),
                    });
                }
            }
            for expr in &template.expressions {
                extract_literals_from_expression(expr, source, literals);
            }
        }
        Expression::RegExpLiteral(regex) => {
            let (line, col) = get_line_col(source, regex.span.start as usize);
            let pattern_str = regex.regex.pattern.text.as_str();
            if pattern_str.len() >= 3 {
                literals.push(JsStringLiteral {
                    value: pattern_str.to_string(),
                    line,
                    column: col,
                    literal_type: "regex".to_string(),
                });
            }
        }
        Expression::ArrayExpression(arr) => {
            for elem in &arr.elements {
                if let ArrayExpressionElement::SpreadElement(spread) = elem {
                    extract_literals_from_expression(&spread.argument, source, literals);
                } else if let ArrayExpressionElement::Elision(_) = elem {
                    // Skip elision
                } else {
                    // Expression
                    if let Some(expr) = elem.as_expression() {
                        extract_literals_from_expression(expr, source, literals);
                    }
                }
            }
        }
        Expression::ObjectExpression(obj) => {
            for prop in &obj.properties {
                match prop {
                    ObjectPropertyKind::ObjectProperty(p) => {
                        extract_literals_from_expression(&p.value, source, literals);
                        if let PropertyKey::StringLiteral(key) = &p.key {
                            let (line, col) = get_line_col(source, key.span.start as usize);
                            if key.value.len() >= 3 {
                                literals.push(JsStringLiteral {
                                    value: key.value.to_string(),
                                    line,
                                    column: col,
                                    literal_type: "string".to_string(),
                                });
                            }
                        }
                    }
                    ObjectPropertyKind::SpreadProperty(spread) => {
                        extract_literals_from_expression(&spread.argument, source, literals);
                    }
                }
            }
        }
        Expression::CallExpression(call) => {
            extract_literals_from_expression(&call.callee, source, literals);
            for arg in &call.arguments {
                if let Argument::SpreadElement(spread) = arg {
                    extract_literals_from_expression(&spread.argument, source, literals);
                } else if let Some(expr) = arg.as_expression() {
                    extract_literals_from_expression(expr, source, literals);
                }
            }
        }
        Expression::NewExpression(new_expr) => {
            extract_literals_from_expression(&new_expr.callee, source, literals);
            for arg in &new_expr.arguments {
                if let Argument::SpreadElement(spread) = arg {
                    extract_literals_from_expression(&spread.argument, source, literals);
                } else if let Some(expr) = arg.as_expression() {
                    extract_literals_from_expression(expr, source, literals);
                }
            }
        }
        Expression::ComputedMemberExpression(computed) => {
            extract_literals_from_expression(&computed.object, source, literals);
            extract_literals_from_expression(&computed.expression, source, literals);
        }
        Expression::StaticMemberExpression(static_member) => {
            extract_literals_from_expression(&static_member.object, source, literals);
        }
        Expression::PrivateFieldExpression(private) => {
            extract_literals_from_expression(&private.object, source, literals);
        }
        Expression::BinaryExpression(binary) => {
            extract_literals_from_expression(&binary.left, source, literals);
            extract_literals_from_expression(&binary.right, source, literals);
        }
        Expression::LogicalExpression(logical) => {
            extract_literals_from_expression(&logical.left, source, literals);
            extract_literals_from_expression(&logical.right, source, literals);
        }
        Expression::UnaryExpression(unary) => {
            extract_literals_from_expression(&unary.argument, source, literals);
        }
        Expression::ConditionalExpression(cond) => {
            extract_literals_from_expression(&cond.test, source, literals);
            extract_literals_from_expression(&cond.consequent, source, literals);
            extract_literals_from_expression(&cond.alternate, source, literals);
        }
        Expression::AssignmentExpression(assign) => {
            extract_literals_from_expression(&assign.right, source, literals);
        }
        Expression::SequenceExpression(seq) => {
            for expr in &seq.expressions {
                extract_literals_from_expression(expr, source, literals);
            }
        }
        Expression::ArrowFunctionExpression(arrow) => {
            // arrow.body is now FunctionBody directly
            for s in &arrow.body.statements {
                extract_literals_from_statement(s, source, literals);
            }
        }
        Expression::FunctionExpression(func) => {
            if let Some(body) = &func.body {
                for s in &body.statements {
                    extract_literals_from_statement(s, source, literals);
                }
            }
        }
        Expression::ClassExpression(class) => {
            extract_literals_from_class(&class.body, source, literals);
        }
        Expression::TaggedTemplateExpression(tagged) => {
            extract_literals_from_expression(&tagged.tag, source, literals);
            // Template literals
            for quasi in &tagged.quasi.quasis {
                let value = quasi
                    .value
                    .cooked
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("");
                if value.len() >= 3 {
                    let (line, col) = get_line_col(source, quasi.span.start as usize);
                    literals.push(JsStringLiteral {
                        value: value.to_string(),
                        line,
                        column: col,
                        literal_type: "template".to_string(),
                    });
                }
            }
            for expr in &tagged.quasi.expressions {
                extract_literals_from_expression(expr, source, literals);
            }
        }
        Expression::AwaitExpression(await_expr) => {
            extract_literals_from_expression(&await_expr.argument, source, literals);
        }
        Expression::YieldExpression(yield_expr) => {
            if let Some(arg) = &yield_expr.argument {
                extract_literals_from_expression(arg, source, literals);
            }
        }
        Expression::ParenthesizedExpression(paren) => {
            extract_literals_from_expression(&paren.expression, source, literals);
        }
        _ => {}
    }
}

/// Get line and column from byte offset
fn get_line_col(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 1u32;
    let mut col = 1u32;

    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

/// Get dictionary by ID or name
#[op2(async)]
#[serde]
async fn op_get_dictionary(
    #[string] id_or_name: String,
) -> Result<Option<dictionary_runtime::JsDictionary>, deno_error::JsErrorBox> {
    dictionary_runtime::get_dictionary(id_or_name).await
}

/// Get the configured default dictionary ID for a type
#[op2(async)]
#[string]
async fn op_get_default_dictionary_id(
    #[string] dict_type: String,
) -> Result<String, deno_error::JsErrorBox> {
    dictionary_runtime::get_default_dictionary_id(dict_type).await
}

/// Get words from a dictionary by ID or name
#[op2(async)]
#[serde]
async fn op_get_dictionary_words(
    #[string] id_or_name: String,
    #[smi] limit: Option<i32>,
) -> Result<Vec<String>, deno_error::JsErrorBox> {
    dictionary_runtime::get_dictionary_words(id_or_name, limit).await
}

/// Get structured entries from a dictionary by ID or name
#[op2(async)]
#[serde]
async fn op_get_dictionary_entries(
    #[string] id_or_name: String,
    #[smi] limit: Option<i32>,
) -> Result<Vec<dictionary_runtime::JsDictionaryEntry>, deno_error::JsErrorBox> {
    dictionary_runtime::get_dictionary_entries(id_or_name, limit).await
}

/// List dictionaries with optional filter
#[op2(async)]
#[serde]
async fn op_list_dictionaries(
    #[string] dict_type: Option<String>,
    #[string] category: Option<String>,
) -> Result<Vec<dictionary_runtime::JsDictionary>, deno_error::JsErrorBox> {
    dictionary_runtime::list_dictionaries(dict_type, category).await
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

    #[test]
    fn test_build_active_probe_cooldown_key_uses_host_and_path() {
        assert_eq!(
            build_active_probe_cooldown_key("https://example.com:8443/api/items?id=1"),
            "example.com:8443/api/items"
        );
        assert_eq!(
            build_active_probe_cooldown_key("https://example.com"),
            "example.com/"
        );
        assert_eq!(build_active_probe_cooldown_key("not-a-url"), "global");
    }
}

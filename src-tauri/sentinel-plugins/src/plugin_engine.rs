//! QuickJS-backed plugin runtime
//!
//! Provides JS/TS plugin execution with:
//! - ESM/TypeScript module loading (one_parser strips types, QuickJS executes)
//! - Plugin load and hot reload
//! - Host-function bridge to Rust plugin ops
//! - Memory and stack limits

use crate::error::{PluginError, Result};
use crate::permissions::PluginPermissions;
use crate::plugin_context::PluginContext;
use crate::plugin_fetch_types::{FetchBody, FetchOptions, FetchResponse};
use crate::plugin_ops::{ActiveProbeEvent, JsFinding};
use crate::sentinel_js_runtime::{self, SentinelJsRuntime};
use crate::runtime_config::get_plugin_runtime_settings;
use crate::runtime_events::emit_active_probe_event;
use crate::types::{Finding, PluginMetadata};
use std::collections::HashSet;
use std::path::PathBuf;
use tracing::debug;

pub(crate) use sentinel_js_runtime::with_plugin_ctx;

pub(crate) fn set_plugin_ctx(ctx: &PluginContext) {
    sentinel_js_runtime::set_plugin_ctx(ctx);
}

pub(crate) fn clear_plugin_ctx() {
    sentinel_js_runtime::clear_plugin_ctx();
}

fn sanitize_plugin_source(source: &str) -> String {
    let trimmed = source.trim();

    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }

    let mut lines = trimmed.lines();
    let _opening_fence = lines.next();

    let mut content_lines = Vec::new();
    for line in lines {
        if line.trim_start().starts_with("```") {
            break;
        }
        content_lines.push(line);
    }

    content_lines.join("\n").trim().to_string()
}

fn dedupe_findings(findings: Vec<Finding>) -> Vec<Finding> {
    let mut seen = HashSet::new();
    findings
        .into_iter()
        .filter(|finding| seen.insert(finding.calculate_signature()))
        .collect()
}

fn is_esm_module(code: &str) -> bool {
    let trimmed = code.trim();
    trimmed.contains("import ") || trimmed.contains("export ")
}

// ---------------------------------------------------------------------------
// AST literal extraction (uses one_parser, no One VM needed)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct JsParseResult {
    pub success: bool,
    pub literals: Vec<JsStringLiteral>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct JsStringLiteral {
    pub value: String,
    line: u32,
    column: u32,
    #[serde(rename = "type")]
    literal_type: String,
}

pub(crate) fn parse_js_literals(_code: String, _filename: Option<String>) -> JsParseResult {
    // AST literal extraction previously used one_parser.
    // TODO: Reimplement using oxc if needed.
    JsParseResult {
        success: true,
        literals: Vec::new(),
        errors: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// HTTP fetch (shared with sentinel_js_runtime host functions)
// ---------------------------------------------------------------------------

pub(crate) async fn plugin_fetch(url: String, options: FetchOptions) -> FetchResponse {
    use crate::plugin_fetch_context::{build_plugin_request_schedule, fetch_policy_kind_for_context};
    use crate::request_scheduler::configured_policy_for_kind;
    use crate::{
        complete_active_probe, enqueue_active_probe, fail_active_probe,
        mark_active_probe_running, ActiveProbeRequest,
    };
    use crate::{
        complete_plugin_request, enqueue_plugin_request, fail_plugin_request,
        mark_plugin_request_running,
    };
    use std::time::{Duration, Instant};
    use tokio::time::timeout;

    let method = options.method.to_uppercase();
    let follow_redirects = !matches!(options.redirect.as_deref(), Some("manual"));
    let max_redirects = options.max_redirects.unwrap_or(10);
    let max_body_bytes = options.max_body_bytes.filter(|value| *value > 0);
    let request_id = options
        .request_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let active_probe = options.active_probe.clone();
    let runtime_settings = get_plugin_runtime_settings();
    let active_probe_defaults = runtime_settings.active_probe.clone();
    let plugin_requested_timeout = options.timeout.unwrap_or(0);
    let timeout_ms = if active_probe.is_some() {
        active_probe_defaults.timeout_ms
    } else if plugin_requested_timeout > 0 {
        plugin_requested_timeout.clamp(1_000, 120_000)
    } else {
        8_000
    };

    let plugin_ctx = with_plugin_ctx(|ctx| ctx.clone());

    let plugin_fetch_schedule = if active_probe.is_none() {
        match fetch_policy_kind_for_context(&plugin_ctx) {
            Ok(Some(kind)) => {
                match build_plugin_request_schedule(&plugin_ctx, kind, &request_id, &method, &url)
                {
                    Ok(schedule) => Some(schedule),
                    Err(error) => {
                        return FetchResponse {
                            success: false,
                            status: 0,
                            headers: std::collections::HashMap::new(),
                            body: String::new(),
                            body_bytes: Vec::new(),
                            ok: false,
                            redirected: false,
                            final_url: url,
                            error: Some(error),
                        };
                    }
                }
            }
            Ok(None) => None,
            Err(error) => {
                return FetchResponse {
                    success: false,
                    status: 0,
                    headers: std::collections::HashMap::new(),
                    body: String::new(),
                    body_bytes: Vec::new(),
                    ok: false,
                    redirected: false,
                    final_url: url,
                    error: Some(error),
                };
            }
        }
    } else {
        None
    };

    let client = match get_fetch_client(follow_redirects, max_redirects).await {
        Ok(client) => client,
        Err(err) => {
            return FetchResponse {
                success: false,
                status: 0,
                headers: std::collections::HashMap::new(),
                body: String::new(),
                body_bytes: Vec::new(),
                ok: false,
                redirected: false,
                final_url: url.clone(),
                error: Some(format!("Failed to build HTTP client: {err}")),
            };
        }
    };

    let active_probe_request = if let Some(active_probe) = active_probe {
        let Some(traffic_request_id) = plugin_ctx.traffic_request_id() else {
            return FetchResponse {
                success: false,
                status: 0,
                headers: std::collections::HashMap::new(),
                body: String::new(),
                body_bytes: Vec::new(),
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

    let mut effective_timeout_ms = timeout_ms;
    if let Some(active_probe_request) = active_probe_request.as_ref() {
        let dispatch_rx = match enqueue_active_probe(active_probe_request.clone()) {
            Ok(dispatch_rx) => dispatch_rx,
            Err(error) => {
                return FetchResponse {
                    success: false,
                    status: 0,
                    headers: std::collections::HashMap::new(),
                    body: String::new(),
                    body_bytes: Vec::new(),
                    ok: false,
                    redirected: false,
                    final_url: url.clone(),
                    error: Some(error),
                };
            }
        };
        let grant = match dispatch_rx.await {
            Ok(Ok(grant)) => grant,
            Ok(Err(error)) => {
                return FetchResponse {
                    success: false,
                    status: 0,
                    headers: std::collections::HashMap::new(),
                    body: String::new(),
                    body_bytes: Vec::new(),
                    ok: false,
                    redirected: false,
                    final_url: url.clone(),
                    error: Some(error),
                };
            }
            Err(_) => {
                return FetchResponse {
                    success: false,
                    status: 0,
                    headers: std::collections::HashMap::new(),
                    body: String::new(),
                    body_bytes: Vec::new(),
                    ok: false,
                    redirected: false,
                    final_url: url.clone(),
                    error: Some(
                        "Active probe scheduler dropped dispatch grant".to_string(),
                    ),
                };
            }
        };
        effective_timeout_ms = grant.timeout_ms;
        if grant.total_wait_ms > 0 {
            tokio::time::sleep(Duration::from_millis(grant.total_wait_ms)).await;
        }
        mark_active_probe_running(&request_id);
    }

    if let Some(schedule) = plugin_fetch_schedule.as_ref() {
        let policy = configured_policy_for_kind(schedule.kind);
        let dispatch_rx = match enqueue_plugin_request(schedule.clone(), policy) {
            Ok(dispatch_rx) => dispatch_rx,
            Err(error) => {
                return FetchResponse {
                    success: false,
                    status: 0,
                    headers: std::collections::HashMap::new(),
                    body: String::new(),
                    body_bytes: Vec::new(),
                    ok: false,
                    redirected: false,
                    final_url: url.clone(),
                    error: Some(error),
                };
            }
        };
        let grant = match dispatch_rx.await {
            Ok(Ok(grant)) => grant,
            Ok(Err(error)) => {
                return FetchResponse {
                    success: false,
                    status: 0,
                    headers: std::collections::HashMap::new(),
                    body: String::new(),
                    body_bytes: Vec::new(),
                    ok: false,
                    redirected: false,
                    final_url: url.clone(),
                    error: Some(error),
                };
            }
            Err(_) => {
                return FetchResponse {
                    success: false,
                    status: 0,
                    headers: std::collections::HashMap::new(),
                    body: String::new(),
                    body_bytes: Vec::new(),
                    ok: false,
                    redirected: false,
                    final_url: url.clone(),
                    error: Some(
                        "Plugin request scheduler dropped dispatch grant".to_string(),
                    ),
                };
            }
        };
        effective_timeout_ms = effective_timeout_ms.max(grant.timeout_ms);
        if grant.total_wait_ms > 0 {
            tokio::time::sleep(Duration::from_millis(grant.total_wait_ms)).await;
        }
        mark_plugin_request_running(schedule.kind, &request_id);
    }

    let request_started_at = Instant::now();
    let mut req_builder = match method.as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url),
        "HEAD" => client.head(&url),
        _ => client.get(&url),
    };

    for (key, value) in &options.headers {
        req_builder = req_builder.header(key, value);
    }

    req_builder = req_builder.timeout(Duration::from_millis(effective_timeout_ms));

    if let Some(body) = options.body {
        req_builder = match body {
            FetchBody::Text { text } => req_builder.body(text),
            FetchBody::Bytes { bytes } => req_builder.body(bytes),
        };
    }

    let response = match timeout(
        Duration::from_millis(effective_timeout_ms.saturating_add(1000)),
        req_builder.send(),
    )
    .await
    {
        Ok(Ok(response)) => {
            let status = response.status().as_u16();
            let ok = response.status().is_success();
            let final_url = response.url().to_string();
            let redirected = final_url != url;
            let mut headers = std::collections::HashMap::new();
            for (key, value) in response.headers() {
                if let Ok(text) = value.to_str() {
                    headers.insert(key.to_string(), text.to_string());
                }
            }

            let body_result = read_response_body(response, max_body_bytes).await;
            match body_result {
                Ok((body, body_bytes)) => FetchResponse {
                    success: true,
                    status,
                    headers,
                    body,
                    body_bytes,
                    ok,
                    redirected,
                    final_url,
                    error: None,
                },
                Err(err) => FetchResponse {
                    success: false,
                    status,
                    headers,
                    body: String::new(),
                    body_bytes: Vec::new(),
                    ok: false,
                    redirected,
                    final_url,
                    error: Some(format!("Failed to read response body: {err}")),
                },
            }
        }
        Ok(Err(err)) => FetchResponse {
            success: false,
            status: 0,
            headers: std::collections::HashMap::new(),
            body: String::new(),
            body_bytes: Vec::new(),
            ok: false,
            redirected: false,
            final_url: url.clone(),
            error: Some(format!("HTTP request failed: {err}")),
        },
        Err(_) => FetchResponse {
            success: false,
            status: 0,
            headers: std::collections::HashMap::new(),
            body: String::new(),
            body_bytes: Vec::new(),
            ok: false,
            redirected: false,
            final_url: url.clone(),
            error: Some(format!(
                "HTTP request timeout after {effective_timeout_ms} ms"
            )),
        },
    };

    if active_probe_request.is_some() {
        let response_elapsed_ms = Some(request_started_at.elapsed().as_millis() as u64);
        if response.success {
            complete_active_probe(&request_id, Some(response.status), response_elapsed_ms);
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

    if let Some(schedule) = plugin_fetch_schedule.as_ref() {
        let response_elapsed_ms = Some(request_started_at.elapsed().as_millis() as u64);
        if response.success {
            complete_plugin_request(
                schedule.kind,
                &request_id,
                Some(response.status),
                response_elapsed_ms,
            );
        } else {
            fail_plugin_request(
                schedule.kind,
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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct FetchClientKey {
    proxy_url: Option<String>,
    follow_redirects: bool,
    max_redirects: usize,
}

static FETCH_CLIENTS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<FetchClientKey, reqwest::Client>>,
> = std::sync::OnceLock::new();

async fn get_fetch_client(
    follow_redirects: bool,
    max_redirects: usize,
) -> std::result::Result<reqwest::Client, reqwest::Error> {
    let cache =
        FETCH_CLIENTS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
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

    if let Some(client) = cache.lock().unwrap().get(&key).cloned() {
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
        if let Ok(proxy) = reqwest::Proxy::all(proxy_url) {
            builder = builder.proxy(proxy);
        }
    } else {
        builder = builder.no_proxy();
    }

    let client = builder.build()?;
    cache.lock().unwrap().insert(key, client.clone());
    Ok(client)
}

async fn read_response_body(
    mut response: reqwest::Response,
    max_body_bytes: Option<usize>,
) -> std::result::Result<(String, Vec<u8>), reqwest::Error> {
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    if let Some(limit) = max_body_bytes.filter(|value| *value > 0) {
        let mut body = Vec::with_capacity(limit.min(8192));
        while let Some(chunk) = response.chunk().await? {
            let remaining = limit.saturating_sub(body.len());
            if remaining == 0 {
                break;
            }
            if chunk.len() <= remaining {
                body.extend_from_slice(&chunk);
            } else {
                body.extend_from_slice(&chunk[..remaining]);
            }
        }
        return Ok((decode_response_text(&body, content_type.as_deref()), body));
    }

    let body = response.bytes().await?.to_vec();
    Ok((decode_response_text(&body, content_type.as_deref()), body))
}

fn decode_response_text(body: &[u8], content_type: Option<&str>) -> String {
    use encoding_rs::Encoding;

    let charset = content_type
        .and_then(|value| {
            value.split(';').skip(1).find_map(|part| {
                let (name, value) = part.split_once('=')?;
                if !name.trim().eq_ignore_ascii_case("charset") {
                    return None;
                }
                let normalized = value.trim().trim_matches('"').trim_matches('\'');
                if normalized.is_empty() {
                    None
                } else {
                    Some(normalized)
                }
            })
        })
        .and_then(|label| Encoding::for_label(label.as_bytes()));

    if let Some(encoding) = charset {
        let (decoded, _, _) = encoding.decode(body);
        return decoded.into_owned();
    }

    String::from_utf8_lossy(body).into_owned()
}

fn build_active_probe_cooldown_key(url: &str) -> String {
    let Ok(parsed) = reqwest::Url::parse(url) else {
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

// ---------------------------------------------------------------------------
// PluginEngine — QuickJS-backed
// ---------------------------------------------------------------------------

/// Plugin engine backed by One Engine (custom JS interpreter).
pub struct PluginEngine {
    runtime: SentinelJsRuntime,
    plugin_context: PluginContext,
    metadata: Option<PluginMetadata>,
    plugin_path: Option<PathBuf>,
    loaded_plugin_id: Option<String>,
    loaded_code_hash: Option<u64>,
}

impl PluginEngine {
    /// Create a new plugin engine instance.
    ///
    /// The `PluginContext` is installed into the thread-local for the
    /// lifetime of this engine.  Each executor thread owns exactly one
    /// engine, so the context stays valid until `Drop`.
    pub fn new() -> Result<Self> {
        let plugin_context = PluginContext::new();

        set_plugin_ctx(&plugin_context);

        let mut runtime = SentinelJsRuntime::new()?;
        runtime.register_host_functions(&PluginPermissions::default())?;
        runtime.eval_bootstrap(&plugin_context)?;

        let platform_patch = format!(
            r#"
            if (typeof Deno !== "undefined" && Deno.build) {{
                Deno.build.os = "{}";
                Deno.build.arch = "{}";
            }}
            "#,
            std::env::consts::OS,
            std::env::consts::ARCH
        );
        runtime.eval(&platform_patch)?;

        Ok(Self {
            runtime,
            plugin_context,
            metadata: None,
            plugin_path: None,
            loaded_plugin_id: None,
            loaded_code_hash: None,
        })
    }

    /// Refresh the thread-local PluginContext (needed if the engine migrated
    /// to a different thread, which normally does not happen).
    fn ensure_ctx(&self) {
        set_plugin_ctx(&self.plugin_context);
    }

    /// Load plugin code with metadata supplied by the database.
    pub async fn load_plugin_with_metadata(
        &mut self,
        code: &str,
        metadata: PluginMetadata,
    ) -> Result<()> {
        async {
            let plugin_id = metadata.id.clone();
            let sanitized_code = sanitize_plugin_source(code);
            let code_hash = crate::compile_cache::hash_plugin_source(&sanitized_code);

            if self.loaded_plugin_id.as_deref() == Some(plugin_id.as_str())
                && self.loaded_code_hash == Some(code_hash)
            {
                self.plugin_context.set_plugin_id(Some(plugin_id.clone()));
                self.plugin_context
                    .set_plugin_main_category(Some(metadata.main_category.to_string()));
                self.plugin_context
                    .set_monitor_type(metadata.monitor_type.clone());
                self.plugin_path = Some(PathBuf::from(format!("db://{plugin_id}")));
                debug!(
                    "Skipped reload for unchanged plugin: {} v{}",
                    metadata.name, metadata.version
                );
                self.metadata = Some(metadata);
                return Ok(());
            }

            let source_is_module = is_esm_module(&sanitized_code);

            // Strip TypeScript types and cache the result
            let (js_source, _) = crate::compile_cache::compile_cached(
                &plugin_id,
                &sanitized_code,
                source_is_module,
            )
            .map_err(PluginError::Load)?;

            // Determine module mode from the STRIPPED source, not the original.
            // strip_typescript_for_script removes `export` keywords, so plugins
            // with only exports (no imports) become plain scripts whose top-level
            // declarations live in the global scope — required for
            // call_plugin_function to locate analyze/run/execute.
            let is_module = is_esm_module(&js_source);

            debug!(
                "Plugin {}: source_is_module={source_is_module}, stripped_is_module={is_module}, js_len={}",
                plugin_id, js_source.len()
            );
            eprintln!(
                "[DEBUG] Plugin {}: source_is_module={source_is_module}, stripped_is_module={is_module}, js_len={}",
                plugin_id, js_source.len()
            );

            self.ensure_ctx();
            self.runtime.load_plugin(&js_source, is_module)?;

            self.loaded_plugin_id = Some(plugin_id.clone());
            self.loaded_code_hash = Some(code_hash);

            self.plugin_context.set_plugin_id(Some(plugin_id.clone()));
            self.plugin_context
                .set_plugin_main_category(Some(metadata.main_category.to_string()));
            self.plugin_context
                .set_monitor_type(metadata.monitor_type.clone());

            debug!(
                "Loaded plugin via QuickJS: {} v{}",
                metadata.name, metadata.version
            );
            self.metadata = Some(metadata);
            self.plugin_path = Some(PathBuf::from(format!("db://{plugin_id}")));
            Ok(())
        }
        .await
    }

    /// Scan a full HTTP transaction.
    pub async fn scan_transaction(
        &mut self,
        transaction: &crate::types::HttpTransaction,
    ) -> Result<Vec<Finding>> {
        self.scan_transaction_with_sink(transaction, None).await
    }

    pub async fn scan_transaction_with_sink(
        &mut self,
        transaction: &crate::types::HttpTransaction,
        finding_sink: Option<tokio::sync::mpsc::UnboundedSender<Finding>>,
    ) -> Result<Vec<Finding>> {
        async {
            let plugin_id = self
                .metadata
                .as_ref()
                .map(|metadata| metadata.id.clone())
                .unwrap_or_else(|| "unknown-plugin".to_string());

            let _ = self.plugin_context.take_findings();
            let _ = self.plugin_context.take_last_result();
            self.plugin_context.set_finding_sink(finding_sink.clone());
            self.plugin_context
                .set_traffic_request_id(Some(transaction.request.id.clone()));

            emit_active_probe_event(&ActiveProbeEvent {
                plugin_id: Some(plugin_id.clone()),
                traffic_request_id: Some(transaction.request.id.clone()),
                request_id: format!("{}:plugin_invoked", transaction.request.id),
                phase: "plugin_invoked".to_string(),
                method: transaction.request.method.clone(),
                url: transaction.request.url.clone(),
                probe_label: Some(plugin_id.clone()),
                target_name: None,
                target_path: None,
                target_location: None,
                probe_value: None,
                technique: None,
                probe_class: None,
                probe_priority: None,
                cooldown_key: None,
                cooldown_wait_ms: None,
                jitter_wait_ms: None,
                total_wait_ms: None,
                adaptive_penalty_ms: None,
                status: None,
                error: None,
                reason: None,
                target_count: None,
                active_slots: None,
                max_concurrent_per_host: None,
                queue_depth: None,
                response_elapsed_ms: None,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });

            let combined = serde_json::to_value(transaction).map_err(|err| {
                PluginError::Execution(format!("Failed to serialize transaction: {err}"))
            })?;

            let result = self.call_plugin_function("scan_transaction", &combined);

            self.plugin_context.set_finding_sink(None);
            self.plugin_context.set_traffic_request_id(None);

            if let Err(err) = result {
                emit_active_probe_event(&ActiveProbeEvent {
                    plugin_id: Some(plugin_id),
                    traffic_request_id: Some(transaction.request.id.clone()),
                    request_id: format!("{}:plugin_failed", transaction.request.id),
                    phase: "plugin_failed".to_string(),
                    method: transaction.request.method.clone(),
                    url: transaction.request.url.clone(),
                    probe_label: Some("scan_transaction".to_string()),
                    target_name: None,
                    target_path: None,
                    target_location: None,
                    probe_value: None,
                    technique: None,
                    probe_class: None,
                    probe_priority: None,
                    cooldown_key: None,
                    cooldown_wait_ms: None,
                    jitter_wait_ms: None,
                    total_wait_ms: None,
                    adaptive_penalty_ms: None,
                    status: None,
                    error: Some(err.to_string()),
                    reason: None,
                    target_count: None,
                    active_slots: None,
                    max_concurrent_per_host: None,
                    queue_depth: None,
                    response_elapsed_ms: None,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });
                debug!("Plugin execution failed or function not found: {err}");
                return Ok(vec![]);
            }

            let raw_result = self.plugin_context.take_last_result();
            let emitted_findings = self.plugin_context.take_findings();

            let mut findings: Vec<Finding> = if let Some(val) = raw_result {
                if let Ok(js_findings) = serde_json::from_value::<Vec<JsFinding>>(val) {
                    js_findings.into_iter().map(Finding::from).collect()
                } else {
                    debug!("Plugin returned non-array or invalid finding format");
                    vec![]
                }
            } else {
                vec![]
            };

            findings = if finding_sink.is_some() {
                let emitted_signatures = emitted_findings
                    .iter()
                    .map(Finding::calculate_signature)
                    .collect::<HashSet<_>>();

                dedupe_findings(
                    findings
                        .into_iter()
                        .filter(|finding| {
                            !emitted_signatures.contains(&finding.calculate_signature())
                        })
                        .collect(),
                )
            } else {
                let mut combined = emitted_findings;
                combined.extend(findings);
                dedupe_findings(combined)
            };

            emit_active_probe_event(&ActiveProbeEvent {
                plugin_id: Some(plugin_id),
                traffic_request_id: Some(transaction.request.id.clone()),
                request_id: format!("{}:plugin_completed", transaction.request.id),
                phase: "plugin_completed".to_string(),
                method: transaction.request.method.clone(),
                url: transaction.request.url.clone(),
                probe_label: Some("scan_transaction".to_string()),
                target_name: None,
                target_path: None,
                target_location: None,
                probe_value: None,
                technique: None,
                probe_class: None,
                probe_priority: None,
                cooldown_key: None,
                cooldown_wait_ms: None,
                jitter_wait_ms: None,
                total_wait_ms: None,
                adaptive_penalty_ms: None,
                status: None,
                error: None,
                reason: None,
                target_count: Some(findings.len() as u32),
                active_slots: None,
                max_concurrent_per_host: None,
                queue_depth: None,
                response_elapsed_ms: None,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });

            Ok(findings)
        }
        .await
    }

    /// Execute an agent plugin entrypoint.
    pub async fn execute_agent(
        &mut self,
        input: &serde_json::Value,
    ) -> Result<(Vec<Finding>, Option<serde_json::Value>)> {
        self.execute_agent_with_runtime_context(input, None, None)
            .await
    }

    pub async fn execute_agent_with_runtime_context(
        &mut self,
        input: &serde_json::Value,
        execution_context: Option<String>,
        run_id: Option<String>,
    ) -> Result<(Vec<Finding>, Option<serde_json::Value>)> {
        async {
            self.plugin_context.set_execution_context(execution_context);
            self.plugin_context.set_run_id(run_id);

            if let Err(e1) = self.call_plugin_function("analyze", input) {
                if let Err(e2) = self.call_plugin_function("run", input) {
                    self.call_plugin_function("execute", input).map_err(|e3| {
                        PluginError::Execution(format!(
                            "Failed to call agent entrypoint (analyze/run/execute): \
                             analyze_err={e1:?}, run_err={e2:?}, execute_err={e3:?}"
                        ))
                    })?;
                }
            }

            let raw_result = self.plugin_context.take_last_result();
            if let Some(val) = raw_result {
                let findings: Vec<Finding> = if let Some(findings_val) = val.get("findings") {
                    if let Ok(js_findings) =
                        serde_json::from_value::<Vec<JsFinding>>(findings_val.clone())
                    {
                        js_findings.into_iter().map(Finding::from).collect()
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                };
                Ok((findings, Some(val)))
            } else {
                Ok((vec![], None))
            }
        }
        .await
    }

    /// Get the plugin input schema at runtime.
    pub async fn get_input_schema(&mut self) -> Result<serde_json::Value> {
        async {
            let call_script =
                "(function() {\
                 var __schema__ = null;\
                 if (typeof get_input_schema !== 'undefined') { __schema__ = get_input_schema(); }\
                 else if (typeof getInputSchema !== 'undefined') { __schema__ = getInputSchema(); }\
                 if (__schema__ !== null && __schema__ !== undefined) { __sentinel_return(__schema__); }\
                 })()";

            self.ensure_ctx();
            self.runtime.eval(call_script)?;
            if let Some(schema) = self.plugin_context.take_last_result() {
                return Ok(schema);
            }

            Ok(serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string", "description": "Tool input parameter"}
                }
            }))
        }
        .await
    }

    /// Get the plugin output schema at runtime.
    pub async fn get_output_schema(&mut self) -> Result<serde_json::Value> {
        async {
            let call_script =
                "(function() {\
                 var __schema__ = null;\
                 if (typeof get_output_schema !== 'undefined') { __schema__ = get_output_schema(); }\
                 else if (typeof getOutputSchema !== 'undefined') { __schema__ = getOutputSchema(); }\
                 if (__schema__ !== null && __schema__ !== undefined) { __sentinel_return(__schema__); }\
                 })()";

            self.ensure_ctx();
            self.runtime.eval(call_script)?;
            if let Some(schema) = self.plugin_context.take_last_result() {
                return Ok(schema);
            }

            Ok(serde_json::json!({
                "type": "object",
                "properties": {
                    "success": {"type": "boolean"},
                    "data": {"type": "object"},
                    "error": {"type": "string"}
                }
            }))
        }
        .await
    }

    fn call_plugin_function(
        &mut self,
        fn_name: &str,
        args: &serde_json::Value,
    ) -> Result<()> {
        self.runtime.set_json_global("__input", args)?;

        // Wrap in async IIFE and always unwrap via Promise.resolve().
        // In One Engine, checking `typeof result.then === "function"` is unreliable
        // for some Promise implementations and can leak unresolved Promise objects
        // (serialized as { then/catch/finally }) back to host.
        let call_code = format!(
            r#"(async function() {{
                try {{
                    var __result = await Promise.resolve({fn_name}(__input));
                    Sentinel.resolve(__result);
                }} catch (__error) {{
                    var __message = (__error && __error.message) ? __error.message : String(__error);
                    var __stack = (__error && __error.stack) ? __error.stack : "";
                    Sentinel.resolve({{ success: false, error: __message, _debug_stack: __stack }});
                }}
            }})()"#
        );

        self.ensure_ctx();
        let eval_result = self.runtime.eval(&call_code);
        if let Err(ref e) = eval_result {
            eprintln!("[DEBUG] call_plugin_function({fn_name}): eval error: {e:?}");
        }
        eval_result?;
        let jobs_result = self.runtime.execute_pending_jobs();
        if let Err(ref e) = jobs_result {
            eprintln!("[DEBUG] call_plugin_function({fn_name}): pending_jobs error: {e:?}");
        }
        jobs_result?;
        debug!("Plugin function {fn_name} executed successfully");
        Ok(())
    }

    /// Get plugin metadata.
    pub fn get_metadata(&self) -> Option<&PluginMetadata> {
        self.metadata.as_ref()
    }

    /// Set memory limit on the runtime (no-op — SentinelJsRuntime uses memory limits set at creation).
    pub fn set_fuel(&mut self, _fuel: u64) {
    }
}

impl Drop for PluginEngine {
    fn drop(&mut self) {
        clear_plugin_ctx();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_monitor_structure() {
        let mut engine = PluginEngine::new().unwrap();

        let metadata = PluginMetadata {
            id: "test-api-monitor".to_string(),
            name: "API Monitor Test".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            main_category: crate::types::PluginMainCategory::Bounty,
            category: crate::types::PluginCategory::parse_for_main_category(
                crate::types::PluginMainCategory::Bounty,
                "monitor",
            )
            .unwrap(),
            default_severity: crate::types::Severity::High,
            tags: vec![],
            description: None,
            monitor_type: None,
            target_asset_types: vec![],
            input_mode: None,
            seed_bindings: vec![],
        };

        let code = r#"
declare const Sentinel: {
    Monitor?: {
        reportProgress?: (payload: Record<string, unknown>) => Promise<boolean>;
    };
};

interface ToolInput { targets: string[]; timeout?: number; }
interface ToolOutput { success: boolean; error?: string; data?: unknown; }

type PluginGlobals = typeof globalThis & {
    get_input_schema?: typeof get_input_schema;
    get_output_schema?: typeof get_output_schema;
    analyze?: typeof analyze;
};

const pluginGlobals = globalThis as PluginGlobals;

function generateId(): string {
    return 'xxxxxxxx'.replace(/[xy]/g, (c) => {
        const r = Math.random() * 16 | 0;
        return r.toString(16);
    });
}

async function runWithConcurrency<T>(tasks: Array<() => Promise<T>>, concurrency: number): Promise<T[]> {
    const results: T[] = [];
    let index = 0;
    const workers = Array.from({ length: 2 }, async () => {
        while (index < tasks.length) {
            const current = index++;
            results[current] = await tasks[current]();
        }
    });
    await Promise.all(workers);
    return results;
}

const EXCLUDED_EXACT_PATHS = new Set(["/", "/favicon.ico"]);
const VENDOR_PATTERNS = [/^vendor\.js$/i];

function isExcluded(path: string): boolean {
    return EXCLUDED_EXACT_PATHS.has(path);
}

function normalizeUrl(url: string, base: string): string | null {
    try { return new URL(url, base).pathname; } catch { return null; }
}

export function get_input_schema() {
    return {
        type: "object",
        properties: {
            targets: { type: "array", items: { type: "string" } },
            timeout: { type: "number" },
        },
        required: ["targets"],
    };
}

pluginGlobals.get_input_schema = get_input_schema;

export function get_output_schema() {
    return {
        type: "object",
        properties: {
            success: { type: "boolean" },
            data: { type: "object" },
        },
    };
}

pluginGlobals.get_output_schema = get_output_schema;

export async function analyze(input: ToolInput): Promise<ToolOutput> {
    try {
        if (!input.targets || !Array.isArray(input.targets)) {
            return { success: false, error: "Invalid input: targets array is required" };
        }
        const validTargets = input.targets.filter(t => typeof t === 'string' && t.trim().length > 0);
        if (validTargets.length === 0) {
            return { success: false, error: "No valid targets provided" };
        }
        return { success: true, data: { targets_processed: validTargets.length } };
    } catch (error: any) {
        return { success: false, error: error instanceof Error ? error.message : String(error) };
    }
}

pluginGlobals.analyze = analyze;
"#;

        let load_result = engine.load_plugin_with_metadata(code, metadata).await;
        if let Err(ref e) = load_result {
            eprintln!("load_plugin_with_metadata failed: {:?}", e);
        }
        assert!(load_result.is_ok(), "Plugin should load successfully");

        let input = serde_json::json!({
            "targets": ["https://example.com"],
            "timeout": 5000
        });

        let result = engine
            .execute_agent_with_runtime_context(&input, None, None)
            .await;

        if let Err(ref e) = result {
            eprintln!("execute_agent failed: {:?}", e);
        }
        assert!(
            result.is_ok(),
            "analyze should be callable: {:?}",
            result.err()
        );

        let (_findings, output) = result.unwrap();
        let output = output.expect("analyze should return output");
        assert_eq!(output.get("success").and_then(|v| v.as_bool()), Some(true));
        assert!(
            output.get("then").is_none() && output.get("catch").is_none() && output.get("finally").is_none(),
            "output should be resolved value, not a Promise-like object: {output:?}"
        );
    }

    #[tokio::test]
    async fn test_plugin_engine_creation() {
        let engine = PluginEngine::new();
        if let Err(ref e) = engine {
            eprintln!("PluginEngine::new() failed: {:?}", e);
        }
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_plugin_scan_basic() {
        use crate::types::PluginMetadata;
        let mut engine = PluginEngine::new().unwrap();

        let metadata = PluginMetadata {
            id: "test-scan".to_string(),
            name: "Test Scan".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            main_category: crate::types::PluginMainCategory::Traffic,
            category: crate::types::PluginCategory::parse_for_main_category(
                crate::types::PluginMainCategory::Traffic,
                "test",
            )
            .unwrap(),
            default_severity: crate::types::Severity::Info,
            tags: vec![],
            description: None,
            monitor_type: None,
            target_asset_types: vec![],
            input_mode: None,
            seed_bindings: vec![],
        };

        let code = r#"
function scan_transaction(transaction) {
    Sentinel.emitFinding({
        vuln_type: "test",
        title: "Test Finding",
        description: "desc",
        severity: "info",
        confidence: "high"
    });
}
"#;
        engine
            .load_plugin_with_metadata(code, metadata)
            .await
            .unwrap();

        let txn = crate::types::HttpTransaction {
            request: crate::types::RequestContext {
                id: "test-req-1".to_string(),
                method: "GET".to_string(),
                url: "https://example.com/test".to_string(),
                http_version: Some("HTTP/1.1".to_string()),
                headers: std::collections::HashMap::new(),
                body: vec![],
                content_type: None,
                query_params: std::collections::HashMap::new(),
                is_https: true,
                timestamp: chrono::Utc::now(),
                was_edited: false,
                edited_method: None,
                edited_url: None,
                edited_headers: None,
                edited_body: None,
            },
            response: None,
        };

        let findings = engine.scan_transaction(&txn).await.unwrap();
        assert!(!findings.is_empty());
        assert_eq!(findings[0].title, "Test Finding");
    }

    /// Verify that every real plugin file from sentinel-plugin/plugins/
    /// can be TypeScript-stripped and loaded (parsed) by QuickJS without errors.
    #[tokio::test]
    async fn test_all_real_plugins_load() {
        use crate::types::PluginMetadata;
        use std::path::Path;

        let plugins_dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../sentinel-plugin/plugins");
        if !plugins_dir.exists() {
            eprintln!(
                "SKIP: sentinel-plugin/plugins not found at {}",
                plugins_dir.display()
            );
            return;
        }

        let mut ts_files = Vec::new();
        for entry in walkdir::WalkDir::new(&plugins_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().extension().map_or(false, |ext| ext == "ts") {
                ts_files.push(entry.path().to_path_buf());
            }
        }

        assert!(
            !ts_files.is_empty(),
            "Expected to find .ts plugin files in {}",
            plugins_dir.display()
        );

        let mut successes = Vec::new();
        let mut failures = Vec::new();

        for ts_file in &ts_files {
            let code = std::fs::read_to_string(ts_file).expect("read plugin file");
            let relative = ts_file.strip_prefix(&plugins_dir).unwrap_or(ts_file);
            let plugin_name = relative.display().to_string();

            let mut engine = match PluginEngine::new() {
                Ok(e) => e,
                Err(e) => {
                    failures.push((plugin_name, format!("engine creation: {e}")));
                    continue;
                }
            };

            let metadata = PluginMetadata {
                id: plugin_name.clone(),
                name: plugin_name.clone(),
                version: "1.0.0".to_string(),
                author: None,
                main_category: crate::types::PluginMainCategory::Bounty,
                category: crate::types::PluginCategory::parse_for_main_category(
                    crate::types::PluginMainCategory::Bounty,
                    "test",
                )
                .unwrap(),
                default_severity: crate::types::Severity::Info,
                tags: vec![],
                description: None,
                monitor_type: None,
                target_asset_types: vec![],
                input_mode: None,
                seed_bindings: vec![],
            };

            match engine.load_plugin_with_metadata(&code, metadata).await {
                Ok(()) => successes.push(plugin_name),
                Err(e) => failures.push((plugin_name, format!("{e}"))),
            }
        }

        eprintln!("\n=== Plugin Compatibility Report ===");
        eprintln!("Passed: {}/{}", successes.len(), ts_files.len());
        for name in &successes {
            eprintln!("  ✓ {name}");
        }
        if !failures.is_empty() {
            eprintln!("Failed: {}/{}", failures.len(), ts_files.len());
            for (name, err) in &failures {
                eprintln!("  ✗ {name}: {err}");
            }
        }
        eprintln!("=================================\n");

        assert!(
            failures.is_empty(),
            "{} out of {} plugins failed to load:\n{}",
            failures.len(),
            ts_files.len(),
            failures
                .iter()
                .map(|(n, e)| format!("  {n}: {e}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_http_prober_fetch_headers_foreach() {
        let mut engine = PluginEngine::new().unwrap();

        let metadata = PluginMetadata {
            id: "http-prober-test".to_string(),
            name: "HTTP Prober".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            main_category: crate::types::PluginMainCategory::Bounty,
            category: crate::types::PluginCategory::parse_for_main_category(
                crate::types::PluginMainCategory::Bounty,
                "scanner",
            )
            .unwrap(),
            default_severity: crate::types::Severity::Info,
            tags: vec![],
            description: None,
            monitor_type: None,
            target_asset_types: vec![],
            input_mode: None,
            seed_bindings: vec![],
        };

        let plugin_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../sentinel-plugin/plugins/bounty/http_prober.ts");
        let code = std::fs::read_to_string(&plugin_path)
            .expect("http_prober.ts not found");

        engine
            .load_plugin_with_metadata(&code, metadata)
            .await
            .expect("Failed to load http_prober plugin");

        let input = serde_json::json!({
            "targets": ["https://httpbin.org/get"]
        });

        let result = engine.execute_agent(&input).await;
        assert!(result.is_ok(), "Plugin execution failed: {:?}", result.err());

        let (_findings, output) = result.unwrap();
        let output = output.expect("Plugin returned no output");
        eprintln!("HTTP Prober output: {}", serde_json::to_string_pretty(&output).unwrap());

        let success = output.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        let error = output.get("error").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            success || error.is_empty(),
            "Plugin returned error: {error}"
        );
        assert!(
            !output.get("then").is_some(),
            "Output contains unresolved Promise (then/catch/finally)"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_http_prober_stripped_source_debug() {
        let plugin_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../sentinel-plugin/plugins/bounty/http_prober.ts");
        let code = std::fs::read_to_string(&plugin_path)
            .expect("http_prober.ts not found");

        let stripped = crate::ts_strip::strip_typescript_for_script(&code).unwrap();
        let has_import = stripped.contains("import ");
        let has_export = stripped.contains("export ");
        let is_module = is_esm_module(&stripped);
        eprintln!("Stripped source: has_import={has_import}, has_export={has_export}, is_module={is_module}");

        if has_import || has_export {
            for (i, line) in stripped.lines().enumerate() {
                let t = line.trim();
                if t.contains("import ") || t.contains("export ") {
                    eprintln!("  line {}: {}", i + 1, line);
                }
            }
        }

        // Load the plugin and try calling analyze with a simpler approach
        let mut engine = PluginEngine::new().unwrap();
        let metadata = PluginMetadata {
            id: "http-prober-debug".to_string(),
            name: "HTTP Prober Debug".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            main_category: crate::types::PluginMainCategory::Bounty,
            category: crate::types::PluginCategory::parse_for_main_category(
                crate::types::PluginMainCategory::Bounty,
                "scanner",
            )
            .unwrap(),
            default_severity: crate::types::Severity::Info,
            tags: vec![],
            description: None,
            monitor_type: None,
            target_asset_types: vec![],
            input_mode: None,
            seed_bindings: vec![],
        };

        engine
            .load_plugin_with_metadata(&code, metadata)
            .await
            .expect("Failed to load http_prober plugin");

        // Test: can we call the function at all?
        let check_code = r#"
            (function() {
                try {
                    var msg = "analyze type: " + typeof analyze;
                    if (typeof globalThis === "object") {
                        msg += ", globalThis.analyze type: " + typeof globalThis.analyze;
                    }
                    Sentinel.resolve({ check: msg });
                } catch (e) {
                    Sentinel.resolve({ check_error: e.message || String(e) });
                }
            })()
        "#;
        engine.runtime.eval(check_code).expect("check eval");
        engine.runtime.execute_pending_jobs().expect("check jobs");
        let check_result = engine.plugin_context.take_last_result();
        eprintln!("Check result: {:?}", check_result);

        // Test: can the One engine catch TypeErrors?
        engine.plugin_context.take_last_result();
        let catch_test = r#"
            (function() {
                try {
                    var x = undefined;
                    x.trim();
                } catch (e) {
                    Sentinel.resolve({ caught: true, error: e.message || String(e) });
                }
            })()
        "#;
        let catch_result = engine.runtime.eval(catch_test);
        if let Err(ref e) = catch_result {
            eprintln!("TypeError catch test FAILED - eval error: {e:?}");
        } else {
            engine.runtime.execute_pending_jobs().ok();
            let r = engine.plugin_context.take_last_result();
            eprintln!("TypeError catch test: {:?}", r);
        }

        // Test: what about the specific code pattern in analyze?
        engine.plugin_context.take_last_result();
        let pattern_test = r#"
            (function() {
                try {
                    var targets = ["https://example.com"];
                    var valid = targets.filter(function(target) {
                        return typeof target === "string" && target.trim().length > 0;
                    });
                    Sentinel.resolve({ filter_works: true, count: valid.length });
                } catch (e) {
                    Sentinel.resolve({ filter_error: e.message || String(e) });
                }
            })()
        "#;
        let pattern_result = engine.runtime.eval(pattern_test);
        if let Err(ref e) = pattern_result {
            eprintln!("Pattern test eval error: {e:?}");
        } else {
            engine.runtime.execute_pending_jobs().ok();
            let r = engine.plugin_context.take_last_result();
            eprintln!("Pattern test: {:?}", r);
        }

        // Test: call analyze with proper input object
        engine.plugin_context.take_last_result();
        engine.runtime.set_json_global("__test_input", &serde_json::json!({
            "targets": ["https://example.com"]
        })).unwrap();
        let analyze_test = r#"
            (function() {
                try {
                    var result = analyze(__test_input);
                    Sentinel.resolve(result);
                } catch (e) {
                    Sentinel.resolve({ caught: true, error: e.message || String(e) });
                }
            })()
        "#;
        let analyze_result = engine.runtime.eval(analyze_test);
        if let Err(ref e) = analyze_result {
            eprintln!("Analyze eval error: {e:?}");
        } else {
            engine.runtime.execute_pending_jobs().ok();
            let r = engine.plugin_context.take_last_result();
            if let Some(v) = &r {
                let keys: Vec<_> = v.as_object().map(|o| o.keys().collect()).unwrap_or_default();
                eprintln!("Analyze result keys: {:?}", keys);
            } else {
                eprintln!("Analyze result: None");
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_http_prober_deepseek() {
        let plugin_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../sentinel-plugin/plugins/bounty/http_prober.ts");
        let code = std::fs::read_to_string(&plugin_path)
            .expect("http_prober.ts not found");

        let mut engine = PluginEngine::new().unwrap();
        let metadata = PluginMetadata {
            id: "http-prober-e2e".to_string(),
            name: "HTTP Prober E2E".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            main_category: crate::types::PluginMainCategory::Bounty,
            category: crate::types::PluginCategory::parse_for_main_category(
                crate::types::PluginMainCategory::Bounty,
                "scanner",
            )
            .unwrap(),
            default_severity: crate::types::Severity::Info,
            tags: vec![],
            description: None,
            monitor_type: None,
            target_asset_types: vec![],
            input_mode: None,
            seed_bindings: vec![],
        };

        engine
            .load_plugin_with_metadata(&code, metadata)
            .await
            .expect("Failed to load http_prober plugin");

        let input = serde_json::json!({
            "targets": ["https://www.deepseek.com/"]
        });

        engine
            .call_plugin_function("analyze", &input)
            .expect("call_plugin_function failed");

        let result = engine.plugin_context.take_last_result();
        eprintln!("=== HTTP Prober Result ===");
        if let Some(ref v) = result {
            eprintln!("{}", serde_json::to_string_pretty(v).unwrap());
        } else {
            eprintln!("No result returned!");
        }
        assert!(result.is_some(), "analyze should return a result");
        let val = result.unwrap();
        let success = val.get("success").and_then(|v| v.as_bool());
        eprintln!("success = {:?}", success);
        assert_ne!(success, Some(false), "analyze should not return success: false");
    }

}

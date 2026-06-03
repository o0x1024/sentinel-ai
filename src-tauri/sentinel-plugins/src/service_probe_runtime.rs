use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use tracing::warn;

use crate::dictionary_runtime;
use crate::monitor_progress::MonitorProgressContext;
use crate::runtime_events::emit_monitor_task_progress;
use crate::service_probe_engine::resolve_service_probe_engine;
use crate::service_probe_native::{collect_probe_evidence, match_evidence, service_key};

const DEFAULT_CONCURRENCY: usize = 16;
const MAX_CONCURRENCY: usize = 64;
const DEFAULT_TIMEOUT_MS: u64 = 3000;

#[derive(Debug, Clone, Serialize)]
struct MonitorTaskServiceProbeProgressEvent {
    task_id: String,
    task_name: String,
    program_id: String,
    execution_mode: String,
    status: String,
    progress: u32,
    completed_steps: u32,
    total_steps: u32,
    current_plugin: Option<String>,
    current_plugin_index: Option<u32>,
    target_count: u32,
    imported_assets: u32,
    indeterminate: bool,
    message: Option<String>,
    started_at: String,
    updated_at: String,
    scan_completed_targets: u32,
    scan_total_targets: u32,
    scan_completed_units: u32,
    scan_total_units: u32,
    current_target: Option<String>,
}

#[derive(Default)]
struct ServiceProbeAggregateProgress {
    per_target_fraction: Mutex<HashMap<String, f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceProbeTarget {
    pub host: String,
    #[serde(default)]
    pub connect_ip: Option<String>,
    pub port: u16,
    #[serde(default)]
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceProbeRequest {
    #[serde(default)]
    pub targets: Vec<ServiceProbeTarget>,
    #[serde(default)]
    pub rules: Vec<ServiceProbeRule>,
    #[serde(default)]
    pub dictionary_id: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub concurrency: Option<usize>,
    #[serde(default)]
    pub follow_http_redirects: Option<bool>,
    #[serde(default)]
    pub read_banner: Option<bool>,
    #[serde(default)]
    pub engine: Option<String>,
    #[serde(default, alias = "monitor_progress")]
    pub monitor_progress: Option<MonitorProgressContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceProbeRule {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub word: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceProbeResult {
    pub target: String,
    pub success: bool,
    pub available: bool,
    pub host: String,
    pub connect_ip: Option<String>,
    pub port: u16,
    pub protocol: String,
    pub service_name: Option<String>,
    pub product_name: Option<String>,
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub banner: Option<String>,
    pub server_header: Option<String>,
    pub title: Option<String>,
    pub status_code: Option<u16>,
    pub confidence: Option<f64>,
    pub matched_rule_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceProbeResponse {
    pub success: bool,
    pub results: Vec<ServiceProbeResult>,
    pub rule_count: usize,
    pub engine_requested: String,
    pub engine_used: String,
    pub engine_experimental: bool,
    pub fallback_reason: Option<String>,
    pub error: Option<String>,
}

async fn load_rules_from_dictionary(id_or_name: &str) -> Vec<ServiceProbeRule> {
    match dictionary_runtime::get_dictionary_entries(id_or_name.to_string(), Some(10000)).await {
        Ok(entries) => entries
            .into_iter()
            .map(|entry| ServiceProbeRule {
                id: None,
                word: entry.word,
                category: entry.category,
                metadata: entry.metadata.unwrap_or(Value::Null),
            })
            .collect(),
        Err(error) => {
            warn!(
                "Failed loading dictionary entries for service probe runtime: {}",
                error
            );
            Vec::new()
        }
    }
}

async fn resolve_rule_dictionary_id(requested: Option<&str>) -> Option<String> {
    let requested = requested.map(str::trim).filter(|value| !value.is_empty());
    if let Some(value) = requested {
        return Some(value.to_string());
    }

    if let Ok(default_id) =
        dictionary_runtime::get_default_dictionary_id("service_probe_rule".to_string()).await
    {
        if !default_id.trim().is_empty() {
            return Some(default_id);
        }
    }

    if let Ok(default_id) =
        dictionary_runtime::get_default_dictionary_id("fingerprint_rule".to_string()).await
    {
        if !default_id.trim().is_empty() {
            return Some(default_id);
        }
    }

    Some("builtin_service_fingerprint_rules".to_string())
}

fn emit_service_probe_progress(
    context: &MonitorProgressContext,
    aggregate_state: &ServiceProbeAggregateProgress,
    host: &str,
    target_fraction: f64,
    total_targets: usize,
    completed_units: usize,
    total_units: usize,
    current_target: Option<&str>,
    message: String,
) {
    let total_steps = context.total_steps.max(1);
    let completed_steps = context.completed_steps.min(total_steps);

    let (aggregate_fraction, completed_targets) = {
        let mut guard = aggregate_state
            .per_target_fraction
            .lock()
            .expect("service probe aggregate progress poisoned");
        guard.insert(host.to_string(), target_fraction.clamp(0.0, 1.0));
        let aggregate_fraction = if total_targets == 0 {
            1.0
        } else {
            guard.values().copied().sum::<f64>() / total_targets as f64
        };
        let completed_targets = guard.values().filter(|value| **value >= 0.999_999).count();
        (aggregate_fraction.clamp(0.0, 1.0), completed_targets)
    };
    let overall_progress = (((completed_steps as f64 + aggregate_fraction) / total_steps as f64)
        * 100.0)
        .round() as u32;

    emit_monitor_task_progress(&MonitorTaskServiceProbeProgressEvent {
        task_id: context.task_id.clone(),
        task_name: context.task_name.clone(),
        program_id: context.program_id.clone(),
        execution_mode: context.execution_mode.clone(),
        status: "running".to_string(),
        progress: overall_progress.min(99),
        completed_steps,
        total_steps,
        current_plugin: Some(context.current_plugin.clone()),
        current_plugin_index: Some(context.current_plugin_index),
        target_count: total_targets as u32,
        imported_assets: context.imported_assets,
        indeterminate: false,
        message: Some(message),
        started_at: context.started_at.clone(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        scan_completed_targets: completed_targets as u32,
        scan_total_targets: total_targets as u32,
        scan_completed_units: completed_units as u32,
        scan_total_units: total_units as u32,
        current_target: current_target.map(|value| value.to_string()),
    });
}

pub async fn probe_services(request: ServiceProbeRequest) -> ServiceProbeResponse {
    if request.targets.is_empty() {
        let engine_resolution = resolve_service_probe_engine(request.engine.as_deref());
        return ServiceProbeResponse {
            success: false,
            results: Vec::new(),
            rule_count: 0,
            engine_requested: engine_resolution.requested,
            engine_used: engine_resolution.used,
            engine_experimental: engine_resolution.experimental,
            fallback_reason: engine_resolution.fallback_reason,
            error: Some("targets array is required".to_string()),
        };
    }

    let timeout_ms = DEFAULT_TIMEOUT_MS;
    let concurrency = request
        .concurrency
        .unwrap_or(DEFAULT_CONCURRENCY)
        .clamp(1, MAX_CONCURRENCY);
    let follow_http_redirects = request.follow_http_redirects.unwrap_or(true);
    let read_banner = request.read_banner.unwrap_or(true);
    let engine_resolution = resolve_service_probe_engine(request.engine.as_deref());
    let rules = if request.rules.is_empty() {
        if let Some(dictionary_id) =
            resolve_rule_dictionary_id(request.dictionary_id.as_deref()).await
        {
            load_rules_from_dictionary(&dictionary_id).await
        } else {
            Vec::new()
        }
    } else {
        request.rules
    };

    let mut deduped_targets = Vec::new();
    let mut seen = BTreeSet::new();
    for target in request.targets {
        let key = service_key(
            target.host.trim(),
            target.port,
            target.connect_ip.as_deref(),
        );
        if key == ":" || !seen.insert(key) {
            continue;
        }
        deduped_targets.push(target);
    }

    let rule_count = rules.len();
    let rules = Arc::new(rules);
    let total_targets = deduped_targets.len();
    let progress_context = request.monitor_progress.clone();
    let completed_targets = Arc::new(AtomicUsize::new(0));
    let aggregate_state = progress_context
        .as_ref()
        .map(|_| Arc::new(ServiceProbeAggregateProgress::default()));

    if let (Some(context), Some(state)) = (progress_context.as_ref(), aggregate_state.as_ref()) {
        emit_service_probe_progress(
            context,
            state,
            "__service_probe__",
            0.0,
            total_targets,
            0,
            0,
            None,
            format!("Preparing service probe for {} targets", total_targets),
        );
    }

    let mut results: Vec<ServiceProbeResult> =
        stream::iter(deduped_targets.into_iter().map(|target| {
            let rules = Arc::clone(&rules);
            let progress_context = progress_context.clone();
            let completed_targets = Arc::clone(&completed_targets);
            let aggregate_state = aggregate_state.clone();
            async move {
                let target_label = service_key(
                    target.host.trim(),
                    target.port,
                    target.connect_ip.as_deref(),
                );
                let connect_ip = target.connect_ip.clone();
                let emit_stage = |completed_units: u32, total_units: u32, message: &str| {
                    if let (Some(context), Some(state)) =
                        (progress_context.as_ref(), aggregate_state.as_ref())
                    {
                        let total_units = total_units.max(1);
                        emit_service_probe_progress(
                            context,
                            state,
                            target_label.as_str(),
                            completed_units as f64 / total_units as f64,
                            total_targets,
                            completed_units as usize,
                            total_units as usize,
                            Some(target_label.as_str()),
                            format!("{message} for {target_label}"),
                        );
                    }
                };

                let result = match collect_probe_evidence(
                    target,
                    rules.as_ref(),
                    timeout_ms,
                    follow_http_redirects,
                    read_banner,
                    Some(&emit_stage),
                )
                .await
                {
                    Ok(evidence) => {
                        emit_stage(3, 4, "Matching fingerprints");

                        let mut result = match_evidence(evidence, rules.as_ref());
                        result.target = target_label.clone();
                        result.connect_ip = connect_ip.clone();
                        result
                    }
                    Err(mut invalid_result) => {
                        invalid_result.target = target_label.clone();
                        invalid_result.connect_ip = connect_ip.clone();
                        invalid_result
                    }
                };

                if let (Some(context), Some(state)) =
                    (progress_context.as_ref(), aggregate_state.as_ref())
                {
                    let completed = completed_targets.fetch_add(1, Ordering::Relaxed) + 1;
                    emit_service_probe_progress(
                        context,
                        state,
                        target_label.as_str(),
                        1.0,
                        total_targets,
                        4,
                        4,
                        Some(result.target.as_str()),
                        format!(
                            "Probing services: target progress {}/{} ({})",
                            completed, total_targets, target_label
                        ),
                    );
                }

                result
            }
        }))
        .buffer_unordered(concurrency)
        .collect()
        .await;

    results.sort_by(|left, right| left.target.cmp(&right.target));
    let success = results.iter().any(|result| result.success);

    ServiceProbeResponse {
        success,
        results,
        rule_count,
        engine_requested: engine_resolution.requested,
        engine_used: engine_resolution.used,
        engine_experimental: engine_resolution.experimental,
        fallback_reason: engine_resolution.fallback_reason,
        error: None,
    }
}

pub async fn op_probe_services(request: ServiceProbeRequest) -> ServiceProbeResponse {
    probe_services(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::net::Ipv4Addr;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn probe_services_uses_logical_host_with_connect_ip_override() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("test listener should bind");
        let addr = listener
            .local_addr()
            .expect("test listener should expose local address");

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("test listener should accept one client");
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer).await;
            let _ = stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nServer: TestHTTP/1.0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await;
            let _ = stream.shutdown().await;
        });

        let response = probe_services(ServiceProbeRequest {
            targets: vec![ServiceProbeTarget {
                host: "app.example.test".to_string(),
                connect_ip: Some(Ipv4Addr::LOCALHOST.to_string()),
                port: addr.port(),
                protocol: "http".to_string(),
            }],
            rules: vec![ServiceProbeRule {
                id: None,
                word: "__never_match__".to_string(),
                category: None,
                metadata: Value::Null,
            }],
            dictionary_id: None,
            timeout_ms: Some(3000),
            concurrency: Some(1),
            follow_http_redirects: Some(false),
            read_banner: Some(true),
            engine: None,
            monitor_progress: None,
        })
        .await;

        server.await.expect("test server task should complete");

        assert!(response.error.is_none(), "service probe should not fail");
        assert_eq!(response.results.len(), 1);
        let result = &response.results[0];
        assert_eq!(result.host, "app.example.test");
        assert_eq!(result.connect_ip.as_deref(), Some("127.0.0.1"));
        assert_eq!(
            result.target,
            format!("app.example.test@127.0.0.1:{}", addr.port())
        );
        assert_eq!(result.service_name.as_deref(), Some("http"));
        assert_eq!(result.server_header.as_deref(), Some("TestHTTP/1.0"));
        assert_eq!(result.status_code, Some(200));
    }
}

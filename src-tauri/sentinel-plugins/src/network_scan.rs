use deno_core::op2;
use futures::stream::{self, StreamExt};
use rustscan::input::ScanOrder;
use rustscan::port_strategy::PortStrategy;
use rustscan::scanner::Scanner;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use crate::monitor_progress::MonitorProgressContext;
use crate::runtime_events::emit_monitor_task_progress;

const DEFAULT_BATCH_SIZE: u16 = 512;
const MAX_BATCH_SIZE: u16 = 2048;
const DEFAULT_CONCURRENCY: usize = 500;
const MAX_CONCURRENCY: usize = 1000;
const DEFAULT_TIMEOUT_MS: u64 = 1500;
const DEFAULT_TRIES: u8 = 1;

#[derive(Debug, Clone, Serialize)]
struct MonitorTaskSubProgressEvent {
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
struct ScanAggregateProgress {
    per_target_fraction: Mutex<HashMap<String, f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortScanTarget {
    pub host: String,
    #[serde(default)]
    pub ports: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PortScanRequest {
    #[serde(default)]
    pub targets: Vec<PortScanTarget>,
    #[serde(default)]
    pub ports: Vec<u16>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub batch_size: Option<u16>,
    #[serde(default)]
    pub concurrency: Option<usize>,
    #[serde(default)]
    pub tries: Option<u8>,
    #[serde(default)]
    pub monitor_progress: Option<MonitorProgressContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortScanResult {
    pub host: String,
    pub resolved_ips: Vec<String>,
    pub open_ports: Vec<u16>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortScanSummary {
    pub total_targets: usize,
    pub successful_scans: usize,
    pub failed_scans: usize,
    pub total_open_ports: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortScanResponse {
    pub success: bool,
    pub results: Vec<PortScanResult>,
    pub summary: PortScanSummary,
    pub error: Option<String>,
}

fn normalize_host(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim()
        .to_string()
}

fn normalize_ports(ports: &[u16]) -> Vec<u16> {
    let mut deduped = BTreeSet::new();
    for port in ports {
        if *port > 0 {
            deduped.insert(*port);
        }
    }
    deduped.into_iter().collect()
}

async fn resolve_target_ips(host: &str) -> Result<Vec<IpAddr>, String> {
    let normalized_host = normalize_host(host);
    if normalized_host.is_empty() {
        return Err("empty host".to_string());
    }

    if let Ok(ip) = normalized_host.parse::<IpAddr>() {
        return Ok(vec![ip]);
    }

    let resolved = tokio::net::lookup_host((normalized_host.as_str(), 0))
        .await
        .map_err(|error| format!("DNS resolution failed for {}: {}", normalized_host, error))?;

    let mut ips = BTreeSet::new();
    for addr in resolved {
        ips.insert(addr.ip());
    }

    if ips.is_empty() {
        return Err(format!("No IPs resolved for {}", normalized_host));
    }

    Ok(ips.into_iter().collect())
}

async fn scan_target(
    target: PortScanTarget,
    default_ports: Vec<u16>,
    timeout_ms: u64,
    batch_size: u16,
    tries: u8,
) -> PortScanResult {
    let host = normalize_host(&target.host);
    let scan_ports = if target.ports.is_empty() {
        default_ports
    } else {
        normalize_ports(&target.ports)
    };

    if host.is_empty() {
        return PortScanResult {
            host: target.host,
            resolved_ips: Vec::new(),
            open_ports: Vec::new(),
            error: Some("Empty host".to_string()),
        };
    }

    if scan_ports.is_empty() {
        return PortScanResult {
            host,
            resolved_ips: Vec::new(),
            open_ports: Vec::new(),
            error: Some("No ports configured to scan".to_string()),
        };
    }

    let resolved_ips = match resolve_target_ips(&host).await {
        Ok(ips) => ips,
        Err(error) => {
            return PortScanResult {
                host,
                resolved_ips: Vec::new(),
                open_ports: Vec::new(),
                error: Some(error),
            };
        }
    };

    let strategy = PortStrategy::pick(&None, Some(scan_ports.clone()), ScanOrder::Serial);
    let scanner = Scanner::new(
        &resolved_ips,
        batch_size.min(MAX_BATCH_SIZE).max(1),
        Duration::from_millis(timeout_ms),
        tries.max(1),
        true,
        strategy,
        true,
        Vec::new(),
        false,
    );

    let sockets = scanner.run().await;
    let mut open_ports = BTreeSet::new();
    for socket in sockets {
        open_ports.insert(socket.port());
    }

    PortScanResult {
        host,
        resolved_ips: resolved_ips.into_iter().map(|ip| ip.to_string()).collect(),
        open_ports: open_ports.into_iter().collect(),
        error: None,
    }
}

async fn try_connect_socket(socket: SocketAddr, timeout_ms: u64, tries: u8) -> Option<u16> {
    for _ in 0..tries.max(1) {
        let connect_result = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            tokio::net::TcpStream::connect(socket),
        )
        .await;

        if let Ok(Ok(stream)) = connect_result {
            drop(stream);
            return Some(socket.port());
        }
    }

    None
}

fn update_aggregate_fraction(
    state: &ScanAggregateProgress,
    host: &str,
    fraction: f64,
    total_targets: usize,
) -> (f64, usize) {
    let mut guard = state.per_target_fraction.lock().unwrap();
    guard.insert(host.to_string(), fraction.clamp(0.0, 1.0));
    let aggregate_fraction = if total_targets == 0 {
        1.0
    } else {
        guard.values().copied().sum::<f64>() / total_targets as f64
    };
    let completed_targets = guard.values().filter(|value| **value >= 0.999_999).count();
    (aggregate_fraction.clamp(0.0, 1.0), completed_targets)
}

fn emit_scan_progress(
    context: &MonitorProgressContext,
    aggregate_state: &ScanAggregateProgress,
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
    let (aggregate_fraction, completed_targets) =
        update_aggregate_fraction(aggregate_state, host, target_fraction, total_targets);
    let overall_progress = (((completed_steps as f64 + aggregate_fraction) / total_steps as f64)
        * 100.0)
        .round() as u32;

    emit_monitor_task_progress(&MonitorTaskSubProgressEvent {
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

async fn scan_target_with_progress(
    target: PortScanTarget,
    default_ports: Vec<u16>,
    timeout_ms: u64,
    batch_size: u16,
    tries: u8,
    progress_context: Option<MonitorProgressContext>,
    aggregate_state: Option<Arc<ScanAggregateProgress>>,
    total_targets: usize,
) -> PortScanResult {
    let host = normalize_host(&target.host);
    let scan_ports = if target.ports.is_empty() {
        default_ports
    } else {
        normalize_ports(&target.ports)
    };

    if host.is_empty() {
        return PortScanResult {
            host: target.host,
            resolved_ips: Vec::new(),
            open_ports: Vec::new(),
            error: Some("Empty host".to_string()),
        };
    }

    if scan_ports.is_empty() {
        return PortScanResult {
            host,
            resolved_ips: Vec::new(),
            open_ports: Vec::new(),
            error: Some("No ports configured to scan".to_string()),
        };
    }

    let resolved_ips = match resolve_target_ips(&host).await {
        Ok(ips) => ips,
        Err(error) => {
            return PortScanResult {
                host,
                resolved_ips: Vec::new(),
                open_ports: Vec::new(),
                error: Some(error),
            };
        }
    };

    let sockets: Vec<SocketAddr> = scan_ports
        .iter()
        .flat_map(|port| {
            resolved_ips
                .iter()
                .copied()
                .map(move |ip| SocketAddr::new(ip, *port))
        })
        .collect();
    let total_units = sockets.len();
    let emit_every = (total_units / 100).max(1).min(128);

    if let (Some(context), Some(state)) = (progress_context.as_ref(), aggregate_state.as_ref()) {
        emit_scan_progress(
            context,
            state,
            host.as_str(),
            0.0,
            total_targets,
            0,
            total_units,
            Some(host.as_str()),
            if total_units > 0 {
                format!("Scanning {} sockets for {}", total_units, host)
            } else {
                format!("Scanning {}", host)
            },
        );
    }

    let mut completed_units = 0usize;
    let mut open_ports = BTreeSet::new();
    let mut socket_stream = stream::iter(
        sockets
            .into_iter()
            .map(|socket| async move { try_connect_socket(socket, timeout_ms, tries).await }),
    )
    .buffer_unordered(batch_size.min(MAX_BATCH_SIZE).max(1) as usize);

    while let Some(open_port) = socket_stream.next().await {
        completed_units += 1;
        if let Some(port) = open_port {
            open_ports.insert(port);
        }

        if completed_units % emit_every == 0 || completed_units == total_units {
            if let (Some(context), Some(state)) =
                (progress_context.as_ref(), aggregate_state.as_ref())
            {
                let target_fraction = if total_units == 0 {
                    1.0
                } else {
                    completed_units as f64 / total_units as f64
                };
                emit_scan_progress(
                    context,
                    state,
                    host.as_str(),
                    target_fraction,
                    total_targets,
                    completed_units,
                    total_units,
                    Some(host.as_str()),
                    format!(
                        "Scanning {}: socket progress {}/{}",
                        host, completed_units, total_units
                    ),
                );
            }
        }
    }

    PortScanResult {
        host,
        resolved_ips: resolved_ips.into_iter().map(|ip| ip.to_string()).collect(),
        open_ports: open_ports.into_iter().collect(),
        error: None,
    }
}

#[op2(async)]
#[serde]
pub async fn op_scan_ports(#[serde] request: PortScanRequest) -> PortScanResponse {
    if request.targets.is_empty() {
        return PortScanResponse {
            success: false,
            results: Vec::new(),
            summary: PortScanSummary {
                total_targets: 0,
                successful_scans: 0,
                failed_scans: 0,
                total_open_ports: 0,
            },
            error: Some("targets array is required".to_string()),
        };
    }

    let default_ports = normalize_ports(&request.ports);
    let timeout_ms = request.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS).max(100);
    let tries = request.tries.unwrap_or(DEFAULT_TRIES).max(1);
    let batch_size = request
        .batch_size
        .unwrap_or(DEFAULT_BATCH_SIZE)
        .min(MAX_BATCH_SIZE)
        .max(1);
    let concurrency = request
        .concurrency
        .unwrap_or(DEFAULT_CONCURRENCY)
        .clamp(1, MAX_CONCURRENCY);
    let total_targets = request.targets.len();
    let progress_context = request.monitor_progress.clone();
    let aggregate_state = progress_context
        .as_ref()
        .map(|_| Arc::new(ScanAggregateProgress::default()));

    if let (Some(context), Some(state)) = (progress_context.as_ref(), aggregate_state.as_ref()) {
        emit_scan_progress(
            context,
            state,
            "__scan__",
            0.0,
            total_targets,
            0,
            0,
            None,
            format!("Preparing port scan for {} targets", total_targets),
        );
    }

    let mut results: Vec<PortScanResult> =
        stream::iter(request.targets.into_iter().map(|target| {
            let default_ports = default_ports.clone();
            let progress_context = progress_context.clone();
            let aggregate_state = aggregate_state.clone();
            async move {
                if progress_context.is_some() {
                    scan_target_with_progress(
                        target,
                        default_ports,
                        timeout_ms,
                        batch_size,
                        tries,
                        progress_context,
                        aggregate_state,
                        total_targets,
                    )
                    .await
                } else {
                    scan_target(target, default_ports, timeout_ms, batch_size, tries).await
                }
            }
        }))
        .buffer_unordered(concurrency)
        .collect()
        .await;

    results.sort_by(|left, right| left.host.cmp(&right.host));

    let successful_scans = results
        .iter()
        .filter(|result| result.error.is_none())
        .count();
    let failed_scans = results.len().saturating_sub(successful_scans);
    let total_open_ports = results
        .iter()
        .map(|result| result.open_ports.len())
        .sum::<usize>();

    PortScanResponse {
        success: failed_scans < results.len(),
        summary: PortScanSummary {
            total_targets: results.len(),
            successful_scans,
            failed_scans,
            total_open_ports,
        },
        results,
        error: None,
    }
}

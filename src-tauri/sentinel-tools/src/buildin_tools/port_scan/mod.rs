mod connect_engine;
mod protocol;
mod raw_syn_engine;
mod raw_syn_routing;
mod sctp;
mod targeting;
mod tcp;
mod udp;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::time::Instant;

use rig::tool::Tool;
use schemars::JsonSchema;
use sentinel_plugins::{
    emit_plugin_monitor_progress, probe_services, MonitorProgressContext,
    PluginMonitorProgressUpdate, ServiceProbeRequest, ServiceProbeTarget,
};
use serde::{Deserialize, Serialize};

use self::connect_engine::scan_connect_sockets;
use self::protocol::{PortScanSocketRef, ScanProtocol};
use self::raw_syn_engine::{scan_raw_syn_sockets, RawSynScanConfig};
use self::targeting::{
    build_socket_tasks, collect_socket_targets, count_socket_attempts, expand_target_specs,
    normalize_excluded_ports, normalize_excluded_targets, normalize_input_targets,
    normalize_requested_ports, resolve_hosts, HostAccumulator, SocketOutcome,
};

const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 1_500;
const DEFAULT_SCAN_CONCURRENCY: usize = 1_024;
const MAX_SCAN_CONCURRENCY: usize = 8_192;
const DEFAULT_CONNECT_TRIES: u8 = 1;
const DEFAULT_PROBE_TIMEOUT_MS: u64 = 2_000;
const DEFAULT_PROBE_CONCURRENCY: usize = 32;
const MAX_PROBE_CONCURRENCY: usize = 128;
const DEFAULT_MAX_HOSTS: usize = 4_096;
const DEFAULT_MAX_SOCKET_ATTEMPTS: usize = 4_096;
const DEFAULT_RAW_TTL: u8 = 64;
const DEFAULT_RAW_SHARD_INDEX: usize = 1;
const DEFAULT_RAW_SHARD_COUNT: usize = 1;

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PortScanEngine {
    #[default]
    Connect,
    RawSyn,
}

impl PortScanEngine {
    fn label(self) -> &'static str {
        match self {
            Self::Connect => "connect",
            Self::RawSyn => "raw_syn",
        }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct PortScanArgs {
    /// Scan engine selection. `connect` uses async TCP connect calls. `raw_syn` sends raw IPv4 TCP/UDP/SCTP probes.
    #[serde(default)]
    pub engine: PortScanEngine,
    /// Target specifications. Accepts single IPs, domains, CIDR blocks, or hyphen IP ranges.
    #[serde(default)]
    pub targets: Vec<String>,
    /// Single target shortcut. Merged into `targets` during normalization.
    #[serde(default)]
    pub target: Option<String>,
    /// Excluded target specifications. Accepts the same syntax as `targets`.
    #[serde(default)]
    pub exclude_targets: Vec<String>,
    /// Single excluded target shortcut. Merged into `exclude_targets` during normalization.
    #[serde(default)]
    pub exclude_target: Option<String>,
    /// Masscan-style port expression, for example `80,443,U:53,S:2905,8000-8100`.
    #[serde(default)]
    pub port_spec: Option<String>,
    /// Explicit TCP port list. Merged with the TCP portion of `port_spec`.
    #[serde(default)]
    pub ports: Vec<u16>,
    /// Masscan-style excluded port expression.
    #[serde(default)]
    pub exclude_port_spec: Option<String>,
    /// Explicit excluded TCP port list.
    #[serde(default)]
    pub exclude_ports: Vec<u16>,
    /// Timeout for each connect attempt or SYN response window in milliseconds.
    #[serde(default = "default_connect_timeout_ms")]
    pub timeout_ms: u64,
    /// Maximum number of concurrent socket attempts or in-flight SYN probes.
    #[serde(default = "default_scan_concurrency")]
    pub concurrency: usize,
    /// Retry count for each socket attempt or SYN transmission.
    #[serde(default = "default_connect_tries")]
    pub tries: u8,
    /// Optional network device used by the `raw_syn` engine.
    #[serde(default)]
    pub device: Option<String>,
    /// IPv4 TTL used by the `raw_syn` engine.
    #[serde(default = "default_raw_ttl")]
    pub raw_ttl: u8,
    /// First source port reserved for raw SYN probes.
    #[serde(default)]
    pub raw_source_port_first: Option<u16>,
    /// Last source port reserved for raw SYN probes.
    #[serde(default)]
    pub raw_source_port_last: Option<u16>,
    /// Raw SYN maximum send rate in packets-per-second. When omitted, the sender runs unthrottled.
    #[serde(default)]
    pub max_rate: Option<f64>,
    /// Deterministic seed used by the raw SYN task permutation order.
    #[serde(default)]
    pub seed: Option<u64>,
    /// Shard index for raw SYN scans. Uses 1-based indexing, matching masscan-style `1/N`.
    #[serde(default = "default_raw_shard_index")]
    pub shard_index: usize,
    /// Total shard count for raw SYN scans.
    #[serde(default = "default_raw_shard_count")]
    pub shard_count: usize,
    /// Resume offset into the raw SYN scan space before sharding is applied.
    #[serde(default)]
    pub resume_index: usize,
    /// Optional limit on how many scan-space positions to execute after `resume_index`.
    #[serde(default)]
    pub resume_count: Option<usize>,
    /// Run built-in service fingerprinting for common HTTP/SSH sockets after the port scan finishes.
    #[serde(default = "default_service_probe")]
    pub service_probe: bool,
    /// Read protocol banners during service probing.
    #[serde(default = "default_read_banner")]
    pub read_banner: bool,
    /// Follow redirects during HTTP/HTTPS service probing.
    #[serde(default = "default_follow_http_redirects")]
    pub follow_http_redirects: bool,
    /// Timeout for each service probe in milliseconds.
    #[serde(default = "default_probe_timeout_ms")]
    pub probe_timeout_ms: u64,
    /// Maximum number of concurrent service probes.
    #[serde(default = "default_probe_concurrency")]
    pub probe_concurrency: usize,
    /// Include closed sockets in detailed `results` and `ports` output.
    #[serde(default)]
    pub include_closed: bool,
    /// Include filtered sockets in detailed `results` and `ports` output.
    #[serde(default)]
    pub include_filtered: bool,
    /// Hard cap for expanded hosts after parsing CIDR blocks and IP ranges.
    #[serde(default = "default_max_hosts")]
    pub max_hosts: usize,
    /// Hard cap for total socket attempts (`expanded_hosts * requested_ports`).
    #[serde(default = "default_max_socket_attempts")]
    pub max_socket_attempts: usize,
    #[schemars(skip)]
    #[serde(default, alias = "__monitorExecution")]
    pub monitor_progress: Option<MonitorProgressContext>,
}

fn default_connect_timeout_ms() -> u64 {
    DEFAULT_CONNECT_TIMEOUT_MS
}

fn default_scan_concurrency() -> usize {
    DEFAULT_SCAN_CONCURRENCY
}

fn default_connect_tries() -> u8 {
    DEFAULT_CONNECT_TRIES
}

fn default_service_probe() -> bool {
    true
}

fn default_raw_ttl() -> u8 {
    DEFAULT_RAW_TTL
}

fn default_raw_shard_index() -> usize {
    DEFAULT_RAW_SHARD_INDEX
}

fn default_raw_shard_count() -> usize {
    DEFAULT_RAW_SHARD_COUNT
}

fn default_read_banner() -> bool {
    true
}

fn default_follow_http_redirects() -> bool {
    true
}

fn default_probe_timeout_ms() -> u64 {
    DEFAULT_PROBE_TIMEOUT_MS
}

fn default_probe_concurrency() -> usize {
    DEFAULT_PROBE_CONCURRENCY
}

fn default_max_hosts() -> usize {
    DEFAULT_MAX_HOSTS
}

fn default_max_socket_attempts() -> usize {
    DEFAULT_MAX_SOCKET_ATTEMPTS
}

#[derive(Debug, Clone, Serialize)]
pub struct PortScanOutput {
    pub success: bool,
    pub data: PortScanData,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortScanData {
    pub input_targets: Vec<String>,
    pub expanded_hosts: Vec<String>,
    pub requested_ports: Vec<PortScanSocketRef>,
    pub results: Vec<PortScanHostResult>,
    pub ports: Vec<PortScanPortResult>,
    pub summary: PortScanSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortScanHostResult {
    pub host: String,
    pub resolved_ips: Vec<String>,
    pub open_ports: Vec<PortScanSocketRef>,
    pub closed_ports: Vec<PortScanSocketRef>,
    pub filtered_ports: Vec<PortScanSocketRef>,
    pub open_socket_count: usize,
    pub closed_socket_count: usize,
    pub filtered_socket_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortScanPortResult {
    pub host: String,
    pub ip: String,
    pub port: u16,
    pub protocol: String,
    pub status: String,
    pub reason: Option<String>,
    pub ttl: Option<u8>,
    pub service_name: Option<String>,
    pub product_name: Option<String>,
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub banner: Option<String>,
    pub server_header: Option<String>,
    pub title: Option<String>,
    pub status_code: Option<u16>,
    pub confidence: Option<f64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortScanSummary {
    pub engine: String,
    pub total_input_targets: usize,
    pub total_expanded_hosts: usize,
    pub total_requested_ports: usize,
    pub total_socket_attempts: usize,
    pub successful_scans: usize,
    pub failed_scans: usize,
    pub total_open_ports: usize,
    pub total_closed_ports: usize,
    pub total_filtered_ports: usize,
    pub total_settled_ports: usize,
    pub service_probed_ports: usize,
    pub service_probe_error: Option<String>,
    pub raw_source_port_first: Option<u16>,
    pub raw_source_port_last: Option<u16>,
    pub max_rate: Option<f64>,
    pub seed: Option<u64>,
    pub shard_index: Option<usize>,
    pub shard_count: Option<usize>,
    pub resume_index: Option<usize>,
    pub resume_count: Option<usize>,
    pub duration_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum PortScanError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("port scan failed: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Clone, Default)]
pub struct PortScanTool;

impl PortScanTool {
    pub const NAME: &'static str = "port_scan";
    pub const DESCRIPTION: &'static str = concat!(
        "Masscan-style TCP/UDP/SCTP port scanner for focused recon. Accepts single IPs, domains, CIDR blocks, ",
        "and hyphen IP ranges, plus masscan-style port expressions such as 80,443,U:53,S:2905,8000-8100. ",
        "Supports `connect` and `raw_syn` engines. Returns settled socket results grouped by host and flattened ",
        "as host/ip/protocol/port/status records. By default detailed output only includes open sockets, while summary ",
        "still tracks open/closed/filtered totals. Optionally fingerprints common HTTP/SSH TCP sockets with built-in ",
        "service probing and banner reads. Use for targeted network exposure checks, not for generic HTTP inspection or ",
        "internet-wide scans."
    );
}

impl Tool for PortScanTool {
    const NAME: &'static str = Self::NAME;
    type Args = PortScanArgs;
    type Output = PortScanOutput;
    type Error = PortScanError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(PortScanArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let started_at = Instant::now();
        let input_targets = normalize_input_targets(&args);
        let excluded_input_targets = normalize_excluded_targets(&args);
        if input_targets.is_empty() {
            return Err(PortScanError::InvalidArgs(
                "at least one target is required".to_string(),
            ));
        }

        let mut requested_ports = normalize_requested_ports(&args)?;
        let excluded_ports = normalize_excluded_ports(&args)?;
        if !excluded_ports.is_empty() {
            requested_ports.retain(|port| !excluded_ports.contains(port));
            if requested_ports.is_empty() {
                return Err(PortScanError::InvalidArgs(
                    "all requested ports were excluded".to_string(),
                ));
            }
        }
        if matches!(args.engine, PortScanEngine::Connect)
            && requested_ports
                .iter()
                .any(|port| port.protocol != ScanProtocol::Tcp)
        {
            return Err(PortScanError::InvalidArgs(
                "connect engine only supports TCP ports; use raw_syn for UDP or SCTP scans"
                    .to_string(),
            ));
        }

        let mut expanded_hosts = expand_target_specs(&input_targets, args.max_hosts)?;
        if !excluded_input_targets.is_empty() {
            let excluded_hosts = expand_target_specs(&excluded_input_targets, args.max_hosts)?
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>();
            expanded_hosts.retain(|host| !excluded_hosts.contains(host));
        }
        if expanded_hosts.is_empty() {
            return Err(PortScanError::InvalidArgs(
                "no scanable hosts remained after normalization".to_string(),
            ));
        }

        let rough_socket_attempts = expanded_hosts
            .len()
            .checked_mul(requested_ports.len())
            .ok_or_else(|| PortScanError::InvalidArgs("scan scope overflowed usize".to_string()))?;
        if rough_socket_attempts > args.max_socket_attempts {
            return Err(PortScanError::InvalidArgs(format!(
                "scan scope too large: {} hosts * {} ports = {} socket attempts exceeds max_socket_attempts {}",
                expanded_hosts.len(),
                requested_ports.len(),
                rough_socket_attempts,
                args.max_socket_attempts
            )));
        }

        emit_port_scan_progress(
            args.monitor_progress.as_ref(),
            0,
            5,
            Some(format!(
                "Prepared {} input target(s), {} expanded host(s), {} port target(s) with {} engine",
                input_targets.len(),
                expanded_hosts.len(),
                requested_ports.len(),
                args.engine.label()
            )),
            None,
            Some("preparing".to_string()),
            Some("Preparing targets".to_string()),
            Some(false),
        );

        let mut host_accumulators = BTreeMap::new();
        for host in &expanded_hosts {
            host_accumulators.insert(host.clone(), HostAccumulator::new(host));
        }

        emit_port_scan_progress(
            args.monitor_progress.as_ref(),
            1,
            5,
            Some(format!(
                "Resolving {} host(s) into socket targets",
                expanded_hosts.len()
            )),
            expanded_hosts.first().cloned(),
            Some("resolving_targets".to_string()),
            Some("Resolving targets".to_string()),
            Some(false),
        );

        let resolution_results = resolve_hosts(&expanded_hosts).await;
        let resolved_targets =
            collect_socket_targets(resolution_results, args.engine, &mut host_accumulators);
        let total_socket_attempts =
            count_socket_attempts(resolved_targets.len(), requested_ports.len())?;
        if total_socket_attempts > args.max_socket_attempts {
            return Err(PortScanError::InvalidArgs(format!(
                "resolved scan scope too large: {} socket attempts exceeds max_socket_attempts {}",
                total_socket_attempts, args.max_socket_attempts
            )));
        }

        emit_port_scan_progress(
            args.monitor_progress.as_ref(),
            2,
            5,
            Some(format!(
                "Scanning {} socket(s) across {} host(s) with {} engine",
                total_socket_attempts,
                expanded_hosts.len(),
                args.engine.label()
            )),
            expanded_hosts.first().cloned(),
            Some("scanning_ports".to_string()),
            Some("Scanning ports".to_string()),
            Some(false),
        );

        let scan_concurrency = args.concurrency.clamp(1, MAX_SCAN_CONCURRENCY);
        let scan_result = match args.engine {
            PortScanEngine::Connect => {
                let socket_tasks = build_socket_tasks(&resolved_targets, &requested_ports);
                scan_connect_sockets(
                    socket_tasks,
                    args.timeout_ms.max(100),
                    scan_concurrency,
                    args.tries.max(1),
                    args.monitor_progress.as_ref(),
                )
                .await
            }
            PortScanEngine::RawSyn => {
                scan_raw_syn_sockets(
                    resolved_targets,
                    requested_ports.clone(),
                    RawSynScanConfig {
                        timeout_ms: args.timeout_ms.max(100),
                        concurrency: scan_concurrency,
                        tries: args.tries.max(1),
                        ttl: args.raw_ttl.max(1),
                        device: args.device.clone(),
                        source_port_first: args.raw_source_port_first,
                        source_port_last: args.raw_source_port_last,
                        max_rate: args.max_rate,
                        seed: args.seed,
                        shard_index: args.shard_index,
                        shard_count: args.shard_count,
                        resume_index: args.resume_index,
                        resume_count: args.resume_count,
                        monitor_progress: args.monitor_progress.clone(),
                    },
                )
                .await?
            }
        };

        for (host, error) in scan_result.host_errors {
            if let Some(accumulator) = host_accumulators.get_mut(&host) {
                if accumulator.error.is_none() {
                    accumulator.error = Some(error);
                }
            }
        }

        let mut port_entries =
            port_entries_from_outcomes(scan_result.socket_outcomes, &mut host_accumulators);
        let service_probe = enrich_open_ports_with_service_probe(&args, &mut port_entries).await;

        port_entries.sort_by(|left, right| {
            (&left.host, &left.ip, left.protocol.as_str(), left.port).cmp(&(
                &right.host,
                &right.ip,
                right.protocol.as_str(),
                right.port,
            ))
        });

        let results: Vec<PortScanHostResult> = host_accumulators
            .into_values()
            .map(|accumulator| accumulator.into_result(args.include_closed, args.include_filtered))
            .collect();

        let successful_scans = results
            .iter()
            .filter(|result| result.error.is_none())
            .count();
        let failed_scans = results.len().saturating_sub(successful_scans);
        let total_open_ports = port_entries
            .iter()
            .filter(|entry| entry.status == "open")
            .count();
        let total_closed_ports = port_entries
            .iter()
            .filter(|entry| entry.status == "closed")
            .count();
        let total_filtered_ports = port_entries
            .iter()
            .filter(|entry| entry.status == "filtered")
            .count();
        let total_settled_ports = port_entries.len();
        let output = PortScanOutput {
            success: successful_scans > 0,
            data: PortScanData {
                input_targets: input_targets.clone(),
                expanded_hosts: expanded_hosts.clone(),
                requested_ports: requested_ports
                    .iter()
                    .copied()
                    .map(PortScanSocketRef::from)
                    .collect(),
                results,
                ports: port_entries
                    .into_iter()
                    .filter(|entry| {
                        should_include_port_result(
                            entry.status.as_str(),
                            args.include_closed,
                            args.include_filtered,
                        )
                    })
                    .collect(),
                summary: PortScanSummary {
                    engine: args.engine.label().to_string(),
                    total_input_targets: input_targets.len(),
                    total_expanded_hosts: expanded_hosts.len(),
                    total_requested_ports: requested_ports.len(),
                    total_socket_attempts,
                    successful_scans,
                    failed_scans,
                    total_open_ports,
                    total_closed_ports,
                    total_filtered_ports,
                    total_settled_ports,
                    service_probed_ports: service_probe.enriched_ports,
                    service_probe_error: service_probe.error,
                    raw_source_port_first: matches!(args.engine, PortScanEngine::RawSyn)
                        .then_some(args.raw_source_port_first.unwrap_or(40_000)),
                    raw_source_port_last: matches!(args.engine, PortScanEngine::RawSyn)
                        .then_some(args.raw_source_port_last.unwrap_or(u16::MAX)),
                    max_rate: matches!(args.engine, PortScanEngine::RawSyn)
                        .then_some(args.max_rate)
                        .flatten(),
                    seed: matches!(args.engine, PortScanEngine::RawSyn)
                        .then_some(args.seed)
                        .flatten(),
                    shard_index: matches!(args.engine, PortScanEngine::RawSyn)
                        .then_some(args.shard_index),
                    shard_count: matches!(args.engine, PortScanEngine::RawSyn)
                        .then_some(args.shard_count),
                    resume_index: matches!(args.engine, PortScanEngine::RawSyn)
                        .then_some(args.resume_index),
                    resume_count: matches!(args.engine, PortScanEngine::RawSyn)
                        .then_some(args.resume_count)
                        .flatten(),
                    duration_ms: started_at.elapsed().as_millis() as u64,
                },
            },
        };

        emit_port_scan_progress(
            args.monitor_progress.as_ref(),
            5,
            5,
            Some(format!(
                "Completed {} port scan with {} open, {} closed, {} filtered socket(s) across {} host(s)",
                args.engine.label(),
                output.data.summary.total_open_ports,
                output.data.summary.total_closed_ports,
                output.data.summary.total_filtered_ports,
                output.data.summary.total_expanded_hosts
            )),
            expanded_hosts.first().cloned(),
            Some("completed".to_string()),
            Some("Port scan completed".to_string()),
            Some(false),
        );

        Ok(output)
    }
}

pub(crate) fn emit_port_scan_progress(
    context: Option<&MonitorProgressContext>,
    current: u32,
    total: u32,
    message: Option<String>,
    current_target: Option<String>,
    phase: Option<String>,
    phase_label: Option<String>,
    indeterminate: Option<bool>,
) {
    let Some(context) = context else {
        return;
    };

    emit_plugin_monitor_progress(
        context,
        PluginMonitorProgressUpdate {
            current: Some(current),
            total: Some(total.max(1)),
            message,
            current_target,
            phase,
            phase_label,
            indeterminate,
        },
    );
}

#[derive(Debug, Default)]
pub(crate) struct ScanEngineResult {
    pub socket_outcomes: Vec<SocketOutcome>,
    pub host_errors: BTreeMap<String, String>,
}

#[derive(Debug, Default)]
struct ServiceProbeEnrichmentResult {
    enriched_ports: usize,
    error: Option<String>,
}

fn port_entries_from_outcomes(
    socket_outcomes: Vec<SocketOutcome>,
    host_accumulators: &mut BTreeMap<String, HostAccumulator>,
) -> Vec<PortScanPortResult> {
    let mut port_entries = Vec::with_capacity(socket_outcomes.len());
    for outcome in socket_outcomes {
        if let Some(accumulator) = host_accumulators.get_mut(&outcome.host) {
            accumulator.record_outcome(&outcome);
        }

        port_entries.push(PortScanPortResult {
            host: outcome.host,
            ip: outcome.ip.to_string(),
            port: outcome.port,
            protocol: outcome.protocol.label().to_string(),
            status: outcome.status.label().to_string(),
            reason: outcome.reason,
            ttl: outcome.ttl,
            service_name: None,
            product_name: None,
            vendor: None,
            version: None,
            banner: None,
            server_header: None,
            title: None,
            status_code: None,
            confidence: None,
            error: None,
        });
    }

    port_entries
}

fn should_include_port_result(status: &str, include_closed: bool, include_filtered: bool) -> bool {
    match status {
        "open" => true,
        "closed" => include_closed,
        "filtered" => include_filtered,
        _ => false,
    }
}

fn is_common_http_port(port: u16) -> bool {
    matches!(
        port,
        80 | 81 | 443 | 8000 | 8080 | 8081 | 8443 | 8888 | 9000
    )
}

fn is_common_ssh_port(port: u16) -> bool {
    matches!(port, 22 | 2222)
}

fn should_probe_common_service(entry: &PortScanPortResult) -> bool {
    entry.protocol == "tcp"
        && entry.status == "open"
        && (is_common_http_port(entry.port) || is_common_ssh_port(entry.port))
}

fn has_supported_common_service_evidence(result: &sentinel_plugins::ServiceProbeResult) -> bool {
    if !result.success || !result.available {
        return false;
    }

    match result.service_name.as_deref() {
        Some("http" | "https") => {
            result.status_code.is_some() || result.server_header.as_deref().is_some()
        }
        Some("ssh") => result
            .banner
            .as_deref()
            .map(|banner| banner.to_ascii_lowercase().contains("ssh"))
            .unwrap_or(false),
        _ => false,
    }
}

async fn enrich_open_ports_with_service_probe(
    args: &PortScanArgs,
    port_entries: &mut [PortScanPortResult],
) -> ServiceProbeEnrichmentResult {
    if !args.service_probe || port_entries.is_empty() {
        return ServiceProbeEnrichmentResult::default();
    }

    let tcp_indexes = port_entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| should_probe_common_service(entry).then_some(index))
        .collect::<Vec<_>>();
    if tcp_indexes.is_empty() {
        return ServiceProbeEnrichmentResult::default();
    }

    emit_port_scan_progress(
        args.monitor_progress.as_ref(),
        3,
        5,
        Some(format!(
            "Probing {} open socket(s) for services and banners",
            tcp_indexes.len()
        )),
        tcp_indexes
            .first()
            .and_then(|index| port_entries.get(*index))
            .map(|entry| format!("{}:{}", entry.ip, entry.port)),
        Some("probing_services".to_string()),
        Some("Probing services".to_string()),
        Some(false),
    );

    let probe_concurrency = args.probe_concurrency.clamp(1, MAX_PROBE_CONCURRENCY);
    let probe_response = probe_services(ServiceProbeRequest {
        targets: tcp_indexes
            .iter()
            .filter_map(|index| port_entries.get(*index))
            .map(|entry| ServiceProbeTarget {
                host: entry.host.clone(),
                connect_ip: Some(entry.ip.clone()),
                port: entry.port,
                protocol: "tcp".to_string(),
            })
            .collect(),
        rules: Vec::new(),
        dictionary_id: None,
        timeout_ms: Some(args.probe_timeout_ms.max(250)),
        concurrency: Some(probe_concurrency),
        follow_http_redirects: Some(args.follow_http_redirects),
        read_banner: Some(args.read_banner),
        engine: None,
        monitor_progress: None,
    })
    .await;

    if let Some(error) = probe_response.error.clone() {
        for index in tcp_indexes {
            if let Some(entry) = port_entries.get_mut(index) {
                entry.error = Some(error.clone());
            }
        }
        return ServiceProbeEnrichmentResult {
            enriched_ports: 0,
            error: Some(error),
        };
    }

    let mut probe_map = BTreeMap::new();
    for result in probe_response.results {
        let key = service_probe_result_key(
            &result.host,
            result.connect_ip.as_deref().unwrap_or(result.host.as_str()),
            result.port,
        );
        probe_map.insert(key, result);
    }

    let mut enriched = 0usize;
    for index in tcp_indexes {
        let Some(entry) = port_entries.get_mut(index) else {
            continue;
        };
        let key = service_probe_result_key(&entry.host, &entry.ip, entry.port);
        let Some(result) = probe_map.get(&key) else {
            continue;
        };

        if !has_supported_common_service_evidence(result) {
            entry.error = result.error.clone();
            continue;
        }

        entry.protocol = if result.protocol.trim().is_empty() {
            "tcp".to_string()
        } else {
            result.protocol.clone()
        };
        entry.service_name = result.service_name.clone();
        entry.product_name = result.product_name.clone();
        entry.vendor = result.vendor.clone();
        entry.version = result.version.clone();
        entry.banner = result.banner.clone();
        entry.server_header = result.server_header.clone();
        entry.title = result.title.clone();
        entry.status_code = result.status_code;
        entry.confidence = result.confidence;
        entry.error = result.error.clone();
        enriched += 1;
    }

    ServiceProbeEnrichmentResult {
        enriched_ports: enriched,
        error: None,
    }
}

fn service_probe_result_key(host: &str, connect_ip: &str, port: u16) -> String {
    let host = host.trim();
    let connect_ip = connect_ip.trim();
    if connect_ip.is_empty() || connect_ip == host {
        format!("{host}:{port}")
    } else {
        format!("{host}@{connect_ip}:{port}")
    }
}

pub(crate) fn scan_progress_step(current: usize, total: usize) -> u32 {
    if total == 0 {
        return 0;
    }

    ((current as f64 / total as f64) * 2.0)
        .round()
        .clamp(0.0, 2.0) as u32
}

pub(crate) fn progress_emit_every(total: usize) -> usize {
    (total / 100).max(1).min(256)
}

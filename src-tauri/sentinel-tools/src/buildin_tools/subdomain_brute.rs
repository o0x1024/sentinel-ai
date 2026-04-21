use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use rig::tool::Tool;
use rsubdomain::{
    BruteForceProgress, BruteForceProgressPhase, DnsRecord, DnsResolveResult, ProgressCallback,
    QueryType, SubdomainBruteConfig, SubdomainBruteEngine, SubdomainResult, VerifyResult,
};
use schemars::JsonSchema;
use sentinel_plugins::{
    dictionary_runtime, emit_plugin_monitor_progress, MonitorProgressContext,
    PluginMonitorProgressUpdate,
};
use serde::{Deserialize, Serialize};

const DEFAULT_DICTIONARY_LIMIT: i32 = 10_000;
const MAX_DICTIONARY_LIMIT: i32 = 100_000;
const FALLBACK_DICTIONARY: &[&str] = &[
    "www",
    "mail",
    "ftp",
    "api",
    "admin",
    "dev",
    "test",
    "staging",
    "beta",
    "cdn",
    "static",
    "img",
    "m",
    "app",
    "portal",
    "support",
    "docs",
    "vpn",
    "blog",
    "status",
    "mx",
    "ns1",
    "ns2",
    "autodiscover",
];

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubdomainBruteArgs {
    /// Root domains to brute force, such as example.com.
    #[serde(default)]
    pub targets: Vec<String>,
    /// Single root domain shortcut. Merged into `targets` during normalization.
    #[serde(default)]
    pub domain: Option<String>,
    /// Optional shared dictionary identifier from the runtime dictionary registry.
    #[serde(default)]
    pub dictionary_id: Option<String>,
    /// Inline subdomain prefixes. Used together with the runtime dictionary when provided.
    #[serde(default)]
    pub dictionary: Vec<String>,
    /// Custom DNS resolvers in host:port form.
    #[serde(default)]
    pub resolvers: Vec<String>,
    /// Skip wildcard domains to reduce noisy matches.
    #[serde(default = "default_skip_wildcard")]
    pub skip_wildcard: bool,
    /// Optional global bandwidth cap accepted by rsubdomain, for example `3M`.
    #[serde(default = "default_bandwidth_limit")]
    pub bandwidth_limit: Option<String>,
    /// Enable HTTP verification for discovered subdomains.
    #[serde(default)]
    pub verify_mode: bool,
    /// Maximum DNS retries per query.
    #[serde(default = "default_max_retries")]
    pub max_retries: u8,
    /// Maximum wait time in seconds for the brute force round to finish.
    #[serde(default = "default_max_wait_seconds")]
    pub max_wait_seconds: u64,
    /// HTTP verification timeout in seconds for each discovered subdomain.
    #[serde(default = "default_verify_timeout_seconds")]
    pub verify_timeout_seconds: u64,
    /// Maximum parallel HTTP verification workers.
    #[serde(default = "default_verify_concurrency")]
    pub verify_concurrency: usize,
    /// Resolve and return DNS records for matched subdomains.
    #[serde(default = "default_resolve_records")]
    pub resolve_records: bool,
    /// DNS record types to query. Defaults to `["a"]` when omitted.
    #[serde(default)]
    pub query_types: Vec<SubdomainQueryTypeInput>,
    /// Return raw DNS records from rsubdomain without extra normalization.
    #[serde(default)]
    pub raw_records: bool,
    /// Optional network device passed through to rsubdomain.
    #[serde(default)]
    pub device: Option<String>,
    /// Previous snapshots used to compute change events.
    #[serde(default)]
    pub previous_snapshots: HashMap<String, SubdomainSnapshot>,
    /// Maximum number of dictionary entries to use from the merged dictionary.
    #[serde(default = "default_dictionary_limit")]
    pub dictionary_limit: i32,
    #[schemars(skip)]
    #[serde(default, alias = "__monitorExecution")]
    pub monitor_progress: Option<MonitorProgressContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubdomainSnapshot {
    pub domain: String,
    pub subdomains: Vec<String>,
    pub last_checked: String,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SubdomainQueryTypeInput {
    A,
    Aaaa,
    Cname,
    Mx,
    Ns,
    Txt,
}

impl From<SubdomainQueryTypeInput> for QueryType {
    fn from(value: SubdomainQueryTypeInput) -> Self {
        match value {
            SubdomainQueryTypeInput::A => QueryType::A,
            SubdomainQueryTypeInput::Aaaa => QueryType::Aaaa,
            SubdomainQueryTypeInput::Cname => QueryType::Cname,
            SubdomainQueryTypeInput::Mx => QueryType::Mx,
            SubdomainQueryTypeInput::Ns => QueryType::Ns,
            SubdomainQueryTypeInput::Txt => QueryType::Txt,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SubdomainBruteOutput {
    pub success: bool,
    pub data: SubdomainBruteData,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubdomainBruteData {
    pub targets: Vec<String>,
    pub subdomains: Vec<String>,
    pub results: Vec<SubdomainBruteResultEntry>,
    pub change_events: Vec<ChangeEvent>,
    pub snapshots: HashMap<String, SubdomainSnapshot>,
    pub summary: SubdomainBruteSummary,
    pub surface_artifacts: SurfaceArtifacts,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubdomainBruteSummary {
    pub total_targets: usize,
    pub dictionary_size: usize,
    pub total_discovered: usize,
    pub domains_with_results: usize,
    pub alive_domains: usize,
    pub changes_detected: usize,
    pub dictionary_source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubdomainBruteResultEntry {
    pub root_domain: String,
    pub domain: String,
    pub ip: String,
    pub record_type: String,
    pub verified: Option<VerifiedDomain>,
    pub dns_records: Option<ResolvedDnsRecords>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifiedDomain {
    pub http_status: Option<u16>,
    pub https_status: Option<u16>,
    pub http_alive: bool,
    pub https_alive: bool,
    pub redirect_url: Option<String>,
    pub server_header: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedDnsRecords {
    pub domain: String,
    pub has_records: bool,
    pub records: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangeEvent {
    pub id: String,
    pub asset_id: String,
    pub event_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub detection_method: String,
    pub tags: Vec<String>,
    pub auto_trigger_enabled: bool,
    pub risk_score: i32,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SurfaceArtifacts {
    pub domains: Vec<serde_json::Value>,
    pub ips: Vec<serde_json::Value>,
    pub relations: Vec<serde_json::Value>,
    pub evidences: Vec<serde_json::Value>,
    pub changes: Vec<serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum SubdomainBruteError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("subdomain brute force failed: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Clone, Default)]
pub struct SubdomainBruteTool;

impl SubdomainBruteTool {
    pub const NAME: &'static str = "subdomain_brute";
    pub const DESCRIPTION: &'static str = concat!(
        "Dictionary-based DNS subdomain enumeration and monitoring for root domains, powered by rsubdomain. ",
        "Use when the input is a base domain such as example.com and you want to discover exposed subdomains, ",
        "compare against previous snapshots, or generate DNS change events. Best for asset discovery and scheduled monitoring, ",
        "not for generic web search or single-URL HTTP probing."
    );
}

impl Tool for SubdomainBruteTool {
    const NAME: &'static str = Self::NAME;
    type Args = SubdomainBruteArgs;
    type Output = SubdomainBruteOutput;
    type Error = SubdomainBruteError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SubdomainBruteArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let targets = normalize_input_targets(&args);
        if targets.is_empty() {
            return Err(SubdomainBruteError::InvalidArgs(
                "at least one domain target is required".to_string(),
            ));
        }

        emit_subdomain_progress(
            args.monitor_progress.as_ref(),
            0,
            4,
            Some(format!(
                "Preparing {} target(s) for dictionary brute force",
                targets.len()
            )),
            targets.first().cloned(),
            Some("preparing".to_string()),
            Some("Preparing targets".to_string()),
            Some(true),
        );

        let dictionary_limit = args.dictionary_limit.clamp(1, MAX_DICTIONARY_LIMIT);
        let (dictionary, dictionary_source) = load_dictionary_words(&args, dictionary_limit).await;
        if dictionary.is_empty() {
            return Err(SubdomainBruteError::InvalidArgs(
                "dictionary is empty after normalization".to_string(),
            ));
        }

        emit_subdomain_progress(
            args.monitor_progress.as_ref(),
            1,
            4,
            Some(format!(
                "Loaded {} dictionary entries from {}",
                dictionary.len(),
                dictionary_source
            )),
            targets.first().cloned(),
            Some("dictionary_loaded".to_string()),
            Some("Dictionary loaded".to_string()),
            Some(false),
        );

        let config = build_brute_config(&args, &targets, &dictionary);
        emit_subdomain_progress(
            args.monitor_progress.as_ref(),
            2,
            4,
            Some(format!(
                "Brute forcing {} target(s) with {} candidate subdomains",
                targets.len(),
                dictionary.len()
            )),
            targets.first().cloned(),
            Some("bruteforcing".to_string()),
            Some("Brute forcing DNS".to_string()),
            Some(true),
        );
        let raw_results = execute_subdomain_brute(config).await?;

        emit_subdomain_progress(
            args.monitor_progress.as_ref(),
            3,
            4,
            Some(format!(
                "Processing {} discovered result(s) into monitor artifacts",
                raw_results.len()
            )),
            targets.first().cloned(),
            Some("processing_results".to_string()),
            Some("Processing results".to_string()),
            Some(false),
        );

        let timestamp = chrono::Utc::now().to_rfc3339();
        let grouped = group_subdomains_by_root(&targets, &raw_results);

        let mut snapshots = HashMap::new();
        let mut change_events = Vec::new();

        for target in &targets {
            let subdomains = grouped.get(target).cloned().unwrap_or_default();
            snapshots.insert(
                target.clone(),
                SubdomainSnapshot {
                    domain: target.clone(),
                    subdomains: subdomains.clone(),
                    last_checked: timestamp.clone(),
                },
            );

            if let Some(previous) = args.previous_snapshots.get(target) {
                let previous_set: BTreeSet<String> = previous.subdomains.iter().cloned().collect();
                let current_set: BTreeSet<String> = subdomains.iter().cloned().collect();
                let added: Vec<String> = current_set.difference(&previous_set).cloned().collect();
                let removed: Vec<String> = previous_set.difference(&current_set).cloned().collect();

                if !added.is_empty() {
                    change_events.push(create_subdomain_change_event(
                        target,
                        "asset_discovered",
                        &added,
                        &timestamp,
                    ));
                }
                if !removed.is_empty() {
                    change_events.push(create_subdomain_change_event(
                        target,
                        "asset_removed",
                        &removed,
                        &timestamp,
                    ));
                }
            }
        }

        let result_entries: Vec<SubdomainBruteResultEntry> = raw_results
            .iter()
            .map(|result| SubdomainBruteResultEntry {
                root_domain: match_root_domain(&targets, &result.domain),
                domain: result.domain.clone(),
                ip: result.ip.clone(),
                record_type: result.record_type.clone(),
                verified: result.verified.as_ref().map(convert_verified_domain),
                dns_records: result.dns_records.as_ref().map(convert_dns_records),
            })
            .collect();

        let subdomains = unique_sorted(
            raw_results
                .iter()
                .map(|result| result.domain.clone())
                .collect(),
        );
        let alive_domains = result_entries
            .iter()
            .filter(|entry| {
                entry
                    .verified
                    .as_ref()
                    .map(|value| value.http_alive || value.https_alive)
                    .unwrap_or(false)
            })
            .count();

        let surface_artifacts = build_surface_artifacts(&targets, &result_entries, &change_events);
        let output = SubdomainBruteOutput {
            success: true,
            data: SubdomainBruteData {
                targets: targets.clone(),
                subdomains,
                results: result_entries,
                change_events: change_events.clone(),
                snapshots,
                summary: SubdomainBruteSummary {
                    total_targets: targets.len(),
                    dictionary_size: dictionary.len(),
                    total_discovered: raw_results.len(),
                    domains_with_results: grouped
                        .values()
                        .filter(|items| !items.is_empty())
                        .count(),
                    alive_domains,
                    changes_detected: change_events.len(),
                    dictionary_source,
                },
                surface_artifacts,
            },
        };

        emit_subdomain_progress(
            args.monitor_progress.as_ref(),
            4,
            4,
            Some(format!(
                "Completed subdomain brute force. Discovered {} subdomain(s) across {} target(s)",
                output.data.summary.total_discovered, output.data.summary.total_targets
            )),
            targets.first().cloned(),
            Some("completed".to_string()),
            Some("Subdomain brute force completed".to_string()),
            Some(false),
        );

        Ok(output)
    }
}

fn emit_subdomain_progress(
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

fn build_brute_config(
    args: &SubdomainBruteArgs,
    targets: &[String],
    dictionary: &[String],
) -> SubdomainBruteConfig {
    let query_types = if args.query_types.is_empty() {
        vec![QueryType::A]
    } else {
        args.query_types
            .iter()
            .copied()
            .map(QueryType::from)
            .collect()
    };

    SubdomainBruteConfig {
        domains: targets.to_vec(),
        resolvers: args
            .resolvers
            .iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect(),
        dictionary_file: None,
        dictionary: Some(dictionary.to_vec()),
        skip_wildcard: args.skip_wildcard,
        bandwidth_limit: args.bandwidth_limit.clone(),
        verify_mode: args.verify_mode,
        max_retries: args.max_retries,
        max_wait_seconds: args.max_wait_seconds,
        verify_timeout_seconds: args.verify_timeout_seconds,
        verify_concurrency: args.verify_concurrency.max(1),
        resolve_records: args.resolve_records,
        query_types,
        silent: true,
        raw_records: args.raw_records,
        device: args.device.clone(),
        progress_callback: build_progress_callback(args.monitor_progress.as_ref()),
    }
}

fn build_progress_callback(context: Option<&MonitorProgressContext>) -> Option<ProgressCallback> {
    let context = context.cloned()?;

    Some(Arc::new(move |progress: BruteForceProgress| {
        let total_units = saturating_progress_units(progress.total_queries.max(1));
        let completed_units = match progress.phase {
            BruteForceProgressPhase::SendingQueries => {
                saturating_progress_units(progress.sent_queries)
            }
            BruteForceProgressPhase::WaitingForResponses | BruteForceProgressPhase::Completed => {
                total_units
            }
        };

        let (phase, phase_label, indeterminate, message) = match progress.phase {
            BruteForceProgressPhase::SendingQueries => (
                "bruteforcing".to_string(),
                "Brute forcing DNS".to_string(),
                false,
                format!(
                    "Sent {}/{} DNS queries, discovered {} subdomain(s)",
                    progress.sent_queries, progress.total_queries, progress.discovered_domains
                ),
            ),
            BruteForceProgressPhase::WaitingForResponses => (
                "waiting_responses".to_string(),
                "Waiting for DNS responses".to_string(),
                true,
                format!(
                    "All {} DNS queries sent, waiting for responses, discovered {} subdomain(s)",
                    progress.total_queries, progress.discovered_domains
                ),
            ),
            BruteForceProgressPhase::Completed => (
                "dns_queries_completed".to_string(),
                "DNS brute force finished".to_string(),
                false,
                format!(
                    "DNS brute force finished with {} discovered subdomain(s)",
                    progress.discovered_domains
                ),
            ),
        };

        emit_plugin_monitor_progress(
            &context,
            PluginMonitorProgressUpdate {
                current: Some(completed_units),
                total: Some(total_units),
                message: Some(message),
                current_target: progress.current_target.clone(),
                phase: Some(phase),
                phase_label: Some(phase_label),
                indeterminate: Some(indeterminate),
            },
        );
    }))
}

fn saturating_progress_units(value: usize) -> u32 {
    value.min(u32::MAX as usize) as u32
}

async fn execute_subdomain_brute(
    config: SubdomainBruteConfig,
) -> Result<Vec<SubdomainResult>, SubdomainBruteError> {
    tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .map_err(|error| SubdomainBruteError::ExecutionFailed(error.to_string()))?;
        let local = tokio::task::LocalSet::new();

        local.block_on(&runtime, async move {
            let engine = SubdomainBruteEngine::new(config)
                .await
                .map_err(|error| SubdomainBruteError::ExecutionFailed(error.to_string()))?;
            engine
                .run_brute_force()
                .await
                .map_err(|error| SubdomainBruteError::ExecutionFailed(error.to_string()))
        })
    })
    .await
    .map_err(|error| SubdomainBruteError::ExecutionFailed(error.to_string()))?
}

fn default_skip_wildcard() -> bool {
    true
}

fn default_resolve_records() -> bool {
    true
}

fn default_max_retries() -> u8 {
    5
}

fn default_max_wait_seconds() -> u64 {
    300
}

fn default_verify_timeout_seconds() -> u64 {
    10
}

fn default_verify_concurrency() -> usize {
    50
}

fn default_bandwidth_limit() -> Option<String> {
    Some("3M".to_string())
}

fn default_dictionary_limit() -> i32 {
    DEFAULT_DICTIONARY_LIMIT
}

fn normalize_input_targets(args: &SubdomainBruteArgs) -> Vec<String> {
    let mut candidates = args.targets.clone();
    if let Some(domain) = &args.domain {
        candidates.push(domain.clone());
    }

    let normalized = unique_sorted(
        candidates
            .into_iter()
            .filter_map(|value| normalize_domain_target(&value))
            .collect(),
    );

    reduce_to_root_targets(normalized)
}

fn normalize_domain_target(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_scheme = trimmed
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("ws://")
        .trim_start_matches("wss://");
    let host_port = without_scheme
        .split('/')
        .next()
        .unwrap_or(without_scheme)
        .trim();
    let host = host_port
        .trim_start_matches("*.")
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(':')
        .next()
        .unwrap_or(host_port)
        .trim()
        .trim_end_matches('.')
        .to_lowercase();

    if host.is_empty() || host.contains(' ') || !host.contains('.') {
        return None;
    }

    Some(host)
}

fn reduce_to_root_targets(mut domains: Vec<String>) -> Vec<String> {
    domains.sort_by(|left, right| {
        left.matches('.')
            .count()
            .cmp(&right.matches('.').count())
            .then_with(|| left.cmp(right))
    });

    let mut selected: Vec<String> = Vec::new();
    for domain in domains {
        let covered = selected
            .iter()
            .any(|root| domain == *root || domain.ends_with(&format!(".{root}")));
        if !covered {
            selected.push(domain);
        }
    }
    selected
}

async fn load_dictionary_words(
    args: &SubdomainBruteArgs,
    dictionary_limit: i32,
) -> (Vec<String>, String) {
    if !args.dictionary.is_empty() {
        return (
            unique_sorted(
                args.dictionary
                    .iter()
                    .map(|item| item.trim().to_lowercase())
                    .filter(|item| !item.is_empty())
                    .collect(),
            ),
            "inline".to_string(),
        );
    }

    if let Some(dictionary_id) = args.dictionary_id.as_ref().map(|item| item.trim()) {
        if !dictionary_id.is_empty() {
            if let Ok(words) = dictionary_runtime::get_dictionary_words(
                dictionary_id.to_string(),
                Some(dictionary_limit),
            )
            .await
            {
                let normalized = unique_sorted(words);
                if !normalized.is_empty() {
                    return (normalized, dictionary_id.to_string());
                }
            }
        }
    }

    if let Ok(default_id) =
        dictionary_runtime::get_default_dictionary_id("subdomain".to_string()).await
    {
        let trimmed = default_id.trim();
        if !trimmed.is_empty() {
            if let Ok(words) = dictionary_runtime::get_dictionary_words(
                trimmed.to_string(),
                Some(dictionary_limit),
            )
            .await
            {
                let normalized = unique_sorted(words);
                if !normalized.is_empty() {
                    return (normalized, trimmed.to_string());
                }
            }
        }
    }

    if let Ok(words) = dictionary_runtime::get_dictionary_words(
        "builtin_subdomain_common".to_string(),
        Some(dictionary_limit),
    )
    .await
    {
        let normalized = unique_sorted(words);
        if !normalized.is_empty() {
            return (normalized, "builtin_subdomain_common".to_string());
        }
    }

    (
        unique_sorted(
            FALLBACK_DICTIONARY
                .iter()
                .map(|item| (*item).to_string())
                .collect(),
        ),
        "fallback_builtin".to_string(),
    )
}

fn unique_sorted(values: Vec<String>) -> Vec<String> {
    let mut set = BTreeSet::new();
    for value in values {
        let trimmed = value.trim().to_lowercase();
        if !trimmed.is_empty() {
            set.insert(trimmed);
        }
    }
    set.into_iter().collect()
}

fn group_subdomains_by_root(
    targets: &[String],
    results: &[SubdomainResult],
) -> HashMap<String, Vec<String>> {
    let mut grouped: HashMap<String, Vec<String>> = targets
        .iter()
        .map(|target| (target.clone(), Vec::new()))
        .collect();

    for result in results {
        let root = match_root_domain(targets, &result.domain);
        grouped.entry(root).or_default().push(result.domain.clone());
    }

    for subdomains in grouped.values_mut() {
        *subdomains = unique_sorted(std::mem::take(subdomains));
    }

    grouped
}

fn match_root_domain(targets: &[String], domain: &str) -> String {
    targets
        .iter()
        .filter(|target| domain == target.as_str() || domain.ends_with(&format!(".{target}")))
        .max_by_key(|target| target.len())
        .cloned()
        .unwrap_or_else(|| domain.to_string())
}

fn convert_verified_domain(value: &VerifyResult) -> VerifiedDomain {
    VerifiedDomain {
        http_status: value.http_status,
        https_status: value.https_status,
        http_alive: value.http_alive,
        https_alive: value.https_alive,
        redirect_url: value.redirect_url.clone(),
        server_header: value.server_header.clone(),
        title: value.title.clone(),
    }
}

fn convert_dns_records(value: &DnsResolveResult) -> ResolvedDnsRecords {
    let mut records = HashMap::new();
    for (record_type, items) in &value.records {
        records.insert(
            record_type.clone(),
            items.iter().map(format_dns_record).collect(),
        );
    }

    ResolvedDnsRecords {
        domain: value.domain.clone(),
        has_records: value.has_records,
        records,
    }
}

fn format_dns_record(record: &DnsRecord) -> String {
    match record {
        DnsRecord::A(value)
        | DnsRecord::AAAA(value)
        | DnsRecord::CNAME(value)
        | DnsRecord::NS(value)
        | DnsRecord::TXT(value)
        | DnsRecord::SOA(value)
        | DnsRecord::PTR(value) => value.clone(),
        DnsRecord::MX(priority, value) => format!("{priority} {value}"),
    }
}

fn create_subdomain_change_event(
    target: &str,
    event_type: &str,
    subdomains: &[String],
    timestamp: &str,
) -> ChangeEvent {
    let discovered = event_type == "asset_discovered";
    let severity = if subdomains.len() >= 10 {
        "medium"
    } else {
        "low"
    };
    let risk_score = if discovered {
        44 + subdomains.len().min(10) as i32
    } else {
        56 + subdomains.len().min(10) as i32
    };

    ChangeEvent {
        id: format!("{event_type}-{target}-{timestamp}")
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                    ch
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .chars()
            .take(120)
            .collect(),
        asset_id: target.to_string(),
        event_type: event_type.to_string(),
        severity: severity.to_string(),
        title: if discovered {
            format!("New subdomains discovered for {target}")
        } else {
            format!("Subdomains no longer observed for {target}")
        },
        description: if discovered {
            format!(
                "Discovered {} subdomain(s): {}",
                subdomains.len(),
                subdomains.join(", ")
            )
        } else {
            format!(
                "No longer observed {} subdomain(s): {}",
                subdomains.len(),
                subdomains.join(", ")
            )
        },
        old_value: (!discovered).then(|| subdomains.join("\n")),
        new_value: discovered.then(|| subdomains.join("\n")),
        detection_method: "subdomain_brute".to_string(),
        tags: vec![
            "subdomain".to_string(),
            if discovered {
                "discovered".to_string()
            } else {
                "removed".to_string()
            },
            "dns".to_string(),
            "dictionary".to_string(),
        ],
        auto_trigger_enabled: true,
        risk_score,
        metadata: serde_json::json!({
            "domain": target,
            "subdomains": subdomains,
        }),
    }
}

fn build_surface_artifacts(
    _targets: &[String],
    results: &[SubdomainBruteResultEntry],
    change_events: &[ChangeEvent],
) -> SurfaceArtifacts {
    let mut domains = BTreeSet::new();
    let mut ips = BTreeSet::new();
    let mut domain_items = Vec::new();
    let mut ip_items = Vec::new();
    let mut relations = BTreeSet::new();
    let mut relation_items = Vec::new();
    let mut evidences = Vec::new();

    for result in results {
        if domains.insert(result.domain.clone()) {
            domain_items.push(serde_json::json!({
                "fqdn": result.domain,
                "root_domain": result.root_domain,
                "main_domain": result.root_domain,
                "source": "subdomain_brute",
                "confidence": 0.98,
            }));
        }

        if result.domain != result.root_domain {
            let relation_key = format!(
                "{}->{}->contains_subdomain",
                result.root_domain, result.domain
            );
            if relations.insert(relation_key) {
                relation_items.push(serde_json::json!({
                    "from_type": "domain",
                    "from_key": result.root_domain,
                    "to_type": "domain",
                    "to_key": result.domain,
                    "relation_type": "contains_subdomain",
                    "source": "subdomain_brute",
                    "confidence": 0.98,
                }));
            }
        }

        let ip_value = result.ip.trim();
        if !ip_value.is_empty() && ip_value.parse::<std::net::IpAddr>().is_ok() {
            if ips.insert(ip_value.to_string()) {
                ip_items.push(serde_json::json!({
                    "ip_address": ip_value,
                    "ip_version": if ip_value.contains(':') { "IPv6" } else { "IPv4" },
                    "source": "subdomain_brute",
                    "confidence": 0.98,
                }));
            }

            let relation_key = format!("{}->{}->resolves_to", result.domain, ip_value);
            if relations.insert(relation_key) {
                relation_items.push(serde_json::json!({
                    "from_type": "domain",
                    "from_key": result.domain,
                    "to_type": "ip",
                    "to_key": ip_value,
                    "relation_type": "resolves_to",
                    "source": "subdomain_brute",
                    "confidence": 0.98,
                }));
            }
        }

        evidences.push(serde_json::json!({
            "asset_type": "domain",
            "asset_key": result.domain,
            "evidence_type": "subdomain_bruteforce",
            "title": format!("Dictionary brute force discovery: {}", result.domain),
            "content_json": {
                "root_domain": result.root_domain,
                "ip": result.ip,
                "record_type": result.record_type,
                "verified": result.verified,
                "dns_records": result.dns_records,
            },
            "source": "subdomain_brute",
        }));
    }

    SurfaceArtifacts {
        domains: domain_items,
        ips: ip_items,
        relations: relation_items,
        evidences,
        changes: change_events
            .iter()
            .map(|event| {
                serde_json::json!({
                    "asset_key": event.asset_id,
                    "asset_type": "domain",
                    "change_type": event.event_type,
                    "severity": event.severity,
                    "title": event.title,
                    "description": event.description,
                    "old_value": event.old_value,
                    "new_value": event.new_value,
                    "risk_score": event.risk_score,
                    "source": "subdomain_brute",
                    "metadata": event.metadata,
                })
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_args() -> SubdomainBruteArgs {
        SubdomainBruteArgs {
            targets: vec!["example.com".to_string()],
            domain: None,
            dictionary_id: None,
            dictionary: vec!["www".to_string()],
            resolvers: vec!["1.1.1.1:53".to_string()],
            skip_wildcard: default_skip_wildcard(),
            bandwidth_limit: default_bandwidth_limit(),
            verify_mode: false,
            max_retries: default_max_retries(),
            max_wait_seconds: default_max_wait_seconds(),
            verify_timeout_seconds: default_verify_timeout_seconds(),
            verify_concurrency: default_verify_concurrency(),
            resolve_records: default_resolve_records(),
            query_types: Vec::new(),
            raw_records: false,
            device: None,
            previous_snapshots: HashMap::new(),
            dictionary_limit: default_dictionary_limit(),
            monitor_progress: None,
        }
    }

    #[test]
    fn query_type_input_maps_to_rsubdomain_query_type() {
        assert!(matches!(
            QueryType::from(SubdomainQueryTypeInput::A),
            QueryType::A
        ));
        assert!(matches!(
            QueryType::from(SubdomainQueryTypeInput::Aaaa),
            QueryType::Aaaa
        ));
        assert!(matches!(
            QueryType::from(SubdomainQueryTypeInput::Cname),
            QueryType::Cname
        ));
        assert!(matches!(
            QueryType::from(SubdomainQueryTypeInput::Mx),
            QueryType::Mx
        ));
        assert!(matches!(
            QueryType::from(SubdomainQueryTypeInput::Ns),
            QueryType::Ns
        ));
        assert!(matches!(
            QueryType::from(SubdomainQueryTypeInput::Txt),
            QueryType::Txt
        ));
    }

    #[test]
    fn build_brute_config_defaults_to_a_query_type() {
        let args = sample_args();
        let config = build_brute_config(&args, &args.targets, &args.dictionary);

        assert_eq!(config.domains, vec!["example.com".to_string()]);
        assert_eq!(config.dictionary, Some(vec!["www".to_string()]));
        assert!(matches!(config.query_types.as_slice(), [QueryType::A]));
    }

    #[test]
    fn build_brute_config_clamps_verify_concurrency() {
        let mut args = sample_args();
        args.verify_mode = true;
        args.verify_concurrency = 0;
        args.query_types = vec![SubdomainQueryTypeInput::Aaaa, SubdomainQueryTypeInput::Txt];

        let config = build_brute_config(&args, &args.targets, &args.dictionary);

        assert_eq!(config.verify_concurrency, 1);
        assert_eq!(config.query_types.len(), 2);
        assert!(matches!(config.query_types[0], QueryType::Aaaa));
        assert!(matches!(config.query_types[1], QueryType::Txt));
    }
}

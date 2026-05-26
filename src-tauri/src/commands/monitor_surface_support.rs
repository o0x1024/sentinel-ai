use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;

use chrono::Utc;
use sentinel_bounty::services::{MonitorPluginConfig, MonitorTask};
use sentinel_db::{
    derive_domain_hierarchy, Database, DatabaseService, ProgramScopeRow, SurfaceAssetFilter,
    SurfaceAssetRow, SurfaceDiscoveryRunRow, SurfaceObservationRow,
};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::commands::monitor_surface::{
    materialize_surface_artifacts, SurfaceMaterializationStats,
};

const MONITOR_TARGET_PAGE_SIZE: i64 = 5_000;
const HTTP_SERVICE_PORTS: [i32; 4] = [80, 443, 8080, 8443];
const HTTPS_SERVICE_PORTS: [i32; 3] = [443, 8443, 9443];

fn extract_host(value: &str) -> Option<String> {
    let normalized = value
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("wss://")
        .trim_start_matches("ws://")
        .trim_matches('/')
        .to_string();
    let host_port = normalized.split('/').next()?.trim();
    if host_port.is_empty() {
        return None;
    }

    Some(
        host_port
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(':')
            .next()
            .unwrap_or(host_port)
            .to_string(),
    )
}

#[derive(Debug, Clone, Default)]
struct MonitorScopeBoundary {
    domains: HashSet<String>,
    ips: HashSet<String>,
}

impl MonitorScopeBoundary {
    fn is_empty(&self) -> bool {
        self.domains.is_empty() && self.ips.is_empty()
    }
}

fn normalize_scope_boundary_value(value: &str) -> Option<String> {
    let normalized = extract_host(value).unwrap_or_else(|| value.trim().to_string());
    let normalized = normalized
        .trim()
        .trim_start_matches("*.")
        .trim_start_matches('.')
        .trim()
        .to_lowercase();
    if normalized.is_empty() {
        return None;
    }
    Some(normalized)
}

fn build_monitor_scope_boundary(scopes: &[ProgramScopeRow]) -> MonitorScopeBoundary {
    let mut boundary = MonitorScopeBoundary::default();

    for scope in scopes {
        if scope.scope_type != "in_scope" {
            continue;
        }

        let Some(normalized) = normalize_scope_boundary_value(&scope.target) else {
            continue;
        };

        match scope.target_type.as_str() {
            "ip" => {
                boundary.ips.insert(normalized);
            }
            "url" | "domain" | "wildcard" | "host" => {
                boundary.domains.insert(normalized);
            }
            _ => {}
        }
    }

    boundary
}

fn monitor_scope_boundary_allows_target(boundary: &MonitorScopeBoundary, target: &str) -> bool {
    if boundary.is_empty() {
        return true;
    }

    let Some(normalized) = normalize_scope_boundary_value(target) else {
        return false;
    };

    if normalized.parse::<IpAddr>().is_ok() {
        return boundary.ips.contains(&normalized);
    }

    if boundary.domains.contains(&normalized) {
        return true;
    }

    boundary
        .domains
        .iter()
        .any(|domain| normalized.ends_with(&format!(".{domain}")))
}

pub(crate) fn extract_surface_artifacts(output: &Value) -> Option<Map<String, Value>> {
    output
        .get("data")
        .and_then(|data| data.get("surface_artifacts"))
        .and_then(Value::as_object)
        .cloned()
        .or_else(|| {
            output
                .get("surface_artifacts")
                .and_then(Value::as_object)
                .cloned()
        })
}

pub(crate) fn surface_artifact_item_count(surface_artifacts: &Map<String, Value>) -> i32 {
    surface_artifacts
        .values()
        .filter(|payload| !payload.is_null())
        .map(|payload| {
            payload
                .as_array()
                .map(|items| items.len() as i32)
                .unwrap_or(1)
        })
        .sum()
}

pub(crate) fn surface_observation_count_from_output(
    output: &Value,
    surface_artifacts: &Map<String, Value>,
) -> i32 {
    output
        .get("data")
        .and_then(|data| data.get("summary"))
        .and_then(|summary| {
            summary
                .get("totalTargets")
                .or_else(|| summary.get("scannedTargets"))
                .or_else(|| summary.get("targets_scanned"))
        })
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .or_else(|| {
            output
                .get("data")
                .and_then(|data| data.get("results"))
                .and_then(Value::as_array)
                .map(|items| items.len() as i32)
                .filter(|value| *value > 0)
        })
        .unwrap_or_else(|| surface_artifact_item_count(surface_artifacts))
}

pub(crate) async fn ingest_surface_plugin_output(
    db_service: &Arc<DatabaseService>,
    program_id: &str,
    plugin_id: &str,
    trigger_source: &str,
    schedule_id: Option<&str>,
    output: &Value,
    metadata: Option<Value>,
) -> Result<Option<SurfaceMaterializationStats>, String> {
    let Some(surface_artifacts) = extract_surface_artifacts(output) else {
        return Ok(None);
    };

    let run_id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();
    let run = SurfaceDiscoveryRunRow {
        id: run_id.clone(),
        program_id: program_id.to_string(),
        trigger_source: trigger_source.to_string(),
        schedule_id: schedule_id.map(str::to_string),
        workflow_id: None,
        workflow_template_id: None,
        plugin_id: Some(plugin_id.to_string()),
        status: "running".to_string(),
        seed_count: None,
        observation_count: Some(0),
        imported_asset_count: Some(0),
        changed_asset_count: Some(0),
        error_message: None,
        started_at: started_at.clone(),
        completed_at: None,
        metadata_json: metadata.map(|value| value.to_string()),
    };

    db_service
        .create_surface_discovery_run(&run)
        .await
        .map_err(|e| e.to_string())?;

    let mut observations = Vec::with_capacity(surface_artifacts.len());
    for (artifact_type, payload) in &surface_artifacts {
        if payload.is_null() {
            continue;
        }

        observations.push(SurfaceObservationRow {
            id: Uuid::new_v4().to_string(),
            run_id: run_id.clone(),
            program_id: program_id.to_string(),
            artifact_type: artifact_type.clone(),
            object_key: None,
            payload_json: payload.to_string(),
            source_plugin: Some(plugin_id.to_string()),
            confidence_score: None,
            observed_at: Utc::now().to_rfc3339(),
            normalized: false,
            metadata_json: Some(
                json!({
                    "trigger_source": trigger_source,
                    "plugin_id": plugin_id,
                    "schedule_id": schedule_id,
                })
                .to_string(),
            ),
        });
    }

    db_service
        .create_surface_observations(&observations)
        .await
        .map_err(|e| e.to_string())?;

    let stats = materialize_surface_artifacts(
        db_service,
        program_id,
        Some(&run_id),
        plugin_id,
        &surface_artifacts,
    )
    .await?;
    let observation_count = surface_observation_count_from_output(output, &surface_artifacts);
    let affected_asset_count = stats.created_assets + stats.enriched_assets;

    db_service
        .update_surface_discovery_run(
            &run_id,
            "completed",
            Some(observation_count),
            Some(affected_asset_count as i32),
            Some(stats.changed_assets as i32),
            None,
            Some(&Utc::now().to_rfc3339()),
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(Some(stats))
}

pub(crate) async fn collect_monitor_targets(
    db_service: &Arc<DatabaseService>,
    program_id: &str,
) -> Result<Vec<String>, String> {
    let scopes = db_service
        .list_program_scopes(Some(program_id), None)
        .await
        .map_err(|e| e.to_string())?;
    let scope_boundary = build_monitor_scope_boundary(&scopes);

    let mut seen = HashSet::new();
    let mut targets = Vec::new();

    for scope in scopes {
        if matches!(
            scope.target_type.as_str(),
            "wildcard" | "domain" | "url" | "ip"
        ) {
            let target = scope.target.trim().to_string();
            if !target.is_empty() && seen.insert(target.clone()) {
                targets.push(target);
            }
        }
    }

    if let Ok(assets) = db_service
        .list_bounty_assets(
            Some(program_id),
            None,
            Some("domain"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
    {
        for asset in assets {
            if asset.asset_type == "domain" && asset.is_wildcard != Some(true) {
                let target = asset.canonical_url.trim().to_string();
                if !target.is_empty()
                    && monitor_scope_boundary_allows_target(&scope_boundary, &target)
                    && seen.insert(target.clone())
                {
                    targets.push(target);
                }
            }
        }
    }

    visit_surface_assets(
        db_service,
        &SurfaceAssetFilter {
            program_id: Some(program_id.to_string()),
            asset_type: None,
            status: None,
            search: None,
            favicon_hash: None,
            has_favicon_hash: None,
            http_status_code: None,
            service_name: None,
            transport_protocol: None,
            view_state: None,
            is_favorite: None,
            column_filters: None,
            limit: None,
            offset: None,
        },
        |asset| {
            if !matches!(
                asset.asset_type.as_str(),
                "domain" | "url" | "web" | "website" | "host" | "ip"
            ) {
                return;
            }
            let target = asset.asset_name.trim().to_string();
            if !target.is_empty()
                && monitor_scope_boundary_allows_target(&scope_boundary, &target)
                && seen.insert(target.clone())
            {
                targets.push(target);
            }
        },
    )
    .await?;

    Ok(targets)
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MonitorResolvedTargets {
    pub targets: Vec<String>,
    pub target_objects: Vec<Value>,
    pub extra_input: Map<String, Value>,
}

#[derive(Debug, Clone, Copy)]
enum MonitorInputMode {
    Asset,
    Seed,
    Hybrid,
}

fn push_unique_resolved_target(
    resolved: &mut MonitorResolvedTargets,
    seen: &mut HashSet<String>,
    target: &str,
    target_object: Value,
) {
    let normalized = target.trim().to_string();
    if !normalized.is_empty() && seen.insert(normalized.clone()) {
        resolved.targets.push(normalized);
        resolved.target_objects.push(target_object);
    }
}

fn finalize_monitor_resolved_targets(
    _plugin_id: &str,
    resolved: MonitorResolvedTargets,
) -> MonitorResolvedTargets {
    resolved
}

fn monitor_target_source_label(value: &str) -> &str {
    match value {
        "scope" => "项目范围",
        "surface_asset" => "资产库",
        "bounty_asset" => "漏洞赏金资产",
        "seed" => "项目种子",
        "fallback" => "兜底目标",
        _ => value,
    }
}

fn monitor_target_type_label(value: &str) -> &str {
    match value {
        "domain" => "域名",
        "web" => "Web",
        "service" => "服务",
        "ip" => "IP",
        "host" => "主机",
        "api" => "API清单",
        "generic" => "通用",
        _ => value,
    }
}

pub(crate) fn format_monitor_target_breakdown(resolved: &MonitorResolvedTargets) -> Option<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for target_object in &resolved.target_objects {
        let Some(object) = target_object.as_object() else {
            continue;
        };
        let source = object
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let target_type = object
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let label = format!(
            "{}/{}",
            monitor_target_source_label(source),
            monitor_target_type_label(target_type)
        );
        *counts.entry(label).or_default() += 1;
    }

    if counts.is_empty() {
        return None;
    }

    let parts = counts
        .into_iter()
        .map(|(label, count)| format!("{label} {count}"))
        .collect::<Vec<_>>();
    let label = format!("目标来源：{}", parts.join(", "));
    Some(label)
}

fn normalize_monitor_target_asset_type(value: &str) -> Option<&'static str> {
    match value.trim().to_lowercase().as_str() {
        "url" | "web" | "website" => Some("web"),
        "domain" | "wildcard" => Some("domain"),
        "domain_root" | "root_domain" => Some("domain_root"),
        "domain_level_1" | "subdomain_level_1" | "first_level_subdomain" => Some("domain_level_1"),
        "domain_level_2" | "subdomain_level_2" | "second_level_subdomain" => Some("domain_level_2"),
        "domain_level_3_plus" | "subdomain_level_3_plus" | "third_level_subdomain" => {
            Some("domain_level_3_plus")
        }
        "host" | "hostname" => Some("host"),
        "ip" | "ip_address" => Some("ip"),
        "service" | "port" | "endpoint" => Some("service"),
        "api" | "api_path" | "api_endpoint" => Some("api"),
        _ => None,
    }
}

fn requested_domain_target_types(requested_asset_types: &HashSet<String>) -> bool {
    requested_asset_types.contains("domain")
        || requested_asset_types.contains("domain_root")
        || requested_asset_types.contains("domain_level_1")
        || requested_asset_types.contains("domain_level_2")
        || requested_asset_types.contains("domain_level_3_plus")
}

fn requested_domain_hierarchy_matches(
    requested_asset_types: &HashSet<String>,
    value: &str,
    root_domain: Option<&str>,
    subdomain_level: Option<i32>,
) -> bool {
    if !requested_domain_target_types(requested_asset_types) {
        return false;
    }

    if requested_asset_types.contains("domain") {
        return true;
    }

    let derived = derive_domain_hierarchy(value);
    let effective_root_domain = root_domain
        .filter(|candidate| !candidate.trim().is_empty())
        .map(str::to_string)
        .or_else(|| derived.as_ref().map(|item| item.root_domain.clone()));
    let effective_subdomain_level =
        subdomain_level.or_else(|| derived.as_ref().map(|item| item.subdomain_level));

    if effective_root_domain.is_none() || effective_subdomain_level.is_none() {
        return false;
    }

    match effective_subdomain_level.unwrap_or_default() {
        0 => requested_asset_types.contains("domain_root"),
        1 => requested_asset_types.contains("domain_level_1"),
        2 => requested_asset_types.contains("domain_level_2"),
        level if level >= 3 => requested_asset_types.contains("domain_level_3_plus"),
        _ => false,
    }
}

fn normalized_monitor_plugin_id(plugin_id: &str) -> &str {
    plugin_id.strip_prefix("plugin__").unwrap_or(plugin_id)
}

fn plugin_uses_in_scope_domain_targets_only(plugin_id: &str) -> bool {
    matches!(plugin_id, "subdomain_enumerator" | "subdomain_brute")
}

fn normalize_plugin_input_mode(value: Option<&str>) -> Option<MonitorInputMode> {
    match value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "asset" => Some(MonitorInputMode::Asset),
        "seed" => Some(MonitorInputMode::Seed),
        "hybrid" => Some(MonitorInputMode::Hybrid),
        _ => None,
    }
}

async fn inject_monitor_seed_inputs(
    db_service: &Arc<DatabaseService>,
    task: &MonitorTask,
    plugin: &MonitorPluginConfig,
    resolved: &mut MonitorResolvedTargets,
) -> Result<(), String> {
    let seed_config = plugin.normalized_seed_config();
    if seed_config.bindings.is_empty() {
        return Ok(());
    }

    db_service
        .rename_surface_seed_type("fofa_icon_hash", "favicon_hash")
        .await
        .map_err(|e| e.to_string())?;
    let seeds = db_service
        .list_surface_seeds(Some(&task.program_id), Some("active"))
        .await
        .map_err(|e| e.to_string())?;
    let mut seed_groups: HashMap<String, Vec<String>> = HashMap::new();
    let mut seen_seed_keys = HashSet::new();

    for binding in seed_config.bindings {
        if binding.use_project_seeds {
            let selected_project_values: HashSet<String> = binding
                .selected_project_values
                .iter()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect();

            for seed in seeds
                .iter()
                .filter(|seed| seed.seed_type == binding.seed_type)
            {
                if !selected_project_values.is_empty()
                    && !selected_project_values.contains(&seed.seed_value)
                {
                    continue;
                }

                let dedupe_key = format!("{}::{}", binding.input_key, seed.seed_value);
                if !seen_seed_keys.insert(dedupe_key) {
                    continue;
                }
                seed_groups
                    .entry(binding.input_key.clone())
                    .or_default()
                    .push(seed.seed_value.clone());
            }
        }

        for manual_value in binding.manual_values {
            let dedupe_key = format!("{}::{}", binding.input_key, manual_value);
            if !seen_seed_keys.insert(dedupe_key) {
                continue;
            }
            seed_groups
                .entry(binding.input_key.clone())
                .or_default()
                .push(manual_value);
        }
    }

    for (key, values) in seed_groups {
        resolved.extra_input.insert(
            key,
            Value::Array(values.into_iter().map(Value::String).collect()),
        );
    }

    Ok(())
}

async fn resolve_plugin_input_mode(
    db_service: &Arc<DatabaseService>,
    plugin: &MonitorPluginConfig,
) -> MonitorInputMode {
    let normalized_plugin_id = normalized_monitor_plugin_id(&plugin.plugin_id);

    let plugin_metadata = db_service
        .get_plugin_from_registry(normalized_plugin_id)
        .await
        .ok()
        .flatten()
        .map(|record| record.metadata);

    normalize_plugin_input_mode(
        plugin_metadata
            .as_ref()
            .and_then(|metadata| metadata.input_mode.as_deref()),
    )
    .unwrap_or(MonitorInputMode::Asset)
}

fn service_port_from_details(details: &Map<String, Value>) -> Option<i32> {
    details
        .get("port_number")
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
}

fn service_protocol_from_details(details: &Map<String, Value>) -> Option<String> {
    details
        .get("protocol_name")
        .and_then(Value::as_str)
        .or_else(|| details.get("transport_protocol").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase())
}

fn service_name_from_details(details: &Map<String, Value>) -> Option<String> {
    details
        .get("application_service_name")
        .and_then(Value::as_str)
        .or_else(|| details.get("protocol_name").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase())
}

fn service_encrypted_transport_from_details(details: &Map<String, Value>) -> Option<bool> {
    details.get("encrypted_transport").and_then(Value::as_bool)
}

fn is_http_like_service(
    port: Option<i32>,
    protocol: Option<&str>,
    service_name: Option<&str>,
) -> bool {
    let normalized_protocol = protocol
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase());
    let normalized_service_name = service_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase());

    if matches!(normalized_protocol.as_deref(), Some("http" | "https")) {
        return true;
    }
    if matches!(normalized_service_name.as_deref(), Some("http" | "https")) {
        return true;
    }

    port.is_some_and(|value| HTTP_SERVICE_PORTS.contains(&value))
}

fn is_https_like_service(
    port: Option<i32>,
    protocol: Option<&str>,
    service_name: Option<&str>,
    encrypted_transport: Option<bool>,
) -> bool {
    let normalized_protocol = protocol
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase());
    let normalized_service_name = service_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase());

    if matches!(
        normalized_protocol.as_deref(),
        Some("https" | "tls" | "ssl" | "https-alt")
    ) {
        return true;
    }
    if matches!(
        normalized_service_name.as_deref(),
        Some("https" | "tls" | "ssl" | "https-alt")
    ) {
        return true;
    }
    if matches!(encrypted_transport, Some(true))
        && is_http_like_service(port, protocol, service_name)
    {
        return true;
    }

    port.is_some_and(|value| HTTPS_SERVICE_PORTS.contains(&value))
}

fn build_surface_service_target_object(
    asset: &SurfaceAssetRow,
    details: Option<&Map<String, Value>>,
) -> Value {
    let host = extract_host(&asset.asset_name);
    let port = details.and_then(service_port_from_details);
    let protocol = details.and_then(service_protocol_from_details);
    let service_name = details.and_then(service_name_from_details);

    json!({
        "type": "service",
        "value": &asset.asset_name,
        "source": "surface_asset",
        "asset_id": &asset.id,
        "host": host,
        "port": port,
        "protocol": protocol,
        "service_name": service_name,
    })
}

fn format_service_target(
    hostname: Option<&str>,
    ip_or_host: Option<&str>,
    port: Option<i32>,
    fallback: Option<&str>,
) -> Option<String> {
    if let Some(port) = port.filter(|port| *port > 0) {
        if let Some(hostname) = hostname.map(str::trim).filter(|value| !value.is_empty()) {
            return Some(format!("{hostname}:{port}"));
        }
        if let Some(ip_or_host) = ip_or_host.map(str::trim).filter(|value| !value.is_empty()) {
            return Some(format!("{ip_or_host}:{port}"));
        }
    }

    fallback
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

async fn visit_surface_assets<F>(
    db_service: &Arc<DatabaseService>,
    base_filter: &SurfaceAssetFilter,
    mut visitor: F,
) -> Result<(), String>
where
    F: FnMut(SurfaceAssetRow),
{
    let mut offset = 0i64;

    loop {
        let assets = db_service
            .list_surface_assets(&SurfaceAssetFilter {
                program_id: base_filter.program_id.clone(),
                asset_type: base_filter.asset_type.clone(),
                status: base_filter.status.clone(),
                search: base_filter.search.clone(),
                favicon_hash: base_filter.favicon_hash.clone(),
                has_favicon_hash: base_filter.has_favicon_hash,
                http_status_code: base_filter.http_status_code,
                service_name: base_filter.service_name.clone(),
                transport_protocol: base_filter.transport_protocol.clone(),
                view_state: base_filter.view_state.clone(),
                is_favorite: base_filter.is_favorite,
                column_filters: base_filter.column_filters.clone(),
                limit: Some(MONITOR_TARGET_PAGE_SIZE),
                offset: Some(offset),
            })
            .await
            .map_err(|e| e.to_string())?;

        let fetched = assets.len();
        if fetched == 0 {
            break;
        }

        for asset in assets {
            visitor(asset);
        }

        if fetched < MONITOR_TARGET_PAGE_SIZE as usize {
            break;
        }

        offset += MONITOR_TARGET_PAGE_SIZE;
    }

    Ok(())
}

async fn resolve_plugin_target_asset_types(
    db_service: &Arc<DatabaseService>,
    plugin: &MonitorPluginConfig,
    input_mode: MonitorInputMode,
) -> Vec<String> {
    let normalized_plugin_id = normalized_monitor_plugin_id(&plugin.plugin_id);

    if matches!(input_mode, MonitorInputMode::Seed) {
        return Vec::new();
    }

    let plugin_metadata = db_service
        .get_plugin_from_registry(normalized_plugin_id)
        .await
        .ok()
        .flatten()
        .map(|record| record.metadata);

    let Some(plugin_metadata) = plugin_metadata else {
        return plugin.resolved_target_asset_types(&[]);
    };

    plugin.resolved_target_asset_types(&plugin_metadata.target_asset_types)
}

fn plugin_declares_seed_config(plugin: &MonitorPluginConfig) -> bool {
    !plugin.normalized_seed_config().bindings.is_empty()
}

pub(crate) async fn collect_monitor_target_payload_for_plugin(
    db_service: &Arc<DatabaseService>,
    task: &MonitorTask,
    plugin: &MonitorPluginConfig,
) -> Result<MonitorResolvedTargets, String> {
    let normalized_plugin_id = normalized_monitor_plugin_id(&plugin.plugin_id);
    let http_prober_mode = normalized_plugin_id == "http_prober";
    let https_service_targets_only = matches!(normalized_plugin_id, "cert_monitor" | "ssl_scanner");
    let input_mode = resolve_plugin_input_mode(db_service, plugin).await;
    let in_scope_domain_targets_only =
        plugin_uses_in_scope_domain_targets_only(normalized_plugin_id);
    let mut requested_asset_types: HashSet<String> =
        resolve_plugin_target_asset_types(db_service, plugin, input_mode)
            .await
            .into_iter()
            .filter_map(|value| normalize_monitor_target_asset_type(&value).map(str::to_string))
            .collect();
    if normalized_plugin_id == "cert_monitor" {
        requested_asset_types.clear();
        requested_asset_types.insert("domain".to_string());
    }

    if matches!(input_mode, MonitorInputMode::Seed) {
        let mut resolved = MonitorResolvedTargets::default();
        inject_monitor_seed_inputs(db_service, task, plugin, &mut resolved).await?;
        return Ok(finalize_monitor_resolved_targets(
            normalized_plugin_id,
            resolved,
        ));
    }

    if requested_asset_types.is_empty() && !plugin_declares_seed_config(plugin) {
        let targets = collect_monitor_targets(db_service, &task.program_id).await?;
        let target_objects = targets
            .iter()
            .map(|target| json!({ "type": "generic", "value": target, "source": "fallback" }))
            .collect();
        return Ok(finalize_monitor_resolved_targets(
            normalized_plugin_id,
            MonitorResolvedTargets {
                targets,
                target_objects,
                extra_input: Map::new(),
            },
        ));
    }

    let scopes = db_service
        .list_program_scopes(
            Some(&task.program_id),
            in_scope_domain_targets_only.then_some("in_scope"),
        )
        .await
        .map_err(|e| e.to_string())?;
    let scope_boundary = build_monitor_scope_boundary(&scopes);

    let mut seen = HashSet::new();
    let mut resolved = MonitorResolvedTargets::default();
    let mut surface_domain_assets = Vec::new();
    let mut surface_service_assets = Vec::new();
    for scope in scopes {
        match scope.target_type.as_str() {
            "url" if requested_asset_types.contains("web") => push_unique_resolved_target(
                &mut resolved,
                &mut seen,
                &scope.target,
                json!({ "type": "web", "value": scope.target, "source": "scope" }),
            ),
            "wildcard" | "domain"
                if requested_domain_hierarchy_matches(
                    &requested_asset_types,
                    &scope.target,
                    None,
                    None,
                ) =>
            {
                push_unique_resolved_target(
                    &mut resolved,
                    &mut seen,
                    &scope.target,
                    json!({ "type": "domain", "value": scope.target, "source": "scope" }),
                )
            }
            "ip" if requested_asset_types.contains("ip") => push_unique_resolved_target(
                &mut resolved,
                &mut seen,
                &scope.target,
                json!({ "type": "ip", "value": scope.target, "source": "scope" }),
            ),
            _ => {}
        }
    }

    if in_scope_domain_targets_only {
        if matches!(input_mode, MonitorInputMode::Hybrid) {
            inject_monitor_seed_inputs(db_service, task, plugin, &mut resolved).await?;
        }
        return Ok(finalize_monitor_resolved_targets(
            normalized_plugin_id,
            resolved,
        ));
    }

    visit_surface_assets(
        db_service,
        &SurfaceAssetFilter {
            program_id: Some(task.program_id.clone()),
            asset_type: None,
            status: None,
            search: None,
            favicon_hash: None,
            has_favicon_hash: None,
            http_status_code: None,
            service_name: None,
            transport_protocol: None,
            view_state: None,
            is_favorite: None,
            column_filters: None,
            limit: None,
            offset: None,
        },
        |asset| {
            if !monitor_scope_boundary_allows_target(&scope_boundary, &asset.asset_name) {
                return;
            }

            if matches!(asset.asset_type.as_str(), "url" | "web" | "website")
                && requested_asset_types.contains("web")
            {
                push_unique_resolved_target(
                    &mut resolved,
                    &mut seen,
                    &asset.asset_name,
                    json!({
                        "type": "web",
                        "value": &asset.asset_name,
                        "source": "surface_asset",
                        "asset_id": &asset.id,
                    }),
                );
            }

            let should_include = match asset.asset_type.as_str() {
                "domain" => false,
                "host" => requested_asset_types.contains("host"),
                "ip" => requested_asset_types.contains("ip"),
                _ => false,
            };
            if should_include {
                push_unique_resolved_target(
                    &mut resolved,
                    &mut seen,
                    &asset.asset_name,
                    json!({
                        "type": asset.asset_type.as_str(),
                        "value": &asset.asset_name,
                        "source": "surface_asset",
                        "asset_id": &asset.id,
                    }),
                );
            }

            if asset.asset_type == "domain" && requested_domain_target_types(&requested_asset_types)
            {
                surface_domain_assets.push(asset.clone());
            }

            if requested_asset_types.contains("service")
                && matches!(asset.asset_type.as_str(), "service" | "port")
            {
                if http_prober_mode {
                    if asset.asset_type == "service" {
                        surface_service_assets.push(asset);
                    }
                } else if https_service_targets_only {
                    if asset.asset_type == "service" {
                        surface_service_assets.push(asset);
                    }
                } else {
                    push_unique_resolved_target(
                        &mut resolved,
                        &mut seen,
                        &asset.asset_name,
                        json!({
                            "type": "service",
                            "value": &asset.asset_name,
                            "source": "surface_asset",
                            "asset_id": &asset.id,
                        }),
                    );
                }
            }
        },
    )
    .await?;

    let domain_details_by_id = if !surface_domain_assets.is_empty()
        && requested_domain_target_types(&requested_asset_types)
    {
        Some(
            db_service
                .list_surface_typed_details_map(&surface_domain_assets)
                .await
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };

    if requested_domain_target_types(&requested_asset_types) && !surface_domain_assets.is_empty() {
        for asset in &surface_domain_assets {
            let details = domain_details_by_id
                .as_ref()
                .and_then(|items| items.get(&asset.id))
                .and_then(Value::as_object);
            let root_domain = details
                .and_then(|item| item.get("root_domain"))
                .and_then(Value::as_str);
            let subdomain_level = details
                .and_then(|item| item.get("subdomain_level"))
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok());

            if !requested_domain_hierarchy_matches(
                &requested_asset_types,
                &asset.asset_name,
                root_domain,
                subdomain_level,
            ) {
                continue;
            }

            push_unique_resolved_target(
                &mut resolved,
                &mut seen,
                &asset.asset_name,
                json!({
                    "type": "domain",
                    "value": &asset.asset_name,
                    "source": "surface_asset",
                    "asset_id": &asset.id,
                    "root_domain": root_domain,
                    "subdomain_level": subdomain_level,
                }),
            );
        }
    }

    if matches!(input_mode, MonitorInputMode::Hybrid) {
        inject_monitor_seed_inputs(db_service, task, plugin, &mut resolved).await?;
    }

    if requested_asset_types.contains("service")
        && (http_prober_mode || https_service_targets_only)
        && !surface_service_assets.is_empty()
    {
        let typed_details_by_id = db_service
            .list_surface_typed_details_map(&surface_service_assets)
            .await
            .map_err(|e| e.to_string())?;

        for asset in &surface_service_assets {
            let details = typed_details_by_id
                .get(&asset.id)
                .and_then(Value::as_object);
            let port = details.and_then(service_port_from_details);
            let protocol = details
                .and_then(service_protocol_from_details)
                .unwrap_or_default();
            let service_name = details
                .and_then(service_name_from_details)
                .unwrap_or_default();
            let encrypted_transport = details.and_then(service_encrypted_transport_from_details);

            let protocol_ref = (!protocol.is_empty()).then_some(protocol.as_str());
            let service_name_ref = (!service_name.is_empty()).then_some(service_name.as_str());
            let should_include = if https_service_targets_only {
                is_https_like_service(port, protocol_ref, service_name_ref, encrypted_transport)
            } else {
                is_http_like_service(port, protocol_ref, service_name_ref)
            };

            if !should_include {
                continue;
            }

            push_unique_resolved_target(
                &mut resolved,
                &mut seen,
                &asset.asset_name,
                build_surface_service_target_object(asset, details),
            );
        }
    }

    if let Ok(assets) = db_service
        .list_bounty_assets(
            Some(&task.program_id),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
    {
        for asset in assets {
            if !monitor_scope_boundary_allows_target(&scope_boundary, &asset.canonical_url) {
                continue;
            }

            let should_include = match asset.asset_type.as_str() {
                "domain" => {
                    asset.is_wildcard != Some(true)
                        && requested_domain_hierarchy_matches(
                            &requested_asset_types,
                            &asset.canonical_url,
                            asset.root_domain.as_deref(),
                            asset.subdomain_level,
                        )
                }
                "url" | "website" | "web" => requested_asset_types.contains("web"),
                "host" => requested_asset_types.contains("host"),
                "ip" => requested_asset_types.contains("ip"),
                _ => false,
            };
            if should_include {
                push_unique_resolved_target(
                    &mut resolved,
                    &mut seen,
                    &asset.canonical_url,
                    json!({
                        "type": asset.asset_type.as_str(),
                        "value": &asset.canonical_url,
                        "source": "bounty_asset",
                        "asset_id": &asset.id,
                    }),
                );
            }

            if requested_asset_types.contains("service")
                && matches!(asset.asset_type.as_str(), "service" | "port")
            {
                if http_prober_mode && asset.asset_type != "service" {
                    continue;
                }
                if https_service_targets_only && asset.asset_type != "service" {
                    continue;
                }
                let canonical_host = extract_host(&asset.canonical_url);
                if let Some(target) = format_service_target(
                    asset.hostname.as_deref(),
                    canonical_host.as_deref(),
                    asset.port,
                    Some(&asset.canonical_url),
                ) {
                    if http_prober_mode
                        && !is_http_like_service(
                            asset.port,
                            asset.transport_protocol.as_deref(),
                            asset.service_name.as_deref(),
                        )
                    {
                        continue;
                    }
                    if https_service_targets_only
                        && !is_https_like_service(
                            asset.port,
                            asset
                                .protocol
                                .as_deref()
                                .or(asset.transport_protocol.as_deref()),
                            asset.service_name.as_deref(),
                            asset.ssl_enabled,
                        )
                    {
                        continue;
                    }
                    push_unique_resolved_target(
                        &mut resolved,
                        &mut seen,
                        &target,
                        json!({
                            "type": "service",
                            "value": &target,
                            "source": "bounty_asset",
                            "asset_id": &asset.id,
                            "host": asset.hostname.as_deref().or(canonical_host.as_deref()),
                            "port": asset.port,
                            "protocol": asset.transport_protocol.as_deref(),
                            "service_name": asset.service_name.as_deref(),
                            "ssl_enabled": asset.ssl_enabled,
                        }),
                    );
                }
            }
        }
    }

    if requested_asset_types.contains("api") {
        if let Ok(observations) = db_service
            .list_latest_surface_observations_by_target(
                Some(&task.program_id),
                Some("api_monitor"),
                Some("api_snapshot"),
            )
            .await
        {
            let mut api_paths: Vec<String> = Vec::new();
            let mut api_path_seen: HashSet<String> = HashSet::new();

            for obs in &observations {
                let base_url = match &obs.object_key {
                    Some(key) => key.trim_end_matches('/').to_string(),
                    None => continue,
                };

                let payload: Value = match serde_json::from_str(&obs.payload_json) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let endpoints = payload
                    .get("snapshot")
                    .and_then(|s| s.get("apiEndpoints"))
                    .and_then(Value::as_array);

                if let Some(endpoints) = endpoints {
                    for ep in endpoints {
                        if let Some(path) = ep.get("path").and_then(Value::as_str) {
                            let full_url = if path.starts_with('/') {
                                format!("{}{}", base_url, path)
                            } else {
                                format!("{}/{}", base_url, path)
                            };
                            if api_path_seen.insert(full_url.clone()) {
                                api_paths.push(full_url);
                            }
                        }
                    }
                }
            }

            if !api_paths.is_empty() {
                resolved.extra_input.insert(
                    "api_paths".to_string(),
                    Value::Array(
                        api_paths
                            .into_iter()
                            .map(Value::String)
                            .collect(),
                    ),
                );
            }
        }
    }

    Ok(finalize_monitor_resolved_targets(
        normalized_plugin_id,
        resolved,
    ))
}

#[cfg(test)]
mod tests {
    use super::surface_observation_count_from_output;
    use serde_json::{json, Map, Value};

    #[test]
    fn observation_count_prefers_plugin_total_targets() {
        let output = json!({
            "data": {
                "summary": {
                    "totalTargets": 1835,
                    "successfulFingerprints": 6
                },
                "surface_artifacts": {
                    "fingerprints": [],
                    "evidences": [{}, {}, {}, {}, {}, {}]
                }
            }
        });
        let surface_artifacts = output
            .get("data")
            .and_then(|data| data.get("surface_artifacts"))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_else(Map::new);

        assert_eq!(
            surface_observation_count_from_output(&output, &surface_artifacts),
            1835
        );
    }

    #[test]
    fn observation_count_falls_back_to_artifact_items() {
        let mut surface_artifacts = Map::new();
        surface_artifacts.insert("fingerprints".to_string(), json!([]));
        surface_artifacts.insert("evidences".to_string(), json!([{}, {}]));

        assert_eq!(
            surface_observation_count_from_output(&json!({}), &surface_artifacts),
            2
        );
    }
}

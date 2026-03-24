use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;

use chrono::Utc;
use sentinel_db::{
    DatabaseService, SurfaceAssetFilter, SurfaceDiscoveryRunRow, SurfaceObservationRow,
};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::commands::monitor_surface::materialize_surface_artifacts;

fn normalize_host_like(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("wss://")
        .trim_start_matches("ws://")
        .trim_matches('/')
        .to_string()
}

fn extract_host(value: &str) -> Option<String> {
    let normalized = normalize_host_like(value);
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

fn is_ip_literal(value: &str) -> bool {
    IpAddr::from_str(value).is_ok()
}

fn push_unique(items: &mut Vec<Value>, seen: &mut HashSet<String>, key: String, value: Value) {
    if seen.insert(key) {
        items.push(value);
    }
}

#[allow(clippy::too_many_arguments)]
fn absorb_web_artifact(
    url: &str,
    domains: &mut Vec<Value>,
    ips: &mut Vec<Value>,
    webs: &mut Vec<Value>,
    relations: &mut Vec<Value>,
    seen_domains: &mut HashSet<String>,
    seen_ips: &mut HashSet<String>,
    seen_webs: &mut HashSet<String>,
    seen_relations: &mut HashSet<String>,
) {
    let canonical_url = url.trim().to_string();
    if canonical_url.is_empty() {
        return;
    }

    push_unique(
        webs,
        seen_webs,
        canonical_url.clone(),
        json!({ "canonical_url": canonical_url, "source": "legacy_output" }),
    );

    if let Some(host) = extract_host(url) {
        if is_ip_literal(&host) {
            push_unique(
                ips,
                seen_ips,
                host.clone(),
                json!({ "ip_address": host, "source": "legacy_output" }),
            );
            let relation_key = format!("ip:{host}->web:{url}");
            push_unique(
                relations,
                seen_relations,
                relation_key,
                json!({
                    "from_type": "ip",
                    "from_key": host,
                    "to_type": "web",
                    "to_key": url,
                    "relation_type": "exposes_web"
                }),
            );
        } else {
            push_unique(
                domains,
                seen_domains,
                host.clone(),
                json!({ "fqdn": host, "source": "legacy_output" }),
            );
            let relation_key = format!("domain:{host}->web:{url}");
            push_unique(
                relations,
                seen_relations,
                relation_key,
                json!({
                    "from_type": "domain",
                    "from_key": host,
                    "to_type": "web",
                    "to_key": url,
                    "relation_type": "exposes_web"
                }),
            );
        }
    }
}

fn synthesize_surface_artifacts(output: &Value) -> Option<Map<String, Value>> {
    let data = output.get("data").unwrap_or(output);
    let mut artifacts = Map::new();

    let mut domains = Vec::new();
    let mut ips = Vec::new();
    let mut webs = Vec::new();
    let mut relations = Vec::new();

    let mut seen_domains = HashSet::new();
    let mut seen_ips = HashSet::new();
    let mut seen_webs = HashSet::new();
    let mut seen_relations = HashSet::new();

    if let Some(items) = data.get("subdomains").and_then(Value::as_array) {
        for item in items {
            let Some(fqdn) = item
                .as_str()
                .or_else(|| item.get("subdomain").and_then(Value::as_str))
                .or_else(|| item.get("domain").and_then(Value::as_str))
                .map(normalize_host_like)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };

            push_unique(
                &mut domains,
                &mut seen_domains,
                fqdn.clone(),
                json!({ "fqdn": fqdn, "source": "legacy_output" }),
            );
        }
    }

    if let Some(items) = data.get("ips").and_then(Value::as_array) {
        for item in items {
            let Some(ip) = item
                .as_str()
                .or_else(|| item.get("ip").and_then(Value::as_str))
                .or_else(|| item.get("ip_address").and_then(Value::as_str))
                .or_else(|| item.get("address").and_then(Value::as_str))
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };

            push_unique(
                &mut ips,
                &mut seen_ips,
                ip.to_string(),
                json!({ "ip_address": ip, "source": "legacy_output" }),
            );
        }
    }

    if let Some(items) = data.get("urls").and_then(Value::as_array) {
        for item in items {
            let Some(url) = item
                .as_str()
                .or_else(|| item.get("url").and_then(Value::as_str))
                .or_else(|| item.get("value").and_then(Value::as_str))
            else {
                continue;
            };
            absorb_web_artifact(
                url,
                &mut domains,
                &mut ips,
                &mut webs,
                &mut relations,
                &mut seen_domains,
                &mut seen_ips,
                &mut seen_webs,
                &mut seen_relations,
            );
        }
    }

    if let Some(items) = data.get("results").and_then(Value::as_array) {
        for item in items {
            if item.get("alive").and_then(Value::as_bool) == Some(false) {
                continue;
            }
            if let Some(url) = item.get("url").and_then(Value::as_str) {
                absorb_web_artifact(
                    url,
                    &mut domains,
                    &mut ips,
                    &mut webs,
                    &mut relations,
                    &mut seen_domains,
                    &mut seen_ips,
                    &mut seen_webs,
                    &mut seen_relations,
                );
            }
        }
    }

    if let Some(items) = data.get("assets").and_then(Value::as_array) {
        for item in items {
            let asset_type = item.get("type").and_then(Value::as_str).unwrap_or("domain");
            match asset_type {
                "domain" => {
                    if let Some(fqdn) = item
                        .get("value")
                        .or_else(|| item.get("domain"))
                        .and_then(Value::as_str)
                        .map(normalize_host_like)
                        .filter(|value| !value.is_empty())
                    {
                        push_unique(
                            &mut domains,
                            &mut seen_domains,
                            fqdn.clone(),
                            json!({ "fqdn": fqdn, "source": "legacy_output" }),
                        );
                    }
                }
                "ip" => {
                    if let Some(ip) = item
                        .get("value")
                        .or_else(|| item.get("ip"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                    {
                        push_unique(
                            &mut ips,
                            &mut seen_ips,
                            ip.to_string(),
                            json!({ "ip_address": ip, "source": "legacy_output" }),
                        );
                    }
                }
                "web" | "website" | "url" => {
                    if let Some(url) = item
                        .get("value")
                        .or_else(|| item.get("url"))
                        .and_then(Value::as_str)
                    {
                        absorb_web_artifact(
                            url,
                            &mut domains,
                            &mut ips,
                            &mut webs,
                            &mut relations,
                            &mut seen_domains,
                            &mut seen_ips,
                            &mut seen_webs,
                            &mut seen_relations,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    if !domains.is_empty() {
        artifacts.insert("domains".to_string(), Value::Array(domains));
    }
    if !ips.is_empty() {
        artifacts.insert("ips".to_string(), Value::Array(ips));
    }
    if !webs.is_empty() {
        artifacts.insert("webs".to_string(), Value::Array(webs));
    }
    if !relations.is_empty() {
        artifacts.insert("relations".to_string(), Value::Array(relations));
    }

    if let Some(changes) = data.get("changes").and_then(Value::as_array) {
        if !changes.is_empty() {
            artifacts.insert("changes".to_string(), Value::Array(changes.clone()));
        }
    }

    if let Some(certificates) = data.get("certificates").and_then(Value::as_array) {
        if !certificates.is_empty() {
            artifacts.insert(
                "certificates".to_string(),
                Value::Array(certificates.clone()),
            );
        }
    }

    if artifacts.is_empty() {
        None
    } else {
        Some(artifacts)
    }
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
        .or_else(|| synthesize_surface_artifacts(output))
}

pub(crate) async fn ingest_surface_plugin_output(
    db_service: &Arc<DatabaseService>,
    program_id: &str,
    plugin_id: &str,
    trigger_source: &str,
    schedule_id: Option<&str>,
    output: &Value,
    metadata: Option<Value>,
) -> Result<usize, String> {
    let Some(surface_artifacts) = extract_surface_artifacts(output) else {
        return Ok(0);
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

    let mut observation_count = 0i32;
    for (artifact_type, payload) in &surface_artifacts {
        if payload.is_null() {
            continue;
        }

        let item_count = payload
            .as_array()
            .map(|items| items.len() as i32)
            .unwrap_or(1);
        observation_count += item_count.max(1);

        let observation = SurfaceObservationRow {
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
        };

        db_service
            .create_surface_observation(&observation)
            .await
            .map_err(|e| e.to_string())?;
    }

    let materialized = materialize_surface_artifacts(
        db_service,
        program_id,
        Some(&run_id),
        plugin_id,
        &surface_artifacts,
    )
    .await?;

    db_service
        .update_surface_discovery_run(
            &run_id,
            "completed",
            Some(observation_count),
            Some(materialized as i32),
            Some(0),
            None,
            Some(&Utc::now().to_rfc3339()),
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(materialized)
}

pub(crate) async fn collect_monitor_targets(
    db_service: &Arc<DatabaseService>,
    program_id: &str,
) -> Result<Vec<String>, String> {
    let scopes = db_service
        .list_program_scopes(Some(program_id), None)
        .await
        .map_err(|e| e.to_string())?;

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
            Some(10_000),
            Some(0),
        )
        .await
    {
        for asset in assets {
            if asset.asset_type == "domain" && asset.is_wildcard != Some(true) {
                let target = asset.canonical_url.trim().to_string();
                if !target.is_empty() && seen.insert(target.clone()) {
                    targets.push(target);
                }
            }
        }
    }

    if let Ok(surface_assets) = db_service
        .list_surface_assets(&SurfaceAssetFilter {
            program_id: Some(program_id.to_string()),
            asset_type: None,
            status: None,
            search: None,
            limit: Some(10_000),
            offset: Some(0),
        })
        .await
    {
        for asset in surface_assets {
            if !matches!(asset.asset_type.as_str(), "domain" | "web" | "host" | "ip") {
                continue;
            }
            let target = asset.asset_name.trim().to_string();
            if !target.is_empty() && seen.insert(target.clone()) {
                targets.push(target);
            }
        }
    }

    Ok(targets)
}

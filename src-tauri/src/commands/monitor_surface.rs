use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::Utc;
use sentinel_db::{
    DatabaseService, SurfaceAssetRow, SurfaceChangeLogRow, SurfaceEvidenceRow,
    SurfaceFingerprintRow, SurfaceRelationRow,
};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SurfaceMaterializationStats {
    pub created_assets: usize,
    pub enriched_assets: usize,
    pub changed_assets: usize,
    pub skipped_missing_assets: usize,
}

#[derive(Debug, Default)]
struct SurfaceEnrichmentOutcome {
    existing_asset_ids: Vec<String>,
    skipped_missing_assets: usize,
}

impl SurfaceEnrichmentOutcome {
    fn record_existing_asset(&mut self, asset_id: String) {
        if !self.existing_asset_ids.contains(&asset_id) {
            self.existing_asset_ids.push(asset_id);
        }
    }
}

async fn upsert_surface_shell_asset(
    db_service: &Arc<DatabaseService>,
    program_id: &str,
    discovery_task_id: Option<&str>,
    asset_type: &str,
    asset_name: &str,
    display_name: Option<String>,
    source: &str,
    metadata: Option<serde_json::Value>,
) -> Result<String, String> {
    let now = Utc::now().to_rfc3339();
    let asset = SurfaceAssetRow {
        id: Uuid::new_v4().to_string(),
        program_id: program_id.to_string(),
        asset_type: asset_type.to_string(),
        asset_name: asset_name.to_string(),
        display_name,
        description: None,
        org_id: None,
        business_unit: None,
        project: None,
        owner: None,
        maintainer: None,
        contact: None,
        env: None,
        internet_exposure: Some("internet".to_string()),
        criticality: None,
        data_level: None,
        source: Some(source.to_string()),
        first_seen_at: now.clone(),
        last_seen_at: now.clone(),
        last_verified_at: Some(now.clone()),
        discovery_task_id: discovery_task_id.map(str::to_string),
        status: "active".to_string(),
        alive_status: Some("alive".to_string()),
        confidence_score: metadata
            .as_ref()
            .and_then(|value| value.get("confidence"))
            .and_then(|value| value.as_f64())
            .or(Some(0.9)),
        fingerprint_confidence: None,
        risk_score: metadata
            .as_ref()
            .and_then(|value| value.get("risk_score"))
            .and_then(|value| value.as_f64()),
        risk_level: metadata
            .as_ref()
            .and_then(|value| value.get("risk_level"))
            .and_then(|value| value.as_str())
            .map(str::to_string),
        vulnerabilities_count: Some(0),
        weak_password_flag: Some(false),
        expired_cert_flag: Some(false),
        exposed_to_internet_flag: Some(true),
        viewed_at: None,
        viewed_by: None,
        is_favorite: false,
        metadata_json: metadata.map(|value| value.to_string()),
        created_at: now.clone(),
        updated_at: now,
        created_by: Some("monitor_pipeline".to_string()),
        updated_by: Some("monitor_pipeline".to_string()),
    };

    db_service
        .upsert_surface_asset(&asset)
        .await
        .map(|row| row.id)
        .map_err(|e| e.to_string())
}

async fn upsert_artifact_asset(
    db_service: &Arc<DatabaseService>,
    ids: &mut HashMap<(String, String), String>,
    program_id: &str,
    discovery_task_id: Option<&str>,
    plugin_id: &str,
    asset_type: &str,
    asset_name: &str,
    display_name: Option<String>,
    artifact: serde_json::Value,
) -> Result<(), String> {
    let normalized_asset_name = normalize_surface_asset_key(asset_type, asset_name);
    let id = upsert_surface_shell_asset(
        db_service,
        program_id,
        discovery_task_id,
        asset_type,
        &normalized_asset_name,
        display_name,
        plugin_id,
        Some(artifact.clone()),
    )
    .await?;

    db_service
        .upsert_surface_extension_from_artifact(asset_type, &id, &artifact)
        .await
        .map_err(|e| e.to_string())?;

    ids.insert((asset_type.to_string(), normalized_asset_name), id);
    Ok(())
}

fn is_topology_asset_type(asset_type: &str) -> bool {
    matches!(
        asset_type,
        "org" | "domain" | "ip" | "host" | "port" | "service" | "web" | "certificate"
    )
}

fn json_to_string(value: Option<&Value>) -> Option<String> {
    value.map(ToString::to_string)
}

fn fingerprint_identity(asset_type: &str, asset_key: &str) -> (String, String) {
    (
        asset_type.to_string(),
        normalize_surface_asset_key(asset_type, asset_key),
    )
}

fn normalize_surface_asset_key(asset_type: &str, asset_key: &str) -> String {
    if asset_type != "web" {
        return asset_key.trim().to_string();
    }

    normalize_web_asset_key(asset_key)
}

fn normalize_web_asset_key(asset_key: &str) -> String {
    let trimmed = asset_key.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let Ok(mut url) = url::Url::parse(trimmed) else {
        return trimmed.trim_end_matches('/').to_string();
    };

    if url.path() == "/" {
        url.set_path("");
    }
    if url.query().is_none() && url.fragment().is_none() {
        return url.to_string().trim_end_matches('/').to_string();
    }

    url.to_string()
}

fn favicon_hash_from_evidence(object: &serde_json::Map<String, Value>) -> Option<&str> {
    let evidence_type = object
        .get("evidence_type")
        .and_then(|value| value.as_str())?;
    if evidence_type != "favicon_metadata" {
        return None;
    }

    object
        .get("content_json")
        .and_then(|value| value.get("icon_hash").or_else(|| value.get("favicon_hash")))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
}

fn collect_materialized_surface_asset_ids(
    ids: &HashMap<(String, String), String>,
    enriched_asset_ids: &[String],
) -> Vec<String> {
    let mut affected_asset_ids: HashSet<String> = ids.values().cloned().collect();
    affected_asset_ids.extend(enriched_asset_ids.iter().cloned());

    let mut affected_asset_ids: Vec<String> = affected_asset_ids.into_iter().collect();
    affected_asset_ids.sort();
    affected_asset_ids
}

fn required_string_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
    plugin_id: &str,
    artifact_type: &str,
    index: usize,
) -> Result<String, String> {
    object
        .get(field)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            format!("{plugin_id} {artifact_type}[{index}] missing required string field `{field}`")
        })
}

fn required_i64_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
    plugin_id: &str,
    artifact_type: &str,
    index: usize,
) -> Result<i64, String> {
    object
        .get(field)
        .and_then(|value| value.as_i64())
        .ok_or_else(|| {
            format!("{plugin_id} {artifact_type}[{index}] missing required integer field `{field}`")
        })
}

fn validate_surface_web_artifact(web: &Value, plugin_id: &str, index: usize) -> Result<(), String> {
    let object = web
        .as_object()
        .ok_or_else(|| format!("{plugin_id} webs[{index}] must be an object"))?;

    required_string_field(object, "canonical_url", plugin_id, "webs", index)?;
    required_string_field(object, "scheme", plugin_id, "webs", index)?;
    required_i64_field(object, "http_status_code", plugin_id, "webs", index)?;
    required_string_field(object, "content_summary", plugin_id, "webs", index)?;

    if !matches!(object.get("response_headers"), Some(Value::Object(_))) {
        return Err(format!(
            "{plugin_id} webs[{index}] missing required object field `response_headers`"
        ));
    }

    Ok(())
}

fn validate_surface_fingerprint_artifact(
    fingerprint: &Value,
    plugin_id: &str,
    index: usize,
) -> Result<(), String> {
    let object = fingerprint
        .as_object()
        .ok_or_else(|| format!("{plugin_id} fingerprints[{index}] must be an object"))?;

    for field in [
        "asset_type",
        "asset_key",
        "fingerprint_type",
        "fingerprint_value",
        "rule_id",
        "rule_word",
        "rule_name",
        "normalized_product",
        "normalized_category",
    ] {
        required_string_field(object, field, plugin_id, "fingerprints", index)?;
    }

    Ok(())
}

fn validate_surface_evidence_artifact(
    evidence: &Value,
    plugin_id: &str,
    index: usize,
) -> Result<(), String> {
    let object = evidence
        .as_object()
        .ok_or_else(|| format!("{plugin_id} evidences[{index}] must be an object"))?;

    for field in ["asset_type", "asset_key", "evidence_type", "title"] {
        required_string_field(object, field, plugin_id, "evidences", index)?;
    }

    let has_content = object
        .get("content_text")
        .and_then(|value| value.as_str())
        .is_some()
        || object
            .get("content_path")
            .and_then(|value| value.as_str())
            .is_some()
        || object.get("content_json").is_some();

    if !has_content {
        return Err(format!(
            "{plugin_id} evidences[{index}] requires one of `content_text`, `content_path`, or `content_json`"
        ));
    }

    Ok(())
}

async fn resolve_surface_asset_id(
    db_service: &Arc<DatabaseService>,
    ids: &HashMap<(String, String), String>,
    program_id: &str,
    asset_type: Option<&str>,
    asset_key: &str,
) -> Result<Option<String>, String> {
    if let Some(asset_type) = asset_type {
        let normalized_asset_key = normalize_surface_asset_key(asset_type, asset_key);
        if let Some(id) = ids.get(&(asset_type.to_string(), normalized_asset_key.clone())) {
            return Ok(Some(id.clone()));
        }

        return db_service
            .get_surface_asset_by_identity(program_id, asset_type, &normalized_asset_key)
            .await
            .map(|asset| asset.map(|asset| asset.id))
            .map_err(|e| e.to_string());
    }

    for candidate_type in [
        "web",
        "domain",
        "service",
        "port",
        "host",
        "ip",
        "certificate",
    ] {
        let normalized_asset_key = normalize_surface_asset_key(candidate_type, asset_key);
        if let Some(id) = ids.get(&(candidate_type.to_string(), normalized_asset_key.clone())) {
            return Ok(Some(id.clone()));
        }

        if let Some(asset) = db_service
            .get_surface_asset_by_identity(program_id, candidate_type, &normalized_asset_key)
            .await
            .map_err(|e| e.to_string())?
        {
            return Ok(Some(asset.id));
        }
    }

    Ok(None)
}

async fn materialize_surface_fingerprints(
    db_service: &Arc<DatabaseService>,
    ids: &HashMap<(String, String), String>,
    program_id: &str,
    plugin_id: &str,
    artifacts: &serde_json::Map<String, Value>,
) -> Result<SurfaceEnrichmentOutcome, String> {
    let mut outcome = SurfaceEnrichmentOutcome::default();
    let Some(fingerprints) = artifacts
        .get("fingerprints")
        .and_then(|value| value.as_array())
    else {
        return Ok(outcome);
    };

    let mut affected_asset_ids = Vec::new();
    for (index, fingerprint) in fingerprints.iter().enumerate() {
        validate_surface_fingerprint_artifact(fingerprint, plugin_id, index)?;

        let object = fingerprint
            .as_object()
            .ok_or_else(|| format!("{plugin_id} fingerprints[{index}] must be an object"))?;
        let asset_type =
            required_string_field(object, "asset_type", plugin_id, "fingerprints", index)?;
        let asset_key =
            required_string_field(object, "asset_key", plugin_id, "fingerprints", index)?;
        let created_in_current_run =
            ids.contains_key(&fingerprint_identity(&asset_type, &asset_key));
        let fingerprint_value = required_string_field(
            object,
            "fingerprint_value",
            plugin_id,
            "fingerprints",
            index,
        )?;

        let Some(asset_id) =
            resolve_surface_asset_id(db_service, ids, program_id, Some(&asset_type), &asset_key)
                .await?
        else {
            outcome.skipped_missing_assets += 1;
            continue;
        };
        affected_asset_ids.push(asset_id.clone());

        let observed_at = Utc::now().to_rfc3339();
        let fingerprint_type =
            required_string_field(object, "fingerprint_type", plugin_id, "fingerprints", index)?;
        let fingerprint_row = SurfaceFingerprintRow {
            id: Uuid::new_v4().to_string(),
            program_id: program_id.to_string(),
            asset_id: asset_id.clone(),
            fingerprint_type: fingerprint_type.clone(),
            fingerprint_key: object
                .get("fingerprint_key")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            fingerprint_value: fingerprint_value.clone(),
            rule_id: Some(required_string_field(
                object,
                "rule_id",
                plugin_id,
                "fingerprints",
                index,
            )?),
            rule_word: Some(required_string_field(
                object,
                "rule_word",
                plugin_id,
                "fingerprints",
                index,
            )?),
            rule_name: Some(required_string_field(
                object,
                "rule_name",
                plugin_id,
                "fingerprints",
                index,
            )?),
            normalized_product: Some(required_string_field(
                object,
                "normalized_product",
                plugin_id,
                "fingerprints",
                index,
            )?),
            normalized_vendor: object
                .get("normalized_vendor")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            normalized_category: Some(required_string_field(
                object,
                "normalized_category",
                plugin_id,
                "fingerprints",
                index,
            )?),
            normalized_family: object
                .get("normalized_family")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            version: object
                .get("version")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            is_primary: object.get("is_primary").and_then(|value| value.as_bool()),
            match_source_part: object
                .get("match_source_part")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            confidence_score: object.get("confidence").and_then(|value| value.as_f64()),
            source: Some(plugin_id.to_string()),
            observed_at: observed_at.clone(),
            metadata_json: Some(fingerprint.to_string()),
        };

        if asset_type == "web" && fingerprint_type == "favicon" {
            db_service
                .replace_surface_favicon_fingerprint(&fingerprint_row)
                .await
                .map_err(|e| e.to_string())?;
            db_service
                .update_surface_web_favicon_hash(&asset_id, &fingerprint_value, &observed_at)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            db_service
                .create_surface_fingerprint(&fingerprint_row)
                .await
                .map_err(|e| e.to_string())?;
        }

        if !created_in_current_run {
            outcome.record_existing_asset(asset_id.clone());
        }

        if let Some(evidence_text) = object.get("evidence").and_then(|value| value.as_str()) {
            let evidence_row = SurfaceEvidenceRow {
                id: Uuid::new_v4().to_string(),
                program_id: program_id.to_string(),
                asset_id: Some(asset_id),
                evidence_type: "fingerprint_evidence".to_string(),
                title: Some(format!(
                    "Fingerprint Evidence: {}",
                    fingerprint_row.fingerprint_value
                )),
                content_text: Some(evidence_text.to_string()),
                content_path: None,
                content_json: Some(fingerprint.to_string()),
                collected_at: observed_at,
                collected_by: Some(plugin_id.to_string()),
                probe_node: None,
                metadata_json: Some(
                    serde_json::json!({
                        "source": plugin_id,
                        "fingerprint_type": fingerprint_row.fingerprint_type,
                    })
                    .to_string(),
                ),
            };

            if asset_type == "web" && fingerprint_type == "favicon" {
                db_service
                    .replace_surface_favicon_fingerprint_evidence(&evidence_row)
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                db_service
                    .create_surface_evidence(&evidence_row)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    if !affected_asset_ids.is_empty() {
        affected_asset_ids.sort();
        affected_asset_ids.dedup();
        db_service
            .refresh_surface_asset_classifications(program_id, &affected_asset_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(outcome)
}

async fn materialize_surface_evidence(
    db_service: &Arc<DatabaseService>,
    ids: &HashMap<(String, String), String>,
    program_id: &str,
    plugin_id: &str,
    artifacts: &serde_json::Map<String, Value>,
) -> Result<SurfaceEnrichmentOutcome, String> {
    let mut outcome = SurfaceEnrichmentOutcome::default();
    let Some(evidences) = artifacts
        .get("evidences")
        .and_then(|value| value.as_array())
    else {
        return Ok(outcome);
    };

    for (index, evidence) in evidences.iter().enumerate() {
        validate_surface_evidence_artifact(evidence, plugin_id, index)?;

        let object = evidence
            .as_object()
            .ok_or_else(|| format!("{plugin_id} evidences[{index}] must be an object"))?;
        let asset_type =
            required_string_field(object, "asset_type", plugin_id, "evidences", index)?;
        let asset_key = required_string_field(object, "asset_key", plugin_id, "evidences", index)?;
        let created_in_current_run =
            ids.contains_key(&fingerprint_identity(&asset_type, &asset_key));
        let asset_id =
            resolve_surface_asset_id(db_service, ids, program_id, Some(&asset_type), &asset_key)
                .await?;
        let collected_at = Utc::now().to_rfc3339();
        if asset_id.is_none() {
            outcome.skipped_missing_assets += 1;
        }

        let row = SurfaceEvidenceRow {
            id: Uuid::new_v4().to_string(),
            program_id: program_id.to_string(),
            asset_id: asset_id.clone(),
            evidence_type: required_string_field(
                object,
                "evidence_type",
                plugin_id,
                "evidences",
                index,
            )?,
            title: Some(required_string_field(
                object,
                "title",
                plugin_id,
                "evidences",
                index,
            )?),
            content_text: object
                .get("content_text")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            content_path: object
                .get("content_path")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            content_json: json_to_string(object.get("content_json")),
            collected_at: collected_at.clone(),
            collected_by: Some(plugin_id.to_string()),
            probe_node: object
                .get("probe_node")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            metadata_json: Some(evidence.to_string()),
        };

        if let Some(asset_id) = asset_id {
            if asset_type == "web" {
                if let Some(favicon_hash) = favicon_hash_from_evidence(object) {
                    db_service
                        .replace_surface_favicon_metadata_evidence(&row)
                        .await
                        .map_err(|e| e.to_string())?;
                    db_service
                        .update_surface_web_favicon_hash(&asset_id, favicon_hash, &collected_at)
                        .await
                        .map_err(|e| e.to_string())?;
                } else {
                    db_service
                        .create_surface_evidence(&row)
                        .await
                        .map_err(|e| e.to_string())?;
                }
            } else {
                db_service
                    .create_surface_evidence(&row)
                    .await
                    .map_err(|e| e.to_string())?;
            }

            if !created_in_current_run {
                outcome.record_existing_asset(asset_id);
            }
        } else {
            db_service
                .create_surface_evidence(&row)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(outcome)
}

async fn materialize_surface_changes(
    db_service: &Arc<DatabaseService>,
    ids: &HashMap<(String, String), String>,
    program_id: &str,
    run_id: Option<&str>,
    plugin_id: &str,
    artifacts: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    let Some(changes) = artifacts.get("changes").and_then(|value| value.as_array()) else {
        return Ok(());
    };

    for change in changes {
        let asset_type = change.get("asset_type").and_then(|value| value.as_str());
        let asset_key = change.get("asset_key").and_then(|value| value.as_str());
        let asset_id = match asset_key {
            Some(asset_key) => {
                resolve_surface_asset_id(db_service, ids, program_id, asset_type, asset_key).await?
            }
            None => None,
        };

        let title = change
            .get("title")
            .and_then(|value| value.as_str())
            .unwrap_or("Surface Change")
            .to_string();
        let description = change
            .get("description")
            .and_then(|value| value.as_str())
            .map(str::to_string);

        let row = SurfaceChangeLogRow {
            id: Uuid::new_v4().to_string(),
            program_id: program_id.to_string(),
            asset_id,
            relation_id: None,
            change_type: change
                .get("change_type")
                .and_then(|value| value.as_str())
                .unwrap_or("change_detected")
                .to_string(),
            old_value_json: json_to_string(change.get("old_value")),
            new_value_json: json_to_string(change.get("new_value")),
            summary: description
                .map(|description| format!("{title}: {description}"))
                .unwrap_or(title),
            detected_at: Utc::now().to_rfc3339(),
            source_run_id: run_id.map(str::to_string),
            risk_delta: change.get("risk_score").and_then(|value| value.as_f64()),
            metadata_json: Some(
                serde_json::json!({
                    "source": plugin_id,
                    "payload": change,
                })
                .to_string(),
            ),
        };

        db_service
            .create_surface_change_log(&row)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub(crate) async fn materialize_surface_artifacts(
    db_service: &Arc<DatabaseService>,
    program_id: &str,
    run_id: Option<&str>,
    plugin_id: &str,
    artifacts: &serde_json::Map<String, serde_json::Value>,
) -> Result<SurfaceMaterializationStats, String> {
    let mut ids: HashMap<(String, String), String> = HashMap::new();
    let mut stats = SurfaceMaterializationStats::default();

    if let Some(domains) = artifacts.get("domains").and_then(|value| value.as_array()) {
        for domain in domains {
            if let Some(fqdn) = domain.get("fqdn").and_then(|value| value.as_str()) {
                upsert_artifact_asset(
                    db_service,
                    &mut ids,
                    program_id,
                    run_id,
                    plugin_id,
                    "domain",
                    fqdn,
                    Some(fqdn.to_string()),
                    domain.clone(),
                )
                .await?;
                stats.created_assets += 1;
            }
        }
    }

    if let Some(ips) = artifacts.get("ips").and_then(|value| value.as_array()) {
        for ip in ips {
            let asset_name = ip
                .get("ip_address")
                .or_else(|| ip.get("ip"))
                .and_then(|value| value.as_str());
            if let Some(asset_name) = asset_name {
                upsert_artifact_asset(
                    db_service,
                    &mut ids,
                    program_id,
                    run_id,
                    plugin_id,
                    "ip",
                    asset_name,
                    Some(asset_name.to_string()),
                    ip.clone(),
                )
                .await?;
                stats.created_assets += 1;
            }
        }
    }

    if let Some(hosts) = artifacts.get("hosts").and_then(|value| value.as_array()) {
        for host in hosts {
            let asset_name = host
                .get("hostname")
                .or_else(|| host.get("fqdn"))
                .and_then(|value| value.as_str());
            if let Some(asset_name) = asset_name {
                upsert_artifact_asset(
                    db_service,
                    &mut ids,
                    program_id,
                    run_id,
                    plugin_id,
                    "host",
                    asset_name,
                    Some(asset_name.to_string()),
                    host.clone(),
                )
                .await?;
                stats.created_assets += 1;
            }
        }
    }

    if let Some(ports) = artifacts.get("ports").and_then(|value| value.as_array()) {
        for port in ports {
            let host_key = port
                .get("host_key")
                .or_else(|| port.get("ip_or_host"))
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let port_number = port
                .get("port")
                .and_then(|value| value.as_i64())
                .unwrap_or_default();
            let transport_protocol = port
                .get("transport_protocol")
                .and_then(|value| value.as_str())
                .unwrap_or("tcp");
            let asset_name = format!("{host_key}:{port_number}/{transport_protocol}");

            upsert_artifact_asset(
                db_service,
                &mut ids,
                program_id,
                run_id,
                plugin_id,
                "port",
                &asset_name,
                Some(asset_name.clone()),
                port.clone(),
            )
            .await?;
            stats.created_assets += 1;
        }
    }

    if let Some(services) = artifacts.get("services").and_then(|value| value.as_array()) {
        for service in services {
            let host_key = service
                .get("host_key")
                .or_else(|| service.get("ip_or_host"))
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let port_number = service
                .get("port")
                .and_then(|value| value.as_i64())
                .unwrap_or_default();
            let transport_protocol = service
                .get("transport_protocol")
                .and_then(|value| value.as_str())
                .unwrap_or("tcp");
            let asset_name = format!("{host_key}:{port_number}/{transport_protocol}");
            let display_name = service
                .get("application_service_name")
                .or_else(|| service.get("protocol_name"))
                .and_then(|value| value.as_str())
                .map(str::to_string);

            upsert_artifact_asset(
                db_service,
                &mut ids,
                program_id,
                run_id,
                plugin_id,
                "service",
                &asset_name,
                display_name,
                service.clone(),
            )
            .await?;
            stats.created_assets += 1;
        }
    }

    if let Some(webs) = artifacts.get("webs").and_then(|value| value.as_array()) {
        for (index, web) in webs.iter().enumerate() {
            validate_surface_web_artifact(web, plugin_id, index)?;

            let object = web
                .as_object()
                .ok_or_else(|| format!("{plugin_id} webs[{index}] must be an object"))?;
            let canonical_url =
                required_string_field(object, "canonical_url", plugin_id, "webs", index)?;
            let display_name = object
                .get("site_title")
                .and_then(|value| value.as_str())
                .map(str::to_string)
                .or_else(|| Some(canonical_url.clone()));

            upsert_artifact_asset(
                db_service,
                &mut ids,
                program_id,
                run_id,
                plugin_id,
                "web",
                &canonical_url,
                display_name,
                web.clone(),
            )
            .await?;
            stats.created_assets += 1;
        }
    }

    if let Some(certs) = artifacts
        .get("certificates")
        .and_then(|value| value.as_array())
    {
        for certificate in certs {
            if let Some(sha256) = certificate.get("sha256").and_then(|value| value.as_str()) {
                upsert_artifact_asset(
                    db_service,
                    &mut ids,
                    program_id,
                    run_id,
                    plugin_id,
                    "certificate",
                    sha256,
                    Some(sha256.to_string()),
                    certificate.clone(),
                )
                .await?;
                stats.created_assets += 1;
            }
        }
    }

    if let Some(relations) = artifacts
        .get("relations")
        .and_then(|value| value.as_array())
    {
        for relation in relations {
            let from_type = relation
                .get("from_type")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let from_key = relation
                .get("from_key")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let to_type = relation
                .get("to_type")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let to_key = relation
                .get("to_key")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let relation_type = relation
                .get("relation_type")
                .and_then(|value| value.as_str())
                .unwrap_or("related_to");

            if !is_topology_asset_type(from_type) || !is_topology_asset_type(to_type) {
                continue;
            }

            if !ids.contains_key(&(from_type.to_string(), from_key.to_string())) {
                let id = upsert_surface_shell_asset(
                    db_service,
                    program_id,
                    run_id,
                    from_type,
                    from_key,
                    Some(from_key.to_string()),
                    plugin_id,
                    None,
                )
                .await?;
                ids.insert((from_type.to_string(), from_key.to_string()), id);
            }

            if !ids.contains_key(&(to_type.to_string(), to_key.to_string())) {
                let id = upsert_surface_shell_asset(
                    db_service,
                    program_id,
                    run_id,
                    to_type,
                    to_key,
                    Some(to_key.to_string()),
                    plugin_id,
                    None,
                )
                .await?;
                ids.insert((to_type.to_string(), to_key.to_string()), id);
            }

            let now = Utc::now().to_rfc3339();
            let relation_row = SurfaceRelationRow {
                id: Uuid::new_v4().to_string(),
                program_id: program_id.to_string(),
                from_asset_id: ids[&(from_type.to_string(), from_key.to_string())].clone(),
                to_asset_id: ids[&(to_type.to_string(), to_key.to_string())].clone(),
                relation_type: relation_type.to_string(),
                source: Some(plugin_id.to_string()),
                confidence_score: relation.get("confidence").and_then(|value| value.as_f64()),
                evidence_id: None,
                first_seen_at: now.clone(),
                last_seen_at: now.clone(),
                active: true,
                metadata_json: Some(relation.to_string()),
                created_at: now.clone(),
                updated_at: now,
            };

            db_service
                .create_surface_relation_if_missing(&relation_row)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    let mut enriched_asset_ids = Vec::new();
    let fingerprint_outcome =
        materialize_surface_fingerprints(db_service, &ids, program_id, plugin_id, artifacts)
            .await?;
    stats.skipped_missing_assets += fingerprint_outcome.skipped_missing_assets;
    enriched_asset_ids.extend(fingerprint_outcome.existing_asset_ids);

    let evidence_outcome =
        materialize_surface_evidence(db_service, &ids, program_id, plugin_id, artifacts).await?;
    stats.skipped_missing_assets += evidence_outcome.skipped_missing_assets;
    enriched_asset_ids.extend(evidence_outcome.existing_asset_ids);

    materialize_surface_changes(db_service, &ids, program_id, run_id, plugin_id, artifacts).await?;

    enriched_asset_ids.sort();
    enriched_asset_ids.dedup();
    stats.enriched_assets = enriched_asset_ids.len();
    stats.changed_assets = enriched_asset_ids.len();

    let affected_asset_ids = collect_materialized_surface_asset_ids(&ids, &enriched_asset_ids);
    db_service
        .mark_surface_assets_new(&affected_asset_ids)
        .await
        .map_err(|e| e.to_string())?;

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::{
        collect_materialized_surface_asset_ids, favicon_hash_from_evidence, fingerprint_identity,
    };
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn web_asset_identity_ignores_root_trailing_slash() {
        assert_eq!(
            fingerprint_identity("web", "https://www.example.com/"),
            ("web".to_string(), "https://www.example.com".to_string())
        );
        assert_eq!(
            fingerprint_identity("web", "https://www.example.com/path/"),
            (
                "web".to_string(),
                "https://www.example.com/path".to_string()
            )
        );
    }

    #[test]
    fn favicon_evidence_requires_icon_hash_semantics() {
        let evidence = json!({
            "evidence_type": "favicon_metadata",
            "content_json": {
                "icon_hash": "-123456",
                "sha256": "not-the-persisted-favicon-hash"
            }
        });
        let object = evidence.as_object().unwrap();

        assert_eq!(favicon_hash_from_evidence(object), Some("-123456"));
    }

    #[test]
    fn materialized_asset_ids_include_created_and_enriched_once() {
        let ids = HashMap::from([
            (
                ("web".to_string(), "https://a.example".to_string()),
                "asset-a".to_string(),
            ),
            (
                ("web".to_string(), "https://b.example".to_string()),
                "asset-b".to_string(),
            ),
        ]);
        let enriched_asset_ids = vec!["asset-b".to_string(), "asset-c".to_string()];

        assert_eq!(
            collect_materialized_surface_asset_ids(&ids, &enriched_asset_ids),
            vec![
                "asset-a".to_string(),
                "asset-b".to_string(),
                "asset-c".to_string()
            ]
        );
    }
}

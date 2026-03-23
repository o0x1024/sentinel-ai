use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use sentinel_db::{
    DatabaseService, SurfaceAssetRow, SurfaceChangeLogRow, SurfaceEvidenceRow,
    SurfaceFingerprintRow, SurfaceRelationRow,
};
use serde_json::Value;
use uuid::Uuid;

async fn upsert_surface_shell_asset(
    db_service: &Arc<DatabaseService>,
    program_id: &str,
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
        discovery_task_id: None,
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
    plugin_id: &str,
    asset_type: &str,
    asset_name: &str,
    display_name: Option<String>,
    artifact: serde_json::Value,
) -> Result<(), String> {
    let id = upsert_surface_shell_asset(
        db_service,
        program_id,
        asset_type,
        asset_name,
        display_name,
        plugin_id,
        Some(artifact.clone()),
    )
    .await?;

    db_service
        .upsert_surface_extension_from_artifact(asset_type, &id, &artifact)
        .await
        .map_err(|e| e.to_string())?;

    ids.insert((asset_type.to_string(), asset_name.to_string()), id);
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

async fn resolve_surface_asset_id(
    db_service: &Arc<DatabaseService>,
    ids: &HashMap<(String, String), String>,
    program_id: &str,
    asset_type: Option<&str>,
    asset_key: &str,
) -> Result<Option<String>, String> {
    if let Some(asset_type) = asset_type {
        if let Some(id) = ids.get(&(asset_type.to_string(), asset_key.to_string())) {
            return Ok(Some(id.clone()));
        }

        return db_service
            .get_surface_asset_by_identity(program_id, asset_type, asset_key)
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
        if let Some(id) = ids.get(&(candidate_type.to_string(), asset_key.to_string())) {
            return Ok(Some(id.clone()));
        }

        if let Some(asset) = db_service
            .get_surface_asset_by_identity(program_id, candidate_type, asset_key)
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
) -> Result<(), String> {
    let Some(fingerprints) = artifacts
        .get("fingerprints")
        .and_then(|value| value.as_array())
    else {
        return Ok(());
    };

    for fingerprint in fingerprints {
        let asset_type = fingerprint
            .get("asset_type")
            .and_then(|value| value.as_str())
            .or_else(|| {
                if fingerprint.get("web_key").is_some() {
                    Some("web")
                } else {
                    None
                }
            });
        let asset_key = fingerprint
            .get("asset_key")
            .and_then(|value| value.as_str())
            .or_else(|| fingerprint.get("web_key").and_then(|value| value.as_str()));
        let fingerprint_value = fingerprint
            .get("fingerprint_value")
            .and_then(|value| value.as_str());

        let (Some(asset_key), Some(fingerprint_value)) = (asset_key, fingerprint_value) else {
            continue;
        };

        let Some(asset_id) =
            resolve_surface_asset_id(db_service, ids, program_id, asset_type, asset_key).await?
        else {
            continue;
        };

        let observed_at = Utc::now().to_rfc3339();
        let fingerprint_row = SurfaceFingerprintRow {
            id: Uuid::new_v4().to_string(),
            program_id: program_id.to_string(),
            asset_id: asset_id.clone(),
            fingerprint_type: fingerprint
                .get("fingerprint_type")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown")
                .to_string(),
            fingerprint_key: fingerprint
                .get("fingerprint_key")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            fingerprint_value: fingerprint_value.to_string(),
            confidence_score: fingerprint
                .get("confidence")
                .and_then(|value| value.as_f64()),
            source: Some(plugin_id.to_string()),
            observed_at: observed_at.clone(),
            metadata_json: Some(fingerprint.to_string()),
        };

        db_service
            .create_surface_fingerprint(&fingerprint_row)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(evidence_text) = fingerprint.get("evidence").and_then(|value| value.as_str()) {
            let evidence_row = SurfaceEvidenceRow {
                id: Uuid::new_v4().to_string(),
                program_id: program_id.to_string(),
                asset_id: Some(asset_id),
                evidence_type: "fingerprint_evidence".to_string(),
                title: fingerprint
                    .get("fingerprint_value")
                    .and_then(|value| value.as_str())
                    .map(|value| format!("Fingerprint Evidence: {value}")),
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

            db_service
                .create_surface_evidence(&evidence_row)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

async fn materialize_surface_evidence(
    db_service: &Arc<DatabaseService>,
    ids: &HashMap<(String, String), String>,
    program_id: &str,
    plugin_id: &str,
    artifacts: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    let Some(evidences) = artifacts
        .get("evidences")
        .and_then(|value| value.as_array())
    else {
        return Ok(());
    };

    for evidence in evidences {
        let asset_type = evidence.get("asset_type").and_then(|value| value.as_str());
        let asset_key = evidence.get("asset_key").and_then(|value| value.as_str());
        let asset_id = match asset_key {
            Some(asset_key) => {
                resolve_surface_asset_id(db_service, ids, program_id, asset_type, asset_key).await?
            }
            None => None,
        };
        let collected_at = Utc::now().to_rfc3339();

        let row = SurfaceEvidenceRow {
            id: Uuid::new_v4().to_string(),
            program_id: program_id.to_string(),
            asset_id,
            evidence_type: evidence
                .get("evidence_type")
                .and_then(|value| value.as_str())
                .unwrap_or("probe_artifact")
                .to_string(),
            title: evidence
                .get("title")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            content_text: evidence
                .get("content_text")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            content_path: evidence
                .get("content_path")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            content_json: json_to_string(evidence.get("content_json")),
            collected_at,
            collected_by: Some(plugin_id.to_string()),
            probe_node: evidence
                .get("probe_node")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            metadata_json: Some(evidence.to_string()),
        };

        db_service
            .create_surface_evidence(&row)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

async fn materialize_surface_changes(
    db_service: &Arc<DatabaseService>,
    ids: &HashMap<(String, String), String>,
    program_id: &str,
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
            source_run_id: None,
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
    plugin_id: &str,
    artifacts: &serde_json::Map<String, serde_json::Value>,
) -> Result<usize, String> {
    let mut ids: HashMap<(String, String), String> = HashMap::new();
    let mut materialized = 0usize;

    if let Some(domains) = artifacts.get("domains").and_then(|value| value.as_array()) {
        for domain in domains {
            if let Some(fqdn) = domain.get("fqdn").and_then(|value| value.as_str()) {
                upsert_artifact_asset(
                    db_service,
                    &mut ids,
                    program_id,
                    plugin_id,
                    "domain",
                    fqdn,
                    Some(fqdn.to_string()),
                    domain.clone(),
                )
                .await?;
                materialized += 1;
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
                    plugin_id,
                    "ip",
                    asset_name,
                    Some(asset_name.to_string()),
                    ip.clone(),
                )
                .await?;
                materialized += 1;
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
                    plugin_id,
                    "host",
                    asset_name,
                    Some(asset_name.to_string()),
                    host.clone(),
                )
                .await?;
                materialized += 1;
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
                plugin_id,
                "port",
                &asset_name,
                Some(asset_name.clone()),
                port.clone(),
            )
            .await?;
            materialized += 1;
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
                plugin_id,
                "service",
                &asset_name,
                display_name,
                service.clone(),
            )
            .await?;
            materialized += 1;
        }
    }

    if let Some(webs) = artifacts.get("webs").and_then(|value| value.as_array()) {
        for web in webs {
            if let Some(canonical_url) = web
                .get("canonical_url")
                .or_else(|| web.get("url"))
                .and_then(|value| value.as_str())
            {
                let display_name = web
                    .get("site_title")
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
                    .or_else(|| Some(canonical_url.to_string()));

                upsert_artifact_asset(
                    db_service,
                    &mut ids,
                    program_id,
                    plugin_id,
                    "web",
                    canonical_url,
                    display_name,
                    web.clone(),
                )
                .await?;
                materialized += 1;
            }
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
                    plugin_id,
                    "certificate",
                    sha256,
                    Some(sha256.to_string()),
                    certificate.clone(),
                )
                .await?;
                materialized += 1;
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

    materialize_surface_fingerprints(db_service, &ids, program_id, plugin_id, artifacts).await?;
    materialize_surface_evidence(db_service, &ids, program_id, plugin_id, artifacts).await?;
    materialize_surface_changes(db_service, &ids, program_id, plugin_id, artifacts).await?;

    Ok(materialized)
}

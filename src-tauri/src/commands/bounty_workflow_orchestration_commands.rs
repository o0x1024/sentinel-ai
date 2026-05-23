//! Bug bounty workflow orchestration artifact and plugin-port commands.

use super::bounty_asset_commands::canonicalize_url;
use super::bounty_commands::ensure_bounty_feature;
use chrono::Utc;
use sentinel_db::{BountyAssetRow, BountyFindingRow, DatabaseService};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

// ============================================================================
// Workflow Orchestration Commands (P0)
// ============================================================================

/// Workflow step input resolution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveStepInputsRequest {
    pub step_id: String,
    pub step_name: String,
    pub plugin_id: Option<String>,
    pub tool_name: Option<String>,
    pub config: serde_json::Value,
    pub depends_on: Vec<String>,
    pub upstream_results: std::collections::HashMap<String, serde_json::Value>,
    pub initial_inputs: serde_json::Value,
}

/// Resolved step inputs response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveStepInputsResponse {
    pub resolved_config: serde_json::Value,
    pub resolved_from_upstream: Vec<String>,
}

/// Resolve workflow step inputs from upstream outputs
#[tauri::command]
pub async fn bounty_resolve_step_inputs(
    request: ResolveStepInputsRequest,
) -> Result<ResolveStepInputsResponse, String> {
    ensure_bounty_feature()?;

    use sentinel_bounty::services::{StepContext, WorkflowOrchestrator};

    let orchestrator = WorkflowOrchestrator::new();

    let step = StepContext {
        step_id: request.step_id,
        step_name: request.step_name,
        plugin_id: request.plugin_id,
        tool_name: request.tool_name,
        config: request.config.clone(),
        depends_on: request.depends_on,
        retry_config: None,
        target_host: None,
    };

    let resolved =
        orchestrator.resolve_step_inputs(&step, &request.upstream_results, &request.initial_inputs);

    // Identify which params were resolved from upstream
    let mut resolved_from_upstream = Vec::new();
    if let (Some(orig_obj), Some(resolved_obj)) = (request.config.as_object(), resolved.as_object())
    {
        for (key, resolved_val) in resolved_obj {
            let orig_val = orig_obj.get(key);
            let was_empty = orig_val.map(|v| is_value_empty(v)).unwrap_or(true);
            let is_now_filled = !is_value_empty(resolved_val);
            if was_empty && is_now_filled {
                resolved_from_upstream.push(key.clone());
            }
        }
    }

    Ok(ResolveStepInputsResponse {
        resolved_config: resolved,
        resolved_from_upstream,
    })
}

fn is_value_empty(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) => s.is_empty(),
        serde_json::Value::Array(arr) => arr.is_empty(),
        serde_json::Value::Object(obj) => obj.is_empty(),
        _ => false,
    }
}

/// Process step output request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStepOutputRequest {
    pub execution_id: String,
    pub step_id: String,
    pub step_name: String,
    pub plugin_id: Option<String>,
    pub raw_output: serde_json::Value,
}

/// Processed artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedArtifact {
    pub id: String,
    pub step_id: String,
    pub artifact_type: String,
    pub data: serde_json::Value,
    pub count: Option<usize>,
    pub source: Option<String>,
}

/// Process step output response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStepOutputResponse {
    pub artifacts: Vec<ProcessedArtifact>,
    pub summary: ArtifactSummaryResponse,
    pub validation_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactSummaryResponse {
    pub findings: usize,
    pub subdomains: usize,
    pub live_hosts: usize,
    pub technologies: usize,
    pub endpoints: usize,
    pub secrets: usize,
    pub directories: usize,
}

/// Process workflow step output into typed artifacts
#[tauri::command]
pub async fn bounty_process_step_output(
    request: ProcessStepOutputRequest,
) -> Result<ProcessStepOutputResponse, String> {
    ensure_bounty_feature()?;

    use sentinel_bounty::services::{ArtifactType, StepContext, WorkflowOrchestrator};

    let orchestrator = WorkflowOrchestrator::new();

    let step = StepContext {
        step_id: request.step_id,
        step_name: request.step_name,
        plugin_id: request.plugin_id.clone(),
        tool_name: None,
        config: serde_json::json!({}),
        depends_on: vec![],
        retry_config: None,
        target_host: None,
    };

    let (artifacts, validation_error) = match orchestrator.try_process_step_output(
        &step,
        &request.execution_id,
        &request.raw_output,
    ) {
        Ok(artifacts) => (artifacts, None),
        Err(error) => (Vec::new(), Some(error)),
    };

    let mut summary = ArtifactSummaryResponse::default();
    let processed: Vec<ProcessedArtifact> = artifacts
        .iter()
        .map(|a| {
            // Update summary
            match a.artifact_type {
                ArtifactType::Finding => summary.findings += a.metadata.count.unwrap_or(1),
                ArtifactType::Subdomains => summary.subdomains += a.metadata.count.unwrap_or(0),
                ArtifactType::LiveHosts => summary.live_hosts += a.metadata.count.unwrap_or(0),
                ArtifactType::Technologies => summary.technologies += a.metadata.count.unwrap_or(0),
                ArtifactType::Endpoints => summary.endpoints += a.metadata.count.unwrap_or(0),
                ArtifactType::Secrets => summary.secrets += a.metadata.count.unwrap_or(0),
                ArtifactType::Directories => summary.directories += a.metadata.count.unwrap_or(0),
                _ => {}
            }

            ProcessedArtifact {
                id: a.id.clone(),
                step_id: a.step_id.clone(),
                artifact_type: a.artifact_type.as_str().to_string(),
                data: a.data.clone(),
                count: a.metadata.count,
                source: a.metadata.source.clone(),
            }
        })
        .collect();

    Ok(ProcessStepOutputResponse {
        artifacts: processed,
        summary,
        validation_error,
    })
}

/// Sink artifacts request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkArtifactsRequest {
    pub program_id: String,
    pub scope_id: Option<String>,
    pub execution_id: String,
    pub artifacts: Vec<ProcessedArtifact>,
    pub auto_create_findings: Option<bool>,
    pub auto_update_assets: Option<bool>,
    pub deduplicate: Option<bool>,
}

/// Sink artifacts response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkArtifactsResponse {
    pub findings_created: Vec<String>,
    pub assets_created: Vec<String>,
    pub assets_updated: Vec<String>,
    pub subdomains_imported: usize,
    pub live_hosts_imported: usize,
    pub skipped_duplicates: usize,
    pub errors: Vec<String>,
}

/// Sink workflow artifacts to database
#[tauri::command]
pub async fn bounty_sink_artifacts(
    db_service: State<'_, Arc<DatabaseService>>,
    request: SinkArtifactsRequest,
) -> Result<SinkArtifactsResponse, String> {
    ensure_bounty_feature()?;

    let now = Utc::now().to_rfc3339();
    let mut response = SinkArtifactsResponse {
        findings_created: vec![],
        assets_created: vec![],
        assets_updated: vec![],
        subdomains_imported: 0,
        live_hosts_imported: 0,
        skipped_duplicates: 0,
        errors: vec![],
    };

    let auto_create_findings = request.auto_create_findings.unwrap_or(true);
    let auto_update_assets = request.auto_update_assets.unwrap_or(true);
    let deduplicate = request.deduplicate.unwrap_or(true);

    for artifact in request.artifacts {
        match artifact.artifact_type.as_str() {
            "finding" => {
                if !auto_create_findings {
                    continue;
                }

                // Extract finding data
                if let Ok(finding_data) =
                    serde_json::from_value::<FindingArtifactData>(artifact.data.clone())
                {
                    // Generate fingerprint for deduplication
                    let fingerprint = format!(
                        "{}:{}:{}:{}",
                        request.program_id,
                        finding_data.finding_type,
                        finding_data.affected_url.as_deref().unwrap_or(""),
                        artifact.step_id
                    );
                    let fingerprint = format!("{:x}", md5::compute(fingerprint.as_bytes()));

                    // Check duplicate
                    if deduplicate {
                        if db_service
                            .get_bounty_finding_by_fingerprint(&fingerprint)
                            .await
                            .map_err(|e| e.to_string())?
                            .is_some()
                        {
                            response.skipped_duplicates += 1;
                            continue;
                        }
                    }

                    let finding_id = Uuid::new_v4().to_string();
                    let finding = BountyFindingRow {
                        id: finding_id.clone(),
                        program_id: request.program_id.clone(),
                        scope_id: request.scope_id.clone(),
                        asset_id: None,
                        title: finding_data.title,
                        description: finding_data.description,
                        finding_type: finding_data.finding_type,
                        severity: finding_data
                            .severity
                            .unwrap_or_else(|| "medium".to_string()),
                        status: "new".to_string(),
                        confidence: finding_data
                            .confidence
                            .unwrap_or_else(|| "medium".to_string()),
                        cvss_score: None,
                        cwe_id: finding_data.cwe_id,
                        affected_url: finding_data.affected_url,
                        affected_parameter: finding_data.affected_parameter,
                        reproduction_steps_json: finding_data
                            .reproduction_steps
                            .map(|s| serde_json::to_string(&s).unwrap_or_default()),
                        impact: finding_data.impact,
                        remediation: finding_data.remediation,
                        evidence_ids_json: None,
                        tags_json: Some(
                            serde_json::to_string(&vec!["workflow", "automated"])
                                .unwrap_or_default(),
                        ),
                        metadata_json: Some(
                            serde_json::to_string(&serde_json::json!({
                                "source": "workflow",
                                "execution_id": request.execution_id,
                                "step_id": artifact.step_id,
                            }))
                            .unwrap_or_default(),
                        ),
                        fingerprint,
                        duplicate_of: None,
                        first_seen_at: now.clone(),
                        last_seen_at: now.clone(),
                        verified_at: None,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                        created_by: "workflow".to_string(),
                    };

                    match db_service.create_bounty_finding(&finding).await {
                        Ok(_) => response.findings_created.push(finding_id),
                        Err(e) => response
                            .errors
                            .push(format!("Failed to create finding: {}", e)),
                    }
                }
            }
            "subdomains" => {
                if !auto_update_assets {
                    continue;
                }

                // Extract subdomains and create assets
                if let Some(subdomains) = artifact.data.get("subdomains").and_then(|v| v.as_array())
                {
                    for subdomain_entry in subdomains {
                        let subdomain = subdomain_entry
                            .get("subdomain")
                            .and_then(|v| v.as_str())
                            .unwrap_or_else(|| subdomain_entry.as_str().unwrap_or(""));

                        if subdomain.is_empty() {
                            continue;
                        }

                        // Create asset with canonical URL
                        let canonical_url = format!("https://{}", subdomain);

                        // Check if asset exists
                        let existing = db_service
                            .get_bounty_asset_by_canonical_url(&request.program_id, &canonical_url)
                            .await
                            .map_err(|e| e.to_string())?;

                        if existing.is_none() {
                            let asset = BountyAssetRow {
                                id: Uuid::new_v4().to_string(),
                                program_id: request.program_id.clone(),
                                scope_id: request.scope_id.clone(),
                                asset_type: "domain".to_string(),
                                canonical_url: canonical_url.clone(),
                                original_urls_json: None,
                                hostname: Some(subdomain.to_string()),
                                port: None,
                                path: None,
                                protocol: Some("https".to_string()),
                                ip_addresses_json: None,
                                dns_records_json: None,
                                tech_stack_json: None,
                                fingerprint: None,
                                tags_json: None,
                                labels_json: None,
                                priority_score: Some(0.0),
                                risk_score: Some(0.0),
                                is_alive: true,
                                last_checked_at: None,
                                first_seen_at: now.clone(),
                                last_seen_at: now.clone(),
                                findings_count: 0,
                                change_events_count: 0,
                                metadata_json: Some(
                                    serde_json::to_string(&serde_json::json!({
                                        "source": "workflow_subdomain_enum",
                                        "execution_id": request.execution_id,
                                    }))
                                    .unwrap_or_default(),
                                ),
                                created_at: now.clone(),
                                updated_at: now.clone(),
                                // ASM fields - all None by default
                                ip_version: None,
                                asn: None,
                                asn_org: None,
                                isp: None,
                                country: None,
                                city: None,
                                latitude: None,
                                longitude: None,
                                is_cloud: None,
                                cloud_provider: None,
                                service_name: None,
                                service_version: None,
                                service_product: None,
                                banner: None,
                                transport_protocol: None,
                                cpe: None,
                                domain_registrar: None,
                                registration_date: None,
                                expiration_date: None,
                                nameservers_json: None,
                                mx_records_json: None,
                                txt_records_json: None,
                                whois_data_json: None,
                                is_wildcard: None,
                                parent_domain: None,
                                root_domain: None,
                                subdomain_level: None,
                                http_status: None,
                                response_time_ms: None,
                                content_length: None,
                                content_type: None,
                                title: None,
                                favicon_hash: None,
                                headers_json: None,
                                waf_detected: None,
                                cdn_detected: None,
                                screenshot_path: None,
                                body_hash: None,
                                certificate_id: None,
                                ssl_enabled: None,
                                certificate_subject: None,
                                certificate_issuer: None,
                                certificate_valid_from: None,
                                certificate_valid_to: None,
                                certificate_san_json: None,
                                exposure_level: None,
                                attack_surface_score: None,
                                vulnerability_count: None,
                                cvss_max_score: None,
                                exploit_available: None,
                                asset_category: None,
                                asset_owner: None,
                                business_unit: None,
                                criticality: None,
                                discovery_method: None,
                                data_sources_json: None,
                                confidence_score: None,
                                monitoring_enabled: None,
                                scan_frequency: None,
                                last_scan_type: None,
                                parent_asset_id: None,
                                related_assets_json: None,
                            };

                            if db_service.create_bounty_asset(&asset).await.is_ok() {
                                response.assets_created.push(asset.id);
                            }
                        }
                        response.subdomains_imported += 1;
                    }
                }
            }
            "live_hosts" => {
                if !auto_update_assets {
                    continue;
                }

                // Extract live hosts and update assets
                if let Some(hosts) = artifact.data.get("hosts").and_then(|v| v.as_array()) {
                    for host in hosts {
                        let url = host.get("url").and_then(|v| v.as_str()).unwrap_or("");
                        if url.is_empty() {
                            continue;
                        }

                        let status_code = host
                            .get("status_code")
                            .or_else(|| host.get("statusCode"))
                            .and_then(|v| v.as_i64())
                            .map(|n| n as i32);
                        let title = host
                            .get("title")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let tech: Option<Vec<String>> = host
                            .get("technologies")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            });

                        // Try to find existing asset
                        let (canonical_url, _, _, _, _) = canonicalize_url(url);
                        if let Ok(Some(mut asset)) = db_service
                            .get_bounty_asset_by_canonical_url(&request.program_id, &canonical_url)
                            .await
                        {
                            // Update existing asset - store status/title in metadata
                            let mut metadata: serde_json::Map<String, serde_json::Value> = asset
                                .metadata_json
                                .as_ref()
                                .and_then(|s| serde_json::from_str(s).ok())
                                .unwrap_or_default();
                            if let Some(sc) = status_code {
                                metadata.insert("status_code".to_string(), serde_json::json!(sc));
                            }
                            if let Some(ref t) = title {
                                metadata.insert("title".to_string(), serde_json::json!(t));
                            }
                            asset.metadata_json =
                                Some(serde_json::to_string(&metadata).unwrap_or_default());
                            asset.is_alive =
                                status_code.map(|c| c >= 200 && c < 400).unwrap_or(true);
                            if let Some(t) = tech {
                                asset.tech_stack_json =
                                    Some(serde_json::to_string(&t).unwrap_or_default());
                            }
                            asset.last_seen_at = now.clone();
                            asset.updated_at = now.clone();

                            if db_service.update_bounty_asset(&asset).await.is_ok() {
                                response.assets_updated.push(asset.id);
                            }
                        }
                        response.live_hosts_imported += 1;
                    }
                }
            }
            _ => {
                // Other artifact types - log but don't process
            }
        }
    }

    Ok(response)
}

/// Finding artifact data for deserialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FindingArtifactData {
    pub title: String,
    pub description: String,
    pub finding_type: String,
    pub severity: Option<String>,
    pub confidence: Option<String>,
    pub affected_url: Option<String>,
    pub affected_parameter: Option<String>,
    pub cwe_id: Option<String>,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub reproduction_steps: Option<Vec<String>>,
}

/// Retry configuration for workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_type: String, // "fixed", "linear", "exponential"
    pub backoff_multiplier: Option<f64>,
}

/// Get default retry configuration
#[tauri::command]
pub async fn bounty_get_default_retry_config() -> Result<StepRetryConfig, String> {
    Ok(StepRetryConfig {
        max_attempts: 3,
        initial_delay_ms: 1000,
        max_delay_ms: 30000,
        backoff_type: "exponential".to_string(),
        backoff_multiplier: Some(2.0),
    })
}

/// Rate limiter stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterStats {
    pub global_available: usize,
    pub global_limit: usize,
    pub per_host_limit: usize,
    pub per_host_delay_ms: u64,
}

/// Get rate limiter statistics
#[tauri::command]
pub async fn bounty_get_rate_limiter_stats() -> Result<RateLimiterStats, String> {
    use sentinel_bounty::services::WorkflowOrchestrator;

    let orchestrator = WorkflowOrchestrator::new();
    let stats = orchestrator.rate_limiter().stats();

    Ok(RateLimiterStats {
        global_available: stats.global_available,
        global_limit: stats.global_limit,
        per_host_limit: stats.per_host_limit,
        per_host_delay_ms: stats.per_host_delay_ms,
    })
}

/// Plugin port info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPortInfo {
    pub plugin_id: String,
    pub output_ports: Vec<PortDef>,
    pub input_params: Vec<InputParamDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortDef {
    pub name: String,
    pub artifact_type: String,
    pub fields: Vec<ArtifactFieldDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactFieldDef {
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputParamDef {
    pub name: String,
    pub expected_artifact_type: String,
    pub extract_path: Option<String>,
    pub required: bool,
}

/// Get plugin port definitions for data flow
#[tauri::command]
pub async fn bounty_get_plugin_ports(plugin_id: String) -> Result<Option<PluginPortInfo>, String> {
    use sentinel_bounty::services::PluginPortRegistry;

    let registry = PluginPortRegistry::new();

    let output_ports: Vec<PortDef> = registry
        .get_output_ports(&plugin_id)
        .map(|ports| {
            ports
                .iter()
                .map(|(name, atype)| PortDef {
                    name: name.clone(),
                    artifact_type: atype.as_str().to_string(),
                    fields: registry
                        .get_output_field_specs(&plugin_id, name)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|field| ArtifactFieldDef {
                            name: field.name,
                            field_type: field.field_type,
                            required: field.required,
                            description: field.description,
                        })
                        .collect(),
                })
                .collect()
        })
        .unwrap_or_default();

    let input_params: Vec<InputParamDef> = registry
        .get_input_specs(&plugin_id)
        .map(|specs| {
            specs
                .iter()
                .map(|(name, spec)| InputParamDef {
                    name: name.clone(),
                    expected_artifact_type: spec.artifact_type.as_str().to_string(),
                    extract_path: spec.extract_path.clone(),
                    required: spec.required,
                })
                .collect()
        })
        .unwrap_or_default();

    if output_ports.is_empty() && input_params.is_empty() {
        return Ok(None);
    }

    Ok(Some(PluginPortInfo {
        plugin_id,
        output_ports,
        input_params,
    }))
}

/// Get all registered plugin ports
#[tauri::command]
pub async fn bounty_list_plugin_ports() -> Result<Vec<PluginPortInfo>, String> {
    use sentinel_bounty::services::PluginPortRegistry;

    let registry = PluginPortRegistry::new();

    // List of known builtin plugins
    let plugin_ids = vec![
        "subdomain_enumerator",
        "cidr_mapper",
        "dns_resolver",
        "http_prober",
        "tech_fingerprinter",
        "favicon_fingerprinter",
        "port_monitor",
        "service_probe",
        "cert_monitor",
        "directory_bruteforcer",
        "js_analyzer",
        "sensitive_file_scanner",
        "risk_scanner",
        "ssrf_detector",
        "cors_misconfiguration",
        "open_redirect_detector",
        "nextjs_rce_scanner",
        "subdomain_takeover",
    ];

    let mut result = Vec::new();
    for plugin_id in plugin_ids {
        let output_ports = registry
            .get_output_ports(plugin_id)
            .map(|ports| {
                ports
                    .iter()
                    .map(|(name, atype)| PortDef {
                        name: name.clone(),
                        artifact_type: atype.as_str().to_string(),
                        fields: registry
                            .get_output_field_specs(plugin_id, name)
                            .cloned()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|field| ArtifactFieldDef {
                                name: field.name,
                                field_type: field.field_type,
                                required: field.required,
                                description: field.description,
                            })
                            .collect(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let input_params = registry
            .get_input_specs(plugin_id)
            .map(|specs| {
                specs
                    .iter()
                    .map(|(name, spec)| InputParamDef {
                        name: name.clone(),
                        expected_artifact_type: spec.artifact_type.as_str().to_string(),
                        extract_path: spec.extract_path.clone(),
                        required: spec.required,
                    })
                    .collect()
            })
            .unwrap_or_default();

        result.push(PluginPortInfo {
            plugin_id: plugin_id.to_string(),
            output_ports,
            input_params,
        });
    }

    Ok(result)
}

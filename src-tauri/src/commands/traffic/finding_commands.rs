use serde::{Deserialize, Serialize};
use tauri::State;

use sentinel_traffic::{EvidenceRecord, VulnerabilityFilters, VulnerabilityRecord};

use super::TrafficAnalysisState;
use crate::commands::command_response_support::CommandResponse;
use crate::services::system_agents::finding_lifecycle::{
    derive_lifecycle_from_finding, TrafficFindingLifecycle,
};

fn select_primary_evidence<'a>(evidence: &'a [EvidenceRecord]) -> Option<&'a EvidenceRecord> {
    evidence
        .iter()
        .find(|item| !item.location.starts_with("system_agent_"))
        .or_else(|| {
            evidence.iter().find(|item| {
                !matches!(
                    item.location.as_str(),
                    "system_agent_verification" | "system_agent_feedback"
                )
            })
        })
        .or_else(|| evidence.first())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficFindingLifecycleStats {
    pub formal_total: i64,
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
    pub candidate: i64,
    pub verified: i64,
    pub false_positive: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingDetail {
    pub vulnerability: VulnerabilityRecord,
    pub evidence: Vec<EvidenceRecord>,
}

#[derive(Debug, Serialize)]
struct ReportSummary {
    total: usize,
    critical: usize,
    high: usize,
    medium: usize,
    low: usize,
    info: usize,
    critical_percent: f64,
    high_percent: f64,
    medium_percent: f64,
    low_percent: f64,
    info_percent: f64,
}

#[derive(Debug, Serialize)]
struct ReportFinding {
    id: String,
    title: String,
    description: String,
    severity: String,
    vuln_type: String,
    plugin_id: String,
    url: String,
    method: String,
    location: String,
    evidence: String,
    confidence: String,
    cwe: Option<String>,
    owasp: Option<String>,
    remediation: Option<String>,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct ReportData {
    report_title: String,
    generated_at: String,
    scan_scope: String,
    summary: ReportSummary,
    findings: Vec<ReportFinding>,
}

#[tauri::command]
pub async fn get_finding_lifecycle_stats(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<TrafficFindingLifecycleStats>, String> {
    let db_service = state.get_db_service();
    let records = db_service
        .list_traffic_vulnerabilities_with_evidence(VulnerabilityFilters::default())
        .await
        .map_err(|e| format!("Database error: {e}"))?;

    let mut formal_total = 0;
    let mut critical = 0;
    let mut high = 0;
    let mut medium = 0;
    let mut low = 0;
    let mut candidate = 0;
    let mut verified = 0;
    let mut false_positive = 0;

    for record in records {
        let lifecycle = derive_lifecycle_from_finding(&record);
        match lifecycle {
            TrafficFindingLifecycle::Hypothesis => {
                candidate += 1;
            }
            TrafficFindingLifecycle::Verified => {
                verified += 1;
                formal_total += 1;
                increment_severity_counter(
                    &record.vulnerability.severity,
                    &mut critical,
                    &mut high,
                    &mut medium,
                    &mut low,
                );
            }
            TrafficFindingLifecycle::FalsePositive => {
                false_positive += 1;
            }
            TrafficFindingLifecycle::Fixed | TrafficFindingLifecycle::FormalOpen => {
                formal_total += 1;
                increment_severity_counter(
                    &record.vulnerability.severity,
                    &mut critical,
                    &mut high,
                    &mut medium,
                    &mut low,
                );
            }
        }
    }

    Ok(CommandResponse::ok(TrafficFindingLifecycleStats {
        formal_total,
        critical,
        high,
        medium,
        low,
        candidate,
        verified,
        false_positive,
    }))
}

#[tauri::command]
pub async fn get_finding(
    state: State<'_, TrafficAnalysisState>,
    finding_id: String,
) -> Result<CommandResponse<Option<FindingDetail>>, String> {
    let db_service = state.get_db_service();

    let vulnerability = db_service
        .get_traffic_vulnerability_by_id(&finding_id)
        .await
        .map_err(|e| format!("Failed to fetch vulnerability: {}", e))?;

    if vulnerability.is_none() {
        return Ok(CommandResponse::ok(None));
    }

    let vulnerability = vulnerability.unwrap();

    let evidence = db_service
        .get_traffic_evidence_by_vuln_id(&finding_id)
        .await
        .map_err(|e| format!("Failed to fetch evidence: {}", e))?;

    let detail = FindingDetail {
        vulnerability,
        evidence,
    };

    tracing::debug!(
        "Fetched finding detail: {} with {} evidence",
        finding_id,
        detail.evidence.len()
    );

    Ok(CommandResponse::ok(Some(detail)))
}

#[tauri::command]
pub async fn update_finding_status(
    state: State<'_, TrafficAnalysisState>,
    finding_id: String,
    status: String,
) -> Result<CommandResponse<String>, String> {
    let valid_statuses = ["open", "candidate", "reviewed", "false_positive", "fixed"];
    if !valid_statuses.contains(&status.as_str()) {
        return Ok(CommandResponse::err(format!(
            "Invalid status: {}. Must be one of: {}",
            status,
            valid_statuses.join(", ")
        )));
    }

    let db_service = state.get_db_service();
    db_service
        .update_traffic_vulnerability_status(&finding_id, &status)
        .await
        .map_err(|e| format!("Failed to update vulnerability status: {}", e))?;

    tracing::info!("Updated finding {} status to {}", finding_id, status);

    Ok(CommandResponse::ok(format!(
        "Finding {} status updated to {}",
        finding_id, status
    )))
}

#[tauri::command]
pub async fn mark_findings_read(
    state: State<'_, TrafficAnalysisState>,
    finding_ids: Vec<String>,
) -> Result<CommandResponse<u64>, String> {
    let db_service = state.get_db_service();
    let updated = db_service
        .mark_traffic_vulnerabilities_viewed(&finding_ids, "local")
        .await
        .map_err(|e| format!("Failed to mark findings as read: {}", e))?;

    Ok(CommandResponse::ok(updated))
}

#[tauri::command]
pub async fn export_findings_html(
    state: State<'_, TrafficAnalysisState>,
    filters: Option<VulnerabilityFilters>,
) -> Result<CommandResponse<String>, String> {
    use std::fs;
    use tera::{Context, Tera};

    tracing::info!("Exporting HTML report with filters: {:?}", filters);

    let db_service = state.get_db_service();
    let filters = filters.unwrap_or(VulnerabilityFilters {
        vuln_type: None,
        severity: None,
        status: None,
        status_in: None,
        plugin_id: None,
        exclude_plugin_id: None,
        limit: Some(1000),
        offset: Some(0),
    });

    let vulnerabilities = db_service
        .list_traffic_vulnerabilities(filters.clone())
        .await
        .map_err(|e| format!("Failed to list vulnerabilities: {}", e))?;

    let total = vulnerabilities.len();
    let critical = vulnerabilities
        .iter()
        .filter(|v| v.severity == "critical")
        .count();
    let high = vulnerabilities
        .iter()
        .filter(|v| v.severity == "high")
        .count();
    let medium = vulnerabilities
        .iter()
        .filter(|v| v.severity == "medium")
        .count();
    let low = vulnerabilities
        .iter()
        .filter(|v| v.severity == "low")
        .count();
    let info = vulnerabilities
        .iter()
        .filter(|v| v.severity == "info")
        .count();

    let total_f = total as f64;
    let summary = ReportSummary {
        total,
        critical,
        high,
        medium,
        low,
        info,
        critical_percent: if total > 0 {
            (critical as f64 / total_f) * 100.0
        } else {
            0.0
        },
        high_percent: if total > 0 {
            (high as f64 / total_f) * 100.0
        } else {
            0.0
        },
        medium_percent: if total > 0 {
            (medium as f64 / total_f) * 100.0
        } else {
            0.0
        },
        low_percent: if total > 0 {
            (low as f64 / total_f) * 100.0
        } else {
            0.0
        },
        info_percent: if total > 0 {
            (info as f64 / total_f) * 100.0
        } else {
            0.0
        },
    };

    let mut findings: Vec<ReportFinding> = Vec::new();
    for v in vulnerabilities {
        let evidence_list = db_service
            .get_traffic_evidence_by_vuln_id(&v.id)
            .await
            .unwrap_or_default();

        let first_evidence = select_primary_evidence(&evidence_list);
        findings.push(ReportFinding {
            id: v.id.clone(),
            title: v.title.clone(),
            description: v.description.clone(),
            severity: v.severity.clone(),
            vuln_type: v.vuln_type.clone(),
            plugin_id: v.plugin_id.clone(),
            url: first_evidence.map(|e| e.url.clone()).unwrap_or_default(),
            method: first_evidence.map(|e| e.method.clone()).unwrap_or_default(),
            location: first_evidence
                .map(|e| e.location.clone())
                .unwrap_or_default(),
            evidence: first_evidence
                .map(|e| e.evidence_snippet.clone())
                .unwrap_or_default(),
            confidence: v.confidence.clone(),
            cwe: v.cwe.clone(),
            owasp: v.owasp.clone(),
            remediation: v.remediation.clone(),
            created_at: v.first_seen_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        });
    }

    let now = chrono::Utc::now();
    let report_data = ReportData {
        report_title: format!("流量分析报告 - {}", now.format("%Y年%m月%d日")),
        generated_at: now.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        scan_scope: filters
            .plugin_id
            .clone()
            .map(|p| format!("插件: {}", p))
            .unwrap_or_else(|| "全部".to_string()),
        summary,
        findings,
    };

    let template_path = std::env::current_dir()
        .map_err(|e| format!("Failed to get current dir: {}", e))?
        .join("templates/vulnerability_report.html");

    if !template_path.exists() {
        return Err(format!("Template not found: {:?}", template_path));
    }

    let template_content = fs::read_to_string(&template_path)
        .map_err(|e| format!("Failed to read template: {}", e))?;

    let mut tera = Tera::default();
    tera.add_raw_template("report", &template_content)
        .map_err(|e| format!("Failed to parse template: {}", e))?;

    let mut context = Context::new();
    context.insert("report_title", &report_data.report_title);
    context.insert("generated_at", &report_data.generated_at);
    context.insert("scan_scope", &report_data.scan_scope);
    context.insert("summary", &report_data.summary);
    context.insert("findings", &report_data.findings);

    let html = tera
        .render("report", &context)
        .map_err(|e| format!("Failed to render template: {}", e))?;

    let output_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".sentinel-ai")
        .join("reports");

    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let filename = format!(
        "traffic_analysis_report_{}.html",
        now.format("%Y%m%d_%H%M%S")
    );
    let output_path = output_dir.join(&filename);

    fs::write(&output_path, html).map_err(|e| format!("Failed to write report: {}", e))?;

    let path_str = output_path.to_string_lossy().to_string();
    tracing::info!("HTML report exported to: {}", path_str);

    Ok(CommandResponse::ok(path_str))
}

#[tauri::command]
pub async fn delete_traffic_vulnerability(
    state: State<'_, TrafficAnalysisState>,
    vuln_id: String,
) -> Result<CommandResponse<()>, String> {
    let db = state.get_db_service();
    let signature = db
        .delete_traffic_vulnerability(&vuln_id)
        .await
        .map_err(|e| format!("Failed to delete vulnerability: {}", e))?;

    if let Some(sig) = signature {
        let mut cache = state.dedupe_cache.write().await;
        let removed = cache.remove(&sig);
        tracing::info!(
            "Vulnerability deleted: {} - Removed from dedupe cache: {} (signature: {})",
            vuln_id,
            removed,
            &sig[..8.min(sig.len())]
        );
    } else {
        tracing::info!("Vulnerability deleted: {} (no signature found)", vuln_id);
    }
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn delete_traffic_vulnerabilities_batch(
    state: State<'_, TrafficAnalysisState>,
    vuln_ids: Vec<String>,
) -> Result<CommandResponse<()>, String> {
    let db = state.get_db_service();
    let mut signatures_to_remove = Vec::new();

    let mut deleted_count = 0;
    for vuln_id in &vuln_ids {
        match db.delete_traffic_vulnerability(vuln_id).await {
            Ok(Some(sig)) => {
                signatures_to_remove.push(sig);
                deleted_count += 1;
            }
            Ok(None) => {
                deleted_count += 1;
            }
            Err(e) => {
                tracing::warn!("Failed to delete vulnerability {}: {}", vuln_id, e);
            }
        }
    }

    if !signatures_to_remove.is_empty() {
        let mut cache = state.dedupe_cache.write().await;
        for sig in &signatures_to_remove {
            cache.remove(sig);
        }
        tracing::debug!(
            "Removed {} signatures from dedupe cache",
            signatures_to_remove.len()
        );
    }

    tracing::info!(
        "Batch deleted {} vulnerabilities out of {}",
        deleted_count,
        vuln_ids.len()
    );
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn delete_all_traffic_vulnerabilities(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<()>, String> {
    let db = state.get_db_service();

    db.delete_all_traffic_vulnerabilities()
        .await
        .map_err(|e| format!("Failed to delete all vulnerabilities: {}", e))?;

    let mut cache = state.dedupe_cache.write().await;
    cache.clear();
    tracing::info!("All vulnerabilities deleted and dedupe cache cleared");

    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn clear_vulnerability_dedupe_cache(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<()>, String> {
    let mut cache = state.dedupe_cache.write().await;
    let size = cache.len();
    cache.clear();
    tracing::info!("Vulnerability dedupe cache cleared ({} entries)", size);
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn get_vulnerability_dedupe_cache_info(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<serde_json::Value>, String> {
    let cache = state.dedupe_cache.read().await;
    let size = cache.len();

    let samples: Vec<String> = cache
        .iter()
        .take(10)
        .map(|s| s[..8.min(s.len())].to_string())
        .collect();

    let info = serde_json::json!({
        "cache_size": size,
        "sample_signatures": samples,
    });

    tracing::debug!("Dedupe cache info: {} entries", size);
    Ok(CommandResponse::ok(info))
}

fn increment_severity_counter(
    severity: &str,
    critical: &mut i64,
    high: &mut i64,
    medium: &mut i64,
    low: &mut i64,
) {
    match severity {
        "critical" => *critical += 1,
        "high" => *high += 1,
        "medium" => *medium += 1,
        "low" => *low += 1,
        _ => {}
    }
}

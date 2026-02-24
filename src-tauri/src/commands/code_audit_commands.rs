//! 代码审计 Tauri 命令
//!
//! 提供前端调用的代码审计相关命令

use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use sentinel_db::DatabaseService;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// 命令响应
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> CommandResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAuditFindingInput {
    pub id: String,
    pub title: Option<String>,
    pub severity: Option<String>,
    pub severity_raw: Option<String>,
    pub lifecycle_stage: Option<String>,
    pub verification_status: Option<String>,
    pub confidence: Option<f64>,
    pub files: Option<Vec<String>>,
    pub fix: Option<String>,
    pub status: Option<String>,
    pub cwe: Option<String>,
    pub description: Option<String>,
    pub source: Option<serde_json::Value>,
    pub sink: Option<serde_json::Value>,
    pub trace_path: Option<Vec<serde_json::Value>>,
    pub evidence: Option<Vec<String>>,
    pub required_evidence: Option<Vec<String>>,
    pub verifier: Option<serde_json::Value>,
    pub judge: Option<serde_json::Value>,
    pub provenance: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertAgentAuditFindingsRequest {
    pub conversation_id: String,
    pub findings: Vec<AgentAuditFindingInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertAgentAuditFindingsResult {
    pub inserted: usize,
    pub updated_hits: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAuditFindingView {
    pub id: String,
    pub conversation_id: String,
    pub finding_id: String,
    pub title: String,
    pub severity: String,
    pub status: String,
    pub lifecycle_stage: String,
    pub verification_status: String,
    pub confidence: Option<f64>,
    pub cwe: Option<String>,
    pub files: Vec<String>,
    pub source: Option<serde_json::Value>,
    pub sink: Option<serde_json::Value>,
    pub trace_path: Vec<serde_json::Value>,
    pub evidence: Vec<String>,
    pub required_evidence: Vec<String>,
    pub verifier: Option<serde_json::Value>,
    pub judge: Option<serde_json::Value>,
    pub provenance: Option<serde_json::Value>,
    pub fix: Option<String>,
    pub description: String,
    pub severity_raw: Option<String>,
    pub source_message_id: Option<String>,
    pub hit_count: i64,
    pub last_transition_at: Option<chrono::DateTime<chrono::Utc>>,
    pub first_seen_at: chrono::DateTime<chrono::Utc>,
    pub last_seen_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionAgentAuditFindingLifecycleRequest {
    pub finding_id: String,
    pub lifecycle_stage: String,
    #[serde(default)]
    pub verification_status: Option<String>,
    #[serde(default)]
    pub judge: Option<serde_json::Value>,
    #[serde(default)]
    pub verifier: Option<serde_json::Value>,
    #[serde(default)]
    pub provenance: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAuditQualityGateThresholds {
    pub min_evidence_rate: f64,
    pub max_uncertain_rate: f64,
    pub max_false_positive_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAuditQualityGateMetrics {
    pub total_findings: i64,
    pub with_evidence_count: i64,
    pub uncertain_count: i64,
    pub false_positive_or_rejected_count: i64,
    pub evidence_rate: f64,
    pub uncertain_rate: f64,
    pub false_positive_rate: f64,
    pub thresholds: AgentAuditQualityGateThresholds,
    pub threshold_source: String,
    pub gate_passed: bool,
}

fn map_audit_status_to_traffic_status(status: Option<&str>) -> String {
    match status.unwrap_or("").to_lowercase().as_str() {
        "confirmed" => "reviewed".to_string(),
        "rejected" => "false_positive".to_string(),
        "fixed" => "fixed".to_string(),
        _ => "open".to_string(),
    }
}

fn normalize_lifecycle_stage(input: Option<&str>) -> String {
    match input.unwrap_or("").trim().to_lowercase().as_str() {
        "candidate" => "candidate".to_string(),
        "triaged" => "triaged".to_string(),
        "verified" => "verified".to_string(),
        "confirmed" => "confirmed".to_string(),
        "rejected" => "rejected".to_string(),
        "archived" => "archived".to_string(),
        _ => "confirmed".to_string(),
    }
}

fn normalize_verification_status(input: Option<&str>) -> String {
    match input.unwrap_or("").trim().to_lowercase().as_str() {
        "unverified" => "unverified".to_string(),
        "pending" => "pending".to_string(),
        "passed" => "passed".to_string(),
        "failed" => "failed".to_string(),
        "needs_more_evidence" => "needs_more_evidence".to_string(),
        _ => "unverified".to_string(),
    }
}

fn infer_lifecycle_from_status(status: Option<&str>) -> Option<String> {
    match status.unwrap_or("").trim().to_lowercase().as_str() {
        "confirmed" => Some("confirmed".to_string()),
        "rejected" | "false_positive" => Some("rejected".to_string()),
        "fixed" => Some("archived".to_string()),
        _ => None,
    }
}

fn is_valid_lifecycle_transition(current: &str, next: &str) -> bool {
    if current == next {
        return true;
    }
    matches!(
        (current, next),
        ("candidate", "triaged")
            | ("candidate", "rejected")
            | ("candidate", "verified")
            | ("candidate", "confirmed")
            | ("triaged", "verified")
            | ("triaged", "rejected")
            | ("triaged", "confirmed")
            | ("verified", "confirmed")
            | ("verified", "rejected")
            | ("confirmed", "archived")
            | ("confirmed", "rejected")
            | ("rejected", "candidate")
            | ("rejected", "triaged")
            | ("archived", "candidate")
    )
}

fn normalize_severity(severity: Option<&str>) -> String {
    match severity.unwrap_or("").to_lowercase().as_str() {
        "critical" => "critical".to_string(),
        "严重" => "critical".to_string(),
        "high" => "high".to_string(),
        "高" => "high".to_string(),
        "高危" => "high".to_string(),
        "medium" => "medium".to_string(),
        "中" => "medium".to_string(),
        "中危" => "medium".to_string(),
        "low" => "low".to_string(),
        "低" => "low".to_string(),
        "低危" => "low".to_string(),
        "info" => "info".to_string(),
        "unknown" => "unknown".to_string(),
        _ => "unknown".to_string(),
    }
}

fn infer_severity_from_content(
    title: Option<&str>,
    description: Option<&str>,
    cwe: Option<&str>,
) -> String {
    let merged = format!(
        "{}\n{}\n{}",
        title.unwrap_or_default(),
        description.unwrap_or_default(),
        cwe.unwrap_or_default()
    )
    .to_lowercase();

    if merged.contains("sql注入") || merged.contains("sql injection") || merged.contains("cwe-89") {
        return "high".to_string();
    }
    if merged.contains("command injection") || merged.contains("命令注入") || merged.contains("rce") {
        return "critical".to_string();
    }
    if merged.contains("xxe") || merged.contains("ssti") {
        return "high".to_string();
    }
    if merged.contains("xss") {
        return "medium".to_string();
    }
    if merged.contains("csrf") {
        return "medium".to_string();
    }
    if merged.contains("弱口令") || merged.contains("weak password") {
        return "medium".to_string();
    }
    "info".to_string()
}

fn extract_files_from_text(input: &str) -> Vec<String> {
    let re = Regex::new(r#"((?:[\w\.-]+/)+[\w\.-]+\.[A-Za-z0-9]+)"#)
        .expect("file path regex should compile");
    let mut files: Vec<String> = Vec::new();
    for cap in re.captures_iter(input) {
        if let Some(m) = cap.get(1) {
            let v = m.as_str().to_string();
            if !files.iter().any(|item| item == &v) {
                files.push(v);
            }
        }
    }
    files
}

fn build_agent_audit_signature(
    conversation_id: &str,
    finding_id: &str,
    title: &str,
    cwe: Option<&str>,
    files: &[String],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(conversation_id.as_bytes());
    hasher.update(finding_id.as_bytes());
    hasher.update(title.as_bytes());
    hasher.update(cwe.unwrap_or("").as_bytes());
    for file in files {
        hasher.update(file.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn map_agent_audit_record_to_view(
    row: sentinel_db::AgentAuditFindingRecord,
) -> AgentAuditFindingView {
    let files = row
        .files_json
        .as_ref()
        .and_then(|v| serde_json::from_str::<Vec<String>>(v).ok())
        .unwrap_or_default();

    AgentAuditFindingView {
        id: row.id,
        conversation_id: row.conversation_id,
        finding_id: row.finding_id,
        title: row.title,
        severity: row.severity,
        status: row.status,
        lifecycle_stage: row.lifecycle_stage,
        verification_status: row.verification_status,
        confidence: row.confidence,
        cwe: row.cwe,
        files,
        source: row
            .source_json
            .as_ref()
            .and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok()),
        sink: row
            .sink_json
            .as_ref()
            .and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok()),
        trace_path: row
            .trace_path_json
            .as_ref()
            .and_then(|v| serde_json::from_str::<Vec<serde_json::Value>>(v).ok())
            .unwrap_or_default(),
        evidence: row
            .evidence_json
            .as_ref()
            .and_then(|v| serde_json::from_str::<Vec<String>>(v).ok())
            .unwrap_or_default(),
        required_evidence: row
            .required_evidence_json
            .as_ref()
            .and_then(|v| serde_json::from_str::<Vec<String>>(v).ok())
            .unwrap_or_default(),
        verifier: row
            .verifier_json
            .as_ref()
            .and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok()),
        judge: row
            .judge_json
            .as_ref()
            .and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok()),
        provenance: row
            .provenance_json
            .as_ref()
            .and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok()),
        fix: row.fix,
        description: row.description,
        severity_raw: row.severity_raw,
        source_message_id: row.source_message_id,
        hit_count: row.hit_count,
        last_transition_at: row.last_transition_at,
        first_seen_at: row.first_seen_at,
        last_seen_at: row.last_seen_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn default_quality_gate_thresholds() -> AgentAuditQualityGateThresholds {
    AgentAuditQualityGateThresholds {
        min_evidence_rate: 0.70,
        max_uncertain_rate: 0.30,
        max_false_positive_rate: 0.20,
    }
}

fn normalize_quality_gate_thresholds(
    thresholds: AgentAuditQualityGateThresholds,
) -> AgentAuditQualityGateThresholds {
    AgentAuditQualityGateThresholds {
        min_evidence_rate: thresholds.min_evidence_rate.clamp(0.0, 1.0),
        max_uncertain_rate: thresholds.max_uncertain_rate.clamp(0.0, 1.0),
        max_false_positive_rate: thresholds.max_false_positive_rate.clamp(0.0, 1.0),
    }
}

fn quality_gate_rates(
    total_findings: i64,
    with_evidence_count: i64,
    uncertain_count: i64,
    false_positive_or_rejected_count: i64,
) -> (f64, f64, f64) {
    if total_findings <= 0 {
        return (0.0, 0.0, 0.0);
    }
    let total = total_findings as f64;
    (
        (with_evidence_count as f64) / total,
        (uncertain_count as f64) / total,
        (false_positive_or_rejected_count as f64) / total,
    )
}

fn has_non_empty_string_array_json(raw: Option<&String>) -> bool {
    let Some(raw) = raw else {
        return false;
    };
    serde_json::from_str::<Vec<String>>(raw)
        .map(|items| items.iter().any(|v| !v.trim().is_empty()))
        .unwrap_or(false)
}

fn build_quality_gate_metrics(
    records: &[sentinel_db::AgentAuditFindingRecord],
    thresholds: AgentAuditQualityGateThresholds,
    threshold_source: String,
) -> AgentAuditQualityGateMetrics {
    let thresholds = normalize_quality_gate_thresholds(thresholds);
    let total_findings = records.len() as i64;
    let with_evidence_count = records
        .iter()
        .filter(|r| has_non_empty_string_array_json(r.evidence_json.as_ref()))
        .count() as i64;
    let uncertain_count = records
        .iter()
        .filter(|r| {
            r.verification_status == "needs_more_evidence"
                || r.lifecycle_stage == "candidate"
                || r.lifecycle_stage == "triaged"
                || r.lifecycle_stage == "verified"
        })
        .count() as i64;
    let false_positive_or_rejected_count = records
        .iter()
        .filter(|r| r.status == "false_positive" || r.lifecycle_stage == "rejected")
        .count() as i64;
    let (evidence_rate, uncertain_rate, false_positive_rate) = quality_gate_rates(
        total_findings,
        with_evidence_count,
        uncertain_count,
        false_positive_or_rejected_count,
    );

    let gate_passed = total_findings == 0
        || (evidence_rate >= thresholds.min_evidence_rate
            && uncertain_rate <= thresholds.max_uncertain_rate
            && false_positive_rate <= thresholds.max_false_positive_rate);

    AgentAuditQualityGateMetrics {
        total_findings,
        with_evidence_count,
        uncertain_count,
        false_positive_or_rejected_count,
        evidence_rate,
        uncertain_rate,
        false_positive_rate,
        thresholds,
        threshold_source,
        gate_passed,
    }
}

async fn resolve_quality_gate_thresholds(
    db_service: &Arc<DatabaseService>,
    conversation_id: Option<&str>,
    thresholds_override: Option<AgentAuditQualityGateThresholds>,
) -> (AgentAuditQualityGateThresholds, String) {
    if let Some(override_thresholds) = thresholds_override {
        return (
            normalize_quality_gate_thresholds(override_thresholds),
            "runtime_override".to_string(),
        );
    }

    if let Some(conversation_id) = conversation_id.filter(|v| !v.trim().is_empty()) {
        let scoped = db_service
            .get_config_internal("agent_audit_quality_gate_conversation", conversation_id)
            .await
            .ok()
            .flatten()
            .and_then(|raw| serde_json::from_str::<AgentAuditQualityGateThresholds>(&raw).ok())
            .map(normalize_quality_gate_thresholds);
        if let Some(thresholds) = scoped {
            return (thresholds, "conversation_override".to_string());
        }
    }

    let global = db_service
        .get_config_internal("agent_audit_quality_gate", "thresholds")
        .await
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<AgentAuditQualityGateThresholds>(&raw).ok())
        .map(normalize_quality_gate_thresholds);
    if let Some(thresholds) = global {
        return (thresholds, "global_config".to_string());
    }

    (
        default_quality_gate_thresholds(),
        "builtin_default".to_string(),
    )
}

pub async fn upsert_agent_audit_findings_with_db(
    db_service: &Arc<DatabaseService>,
    request: UpsertAgentAuditFindingsRequest,
) -> Result<UpsertAgentAuditFindingsResult, String> {
    let mut inserted = 0usize;
    let mut updated_hits = 0usize;

    for finding in request.findings {
        let mut files = finding.files.clone().unwrap_or_default();
        let title = finding
            .title
            .clone()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| format!("Audit finding {}", finding.id));
        let severity_raw = finding
            .severity_raw
            .clone()
            .or_else(|| finding.severity.clone());
        let mut severity = normalize_severity(severity_raw.as_deref());
        let inferred_severity = infer_severity_from_content(
            finding.title.as_deref(),
            finding.description.as_deref(),
            finding.cwe.as_deref(),
        );
        if severity == "unknown" || (severity == "info" && inferred_severity != "info") {
            severity = inferred_severity;
        }
        let status = map_audit_status_to_traffic_status(finding.status.as_deref());
        let inferred_lifecycle = infer_lifecycle_from_status(finding.status.as_deref());
        let lifecycle_stage = normalize_lifecycle_stage(
            finding
                .lifecycle_stage
                .as_deref()
                .or(inferred_lifecycle.as_deref()),
        );
        let verification_status = normalize_verification_status(finding.verification_status.as_deref());
        let confidence = finding.confidence;
        let now = Utc::now();
        if files.is_empty() {
            let mut from_text = extract_files_from_text(&title);
            if let Some(desc) = finding.description.as_ref() {
                from_text.extend(extract_files_from_text(desc));
            }
            if !from_text.is_empty() {
                files = from_text;
            }
        }
        let signature = build_agent_audit_signature(
            &request.conversation_id,
            &finding.id,
            &title,
            finding.cwe.as_deref(),
            &files,
        );
        let files_json = if files.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&files).unwrap_or_else(|_| "[]".to_string()))
        };
        let source_json = finding
            .source
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());
        let sink_json = finding
            .sink
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());
        let trace_path_json = finding
            .trace_path
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());
        let evidence_json = finding
            .evidence
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());
        let required_evidence_json = finding
            .required_evidence
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());
        let verifier_json = finding
            .verifier
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());
        let judge_json = finding
            .judge
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());
        let provenance_json = finding
            .provenance
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok());

        let mut description = finding.description.clone().unwrap_or_default();
        if description.trim().is_empty() {
            description = title.clone();
        }
        if !files.is_empty() {
            description.push_str("\n\nAffected files:\n");
            for file in files.iter().take(10) {
                description.push_str("- ");
                description.push_str(file);
                description.push('\n');
            }
        }
        if let Some(fix) = finding.fix.as_ref().filter(|v| !v.trim().is_empty()) {
            description.push_str("\nRecommended fix:\n");
            description.push_str(fix);
        }
        let audit_finding = sentinel_db::AgentAuditFindingRecord {
            id: Uuid::new_v4().to_string(),
            conversation_id: request.conversation_id.clone(),
            finding_id: finding.id.clone(),
            signature: signature.clone(),
            title,
            severity,
            status,
            lifecycle_stage: lifecycle_stage.clone(),
            verification_status,
            confidence,
            cwe: finding.cwe.clone(),
            files_json,
            source_json,
            sink_json,
            trace_path_json,
            evidence_json,
            required_evidence_json,
            verifier_json,
            judge_json,
            provenance_json,
            fix: finding.fix.clone(),
            description,
            severity_raw,
            source_message_id: None,
            hit_count: 1,
            last_transition_at: Some(now),
            first_seen_at: now,
            last_seen_at: now,
            created_at: now,
            updated_at: now,
        };

        let exists = db_service
            .check_agent_audit_signature_exists(&signature)
            .await
            .map_err(|e| format!("Failed checking finding signature: {}", e))?;

        if exists {
            if let Some(existing) = db_service
                .get_agent_audit_finding_by_signature(&signature)
                .await
                .map_err(|e| format!("Failed loading existing finding by signature: {}", e))?
            {
                if !is_valid_lifecycle_transition(&existing.lifecycle_stage, &lifecycle_stage) {
                    return Err(format!(
                        "Invalid lifecycle transition for finding {}: {} -> {}",
                        finding.id, existing.lifecycle_stage, lifecycle_stage
                    ));
                }
            }
            db_service
                .update_agent_audit_finding_hit(&signature, &audit_finding)
                .await
                .map_err(|e| format!("Failed updating finding hit count: {}", e))?;
            updated_hits += 1;
        } else {
            db_service
                .insert_agent_audit_finding(&audit_finding)
                .await
                .map_err(|e| format!("Failed inserting audit finding: {}", e))?;
            inserted += 1;
        }
    }

    Ok(UpsertAgentAuditFindingsResult {
        inserted,
        updated_hits,
        total: inserted + updated_hits,
    })
}

/// 将 Agent 审计发现写入安全中心代码审计库（agent_audit_findings）
#[tauri::command]
pub async fn upsert_agent_audit_findings(
    db_service: State<'_, Arc<DatabaseService>>,
    request: UpsertAgentAuditFindingsRequest,
) -> Result<CommandResponse<UpsertAgentAuditFindingsResult>, String> {
    let result = upsert_agent_audit_findings_with_db(&db_service, request).await?;
    Ok(CommandResponse::ok(result))
}

/// 列出 Agent 代码审计发现
#[tauri::command]
pub async fn list_agent_audit_findings(
    db_service: State<'_, Arc<DatabaseService>>,
    limit: Option<i64>,
    offset: Option<i64>,
    severity_filter: Option<String>,
    status_filter: Option<String>,
    lifecycle_stage_filter: Option<String>,
    conversation_id: Option<String>,
    search: Option<String>,
) -> Result<CommandResponse<Vec<AgentAuditFindingView>>, String> {
    let filters = sentinel_db::AgentAuditFindingFilters {
        conversation_id: conversation_id.clone(),
        severity: severity_filter,
        status: status_filter,
        lifecycle_stage: lifecycle_stage_filter,
        search,
        limit: Some(limit.unwrap_or(10)),
        offset,
    };

    match db_service.list_agent_audit_findings(filters).await {
        Ok(records) => Ok(CommandResponse::ok(
            records
                .into_iter()
                .map(map_agent_audit_record_to_view)
                .collect(),
        )),
        Err(e) => {
            tracing::error!("Failed to load agent audit findings: {}", e);
            Ok(CommandResponse::err(format!("Database error: {}", e)))
        }
    }
}

/// 统计 Agent 代码审计发现总数（用于分页）
#[tauri::command]
pub async fn count_agent_audit_findings(
    db_service: State<'_, Arc<DatabaseService>>,
    severity_filter: Option<String>,
    status_filter: Option<String>,
    lifecycle_stage_filter: Option<String>,
    conversation_id: Option<String>,
    search: Option<String>,
) -> Result<CommandResponse<i64>, String> {
    let filters = sentinel_db::AgentAuditFindingFilters {
        conversation_id: conversation_id.clone(),
        severity: severity_filter,
        status: status_filter,
        lifecycle_stage: lifecycle_stage_filter,
        search,
        ..Default::default()
    };

    match db_service.count_agent_audit_findings(filters).await {
        Ok(count) => Ok(CommandResponse::ok(count)),
        Err(e) => {
            tracing::error!("Failed to count agent audit findings: {}", e);
            Ok(CommandResponse::err(format!("Database error: {}", e)))
        }
    }
}

/// 获取 Agent 代码审计质量门禁指标
#[tauri::command]
pub async fn get_agent_audit_quality_gate_metrics(
    db_service: State<'_, Arc<DatabaseService>>,
    severity_filter: Option<String>,
    status_filter: Option<String>,
    lifecycle_stage_filter: Option<String>,
    conversation_id: Option<String>,
    search: Option<String>,
    thresholds_override: Option<AgentAuditQualityGateThresholds>,
) -> Result<CommandResponse<AgentAuditQualityGateMetrics>, String> {
    let filters = sentinel_db::AgentAuditFindingFilters {
        conversation_id: conversation_id.clone(),
        severity: severity_filter,
        status: status_filter,
        lifecycle_stage: lifecycle_stage_filter,
        search,
        limit: None,
        offset: None,
    };

    let (thresholds, threshold_source) = resolve_quality_gate_thresholds(
        db_service.inner(),
        conversation_id.as_deref(),
        thresholds_override,
    )
    .await;

    match db_service.list_agent_audit_findings(filters).await {
        Ok(records) => Ok(CommandResponse::ok(build_quality_gate_metrics(
            &records,
            thresholds,
            threshold_source,
        ))),
        Err(e) => {
            tracing::error!("Failed to build audit quality gate metrics: {}", e);
            Ok(CommandResponse::err(format!("Database error: {}", e)))
        }
    }
}

#[tauri::command]
pub async fn get_agent_audit_quality_gate_thresholds(
    db_service: State<'_, Arc<DatabaseService>>,
    conversation_id: Option<String>,
) -> Result<CommandResponse<AgentAuditQualityGateThresholds>, String> {
    let (thresholds, _source) =
        resolve_quality_gate_thresholds(db_service.inner(), conversation_id.as_deref(), None).await;
    Ok(CommandResponse::ok(thresholds))
}

#[tauri::command]
pub async fn save_agent_audit_quality_gate_thresholds(
    db_service: State<'_, Arc<DatabaseService>>,
    thresholds: AgentAuditQualityGateThresholds,
    conversation_id: Option<String>,
) -> Result<CommandResponse<AgentAuditQualityGateThresholds>, String> {
    let normalized = normalize_quality_gate_thresholds(thresholds);
    let payload = serde_json::to_string(&normalized)
        .map_err(|e| format!("Failed to serialize thresholds: {}", e))?;
    let (category, key, description) = if let Some(conversation_id) =
        conversation_id.filter(|v| !v.trim().is_empty())
    {
        (
            "agent_audit_quality_gate_conversation",
            conversation_id,
            "Conversation scoped audit quality gate thresholds",
        )
    } else {
        (
            "agent_audit_quality_gate",
            "thresholds".to_string(),
            "Audit quality gate thresholds",
        )
    };
    db_service
        .set_config_internal(
            category,
            &key,
            &payload,
            Some(description),
        )
        .await
        .map_err(|e| format!("Failed to save audit quality gate thresholds: {}", e))?;
    Ok(CommandResponse::ok(normalized))
}

/// 获取 Agent 代码审计发现详情
#[tauri::command]
pub async fn get_agent_audit_finding(
    db_service: State<'_, Arc<DatabaseService>>,
    finding_id: String,
) -> Result<CommandResponse<Option<AgentAuditFindingView>>, String> {
    match db_service.get_agent_audit_finding_by_id(&finding_id).await {
        Ok(record) => Ok(CommandResponse::ok(record.map(map_agent_audit_record_to_view))),
        Err(e) => {
            tracing::error!("Failed to get agent audit finding: {}", e);
            Ok(CommandResponse::err(format!("Database error: {}", e)))
        }
    }
}

/// 更新 Agent 代码审计发现状态
#[tauri::command]
pub async fn update_agent_audit_finding_status(
    db_service: State<'_, Arc<DatabaseService>>,
    finding_id: String,
    status: String,
) -> Result<CommandResponse<String>, String> {
    let valid_statuses = ["open", "reviewed", "false_positive", "fixed"];
    if !valid_statuses.contains(&status.as_str()) {
        return Ok(CommandResponse::err(format!(
            "Invalid status: {}. Must be one of: {}",
            status,
            valid_statuses.join(", ")
        )));
    }

    db_service
        .update_agent_audit_finding_status(&finding_id, &status)
        .await
        .map_err(|e| format!("Failed to update agent audit finding status: {}", e))?;

    Ok(CommandResponse::ok(format!(
        "Agent audit finding {} status updated to {}",
        finding_id, status
    )))
}

/// 删除单个 Agent 代码审计发现
#[tauri::command]
pub async fn delete_agent_audit_finding(
    db_service: State<'_, Arc<DatabaseService>>,
    finding_id: String,
) -> Result<CommandResponse<()>, String> {
    db_service
        .delete_agent_audit_finding(&finding_id)
        .await
        .map_err(|e| format!("Failed to delete agent audit finding: {}", e))?;
    Ok(CommandResponse::ok(()))
}

/// 批量删除 Agent 代码审计发现
#[tauri::command]
pub async fn delete_agent_audit_findings_batch(
    db_service: State<'_, Arc<DatabaseService>>,
    finding_ids: Vec<String>,
) -> Result<CommandResponse<()>, String> {
    for finding_id in &finding_ids {
        if let Err(e) = db_service.delete_agent_audit_finding(finding_id).await {
            tracing::warn!(
                "Failed to delete agent audit finding {}: {}",
                finding_id,
                e
            );
        }
    }
    Ok(CommandResponse::ok(()))
}

/// 删除全部 Agent 代码审计发现
#[tauri::command]
pub async fn delete_all_agent_audit_findings(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<CommandResponse<()>, String> {
    db_service
        .delete_all_agent_audit_findings()
        .await
        .map_err(|e| format!("Failed to delete all agent audit findings: {}", e))?;
    Ok(CommandResponse::ok(()))
}

#[tauri::command]
pub async fn transition_agent_audit_finding_lifecycle(
    db_service: State<'_, Arc<DatabaseService>>,
    request: TransitionAgentAuditFindingLifecycleRequest,
) -> Result<CommandResponse<String>, String> {
    let target_stage = normalize_lifecycle_stage(Some(&request.lifecycle_stage));
    let current = db_service
        .get_agent_audit_finding_by_id(&request.finding_id)
        .await
        .map_err(|e| format!("Failed loading finding: {}", e))?
        .ok_or_else(|| format!("Finding not found: {}", request.finding_id))?;

    if !is_valid_lifecycle_transition(&current.lifecycle_stage, &target_stage) {
        return Ok(CommandResponse::err(format!(
            "Invalid lifecycle transition: {} -> {}",
            current.lifecycle_stage, target_stage
        )));
    }

    let verification_status = request
        .verification_status
        .as_deref()
        .map(|v| normalize_verification_status(Some(v)));
    let judge_json = request.judge.and_then(|v| serde_json::to_string(&v).ok());
    let verifier_json = request.verifier.and_then(|v| serde_json::to_string(&v).ok());
    let provenance_json = request.provenance.and_then(|v| serde_json::to_string(&v).ok());

    db_service
        .update_agent_audit_finding_lifecycle(
            &request.finding_id,
            &target_stage,
            verification_status.as_deref(),
            judge_json.as_deref(),
            verifier_json.as_deref(),
            provenance_json.as_deref(),
        )
        .await
        .map_err(|e| format!("Failed to transition lifecycle: {}", e))?;

    Ok(CommandResponse::ok(format!(
        "Lifecycle transitioned: {} -> {}",
        current.lifecycle_stage, target_stage
    )))
}

#[cfg(test)]
mod tests {
    use super::{
        infer_lifecycle_from_status, is_valid_lifecycle_transition, normalize_lifecycle_stage,
        normalize_quality_gate_thresholds, normalize_verification_status, resolve_quality_gate_thresholds,
        quality_gate_rates,
    };
    use sentinel_db::DatabaseService;
    use std::sync::Arc;

    #[test]
    fn normalize_lifecycle_stage_defaults_to_confirmed() {
        assert_eq!(normalize_lifecycle_stage(None), "confirmed");
        assert_eq!(normalize_lifecycle_stage(Some("unknown_stage")), "confirmed");
        assert_eq!(normalize_lifecycle_stage(Some("Triaged")), "triaged");
    }

    #[test]
    fn normalize_verification_status_defaults_to_unverified() {
        assert_eq!(normalize_verification_status(None), "unverified");
        assert_eq!(
            normalize_verification_status(Some("not_exist")),
            "unverified"
        );
        assert_eq!(normalize_verification_status(Some("PASSED")), "passed");
    }

    #[test]
    fn lifecycle_transition_state_machine_is_enforced() {
        assert!(is_valid_lifecycle_transition("candidate", "triaged"));
        assert!(is_valid_lifecycle_transition("verified", "confirmed"));
        assert!(is_valid_lifecycle_transition("confirmed", "archived"));

        assert!(!is_valid_lifecycle_transition("candidate", "archived"));
        assert!(!is_valid_lifecycle_transition("archived", "confirmed"));
        assert!(!is_valid_lifecycle_transition("verified", "candidate"));
    }

    #[test]
    fn lifecycle_progression_candidate_to_archived_requires_confirmation_step() {
        let mut stage = "candidate".to_string();

        assert!(is_valid_lifecycle_transition(&stage, "confirmed"));
        stage = "confirmed".to_string();

        assert!(is_valid_lifecycle_transition(&stage, "archived"));
        assert!(!is_valid_lifecycle_transition("candidate", "archived"));
    }

    #[test]
    fn status_to_lifecycle_inference_matches_expected_mapping() {
        assert_eq!(
            infer_lifecycle_from_status(Some("confirmed")),
            Some("confirmed".to_string())
        );
        assert_eq!(
            infer_lifecycle_from_status(Some("false_positive")),
            Some("rejected".to_string())
        );
        assert_eq!(
            infer_lifecycle_from_status(Some("fixed")),
            Some("archived".to_string())
        );
        assert_eq!(infer_lifecycle_from_status(Some("open")), None);
    }

    #[test]
    fn quality_gate_rate_calculation_is_correct() {
        let (evidence_rate, uncertain_rate, false_positive_rate) = quality_gate_rates(10, 7, 2, 1);
        assert!((evidence_rate - 0.7).abs() < f64::EPSILON);
        assert!((uncertain_rate - 0.2).abs() < f64::EPSILON);
        assert!((false_positive_rate - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn quality_gate_thresholds_are_clamped_to_0_1() {
        let normalized = normalize_quality_gate_thresholds(super::AgentAuditQualityGateThresholds {
            min_evidence_rate: 2.0,
            max_uncertain_rate: -0.2,
            max_false_positive_rate: 1.5,
        });
        assert!((normalized.min_evidence_rate - 1.0).abs() < f64::EPSILON);
        assert!((normalized.max_uncertain_rate - 0.0).abs() < f64::EPSILON);
        assert!((normalized.max_false_positive_rate - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn runtime_override_thresholds_have_highest_priority() {
        let db = Arc::new(DatabaseService::new());
        let (resolved, source) = resolve_quality_gate_thresholds(
            &db,
            Some("conv-1"),
            Some(super::AgentAuditQualityGateThresholds {
                min_evidence_rate: 0.55,
                max_uncertain_rate: 0.22,
                max_false_positive_rate: 0.11,
            }),
        )
        .await;
        assert_eq!(source, "runtime_override");
        assert!((resolved.min_evidence_rate - 0.55).abs() < f64::EPSILON);
    }
}

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
    pub confidence: Option<f64>,
    pub cwe: Option<String>,
    pub files: Vec<String>,
    pub source: Option<serde_json::Value>,
    pub sink: Option<serde_json::Value>,
    pub trace_path: Vec<serde_json::Value>,
    pub evidence: Vec<String>,
    pub fix: Option<String>,
    pub description: String,
    pub severity_raw: Option<String>,
    pub source_message_id: Option<String>,
    pub hit_count: i64,
    pub first_seen_at: chrono::DateTime<chrono::Utc>,
    pub last_seen_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn map_audit_status_to_traffic_status(status: Option<&str>) -> String {
    match status.unwrap_or("").to_lowercase().as_str() {
        "confirmed" => "reviewed".to_string(),
        "rejected" => "false_positive".to_string(),
        "fixed" => "fixed".to_string(),
        _ => "open".to_string(),
    }
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
        fix: row.fix,
        description: row.description,
        severity_raw: row.severity_raw,
        source_message_id: row.source_message_id,
        hit_count: row.hit_count,
        first_seen_at: row.first_seen_at,
        last_seen_at: row.last_seen_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
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
            confidence,
            cwe: finding.cwe.clone(),
            files_json,
            source_json,
            sink_json,
            trace_path_json,
            evidence_json,
            fix: finding.fix.clone(),
            description,
            severity_raw,
            source_message_id: None,
            hit_count: 1,
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
    conversation_id: Option<String>,
    search: Option<String>,
) -> Result<CommandResponse<Vec<AgentAuditFindingView>>, String> {
    let filters = sentinel_db::AgentAuditFindingFilters {
        conversation_id,
        severity: severity_filter,
        status: status_filter,
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
    conversation_id: Option<String>,
    search: Option<String>,
) -> Result<CommandResponse<i64>, String> {
    let filters = sentinel_db::AgentAuditFindingFilters {
        conversation_id,
        severity: severity_filter,
        status: status_filter,
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

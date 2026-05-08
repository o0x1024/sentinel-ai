//! Bug Bounty commands for Tauri

use super::bounty_workflow_event_support::bounty_trigger_workflows_for_event_internal;
use crate::services::ensure_bug_bounty_access;
use chrono::Utc;
use sentinel_bounty::services::{
    CreateFindingInput, CreateProgramInput, CreateSubmissionInput, FindingService,
    ProgramDbService, SubmissionDbService, UpdateFindingInput, UpdateProgramInput,
    UpdateSubmissionInput,
};
use sentinel_db::{
    BountyChangeEventRow, BountyChangeEventStats, BountyEvidenceRow, BountyFindingRow,
    BountyFindingStats, BountyProgramRow, BountySubmissionRow, BountySubmissionStats,
    DatabaseService, FindingQueryFilter, ProgramQueryFilter, ProgramScopeRow,
    SubmissionQueryFilter,
};
use sentinel_traffic::PluginManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, State};
use uuid::Uuid;

pub(crate) fn ensure_bounty_feature() -> Result<(), String> {
    ensure_bug_bounty_access()
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProgramRequest {
    pub name: String,
    pub organization: String,
    pub platform: Option<String>,
    pub platform_handle: Option<String>,
    pub url: Option<String>,
    pub program_type: Option<String>,
    pub description: Option<String>,
    pub rewards: Option<serde_json::Value>,
    pub rules: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProgramRequest {
    pub name: Option<String>,
    pub organization: Option<String>,
    pub platform: Option<String>,
    pub platform_handle: Option<String>,
    pub url: Option<String>,
    pub program_type: Option<String>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub rewards: Option<serde_json::Value>,
    pub response_sla_days: Option<i32>,
    pub resolution_sla_days: Option<i32>,
    pub rules: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub priority_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProgramFilter {
    pub platforms: Option<Vec<String>>,
    pub statuses: Option<Vec<String>>,
    pub program_types: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub search: Option<String>,
    pub min_priority: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScopeRequest {
    pub program_id: String,
    pub scope_type: String,
    pub target_type: String,
    pub target: String,
    pub description: Option<String>,
    pub allowed_tests: Option<Vec<String>>,
    pub instructions: Option<Vec<String>>,
    pub requires_auth: Option<bool>,
    pub priority: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScopeRequest {
    pub scope_type: Option<String>,
    pub target_type: Option<String>,
    pub target: Option<String>,
    pub description: Option<String>,
    pub allowed_tests: Option<Vec<String>>,
    pub instructions: Option<Vec<String>>,
    pub requires_auth: Option<bool>,
    pub priority: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScopeFilter {
    pub program_ids: Option<Vec<String>>,
    pub scope_types: Option<Vec<String>>,
    pub target_types: Option<Vec<String>>,
    pub requires_auth: Option<bool>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeValidation {
    pub in_scope: bool,
    pub matched_scope: Option<ProgramScopeRow>,
    pub reason: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramStats {
    pub total_programs: i32,
    pub active_programs: i32,
    pub by_platform: std::collections::HashMap<String, i32>,
    pub by_type: std::collections::HashMap<String, i32>,
    pub total_submissions: i32,
    pub total_accepted: i32,
    pub total_earnings: f64,
}

// ============================================================================
// Program Commands
// ============================================================================

/// Create a new bug bounty program
#[tauri::command]
pub async fn bounty_create_program(
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateProgramRequest,
) -> Result<BountyProgramRow, String> {
    ensure_bounty_feature()?;

    let input = CreateProgramInput {
        name: request.name,
        organization: request.organization,
        platform: request.platform,
        platform_handle: request.platform_handle,
        url: request.url,
        program_type: request.program_type,
        description: request.description,
        rewards: request.rewards,
        rules: request.rules,
        tags: request.tags,
    };
    ProgramDbService::create_program(db_service.inner().as_ref(), input)
        .await
        .map_err(|e| e.to_string())
}

/// Get a program by ID
#[tauri::command]
pub async fn bounty_get_program(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<BountyProgramRow>, String> {
    ensure_bounty_feature()?;

    db_service
        .get_bounty_program(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Update a program
#[tauri::command]
pub async fn bounty_update_program(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: UpdateProgramRequest,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    let input = UpdateProgramInput {
        name: request.name,
        organization: request.organization,
        platform: request.platform,
        platform_handle: request.platform_handle,
        url: request.url,
        program_type: request.program_type,
        status: request.status,
        description: request.description,
        rewards: request.rewards,
        response_sla_days: request.response_sla_days,
        resolution_sla_days: request.resolution_sla_days,
        rules: request.rules,
        tags: request.tags,
        priority_score: request.priority_score,
    };
    ProgramDbService::update_program(db_service.inner().as_ref(), &id, input)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a program
#[tauri::command]
pub async fn bounty_delete_program(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    ProgramDbService::delete_program(db_service.inner().as_ref(), &id)
        .await
        .map_err(|e| e.to_string())
}

/// List programs with optional filter
#[tauri::command]
pub async fn bounty_list_programs(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<ProgramFilter>,
) -> Result<Vec<BountyProgramRow>, String> {
    ensure_bounty_feature()?;

    let filter = filter.unwrap_or_default();

    db_service
        .list_bounty_programs_filtered(ProgramQueryFilter {
            platforms: filter.platforms.as_deref(),
            statuses: filter.statuses.as_deref(),
            program_types: filter.program_types.as_deref(),
            tags: filter.tags.as_deref(),
            search: filter.search.as_deref(),
            min_priority: filter.min_priority,
            limit: None,
            offset: None,
        })
        .await
        .map_err(|e| e.to_string())
}

/// Get program statistics
#[tauri::command]
pub async fn bounty_get_program_stats(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<ProgramStats, String> {
    ensure_bounty_feature()?;

    let stats = db_service
        .get_bounty_program_stats_live()
        .await
        .map_err(|e| e.to_string())?;

    Ok(ProgramStats {
        total_programs: stats.total_programs,
        active_programs: stats.active_programs,
        by_platform: std::collections::HashMap::new(),
        by_type: std::collections::HashMap::new(),
        total_submissions: stats.total_submissions,
        total_accepted: stats.total_accepted,
        total_earnings: stats.total_earnings as f64,
    })
}

// ============================================================================
// Scope Commands
// ============================================================================

/// Create a new scope for a program
#[tauri::command]
pub async fn bounty_create_scope(
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateScopeRequest,
) -> Result<ProgramScopeRow, String> {
    ensure_bounty_feature()?;

    let now = Utc::now().to_rfc3339();

    let scope = ProgramScopeRow {
        id: Uuid::new_v4().to_string(),
        program_id: request.program_id,
        scope_type: request.scope_type,
        target_type: request.target_type,
        target: request.target,
        description: request.description,
        allowed_tests_json: request
            .allowed_tests
            .map(|t| serde_json::to_string(&t).unwrap_or_default()),
        instructions_json: request
            .instructions
            .map(|i| serde_json::to_string(&i).unwrap_or_default()),
        requires_auth: request.requires_auth.unwrap_or(false),
        test_accounts_json: None,
        asset_count: 0,
        finding_count: 0,
        priority: request.priority.unwrap_or(0.0),
        metadata_json: None,
        created_at: now.clone(),
        updated_at: now,
    };

    db_service
        .create_program_scope(&scope)
        .await
        .map_err(|e| e.to_string())?;
    Ok(scope)
}

/// Get a scope by ID
#[tauri::command]
pub async fn bounty_get_scope(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<ProgramScopeRow>, String> {
    ensure_bounty_feature()?;

    db_service
        .get_program_scope(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Update a scope
#[tauri::command]
pub async fn bounty_update_scope(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: UpdateScopeRequest,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    let existing = db_service
        .get_program_scope(&id)
        .await
        .map_err(|e| e.to_string())?;

    let Some(mut scope) = existing else {
        return Ok(false);
    };

    // Apply updates
    if let Some(scope_type) = request.scope_type {
        scope.scope_type = scope_type;
    }
    if let Some(target_type) = request.target_type {
        scope.target_type = target_type;
    }
    if let Some(target) = request.target {
        scope.target = target;
    }
    if request.description.is_some() {
        scope.description = request.description;
    }
    if let Some(allowed_tests) = request.allowed_tests {
        scope.allowed_tests_json = Some(serde_json::to_string(&allowed_tests).unwrap_or_default());
    }
    if let Some(instructions) = request.instructions {
        scope.instructions_json = Some(serde_json::to_string(&instructions).unwrap_or_default());
    }
    if let Some(requires_auth) = request.requires_auth {
        scope.requires_auth = requires_auth;
    }
    if let Some(priority) = request.priority {
        scope.priority = priority;
    }

    scope.updated_at = Utc::now().to_rfc3339();

    db_service
        .update_program_scope(&scope)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a scope
#[tauri::command]
pub async fn bounty_delete_scope(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    db_service
        .delete_program_scope(&id)
        .await
        .map_err(|e| e.to_string())
}

/// List scopes with optional filter
#[tauri::command]
pub async fn bounty_list_scopes(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<ScopeFilter>,
) -> Result<Vec<ProgramScopeRow>, String> {
    ensure_bounty_feature()?;

    let filter = filter.unwrap_or_default();

    let program_id = filter
        .program_ids
        .as_ref()
        .and_then(|ids| ids.first())
        .map(|s| s.as_str());
    let scope_type = filter
        .scope_types
        .as_ref()
        .and_then(|types| types.first())
        .map(|s| s.as_str());

    db_service
        .list_program_scopes(program_id, scope_type)
        .await
        .map_err(|e| e.to_string())
}

/// Validate if a target is in scope for a program
#[tauri::command]
pub async fn bounty_validate_scope(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
    target: String,
) -> Result<ScopeValidation, String> {
    ensure_bounty_feature()?;

    let scopes = db_service
        .list_program_scopes(Some(&program_id), None)
        .await
        .map_err(|e| e.to_string())?;

    // Check out-of-scope first
    for scope in scopes.iter().filter(|s| s.scope_type == "out_of_scope") {
        if target_matches(&scope.target, &scope.target_type, &target) {
            return Ok(ScopeValidation {
                in_scope: false,
                matched_scope: None,
                reason: Some(format!(
                    "Target matches out-of-scope rule: {}",
                    scope.target
                )),
                warnings: vec![],
            });
        }
    }

    // Check in-scope
    for scope in scopes.iter().filter(|s| s.scope_type == "in_scope") {
        if target_matches(&scope.target, &scope.target_type, &target) {
            return Ok(ScopeValidation {
                in_scope: true,
                matched_scope: Some(scope.clone()),
                reason: None,
                warnings: vec![],
            });
        }
    }

    Ok(ScopeValidation {
        in_scope: false,
        matched_scope: None,
        reason: Some("Target does not match any in-scope rule".to_string()),
        warnings: vec![],
    })
}

/// Check if a target matches a scope pattern
fn target_matches(scope_target: &str, target_type: &str, target: &str) -> bool {
    match target_type {
        "domain" => target == scope_target || target.ends_with(&format!(".{}", scope_target)),
        "wildcard_domain" => {
            let base = scope_target.trim_start_matches("*.");
            target.ends_with(base)
        }
        "url" => target.starts_with(scope_target),
        _ => target == scope_target,
    }
}

// ============================================================================
// Finding Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFindingRequest {
    pub program_id: String,
    pub scope_id: Option<String>,
    pub asset_id: Option<String>,
    pub title: String,
    pub description: String,
    pub finding_type: String,
    pub severity: Option<String>,
    pub confidence: Option<String>,
    pub cvss_score: Option<f64>,
    pub cwe_id: Option<String>,
    pub affected_url: Option<String>,
    pub affected_parameter: Option<String>,
    pub reproduction_steps: Option<Vec<String>>,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFindingRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub finding_type: Option<String>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub confidence: Option<String>,
    pub cvss_score: Option<f64>,
    pub cwe_id: Option<String>,
    pub affected_url: Option<String>,
    pub affected_parameter: Option<String>,
    pub reproduction_steps: Option<Vec<String>>,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub tags: Option<Vec<String>>,
    pub duplicate_of: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FindingFilter {
    pub program_id: Option<String>,
    pub scope_id: Option<String>,
    pub severities: Option<Vec<String>>,
    pub statuses: Option<Vec<String>>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

// ============================================================================
// Finding Commands
// ============================================================================

/// Create a new finding
#[tauri::command]
pub async fn bounty_create_finding(
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateFindingRequest,
) -> Result<BountyFindingRow, String> {
    ensure_bounty_feature()?;

    let input = CreateFindingInput {
        program_id: request.program_id,
        scope_id: request.scope_id,
        asset_id: request.asset_id,
        title: request.title,
        description: request.description,
        finding_type: request.finding_type,
        severity: request.severity,
        confidence: request.confidence,
        cvss_score: request.cvss_score,
        cwe_id: request.cwe_id,
        affected_url: request.affected_url,
        affected_parameter: request.affected_parameter,
        reproduction_steps: request.reproduction_steps,
        impact: request.impact,
        remediation: request.remediation,
        tags: request.tags,
    };
    FindingService::create_finding(db_service.inner().as_ref(), input)
        .await
        .map_err(|e| e.to_string())
}

/// Get a finding by ID
#[tauri::command]
pub async fn bounty_get_finding(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<BountyFindingRow>, String> {
    ensure_bounty_feature()?;

    db_service
        .get_bounty_finding(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Update a finding
#[tauri::command]
pub async fn bounty_update_finding(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: UpdateFindingRequest,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    let input = UpdateFindingInput {
        title: request.title,
        description: request.description,
        finding_type: request.finding_type,
        severity: request.severity,
        status: request.status,
        confidence: request.confidence,
        cvss_score: request.cvss_score,
        cwe_id: request.cwe_id,
        affected_url: request.affected_url,
        affected_parameter: request.affected_parameter,
        reproduction_steps: request.reproduction_steps,
        impact: request.impact,
        remediation: request.remediation,
        tags: request.tags,
        duplicate_of: request.duplicate_of,
    };
    FindingService::update_finding(db_service.inner().as_ref(), &id, input)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a finding
#[tauri::command]
pub async fn bounty_delete_finding(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    db_service
        .delete_bounty_finding(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Batch delete findings
#[tauri::command]
pub async fn bounty_batch_delete_findings(
    db_service: State<'_, Arc<DatabaseService>>,
    ids: Vec<String>,
) -> Result<u64, String> {
    ensure_bounty_feature()?;

    FindingService::batch_delete_findings(db_service.inner().as_ref(), ids)
        .await
        .map_err(|e| e.to_string())
}

/// Delete all findings
#[tauri::command]
pub async fn bounty_delete_all_findings(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<u64, String> {
    ensure_bounty_feature()?;

    db_service
        .delete_all_bounty_findings()
        .await
        .map_err(|e| e.to_string())
}

/// Batch update finding status
#[tauri::command]
pub async fn bounty_batch_update_finding_status(
    db_service: State<'_, Arc<DatabaseService>>,
    ids: Vec<String>,
    status: String,
) -> Result<u64, String> {
    ensure_bounty_feature()?;

    FindingService::batch_update_finding_status(db_service.inner().as_ref(), ids, status)
        .await
        .map_err(|e| e.to_string())
}

/// List findings with optional filter
#[tauri::command]
pub async fn bounty_list_findings(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<FindingFilter>,
) -> Result<Vec<BountyFindingRow>, String> {
    ensure_bounty_feature()?;

    let filter = filter.unwrap_or_default();

    db_service
        .list_bounty_findings_filtered(FindingQueryFilter {
            program_id: filter.program_id.as_deref(),
            scope_id: filter.scope_id.as_deref(),
            severities: filter.severities.as_deref(),
            statuses: filter.statuses.as_deref(),
            search: filter.search.as_deref(),
            sort_by: filter.sort_by.as_deref(),
            sort_dir: filter.sort_dir.as_deref(),
            limit: filter.limit,
            offset: filter.offset,
        })
        .await
        .map_err(|e| e.to_string())
}

/// Count findings with optional filter
#[tauri::command]
pub async fn bounty_count_findings(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<FindingFilter>,
) -> Result<i64, String> {
    ensure_bounty_feature()?;

    let filter = filter.unwrap_or_default();

    db_service
        .count_bounty_findings_filtered(FindingQueryFilter {
            program_id: filter.program_id.as_deref(),
            scope_id: filter.scope_id.as_deref(),
            severities: filter.severities.as_deref(),
            statuses: filter.statuses.as_deref(),
            search: filter.search.as_deref(),
            sort_by: None,
            sort_dir: None,
            limit: None,
            offset: None,
        })
        .await
        .map_err(|e| e.to_string())
}

/// Get finding statistics
#[tauri::command]
pub async fn bounty_get_finding_stats(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
) -> Result<BountyFindingStats, String> {
    ensure_bounty_feature()?;

    db_service
        .get_bounty_finding_stats_live(program_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Evidence Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEvidenceRequest {
    pub finding_id: String,
    pub evidence_type: String,
    pub title: String,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub file_url: Option<String>,
    pub content: Option<String>,
    pub mime_type: Option<String>,
    pub http_request: Option<serde_json::Value>,
    pub http_response: Option<serde_json::Value>,
    pub diff: Option<String>,
    pub tags: Option<Vec<String>>,
    pub display_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEvidenceRequest {
    pub evidence_type: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub file_url: Option<String>,
    pub content: Option<String>,
    pub mime_type: Option<String>,
    pub http_request: Option<serde_json::Value>,
    pub http_response: Option<serde_json::Value>,
    pub diff: Option<String>,
    pub tags: Option<Vec<String>>,
    pub display_order: Option<i32>,
}

// ============================================================================
// Evidence Commands
// ============================================================================

/// Create new evidence
#[tauri::command]
pub async fn bounty_create_evidence(
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateEvidenceRequest,
) -> Result<BountyEvidenceRow, String> {
    ensure_bounty_feature()?;

    let now = Utc::now().to_rfc3339();

    let evidence = BountyEvidenceRow {
        id: Uuid::new_v4().to_string(),
        finding_id: request.finding_id,
        evidence_type: request.evidence_type,
        title: request.title,
        description: request.description,
        file_path: request.file_path,
        file_url: request.file_url,
        content: request.content,
        mime_type: request.mime_type,
        file_size: None,
        http_request_json: request
            .http_request
            .map(|r| serde_json::to_string(&r).unwrap_or_default()),
        http_response_json: request
            .http_response
            .map(|r| serde_json::to_string(&r).unwrap_or_default()),
        diff: request.diff,
        tags_json: request
            .tags
            .map(|t| serde_json::to_string(&t).unwrap_or_default()),
        metadata_json: None,
        display_order: request.display_order.unwrap_or(0),
        created_at: now.clone(),
        updated_at: now,
    };

    db_service
        .create_bounty_evidence(&evidence)
        .await
        .map_err(|e| e.to_string())?;
    Ok(evidence)
}

/// Get evidence by ID
#[tauri::command]
pub async fn bounty_get_evidence(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<BountyEvidenceRow>, String> {
    ensure_bounty_feature()?;

    db_service
        .get_bounty_evidence(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Update evidence
#[tauri::command]
pub async fn bounty_update_evidence(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: UpdateEvidenceRequest,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    let existing = db_service
        .get_bounty_evidence(&id)
        .await
        .map_err(|e| e.to_string())?;

    let Some(mut evidence) = existing else {
        return Ok(false);
    };

    if let Some(evidence_type) = request.evidence_type {
        evidence.evidence_type = evidence_type;
    }
    if let Some(title) = request.title {
        evidence.title = title;
    }
    if request.description.is_some() {
        evidence.description = request.description;
    }
    if request.file_path.is_some() {
        evidence.file_path = request.file_path;
    }
    if request.file_url.is_some() {
        evidence.file_url = request.file_url;
    }
    if request.content.is_some() {
        evidence.content = request.content;
    }
    if request.mime_type.is_some() {
        evidence.mime_type = request.mime_type;
    }
    if let Some(req) = request.http_request {
        evidence.http_request_json = Some(serde_json::to_string(&req).unwrap_or_default());
    }
    if let Some(res) = request.http_response {
        evidence.http_response_json = Some(serde_json::to_string(&res).unwrap_or_default());
    }
    if request.diff.is_some() {
        evidence.diff = request.diff;
    }
    if let Some(tags) = request.tags {
        evidence.tags_json = Some(serde_json::to_string(&tags).unwrap_or_default());
    }
    if let Some(order) = request.display_order {
        evidence.display_order = order;
    }

    evidence.updated_at = Utc::now().to_rfc3339();

    db_service
        .update_bounty_evidence(&evidence)
        .await
        .map_err(|e| e.to_string())
}

/// Delete evidence
#[tauri::command]
pub async fn bounty_delete_evidence(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    db_service
        .delete_bounty_evidence(&id)
        .await
        .map_err(|e| e.to_string())
}

/// List evidence for a finding
#[tauri::command]
pub async fn bounty_list_evidence(
    db_service: State<'_, Arc<DatabaseService>>,
    finding_id: String,
) -> Result<Vec<BountyEvidenceRow>, String> {
    ensure_bounty_feature()?;

    db_service
        .list_bounty_evidence(&finding_id)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Submission Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubmissionRequest {
    pub program_id: String,
    pub finding_id: String,
    pub title: String,
    pub vulnerability_type: String,
    pub severity: Option<String>,
    pub cvss_score: Option<f64>,
    pub cwe_id: Option<String>,
    pub description: String,
    pub reproduction_steps: Option<Vec<String>>,
    pub impact: String,
    pub remediation: Option<String>,
    pub evidence_ids: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSubmissionRequest {
    pub platform_submission_id: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub vulnerability_type: Option<String>,
    pub severity: Option<String>,
    pub cvss_score: Option<f64>,
    pub cwe_id: Option<String>,
    pub description: Option<String>,
    pub reproduction_steps: Option<Vec<String>>,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub evidence_ids: Option<Vec<String>>,
    pub platform_url: Option<String>,
    pub reward_amount: Option<f64>,
    pub reward_currency: Option<String>,
    pub bonus_amount: Option<f64>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubmissionFilter {
    pub program_id: Option<String>,
    pub finding_id: Option<String>,
    pub statuses: Option<Vec<String>>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

// ============================================================================
// Submission Commands
// ============================================================================

/// Create a new submission
#[tauri::command]
pub async fn bounty_create_submission(
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateSubmissionRequest,
) -> Result<BountySubmissionRow, String> {
    ensure_bounty_feature()?;

    let input = CreateSubmissionInput {
        program_id: request.program_id,
        finding_id: request.finding_id,
        title: request.title,
        vulnerability_type: request.vulnerability_type,
        severity: request.severity,
        cvss_score: request.cvss_score,
        cwe_id: request.cwe_id,
        description: request.description,
        reproduction_steps: request.reproduction_steps,
        impact: request.impact,
        remediation: request.remediation,
        evidence_ids: request.evidence_ids,
        tags: request.tags,
    };
    SubmissionDbService::create_submission(db_service.inner().as_ref(), input)
        .await
        .map_err(|e| e.to_string())
}

/// Get a submission by ID
#[tauri::command]
pub async fn bounty_get_submission(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<BountySubmissionRow>, String> {
    ensure_bounty_feature()?;

    db_service
        .get_bounty_submission(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Update a submission
#[tauri::command]
pub async fn bounty_update_submission(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: UpdateSubmissionRequest,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    let input = UpdateSubmissionInput {
        platform_submission_id: request.platform_submission_id,
        title: request.title,
        status: request.status,
        priority: request.priority,
        vulnerability_type: request.vulnerability_type,
        severity: request.severity,
        cvss_score: request.cvss_score,
        cwe_id: request.cwe_id,
        description: request.description,
        reproduction_steps: request.reproduction_steps,
        impact: request.impact,
        remediation: request.remediation,
        evidence_ids: request.evidence_ids,
        platform_url: request.platform_url,
        reward_amount: request.reward_amount,
        reward_currency: request.reward_currency,
        bonus_amount: request.bonus_amount,
        tags: request.tags,
    };
    SubmissionDbService::update_submission(db_service.inner().as_ref(), &id, input)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a submission
#[tauri::command]
pub async fn bounty_delete_submission(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    db_service
        .delete_bounty_submission(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Batch delete submissions
#[tauri::command]
pub async fn bounty_batch_delete_submissions(
    db_service: State<'_, Arc<DatabaseService>>,
    ids: Vec<String>,
) -> Result<u64, String> {
    ensure_bounty_feature()?;

    SubmissionDbService::batch_delete_submissions(db_service.inner().as_ref(), ids)
        .await
        .map_err(|e| e.to_string())
}

/// Batch update submission status
#[tauri::command]
pub async fn bounty_batch_update_submission_status(
    db_service: State<'_, Arc<DatabaseService>>,
    ids: Vec<String>,
    status: String,
) -> Result<u64, String> {
    ensure_bounty_feature()?;

    SubmissionDbService::batch_update_submission_status(db_service.inner().as_ref(), ids, status)
        .await
        .map_err(|e| e.to_string())
}

/// List submissions with optional filter
#[tauri::command]
pub async fn bounty_list_submissions(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<SubmissionFilter>,
) -> Result<Vec<BountySubmissionRow>, String> {
    ensure_bounty_feature()?;

    let filter = filter.unwrap_or_default();

    db_service
        .list_bounty_submissions_filtered(SubmissionQueryFilter {
            program_id: filter.program_id.as_deref(),
            finding_id: filter.finding_id.as_deref(),
            statuses: filter.statuses.as_deref(),
            search: filter.search.as_deref(),
            sort_by: filter.sort_by.as_deref(),
            sort_dir: filter.sort_dir.as_deref(),
            limit: filter.limit,
            offset: filter.offset,
        })
        .await
        .map_err(|e| e.to_string())
}

/// Count submissions with optional filter
#[tauri::command]
pub async fn bounty_count_submissions(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<SubmissionFilter>,
) -> Result<i64, String> {
    ensure_bounty_feature()?;

    let filter = filter.unwrap_or_default();

    db_service
        .count_bounty_submissions_filtered(SubmissionQueryFilter {
            program_id: filter.program_id.as_deref(),
            finding_id: filter.finding_id.as_deref(),
            statuses: filter.statuses.as_deref(),
            search: filter.search.as_deref(),
            sort_by: None,
            sort_dir: None,
            limit: None,
            offset: None,
        })
        .await
        .map_err(|e| e.to_string())
}

/// Get submission statistics
#[tauri::command]
pub async fn bounty_get_submission_stats(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
) -> Result<BountySubmissionStats, String> {
    ensure_bounty_feature()?;

    db_service
        .get_bounty_submission_stats_live(program_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Change Event Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChangeEventRequest {
    pub program_id: Option<String>,
    pub asset_id: String,
    pub event_type: String,
    pub severity: Option<String>,
    pub title: String,
    pub description: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub diff: Option<String>,
    pub affected_scope: Option<String>,
    pub detection_method: String,
    pub tags: Option<Vec<String>>,
    pub auto_trigger_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateChangeEventRequest {
    pub status: Option<String>,
    pub severity: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub generated_findings: Option<Vec<String>>,
    pub risk_score: Option<f64>,
    pub auto_trigger_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangeEventFilter {
    pub program_id: Option<String>,
    pub asset_id: Option<String>,
    pub event_types: Option<Vec<String>>,
    pub severities: Option<Vec<String>>,
    pub statuses: Option<Vec<String>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

// ============================================================================
// Change Event Commands
// ============================================================================

/// Create a new change event
#[tauri::command]
pub async fn bounty_create_change_event(
    app_handle: AppHandle,
    plugin_manager: State<'_, Arc<PluginManager>>,
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateChangeEventRequest,
) -> Result<BountyChangeEventRow, String> {
    ensure_bounty_feature()?;

    let now = Utc::now().to_rfc3339();

    // Calculate risk score based on severity and event type
    let severity = request
        .severity
        .clone()
        .unwrap_or_else(|| "medium".to_string());
    let risk_score = calculate_risk_score(&severity, &request.event_type);

    let event = BountyChangeEventRow {
        id: Uuid::new_v4().to_string(),
        program_id: request.program_id,
        asset_id: request.asset_id,
        event_type: request.event_type,
        severity,
        status: "new".to_string(),
        title: request.title,
        description: request.description,
        old_value: request.old_value,
        new_value: request.new_value,
        diff: request.diff,
        affected_scope: request.affected_scope,
        detection_method: request.detection_method,
        generated_findings_json: None,
        tags_json: request
            .tags
            .map(|t| serde_json::to_string(&t).unwrap_or_default()),
        metadata_json: None,
        risk_score,
        auto_trigger_enabled: request.auto_trigger_enabled.unwrap_or(false),
        created_at: now.clone(),
        updated_at: now,
        resolved_at: None,
    };

    db_service
        .create_bounty_change_event(&event)
        .await
        .map_err(|e| e.to_string())?;

    // Auto-trigger workflows if enabled
    if event.auto_trigger_enabled {
        let _ = bounty_trigger_workflows_for_event_internal(
            app_handle,
            (*db_service).clone(),
            (*plugin_manager).clone(),
            event.id.clone(),
        )
        .await;
    }

    Ok(event)
}

/// Helper function to calculate risk score
fn calculate_risk_score(severity: &str, event_type: &str) -> f64 {
    let mut score: f64 = 0.0;

    // Base score from severity
    score += match severity {
        "critical" => 40.0,
        "high" => 30.0,
        "medium" => 20.0,
        "low" => 10.0,
        _ => 15.0,
    };

    // Event type importance
    score += match event_type {
        "asset_discovered" => 20.0,
        "certificate_change" => 15.0,
        "configuration_exposed" => 25.0,
        "api_change" => 15.0,
        "dns_change" => 15.0,
        _ => 10.0,
    };

    // Bonus for high-value changes
    if event_type == "asset_discovered" || event_type == "configuration_exposed" {
        score += 15.0;
    }

    score.min(100.0)
}

/// Get a change event by ID
#[tauri::command]
pub async fn bounty_get_change_event(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<BountyChangeEventRow>, String> {
    ensure_bounty_feature()?;

    db_service
        .get_bounty_change_event(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Update a change event
#[tauri::command]
pub async fn bounty_update_change_event(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: UpdateChangeEventRequest,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    let existing = db_service
        .get_bounty_change_event(&id)
        .await
        .map_err(|e| e.to_string())?;

    let Some(mut event) = existing else {
        return Ok(false);
    };

    // Apply updates
    if let Some(status) = request.status {
        event.status = status.clone();
        // Set resolved_at when status changes to resolved/ignored/acknowledged
        if ["resolved", "ignored", "acknowledged"].contains(&status.as_str())
            && event.resolved_at.is_none()
        {
            event.resolved_at = Some(Utc::now().to_rfc3339());
        }
    }
    if let Some(severity) = request.severity {
        event.severity = severity;
    }
    if let Some(description) = request.description {
        event.description = description;
    }
    if let Some(tags) = request.tags {
        event.tags_json = Some(serde_json::to_string(&tags).unwrap_or_default());
    }
    if let Some(findings) = request.generated_findings {
        event.generated_findings_json = Some(serde_json::to_string(&findings).unwrap_or_default());
    }
    if let Some(score) = request.risk_score {
        event.risk_score = score;
    }
    if let Some(auto_trigger) = request.auto_trigger_enabled {
        event.auto_trigger_enabled = auto_trigger;
    }

    event.updated_at = Utc::now().to_rfc3339();

    db_service
        .update_bounty_change_event(&event)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a change event
#[tauri::command]
pub async fn bounty_delete_change_event(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    db_service
        .delete_bounty_change_event(&id)
        .await
        .map_err(|e| e.to_string())
}

/// List change events with optional filter
#[tauri::command]
pub async fn bounty_list_change_events(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<ChangeEventFilter>,
) -> Result<Vec<BountyChangeEventRow>, String> {
    ensure_bounty_feature()?;

    let filter = filter.unwrap_or_default();

    db_service
        .list_bounty_change_events(
            filter.program_id.as_deref(),
            filter.asset_id.as_deref(),
            filter.event_types.as_deref(),
            filter.severities.as_deref(),
            filter.statuses.as_deref(),
            filter.limit,
            filter.offset,
        )
        .await
        .map_err(|e| e.to_string())
}

/// Get change event statistics
#[tauri::command]
pub async fn bounty_get_change_event_stats(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
) -> Result<BountyChangeEventStats, String> {
    ensure_bounty_feature()?;

    db_service
        .get_bounty_change_event_stats_live(program_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Update change event status
#[tauri::command]
pub async fn bounty_update_change_event_status(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    status: String,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    let resolved_at = if ["resolved", "ignored", "acknowledged"].contains(&status.as_str()) {
        Some(Utc::now().to_rfc3339())
    } else {
        None
    };

    db_service
        .update_bounty_change_event_status(&id, &status, resolved_at.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Batch update change event status.
#[tauri::command]
pub async fn bounty_batch_update_change_event_status(
    db_service: State<'_, Arc<DatabaseService>>,
    ids: Vec<String>,
    status: String,
) -> Result<u64, String> {
    ensure_bounty_feature()?;

    let resolved_at = if ["resolved", "ignored", "acknowledged"].contains(&status.as_str()) {
        Some(Utc::now().to_rfc3339())
    } else {
        None
    };
    let updated_at = Utc::now().to_rfc3339();

    db_service
        .batch_update_bounty_change_event_status(&ids, &status, resolved_at.as_deref(), &updated_at)
        .await
        .map_err(|e| e.to_string())
}

/// Batch delete change events.
#[tauri::command]
pub async fn bounty_batch_delete_change_events(
    db_service: State<'_, Arc<DatabaseService>>,
    ids: Vec<String>,
) -> Result<u64, String> {
    ensure_bounty_feature()?;

    db_service
        .batch_delete_bounty_change_events(&ids)
        .await
        .map_err(|e| e.to_string())
}

/// Add a generated finding to a change event
#[tauri::command]
pub async fn bounty_add_generated_finding(
    db_service: State<'_, Arc<DatabaseService>>,
    event_id: String,
    finding_id: String,
) -> Result<bool, String> {
    ensure_bounty_feature()?;

    db_service
        .add_generated_finding_to_change_event(&event_id, &finding_id)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Traffic Integration - Auto Evidence Generation
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTrafficFindingRequest {
    pub traffic_vuln_id: String,
    pub program_id: String,
    pub scope_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTrafficFindingResponse {
    pub finding_id: String,
    pub evidence_id: String,
}

/// Import a traffic vulnerability as a bounty finding with auto-generated evidence
#[tauri::command]
pub async fn bounty_import_traffic_finding(
    db_service: State<'_, Arc<DatabaseService>>,
    request: ImportTrafficFindingRequest,
) -> Result<ImportTrafficFindingResponse, String> {
    ensure_bounty_feature()?;

    let now = Utc::now().to_rfc3339();

    // Get traffic vulnerability with evidence
    let traffic_vuln = db_service
        .get_traffic_vulnerability_by_id(&request.traffic_vuln_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Traffic vulnerability not found".to_string())?;

    let traffic_evidence = db_service
        .get_traffic_evidence_by_vuln_id(&request.traffic_vuln_id)
        .await
        .map_err(|e| e.to_string())?;

    // Create bounty finding
    let finding_id = Uuid::new_v4().to_string();
    let fingerprint = format!(
        "{}:{}:{}",
        request.program_id,
        traffic_vuln.vuln_type,
        traffic_evidence
            .first()
            .map(|e| e.url.as_str())
            .unwrap_or("")
    );
    let fingerprint = format!("{:x}", md5::compute(fingerprint.as_bytes()));

    // Check for duplicate
    if let Some(_existing) = db_service
        .get_bounty_finding_by_fingerprint(&fingerprint)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err("A similar finding already exists for this program".to_string());
    }

    let affected_url = traffic_evidence.first().map(|e| e.url.clone());

    let finding = BountyFindingRow {
        id: finding_id.clone(),
        program_id: request.program_id,
        scope_id: request.scope_id,
        asset_id: None,
        title: traffic_vuln.title.clone(),
        description: traffic_vuln.description.clone(),
        finding_type: traffic_vuln.vuln_type.clone(),
        severity: traffic_vuln.severity.to_lowercase(),
        status: "new".to_string(),
        confidence: traffic_vuln.confidence.to_lowercase(),
        cvss_score: None,
        cwe_id: traffic_vuln.cwe.clone(),
        affected_url,
        affected_parameter: traffic_evidence.first().map(|e| e.location.clone()),
        reproduction_steps_json: None,
        impact: None,
        remediation: traffic_vuln.remediation.clone(),
        evidence_ids_json: None,
        tags_json: Some(
            serde_json::to_string(&vec!["traffic", "auto-imported"]).unwrap_or_default(),
        ),
        metadata_json: Some(
            serde_json::to_string(&serde_json::json!({
                "source": "traffic_analysis",
                "traffic_vuln_id": request.traffic_vuln_id,
                "plugin_id": traffic_vuln.plugin_id,
                "original_signature": traffic_vuln.signature,
            }))
            .unwrap_or_default(),
        ),
        fingerprint,
        duplicate_of: None,
        first_seen_at: traffic_vuln.first_seen_at.to_rfc3339(),
        last_seen_at: now.clone(),
        verified_at: None,
        created_at: now.clone(),
        updated_at: now.clone(),
        created_by: "traffic_import".to_string(),
    };

    db_service
        .create_bounty_finding(&finding)
        .await
        .map_err(|e| e.to_string())?;

    // Create evidence from traffic evidence
    let evidence_id = Uuid::new_v4().to_string();
    let first_traffic_evidence = traffic_evidence.first();

    let evidence = BountyEvidenceRow {
        id: evidence_id.clone(),
        finding_id: finding_id.clone(),
        evidence_type: "http_transaction".to_string(),
        title: format!("{} - HTTP Evidence", traffic_vuln.title),
        description: Some(format!(
            "Auto-imported from traffic analysis (plugin: {})",
            traffic_vuln.plugin_id
        )),
        file_path: None,
        file_url: None,
        content: first_traffic_evidence.map(|e| e.evidence_snippet.clone()),
        mime_type: Some("text/plain".to_string()),
        file_size: None,
        http_request_json: first_traffic_evidence.map(|e| {
            serde_json::to_string(&serde_json::json!({
                "method": e.method,
                "url": e.url,
                "headers": e.request_headers,
                "body": e.request_body,
            }))
            .unwrap_or_default()
        }),
        http_response_json: first_traffic_evidence.map(|e| {
            serde_json::to_string(&serde_json::json!({
                "status_code": e.response_status,
                "headers": e.response_headers,
                "body": e.response_body,
            }))
            .unwrap_or_default()
        }),
        diff: None,
        tags_json: Some(
            serde_json::to_string(&vec!["auto-generated", "traffic"]).unwrap_or_default(),
        ),
        metadata_json: Some(
            serde_json::to_string(&serde_json::json!({
                "traffic_evidence_id": first_traffic_evidence.map(|e| &e.id),
                "traffic_vuln_id": request.traffic_vuln_id,
            }))
            .unwrap_or_default(),
        ),
        display_order: 0,
        created_at: now.clone(),
        updated_at: now,
    };

    db_service
        .create_bounty_evidence(&evidence)
        .await
        .map_err(|e| e.to_string())?;

    // Update finding with evidence ID
    let mut updated_finding = finding;
    updated_finding.evidence_ids_json =
        Some(serde_json::to_string(&vec![&evidence_id]).unwrap_or_default());
    db_service
        .update_bounty_finding(&updated_finding)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ImportTrafficFindingResponse {
        finding_id,
        evidence_id,
    })
}

/// Batch import traffic vulnerabilities as bounty findings
#[tauri::command]
pub async fn bounty_batch_import_traffic_findings(
    db_service: State<'_, Arc<DatabaseService>>,
    traffic_vuln_ids: Vec<String>,
    program_id: String,
    scope_id: Option<String>,
) -> Result<Vec<ImportTrafficFindingResponse>, String> {
    ensure_bounty_feature()?;

    let mut results = Vec::new();

    for vuln_id in traffic_vuln_ids {
        match bounty_import_traffic_finding_internal(
            &db_service,
            &vuln_id,
            &program_id,
            scope_id.as_deref(),
        )
        .await
        {
            Ok(response) => results.push(response),
            Err(e) => {
                tracing::warn!("Failed to import traffic finding {}: {}", vuln_id, e);
            }
        }
    }

    Ok(results)
}

/// Internal function for importing traffic finding (used by batch import)
async fn bounty_import_traffic_finding_internal(
    db_service: &Arc<DatabaseService>,
    traffic_vuln_id: &str,
    program_id: &str,
    scope_id: Option<&str>,
) -> Result<ImportTrafficFindingResponse, String> {
    let now = Utc::now().to_rfc3339();

    let traffic_vuln = db_service
        .get_traffic_vulnerability_by_id(traffic_vuln_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Traffic vulnerability not found".to_string())?;

    let traffic_evidence = db_service
        .get_traffic_evidence_by_vuln_id(traffic_vuln_id)
        .await
        .map_err(|e| e.to_string())?;

    let finding_id = Uuid::new_v4().to_string();
    let fingerprint = format!(
        "{}:{}:{}",
        program_id,
        traffic_vuln.vuln_type,
        traffic_evidence
            .first()
            .map(|e| e.url.as_str())
            .unwrap_or("")
    );
    let fingerprint = format!("{:x}", md5::compute(fingerprint.as_bytes()));

    if let Some(_existing) = db_service
        .get_bounty_finding_by_fingerprint(&fingerprint)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err("Duplicate finding".to_string());
    }

    let affected_url = traffic_evidence.first().map(|e| e.url.clone());

    let finding = BountyFindingRow {
        id: finding_id.clone(),
        program_id: program_id.to_string(),
        scope_id: scope_id.map(|s| s.to_string()),
        asset_id: None,
        title: traffic_vuln.title.clone(),
        description: traffic_vuln.description.clone(),
        finding_type: traffic_vuln.vuln_type.clone(),
        severity: traffic_vuln.severity.to_lowercase(),
        status: "new".to_string(),
        confidence: traffic_vuln.confidence.to_lowercase(),
        cvss_score: None,
        cwe_id: traffic_vuln.cwe.clone(),
        affected_url,
        affected_parameter: traffic_evidence.first().map(|e| e.location.clone()),
        reproduction_steps_json: None,
        impact: None,
        remediation: traffic_vuln.remediation.clone(),
        evidence_ids_json: None,
        tags_json: Some(
            serde_json::to_string(&vec!["traffic", "auto-imported"]).unwrap_or_default(),
        ),
        metadata_json: Some(
            serde_json::to_string(&serde_json::json!({
                "source": "traffic_analysis",
                "traffic_vuln_id": traffic_vuln_id,
                "plugin_id": traffic_vuln.plugin_id,
            }))
            .unwrap_or_default(),
        ),
        fingerprint,
        duplicate_of: None,
        first_seen_at: traffic_vuln.first_seen_at.to_rfc3339(),
        last_seen_at: now.clone(),
        verified_at: None,
        created_at: now.clone(),
        updated_at: now.clone(),
        created_by: "traffic_import".to_string(),
    };

    db_service
        .create_bounty_finding(&finding)
        .await
        .map_err(|e| e.to_string())?;

    let evidence_id = Uuid::new_v4().to_string();
    let first_traffic_evidence = traffic_evidence.first();

    let evidence = BountyEvidenceRow {
        id: evidence_id.clone(),
        finding_id: finding_id.clone(),
        evidence_type: "http_transaction".to_string(),
        title: format!("{} - HTTP Evidence", traffic_vuln.title),
        description: Some(format!("Auto-imported from traffic analysis")),
        file_path: None,
        file_url: None,
        content: first_traffic_evidence.map(|e| e.evidence_snippet.clone()),
        mime_type: Some("text/plain".to_string()),
        file_size: None,
        http_request_json: first_traffic_evidence.map(|e| {
            serde_json::to_string(&serde_json::json!({
                "method": e.method,
                "url": e.url,
                "headers": e.request_headers,
                "body": e.request_body,
            }))
            .unwrap_or_default()
        }),
        http_response_json: first_traffic_evidence.map(|e| {
            serde_json::to_string(&serde_json::json!({
                "status_code": e.response_status,
                "headers": e.response_headers,
                "body": e.response_body,
            }))
            .unwrap_or_default()
        }),
        diff: None,
        tags_json: Some(serde_json::to_string(&vec!["auto-generated"]).unwrap_or_default()),
        metadata_json: None,
        display_order: 0,
        created_at: now.clone(),
        updated_at: now,
    };

    db_service
        .create_bounty_evidence(&evidence)
        .await
        .map_err(|e| e.to_string())?;

    let mut updated_finding = finding;
    updated_finding.evidence_ids_json =
        Some(serde_json::to_string(&vec![&evidence_id]).unwrap_or_default());
    db_service
        .update_bounty_finding(&updated_finding)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ImportTrafficFindingResponse {
        finding_id,
        evidence_id,
    })
}

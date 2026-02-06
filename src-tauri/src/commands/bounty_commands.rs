//! Bug Bounty commands for Tauri

use sentinel_db::{
    Database, DatabaseService, BountyProgramRow, ProgramScopeRow,
    BountyFindingRow, BountyEvidenceRow, BountySubmissionRow,
    BountyFindingStats, BountySubmissionStats,
    BountyChangeEventRow, BountyChangeEventStats,
    BountyWorkflowTemplateRow, BountyWorkflowBindingRow,
    BountyAssetRow, BountyAssetStats,
};
use sentinel_bounty::services::{
    FindingService, CreateFindingInput, UpdateFindingInput,
    ProgramDbService, CreateProgramInput, UpdateProgramInput,
    SubmissionDbService, CreateSubmissionInput, UpdateSubmissionInput,
};
use sentinel_traffic::PluginManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, State, Emitter};
use chrono::Utc;
use uuid::Uuid;


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
    ProgramDbService::create_program(db_service.inner().as_ref(), input).await.map_err(|e| e.to_string())
}

/// Get a program by ID
#[tauri::command]
pub async fn bounty_get_program(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<BountyProgramRow>, String> {
    db_service.get_bounty_program(&id).await.map_err(|e| e.to_string())
}

/// Update a program
#[tauri::command]
pub async fn bounty_update_program(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: UpdateProgramRequest,
) -> Result<bool, String> {
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
    ProgramDbService::update_program(db_service.inner().as_ref(), &id, input).await.map_err(|e| e.to_string())
}

/// Delete a program
#[tauri::command]
pub async fn bounty_delete_program(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    ProgramDbService::delete_program(db_service.inner().as_ref(), &id).await.map_err(|e| e.to_string())
}

/// List programs with optional filter
#[tauri::command]
pub async fn bounty_list_programs(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<ProgramFilter>,
) -> Result<Vec<BountyProgramRow>, String> {
    let filter = filter.unwrap_or_default();
    
    db_service.list_bounty_programs(
        filter.platforms.as_deref(),
        filter.statuses.as_deref(),
        filter.search.as_deref(),
        None,
        None,
    ).await.map_err(|e| e.to_string())
}

/// Get program statistics
#[tauri::command]
pub async fn bounty_get_program_stats(
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<ProgramStats, String> {
    let stats = db_service.get_bounty_program_stats().await.map_err(|e| e.to_string())?;
    
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
    let now = Utc::now().to_rfc3339();
    
    let scope = ProgramScopeRow {
        id: Uuid::new_v4().to_string(),
        program_id: request.program_id,
        scope_type: request.scope_type,
        target_type: request.target_type,
        target: request.target,
        description: request.description,
        allowed_tests_json: request.allowed_tests.map(|t| serde_json::to_string(&t).unwrap_or_default()),
        instructions_json: request.instructions.map(|i| serde_json::to_string(&i).unwrap_or_default()),
        requires_auth: request.requires_auth.unwrap_or(false),
        test_accounts_json: None,
        asset_count: 0,
        finding_count: 0,
        priority: request.priority.unwrap_or(0.0),
        metadata_json: None,
        created_at: now.clone(),
        updated_at: now,
    };

    db_service.create_program_scope(&scope).await.map_err(|e| e.to_string())?;
    Ok(scope)
}

/// Get a scope by ID
#[tauri::command]
pub async fn bounty_get_scope(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<ProgramScopeRow>, String> {
    db_service.get_program_scope(&id).await.map_err(|e| e.to_string())
}

/// Update a scope
#[tauri::command]
pub async fn bounty_update_scope(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: UpdateScopeRequest,
) -> Result<bool, String> {
    let existing = db_service.get_program_scope(&id).await.map_err(|e| e.to_string())?;
    
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

    db_service.update_program_scope(&scope).await.map_err(|e| e.to_string())
}

/// Delete a scope
#[tauri::command]
pub async fn bounty_delete_scope(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    db_service.delete_program_scope(&id).await.map_err(|e| e.to_string())
}

/// List scopes with optional filter
#[tauri::command]
pub async fn bounty_list_scopes(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<ScopeFilter>,
) -> Result<Vec<ProgramScopeRow>, String> {
    let filter = filter.unwrap_or_default();
    
    let program_id = filter.program_ids.as_ref().and_then(|ids| ids.first()).map(|s| s.as_str());
    let scope_type = filter.scope_types.as_ref().and_then(|types| types.first()).map(|s| s.as_str());
    
    db_service.list_program_scopes(program_id, scope_type).await.map_err(|e| e.to_string())
}

/// Validate if a target is in scope for a program
#[tauri::command]
pub async fn bounty_validate_scope(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
    target: String,
) -> Result<ScopeValidation, String> {
    let scopes = db_service.list_program_scopes(Some(&program_id), None)
        .await
        .map_err(|e| e.to_string())?;
    
    // Check out-of-scope first
    for scope in scopes.iter().filter(|s| s.scope_type == "out_of_scope") {
        if target_matches(&scope.target, &scope.target_type, &target) {
            return Ok(ScopeValidation {
                in_scope: false,
                matched_scope: None,
                reason: Some(format!("Target matches out-of-scope rule: {}", scope.target)),
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
    FindingService::create_finding(db_service.inner().as_ref(), input).await.map_err(|e| e.to_string())
}

/// Get a finding by ID
#[tauri::command]
pub async fn bounty_get_finding(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<BountyFindingRow>, String> {
    db_service.get_bounty_finding(&id).await.map_err(|e| e.to_string())
}

/// Update a finding
#[tauri::command]
pub async fn bounty_update_finding(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: UpdateFindingRequest,
) -> Result<bool, String> {
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
    FindingService::update_finding(db_service.inner().as_ref(), &id, input).await.map_err(|e| e.to_string())
}

/// Delete a finding
#[tauri::command]
pub async fn bounty_delete_finding(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    db_service.delete_bounty_finding(&id).await.map_err(|e| e.to_string())
}

/// List findings with optional filter
#[tauri::command]
pub async fn bounty_list_findings(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<FindingFilter>,
) -> Result<Vec<BountyFindingRow>, String> {
    let filter = filter.unwrap_or_default();
    
    db_service.list_bounty_findings(
        filter.program_id.as_deref(),
        filter.scope_id.as_deref(),
        filter.severities.as_deref(),
        filter.statuses.as_deref(),
        filter.search.as_deref(),
        filter.sort_by.as_deref(),
        filter.sort_dir.as_deref(),
        filter.limit,
        filter.offset,
    ).await.map_err(|e| e.to_string())
}

/// Get finding statistics
#[tauri::command]
pub async fn bounty_get_finding_stats(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
) -> Result<BountyFindingStats, String> {
    db_service.get_bounty_finding_stats(program_id.as_deref()).await.map_err(|e| e.to_string())
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
        http_request_json: request.http_request.map(|r| serde_json::to_string(&r).unwrap_or_default()),
        http_response_json: request.http_response.map(|r| serde_json::to_string(&r).unwrap_or_default()),
        diff: request.diff,
        tags_json: request.tags.map(|t| serde_json::to_string(&t).unwrap_or_default()),
        metadata_json: None,
        display_order: request.display_order.unwrap_or(0),
        created_at: now.clone(),
        updated_at: now,
    };

    db_service.create_bounty_evidence(&evidence).await.map_err(|e| e.to_string())?;
    Ok(evidence)
}

/// Get evidence by ID
#[tauri::command]
pub async fn bounty_get_evidence(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<BountyEvidenceRow>, String> {
    db_service.get_bounty_evidence(&id).await.map_err(|e| e.to_string())
}

/// Update evidence
#[tauri::command]
pub async fn bounty_update_evidence(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: UpdateEvidenceRequest,
) -> Result<bool, String> {
    let existing = db_service.get_bounty_evidence(&id).await.map_err(|e| e.to_string())?;
    
    let Some(mut evidence) = existing else {
        return Ok(false);
    };

    if let Some(evidence_type) = request.evidence_type { evidence.evidence_type = evidence_type; }
    if let Some(title) = request.title { evidence.title = title; }
    if request.description.is_some() { evidence.description = request.description; }
    if request.file_path.is_some() { evidence.file_path = request.file_path; }
    if request.file_url.is_some() { evidence.file_url = request.file_url; }
    if request.content.is_some() { evidence.content = request.content; }
    if request.mime_type.is_some() { evidence.mime_type = request.mime_type; }
    if let Some(req) = request.http_request {
        evidence.http_request_json = Some(serde_json::to_string(&req).unwrap_or_default());
    }
    if let Some(res) = request.http_response {
        evidence.http_response_json = Some(serde_json::to_string(&res).unwrap_or_default());
    }
    if request.diff.is_some() { evidence.diff = request.diff; }
    if let Some(tags) = request.tags {
        evidence.tags_json = Some(serde_json::to_string(&tags).unwrap_or_default());
    }
    if let Some(order) = request.display_order { evidence.display_order = order; }

    evidence.updated_at = Utc::now().to_rfc3339();

    db_service.update_bounty_evidence(&evidence).await.map_err(|e| e.to_string())
}

/// Delete evidence
#[tauri::command]
pub async fn bounty_delete_evidence(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    db_service.delete_bounty_evidence(&id).await.map_err(|e| e.to_string())
}

/// List evidence for a finding
#[tauri::command]
pub async fn bounty_list_evidence(
    db_service: State<'_, Arc<DatabaseService>>,
    finding_id: String,
) -> Result<Vec<BountyEvidenceRow>, String> {
    db_service.list_bounty_evidence(&finding_id).await.map_err(|e| e.to_string())
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
    SubmissionDbService::create_submission(db_service.inner().as_ref(), input).await.map_err(|e| e.to_string())
}

/// Get a submission by ID
#[tauri::command]
pub async fn bounty_get_submission(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<BountySubmissionRow>, String> {
    db_service.get_bounty_submission(&id).await.map_err(|e| e.to_string())
}

/// Update a submission
#[tauri::command]
pub async fn bounty_update_submission(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: UpdateSubmissionRequest,
) -> Result<bool, String> {
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
    SubmissionDbService::update_submission(db_service.inner().as_ref(), &id, input).await.map_err(|e| e.to_string())
}

/// Delete a submission
#[tauri::command]
pub async fn bounty_delete_submission(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    db_service.delete_bounty_submission(&id).await.map_err(|e| e.to_string())
}

/// List submissions with optional filter
#[tauri::command]
pub async fn bounty_list_submissions(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<SubmissionFilter>,
) -> Result<Vec<BountySubmissionRow>, String> {
    let filter = filter.unwrap_or_default();
    
    db_service.list_bounty_submissions(
        filter.program_id.as_deref(),
        filter.finding_id.as_deref(),
        filter.statuses.as_deref(),
        filter.search.as_deref(),
        filter.sort_by.as_deref(),
        filter.sort_dir.as_deref(),
        filter.limit,
        filter.offset,
    ).await.map_err(|e| e.to_string())
}

/// Get submission statistics
#[tauri::command]
pub async fn bounty_get_submission_stats(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
) -> Result<BountySubmissionStats, String> {
    db_service.get_bounty_submission_stats(program_id.as_deref()).await.map_err(|e| e.to_string())
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
    pub triggered_workflows: Option<Vec<String>>,
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
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateChangeEventRequest,
) -> Result<BountyChangeEventRow, String> {
    let now = Utc::now().to_rfc3339();
    
    // Calculate risk score based on severity and event type
    let severity = request.severity.clone().unwrap_or_else(|| "medium".to_string());
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
        triggered_workflows_json: None,
        generated_findings_json: None,
        tags_json: request.tags.map(|t| serde_json::to_string(&t).unwrap_or_default()),
        metadata_json: None,
        risk_score,
        auto_trigger_enabled: request.auto_trigger_enabled.unwrap_or(false),
        created_at: now.clone(),
        updated_at: now,
        resolved_at: None,
    };

    db_service.create_bounty_change_event(&event).await.map_err(|e| e.to_string())?;
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
    db_service.get_bounty_change_event(&id).await.map_err(|e| e.to_string())
}

/// Update a change event
#[tauri::command]
pub async fn bounty_update_change_event(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: UpdateChangeEventRequest,
) -> Result<bool, String> {
    let existing = db_service.get_bounty_change_event(&id).await.map_err(|e| e.to_string())?;
    
    let Some(mut event) = existing else {
        return Ok(false);
    };

    // Apply updates
    if let Some(status) = request.status {
        event.status = status.clone();
        // Set resolved_at when status changes to resolved/ignored/acknowledged
        if ["resolved", "ignored", "acknowledged"].contains(&status.as_str()) && event.resolved_at.is_none() {
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
    if let Some(workflows) = request.triggered_workflows {
        event.triggered_workflows_json = Some(serde_json::to_string(&workflows).unwrap_or_default());
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

    db_service.update_bounty_change_event(&event).await.map_err(|e| e.to_string())
}

/// Delete a change event
#[tauri::command]
pub async fn bounty_delete_change_event(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    db_service.delete_bounty_change_event(&id).await.map_err(|e| e.to_string())
}

/// List change events with optional filter
#[tauri::command]
pub async fn bounty_list_change_events(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: Option<ChangeEventFilter>,
) -> Result<Vec<BountyChangeEventRow>, String> {
    let filter = filter.unwrap_or_default();
    
    db_service.list_bounty_change_events(
        filter.program_id.as_deref(),
        filter.asset_id.as_deref(),
        filter.event_types.as_deref(),
        filter.severities.as_deref(),
        filter.statuses.as_deref(),
        filter.limit,
        filter.offset,
    ).await.map_err(|e| e.to_string())
}

/// Get change event statistics
#[tauri::command]
pub async fn bounty_get_change_event_stats(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
) -> Result<BountyChangeEventStats, String> {
    db_service.get_bounty_change_event_stats(program_id.as_deref()).await.map_err(|e| e.to_string())
}

/// Update change event status
#[tauri::command]
pub async fn bounty_update_change_event_status(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    status: String,
) -> Result<bool, String> {
    let resolved_at = if ["resolved", "ignored", "acknowledged"].contains(&status.as_str()) {
        Some(Utc::now().to_rfc3339())
    } else {
        None
    };
    
    db_service.update_bounty_change_event_status(&id, &status, resolved_at.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Add a triggered workflow to a change event
#[tauri::command]
pub async fn bounty_add_triggered_workflow(
    db_service: State<'_, Arc<DatabaseService>>,
    event_id: String,
    workflow_id: String,
) -> Result<bool, String> {
    db_service.add_triggered_workflow_to_change_event(&event_id, &workflow_id)
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
    db_service.add_generated_finding_to_change_event(&event_id, &finding_id)
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
    let now = Utc::now().to_rfc3339();
    
    // Get traffic vulnerability with evidence
    let traffic_vuln = db_service.get_traffic_vulnerability_by_id(&request.traffic_vuln_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Traffic vulnerability not found".to_string())?;
    
    let traffic_evidence = db_service.get_traffic_evidence_by_vuln_id(&request.traffic_vuln_id)
        .await
        .map_err(|e| e.to_string())?;
    
    // Create bounty finding
    let finding_id = Uuid::new_v4().to_string();
    let fingerprint = format!(
        "{}:{}:{}",
        request.program_id,
        traffic_vuln.vuln_type,
        traffic_evidence.first().map(|e| e.url.as_str()).unwrap_or("")
    );
    let fingerprint = format!("{:x}", md5::compute(fingerprint.as_bytes()));
    
    // Check for duplicate
    if let Some(_existing) = db_service.get_bounty_finding_by_fingerprint(&fingerprint)
        .await.map_err(|e| e.to_string())? {
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
        tags_json: Some(serde_json::to_string(&vec!["traffic", "auto-imported"]).unwrap_or_default()),
        metadata_json: Some(serde_json::to_string(&serde_json::json!({
            "source": "traffic_analysis",
            "traffic_vuln_id": request.traffic_vuln_id,
            "plugin_id": traffic_vuln.plugin_id,
            "original_signature": traffic_vuln.signature,
        })).unwrap_or_default()),
        fingerprint,
        duplicate_of: None,
        first_seen_at: traffic_vuln.first_seen_at.to_rfc3339(),
        last_seen_at: now.clone(),
        verified_at: None,
        created_at: now.clone(),
        updated_at: now.clone(),
        created_by: "traffic_import".to_string(),
    };
    
    db_service.create_bounty_finding(&finding).await.map_err(|e| e.to_string())?;
    
    // Create evidence from traffic evidence
    let evidence_id = Uuid::new_v4().to_string();
    let first_traffic_evidence = traffic_evidence.first();
    
    let evidence = BountyEvidenceRow {
        id: evidence_id.clone(),
        finding_id: finding_id.clone(),
        evidence_type: "http_transaction".to_string(),
        title: format!("{} - HTTP Evidence", traffic_vuln.title),
        description: Some(format!("Auto-imported from traffic analysis (plugin: {})", traffic_vuln.plugin_id)),
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
            })).unwrap_or_default()
        }),
        http_response_json: first_traffic_evidence.map(|e| {
            serde_json::to_string(&serde_json::json!({
                "status_code": e.response_status,
                "headers": e.response_headers,
                "body": e.response_body,
            })).unwrap_or_default()
        }),
        diff: None,
        tags_json: Some(serde_json::to_string(&vec!["auto-generated", "traffic"]).unwrap_or_default()),
        metadata_json: Some(serde_json::to_string(&serde_json::json!({
            "traffic_evidence_id": first_traffic_evidence.map(|e| &e.id),
            "traffic_vuln_id": request.traffic_vuln_id,
        })).unwrap_or_default()),
        display_order: 0,
        created_at: now.clone(),
        updated_at: now,
    };
    
    db_service.create_bounty_evidence(&evidence).await.map_err(|e| e.to_string())?;
    
    // Update finding with evidence ID
    let mut updated_finding = finding;
    updated_finding.evidence_ids_json = Some(serde_json::to_string(&vec![&evidence_id]).unwrap_or_default());
    db_service.update_bounty_finding(&updated_finding).await.map_err(|e| e.to_string())?;
    
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
    let mut results = Vec::new();
    
    for vuln_id in traffic_vuln_ids {
        match bounty_import_traffic_finding_internal(
            &db_service,
            &vuln_id,
            &program_id,
            scope_id.as_deref(),
        ).await {
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
    
    let traffic_vuln = db_service.get_traffic_vulnerability_by_id(traffic_vuln_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Traffic vulnerability not found".to_string())?;
    
    let traffic_evidence = db_service.get_traffic_evidence_by_vuln_id(traffic_vuln_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let finding_id = Uuid::new_v4().to_string();
    let fingerprint = format!(
        "{}:{}:{}",
        program_id,
        traffic_vuln.vuln_type,
        traffic_evidence.first().map(|e| e.url.as_str()).unwrap_or("")
    );
    let fingerprint = format!("{:x}", md5::compute(fingerprint.as_bytes()));
    
    if let Some(_existing) = db_service.get_bounty_finding_by_fingerprint(&fingerprint)
        .await.map_err(|e| e.to_string())? {
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
        tags_json: Some(serde_json::to_string(&vec!["traffic", "auto-imported"]).unwrap_or_default()),
        metadata_json: Some(serde_json::to_string(&serde_json::json!({
            "source": "traffic_analysis",
            "traffic_vuln_id": traffic_vuln_id,
            "plugin_id": traffic_vuln.plugin_id,
        })).unwrap_or_default()),
        fingerprint,
        duplicate_of: None,
        first_seen_at: traffic_vuln.first_seen_at.to_rfc3339(),
        last_seen_at: now.clone(),
        verified_at: None,
        created_at: now.clone(),
        updated_at: now.clone(),
        created_by: "traffic_import".to_string(),
    };
    
    db_service.create_bounty_finding(&finding).await.map_err(|e| e.to_string())?;
    
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
            })).unwrap_or_default()
        }),
        http_response_json: first_traffic_evidence.map(|e| {
            serde_json::to_string(&serde_json::json!({
                "status_code": e.response_status,
                "headers": e.response_headers,
                "body": e.response_body,
            })).unwrap_or_default()
        }),
        diff: None,
        tags_json: Some(serde_json::to_string(&vec!["auto-generated"]).unwrap_or_default()),
        metadata_json: None,
        display_order: 0,
        created_at: now.clone(),
        updated_at: now,
    };
    
    db_service.create_bounty_evidence(&evidence).await.map_err(|e| e.to_string())?;
    
    let mut updated_finding = finding;
    updated_finding.evidence_ids_json = Some(serde_json::to_string(&vec![&evidence_id]).unwrap_or_default());
    db_service.update_bounty_finding(&updated_finding).await.map_err(|e| e.to_string())?;
    
    Ok(ImportTrafficFindingResponse {
        finding_id,
        evidence_id,
    })
}

// ============================================================================
// Export Report - One-click Submission Package
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReportRequest {
    pub finding_ids: Vec<String>,
    pub format: String, // "markdown", "json", "html"
    pub language: String, // "en", "zh"
    pub include_evidence: bool,
    pub template_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReportResponse {
    pub filename: String,
    pub content: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingExportData {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub finding_type: String,
    pub affected_url: Option<String>,
    pub affected_parameter: Option<String>,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub reproduction_steps: Option<Vec<String>>,
    pub evidence: Vec<EvidenceExportData>,
    pub cwe_id: Option<String>,
    pub cvss_score: Option<f32>,
    pub tags: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceExportData {
    pub id: String,
    pub evidence_type: String,
    pub title: String,
    pub description: Option<String>,
    pub content: Option<String>,
    pub http_request: Option<serde_json::Value>,
    pub http_response: Option<serde_json::Value>,
}

/// Export findings as a submission report
#[tauri::command]
pub async fn bounty_export_report(
    db_service: State<'_, Arc<DatabaseService>>,
    request: ExportReportRequest,
) -> Result<ExportReportResponse, String> {
    let mut findings_data: Vec<FindingExportData> = Vec::new();
    
    for finding_id in &request.finding_ids {
        let finding = db_service.get_bounty_finding(finding_id)
            .await.map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Finding {} not found", finding_id))?;
        
        let mut evidence_data = Vec::new();
        if request.include_evidence {
            let evidences = db_service.list_bounty_evidence(finding_id)
                .await.map_err(|e| e.to_string())?;
            
            for ev in evidences {
                evidence_data.push(EvidenceExportData {
                    id: ev.id,
                    evidence_type: ev.evidence_type,
                    title: ev.title,
                    description: ev.description,
                    content: ev.content,
                    http_request: ev.http_request_json.and_then(|s| serde_json::from_str(&s).ok()),
                    http_response: ev.http_response_json.and_then(|s| serde_json::from_str(&s).ok()),
                });
            }
        }
        
        let reproduction_steps: Option<Vec<String>> = finding.reproduction_steps_json
            .and_then(|s| serde_json::from_str(&s).ok());
        
        let tags: Vec<String> = finding.tags_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        
        findings_data.push(FindingExportData {
            id: finding.id,
            title: finding.title,
            description: finding.description,
            severity: finding.severity,
            finding_type: finding.finding_type,
            affected_url: finding.affected_url,
            affected_parameter: finding.affected_parameter,
            impact: finding.impact,
            remediation: finding.remediation,
            reproduction_steps,
            evidence: evidence_data,
            cwe_id: finding.cwe_id,
            cvss_score: finding.cvss_score.map(|s| s as f32),
            tags,
            created_at: finding.created_at,
        });
    }
    
    let (content, mime_type, ext) = match request.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&findings_data).map_err(|e| e.to_string())?;
            (json, "application/json", "json")
        }
        "html" => {
            let html = generate_html_report(&findings_data, &request.language);
            (html, "text/html", "html")
        }
        _ => {
            // Default to markdown
            let md = generate_markdown_report(&findings_data, &request.language);
            (md, "text/markdown", "md")
        }
    };
    
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("bounty_report_{}.{}", timestamp, ext);
    
    Ok(ExportReportResponse {
        filename,
        content,
        mime_type: mime_type.to_string(),
    })
}

/// Generate markdown report
fn generate_markdown_report(findings: &[FindingExportData], language: &str) -> String {
    let mut md = String::new();
    let is_zh = language == "zh";
    
    md.push_str(if is_zh { "# 漏洞报告\n\n" } else { "# Vulnerability Report\n\n" });
    md.push_str(&format!(
        "{}: {}\n\n",
        if is_zh { "生成时间" } else { "Generated" },
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));
    
    md.push_str(if is_zh { "## 概要\n\n" } else { "## Summary\n\n" });
    let severity_counts = count_severities(findings);
    md.push_str(&format!(
        "- {}: {}\n",
        if is_zh { "发现总数" } else { "Total Findings" },
        findings.len()
    ));
    for (sev, count) in &severity_counts {
        md.push_str(&format!("- {}: {}\n", sev.to_uppercase(), count));
    }
    md.push_str("\n---\n\n");
    
    for (i, finding) in findings.iter().enumerate() {
        md.push_str(&format!(
            "## {}. {} [{}]\n\n",
            i + 1, finding.title, finding.severity.to_uppercase()
        ));
        
        md.push_str(&format!(
            "**{}**: {}\n\n",
            if is_zh { "漏洞类型" } else { "Type" },
            finding.finding_type
        ));
        
        if let Some(ref url) = finding.affected_url {
            md.push_str(&format!(
                "**{}**: `{}`\n\n",
                if is_zh { "影响URL" } else { "Affected URL" },
                url
            ));
        }
        
        if let Some(ref cwe) = finding.cwe_id {
            md.push_str(&format!("**CWE**: {}\n\n", cwe));
        }
        
        md.push_str(&format!(
            "### {}\n\n{}\n\n",
            if is_zh { "描述" } else { "Description" },
            finding.description
        ));
        
        if let Some(ref impact) = finding.impact {
            md.push_str(&format!(
                "### {}\n\n{}\n\n",
                if is_zh { "影响" } else { "Impact" },
                impact
            ));
        }
        
        if let Some(ref steps) = finding.reproduction_steps {
            md.push_str(&format!(
                "### {}\n\n",
                if is_zh { "复现步骤" } else { "Reproduction Steps" }
            ));
            for (j, step) in steps.iter().enumerate() {
                md.push_str(&format!("{}. {}\n", j + 1, step));
            }
            md.push_str("\n");
        }
        
        if !finding.evidence.is_empty() {
            md.push_str(&format!(
                "### {}\n\n",
                if is_zh { "证据" } else { "Evidence" }
            ));
            for ev in &finding.evidence {
                md.push_str(&format!("#### {}\n\n", ev.title));
                if let Some(ref content) = ev.content {
                    md.push_str(&format!("```\n{}\n```\n\n", content));
                }
            }
        }
        
        if let Some(ref remediation) = finding.remediation {
            md.push_str(&format!(
                "### {}\n\n{}\n\n",
                if is_zh { "修复建议" } else { "Remediation" },
                remediation
            ));
        }
        
        md.push_str("\n---\n\n");
    }
    
    md
}

/// Generate HTML report
fn generate_html_report(findings: &[FindingExportData], language: &str) -> String {
    let is_zh = language == "zh";
    let title = if is_zh { "漏洞报告" } else { "Vulnerability Report" };
    
    let mut html = format!(r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>{}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 900px; margin: 0 auto; padding: 20px; }}
        h1 {{ color: #333; border-bottom: 2px solid #333; padding-bottom: 10px; }}
        h2 {{ color: #444; margin-top: 30px; }}
        .finding {{ background: #f9f9f9; border-radius: 8px; padding: 20px; margin: 20px 0; }}
        .severity {{ display: inline-block; padding: 4px 12px; border-radius: 4px; font-weight: bold; color: white; }}
        .critical {{ background: #dc3545; }}
        .high {{ background: #fd7e14; }}
        .medium {{ background: #ffc107; color: #333; }}
        .low {{ background: #28a745; }}
        pre {{ background: #2d2d2d; color: #f8f8f2; padding: 15px; border-radius: 5px; overflow-x: auto; }}
        code {{ background: #e9ecef; padding: 2px 6px; border-radius: 3px; }}
    </style>
</head>
<body>
    <h1>{}</h1>
    <p>{}: {}</p>
"#, title, title, 
    if is_zh { "生成时间" } else { "Generated" },
    Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
);

    for (i, finding) in findings.iter().enumerate() {
        html.push_str(&format!(
            r#"<div class="finding">
            <h2>{}. {} <span class="severity {}">{}</span></h2>
            <p><strong>{}</strong>: {}</p>
            <h3>{}</h3><p>{}</p>
"#,
            i + 1, 
            html_escape(&finding.title),
            finding.severity.to_lowercase(),
            finding.severity.to_uppercase(),
            if is_zh { "类型" } else { "Type" },
            html_escape(&finding.finding_type),
            if is_zh { "描述" } else { "Description" },
            html_escape(&finding.description)
        ));

        if let Some(ref remediation) = finding.remediation {
            html.push_str(&format!(
                "<h3>{}</h3><p>{}</p>",
                if is_zh { "修复建议" } else { "Remediation" },
                html_escape(remediation)
            ));
        }

        html.push_str("</div>");
    }

    html.push_str("</body></html>");
    html
}

fn count_severities(findings: &[FindingExportData]) -> Vec<(String, usize)> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for f in findings {
        *counts.entry(f.severity.to_lowercase()).or_insert(0) += 1;
    }
    let order = ["critical", "high", "medium", "low", "info"];
    let mut result = Vec::new();
    for sev in order {
        if let Some(&count) = counts.get(sev) {
            result.push((sev.to_string(), count));
        }
    }
    result
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

// ============================================================================
// Workflow Template Commands
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowTemplateRequest {
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub workflow_definition_id: Option<String>,
    pub steps: Vec<WorkflowStepDefinition>,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub tags: Option<Vec<String>>,
    pub estimated_duration_mins: Option<i32>,
}

/// Input mapping for workflow step - defines how to get data from upstream steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputMapping {
    /// Target field name in current step's input (e.g., "targets")
    pub target_field: String,
    /// Source step ID (e.g., "step_subdomain_enum")
    pub source_step_id: String,
    /// JSONPath expression to extract data (e.g., "$.data.subdomains")
    pub source_path: String,
    /// Optional transform: "first", "flatten", "map:fieldName"
    #[serde(default)]
    pub transform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepDefinition {
    pub id: String,
    pub name: String,
    pub step_type: String, // "tool", "plugin", "condition", "parallel"
    pub tool_name: Option<String>,
    pub plugin_id: Option<String>,
    pub config: serde_json::Value,
    pub depends_on: Vec<String>,
    /// Explicit input mappings from upstream steps
    #[serde(default)]
    pub input_mappings: Vec<InputMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkflowBindingRequest {
    pub program_id: String,
    pub scope_id: Option<String>,
    pub workflow_template_id: String,
    pub is_enabled: Option<bool>,
    pub auto_run_on_change: Option<bool>,
    pub trigger_conditions: Option<serde_json::Value>,
    pub schedule_cron: Option<String>,
}

/// Create a workflow template
#[tauri::command]
pub async fn bounty_create_workflow_template(
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateWorkflowTemplateRequest,
) -> Result<BountyWorkflowTemplateRow, String> {
    let now = Utc::now().to_rfc3339();
    
    let template = BountyWorkflowTemplateRow {
        id: Uuid::new_v4().to_string(),
        name: request.name,
        description: request.description,
        category: request.category,
        workflow_definition_id: request.workflow_definition_id,
        steps_json: serde_json::to_string(&request.steps).unwrap_or_default(),
        input_schema_json: request.input_schema.map(|s| serde_json::to_string(&s).unwrap_or_default()),
        output_schema_json: request.output_schema.map(|s| serde_json::to_string(&s).unwrap_or_default()),
        tags_json: request.tags.map(|t| serde_json::to_string(&t).unwrap_or_default()),
        is_built_in: false,
        estimated_duration_mins: request.estimated_duration_mins,
        created_at: now.clone(),
        updated_at: now,
    };
    
    db_service.create_bounty_workflow_template(&template).await.map_err(|e| e.to_string())?;
    Ok(template)
}

/// Get a workflow template by ID
#[tauri::command]
pub async fn bounty_get_workflow_template(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<BountyWorkflowTemplateRow>, String> {
    db_service.get_bounty_workflow_template(&id).await.map_err(|e| e.to_string())
}

/// List workflow templates
#[tauri::command]
pub async fn bounty_list_workflow_templates(
    db_service: State<'_, Arc<DatabaseService>>,
    category: Option<String>,
    is_built_in: Option<bool>,
) -> Result<Vec<BountyWorkflowTemplateRow>, String> {
    db_service.list_bounty_workflow_templates(category.as_deref(), is_built_in)
        .await.map_err(|e| e.to_string())
}

/// Delete a workflow template
#[tauri::command]
pub async fn bounty_delete_workflow_template(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    db_service.delete_bounty_workflow_template(&id).await.map_err(|e| e.to_string())
}

/// Update a workflow template
#[tauri::command]
pub async fn bounty_update_workflow_template(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
    request: CreateWorkflowTemplateRequest,
) -> Result<BountyWorkflowTemplateRow, String> {
    let existing = db_service.get_bounty_workflow_template(&id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Template not found".to_string())?;
    
    let now = Utc::now().to_rfc3339();
    
    let template = BountyWorkflowTemplateRow {
        id: existing.id,
        name: request.name,
        description: request.description,
        category: request.category,
        workflow_definition_id: request.workflow_definition_id,
        steps_json: serde_json::to_string(&request.steps).unwrap_or_default(),
        input_schema_json: request.input_schema.map(|s| serde_json::to_string(&s).unwrap_or_default()),
        output_schema_json: request.output_schema.map(|s| serde_json::to_string(&s).unwrap_or_default()),
        tags_json: request.tags.map(|t| serde_json::to_string(&t).unwrap_or_default()),
        is_built_in: existing.is_built_in,
        estimated_duration_mins: request.estimated_duration_mins,
        created_at: existing.created_at,
        updated_at: now,
    };
    
    db_service.update_bounty_workflow_template(&template).await.map_err(|e| e.to_string())?;
    Ok(template)
}

/// Run a workflow template
#[tauri::command]
pub async fn bounty_run_workflow_template(
    app_handle: AppHandle,
    db_service: State<'_, Arc<DatabaseService>>,
    plugin_manager: State<'_, Arc<PluginManager>>,
    template_id: String,
    program_id: Option<String>,
    inputs: serde_json::Value,
) -> Result<String, String> {
    let template = db_service.get_bounty_workflow_template(&template_id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Template not found".to_string())?;
    
    let steps: Vec<WorkflowStepDefinition> = serde_json::from_str(&template.steps_json)
        .map_err(|e| format!("Invalid steps: {}", e))?;
    
    if steps.is_empty() {
        return Err("Template has no steps".to_string());
    }
    
    // Generate execution ID
    let execution_id = Uuid::new_v4().to_string();
    
    log::info!("Starting workflow execution {} for template {}", execution_id, template_id);
    log::info!("Inputs: {:?}", inputs);
    log::info!("Steps: {}", steps.len());
    
    // Debug: log input_mappings for each step
    for step in &steps {
        if !step.input_mappings.is_empty() {
            log::info!("Step '{}' has {} input mappings: {:?}", 
                step.id, step.input_mappings.len(), step.input_mappings);
        }
    }
    
    // Clone for async task
    let exec_id = execution_id.clone();
    let db = db_service.inner().clone();
    let pm = plugin_manager.inner().clone();
    let app = app_handle.clone();
    let initial_inputs = inputs.clone();
    
    // Spawn async task to execute workflow
    tokio::spawn(async move {
        execute_workflow_steps(
            exec_id,
            steps,
            initial_inputs,
            program_id,
            db,
            pm,
            app,
        ).await;
    });
    
    Ok(execution_id)
}

/// Execute workflow steps asynchronously
async fn execute_workflow_steps(
    execution_id: String,
    steps: Vec<WorkflowStepDefinition>,
    initial_inputs: serde_json::Value,
    program_id: Option<String>,
    db: Arc<DatabaseService>,
    plugin_manager: Arc<PluginManager>,
    app_handle: AppHandle,
) {
    let total_steps = steps.len();
    let mut completed_steps = 0;
    let mut step_results: HashMap<String, serde_json::Value> = HashMap::new();
    let mut errors: Vec<serde_json::Value> = Vec::new();
    
    // Build dependency graph
    let step_map: HashMap<String, &WorkflowStepDefinition> = steps.iter()
        .map(|s| (s.id.clone(), s))
        .collect();
    
    // Topological sort - execute in dependency order
    let execution_order = topological_sort_steps(&steps);
    
    for step_id in execution_order {
        let step = match step_map.get(&step_id) {
            Some(s) => *s,
            None => continue,
        };
        
        log::info!("Executing step: {} ({})", step.name, step.id);
        
        // Emit step start event (running status)
        let _ = app_handle.emit("workflow:step-start", &serde_json::json!({
            "execution_id": execution_id,
            "step_id": step.id,
            "step_name": step.name,
            "status": "running"
        }));
        
        // Resolve inputs from dependencies and initial inputs
        let resolved_inputs = resolve_step_inputs(step, &step_results, &initial_inputs);
        
        // Execute step
        let result = execute_single_step(
            step,
            &resolved_inputs,
            &plugin_manager,
            &db,
            program_id.as_deref(),
        ).await;
        
        match result {
            Ok(output) => {
                step_results.insert(step.id.clone(), output.clone());
                completed_steps += 1;
                
                // Emit step complete event
                let _ = app_handle.emit("workflow:step-complete", &serde_json::json!({
                    "execution_id": execution_id,
                    "step_id": step.id,
                    "step_name": step.name,
                    "result": output,
                    "success": true
                }));
            }
            Err(e) => {
                log::error!("Step {} failed: {}", step.id, e);
                errors.push(serde_json::json!({
                    "step_id": step.id,
                    "step_name": step.name,
                    "error": e
                }));
                
                // Mark as failed but continue with other steps
                step_results.insert(step.id.clone(), serde_json::json!({
                    "success": false,
                    "error": e
                }));
                completed_steps += 1;
                
                let _ = app_handle.emit("workflow:step-complete", &serde_json::json!({
                    "execution_id": execution_id,
                    "step_id": step.id,
                    "step_name": step.name,
                    "error": e,
                    "success": false
                }));
            }
        }
        
        // Emit progress event
        let progress = ((completed_steps as f32 / total_steps as f32) * 100.0) as u32;
        let _ = app_handle.emit("workflow:progress", &serde_json::json!({
            "execution_id": execution_id,
            "progress": progress,
            "completed_steps": completed_steps,
            "total_steps": total_steps
        }));
    }
    
    // Emit completion event
    let status = if errors.is_empty() { "completed" } else { "completed_with_errors" };
    let _ = app_handle.emit("workflow:run-complete", &serde_json::json!({
        "execution_id": execution_id,
        "status": status,
        "total_steps": total_steps,
        "completed_steps": completed_steps,
        "errors": errors,
        "results": step_results
    }));
    
    log::info!("Workflow execution {} completed with status: {}", execution_id, status);
}

/// Topological sort for step execution order
fn topological_sort_steps(steps: &[WorkflowStepDefinition]) -> Vec<String> {
    let mut result = Vec::new();
    let mut visited: HashMap<String, bool> = HashMap::new();
    let step_map: HashMap<String, &WorkflowStepDefinition> = steps.iter()
        .map(|s| (s.id.clone(), s))
        .collect();
    
    fn visit(
        step_id: &str,
        step_map: &HashMap<String, &WorkflowStepDefinition>,
        visited: &mut HashMap<String, bool>,
        result: &mut Vec<String>,
    ) {
        if visited.get(step_id).copied().unwrap_or(false) {
            return;
        }
        visited.insert(step_id.to_string(), true);
        
        if let Some(step) = step_map.get(step_id) {
            for dep in &step.depends_on {
                visit(dep, step_map, visited, result);
            }
        }
        result.push(step_id.to_string());
    }
    
    for step in steps {
        visit(&step.id, &step_map, &mut visited, &mut result);
    }
    
    result
}

/// Resolve step inputs from dependencies and initial inputs using explicit mappings
fn resolve_step_inputs(
    step: &WorkflowStepDefinition,
    step_results: &HashMap<String, serde_json::Value>,
    initial_inputs: &serde_json::Value,
) -> serde_json::Value {
    let mut resolved = step.config.clone();
    
    log::info!("resolve_step_inputs for step '{}': input_mappings count = {}", 
        step.id, step.input_mappings.len());
    
    // 1. Merge initial inputs (lowest priority)
    if let (Some(config_obj), Some(initial_obj)) = (resolved.as_object_mut(), initial_inputs.as_object()) {
        for (key, value) in initial_obj {
            if !config_obj.contains_key(key) || is_empty_value(config_obj.get(key).unwrap()) {
                config_obj.insert(key.clone(), value.clone());
            }
        }
    }
    
    // 2. Apply explicit input mappings (highest priority)
    log::info!("Processing {} input mappings for step '{}'", step.input_mappings.len(), step.id);
    for mapping in &step.input_mappings {
        log::info!("Processing mapping: target={}, source_step={}, source_path={}", 
            mapping.target_field, mapping.source_step_id, mapping.source_path);
        
        if let Some(source_result) = step_results.get(&mapping.source_step_id) {
            log::info!("Found source_result for step '{}', keys: {:?}", 
                mapping.source_step_id, 
                source_result.as_object().map(|o| o.keys().collect::<Vec<_>>()));
            
            if let Some(value) = extract_by_jsonpath(source_result, &mapping.source_path) {
                log::info!("Extracted value type: {:?}, len: {:?}", 
                    value.as_array().map(|_| "array"),
                    value.as_array().map(|a| a.len()));
                let transformed = apply_transform(value, mapping.transform.as_deref());
                if let Some(obj) = resolved.as_object_mut() {
                    log::info!(
                        "Applied mapping: {}.{} -> {} (transform: {:?})",
                        mapping.source_step_id,
                        mapping.source_path,
                        mapping.target_field,
                        mapping.transform
                    );
                    obj.insert(mapping.target_field.clone(), transformed);
                }
            } else {
                log::warn!(
                    "Failed to extract value from path '{}' in step '{}'",
                    mapping.source_path,
                    mapping.source_step_id
                );
            }
        } else {
            log::warn!(
                "Source step '{}' not found in results. Available steps: {:?}",
                mapping.source_step_id,
                step_results.keys().collect::<Vec<_>>()
            );
            log::warn!(
                "Source step '{}' not found in results for mapping to '{}'",
                mapping.source_step_id,
                mapping.target_field
            );
        }
    }
    
    // 3. Auto-resolve common fields if no explicit mappings (fallback for backward compatibility)
    if step.input_mappings.is_empty() {
        for dep_id in &step.depends_on {
            if let Some(dep_result) = step_results.get(dep_id) {
                if let Some(config_obj) = resolved.as_object_mut() {
                    let output_data = dep_result.get("output")
                        .or_else(|| dep_result.get("data"))
                        .unwrap_or(dep_result);
                    
                    // Auto-resolve subdomains -> targets
                    if let Some(subdomains) = output_data.get("subdomains") {
                        if !config_obj.contains_key("targets") || is_empty_value(config_obj.get("targets").unwrap()) {
                            if let Some(arr) = subdomains.as_array() {
                                let targets: Vec<String> = arr.iter()
                                    .filter_map(|s| s.as_str().map(|s| s.to_string())
                                        .or_else(|| s.get("subdomain").and_then(|v| v.as_str()).map(|s| s.to_string())))
                                    .collect();
                                if !targets.is_empty() {
                                    log::info!("Auto-resolved {} subdomains as targets", targets.len());
                                    config_obj.insert("targets".to_string(), serde_json::json!(targets));
                                }
                            }
                        }
                    }
                    
                    // Auto-resolve results[*].url -> urls
                    if let Some(results) = output_data.get("results") {
                        if !config_obj.contains_key("urls") || is_empty_value(config_obj.get("urls").unwrap()) {
                            if let Some(arr) = results.as_array() {
                                let urls: Vec<String> = arr.iter()
                                    .filter_map(|r| r.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()))
                                    .collect();
                                if !urls.is_empty() {
                                    log::info!("Auto-resolved {} results as urls", urls.len());
                                    config_obj.insert("urls".to_string(), serde_json::json!(urls));
                                }
                            }
                        }
                    }
                    
                    // Auto-resolve domain/url
                    if let Some(url) = output_data.get("url").and_then(|v| v.as_str()) {
                        if !config_obj.contains_key("url") || is_empty_value(config_obj.get("url").unwrap()) {
                            config_obj.insert("url".to_string(), serde_json::json!(url));
                        }
                    }
                    if let Some(domain) = output_data.get("domain").and_then(|v| v.as_str()) {
                        if !config_obj.contains_key("domain") || is_empty_value(config_obj.get("domain").unwrap()) {
                            config_obj.insert("domain".to_string(), serde_json::json!(domain));
                        }
                    }
                }
            }
        }
    }
    
    resolved
}

/// Extract value from JSON data using JSONPath expression
/// Supports: $.field, $.field.subfield, $.field[0], $.field[*].name
/// Note: step_results wraps plugin output in {"output": ...}, so we try both direct path
/// and path prefixed with "output." for compatibility
fn extract_by_jsonpath(data: &serde_json::Value, path: &str) -> Option<serde_json::Value> {
    // Remove leading "$." if present
    let path = path.strip_prefix("$.").unwrap_or(path);
    let path = path.strip_prefix("$").unwrap_or(path);
    let path = path.trim_start_matches('.');
    
    if path.is_empty() {
        return Some(data.clone());
    }
    
    // Try direct path first
    let parts: Vec<&str> = split_jsonpath(path);
    if let Some(result) = extract_recursive(data, &parts) {
        return Some(result);
    }
    
    // If not found and data has "output" field, try extracting from output
    // This handles the step_results wrapper: {"success": true, "output": {...}}
    if let Some(output) = data.get("output") {
        log::debug!("Path '{}' not found at root, trying under 'output' field", path);
        if let Some(result) = extract_recursive(output, &parts) {
            return Some(result);
        }
    }
    
    None
}

/// Split JSONPath into parts, handling brackets correctly
fn split_jsonpath(path: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_bracket = false;
    
    for (i, c) in path.char_indices() {
        match c {
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            '.' if !in_bracket => {
                if i > start {
                    parts.push(&path[start..i]);
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    
    if start < path.len() {
        parts.push(&path[start..]);
    }
    
    parts
}

/// Recursively extract value from JSON
fn extract_recursive(data: &serde_json::Value, parts: &[&str]) -> Option<serde_json::Value> {
    if parts.is_empty() {
        return Some(data.clone());
    }
    
    let part = parts[0];
    let remaining = &parts[1..];
    
    // Handle array wildcard: field[*]
    if part.ends_with("[*]") {
        let field = &part[..part.len() - 3];
        let arr = if field.is_empty() {
            data.as_array()?
        } else {
            data.get(field)?.as_array()?
        };
        
        if remaining.is_empty() {
            return Some(serde_json::Value::Array(arr.clone()));
        }
        
        let mapped: Vec<serde_json::Value> = arr
            .iter()
            .filter_map(|item| extract_recursive(item, remaining))
            .collect();
        
        return Some(serde_json::Value::Array(mapped));
    }
    
    // Handle array index: field[0]
    if let Some(bracket_pos) = part.find('[') {
        if part.ends_with(']') {
            let field = &part[..bracket_pos];
            let idx_str = &part[bracket_pos + 1..part.len() - 1];
            
            if let Ok(idx) = idx_str.parse::<usize>() {
                let arr = if field.is_empty() {
                    data.as_array()?
                } else {
                    data.get(field)?.as_array()?
                };
                return extract_recursive(arr.get(idx)?, remaining);
            }
        }
    }
    
    // Regular field access
    extract_recursive(data.get(part)?, remaining)
}

/// Apply transform to extracted value
fn apply_transform(value: serde_json::Value, transform: Option<&str>) -> serde_json::Value {
    match transform {
        None => value,
        Some("first") => {
            // Get first element of array
            value
                .as_array()
                .and_then(|a| a.first().cloned())
                .unwrap_or(value)
        }
        Some("flatten") => {
            // Flatten nested arrays
            if let Some(arr) = value.as_array() {
                let flat: Vec<serde_json::Value> = arr
                    .iter()
                    .flat_map(|v| {
                        v.as_array()
                            .cloned()
                            .unwrap_or_else(|| vec![v.clone()])
                    })
                    .collect();
                serde_json::Value::Array(flat)
            } else {
                value
            }
        }
        Some(t) if t.starts_with("map:") => {
            // Extract field from each object in array: map:fieldName
            let field = &t[4..];
            if let Some(arr) = value.as_array() {
                let mapped: Vec<serde_json::Value> = arr
                    .iter()
                    .filter_map(|item| item.get(field).cloned())
                    .collect();
                serde_json::Value::Array(mapped)
            } else {
                value
            }
        }
        Some(unknown) => {
            log::warn!("Unknown transform: {}", unknown);
            value
        }
    }
}

fn is_empty_value(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) => s.is_empty(),
        serde_json::Value::Array(arr) => arr.is_empty(),
        serde_json::Value::Object(obj) => obj.is_empty(),
        _ => false,
    }
}

/// Execute a single workflow step
async fn execute_single_step(
    step: &WorkflowStepDefinition,
    inputs: &serde_json::Value,
    plugin_manager: &Arc<PluginManager>,
    db: &Arc<DatabaseService>,
    program_id: Option<&str>,
) -> Result<serde_json::Value, String> {
    let plugin_id = step.plugin_id.as_deref()
        .or(step.tool_name.as_deref())
        .ok_or_else(|| "Step has no plugin_id or tool_name".to_string())?;
    
    log::info!("Executing plugin '{}' with inputs: {:?}", plugin_id, inputs);
    
    // Ensure plugin is loaded
    if plugin_manager.get_plugin(plugin_id).await.is_none() {
        log::info!("Plugin '{}' not in memory, loading from database...", plugin_id);
        
        if let Ok(Some(plugin_data)) = db.get_plugin_from_registry(plugin_id).await {
            let metadata = sentinel_traffic::PluginMetadata {
                id: plugin_id.to_string(),
                name: plugin_data.metadata.name.clone(),
                version: plugin_data.metadata.version.clone(),
                author: plugin_data.metadata.author.clone(),
                main_category: plugin_data.metadata.main_category.clone(),
                category: plugin_data.metadata.category.clone(),
                description: plugin_data.metadata.description.clone(),
                default_severity: sentinel_traffic::types::Severity::Medium,
                tags: plugin_data.metadata.tags.clone(),
            };
            
            let code = db.get_plugin_code(plugin_id).await.ok().flatten().unwrap_or_default();
            // Register as enabled for workflow execution
            let _ = plugin_manager.register_plugin(plugin_id.to_string(), metadata, true).await;
            let _ = plugin_manager.set_plugin_code(plugin_id.to_string(), code).await;
            log::info!("Plugin '{}' loaded from database and enabled", plugin_id);
        } else {
            return Err(format!("Plugin '{}' not found in database", plugin_id));
        }
    } else {
        // Plugin exists in memory, ensure it's enabled for execution
        if let Err(e) = plugin_manager.enable_plugin(plugin_id).await {
            log::warn!("Failed to enable plugin '{}': {}", plugin_id, e);
        }
    }
    
    // Execute plugin
    match plugin_manager.execute_agent(plugin_id, inputs).await {
        Ok((findings, output)) => {
            let result = serde_json::json!({
                "success": true,
                "plugin_id": plugin_id,
                "findings_count": findings.len(),
                "findings": findings,
                "output": output
            });
            
            // Auto-sink findings to database if program_id is provided
            if let Some(pid) = program_id {
                for finding in &findings {
                    if let Err(e) = auto_sink_finding(db, pid, finding).await {
                        log::warn!("Failed to auto-sink finding: {}", e);
                    }
                }
            }
            
            Ok(result)
        }
        Err(e) => {
            Err(format!("Plugin execution failed: {}", e))
        }
    }
}

/// Auto-sink finding to database
async fn auto_sink_finding(
    db: &Arc<DatabaseService>,
    program_id: &str,
    finding: &sentinel_plugins::Finding,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    
    // Calculate fingerprint for deduplication
    let fingerprint = finding.calculate_signature();
    
    let finding_row = BountyFindingRow {
        id: Uuid::new_v4().to_string(),
        program_id: program_id.to_string(),
        scope_id: None,
        asset_id: None,
        title: finding.title.clone(),
        description: finding.description.clone(),
        finding_type: finding.vuln_type.clone(),
        severity: format!("{:?}", finding.severity).to_lowercase(),
        status: "new".to_string(),
        confidence: format!("{:?}", finding.confidence).to_lowercase(),
        cvss_score: None,
        cwe_id: finding.cwe.clone(),
        affected_url: Some(finding.url.clone()),
        affected_parameter: Some(finding.location.clone()),
        reproduction_steps_json: None,
        impact: None,
        remediation: finding.remediation.clone(),
        evidence_ids_json: None,
        tags_json: None,
        metadata_json: None,
        fingerprint,
        duplicate_of: None,
        first_seen_at: now.clone(),
        last_seen_at: now.clone(),
        verified_at: None,
        created_at: now.clone(),
        updated_at: now,
        created_by: "workflow".to_string(),
    };
    
    db.create_bounty_finding(&finding_row).await.map_err(|e| e.to_string())?;
    log::info!("Auto-sinked finding: {}", finding_row.title);
    
    Ok(())
}

/// Create a workflow binding
#[tauri::command]
pub async fn bounty_create_workflow_binding(
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateWorkflowBindingRequest,
) -> Result<BountyWorkflowBindingRow, String> {
    let now = Utc::now().to_rfc3339();
    
    let binding = BountyWorkflowBindingRow {
        id: Uuid::new_v4().to_string(),
        program_id: request.program_id,
        scope_id: request.scope_id,
        workflow_template_id: request.workflow_template_id,
        is_enabled: request.is_enabled.unwrap_or(true),
        auto_run_on_change: request.auto_run_on_change.unwrap_or(false),
        trigger_conditions_json: request.trigger_conditions.map(|c| serde_json::to_string(&c).unwrap_or_default()),
        schedule_cron: request.schedule_cron,
        last_run_at: None,
        last_run_status: None,
        run_count: 0,
        created_at: now.clone(),
        updated_at: now,
    };
    
    db_service.create_bounty_workflow_binding(&binding).await.map_err(|e| e.to_string())?;
    Ok(binding)
}

/// List workflow bindings
#[tauri::command]
pub async fn bounty_list_workflow_bindings(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
    scope_id: Option<String>,
    is_enabled: Option<bool>,
) -> Result<Vec<BountyWorkflowBindingRow>, String> {
    db_service.list_bounty_workflow_bindings(
        program_id.as_deref(),
        scope_id.as_deref(),
        is_enabled,
    ).await.map_err(|e| e.to_string())
}

/// Delete a workflow binding
#[tauri::command]
pub async fn bounty_delete_workflow_binding(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    db_service.delete_bounty_workflow_binding(&id).await.map_err(|e| e.to_string())
}

/// Initialize built-in workflow templates (disabled - no built-in templates)
#[tauri::command]
pub async fn bounty_init_builtin_templates(
    _db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<BountyWorkflowTemplateRow>, String> {
    // Built-in templates have been removed. Users should create their own templates.
    Ok(Vec::new())
}

// ============================================================================
// Workflow Step Output → Finding/Evidence (Step-level Artifact Sinking)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepOutput {
    pub step_id: String,
    pub step_name: String,
    pub output_type: String, // "finding", "evidence", "asset", "data"
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkWorkflowOutputRequest {
    pub program_id: String,
    pub workflow_run_id: String,
    pub binding_id: Option<String>,
    pub outputs: Vec<WorkflowStepOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkWorkflowOutputResponse {
    pub findings_created: Vec<String>,
    pub evidence_created: Vec<String>,
    pub assets_updated: i32,
}

/// Sink workflow step outputs to bounty findings/evidence
#[tauri::command]
pub async fn bounty_sink_workflow_outputs(
    db_service: State<'_, Arc<DatabaseService>>,
    request: SinkWorkflowOutputRequest,
) -> Result<SinkWorkflowOutputResponse, String> {
    let now = Utc::now().to_rfc3339();
    let mut findings_created = Vec::new();
    let mut evidence_created = Vec::new();
    let mut assets_updated = 0;

    for output in request.outputs {
        match output.output_type.as_str() {
            "finding" => {
                // Extract finding data from step output
                if let Some(finding_data) = extract_finding_from_output(&output.data) {
                    let finding_id = Uuid::new_v4().to_string();
                    let fingerprint = format!(
                        "{}:{}:{}:{}",
                        request.program_id,
                        finding_data.finding_type,
                        finding_data.affected_url.as_deref().unwrap_or(""),
                        output.step_id
                    );
                    let fingerprint = format!("{:x}", md5::compute(fingerprint.as_bytes()));

                    // Check for duplicate
                    if db_service.get_bounty_finding_by_fingerprint(&fingerprint)
                        .await.map_err(|e| e.to_string())?.is_some() {
                        continue;
                    }

                    let finding = BountyFindingRow {
                        id: finding_id.clone(),
                        program_id: request.program_id.clone(),
                        scope_id: None,
                        asset_id: None,
                        title: finding_data.title,
                        description: finding_data.description,
                        finding_type: finding_data.finding_type,
                        severity: finding_data.severity.unwrap_or_else(|| "medium".to_string()),
                        status: "new".to_string(),
                        confidence: finding_data.confidence.unwrap_or_else(|| "medium".to_string()),
                        cvss_score: None,
                        cwe_id: finding_data.cwe_id,
                        affected_url: finding_data.affected_url,
                        affected_parameter: finding_data.affected_parameter,
                        reproduction_steps_json: finding_data.reproduction_steps.map(|s| 
                            serde_json::to_string(&s).unwrap_or_default()
                        ),
                        impact: finding_data.impact,
                        remediation: finding_data.remediation,
                        evidence_ids_json: None,
                        tags_json: Some(serde_json::to_string(&vec!["workflow", "automated"]).unwrap_or_default()),
                        metadata_json: Some(serde_json::to_string(&serde_json::json!({
                            "source": "workflow",
                            "workflow_run_id": request.workflow_run_id,
                            "step_id": output.step_id,
                            "step_name": output.step_name,
                        })).unwrap_or_default()),
                        fingerprint,
                        duplicate_of: None,
                        first_seen_at: now.clone(),
                        last_seen_at: now.clone(),
                        verified_at: None,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                        created_by: "workflow".to_string(),
                    };

                    db_service.create_bounty_finding(&finding).await.map_err(|e| e.to_string())?;
                    findings_created.push(finding_id);
                }
            }
            "evidence" => {
                // Extract evidence data from step output
                if let Some(evidence_data) = extract_evidence_from_output(&output.data) {
                    let evidence_id = Uuid::new_v4().to_string();

                    let evidence = BountyEvidenceRow {
                        id: evidence_id.clone(),
                        finding_id: evidence_data.finding_id.unwrap_or_default(),
                        evidence_type: evidence_data.evidence_type,
                        title: format!("{} - {}", output.step_name, evidence_data.title),
                        description: evidence_data.description,
                        file_path: evidence_data.file_path,
                        file_url: evidence_data.file_url,
                        content: evidence_data.content,
                        mime_type: evidence_data.mime_type,
                        file_size: None,
                        http_request_json: evidence_data.http_request.map(|r| 
                            serde_json::to_string(&r).unwrap_or_default()
                        ),
                        http_response_json: evidence_data.http_response.map(|r| 
                            serde_json::to_string(&r).unwrap_or_default()
                        ),
                        diff: evidence_data.diff,
                        tags_json: Some(serde_json::to_string(&vec!["workflow", "automated"]).unwrap_or_default()),
                        metadata_json: Some(serde_json::to_string(&serde_json::json!({
                            "workflow_run_id": request.workflow_run_id,
                            "step_id": output.step_id,
                        })).unwrap_or_default()),
                        display_order: 0,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                    };

                    db_service.create_bounty_evidence(&evidence).await.map_err(|e| e.to_string())?;
                    evidence_created.push(evidence_id);
                }
            }
            "asset" | "data" => {
                // For now, just count these
                assets_updated += 1;
            }
            _ => {}
        }
    }

    // Update binding run status if provided
    if let Some(binding_id) = request.binding_id {
        let status = if findings_created.is_empty() { "completed" } else { "findings_generated" };
        let _ = db_service.update_bounty_workflow_binding_run_status(&binding_id, status).await;
    }

    Ok(SinkWorkflowOutputResponse {
        findings_created,
        evidence_created,
        assets_updated,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtractedFinding {
    title: String,
    description: String,
    finding_type: String,
    severity: Option<String>,
    confidence: Option<String>,
    affected_url: Option<String>,
    affected_parameter: Option<String>,
    cwe_id: Option<String>,
    impact: Option<String>,
    remediation: Option<String>,
    reproduction_steps: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExtractedEvidence {
    finding_id: Option<String>,
    evidence_type: String,
    title: String,
    description: Option<String>,
    file_path: Option<String>,
    file_url: Option<String>,
    content: Option<String>,
    mime_type: Option<String>,
    http_request: Option<serde_json::Value>,
    http_response: Option<serde_json::Value>,
    diff: Option<String>,
}

fn extract_finding_from_output(data: &serde_json::Value) -> Option<ExtractedFinding> {
    // Try to parse the data as a finding structure
    if let Some(obj) = data.as_object() {
        Some(ExtractedFinding {
            title: obj.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled Finding").to_string(),
            description: obj.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            finding_type: obj.get("type").or(obj.get("finding_type")).and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
            severity: obj.get("severity").and_then(|v| v.as_str()).map(|s| s.to_string()),
            confidence: obj.get("confidence").and_then(|v| v.as_str()).map(|s| s.to_string()),
            affected_url: obj.get("url").or(obj.get("affected_url")).and_then(|v| v.as_str()).map(|s| s.to_string()),
            affected_parameter: obj.get("parameter").or(obj.get("affected_parameter")).and_then(|v| v.as_str()).map(|s| s.to_string()),
            cwe_id: obj.get("cwe").or(obj.get("cwe_id")).and_then(|v| v.as_str()).map(|s| s.to_string()),
            impact: obj.get("impact").and_then(|v| v.as_str()).map(|s| s.to_string()),
            remediation: obj.get("remediation").and_then(|v| v.as_str()).map(|s| s.to_string()),
            reproduction_steps: obj.get("steps").or(obj.get("reproduction_steps")).and_then(|v| {
                if let Some(arr) = v.as_array() {
                    Some(arr.iter().filter_map(|s| s.as_str().map(|s| s.to_string())).collect())
                } else {
                    None
                }
            }),
        })
    } else {
        None
    }
}

fn extract_evidence_from_output(data: &serde_json::Value) -> Option<ExtractedEvidence> {
    if let Some(obj) = data.as_object() {
        Some(ExtractedEvidence {
            finding_id: obj.get("finding_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
            evidence_type: obj.get("type").or(obj.get("evidence_type")).and_then(|v| v.as_str()).unwrap_or("other").to_string(),
            title: obj.get("title").and_then(|v| v.as_str()).unwrap_or("Evidence").to_string(),
            description: obj.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
            file_path: obj.get("file_path").and_then(|v| v.as_str()).map(|s| s.to_string()),
            file_url: obj.get("file_url").and_then(|v| v.as_str()).map(|s| s.to_string()),
            content: obj.get("content").and_then(|v| v.as_str()).map(|s| s.to_string()),
            mime_type: obj.get("mime_type").and_then(|v| v.as_str()).map(|s| s.to_string()),
            http_request: obj.get("request").or(obj.get("http_request")).cloned(),
            http_response: obj.get("response").or(obj.get("http_response")).cloned(),
            diff: obj.get("diff").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    } else {
        None
    }
}

// ============================================================================
// Change Event → Workflow Trigger (A3)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerCondition {
    pub event_types: Option<Vec<String>>,
    pub min_severity: Option<String>,
    pub asset_tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTriggerResult {
    pub binding_id: String,
    pub template_id: String,
    pub template_name: String,
    pub triggered: bool,
    pub reason: Option<String>,
}

/// Get workflows that should be triggered for a change event
#[tauri::command]
pub async fn bounty_get_triggered_workflows(
    db_service: State<'_, Arc<DatabaseService>>,
    event_id: String,
) -> Result<Vec<WorkflowTriggerResult>, String> {
    let event = db_service.get_bounty_change_event(&event_id)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Change event not found".to_string())?;

    let program_id = event.program_id.as_ref().ok_or_else(|| "Event has no program_id".to_string())?;

    // Get all auto-trigger bindings for this program
    let bindings = db_service.get_auto_trigger_workflow_bindings(program_id)
        .await.map_err(|e| e.to_string())?;

    let mut results = Vec::new();

    for binding in bindings {
        let template = db_service.get_bounty_workflow_template(&binding.workflow_template_id)
            .await.map_err(|e| e.to_string())?;

        let template_name = template.as_ref().map(|t| t.name.clone()).unwrap_or_default();

        // Check trigger conditions
        let (should_trigger, reason) = if let Some(conditions_json) = &binding.trigger_conditions_json {
            if let Ok(conditions) = serde_json::from_str::<TriggerCondition>(conditions_json) {
                check_trigger_conditions(&event, &conditions)
            } else {
                (true, None) // If conditions can't be parsed, trigger anyway
            }
        } else {
            (true, None) // No conditions = always trigger
        };

        results.push(WorkflowTriggerResult {
            binding_id: binding.id,
            template_id: binding.workflow_template_id,
            template_name,
            triggered: should_trigger,
            reason,
        });
    }

    Ok(results)
}

/// Trigger workflows for a change event
#[tauri::command]
pub async fn bounty_trigger_workflows_for_event(
    db_service: State<'_, Arc<DatabaseService>>,
    event_id: String,
) -> Result<Vec<String>, String> {
    let triggered_results = bounty_get_triggered_workflows_internal(&db_service, &event_id).await?;
    
    let mut triggered_workflow_ids = Vec::new();

    for result in triggered_results {
        if result.triggered {
            // Record that this workflow was triggered
            let _ = db_service.add_triggered_workflow_to_change_event(&event_id, &result.binding_id).await;
            triggered_workflow_ids.push(result.binding_id.clone());

            // Update binding run status
            let _ = db_service.update_bounty_workflow_binding_run_status(&result.binding_id, "triggered").await;
        }
    }

    // Update event status if workflows were triggered
    if !triggered_workflow_ids.is_empty() {
        let _ = db_service.update_bounty_change_event_status(&event_id, "workflow_triggered", None).await;
    }

    Ok(triggered_workflow_ids)
}

async fn bounty_get_triggered_workflows_internal(
    db_service: &Arc<DatabaseService>,
    event_id: &str,
) -> Result<Vec<WorkflowTriggerResult>, String> {
    let event = db_service.get_bounty_change_event(event_id)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Change event not found".to_string())?;

    let program_id = event.program_id.as_ref().ok_or_else(|| "Event has no program_id".to_string())?;

    let bindings = db_service.get_auto_trigger_workflow_bindings(program_id)
        .await.map_err(|e| e.to_string())?;

    let mut results = Vec::new();

    for binding in bindings {
        let template = db_service.get_bounty_workflow_template(&binding.workflow_template_id)
            .await.map_err(|e| e.to_string())?;

        let template_name = template.as_ref().map(|t| t.name.clone()).unwrap_or_default();

        let (should_trigger, reason) = if let Some(conditions_json) = &binding.trigger_conditions_json {
            if let Ok(conditions) = serde_json::from_str::<TriggerCondition>(conditions_json) {
                check_trigger_conditions(&event, &conditions)
            } else {
                (true, None)
            }
        } else {
            (true, None)
        };

        results.push(WorkflowTriggerResult {
            binding_id: binding.id,
            template_id: binding.workflow_template_id,
            template_name,
            triggered: should_trigger,
            reason,
        });
    }

    Ok(results)
}

fn check_trigger_conditions(event: &BountyChangeEventRow, conditions: &TriggerCondition) -> (bool, Option<String>) {
    // Check event type
    if let Some(ref allowed_types) = conditions.event_types {
        if !allowed_types.contains(&event.event_type) {
            return (false, Some(format!("Event type '{}' not in allowed types", event.event_type)));
        }
    }

    // Check minimum severity
    if let Some(ref min_severity) = conditions.min_severity {
        let event_severity_rank = severity_rank(&event.severity);
        let min_severity_rank = severity_rank(min_severity);
        if event_severity_rank < min_severity_rank {
            return (false, Some(format!("Severity '{}' below minimum '{}'", event.severity, min_severity)));
        }
    }

    // Check asset tags (if event has tags)
    if let Some(ref required_tags) = conditions.asset_tags {
        if let Some(ref tags_json) = event.tags_json {
            if let Ok(event_tags) = serde_json::from_str::<Vec<String>>(tags_json) {
                let has_required_tag = required_tags.iter().any(|t| event_tags.contains(t));
                if !has_required_tag {
                    return (false, Some("Event does not have required tags".to_string()));
                }
            }
        }
    }

    (true, None)
}

fn severity_rank(severity: &str) -> i32 {
    match severity.to_lowercase().as_str() {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

// ============================================================================
// Bounty Asset Commands (P1-B3: Asset Consolidation)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAssetRequest {
    pub program_id: String,
    pub scope_id: Option<String>,
    pub asset_type: Option<String>,
    pub url: String,
    pub tags: Option<Vec<String>>,
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFilter {
    pub program_id: Option<String>,
    pub scope_id: Option<String>,
    pub asset_type: Option<String>,
    pub is_alive: Option<bool>,
    pub has_findings: Option<bool>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Canonicalize a URL
fn canonicalize_url(url: &str) -> (String, Option<String>, Option<i32>, Option<String>, Option<String>) {
    use url::Url;
    
    if let Ok(parsed) = Url::parse(url) {
        let hostname = parsed.host_str().map(|s| s.to_lowercase());
        let port = parsed.port().map(|p| p as i32);
        let path = Some(parsed.path().to_string());
        let protocol = Some(parsed.scheme().to_string());
        
        // Build canonical URL (normalized)
        let canonical = format!(
            "{}://{}{}{}",
            parsed.scheme(),
            hostname.as_deref().unwrap_or(""),
            port.map(|p| format!(":{}", p)).unwrap_or_default(),
            parsed.path()
        ).to_lowercase();
        
        (canonical, hostname, port, path, protocol)
    } else {
        // If URL parsing fails, use the original
        (url.to_lowercase(), None, None, None, None)
    }
}

/// Create or merge a bounty asset
#[tauri::command]
pub async fn bounty_create_asset(
    db_service: State<'_, Arc<DatabaseService>>,
    request: CreateAssetRequest,
) -> Result<BountyAssetRow, String> {
    let now = Utc::now().to_rfc3339();
    let (canonical_url, hostname, port, path, protocol) = canonicalize_url(&request.url);
    
    // Check if asset already exists by canonical URL
    if let Some(mut existing) = db_service.get_bounty_asset_by_canonical_url(&request.program_id, &canonical_url)
        .await.map_err(|e| e.to_string())? 
    {
        // Merge: add original URL if different
        db_service.merge_bounty_asset_url(&existing.id, &request.url)
            .await.map_err(|e| e.to_string())?;
        
        // Update last_seen_at
        existing.last_seen_at = now;
        db_service.update_bounty_asset(&existing).await.map_err(|e| e.to_string())?;
        
        return Ok(existing);
    }
    
    let asset = BountyAssetRow {
        id: Uuid::new_v4().to_string(),
        program_id: request.program_id,
        scope_id: request.scope_id,
        asset_type: request.asset_type.unwrap_or_else(|| "url".to_string()),
        canonical_url,
        original_urls_json: Some(serde_json::to_string(&vec![request.url]).unwrap_or_default()),
        hostname,
        port,
        path,
        protocol,
        ip_addresses_json: None,
        dns_records_json: None,
        tech_stack_json: None,
        fingerprint: None,
        tags_json: request.tags.map(|t| serde_json::to_string(&t).unwrap_or_default()),
        labels_json: request.labels.map(|l| serde_json::to_string(&l).unwrap_or_default()),
        priority_score: Some(0.0),
        risk_score: Some(0.0),
        is_alive: true,
        last_checked_at: None,
        first_seen_at: now.clone(),
        last_seen_at: now.clone(),
        findings_count: 0,
        change_events_count: 0,
        metadata_json: None,
        created_at: now.clone(),
        updated_at: now,
        // ASM fields - all None by default
        ip_version: None, asn: None, asn_org: None, isp: None, country: None,
        city: None, latitude: None, longitude: None, is_cloud: None, cloud_provider: None,
        service_name: None, service_version: None, service_product: None, banner: None,
        transport_protocol: None, cpe: None, domain_registrar: None, registration_date: None,
        expiration_date: None, nameservers_json: None, mx_records_json: None,
        txt_records_json: None, whois_data_json: None, is_wildcard: None, parent_domain: None,
        http_status: None, response_time_ms: None, content_length: None, content_type: None,
        title: None, favicon_hash: None, headers_json: None, waf_detected: None,
        cdn_detected: None, screenshot_path: None, body_hash: None, certificate_id: None,
        ssl_enabled: None, certificate_subject: None, certificate_issuer: None,
        certificate_valid_from: None, certificate_valid_to: None, certificate_san_json: None,
        exposure_level: None, attack_surface_score: None, vulnerability_count: None,
        cvss_max_score: None, exploit_available: None, asset_category: None, asset_owner: None,
        business_unit: None, criticality: None, discovery_method: None, data_sources_json: None,
        confidence_score: None, monitoring_enabled: None, scan_frequency: None,
        last_scan_type: None, parent_asset_id: None, related_assets_json: None,
    };
    
    db_service.create_bounty_asset(&asset).await.map_err(|e| e.to_string())?;
    Ok(asset)
}

/// Get a bounty asset by ID
#[tauri::command]
pub async fn bounty_get_asset(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<Option<BountyAssetRow>, String> {
    db_service.get_bounty_asset(&id).await.map_err(|e| e.to_string())
}

/// List bounty assets
#[tauri::command]
pub async fn bounty_list_assets(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: AssetFilter,
) -> Result<Vec<BountyAssetRow>, String> {
    db_service.list_bounty_assets(
        filter.program_id.as_deref(),
        filter.scope_id.as_deref(),
        filter.asset_type.as_deref(),
        filter.is_alive,
        filter.has_findings,
        filter.search.as_deref(),
        filter.sort_by.as_deref(),
        filter.sort_dir.as_deref(),
        filter.limit,
        filter.offset,
    ).await.map_err(|e| e.to_string())
}

/// Delete a bounty asset
#[tauri::command]
pub async fn bounty_delete_asset(
    db_service: State<'_, Arc<DatabaseService>>,
    id: String,
) -> Result<bool, String> {
    db_service.delete_bounty_asset(&id).await.map_err(|e| e.to_string())
}

/// Get bounty asset statistics
#[tauri::command]
pub async fn bounty_get_asset_stats(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
) -> Result<BountyAssetStats, String> {
    db_service.get_bounty_asset_stats(program_id.as_deref())
        .await.map_err(|e| e.to_string())
}

/// Get top priority assets
#[tauri::command]
pub async fn bounty_get_top_priority_assets(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
    limit: Option<i64>,
) -> Result<Vec<BountyAssetRow>, String> {
    db_service.get_top_priority_assets(&program_id, limit.unwrap_or(10))
        .await.map_err(|e| e.to_string())
}

/// Bulk import assets from scope
#[tauri::command]
pub async fn bounty_import_assets_from_scope(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
    scope_id: String,
) -> Result<i32, String> {
    // Get scope
    let scope = db_service.get_program_scope(&scope_id)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Scope not found".to_string())?;
    
    let now = Utc::now().to_rfc3339();
    let mut count = 0;

    // Parse scope target as URLs or domains
    let values: Vec<&str> = scope.target.lines()
        .filter(|l: &&str| !l.trim().is_empty())
        .collect();

    for value in values {
        let url = if value.starts_with("http://") || value.starts_with("https://") {
            value.to_string()
        } else {
            format!("https://{}", value)
        };

        let (canonical_url, hostname, port, path, protocol) = canonicalize_url(&url);

        // Check if exists
        if db_service.get_bounty_asset_by_canonical_url(&program_id, &canonical_url)
            .await.map_err(|e| e.to_string())?.is_some() 
        {
            continue;
        }

        let asset = BountyAssetRow {
            id: Uuid::new_v4().to_string(),
            program_id: program_id.clone(),
            scope_id: Some(scope_id.clone()),
            asset_type: match scope.scope_type.as_str() {
                "domain" | "wildcard" => "domain".to_string(),
                "url" => "url".to_string(),
                "ip" | "ip_range" => "ip".to_string(),
                _ => "other".to_string(),
            },
            canonical_url,
            original_urls_json: Some(serde_json::to_string(&vec![url]).unwrap_or_default()),
            hostname,
            port,
            path,
            protocol,
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
            metadata_json: None,
            created_at: now.clone(),
            updated_at: now.clone(),
            // ASM fields - all None by default
            ip_version: None, asn: None, asn_org: None, isp: None, country: None,
            city: None, latitude: None, longitude: None, is_cloud: None, cloud_provider: None,
            service_name: None, service_version: None, service_product: None, banner: None,
            transport_protocol: None, cpe: None, domain_registrar: None, registration_date: None,
            expiration_date: None, nameservers_json: None, mx_records_json: None,
            txt_records_json: None, whois_data_json: None, is_wildcard: None, parent_domain: None,
            http_status: None, response_time_ms: None, content_length: None, content_type: None,
            title: None, favicon_hash: None, headers_json: None, waf_detected: None,
            cdn_detected: None, screenshot_path: None, body_hash: None, certificate_id: None,
            ssl_enabled: None, certificate_subject: None, certificate_issuer: None,
            certificate_valid_from: None, certificate_valid_to: None, certificate_san_json: None,
            exposure_level: None, attack_surface_score: None, vulnerability_count: None,
            cvss_max_score: None, exploit_available: None, asset_category: None, asset_owner: None,
            business_unit: None, criticality: None, discovery_method: None, data_sources_json: None,
            confidence_score: None, monitoring_enabled: None, scan_frequency: None,
            last_scan_type: None, parent_asset_id: None, related_assets_json: None,
        };

        db_service.create_bounty_asset(&asset).await.map_err(|e| e.to_string())?;
        count += 1;
    }

    Ok(count)
}

// ============================================================================
// P1-B4: Fingerprint & Label System
// ============================================================================

/// Predefined high-value labels
pub const HIGH_VALUE_LABELS: &[&str] = &[
    "admin-panel",
    "api-endpoint",
    "auth-system",
    "payment-gateway",
    "user-data",
    "file-upload",
    "debug-enabled",
    "exposed-config",
    "vulnerable-tech",
    "outdated-software",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAssetFingerprintRequest {
    pub asset_id: String,
    pub tech_stack: Option<Vec<TechStackItem>>,
    pub ip_addresses: Option<Vec<String>>,
    pub dns_records: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechStackItem {
    pub name: String,
    pub version: Option<String>,
    pub category: String, // "framework", "server", "cms", "library", "language"
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAssetLabelsRequest {
    pub asset_id: String,
    pub labels: Vec<String>,
}

/// Generate fingerprint from asset data
fn generate_asset_fingerprint(
    hostname: Option<&str>,
    tech_stack: &[TechStackItem],
    port: Option<i32>,
) -> String {
    let mut data = String::new();
    
    if let Some(h) = hostname {
        data.push_str(h);
    }
    if let Some(p) = port {
        data.push_str(&format!(":{}", p));
    }
    
    // Sort tech stack for consistent fingerprint
    let mut techs: Vec<String> = tech_stack.iter()
        .map(|t| format!("{}:{}", t.name, t.version.as_deref().unwrap_or("")))
        .collect();
    techs.sort();
    data.push_str(&techs.join(","));
    
    format!("{:x}", md5::compute(data.as_bytes()))
}

/// Update asset fingerprint and tech stack
#[tauri::command]
pub async fn bounty_update_asset_fingerprint(
    db_service: State<'_, Arc<DatabaseService>>,
    request: UpdateAssetFingerprintRequest,
) -> Result<BountyAssetRow, String> {
    let mut asset = db_service.get_bounty_asset(&request.asset_id)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Asset not found".to_string())?;
    
    let now = Utc::now().to_rfc3339();
    
    // Update tech stack
    if let Some(tech_stack) = &request.tech_stack {
        asset.tech_stack_json = Some(serde_json::to_string(tech_stack).unwrap_or_default());
        
        // Generate fingerprint
        asset.fingerprint = Some(generate_asset_fingerprint(
            asset.hostname.as_deref(),
            tech_stack,
            asset.port,
        ));
        
        // Auto-add labels based on tech stack
        let mut labels: Vec<String> = asset.labels_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        
        for tech in tech_stack {
            // Check for high-value technologies
            let tech_lower = tech.name.to_lowercase();
            if tech_lower.contains("admin") || tech_lower.contains("管理") {
                if !labels.contains(&"admin-panel".to_string()) {
                    labels.push("admin-panel".to_string());
                }
            }
            if tech_lower.contains("api") || tech_lower.contains("graphql") || tech_lower.contains("rest") {
                if !labels.contains(&"api-endpoint".to_string()) {
                    labels.push("api-endpoint".to_string());
                }
            }
            if tech_lower.contains("debug") || tech_lower.contains("devtools") {
                if !labels.contains(&"debug-enabled".to_string()) {
                    labels.push("debug-enabled".to_string());
                }
            }
            // Check for outdated/vulnerable versions
            if let Some(version) = &tech.version {
                if is_potentially_vulnerable(&tech.name, version) {
                    if !labels.contains(&"vulnerable-tech".to_string()) {
                        labels.push("vulnerable-tech".to_string());
                    }
                }
            }
        }
        
        asset.labels_json = Some(serde_json::to_string(&labels).unwrap_or_default());
    }
    
    // Update IP addresses
    if let Some(ips) = request.ip_addresses {
        asset.ip_addresses_json = Some(serde_json::to_string(&ips).unwrap_or_default());
    }
    
    // Update DNS records
    if let Some(dns) = request.dns_records {
        asset.dns_records_json = Some(serde_json::to_string(&dns).unwrap_or_default());
    }
    
    asset.last_checked_at = Some(now.clone());
    asset.updated_at = now;
    
    db_service.update_bounty_asset(&asset).await.map_err(|e| e.to_string())?;
    Ok(asset)
}

/// Check if a technology version is potentially vulnerable
fn is_potentially_vulnerable(name: &str, version: &str) -> bool {
    let name_lower = name.to_lowercase();
    
    // Known vulnerable version patterns (simplified)
    let vulnerable_patterns: &[(&str, &str)] = &[
        ("apache", "2.4.49"), // Path traversal
        ("apache", "2.4.50"),
        ("log4j", "2.14"),
        ("log4j", "2.15"),
        ("spring", "5.3.17"), // Spring4Shell
        ("wordpress", "5.8"),
        ("jquery", "1."),
        ("jquery", "2."),
        ("angular", "1."),
    ];
    
    for (tech, ver_pattern) in vulnerable_patterns {
        if name_lower.contains(tech) && version.starts_with(ver_pattern) {
            return true;
        }
    }
    
    false
}

/// Add labels to an asset
#[tauri::command]
pub async fn bounty_add_asset_labels(
    db_service: State<'_, Arc<DatabaseService>>,
    request: AddAssetLabelsRequest,
) -> Result<BountyAssetRow, String> {
    let mut asset = db_service.get_bounty_asset(&request.asset_id)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Asset not found".to_string())?;
    
    let mut labels: Vec<String> = asset.labels_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    
    for label in request.labels {
        if !labels.contains(&label) {
            labels.push(label);
        }
    }
    
    asset.labels_json = Some(serde_json::to_string(&labels).unwrap_or_default());
    asset.updated_at = Utc::now().to_rfc3339();
    
    db_service.update_bounty_asset(&asset).await.map_err(|e| e.to_string())?;
    Ok(asset)
}

/// Get available high-value labels
#[tauri::command]
pub async fn bounty_get_high_value_labels() -> Result<Vec<String>, String> {
    Ok(HIGH_VALUE_LABELS.iter().map(|s| s.to_string()).collect())
}

/// Get assets by label
#[tauri::command]
pub async fn bounty_get_assets_by_label(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
    label: String,
) -> Result<Vec<BountyAssetRow>, String> {
    // Get all assets for program
    let assets = db_service.list_bounty_assets(
        Some(&program_id),
        None, None, None, None, None, None, None, None, None,
    ).await.map_err(|e| e.to_string())?;
    
    // Filter by label
    let filtered: Vec<BountyAssetRow> = assets.into_iter()
        .filter(|a| {
            if let Some(ref labels_json) = a.labels_json {
                if let Ok(labels) = serde_json::from_str::<Vec<String>>(labels_json) {
                    return labels.contains(&label);
                }
            }
            false
        })
        .collect();
    
    Ok(filtered)
}

/// Get assets by tech stack
#[tauri::command]
pub async fn bounty_get_assets_by_tech(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
    tech_name: String,
) -> Result<Vec<BountyAssetRow>, String> {
    let assets = db_service.list_bounty_assets(
        Some(&program_id),
        None, None, None, None, None, None, None, None, None,
    ).await.map_err(|e| e.to_string())?;
    
    let tech_lower = tech_name.to_lowercase();
    let filtered: Vec<BountyAssetRow> = assets.into_iter()
        .filter(|a| {
            if let Some(ref tech_json) = a.tech_stack_json {
                if let Ok(techs) = serde_json::from_str::<Vec<TechStackItem>>(tech_json) {
                    return techs.iter().any(|t| t.name.to_lowercase().contains(&tech_lower));
                }
            }
            false
        })
        .collect();
    
    Ok(filtered)
}

// ============================================================================
// P1-B5: Priority Scoring System
// ============================================================================

/// Label weights for priority calculation
fn get_label_weight(label: &str) -> f64 {
    match label {
        "admin-panel" => 3.0,
        "payment-gateway" => 3.0,
        "user-data" => 2.5,
        "auth-system" => 2.5,
        "file-upload" => 2.0,
        "api-endpoint" => 1.5,
        "debug-enabled" => 2.0,
        "exposed-config" => 2.5,
        "vulnerable-tech" => 3.0,
        "outdated-software" => 2.0,
        _ => 0.5,
    }
}

/// Calculate priority score for an asset
fn calculate_priority_score(
    labels: &[String],
    tech_stack: &[TechStackItem],
    findings_count: i32,
    change_events_count: i32,
    is_alive: bool,
) -> f64 {
    if !is_alive {
        return 0.0;
    }
    
    let mut score = 0.0;
    
    // Label-based score (max ~9.0)
    for label in labels {
        score += get_label_weight(label);
    }
    
    // Tech stack complexity bonus (max ~2.0)
    let tech_bonus = (tech_stack.len() as f64 * 0.2).min(2.0);
    score += tech_bonus;
    
    // Findings history bonus (max ~3.0)
    let findings_bonus = (findings_count as f64 * 0.5).min(3.0);
    score += findings_bonus;
    
    // Change frequency bonus (max ~2.0)
    let change_bonus = (change_events_count as f64 * 0.3).min(2.0);
    score += change_bonus;
    
    // Vulnerable tech stack multiplier
    let has_vulnerable = labels.iter().any(|l| l == "vulnerable-tech");
    if has_vulnerable {
        score *= 1.2;
    }
    
    // Normalize to 0-10 scale
    (score.min(10.0) * 10.0).round() / 10.0
}

/// Recalculate priority score for an asset
#[tauri::command]
pub async fn bounty_recalculate_asset_priority(
    db_service: State<'_, Arc<DatabaseService>>,
    asset_id: String,
) -> Result<BountyAssetRow, String> {
    let mut asset = db_service.get_bounty_asset(&asset_id)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Asset not found".to_string())?;
    
    let labels: Vec<String> = asset.labels_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    
    let tech_stack: Vec<TechStackItem> = asset.tech_stack_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    
    let priority = calculate_priority_score(
        &labels,
        &tech_stack,
        asset.findings_count,
        asset.change_events_count,
        asset.is_alive,
    );
    
    asset.priority_score = Some(priority);
    asset.updated_at = Utc::now().to_rfc3339();
    
    db_service.update_bounty_asset(&asset).await.map_err(|e| e.to_string())?;
    Ok(asset)
}

/// Recalculate priority for all assets in a program
#[tauri::command]
pub async fn bounty_recalculate_all_asset_priorities(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
) -> Result<i32, String> {
    let assets = db_service.list_bounty_assets(
        Some(&program_id),
        None, None, None, None, None, None, None, None, None,
    ).await.map_err(|e| e.to_string())?;
    
    let mut count = 0;
    let now = Utc::now().to_rfc3339();
    
    for mut asset in assets {
        let labels: Vec<String> = asset.labels_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        
        let tech_stack: Vec<TechStackItem> = asset.tech_stack_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        
        let priority = calculate_priority_score(
            &labels,
            &tech_stack,
            asset.findings_count,
            asset.change_events_count,
            asset.is_alive,
        );
        
        if asset.priority_score != Some(priority) {
            asset.priority_score = Some(priority);
            asset.updated_at = now.clone();
            db_service.update_bounty_asset(&asset).await.map_err(|e| e.to_string())?;
            count += 1;
        }
    }
    
    Ok(count)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityQueueItem {
    pub asset: BountyAssetRow,
    pub reason: String,
}

/// Get high-value priority queue
#[tauri::command]
pub async fn bounty_get_priority_queue(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: String,
    limit: Option<i64>,
) -> Result<Vec<PriorityQueueItem>, String> {
    let assets = db_service.get_top_priority_assets(&program_id, limit.unwrap_or(20))
        .await.map_err(|e| e.to_string())?;
    
    let mut queue: Vec<PriorityQueueItem> = assets.into_iter()
        .map(|asset| {
            let labels: Vec<String> = asset.labels_json
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            
            let reason = generate_priority_reason(&labels, asset.findings_count, asset.change_events_count);
            
            PriorityQueueItem { asset, reason }
        })
        .collect();
    
    // Sort by priority score descending
    queue.sort_by(|a, b| {
        b.asset.priority_score.unwrap_or(0.0)
            .partial_cmp(&a.asset.priority_score.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    
    Ok(queue)
}

fn generate_priority_reason(labels: &[String], findings_count: i32, change_events_count: i32) -> String {
    let mut reasons = Vec::new();
    
    let high_value_labels: Vec<&str> = labels.iter()
        .filter(|l| get_label_weight(l) >= 2.0)
        .map(|s| s.as_str())
        .collect();
    
    if !high_value_labels.is_empty() {
        reasons.push(format!("High-value labels: {}", high_value_labels.join(", ")));
    }
    
    if findings_count > 0 {
        reasons.push(format!("{} previous findings", findings_count));
    }
    
    if change_events_count > 0 {
        reasons.push(format!("{} change events", change_events_count));
    }
    
    if reasons.is_empty() {
        "Standard priority".to_string()
    } else {
        reasons.join("; ")
    }
}

// ============================================================================
// D3: Submission & Retest Operations
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionTimelineEvent {
    pub id: String,
    pub event_type: String, // "submitted", "triaged", "response", "update", "resolved", "retest"
    pub timestamp: String,
    pub content: String,
    pub actor: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTimelineEventRequest {
    pub submission_id: String,
    pub event_type: String,
    pub content: String,
    pub actor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionWithTimeline {
    pub submission: BountySubmissionRow,
    pub timeline: Vec<SubmissionTimelineEvent>,
    pub days_since_submission: i64,
    pub needs_followup: bool,
    pub next_action: Option<String>,
}

/// Add a timeline event to submission
#[tauri::command]
pub async fn bounty_add_submission_timeline_event(
    db_service: State<'_, Arc<DatabaseService>>,
    request: AddTimelineEventRequest,
) -> Result<BountySubmissionRow, String> {
    let mut submission = db_service.get_bounty_submission(&request.submission_id)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Submission not found".to_string())?;
    
    let now = Utc::now().to_rfc3339();
    
    // Parse existing timeline
    let mut timeline: Vec<SubmissionTimelineEvent> = submission.timeline_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    
    // Add new event
    let event = SubmissionTimelineEvent {
        id: Uuid::new_v4().to_string(),
        event_type: request.event_type.clone(),
        timestamp: now.clone(),
        content: request.content,
        actor: request.actor,
        metadata: None,
    };
    
    timeline.push(event);
    
    // Update submission
    submission.timeline_json = Some(serde_json::to_string(&timeline).unwrap_or_default());
    submission.updated_at = now;
    
    // Update status if needed
    match request.event_type.as_str() {
        "response" => {
            if submission.status == "submitted" {
                submission.status = "triaged".to_string();
            }
        }
        "resolved" => {
            submission.status = "accepted".to_string();
        }
        _ => {}
    }
    
    db_service.update_bounty_submission(&submission).await.map_err(|e| e.to_string())?;
    Ok(submission)
}

/// Get submission with timeline analysis
#[tauri::command]
pub async fn bounty_get_submission_with_timeline(
    db_service: State<'_, Arc<DatabaseService>>,
    submission_id: String,
) -> Result<SubmissionWithTimeline, String> {
    let submission = db_service.get_bounty_submission(&submission_id)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Submission not found".to_string())?;
    
    let timeline: Vec<SubmissionTimelineEvent> = submission.timeline_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    
    // Calculate days since submission
    let submitted_at_str = submission.submitted_at.as_deref().unwrap_or(&submission.created_at);
    let submitted_at = chrono::DateTime::parse_from_rfc3339(submitted_at_str)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| Utc::now());
    let days_since = (Utc::now() - submitted_at).num_days();
    
    // Determine if followup is needed
    let last_response = timeline.iter()
        .filter(|e| e.event_type == "response")
        .last();
    
    let needs_followup = if let Some(resp) = last_response {
        let resp_time = chrono::DateTime::parse_from_rfc3339(&resp.timestamp)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| Utc::now());
        (Utc::now() - resp_time).num_days() > 7
    } else {
        days_since > 7 && submission.status == "submitted"
    };
    
    // Suggest next action
    let next_action = match submission.status.as_str() {
        "submitted" if days_since > 7 => Some("Send follow-up message".to_string()),
        "triaged" if days_since > 14 => Some("Request update".to_string()),
        "accepted" => Some("Verify fix and close".to_string()),
        "needs_more_info" => Some("Provide additional information".to_string()),
        _ => None,
    };
    
    Ok(SubmissionWithTimeline {
        submission,
        timeline,
        days_since_submission: days_since,
        needs_followup,
        next_action,
    })
}

/// Get submissions needing followup
#[tauri::command]
pub async fn bounty_get_submissions_needing_followup(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
    days_threshold: Option<i64>,
) -> Result<Vec<SubmissionWithTimeline>, String> {
    let submissions = db_service.list_bounty_submissions(
        program_id.as_deref(),
        None, // finding_id
        None, // statuses
        None, // search
        None, // sort_by
        None, // sort_dir
        None, // limit
        None, // offset
    ).await.map_err(|e| e.to_string())?;
    
    let threshold = days_threshold.unwrap_or(7);
    let mut needs_followup = Vec::new();
    
    for submission in submissions {
        if submission.status == "accepted" || submission.status == "rejected" || submission.status == "closed" {
            continue;
        }
        
        let submitted_at_str = submission.submitted_at.as_deref().unwrap_or(&submission.created_at);
        let submitted_at = chrono::DateTime::parse_from_rfc3339(submitted_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| Utc::now());
        let days_since = (Utc::now() - submitted_at).num_days();
        
        let timeline: Vec<SubmissionTimelineEvent> = submission.timeline_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        
        let default_timestamp = submitted_at_str.to_string();
        let last_activity = timeline.last()
            .map(|e| e.timestamp.as_str())
            .unwrap_or(&default_timestamp);
        
        let last_activity_time = chrono::DateTime::parse_from_rfc3339(last_activity)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| Utc::now());
        let days_since_activity = (Utc::now() - last_activity_time).num_days();
        
        if days_since_activity >= threshold {
            let next_action = match submission.status.as_str() {
                "submitted" => Some("Send follow-up message".to_string()),
                "triaged" => Some("Request update".to_string()),
                "needs_more_info" => Some("Provide additional information".to_string()),
                _ => Some("Check status".to_string()),
            };
            
            needs_followup.push(SubmissionWithTimeline {
                submission,
                timeline,
                days_since_submission: days_since,
                needs_followup: true,
                next_action,
            });
        }
    }
    
    // Sort by days since submission (oldest first)
    needs_followup.sort_by(|a, b| b.days_since_submission.cmp(&a.days_since_submission));
    
    Ok(needs_followup)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetestRequest {
    pub submission_id: String,
    pub finding_id: String,
    pub workflow_template_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetestResult {
    pub submission_id: String,
    pub finding_id: String,
    pub is_fixed: bool,
    pub retested_at: String,
    pub notes: Option<String>,
    pub workflow_run_id: Option<String>,
}

/// Schedule a retest for a finding
#[tauri::command]
pub async fn bounty_schedule_retest(
    db_service: State<'_, Arc<DatabaseService>>,
    request: RetestRequest,
) -> Result<RetestResult, String> {
    let now = Utc::now().to_rfc3339();
    
    // Verify submission and finding exist
    let submission = db_service.get_bounty_submission(&request.submission_id)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Submission not found".to_string())?;
    
    let _finding = db_service.get_bounty_finding(&request.finding_id)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Finding not found".to_string())?;
    
    // Add timeline event for retest scheduled
    let mut timeline: Vec<SubmissionTimelineEvent> = submission.timeline_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    
    timeline.push(SubmissionTimelineEvent {
        id: Uuid::new_v4().to_string(),
        event_type: "retest".to_string(),
        timestamp: now.clone(),
        content: format!("Retest scheduled for finding {}", request.finding_id),
        actor: None,
        metadata: request.workflow_template_id.as_ref().map(|id| {
            serde_json::json!({"workflow_template_id": id})
        }),
    });
    
    let mut updated_submission = submission.clone();
    updated_submission.timeline_json = Some(serde_json::to_string(&timeline).unwrap_or_default());
    updated_submission.updated_at = now.clone();
    
    db_service.update_bounty_submission(&updated_submission).await.map_err(|e| e.to_string())?;
    
    Ok(RetestResult {
        submission_id: request.submission_id,
        finding_id: request.finding_id,
        is_fixed: false, // Will be determined after actual retest
        retested_at: now,
        notes: Some("Retest scheduled".to_string()),
        workflow_run_id: None, // Would be populated if workflow is triggered
    })
}

/// Record retest result
#[tauri::command]
pub async fn bounty_record_retest_result(
    db_service: State<'_, Arc<DatabaseService>>,
    submission_id: String,
    finding_id: String,
    is_fixed: bool,
    notes: Option<String>,
) -> Result<RetestResult, String> {
    let now = Utc::now().to_rfc3339();
    
    let submission = db_service.get_bounty_submission(&submission_id)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Submission not found".to_string())?;
    
    let mut finding = db_service.get_bounty_finding(&finding_id)
        .await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Finding not found".to_string())?;
    
    // Update finding status
    if is_fixed {
        finding.status = "fixed".to_string();
        finding.verified_at = Some(now.clone());
    } else {
        finding.status = "not_fixed".to_string();
    }
    finding.updated_at = now.clone();
    db_service.update_bounty_finding(&finding).await.map_err(|e| e.to_string())?;
    
    // Add timeline event
    let mut timeline: Vec<SubmissionTimelineEvent> = submission.timeline_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    
    let result_text = if is_fixed { "Fixed" } else { "Not Fixed" };
    timeline.push(SubmissionTimelineEvent {
        id: Uuid::new_v4().to_string(),
        event_type: "retest".to_string(),
        timestamp: now.clone(),
        content: format!("Retest completed: {} - {}", result_text, notes.as_deref().unwrap_or("")),
        actor: None,
        metadata: Some(serde_json::json!({"is_fixed": is_fixed})),
    });
    
    let mut updated_submission = submission;
    updated_submission.timeline_json = Some(serde_json::to_string(&timeline).unwrap_or_default());
    updated_submission.updated_at = now.clone();
    
    db_service.update_bounty_submission(&updated_submission).await.map_err(|e| e.to_string())?;
    
    Ok(RetestResult {
        submission_id,
        finding_id,
        is_fixed,
        retested_at: now,
        notes,
        workflow_run_id: None,
    })
}

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
    use sentinel_bounty::services::{WorkflowOrchestrator, StepContext};
    
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
    
    let resolved = orchestrator.resolve_step_inputs(
        &step,
        &request.upstream_results,
        &request.initial_inputs,
    );
    
    // Identify which params were resolved from upstream
    let mut resolved_from_upstream = Vec::new();
    if let (Some(orig_obj), Some(resolved_obj)) = (request.config.as_object(), resolved.as_object()) {
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
    use sentinel_bounty::services::{WorkflowOrchestrator, StepContext, ArtifactType};
    
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
    
    let artifacts = orchestrator.process_step_output(&step, &request.execution_id, &request.raw_output);
    
    let mut summary = ArtifactSummaryResponse::default();
    let processed: Vec<ProcessedArtifact> = artifacts.iter().map(|a| {
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
    }).collect();
    
    Ok(ProcessStepOutputResponse {
        artifacts: processed,
        summary,
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
                if !auto_create_findings { continue; }
                
                // Extract finding data
                if let Ok(finding_data) = serde_json::from_value::<FindingArtifactData>(artifact.data.clone()) {
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
                        if db_service.get_bounty_finding_by_fingerprint(&fingerprint)
                            .await.map_err(|e| e.to_string())?.is_some() {
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
                        severity: finding_data.severity.unwrap_or_else(|| "medium".to_string()),
                        status: "new".to_string(),
                        confidence: finding_data.confidence.unwrap_or_else(|| "medium".to_string()),
                        cvss_score: None,
                        cwe_id: finding_data.cwe_id,
                        affected_url: finding_data.affected_url,
                        affected_parameter: finding_data.affected_parameter,
                        reproduction_steps_json: finding_data.reproduction_steps.map(|s| 
                            serde_json::to_string(&s).unwrap_or_default()
                        ),
                        impact: finding_data.impact,
                        remediation: finding_data.remediation,
                        evidence_ids_json: None,
                        tags_json: Some(serde_json::to_string(&vec!["workflow", "automated"]).unwrap_or_default()),
                        metadata_json: Some(serde_json::to_string(&serde_json::json!({
                            "source": "workflow",
                            "execution_id": request.execution_id,
                            "step_id": artifact.step_id,
                        })).unwrap_or_default()),
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
                        Err(e) => response.errors.push(format!("Failed to create finding: {}", e)),
                    }
                }
            }
            "subdomains" => {
                if !auto_update_assets { continue; }
                
                // Extract subdomains and create assets
                if let Some(subdomains) = artifact.data.get("subdomains").and_then(|v| v.as_array()) {
                    for subdomain_entry in subdomains {
                        let subdomain = subdomain_entry.get("subdomain")
                            .and_then(|v| v.as_str())
                            .unwrap_or_else(|| subdomain_entry.as_str().unwrap_or(""));
                        
                        if subdomain.is_empty() { continue; }
                        
                        // Create asset with canonical URL
                        let canonical_url = format!("https://{}", subdomain);
                        
                        // Check if asset exists
                        let existing = db_service.get_bounty_asset_by_canonical_url(&request.program_id, &canonical_url)
                            .await.map_err(|e| e.to_string())?;
                        
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
                                metadata_json: Some(serde_json::to_string(&serde_json::json!({
                                    "source": "workflow_subdomain_enum",
                                    "execution_id": request.execution_id,
                                })).unwrap_or_default()),
                                created_at: now.clone(),
                                updated_at: now.clone(),
                                // ASM fields - all None by default
                                ip_version: None, asn: None, asn_org: None, isp: None, country: None,
                                city: None, latitude: None, longitude: None, is_cloud: None, cloud_provider: None,
                                service_name: None, service_version: None, service_product: None, banner: None,
                                transport_protocol: None, cpe: None, domain_registrar: None, registration_date: None,
                                expiration_date: None, nameservers_json: None, mx_records_json: None,
                                txt_records_json: None, whois_data_json: None, is_wildcard: None, parent_domain: None,
                                http_status: None, response_time_ms: None, content_length: None, content_type: None,
                                title: None, favicon_hash: None, headers_json: None, waf_detected: None,
                                cdn_detected: None, screenshot_path: None, body_hash: None, certificate_id: None,
                                ssl_enabled: None, certificate_subject: None, certificate_issuer: None,
                                certificate_valid_from: None, certificate_valid_to: None, certificate_san_json: None,
                                exposure_level: None, attack_surface_score: None, vulnerability_count: None,
                                cvss_max_score: None, exploit_available: None, asset_category: None, asset_owner: None,
                                business_unit: None, criticality: None, discovery_method: None, data_sources_json: None,
                                confidence_score: None, monitoring_enabled: None, scan_frequency: None,
                                last_scan_type: None, parent_asset_id: None, related_assets_json: None,
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
                if !auto_update_assets { continue; }
                
                // Extract live hosts and update assets
                if let Some(hosts) = artifact.data.get("hosts").and_then(|v| v.as_array()) {
                    for host in hosts {
                        let url = host.get("url").and_then(|v| v.as_str()).unwrap_or("");
                        if url.is_empty() { continue; }
                        
                        let status_code = host.get("status_code")
                            .or_else(|| host.get("statusCode"))
                            .and_then(|v| v.as_i64())
                            .map(|n| n as i32);
                        let title = host.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
                        let tech: Option<Vec<String>> = host.get("technologies")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());
                        
                        // Try to find existing asset
                        let (canonical_url, _, _, _, _) = canonicalize_url(url);
                        if let Ok(Some(mut asset)) = db_service.get_bounty_asset_by_canonical_url(&request.program_id, &canonical_url).await {
                            // Update existing asset - store status/title in metadata
                            let mut metadata: serde_json::Map<String, serde_json::Value> = asset.metadata_json
                                .as_ref()
                                .and_then(|s| serde_json::from_str(s).ok())
                                .unwrap_or_default();
                            if let Some(sc) = status_code {
                                metadata.insert("status_code".to_string(), serde_json::json!(sc));
                            }
                            if let Some(ref t) = title {
                                metadata.insert("title".to_string(), serde_json::json!(t));
                            }
                            asset.metadata_json = Some(serde_json::to_string(&metadata).unwrap_or_default());
                            asset.is_alive = status_code.map(|c| c >= 200 && c < 400).unwrap_or(true);
                            if let Some(t) = tech {
                                asset.tech_stack_json = Some(serde_json::to_string(&t).unwrap_or_default());
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
pub async fn bounty_get_plugin_ports(
    plugin_id: String,
) -> Result<Option<PluginPortInfo>, String> {
    use sentinel_bounty::services::PluginPortRegistry;
    
    let registry = PluginPortRegistry::new();
    
    let output_ports: Vec<PortDef> = registry.get_output_ports(&plugin_id)
        .map(|ports| ports.iter().map(|(name, atype)| PortDef {
            name: name.clone(),
            artifact_type: atype.as_str().to_string(),
        }).collect())
        .unwrap_or_default();
    
    let input_params: Vec<InputParamDef> = registry.get_input_specs(&plugin_id)
        .map(|specs| specs.iter().map(|(name, spec)| InputParamDef {
            name: name.clone(),
            expected_artifact_type: spec.artifact_type.as_str().to_string(),
            extract_path: spec.extract_path.clone(),
            required: spec.required,
        }).collect())
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
        "http_prober",
        "tech_fingerprinter",
        "directory_bruteforcer",
        "js_analyzer",
        "ssrf_detector",
        "cors_misconfiguration",
        "open_redirect_detector",
        "nextjs_rce_scanner",
        "subdomain_takeover",
    ];
    
    let mut result = Vec::new();
    for plugin_id in plugin_ids {
        let output_ports = registry.get_output_ports(plugin_id)
            .map(|ports| ports.iter().map(|(name, atype)| PortDef {
                name: name.clone(),
                artifact_type: atype.as_str().to_string(),
            }).collect())
            .unwrap_or_default();
        
        let input_params = registry.get_input_specs(plugin_id)
            .map(|specs| specs.iter().map(|(name, spec)| InputParamDef {
                name: name.clone(),
                expected_artifact_type: spec.artifact_type.as_str().to_string(),
                extract_path: spec.extract_path.clone(),
                required: spec.required,
            }).collect())
            .unwrap_or_default();
        
        result.push(PluginPortInfo {
            plugin_id: plugin_id.to_string(),
            output_ports,
            input_params,
        });
    }
    
    Ok(result)
}

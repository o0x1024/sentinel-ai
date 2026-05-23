use chrono::Utc;
use sentinel_db::{
    BountyEvidenceRow, BountyFindingRow, BountyFindingStats, BountyProgramRow, BountySubmissionRow,
    BountySubmissionStats, DatabaseService, FindingQueryFilter, ProgramQueryFilter,
    ProgramScopeRow, SubmissionQueryFilter,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::services::{
    CreateFindingInput, CreateProgramInput, CreateSubmissionInput, FindingService,
    ProgramDbService, SubmissionDbService, UpdateFindingInput, UpdateProgramInput,
    UpdateSubmissionInput,
};

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
pub struct CreateScopesRequest {
    pub scopes: Vec<CreateScopeRequest>,
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
    pub by_platform: HashMap<String, i32>,
    pub by_type: HashMap<String, i32>,
    pub total_submissions: i32,
    pub total_accepted: i32,
    pub total_earnings: f64,
}

pub async fn create_program(
    db: &DatabaseService,
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

    ProgramDbService::create_program(db, input)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_program(
    db: &DatabaseService,
    id: &str,
) -> Result<Option<BountyProgramRow>, String> {
    db.get_bounty_program(id).await.map_err(|e| e.to_string())
}

pub async fn update_program(
    db: &DatabaseService,
    id: &str,
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

    ProgramDbService::update_program(db, id, input)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_program(db: &DatabaseService, id: &str) -> Result<bool, String> {
    ProgramDbService::delete_program(db, id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn list_programs(
    db: &DatabaseService,
    filter: Option<ProgramFilter>,
) -> Result<Vec<BountyProgramRow>, String> {
    let filter = filter.unwrap_or_default();

    db.list_bounty_programs_filtered(ProgramQueryFilter {
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

pub async fn get_program_stats(db: &DatabaseService) -> Result<ProgramStats, String> {
    let stats = db
        .get_bounty_program_stats_live()
        .await
        .map_err(|e| e.to_string())?;

    Ok(ProgramStats {
        total_programs: stats.total_programs,
        active_programs: stats.active_programs,
        by_platform: HashMap::new(),
        by_type: HashMap::new(),
        total_submissions: stats.total_submissions,
        total_accepted: stats.total_accepted,
        total_earnings: stats.total_earnings as f64,
    })
}

fn build_scope_row(request: CreateScopeRequest, now: &str) -> ProgramScopeRow {
    ProgramScopeRow {
        id: Uuid::new_v4().to_string(),
        program_id: request.program_id,
        scope_type: request.scope_type,
        target_type: request.target_type,
        target: request.target,
        description: request.description,
        allowed_tests_json: request
            .allowed_tests
            .map(|tests| serde_json::to_string(&tests).unwrap_or_default()),
        instructions_json: request
            .instructions
            .map(|instructions| serde_json::to_string(&instructions).unwrap_or_default()),
        requires_auth: request.requires_auth.unwrap_or(false),
        test_accounts_json: None,
        asset_count: 0,
        finding_count: 0,
        priority: request.priority.unwrap_or(0.0),
        metadata_json: None,
        created_at: now.to_string(),
        updated_at: now.to_string(),
    }
}

pub async fn create_scope(
    db: &DatabaseService,
    request: CreateScopeRequest,
) -> Result<ProgramScopeRow, String> {
    let now = Utc::now().to_rfc3339();
    let scope = build_scope_row(request, &now);

    db.create_program_scope(&scope)
        .await
        .map_err(|e| e.to_string())?;
    Ok(scope)
}

pub async fn create_scopes(
    db: &DatabaseService,
    request: CreateScopesRequest,
) -> Result<Vec<ProgramScopeRow>, String> {
    if request.scopes.is_empty() {
        return Err("At least one scope is required".to_string());
    }

    let now = Utc::now().to_rfc3339();
    let scopes: Vec<ProgramScopeRow> = request
        .scopes
        .into_iter()
        .map(|scope| build_scope_row(scope, &now))
        .collect();

    db.create_program_scopes(&scopes)
        .await
        .map_err(|e| e.to_string())?;

    Ok(scopes)
}

pub async fn get_scope(db: &DatabaseService, id: &str) -> Result<Option<ProgramScopeRow>, String> {
    db.get_program_scope(id).await.map_err(|e| e.to_string())
}

pub async fn update_scope(
    db: &DatabaseService,
    id: &str,
    request: UpdateScopeRequest,
) -> Result<bool, String> {
    let existing = db.get_program_scope(id).await.map_err(|e| e.to_string())?;

    let Some(mut scope) = existing else {
        return Ok(false);
    };

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

    db.update_program_scope(&scope)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_scope(db: &DatabaseService, id: &str) -> Result<bool, String> {
    db.delete_program_scope(id).await.map_err(|e| e.to_string())
}

pub async fn list_scopes(
    db: &DatabaseService,
    filter: Option<ScopeFilter>,
) -> Result<Vec<ProgramScopeRow>, String> {
    let filter = filter.unwrap_or_default();

    let program_id = filter
        .program_ids
        .as_ref()
        .and_then(|ids| ids.first())
        .map(|id| id.as_str());
    let scope_type = filter
        .scope_types
        .as_ref()
        .and_then(|types| types.first())
        .map(|scope_type| scope_type.as_str());

    db.list_program_scopes(program_id, scope_type)
        .await
        .map_err(|e| e.to_string())
}

pub async fn validate_scope(
    db: &DatabaseService,
    program_id: &str,
    target: &str,
) -> Result<ScopeValidation, String> {
    let scopes = db
        .list_program_scopes(Some(program_id), None)
        .await
        .map_err(|e| e.to_string())?;

    for scope in scopes
        .iter()
        .filter(|scope| scope.scope_type == "out_of_scope")
    {
        if target_matches(&scope.target, &scope.target_type, target) {
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

    for scope in scopes.iter().filter(|scope| scope.scope_type == "in_scope") {
        if target_matches(&scope.target, &scope.target_type, target) {
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

pub async fn create_finding(
    db: &DatabaseService,
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

    FindingService::create_finding(db, input)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_finding(
    db: &DatabaseService,
    id: &str,
) -> Result<Option<BountyFindingRow>, String> {
    db.get_bounty_finding(id).await.map_err(|e| e.to_string())
}

pub async fn update_finding(
    db: &DatabaseService,
    id: &str,
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

    FindingService::update_finding(db, id, input)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_finding(db: &DatabaseService, id: &str) -> Result<bool, String> {
    db.delete_bounty_finding(id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn batch_delete_findings(db: &DatabaseService, ids: Vec<String>) -> Result<u64, String> {
    FindingService::batch_delete_findings(db, ids)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_all_findings(db: &DatabaseService) -> Result<u64, String> {
    db.delete_all_bounty_findings()
        .await
        .map_err(|e| e.to_string())
}

pub async fn batch_update_finding_status(
    db: &DatabaseService,
    ids: Vec<String>,
    status: String,
) -> Result<u64, String> {
    FindingService::batch_update_finding_status(db, ids, status)
        .await
        .map_err(|e| e.to_string())
}

pub async fn list_findings(
    db: &DatabaseService,
    filter: Option<FindingFilter>,
) -> Result<Vec<BountyFindingRow>, String> {
    let filter = filter.unwrap_or_default();

    db.list_bounty_findings_filtered(FindingQueryFilter {
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

pub async fn count_findings(
    db: &DatabaseService,
    filter: Option<FindingFilter>,
) -> Result<i64, String> {
    let filter = filter.unwrap_or_default();

    db.count_bounty_findings_filtered(FindingQueryFilter {
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

pub async fn get_finding_stats(
    db: &DatabaseService,
    program_id: Option<String>,
) -> Result<BountyFindingStats, String> {
    db.get_bounty_finding_stats_live(program_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

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

pub async fn create_evidence(
    db: &DatabaseService,
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
        http_request_json: request
            .http_request
            .map(|request| serde_json::to_string(&request).unwrap_or_default()),
        http_response_json: request
            .http_response
            .map(|response| serde_json::to_string(&response).unwrap_or_default()),
        diff: request.diff,
        tags_json: request
            .tags
            .map(|tags| serde_json::to_string(&tags).unwrap_or_default()),
        metadata_json: None,
        display_order: request.display_order.unwrap_or(0),
        created_at: now.clone(),
        updated_at: now,
    };

    db.create_bounty_evidence(&evidence)
        .await
        .map_err(|e| e.to_string())?;
    Ok(evidence)
}

pub async fn get_evidence(
    db: &DatabaseService,
    id: &str,
) -> Result<Option<BountyEvidenceRow>, String> {
    db.get_bounty_evidence(id).await.map_err(|e| e.to_string())
}

pub async fn update_evidence(
    db: &DatabaseService,
    id: &str,
    request: UpdateEvidenceRequest,
) -> Result<bool, String> {
    let existing = db
        .get_bounty_evidence(id)
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

    db.update_bounty_evidence(&evidence)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_evidence(db: &DatabaseService, id: &str) -> Result<bool, String> {
    db.delete_bounty_evidence(id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn list_evidence(
    db: &DatabaseService,
    finding_id: &str,
) -> Result<Vec<BountyEvidenceRow>, String> {
    db.list_bounty_evidence(finding_id)
        .await
        .map_err(|e| e.to_string())
}

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

pub async fn create_submission(
    db: &DatabaseService,
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

    SubmissionDbService::create_submission(db, input)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_submission(
    db: &DatabaseService,
    id: &str,
) -> Result<Option<BountySubmissionRow>, String> {
    db.get_bounty_submission(id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn update_submission(
    db: &DatabaseService,
    id: &str,
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

    SubmissionDbService::update_submission(db, id, input)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_submission(db: &DatabaseService, id: &str) -> Result<bool, String> {
    db.delete_bounty_submission(id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn batch_delete_submissions(
    db: &DatabaseService,
    ids: Vec<String>,
) -> Result<u64, String> {
    SubmissionDbService::batch_delete_submissions(db, ids)
        .await
        .map_err(|e| e.to_string())
}

pub async fn batch_update_submission_status(
    db: &DatabaseService,
    ids: Vec<String>,
    status: String,
) -> Result<u64, String> {
    SubmissionDbService::batch_update_submission_status(db, ids, status)
        .await
        .map_err(|e| e.to_string())
}

pub async fn list_submissions(
    db: &DatabaseService,
    filter: Option<SubmissionFilter>,
) -> Result<Vec<BountySubmissionRow>, String> {
    let filter = filter.unwrap_or_default();

    db.list_bounty_submissions_filtered(SubmissionQueryFilter {
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

pub async fn count_submissions(
    db: &DatabaseService,
    filter: Option<SubmissionFilter>,
) -> Result<i64, String> {
    let filter = filter.unwrap_or_default();

    db.count_bounty_submissions_filtered(SubmissionQueryFilter {
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

pub async fn get_submission_stats(
    db: &DatabaseService,
    program_id: Option<String>,
) -> Result<BountySubmissionStats, String> {
    db.get_bounty_submission_stats_live(program_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

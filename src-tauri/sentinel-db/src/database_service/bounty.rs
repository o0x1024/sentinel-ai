//! Bug Bounty database operations

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::info;

use super::service::DatabaseService;

// ============================================================================
// Database Models
// ============================================================================

/// Bug Bounty Program database model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountyProgramRow {
    pub id: String,
    pub name: String,
    pub organization: String,
    pub platform: String,
    pub platform_handle: Option<String>,
    pub url: Option<String>,
    pub program_type: String,
    pub status: String,
    pub description: Option<String>,
    pub rewards_json: Option<String>,
    pub response_sla_days: Option<i32>,
    pub resolution_sla_days: Option<i32>,
    pub rules_json: Option<String>,
    pub tags_json: Option<String>,
    pub metadata_json: Option<String>,
    pub priority_score: f64,
    pub total_submissions: i32,
    pub accepted_submissions: i32,
    pub total_earnings: f64,
    pub created_at: String,
    pub updated_at: String,
    pub last_activity_at: Option<String>,
}

/// Program Scope database model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProgramScopeRow {
    pub id: String,
    pub program_id: String,
    pub scope_type: String,
    pub target_type: String,
    pub target: String,
    pub description: Option<String>,
    pub allowed_tests_json: Option<String>,
    pub instructions_json: Option<String>,
    pub requires_auth: bool,
    pub test_accounts_json: Option<String>,
    pub asset_count: i32,
    pub finding_count: i32,
    pub priority: f64,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Bounty Finding database model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountyFindingRow {
    pub id: String,
    pub program_id: String,
    pub scope_id: Option<String>,
    pub asset_id: Option<String>,
    pub title: String,
    pub description: String,
    pub finding_type: String,
    pub severity: String,
    pub status: String,
    pub confidence: String,
    pub cvss_score: Option<f64>,
    pub cwe_id: Option<String>,
    pub affected_url: Option<String>,
    pub affected_parameter: Option<String>,
    pub reproduction_steps_json: Option<String>,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub evidence_ids_json: Option<String>,
    pub tags_json: Option<String>,
    pub metadata_json: Option<String>,
    pub fingerprint: String,
    pub duplicate_of: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: String,
}

/// Bounty Submission database model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountySubmissionRow {
    pub id: String,
    pub program_id: String,
    pub finding_id: String,
    pub platform_submission_id: Option<String>,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub vulnerability_type: String,
    pub severity: String,
    pub cvss_score: Option<f64>,
    pub cwe_id: Option<String>,
    pub description: String,
    pub reproduction_steps_json: Option<String>,
    pub impact: String,
    pub remediation: Option<String>,
    pub evidence_ids_json: Option<String>,
    pub platform_url: Option<String>,
    pub reward_amount: Option<f64>,
    pub reward_currency: Option<String>,
    pub bonus_amount: Option<f64>,
    pub response_time_hours: Option<i32>,
    pub resolution_time_hours: Option<i32>,
    pub requires_retest: bool,
    pub retest_at: Option<String>,
    pub last_retest_at: Option<String>,
    pub communications_json: Option<String>,
    pub timeline_json: Option<String>,
    pub tags_json: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub submitted_at: Option<String>,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub created_by: String,
}

// ============================================================================
// Database Operations
// ============================================================================

impl DatabaseService {
    // ------------------------------------------------------------------------
    // Program CRUD
    // ------------------------------------------------------------------------

    /// Create a new bounty program
    pub async fn create_bounty_program(&self, program: &BountyProgramRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO bounty_programs (
                id, name, organization, platform, platform_handle, url,
                program_type, status, description, rewards_json,
                response_sla_days, resolution_sla_days, rules_json, tags_json,
                metadata_json, priority_score, total_submissions, accepted_submissions,
                total_earnings, created_at, updated_at, last_activity_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&program.id)
        .bind(&program.name)
        .bind(&program.organization)
        .bind(&program.platform)
        .bind(&program.platform_handle)
        .bind(&program.url)
        .bind(&program.program_type)
        .bind(&program.status)
        .bind(&program.description)
        .bind(&program.rewards_json)
        .bind(program.response_sla_days)
        .bind(program.resolution_sla_days)
        .bind(&program.rules_json)
        .bind(&program.tags_json)
        .bind(&program.metadata_json)
        .bind(program.priority_score)
        .bind(program.total_submissions)
        .bind(program.accepted_submissions)
        .bind(program.total_earnings)
        .bind(&program.created_at)
        .bind(&program.updated_at)
        .bind(&program.last_activity_at)
        .execute(self.pool())
        .await?;

        info!("Created bounty program: {}", program.id);
        Ok(())
    }

    /// Get a bounty program by ID
    pub async fn get_bounty_program(&self, id: &str) -> Result<Option<BountyProgramRow>> {
        let row = sqlx::query_as::<_, BountyProgramRow>(
            "SELECT * FROM bounty_programs WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row)
    }

    /// Update a bounty program
    pub async fn update_bounty_program(&self, program: &BountyProgramRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_programs SET
                name = ?, organization = ?, platform = ?, platform_handle = ?,
                url = ?, program_type = ?, status = ?, description = ?,
                rewards_json = ?, response_sla_days = ?, resolution_sla_days = ?,
                rules_json = ?, tags_json = ?, metadata_json = ?, priority_score = ?,
                total_submissions = ?, accepted_submissions = ?, total_earnings = ?,
                updated_at = ?, last_activity_at = ?
            WHERE id = ?"#
        )
        .bind(&program.name)
        .bind(&program.organization)
        .bind(&program.platform)
        .bind(&program.platform_handle)
        .bind(&program.url)
        .bind(&program.program_type)
        .bind(&program.status)
        .bind(&program.description)
        .bind(&program.rewards_json)
        .bind(program.response_sla_days)
        .bind(program.resolution_sla_days)
        .bind(&program.rules_json)
        .bind(&program.tags_json)
        .bind(&program.metadata_json)
        .bind(program.priority_score)
        .bind(program.total_submissions)
        .bind(program.accepted_submissions)
        .bind(program.total_earnings)
        .bind(&program.updated_at)
        .bind(&program.last_activity_at)
        .bind(&program.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty program
    pub async fn delete_bounty_program(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_programs WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List bounty programs with optional filtering
    pub async fn list_bounty_programs(
        &self,
        platforms: Option<&[String]>,
        statuses: Option<&[String]>,
        search: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<BountyProgramRow>> {
        let mut query = String::from("SELECT * FROM bounty_programs WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(platforms) = platforms {
            if !platforms.is_empty() {
                let placeholders = platforms.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                query.push_str(&format!(" AND platform IN ({})", placeholders));
                params.extend(platforms.iter().cloned());
            }
        }

        if let Some(statuses) = statuses {
            if !statuses.is_empty() {
                let placeholders = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                query.push_str(&format!(" AND status IN ({})", placeholders));
                params.extend(statuses.iter().cloned());
            }
        }

        if let Some(search) = search {
            if !search.is_empty() {
                query.push_str(" AND (name LIKE ? OR organization LIKE ?)");
                let search_pattern = format!("%{}%", search);
                params.push(search_pattern.clone());
                params.push(search_pattern);
            }
        }

        query.push_str(" ORDER BY priority_score DESC, updated_at DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        // Build dynamic query
        let mut sqlx_query = sqlx::query_as::<_, BountyProgramRow>(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param);
        }

        let rows = sqlx_query.fetch_all(self.pool()).await?;
        Ok(rows)
    }

    /// Get bounty program statistics
    pub async fn get_bounty_program_stats(&self) -> Result<BountyProgramStats> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bounty_programs")
            .fetch_one(self.pool())
            .await?;

        let active: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM bounty_programs WHERE status = 'active'"
        )
        .fetch_one(self.pool())
        .await?;

        let totals: (i64, i64, f64) = sqlx::query_as(
            "SELECT COALESCE(SUM(total_submissions), 0), COALESCE(SUM(accepted_submissions), 0), COALESCE(SUM(total_earnings), 0.0) FROM bounty_programs"
        )
        .fetch_one(self.pool())
        .await?;

        Ok(BountyProgramStats {
            total_programs: total.0 as i32,
            active_programs: active.0 as i32,
            total_submissions: totals.0 as i32,
            total_accepted: totals.1 as i32,
            total_earnings: totals.2,
        })
    }

    // ------------------------------------------------------------------------
    // Scope CRUD
    // ------------------------------------------------------------------------

    /// Create a new program scope
    pub async fn create_program_scope(&self, scope: &ProgramScopeRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO bounty_scopes (
                id, program_id, scope_type, target_type, target, description,
                allowed_tests_json, instructions_json, requires_auth, test_accounts_json,
                asset_count, finding_count, priority, metadata_json, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&scope.id)
        .bind(&scope.program_id)
        .bind(&scope.scope_type)
        .bind(&scope.target_type)
        .bind(&scope.target)
        .bind(&scope.description)
        .bind(&scope.allowed_tests_json)
        .bind(&scope.instructions_json)
        .bind(scope.requires_auth)
        .bind(&scope.test_accounts_json)
        .bind(scope.asset_count)
        .bind(scope.finding_count)
        .bind(scope.priority)
        .bind(&scope.metadata_json)
        .bind(&scope.created_at)
        .bind(&scope.updated_at)
        .execute(self.pool())
        .await?;

        info!("Created program scope: {}", scope.id);
        Ok(())
    }

    /// Get a program scope by ID
    pub async fn get_program_scope(&self, id: &str) -> Result<Option<ProgramScopeRow>> {
        let row = sqlx::query_as::<_, ProgramScopeRow>(
            "SELECT * FROM bounty_scopes WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row)
    }

    /// Update a program scope
    pub async fn update_program_scope(&self, scope: &ProgramScopeRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_scopes SET
                scope_type = ?, target_type = ?, target = ?, description = ?,
                allowed_tests_json = ?, instructions_json = ?, requires_auth = ?,
                test_accounts_json = ?, asset_count = ?, finding_count = ?,
                priority = ?, metadata_json = ?, updated_at = ?
            WHERE id = ?"#
        )
        .bind(&scope.scope_type)
        .bind(&scope.target_type)
        .bind(&scope.target)
        .bind(&scope.description)
        .bind(&scope.allowed_tests_json)
        .bind(&scope.instructions_json)
        .bind(scope.requires_auth)
        .bind(&scope.test_accounts_json)
        .bind(scope.asset_count)
        .bind(scope.finding_count)
        .bind(scope.priority)
        .bind(&scope.metadata_json)
        .bind(&scope.updated_at)
        .bind(&scope.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a program scope
    pub async fn delete_program_scope(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_scopes WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List scopes for a program
    pub async fn list_program_scopes(
        &self,
        program_id: Option<&str>,
        scope_type: Option<&str>,
    ) -> Result<Vec<ProgramScopeRow>> {
        let mut query = String::from("SELECT * FROM bounty_scopes WHERE 1=1");

        if let Some(pid) = program_id {
            query.push_str(&format!(" AND program_id = '{}'", pid));
        }

        if let Some(st) = scope_type {
            query.push_str(&format!(" AND scope_type = '{}'", st));
        }

        query.push_str(" ORDER BY priority DESC, created_at DESC");

        let rows = sqlx::query_as::<_, ProgramScopeRow>(&query)
            .fetch_all(self.pool())
            .await?;

        Ok(rows)
    }
}

/// Program statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BountyProgramStats {
    pub total_programs: i32,
    pub active_programs: i32,
    pub total_submissions: i32,
    pub total_accepted: i32,
    pub total_earnings: f64,
}

/// Bounty Evidence database model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountyEvidenceRow {
    pub id: String,
    pub finding_id: String,
    pub evidence_type: String,
    pub title: String,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub file_url: Option<String>,
    pub content: Option<String>,
    pub mime_type: Option<String>,
    pub file_size: Option<i64>,
    pub http_request_json: Option<String>,
    pub http_response_json: Option<String>,
    pub diff: Option<String>,
    pub tags_json: Option<String>,
    pub metadata_json: Option<String>,
    pub display_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl DatabaseService {
    // ------------------------------------------------------------------------
    // Finding CRUD
    // ------------------------------------------------------------------------

    /// Create a new bounty finding
    pub async fn create_bounty_finding(&self, finding: &BountyFindingRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO bounty_findings (
                id, program_id, scope_id, asset_id, title, description, finding_type,
                severity, status, confidence, cvss_score, cwe_id, affected_url,
                affected_parameter, reproduction_steps_json, impact, remediation,
                evidence_ids_json, tags_json, metadata_json, fingerprint, duplicate_of,
                first_seen_at, last_seen_at, verified_at, created_at, updated_at, created_by
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&finding.id)
        .bind(&finding.program_id)
        .bind(&finding.scope_id)
        .bind(&finding.asset_id)
        .bind(&finding.title)
        .bind(&finding.description)
        .bind(&finding.finding_type)
        .bind(&finding.severity)
        .bind(&finding.status)
        .bind(&finding.confidence)
        .bind(finding.cvss_score)
        .bind(&finding.cwe_id)
        .bind(&finding.affected_url)
        .bind(&finding.affected_parameter)
        .bind(&finding.reproduction_steps_json)
        .bind(&finding.impact)
        .bind(&finding.remediation)
        .bind(&finding.evidence_ids_json)
        .bind(&finding.tags_json)
        .bind(&finding.metadata_json)
        .bind(&finding.fingerprint)
        .bind(&finding.duplicate_of)
        .bind(&finding.first_seen_at)
        .bind(&finding.last_seen_at)
        .bind(&finding.verified_at)
        .bind(&finding.created_at)
        .bind(&finding.updated_at)
        .bind(&finding.created_by)
        .execute(self.pool())
        .await?;

        info!("Created bounty finding: {}", finding.id);
        Ok(())
    }

    /// Get a bounty finding by ID
    pub async fn get_bounty_finding(&self, id: &str) -> Result<Option<BountyFindingRow>> {
        let row = sqlx::query_as::<_, BountyFindingRow>(
            "SELECT * FROM bounty_findings WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row)
    }

    /// Get a bounty finding by fingerprint (for deduplication)
    pub async fn get_bounty_finding_by_fingerprint(&self, fingerprint: &str) -> Result<Option<BountyFindingRow>> {
        let row = sqlx::query_as::<_, BountyFindingRow>(
            "SELECT * FROM bounty_findings WHERE fingerprint = ?"
        )
        .bind(fingerprint)
        .fetch_optional(self.pool())
        .await?;

        Ok(row)
    }

    /// Update a bounty finding
    pub async fn update_bounty_finding(&self, finding: &BountyFindingRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_findings SET
                scope_id = ?, asset_id = ?, title = ?, description = ?, finding_type = ?,
                severity = ?, status = ?, confidence = ?, cvss_score = ?, cwe_id = ?,
                affected_url = ?, affected_parameter = ?, reproduction_steps_json = ?,
                impact = ?, remediation = ?, evidence_ids_json = ?, tags_json = ?,
                metadata_json = ?, duplicate_of = ?, last_seen_at = ?, verified_at = ?,
                updated_at = ?
            WHERE id = ?"#
        )
        .bind(&finding.scope_id)
        .bind(&finding.asset_id)
        .bind(&finding.title)
        .bind(&finding.description)
        .bind(&finding.finding_type)
        .bind(&finding.severity)
        .bind(&finding.status)
        .bind(&finding.confidence)
        .bind(finding.cvss_score)
        .bind(&finding.cwe_id)
        .bind(&finding.affected_url)
        .bind(&finding.affected_parameter)
        .bind(&finding.reproduction_steps_json)
        .bind(&finding.impact)
        .bind(&finding.remediation)
        .bind(&finding.evidence_ids_json)
        .bind(&finding.tags_json)
        .bind(&finding.metadata_json)
        .bind(&finding.duplicate_of)
        .bind(&finding.last_seen_at)
        .bind(&finding.verified_at)
        .bind(&finding.updated_at)
        .bind(&finding.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty finding
    pub async fn delete_bounty_finding(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_findings WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List bounty findings with optional filtering
    pub async fn list_bounty_findings(
        &self,
        program_id: Option<&str>,
        scope_id: Option<&str>,
        severities: Option<&[String]>,
        statuses: Option<&[String]>,
        search: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<BountyFindingRow>> {
        let mut query = String::from("SELECT * FROM bounty_findings WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(pid) = program_id {
            query.push_str(" AND program_id = ?");
            params.push(pid.to_string());
        }

        if let Some(sid) = scope_id {
            query.push_str(" AND scope_id = ?");
            params.push(sid.to_string());
        }

        if let Some(severities) = severities {
            if !severities.is_empty() {
                let placeholders = severities.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                query.push_str(&format!(" AND severity IN ({})", placeholders));
                params.extend(severities.iter().cloned());
            }
        }

        if let Some(statuses) = statuses {
            if !statuses.is_empty() {
                let placeholders = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                query.push_str(&format!(" AND status IN ({})", placeholders));
                params.extend(statuses.iter().cloned());
            }
        }

        if let Some(search) = search {
            if !search.is_empty() {
                query.push_str(" AND (title LIKE ? OR description LIKE ?)");
                let search_pattern = format!("%{}%", search);
                params.push(search_pattern.clone());
                params.push(search_pattern);
            }
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut sqlx_query = sqlx::query_as::<_, BountyFindingRow>(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param);
        }

        let rows = sqlx_query.fetch_all(self.pool()).await?;
        Ok(rows)
    }

    /// Get bounty finding statistics
    pub async fn get_bounty_finding_stats(&self, program_id: Option<&str>) -> Result<BountyFindingStats> {
        let base_where = program_id.map(|_| "WHERE program_id = ?").unwrap_or("");
        
        let total_query = format!("SELECT COUNT(*) FROM bounty_findings {}", base_where);
        let mut q = sqlx::query_as::<_, (i64,)>(&total_query);
        if let Some(pid) = program_id {
            q = q.bind(pid);
        }
        let total: (i64,) = q.fetch_one(self.pool()).await?;

        let by_severity_query = format!(
            "SELECT severity, COUNT(*) FROM bounty_findings {} GROUP BY severity",
            base_where
        );
        let mut q = sqlx::query_as::<_, (String, i64)>(&by_severity_query);
        if let Some(pid) = program_id {
            q = q.bind(pid);
        }
        let severity_rows: Vec<(String, i64)> = q.fetch_all(self.pool()).await?;
        let by_severity: std::collections::HashMap<String, i32> = severity_rows
            .into_iter()
            .map(|(k, v)| (k, v as i32))
            .collect();

        let by_status_query = format!(
            "SELECT status, COUNT(*) FROM bounty_findings {} GROUP BY status",
            base_where
        );
        let mut q = sqlx::query_as::<_, (String, i64)>(&by_status_query);
        if let Some(pid) = program_id {
            q = q.bind(pid);
        }
        let status_rows: Vec<(String, i64)> = q.fetch_all(self.pool()).await?;
        let by_status: std::collections::HashMap<String, i32> = status_rows
            .into_iter()
            .map(|(k, v)| (k, v as i32))
            .collect();

        Ok(BountyFindingStats {
            total_findings: total.0 as i32,
            by_severity,
            by_status,
        })
    }

    // ------------------------------------------------------------------------
    // Evidence CRUD
    // ------------------------------------------------------------------------

    /// Create a new bounty evidence
    pub async fn create_bounty_evidence(&self, evidence: &BountyEvidenceRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO bounty_evidence (
                id, finding_id, evidence_type, title, description, file_path, file_url,
                content, mime_type, file_size, http_request_json, http_response_json,
                diff, tags_json, metadata_json, display_order, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&evidence.id)
        .bind(&evidence.finding_id)
        .bind(&evidence.evidence_type)
        .bind(&evidence.title)
        .bind(&evidence.description)
        .bind(&evidence.file_path)
        .bind(&evidence.file_url)
        .bind(&evidence.content)
        .bind(&evidence.mime_type)
        .bind(evidence.file_size)
        .bind(&evidence.http_request_json)
        .bind(&evidence.http_response_json)
        .bind(&evidence.diff)
        .bind(&evidence.tags_json)
        .bind(&evidence.metadata_json)
        .bind(evidence.display_order)
        .bind(&evidence.created_at)
        .bind(&evidence.updated_at)
        .execute(self.pool())
        .await?;

        info!("Created bounty evidence: {}", evidence.id);
        Ok(())
    }

    /// Get a bounty evidence by ID
    pub async fn get_bounty_evidence(&self, id: &str) -> Result<Option<BountyEvidenceRow>> {
        let row = sqlx::query_as::<_, BountyEvidenceRow>(
            "SELECT * FROM bounty_evidence WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row)
    }

    /// Update a bounty evidence
    pub async fn update_bounty_evidence(&self, evidence: &BountyEvidenceRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_evidence SET
                evidence_type = ?, title = ?, description = ?, file_path = ?,
                file_url = ?, content = ?, mime_type = ?, file_size = ?,
                http_request_json = ?, http_response_json = ?, diff = ?,
                tags_json = ?, metadata_json = ?, display_order = ?, updated_at = ?
            WHERE id = ?"#
        )
        .bind(&evidence.evidence_type)
        .bind(&evidence.title)
        .bind(&evidence.description)
        .bind(&evidence.file_path)
        .bind(&evidence.file_url)
        .bind(&evidence.content)
        .bind(&evidence.mime_type)
        .bind(evidence.file_size)
        .bind(&evidence.http_request_json)
        .bind(&evidence.http_response_json)
        .bind(&evidence.diff)
        .bind(&evidence.tags_json)
        .bind(&evidence.metadata_json)
        .bind(evidence.display_order)
        .bind(&evidence.updated_at)
        .bind(&evidence.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty evidence
    pub async fn delete_bounty_evidence(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_evidence WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List evidence for a finding
    pub async fn list_bounty_evidence(&self, finding_id: &str) -> Result<Vec<BountyEvidenceRow>> {
        let rows = sqlx::query_as::<_, BountyEvidenceRow>(
            "SELECT * FROM bounty_evidence WHERE finding_id = ? ORDER BY display_order, created_at"
        )
        .bind(finding_id)
        .fetch_all(self.pool())
        .await?;

        Ok(rows)
    }

    // ------------------------------------------------------------------------
    // Submission CRUD
    // ------------------------------------------------------------------------

    /// Create a new bounty submission
    pub async fn create_bounty_submission(&self, submission: &BountySubmissionRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO bounty_submissions (
                id, program_id, finding_id, platform_submission_id, title, status,
                priority, vulnerability_type, severity, cvss_score, cwe_id, description,
                reproduction_steps_json, impact, remediation, evidence_ids_json, platform_url,
                reward_amount, reward_currency, bonus_amount, response_time_hours,
                resolution_time_hours, requires_retest, retest_at, last_retest_at,
                communications_json, timeline_json, tags_json, metadata_json, created_at, submitted_at,
                updated_at, closed_at, created_by
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&submission.id)
        .bind(&submission.program_id)
        .bind(&submission.finding_id)
        .bind(&submission.platform_submission_id)
        .bind(&submission.title)
        .bind(&submission.status)
        .bind(&submission.priority)
        .bind(&submission.vulnerability_type)
        .bind(&submission.severity)
        .bind(submission.cvss_score)
        .bind(&submission.cwe_id)
        .bind(&submission.description)
        .bind(&submission.reproduction_steps_json)
        .bind(&submission.impact)
        .bind(&submission.remediation)
        .bind(&submission.evidence_ids_json)
        .bind(&submission.platform_url)
        .bind(submission.reward_amount)
        .bind(&submission.reward_currency)
        .bind(submission.bonus_amount)
        .bind(submission.response_time_hours)
        .bind(submission.resolution_time_hours)
        .bind(submission.requires_retest)
        .bind(&submission.retest_at)
        .bind(&submission.last_retest_at)
        .bind(&submission.communications_json)
        .bind(&submission.timeline_json)
        .bind(&submission.tags_json)
        .bind(&submission.metadata_json)
        .bind(&submission.created_at)
        .bind(&submission.submitted_at)
        .bind(&submission.updated_at)
        .bind(&submission.closed_at)
        .bind(&submission.created_by)
        .execute(self.pool())
        .await?;

        info!("Created bounty submission: {}", submission.id);
        Ok(())
    }

    /// Get a bounty submission by ID
    pub async fn get_bounty_submission(&self, id: &str) -> Result<Option<BountySubmissionRow>> {
        let row = sqlx::query_as::<_, BountySubmissionRow>(
            "SELECT * FROM bounty_submissions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row)
    }

    /// Update a bounty submission
    pub async fn update_bounty_submission(&self, submission: &BountySubmissionRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_submissions SET
                platform_submission_id = ?, title = ?, status = ?, priority = ?,
                vulnerability_type = ?, severity = ?, cvss_score = ?, cwe_id = ?,
                description = ?, reproduction_steps_json = ?, impact = ?, remediation = ?,
                evidence_ids_json = ?, platform_url = ?, reward_amount = ?, reward_currency = ?,
                bonus_amount = ?, response_time_hours = ?, resolution_time_hours = ?,
                requires_retest = ?, retest_at = ?, last_retest_at = ?, communications_json = ?,
                timeline_json = ?, tags_json = ?, metadata_json = ?, submitted_at = ?, updated_at = ?, closed_at = ?
            WHERE id = ?"#
        )
        .bind(&submission.platform_submission_id)
        .bind(&submission.title)
        .bind(&submission.status)
        .bind(&submission.priority)
        .bind(&submission.vulnerability_type)
        .bind(&submission.severity)
        .bind(submission.cvss_score)
        .bind(&submission.cwe_id)
        .bind(&submission.description)
        .bind(&submission.reproduction_steps_json)
        .bind(&submission.impact)
        .bind(&submission.remediation)
        .bind(&submission.evidence_ids_json)
        .bind(&submission.platform_url)
        .bind(submission.reward_amount)
        .bind(&submission.reward_currency)
        .bind(submission.bonus_amount)
        .bind(submission.response_time_hours)
        .bind(submission.resolution_time_hours)
        .bind(submission.requires_retest)
        .bind(&submission.retest_at)
        .bind(&submission.last_retest_at)
        .bind(&submission.communications_json)
        .bind(&submission.timeline_json)
        .bind(&submission.tags_json)
        .bind(&submission.metadata_json)
        .bind(&submission.submitted_at)
        .bind(&submission.updated_at)
        .bind(&submission.closed_at)
        .bind(&submission.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty submission
    pub async fn delete_bounty_submission(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_submissions WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List bounty submissions with optional filtering
    pub async fn list_bounty_submissions(
        &self,
        program_id: Option<&str>,
        finding_id: Option<&str>,
        statuses: Option<&[String]>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<BountySubmissionRow>> {
        let mut query = String::from("SELECT * FROM bounty_submissions WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(pid) = program_id {
            query.push_str(" AND program_id = ?");
            params.push(pid.to_string());
        }

        if let Some(fid) = finding_id {
            query.push_str(" AND finding_id = ?");
            params.push(fid.to_string());
        }

        if let Some(statuses) = statuses {
            if !statuses.is_empty() {
                let placeholders = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                query.push_str(&format!(" AND status IN ({})", placeholders));
                params.extend(statuses.iter().cloned());
            }
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut sqlx_query = sqlx::query_as::<_, BountySubmissionRow>(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param);
        }

        let rows = sqlx_query.fetch_all(self.pool()).await?;
        Ok(rows)
    }

    /// Get bounty submission statistics
    pub async fn get_bounty_submission_stats(&self, program_id: Option<&str>) -> Result<BountySubmissionStats> {
        let base_where = program_id.map(|_| "WHERE program_id = ?").unwrap_or("");
        
        let total_query = format!("SELECT COUNT(*) FROM bounty_submissions {}", base_where);
        let mut q = sqlx::query_as::<_, (i64,)>(&total_query);
        if let Some(pid) = program_id {
            q = q.bind(pid);
        }
        let total: (i64,) = q.fetch_one(self.pool()).await?;

        let accepted_query = format!(
            "SELECT COUNT(*) FROM bounty_submissions {} {} status IN ('accepted', 'resolved', 'paid')",
            base_where,
            if program_id.is_some() { "AND" } else { "WHERE" }
        );
        let mut q = sqlx::query_as::<_, (i64,)>(&accepted_query);
        if let Some(pid) = program_id {
            q = q.bind(pid);
        }
        let accepted: (i64,) = q.fetch_one(self.pool()).await?;

        let earnings_query = format!(
            "SELECT COALESCE(SUM(reward_amount), 0.0), COALESCE(SUM(bonus_amount), 0.0) FROM bounty_submissions {}",
            base_where
        );
        let mut q = sqlx::query_as::<_, (f64, f64)>(&earnings_query);
        if let Some(pid) = program_id {
            q = q.bind(pid);
        }
        let earnings: (f64, f64) = q.fetch_one(self.pool()).await?;

        Ok(BountySubmissionStats {
            total_submissions: total.0 as i32,
            accepted_submissions: accepted.0 as i32,
            total_rewards: earnings.0,
            total_bonuses: earnings.1,
        })
    }
}

/// Finding statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BountyFindingStats {
    pub total_findings: i32,
    pub by_severity: std::collections::HashMap<String, i32>,
    pub by_status: std::collections::HashMap<String, i32>,
}

/// Submission statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BountySubmissionStats {
    pub total_submissions: i32,
    pub accepted_submissions: i32,
    pub total_rewards: f64,
    pub total_bonuses: f64,
}

// ============================================================================
// Change Event Models
// ============================================================================

/// Change Event database model for ASM monitoring
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountyChangeEventRow {
    pub id: String,
    pub program_id: Option<String>,
    pub asset_id: String,
    pub event_type: String,
    pub severity: String,
    pub status: String,
    pub title: String,
    pub description: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub diff: Option<String>,
    pub affected_scope: Option<String>,
    pub detection_method: String,
    pub triggered_workflows_json: Option<String>,
    pub generated_findings_json: Option<String>,
    pub tags_json: Option<String>,
    pub metadata_json: Option<String>,
    pub risk_score: f64,
    pub auto_trigger_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub resolved_at: Option<String>,
}

/// Change event statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BountyChangeEventStats {
    pub total_events: i32,
    pub by_type: std::collections::HashMap<String, i32>,
    pub by_severity: std::collections::HashMap<String, i32>,
    pub by_status: std::collections::HashMap<String, i32>,
    pub pending_review: i32,
    pub average_risk_score: f64,
}

impl DatabaseService {
    // ------------------------------------------------------------------------
    // Change Event CRUD
    // ------------------------------------------------------------------------

    /// Create a new change event
    pub async fn create_bounty_change_event(&self, event: &BountyChangeEventRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO bounty_change_events (
                id, program_id, asset_id, event_type, severity, status, title, description,
                old_value, new_value, diff, affected_scope, detection_method,
                triggered_workflows_json, generated_findings_json, tags_json, metadata_json,
                risk_score, auto_trigger_enabled, created_at, updated_at, resolved_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&event.id)
        .bind(&event.program_id)
        .bind(&event.asset_id)
        .bind(&event.event_type)
        .bind(&event.severity)
        .bind(&event.status)
        .bind(&event.title)
        .bind(&event.description)
        .bind(&event.old_value)
        .bind(&event.new_value)
        .bind(&event.diff)
        .bind(&event.affected_scope)
        .bind(&event.detection_method)
        .bind(&event.triggered_workflows_json)
        .bind(&event.generated_findings_json)
        .bind(&event.tags_json)
        .bind(&event.metadata_json)
        .bind(event.risk_score)
        .bind(event.auto_trigger_enabled)
        .bind(&event.created_at)
        .bind(&event.updated_at)
        .bind(&event.resolved_at)
        .execute(self.pool())
        .await?;

        info!("Created bounty change event: {}", event.id);
        Ok(())
    }

    /// Get a change event by ID
    pub async fn get_bounty_change_event(&self, id: &str) -> Result<Option<BountyChangeEventRow>> {
        let row = sqlx::query_as::<_, BountyChangeEventRow>(
            "SELECT * FROM bounty_change_events WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row)
    }

    /// Update a change event
    pub async fn update_bounty_change_event(&self, event: &BountyChangeEventRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_change_events SET
                program_id = ?, asset_id = ?, event_type = ?, severity = ?, status = ?,
                title = ?, description = ?, old_value = ?, new_value = ?, diff = ?,
                affected_scope = ?, detection_method = ?, triggered_workflows_json = ?,
                generated_findings_json = ?, tags_json = ?, metadata_json = ?,
                risk_score = ?, auto_trigger_enabled = ?, updated_at = ?, resolved_at = ?
            WHERE id = ?"#
        )
        .bind(&event.program_id)
        .bind(&event.asset_id)
        .bind(&event.event_type)
        .bind(&event.severity)
        .bind(&event.status)
        .bind(&event.title)
        .bind(&event.description)
        .bind(&event.old_value)
        .bind(&event.new_value)
        .bind(&event.diff)
        .bind(&event.affected_scope)
        .bind(&event.detection_method)
        .bind(&event.triggered_workflows_json)
        .bind(&event.generated_findings_json)
        .bind(&event.tags_json)
        .bind(&event.metadata_json)
        .bind(event.risk_score)
        .bind(event.auto_trigger_enabled)
        .bind(&event.updated_at)
        .bind(&event.resolved_at)
        .bind(&event.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a change event
    pub async fn delete_bounty_change_event(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_change_events WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List change events with optional filtering
    pub async fn list_bounty_change_events(
        &self,
        program_id: Option<&str>,
        asset_id: Option<&str>,
        event_types: Option<&[String]>,
        severities: Option<&[String]>,
        statuses: Option<&[String]>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<BountyChangeEventRow>> {
        let mut query = String::from("SELECT * FROM bounty_change_events WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(pid) = program_id {
            query.push_str(" AND program_id = ?");
            params.push(pid.to_string());
        }

        if let Some(aid) = asset_id {
            query.push_str(" AND asset_id = ?");
            params.push(aid.to_string());
        }

        if let Some(types) = event_types {
            if !types.is_empty() {
                let placeholders = types.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                query.push_str(&format!(" AND event_type IN ({})", placeholders));
                params.extend(types.iter().cloned());
            }
        }

        if let Some(sevs) = severities {
            if !sevs.is_empty() {
                let placeholders = sevs.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                query.push_str(&format!(" AND severity IN ({})", placeholders));
                params.extend(sevs.iter().cloned());
            }
        }

        if let Some(stats) = statuses {
            if !stats.is_empty() {
                let placeholders = stats.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                query.push_str(&format!(" AND status IN ({})", placeholders));
                params.extend(stats.iter().cloned());
            }
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut sqlx_query = sqlx::query_as::<_, BountyChangeEventRow>(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param);
        }

        let rows = sqlx_query.fetch_all(self.pool()).await?;
        Ok(rows)
    }

    /// Get change event statistics
    pub async fn get_bounty_change_event_stats(&self, program_id: Option<&str>) -> Result<BountyChangeEventStats> {
        let base_where = program_id.map(|_| "WHERE program_id = ?").unwrap_or("");
        
        // Total events
        let total_query = format!("SELECT COUNT(*) FROM bounty_change_events {}", base_where);
        let mut q = sqlx::query_as::<_, (i64,)>(&total_query);
        if let Some(pid) = program_id {
            q = q.bind(pid);
        }
        let total: (i64,) = q.fetch_one(self.pool()).await?;

        // By type
        let by_type_query = format!(
            "SELECT event_type, COUNT(*) FROM bounty_change_events {} GROUP BY event_type",
            base_where
        );
        let mut q = sqlx::query_as::<_, (String, i64)>(&by_type_query);
        if let Some(pid) = program_id {
            q = q.bind(pid);
        }
        let type_rows: Vec<(String, i64)> = q.fetch_all(self.pool()).await?;
        let by_type: std::collections::HashMap<String, i32> = type_rows
            .into_iter()
            .map(|(k, v)| (k, v as i32))
            .collect();

        // By severity
        let by_severity_query = format!(
            "SELECT severity, COUNT(*) FROM bounty_change_events {} GROUP BY severity",
            base_where
        );
        let mut q = sqlx::query_as::<_, (String, i64)>(&by_severity_query);
        if let Some(pid) = program_id {
            q = q.bind(pid);
        }
        let severity_rows: Vec<(String, i64)> = q.fetch_all(self.pool()).await?;
        let by_severity: std::collections::HashMap<String, i32> = severity_rows
            .into_iter()
            .map(|(k, v)| (k, v as i32))
            .collect();

        // By status
        let by_status_query = format!(
            "SELECT status, COUNT(*) FROM bounty_change_events {} GROUP BY status",
            base_where
        );
        let mut q = sqlx::query_as::<_, (String, i64)>(&by_status_query);
        if let Some(pid) = program_id {
            q = q.bind(pid);
        }
        let status_rows: Vec<(String, i64)> = q.fetch_all(self.pool()).await?;
        let by_status: std::collections::HashMap<String, i32> = status_rows
            .into_iter()
            .map(|(k, v)| (k, v as i32))
            .collect();

        // Pending review count
        let pending_query = format!(
            "SELECT COUNT(*) FROM bounty_change_events {} {} status IN ('new', 'analyzing', 'review_required')",
            base_where,
            if program_id.is_some() { "AND" } else { "WHERE" }
        );
        let mut q = sqlx::query_as::<_, (i64,)>(&pending_query);
        if let Some(pid) = program_id {
            q = q.bind(pid);
        }
        let pending: (i64,) = q.fetch_one(self.pool()).await?;

        // Average risk score
        let avg_query = format!(
            "SELECT COALESCE(AVG(risk_score), 0.0) FROM bounty_change_events {}",
            base_where
        );
        let mut q = sqlx::query_as::<_, (f64,)>(&avg_query);
        if let Some(pid) = program_id {
            q = q.bind(pid);
        }
        let avg: (f64,) = q.fetch_one(self.pool()).await?;

        Ok(BountyChangeEventStats {
            total_events: total.0 as i32,
            by_type,
            by_severity,
            by_status,
            pending_review: pending.0 as i32,
            average_risk_score: avg.0,
        })
    }

    /// Update change event status
    pub async fn update_bounty_change_event_status(
        &self,
        id: &str,
        status: &str,
        resolved_at: Option<&str>,
    ) -> Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE bounty_change_events SET status = ?, resolved_at = ?, updated_at = ? WHERE id = ?"
        )
        .bind(status)
        .bind(resolved_at)
        .bind(&now)
        .bind(id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Add triggered workflow to change event
    pub async fn add_triggered_workflow_to_change_event(
        &self,
        event_id: &str,
        workflow_id: &str,
    ) -> Result<bool> {
        let event = self.get_bounty_change_event(event_id).await?;
        let Some(mut event) = event else {
            return Ok(false);
        };

        let mut workflows: Vec<String> = event.triggered_workflows_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        if !workflows.contains(&workflow_id.to_string()) {
            workflows.push(workflow_id.to_string());
            event.triggered_workflows_json = Some(serde_json::to_string(&workflows).unwrap_or_default());
            event.status = "workflow_triggered".to_string();
            event.updated_at = chrono::Utc::now().to_rfc3339();
            return self.update_bounty_change_event(&event).await;
        }

        Ok(true)
    }

    /// Add generated finding to change event
    pub async fn add_generated_finding_to_change_event(
        &self,
        event_id: &str,
        finding_id: &str,
    ) -> Result<bool> {
        let event = self.get_bounty_change_event(event_id).await?;
        let Some(mut event) = event else {
            return Ok(false);
        };

        let mut findings: Vec<String> = event.generated_findings_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        if !findings.contains(&finding_id.to_string()) {
            findings.push(finding_id.to_string());
            event.generated_findings_json = Some(serde_json::to_string(&findings).unwrap_or_default());
            event.updated_at = chrono::Utc::now().to_rfc3339();
            return self.update_bounty_change_event(&event).await;
        }

        Ok(true)
    }
}

// ============================================================================
// Workflow Template Models
// ============================================================================

/// Bounty workflow template
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountyWorkflowTemplateRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub workflow_definition_id: Option<String>,
    pub steps_json: String,
    pub input_schema_json: Option<String>,
    pub output_schema_json: Option<String>,
    pub tags_json: Option<String>,
    pub is_built_in: bool,
    pub estimated_duration_mins: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

/// Bounty workflow binding (template → program/scope)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountyWorkflowBindingRow {
    pub id: String,
    pub program_id: String,
    pub scope_id: Option<String>,
    pub workflow_template_id: String,
    pub is_enabled: bool,
    pub auto_run_on_change: bool,
    pub trigger_conditions_json: Option<String>,
    pub schedule_cron: Option<String>,
    pub last_run_at: Option<String>,
    pub last_run_status: Option<String>,
    pub run_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl DatabaseService {
    // ------------------------------------------------------------------------
    // Workflow Template CRUD
    // ------------------------------------------------------------------------

    /// Create a workflow template
    pub async fn create_bounty_workflow_template(&self, template: &BountyWorkflowTemplateRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO bounty_workflow_templates (
                id, name, description, category, workflow_definition_id, steps_json,
                input_schema_json, output_schema_json, tags_json, is_built_in,
                estimated_duration_mins, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&template.id)
        .bind(&template.name)
        .bind(&template.description)
        .bind(&template.category)
        .bind(&template.workflow_definition_id)
        .bind(&template.steps_json)
        .bind(&template.input_schema_json)
        .bind(&template.output_schema_json)
        .bind(&template.tags_json)
        .bind(template.is_built_in)
        .bind(template.estimated_duration_mins)
        .bind(&template.created_at)
        .bind(&template.updated_at)
        .execute(self.pool())
        .await?;

        info!("Created bounty workflow template: {}", template.id);
        Ok(())
    }

    /// Get a workflow template by ID
    pub async fn get_bounty_workflow_template(&self, id: &str) -> Result<Option<BountyWorkflowTemplateRow>> {
        let row = sqlx::query_as::<_, BountyWorkflowTemplateRow>(
            "SELECT * FROM bounty_workflow_templates WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row)
    }

    /// List workflow templates
    pub async fn list_bounty_workflow_templates(
        &self,
        category: Option<&str>,
        is_built_in: Option<bool>,
    ) -> Result<Vec<BountyWorkflowTemplateRow>> {
        let mut query = String::from("SELECT * FROM bounty_workflow_templates WHERE 1=1");

        if let Some(cat) = category {
            query.push_str(&format!(" AND category = '{}'", cat));
        }
        if let Some(built_in) = is_built_in {
            query.push_str(&format!(" AND is_built_in = {}", if built_in { 1 } else { 0 }));
        }

        query.push_str(" ORDER BY name ASC");

        let rows = sqlx::query_as::<_, BountyWorkflowTemplateRow>(&query)
            .fetch_all(self.pool())
            .await?;

        Ok(rows)
    }

    /// Update a workflow template
    pub async fn update_bounty_workflow_template(&self, template: &BountyWorkflowTemplateRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_workflow_templates SET
                name = ?, description = ?, category = ?, workflow_definition_id = ?,
                steps_json = ?, input_schema_json = ?, output_schema_json = ?,
                tags_json = ?, estimated_duration_mins = ?, updated_at = ?
            WHERE id = ?"#
        )
        .bind(&template.name)
        .bind(&template.description)
        .bind(&template.category)
        .bind(&template.workflow_definition_id)
        .bind(&template.steps_json)
        .bind(&template.input_schema_json)
        .bind(&template.output_schema_json)
        .bind(&template.tags_json)
        .bind(template.estimated_duration_mins)
        .bind(&template.updated_at)
        .bind(&template.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a workflow template
    pub async fn delete_bounty_workflow_template(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_workflow_templates WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ------------------------------------------------------------------------
    // Workflow Binding CRUD
    // ------------------------------------------------------------------------

    /// Create a workflow binding
    pub async fn create_bounty_workflow_binding(&self, binding: &BountyWorkflowBindingRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO bounty_workflow_bindings (
                id, program_id, scope_id, workflow_template_id, is_enabled,
                auto_run_on_change, trigger_conditions_json, schedule_cron,
                last_run_at, last_run_status, run_count, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&binding.id)
        .bind(&binding.program_id)
        .bind(&binding.scope_id)
        .bind(&binding.workflow_template_id)
        .bind(binding.is_enabled)
        .bind(binding.auto_run_on_change)
        .bind(&binding.trigger_conditions_json)
        .bind(&binding.schedule_cron)
        .bind(&binding.last_run_at)
        .bind(&binding.last_run_status)
        .bind(binding.run_count)
        .bind(&binding.created_at)
        .bind(&binding.updated_at)
        .execute(self.pool())
        .await?;

        info!("Created bounty workflow binding: {}", binding.id);
        Ok(())
    }

    /// Get a workflow binding by ID
    pub async fn get_bounty_workflow_binding(&self, id: &str) -> Result<Option<BountyWorkflowBindingRow>> {
        let row = sqlx::query_as::<_, BountyWorkflowBindingRow>(
            "SELECT * FROM bounty_workflow_bindings WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row)
    }

    /// List workflow bindings for a program
    pub async fn list_bounty_workflow_bindings(
        &self,
        program_id: Option<&str>,
        scope_id: Option<&str>,
        is_enabled: Option<bool>,
    ) -> Result<Vec<BountyWorkflowBindingRow>> {
        let mut query = String::from("SELECT * FROM bounty_workflow_bindings WHERE 1=1");

        if let Some(pid) = program_id {
            query.push_str(&format!(" AND program_id = '{}'", pid));
        }
        if let Some(sid) = scope_id {
            query.push_str(&format!(" AND scope_id = '{}'", sid));
        }
        if let Some(enabled) = is_enabled {
            query.push_str(&format!(" AND is_enabled = {}", if enabled { 1 } else { 0 }));
        }

        query.push_str(" ORDER BY created_at DESC");

        let rows = sqlx::query_as::<_, BountyWorkflowBindingRow>(&query)
            .fetch_all(self.pool())
            .await?;

        Ok(rows)
    }

    /// Update a workflow binding
    pub async fn update_bounty_workflow_binding(&self, binding: &BountyWorkflowBindingRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_workflow_bindings SET
                scope_id = ?, is_enabled = ?, auto_run_on_change = ?,
                trigger_conditions_json = ?, schedule_cron = ?,
                last_run_at = ?, last_run_status = ?, run_count = ?, updated_at = ?
            WHERE id = ?"#
        )
        .bind(&binding.scope_id)
        .bind(binding.is_enabled)
        .bind(binding.auto_run_on_change)
        .bind(&binding.trigger_conditions_json)
        .bind(&binding.schedule_cron)
        .bind(&binding.last_run_at)
        .bind(&binding.last_run_status)
        .bind(binding.run_count)
        .bind(&binding.updated_at)
        .bind(&binding.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a workflow binding
    pub async fn delete_bounty_workflow_binding(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_workflow_bindings WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update binding run status
    pub async fn update_bounty_workflow_binding_run_status(
        &self,
        id: &str,
        status: &str,
    ) -> Result<bool> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            r#"UPDATE bounty_workflow_bindings SET
                last_run_at = ?, last_run_status = ?, run_count = run_count + 1, updated_at = ?
            WHERE id = ?"#
        )
        .bind(&now)
        .bind(status)
        .bind(&now)
        .bind(id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get bindings that should auto-run on change events
    pub async fn get_auto_trigger_workflow_bindings(
        &self,
        program_id: &str,
    ) -> Result<Vec<BountyWorkflowBindingRow>> {
        let rows = sqlx::query_as::<_, BountyWorkflowBindingRow>(
            r#"SELECT * FROM bounty_workflow_bindings
               WHERE program_id = ? AND is_enabled = 1 AND auto_run_on_change = 1"#
        )
        .bind(program_id)
        .fetch_all(self.pool())
        .await?;

        Ok(rows)
    }
}

// ============================================================================
// Bounty Asset Models (P1-B3: Asset Consolidation)
// ============================================================================

/// Bounty asset model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BountyAssetRow {
    pub id: String,
    pub program_id: String,
    pub scope_id: Option<String>,
    pub asset_type: String,
    pub canonical_url: String,
    pub original_urls_json: Option<String>,
    pub hostname: Option<String>,
    pub port: Option<i32>,
    pub path: Option<String>,
    pub protocol: Option<String>,
    pub ip_addresses_json: Option<String>,
    pub dns_records_json: Option<String>,
    pub tech_stack_json: Option<String>,
    pub fingerprint: Option<String>,
    pub tags_json: Option<String>,
    pub labels_json: Option<String>,
    pub priority_score: Option<f64>,
    pub risk_score: Option<f64>,
    pub is_alive: bool,
    pub last_checked_at: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub findings_count: i32,
    pub change_events_count: i32,
    pub metadata_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Asset statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BountyAssetStats {
    pub total_assets: i32,
    pub alive_assets: i32,
    pub by_type: std::collections::HashMap<String, i32>,
    pub with_findings: i32,
    pub high_priority: i32,
}

impl DatabaseService {
    // ------------------------------------------------------------------------
    // Bounty Asset CRUD
    // ------------------------------------------------------------------------

    /// Create a bounty asset
    pub async fn create_bounty_asset(&self, asset: &BountyAssetRow) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO bounty_assets (
                id, program_id, scope_id, asset_type, canonical_url, original_urls_json,
                hostname, port, path, protocol, ip_addresses_json, dns_records_json,
                tech_stack_json, fingerprint, tags_json, labels_json, priority_score,
                risk_score, is_alive, last_checked_at, first_seen_at, last_seen_at,
                findings_count, change_events_count, metadata_json, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&asset.id)
        .bind(&asset.program_id)
        .bind(&asset.scope_id)
        .bind(&asset.asset_type)
        .bind(&asset.canonical_url)
        .bind(&asset.original_urls_json)
        .bind(&asset.hostname)
        .bind(asset.port)
        .bind(&asset.path)
        .bind(&asset.protocol)
        .bind(&asset.ip_addresses_json)
        .bind(&asset.dns_records_json)
        .bind(&asset.tech_stack_json)
        .bind(&asset.fingerprint)
        .bind(&asset.tags_json)
        .bind(&asset.labels_json)
        .bind(asset.priority_score)
        .bind(asset.risk_score)
        .bind(asset.is_alive)
        .bind(&asset.last_checked_at)
        .bind(&asset.first_seen_at)
        .bind(&asset.last_seen_at)
        .bind(asset.findings_count)
        .bind(asset.change_events_count)
        .bind(&asset.metadata_json)
        .bind(&asset.created_at)
        .bind(&asset.updated_at)
        .execute(self.pool())
        .await?;

        info!("Created bounty asset: {}", asset.id);
        Ok(())
    }

    /// Get a bounty asset by ID
    pub async fn get_bounty_asset(&self, id: &str) -> Result<Option<BountyAssetRow>> {
        let row = sqlx::query_as::<_, BountyAssetRow>(
            "SELECT * FROM bounty_assets WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row)
    }

    /// Get a bounty asset by canonical URL
    pub async fn get_bounty_asset_by_canonical_url(
        &self,
        program_id: &str,
        canonical_url: &str,
    ) -> Result<Option<BountyAssetRow>> {
        let row = sqlx::query_as::<_, BountyAssetRow>(
            "SELECT * FROM bounty_assets WHERE program_id = ? AND canonical_url = ?"
        )
        .bind(program_id)
        .bind(canonical_url)
        .fetch_optional(self.pool())
        .await?;

        Ok(row)
    }

    /// Get a bounty asset by fingerprint
    pub async fn get_bounty_asset_by_fingerprint(
        &self,
        program_id: &str,
        fingerprint: &str,
    ) -> Result<Option<BountyAssetRow>> {
        let row = sqlx::query_as::<_, BountyAssetRow>(
            "SELECT * FROM bounty_assets WHERE program_id = ? AND fingerprint = ?"
        )
        .bind(program_id)
        .bind(fingerprint)
        .fetch_optional(self.pool())
        .await?;

        Ok(row)
    }

    /// List bounty assets
    pub async fn list_bounty_assets(
        &self,
        program_id: Option<&str>,
        scope_id: Option<&str>,
        asset_type: Option<&str>,
        is_alive: Option<bool>,
        has_findings: Option<bool>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<BountyAssetRow>> {
        let mut query = String::from("SELECT * FROM bounty_assets WHERE 1=1");

        if let Some(pid) = program_id {
            query.push_str(&format!(" AND program_id = '{}'", pid));
        }
        if let Some(sid) = scope_id {
            query.push_str(&format!(" AND scope_id = '{}'", sid));
        }
        if let Some(at) = asset_type {
            query.push_str(&format!(" AND asset_type = '{}'", at));
        }
        if let Some(alive) = is_alive {
            query.push_str(&format!(" AND is_alive = {}", if alive { 1 } else { 0 }));
        }
        if let Some(findings) = has_findings {
            if findings {
                query.push_str(" AND findings_count > 0");
            } else {
                query.push_str(" AND findings_count = 0");
            }
        }

        query.push_str(" ORDER BY priority_score DESC, last_seen_at DESC");

        if let Some(lim) = limit {
            query.push_str(&format!(" LIMIT {}", lim));
        }
        if let Some(off) = offset {
            query.push_str(&format!(" OFFSET {}", off));
        }

        let rows = sqlx::query_as::<_, BountyAssetRow>(&query)
            .fetch_all(self.pool())
            .await?;

        Ok(rows)
    }

    /// Update a bounty asset
    pub async fn update_bounty_asset(&self, asset: &BountyAssetRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_assets SET
                scope_id = ?, asset_type = ?, canonical_url = ?, original_urls_json = ?,
                hostname = ?, port = ?, path = ?, protocol = ?, ip_addresses_json = ?,
                dns_records_json = ?, tech_stack_json = ?, fingerprint = ?, tags_json = ?,
                labels_json = ?, priority_score = ?, risk_score = ?, is_alive = ?,
                last_checked_at = ?, last_seen_at = ?, findings_count = ?,
                change_events_count = ?, metadata_json = ?, updated_at = ?
            WHERE id = ?"#
        )
        .bind(&asset.scope_id)
        .bind(&asset.asset_type)
        .bind(&asset.canonical_url)
        .bind(&asset.original_urls_json)
        .bind(&asset.hostname)
        .bind(asset.port)
        .bind(&asset.path)
        .bind(&asset.protocol)
        .bind(&asset.ip_addresses_json)
        .bind(&asset.dns_records_json)
        .bind(&asset.tech_stack_json)
        .bind(&asset.fingerprint)
        .bind(&asset.tags_json)
        .bind(&asset.labels_json)
        .bind(asset.priority_score)
        .bind(asset.risk_score)
        .bind(asset.is_alive)
        .bind(&asset.last_checked_at)
        .bind(&asset.last_seen_at)
        .bind(asset.findings_count)
        .bind(asset.change_events_count)
        .bind(&asset.metadata_json)
        .bind(&asset.updated_at)
        .bind(&asset.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty asset
    pub async fn delete_bounty_asset(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_assets WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get bounty asset statistics
    pub async fn get_bounty_asset_stats(&self, program_id: Option<&str>) -> Result<BountyAssetStats> {
        let pool = self.pool();
        let filter = program_id.map(|p| format!(" WHERE program_id = '{}'", p)).unwrap_or_default();

        let total: (i32,) = sqlx::query_as(&format!(
            "SELECT COUNT(*) FROM bounty_assets{}", filter
        )).fetch_one(pool).await?;

        let alive: (i32,) = sqlx::query_as(&format!(
            "SELECT COUNT(*) FROM bounty_assets{} AND is_alive = 1",
            if filter.is_empty() { " WHERE 1=1" } else { &filter }
        )).fetch_one(pool).await?;

        let with_findings: (i32,) = sqlx::query_as(&format!(
            "SELECT COUNT(*) FROM bounty_assets{} AND findings_count > 0",
            if filter.is_empty() { " WHERE 1=1" } else { &filter }
        )).fetch_one(pool).await?;

        let high_priority: (i32,) = sqlx::query_as(&format!(
            "SELECT COUNT(*) FROM bounty_assets{} AND priority_score >= 7.0",
            if filter.is_empty() { " WHERE 1=1" } else { &filter }
        )).fetch_one(pool).await?;

        // Get by type
        let type_rows: Vec<(String, i32)> = sqlx::query_as(&format!(
            "SELECT asset_type, COUNT(*) FROM bounty_assets{} GROUP BY asset_type",
            filter
        )).fetch_all(pool).await?;

        let mut by_type = std::collections::HashMap::new();
        for (t, c) in type_rows {
            by_type.insert(t, c);
        }

        Ok(BountyAssetStats {
            total_assets: total.0,
            alive_assets: alive.0,
            by_type,
            with_findings: with_findings.0,
            high_priority: high_priority.0,
        })
    }

    /// Merge asset URLs (add original URL to existing asset)
    pub async fn merge_bounty_asset_url(
        &self,
        asset_id: &str,
        original_url: &str,
    ) -> Result<bool> {
        let asset = self.get_bounty_asset(asset_id).await?;
        let Some(mut asset) = asset else {
            return Ok(false);
        };

        let mut urls: Vec<String> = asset.original_urls_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        if !urls.contains(&original_url.to_string()) {
            urls.push(original_url.to_string());
            asset.original_urls_json = Some(serde_json::to_string(&urls).unwrap_or_default());
            asset.updated_at = chrono::Utc::now().to_rfc3339();
            return self.update_bounty_asset(&asset).await;
        }

        Ok(true)
    }

    /// Get top priority assets
    pub async fn get_top_priority_assets(
        &self,
        program_id: &str,
        limit: i64,
    ) -> Result<Vec<BountyAssetRow>> {
        let rows = sqlx::query_as::<_, BountyAssetRow>(
            r#"SELECT * FROM bounty_assets
               WHERE program_id = ? AND is_alive = 1
               ORDER BY priority_score DESC
               LIMIT ?"#
        )
        .bind(program_id)
        .bind(limit)
        .fetch_all(self.pool())
        .await?;

        Ok(rows)
    }
}

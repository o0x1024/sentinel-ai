//! Bug Bounty database operations

use anyhow::Result;
use chrono;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::info;

use super::service::DatabaseService;

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert string timestamp to DateTime<Utc> for database binding
fn timestamp_string_to_datetime(s: &str) -> chrono::DateTime<chrono::Utc> {
    s.parse::<chrono::DateTime<chrono::Utc>>()
        .unwrap_or_else(|_| chrono::Utc::now())
}

/// Convert optional string timestamp to Option<DateTime<Utc>> for database binding
fn optional_timestamp_string_to_datetime(s: &Option<String>) -> Option<chrono::DateTime<chrono::Utc>> {
    s.as_ref().and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok())
}

/// Convert TIMESTAMP WITH TIME ZONE to String for struct fields
fn timestamp_to_string(row: &sqlx::postgres::PgRow, column: &str) -> String {
    row.try_get::<chrono::DateTime<chrono::Utc>, _>(column)
        .map(|dt| dt.to_rfc3339())
        .or_else(|_| row.try_get::<String, _>(column))
        .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339())
}

/// Convert optional TIMESTAMP WITH TIME ZONE to Option<String> for struct fields
fn optional_timestamp_to_string(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<chrono::DateTime<chrono::Utc>, _>(column)
        .map(|dt| Some(dt.to_rfc3339()))
        .or_else(|_| row.try_get::<Option<String>, _>(column))
        .unwrap_or(None)
}

/// Map PgRow to BountyProgramRow
fn row_to_bounty_program(row: sqlx::postgres::PgRow) -> BountyProgramRow {
    BountyProgramRow {
        id: row.get("id"),
        name: row.get("name"),
        organization: row.get("organization"),
        platform: row.get("platform"),
        platform_handle: row.get("platform_handle"),
        url: row.get("url"),
        program_type: row.get("program_type"),
        status: row.get("status"),
        description: row.get("description"),
        rewards_json: row.get("rewards_json"),
        response_sla_days: row.get("response_sla_days"),
        resolution_sla_days: row.get("resolution_sla_days"),
        rules_json: row.get("rules_json"),
        tags_json: row.get("tags_json"),
        metadata_json: row.get("metadata_json"),
        priority_score: row.get("priority_score"),
        total_submissions: row.get("total_submissions"),
        accepted_submissions: row.get("accepted_submissions"),
        total_earnings: row.get("total_earnings"),
        created_at: timestamp_to_string(&row, "created_at"),
        updated_at: timestamp_to_string(&row, "updated_at"),
        last_activity_at: optional_timestamp_to_string(&row, "last_activity_at"),
    }
}

/// Map PgRow to ProgramScopeRow
fn row_to_program_scope(row: sqlx::postgres::PgRow) -> ProgramScopeRow {
    ProgramScopeRow {
        id: row.get("id"),
        program_id: row.get("program_id"),
        scope_type: row.get("scope_type"),
        target_type: row.get("target_type"),
        target: row.get("target"),
        description: row.get("description"),
        allowed_tests_json: row.get("allowed_tests_json"),
        instructions_json: row.get("instructions_json"),
        requires_auth: row.get("requires_auth"),
        test_accounts_json: row.get("test_accounts_json"),
        asset_count: row.get("asset_count"),
        finding_count: row.get("finding_count"),
        priority: row.get("priority"),
        metadata_json: row.get("metadata_json"),
        created_at: timestamp_to_string(&row, "created_at"),
        updated_at: timestamp_to_string(&row, "updated_at"),
    }
}

/// Map PgRow to BountyFindingRow
fn row_to_bounty_finding(row: sqlx::postgres::PgRow) -> BountyFindingRow {
    BountyFindingRow {
        id: row.get("id"),
        program_id: row.get("program_id"),
        scope_id: row.get("scope_id"),
        asset_id: row.get("asset_id"),
        title: row.get("title"),
        description: row.get("description"),
        finding_type: row.get("finding_type"),
        severity: row.get("severity"),
        status: row.get("status"),
        confidence: row.get("confidence"),
        cvss_score: row.get("cvss_score"),
        cwe_id: row.get("cwe_id"),
        affected_url: row.get("affected_url"),
        affected_parameter: row.get("affected_parameter"),
        reproduction_steps_json: row.get("reproduction_steps_json"),
        impact: row.get("impact"),
        remediation: row.get("remediation"),
        evidence_ids_json: row.get("evidence_ids_json"),
        tags_json: row.get("tags_json"),
        metadata_json: row.get("metadata_json"),
        fingerprint: row.get("fingerprint"),
        duplicate_of: row.get("duplicate_of"),
        first_seen_at: timestamp_to_string(&row, "first_seen_at"),
        last_seen_at: timestamp_to_string(&row, "last_seen_at"),
        verified_at: optional_timestamp_to_string(&row, "verified_at"),
        created_at: timestamp_to_string(&row, "created_at"),
        updated_at: timestamp_to_string(&row, "updated_at"),
        created_by: row.get("created_by"),
    }
}

/// Map PgRow to BountySubmissionRow
fn row_to_bounty_submission(row: sqlx::postgres::PgRow) -> BountySubmissionRow {
    BountySubmissionRow {
        id: row.get("id"),
        program_id: row.get("program_id"),
        finding_id: row.get("finding_id"),
        platform_submission_id: row.get("platform_submission_id"),
        title: row.get("title"),
        status: row.get("status"),
        priority: row.get("priority"),
        vulnerability_type: row.get("vulnerability_type"),
        severity: row.get("severity"),
        cvss_score: row.get("cvss_score"),
        cwe_id: row.get("cwe_id"),
        description: row.get("description"),
        reproduction_steps_json: row.get("reproduction_steps_json"),
        impact: row.get("impact"),
        remediation: row.get("remediation"),
        evidence_ids_json: row.get("evidence_ids_json"),
        platform_url: row.get("platform_url"),
        reward_amount: row.get("reward_amount"),
        reward_currency: row.get("reward_currency"),
        bonus_amount: row.get("bonus_amount"),
        response_time_hours: row.get("response_time_hours"),
        resolution_time_hours: row.get("resolution_time_hours"),
        requires_retest: row.get("requires_retest"),
        retest_at: optional_timestamp_to_string(&row, "retest_at"),
        last_retest_at: optional_timestamp_to_string(&row, "last_retest_at"),
        communications_json: row.get("communications_json"),
        timeline_json: row.get("timeline_json"),
        tags_json: row.get("tags_json"),
        metadata_json: row.get("metadata_json"),
        created_at: timestamp_to_string(&row, "created_at"),
        submitted_at: optional_timestamp_to_string(&row, "submitted_at"),
        updated_at: timestamp_to_string(&row, "updated_at"),
        closed_at: optional_timestamp_to_string(&row, "closed_at"),
        created_by: row.get("created_by"),
    }
}

/// Map PgRow to BountyEvidenceRow
fn row_to_bounty_evidence(row: sqlx::postgres::PgRow) -> BountyEvidenceRow {
    BountyEvidenceRow {
        id: row.get("id"),
        finding_id: row.get("finding_id"),
        evidence_type: row.get("evidence_type"),
        title: row.get("title"),
        description: row.get("description"),
        file_path: row.get("file_path"),
        file_url: row.get("file_url"),
        content: row.get("content"),
        mime_type: row.get("mime_type"),
        file_size: row.get("file_size"),
        http_request_json: row.get("http_request_json"),
        http_response_json: row.get("http_response_json"),
        diff: row.get("diff"),
        tags_json: row.get("tags_json"),
        metadata_json: row.get("metadata_json"),
        display_order: row.get("display_order"),
        created_at: timestamp_to_string(&row, "created_at"),
        updated_at: timestamp_to_string(&row, "updated_at"),
    }
}

/// Map PgRow to BountyChangeEventRow
fn row_to_bounty_change_event(row: sqlx::postgres::PgRow) -> BountyChangeEventRow {
    BountyChangeEventRow {
        id: row.get("id"),
        program_id: row.get("program_id"),
        asset_id: row.get("asset_id"),
        event_type: row.get("event_type"),
        severity: row.get("severity"),
        status: row.get("status"),
        title: row.get("title"),
        description: row.get("description"),
        old_value: row.get("old_value"),
        new_value: row.get("new_value"),
        diff: row.get("diff"),
        affected_scope: row.get("affected_scope"),
        detection_method: row.get("detection_method"),
        triggered_workflows_json: row.get("triggered_workflows_json"),
        generated_findings_json: row.get("generated_findings_json"),
        tags_json: row.get("tags_json"),
        metadata_json: row.get("metadata_json"),
        risk_score: row.get("risk_score"),
        auto_trigger_enabled: row.get("auto_trigger_enabled"),
        created_at: timestamp_to_string(&row, "created_at"),
        updated_at: timestamp_to_string(&row, "updated_at"),
        resolved_at: optional_timestamp_to_string(&row, "resolved_at"),
    }
}

/// Map PgRow to BountyWorkflowTemplateRow
fn row_to_bounty_workflow_template(row: sqlx::postgres::PgRow) -> BountyWorkflowTemplateRow {
    BountyWorkflowTemplateRow {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        category: row.get("category"),
        workflow_definition_id: row.get("workflow_definition_id"),
        steps_json: row.get("steps_json"),
        input_schema_json: row.get("input_schema_json"),
        output_schema_json: row.get("output_schema_json"),
        tags_json: row.get("tags_json"),
        is_built_in: row.get("is_built_in"),
        estimated_duration_mins: row.get("estimated_duration_mins"),
        created_at: timestamp_to_string(&row, "created_at"),
        updated_at: timestamp_to_string(&row, "updated_at"),
    }
}

/// Map PgRow to BountyWorkflowBindingRow
fn row_to_bounty_workflow_binding(row: sqlx::postgres::PgRow) -> BountyWorkflowBindingRow {
    BountyWorkflowBindingRow {
        id: row.get("id"),
        program_id: row.get("program_id"),
        scope_id: row.get("scope_id"),
        workflow_template_id: row.get("workflow_template_id"),
        is_enabled: row.get("is_enabled"),
        auto_run_on_change: row.get("auto_run_on_change"),
        trigger_conditions_json: row.get("trigger_conditions_json"),
        schedule_cron: row.get("schedule_cron"),
        last_run_at: optional_timestamp_to_string(&row, "last_run_at"),
        last_run_status: row.get("last_run_status"),
        run_count: row.get("run_count"),
        created_at: timestamp_to_string(&row, "created_at"),
        updated_at: timestamp_to_string(&row, "updated_at"),
    }
}

/// Map PgRow to BountyAssetRow
fn row_to_bounty_asset(row: sqlx::postgres::PgRow) -> BountyAssetRow {
    BountyAssetRow {
        id: row.get("id"),
        program_id: row.get("program_id"),
        scope_id: row.get("scope_id"),
        asset_type: row.get("asset_type"),
        canonical_url: row.get("canonical_url"),
        original_urls_json: row.get("original_urls_json"),
        hostname: row.get("hostname"),
        port: row.get("port"),
        path: row.get("path"),
        protocol: row.get("protocol"),
        ip_addresses_json: row.get("ip_addresses_json"),
        dns_records_json: row.get("dns_records_json"),
        tech_stack_json: row.get("tech_stack_json"),
        fingerprint: row.get("fingerprint"),
        tags_json: row.get("tags_json"),
        labels_json: row.get("labels_json"),
        priority_score: row.get("priority_score"),
        risk_score: row.get("risk_score"),
        is_alive: row.get("is_alive"),
        last_checked_at: optional_timestamp_to_string(&row, "last_checked_at"),
        first_seen_at: timestamp_to_string(&row, "first_seen_at"),
        last_seen_at: timestamp_to_string(&row, "last_seen_at"),
        findings_count: row.get("findings_count"),
        change_events_count: row.get("change_events_count"),
        metadata_json: row.get("metadata_json"),
        created_at: timestamp_to_string(&row, "created_at"),
        updated_at: timestamp_to_string(&row, "updated_at"),
        // ASM fields
        ip_version: row.get("ip_version"),
        asn: row.get("asn"),
        asn_org: row.get("asn_org"),
        isp: row.get("isp"),
        country: row.get("country"),
        city: row.get("city"),
        latitude: row.get("latitude"),
        longitude: row.get("longitude"),
        is_cloud: row.get("is_cloud"),
        cloud_provider: row.get("cloud_provider"),
        service_name: row.get("service_name"),
        service_version: row.get("service_version"),
        service_product: row.get("service_product"),
        banner: row.get("banner"),
        transport_protocol: row.get("transport_protocol"),
        cpe: row.get("cpe"),
        domain_registrar: row.get("domain_registrar"),
        registration_date: row.get("registration_date"),
        expiration_date: row.get("expiration_date"),
        nameservers_json: row.get("nameservers_json"),
        mx_records_json: row.get("mx_records_json"),
        txt_records_json: row.get("txt_records_json"),
        whois_data_json: row.get("whois_data_json"),
        is_wildcard: row.get("is_wildcard"),
        parent_domain: row.get("parent_domain"),
        http_status: row.get("http_status"),
        response_time_ms: row.get("response_time_ms"),
        content_length: row.get("content_length"),
        content_type: row.get("content_type"),
        title: row.get("title"),
        favicon_hash: row.get("favicon_hash"),
        headers_json: row.get("headers_json"),
        waf_detected: row.get("waf_detected"),
        cdn_detected: row.get("cdn_detected"),
        screenshot_path: row.get("screenshot_path"),
        body_hash: row.get("body_hash"),
        certificate_id: row.get("certificate_id"),
        ssl_enabled: row.get("ssl_enabled"),
        certificate_subject: row.get("certificate_subject"),
        certificate_issuer: row.get("certificate_issuer"),
        certificate_valid_from: row.get("certificate_valid_from"),
        certificate_valid_to: row.get("certificate_valid_to"),
        certificate_san_json: row.get("certificate_san_json"),
        exposure_level: row.get("exposure_level"),
        attack_surface_score: row.get("attack_surface_score"),
        vulnerability_count: row.get("vulnerability_count"),
        cvss_max_score: row.get("cvss_max_score"),
        exploit_available: row.get("exploit_available"),
        asset_category: row.get("asset_category"),
        asset_owner: row.get("asset_owner"),
        business_unit: row.get("business_unit"),
        criticality: row.get("criticality"),
        discovery_method: row.get("discovery_method"),
        data_sources_json: row.get("data_sources_json"),
        confidence_score: row.get("confidence_score"),
        monitoring_enabled: row.get("monitoring_enabled"),
        scan_frequency: row.get("scan_frequency"),
        last_scan_type: row.get("last_scan_type"),
        parent_asset_id: row.get("parent_asset_id"),
        related_assets_json: row.get("related_assets_json"),
    }
}

// ============================================================================
// Database Models
// ============================================================================

/// Bug Bounty Program database model
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)"#
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
        .bind(timestamp_string_to_datetime(&program.created_at))
        .bind(timestamp_string_to_datetime(&program.updated_at))
        .bind(optional_timestamp_string_to_datetime(&program.last_activity_at))
        .execute(self.pool())
        .await?;

        info!("Created bounty program: {}", program.id);
        Ok(())
    }

    /// Get a bounty program by ID
    pub async fn get_bounty_program(&self, id: &str) -> Result<Option<BountyProgramRow>> {
        let row = sqlx::query("SELECT * FROM bounty_programs WHERE id = $1")
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(row_to_bounty_program))
    }

    /// Update a bounty program
    pub async fn update_bounty_program(&self, program: &BountyProgramRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_programs SET
                name = $1, organization = $2, platform = $3, platform_handle = $4,
                url = $5, program_type = $6, status = $7, description = $8,
                rewards_json = $9, response_sla_days = $10, resolution_sla_days = $11,
                rules_json = $12, tags_json = $13, metadata_json = $14, priority_score = $15,
                total_submissions = $16, accepted_submissions = $17, total_earnings = $18,
                updated_at = $19, last_activity_at = $20
            WHERE id = $21"#
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
        .bind(timestamp_string_to_datetime(&program.updated_at))
        .bind(optional_timestamp_string_to_datetime(&program.last_activity_at))
        .bind(&program.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty program
    pub async fn delete_bounty_program(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_programs WHERE id = $1")
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
                let mut placeholders = Vec::new();
                for _ in platforms {
                    placeholders.push(format!("${}", params.len() + 1 + placeholders.len()));
                }
                query.push_str(&format!(" AND platform IN ({})", placeholders.join(",")));
                params.extend(platforms.iter().cloned());
            }
        }

        if let Some(statuses) = statuses {
            if !statuses.is_empty() {
                let mut placeholders = Vec::new();
                for _ in statuses {
                    placeholders.push(format!("${}", params.len() + 1 + placeholders.len()));
                }
                query.push_str(&format!(" AND status IN ({})", placeholders.join(",")));
                params.extend(statuses.iter().cloned());
            }
        }

        if let Some(search) = search {
            if !search.is_empty() {
                let p1 = params.len() + 1;
                let p2 = params.len() + 2;
                query.push_str(&format!(" AND (name LIKE ${} OR organization LIKE ${})", p1, p2));
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
        let mut sqlx_query = sqlx::query(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param);
        }

        let rows = sqlx_query.fetch_all(self.pool()).await?;
        Ok(rows.into_iter().map(row_to_bounty_program).collect())
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
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)"#
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
        .bind(timestamp_string_to_datetime(&scope.created_at))
        .bind(timestamp_string_to_datetime(&scope.updated_at))
        .execute(self.pool())
        .await?;

        info!("Created program scope: {}", scope.id);
        Ok(())
    }

    /// Get a program scope by ID
    pub async fn get_program_scope(&self, id: &str) -> Result<Option<ProgramScopeRow>> {
        let row = sqlx::query("SELECT * FROM bounty_scopes WHERE id = $1")
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(row_to_program_scope))
    }

    /// Update a program scope
    pub async fn update_program_scope(&self, scope: &ProgramScopeRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_scopes SET
                scope_type = $1, target_type = $2, target = $3, description = $4,
                allowed_tests_json = $5, instructions_json = $6, requires_auth = $7,
                test_accounts_json = $8, asset_count = $9, finding_count = $10,
                priority = $11, metadata_json = $12, updated_at = $13
            WHERE id = $14"#
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
        .bind(timestamp_string_to_datetime(&scope.updated_at))
        .bind(&scope.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a program scope
    pub async fn delete_program_scope(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_scopes WHERE id = $1")
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
        let mut params: Vec<String> = Vec::new();

        if let Some(pid) = program_id {
            params.push(pid.to_string());
            query.push_str(&format!(" AND program_id = ${}", params.len()));
        }

        if let Some(st) = scope_type {
            params.push(st.to_string());
            query.push_str(&format!(" AND scope_type = ${}", params.len()));
        }

        query.push_str(" ORDER BY priority DESC, created_at DESC");

        let mut sqlx_query = sqlx::query(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param);
        }
            
        let rows = sqlx_query.fetch_all(self.pool()).await?;

        Ok(rows.into_iter().map(row_to_program_scope).collect())
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28)"#
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
        .bind(timestamp_string_to_datetime(&finding.first_seen_at))
        .bind(timestamp_string_to_datetime(&finding.last_seen_at))
        .bind(optional_timestamp_string_to_datetime(&finding.verified_at))
        .bind(timestamp_string_to_datetime(&finding.created_at))
        .bind(timestamp_string_to_datetime(&finding.updated_at))
        .bind(&finding.created_by)
        .execute(self.pool())
        .await?;

        info!("Created bounty finding: {}", finding.id);
        Ok(())
    }

    /// Get a bounty finding by ID
    pub async fn get_bounty_finding(&self, id: &str) -> Result<Option<BountyFindingRow>> {
        let row = sqlx::query("SELECT * FROM bounty_findings WHERE id = $1")
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(row_to_bounty_finding))
    }

    /// Get a bounty finding by fingerprint (for deduplication)
    pub async fn get_bounty_finding_by_fingerprint(&self, fingerprint: &str) -> Result<Option<BountyFindingRow>> {
        let row = sqlx::query("SELECT * FROM bounty_findings WHERE fingerprint = $1")
        .bind(fingerprint)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(row_to_bounty_finding))
    }

    /// Update a bounty finding
    pub async fn update_bounty_finding(&self, finding: &BountyFindingRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_findings SET
                scope_id = $1, asset_id = $2, title = $3, description = $4, finding_type = $5,
                severity = $6, status = $7, confidence = $8, cvss_score = $9, cwe_id = $10,
                affected_url = $11, affected_parameter = $12, reproduction_steps_json = $13,
                impact = $14, remediation = $15, evidence_ids_json = $16, tags_json = $17,
                metadata_json = $18, duplicate_of = $19, last_seen_at = $20, verified_at = $21,
                updated_at = $22
            WHERE id = $23"#
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
        .bind(timestamp_string_to_datetime(&finding.last_seen_at))
        .bind(optional_timestamp_string_to_datetime(&finding.verified_at))
        .bind(timestamp_string_to_datetime(&finding.updated_at))
        .bind(&finding.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty finding
    pub async fn delete_bounty_finding(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_findings WHERE id = $1")
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
            params.push(pid.to_string());
            query.push_str(&format!(" AND program_id = ${}", params.len()));
        }

        if let Some(sid) = scope_id {
            params.push(sid.to_string());
            query.push_str(&format!(" AND scope_id = ${}", params.len()));
        }

        if let Some(severities) = severities {
            if !severities.is_empty() {
                let mut placeholders = Vec::new();
                for _ in severities {
                    placeholders.push(format!("${}", params.len() + 1 + placeholders.len()));
                }
                query.push_str(&format!(" AND severity IN ({})", placeholders.join(",")));
                params.extend(severities.iter().cloned());
            }
        }

        if let Some(statuses) = statuses {
            if !statuses.is_empty() {
                let mut placeholders = Vec::new();
                for _ in statuses {
                    placeholders.push(format!("${}", params.len() + 1 + placeholders.len()));
                }
                query.push_str(&format!(" AND status IN ({})", placeholders.join(",")));
                params.extend(statuses.iter().cloned());
            }
        }

        if let Some(search) = search {
            if !search.is_empty() {
                let p1 = params.len() + 1;
                let p2 = params.len() + 2;
                query.push_str(&format!(" AND (title LIKE ${} OR description LIKE ${})", p1, p2));
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

        let mut sqlx_query = sqlx::query(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param);
        }

        let rows = sqlx_query.fetch_all(self.pool()).await?;
        Ok(rows.into_iter().map(row_to_bounty_finding).collect())
    }

    /// Get bounty finding statistics
    pub async fn get_bounty_finding_stats(&self, program_id: Option<&str>) -> Result<BountyFindingStats> {
        let base_where = program_id.map(|_| "WHERE program_id = $1").unwrap_or("");
        
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
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)"#
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
        .bind(timestamp_string_to_datetime(&evidence.created_at))
        .bind(timestamp_string_to_datetime(&evidence.updated_at))
        .execute(self.pool())
        .await?;

        info!("Created bounty evidence: {}", evidence.id);
        Ok(())
    }

    /// Get a bounty evidence by ID
    pub async fn get_bounty_evidence(&self, id: &str) -> Result<Option<BountyEvidenceRow>> {
        let row = sqlx::query("SELECT * FROM bounty_evidence WHERE id = $1")
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(row_to_bounty_evidence))
    }

    /// Update a bounty evidence
    pub async fn update_bounty_evidence(&self, evidence: &BountyEvidenceRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_evidence SET
                evidence_type = $1, title = $2, description = $3, file_path = $4,
                file_url = $5, content = $6, mime_type = $7, file_size = $8,
                http_request_json = $9, http_response_json = $10, diff = $11,
                tags_json = $12, metadata_json = $13, display_order = $14, updated_at = $15
            WHERE id = $16"#
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
        .bind(timestamp_string_to_datetime(&evidence.updated_at))
        .bind(&evidence.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty evidence
    pub async fn delete_bounty_evidence(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_evidence WHERE id = $1")
            .bind(id)
            .execute(self.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List evidence for a finding
    pub async fn list_bounty_evidence(&self, finding_id: &str) -> Result<Vec<BountyEvidenceRow>> {
        let rows = sqlx::query("SELECT * FROM bounty_evidence WHERE finding_id = $1 ORDER BY display_order, created_at")
        .bind(finding_id)
        .fetch_all(self.pool())
        .await?;

        Ok(rows.into_iter().map(row_to_bounty_evidence).collect())
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
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34)"#
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
        .bind(timestamp_string_to_datetime(&submission.created_at))
        .bind(optional_timestamp_string_to_datetime(&submission.submitted_at))
        .bind(timestamp_string_to_datetime(&submission.updated_at))
        .bind(optional_timestamp_string_to_datetime(&submission.closed_at))
        .bind(&submission.created_by)
        .execute(self.pool())
        .await?;

        info!("Created bounty submission: {}", submission.id);
        Ok(())
    }

    /// Get a bounty submission by ID
    pub async fn get_bounty_submission(&self, id: &str) -> Result<Option<BountySubmissionRow>> {
        let row = sqlx::query("SELECT * FROM bounty_submissions WHERE id = $1")
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(row_to_bounty_submission))
    }

    /// Update a bounty submission
    pub async fn update_bounty_submission(&self, submission: &BountySubmissionRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_submissions SET
                platform_submission_id = $1, title = $2, status = $3, priority = $4,
                vulnerability_type = $5, severity = $6, cvss_score = $7, cwe_id = $8,
                description = $9, reproduction_steps_json = $10, impact = $11, remediation = $12,
                evidence_ids_json = $13, platform_url = $14, reward_amount = $15, reward_currency = $16,
                bonus_amount = $17, response_time_hours = $18, resolution_time_hours = $19,
                requires_retest = $20, retest_at = $21, last_retest_at = $22, communications_json = $23,
                timeline_json = $24, tags_json = $25, metadata_json = $26, submitted_at = $27, updated_at = $28, closed_at = $29
            WHERE id = $30"#
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
        .bind(optional_timestamp_string_to_datetime(&submission.retest_at))
        .bind(optional_timestamp_string_to_datetime(&submission.last_retest_at))
        .bind(&submission.communications_json)
        .bind(&submission.timeline_json)
        .bind(&submission.tags_json)
        .bind(&submission.metadata_json)
        .bind(optional_timestamp_string_to_datetime(&submission.submitted_at))
        .bind(timestamp_string_to_datetime(&submission.updated_at))
        .bind(optional_timestamp_string_to_datetime(&submission.closed_at))
        .bind(&submission.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty submission
    pub async fn delete_bounty_submission(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_submissions WHERE id = $1")
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
            params.push(pid.to_string());
            query.push_str(&format!(" AND program_id = ${}", params.len()));
        }

        if let Some(fid) = finding_id {
            params.push(fid.to_string());
            query.push_str(&format!(" AND finding_id = ${}", params.len()));
        }

        if let Some(statuses) = statuses {
            if !statuses.is_empty() {
                let mut placeholders = Vec::new();
                for _ in statuses {
                    placeholders.push(format!("${}", params.len() + 1 + placeholders.len()));
                }
                query.push_str(&format!(" AND status IN ({})", placeholders.join(",")));
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

        let mut sqlx_query = sqlx::query(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param);
        }

        let rows = sqlx_query.fetch_all(self.pool()).await?;
        Ok(rows.into_iter().map(row_to_bounty_submission).collect())
    }

    /// Get bounty submission statistics
    pub async fn get_bounty_submission_stats(&self, program_id: Option<&str>) -> Result<BountySubmissionStats> {
        let base_where = program_id.map(|_| "WHERE program_id = $1").unwrap_or("");
        
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)"#
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
        .bind(timestamp_string_to_datetime(&event.created_at))
        .bind(timestamp_string_to_datetime(&event.updated_at))
        .bind(optional_timestamp_string_to_datetime(&event.resolved_at))
        .execute(self.pool())
        .await?;

        info!("Created bounty change event: {}", event.id);
        Ok(())
    }

    /// Get a change event by ID
    pub async fn get_bounty_change_event(&self, id: &str) -> Result<Option<BountyChangeEventRow>> {
        let row = sqlx::query("SELECT * FROM bounty_change_events WHERE id = $1")
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(row_to_bounty_change_event))
    }

    /// Update a change event
    pub async fn update_bounty_change_event(&self, event: &BountyChangeEventRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_change_events SET
                program_id = $1, asset_id = $2, event_type = $3, severity = $4, status = $5,
                title = $6, description = $7, old_value = $8, new_value = $9, diff = $10,
                affected_scope = $11, detection_method = $12, triggered_workflows_json = $13,
                generated_findings_json = $14, tags_json = $15, metadata_json = $16,
                risk_score = $17, auto_trigger_enabled = $18, updated_at = $19, resolved_at = $20
            WHERE id = $21"#
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
        .bind(timestamp_string_to_datetime(&event.updated_at))
        .bind(optional_timestamp_string_to_datetime(&event.resolved_at))
        .bind(&event.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a change event
    pub async fn delete_bounty_change_event(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_change_events WHERE id = $1")
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
            params.push(pid.to_string());
            query.push_str(&format!(" AND program_id = ${}", params.len()));
        }

        if let Some(aid) = asset_id {
            params.push(aid.to_string());
            query.push_str(&format!(" AND asset_id = ${}", params.len()));
        }

        if let Some(types) = event_types {
            if !types.is_empty() {
                let mut placeholders = Vec::new();
                for _ in types {
                    placeholders.push(format!("${}", params.len() + 1 + placeholders.len()));
                }
                query.push_str(&format!(" AND event_type IN ({})", placeholders.join(",")));
                params.extend(types.iter().cloned());
            }
        }

        if let Some(sevs) = severities {
            if !sevs.is_empty() {
                let mut placeholders = Vec::new();
                for _ in sevs {
                    placeholders.push(format!("${}", params.len() + 1 + placeholders.len()));
                }
                query.push_str(&format!(" AND severity IN ({})", placeholders.join(",")));
                params.extend(sevs.iter().cloned());
            }
        }

        if let Some(stats) = statuses {
            if !stats.is_empty() {
                let mut placeholders = Vec::new();
                for _ in stats {
                    placeholders.push(format!("${}", params.len() + 1 + placeholders.len()));
                }
                query.push_str(&format!(" AND status IN ({})", placeholders.join(",")));
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

        let mut sqlx_query = sqlx::query(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param);
        }

        let rows = sqlx_query.fetch_all(self.pool()).await?;
        Ok(rows.into_iter().map(row_to_bounty_change_event).collect())
    }

    /// Get change event statistics
    pub async fn get_bounty_change_event_stats(&self, program_id: Option<&str>) -> Result<BountyChangeEventStats> {
        let base_where = program_id.map(|_| "WHERE program_id = $1").unwrap_or("");
        
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
            "UPDATE bounty_change_events SET status = $1, resolved_at = $2, updated_at = $3 WHERE id = $4"
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#
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
        .bind(timestamp_string_to_datetime(&template.created_at))
        .bind(timestamp_string_to_datetime(&template.updated_at))
        .execute(self.pool())
        .await?;

        info!("Created bounty workflow template: {}", template.id);
        Ok(())
    }

    /// Get a workflow template by ID
    pub async fn get_bounty_workflow_template(&self, id: &str) -> Result<Option<BountyWorkflowTemplateRow>> {
        let row = sqlx::query("SELECT * FROM bounty_workflow_templates WHERE id = $1")
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(row_to_bounty_workflow_template))
    }

    /// List workflow templates
    pub async fn list_bounty_workflow_templates(
        &self,
        category: Option<&str>,
        is_built_in: Option<bool>,
    ) -> Result<Vec<BountyWorkflowTemplateRow>> {
        let mut query = String::from("SELECT * FROM bounty_workflow_templates WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(cat) = category {
            params.push(cat.to_string());
            query.push_str(&format!(" AND category = ${}", params.len()));
        }
        if let Some(built_in) = is_built_in {
            query.push_str(&format!(" AND is_built_in = {}", built_in));
        }

        query.push_str(" ORDER BY name ASC");
        
        // Note: is_built_in is handled via literal boolean formatting for Postgres (TRUE/FALSE)
        // category is bound.

        let mut sqlx_query = sqlx::query(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param.clone());
        }

        let rows = sqlx_query.fetch_all(self.pool()).await?;

        Ok(rows.into_iter().map(row_to_bounty_workflow_template).collect())
    }

    /// Update a workflow template
    pub async fn update_bounty_workflow_template(&self, template: &BountyWorkflowTemplateRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_workflow_templates SET
                name = $1, description = $2, category = $3, workflow_definition_id = $4,
                steps_json = $5, input_schema_json = $6, output_schema_json = $7,
                tags_json = $8, estimated_duration_mins = $9, updated_at = $10
            WHERE id = $11"#
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
        .bind(timestamp_string_to_datetime(&template.updated_at))
        .bind(&template.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a workflow template
    pub async fn delete_bounty_workflow_template(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_workflow_templates WHERE id = $1")
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
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#
        )
        .bind(&binding.id)
        .bind(&binding.program_id)
        .bind(&binding.scope_id)
        .bind(&binding.workflow_template_id)
        .bind(binding.is_enabled)
        .bind(binding.auto_run_on_change)
        .bind(&binding.trigger_conditions_json)
        .bind(&binding.schedule_cron)
        .bind(optional_timestamp_string_to_datetime(&binding.last_run_at))
        .bind(&binding.last_run_status)
        .bind(binding.run_count)
        .bind(timestamp_string_to_datetime(&binding.created_at))
        .bind(timestamp_string_to_datetime(&binding.updated_at))
        .execute(self.pool())
        .await?;

        info!("Created bounty workflow binding: {}", binding.id);
        Ok(())
    }

    /// Get a workflow binding by ID
    pub async fn get_bounty_workflow_binding(&self, id: &str) -> Result<Option<BountyWorkflowBindingRow>> {
        let row = sqlx::query("SELECT * FROM bounty_workflow_bindings WHERE id = $1")
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(row_to_bounty_workflow_binding))
    }

    /// List workflow bindings for a program
    pub async fn list_bounty_workflow_bindings(
        &self,
        program_id: Option<&str>,
        scope_id: Option<&str>,
        is_enabled: Option<bool>,
    ) -> Result<Vec<BountyWorkflowBindingRow>> {
        let mut query = String::from("SELECT * FROM bounty_workflow_bindings WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(pid) = program_id {
            params.push(pid.to_string());
            query.push_str(&format!(" AND program_id = ${}", params.len()));
        }
        if let Some(sid) = scope_id {
            params.push(sid.to_string());
            query.push_str(&format!(" AND scope_id = ${}", params.len()));
        }
        if let Some(enabled) = is_enabled {
            query.push_str(&format!(" AND is_enabled = {}", enabled));
        }

        query.push_str(" ORDER BY created_at DESC");

        let mut sqlx_query = sqlx::query(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param.clone());
        }

        let rows = sqlx_query.fetch_all(self.pool()).await?;

        Ok(rows.into_iter().map(row_to_bounty_workflow_binding).collect())
    }

    /// Update a workflow binding
    pub async fn update_bounty_workflow_binding(&self, binding: &BountyWorkflowBindingRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_workflow_bindings SET
                scope_id = $1, is_enabled = $2, auto_run_on_change = $3,
                trigger_conditions_json = $4, schedule_cron = $5,
                last_run_at = $6, last_run_status = $7, run_count = $8, updated_at = $9
            WHERE id = $10"#
        )
        .bind(&binding.scope_id)
        .bind(binding.is_enabled)
        .bind(binding.auto_run_on_change)
        .bind(&binding.trigger_conditions_json)
        .bind(&binding.schedule_cron)
        .bind(optional_timestamp_string_to_datetime(&binding.last_run_at))
        .bind(&binding.last_run_status)
        .bind(binding.run_count)
        .bind(timestamp_string_to_datetime(&binding.updated_at))
        .bind(&binding.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a workflow binding
    pub async fn delete_bounty_workflow_binding(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_workflow_bindings WHERE id = $1")
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
                last_run_at = $1, last_run_status = $2, run_count = run_count + 1, updated_at = $3
            WHERE id = $4"#
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
        let rows = sqlx::query(
            r#"SELECT * FROM bounty_workflow_bindings
               WHERE program_id = $1 AND is_enabled = TRUE AND auto_run_on_change = TRUE"#
        )
        .bind(program_id)
        .fetch_all(self.pool())
        .await?;

        Ok(rows.into_iter().map(row_to_bounty_workflow_binding).collect())
    }
}

// ============================================================================
// Bounty Asset Models (P1-B3: Asset Consolidation)
// ============================================================================

/// Bounty asset model (Enhanced for ASM - Attack Surface Management)
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    
    // ========== P0: Core ASM Attributes ==========
    
    // IP Asset Attributes
    pub ip_version: Option<String>,           // IPv4/IPv6
    pub asn: Option<i32>,                     // Autonomous System Number
    pub asn_org: Option<String>,              // ASN Organization
    pub isp: Option<String>,                  // ISP Provider
    pub country: Option<String>,              // Country Code
    pub city: Option<String>,                 // City
    pub latitude: Option<f64>,                // Latitude
    pub longitude: Option<f64>,               // Longitude
    pub is_cloud: Option<bool>,               // Is Cloud Service
    pub cloud_provider: Option<String>,       // AWS/Azure/GCP/Alibaba
    
    // Port/Service Attributes
    pub service_name: Option<String>,         // Service name (ssh, http, mysql)
    pub service_version: Option<String>,      // Service version
    pub service_product: Option<String>,      // Product name (nginx, apache)
    pub banner: Option<String>,               // Service banner
    pub transport_protocol: Option<String>,   // TCP/UDP
    pub cpe: Option<String>,                  // Common Platform Enumeration
    
    // Domain Attributes
    pub domain_registrar: Option<String>,     // Domain registrar
    pub registration_date: Option<String>,    // Registration date
    pub expiration_date: Option<String>,      // Expiration date
    pub nameservers_json: Option<String>,     // NS servers
    pub mx_records_json: Option<String>,      // MX records
    pub txt_records_json: Option<String>,     // TXT records
    pub whois_data_json: Option<String>,      // WHOIS data
    pub is_wildcard: Option<bool>,            // Is wildcard domain
    pub parent_domain: Option<String>,        // Parent domain
    
    // Web/URL Attributes
    pub http_status: Option<i32>,             // HTTP status code
    pub response_time_ms: Option<i32>,        // Response time
    pub content_length: Option<i64>,          // Content length
    pub content_type: Option<String>,         // Content-Type header
    pub title: Option<String>,                // Page title
    pub favicon_hash: Option<String>,         // Favicon hash
    pub headers_json: Option<String>,         // HTTP headers
    pub waf_detected: Option<String>,         // WAF detection
    pub cdn_detected: Option<String>,         // CDN detection
    pub screenshot_path: Option<String>,      // Screenshot path
    pub body_hash: Option<String>,            // Page body hash
    
    // Certificate Attributes
    pub certificate_id: Option<String>,       // Related certificate ID
    pub ssl_enabled: Option<bool>,            // SSL/TLS enabled
    pub certificate_subject: Option<String>,  // Certificate subject
    pub certificate_issuer: Option<String>,   // Certificate issuer
    pub certificate_valid_from: Option<String>, // Certificate valid from
    pub certificate_valid_to: Option<String>,   // Certificate valid to
    pub certificate_san_json: Option<String>,   // Subject Alternative Names
    
    // Attack Surface & Risk
    pub exposure_level: Option<String>,       // internet/intranet/private
    pub attack_surface_score: Option<f64>,    // Attack surface score (0-100)
    pub vulnerability_count: Option<i32>,     // Known vulnerabilities count
    pub cvss_max_score: Option<f64>,          // Highest CVSS score
    pub exploit_available: Option<bool>,      // Exploit available
    
    // Asset Classification
    pub asset_category: Option<String>,       // external/internal/third-party
    pub asset_owner: Option<String>,          // Asset owner
    pub business_unit: Option<String>,        // Business unit
    pub criticality: Option<String>,          // critical/high/medium/low
    
    // Discovery & Monitoring
    pub discovery_method: Option<String>,     // passive/active/manual
    pub data_sources_json: Option<String>,    // Data sources (shodan, censys, etc)
    pub confidence_score: Option<f64>,        // Confidence score (0-1)
    pub monitoring_enabled: Option<bool>,     // Monitoring enabled
    pub scan_frequency: Option<String>,       // Scan frequency (daily/weekly/monthly)
    pub last_scan_type: Option<String>,       // Last scan type
    
    // Asset Relationships
    pub parent_asset_id: Option<String>,      // Parent asset ID
    pub related_assets_json: Option<String>,  // Related assets
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
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27)"#
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
        .bind(optional_timestamp_string_to_datetime(&asset.last_checked_at))
        .bind(timestamp_string_to_datetime(&asset.first_seen_at))
        .bind(timestamp_string_to_datetime(&asset.last_seen_at))
        .bind(asset.findings_count)
        .bind(asset.change_events_count)
        .bind(&asset.metadata_json)
        .bind(timestamp_string_to_datetime(&asset.created_at))
        .bind(timestamp_string_to_datetime(&asset.updated_at))
        .execute(self.pool())
        .await?;

        info!("Created bounty asset: {}", asset.id);
        Ok(())
    }

    /// Get a bounty asset by ID
    pub async fn get_bounty_asset(&self, id: &str) -> Result<Option<BountyAssetRow>> {
        let row = sqlx::query("SELECT * FROM bounty_assets WHERE id = $1")
        .bind(id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(row_to_bounty_asset))
    }

    /// Get a bounty asset by canonical URL
    pub async fn get_bounty_asset_by_canonical_url(
        &self,
        program_id: &str,
        canonical_url: &str,
    ) -> Result<Option<BountyAssetRow>> {
        let row = sqlx::query("SELECT * FROM bounty_assets WHERE program_id = $1 AND canonical_url = $2")
        .bind(program_id)
        .bind(canonical_url)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(row_to_bounty_asset))
    }

    /// Get a bounty asset by fingerprint
    pub async fn get_bounty_asset_by_fingerprint(
        &self,
        program_id: &str,
        fingerprint: &str,
    ) -> Result<Option<BountyAssetRow>> {
        let row = sqlx::query("SELECT * FROM bounty_assets WHERE program_id = $1 AND fingerprint = $2")
        .bind(program_id)
        .bind(fingerprint)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(row_to_bounty_asset))
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
        let mut params: Vec<String> = Vec::new();

        if let Some(pid) = program_id {
            params.push(pid.to_string());
            query.push_str(&format!(" AND program_id = ${}", params.len()));
        }
        if let Some(sid) = scope_id {
            params.push(sid.to_string());
            query.push_str(&format!(" AND scope_id = ${}", params.len()));
        }
        if let Some(at) = asset_type {
            params.push(at.to_string());
            query.push_str(&format!(" AND asset_type = ${}", params.len()));
        }
        if let Some(alive) = is_alive {
            query.push_str(&format!(" AND is_alive = {}", alive));
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

        let mut sqlx_query = sqlx::query(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param.clone());
        }

        let rows = sqlx_query.fetch_all(self.pool()).await?;

        Ok(rows.into_iter().map(row_to_bounty_asset).collect())
    }

    /// Update a bounty asset
    pub async fn update_bounty_asset(&self, asset: &BountyAssetRow) -> Result<bool> {
        let result = sqlx::query(
            r#"UPDATE bounty_assets SET
                scope_id = $1, asset_type = $2, canonical_url = $3, original_urls_json = $4,
                hostname = $5, port = $6, path = $7, protocol = $8, ip_addresses_json = $9,
                dns_records_json = $10, tech_stack_json = $11, fingerprint = $12, tags_json = $13,
                labels_json = $14, priority_score = $15, risk_score = $16, is_alive = $17,
                last_checked_at = $18, last_seen_at = $19, findings_count = $20,
                change_events_count = $21, metadata_json = $22, updated_at = $23
            WHERE id = $24"#
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
        .bind(optional_timestamp_string_to_datetime(&asset.last_checked_at))
        .bind(timestamp_string_to_datetime(&asset.last_seen_at))
        .bind(asset.findings_count)
        .bind(asset.change_events_count)
        .bind(&asset.metadata_json)
        .bind(timestamp_string_to_datetime(&asset.updated_at))
        .bind(&asset.id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty asset
    pub async fn delete_bounty_asset(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM bounty_assets WHERE id = $1")
            .bind(id)
            .execute(self.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get bounty asset statistics
    pub async fn get_bounty_asset_stats(&self, program_id: Option<&str>) -> Result<BountyAssetStats> {
        let pool = self.pool();
        let filter = program_id.map(|_| " WHERE program_id = $1").unwrap_or_default();

        // Note: For stats query with simple counts, binding $1 works even if ignored by some drivers, but Postgres is strict.
        // We need to use `bind` only if program_id exists.

        // Total
        let sql_total = format!("SELECT COUNT(*) FROM bounty_assets{}", filter);
        let mut q = sqlx::query_as::<_, (i32,)>(&sql_total);
        if let Some(p) = program_id { q = q.bind(p); }
        let total = q.fetch_one(pool).await?;

        // Alive
        let sql_alive = format!(
            "SELECT COUNT(*) FROM bounty_assets{} AND is_alive = TRUE",
            if filter.is_empty() { " WHERE 1=1" } else { &filter }
        );
        let mut q = sqlx::query_as::<_, (i32,)>(&sql_alive);
        if let Some(p) = program_id { q = q.bind(p); }
        let alive = q.fetch_one(pool).await?;

        // With findings
        let sql_findings = format!(
            "SELECT COUNT(*) FROM bounty_assets{} AND findings_count > 0",
            if filter.is_empty() { " WHERE 1=1" } else { &filter }
        );
        let mut q = sqlx::query_as::<_, (i32,)>(&sql_findings);
        if let Some(p) = program_id { q = q.bind(p); }
        let with_findings = q.fetch_one(pool).await?;

        // High priority
        let sql_high = format!(
            "SELECT COUNT(*) FROM bounty_assets{} AND priority_score >= 7.0",
            if filter.is_empty() { " WHERE 1=1" } else { &filter }
        );
        let mut q = sqlx::query_as::<_, (i32,)>(&sql_high);
        if let Some(p) = program_id { q = q.bind(p); }
        let high_priority = q.fetch_one(pool).await?;

        // Get by type
        let sql_by_type = format!(
            "SELECT asset_type, COUNT(*) FROM bounty_assets{} GROUP BY asset_type",
            filter
        );
        let mut q = sqlx::query_as::<_, (String, i32)>(&sql_by_type);
        if let Some(p) = program_id { q = q.bind(p); }
        let type_rows = q.fetch_all(pool).await?;

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
        let rows = sqlx::query(
            r#"SELECT * FROM bounty_assets
               WHERE program_id = $1 AND is_alive = TRUE
               ORDER BY priority_score DESC
               LIMIT $2"#
        )
        .bind(program_id)
        .bind(limit)
        .fetch_all(self.pool())
        .await?;

        Ok(rows.into_iter().map(row_to_bounty_asset).collect())
    }
}

//! Bug Bounty database operations

use chrono;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::database_service::sqlx_compat::PgRow;

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert string timestamp to DateTime<Utc> for database binding
pub(super) fn timestamp_string_to_datetime(s: &str) -> chrono::DateTime<chrono::Utc> {
    s.parse::<chrono::DateTime<chrono::Utc>>()
        .unwrap_or_else(|_| chrono::Utc::now())
}

/// Convert optional string timestamp to Option<DateTime<Utc>> for database binding
pub(super) fn optional_timestamp_string_to_datetime(
    s: &Option<String>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    s.as_ref()
        .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok())
}

/// Convert TIMESTAMP WITH TIME ZONE to String for struct fields
pub(super) fn timestamp_to_string(row: &PgRow, column: &str) -> String {
    row.try_get::<chrono::DateTime<chrono::Utc>, _>(column)
        .map(|dt| dt.to_rfc3339())
        .or_else(|_| row.try_get::<String, _>(column))
        .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339())
}

/// Convert optional TIMESTAMP WITH TIME ZONE to Option<String> for struct fields
pub(super) fn optional_timestamp_to_string(row: &PgRow, column: &str) -> Option<String> {
    row.try_get::<chrono::DateTime<chrono::Utc>, _>(column)
        .map(|dt| Some(dt.to_rfc3339()))
        .or_else(|_| row.try_get::<Option<String>, _>(column))
        .unwrap_or(None)
}

/// Map PgRow to BountyProgramRow
pub(super) fn row_to_bounty_program(row: PgRow) -> BountyProgramRow {
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
pub(super) fn row_to_program_scope(row: PgRow) -> ProgramScopeRow {
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
pub(super) fn row_to_bounty_finding(row: PgRow) -> BountyFindingRow {
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
pub(super) fn row_to_bounty_submission(row: PgRow) -> BountySubmissionRow {
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
pub(super) fn row_to_bounty_evidence(row: PgRow) -> BountyEvidenceRow {
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
pub(super) fn row_to_bounty_change_event(row: PgRow) -> BountyChangeEventRow {
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
pub(super) fn row_to_bounty_workflow_template(row: PgRow) -> BountyWorkflowTemplateRow {
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

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct BountyProgramCompatRow {
    id: String,
    name: String,
    organization: String,
    platform: String,
    platform_handle: Option<String>,
    url: Option<String>,
    program_type: String,
    status: String,
    description: Option<String>,
    rewards_json: Option<String>,
    response_sla_days: Option<i32>,
    resolution_sla_days: Option<i32>,
    rules_json: Option<String>,
    tags_json: Option<String>,
    metadata_json: Option<String>,
    priority_score: f64,
    total_submissions: i32,
    accepted_submissions: i32,
    total_earnings: f64,
    created_at: String,
    updated_at: String,
    last_activity_at: Option<String>,
}

impl From<BountyProgramCompatRow> for BountyProgramRow {
    fn from(row: BountyProgramCompatRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            organization: row.organization,
            platform: row.platform,
            platform_handle: row.platform_handle,
            url: row.url,
            program_type: row.program_type,
            status: row.status,
            description: row.description,
            rewards_json: row.rewards_json,
            response_sla_days: row.response_sla_days,
            resolution_sla_days: row.resolution_sla_days,
            rules_json: row.rules_json,
            tags_json: row.tags_json,
            metadata_json: row.metadata_json,
            priority_score: row.priority_score,
            total_submissions: row.total_submissions,
            accepted_submissions: row.accepted_submissions,
            total_earnings: row.total_earnings,
            created_at: row.created_at,
            updated_at: row.updated_at,
            last_activity_at: row.last_activity_at,
        }
    }
}

/// Map PgRow to BountyWorkflowBindingRow
pub(super) fn row_to_bounty_workflow_binding(row: PgRow) -> BountyWorkflowBindingRow {
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
pub(super) fn row_to_bounty_asset(row: PgRow) -> BountyAssetRow {
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
        root_domain: row.get("root_domain"),
        subdomain_level: row.get("subdomain_level"),
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
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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

// ============================================================================
// Workflow Template Models
// ============================================================================

/// Bounty workflow template
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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

// ============================================================================
// Bounty Asset Models (P1-B3: Asset Consolidation)
// ============================================================================

/// Bounty asset model (Enhanced for ASM - Attack Surface Management)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
    pub ip_version: Option<String>,     // IPv4/IPv6
    pub asn: Option<i32>,               // Autonomous System Number
    pub asn_org: Option<String>,        // ASN Organization
    pub isp: Option<String>,            // ISP Provider
    pub country: Option<String>,        // Country Code
    pub city: Option<String>,           // City
    pub latitude: Option<f64>,          // Latitude
    pub longitude: Option<f64>,         // Longitude
    pub is_cloud: Option<bool>,         // Is Cloud Service
    pub cloud_provider: Option<String>, // AWS/Azure/GCP/Alibaba

    // Port/Service Attributes
    pub service_name: Option<String>, // Service name (ssh, http, mysql)
    pub service_version: Option<String>, // Service version
    pub service_product: Option<String>, // Product name (nginx, apache)
    pub banner: Option<String>,       // Service banner
    pub transport_protocol: Option<String>, // TCP/UDP
    pub cpe: Option<String>,          // Common Platform Enumeration

    // Domain Attributes
    pub domain_registrar: Option<String>,  // Domain registrar
    pub registration_date: Option<String>, // Registration date
    pub expiration_date: Option<String>,   // Expiration date
    pub nameservers_json: Option<String>,  // NS servers
    pub mx_records_json: Option<String>,   // MX records
    pub txt_records_json: Option<String>,  // TXT records
    pub whois_data_json: Option<String>,   // WHOIS data
    pub is_wildcard: Option<bool>,         // Is wildcard domain
    pub parent_domain: Option<String>,     // Parent domain
    pub root_domain: Option<String>,       // Registrable root domain
    pub subdomain_level: Option<i32>,      // 0=root, 1=first-level subdomain

    // Web/URL Attributes
    pub http_status: Option<i32>,        // HTTP status code
    pub response_time_ms: Option<i32>,   // Response time
    pub content_length: Option<i64>,     // Content length
    pub content_type: Option<String>,    // Content-Type header
    pub title: Option<String>,           // Page title
    pub favicon_hash: Option<String>,    // Favicon hash
    pub headers_json: Option<String>,    // HTTP headers
    pub waf_detected: Option<String>,    // WAF detection
    pub cdn_detected: Option<String>,    // CDN detection
    pub screenshot_path: Option<String>, // Screenshot path
    pub body_hash: Option<String>,       // Page body hash

    // Certificate Attributes
    pub certificate_id: Option<String>, // Related certificate ID
    pub ssl_enabled: Option<bool>,      // SSL/TLS enabled
    pub certificate_subject: Option<String>, // Certificate subject
    pub certificate_issuer: Option<String>, // Certificate issuer
    pub certificate_valid_from: Option<String>, // Certificate valid from
    pub certificate_valid_to: Option<String>, // Certificate valid to
    pub certificate_san_json: Option<String>, // Subject Alternative Names

    // Attack Surface & Risk
    pub exposure_level: Option<String>, // internet/intranet/private
    pub attack_surface_score: Option<f64>, // Attack surface score (0-100)
    pub vulnerability_count: Option<i32>, // Known vulnerabilities count
    pub cvss_max_score: Option<f64>,    // Highest CVSS score
    pub exploit_available: Option<bool>, // Exploit available

    // Asset Classification
    pub asset_category: Option<String>, // external/internal/third-party
    pub asset_owner: Option<String>,    // Asset owner
    pub business_unit: Option<String>,  // Business unit
    pub criticality: Option<String>,    // critical/high/medium/low

    // Discovery & Monitoring
    pub discovery_method: Option<String>, // passive/active/manual
    pub data_sources_json: Option<String>, // Data sources (shodan, censys, etc)
    pub confidence_score: Option<f64>,    // Confidence score (0-1)
    pub monitoring_enabled: Option<bool>, // Monitoring enabled
    pub scan_frequency: Option<String>,   // Scan frequency (daily/weekly/monthly)
    pub last_scan_type: Option<String>,   // Last scan type

    // Asset Relationships
    pub parent_asset_id: Option<String>,     // Parent asset ID
    pub related_assets_json: Option<String>, // Related assets
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

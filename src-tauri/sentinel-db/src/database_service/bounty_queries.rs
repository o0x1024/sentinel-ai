use anyhow::Result;
use sqlx::{Database, Encode, QueryBuilder, Row, Type};

use crate::database_service::bounty::{
    BountyChangeEventStats, BountyFindingRow, BountyFindingStats, BountyProgramRow,
    BountyProgramStats, BountySubmissionRow, BountySubmissionStats,
};
use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{PgRow, Postgres};

#[derive(Debug, Clone, sqlx::FromRow)]
struct BountyProgramCompatRow {
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

#[derive(Debug, Clone)]
pub struct ProgramQueryFilter<'a> {
    pub platforms: Option<&'a [String]>,
    pub statuses: Option<&'a [String]>,
    pub program_types: Option<&'a [String]>,
    pub tags: Option<&'a [String]>,
    pub search: Option<&'a str>,
    pub min_priority: Option<f64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct FindingQueryFilter<'a> {
    pub program_id: Option<&'a str>,
    pub scope_id: Option<&'a str>,
    pub severities: Option<&'a [String]>,
    pub statuses: Option<&'a [String]>,
    pub search: Option<&'a str>,
    pub sort_by: Option<&'a str>,
    pub sort_dir: Option<&'a str>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SubmissionQueryFilter<'a> {
    pub program_id: Option<&'a str>,
    pub finding_id: Option<&'a str>,
    pub statuses: Option<&'a [String]>,
    pub search: Option<&'a str>,
    pub sort_by: Option<&'a str>,
    pub sort_dir: Option<&'a str>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

fn timestamp_to_string(row: &PgRow, column: &str) -> String {
    row.try_get::<chrono::DateTime<chrono::Utc>, _>(column)
        .map(|dt| dt.to_rfc3339())
        .or_else(|_| row.try_get::<String, _>(column))
        .unwrap_or_else(|_| chrono::Utc::now().to_rfc3339())
}

fn optional_timestamp_to_string(row: &PgRow, column: &str) -> Option<String> {
    row.try_get::<chrono::DateTime<chrono::Utc>, _>(column)
        .map(|dt| Some(dt.to_rfc3339()))
        .or_else(|_| row.try_get::<Option<String>, _>(column))
        .unwrap_or(None)
}

fn row_to_bounty_program(row: PgRow) -> BountyProgramRow {
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

fn row_to_bounty_finding(row: PgRow) -> BountyFindingRow {
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

fn row_to_bounty_submission(row: PgRow) -> BountySubmissionRow {
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

fn push_string_equals<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    column: &str,
    value: Option<&str>,
) where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
{
    if let Some(value) = value {
        query_builder
            .push(" AND ")
            .push(column)
            .push(" = ")
            .push_bind(value.to_string());
    }
}

fn push_string_list<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    column: &str,
    values: Option<&[String]>,
) where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
{
    if let Some(values) = values {
        if values.is_empty() {
            return;
        }
        query_builder.push(" AND ").push(column).push(" IN (");
        {
            let mut separated = query_builder.separated(", ");
            for value in values {
                separated.push_bind(value.clone());
            }
        }
        query_builder.push(")");
    }
}

fn push_search_clause<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    columns: &[&str],
    search: Option<&str>,
) where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
{
    let Some(search) = search.map(str::trim).filter(|s| !s.is_empty()) else {
        return;
    };
    let pattern = format!("%{}%", search.to_lowercase());
    query_builder.push(" AND (");
    for (index, column) in columns.iter().enumerate() {
        if index > 0 {
            query_builder.push(" OR ");
        }
        query_builder
            .push("LOWER(COALESCE(")
            .push(*column)
            .push(", '')) LIKE ")
            .push_bind(pattern.clone());
    }
    query_builder.push(")");
}

fn push_program_tag_clause<'args, DB>(
    query_builder: &mut QueryBuilder<'args, DB>,
    tags: Option<&[String]>,
) where
    DB: Database,
    String: for<'q> Encode<'q, DB> + Type<DB>,
{
    let Some(tags) = tags else {
        return;
    };
    if tags.is_empty() {
        return;
    }

    query_builder.push(" AND (");
    for (index, tag) in tags.iter().enumerate() {
        if index > 0 {
            query_builder.push(" OR ");
        }
        query_builder
            .push("LOWER(COALESCE(tags_json, '')) LIKE ")
            .push_bind(format!("%\"{}\"%", tag.to_lowercase()));
    }
    query_builder.push(")");
}

fn finding_sort_column(sort_by: Option<&str>) -> &'static str {
    match sort_by.unwrap_or("created_at") {
        "created_at" => "created_at",
        "updated_at" => "updated_at",
        "severity" => "severity",
        "status" => "status",
        "title" => "title",
        "cvss_score" => "cvss_score",
        "last_seen_at" => "last_seen_at",
        _ => "created_at",
    }
}

fn submission_sort_column(sort_by: Option<&str>) -> &'static str {
    match sort_by.unwrap_or("created_at") {
        "created_at" => "created_at",
        "updated_at" => "updated_at",
        "severity" => "severity",
        "status" => "status",
        "priority" => "priority",
        "reward_amount" => "reward_amount",
        "submitted_at" => "submitted_at",
        _ => "created_at",
    }
}

fn sort_direction(sort_dir: Option<&str>) -> &'static str {
    match sort_dir.unwrap_or("desc").to_lowercase().as_str() {
        "asc" => "ASC",
        _ => "DESC",
    }
}

const BOUNTY_BATCH_MUTATION_SIZE: usize = 200;

fn dedupe_non_empty_ids(ids: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::with_capacity(ids.len());
    let mut deduped = Vec::with_capacity(ids.len());
    for id in ids {
        let id = id.trim();
        if id.is_empty() {
            continue;
        }
        if seen.insert(id.to_string()) {
            deduped.push(id.to_string());
        }
    }
    deduped
}

impl DatabaseService {
    pub async fn list_bounty_programs_filtered(
        &self,
        filter: ProgramQueryFilter<'_>,
    ) -> Result<Vec<BountyProgramRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let sqlite_mysql_select = r#"
            SELECT
                id, name, organization, platform, platform_handle, url, program_type, status,
                description, rewards_json, response_sla_days, resolution_sla_days, rules_json,
                tags_json, metadata_json, priority_score, total_submissions, accepted_submissions,
                total_earnings, CAST(created_at AS TEXT) AS created_at,
                CAST(updated_at AS TEXT) AS updated_at,
                CAST(last_activity_at AS TEXT) AS last_activity_at
            FROM bounty_programs
            WHERE 1=1
        "#;
        let pg_select = "SELECT * FROM bounty_programs WHERE 1=1";

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut qb = QueryBuilder::<sqlx::Sqlite>::new(sqlite_mysql_select);
                push_string_list(&mut qb, "platform", filter.platforms);
                push_string_list(&mut qb, "status", filter.statuses);
                push_string_list(&mut qb, "program_type", filter.program_types);
                push_program_tag_clause(&mut qb, filter.tags);
                push_search_clause(&mut qb, &["name", "organization"], filter.search);
                if let Some(min_priority) = filter.min_priority {
                    qb.push(" AND priority_score >= ").push_bind(min_priority);
                }
                qb.push(" ORDER BY priority_score DESC, updated_at DESC");
                if let Some(limit) = filter.limit {
                    qb.push(" LIMIT ").push_bind(limit as i64);
                }
                if let Some(offset) = filter.offset {
                    qb.push(" OFFSET ").push_bind(offset as i64);
                }
                let rows = qb
                    .build_query_as::<BountyProgramCompatRow>()
                    .fetch_all(pool)
                    .await?;
                Ok(rows.into_iter().map(Into::into).collect())
            }
            DatabasePool::MySQL(pool) => {
                let mut qb = QueryBuilder::<crate::database_service::sqlx_compat::MySql>::new(
                    sqlite_mysql_select,
                );
                push_string_list(&mut qb, "platform", filter.platforms);
                push_string_list(&mut qb, "status", filter.statuses);
                push_string_list(&mut qb, "program_type", filter.program_types);
                push_program_tag_clause(&mut qb, filter.tags);
                push_search_clause(&mut qb, &["name", "organization"], filter.search);
                if let Some(min_priority) = filter.min_priority {
                    qb.push(" AND priority_score >= ").push_bind(min_priority);
                }
                qb.push(" ORDER BY priority_score DESC, updated_at DESC");
                if let Some(limit) = filter.limit {
                    qb.push(" LIMIT ").push_bind(limit as i64);
                }
                if let Some(offset) = filter.offset {
                    qb.push(" OFFSET ").push_bind(offset as i64);
                }
                let rows = qb
                    .build_query_as::<BountyProgramCompatRow>()
                    .fetch_all(pool)
                    .await?;
                Ok(rows.into_iter().map(Into::into).collect())
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut qb = QueryBuilder::<Postgres>::new(pg_select);
                push_string_list(&mut qb, "platform", filter.platforms);
                push_string_list(&mut qb, "status", filter.statuses);
                push_string_list(&mut qb, "program_type", filter.program_types);
                push_program_tag_clause(&mut qb, filter.tags);
                push_search_clause(&mut qb, &["name", "organization"], filter.search);
                if let Some(min_priority) = filter.min_priority {
                    qb.push(" AND priority_score >= ").push_bind(min_priority);
                }
                qb.push(" ORDER BY priority_score DESC, updated_at DESC");
                if let Some(limit) = filter.limit {
                    qb.push(" LIMIT ").push_bind(limit as i64);
                }
                if let Some(offset) = filter.offset {
                    qb.push(" OFFSET ").push_bind(offset as i64);
                }
                let rows = qb.build().fetch_all(pool).await?;
                Ok(rows.into_iter().map(row_to_bounty_program).collect())
            }
        }
    }

    pub async fn get_bounty_program_stats_live(&self) -> Result<BountyProgramStats> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let program_sql = "SELECT COUNT(*) AS total_programs, COUNT(CASE WHEN status = 'active' THEN 1 END) AS active_programs FROM bounty_programs";
        let submission_sql = "SELECT COUNT(*) AS total_submissions, COUNT(CASE WHEN status IN ('accepted', 'resolved', 'paid') THEN 1 END) AS total_accepted, COALESCE(SUM(COALESCE(reward_amount, 0) + COALESCE(bonus_amount, 0)), 0.0) AS total_earnings FROM bounty_submissions";

        let (total_programs, active_programs, total_submissions, total_accepted, total_earnings) =
            match runtime {
                DatabasePool::SQLite(pool) => {
                    let programs: (i64, i64) = sqlx::query_as(program_sql).fetch_one(pool).await?;
                    let submissions: (i64, i64, f64) =
                        sqlx::query_as(submission_sql).fetch_one(pool).await?;
                    (
                        programs.0,
                        programs.1,
                        submissions.0,
                        submissions.1,
                        submissions.2,
                    )
                }
                DatabasePool::MySQL(pool) => {
                    let programs: (i64, i64) = sqlx::query_as(program_sql).fetch_one(pool).await?;
                    let submissions: (i64, i64, f64) =
                        sqlx::query_as(submission_sql).fetch_one(pool).await?;
                    (
                        programs.0,
                        programs.1,
                        submissions.0,
                        submissions.1,
                        submissions.2,
                    )
                }
                DatabasePool::PostgreSQL(pool) => {
                    let programs: (i64, i64) = sqlx::query_as(program_sql).fetch_one(pool).await?;
                    let submissions: (i64, i64, f64) =
                        sqlx::query_as(submission_sql).fetch_one(pool).await?;
                    (
                        programs.0,
                        programs.1,
                        submissions.0,
                        submissions.1,
                        submissions.2,
                    )
                }
            };

        Ok(BountyProgramStats {
            total_programs: total_programs as i32,
            active_programs: active_programs as i32,
            total_submissions: total_submissions as i32,
            total_accepted: total_accepted as i32,
            total_earnings,
        })
    }

    pub async fn list_bounty_findings_filtered(
        &self,
        filter: FindingQueryFilter<'_>,
    ) -> Result<Vec<BountyFindingRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let sql = "SELECT * FROM bounty_findings WHERE 1=1";
        let sort_column = finding_sort_column(filter.sort_by);
        let sort_dir = sort_direction(filter.sort_dir);

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut qb = QueryBuilder::<sqlx::Sqlite>::new(sql);
                push_string_equals(&mut qb, "program_id", filter.program_id);
                push_string_equals(&mut qb, "scope_id", filter.scope_id);
                push_string_list(&mut qb, "severity", filter.severities);
                push_string_list(&mut qb, "status", filter.statuses);
                push_search_clause(&mut qb, &["title", "description"], filter.search);
                qb.push(" ORDER BY ")
                    .push(sort_column)
                    .push(" ")
                    .push(sort_dir);
                if let Some(limit) = filter.limit {
                    qb.push(" LIMIT ").push_bind(limit as i64);
                }
                if let Some(offset) = filter.offset {
                    qb.push(" OFFSET ").push_bind(offset as i64);
                }
                Ok(qb
                    .build_query_as::<BountyFindingRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::MySQL(pool) => {
                let mut qb = QueryBuilder::<crate::database_service::sqlx_compat::MySql>::new(sql);
                push_string_equals(&mut qb, "program_id", filter.program_id);
                push_string_equals(&mut qb, "scope_id", filter.scope_id);
                push_string_list(&mut qb, "severity", filter.severities);
                push_string_list(&mut qb, "status", filter.statuses);
                push_search_clause(&mut qb, &["title", "description"], filter.search);
                qb.push(" ORDER BY ")
                    .push(sort_column)
                    .push(" ")
                    .push(sort_dir);
                if let Some(limit) = filter.limit {
                    qb.push(" LIMIT ").push_bind(limit as i64);
                }
                if let Some(offset) = filter.offset {
                    qb.push(" OFFSET ").push_bind(offset as i64);
                }
                Ok(qb
                    .build_query_as::<BountyFindingRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut qb = QueryBuilder::<Postgres>::new(sql);
                push_string_equals(&mut qb, "program_id", filter.program_id);
                push_string_equals(&mut qb, "scope_id", filter.scope_id);
                push_string_list(&mut qb, "severity", filter.severities);
                push_string_list(&mut qb, "status", filter.statuses);
                push_search_clause(&mut qb, &["title", "description"], filter.search);
                qb.push(" ORDER BY ")
                    .push(sort_column)
                    .push(" ")
                    .push(sort_dir);
                if let Some(limit) = filter.limit {
                    qb.push(" LIMIT ").push_bind(limit as i64);
                }
                if let Some(offset) = filter.offset {
                    qb.push(" OFFSET ").push_bind(offset as i64);
                }
                let rows = qb.build().fetch_all(pool).await?;
                Ok(rows.into_iter().map(row_to_bounty_finding).collect())
            }
        }
    }

    pub async fn count_bounty_findings_filtered(
        &self,
        filter: FindingQueryFilter<'_>,
    ) -> Result<i64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let sql = "SELECT COUNT(*) AS count FROM bounty_findings WHERE 1=1";

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut qb = QueryBuilder::<sqlx::Sqlite>::new(sql);
                push_string_equals(&mut qb, "program_id", filter.program_id);
                push_string_equals(&mut qb, "scope_id", filter.scope_id);
                push_string_list(&mut qb, "severity", filter.severities);
                push_string_list(&mut qb, "status", filter.statuses);
                push_search_clause(&mut qb, &["title", "description"], filter.search);
                Ok(qb.build_query_scalar::<i64>().fetch_one(pool).await?)
            }
            DatabasePool::MySQL(pool) => {
                let mut qb = QueryBuilder::<crate::database_service::sqlx_compat::MySql>::new(sql);
                push_string_equals(&mut qb, "program_id", filter.program_id);
                push_string_equals(&mut qb, "scope_id", filter.scope_id);
                push_string_list(&mut qb, "severity", filter.severities);
                push_string_list(&mut qb, "status", filter.statuses);
                push_search_clause(&mut qb, &["title", "description"], filter.search);
                Ok(qb.build_query_scalar::<i64>().fetch_one(pool).await?)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut qb = QueryBuilder::<Postgres>::new(sql);
                push_string_equals(&mut qb, "program_id", filter.program_id);
                push_string_equals(&mut qb, "scope_id", filter.scope_id);
                push_string_list(&mut qb, "severity", filter.severities);
                push_string_list(&mut qb, "status", filter.statuses);
                push_search_clause(&mut qb, &["title", "description"], filter.search);
                Ok(qb.build_query_scalar::<i64>().fetch_one(pool).await?)
            }
        }
    }

    pub async fn list_bounty_submissions_filtered(
        &self,
        filter: SubmissionQueryFilter<'_>,
    ) -> Result<Vec<BountySubmissionRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let sql = "SELECT * FROM bounty_submissions WHERE 1=1";
        let sort_column = submission_sort_column(filter.sort_by);
        let sort_dir = sort_direction(filter.sort_dir);

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut qb = QueryBuilder::<sqlx::Sqlite>::new(sql);
                push_string_equals(&mut qb, "program_id", filter.program_id);
                push_string_equals(&mut qb, "finding_id", filter.finding_id);
                push_string_list(&mut qb, "status", filter.statuses);
                push_search_clause(&mut qb, &["title", "description"], filter.search);
                qb.push(" ORDER BY ")
                    .push(sort_column)
                    .push(" ")
                    .push(sort_dir);
                if let Some(limit) = filter.limit {
                    qb.push(" LIMIT ").push_bind(limit as i64);
                }
                if let Some(offset) = filter.offset {
                    qb.push(" OFFSET ").push_bind(offset as i64);
                }
                Ok(qb
                    .build_query_as::<BountySubmissionRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::MySQL(pool) => {
                let mut qb = QueryBuilder::<crate::database_service::sqlx_compat::MySql>::new(sql);
                push_string_equals(&mut qb, "program_id", filter.program_id);
                push_string_equals(&mut qb, "finding_id", filter.finding_id);
                push_string_list(&mut qb, "status", filter.statuses);
                push_search_clause(&mut qb, &["title", "description"], filter.search);
                qb.push(" ORDER BY ")
                    .push(sort_column)
                    .push(" ")
                    .push(sort_dir);
                if let Some(limit) = filter.limit {
                    qb.push(" LIMIT ").push_bind(limit as i64);
                }
                if let Some(offset) = filter.offset {
                    qb.push(" OFFSET ").push_bind(offset as i64);
                }
                Ok(qb
                    .build_query_as::<BountySubmissionRow>()
                    .fetch_all(pool)
                    .await?)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut qb = QueryBuilder::<Postgres>::new(sql);
                push_string_equals(&mut qb, "program_id", filter.program_id);
                push_string_equals(&mut qb, "finding_id", filter.finding_id);
                push_string_list(&mut qb, "status", filter.statuses);
                push_search_clause(&mut qb, &["title", "description"], filter.search);
                qb.push(" ORDER BY ")
                    .push(sort_column)
                    .push(" ")
                    .push(sort_dir);
                if let Some(limit) = filter.limit {
                    qb.push(" LIMIT ").push_bind(limit as i64);
                }
                if let Some(offset) = filter.offset {
                    qb.push(" OFFSET ").push_bind(offset as i64);
                }
                let rows = qb.build().fetch_all(pool).await?;
                Ok(rows.into_iter().map(row_to_bounty_submission).collect())
            }
        }
    }

    pub async fn count_bounty_submissions_filtered(
        &self,
        filter: SubmissionQueryFilter<'_>,
    ) -> Result<i64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let sql = "SELECT COUNT(*) AS count FROM bounty_submissions WHERE 1=1";

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut qb = QueryBuilder::<sqlx::Sqlite>::new(sql);
                push_string_equals(&mut qb, "program_id", filter.program_id);
                push_string_equals(&mut qb, "finding_id", filter.finding_id);
                push_string_list(&mut qb, "status", filter.statuses);
                push_search_clause(&mut qb, &["title", "description"], filter.search);
                Ok(qb.build_query_scalar::<i64>().fetch_one(pool).await?)
            }
            DatabasePool::MySQL(pool) => {
                let mut qb = QueryBuilder::<crate::database_service::sqlx_compat::MySql>::new(sql);
                push_string_equals(&mut qb, "program_id", filter.program_id);
                push_string_equals(&mut qb, "finding_id", filter.finding_id);
                push_string_list(&mut qb, "status", filter.statuses);
                push_search_clause(&mut qb, &["title", "description"], filter.search);
                Ok(qb.build_query_scalar::<i64>().fetch_one(pool).await?)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut qb = QueryBuilder::<Postgres>::new(sql);
                push_string_equals(&mut qb, "program_id", filter.program_id);
                push_string_equals(&mut qb, "finding_id", filter.finding_id);
                push_string_list(&mut qb, "status", filter.statuses);
                push_search_clause(&mut qb, &["title", "description"], filter.search);
                Ok(qb.build_query_scalar::<i64>().fetch_one(pool).await?)
            }
        }
    }

    pub async fn batch_update_bounty_change_event_status(
        &self,
        ids: &[String],
        status: &str,
        resolved_at: Option<&str>,
        updated_at: &str,
    ) -> Result<u64> {
        let ids = dedupe_non_empty_ids(ids);
        if ids.is_empty() {
            return Ok(0);
        }

        let runtime = self.get_runtime_pool()?;
        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut tx = pool.begin().await?;
                let mut updated = 0;
                for batch in ids.chunks(BOUNTY_BATCH_MUTATION_SIZE) {
                    let mut qb = QueryBuilder::<sqlx::Sqlite>::new(
                        "UPDATE bounty_change_events SET status = ",
                    );
                    qb.push_bind(status.to_string())
                        .push(", resolved_at = ")
                        .push_bind(resolved_at.map(str::to_string))
                        .push(", updated_at = ")
                        .push_bind(updated_at.to_string())
                        .push(" WHERE id IN (");
                    {
                        let mut separated = qb.separated(", ");
                        for id in batch {
                            separated.push_bind(id.clone());
                        }
                    }
                    qb.push(")");
                    updated += qb.build().execute(&mut *tx).await?.rows_affected();
                }
                tx.commit().await?;
                Ok(updated)
            }
            DatabasePool::MySQL(pool) => {
                let mut tx = pool.begin().await?;
                let mut updated = 0;
                for batch in ids.chunks(BOUNTY_BATCH_MUTATION_SIZE) {
                    let mut qb = QueryBuilder::<crate::database_service::sqlx_compat::MySql>::new(
                        "UPDATE bounty_change_events SET status = ",
                    );
                    qb.push_bind(status.to_string())
                        .push(", resolved_at = ")
                        .push_bind(resolved_at.map(str::to_string))
                        .push(", updated_at = ")
                        .push_bind(updated_at.to_string())
                        .push(" WHERE id IN (");
                    {
                        let mut separated = qb.separated(", ");
                        for id in batch {
                            separated.push_bind(id.clone());
                        }
                    }
                    qb.push(")");
                    updated += qb.build().execute(&mut *tx).await?.rows_affected();
                }
                tx.commit().await?;
                Ok(updated)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut tx = pool.begin().await?;
                let mut updated = 0;
                for batch in ids.chunks(BOUNTY_BATCH_MUTATION_SIZE) {
                    let mut qb =
                        QueryBuilder::<Postgres>::new("UPDATE bounty_change_events SET status = ");
                    qb.push_bind(status.to_string())
                        .push(", resolved_at = ")
                        .push_bind(resolved_at.map(str::to_string))
                        .push(", updated_at = ")
                        .push_bind(updated_at.to_string())
                        .push(" WHERE id IN (");
                    {
                        let mut separated = qb.separated(", ");
                        for id in batch {
                            separated.push_bind(id.clone());
                        }
                    }
                    qb.push(")");
                    updated += qb.build().execute(&mut *tx).await?.rows_affected();
                }
                tx.commit().await?;
                Ok(updated)
            }
        }
    }

    pub async fn batch_delete_bounty_change_events(&self, ids: &[String]) -> Result<u64> {
        let ids = dedupe_non_empty_ids(ids);
        if ids.is_empty() {
            return Ok(0);
        }

        let runtime = self.get_runtime_pool()?;
        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut tx = pool.begin().await?;
                let mut deleted = 0;
                for batch in ids.chunks(BOUNTY_BATCH_MUTATION_SIZE) {
                    let mut qb = QueryBuilder::<sqlx::Sqlite>::new(
                        "DELETE FROM bounty_change_events WHERE id IN (",
                    );
                    {
                        let mut separated = qb.separated(", ");
                        for id in batch {
                            separated.push_bind(id.clone());
                        }
                    }
                    qb.push(")");
                    deleted += qb.build().execute(&mut *tx).await?.rows_affected();
                }
                tx.commit().await?;
                Ok(deleted)
            }
            DatabasePool::MySQL(pool) => {
                let mut tx = pool.begin().await?;
                let mut deleted = 0;
                for batch in ids.chunks(BOUNTY_BATCH_MUTATION_SIZE) {
                    let mut qb = QueryBuilder::<crate::database_service::sqlx_compat::MySql>::new(
                        "DELETE FROM bounty_change_events WHERE id IN (",
                    );
                    {
                        let mut separated = qb.separated(", ");
                        for id in batch {
                            separated.push_bind(id.clone());
                        }
                    }
                    qb.push(")");
                    deleted += qb.build().execute(&mut *tx).await?.rows_affected();
                }
                tx.commit().await?;
                Ok(deleted)
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut tx = pool.begin().await?;
                let mut deleted = 0;
                for batch in ids.chunks(BOUNTY_BATCH_MUTATION_SIZE) {
                    let mut qb = QueryBuilder::<Postgres>::new(
                        "DELETE FROM bounty_change_events WHERE id IN (",
                    );
                    {
                        let mut separated = qb.separated(", ");
                        for id in batch {
                            separated.push_bind(id.clone());
                        }
                    }
                    qb.push(")");
                    deleted += qb.build().execute(&mut *tx).await?.rows_affected();
                }
                tx.commit().await?;
                Ok(deleted)
            }
        }
    }

    pub async fn get_bounty_submission_stats_live(
        &self,
        program_id: Option<&str>,
    ) -> Result<BountySubmissionStats> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let base_sql = r#"
            SELECT
                COUNT(*) AS total_submissions,
                COALESCE(SUM(CASE WHEN status IN ('accepted', 'resolved', 'paid') THEN 1 ELSE 0 END), 0) AS accepted_submissions,
                COALESCE(SUM(reward_amount), 0.0) AS total_rewards,
                COALESCE(SUM(bonus_amount), 0.0) AS total_bonuses
            FROM bounty_submissions
        "#;

        match runtime {
            DatabasePool::SQLite(pool) => {
                let sql = if program_id.is_some() {
                    format!("{base_sql} WHERE program_id = ?")
                } else {
                    base_sql.to_string()
                };
                let mut query = sqlx::query(&sql);
                if let Some(program_id) = program_id {
                    query = query.bind(program_id);
                }
                let row = query.fetch_one(pool).await?;
                Ok(BountySubmissionStats {
                    total_submissions: row.get::<i64, _>("total_submissions") as i32,
                    accepted_submissions: row.get::<i64, _>("accepted_submissions") as i32,
                    total_rewards: row.get::<f64, _>("total_rewards"),
                    total_bonuses: row.get::<f64, _>("total_bonuses"),
                })
            }
            DatabasePool::MySQL(pool) => {
                let sql = if program_id.is_some() {
                    format!("{base_sql} WHERE program_id = ?")
                } else {
                    base_sql.to_string()
                };
                let mut query = sqlx::query(&sql);
                if let Some(program_id) = program_id {
                    query = query.bind(program_id);
                }
                let row = query.fetch_one(pool).await?;
                Ok(BountySubmissionStats {
                    total_submissions: row.get::<i64, _>("total_submissions") as i32,
                    accepted_submissions: row.get::<i64, _>("accepted_submissions") as i32,
                    total_rewards: row.get::<f64, _>("total_rewards"),
                    total_bonuses: row.get::<f64, _>("total_bonuses"),
                })
            }
            DatabasePool::PostgreSQL(pool) => {
                let sql = if program_id.is_some() {
                    format!("{base_sql} WHERE program_id = $1")
                } else {
                    base_sql.to_string()
                };
                let mut query = sqlx::query(&sql);
                if let Some(program_id) = program_id {
                    query = query.bind(program_id);
                }
                let row = query.fetch_one(pool).await?;
                Ok(BountySubmissionStats {
                    total_submissions: row.get::<i64, _>("total_submissions") as i32,
                    accepted_submissions: row.get::<i64, _>("accepted_submissions") as i32,
                    total_rewards: row.get::<f64, _>("total_rewards"),
                    total_bonuses: row.get::<f64, _>("total_bonuses"),
                })
            }
        }
    }

    pub async fn get_bounty_change_event_stats_live(
        &self,
        program_id: Option<&str>,
    ) -> Result<BountyChangeEventStats> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let sql_without_filter = r#"
            SELECT 'summary' AS bucket, 'total' AS label, COUNT(*) AS count_value, COALESCE(AVG(risk_score), 0.0) AS avg_value FROM bounty_change_events
            UNION ALL
            SELECT 'summary' AS bucket, 'pending' AS label, COUNT(*) AS count_value, 0.0 AS avg_value FROM bounty_change_events WHERE status IN ('new', 'analyzing', 'review_required')
            UNION ALL
            SELECT 'type' AS bucket, event_type AS label, COUNT(*) AS count_value, 0.0 AS avg_value FROM bounty_change_events GROUP BY event_type
            UNION ALL
            SELECT 'severity' AS bucket, severity AS label, COUNT(*) AS count_value, 0.0 AS avg_value FROM bounty_change_events GROUP BY severity
            UNION ALL
            SELECT 'status' AS bucket, status AS label, COUNT(*) AS count_value, 0.0 AS avg_value FROM bounty_change_events GROUP BY status
        "#;

        let sql_with_filter = |placeholder: &str| {
            format!(
                r#"
                SELECT 'summary' AS bucket, 'total' AS label, COUNT(*) AS count_value, COALESCE(AVG(risk_score), 0.0) AS avg_value FROM bounty_change_events WHERE program_id = {placeholder}
                UNION ALL
                SELECT 'summary' AS bucket, 'pending' AS label, COUNT(*) AS count_value, 0.0 AS avg_value FROM bounty_change_events WHERE program_id = {placeholder} AND status IN ('new', 'analyzing', 'review_required')
                UNION ALL
                SELECT 'type' AS bucket, event_type AS label, COUNT(*) AS count_value, 0.0 AS avg_value FROM bounty_change_events WHERE program_id = {placeholder} GROUP BY event_type
                UNION ALL
                SELECT 'severity' AS bucket, severity AS label, COUNT(*) AS count_value, 0.0 AS avg_value FROM bounty_change_events WHERE program_id = {placeholder} GROUP BY severity
                UNION ALL
                SELECT 'status' AS bucket, status AS label, COUNT(*) AS count_value, 0.0 AS avg_value FROM bounty_change_events WHERE program_id = {placeholder} GROUP BY status
                "#
            )
        };

        macro_rules! build_change_event_stats {
            ($rows:expr) => {{
                let mut total_events = 0;
                let mut pending_review = 0;
                let mut average_risk_score = 0.0;
                let mut by_type = std::collections::HashMap::new();
                let mut by_severity = std::collections::HashMap::new();
                let mut by_status = std::collections::HashMap::new();

                for row in $rows {
                    let bucket: String = row.get("bucket");
                    let label: String = row.get("label");
                    let count_value = row.get::<i64, _>("count_value") as i32;
                    match bucket.as_str() {
                        "summary" if label == "total" => {
                            total_events = count_value;
                            average_risk_score = row.get::<f64, _>("avg_value");
                        }
                        "summary" if label == "pending" => pending_review = count_value,
                        "type" => {
                            by_type.insert(label, count_value);
                        }
                        "severity" => {
                            by_severity.insert(label, count_value);
                        }
                        "status" => {
                            by_status.insert(label, count_value);
                        }
                        _ => {}
                    }
                }

                BountyChangeEventStats {
                    total_events,
                    by_type,
                    by_severity,
                    by_status,
                    pending_review,
                    average_risk_score,
                }
            }};
        }

        match runtime {
            DatabasePool::SQLite(pool) => {
                let rows = if let Some(program_id) = program_id {
                    let sql = sql_with_filter("?");
                    sqlx::query(&sql)
                        .bind(program_id)
                        .bind(program_id)
                        .bind(program_id)
                        .bind(program_id)
                        .bind(program_id)
                        .fetch_all(pool)
                        .await?
                } else {
                    sqlx::query(sql_without_filter).fetch_all(pool).await?
                };
                Ok(build_change_event_stats!(rows))
            }
            DatabasePool::MySQL(pool) => {
                let rows = if let Some(program_id) = program_id {
                    let sql = sql_with_filter("?");
                    sqlx::query(&sql)
                        .bind(program_id)
                        .bind(program_id)
                        .bind(program_id)
                        .bind(program_id)
                        .bind(program_id)
                        .fetch_all(pool)
                        .await?
                } else {
                    sqlx::query(sql_without_filter).fetch_all(pool).await?
                };
                Ok(build_change_event_stats!(rows))
            }
            DatabasePool::PostgreSQL(pool) => {
                let rows = if let Some(program_id) = program_id {
                    let sql = sql_with_filter("$1");
                    sqlx::query(&sql).bind(program_id).fetch_all(pool).await?
                } else {
                    sqlx::query(sql_without_filter).fetch_all(pool).await?
                };
                Ok(build_change_event_stats!(rows))
            }
        }
    }

    pub async fn list_existing_bounty_asset_canonical_urls(
        &self,
        program_id: &str,
        canonical_urls: &[String],
    ) -> Result<std::collections::HashSet<String>> {
        let canonical_urls = dedupe_non_empty_ids(canonical_urls);
        if canonical_urls.is_empty() {
            return Ok(std::collections::HashSet::new());
        }

        let runtime = self.get_runtime_pool()?;
        let mut existing = std::collections::HashSet::with_capacity(canonical_urls.len());

        match runtime {
            DatabasePool::SQLite(pool) => {
                for batch in canonical_urls.chunks(BOUNTY_BATCH_MUTATION_SIZE) {
                    let mut qb = QueryBuilder::<sqlx::Sqlite>::new(
                        "SELECT canonical_url FROM bounty_assets WHERE program_id = ",
                    );
                    qb.push_bind(program_id.to_string())
                        .push(" AND canonical_url IN (");
                    {
                        let mut separated = qb.separated(", ");
                        for canonical_url in batch {
                            separated.push_bind(canonical_url.clone());
                        }
                    }
                    qb.push(")");
                    for (canonical_url,) in
                        qb.build_query_as::<(String,)>().fetch_all(&pool).await?
                    {
                        existing.insert(canonical_url);
                    }
                }
            }
            DatabasePool::MySQL(pool) => {
                for batch in canonical_urls.chunks(BOUNTY_BATCH_MUTATION_SIZE) {
                    let mut qb = QueryBuilder::<crate::database_service::sqlx_compat::MySql>::new(
                        "SELECT canonical_url FROM bounty_assets WHERE program_id = ",
                    );
                    qb.push_bind(program_id.to_string())
                        .push(" AND canonical_url IN (");
                    {
                        let mut separated = qb.separated(", ");
                        for canonical_url in batch {
                            separated.push_bind(canonical_url.clone());
                        }
                    }
                    qb.push(")");
                    for (canonical_url,) in
                        qb.build_query_as::<(String,)>().fetch_all(&pool).await?
                    {
                        existing.insert(canonical_url);
                    }
                }
            }
            DatabasePool::PostgreSQL(pool) => {
                for batch in canonical_urls.chunks(BOUNTY_BATCH_MUTATION_SIZE) {
                    let mut qb = QueryBuilder::<Postgres>::new(
                        "SELECT canonical_url FROM bounty_assets WHERE program_id = ",
                    );
                    qb.push_bind(program_id.to_string())
                        .push(" AND canonical_url IN (");
                    {
                        let mut separated = qb.separated(", ");
                        for canonical_url in batch {
                            separated.push_bind(canonical_url.clone());
                        }
                    }
                    qb.push(")");
                    for (canonical_url,) in
                        qb.build_query_as::<(String,)>().fetch_all(&pool).await?
                    {
                        existing.insert(canonical_url);
                    }
                }
            }
        }

        Ok(existing)
    }

    pub async fn get_bounty_finding_stats_live(
        &self,
        program_id: Option<&str>,
    ) -> Result<BountyFindingStats> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let sql_without_filter = r#"
            SELECT 'total' AS bucket, '' AS label, COUNT(*) AS count_value FROM bounty_findings
            UNION ALL
            SELECT 'severity' AS bucket, severity AS label, COUNT(*) AS count_value FROM bounty_findings GROUP BY severity
            UNION ALL
            SELECT 'status' AS bucket, status AS label, COUNT(*) AS count_value FROM bounty_findings GROUP BY status
        "#;
        let sql_with_filter = |placeholder: &str| {
            format!(
                r#"
                SELECT 'total' AS bucket, '' AS label, COUNT(*) AS count_value FROM bounty_findings WHERE program_id = {placeholder}
                UNION ALL
                SELECT 'severity' AS bucket, severity AS label, COUNT(*) AS count_value FROM bounty_findings WHERE program_id = {placeholder} GROUP BY severity
                UNION ALL
                SELECT 'status' AS bucket, status AS label, COUNT(*) AS count_value FROM bounty_findings WHERE program_id = {placeholder} GROUP BY status
                "#
            )
        };

        macro_rules! build_finding_stats {
            ($rows:expr) => {{
                let mut total_findings = 0;
                let mut by_severity = std::collections::HashMap::new();
                let mut by_status = std::collections::HashMap::new();

                for row in $rows {
                    let bucket: String = row.get("bucket");
                    let label: String = row.get("label");
                    let count_value = row.get::<i64, _>("count_value") as i32;
                    match bucket.as_str() {
                        "total" => total_findings = count_value,
                        "severity" => {
                            by_severity.insert(label, count_value);
                        }
                        "status" => {
                            by_status.insert(label, count_value);
                        }
                        _ => {}
                    }
                }

                BountyFindingStats {
                    total_findings,
                    by_severity,
                    by_status,
                }
            }};
        }

        match runtime {
            DatabasePool::SQLite(pool) => {
                let rows = if let Some(program_id) = program_id {
                    let sql = sql_with_filter("?");
                    sqlx::query(&sql)
                        .bind(program_id)
                        .bind(program_id)
                        .bind(program_id)
                        .fetch_all(pool)
                        .await?
                } else {
                    sqlx::query(sql_without_filter).fetch_all(pool).await?
                };
                Ok(build_finding_stats!(rows))
            }
            DatabasePool::MySQL(pool) => {
                let rows = if let Some(program_id) = program_id {
                    let sql = sql_with_filter("?");
                    sqlx::query(&sql)
                        .bind(program_id)
                        .bind(program_id)
                        .bind(program_id)
                        .fetch_all(pool)
                        .await?
                } else {
                    sqlx::query(sql_without_filter).fetch_all(pool).await?
                };
                Ok(build_finding_stats!(rows))
            }
            DatabasePool::PostgreSQL(pool) => {
                let rows = if let Some(program_id) = program_id {
                    let sql = sql_with_filter("$1");
                    sqlx::query(&sql).bind(program_id).fetch_all(pool).await?
                } else {
                    sqlx::query(sql_without_filter).fetch_all(pool).await?
                };
                Ok(build_finding_stats!(rows))
            }
        }
    }
}

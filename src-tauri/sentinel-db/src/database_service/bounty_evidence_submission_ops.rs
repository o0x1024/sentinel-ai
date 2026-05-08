//! Bug bounty evidence and submission database operations.

use anyhow::Result;
use chrono::Utc;
use tracing::info;

use super::bounty::{
    optional_timestamp_string_to_datetime, row_to_bounty_evidence, row_to_bounty_submission,
    timestamp_string_to_datetime, BountyEvidenceRow, BountySubmissionRow, BountySubmissionStats,
};
use super::service::DatabaseService;
use crate::database_service::connection_manager::DatabasePool;

impl DatabaseService {
    // ------------------------------------------------------------------------
    // Evidence CRUD
    // ------------------------------------------------------------------------

    /// Create a new bounty evidence
    pub async fn create_bounty_evidence(&self, evidence: &BountyEvidenceRow) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"INSERT INTO bounty_evidence (
                    id, finding_id, evidence_type, title, description, file_path, file_url,
                    content, mime_type, file_size, http_request_json, http_response_json,
                    diff, tags_json, metadata_json, display_order, created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#;
            match runtime {
                DatabasePool::SQLite(pool) => {
                    sqlx::query(query)
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
                        .execute(pool)
                        .await?;
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query(query)
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
                        .execute(pool)
                        .await?;
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            }

            info!("Created bounty evidence: {}", evidence.id);
            return Ok(());
        }

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
        .execute(self.get_pool()?)
        .await?;

        info!("Created bounty evidence: {}", evidence.id);
        Ok(())
    }

    /// Get a bounty evidence by ID
    pub async fn get_bounty_evidence(&self, id: &str) -> Result<Option<BountyEvidenceRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    Ok(sqlx::query_as("SELECT * FROM bounty_evidence WHERE id = ?")
                        .bind(id)
                        .fetch_optional(pool)
                        .await?)
                }
                DatabasePool::MySQL(pool) => {
                    Ok(sqlx::query_as("SELECT * FROM bounty_evidence WHERE id = ?")
                        .bind(id)
                        .fetch_optional(pool)
                        .await?)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let row = sqlx::query("SELECT * FROM bounty_evidence WHERE id = $1")
            .bind(id)
            .fetch_optional(self.get_pool()?)
            .await?;

        Ok(row.map(row_to_bounty_evidence))
    }

    /// Update a bounty evidence
    pub async fn update_bounty_evidence(&self, evidence: &BountyEvidenceRow) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"UPDATE bounty_evidence SET
                    evidence_type = ?, title = ?, description = ?, file_path = ?,
                    file_url = ?, content = ?, mime_type = ?, file_size = ?,
                    http_request_json = ?, http_response_json = ?, diff = ?,
                    tags_json = ?, metadata_json = ?, display_order = ?, updated_at = ?
                WHERE id = ?"#;
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query(query)
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
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query(query)
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
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query(
            r#"UPDATE bounty_evidence SET
                evidence_type = $1, title = $2, description = $3, file_path = $4,
                file_url = $5, content = $6, mime_type = $7, file_size = $8,
                http_request_json = $9, http_response_json = $10, diff = $11,
                tags_json = $12, metadata_json = $13, display_order = $14, updated_at = $15
            WHERE id = $16"#,
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
        .execute(self.get_pool()?)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty evidence
    pub async fn delete_bounty_evidence(&self, id: &str) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_evidence WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_evidence WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query("DELETE FROM bounty_evidence WHERE id = $1")
            .bind(id)
            .execute(self.get_pool()?)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List evidence for a finding
    pub async fn list_bounty_evidence(&self, finding_id: &str) -> Result<Vec<BountyEvidenceRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"
                SELECT
                    id, finding_id, evidence_type, title, description, file_path, file_url, content,
                    mime_type, file_size, http_request_json, http_response_json, diff, tags_json,
                    metadata_json, display_order, CAST(created_at AS TEXT) AS created_at,
                    CAST(updated_at AS TEXT) AS updated_at
                FROM bounty_evidence
                WHERE finding_id = ?
                ORDER BY display_order, created_at
            "#;
            let rows: Vec<BountyEvidenceRow> = match runtime {
                DatabasePool::SQLite(pool) => {
                    sqlx::query_as(query)
                        .bind(finding_id)
                        .fetch_all(pool)
                        .await?
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query_as(query)
                        .bind(finding_id)
                        .fetch_all(pool)
                        .await?
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
            return Ok(rows);
        }

        let rows = sqlx::query("SELECT * FROM bounty_evidence WHERE finding_id = $1 ORDER BY display_order, created_at")
        .bind(finding_id)
        .fetch_all(self.get_pool()?)
        .await?;

        Ok(rows.into_iter().map(row_to_bounty_evidence).collect())
    }

    // ------------------------------------------------------------------------
    // Submission CRUD
    // ------------------------------------------------------------------------

    /// Create a new bounty submission
    pub async fn create_bounty_submission(&self, submission: &BountySubmissionRow) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"INSERT INTO bounty_submissions (
                    id, program_id, finding_id, platform_submission_id, title, status,
                    priority, vulnerability_type, severity, cvss_score, cwe_id, description,
                    reproduction_steps_json, impact, remediation, evidence_ids_json, platform_url,
                    reward_amount, reward_currency, bonus_amount, response_time_hours,
                    resolution_time_hours, requires_retest, retest_at, last_retest_at,
                    communications_json, timeline_json, tags_json, metadata_json, created_at, submitted_at,
                    updated_at, closed_at, created_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#;
            match runtime {
                DatabasePool::SQLite(pool) => {
                    sqlx::query(query)
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
                        .execute(pool)
                        .await?;
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query(query)
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
                        .execute(pool)
                        .await?;
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            }
            info!("Created bounty submission: {}", submission.id);
            return Ok(());
        }

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
        .execute(self.get_pool()?)
        .await?;

        info!("Created bounty submission: {}", submission.id);
        Ok(())
    }

    /// Get a bounty submission by ID
    pub async fn get_bounty_submission(&self, id: &str) -> Result<Option<BountySubmissionRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_submissions WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_submissions WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?),
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let row = sqlx::query("SELECT * FROM bounty_submissions WHERE id = $1")
            .bind(id)
            .fetch_optional(self.get_pool()?)
            .await?;

        Ok(row.map(row_to_bounty_submission))
    }

    /// Update a bounty submission
    pub async fn update_bounty_submission(&self, submission: &BountySubmissionRow) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"UPDATE bounty_submissions SET
                    platform_submission_id = ?, title = ?, status = ?, priority = ?,
                    vulnerability_type = ?, severity = ?, cvss_score = ?, cwe_id = ?,
                    description = ?, reproduction_steps_json = ?, impact = ?, remediation = ?,
                    evidence_ids_json = ?, platform_url = ?, reward_amount = ?, reward_currency = ?,
                    bonus_amount = ?, response_time_hours = ?, resolution_time_hours = ?,
                    requires_retest = ?, retest_at = ?, last_retest_at = ?, communications_json = ?,
                    timeline_json = ?, tags_json = ?, metadata_json = ?, submitted_at = ?, updated_at = ?, closed_at = ?
                WHERE id = ?"#;
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query(query)
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
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query(query)
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
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

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
        .execute(self.get_pool()?)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty submission
    pub async fn delete_bounty_submission(&self, id: &str) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_submissions WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_submissions WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query("DELETE FROM bounty_submissions WHERE id = $1")
            .bind(id)
            .execute(self.get_pool()?)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Batch delete bounty submissions
    pub async fn batch_delete_bounty_submissions(&self, ids: &[String]) -> Result<u64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let pg_placeholders = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", i + 1))
            .collect::<Vec<_>>()
            .join(",");
        match runtime {
            DatabasePool::SQLite(pool) => {
                let query_str = format!(
                    "DELETE FROM bounty_submissions WHERE id IN ({})",
                    placeholders
                );
                let mut query = sqlx::query(&query_str);
                for id in ids {
                    query = query.bind(id);
                }
                let result = query.execute(pool).await?;
                Ok(result.rows_affected())
            }
            DatabasePool::MySQL(pool) => {
                let query_str = format!(
                    "DELETE FROM bounty_submissions WHERE id IN ({})",
                    placeholders
                );
                let mut query = sqlx::query(&query_str);
                for id in ids {
                    query = query.bind(id);
                }
                let result = query.execute(pool).await?;
                Ok(result.rows_affected())
            }
            DatabasePool::PostgreSQL(_) => {
                let query_str = format!(
                    "DELETE FROM bounty_submissions WHERE id IN ({})",
                    pg_placeholders
                );
                let mut query = sqlx::query(&query_str);
                for id in ids {
                    query = query.bind(id);
                }
                let result = query.execute(self.get_pool()?).await?;
                Ok(result.rows_affected())
            }
        }
    }

    /// Batch update bounty submission status
    pub async fn batch_update_bounty_submission_status(
        &self,
        ids: &[String],
        status: &str,
    ) -> Result<u64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        if ids.is_empty() {
            return Ok(0);
        }
        let now = Utc::now().to_rfc3339();
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let pg_placeholders = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", i + 3))
            .collect::<Vec<_>>()
            .join(",");

        let (close_clause, close_pg) =
            if ["accepted", "rejected", "duplicate", "closed"].contains(&status) {
                (
                    format!(", closed_at = '{}'", now),
                    format!(", closed_at = '{}'", now),
                )
            } else {
                (String::new(), String::new())
            };

        match runtime {
            DatabasePool::SQLite(pool) => {
                let query_str = format!(
                    "UPDATE bounty_submissions SET status = ?, updated_at = ? {} WHERE id IN ({})",
                    close_clause, placeholders
                );
                let mut query = sqlx::query(&query_str).bind(status).bind(&now);
                for id in ids {
                    query = query.bind(id);
                }
                let result = query.execute(pool).await?;
                Ok(result.rows_affected())
            }
            DatabasePool::MySQL(pool) => {
                let query_str = format!(
                    "UPDATE bounty_submissions SET status = ?, updated_at = ? {} WHERE id IN ({})",
                    close_clause, placeholders
                );
                let mut query = sqlx::query(&query_str).bind(status).bind(&now);
                for id in ids {
                    query = query.bind(id);
                }
                let result = query.execute(pool).await?;
                Ok(result.rows_affected())
            }
            DatabasePool::PostgreSQL(_) => {
                let query_str = format!("UPDATE bounty_submissions SET status = $1, updated_at = $2 {} WHERE id IN ({})", close_pg, pg_placeholders);
                let mut query = sqlx::query(&query_str).bind(status).bind(&now);
                for id in ids {
                    query = query.bind(id);
                }
                let result = query.execute(self.get_pool()?).await?;
                Ok(result.rows_affected())
            }
        }
    }

    /// List bounty submissions with optional filtering
    pub async fn list_bounty_submissions(
        &self,
        program_id: Option<&str>,
        finding_id: Option<&str>,
        statuses: Option<&[String]>,
        search: Option<&str>,
        sort_by: Option<&str>,
        sort_dir: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<BountySubmissionRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"
                SELECT
                    id, program_id, finding_id, platform_submission_id, title, status, priority,
                    vulnerability_type, severity, cvss_score, cwe_id, description, reproduction_steps_json,
                    impact, remediation, evidence_ids_json, platform_url, reward_amount, reward_currency,
                    bonus_amount, response_time_hours, resolution_time_hours, requires_retest,
                    CAST(retest_at AS TEXT) AS retest_at, CAST(last_retest_at AS TEXT) AS last_retest_at,
                    communications_json, timeline_json, tags_json, metadata_json,
                    CAST(created_at AS TEXT) AS created_at, CAST(submitted_at AS TEXT) AS submitted_at,
                    CAST(updated_at AS TEXT) AS updated_at, CAST(closed_at AS TEXT) AS closed_at, created_by
                FROM bounty_submissions
            "#;
            let mut submissions: Vec<BountySubmissionRow> = match runtime {
                DatabasePool::SQLite(pool) => sqlx::query_as(query).fetch_all(pool).await?,
                DatabasePool::MySQL(pool) => sqlx::query_as(query).fetch_all(pool).await?,
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };

            if let Some(pid) = program_id {
                submissions.retain(|s| s.program_id == pid);
            }
            if let Some(fid) = finding_id {
                submissions.retain(|s| s.finding_id == fid);
            }
            if let Some(stats) = statuses {
                if !stats.is_empty() {
                    submissions.retain(|s| stats.contains(&s.status));
                }
            }
            if let Some(keyword) = search {
                if !keyword.is_empty() {
                    let needle = keyword.to_lowercase();
                    submissions.retain(|s| {
                        s.title.to_lowercase().contains(&needle)
                            || s.description.to_lowercase().contains(&needle)
                    });
                }
            }

            let order_by = sort_by.unwrap_or("created_at");
            let direction = sort_dir.unwrap_or("desc").to_lowercase();
            submissions.sort_by(|a, b| {
                let ord = match order_by {
                    "created_at" => a.created_at.cmp(&b.created_at),
                    "updated_at" => a.updated_at.cmp(&b.updated_at),
                    "severity" => a.severity.cmp(&b.severity),
                    "status" => a.status.cmp(&b.status),
                    "priority" => a.priority.cmp(&b.priority),
                    "reward_amount" => a
                        .reward_amount
                        .partial_cmp(&b.reward_amount)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    "submitted_at" => a.submitted_at.cmp(&b.submitted_at),
                    _ => a.created_at.cmp(&b.created_at),
                };
                if direction == "asc" {
                    ord
                } else {
                    ord.reverse()
                }
            });

            if let Some(off) = offset {
                submissions = submissions.into_iter().skip(off as usize).collect();
            }
            if let Some(lim) = limit {
                submissions.truncate(lim as usize);
            }
            return Ok(submissions);
        }

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

        if let Some(search) = search {
            if !search.is_empty() {
                let p1 = params.len() + 1;
                let p2 = params.len() + 2;
                query.push_str(&format!(
                    " AND (title ILIKE ${} OR description ILIKE ${})",
                    p1, p2
                ));
                let search_pattern = format!("%{}%", search);
                params.push(search_pattern.clone());
                params.push(search_pattern);
            }
        }

        let order_by = match sort_by.unwrap_or("created_at") {
            "created_at" => "created_at",
            "updated_at" => "updated_at",
            "severity" => "severity",
            "status" => "status",
            "priority" => "priority",
            "reward_amount" => "reward_amount",
            "submitted_at" => "submitted_at",
            _ => "created_at",
        };
        let direction = match sort_dir.unwrap_or("desc").to_lowercase().as_str() {
            "asc" => "ASC",
            _ => "DESC",
        };
        query.push_str(&format!(" ORDER BY {} {}", order_by, direction));

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

        let rows = sqlx_query.fetch_all(self.get_pool()?).await?;
        Ok(rows.into_iter().map(row_to_bounty_submission).collect())
    }

    /// Get bounty submission statistics
    pub async fn get_bounty_submission_stats(
        &self,
        program_id: Option<&str>,
    ) -> Result<BountySubmissionStats> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let total: (i64,) = match (runtime, program_id) {
            (DatabasePool::PostgreSQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_submissions WHERE program_id = $1")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::PostgreSQL(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_submissions")
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_submissions WHERE program_id = ?")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_submissions")
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_submissions WHERE program_id = ?")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_submissions")
                    .fetch_one(pool)
                    .await?
            }
        };

        let accepted: (i64,) = match (runtime, program_id) {
            (DatabasePool::PostgreSQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_submissions WHERE program_id = $1 AND status IN ('accepted', 'resolved', 'paid')")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::PostgreSQL(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_submissions WHERE status IN ('accepted', 'resolved', 'paid')")
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_submissions WHERE program_id = ? AND status IN ('accepted', 'resolved', 'paid')")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_submissions WHERE status IN ('accepted', 'resolved', 'paid')")
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_submissions WHERE program_id = ? AND status IN ('accepted', 'resolved', 'paid')")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_submissions WHERE status IN ('accepted', 'resolved', 'paid')")
                    .fetch_one(pool)
                    .await?
            }
        };

        let earnings: (f64, f64) = match (runtime, program_id) {
            (DatabasePool::PostgreSQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT COALESCE(SUM(reward_amount), 0.0), COALESCE(SUM(bonus_amount), 0.0) FROM bounty_submissions WHERE program_id = $1")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::PostgreSQL(pool), None) => {
                sqlx::query_as("SELECT COALESCE(SUM(reward_amount), 0.0), COALESCE(SUM(bonus_amount), 0.0) FROM bounty_submissions")
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), Some(pid)) => {
                sqlx::query_as("SELECT COALESCE(SUM(reward_amount), 0.0), COALESCE(SUM(bonus_amount), 0.0) FROM bounty_submissions WHERE program_id = ?")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), None) => {
                sqlx::query_as("SELECT COALESCE(SUM(reward_amount), 0.0), COALESCE(SUM(bonus_amount), 0.0) FROM bounty_submissions")
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT COALESCE(SUM(reward_amount), 0.0), COALESCE(SUM(bonus_amount), 0.0) FROM bounty_submissions WHERE program_id = ?")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), None) => {
                sqlx::query_as("SELECT COALESCE(SUM(reward_amount), 0.0), COALESCE(SUM(bonus_amount), 0.0) FROM bounty_submissions")
                    .fetch_one(pool)
                    .await?
            }
        };

        Ok(BountySubmissionStats {
            total_submissions: total.0 as i32,
            accepted_submissions: accepted.0 as i32,
            total_rewards: earnings.0,
            total_bonuses: earnings.1,
        })
    }
}

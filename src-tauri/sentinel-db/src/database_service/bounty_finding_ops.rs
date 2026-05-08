//! Bug bounty finding database operations.

use anyhow::Result;
use chrono::Utc;
use tracing::info;

use super::bounty::{
    optional_timestamp_string_to_datetime, row_to_bounty_finding, timestamp_string_to_datetime,
    BountyFindingRow, BountyFindingStats,
};
use super::service::DatabaseService;
use crate::database_service::connection_manager::DatabasePool;

impl DatabaseService {
    // ------------------------------------------------------------------------
    // Finding CRUD
    // ------------------------------------------------------------------------

    /// Create a new bounty finding
    pub async fn create_bounty_finding(&self, finding: &BountyFindingRow) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"INSERT INTO bounty_findings (
                    id, program_id, scope_id, asset_id, title, description, finding_type,
                    severity, status, confidence, cvss_score, cwe_id, affected_url,
                    affected_parameter, reproduction_steps_json, impact, remediation,
                    evidence_ids_json, tags_json, metadata_json, fingerprint, duplicate_of,
                    first_seen_at, last_seen_at, verified_at, created_at, updated_at, created_by
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#;
            match runtime {
                DatabasePool::SQLite(pool) => {
                    sqlx::query(query)
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
                        .execute(pool)
                        .await?;
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query(query)
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
                        .execute(pool)
                        .await?;
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            }

            info!("Created bounty finding: {}", finding.id);
            return Ok(());
        }

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
        .execute(self.get_pool()?)
        .await?;

        info!("Created bounty finding: {}", finding.id);
        Ok(())
    }

    /// Get a bounty finding by ID
    pub async fn get_bounty_finding(&self, id: &str) -> Result<Option<BountyFindingRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    Ok(sqlx::query_as("SELECT * FROM bounty_findings WHERE id = ?")
                        .bind(id)
                        .fetch_optional(pool)
                        .await?)
                }
                DatabasePool::MySQL(pool) => {
                    Ok(sqlx::query_as("SELECT * FROM bounty_findings WHERE id = ?")
                        .bind(id)
                        .fetch_optional(pool)
                        .await?)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let row = sqlx::query("SELECT * FROM bounty_findings WHERE id = $1")
            .bind(id)
            .fetch_optional(self.get_pool()?)
            .await?;

        Ok(row.map(row_to_bounty_finding))
    }

    /// Get a bounty finding by fingerprint (for deduplication)
    pub async fn get_bounty_finding_by_fingerprint(
        &self,
        fingerprint: &str,
    ) -> Result<Option<BountyFindingRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_findings WHERE fingerprint = ?",
                )
                .bind(fingerprint)
                .fetch_optional(pool)
                .await?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_findings WHERE fingerprint = ?",
                )
                .bind(fingerprint)
                .fetch_optional(pool)
                .await?),
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let row = sqlx::query("SELECT * FROM bounty_findings WHERE fingerprint = $1")
            .bind(fingerprint)
            .fetch_optional(self.get_pool()?)
            .await?;

        Ok(row.map(row_to_bounty_finding))
    }

    /// Get a bounty finding by fingerprint excluding a specific ID
    pub async fn get_bounty_finding_by_fingerprint_excluding(
        &self,
        fingerprint: &str,
        exclude_id: &str,
    ) -> Result<Option<BountyFindingRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_findings WHERE fingerprint = ? AND id <> ?",
                )
                .bind(fingerprint)
                .bind(exclude_id)
                .fetch_optional(pool)
                .await?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_findings WHERE fingerprint = ? AND id <> ?",
                )
                .bind(fingerprint)
                .bind(exclude_id)
                .fetch_optional(pool)
                .await?),
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let row = sqlx::query("SELECT * FROM bounty_findings WHERE fingerprint = $1 AND id <> $2")
            .bind(fingerprint)
            .bind(exclude_id)
            .fetch_optional(self.get_pool()?)
            .await?;

        Ok(row.map(row_to_bounty_finding))
    }

    /// Update a bounty finding
    pub async fn update_bounty_finding(&self, finding: &BountyFindingRow) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"UPDATE bounty_findings SET
                    scope_id = ?, asset_id = ?, title = ?, description = ?, finding_type = ?,
                    severity = ?, status = ?, confidence = ?, cvss_score = ?, cwe_id = ?,
                    affected_url = ?, affected_parameter = ?, reproduction_steps_json = ?,
                    impact = ?, remediation = ?, evidence_ids_json = ?, tags_json = ?,
                    metadata_json = ?, duplicate_of = ?, last_seen_at = ?, verified_at = ?,
                    updated_at = ?
                WHERE id = ?"#;
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query(query)
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
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query(query)
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
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query(
            r#"UPDATE bounty_findings SET
                scope_id = $1, asset_id = $2, title = $3, description = $4, finding_type = $5,
                severity = $6, status = $7, confidence = $8, cvss_score = $9, cwe_id = $10,
                affected_url = $11, affected_parameter = $12, reproduction_steps_json = $13,
                impact = $14, remediation = $15, evidence_ids_json = $16, tags_json = $17,
                metadata_json = $18, duplicate_of = $19, last_seen_at = $20, verified_at = $21,
                updated_at = $22
            WHERE id = $23"#,
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
        .execute(self.get_pool()?)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a bounty finding
    pub async fn delete_bounty_finding(&self, id: &str) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_findings WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_findings WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query("DELETE FROM bounty_findings WHERE id = $1")
            .bind(id)
            .execute(self.get_pool()?)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Batch delete bounty findings
    pub async fn batch_delete_bounty_findings(&self, ids: &[String]) -> Result<u64> {
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
                let query_str =
                    format!("DELETE FROM bounty_findings WHERE id IN ({})", placeholders);
                let mut query = sqlx::query(&query_str);
                for id in ids {
                    query = query.bind(id);
                }
                let result = query.execute(pool).await?;
                Ok(result.rows_affected())
            }
            DatabasePool::MySQL(pool) => {
                let query_str =
                    format!("DELETE FROM bounty_findings WHERE id IN ({})", placeholders);
                let mut query = sqlx::query(&query_str);
                for id in ids {
                    query = query.bind(id);
                }
                let result = query.execute(pool).await?;
                Ok(result.rows_affected())
            }
            DatabasePool::PostgreSQL(_) => {
                let query_str = format!(
                    "DELETE FROM bounty_findings WHERE id IN ({})",
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

    /// Delete all bounty findings
    pub async fn delete_all_bounty_findings(&self) -> Result<u64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                let result = sqlx::query("DELETE FROM bounty_findings")
                    .execute(pool)
                    .await?;
                Ok(result.rows_affected())
            }
            DatabasePool::MySQL(pool) => {
                let result = sqlx::query("DELETE FROM bounty_findings")
                    .execute(pool)
                    .await?;
                Ok(result.rows_affected())
            }
            DatabasePool::PostgreSQL(_) => {
                let result = sqlx::query("DELETE FROM bounty_findings")
                    .execute(self.get_pool()?)
                    .await?;
                Ok(result.rows_affected())
            }
        }
    }

    /// Batch update bounty finding status
    pub async fn batch_update_bounty_finding_status(
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
        match runtime {
            DatabasePool::SQLite(pool) => {
                let query_str = format!(
                    "UPDATE bounty_findings SET status = ?, updated_at = ? WHERE id IN ({})",
                    placeholders
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
                    "UPDATE bounty_findings SET status = ?, updated_at = ? WHERE id IN ({})",
                    placeholders
                );
                let mut query = sqlx::query(&query_str).bind(status).bind(&now);
                for id in ids {
                    query = query.bind(id);
                }
                let result = query.execute(pool).await?;
                Ok(result.rows_affected())
            }
            DatabasePool::PostgreSQL(_) => {
                let query_str = format!(
                    "UPDATE bounty_findings SET status = $1, updated_at = $2 WHERE id IN ({})",
                    pg_placeholders
                );
                let mut query = sqlx::query(&query_str).bind(status).bind(&now);
                for id in ids {
                    query = query.bind(id);
                }
                let result = query.execute(self.get_pool()?).await?;
                Ok(result.rows_affected())
            }
        }
    }

    /// List bounty findings with optional filtering
    pub async fn list_bounty_findings(
        &self,
        program_id: Option<&str>,
        scope_id: Option<&str>,
        severities: Option<&[String]>,
        statuses: Option<&[String]>,
        search: Option<&str>,
        sort_by: Option<&str>,
        sort_dir: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<BountyFindingRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"
                SELECT
                    id, program_id, scope_id, asset_id, title, description, finding_type, severity, status,
                    confidence, cvss_score, cwe_id, affected_url, affected_parameter, reproduction_steps_json,
                    impact, remediation, evidence_ids_json, tags_json, metadata_json, fingerprint, duplicate_of,
                    CAST(first_seen_at AS TEXT) AS first_seen_at, CAST(last_seen_at AS TEXT) AS last_seen_at,
                    CAST(verified_at AS TEXT) AS verified_at, CAST(created_at AS TEXT) AS created_at,
                    CAST(updated_at AS TEXT) AS updated_at, created_by
                FROM bounty_findings
            "#;
            let mut findings: Vec<BountyFindingRow> = match runtime {
                DatabasePool::SQLite(pool) => sqlx::query_as(query).fetch_all(pool).await?,
                DatabasePool::MySQL(pool) => sqlx::query_as(query).fetch_all(pool).await?,
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };

            if let Some(pid) = program_id {
                findings.retain(|f| f.program_id == pid);
            }
            if let Some(sid) = scope_id {
                findings.retain(|f| f.scope_id.as_deref() == Some(sid));
            }
            if let Some(sevs) = severities {
                if !sevs.is_empty() {
                    findings.retain(|f| sevs.contains(&f.severity));
                }
            }
            if let Some(stats) = statuses {
                if !stats.is_empty() {
                    findings.retain(|f| stats.contains(&f.status));
                }
            }
            if let Some(keyword) = search {
                if !keyword.is_empty() {
                    let needle = keyword.to_lowercase();
                    findings.retain(|f| {
                        f.title.to_lowercase().contains(&needle)
                            || f.description.to_lowercase().contains(&needle)
                    });
                }
            }

            let order_by = sort_by.unwrap_or("created_at");
            let direction = sort_dir.unwrap_or("desc").to_lowercase();
            findings.sort_by(|a, b| {
                let ord = match order_by {
                    "created_at" => a.created_at.cmp(&b.created_at),
                    "updated_at" => a.updated_at.cmp(&b.updated_at),
                    "severity" => a.severity.cmp(&b.severity),
                    "status" => a.status.cmp(&b.status),
                    "title" => a.title.cmp(&b.title),
                    "cvss_score" => a
                        .cvss_score
                        .partial_cmp(&b.cvss_score)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    "last_seen_at" => a.last_seen_at.cmp(&b.last_seen_at),
                    _ => a.created_at.cmp(&b.created_at),
                };
                if direction == "asc" {
                    ord
                } else {
                    ord.reverse()
                }
            });

            if let Some(off) = offset {
                findings = findings.into_iter().skip(off as usize).collect();
            }
            if let Some(lim) = limit {
                findings.truncate(lim as usize);
            }
            return Ok(findings);
        }

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
                query.push_str(&format!(
                    " AND (title LIKE ${} OR description LIKE ${})",
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
            "title" => "title",
            "cvss_score" => "cvss_score",
            "last_seen_at" => "last_seen_at",
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
        Ok(rows.into_iter().map(row_to_bounty_finding).collect())
    }

    /// List recent findings for similarity checks
    pub async fn list_bounty_findings_for_similarity(
        &self,
        program_id: &str,
        finding_type: &str,
        limit: i64,
    ) -> Result<Vec<BountyFindingRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"
                SELECT
                    id, program_id, scope_id, asset_id, title, description, finding_type, severity, status,
                    confidence, cvss_score, cwe_id, affected_url, affected_parameter, reproduction_steps_json,
                    impact, remediation, evidence_ids_json, tags_json, metadata_json, fingerprint, duplicate_of,
                    CAST(first_seen_at AS TEXT) AS first_seen_at, CAST(last_seen_at AS TEXT) AS last_seen_at,
                    CAST(verified_at AS TEXT) AS verified_at, CAST(created_at AS TEXT) AS created_at,
                    CAST(updated_at AS TEXT) AS updated_at, created_by
                FROM bounty_findings
                WHERE program_id = ? AND finding_type = ?
                ORDER BY created_at DESC
            "#;
            let mut rows: Vec<BountyFindingRow> = match runtime {
                DatabasePool::SQLite(pool) => {
                    sqlx::query_as(query)
                        .bind(program_id)
                        .bind(finding_type)
                        .fetch_all(pool)
                        .await?
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query_as(query)
                        .bind(program_id)
                        .bind(finding_type)
                        .fetch_all(pool)
                        .await?
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
            rows.truncate(limit.max(0) as usize);
            return Ok(rows);
        }

        let rows = sqlx::query(
            "SELECT * FROM bounty_findings WHERE program_id = $1 AND finding_type = $2 ORDER BY created_at DESC LIMIT $3",
        )
        .bind(program_id)
        .bind(finding_type)
        .bind(limit)
        .fetch_all(self.get_pool()?)
        .await?;

        Ok(rows.into_iter().map(row_to_bounty_finding).collect())
    }

    /// Get bounty finding statistics
    pub async fn get_bounty_finding_stats(
        &self,
        program_id: Option<&str>,
    ) -> Result<BountyFindingStats> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let total: (i64,) = match (runtime, program_id) {
            (DatabasePool::PostgreSQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_findings WHERE program_id = $1")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::PostgreSQL(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_findings")
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_findings WHERE program_id = ?")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_findings")
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_findings WHERE program_id = ?")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_findings")
                    .fetch_one(pool)
                    .await?
            }
        };

        let severity_rows: Vec<(String, i64)> = match (runtime, program_id) {
            (DatabasePool::PostgreSQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT severity, COUNT(*) FROM bounty_findings WHERE program_id = $1 GROUP BY severity")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::PostgreSQL(pool), None) => {
                sqlx::query_as("SELECT severity, COUNT(*) FROM bounty_findings GROUP BY severity")
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), Some(pid)) => {
                sqlx::query_as("SELECT severity, COUNT(*) FROM bounty_findings WHERE program_id = ? GROUP BY severity")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), None) => {
                sqlx::query_as("SELECT severity, COUNT(*) FROM bounty_findings GROUP BY severity")
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT severity, COUNT(*) FROM bounty_findings WHERE program_id = ? GROUP BY severity")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), None) => {
                sqlx::query_as("SELECT severity, COUNT(*) FROM bounty_findings GROUP BY severity")
                    .fetch_all(pool)
                    .await?
            }
        };
        let by_severity: std::collections::HashMap<String, i32> = severity_rows
            .into_iter()
            .map(|(k, v)| (k, v as i32))
            .collect();

        let status_rows: Vec<(String, i64)> = match (runtime, program_id) {
            (DatabasePool::PostgreSQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT status, COUNT(*) FROM bounty_findings WHERE program_id = $1 GROUP BY status")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::PostgreSQL(pool), None) => {
                sqlx::query_as("SELECT status, COUNT(*) FROM bounty_findings GROUP BY status")
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), Some(pid)) => {
                sqlx::query_as("SELECT status, COUNT(*) FROM bounty_findings WHERE program_id = ? GROUP BY status")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), None) => {
                sqlx::query_as("SELECT status, COUNT(*) FROM bounty_findings GROUP BY status")
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT status, COUNT(*) FROM bounty_findings WHERE program_id = ? GROUP BY status")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), None) => {
                sqlx::query_as("SELECT status, COUNT(*) FROM bounty_findings GROUP BY status")
                    .fetch_all(pool)
                    .await?
            }
        };
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
}

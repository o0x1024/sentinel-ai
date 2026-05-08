//! Bug bounty change-event database operations.

use anyhow::Result;
use tracing::info;

use super::bounty::{
    optional_timestamp_string_to_datetime, row_to_bounty_change_event,
    timestamp_string_to_datetime, BountyChangeEventRow, BountyChangeEventStats,
};
use super::service::DatabaseService;
use crate::database_service::connection_manager::DatabasePool;

impl DatabaseService {
    // ------------------------------------------------------------------------
    // Change Event CRUD
    // ------------------------------------------------------------------------

    /// Create a new change event
    pub async fn create_bounty_change_event(&self, event: &BountyChangeEventRow) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"INSERT INTO bounty_change_events (
                    id, program_id, asset_id, event_type, severity, status, title, description,
                    old_value, new_value, diff, affected_scope, detection_method,
                    generated_findings_json, tags_json, metadata_json, risk_score,
                    auto_trigger_enabled, created_at, updated_at, resolved_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#;
            match runtime {
                DatabasePool::SQLite(pool) => {
                    sqlx::query(query)
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
                        .bind(&event.generated_findings_json)
                        .bind(&event.tags_json)
                        .bind(&event.metadata_json)
                        .bind(event.risk_score)
                        .bind(event.auto_trigger_enabled)
                        .bind(&event.created_at)
                        .bind(&event.updated_at)
                        .bind(&event.resolved_at)
                        .execute(pool)
                        .await?;
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query(query)
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
                        .bind(&event.generated_findings_json)
                        .bind(&event.tags_json)
                        .bind(&event.metadata_json)
                        .bind(event.risk_score)
                        .bind(event.auto_trigger_enabled)
                        .bind(&event.created_at)
                        .bind(&event.updated_at)
                        .bind(&event.resolved_at)
                        .execute(pool)
                        .await?;
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            }
            info!("Created bounty change event: {}", event.id);
            return Ok(());
        }

        sqlx::query(
            r#"INSERT INTO bounty_change_events (
                id, program_id, asset_id, event_type, severity, status, title, description,
                old_value, new_value, diff, affected_scope, detection_method,
                generated_findings_json, tags_json, metadata_json, risk_score,
                auto_trigger_enabled, created_at, updated_at, resolved_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)"#
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
        .bind(&event.generated_findings_json)
        .bind(&event.tags_json)
        .bind(&event.metadata_json)
        .bind(event.risk_score)
        .bind(event.auto_trigger_enabled)
        .bind(timestamp_string_to_datetime(&event.created_at))
        .bind(timestamp_string_to_datetime(&event.updated_at))
        .bind(optional_timestamp_string_to_datetime(&event.resolved_at))
        .execute(self.get_pool()?)
        .await?;

        info!("Created bounty change event: {}", event.id);
        Ok(())
    }

    /// Get a change event by ID
    pub async fn get_bounty_change_event(&self, id: &str) -> Result<Option<BountyChangeEventRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_change_events WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_change_events WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?),
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let row = sqlx::query("SELECT * FROM bounty_change_events WHERE id = $1")
            .bind(id)
            .fetch_optional(self.get_pool()?)
            .await?;

        Ok(row.map(row_to_bounty_change_event))
    }

    /// Update a change event
    pub async fn update_bounty_change_event(&self, event: &BountyChangeEventRow) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"UPDATE bounty_change_events SET
                    program_id = ?, asset_id = ?, event_type = ?, severity = ?, status = ?,
                    title = ?, description = ?, old_value = ?, new_value = ?, diff = ?,
                    affected_scope = ?, detection_method = ?, generated_findings_json = ?,
                    tags_json = ?, metadata_json = ?, risk_score = ?, auto_trigger_enabled = ?,
                    updated_at = ?, resolved_at = ?
                WHERE id = ?"#;
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query(query)
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
                        .bind(&event.generated_findings_json)
                        .bind(&event.tags_json)
                        .bind(&event.metadata_json)
                        .bind(event.risk_score)
                        .bind(event.auto_trigger_enabled)
                        .bind(&event.updated_at)
                        .bind(&event.resolved_at)
                        .bind(&event.id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query(query)
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
                        .bind(&event.generated_findings_json)
                        .bind(&event.tags_json)
                        .bind(&event.metadata_json)
                        .bind(event.risk_score)
                        .bind(event.auto_trigger_enabled)
                        .bind(&event.updated_at)
                        .bind(&event.resolved_at)
                        .bind(&event.id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query(
            r#"UPDATE bounty_change_events SET
                program_id = $1, asset_id = $2, event_type = $3, severity = $4, status = $5,
                title = $6, description = $7, old_value = $8, new_value = $9, diff = $10,
                affected_scope = $11, detection_method = $12, generated_findings_json = $13,
                tags_json = $14, metadata_json = $15, risk_score = $16,
                auto_trigger_enabled = $17, updated_at = $18, resolved_at = $19
            WHERE id = $20"#,
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
        .bind(&event.generated_findings_json)
        .bind(&event.tags_json)
        .bind(&event.metadata_json)
        .bind(event.risk_score)
        .bind(event.auto_trigger_enabled)
        .bind(timestamp_string_to_datetime(&event.updated_at))
        .bind(optional_timestamp_string_to_datetime(&event.resolved_at))
        .bind(&event.id)
        .execute(self.get_pool()?)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a change event
    pub async fn delete_bounty_change_event(&self, id: &str) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_change_events WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_change_events WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query("DELETE FROM bounty_change_events WHERE id = $1")
            .bind(id)
            .execute(self.get_pool()?)
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
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"
                SELECT
                    id, program_id, asset_id, event_type, severity, status, title, description, old_value,
                    new_value, diff, affected_scope, detection_method, generated_findings_json,
                    tags_json, metadata_json, risk_score, auto_trigger_enabled,
                    CAST(created_at AS TEXT) AS created_at, CAST(updated_at AS TEXT) AS updated_at,
                    CAST(resolved_at AS TEXT) AS resolved_at
                FROM bounty_change_events
                ORDER BY created_at DESC
            "#;
            let mut events: Vec<BountyChangeEventRow> = match runtime {
                DatabasePool::SQLite(pool) => sqlx::query_as(query).fetch_all(pool).await?,
                DatabasePool::MySQL(pool) => sqlx::query_as(query).fetch_all(pool).await?,
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };

            if let Some(pid) = program_id {
                events.retain(|e| e.program_id.as_deref() == Some(pid));
            }
            if let Some(aid) = asset_id {
                events.retain(|e| e.asset_id == aid);
            }
            if let Some(types) = event_types {
                if !types.is_empty() {
                    events.retain(|e| types.contains(&e.event_type));
                }
            }
            if let Some(sevs) = severities {
                if !sevs.is_empty() {
                    events.retain(|e| sevs.contains(&e.severity));
                }
            }
            if let Some(stats) = statuses {
                if !stats.is_empty() {
                    events.retain(|e| stats.contains(&e.status));
                }
            }

            if let Some(off) = offset {
                events = events.into_iter().skip(off as usize).collect();
            }
            if let Some(lim) = limit {
                events.truncate(lim as usize);
            }
            return Ok(events);
        }

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

        let rows = sqlx_query.fetch_all(self.get_pool()?).await?;
        Ok(rows.into_iter().map(row_to_bounty_change_event).collect())
    }

    /// Get change event statistics
    pub async fn get_bounty_change_event_stats(
        &self,
        program_id: Option<&str>,
    ) -> Result<BountyChangeEventStats> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let total: (i64,) = match (runtime, program_id) {
            (DatabasePool::PostgreSQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_change_events WHERE program_id = $1")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::PostgreSQL(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_change_events")
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_change_events WHERE program_id = ?")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_change_events")
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_change_events WHERE program_id = ?")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_change_events")
                    .fetch_one(pool)
                    .await?
            }
        };

        let type_rows: Vec<(String, i64)> = match (runtime, program_id) {
            (DatabasePool::PostgreSQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT event_type, COUNT(*) FROM bounty_change_events WHERE program_id = $1 GROUP BY event_type")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::PostgreSQL(pool), None) => {
                sqlx::query_as("SELECT event_type, COUNT(*) FROM bounty_change_events GROUP BY event_type")
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), Some(pid)) => {
                sqlx::query_as("SELECT event_type, COUNT(*) FROM bounty_change_events WHERE program_id = ? GROUP BY event_type")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), None) => {
                sqlx::query_as("SELECT event_type, COUNT(*) FROM bounty_change_events GROUP BY event_type")
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT event_type, COUNT(*) FROM bounty_change_events WHERE program_id = ? GROUP BY event_type")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), None) => {
                sqlx::query_as("SELECT event_type, COUNT(*) FROM bounty_change_events GROUP BY event_type")
                    .fetch_all(pool)
                    .await?
            }
        };
        let by_type: std::collections::HashMap<String, i32> =
            type_rows.into_iter().map(|(k, v)| (k, v as i32)).collect();

        let severity_rows: Vec<(String, i64)> = match (runtime, program_id) {
            (DatabasePool::PostgreSQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT severity, COUNT(*) FROM bounty_change_events WHERE program_id = $1 GROUP BY severity")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::PostgreSQL(pool), None) => {
                sqlx::query_as("SELECT severity, COUNT(*) FROM bounty_change_events GROUP BY severity")
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), Some(pid)) => {
                sqlx::query_as("SELECT severity, COUNT(*) FROM bounty_change_events WHERE program_id = ? GROUP BY severity")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), None) => {
                sqlx::query_as("SELECT severity, COUNT(*) FROM bounty_change_events GROUP BY severity")
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT severity, COUNT(*) FROM bounty_change_events WHERE program_id = ? GROUP BY severity")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), None) => {
                sqlx::query_as("SELECT severity, COUNT(*) FROM bounty_change_events GROUP BY severity")
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
                sqlx::query_as("SELECT status, COUNT(*) FROM bounty_change_events WHERE program_id = $1 GROUP BY status")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::PostgreSQL(pool), None) => {
                sqlx::query_as("SELECT status, COUNT(*) FROM bounty_change_events GROUP BY status")
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), Some(pid)) => {
                sqlx::query_as("SELECT status, COUNT(*) FROM bounty_change_events WHERE program_id = ? GROUP BY status")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), None) => {
                sqlx::query_as("SELECT status, COUNT(*) FROM bounty_change_events GROUP BY status")
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT status, COUNT(*) FROM bounty_change_events WHERE program_id = ? GROUP BY status")
                    .bind(pid)
                    .fetch_all(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), None) => {
                sqlx::query_as("SELECT status, COUNT(*) FROM bounty_change_events GROUP BY status")
                    .fetch_all(pool)
                    .await?
            }
        };
        let by_status: std::collections::HashMap<String, i32> = status_rows
            .into_iter()
            .map(|(k, v)| (k, v as i32))
            .collect();

        let pending: (i64,) = match (runtime, program_id) {
            (DatabasePool::PostgreSQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_change_events WHERE program_id = $1 AND status IN ('new', 'analyzing', 'review_required')")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::PostgreSQL(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_change_events WHERE status IN ('new', 'analyzing', 'review_required')")
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_change_events WHERE program_id = ? AND status IN ('new', 'analyzing', 'review_required')")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_change_events WHERE status IN ('new', 'analyzing', 'review_required')")
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_change_events WHERE program_id = ? AND status IN ('new', 'analyzing', 'review_required')")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), None) => {
                sqlx::query_as("SELECT COUNT(*) FROM bounty_change_events WHERE status IN ('new', 'analyzing', 'review_required')")
                    .fetch_one(pool)
                    .await?
            }
        };

        let avg: (f64,) = match (runtime, program_id) {
            (DatabasePool::PostgreSQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT COALESCE(AVG(risk_score), 0.0) FROM bounty_change_events WHERE program_id = $1")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::PostgreSQL(pool), None) => {
                sqlx::query_as("SELECT COALESCE(AVG(risk_score), 0.0) FROM bounty_change_events")
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), Some(pid)) => {
                sqlx::query_as("SELECT COALESCE(AVG(risk_score), 0.0) FROM bounty_change_events WHERE program_id = ?")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::SQLite(pool), None) => {
                sqlx::query_as("SELECT COALESCE(AVG(risk_score), 0.0) FROM bounty_change_events")
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), Some(pid)) => {
                sqlx::query_as("SELECT COALESCE(AVG(risk_score), 0.0) FROM bounty_change_events WHERE program_id = ?")
                    .bind(pid)
                    .fetch_one(pool)
                    .await?
            }
            (DatabasePool::MySQL(pool), None) => {
                sqlx::query_as("SELECT COALESCE(AVG(risk_score), 0.0) FROM bounty_change_events")
                    .fetch_one(pool)
                    .await?
            }
        };

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
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let now = chrono::Utc::now().to_rfc3339();
        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query(
                        "UPDATE bounty_change_events SET status = ?, resolved_at = ?, updated_at = ? WHERE id = ?"
                    )
                    .bind(status)
                    .bind(resolved_at)
                    .bind(&now)
                    .bind(id)
                    .execute(pool)
                    .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query(
                        "UPDATE bounty_change_events SET status = ?, resolved_at = ?, updated_at = ? WHERE id = ?"
                    )
                    .bind(status)
                    .bind(resolved_at)
                    .bind(&now)
                    .bind(id)
                    .execute(pool)
                    .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query(
            "UPDATE bounty_change_events SET status = $1, resolved_at = $2, updated_at = $3 WHERE id = $4"
        )
        .bind(status)
        .bind(resolved_at)
        .bind(&now)
        .bind(id)
        .execute(self.get_pool()?)
        .await?;

        Ok(result.rows_affected() > 0)
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

        let mut findings: Vec<String> = event
            .generated_findings_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        if !findings.contains(&finding_id.to_string()) {
            findings.push(finding_id.to_string());
            event.generated_findings_json =
                Some(serde_json::to_string(&findings).unwrap_or_default());
            event.updated_at = chrono::Utc::now().to_rfc3339();
            return self.update_bounty_change_event(&event).await;
        }

        Ok(true)
    }
}

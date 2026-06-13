use std::collections::HashMap;

use crate::core::models::database::BotExecutionRun;
use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::Result;
use chrono::{DateTime, Utc};

impl DatabaseService {
    pub async fn count_bot_execution_runs_by_status(
        &self,
        since: DateTime<Utc>,
    ) -> Result<HashMap<String, i64>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, (String, i64)>(
                    r#"SELECT status, COUNT(*)::BIGINT
                       FROM bot_execution_runs
                       WHERE started_at >= $1
                       GROUP BY status"#,
                )
                .bind(since)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, (String, i64)>(
                    r#"SELECT status, COUNT(*)
                       FROM bot_execution_runs
                       WHERE started_at >= ?
                       GROUP BY status"#,
                )
                .bind(since)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, (String, i64)>(
                    r#"SELECT status, COUNT(*)
                       FROM bot_execution_runs
                       WHERE started_at >= ?
                       GROUP BY status"#,
                )
                .bind(since)
                .fetch_all(pool)
                .await?
            }
        };

        Ok(rows.into_iter().collect())
    }

    pub async fn list_failed_bot_execution_runs(
        &self,
        since: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<BotExecutionRun>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let records = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, BotExecutionRun>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, sender_id, conversation_id,
                        ai_execution_id, assistant_profile_id, trigger_kind, trigger_bot_message_id,
                        trigger_ai_message_id, task_text, status, result_text, error_message,
                        started_at, completed_at, created_at, updated_at
                    FROM bot_execution_runs
                    WHERE started_at >= $1
                      AND status IN ('failed', 'error')
                    ORDER BY started_at DESC
                    LIMIT $2"#,
                )
                .bind(since)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, BotExecutionRun>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, sender_id, conversation_id,
                        ai_execution_id, assistant_profile_id, trigger_kind, trigger_bot_message_id,
                        trigger_ai_message_id, task_text, status, result_text, error_message,
                        started_at, completed_at, created_at, updated_at
                    FROM bot_execution_runs
                    WHERE started_at >= ?
                      AND status IN ('failed', 'error')
                    ORDER BY started_at DESC
                    LIMIT ?"#,
                )
                .bind(since)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, BotExecutionRun>(
                    r#"SELECT
                        id, transport, account_id, peer_type, peer_id, sender_id, conversation_id,
                        ai_execution_id, assistant_profile_id, trigger_kind, trigger_bot_message_id,
                        trigger_ai_message_id, task_text, status, result_text, error_message,
                        started_at, completed_at, created_at, updated_at
                    FROM bot_execution_runs
                    WHERE started_at >= ?
                      AND status IN ('failed', 'error')
                    ORDER BY started_at DESC
                    LIMIT ?"#,
                )
                .bind(since)
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
        };

        Ok(records)
    }

    pub async fn avg_bot_execution_duration_ms(&self, since: DateTime<Utc>) -> Result<f64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let avg = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let row: Option<(Option<f64>,)> = sqlx::query_as(
                    r#"SELECT AVG(EXTRACT(EPOCH FROM (completed_at - started_at)) * 1000.0)
                       FROM bot_execution_runs
                       WHERE started_at >= $1 AND completed_at IS NOT NULL"#,
                )
                .bind(since)
                .fetch_optional(pool)
                .await?;
                row.and_then(|(value,)| value).unwrap_or(0.0)
            }
            DatabasePool::SQLite(pool) => {
                let row: Option<(Option<f64>,)> = sqlx::query_as(
                    r#"SELECT AVG((julianday(completed_at) - julianday(started_at)) * 86400000.0)
                       FROM bot_execution_runs
                       WHERE started_at >= ? AND completed_at IS NOT NULL"#,
                )
                .bind(since)
                .fetch_optional(pool)
                .await?;
                row.and_then(|(value,)| value).unwrap_or(0.0)
            }
            DatabasePool::MySQL(pool) => {
                let row: Option<(Option<f64>,)> = sqlx::query_as(
                    r#"SELECT AVG(TIMESTAMPDIFF(MICROSECOND, started_at, completed_at) / 1000.0)
                       FROM bot_execution_runs
                       WHERE started_at >= ? AND completed_at IS NOT NULL"#,
                )
                .bind(since)
                .fetch_optional(pool)
                .await?;
                row.and_then(|(value,)| value).unwrap_or(0.0)
            }
        };

        Ok(avg)
    }

    pub async fn count_bot_executions_for_account_in_window(
        &self,
        transport: &str,
        account_id: &str,
        since: DateTime<Utc>,
    ) -> Result<i64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let count = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let row: (i64,) = sqlx::query_as(
                    r#"SELECT COUNT(*)::BIGINT
                       FROM bot_execution_runs
                       WHERE transport = $1 AND account_id = $2 AND started_at >= $3"#,
                )
                .bind(transport)
                .bind(account_id)
                .bind(since)
                .fetch_one(pool)
                .await?;
                row.0
            }
            DatabasePool::SQLite(pool) => {
                let row: (i64,) = sqlx::query_as(
                    r#"SELECT COUNT(*)
                       FROM bot_execution_runs
                       WHERE transport = ? AND account_id = ? AND started_at >= ?"#,
                )
                .bind(transport)
                .bind(account_id)
                .bind(since)
                .fetch_one(pool)
                .await?;
                row.0
            }
            DatabasePool::MySQL(pool) => {
                let row: (i64,) = sqlx::query_as(
                    r#"SELECT COUNT(*)
                       FROM bot_execution_runs
                       WHERE transport = ? AND account_id = ? AND started_at >= ?"#,
                )
                .bind(transport)
                .bind(account_id)
                .bind(since)
                .fetch_one(pool)
                .await?;
                row.0
            }
        };

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::database::BotExecutionRun;
    use crate::database_service::connection_manager::DatabasePool;
    use chrono::Duration;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup() -> DatabaseService {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");
        let mut svc = DatabaseService::new();
        svc.runtime_pool = Some(DatabasePool::SQLite(pool.clone()));
        svc.ensure_bot_compat_schema(&DatabasePool::SQLite(pool))
            .await
            .expect("bot schema");

        let now = Utc::now();
        svc.create_bot_execution_run(&BotExecutionRun {
            id: "run-1".to_string(),
            transport: "weixin".to_string(),
            account_id: "acct-1".to_string(),
            peer_type: "dm".to_string(),
            peer_id: "peer-1".to_string(),
            sender_id: "user".to_string(),
            conversation_id: "weixin:acct-1:dm:peer-1".to_string(),
            ai_execution_id: "ai-1".to_string(),
            assistant_profile_id: None,
            trigger_kind: "message".to_string(),
            trigger_bot_message_id: None,
            trigger_ai_message_id: None,
            task_text: "test".to_string(),
            status: "failed".to_string(),
            result_text: None,
            error_message: Some("boom".to_string()),
            started_at: now - Duration::minutes(5),
            completed_at: Some(now),
            created_at: now,
            updated_at: now,
        })
        .await
        .expect("insert run");

        svc
    }

    #[tokio::test]
    async fn counts_executions_by_status_in_window() {
        let svc = setup().await;
        let since = Utc::now() - Duration::hours(1);
        let counts = svc
            .count_bot_execution_runs_by_status(since)
            .await
            .expect("count");
        assert_eq!(counts.get("failed").copied().unwrap_or(0), 1);
    }

    #[tokio::test]
    async fn lists_failed_executions_in_window() {
        let svc = setup().await;
        let since = Utc::now() - Duration::hours(1);
        let failed = svc
            .list_failed_bot_execution_runs(since, 10)
            .await
            .expect("list failed");
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].id, "run-1");
    }

    #[tokio::test]
    async fn counts_executions_for_account_in_window() {
        let svc = setup().await;
        let since = Utc::now() - Duration::hours(1);
        let count = svc
            .count_bot_executions_for_account_in_window("weixin", "acct-1", since)
            .await
            .expect("count account");
        assert_eq!(count, 1);
    }
}

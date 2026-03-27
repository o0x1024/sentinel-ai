use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BountyChangeEventWorkflowRunRow {
    pub id: String,
    pub event_id: String,
    pub execution_id: String,
    pub binding_id: Option<String>,
    pub workflow_template_id: String,
    pub workflow_template_name: Option<String>,
    pub trigger_mode: String,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl DatabaseService {
    async fn ensure_bounty_change_event_workflow_runs_table(&self) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let create_table_sql = r#"CREATE TABLE IF NOT EXISTS bounty_change_event_workflow_runs (
                id TEXT PRIMARY KEY,
                event_id TEXT NOT NULL,
                execution_id TEXT NOT NULL UNIQUE,
                binding_id TEXT,
                workflow_template_id TEXT NOT NULL,
                workflow_template_name TEXT,
                trigger_mode TEXT NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )"#;
        let create_event_index_sql =
            "CREATE INDEX IF NOT EXISTS idx_bounty_change_event_workflow_runs_event ON bounty_change_event_workflow_runs(event_id)";
        let create_binding_index_sql =
            "CREATE INDEX IF NOT EXISTS idx_bounty_change_event_workflow_runs_event_binding ON bounty_change_event_workflow_runs(event_id, binding_id)";

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(create_table_sql).execute(pool).await?;
                sqlx::query(create_event_index_sql).execute(pool).await?;
                sqlx::query(create_binding_index_sql).execute(pool).await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(create_table_sql).execute(pool).await?;
                sqlx::query(create_event_index_sql).execute(pool).await?;
                sqlx::query(create_binding_index_sql).execute(pool).await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(create_table_sql).execute(pool).await?;
                sqlx::query(create_event_index_sql).execute(pool).await?;
                sqlx::query(create_binding_index_sql).execute(pool).await?;
            }
        }

        Ok(())
    }

    pub async fn create_bounty_change_event_workflow_run(
        &self,
        row: &BountyChangeEventWorkflowRunRow,
    ) -> Result<()> {
        self.ensure_bounty_change_event_workflow_runs_table()
            .await?;

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let sql = r#"INSERT INTO bounty_change_event_workflow_runs (
                id, event_id, execution_id, binding_id, workflow_template_id,
                workflow_template_name, trigger_mode, status, error_message,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO bounty_change_event_workflow_runs (
                        id, event_id, execution_id, binding_id, workflow_template_id,
                        workflow_template_name, trigger_mode, status, error_message,
                        created_at, updated_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#,
                )
                .bind(&row.id)
                .bind(&row.event_id)
                .bind(&row.execution_id)
                .bind(&row.binding_id)
                .bind(&row.workflow_template_id)
                .bind(&row.workflow_template_name)
                .bind(&row.trigger_mode)
                .bind(&row.status)
                .bind(&row.error_message)
                .bind(&row.created_at)
                .bind(&row.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(sql)
                    .bind(&row.id)
                    .bind(&row.event_id)
                    .bind(&row.execution_id)
                    .bind(&row.binding_id)
                    .bind(&row.workflow_template_id)
                    .bind(&row.workflow_template_name)
                    .bind(&row.trigger_mode)
                    .bind(&row.status)
                    .bind(&row.error_message)
                    .bind(&row.created_at)
                    .bind(&row.updated_at)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(sql)
                    .bind(&row.id)
                    .bind(&row.event_id)
                    .bind(&row.execution_id)
                    .bind(&row.binding_id)
                    .bind(&row.workflow_template_id)
                    .bind(&row.workflow_template_name)
                    .bind(&row.trigger_mode)
                    .bind(&row.status)
                    .bind(&row.error_message)
                    .bind(&row.created_at)
                    .bind(&row.updated_at)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn list_bounty_change_event_workflow_runs(
        &self,
        event_id: &str,
    ) -> Result<Vec<BountyChangeEventWorkflowRunRow>> {
        self.ensure_bounty_change_event_workflow_runs_table()
            .await?;

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as(
                r#"SELECT * FROM bounty_change_event_workflow_runs
                   WHERE event_id = $1
                   ORDER BY created_at DESC"#,
            )
            .bind(event_id)
            .fetch_all(pool)
            .await?),
            DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                r#"SELECT * FROM bounty_change_event_workflow_runs
                   WHERE event_id = ?
                   ORDER BY created_at DESC"#,
            )
            .bind(event_id)
            .fetch_all(pool)
            .await?),
            DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                r#"SELECT * FROM bounty_change_event_workflow_runs
                   WHERE event_id = ?
                   ORDER BY created_at DESC"#,
            )
            .bind(event_id)
            .fetch_all(pool)
            .await?),
        }
    }

    pub async fn get_bounty_change_event_workflow_run_for_binding(
        &self,
        event_id: &str,
        binding_id: &str,
    ) -> Result<Option<BountyChangeEventWorkflowRunRow>> {
        self.ensure_bounty_change_event_workflow_runs_table()
            .await?;

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as(
                r#"SELECT * FROM bounty_change_event_workflow_runs
                   WHERE event_id = $1 AND binding_id = $2
                   ORDER BY created_at DESC
                   LIMIT 1"#,
            )
            .bind(event_id)
            .bind(binding_id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                r#"SELECT * FROM bounty_change_event_workflow_runs
                   WHERE event_id = ? AND binding_id = ?
                   ORDER BY created_at DESC
                   LIMIT 1"#,
            )
            .bind(event_id)
            .bind(binding_id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                r#"SELECT * FROM bounty_change_event_workflow_runs
                   WHERE event_id = ? AND binding_id = ?
                   ORDER BY created_at DESC
                   LIMIT 1"#,
            )
            .bind(event_id)
            .bind(binding_id)
            .fetch_optional(pool)
            .await?),
        }
    }

    pub async fn get_bounty_change_event_workflow_run_by_execution_id(
        &self,
        execution_id: &str,
    ) -> Result<Option<BountyChangeEventWorkflowRunRow>> {
        self.ensure_bounty_change_event_workflow_runs_table()
            .await?;

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => Ok(sqlx::query_as(
                r#"SELECT * FROM bounty_change_event_workflow_runs
                   WHERE execution_id = $1
                   LIMIT 1"#,
            )
            .bind(execution_id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                r#"SELECT * FROM bounty_change_event_workflow_runs
                   WHERE execution_id = ?
                   LIMIT 1"#,
            )
            .bind(execution_id)
            .fetch_optional(pool)
            .await?),
            DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                r#"SELECT * FROM bounty_change_event_workflow_runs
                   WHERE execution_id = ?
                   LIMIT 1"#,
            )
            .bind(execution_id)
            .fetch_optional(pool)
            .await?),
        }
    }

    pub async fn update_bounty_change_event_workflow_run_status(
        &self,
        execution_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<bool> {
        self.ensure_bounty_change_event_workflow_runs_table()
            .await?;

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now().to_rfc3339();

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let result = sqlx::query(
                    r#"UPDATE bounty_change_event_workflow_runs
                       SET status = $1, error_message = $2, updated_at = $3
                       WHERE execution_id = $4"#,
                )
                .bind(status)
                .bind(error_message)
                .bind(&now)
                .bind(execution_id)
                .execute(pool)
                .await?;
                Ok(result.rows_affected() > 0)
            }
            DatabasePool::SQLite(pool) => {
                let result = sqlx::query(
                    r#"UPDATE bounty_change_event_workflow_runs
                       SET status = ?, error_message = ?, updated_at = ?
                       WHERE execution_id = ?"#,
                )
                .bind(status)
                .bind(error_message)
                .bind(&now)
                .bind(execution_id)
                .execute(pool)
                .await?;
                Ok(result.rows_affected() > 0)
            }
            DatabasePool::MySQL(pool) => {
                let result = sqlx::query(
                    r#"UPDATE bounty_change_event_workflow_runs
                       SET status = ?, error_message = ?, updated_at = ?
                       WHERE execution_id = ?"#,
                )
                .bind(status)
                .bind(error_message)
                .bind(&now)
                .bind(execution_id)
                .execute(pool)
                .await?;
                Ok(result.rows_affected() > 0)
            }
        }
    }
}

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::QueryBuilder;

use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::database_service::sqlx_compat::{MySql, Postgres};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorTaskPersistRecord {
    pub id: String,
    pub program_id: String,
    pub name: String,
    pub interval_secs: i64,
    pub enabled: bool,
    pub config_json: String,
    pub task_json: String,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub run_count: i64,
    pub events_detected: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl DatabaseService {
    pub async fn list_monitor_task_jsons(&self, program_id: Option<&str>) -> Result<Vec<String>> {
        let runtime = self.get_runtime_pool()?;

        let rows: Vec<(String,)> = match runtime {
            DatabasePool::SQLite(pool) => {
                let mut qb = QueryBuilder::<sqlx::Sqlite>::new(
                    "SELECT task_json FROM monitor_tasks WHERE 1=1",
                );
                if let Some(program_id) = program_id {
                    qb.push(" AND program_id = ")
                        .push_bind(program_id.to_string());
                }
                qb.push(" ORDER BY created_at ASC, id ASC");
                qb.build_query_as().fetch_all(&pool).await?
            }
            DatabasePool::MySQL(pool) => {
                let mut qb =
                    QueryBuilder::<MySql>::new("SELECT task_json FROM monitor_tasks WHERE 1=1");
                if let Some(program_id) = program_id {
                    qb.push(" AND program_id = ")
                        .push_bind(program_id.to_string());
                }
                qb.push(" ORDER BY created_at ASC, id ASC");
                qb.build_query_as().fetch_all(&pool).await?
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut qb =
                    QueryBuilder::<Postgres>::new("SELECT task_json FROM monitor_tasks WHERE 1=1");
                if let Some(program_id) = program_id {
                    qb.push(" AND program_id = ")
                        .push_bind(program_id.to_string());
                }
                qb.push(" ORDER BY created_at ASC, id ASC");
                qb.build_query_as().fetch_all(&pool).await?
            }
        };

        Ok(rows.into_iter().map(|row| row.0).collect())
    }

    pub async fn count_monitor_tasks(&self) -> Result<i64> {
        let runtime = self.get_runtime_pool()?;
        let count = match runtime {
            DatabasePool::SQLite(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM monitor_tasks")
                    .fetch_one(&pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM monitor_tasks")
                    .fetch_one(&pool)
                    .await?
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM monitor_tasks")
                    .fetch_one(&pool)
                    .await?
            }
        };
        Ok(count)
    }

    pub async fn replace_monitor_tasks(&self, tasks: &[MonitorTaskPersistRecord]) -> Result<()> {
        let runtime = self.get_runtime_pool()?;

        match runtime {
            DatabasePool::SQLite(pool) => {
                let mut tx = pool.begin().await?;
                delete_removed_monitor_tasks_sqlite(&mut tx, tasks).await?;
                for task in tasks {
                    sqlx::query(
                        r#"
                        INSERT INTO monitor_tasks (
                            id, program_id, name, interval_secs, enabled, config_json, task_json,
                            last_run_at, next_run_at, run_count, events_detected, created_at, updated_at
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        ON CONFLICT(id) DO UPDATE SET
                            program_id = excluded.program_id,
                            name = excluded.name,
                            interval_secs = excluded.interval_secs,
                            enabled = excluded.enabled,
                            config_json = excluded.config_json,
                            task_json = excluded.task_json,
                            last_run_at = excluded.last_run_at,
                            next_run_at = excluded.next_run_at,
                            run_count = excluded.run_count,
                            events_detected = excluded.events_detected,
                            created_at = excluded.created_at,
                            updated_at = excluded.updated_at
                        "#,
                    )
                    .bind(&task.id)
                    .bind(&task.program_id)
                    .bind(&task.name)
                    .bind(task.interval_secs)
                    .bind(task.enabled)
                    .bind(&task.config_json)
                    .bind(&task.task_json)
                    .bind(&task.last_run_at)
                    .bind(&task.next_run_at)
                    .bind(task.run_count)
                    .bind(task.events_detected)
                    .bind(&task.created_at)
                    .bind(&task.updated_at)
                    .execute(&mut *tx)
                    .await?;
                }
                tx.commit().await?;
            }
            DatabasePool::MySQL(pool) => {
                let mut tx = pool.begin().await?;
                delete_removed_monitor_tasks_mysql(&mut tx, tasks).await?;
                for task in tasks {
                    sqlx::query(
                        r#"
                        INSERT INTO monitor_tasks (
                            id, program_id, name, interval_secs, enabled, config_json, task_json,
                            last_run_at, next_run_at, run_count, events_detected, created_at, updated_at
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        ON DUPLICATE KEY UPDATE
                            program_id = VALUES(program_id),
                            name = VALUES(name),
                            interval_secs = VALUES(interval_secs),
                            enabled = VALUES(enabled),
                            config_json = VALUES(config_json),
                            task_json = VALUES(task_json),
                            last_run_at = VALUES(last_run_at),
                            next_run_at = VALUES(next_run_at),
                            run_count = VALUES(run_count),
                            events_detected = VALUES(events_detected),
                            created_at = VALUES(created_at),
                            updated_at = VALUES(updated_at)
                        "#,
                    )
                    .bind(&task.id)
                    .bind(&task.program_id)
                    .bind(&task.name)
                    .bind(task.interval_secs)
                    .bind(task.enabled)
                    .bind(&task.config_json)
                    .bind(&task.task_json)
                    .bind(&task.last_run_at)
                    .bind(&task.next_run_at)
                    .bind(task.run_count)
                    .bind(task.events_detected)
                    .bind(&task.created_at)
                    .bind(&task.updated_at)
                    .execute(&mut *tx)
                    .await?;
                }
                tx.commit().await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                let mut tx = pool.begin().await?;
                delete_removed_monitor_tasks_postgres(&mut tx, tasks).await?;
                for task in tasks {
                    sqlx::query(
                        r#"
                        INSERT INTO monitor_tasks (
                            id, program_id, name, interval_secs, enabled, config_json, task_json,
                            last_run_at, next_run_at, run_count, events_detected, created_at, updated_at
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                        ON CONFLICT(id) DO UPDATE SET
                            program_id = excluded.program_id,
                            name = excluded.name,
                            interval_secs = excluded.interval_secs,
                            enabled = excluded.enabled,
                            config_json = excluded.config_json,
                            task_json = excluded.task_json,
                            last_run_at = excluded.last_run_at,
                            next_run_at = excluded.next_run_at,
                            run_count = excluded.run_count,
                            events_detected = excluded.events_detected,
                            created_at = excluded.created_at,
                            updated_at = excluded.updated_at
                        "#,
                    )
                    .bind(&task.id)
                    .bind(&task.program_id)
                    .bind(&task.name)
                    .bind(task.interval_secs)
                    .bind(task.enabled)
                    .bind(&task.config_json)
                    .bind(&task.task_json)
                    .bind(&task.last_run_at)
                    .bind(&task.next_run_at)
                    .bind(task.run_count)
                    .bind(task.events_detected)
                    .bind(&task.created_at)
                    .bind(&task.updated_at)
                    .execute(&mut *tx)
                    .await?;
                }
                tx.commit().await?;
            }
        }

        Ok(())
    }
}

async fn delete_removed_monitor_tasks_sqlite(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    tasks: &[MonitorTaskPersistRecord],
) -> Result<()> {
    if tasks.is_empty() {
        sqlx::query("DELETE FROM monitor_tasks")
            .execute(&mut **tx)
            .await?;
        return Ok(());
    }

    let mut qb = QueryBuilder::<sqlx::Sqlite>::new("DELETE FROM monitor_tasks WHERE id NOT IN (");
    {
        let mut separated = qb.separated(", ");
        for task in tasks {
            separated.push_bind(task.id.clone());
        }
    }
    qb.push(")");
    qb.build().execute(&mut **tx).await?;
    Ok(())
}

async fn delete_removed_monitor_tasks_mysql(
    tx: &mut sqlx::Transaction<'_, MySql>,
    tasks: &[MonitorTaskPersistRecord],
) -> Result<()> {
    if tasks.is_empty() {
        sqlx::query("DELETE FROM monitor_tasks")
            .execute(&mut **tx)
            .await?;
        return Ok(());
    }

    let mut qb = QueryBuilder::<MySql>::new("DELETE FROM monitor_tasks WHERE id NOT IN (");
    {
        let mut separated = qb.separated(", ");
        for task in tasks {
            separated.push_bind(task.id.clone());
        }
    }
    qb.push(")");
    qb.build().execute(&mut **tx).await?;
    Ok(())
}

async fn delete_removed_monitor_tasks_postgres(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    tasks: &[MonitorTaskPersistRecord],
) -> Result<()> {
    if tasks.is_empty() {
        sqlx::query("DELETE FROM monitor_tasks")
            .execute(&mut **tx)
            .await?;
        return Ok(());
    }

    let mut qb = QueryBuilder::<Postgres>::new("DELETE FROM monitor_tasks WHERE id NOT IN (");
    {
        let mut separated = qb.separated(", ");
        for task in tasks {
            separated.push_bind(task.id.clone());
        }
    }
    qb.push(")");
    qb.build().execute(&mut **tx).await?;
    Ok(())
}

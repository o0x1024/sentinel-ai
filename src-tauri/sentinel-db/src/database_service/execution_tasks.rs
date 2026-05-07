//! Database operations for execution task persistence

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;

use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;

/// Execution task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl ToString for ExecutionTaskStatus {
    fn to_string(&self) -> String {
        match self {
            ExecutionTaskStatus::Pending => "pending".to_string(),
            ExecutionTaskStatus::InProgress => "in_progress".to_string(),
            ExecutionTaskStatus::Completed => "completed".to_string(),
            ExecutionTaskStatus::Failed => "failed".to_string(),
        }
    }
}

impl From<&str> for ExecutionTaskStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => ExecutionTaskStatus::Pending,
            "in_progress" => ExecutionTaskStatus::InProgress,
            "completed" => ExecutionTaskStatus::Completed,
            "failed" => ExecutionTaskStatus::Failed,
            _ => ExecutionTaskStatus::Pending,
        }
    }
}

/// Execution task database model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExecutionTaskItem {
    pub id: String,
    pub execution_id: String,
    pub item_index: i32,
    pub description: String,
    pub status: String,
    pub result: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Execution task input for insertion/update
#[derive(Debug, Clone)]
pub struct ExecutionTaskInput {
    pub description: String,
    pub status: ExecutionTaskStatus,
    pub result: Option<String>,
}

impl DatabaseService {
    async fn ensure_execution_tasks_schema(&self) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let legacy_table_exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS (
                        SELECT 1 FROM information_schema.tables
                        WHERE table_name = 'agent_todos'
                    )",
                )
                .fetch_one(pool)
                .await?;
                let execution_table_exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS (
                        SELECT 1 FROM information_schema.tables
                        WHERE table_name = 'execution_tasks'
                    )",
                )
                .fetch_one(pool)
                .await?;

                if legacy_table_exists && !execution_table_exists {
                    sqlx::query("ALTER TABLE agent_todos RENAME TO execution_tasks")
                        .execute(pool)
                        .await?;
                }

                sqlx::query(
                    r#"CREATE TABLE IF NOT EXISTS execution_tasks (
                        id TEXT PRIMARY KEY,
                        execution_id TEXT NOT NULL,
                        item_index INTEGER NOT NULL,
                        description TEXT NOT NULL,
                        status TEXT NOT NULL,
                        result TEXT,
                        created_at TIMESTAMPTZ NOT NULL,
                        updated_at TIMESTAMPTZ NOT NULL
                    )"#,
                )
                .execute(pool)
                .await?;

                for index_sql in [
                    "DROP INDEX IF EXISTS idx_agent_todos_execution",
                    "DROP INDEX IF EXISTS idx_agent_todos_execution_index",
                    "DROP INDEX IF EXISTS idx_agent_todos_updated",
                    "CREATE INDEX IF NOT EXISTS idx_execution_tasks_execution ON execution_tasks(execution_id)",
                    "CREATE INDEX IF NOT EXISTS idx_execution_tasks_execution_index ON execution_tasks(execution_id, item_index)",
                    "CREATE INDEX IF NOT EXISTS idx_execution_tasks_updated ON execution_tasks(updated_at DESC)",
                ] {
                    sqlx::query(index_sql).execute(pool).await?;
                }
            }
            DatabasePool::SQLite(pool) => {
                let legacy_table_exists: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
                )
                .bind("agent_todos")
                .fetch_one(pool)
                .await?;
                let execution_table_exists: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
                )
                .bind("execution_tasks")
                .fetch_one(pool)
                .await?;

                if legacy_table_exists > 0 && execution_table_exists == 0 {
                    sqlx::query("ALTER TABLE agent_todos RENAME TO execution_tasks")
                        .execute(pool)
                        .await?;
                }

                sqlx::query(
                    r#"CREATE TABLE IF NOT EXISTS execution_tasks (
                        id TEXT PRIMARY KEY,
                        execution_id TEXT NOT NULL,
                        item_index INTEGER NOT NULL,
                        description TEXT NOT NULL,
                        status TEXT NOT NULL,
                        result TEXT,
                        created_at DATETIME NOT NULL,
                        updated_at DATETIME NOT NULL
                    )"#,
                )
                .execute(pool)
                .await?;

                for index_sql in [
                    "DROP INDEX IF EXISTS idx_agent_todos_execution",
                    "DROP INDEX IF EXISTS idx_agent_todos_execution_index",
                    "DROP INDEX IF EXISTS idx_agent_todos_updated",
                    "CREATE INDEX IF NOT EXISTS idx_execution_tasks_execution ON execution_tasks(execution_id)",
                    "CREATE INDEX IF NOT EXISTS idx_execution_tasks_execution_index ON execution_tasks(execution_id, item_index)",
                    "CREATE INDEX IF NOT EXISTS idx_execution_tasks_updated ON execution_tasks(updated_at DESC)",
                ] {
                    sqlx::query(index_sql).execute(pool).await?;
                }
            }
            DatabasePool::MySQL(pool) => {
                let legacy_table_exists: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = ?",
                )
                .bind("agent_todos")
                .fetch_one(pool)
                .await?;
                let execution_table_exists: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = ?",
                )
                .bind("execution_tasks")
                .fetch_one(pool)
                .await?;

                if legacy_table_exists > 0 && execution_table_exists == 0 {
                    sqlx::query("RENAME TABLE agent_todos TO execution_tasks")
                        .execute(pool)
                        .await?;
                }

                sqlx::query(
                    r#"CREATE TABLE IF NOT EXISTS execution_tasks (
                        id TEXT PRIMARY KEY,
                        execution_id TEXT NOT NULL,
                        item_index INTEGER NOT NULL,
                        description TEXT NOT NULL,
                        status TEXT NOT NULL,
                        result TEXT,
                        created_at DATETIME NOT NULL,
                        updated_at DATETIME NOT NULL
                    )"#,
                )
                .execute(pool)
                .await?;

                for index_sql in [
                    "CREATE INDEX idx_execution_tasks_execution ON execution_tasks(execution_id)",
                    "CREATE INDEX idx_execution_tasks_execution_index ON execution_tasks(execution_id, item_index)",
                    "CREATE INDEX idx_execution_tasks_updated ON execution_tasks(updated_at)",
                ] {
                    let _ = sqlx::query(index_sql).execute(pool).await;
                }
            }
        }

        Ok(())
    }

    /// Get all execution tasks for an execution
    pub async fn get_execution_tasks(&self, execution_id: &str) -> Result<Vec<ExecutionTaskItem>> {
        self.ensure_execution_tasks_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let rows =
            match runtime {
                DatabasePool::PostgreSQL(pool) => sqlx::query_as::<_, ExecutionTaskItem>(
                    "SELECT * FROM execution_tasks WHERE execution_id = $1 ORDER BY item_index ASC",
                )
                .bind(execution_id)
                .fetch_all(pool)
                .await?,
                DatabasePool::SQLite(pool) => sqlx::query_as::<_, ExecutionTaskItem>(
                    "SELECT * FROM execution_tasks WHERE execution_id = ? ORDER BY item_index ASC",
                )
                .bind(execution_id)
                .fetch_all(pool)
                .await?,
                DatabasePool::MySQL(pool) => sqlx::query_as::<_, ExecutionTaskItem>(
                    "SELECT * FROM execution_tasks WHERE execution_id = ? ORDER BY item_index ASC",
                )
                .bind(execution_id)
                .fetch_all(pool)
                .await?,
            };
        Ok(rows)
    }

    /// Save or replace all execution tasks for an execution
    pub async fn save_execution_tasks(
        &self,
        execution_id: &str,
        items: &[ExecutionTaskInput],
    ) -> Result<()> {
        self.ensure_execution_tasks_schema().await?;
        let existing_created_at: HashMap<(i32, String), String> = self
            .get_execution_tasks(execution_id)
            .await?
            .into_iter()
            .map(|item| ((item.item_index, item.description), item.created_at))
            .collect();
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now().to_rfc3339();

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM execution_tasks WHERE execution_id = $1")
                    .bind(execution_id)
                    .execute(pool)
                    .await?;

                for (index, item) in items.iter().enumerate() {
                    let id = format!("{}_{}", execution_id, index);
                    let created_at = existing_created_at
                        .get(&(index as i32, item.description.clone()))
                        .cloned()
                        .unwrap_or_else(|| now.clone());
                    sqlx::query(
                        r#"INSERT INTO execution_tasks (id, execution_id, item_index, description, status, result, created_at, updated_at)
                           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#
                    )
                    .bind(&id)
                    .bind(execution_id)
                    .bind(index as i32)
                    .bind(&item.description)
                    .bind(item.status.to_string())
                    .bind(&item.result)
                    .bind(&created_at)
                    .bind(&now)
                    .execute(pool)
                    .await?;
                }
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM execution_tasks WHERE execution_id = ?")
                    .bind(execution_id)
                    .execute(pool)
                    .await?;

                for (index, item) in items.iter().enumerate() {
                    let id = format!("{}_{}", execution_id, index);
                    let created_at = existing_created_at
                        .get(&(index as i32, item.description.clone()))
                        .cloned()
                        .unwrap_or_else(|| now.clone());
                    sqlx::query(
                        r#"INSERT INTO execution_tasks (id, execution_id, item_index, description, status, result, created_at, updated_at)
                           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#
                    )
                    .bind(&id)
                    .bind(execution_id)
                    .bind(index as i32)
                    .bind(&item.description)
                    .bind(item.status.to_string())
                    .bind(&item.result)
                    .bind(&created_at)
                    .bind(&now)
                    .execute(pool)
                    .await?;
                }
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM execution_tasks WHERE execution_id = ?")
                    .bind(execution_id)
                    .execute(pool)
                    .await?;

                for (index, item) in items.iter().enumerate() {
                    let id = format!("{}_{}", execution_id, index);
                    let created_at = existing_created_at
                        .get(&(index as i32, item.description.clone()))
                        .cloned()
                        .unwrap_or_else(|| now.clone());
                    sqlx::query(
                        r#"INSERT INTO execution_tasks (id, execution_id, item_index, description, status, result, created_at, updated_at)
                           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#
                    )
                    .bind(&id)
                    .bind(execution_id)
                    .bind(index as i32)
                    .bind(&item.description)
                    .bind(item.status.to_string())
                    .bind(&item.result)
                    .bind(&created_at)
                    .bind(&now)
                    .execute(pool)
                    .await?;
                }
            }
        }

        Ok(())
    }

    /// Delete execution tasks created after a timestamp and reindex remaining items.
    pub async fn delete_execution_tasks_after(
        &self,
        execution_id: &str,
        after_timestamp_ms: i64,
    ) -> Result<u64> {
        self.ensure_execution_tasks_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let cutoff = chrono::DateTime::<Utc>::from_timestamp_millis(after_timestamp_ms)
            .ok_or_else(|| anyhow::anyhow!("无效的任务时间戳: {}", after_timestamp_ms))?;
        let now = Utc::now().to_rfc3339();

        let deleted_count = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let deleted_count = sqlx::query(
                    "DELETE FROM execution_tasks WHERE execution_id = $1 AND created_at > $2",
                )
                .bind(execution_id)
                .bind(cutoff)
                .execute(pool)
                .await?
                .rows_affected();

                let remaining = self.get_execution_tasks(execution_id).await?;
                for (new_index, item) in remaining.into_iter().enumerate() {
                    let new_id = format!("{}_{}", execution_id, new_index);
                    if item.item_index != new_index as i32 || item.id != new_id {
                        sqlx::query(
                            "UPDATE execution_tasks SET id = $1, item_index = $2, updated_at = $3 WHERE id = $4",
                        )
                        .bind(&new_id)
                        .bind(new_index as i32)
                        .bind(&now)
                        .bind(&item.id)
                        .execute(pool)
                        .await?;
                    }
                }
                deleted_count
            }
            DatabasePool::SQLite(pool) => {
                let deleted_count = sqlx::query(
                    "DELETE FROM execution_tasks WHERE execution_id = ? AND created_at > ?",
                )
                .bind(execution_id)
                .bind(cutoff.to_rfc3339())
                .execute(pool)
                .await?
                .rows_affected();

                let remaining = self.get_execution_tasks(execution_id).await?;
                for (new_index, item) in remaining.into_iter().enumerate() {
                    let new_id = format!("{}_{}", execution_id, new_index);
                    if item.item_index != new_index as i32 || item.id != new_id {
                        sqlx::query(
                            "UPDATE execution_tasks SET id = ?, item_index = ?, updated_at = ? WHERE id = ?",
                        )
                        .bind(&new_id)
                        .bind(new_index as i32)
                        .bind(&now)
                        .bind(&item.id)
                        .execute(pool)
                        .await?;
                    }
                }
                deleted_count
            }
            DatabasePool::MySQL(pool) => {
                let deleted_count = sqlx::query(
                    "DELETE FROM execution_tasks WHERE execution_id = ? AND created_at > ?",
                )
                .bind(execution_id)
                .bind(cutoff.naive_utc())
                .execute(pool)
                .await?
                .rows_affected();

                let remaining = self.get_execution_tasks(execution_id).await?;
                for (new_index, item) in remaining.into_iter().enumerate() {
                    let new_id = format!("{}_{}", execution_id, new_index);
                    if item.item_index != new_index as i32 || item.id != new_id {
                        sqlx::query(
                            "UPDATE execution_tasks SET id = ?, item_index = ?, updated_at = ? WHERE id = ?",
                        )
                        .bind(&new_id)
                        .bind(new_index as i32)
                        .bind(&now)
                        .bind(&item.id)
                        .execute(pool)
                        .await?;
                    }
                }
                deleted_count
            }
        };

        Ok(deleted_count)
    }

    /// Update a single execution task's status and result
    pub async fn update_execution_task_status(
        &self,
        execution_id: &str,
        item_index: i32,
        status: ExecutionTaskStatus,
        result: Option<&str>,
    ) -> Result<()> {
        self.ensure_execution_tasks_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now().to_rfc3339();

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE execution_tasks SET status = $1, result = $2, updated_at = $3 WHERE execution_id = $4 AND item_index = $5"
                )
                .bind(status.to_string())
                .bind(result)
                .bind(&now)
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE execution_tasks SET status = ?, result = ?, updated_at = ? WHERE execution_id = ? AND item_index = ?"
                )
                .bind(status.to_string())
                .bind(result)
                .bind(&now)
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE execution_tasks SET status = ?, result = ?, updated_at = ? WHERE execution_id = ? AND item_index = ?"
                )
                .bind(status.to_string())
                .bind(result)
                .bind(&now)
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Update a single execution task's description
    pub async fn update_execution_task_description(
        &self,
        execution_id: &str,
        item_index: i32,
        description: &str,
    ) -> Result<()> {
        self.ensure_execution_tasks_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now().to_rfc3339();

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE execution_tasks SET description = $1, updated_at = $2 WHERE execution_id = $3 AND item_index = $4"
                )
                .bind(description)
                .bind(&now)
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE execution_tasks SET description = ?, updated_at = ? WHERE execution_id = ? AND item_index = ?"
                )
                .bind(description)
                .bind(&now)
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE execution_tasks SET description = ?, updated_at = ? WHERE execution_id = ? AND item_index = ?"
                )
                .bind(description)
                .bind(&now)
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Delete all execution tasks for an execution
    pub async fn delete_execution_tasks(&self, execution_id: &str) -> Result<()> {
        self.ensure_execution_tasks_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM execution_tasks WHERE execution_id = $1")
                    .bind(execution_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM execution_tasks WHERE execution_id = ?")
                    .bind(execution_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM execution_tasks WHERE execution_id = ?")
                    .bind(execution_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    /// Delete a single execution task and reindex remaining items
    pub async fn delete_execution_task_item(
        &self,
        execution_id: &str,
        item_index: i32,
    ) -> Result<()> {
        self.ensure_execution_tasks_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now().to_rfc3339();

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "DELETE FROM execution_tasks WHERE execution_id = $1 AND item_index = $2",
                )
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;

                sqlx::query(
                    "UPDATE execution_tasks SET item_index = item_index - 1, updated_at = $1 WHERE execution_id = $2 AND item_index > $3"
                )
                .bind(&now)
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;

                let remaining = self.get_execution_tasks(execution_id).await?;
                for item in remaining {
                    let new_id = format!("{}_{}", execution_id, item.item_index);
                    if item.id != new_id {
                        sqlx::query("UPDATE execution_tasks SET id = $1 WHERE id = $2")
                            .bind(&new_id)
                            .bind(&item.id)
                            .execute(pool)
                            .await?;
                    }
                }
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "DELETE FROM execution_tasks WHERE execution_id = ? AND item_index = ?",
                )
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;

                sqlx::query(
                    "UPDATE execution_tasks SET item_index = item_index - 1, updated_at = ? WHERE execution_id = ? AND item_index > ?"
                )
                .bind(&now)
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;

                let remaining = self.get_execution_tasks(execution_id).await?;
                for item in remaining {
                    let new_id = format!("{}_{}", execution_id, item.item_index);
                    if item.id != new_id {
                        sqlx::query("UPDATE execution_tasks SET id = ? WHERE id = ?")
                            .bind(&new_id)
                            .bind(&item.id)
                            .execute(pool)
                            .await?;
                    }
                }
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "DELETE FROM execution_tasks WHERE execution_id = ? AND item_index = ?",
                )
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;

                sqlx::query(
                    "UPDATE execution_tasks SET item_index = item_index - 1, updated_at = ? WHERE execution_id = ? AND item_index > ?"
                )
                .bind(&now)
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;

                let remaining = self.get_execution_tasks(execution_id).await?;
                for item in remaining {
                    let new_id = format!("{}_{}", execution_id, item.item_index);
                    if item.id != new_id {
                        sqlx::query("UPDATE execution_tasks SET id = ? WHERE id = ?")
                            .bind(&new_id)
                            .bind(&item.id)
                            .execute(pool)
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Insert an execution task at a specific index and shift others
    pub async fn insert_execution_task_item(
        &self,
        execution_id: &str,
        item_index: i32,
        description: &str,
        status: ExecutionTaskStatus,
    ) -> Result<()> {
        self.ensure_execution_tasks_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now().to_rfc3339();

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE execution_tasks SET item_index = item_index + 1, updated_at = $1 WHERE execution_id = $2 AND item_index >= $3"
                )
                .bind(&now)
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;

                let id = format!("{}_{}", execution_id, item_index);
                sqlx::query(
                    r#"INSERT INTO execution_tasks (id, execution_id, item_index, description, status, result, created_at, updated_at)
                       VALUES ($1, $2, $3, $4, $5, NULL, $6, $7)"#
                )
                .bind(&id)
                .bind(execution_id)
                .bind(item_index)
                .bind(description)
                .bind(status.to_string())
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await?;

                let all_items = self.get_execution_tasks(execution_id).await?;
                for item in all_items {
                    let expected_id = format!("{}_{}", execution_id, item.item_index);
                    if item.id != expected_id {
                        sqlx::query("UPDATE execution_tasks SET id = $1 WHERE id = $2")
                            .bind(&expected_id)
                            .bind(&item.id)
                            .execute(pool)
                            .await?;
                    }
                }
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE execution_tasks SET item_index = item_index + 1, updated_at = ? WHERE execution_id = ? AND item_index >= ?"
                )
                .bind(&now)
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;

                let id = format!("{}_{}", execution_id, item_index);
                sqlx::query(
                    r#"INSERT INTO execution_tasks (id, execution_id, item_index, description, status, result, created_at, updated_at)
                       VALUES (?, ?, ?, ?, ?, NULL, ?, ?)"#
                )
                .bind(&id)
                .bind(execution_id)
                .bind(item_index)
                .bind(description)
                .bind(status.to_string())
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await?;

                let all_items = self.get_execution_tasks(execution_id).await?;
                for item in all_items {
                    let expected_id = format!("{}_{}", execution_id, item.item_index);
                    if item.id != expected_id {
                        sqlx::query("UPDATE execution_tasks SET id = ? WHERE id = ?")
                            .bind(&expected_id)
                            .bind(&item.id)
                            .execute(pool)
                            .await?;
                    }
                }
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE execution_tasks SET item_index = item_index + 1, updated_at = ? WHERE execution_id = ? AND item_index >= ?"
                )
                .bind(&now)
                .bind(execution_id)
                .bind(item_index)
                .execute(pool)
                .await?;

                let id = format!("{}_{}", execution_id, item_index);
                sqlx::query(
                    r#"INSERT INTO execution_tasks (id, execution_id, item_index, description, status, result, created_at, updated_at)
                       VALUES (?, ?, ?, ?, ?, NULL, ?, ?)"#
                )
                .bind(&id)
                .bind(execution_id)
                .bind(item_index)
                .bind(description)
                .bind(status.to_string())
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await?;

                let all_items = self.get_execution_tasks(execution_id).await?;
                for item in all_items {
                    let expected_id = format!("{}_{}", execution_id, item.item_index);
                    if item.id != expected_id {
                        sqlx::query("UPDATE execution_tasks SET id = ? WHERE id = ?")
                            .bind(&expected_id)
                            .bind(&item.id)
                            .execute(pool)
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Add execution tasks to the end of the list
    pub async fn append_execution_tasks(
        &self,
        execution_id: &str,
        items: &[ExecutionTaskInput],
    ) -> Result<()> {
        self.ensure_execution_tasks_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now().to_rfc3339();
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let max_index: Option<i32> = sqlx::query_scalar(
                    "SELECT MAX(item_index) FROM execution_tasks WHERE execution_id = $1",
                )
                .bind(execution_id)
                .fetch_one(pool)
                .await?;
                let start_index = max_index.map(|v| v + 1).unwrap_or(0);

                for (offset, item) in items.iter().enumerate() {
                    let index = start_index + offset as i32;
                    let id = format!("{}_{}", execution_id, index);
                    sqlx::query(
                        r#"INSERT INTO execution_tasks (id, execution_id, item_index, description, status, result, created_at, updated_at)
                           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#
                    )
                    .bind(&id)
                    .bind(execution_id)
                    .bind(index)
                    .bind(&item.description)
                    .bind(item.status.to_string())
                    .bind(&item.result)
                    .bind(&now)
                    .bind(&now)
                    .execute(pool)
                    .await?;
                }
            }
            DatabasePool::SQLite(pool) => {
                let max_index: Option<i32> = sqlx::query_scalar(
                    "SELECT MAX(item_index) FROM execution_tasks WHERE execution_id = ?",
                )
                .bind(execution_id)
                .fetch_one(pool)
                .await?;
                let start_index = max_index.map(|v| v + 1).unwrap_or(0);

                for (offset, item) in items.iter().enumerate() {
                    let index = start_index + offset as i32;
                    let id = format!("{}_{}", execution_id, index);
                    sqlx::query(
                        r#"INSERT INTO execution_tasks (id, execution_id, item_index, description, status, result, created_at, updated_at)
                           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#
                    )
                    .bind(&id)
                    .bind(execution_id)
                    .bind(index)
                    .bind(&item.description)
                    .bind(item.status.to_string())
                    .bind(&item.result)
                    .bind(&now)
                    .bind(&now)
                    .execute(pool)
                    .await?;
                }
            }
            DatabasePool::MySQL(pool) => {
                let max_index: Option<i32> = sqlx::query_scalar(
                    "SELECT MAX(item_index) FROM execution_tasks WHERE execution_id = ?",
                )
                .bind(execution_id)
                .fetch_one(pool)
                .await?;
                let start_index = max_index.map(|v| v + 1).unwrap_or(0);

                for (offset, item) in items.iter().enumerate() {
                    let index = start_index + offset as i32;
                    let id = format!("{}_{}", execution_id, index);
                    sqlx::query(
                        r#"INSERT INTO execution_tasks (id, execution_id, item_index, description, status, result, created_at, updated_at)
                           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#
                    )
                    .bind(&id)
                    .bind(execution_id)
                    .bind(index)
                    .bind(&item.description)
                    .bind(item.status.to_string())
                    .bind(&item.result)
                    .bind(&now)
                    .bind(&now)
                    .execute(pool)
                    .await?;
                }
            }
        }

        Ok(())
    }

    /// Check if execution tasks exist for an execution
    pub async fn has_execution_tasks(&self, execution_id: &str) -> Result<bool> {
        self.ensure_execution_tasks_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let count: i64 = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM execution_tasks WHERE execution_id = $1")
                    .bind(execution_id)
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM execution_tasks WHERE execution_id = ?")
                    .bind(execution_id)
                    .fetch_one(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM execution_tasks WHERE execution_id = ?")
                    .bind(execution_id)
                    .fetch_one(pool)
                    .await?
            }
        };
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    fn build_test_service(pool: SqlitePool) -> DatabaseService {
        let mut service = DatabaseService::new();
        service.runtime_pool = Some(DatabasePool::SQLite(pool));
        service
    }

    #[tokio::test]
    async fn get_execution_tasks_creates_table_when_missing() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let service = build_test_service(pool.clone());

        let items = service.get_execution_tasks("exec-1").await.unwrap();
        assert!(items.is_empty());

        let execution_table_exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
        )
        .bind("execution_tasks")
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(execution_table_exists, 1);
    }

    #[tokio::test]
    async fn get_execution_tasks_migrates_legacy_agent_todos_table() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        sqlx::query(
            r#"CREATE TABLE agent_todos (
                id TEXT PRIMARY KEY,
                execution_id TEXT NOT NULL,
                item_index INTEGER NOT NULL,
                description TEXT NOT NULL,
                status TEXT NOT NULL,
                result TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("CREATE INDEX idx_agent_todos_execution ON agent_todos(execution_id)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO agent_todos (id, execution_id, item_index, description, status, result, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind("exec-legacy_0")
        .bind("exec-legacy")
        .bind(0_i32)
        .bind("Read file")
        .bind("pending")
        .bind(Option::<String>::None)
        .bind("2026-04-19T00:00:00Z")
        .bind("2026-04-19T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();

        let service = build_test_service(pool.clone());
        let items = service.get_execution_tasks("exec-legacy").await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].description, "Read file");

        let execution_table_exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
        )
        .bind("execution_tasks")
        .fetch_one(&pool)
        .await
        .unwrap();
        let legacy_table_exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
        )
        .bind("agent_todos")
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(execution_table_exists, 1);
        assert_eq!(legacy_table_exists, 0);
    }
}

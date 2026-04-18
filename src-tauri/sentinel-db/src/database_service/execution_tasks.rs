//! Database operations for execution task persistence

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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
    /// Get all execution tasks for an execution
    pub async fn get_execution_tasks(&self, execution_id: &str) -> Result<Vec<ExecutionTaskItem>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, ExecutionTaskItem>(
                    "SELECT * FROM execution_tasks WHERE execution_id = $1 ORDER BY item_index ASC",
                )
                .bind(execution_id)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, ExecutionTaskItem>(
                    "SELECT * FROM execution_tasks WHERE execution_id = ? ORDER BY item_index ASC",
                )
                .bind(execution_id)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, ExecutionTaskItem>(
                    "SELECT * FROM execution_tasks WHERE execution_id = ? ORDER BY item_index ASC",
                )
                .bind(execution_id)
                .fetch_all(pool)
                .await?
            }
        };
        Ok(rows)
    }

    /// Save or replace all execution tasks for an execution
    pub async fn save_execution_tasks(
        &self,
        execution_id: &str,
        items: &[ExecutionTaskInput],
    ) -> Result<()> {
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
                    .bind(&now)
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
                    .bind(&now)
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
                    .bind(&now)
                    .bind(&now)
                    .execute(pool)
                    .await?;
                }
            }
        }

        Ok(())
    }

    /// Update a single execution task's status and result
    pub async fn update_execution_task_status(
        &self,
        execution_id: &str,
        item_index: i32,
        status: ExecutionTaskStatus,
        result: Option<&str>,
    ) -> Result<()> {
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
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now().to_rfc3339();

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM execution_tasks WHERE execution_id = $1 AND item_index = $2")
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
                sqlx::query("DELETE FROM execution_tasks WHERE execution_id = ? AND item_index = ?")
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
                sqlx::query("DELETE FROM execution_tasks WHERE execution_id = ? AND item_index = ?")
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

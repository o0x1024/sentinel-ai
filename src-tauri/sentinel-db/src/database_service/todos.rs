//! Database operations for agent todos persistence

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::database_service::service::DatabaseService;

/// Todo item status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl ToString for TodoStatus {
    fn to_string(&self) -> String {
        match self {
            TodoStatus::Pending => "pending".to_string(),
            TodoStatus::InProgress => "in_progress".to_string(),
            TodoStatus::Completed => "completed".to_string(),
            TodoStatus::Failed => "failed".to_string(),
        }
    }
}

impl From<&str> for TodoStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => TodoStatus::Pending,
            "in_progress" => TodoStatus::InProgress,
            "completed" => TodoStatus::Completed,
            "failed" => TodoStatus::Failed,
            _ => TodoStatus::Pending,
        }
    }
}

/// Todo item database model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentTodoItem {
    pub id: String,
    pub execution_id: String,
    pub item_index: i32,
    pub description: String,
    pub status: String,
    pub result: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Todo item for insertion/update
#[derive(Debug, Clone)]
pub struct TodoItemInput {
    pub description: String,
    pub status: TodoStatus,
    pub result: Option<String>,
}

impl DatabaseService {
    /// Get all todos for an execution
    pub async fn get_agent_todos(&self, execution_id: &str) -> Result<Vec<AgentTodoItem>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query_as::<_, AgentTodoItem>(
            "SELECT * FROM agent_todos WHERE execution_id = ? ORDER BY item_index ASC"
        )
        .bind(execution_id)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Save or replace all todos for an execution
    pub async fn save_agent_todos(&self, execution_id: &str, items: &[TodoItemInput]) -> Result<()> {
        let pool = self.get_pool()?;
        let now = Utc::now().to_rfc3339();

        // Delete existing todos for this execution
        sqlx::query("DELETE FROM agent_todos WHERE execution_id = ?")
            .bind(execution_id)
            .execute(pool)
            .await?;

        // Insert new todos
        for (index, item) in items.iter().enumerate() {
            let id = format!("{}_{}", execution_id, index);
            sqlx::query(
                r#"INSERT INTO agent_todos (id, execution_id, item_index, description, status, result, created_at, updated_at)
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

        Ok(())
    }

    /// Update a single todo item's status and result
    pub async fn update_agent_todo_status(
        &self,
        execution_id: &str,
        item_index: i32,
        status: TodoStatus,
        result: Option<&str>,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE agent_todos SET status = ?, result = ?, updated_at = ? WHERE execution_id = ? AND item_index = ?"
        )
        .bind(status.to_string())
        .bind(result)
        .bind(&now)
        .bind(execution_id)
        .bind(item_index)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Update a single todo item's description
    pub async fn update_agent_todo_description(
        &self,
        execution_id: &str,
        item_index: i32,
        description: &str,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "UPDATE agent_todos SET description = ?, updated_at = ? WHERE execution_id = ? AND item_index = ?"
        )
        .bind(description)
        .bind(&now)
        .bind(execution_id)
        .bind(item_index)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Delete all todos for an execution
    pub async fn delete_agent_todos(&self, execution_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM agent_todos WHERE execution_id = ?")
            .bind(execution_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Delete a single todo item and reindex remaining items
    pub async fn delete_agent_todo_item(&self, execution_id: &str, item_index: i32) -> Result<()> {
        let pool = self.get_pool()?;
        let now = Utc::now().to_rfc3339();

        // Delete the item
        sqlx::query("DELETE FROM agent_todos WHERE execution_id = ? AND item_index = ?")
            .bind(execution_id)
            .bind(item_index)
            .execute(pool)
            .await?;

        // Reindex items after the deleted one
        sqlx::query(
            "UPDATE agent_todos SET item_index = item_index - 1, updated_at = ? WHERE execution_id = ? AND item_index > ?"
        )
        .bind(&now)
        .bind(execution_id)
        .bind(item_index)
        .execute(pool)
        .await?;

        // Update IDs to match new indices
        let remaining = self.get_agent_todos(execution_id).await?;
        for item in remaining {
            let new_id = format!("{}_{}", execution_id, item.item_index);
            if item.id != new_id {
                sqlx::query("UPDATE agent_todos SET id = ? WHERE id = ?")
                    .bind(&new_id)
                    .bind(&item.id)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    /// Insert a todo item at a specific index and shift others
    pub async fn insert_agent_todo_item(
        &self,
        execution_id: &str,
        item_index: i32,
        description: &str,
        status: TodoStatus,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        let now = Utc::now().to_rfc3339();

        // Shift existing items at and after the insert position
        sqlx::query(
            "UPDATE agent_todos SET item_index = item_index + 1, updated_at = ? WHERE execution_id = ? AND item_index >= ?"
        )
        .bind(&now)
        .bind(execution_id)
        .bind(item_index)
        .execute(pool)
        .await?;

        // Insert new item
        let id = format!("{}_{}", execution_id, item_index);
        sqlx::query(
            r#"INSERT INTO agent_todos (id, execution_id, item_index, description, status, result, created_at, updated_at)
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

        // Update IDs for shifted items
        let all_items = self.get_agent_todos(execution_id).await?;
        for item in all_items {
            let expected_id = format!("{}_{}", execution_id, item.item_index);
            if item.id != expected_id {
                sqlx::query("UPDATE agent_todos SET id = ? WHERE id = ?")
                    .bind(&expected_id)
                    .bind(&item.id)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    /// Add todo items to the end of the list
    pub async fn append_agent_todos(&self, execution_id: &str, items: &[TodoItemInput]) -> Result<()> {
        let pool = self.get_pool()?;
        let now = Utc::now().to_rfc3339();

        // Get current max index
        let max_index: Option<(i32,)> = sqlx::query_as(
            "SELECT MAX(item_index) FROM agent_todos WHERE execution_id = ?"
        )
        .bind(execution_id)
        .fetch_optional(pool)
        .await?;

        let start_index = max_index.map(|r| r.0 + 1).unwrap_or(0);

        for (offset, item) in items.iter().enumerate() {
            let index = start_index + offset as i32;
            let id = format!("{}_{}", execution_id, index);
            sqlx::query(
                r#"INSERT INTO agent_todos (id, execution_id, item_index, description, status, result, created_at, updated_at)
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

        Ok(())
    }

    /// Check if todos exist for an execution
    pub async fn has_agent_todos(&self, execution_id: &str) -> Result<bool> {
        let pool = self.get_pool()?;
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM agent_todos WHERE execution_id = ?"
        )
        .bind(execution_id)
        .fetch_one(pool)
        .await?;
        Ok(count.0 > 0)
    }
}

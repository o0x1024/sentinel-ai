//! AbilityGroup DAO - CRUD for tool ability groups

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

/// AbilityGroup summary (for LLM selection phase 1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityGroupSummary {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// Full AbilityGroup (for phase 2 and management)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityGroup {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub tool_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAbilityGroup {
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub tool_ids: Vec<String>,
}

/// Update payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAbilityGroup {
    pub name: Option<String>,
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub tool_ids: Option<Vec<String>>,
}

/// Create AbilityGroup table SQL
pub const CREATE_ABILITY_GROUP_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS ability_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    instructions TEXT NOT NULL DEFAULT '',
    tool_ids TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)
"#;

/// AbilityGroup DAO
pub struct AbilityGroupDao;

impl AbilityGroupDao {
    /// Create table if not exists
    pub async fn init_table(pool: &SqlitePool) -> Result<()> {
        sqlx::query(CREATE_ABILITY_GROUP_TABLE_SQL)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// List all groups (summary only)
    pub async fn list_summary(pool: &SqlitePool) -> Result<Vec<AbilityGroupSummary>> {
        let rows = sqlx::query("SELECT id, name, description FROM ability_groups ORDER BY name")
            .fetch_all(pool)
            .await?;

        let groups = rows
            .iter()
            .map(|row| AbilityGroupSummary {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
            })
            .collect();

        Ok(groups)
    }

    /// List groups by allowed IDs (summary only)
    pub async fn list_summary_by_ids(
        pool: &SqlitePool,
        ids: &[String],
    ) -> Result<Vec<AbilityGroupSummary>> {
        if ids.is_empty() {
            return Self::list_summary(pool).await;
        }

        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT id, name, description FROM ability_groups WHERE id IN ({}) ORDER BY name",
            placeholders
        );

        let mut q = sqlx::query(&query);
        for id in ids {
            q = q.bind(id);
        }

        let rows = q.fetch_all(pool).await?;

        let groups = rows
            .iter()
            .map(|row| AbilityGroupSummary {
                id: row.get("id"),
                name: row.get("name"),
                description: row.get("description"),
            })
            .collect();

        Ok(groups)
    }

    /// Get full group by ID
    pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<AbilityGroup>> {
        let row = sqlx::query("SELECT * FROM ability_groups WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(|r| Self::row_to_group(&r)))
    }

    /// Get full group by name
    pub async fn get_by_name(pool: &SqlitePool, name: &str) -> Result<Option<AbilityGroup>> {
        let row = sqlx::query("SELECT * FROM ability_groups WHERE name = ?")
            .bind(name)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(|r| Self::row_to_group(&r)))
    }

    /// List all groups (full)
    pub async fn list_all(pool: &SqlitePool) -> Result<Vec<AbilityGroup>> {
        let rows = sqlx::query("SELECT * FROM ability_groups ORDER BY name")
            .fetch_all(pool)
            .await?;

        Ok(rows.iter().map(|r| Self::row_to_group(r)).collect())
    }

    /// Create a new group
    pub async fn create(pool: &SqlitePool, payload: &CreateAbilityGroup) -> Result<AbilityGroup> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let tool_ids_json = serde_json::to_string(&payload.tool_ids)?;

        sqlx::query(
            "INSERT INTO ability_groups (id, name, description, instructions, tool_ids, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&payload.name)
        .bind(&payload.description)
        .bind(&payload.instructions)
        .bind(&tool_ids_json)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(pool)
        .await?;

        Ok(AbilityGroup {
            id,
            name: payload.name.clone(),
            description: payload.description.clone(),
            instructions: payload.instructions.clone(),
            tool_ids: payload.tool_ids.clone(),
            created_at: now,
            updated_at: now,
        })
    }

    /// Update an existing group
    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        payload: &UpdateAbilityGroup,
    ) -> Result<bool> {
        let existing = Self::get(pool, id).await?;
        if existing.is_none() {
            return Ok(false);
        }
        let existing = existing.unwrap();

        let name = payload.name.as_ref().unwrap_or(&existing.name);
        let description = payload.description.as_ref().unwrap_or(&existing.description);
        let instructions = payload
            .instructions
            .as_ref()
            .unwrap_or(&existing.instructions);
        let tool_ids = payload.tool_ids.as_ref().unwrap_or(&existing.tool_ids);
        let tool_ids_json = serde_json::to_string(tool_ids)?;
        let now = Utc::now();

        sqlx::query(
            "UPDATE ability_groups SET name = ?, description = ?, instructions = ?, tool_ids = ?, updated_at = ? WHERE id = ?",
        )
        .bind(name)
        .bind(description)
        .bind(instructions)
        .bind(&tool_ids_json)
        .bind(now.to_rfc3339())
        .bind(id)
        .execute(pool)
        .await?;

        Ok(true)
    }

    /// Delete a group
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM ability_groups WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    fn row_to_group(row: &sqlx::sqlite::SqliteRow) -> AbilityGroup {
        let tool_ids_str: String = row.get("tool_ids");
        let tool_ids: Vec<String> =
            serde_json::from_str(&tool_ids_str).unwrap_or_default();

        let created_at_str: String = row.get("created_at");
        let updated_at_str: String = row.get("updated_at");

        AbilityGroup {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
            instructions: row.get("instructions"),
            tool_ids,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
}


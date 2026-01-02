use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use crate::database_service::service::DatabaseService;

/// Level 1: Basic metadata (for LLM selection phase 1 - initial discovery)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityGroupSummary {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// Level 2: Detailed information (for LLM selection phase 2 - understanding purpose)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityGroupDetail {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub additional_notes: String,
    pub tool_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Level 3: Full details (for phase 3 - execution and management)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityGroup {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub additional_notes: String,
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
    pub additional_notes: String,
    pub tool_ids: Vec<String>,
}

/// Update payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAbilityGroup {
    pub name: Option<String>,
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub additional_notes: Option<String>,
    pub tool_ids: Option<Vec<String>>,
}

impl DatabaseService {
    /// List all groups (summary only)
    pub async fn list_ability_groups_summary_internal(&self) -> Result<Vec<AbilityGroupSummary>> {
        let pool = self.get_pool()?;
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
    pub async fn list_ability_groups_summary_by_ids_internal(
        &self,
        ids: &[String],
    ) -> Result<Vec<AbilityGroupSummary>> {
        if ids.is_empty() {
            return self.list_ability_groups_summary_internal().await;
        }

        let pool = self.get_pool()?;
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

    /// Get Level 2 detail by ID (without tool_ids)
    pub async fn get_ability_group_detail_internal(&self, id: &str) -> Result<Option<AbilityGroupDetail>> {
        let pool = self.get_pool()?;
        let row = sqlx::query("SELECT id, name, description, instructions, additional_notes, tool_ids, created_at, updated_at FROM ability_groups WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(|r| {
            let tool_ids_str: String = r.get("tool_ids");
            let tool_ids: Vec<String> = serde_json::from_str(&tool_ids_str).unwrap_or_default();
            let tool_count = tool_ids.len();

            let created_at_str: String = r.get("created_at");
            let updated_at_str: String = r.get("updated_at");

            AbilityGroupDetail {
                id: r.get("id"),
                name: r.get("name"),
                description: r.get("description"),
                instructions: r.get("instructions"),
                additional_notes: r.get("additional_notes"),
                tool_count,
                created_at: DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            }
        }))
    }

    /// Get Level 3 full group by ID (with tool_ids)
    pub async fn get_ability_group_internal(&self, id: &str) -> Result<Option<AbilityGroup>> {
        let pool = self.get_pool()?;
        let row = sqlx::query("SELECT * FROM ability_groups WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(|r| self.row_to_ability_group(&r)))
    }

    /// Get full group by name
    pub async fn get_ability_group_by_name_internal(&self, name: &str) -> Result<Option<AbilityGroup>> {
        let pool = self.get_pool()?;
        let row = sqlx::query("SELECT * FROM ability_groups WHERE name = ?")
            .bind(name)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(|r| self.row_to_ability_group(&r)))
    }

    /// List all groups (full)
    pub async fn list_all_ability_groups_internal(&self) -> Result<Vec<AbilityGroup>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT * FROM ability_groups ORDER BY name")
            .fetch_all(pool)
            .await?;

        Ok(rows.iter().map(|r| self.row_to_ability_group(r)).collect())
    }

    /// Create a new group
    pub async fn create_ability_group_internal(&self, payload: &CreateAbilityGroup) -> Result<AbilityGroup> {
        let pool = self.get_pool()?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let tool_ids_json = serde_json::to_string(&payload.tool_ids)?;

        sqlx::query(
            "INSERT INTO ability_groups (id, name, description, instructions, additional_notes, tool_ids, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&payload.name)
        .bind(&payload.description)
        .bind(&payload.instructions)
        .bind(&payload.additional_notes)
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
            additional_notes: payload.additional_notes.clone(),
            tool_ids: payload.tool_ids.clone(),
            created_at: now,
            updated_at: now,
        })
    }

    /// Update an existing group
    pub async fn update_ability_group_internal(
        &self,
        id: &str,
        payload: &UpdateAbilityGroup,
    ) -> Result<bool> {
        let pool = self.get_pool()?;
        let existing = self.get_ability_group_internal(id).await?;
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
        let additional_notes = payload
            .additional_notes
            .as_ref()
            .unwrap_or(&existing.additional_notes);
        let tool_ids = payload.tool_ids.as_ref().unwrap_or(&existing.tool_ids);
        let tool_ids_json = serde_json::to_string(tool_ids)?;
        let now = Utc::now();

        sqlx::query(
            "UPDATE ability_groups SET name = ?, description = ?, instructions = ?, additional_notes = ?, tool_ids = ?, updated_at = ? WHERE id = ?",
        )
        .bind(name)
        .bind(description)
        .bind(instructions)
        .bind(additional_notes)
        .bind(&tool_ids_json)
        .bind(now.to_rfc3339())
        .bind(id)
        .execute(pool)
        .await?;

        Ok(true)
    }

    /// Delete a group
    pub async fn delete_ability_group_internal(&self, id: &str) -> Result<bool> {
        let pool = self.get_pool()?;
        let result = sqlx::query("DELETE FROM ability_groups WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    fn row_to_ability_group(&self, row: &sqlx::sqlite::SqliteRow) -> AbilityGroup {
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
            additional_notes: row.get("additional_notes"),
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


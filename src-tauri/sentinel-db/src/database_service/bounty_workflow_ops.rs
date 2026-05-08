//! Bug bounty workflow template and binding database operations.

use anyhow::Result;
use tracing::info;

use super::bounty::{
    optional_timestamp_string_to_datetime, row_to_bounty_workflow_binding,
    row_to_bounty_workflow_template, timestamp_string_to_datetime, BountyWorkflowBindingRow,
    BountyWorkflowTemplateRow,
};
use super::service::DatabaseService;
use crate::database_service::connection_manager::DatabasePool;

impl DatabaseService {
    // ------------------------------------------------------------------------
    // Workflow Template CRUD
    // ------------------------------------------------------------------------

    /// Create a workflow template
    pub async fn create_bounty_workflow_template(
        &self,
        template: &BountyWorkflowTemplateRow,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"INSERT INTO bounty_workflow_templates (
                    id, name, description, category, workflow_definition_id, steps_json,
                    input_schema_json, output_schema_json, tags_json, is_built_in,
                    estimated_duration_mins, created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#;
            match runtime {
                DatabasePool::SQLite(pool) => {
                    sqlx::query(query)
                        .bind(&template.id)
                        .bind(&template.name)
                        .bind(&template.description)
                        .bind(&template.category)
                        .bind(&template.workflow_definition_id)
                        .bind(&template.steps_json)
                        .bind(&template.input_schema_json)
                        .bind(&template.output_schema_json)
                        .bind(&template.tags_json)
                        .bind(template.is_built_in)
                        .bind(template.estimated_duration_mins)
                        .bind(&template.created_at)
                        .bind(&template.updated_at)
                        .execute(pool)
                        .await?;
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query(query)
                        .bind(&template.id)
                        .bind(&template.name)
                        .bind(&template.description)
                        .bind(&template.category)
                        .bind(&template.workflow_definition_id)
                        .bind(&template.steps_json)
                        .bind(&template.input_schema_json)
                        .bind(&template.output_schema_json)
                        .bind(&template.tags_json)
                        .bind(template.is_built_in)
                        .bind(template.estimated_duration_mins)
                        .bind(&template.created_at)
                        .bind(&template.updated_at)
                        .execute(pool)
                        .await?;
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            }
            info!("Created bounty workflow template: {}", template.id);
            return Ok(());
        }

        sqlx::query(
            r#"INSERT INTO bounty_workflow_templates (
                id, name, description, category, workflow_definition_id, steps_json,
                input_schema_json, output_schema_json, tags_json, is_built_in,
                estimated_duration_mins, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#,
        )
        .bind(&template.id)
        .bind(&template.name)
        .bind(&template.description)
        .bind(&template.category)
        .bind(&template.workflow_definition_id)
        .bind(&template.steps_json)
        .bind(&template.input_schema_json)
        .bind(&template.output_schema_json)
        .bind(&template.tags_json)
        .bind(template.is_built_in)
        .bind(template.estimated_duration_mins)
        .bind(timestamp_string_to_datetime(&template.created_at))
        .bind(timestamp_string_to_datetime(&template.updated_at))
        .execute(self.get_pool()?)
        .await?;

        info!("Created bounty workflow template: {}", template.id);
        Ok(())
    }

    /// Get a workflow template by ID
    pub async fn get_bounty_workflow_template(
        &self,
        id: &str,
    ) -> Result<Option<BountyWorkflowTemplateRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_workflow_templates WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_workflow_templates WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?),
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let row = sqlx::query("SELECT * FROM bounty_workflow_templates WHERE id = $1")
            .bind(id)
            .fetch_optional(self.get_pool()?)
            .await?;

        Ok(row.map(row_to_bounty_workflow_template))
    }

    /// List workflow templates
    pub async fn list_bounty_workflow_templates(
        &self,
        category: Option<&str>,
        is_built_in: Option<bool>,
    ) -> Result<Vec<BountyWorkflowTemplateRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"
                SELECT
                    id, name, description, category, workflow_definition_id, steps_json, input_schema_json,
                    output_schema_json, tags_json, is_built_in, estimated_duration_mins,
                    CAST(created_at AS TEXT) AS created_at, CAST(updated_at AS TEXT) AS updated_at
                FROM bounty_workflow_templates
                ORDER BY name ASC
            "#;
            let mut templates: Vec<BountyWorkflowTemplateRow> = match runtime {
                DatabasePool::SQLite(pool) => sqlx::query_as(query).fetch_all(pool).await?,
                DatabasePool::MySQL(pool) => sqlx::query_as(query).fetch_all(pool).await?,
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
            if let Some(cat) = category {
                templates.retain(|t| t.category == cat);
            }
            if let Some(built_in) = is_built_in {
                templates.retain(|t| t.is_built_in == built_in);
            }
            return Ok(templates);
        }

        let mut query = String::from("SELECT * FROM bounty_workflow_templates WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(cat) = category {
            params.push(cat.to_string());
            query.push_str(&format!(" AND category = ${}", params.len()));
        }
        if let Some(built_in) = is_built_in {
            query.push_str(&format!(" AND is_built_in = {}", built_in));
        }

        query.push_str(" ORDER BY name ASC");

        // Note: is_built_in is handled via literal boolean formatting for Postgres (TRUE/FALSE)
        // category is bound.

        let mut sqlx_query = sqlx::query(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param.clone());
        }

        let rows = sqlx_query.fetch_all(self.get_pool()?).await?;

        Ok(rows
            .into_iter()
            .map(row_to_bounty_workflow_template)
            .collect())
    }

    /// Update a workflow template
    pub async fn update_bounty_workflow_template(
        &self,
        template: &BountyWorkflowTemplateRow,
    ) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"UPDATE bounty_workflow_templates SET
                    name = ?, description = ?, category = ?, workflow_definition_id = ?,
                    steps_json = ?, input_schema_json = ?, output_schema_json = ?,
                    tags_json = ?, estimated_duration_mins = ?, updated_at = ?
                WHERE id = ?"#;
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query(query)
                        .bind(&template.name)
                        .bind(&template.description)
                        .bind(&template.category)
                        .bind(&template.workflow_definition_id)
                        .bind(&template.steps_json)
                        .bind(&template.input_schema_json)
                        .bind(&template.output_schema_json)
                        .bind(&template.tags_json)
                        .bind(template.estimated_duration_mins)
                        .bind(&template.updated_at)
                        .bind(&template.id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query(query)
                        .bind(&template.name)
                        .bind(&template.description)
                        .bind(&template.category)
                        .bind(&template.workflow_definition_id)
                        .bind(&template.steps_json)
                        .bind(&template.input_schema_json)
                        .bind(&template.output_schema_json)
                        .bind(&template.tags_json)
                        .bind(template.estimated_duration_mins)
                        .bind(&template.updated_at)
                        .bind(&template.id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query(
            r#"UPDATE bounty_workflow_templates SET
                name = $1, description = $2, category = $3, workflow_definition_id = $4,
                steps_json = $5, input_schema_json = $6, output_schema_json = $7,
                tags_json = $8, estimated_duration_mins = $9, updated_at = $10
            WHERE id = $11"#,
        )
        .bind(&template.name)
        .bind(&template.description)
        .bind(&template.category)
        .bind(&template.workflow_definition_id)
        .bind(&template.steps_json)
        .bind(&template.input_schema_json)
        .bind(&template.output_schema_json)
        .bind(&template.tags_json)
        .bind(template.estimated_duration_mins)
        .bind(timestamp_string_to_datetime(&template.updated_at))
        .bind(&template.id)
        .execute(self.get_pool()?)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a workflow template
    pub async fn delete_bounty_workflow_template(&self, id: &str) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_workflow_templates WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_workflow_templates WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query("DELETE FROM bounty_workflow_templates WHERE id = $1")
            .bind(id)
            .execute(self.get_pool()?)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ------------------------------------------------------------------------
    // Workflow Binding CRUD
    // ------------------------------------------------------------------------

    /// Create a workflow binding
    pub async fn create_bounty_workflow_binding(
        &self,
        binding: &BountyWorkflowBindingRow,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"INSERT INTO bounty_workflow_bindings (
                    id, program_id, scope_id, workflow_template_id, is_enabled,
                    auto_run_on_change, trigger_conditions_json, schedule_cron,
                    last_run_at, last_run_status, run_count, created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#;
            match runtime {
                DatabasePool::SQLite(pool) => {
                    sqlx::query(query)
                        .bind(&binding.id)
                        .bind(&binding.program_id)
                        .bind(&binding.scope_id)
                        .bind(&binding.workflow_template_id)
                        .bind(binding.is_enabled)
                        .bind(binding.auto_run_on_change)
                        .bind(&binding.trigger_conditions_json)
                        .bind(&binding.schedule_cron)
                        .bind(&binding.last_run_at)
                        .bind(&binding.last_run_status)
                        .bind(binding.run_count)
                        .bind(&binding.created_at)
                        .bind(&binding.updated_at)
                        .execute(pool)
                        .await?;
                }
                DatabasePool::MySQL(pool) => {
                    sqlx::query(query)
                        .bind(&binding.id)
                        .bind(&binding.program_id)
                        .bind(&binding.scope_id)
                        .bind(&binding.workflow_template_id)
                        .bind(binding.is_enabled)
                        .bind(binding.auto_run_on_change)
                        .bind(&binding.trigger_conditions_json)
                        .bind(&binding.schedule_cron)
                        .bind(&binding.last_run_at)
                        .bind(&binding.last_run_status)
                        .bind(binding.run_count)
                        .bind(&binding.created_at)
                        .bind(&binding.updated_at)
                        .execute(pool)
                        .await?;
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            }
            info!("Created bounty workflow binding: {}", binding.id);
            return Ok(());
        }

        sqlx::query(
            r#"INSERT INTO bounty_workflow_bindings (
                id, program_id, scope_id, workflow_template_id, is_enabled,
                auto_run_on_change, trigger_conditions_json, schedule_cron,
                last_run_at, last_run_status, run_count, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#,
        )
        .bind(&binding.id)
        .bind(&binding.program_id)
        .bind(&binding.scope_id)
        .bind(&binding.workflow_template_id)
        .bind(binding.is_enabled)
        .bind(binding.auto_run_on_change)
        .bind(&binding.trigger_conditions_json)
        .bind(&binding.schedule_cron)
        .bind(optional_timestamp_string_to_datetime(&binding.last_run_at))
        .bind(&binding.last_run_status)
        .bind(binding.run_count)
        .bind(timestamp_string_to_datetime(&binding.created_at))
        .bind(timestamp_string_to_datetime(&binding.updated_at))
        .execute(self.get_pool()?)
        .await?;

        info!("Created bounty workflow binding: {}", binding.id);
        Ok(())
    }

    /// Get a workflow binding by ID
    pub async fn get_bounty_workflow_binding(
        &self,
        id: &str,
    ) -> Result<Option<BountyWorkflowBindingRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_workflow_bindings WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                    "SELECT * FROM bounty_workflow_bindings WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?),
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let row = sqlx::query("SELECT * FROM bounty_workflow_bindings WHERE id = $1")
            .bind(id)
            .fetch_optional(self.get_pool()?)
            .await?;

        Ok(row.map(row_to_bounty_workflow_binding))
    }

    /// List workflow bindings for a program
    pub async fn list_bounty_workflow_bindings(
        &self,
        program_id: Option<&str>,
        scope_id: Option<&str>,
        is_enabled: Option<bool>,
    ) -> Result<Vec<BountyWorkflowBindingRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"
                SELECT
                    id, program_id, scope_id, workflow_template_id, is_enabled, auto_run_on_change,
                    trigger_conditions_json, schedule_cron, CAST(last_run_at AS TEXT) AS last_run_at,
                    last_run_status, run_count, CAST(created_at AS TEXT) AS created_at,
                    CAST(updated_at AS TEXT) AS updated_at
                FROM bounty_workflow_bindings
                ORDER BY created_at DESC
            "#;
            let mut bindings: Vec<BountyWorkflowBindingRow> = match runtime {
                DatabasePool::SQLite(pool) => sqlx::query_as(query).fetch_all(pool).await?,
                DatabasePool::MySQL(pool) => sqlx::query_as(query).fetch_all(pool).await?,
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
            if let Some(pid) = program_id {
                bindings.retain(|b| b.program_id == pid);
            }
            if let Some(sid) = scope_id {
                bindings.retain(|b| b.scope_id.as_deref() == Some(sid));
            }
            if let Some(enabled) = is_enabled {
                bindings.retain(|b| b.is_enabled == enabled);
            }
            return Ok(bindings);
        }

        let mut query = String::from("SELECT * FROM bounty_workflow_bindings WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(pid) = program_id {
            params.push(pid.to_string());
            query.push_str(&format!(" AND program_id = ${}", params.len()));
        }
        if let Some(sid) = scope_id {
            params.push(sid.to_string());
            query.push_str(&format!(" AND scope_id = ${}", params.len()));
        }
        if let Some(enabled) = is_enabled {
            query.push_str(&format!(" AND is_enabled = {}", enabled));
        }

        query.push_str(" ORDER BY created_at DESC");

        let mut sqlx_query = sqlx::query(&query);
        for param in &params {
            sqlx_query = sqlx_query.bind(param.clone());
        }

        let rows = sqlx_query.fetch_all(self.get_pool()?).await?;

        Ok(rows
            .into_iter()
            .map(row_to_bounty_workflow_binding)
            .collect())
    }

    /// Update a workflow binding
    pub async fn update_bounty_workflow_binding(
        &self,
        binding: &BountyWorkflowBindingRow,
    ) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            let query = r#"UPDATE bounty_workflow_bindings SET
                    scope_id = ?, is_enabled = ?, auto_run_on_change = ?,
                    trigger_conditions_json = ?, schedule_cron = ?,
                    last_run_at = ?, last_run_status = ?, run_count = ?, updated_at = ?
                WHERE id = ?"#;
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query(query)
                        .bind(&binding.scope_id)
                        .bind(binding.is_enabled)
                        .bind(binding.auto_run_on_change)
                        .bind(&binding.trigger_conditions_json)
                        .bind(&binding.schedule_cron)
                        .bind(&binding.last_run_at)
                        .bind(&binding.last_run_status)
                        .bind(binding.run_count)
                        .bind(&binding.updated_at)
                        .bind(&binding.id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query(query)
                        .bind(&binding.scope_id)
                        .bind(binding.is_enabled)
                        .bind(binding.auto_run_on_change)
                        .bind(&binding.trigger_conditions_json)
                        .bind(&binding.schedule_cron)
                        .bind(&binding.last_run_at)
                        .bind(&binding.last_run_status)
                        .bind(binding.run_count)
                        .bind(&binding.updated_at)
                        .bind(&binding.id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query(
            r#"UPDATE bounty_workflow_bindings SET
                scope_id = $1, is_enabled = $2, auto_run_on_change = $3,
                trigger_conditions_json = $4, schedule_cron = $5,
                last_run_at = $6, last_run_status = $7, run_count = $8, updated_at = $9
            WHERE id = $10"#,
        )
        .bind(&binding.scope_id)
        .bind(binding.is_enabled)
        .bind(binding.auto_run_on_change)
        .bind(&binding.trigger_conditions_json)
        .bind(&binding.schedule_cron)
        .bind(optional_timestamp_string_to_datetime(&binding.last_run_at))
        .bind(&binding.last_run_status)
        .bind(binding.run_count)
        .bind(timestamp_string_to_datetime(&binding.updated_at))
        .bind(&binding.id)
        .execute(self.get_pool()?)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a workflow binding
    pub async fn delete_bounty_workflow_binding(&self, id: &str) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_workflow_bindings WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query("DELETE FROM bounty_workflow_bindings WHERE id = ?")
                        .bind(id)
                        .execute(pool)
                        .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let result = sqlx::query("DELETE FROM bounty_workflow_bindings WHERE id = $1")
            .bind(id)
            .execute(self.get_pool()?)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update binding run status
    pub async fn update_bounty_workflow_binding_run_status(
        &self,
        id: &str,
        status: &str,
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
                        r#"UPDATE bounty_workflow_bindings SET
                            last_run_at = ?, last_run_status = ?, run_count = run_count + 1, updated_at = ?
                        WHERE id = ?"#
                    )
                    .bind(&now)
                    .bind(status)
                    .bind(&now)
                    .bind(id)
                    .execute(pool)
                    .await?;
                    Ok(result.rows_affected() > 0)
                }
                DatabasePool::MySQL(pool) => {
                    let result = sqlx::query(
                        r#"UPDATE bounty_workflow_bindings SET
                            last_run_at = ?, last_run_status = ?, run_count = run_count + 1, updated_at = ?
                        WHERE id = ?"#
                    )
                    .bind(&now)
                    .bind(status)
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
            r#"UPDATE bounty_workflow_bindings SET
                last_run_at = $1, last_run_status = $2, run_count = run_count + 1, updated_at = $3
            WHERE id = $4"#,
        )
        .bind(&now)
        .bind(status)
        .bind(&now)
        .bind(id)
        .execute(self.get_pool()?)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get bindings that should auto-run on change events
    pub async fn get_auto_trigger_workflow_bindings(
        &self,
        program_id: &str,
    ) -> Result<Vec<BountyWorkflowBindingRow>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        if matches!(runtime, DatabasePool::SQLite(_) | DatabasePool::MySQL(_)) {
            return match runtime {
                DatabasePool::SQLite(pool) => Ok(sqlx::query_as(
                    r#"SELECT * FROM bounty_workflow_bindings
                       WHERE program_id = ? AND is_enabled = 1 AND auto_run_on_change = 1"#,
                )
                .bind(program_id)
                .fetch_all(pool)
                .await?),
                DatabasePool::MySQL(pool) => Ok(sqlx::query_as(
                    r#"SELECT * FROM bounty_workflow_bindings
                       WHERE program_id = ? AND is_enabled = TRUE AND auto_run_on_change = TRUE"#,
                )
                .bind(program_id)
                .fetch_all(pool)
                .await?),
                DatabasePool::PostgreSQL(_) => unreachable!(),
            };
        }

        let rows = sqlx::query(
            r#"SELECT * FROM bounty_workflow_bindings
               WHERE program_id = $1 AND is_enabled = TRUE AND auto_run_on_change = TRUE"#,
        )
        .bind(program_id)
        .fetch_all(self.get_pool()?)
        .await?;

        Ok(rows
            .into_iter()
            .map(row_to_bounty_workflow_binding)
            .collect())
    }
}

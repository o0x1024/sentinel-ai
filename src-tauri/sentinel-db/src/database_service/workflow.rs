use anyhow::Result;
use crate::core::models::database::ExecutionStatistics;
use crate::database_service::service::DatabaseService;

impl DatabaseService {
    // --- Workflow Runs (original workflow.rs) ---

    pub async fn create_workflow_run_internal(&self, id: &str, workflow_id: &str, workflow_name: &str, version: &str, status: &str, started_at: chrono::DateTime<chrono::Utc>) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "INSERT INTO workflow_runs (id, workflow_id, workflow_name, version, status, started_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(id)
        .bind(workflow_id)
        .bind(workflow_name)
        .bind(version)
        .bind(status)
        .bind(started_at) 
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_workflow_run_status_internal(&self, id: &str, status: &str, completed_at: Option<chrono::DateTime<chrono::Utc>>, error_message: Option<&str>) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "UPDATE workflow_runs SET status = ?, completed_at = ?, error_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
        )
        .bind(status)
        .bind(completed_at)
        .bind(error_message)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_workflow_run_progress_internal(&self, id: &str, progress: u32, completed_steps: u32, total_steps: u32) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "UPDATE workflow_runs SET progress = ?, completed_steps = ?, total_steps = ? WHERE id = ?"
        )
        .bind(progress as i32)
        .bind(completed_steps as i32)
        .bind(total_steps as i32)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn save_workflow_run_step_internal(&self, run_id: &str, step_id: &str, status: &str, started_at: chrono::DateTime<chrono::Utc>) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "INSERT INTO workflow_run_steps (run_id, step_id, status, started_at) VALUES (?, ?, ?, ?)"
        )
        .bind(run_id)
        .bind(step_id)
        .bind(status)
        .bind(started_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_workflow_run_step_status_internal(&self, run_id: &str, step_id: &str, status: &str, completed_at: chrono::DateTime<chrono::Utc>, result_json: Option<String>, error_message: Option<&str>) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "UPDATE workflow_run_steps SET status = ?, completed_at = ?, result_json = ?, error_message = ? WHERE run_id = ? AND step_id = ?"
        )
        .bind(status)
        .bind(completed_at)
        .bind(result_json)
        .bind(error_message)
        .bind(run_id)
        .bind(step_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn list_workflow_runs_internal(&self) -> Result<Vec<serde_json::Value>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT * FROM workflow_runs ORDER BY started_at DESC")
            .fetch_all(pool)
            .await?;

        let mut runs = Vec::new();
        for row in rows {
            let id: String = sqlx::Row::get(&row, "id");
            let workflow_id: String = sqlx::Row::get(&row, "workflow_id");
            runs.push(serde_json::json!({ "id": id, "workflow_id": workflow_id }));
        }
        Ok(runs)
    }

    pub async fn list_workflow_runs_paginated_internal(&self, page: i64, page_size: i64, _search: Option<&str>, workflow_id: Option<&str>) -> Result<(Vec<serde_json::Value>, i64)> {
        let pool = self.get_pool()?;
        let offset = (page - 1) * page_size;

        let mut query_str = "SELECT * FROM workflow_runs WHERE 1=1".to_string();
        if let Some(wid) = workflow_id {
            query_str.push_str(&format!(" AND workflow_id = '{}'", wid));
        }
        query_str.push_str(&format!(" ORDER BY started_at DESC LIMIT {} OFFSET {}", page_size, offset));

        let rows = sqlx::query(&query_str).fetch_all(pool).await?;
        let mut runs = Vec::new();
        for row in rows {
            let id: String = sqlx::Row::get(&row, "id");
            runs.push(serde_json::json!({ "id": id }));
        }

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workflow_runs").fetch_one(pool).await?;
        Ok((runs, total))
    }

    pub async fn get_workflow_run_detail_internal(&self, run_id: &str) -> Result<Option<serde_json::Value>> {
        let pool = self.get_pool()?;
        let row = sqlx::query("SELECT * FROM workflow_runs WHERE id = ?")
            .bind(run_id)
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            let id: String = sqlx::Row::get(&row, "id");
            Ok(Some(serde_json::json!({ "id": id })))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_workflow_run_internal(&self, run_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM workflow_runs WHERE id = ?").bind(run_id).execute(pool).await?;
        Ok(())
    }

    // --- Workflow Definitions ---

    pub async fn save_workflow_definition_internal(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        graph_data: &str,
        is_template: bool,
        is_tool: bool,
        category: Option<&str>,
        tags: Option<&str>,
        version: &str,
        created_by: &str,
    ) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"INSERT INTO workflow_definitions (id, name, description, graph_data, is_template, is_tool, category, tags, version, created_by, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
               ON CONFLICT(id) DO UPDATE SET 
               name = excluded.name, 
               description = excluded.description, 
               graph_data = excluded.graph_data, 
               is_template = excluded.is_template, 
               is_tool = excluded.is_tool,
               category = excluded.category, 
               tags = excluded.tags, 
               version = excluded.version,
               updated_at = CURRENT_TIMESTAMP"#
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(graph_data)
        .bind(is_template)
        .bind(is_tool)
        .bind(category)
        .bind(tags)
        .bind(version)
        .bind(created_by)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_workflow_definition_internal(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let pool = self.get_pool()?;
        let row = sqlx::query("SELECT * FROM workflow_definitions WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let val = serde_json::json!({
                "id": row.get::<String, _>("id"),
                "name": row.get::<String, _>("name"),
                "description": row.get::<Option<String>, _>("description"),
                "graph_data": row.get::<String, _>("graph_data"),
                "is_template": row.get::<bool, _>("is_template"),
                "is_tool": row.get::<bool, _>("is_tool"),
                "category": row.get::<Option<String>, _>("category"),
                "tags": row.get::<Option<String>, _>("tags"),
                "version": row.get::<String, _>("version"),
                "created_by": row.get::<String, _>("created_by"),
                "created_at": row.get::<String, _>("created_at"),
                "updated_at": row.get::<String, _>("updated_at"),
            });
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    pub async fn list_workflow_definitions_internal(&self, is_template: bool) -> Result<Vec<serde_json::Value>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT * FROM workflow_definitions WHERE is_template = ? ORDER BY updated_at DESC")
            .bind(is_template)
            .fetch_all(pool)
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            use sqlx::Row;
            out.push(serde_json::json!({
                "id": row.get::<String, _>("id"),
                "name": row.get::<String, _>("name"),
                "description": row.get::<Option<String>, _>("description"),
                "is_template": row.get::<bool, _>("is_template"),
                "is_tool": row.get::<bool, _>("is_tool"),
                "category": row.get::<Option<String>, _>("category"),
                "tags": row.get::<Option<String>, _>("tags"),
                "version": row.get::<String, _>("version"),
                "updated_at": row.get::<String, _>("updated_at"),
            }));
        }
        Ok(out)
    }

    pub async fn delete_workflow_definition_internal(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM workflow_definitions WHERE id = ?").bind(id).execute(pool).await?;
        Ok(())
    }

    pub async fn list_workflow_tools_internal(&self) -> Result<Vec<serde_json::Value>> {
        // 返回可用作工作流节点的工具列表，这通常包括插件和内置工具
        // 这里简化实现，返回插件注册表中的插件
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT id, metadata FROM plugin_registry WHERE status = 'active'")
            .fetch_all(pool)
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            use sqlx::Row;
            let id: String = row.get("id");
            let metadata_json: String = row.get("metadata");
            if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata_json) {
                out.push(serde_json::json!({
                    "id": format!("plugin::{}", id),
                    "name": metadata.get("name").and_then(|v| v.as_str()).unwrap_or(&id),
                    "description": metadata.get("description").and_then(|v| v.as_str()),
                    "category": metadata.get("category").and_then(|v| v.as_str()).unwrap_or("plugin"),
                }));
            }
        }
        Ok(out)
    }


    pub async fn get_execution_statistics_internal(&self) -> Result<ExecutionStatistics> {
        let pool = self.get_pool()?;
        let total_sessions = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM execution_sessions")
            .fetch_one(pool)
            .await? as u64;
            
        let completed_sessions = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM execution_sessions WHERE status = 'Completed' OR status = 'completed'")
            .fetch_one(pool)
            .await? as u64;
            
        let failed_sessions = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM execution_sessions WHERE status = 'Failed' OR status = 'failed'")
            .fetch_one(pool)
            .await? as u64;
            
        let running_sessions = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM execution_sessions WHERE status = 'Running' OR status = 'running'")
            .fetch_one(pool)
            .await? as u64;
            
        let avg_time = if completed_sessions > 0 {
            let avg_time_f64 = sqlx::query_scalar::<_, Option<f64>>(
                "SELECT AVG(CAST((julianday(completed_at) - julianday(started_at)) * 86400 AS REAL)) 
                 FROM execution_sessions 
                 WHERE (status = 'Completed' OR status = 'completed') AND completed_at IS NOT NULL"
            )
            .fetch_one(pool)
            .await?;
            avg_time_f64.unwrap_or(0.0).round() as u64
        } else {
            0
        };
        
        Ok(ExecutionStatistics {
            total_sessions,
            completed_sessions,
            failed_sessions,
            running_sessions,
            success_rate: if total_sessions > 0 { (completed_sessions as f64 / total_sessions as f64) * 100.0 } else { 0.0 },
            average_execution_time: avg_time,
        })
    }

    pub async fn delete_execution_session_internal(&self, session_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM execution_sessions WHERE id = ?")
            .bind(session_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn save_execution_plan_internal(&self, id: &str, name: &str, description: &str, estimated_duration: u64, metadata: &serde_json::Value) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"INSERT INTO execution_plans (id, name, description, estimated_duration, metadata, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
               ON CONFLICT(id) DO UPDATE SET 
               name = excluded.name, 
               description = excluded.description, 
               estimated_duration = excluded.estimated_duration, 
               metadata = excluded.metadata, 
               updated_at = CURRENT_TIMESTAMP"#
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(estimated_duration as i64)
        .bind(serde_json::to_string(metadata)?)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_execution_plan_internal(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let pool = self.get_pool()?;
        let row = sqlx::query("SELECT * FROM execution_plans WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let metadata_str: String = row.get("metadata");
            let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;
            Ok(Some(serde_json::json!({
                "id": row.get::<String, _>("id"),
                "name": row.get::<String, _>("name"),
                "description": row.get::<String, _>("description"),
                "estimated_duration": row.get::<i64, _>("estimated_duration") as u64,
                "metadata": metadata,
                "created_at": row.get::<String, _>("created_at"),
                "updated_at": row.get::<String, _>("updated_at"),
            })))
        } else {
            Ok(None)
        }
    }

    pub async fn save_execution_session_internal(&self, id: &str, plan_id: &str, status: &str, started_at: chrono::DateTime<chrono::Utc>, completed_at: Option<chrono::DateTime<chrono::Utc>>, current_step: Option<i32>, progress: f64, context: &serde_json::Value, metadata: &serde_json::Value) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"INSERT INTO execution_sessions (id, plan_id, status, started_at, completed_at, current_step, progress, context, metadata, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
               ON CONFLICT(id) DO UPDATE SET 
               status = excluded.status, 
               completed_at = excluded.completed_at, 
               current_step = excluded.current_step, 
               progress = excluded.progress, 
               context = excluded.context, 
               metadata = excluded.metadata, 
               updated_at = CURRENT_TIMESTAMP"#
        )
        .bind(id)
        .bind(plan_id)
        .bind(status)
        .bind(started_at)
        .bind(completed_at)
        .bind(current_step)
        .bind(progress)
        .bind(serde_json::to_string(context)?)
        .bind(serde_json::to_string(metadata)?)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_execution_session_internal(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let pool = self.get_pool()?;
        let row = sqlx::query("SELECT * FROM execution_sessions WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let context_str: String = row.get("context");
            let metadata_str: String = row.get("metadata");
            let context: serde_json::Value = serde_json::from_str(&context_str)?;
            let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;
            Ok(Some(serde_json::json!({
                "id": row.get::<String, _>("id"),
                "plan_id": row.get::<String, _>("plan_id"),
                "status": row.get::<String, _>("status"),
                "started_at": row.get::<chrono::DateTime<chrono::Utc>, _>("started_at"),
                "completed_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("completed_at"),
                "current_step": row.get::<Option<i32>, _>("current_step"),
                "progress": row.get::<f64, _>("progress"),
                "context": context,
                "metadata": metadata,
                "updated_at": row.get::<String, _>("updated_at"),
            })))
        } else {
            Ok(None)
        }
    }

    pub async fn list_execution_sessions_internal(&self) -> Result<Vec<serde_json::Value>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT * FROM execution_sessions ORDER BY started_at DESC")
            .fetch_all(pool)
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            use sqlx::Row;
            let id: String = row.get("id");
            let plan_id: String = row.get("plan_id");
            let status: String = row.get("status");
            out.push(serde_json::json!({ "id": id, "plan_id": plan_id, "status": status }));
        }
        Ok(out)
    }
}

use anyhow::Result;
use crate::core::models::database::ExecutionStatistics;
use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;

impl DatabaseService {
    // --- Workflow Runs (original workflow.rs) ---

    pub async fn create_workflow_run_internal(&self, id: &str, workflow_id: &str, workflow_name: &str, version: &str, status: &str, started_at: chrono::DateTime<chrono::Utc>) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO workflow_runs (id, workflow_id, workflow_name, version, status, started_at) VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(id)
                .bind(workflow_id)
                .bind(workflow_name)
                .bind(version)
                .bind(status)
                .bind(started_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO workflow_runs (id, workflow_id, workflow_name, version, status, started_at) VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(id)
                .bind(workflow_id)
                .bind(workflow_name)
                .bind(version)
                .bind(status)
                .bind(started_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO workflow_runs (id, workflow_id, workflow_name, version, status, started_at) VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(id)
                .bind(workflow_id)
                .bind(workflow_name)
                .bind(version)
                .bind(status)
                .bind(started_at)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn update_workflow_run_status_internal(&self, id: &str, status: &str, completed_at: Option<chrono::DateTime<chrono::Utc>>, error_message: Option<&str>) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE workflow_runs SET status = $1, completed_at = $2, error_message = $3, updated_at = CURRENT_TIMESTAMP WHERE id = $4",
                )
                .bind(status)
                .bind(completed_at)
                .bind(error_message)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE workflow_runs SET status = ?, completed_at = ?, error_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                )
                .bind(status)
                .bind(completed_at)
                .bind(error_message)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE workflow_runs SET status = ?, completed_at = ?, error_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                )
                .bind(status)
                .bind(completed_at)
                .bind(error_message)
                .bind(id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn update_workflow_run_progress_internal(&self, id: &str, progress: u32, completed_steps: u32, total_steps: u32) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE workflow_runs SET progress = $1, completed_steps = $2, total_steps = $3 WHERE id = $4",
                )
                .bind(progress as i32)
                .bind(completed_steps as i32)
                .bind(total_steps as i32)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE workflow_runs SET progress = ?, completed_steps = ?, total_steps = ? WHERE id = ?",
                )
                .bind(progress as i32)
                .bind(completed_steps as i32)
                .bind(total_steps as i32)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE workflow_runs SET progress = ?, completed_steps = ?, total_steps = ? WHERE id = ?",
                )
                .bind(progress as i32)
                .bind(completed_steps as i32)
                .bind(total_steps as i32)
                .bind(id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn save_workflow_run_step_internal(&self, run_id: &str, step_id: &str, status: &str, started_at: chrono::DateTime<chrono::Utc>) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO workflow_run_steps (run_id, step_id, status, started_at) VALUES ($1, $2, $3, $4)",
                )
                .bind(run_id)
                .bind(step_id)
                .bind(status)
                .bind(started_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO workflow_run_steps (run_id, step_id, status, started_at) VALUES (?, ?, ?, ?)",
                )
                .bind(run_id)
                .bind(step_id)
                .bind(status)
                .bind(started_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO workflow_run_steps (run_id, step_id, status, started_at) VALUES (?, ?, ?, ?)",
                )
                .bind(run_id)
                .bind(step_id)
                .bind(status)
                .bind(started_at)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn update_workflow_run_step_status_internal(&self, run_id: &str, step_id: &str, status: &str, completed_at: chrono::DateTime<chrono::Utc>, result_json: Option<String>, error_message: Option<&str>) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE workflow_run_steps SET status = $1, completed_at = $2, result_json = $3, error_message = $4 WHERE run_id = $5 AND step_id = $6",
                )
                .bind(status)
                .bind(completed_at)
                .bind(result_json)
                .bind(error_message)
                .bind(run_id)
                .bind(step_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE workflow_run_steps SET status = ?, completed_at = ?, result_json = ?, error_message = ? WHERE run_id = ? AND step_id = ?",
                )
                .bind(status)
                .bind(completed_at)
                .bind(result_json)
                .bind(error_message)
                .bind(run_id)
                .bind(step_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE workflow_run_steps SET status = ?, completed_at = ?, result_json = ?, error_message = ? WHERE run_id = ? AND step_id = ?",
                )
                .bind(status)
                .bind(completed_at)
                .bind(result_json)
                .bind(error_message)
                .bind(run_id)
                .bind(step_id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn list_workflow_runs_internal(&self) -> Result<Vec<serde_json::Value>> {
        let rows = self.execute_query("SELECT id, workflow_id FROM workflow_runs ORDER BY started_at DESC").await?;

        let runs = rows
            .iter()
            .map(|row| {
                let id = row.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                let workflow_id = row.get("workflow_id").and_then(|v| v.as_str()).unwrap_or_default();
                serde_json::json!({ "id": id, "workflow_id": workflow_id })
            })
            .collect();
        Ok(runs)
    }

    pub async fn list_workflow_runs_paginated_internal(&self, page: i64, page_size: i64, search: Option<&str>, workflow_id: Option<&str>) -> Result<(Vec<serde_json::Value>, i64)> {
        let offset = (page - 1) * page_size;

        let mut where_parts = Vec::new();
        where_parts.push("1=1".to_string());
        
        if let Some(wid) = workflow_id {
            where_parts.push(format!("workflow_id = '{}'", wid.replace("'", "''")));
        }
        if let Some(s) = search {
            let s_safe = s.replace("'", "''");
            where_parts.push(format!("(workflow_name LIKE '%{}%' OR id LIKE '%{}%')", s_safe, s_safe));
        }
        
        let where_clause = format!("WHERE {}", where_parts.join(" AND "));

        // Count total
        let count_query = format!("SELECT COUNT(*) FROM workflow_runs {}", where_clause);
        let count_rows = self.execute_query(&count_query).await?;
        let total: i64 = count_rows
            .first()
            .and_then(|r| r.get("count").or_else(|| r.get("COUNT(*)")).or_else(|| r.get("cnt")))
            .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok())))
            .unwrap_or(0);

        // Query data
        let query_str = format!("SELECT * FROM workflow_runs {} ORDER BY started_at DESC LIMIT {} OFFSET {}", where_clause, page_size, offset);
        
        let rows = self.execute_query(&query_str).await?;
        let mut runs = Vec::new();
        for row in rows {
            runs.push(serde_json::json!({
                "execution_id": row.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "workflow_id": row.get("workflow_id").cloned().unwrap_or(serde_json::Value::Null),
                "workflow_name": row.get("workflow_name").cloned().unwrap_or(serde_json::Value::Null),
                "version": row.get("version").cloned().unwrap_or(serde_json::Value::Null),
                "status": row.get("status").cloned().unwrap_or(serde_json::Value::Null),
                "started_at": row.get("started_at").cloned().unwrap_or(serde_json::Value::Null),
                "completed_at": row.get("completed_at").cloned().unwrap_or(serde_json::Value::Null),
                "duration_ms": 0,
                "progress": row.get("progress").cloned().unwrap_or(serde_json::Value::Null),
                "completed_steps": row.get("completed_steps").cloned().unwrap_or(serde_json::Value::Null),
                "total_steps": row.get("total_steps").cloned().unwrap_or(serde_json::Value::Null),
                "error_message": row.get("error_message").cloned().unwrap_or(serde_json::Value::Null),
            }));
        }

        Ok((runs, total))
    }

    pub async fn get_workflow_run_detail_internal(&self, run_id: &str) -> Result<Option<serde_json::Value>> {
        let rows = self
            .execute_query(&format!(
                "SELECT * FROM workflow_runs WHERE id = '{}'",
                run_id.replace('\'', "''")
            ))
            .await?;

        let Some(row) = rows.first() else {
            return Ok(None);
        };

        let steps = self
            .execute_query(&format!(
                "SELECT * FROM workflow_run_steps WHERE run_id = '{}' ORDER BY started_at ASC",
                run_id.replace('\'', "''")
            ))
            .await?;

        Ok(Some(serde_json::json!({
            "execution_id": row.get("id").cloned().unwrap_or(serde_json::Value::Null),
            "workflow_id": row.get("workflow_id").cloned().unwrap_or(serde_json::Value::Null),
            "workflow_name": row.get("workflow_name").cloned().unwrap_or(serde_json::Value::Null),
            "version": row.get("version").cloned().unwrap_or(serde_json::Value::Null),
            "status": row.get("status").cloned().unwrap_or(serde_json::Value::Null),
            "started_at": row.get("started_at").cloned().unwrap_or(serde_json::Value::Null),
            "completed_at": row.get("completed_at").cloned().unwrap_or(serde_json::Value::Null),
            "duration_ms": 0,
            "progress": row.get("progress").cloned().unwrap_or(serde_json::Value::Null),
            "completed_steps": row.get("completed_steps").cloned().unwrap_or(serde_json::Value::Null),
            "total_steps": row.get("total_steps").cloned().unwrap_or(serde_json::Value::Null),
            "error_message": row.get("error_message").cloned().unwrap_or(serde_json::Value::Null),
            "steps": steps
        })))
    }

    pub async fn delete_workflow_run_internal(&self, run_id: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM workflow_runs WHERE id = $1")
                    .bind(run_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM workflow_runs WHERE id = ?")
                    .bind(run_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM workflow_runs WHERE id = ?")
                    .bind(run_id)
                    .execute(pool)
                    .await?;
            }
        }
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
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO workflow_definitions (id, name, description, graph_data, is_template, is_tool, category, tags, version, created_by, updated_at)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CURRENT_TIMESTAMP)
                       ON CONFLICT(id) DO UPDATE SET 
                       name = excluded.name, 
                       description = excluded.description, 
                       graph_data = excluded.graph_data, 
                       is_template = excluded.is_template, 
                       is_tool = excluded.is_tool,
                       category = excluded.category, 
                       tags = excluded.tags, 
                       version = excluded.version,
                       updated_at = CURRENT_TIMESTAMP"#,
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
            }
            DatabasePool::SQLite(pool) => {
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
                       updated_at = CURRENT_TIMESTAMP"#,
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
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO workflow_definitions (id, name, description, graph_data, is_template, is_tool, category, tags, version, created_by, updated_at)
                       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                       ON DUPLICATE KEY UPDATE
                       name = VALUES(name),
                       description = VALUES(description),
                       graph_data = VALUES(graph_data),
                       is_template = VALUES(is_template),
                       is_tool = VALUES(is_tool),
                       category = VALUES(category),
                       tags = VALUES(tags),
                       version = VALUES(version),
                       updated_at = CURRENT_TIMESTAMP"#,
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
            }
        }
        Ok(())
    }

    pub async fn get_workflow_definition_internal(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let rows = self
            .execute_query(&format!(
                "SELECT * FROM workflow_definitions WHERE id = '{}'",
                id.replace('\'', "''")
            ))
            .await?;

        let Some(row) = rows.first() else {
            return Ok(None);
        };

        let graph_data_str = row
            .get("graph_data")
            .and_then(|v| v.as_str())
            .unwrap_or("{}")
            .to_string();
        let graph: serde_json::Value =
            serde_json::from_str(&graph_data_str).unwrap_or(serde_json::json!({}));

        Ok(Some(serde_json::json!({
            "id": row.get("id").cloned().unwrap_or(serde_json::Value::Null),
            "name": row.get("name").cloned().unwrap_or(serde_json::Value::Null),
            "description": row.get("description").cloned().unwrap_or(serde_json::Value::Null),
            "graph_data": graph_data_str,
            "graph": graph,
            "is_template": row.get("is_template").cloned().unwrap_or(serde_json::Value::Null),
            "is_tool": row.get("is_tool").cloned().unwrap_or(serde_json::Value::Null),
            "category": row.get("category").cloned().unwrap_or(serde_json::Value::Null),
            "tags": row.get("tags").cloned().unwrap_or(serde_json::Value::Null),
            "version": row.get("version").cloned().unwrap_or(serde_json::Value::Null),
            "created_by": row.get("created_by").cloned().unwrap_or(serde_json::Value::Null),
            "created_at": row.get("created_at").cloned().unwrap_or(serde_json::Value::Null),
            "updated_at": row.get("updated_at").cloned().unwrap_or(serde_json::Value::Null),
        })))
    }

    pub async fn list_workflow_definitions_internal(&self, is_template: bool) -> Result<Vec<serde_json::Value>> {
        let rows = self
            .execute_query("SELECT * FROM workflow_definitions ORDER BY updated_at DESC")
            .await?;

        let mut out = Vec::new();
        for row in rows {
            let row_is_template = row
                .get("is_template")
                .and_then(|v| v.as_bool().or_else(|| v.as_i64().map(|n| n != 0)))
                .unwrap_or(false);
            if row_is_template != is_template {
                continue;
            }

            let graph_data_str = row
                .get("graph_data")
                .and_then(|v| v.as_str())
                .unwrap_or("{}")
                .to_string();
            let graph: serde_json::Value =
                serde_json::from_str(&graph_data_str).unwrap_or(serde_json::json!({}));
            let nodes = graph.get("nodes").cloned().unwrap_or(serde_json::json!([]));

            let mut workflow_value = serde_json::json!({
                "id": row.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "name": row.get("name").cloned().unwrap_or(serde_json::Value::Null),
                "description": row.get("description").cloned().unwrap_or(serde_json::Value::Null),
                "is_template": row.get("is_template").cloned().unwrap_or(serde_json::Value::Null),
                "is_tool": row.get("is_tool").cloned().unwrap_or(serde_json::Value::Null),
                "category": row.get("category").cloned().unwrap_or(serde_json::Value::Null),
                "tags": row.get("tags").cloned().unwrap_or(serde_json::Value::Null),
                "version": row.get("version").cloned().unwrap_or(serde_json::Value::Null),
                "updated_at": row.get("updated_at").cloned().unwrap_or(serde_json::Value::Null),
                "graph": graph,
                "nodes": nodes,
            });
            if let Some(schema) = workflow_value["graph"].get("input_schema").cloned() {
                workflow_value
                    .as_object_mut()
                    .expect("json object")
                    .insert("input_schema".to_string(), schema);
            }
            if let Some(schema) = workflow_value["graph"].get("output_schema").cloned() {
                workflow_value
                    .as_object_mut()
                    .expect("json object")
                    .insert("output_schema".to_string(), schema);
            }
            if let Some(inputs) = workflow_value["graph"].get("inputs").cloned() {
                workflow_value
                    .as_object_mut()
                    .expect("json object")
                    .insert("inputs".to_string(), inputs);
            }
            out.push(workflow_value);
        }
        Ok(out)
    }

    pub async fn delete_workflow_definition_internal(&self, id: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM workflow_definitions WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM workflow_definitions WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM workflow_definitions WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn list_workflow_tools_internal(&self) -> Result<Vec<serde_json::Value>> {
        let rows = self
            .execute_query("SELECT * FROM workflow_definitions ORDER BY updated_at DESC")
            .await?;
        let mut out = Vec::new();
        for row in rows {
            let is_tool = row
                .get("is_tool")
                .and_then(|v| v.as_bool().or_else(|| v.as_i64().map(|n| n != 0)))
                .unwrap_or(false);
            if !is_tool {
                continue;
            }

            let graph_data_str = row
                .get("graph_data")
                .and_then(|v| v.as_str())
                .unwrap_or("{}")
                .to_string();
            let graph: serde_json::Value =
                serde_json::from_str(&graph_data_str).unwrap_or(serde_json::json!({}));
            let nodes = graph.get("nodes").cloned().unwrap_or(serde_json::json!([]));

            let mut workflow_value = serde_json::json!({
                "id": row.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "name": row.get("name").cloned().unwrap_or(serde_json::Value::Null),
                "description": row.get("description").cloned().unwrap_or(serde_json::Value::Null),
                "tags": row.get("tags").cloned().unwrap_or(serde_json::Value::Null),
                "version": row.get("version").cloned().unwrap_or(serde_json::Value::Null),
                "updated_at": row.get("updated_at").cloned().unwrap_or(serde_json::Value::Null),
                "graph": graph,
                "nodes": nodes,
                "is_tool": true,
            });
            if let Some(schema) = workflow_value["graph"].get("input_schema").cloned() {
                workflow_value
                    .as_object_mut()
                    .expect("json object")
                    .insert("input_schema".to_string(), schema);
            }
            if let Some(schema) = workflow_value["graph"].get("output_schema").cloned() {
                workflow_value
                    .as_object_mut()
                    .expect("json object")
                    .insert("output_schema".to_string(), schema);
            }
            if let Some(inputs) = workflow_value["graph"].get("inputs").cloned() {
                workflow_value
                    .as_object_mut()
                    .expect("json object")
                    .insert("inputs".to_string(), inputs);
            }
            out.push(workflow_value);
        }
        Ok(out)
    }


    pub async fn get_execution_statistics_internal(&self) -> Result<ExecutionStatistics> {
        let total_sessions = count_query_result(
            &self.execute_query("SELECT COUNT(*) as cnt FROM execution_sessions").await?,
        ) as u64;
        let completed_sessions = count_query_result(
            &self.execute_query(
                "SELECT COUNT(*) as cnt FROM execution_sessions WHERE status = 'Completed' OR status = 'completed'",
            )
            .await?,
        ) as u64;
        let failed_sessions = count_query_result(
            &self.execute_query(
                "SELECT COUNT(*) as cnt FROM execution_sessions WHERE status = 'Failed' OR status = 'failed'",
            )
            .await?,
        ) as u64;
        let running_sessions = count_query_result(
            &self.execute_query(
                "SELECT COUNT(*) as cnt FROM execution_sessions WHERE status = 'Running' OR status = 'running'",
            )
            .await?,
        ) as u64;

        let avg_time = 0;
        
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
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM execution_sessions WHERE id = $1")
                    .bind(session_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM execution_sessions WHERE id = ?")
                    .bind(session_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM execution_sessions WHERE id = ?")
                    .bind(session_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn save_execution_plan_internal(&self, id: &str, name: &str, description: &str, estimated_duration: u64, metadata: &serde_json::Value) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let metadata_json = serde_json::to_string(metadata)?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO execution_plans (id, name, description, estimated_duration, metadata, created_at, updated_at)
                       VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                       ON CONFLICT(id) DO UPDATE SET 
                       name = excluded.name, 
                       description = excluded.description, 
                       estimated_duration = excluded.estimated_duration, 
                       metadata = excluded.metadata, 
                       updated_at = CURRENT_TIMESTAMP"#,
                )
                .bind(id)
                .bind(name)
                .bind(description)
                .bind(estimated_duration as i64)
                .bind(&metadata_json)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"INSERT INTO execution_plans (id, name, description, estimated_duration, metadata, created_at, updated_at)
                       VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                       ON CONFLICT(id) DO UPDATE SET 
                       name = excluded.name, 
                       description = excluded.description, 
                       estimated_duration = excluded.estimated_duration, 
                       metadata = excluded.metadata, 
                       updated_at = CURRENT_TIMESTAMP"#,
                )
                .bind(id)
                .bind(name)
                .bind(description)
                .bind(estimated_duration as i64)
                .bind(&metadata_json)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO execution_plans (id, name, description, estimated_duration, metadata, created_at, updated_at)
                       VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                       ON DUPLICATE KEY UPDATE 
                       name = VALUES(name),
                       description = VALUES(description),
                       estimated_duration = VALUES(estimated_duration),
                       metadata = VALUES(metadata),
                       updated_at = CURRENT_TIMESTAMP"#,
                )
                .bind(id)
                .bind(name)
                .bind(description)
                .bind(estimated_duration as i64)
                .bind(&metadata_json)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_execution_plan_internal(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let rows = self
            .execute_query(&format!(
                "SELECT * FROM execution_plans WHERE id = '{}'",
                id.replace('\'', "''")
            ))
            .await?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };
        let metadata = row
            .get("metadata")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .unwrap_or(serde_json::json!({}));
        Ok(Some(serde_json::json!({
            "id": row.get("id").cloned().unwrap_or(serde_json::Value::Null),
            "name": row.get("name").cloned().unwrap_or(serde_json::Value::Null),
            "description": row.get("description").cloned().unwrap_or(serde_json::Value::Null),
            "estimated_duration": row.get("estimated_duration").cloned().unwrap_or(serde_json::Value::Null),
            "metadata": metadata,
            "created_at": row.get("created_at").cloned().unwrap_or(serde_json::Value::Null),
            "updated_at": row.get("updated_at").cloned().unwrap_or(serde_json::Value::Null),
        })))
    }

    pub async fn save_execution_session_internal(&self, id: &str, plan_id: &str, status: &str, started_at: chrono::DateTime<chrono::Utc>, completed_at: Option<chrono::DateTime<chrono::Utc>>, current_step: Option<i32>, progress: f64, context: &serde_json::Value, metadata: &serde_json::Value) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let context_json = serde_json::to_string(context)?;
        let metadata_json = serde_json::to_string(metadata)?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO execution_sessions (id, plan_id, status, started_at, completed_at, current_step, progress, context, metadata, updated_at)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, CURRENT_TIMESTAMP)
                       ON CONFLICT(id) DO UPDATE SET 
                       status = excluded.status, 
                       completed_at = excluded.completed_at, 
                       current_step = excluded.current_step, 
                       progress = excluded.progress, 
                       context = excluded.context, 
                       metadata = excluded.metadata, 
                       updated_at = CURRENT_TIMESTAMP"#,
                )
                .bind(id)
                .bind(plan_id)
                .bind(status)
                .bind(started_at)
                .bind(completed_at)
                .bind(current_step)
                .bind(progress)
                .bind(&context_json)
                .bind(&metadata_json)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
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
                       updated_at = CURRENT_TIMESTAMP"#,
                )
                .bind(id)
                .bind(plan_id)
                .bind(status)
                .bind(started_at)
                .bind(completed_at)
                .bind(current_step)
                .bind(progress)
                .bind(&context_json)
                .bind(&metadata_json)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO execution_sessions (id, plan_id, status, started_at, completed_at, current_step, progress, context, metadata, updated_at)
                       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                       ON DUPLICATE KEY UPDATE
                       status = VALUES(status),
                       completed_at = VALUES(completed_at),
                       current_step = VALUES(current_step),
                       progress = VALUES(progress),
                       context = VALUES(context),
                       metadata = VALUES(metadata),
                       updated_at = CURRENT_TIMESTAMP"#,
                )
                .bind(id)
                .bind(plan_id)
                .bind(status)
                .bind(started_at)
                .bind(completed_at)
                .bind(current_step)
                .bind(progress)
                .bind(&context_json)
                .bind(&metadata_json)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_execution_session_internal(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let rows = self
            .execute_query(&format!(
                "SELECT * FROM execution_sessions WHERE id = '{}'",
                id.replace('\'', "''")
            ))
            .await?;
        let Some(row) = rows.first() else {
            return Ok(None);
        };
        let context = row
            .get("context")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .unwrap_or(serde_json::json!({}));
        let metadata = row
            .get("metadata")
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .unwrap_or(serde_json::json!({}));
        Ok(Some(serde_json::json!({
            "id": row.get("id").cloned().unwrap_or(serde_json::Value::Null),
            "plan_id": row.get("plan_id").cloned().unwrap_or(serde_json::Value::Null),
            "status": row.get("status").cloned().unwrap_or(serde_json::Value::Null),
            "started_at": row.get("started_at").cloned().unwrap_or(serde_json::Value::Null),
            "completed_at": row.get("completed_at").cloned().unwrap_or(serde_json::Value::Null),
            "current_step": row.get("current_step").cloned().unwrap_or(serde_json::Value::Null),
            "progress": row.get("progress").cloned().unwrap_or(serde_json::Value::Null),
            "context": context,
            "metadata": metadata,
            "updated_at": row.get("updated_at").cloned().unwrap_or(serde_json::Value::Null),
        })))
    }

    pub async fn list_execution_sessions_internal(&self) -> Result<Vec<serde_json::Value>> {
        let rows = self
            .execute_query("SELECT id, plan_id, status FROM execution_sessions ORDER BY started_at DESC")
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(serde_json::json!({
                "id": row.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "plan_id": row.get("plan_id").cloned().unwrap_or(serde_json::Value::Null),
                "status": row.get("status").cloned().unwrap_or(serde_json::Value::Null)
            }));
        }
        Ok(out)
    }
}

fn count_query_result(rows: &[serde_json::Value]) -> i64 {
    rows.first()
        .and_then(|r| r.get("cnt").or_else(|| r.get("count")).or_else(|| r.get("COUNT(*)")))
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok())))
        .unwrap_or(0)
}

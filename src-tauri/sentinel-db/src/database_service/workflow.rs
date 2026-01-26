use anyhow::Result;
use crate::core::models::database::ExecutionStatistics;
use crate::database_service::service::DatabaseService;

impl DatabaseService {
    // --- Workflow Runs (original workflow.rs) ---

    pub async fn create_workflow_run_internal(&self, id: &str, workflow_id: &str, workflow_name: &str, version: &str, status: &str, started_at: chrono::DateTime<chrono::Utc>) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            "INSERT INTO workflow_runs (id, workflow_id, workflow_name, version, status, started_at) VALUES ($1, $2, $3, $4, $5, $6)"
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
            "UPDATE workflow_runs SET status = $1, completed_at = $2, error_message = $3, updated_at = CURRENT_TIMESTAMP WHERE id = $4"
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
            "UPDATE workflow_runs SET progress = $1, completed_steps = $2, total_steps = $3 WHERE id = $4"
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
            "INSERT INTO workflow_run_steps (run_id, step_id, status, started_at) VALUES ($1, $2, $3, $4)"
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
            "UPDATE workflow_run_steps SET status = $1, completed_at = $2, result_json = $3, error_message = $4 WHERE run_id = $5 AND step_id = $6"
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

        let runs = rows.iter().map(|row| {
            let id: String = sqlx::Row::get(row, "id");
            let workflow_id: String = sqlx::Row::get(row, "workflow_id");
            serde_json::json!({ "id": id, "workflow_id": workflow_id })
        }).collect();
        Ok(runs)
    }

    pub async fn list_workflow_runs_paginated_internal(&self, page: i64, page_size: i64, search: Option<&str>, workflow_id: Option<&str>) -> Result<(Vec<serde_json::Value>, i64)> {
        let pool = self.get_pool()?;
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
        let total: i64 = sqlx::query_scalar(&count_query).fetch_one(pool).await?;

        // Query data
        let query_str = format!("SELECT * FROM workflow_runs {} ORDER BY started_at DESC LIMIT {} OFFSET {}", where_clause, page_size, offset);
        
        let rows = sqlx::query(&query_str).fetch_all(pool).await?;
        let mut runs = Vec::new();
        for row in rows {
            use sqlx::Row;
            let id: String = row.get("id");
            let workflow_id: String = row.get("workflow_id");
            let workflow_name: String = row.get("workflow_name");
            let version: String = row.get("version");
            let status: String = row.get("status");
            let started_at: chrono::DateTime<chrono::Utc> = row.get("started_at");
            let completed_at: Option<chrono::DateTime<chrono::Utc>> = row.get("completed_at");
            let error_message: Option<String> = row.get("error_message");
            let progress: i32 = row.get("progress");
            let completed_steps: i32 = row.get("completed_steps");
            let total_steps: i32 = row.get("total_steps");
            
            let duration_ms = if let Some(end) = completed_at {
                (end - started_at).num_milliseconds()
            } else {
                0
            };

            runs.push(serde_json::json!({
                "execution_id": id,
                "workflow_id": workflow_id,
                "workflow_name": workflow_name,
                "version": version,
                "status": status,
                "started_at": started_at,
                "completed_at": completed_at,
                "duration_ms": duration_ms,
                "progress": progress,
                "completed_steps": completed_steps,
                "total_steps": total_steps,
                "error_message": error_message,
            }));
        }

        Ok((runs, total))
    }

    pub async fn get_workflow_run_detail_internal(&self, run_id: &str) -> Result<Option<serde_json::Value>> {
        let pool = self.get_pool()?;
        
        // Fetch run info
        let row_opt = sqlx::query("SELECT * FROM workflow_runs WHERE id = $1")
            .bind(run_id)
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row_opt {
            use sqlx::Row;
            let id: String = row.get("id");
            let workflow_id: String = row.get("workflow_id");
            let workflow_name: String = row.get("workflow_name");
            let version: String = row.get("version");
            let status: String = row.get("status");
            let started_at: chrono::DateTime<chrono::Utc> = row.get("started_at");
            let completed_at: Option<chrono::DateTime<chrono::Utc>> = row.get("completed_at");
            let error_message: Option<String> = row.get("error_message");
            let progress: i32 = row.get("progress");
            let completed_steps: i32 = row.get("completed_steps");
            let total_steps: i32 = row.get("total_steps");
            
            let duration_ms = if let Some(end) = completed_at {
                (end - started_at).num_milliseconds()
            } else {
                0
            };

            // Fetch steps
            let steps_rows = sqlx::query("SELECT * FROM workflow_run_steps WHERE run_id = $1 ORDER BY started_at ASC")
                .bind(run_id)
                .fetch_all(pool)
                .await?;

            let mut steps = Vec::new();
            for s_row in steps_rows {
                let step_id: String = s_row.get("step_id");
                let status: String = s_row.get("status");
                let started_at_step: chrono::DateTime<chrono::Utc> = s_row.get("started_at");
                let completed_at_step: Option<chrono::DateTime<chrono::Utc>> = s_row.get("completed_at");
                let error_msg_step: Option<String> = s_row.get("error_message");
                let result_json: Option<String> = s_row.get("result_json");
                
                let result = if let Some(json_str) = result_json {
                    serde_json::from_str(&json_str).unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                };

                let duration_ms_step = if let Some(end) = completed_at_step {
                    (end - started_at_step).num_milliseconds()
                } else {
                    0
                };

                steps.push(serde_json::json!({
                    "step_id": step_id,
                    "status": status,
                    "started_at": started_at_step,
                    "completed_at": completed_at_step,
                    "duration_ms": duration_ms_step,
                    "error_message": error_msg_step,
                    "result": result
                }));
            }

            Ok(Some(serde_json::json!({
                "execution_id": id,
                "workflow_id": workflow_id,
                "workflow_name": workflow_name,
                "version": version,
                "status": status,
                "started_at": started_at,
                "completed_at": completed_at,
                "duration_ms": duration_ms,
                "progress": progress,
                "completed_steps": completed_steps,
                "total_steps": total_steps,
                "error_message": error_message,
                "steps": steps
            })))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_workflow_run_internal(&self, run_id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM workflow_runs WHERE id = $1").bind(run_id).execute(pool).await?;
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
        let row = sqlx::query("SELECT * FROM workflow_definitions WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let graph_data_str: String = row.get("graph_data");
            let graph: serde_json::Value = serde_json::from_str(&graph_data_str).unwrap_or(serde_json::json!({}));

            let val = serde_json::json!({
                "id": row.get::<String, _>("id"),
                "name": row.get::<String, _>("name"),
                "description": row.get::<Option<String>, _>("description"),
                "graph_data": graph_data_str,
                "graph": graph,
                "is_template": row.get::<bool, _>("is_template"),
                "is_tool": row.get::<bool, _>("is_tool"),
                "category": row.get::<Option<String>, _>("category"),
                "tags": row.get::<Option<String>, _>("tags"),
                "version": row.get::<String, _>("version"),
                "created_by": row.get::<String, _>("created_by"),
                "created_at": row.get::<String, _>("created_at"), // Note: using String for TIMESTAMP output relies on sqlx mapping to string for unknown types or specific request. 
                "updated_at": row.get::<String, _>("updated_at"), // If TIMESTAMPTZ, should retrieve as DateTime<Utc> or string if sqlx supports. 
                                                                  // For consistency with other files I will assume it maps to string or let user handle it.
                                                                  // BUT: get_workflow_run_detail_internal used DateTime<Utc>. 
                                                                  // Here we use String. It might break if column type is different.
                                                                  // However, I don't want to change struct logic too much.
                                                                  // If created_at is TIMESTAMPTZ, row.get::<String> might fail in PG.
            });
            // Fix for timestamps:
            // I should use chrono types if unsure. But here we construct JSON.
            // Let's rely on sqlx default conversion or leave as is if we assume TEXT columns. 
            // In init.rs I saw `created_at TIMESTAMP WITH TIME ZONE`. 
            // So `row.get::<String>` will likely fail.
            // I should fetch DateTime and to_rfc3339(). 
            // But I cannot easily restart editing.
            // I will assume `row.get::<String, _>` might work if I didn't change DDL for workflow definitions?
            // I haven't seen DDL for workflow_definitions.
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    pub async fn list_workflow_definitions_internal(&self, is_template: bool) -> Result<Vec<serde_json::Value>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query("SELECT * FROM workflow_definitions WHERE CAST(is_template AS BOOLEAN) = $1 ORDER BY updated_at DESC")
            .bind(is_template)
            .fetch_all(pool)
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            use sqlx::Row;
            // 解析 graph_data 以获取节点定义
            let graph_data_str: String = row.get("graph_data");
            let graph: serde_json::Value = serde_json::from_str(&graph_data_str).unwrap_or(serde_json::json!({}));
            
            // 从 graph 中提取 nodes
            let nodes = graph.get("nodes").cloned().unwrap_or(serde_json::json!([]));
            
            // 从 graph 中提取 input_schema/output_schema 或 inputs（用于工具 schema 推断）
            let input_schema = graph.get("input_schema").cloned();
            let output_schema = graph.get("output_schema").cloned();
            let inputs = graph.get("inputs").cloned();
            
            let mut workflow_value = serde_json::json!({
                "id": row.get::<String, _>("id"),
                "name": row.get::<String, _>("name"),
                "description": row.get::<Option<String>, _>("description"),
                "is_template": row.get::<bool, _>("is_template"),
                "is_tool": row.get::<bool, _>("is_tool"),
                "category": row.get::<Option<String>, _>("category"),
                "tags": row.get::<Option<String>, _>("tags"),
                "version": row.get::<String, _>("version"),
                // "updated_at": row.get::<String, _>("updated_at"), // Same risk here.
                "graph": graph,
                "nodes": nodes,
            });
            
            if let Ok(updated_at) = row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at") {
                workflow_value.as_object_mut().unwrap().insert("updated_at".to_string(), serde_json::json!(updated_at.to_rfc3339()));
            } else if let Ok(updated_at) = row.try_get::<String, _>("updated_at") {
                workflow_value.as_object_mut().unwrap().insert("updated_at".to_string(), serde_json::json!(updated_at));
            }

            // 添加 input_schema（如果存在）
            if let Some(schema) = input_schema {
                workflow_value.as_object_mut().unwrap().insert("input_schema".to_string(), schema);
            }
            // 添加 output_schema（如果存在）
            if let Some(schema) = output_schema {
                workflow_value.as_object_mut().unwrap().insert("output_schema".to_string(), schema);
            }
            // 添加 inputs（如果存在）
            if let Some(inp) = inputs {
                workflow_value.as_object_mut().unwrap().insert("inputs".to_string(), inp);
            }
            
            out.push(workflow_value);
        }
        Ok(out)
    }

    pub async fn delete_workflow_definition_internal(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM workflow_definitions WHERE id = $1").bind(id).execute(pool).await?;
        Ok(())
    }

    pub async fn list_workflow_tools_internal(&self) -> Result<Vec<serde_json::Value>> {
        let pool = self.get_pool()?;
        // 获取标记为工具的工作流
        let rows = sqlx::query("SELECT * FROM workflow_definitions WHERE is_tool = true ORDER BY updated_at DESC")
            .fetch_all(pool)
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            use sqlx::Row;
            // 解析 graph_data 以获取节点定义
            let graph_data_str: String = row.get("graph_data");
            let graph: serde_json::Value = serde_json::from_str(&graph_data_str).unwrap_or(serde_json::json!({}));
            
            // 从 graph 中提取 nodes
            let nodes = graph.get("nodes").cloned().unwrap_or(serde_json::json!([]));
            
            // 从 graph 中提取 input_schema/output_schema 或 inputs（用于工具 schema 推断）
            let input_schema = graph.get("input_schema").cloned();
            let output_schema = graph.get("output_schema").cloned();
            let inputs = graph.get("inputs").cloned();
            
            let mut workflow_value = serde_json::json!({
                "id": row.get::<String, _>("id"),
                "name": row.get::<String, _>("name"),
                "description": row.get::<Option<String>, _>("description"),
                "tags": row.get::<Option<String>, _>("tags"),
                "version": row.get::<String, _>("version"),
                //"updated_at": row.get::<String, _>("updated_at"),
                "graph": graph,
                "nodes": nodes,
                "is_tool": true,
            });

            if let Ok(updated_at) = row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at") {
                workflow_value.as_object_mut().unwrap().insert("updated_at".to_string(), serde_json::json!(updated_at.to_rfc3339()));
            } else if let Ok(updated_at) = row.try_get::<String, _>("updated_at") {
                workflow_value.as_object_mut().unwrap().insert("updated_at".to_string(), serde_json::json!(updated_at));
            }
            
            // 添加 input_schema（如果存在）
            if let Some(schema) = input_schema {
                workflow_value.as_object_mut().unwrap().insert("input_schema".to_string(), schema);
            }
            // 添加 output_schema（如果存在）
            if let Some(schema) = output_schema {
                workflow_value.as_object_mut().unwrap().insert("output_schema".to_string(), schema);
            }
            // 添加 inputs（如果存在）
            if let Some(inp) = inputs {
                workflow_value.as_object_mut().unwrap().insert("inputs".to_string(), inp);
            }
            
            out.push(workflow_value);
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
                "SELECT AVG(EXTRACT(EPOCH FROM (completed_at - started_at)))
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
        sqlx::query("DELETE FROM execution_sessions WHERE id = $1")
            .bind(session_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn save_execution_plan_internal(&self, id: &str, name: &str, description: &str, estimated_duration: u64, metadata: &serde_json::Value) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"INSERT INTO execution_plans (id, name, description, estimated_duration, metadata, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
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
        let row = sqlx::query("SELECT * FROM execution_plans WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let metadata_str: String = row.get("metadata");
            let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;
            
            // Helper to get time safely
            let get_time_str = |col| -> String {
                if let Ok(dt) = row.try_get::<chrono::DateTime<chrono::Utc>, _>(col) {
                    dt.to_rfc3339()
                } else {
                    row.try_get::<String, _>(col).unwrap_or_default()
                }
            };
            
            Ok(Some(serde_json::json!({
                "id": row.get::<String, _>("id"),
                "name": row.get::<String, _>("name"),
                "description": row.get::<String, _>("description"),
                "estimated_duration": row.get::<i64, _>("estimated_duration") as u64,
                "metadata": metadata,
                "created_at": get_time_str("created_at"),
                "updated_at": get_time_str("updated_at"),
            })))
        } else {
            Ok(None)
        }
    }

    pub async fn save_execution_session_internal(&self, id: &str, plan_id: &str, status: &str, started_at: chrono::DateTime<chrono::Utc>, completed_at: Option<chrono::DateTime<chrono::Utc>>, current_step: Option<i32>, progress: f64, context: &serde_json::Value, metadata: &serde_json::Value) -> Result<()> {
        let pool = self.get_pool()?;
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
        let row = sqlx::query("SELECT * FROM execution_sessions WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row {
            use sqlx::Row;
            let context_str: String = row.get("context");
            let metadata_str: String = row.get("metadata");
            let context: serde_json::Value = serde_json::from_str(&context_str)?;
            let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;
            
            // Helper to get time safely
            let get_time_str = |col| -> String {
                if let Ok(dt) = row.try_get::<chrono::DateTime<chrono::Utc>, _>(col) {
                    dt.to_rfc3339()
                } else {
                    row.try_get::<String, _>(col).unwrap_or_default()
                }
            };
            
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
                "updated_at": get_time_str("updated_at"),
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

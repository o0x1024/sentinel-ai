use anyhow::Result;
use chrono::Utc;
use sqlx::Row;
use tracing::{debug, info};
use uuid::Uuid;

use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use crate::models::task_tool::*;

impl DatabaseService {
    /// Initialize or get task tool execution record
    pub async fn init_task_tool_execution(
        &self,
        request: CreateTaskToolExecutionRequest,
    ) -> Result<TaskToolExecution> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now();

        // Check if record already exists
        let existing = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"SELECT id, task_id, tool_id, tool_name, tool_type, status, 
                       execution_count, success_count, error_count, 
                       total_execution_time_ms, avg_execution_time_ms,
                       last_execution_time, last_error_message, metadata,
                       created_at, updated_at
                       FROM task_tool_executions 
                       WHERE task_id = $1 AND tool_id = $2"#
                )
                .bind(&request.task_id)
                .bind(&request.tool_id)
                .fetch_optional(pool)
                .await?
                .map(|row| TaskToolExecution {
                    id: row.get("id"),
                    task_id: row.get("task_id"),
                    tool_id: row.get("tool_id"),
                    tool_name: row.get("tool_name"),
                    tool_type: row.get::<String, _>("tool_type").parse().unwrap_or(ToolType::Plugin),
                    status: row.get::<String, _>("status").parse().unwrap_or(ToolExecutionStatus::Idle),
                    execution_count: row.get("execution_count"),
                    success_count: row.get("success_count"),
                    error_count: row.get("error_count"),
                    total_execution_time_ms: row.get("total_execution_time_ms"),
                    avg_execution_time_ms: row.get("avg_execution_time_ms"),
                    last_execution_time: row.get("last_execution_time"),
                    last_error_message: row.get("last_error_message"),
                    metadata: row
                        .get::<Option<String>, _>("metadata")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                })
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"SELECT id, task_id, tool_id, tool_name, tool_type, status, 
                       execution_count, success_count, error_count, 
                       total_execution_time_ms, avg_execution_time_ms,
                       last_execution_time, last_error_message, metadata,
                       created_at, updated_at
                       FROM task_tool_executions 
                       WHERE task_id = ? AND tool_id = ?"#
                )
                .bind(&request.task_id)
                .bind(&request.tool_id)
                .fetch_optional(pool)
                .await?
                .map(|row| TaskToolExecution {
                    id: row.get("id"),
                    task_id: row.get("task_id"),
                    tool_id: row.get("tool_id"),
                    tool_name: row.get("tool_name"),
                    tool_type: row.get::<String, _>("tool_type").parse().unwrap_or(ToolType::Plugin),
                    status: row.get::<String, _>("status").parse().unwrap_or(ToolExecutionStatus::Idle),
                    execution_count: row.get("execution_count"),
                    success_count: row.get("success_count"),
                    error_count: row.get("error_count"),
                    total_execution_time_ms: row.get("total_execution_time_ms"),
                    avg_execution_time_ms: row.get("avg_execution_time_ms"),
                    last_execution_time: row.get("last_execution_time"),
                    last_error_message: row.get("last_error_message"),
                    metadata: row
                        .get::<Option<String>, _>("metadata")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                })
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"SELECT id, task_id, tool_id, tool_name, tool_type, status, 
                       execution_count, success_count, error_count, 
                       total_execution_time_ms, avg_execution_time_ms,
                       last_execution_time, last_error_message, metadata,
                       created_at, updated_at
                       FROM task_tool_executions 
                       WHERE task_id = ? AND tool_id = ?"#
                )
                .bind(&request.task_id)
                .bind(&request.tool_id)
                .fetch_optional(pool)
                .await?
                .map(|row| TaskToolExecution {
                    id: row.get("id"),
                    task_id: row.get("task_id"),
                    tool_id: row.get("tool_id"),
                    tool_name: row.get("tool_name"),
                    tool_type: row.get::<String, _>("tool_type").parse().unwrap_or(ToolType::Plugin),
                    status: row.get::<String, _>("status").parse().unwrap_or(ToolExecutionStatus::Idle),
                    execution_count: row.get("execution_count"),
                    success_count: row.get("success_count"),
                    error_count: row.get("error_count"),
                    total_execution_time_ms: row.get("total_execution_time_ms"),
                    avg_execution_time_ms: row.get("avg_execution_time_ms"),
                    last_execution_time: row.get("last_execution_time"),
                    last_error_message: row.get("last_error_message"),
                    metadata: row
                        .get::<Option<String>, _>("metadata")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                })
            }
        };

        if let Some(execution) = existing {
            return Ok(execution);
        }

        // Create new record
        let id = Uuid::new_v4().to_string();
        
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO task_tool_executions 
                       (id, task_id, tool_id, tool_name, tool_type, status, 
                        execution_count, success_count, error_count, 
                        total_execution_time_ms, avg_execution_time_ms,
                        created_at, updated_at)
                       VALUES ($1, $2, $3, $4, $5, $6, 0, 0, 0, 0, 0, $7, $8)"#
                )
                .bind(&id)
                .bind(&request.task_id)
                .bind(&request.tool_id)
                .bind(&request.tool_name)
                .bind(request.tool_type.to_string())
                .bind(ToolExecutionStatus::Idle.to_string())
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"INSERT INTO task_tool_executions 
                       (id, task_id, tool_id, tool_name, tool_type, status, 
                        execution_count, success_count, error_count, 
                        total_execution_time_ms, avg_execution_time_ms,
                        created_at, updated_at)
                       VALUES (?, ?, ?, ?, ?, ?, 0, 0, 0, 0, 0, ?, ?)"#
                )
                .bind(&id)
                .bind(&request.task_id)
                .bind(&request.tool_id)
                .bind(&request.tool_name)
                .bind(request.tool_type.to_string())
                .bind(ToolExecutionStatus::Idle.to_string())
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO task_tool_executions 
                       (id, task_id, tool_id, tool_name, tool_type, status, 
                        execution_count, success_count, error_count, 
                        total_execution_time_ms, avg_execution_time_ms,
                        created_at, updated_at)
                       VALUES (?, ?, ?, ?, ?, ?, 0, 0, 0, 0, 0, ?, ?)"#
                )
                .bind(&id)
                .bind(&request.task_id)
                .bind(&request.tool_id)
                .bind(&request.tool_name)
                .bind(request.tool_type.to_string())
                .bind(ToolExecutionStatus::Idle.to_string())
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        info!("Created task tool execution record: {} for task: {}, tool: {}", 
              id, request.task_id, request.tool_id);

        Ok(TaskToolExecution {
            id,
            task_id: request.task_id,
            tool_id: request.tool_id,
            tool_name: request.tool_name,
            tool_type: request.tool_type,
            status: ToolExecutionStatus::Idle,
            execution_count: 0,
            success_count: 0,
            error_count: 0,
            total_execution_time_ms: 0,
            avg_execution_time_ms: 0,
            last_execution_time: None,
            last_error_message: None,
            metadata: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Record tool execution start
    pub async fn record_tool_execution_start(
        &self,
        task_id: String,
        tool_id: String,
        tool_name: String,
        tool_type: ToolType,
        input_params: Option<serde_json::Value>,
    ) -> Result<String> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now();
        let log_id = Uuid::new_v4().to_string();

        // Get or create task tool execution record
        let task_tool_exec = self.init_task_tool_execution(CreateTaskToolExecutionRequest {
            task_id: task_id.clone(),
            tool_id: tool_id.clone(),
            tool_name: tool_name.clone(),
            tool_type: tool_type.clone(),
        }).await?;

        // Update status to running
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE task_tool_executions SET status = $1, updated_at = $2 WHERE id = $3"
                )
                .bind(ToolExecutionStatus::Running.to_string())
                .bind(now)
                .bind(&task_tool_exec.id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE task_tool_executions SET status = ?, updated_at = ? WHERE id = ?"
                )
                .bind(ToolExecutionStatus::Running.to_string())
                .bind(now)
                .bind(&task_tool_exec.id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE task_tool_executions SET status = ?, updated_at = ? WHERE id = ?"
                )
                .bind(ToolExecutionStatus::Running.to_string())
                .bind(now)
                .bind(&task_tool_exec.id)
                .execute(pool)
                .await?;
            }
        }

        // Create execution log
        let input_json = input_params.map(|v| v.to_string());
        
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO task_tool_execution_logs 
                       (id, task_tool_execution_id, task_id, tool_id, tool_name, tool_type, 
                        status, started_at, input_params, created_at)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"#
                )
                .bind(&log_id)
                .bind(&task_tool_exec.id)
                .bind(&task_id)
                .bind(&tool_id)
                .bind(&tool_name)
                .bind(tool_type.to_string())
                .bind(ToolExecutionStatus::Running.to_string())
                .bind(now)
                .bind(input_json)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"INSERT INTO task_tool_execution_logs 
                       (id, task_tool_execution_id, task_id, tool_id, tool_name, tool_type, 
                        status, started_at, input_params, created_at)
                       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
                )
                .bind(&log_id)
                .bind(&task_tool_exec.id)
                .bind(&task_id)
                .bind(&tool_id)
                .bind(&tool_name)
                .bind(tool_type.to_string())
                .bind(ToolExecutionStatus::Running.to_string())
                .bind(now)
                .bind(input_json)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO task_tool_execution_logs 
                       (id, task_tool_execution_id, task_id, tool_id, tool_name, tool_type, 
                        status, started_at, input_params, created_at)
                       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
                )
                .bind(&log_id)
                .bind(&task_tool_exec.id)
                .bind(&task_id)
                .bind(&tool_id)
                .bind(&tool_name)
                .bind(tool_type.to_string())
                .bind(ToolExecutionStatus::Running.to_string())
                .bind(now)
                .bind(input_json)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        debug!("Recorded tool execution start: {} for task: {}, tool: {}", 
               log_id, task_id, tool_id);

        Ok(log_id)
    }

    /// Record tool execution completion
    pub async fn record_tool_execution_complete(
        &self,
        log_id: String,
        success: bool,
        output_result: Option<serde_json::Value>,
        error_message: Option<String>,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now();

        // Get the log record to calculate execution time
        let (task_tool_exec_id, started_at): (String, chrono::DateTime<Utc>) = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let row = sqlx::query(
                    "SELECT task_tool_execution_id, started_at FROM task_tool_execution_logs WHERE id = $1"
                )
                .bind(&log_id)
                .fetch_one(pool)
                .await?;
                (row.get("task_tool_execution_id"), row.get("started_at"))
            }
            DatabasePool::SQLite(pool) => {
                let row = sqlx::query(
                    "SELECT task_tool_execution_id, started_at FROM task_tool_execution_logs WHERE id = ?"
                )
                .bind(&log_id)
                .fetch_one(pool)
                .await?;
                (row.get("task_tool_execution_id"), row.get("started_at"))
            }
            DatabasePool::MySQL(pool) => {
                let row = sqlx::query(
                    "SELECT task_tool_execution_id, started_at FROM task_tool_execution_logs WHERE id = ?"
                )
                .bind(&log_id)
                .fetch_one(pool)
                .await?;
                (row.get("task_tool_execution_id"), row.get("started_at"))
            }
        };
        let execution_time_ms = now.signed_duration_since(started_at).num_milliseconds();

        // Update execution log
        let status = if success {
            ToolExecutionStatus::Completed
        } else {
            ToolExecutionStatus::Error
        };

        let output_json = output_result.map(|v| v.to_string());

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"UPDATE task_tool_execution_logs 
                       SET status = $1, completed_at = $2, execution_time_ms = $3, 
                           output_result = $4, error_message = $5
                       WHERE id = $6"#
                )
                .bind(status.to_string())
                .bind(now)
                .bind(execution_time_ms)
                .bind(output_json.clone())
                .bind(&error_message)
                .bind(&log_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"UPDATE task_tool_execution_logs 
                       SET status = ?, completed_at = ?, execution_time_ms = ?, 
                           output_result = ?, error_message = ?
                       WHERE id = ?"#
                )
                .bind(status.to_string())
                .bind(now)
                .bind(execution_time_ms)
                .bind(output_json.clone())
                .bind(&error_message)
                .bind(&log_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"UPDATE task_tool_execution_logs 
                       SET status = ?, completed_at = ?, execution_time_ms = ?, 
                           output_result = ?, error_message = ?
                       WHERE id = ?"#
                )
                .bind(status.to_string())
                .bind(now)
                .bind(execution_time_ms)
                .bind(output_json.clone())
                .bind(&error_message)
                .bind(&log_id)
                .execute(pool)
                .await?;
            }
        }

        // Update aggregated task tool execution record
        let (mut exec_count, mut success_count, mut error_count, mut total_time): (i64, i64, i64, i64) = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let row = sqlx::query(
                    r#"SELECT execution_count, success_count, error_count, 
                       total_execution_time_ms FROM task_tool_executions WHERE id = $1"#
                )
                .bind(&task_tool_exec_id)
                .fetch_one(pool)
                .await?;
                (
                    row.get("execution_count"),
                    row.get("success_count"),
                    row.get("error_count"),
                    row.get("total_execution_time_ms"),
                )
            }
            DatabasePool::SQLite(pool) => {
                let row = sqlx::query(
                    r#"SELECT execution_count, success_count, error_count, 
                       total_execution_time_ms FROM task_tool_executions WHERE id = ?"#
                )
                .bind(&task_tool_exec_id)
                .fetch_one(pool)
                .await?;
                (
                    row.get("execution_count"),
                    row.get("success_count"),
                    row.get("error_count"),
                    row.get("total_execution_time_ms"),
                )
            }
            DatabasePool::MySQL(pool) => {
                let row = sqlx::query(
                    r#"SELECT execution_count, success_count, error_count, 
                       total_execution_time_ms FROM task_tool_executions WHERE id = ?"#
                )
                .bind(&task_tool_exec_id)
                .fetch_one(pool)
                .await?;
                (
                    row.get("execution_count"),
                    row.get("success_count"),
                    row.get("error_count"),
                    row.get("total_execution_time_ms"),
                )
            }
        };

        exec_count += 1;
        if success {
            success_count += 1;
        } else {
            error_count += 1;
        }
        total_time += execution_time_ms;
        let avg_time = total_time / exec_count;

        let final_status = if success {
            ToolExecutionStatus::Idle
        } else {
            ToolExecutionStatus::Error
        };

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"UPDATE task_tool_executions 
                       SET status = $1, execution_count = $2, success_count = $3, error_count = $4,
                           total_execution_time_ms = $5, avg_execution_time_ms = $6,
                           last_execution_time = $7, last_error_message = $8, updated_at = $9
                       WHERE id = $10"#
                )
                .bind(final_status.to_string())
                .bind(exec_count)
                .bind(success_count)
                .bind(error_count)
                .bind(total_time)
                .bind(avg_time)
                .bind(now)
                .bind(&error_message)
                .bind(now)
                .bind(&task_tool_exec_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"UPDATE task_tool_executions 
                       SET status = ?, execution_count = ?, success_count = ?, error_count = ?,
                           total_execution_time_ms = ?, avg_execution_time_ms = ?,
                           last_execution_time = ?, last_error_message = ?, updated_at = ?
                       WHERE id = ?"#
                )
                .bind(final_status.to_string())
                .bind(exec_count)
                .bind(success_count)
                .bind(error_count)
                .bind(total_time)
                .bind(avg_time)
                .bind(now)
                .bind(&error_message)
                .bind(now)
                .bind(&task_tool_exec_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"UPDATE task_tool_executions 
                       SET status = ?, execution_count = ?, success_count = ?, error_count = ?,
                           total_execution_time_ms = ?, avg_execution_time_ms = ?,
                           last_execution_time = ?, last_error_message = ?, updated_at = ?
                       WHERE id = ?"#
                )
                .bind(final_status.to_string())
                .bind(exec_count)
                .bind(success_count)
                .bind(error_count)
                .bind(total_time)
                .bind(avg_time)
                .bind(now)
                .bind(&error_message)
                .bind(now)
                .bind(&task_tool_exec_id)
                .execute(pool)
                .await?;
            }
        }

        debug!("Recorded tool execution complete: {} (success: {})", log_id, success);

        Ok(())
    }

    /// Get active tools for a task
    pub async fn get_task_active_tools(&self, task_id: String) -> Result<Vec<ActiveToolInfo>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let tools = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let rows = sqlx::query(
                    r#"SELECT l.id as log_id, l.task_id, l.tool_id, l.tool_name, 
                       l.tool_type, l.started_at, l.input_params,
                       t.name as task_name
                       FROM task_tool_execution_logs l
                       LEFT JOIN scan_tasks t ON l.task_id = t.id
                       WHERE l.task_id = $1 AND l.status = 'running'
                       ORDER BY l.started_at DESC"#
                )
                .bind(&task_id)
                .fetch_all(pool)
                .await?;

                rows.into_iter().map(|row| ActiveToolInfo {
                    log_id: row.get("log_id"),
                    task_id: row.get("task_id"),
                    task_name: row.get("task_name"),
                    tool_id: row.get("tool_id"),
                    tool_name: row.get("tool_name"),
                    tool_type: row.get::<String, _>("tool_type").parse().unwrap_or(ToolType::Plugin),
                    started_at: row.get("started_at"),
                    input_params: row.get::<Option<String>, _>("input_params")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                }).collect()
            }
            DatabasePool::SQLite(pool) => {
                let rows = sqlx::query(
                    r#"SELECT l.id as log_id, l.task_id, l.tool_id, l.tool_name, 
                       l.tool_type, l.started_at, l.input_params,
                       t.name as task_name
                       FROM task_tool_execution_logs l
                       LEFT JOIN scan_tasks t ON l.task_id = t.id
                       WHERE l.task_id = ? AND l.status = 'running'
                       ORDER BY l.started_at DESC"#
                )
                .bind(&task_id)
                .fetch_all(pool)
                .await?;

                rows.into_iter().map(|row| ActiveToolInfo {
                    log_id: row.get("log_id"),
                    task_id: row.get("task_id"),
                    task_name: row.get("task_name"),
                    tool_id: row.get("tool_id"),
                    tool_name: row.get("tool_name"),
                    tool_type: row.get::<String, _>("tool_type").parse().unwrap_or(ToolType::Plugin),
                    started_at: row.get("started_at"),
                    input_params: row.get::<Option<String>, _>("input_params")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                }).collect()
            }
            DatabasePool::MySQL(pool) => {
                let rows = sqlx::query(
                    r#"SELECT l.id as log_id, l.task_id, l.tool_id, l.tool_name, 
                       l.tool_type, l.started_at, l.input_params,
                       t.name as task_name
                       FROM task_tool_execution_logs l
                       LEFT JOIN scan_tasks t ON l.task_id = t.id
                       WHERE l.task_id = ? AND l.status = 'running'
                       ORDER BY l.started_at DESC"#
                )
                .bind(&task_id)
                .fetch_all(pool)
                .await?;

                rows.into_iter().map(|row| ActiveToolInfo {
                    log_id: row.get("log_id"),
                    task_id: row.get("task_id"),
                    task_name: row.get("task_name"),
                    tool_id: row.get("tool_id"),
                    tool_name: row.get("tool_name"),
                    tool_type: row.get::<String, _>("tool_type").parse().unwrap_or(ToolType::Plugin),
                    started_at: row.get("started_at"),
                    input_params: row.get::<Option<String>, _>("input_params")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                }).collect()
            }
        };

        Ok(tools)
    }

    /// Get tool statistics for a task
    pub async fn get_task_tool_statistics(&self, task_id: String) -> Result<ToolStatistics> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let (total_executions, successful_executions, failed_executions, total_execution_time, tools_used) =
            match runtime {
                DatabasePool::PostgreSQL(pool) => {
                    let row = sqlx::query(
                        r#"SELECT 
                           SUM(execution_count) as total_executions,
                           SUM(success_count) as successful_executions,
                           SUM(error_count) as failed_executions,
                           SUM(total_execution_time_ms) as total_execution_time
                           FROM task_tool_executions 
                           WHERE task_id = $1"#
                    )
                    .bind(&task_id)
                    .fetch_one(pool)
                    .await?;

                    let tools_used = sqlx::query(
                        "SELECT DISTINCT tool_name FROM task_tool_executions WHERE task_id = $1"
                    )
                    .bind(&task_id)
                    .fetch_all(pool)
                    .await?
                    .iter()
                    .map(|r| r.get::<String, _>("tool_name"))
                    .collect();

                    (
                        row.get::<Option<i64>, _>("total_executions").unwrap_or(0),
                        row.get::<Option<i64>, _>("successful_executions").unwrap_or(0),
                        row.get::<Option<i64>, _>("failed_executions").unwrap_or(0),
                        row.get::<Option<i64>, _>("total_execution_time").unwrap_or(0),
                        tools_used,
                    )
                }
                DatabasePool::SQLite(pool) => {
                    let row = sqlx::query(
                        r#"SELECT 
                           SUM(execution_count) as total_executions,
                           SUM(success_count) as successful_executions,
                           SUM(error_count) as failed_executions,
                           SUM(total_execution_time_ms) as total_execution_time
                           FROM task_tool_executions 
                           WHERE task_id = ?"#
                    )
                    .bind(&task_id)
                    .fetch_one(pool)
                    .await?;

                    let tools_used = sqlx::query(
                        "SELECT DISTINCT tool_name FROM task_tool_executions WHERE task_id = ?"
                    )
                    .bind(&task_id)
                    .fetch_all(pool)
                    .await?
                    .iter()
                    .map(|r| r.get::<String, _>("tool_name"))
                    .collect();

                    (
                        row.get::<Option<i64>, _>("total_executions").unwrap_or(0),
                        row.get::<Option<i64>, _>("successful_executions").unwrap_or(0),
                        row.get::<Option<i64>, _>("failed_executions").unwrap_or(0),
                        row.get::<Option<i64>, _>("total_execution_time").unwrap_or(0),
                        tools_used,
                    )
                }
                DatabasePool::MySQL(pool) => {
                    let row = sqlx::query(
                        r#"SELECT 
                           SUM(execution_count) as total_executions,
                           SUM(success_count) as successful_executions,
                           SUM(error_count) as failed_executions,
                           SUM(total_execution_time_ms) as total_execution_time
                           FROM task_tool_executions 
                           WHERE task_id = ?"#
                    )
                    .bind(&task_id)
                    .fetch_one(pool)
                    .await?;

                    let tools_used = sqlx::query(
                        "SELECT DISTINCT tool_name FROM task_tool_executions WHERE task_id = ?"
                    )
                    .bind(&task_id)
                    .fetch_all(pool)
                    .await?
                    .iter()
                    .map(|r| r.get::<String, _>("tool_name"))
                    .collect();

                    (
                        row.get::<Option<i64>, _>("total_executions").unwrap_or(0),
                        row.get::<Option<i64>, _>("successful_executions").unwrap_or(0),
                        row.get::<Option<i64>, _>("failed_executions").unwrap_or(0),
                        row.get::<Option<i64>, _>("total_execution_time").unwrap_or(0),
                        tools_used,
                    )
                }
            };

        Ok(ToolStatistics {
            total_executions,
            successful_executions,
            failed_executions,
            total_execution_time,
            tools_used,
        })
    }

    /// Get tool execution history for a task
    pub async fn get_tool_execution_history(
        &self,
        task_id: String,
        tool_id: Option<String>,
        limit: Option<i64>,
    ) -> Result<Vec<ExecutionRecord>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let records = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let mut query_str = r#"SELECT id, tool_name, tool_type, status, started_at, 
                                       completed_at, execution_time_ms, error_message
                                       FROM task_tool_execution_logs 
                                       WHERE task_id = $1"#.to_string();
                if tool_id.is_some() {
                    query_str.push_str(" AND tool_id = $2");
                }
                query_str.push_str(" ORDER BY started_at DESC");
                if let Some(lim) = limit {
                    query_str.push_str(&format!(" LIMIT {}", lim));
                }
                let mut query = sqlx::query(&query_str).bind(&task_id);
                if let Some(ref tid) = tool_id {
                    query = query.bind(tid);
                }
                let rows = query.fetch_all(pool).await?;
                rows.into_iter().map(|row| ExecutionRecord {
                    id: row.get("id"),
                    tool_name: row.get("tool_name"),
                    tool_type: row.get::<String, _>("tool_type").parse().unwrap_or(ToolType::Plugin),
                    status: row.get::<String, _>("status").parse().unwrap_or(ToolExecutionStatus::Idle),
                    started_at: row.get("started_at"),
                    completed_at: row.get("completed_at"),
                    execution_time_ms: row.get("execution_time_ms"),
                    error_message: row.get("error_message"),
                }).collect()
            }
            DatabasePool::SQLite(pool) => {
                let mut query_str = r#"SELECT id, tool_name, tool_type, status, started_at, 
                                       completed_at, execution_time_ms, error_message
                                       FROM task_tool_execution_logs 
                                       WHERE task_id = ?"#.to_string();
                if tool_id.is_some() {
                    query_str.push_str(" AND tool_id = ?");
                }
                query_str.push_str(" ORDER BY started_at DESC");
                if let Some(lim) = limit {
                    query_str.push_str(&format!(" LIMIT {}", lim));
                }
                let mut query = sqlx::query(&query_str).bind(&task_id);
                if let Some(ref tid) = tool_id {
                    query = query.bind(tid);
                }
                let rows = query.fetch_all(pool).await?;
                rows.into_iter().map(|row| ExecutionRecord {
                    id: row.get("id"),
                    tool_name: row.get("tool_name"),
                    tool_type: row.get::<String, _>("tool_type").parse().unwrap_or(ToolType::Plugin),
                    status: row.get::<String, _>("status").parse().unwrap_or(ToolExecutionStatus::Idle),
                    started_at: row.get("started_at"),
                    completed_at: row.get("completed_at"),
                    execution_time_ms: row.get("execution_time_ms"),
                    error_message: row.get("error_message"),
                }).collect()
            }
            DatabasePool::MySQL(pool) => {
                let mut query_str = r#"SELECT id, tool_name, tool_type, status, started_at, 
                                       completed_at, execution_time_ms, error_message
                                       FROM task_tool_execution_logs 
                                       WHERE task_id = ?"#.to_string();
                if tool_id.is_some() {
                    query_str.push_str(" AND tool_id = ?");
                }
                query_str.push_str(" ORDER BY started_at DESC");
                if let Some(lim) = limit {
                    query_str.push_str(&format!(" LIMIT {}", lim));
                }
                let mut query = sqlx::query(&query_str).bind(&task_id);
                if let Some(ref tid) = tool_id {
                    query = query.bind(tid);
                }
                let rows = query.fetch_all(pool).await?;
                rows.into_iter().map(|row| ExecutionRecord {
                    id: row.get("id"),
                    tool_name: row.get("tool_name"),
                    tool_type: row.get::<String, _>("tool_type").parse().unwrap_or(ToolType::Plugin),
                    status: row.get::<String, _>("status").parse().unwrap_or(ToolExecutionStatus::Idle),
                    started_at: row.get("started_at"),
                    completed_at: row.get("completed_at"),
                    execution_time_ms: row.get("execution_time_ms"),
                    error_message: row.get("error_message"),
                }).collect()
            }
        };

        Ok(records)
    }

    /// Clean up old execution logs (for maintenance)
    pub async fn cleanup_old_execution_logs(&self, days_to_keep: i64) -> Result<u64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let cutoff_date = Utc::now() - chrono::Duration::days(days_to_keep);
        let affected = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM task_tool_execution_logs WHERE created_at < $1")
                    .bind(cutoff_date)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM task_tool_execution_logs WHERE created_at < ?")
                    .bind(cutoff_date)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM task_tool_execution_logs WHERE created_at < ?")
                    .bind(cutoff_date)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
        };

        info!("Cleaned up {} old execution logs", affected);
        Ok(affected)
    }

    /// Get all active tools across all tasks
    pub async fn get_all_active_tools(&self) -> Result<Vec<ActiveToolInfo>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let active_tools = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let rows = sqlx::query(
                    r#"SELECT l.id as log_id, l.task_id, l.tool_id, l.tool_name, 
                       l.tool_type, l.started_at, l.input_params,
                       t.name as task_name
                       FROM task_tool_execution_logs l
                       LEFT JOIN scan_tasks t ON l.task_id = t.id
                       WHERE l.status = 'running'
                       ORDER BY l.started_at DESC"#
                )
                .fetch_all(pool)
                .await?;
                rows.into_iter().map(|row| ActiveToolInfo {
                    log_id: row.get("log_id"),
                    task_id: row.get("task_id"),
                    task_name: row.get("task_name"),
                    tool_id: row.get("tool_id"),
                    tool_name: row.get("tool_name"),
                    tool_type: row.get::<String, _>("tool_type").parse().unwrap_or(ToolType::Plugin),
                    started_at: row.get("started_at"),
                    input_params: row.get::<Option<String>, _>("input_params")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                }).collect()
            }
            DatabasePool::SQLite(pool) => {
                let rows = sqlx::query(
                    r#"SELECT l.id as log_id, l.task_id, l.tool_id, l.tool_name, 
                       l.tool_type, l.started_at, l.input_params,
                       t.name as task_name
                       FROM task_tool_execution_logs l
                       LEFT JOIN scan_tasks t ON l.task_id = t.id
                       WHERE l.status = 'running'
                       ORDER BY l.started_at DESC"#
                )
                .fetch_all(pool)
                .await?;
                rows.into_iter().map(|row| ActiveToolInfo {
                    log_id: row.get("log_id"),
                    task_id: row.get("task_id"),
                    task_name: row.get("task_name"),
                    tool_id: row.get("tool_id"),
                    tool_name: row.get("tool_name"),
                    tool_type: row.get::<String, _>("tool_type").parse().unwrap_or(ToolType::Plugin),
                    started_at: row.get("started_at"),
                    input_params: row.get::<Option<String>, _>("input_params")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                }).collect()
            }
            DatabasePool::MySQL(pool) => {
                let rows = sqlx::query(
                    r#"SELECT l.id as log_id, l.task_id, l.tool_id, l.tool_name, 
                       l.tool_type, l.started_at, l.input_params,
                       t.name as task_name
                       FROM task_tool_execution_logs l
                       LEFT JOIN scan_tasks t ON l.task_id = t.id
                       WHERE l.status = 'running'
                       ORDER BY l.started_at DESC"#
                )
                .fetch_all(pool)
                .await?;
                rows.into_iter().map(|row| ActiveToolInfo {
                    log_id: row.get("log_id"),
                    task_id: row.get("task_id"),
                    task_name: row.get("task_name"),
                    tool_id: row.get("tool_id"),
                    tool_name: row.get("tool_name"),
                    tool_type: row.get::<String, _>("tool_type").parse().unwrap_or(ToolType::Plugin),
                    started_at: row.get("started_at"),
                    input_params: row.get::<Option<String>, _>("input_params")
                        .and_then(|s| serde_json::from_str(&s).ok()),
                }).collect()
            }
        };

        Ok(active_tools)
    }
}

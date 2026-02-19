use anyhow::Result;
use crate::core::models::database::{ScanTask, Vulnerability, ToolExecution};
use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;

impl DatabaseService {
    pub async fn create_scan_task_internal(&self, task: &ScanTask) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO scan_tasks (
                        id, project_id, name, description, target_type, targets, scan_type, 
                        tools_config, status, progress, priority, scheduled_at, started_at, 
                        completed_at, execution_time, results_summary, error_message, 
                        created_by, created_at, updated_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)"#,
                )
                .bind(&task.id)
                .bind(&task.project_id)
                .bind(&task.name)
                .bind(&task.description)
                .bind(&task.target_type)
                .bind(&task.targets)
                .bind(&task.scan_type)
                .bind(&task.tools_config)
                .bind(&task.status)
                .bind(task.progress)
                .bind(task.priority)
                .bind(task.scheduled_at)
                .bind(task.started_at)
                .bind(task.completed_at)
                .bind(task.execution_time)
                .bind(&task.results_summary)
                .bind(&task.error_message)
                .bind(&task.created_by)
                .bind(task.created_at)
                .bind(task.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"INSERT INTO scan_tasks (
                        id, project_id, name, description, target_type, targets, scan_type, 
                        tools_config, status, progress, priority, scheduled_at, started_at, 
                        completed_at, execution_time, results_summary, error_message, 
                        created_by, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                )
                .bind(&task.id)
                .bind(&task.project_id)
                .bind(&task.name)
                .bind(&task.description)
                .bind(&task.target_type)
                .bind(&task.targets)
                .bind(&task.scan_type)
                .bind(&task.tools_config)
                .bind(&task.status)
                .bind(task.progress)
                .bind(task.priority)
                .bind(task.scheduled_at)
                .bind(task.started_at)
                .bind(task.completed_at)
                .bind(task.execution_time)
                .bind(&task.results_summary)
                .bind(&task.error_message)
                .bind(&task.created_by)
                .bind(task.created_at)
                .bind(task.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO scan_tasks (
                        id, project_id, name, description, target_type, targets, scan_type, 
                        tools_config, status, progress, priority, scheduled_at, started_at, 
                        completed_at, execution_time, results_summary, error_message, 
                        created_by, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                )
                .bind(&task.id)
                .bind(&task.project_id)
                .bind(&task.name)
                .bind(&task.description)
                .bind(&task.target_type)
                .bind(&task.targets)
                .bind(&task.scan_type)
                .bind(&task.tools_config)
                .bind(&task.status)
                .bind(task.progress)
                .bind(task.priority)
                .bind(task.scheduled_at)
                .bind(task.started_at)
                .bind(task.completed_at)
                .bind(task.execution_time)
                .bind(&task.results_summary)
                .bind(&task.error_message)
                .bind(&task.created_by)
                .bind(task.created_at)
                .bind(task.updated_at)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_scan_tasks_internal(&self, project_id: Option<&str>) -> Result<Vec<ScanTask>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                if let Some(pid) = project_id {
                    sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks WHERE project_id = $1 ORDER BY created_at DESC")
                        .bind(pid)
                        .fetch_all(pool)
                        .await?
                } else {
                    sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks ORDER BY created_at DESC")
                        .fetch_all(pool)
                        .await?
                }
            }
            DatabasePool::SQLite(pool) => {
                if let Some(pid) = project_id {
                    sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks WHERE project_id = ? ORDER BY created_at DESC")
                        .bind(pid)
                        .fetch_all(pool)
                        .await?
                } else {
                    sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks ORDER BY created_at DESC")
                        .fetch_all(pool)
                        .await?
                }
            }
            DatabasePool::MySQL(pool) => {
                if let Some(pid) = project_id {
                    sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks WHERE project_id = ? ORDER BY created_at DESC")
                        .bind(pid)
                        .fetch_all(pool)
                        .await?
                } else {
                    sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks ORDER BY created_at DESC")
                        .fetch_all(pool)
                        .await?
                }
            }
        };
        Ok(rows)
    }

    pub async fn get_scan_task_internal(&self, id: &str) -> Result<Option<ScanTask>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let row = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks WHERE id = ?")
                    .bind(id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks WHERE id = ?")
                    .bind(id)
                    .fetch_optional(pool)
                    .await?
            }
        };
        Ok(row)
    }

    pub async fn get_scan_tasks_by_target_internal(&self, target: &str) -> Result<Vec<ScanTask>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let pattern = format!("%{}%", target);
        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, ScanTask>(
                    "SELECT * FROM scan_tasks WHERE targets LIKE $1 ORDER BY created_at DESC",
                )
                .bind(&pattern)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, ScanTask>(
                    "SELECT * FROM scan_tasks WHERE targets LIKE ? ORDER BY created_at DESC",
                )
                .bind(&pattern)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, ScanTask>(
                    "SELECT * FROM scan_tasks WHERE targets LIKE ? ORDER BY created_at DESC",
                )
                .bind(&pattern)
                .fetch_all(pool)
                .await?
            }
        };
        Ok(rows)
    }

    pub async fn update_scan_task_status_internal(&self, id: &str, status: &str, progress: Option<f64>) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"UPDATE scan_tasks
                       SET status = $1,
                           progress = COALESCE($2, progress),
                           started_at = CASE WHEN $1 = 'running' THEN COALESCE(started_at, CURRENT_TIMESTAMP) ELSE started_at END,
                           completed_at = CASE WHEN $1 IN ('completed', 'failed', 'cancelled') THEN CURRENT_TIMESTAMP ELSE completed_at END,
                           updated_at = CURRENT_TIMESTAMP
                       WHERE id = $3"#,
                )
                .bind(status)
                .bind(progress)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"UPDATE scan_tasks
                       SET status = ?,
                           progress = COALESCE(?, progress),
                           started_at = CASE WHEN ? = 'running' THEN COALESCE(started_at, CURRENT_TIMESTAMP) ELSE started_at END,
                           completed_at = CASE WHEN ? IN ('completed', 'failed', 'cancelled') THEN CURRENT_TIMESTAMP ELSE completed_at END,
                           updated_at = CURRENT_TIMESTAMP
                       WHERE id = ?"#,
                )
                .bind(status)
                .bind(progress)
                .bind(status)
                .bind(status)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"UPDATE scan_tasks
                       SET status = ?,
                           progress = COALESCE(?, progress),
                           started_at = CASE WHEN ? = 'running' THEN COALESCE(started_at, CURRENT_TIMESTAMP) ELSE started_at END,
                           completed_at = CASE WHEN ? IN ('completed', 'failed', 'cancelled') THEN CURRENT_TIMESTAMP ELSE completed_at END,
                           updated_at = CURRENT_TIMESTAMP
                       WHERE id = ?"#,
                )
                .bind(status)
                .bind(progress)
                .bind(status)
                .bind(status)
                .bind(id)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn delete_scan_task_internal(&self, id: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM scan_tasks WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM scan_tasks WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM scan_tasks WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn stop_scan_task_internal(&self, id: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("UPDATE scan_tasks SET status = 'cancelled', updated_at = CURRENT_TIMESTAMP, completed_at = CURRENT_TIMESTAMP WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("UPDATE scan_tasks SET status = 'cancelled', updated_at = CURRENT_TIMESTAMP, completed_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("UPDATE scan_tasks SET status = 'cancelled', updated_at = CURRENT_TIMESTAMP, completed_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn create_vulnerability_internal(&self, v: &Vulnerability) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO vulnerabilities (
                        id, project_id, asset_id, scan_task_id, title, description, 
                        vulnerability_type, severity, cvss_score, cvss_vector, cwe_id, 
                        owasp_category, proof_of_concept, impact, remediation, references_json, 
                        status, verification_status, resolution_date, tags, attachments, notes, 
                        created_at, updated_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)"#,
                )
                .bind(&v.id)
                .bind(&v.project_id)
                .bind(&v.asset_id)
                .bind(&v.scan_task_id)
                .bind(&v.title)
                .bind(&v.description)
                .bind(&v.vulnerability_type)
                .bind(&v.severity)
                .bind(v.cvss_score)
                .bind(&v.cvss_vector)
                .bind(&v.cwe_id)
                .bind(&v.owasp_category)
                .bind(&v.proof_of_concept)
                .bind(&v.impact)
                .bind(&v.remediation)
                .bind(&v.references)
                .bind(&v.status)
                .bind(&v.verification_status)
                .bind(v.resolution_date)
                .bind(&v.tags)
                .bind(&v.attachments)
                .bind(&v.notes)
                .bind(v.created_at)
                .bind(v.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"INSERT INTO vulnerabilities (
                        id, project_id, asset_id, scan_task_id, title, description, 
                        vulnerability_type, severity, cvss_score, cvss_vector, cwe_id, 
                        owasp_category, proof_of_concept, impact, remediation, references_json, 
                        status, verification_status, resolution_date, tags, attachments, notes, 
                        created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                )
                .bind(&v.id)
                .bind(&v.project_id)
                .bind(&v.asset_id)
                .bind(&v.scan_task_id)
                .bind(&v.title)
                .bind(&v.description)
                .bind(&v.vulnerability_type)
                .bind(&v.severity)
                .bind(v.cvss_score)
                .bind(&v.cvss_vector)
                .bind(&v.cwe_id)
                .bind(&v.owasp_category)
                .bind(&v.proof_of_concept)
                .bind(&v.impact)
                .bind(&v.remediation)
                .bind(&v.references)
                .bind(&v.status)
                .bind(&v.verification_status)
                .bind(v.resolution_date)
                .bind(&v.tags)
                .bind(&v.attachments)
                .bind(&v.notes)
                .bind(v.created_at)
                .bind(v.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO vulnerabilities (
                        id, project_id, asset_id, scan_task_id, title, description, 
                        vulnerability_type, severity, cvss_score, cvss_vector, cwe_id, 
                        owasp_category, proof_of_concept, impact, remediation, references_json, 
                        status, verification_status, resolution_date, tags, attachments, notes, 
                        created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                )
                .bind(&v.id)
                .bind(&v.project_id)
                .bind(&v.asset_id)
                .bind(&v.scan_task_id)
                .bind(&v.title)
                .bind(&v.description)
                .bind(&v.vulnerability_type)
                .bind(&v.severity)
                .bind(v.cvss_score)
                .bind(&v.cvss_vector)
                .bind(&v.cwe_id)
                .bind(&v.owasp_category)
                .bind(&v.proof_of_concept)
                .bind(&v.impact)
                .bind(&v.remediation)
                .bind(&v.references)
                .bind(&v.status)
                .bind(&v.verification_status)
                .bind(v.resolution_date)
                .bind(&v.tags)
                .bind(&v.attachments)
                .bind(&v.notes)
                .bind(v.created_at)
                .bind(v.updated_at)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_vulnerabilities_internal(&self, project_id: Option<&str>) -> Result<Vec<Vulnerability>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                if let Some(pid) = project_id {
                    sqlx::query_as::<_, Vulnerability>("SELECT * FROM vulnerabilities WHERE project_id = $1 ORDER BY created_at DESC")
                        .bind(pid)
                        .fetch_all(pool)
                        .await?
                } else {
                    sqlx::query_as::<_, Vulnerability>("SELECT * FROM vulnerabilities ORDER BY created_at DESC")
                        .fetch_all(pool)
                        .await?
                }
            }
            DatabasePool::SQLite(pool) => {
                if let Some(pid) = project_id {
                    sqlx::query_as::<_, Vulnerability>("SELECT * FROM vulnerabilities WHERE project_id = ? ORDER BY created_at DESC")
                        .bind(pid)
                        .fetch_all(pool)
                        .await?
                } else {
                    sqlx::query_as::<_, Vulnerability>("SELECT * FROM vulnerabilities ORDER BY created_at DESC")
                        .fetch_all(pool)
                        .await?
                }
            }
            DatabasePool::MySQL(pool) => {
                if let Some(pid) = project_id {
                    sqlx::query_as::<_, Vulnerability>("SELECT * FROM vulnerabilities WHERE project_id = ? ORDER BY created_at DESC")
                        .bind(pid)
                        .fetch_all(pool)
                        .await?
                } else {
                    sqlx::query_as::<_, Vulnerability>("SELECT * FROM vulnerabilities ORDER BY created_at DESC")
                        .fetch_all(pool)
                        .await?
                }
            }
        };
        Ok(rows)
    }

    pub async fn get_vulnerability_internal(&self, id: &str) -> Result<Option<Vulnerability>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let row = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, Vulnerability>("SELECT * FROM vulnerabilities WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, Vulnerability>("SELECT * FROM vulnerabilities WHERE id = ?")
                    .bind(id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, Vulnerability>("SELECT * FROM vulnerabilities WHERE id = ?")
                    .bind(id)
                    .fetch_optional(pool)
                    .await?
            }
        };
        Ok(row)
    }

    pub async fn update_vulnerability_status_internal(&self, id: &str, status: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("UPDATE vulnerabilities SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(status)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("UPDATE vulnerabilities SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(status)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("UPDATE vulnerabilities SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(status)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn create_tool_execution_internal(&self, exec: &ToolExecution) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO tool_executions (
                        id, tool_id, scan_task_id, command, arguments, status, progress,
                        start_time, end_time, execution_time, output, error_output, exit_code,
                        resource_usage, artifacts, metadata, created_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
                    "#,
                )
                .bind(&exec.id)
                .bind(&exec.tool_id)
                .bind(&exec.scan_task_id)
                .bind(&exec.command)
                .bind(&exec.arguments)
                .bind(&exec.status)
                .bind(exec.progress)
                .bind(exec.start_time)
                .bind(exec.end_time)
                .bind(exec.execution_time)
                .bind(&exec.output)
                .bind(&exec.error_output)
                .bind(exec.exit_code)
                .bind(&exec.resource_usage)
                .bind(&exec.artifacts)
                .bind(&exec.metadata)
                .bind(exec.created_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO tool_executions (
                        id, tool_id, scan_task_id, command, arguments, status, progress,
                        start_time, end_time, execution_time, output, error_output, exit_code,
                        resource_usage, artifacts, metadata, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&exec.id)
                .bind(&exec.tool_id)
                .bind(&exec.scan_task_id)
                .bind(&exec.command)
                .bind(&exec.arguments)
                .bind(&exec.status)
                .bind(exec.progress)
                .bind(exec.start_time)
                .bind(exec.end_time)
                .bind(exec.execution_time)
                .bind(&exec.output)
                .bind(&exec.error_output)
                .bind(exec.exit_code)
                .bind(&exec.resource_usage)
                .bind(&exec.artifacts)
                .bind(&exec.metadata)
                .bind(exec.created_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO tool_executions (
                        id, tool_id, scan_task_id, command, arguments, status, progress,
                        start_time, end_time, execution_time, output, error_output, exit_code,
                        resource_usage, artifacts, metadata, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&exec.id)
                .bind(&exec.tool_id)
                .bind(&exec.scan_task_id)
                .bind(&exec.command)
                .bind(&exec.arguments)
                .bind(&exec.status)
                .bind(exec.progress)
                .bind(exec.start_time)
                .bind(exec.end_time)
                .bind(exec.execution_time)
                .bind(&exec.output)
                .bind(&exec.error_output)
                .bind(exec.exit_code)
                .bind(&exec.resource_usage)
                .bind(&exec.artifacts)
                .bind(&exec.metadata)
                .bind(exec.created_at)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn update_tool_execution_status_internal(
        &self,
        id: &str,
        status: &str,
        progress: Option<f64>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
        execution_time: Option<i32>,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE tool_executions SET status = $1, progress = COALESCE($2, progress), end_time = COALESCE($3, end_time), execution_time = COALESCE($4, execution_time) WHERE id = $5",
                )
                .bind(status)
                .bind(progress)
                .bind(end_time)
                .bind(execution_time)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE tool_executions SET status = ?, progress = COALESCE(?, progress), end_time = COALESCE(?, end_time), execution_time = COALESCE(?, execution_time) WHERE id = ?",
                )
                .bind(status)
                .bind(progress)
                .bind(end_time)
                .bind(execution_time)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE tool_executions SET status = ?, progress = COALESCE(?, progress), end_time = COALESCE(?, end_time), execution_time = COALESCE(?, execution_time) WHERE id = ?",
                )
                .bind(status)
                .bind(progress)
                .bind(end_time)
                .bind(execution_time)
                .bind(id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_tool_executions_by_tool_internal(&self, tool_id: &str) -> Result<Vec<ToolExecution>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, ToolExecution>(
                    "SELECT * FROM tool_executions WHERE tool_id = $1 ORDER BY created_at DESC",
                )
                .bind(tool_id)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, ToolExecution>(
                    "SELECT * FROM tool_executions WHERE tool_id = ? ORDER BY created_at DESC",
                )
                .bind(tool_id)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, ToolExecution>(
                    "SELECT * FROM tool_executions WHERE tool_id = ? ORDER BY created_at DESC",
                )
                .bind(tool_id)
                .fetch_all(pool)
                .await?
            }
        };
        Ok(rows)
    }
}

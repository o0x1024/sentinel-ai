use anyhow::Result;
use crate::core::models::database::{ScanTask, Vulnerability, ToolExecution};
use crate::database_service::service::DatabaseService;

impl DatabaseService {
    pub async fn create_scan_task_internal(&self, task: &ScanTask) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"INSERT INTO scan_tasks (
                id, project_id, name, description, target_type, targets, scan_type, 
                tools_config, status, progress, priority, scheduled_at, started_at, 
                completed_at, execution_time, results_summary, error_message, 
                created_by, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
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
        .bind(&task.scheduled_at)
        .bind(&task.started_at)
        .bind(&task.completed_at)
        .bind(task.execution_time)
        .bind(&task.results_summary)
        .bind(&task.error_message)
        .bind(&task.created_by)
        .bind(task.created_at)
        .bind(task.updated_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_scan_tasks_internal(&self, project_id: Option<&str>) -> Result<Vec<ScanTask>> {
        let pool = self.get_pool()?;
        let query = if let Some(pid) = project_id {
            sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks WHERE project_id = ? ORDER BY created_at DESC").bind(pid)
        } else {
            sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks ORDER BY created_at DESC")
        };
        let rows = query.fetch_all(pool).await?;
        Ok(rows)
    }

    pub async fn get_scan_task_internal(&self, id: &str) -> Result<Option<ScanTask>> {
        let pool = self.get_pool()?;
        let row = sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(row)
    }

    pub async fn get_scan_tasks_by_target_internal(&self, target: &str) -> Result<Vec<ScanTask>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query_as::<_, ScanTask>(
            "SELECT * FROM scan_tasks WHERE targets LIKE ? ORDER BY created_at DESC"
        )
        .bind(format!("%{}%", target))
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    pub async fn update_scan_task_status_internal(&self, id: &str, status: &str, progress: Option<f64>) -> Result<()> {
        let pool = self.get_pool()?;
        let mut query = "UPDATE scan_tasks SET status = ?, updated_at = CURRENT_TIMESTAMP".to_string();

        if progress.is_some() {
            query.push_str(", progress = ?");
        }

        if status == "running" {
            query.push_str(", started_at = CURRENT_TIMESTAMP");
        } else if status == "completed" || status == "failed" || status == "cancelled" {
            query.push_str(", completed_at = CURRENT_TIMESTAMP");
        }

        query.push_str(" WHERE id = ?");

        let mut q = sqlx::query(&query).bind(status);
        if let Some(p) = progress {
            q = q.bind(p);
        }
        q.bind(id).execute(pool).await?;

        Ok(())
    }

    pub async fn delete_scan_task_internal(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM scan_tasks WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn stop_scan_task_internal(&self, id: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE scan_tasks SET status = 'cancelled', updated_at = CURRENT_TIMESTAMP, completed_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn create_vulnerability_internal(&self, v: &Vulnerability) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query(
            r#"INSERT INTO vulnerabilities (
                id, project_id, asset_id, scan_task_id, title, description, 
                vulnerability_type, severity, cvss_score, cvss_vector, cwe_id, 
                owasp_category, proof_of_concept, impact, remediation, references_json, 
                status, verification_status, resolution_date, tags, attachments, notes, 
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
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
        Ok(())
    }

    pub async fn get_vulnerabilities_internal(&self, project_id: Option<&str>) -> Result<Vec<Vulnerability>> {
        let pool = self.get_pool()?;
        let query = if let Some(pid) = project_id {
            sqlx::query_as::<_, Vulnerability>("SELECT * FROM vulnerabilities WHERE project_id = ? ORDER BY created_at DESC").bind(pid)
        } else {
            sqlx::query_as::<_, Vulnerability>("SELECT * FROM vulnerabilities ORDER BY created_at DESC")
        };
        let rows = query.fetch_all(pool).await?;
        Ok(rows)
    }

    pub async fn get_vulnerability_internal(&self, id: &str) -> Result<Option<Vulnerability>> {
        let pool = self.get_pool()?;
        let row = sqlx::query_as::<_, Vulnerability>("SELECT * FROM vulnerabilities WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(row)
    }

    pub async fn update_vulnerability_status_internal(&self, id: &str, status: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("UPDATE vulnerabilities SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn create_tool_execution_internal(&self, exec: &ToolExecution) -> Result<()> {
        let pool = self.get_pool()?;
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
        .bind(&exec.exit_code)
        .bind(&exec.resource_usage)
        .bind(&exec.artifacts)
        .bind(&exec.metadata)
        .bind(exec.created_at)
        .execute(pool)
        .await?;
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
        let pool = self.get_pool()?;
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
        Ok(())
    }

    pub async fn get_tool_executions_by_tool_internal(&self, tool_id: &str) -> Result<Vec<ToolExecution>> {
        let pool = self.get_pool()?;
        let rows = sqlx::query_as::<_, ToolExecution>(
            "SELECT * FROM tool_executions WHERE tool_id = ? ORDER BY created_at DESC",
        )
        .bind(tool_id)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }
}

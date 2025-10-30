use anyhow::Result;
use sentinel_core::models::database::ToolExecution;
use sqlx::sqlite::SqlitePool;

pub async fn create_tool_execution(pool: &SqlitePool, exec: &ToolExecution) -> Result<()> {
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

pub async fn update_tool_execution_status(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    progress: Option<f64>,
    end_time: Option<chrono::DateTime<chrono::Utc>>,
    execution_time: Option<i32>,
) -> Result<()> {
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

pub async fn get_tool_executions_by_tool(pool: &SqlitePool, tool_id: &str) -> Result<Vec<ToolExecution>> {
    let rows = sqlx::query_as::<_, ToolExecution>(
        "SELECT * FROM tool_executions WHERE tool_id = ? ORDER BY created_at DESC",
    )
    .bind(tool_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}



use anyhow::Result;
use sentinel_core::models::database::ScanTask;
use sqlx::sqlite::SqlitePool;

pub async fn create_scan_task(pool: &SqlitePool, task: &ScanTask) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO scan_tasks (
            id, project_id, name, description, target_type, targets,
            scan_type, tools_config, status, progress, priority,
            scheduled_at, created_by
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#,
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
    .bind(&task.created_by)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_scan_tasks(
    pool: &SqlitePool,
    project_id: Option<&str>,
) -> Result<Vec<ScanTask>> {
    let rows = if let Some(project_id) = project_id {
        sqlx::query_as::<_, ScanTask>(
            "SELECT * FROM scan_tasks WHERE project_id = ? ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks ORDER BY created_at DESC")
            .fetch_all(pool)
            .await?
    };

    Ok(rows)
}

pub async fn get_scan_task(pool: &SqlitePool, id: &str) -> Result<Option<ScanTask>> {
    let row = sqlx::query_as::<_, ScanTask>("SELECT * FROM scan_tasks WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn update_scan_task_status(
    pool: &SqlitePool,
    id: &str,
    status: &str,
    progress: Option<f64>,
) -> Result<()> {
    let mut query =
        "UPDATE scan_tasks SET status = ?, updated_at = CURRENT_TIMESTAMP".to_string();

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



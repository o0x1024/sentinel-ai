use anyhow::{anyhow, Result};
use chrono::Utc;
use sentinel_db::database_service::connection_manager::DatabasePool;
use serde_json::json;
use sqlx::Row;

use super::team_v3_commands::TeamV3TaskActionResult;
use super::team_v3_session_state::parse_state_data_text;

pub(crate) async fn get_team_v3_task_action_result(
    pool: &DatabasePool,
    session_id: &str,
    task_id: &str,
) -> Result<TeamV3TaskActionResult> {
    match pool {
        DatabasePool::SQLite(sqlite) => {
            let row = sqlx::query(
                r#"SELECT id, task_key, title, status, owner_agent_id, claimed_by_agent_id
                   FROM team_v3_tasks WHERE session_id = ? AND id = ?"#,
            )
            .bind(session_id)
            .bind(task_id)
            .fetch_optional(sqlite)
            .await?;
            let row = row.ok_or_else(|| anyhow!("task not found"))?;
            Ok(TeamV3TaskActionResult {
                task_id: row.get("id"),
                task_key: row.get("task_key"),
                title: row.get("title"),
                status: row.get("status"),
                owner_agent_id: row.get("owner_agent_id"),
                claimed_by_agent_id: row.get("claimed_by_agent_id"),
            })
        }
        DatabasePool::PostgreSQL(pg) => {
            let row = sqlx::query(
                r#"SELECT id, task_key, title, status, owner_agent_id, claimed_by_agent_id
                   FROM team_v3_tasks WHERE session_id = $1 AND id = $2"#,
            )
            .bind(session_id)
            .bind(task_id)
            .fetch_optional(pg)
            .await?;
            let row = row.ok_or_else(|| anyhow!("task not found"))?;
            Ok(TeamV3TaskActionResult {
                task_id: row.get("id"),
                task_key: row.get("task_key"),
                title: row.get("title"),
                status: row.get("status"),
                owner_agent_id: row.get("owner_agent_id"),
                claimed_by_agent_id: row.get("claimed_by_agent_id"),
            })
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V3 does not support MySQL")),
    }
}

pub(crate) async fn set_team_v3_task_execution_state(
    runtime_pool: &DatabasePool,
    session_id: &str,
    task_id: &str,
    status: &str,
    claimed_by_agent_id: Option<&str>,
    last_error: Option<&str>,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let claim_expires_at = if status == "running" {
        Some((Utc::now() + chrono::Duration::minutes(20)).to_rfc3339())
    } else {
        None
    };
    let raw_metadata: Option<String> = match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT metadata
                   FROM team_v3_tasks
                   WHERE session_id = ? AND id = ?"#,
            )
            .bind(session_id)
            .bind(task_id)
            .fetch_optional(pool)
            .await?;
            row.map(|r| r.get("metadata"))
        }
        DatabasePool::PostgreSQL(pool) => {
            let row = sqlx::query(
                r#"SELECT metadata::text as metadata
                   FROM team_v3_tasks
                   WHERE session_id = $1 AND id = $2"#,
            )
            .bind(session_id)
            .bind(task_id)
            .fetch_optional(pool)
            .await?;
            row.map(|r| r.get("metadata"))
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    };

    let mut metadata = raw_metadata
        .as_deref()
        .map(parse_state_data_text)
        .unwrap_or_else(|| json!({}));
    if !metadata.is_object() {
        metadata = json!({});
    }
    if let Some(metadata_obj) = metadata.as_object_mut() {
        if let Some(error_text) = last_error.map(str::trim).filter(|value| !value.is_empty()) {
            metadata_obj.insert("last_error".to_string(), json!(error_text));
        } else {
            metadata_obj.remove("last_error");
        }
        if status == "running" {
            metadata_obj.insert("started_at".to_string(), json!(now.clone()));
            metadata_obj.remove("completed_at");
        }
        if matches!(status, "completed" | "failed" | "blocked" | "cancelled") {
            metadata_obj.insert("completed_at".to_string(), json!(now.clone()));
        }
    }
    let metadata_text = serde_json::to_string(&metadata)?;

    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_tasks
                   SET status = ?,
                       claimed_by_agent_id = ?,
                       claim_expires_at = ?,
                       metadata = ?,
                       updated_at = ?
                   WHERE session_id = ? AND id = ?"#,
            )
            .bind(status)
            .bind(claimed_by_agent_id)
            .bind(claim_expires_at.as_deref())
            .bind(&metadata_text)
            .bind(&now)
            .bind(session_id)
            .bind(task_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_tasks
                   SET status = $1,
                       claimed_by_agent_id = $2,
                       claim_expires_at = $3,
                       metadata = $4::jsonb,
                       updated_at = $5
                   WHERE session_id = $6 AND id = $7"#,
            )
            .bind(status)
            .bind(claimed_by_agent_id)
            .bind(claim_expires_at.as_deref())
            .bind(&metadata_text)
            .bind(&now)
            .bind(session_id)
            .bind(task_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }
    Ok(())
}

use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::TimeZone;
use sentinel_db::{database_service::connection_manager::DatabasePool, DatabaseService};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use tauri::State;

use super::team_v4_api::{
    append_team_v4_event_internal, load_harness_run_internal, TeamV4Agent,
    TeamV4AppendEventRequest, TeamV4Event, TeamV4HarnessRun, TeamV4Task,
};
use super::team_v4_mapping::{
    map_agent, map_agent_pg, map_event, map_event_pg, map_harness_run, map_harness_run_pg,
    map_task, map_task_pg,
};
use super::team_v4_schema::ensure_team_v4_schema;

type DbState<'r> = State<'r, Arc<DatabaseService>>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamV4FinishHarnessRequest {
    pub status: String,
    pub checkpoint_sequence: Option<i64>,
    pub error: Option<String>,
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamV4ConversationHarnessSnapshot {
    pub agents: Vec<TeamV4Agent>,
    pub events: Vec<TeamV4Event>,
    pub harness_runs: Vec<TeamV4HarnessRun>,
    pub tasks: Vec<TeamV4Task>,
}

fn normalize_terminal_harness_status(status: &str) -> Result<&'static str> {
    match status.trim() {
        "completed" => Ok("completed"),
        "failed" => Ok("failed"),
        "cancelled" => Ok("cancelled"),
        _ => Err(anyhow!("unsupported terminal Team V4 harness status")),
    }
}

fn terminal_harness_event_type(status: &str) -> &'static str {
    match status {
        "completed" => "harness_completed",
        "failed" => "harness_failed",
        "cancelled" => "harness_cancelled",
        _ => "harness_finished",
    }
}

async fn update_harness_terminal_state(
    runtime_pool: &DatabasePool,
    harness_run_id: &str,
    status: &str,
    checkpoint_sequence: Option<i64>,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let checkpoint_sequence = checkpoint_sequence.map(|value| value.max(0));
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v4_harness_runs
                   SET status = ?,
                       checkpoint_sequence = COALESCE(?, checkpoint_sequence),
                       lease_expires_at = NULL,
                       updated_at = ?
                   WHERE id = ?"#,
            )
            .bind(status)
            .bind(checkpoint_sequence)
            .bind(&now)
            .bind(harness_run_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v4_harness_runs
                   SET status = $1,
                       checkpoint_sequence = COALESCE($2, checkpoint_sequence),
                       lease_expires_at = NULL,
                       updated_at = $3
                   WHERE id = $4"#,
            )
            .bind(status)
            .bind(checkpoint_sequence)
            .bind(&now)
            .bind(harness_run_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V4 does not support MySQL")),
    }
    Ok(())
}

async fn update_expired_harness_state(
    runtime_pool: &DatabasePool,
    harness_run_id: &str,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v4_harness_runs
                   SET status = 'expired', lease_expires_at = NULL, updated_at = ?
                   WHERE id = ? AND status = 'running'"#,
            )
            .bind(&now)
            .bind(harness_run_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v4_harness_runs
                   SET status = 'expired', lease_expires_at = NULL, updated_at = $1
                   WHERE id = $2 AND status = 'running'"#,
            )
            .bind(&now)
            .bind(harness_run_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V4 does not support MySQL")),
    }
    Ok(())
}

async fn update_harness_task_terminal_state(
    runtime_pool: &DatabasePool,
    task_id: &str,
    status: &str,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query("UPDATE team_v4_tasks SET status = ?, updated_at = ? WHERE id = ?")
                .bind(status)
                .bind(&now)
                .bind(task_id)
                .execute(pool)
                .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query("UPDATE team_v4_tasks SET status = $1, updated_at = $2 WHERE id = $3")
                .bind(status)
                .bind(&now)
                .bind(task_id)
                .execute(pool)
                .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V4 does not support MySQL")),
    }
    Ok(())
}

#[tauri::command]
pub async fn team_v4_finish_harness_run(
    db: DbState<'_>,
    harness_run_id: String,
    request: TeamV4FinishHarnessRequest,
) -> Result<TeamV4HarnessRun, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;

    let status = normalize_terminal_harness_status(&request.status).map_err(|e| e.to_string())?;
    update_harness_terminal_state(
        &runtime_pool,
        &harness_run_id,
        status,
        request.checkpoint_sequence,
    )
    .await
    .map_err(|e| e.to_string())?;

    let harness = load_harness_run_internal(&runtime_pool, &harness_run_id)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(task_id) = harness.task_id.as_deref() {
        update_harness_task_terminal_state(&runtime_pool, task_id, status)
            .await
            .map_err(|e| e.to_string())?;
    }
    append_team_v4_event_internal(
        &runtime_pool,
        &harness.run_id,
        TeamV4AppendEventRequest {
            actor_id: harness.actor_id.clone(),
            task_id: harness.task_id.clone(),
            event_type: terminal_harness_event_type(status).to_string(),
            visibility: Some("workspace".to_string()),
            payload: Some(json!({
                "harness_run_id": harness.id,
                "status": harness.status,
                "checkpoint_sequence": harness.checkpoint_sequence,
                "error": request.error,
                "details": request.payload.unwrap_or_else(|| json!({})),
            })),
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(harness)
}

async fn expired_running_harnesses(
    runtime_pool: &DatabasePool,
    conversation_id: Option<&str>,
) -> Result<Vec<TeamV4HarnessRun>> {
    let now = chrono::Utc::now().to_rfc3339();
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = if let Some(conversation_id) = conversation_id {
                sqlx::query(
                    r#"SELECT h.id, h.run_id, h.actor_id, h.task_id, h.status, h.lease_expires_at,
                              h.last_heartbeat_at, h.checkpoint_sequence, h.metadata, h.created_at, h.updated_at
                       FROM team_v4_harness_runs h
                       JOIN team_v4_runs r ON r.id = h.run_id
                       WHERE r.conversation_id = ? AND h.status = 'running'
                         AND h.lease_expires_at IS NOT NULL AND h.lease_expires_at < ?"#,
                )
                .bind(conversation_id)
                .bind(&now)
                .fetch_all(pool)
                .await?
            } else {
                sqlx::query(
                    r#"SELECT id, run_id, actor_id, task_id, status, lease_expires_at,
                              last_heartbeat_at, checkpoint_sequence, metadata, created_at, updated_at
                       FROM team_v4_harness_runs
                       WHERE status = 'running'
                         AND lease_expires_at IS NOT NULL AND lease_expires_at < ?"#,
                )
                .bind(&now)
                .fetch_all(pool)
                .await?
            };
            rows.into_iter().map(map_harness_run).collect()
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = if let Some(conversation_id) = conversation_id {
                sqlx::query(
                    r#"SELECT h.id, h.run_id, h.actor_id, h.task_id, h.status,
                              h.lease_expires_at::text as lease_expires_at,
                              h.last_heartbeat_at::text as last_heartbeat_at,
                              h.checkpoint_sequence, h.metadata::text as metadata,
                              h.created_at::text as created_at, h.updated_at::text as updated_at
                       FROM team_v4_harness_runs h
                       JOIN team_v4_runs r ON r.id = h.run_id
                       WHERE r.conversation_id = $1 AND h.status = 'running'
                         AND h.lease_expires_at IS NOT NULL AND h.lease_expires_at < $2"#,
                )
                .bind(conversation_id)
                .bind(&now)
                .fetch_all(pool)
                .await?
            } else {
                sqlx::query(
                    r#"SELECT id, run_id, actor_id, task_id, status,
                              lease_expires_at::text as lease_expires_at,
                              last_heartbeat_at::text as last_heartbeat_at,
                              checkpoint_sequence, metadata::text as metadata,
                              created_at::text as created_at, updated_at::text as updated_at
                       FROM team_v4_harness_runs
                       WHERE status = 'running'
                         AND lease_expires_at IS NOT NULL AND lease_expires_at < $1"#,
                )
                .bind(&now)
                .fetch_all(pool)
                .await?
            };
            rows.into_iter().map(map_harness_run_pg).collect()
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V4 does not support MySQL")),
    }
}

async fn mark_expired_harnesses(
    runtime_pool: &DatabasePool,
    conversation_id: Option<&str>,
) -> Result<u64> {
    let expired = expired_running_harnesses(runtime_pool, conversation_id).await?;
    let mut updated = 0_u64;
    for harness in expired {
        update_expired_harness_state(runtime_pool, &harness.id).await?;
        append_team_v4_event_internal(
            runtime_pool,
            &harness.run_id,
            TeamV4AppendEventRequest {
                actor_id: harness.actor_id.clone(),
                task_id: harness.task_id.clone(),
                event_type: "harness_expired".to_string(),
                visibility: Some("workspace".to_string()),
                payload: Some(json!({
                    "harness_run_id": harness.id,
                    "lease_expires_at": harness.lease_expires_at,
                    "last_heartbeat_at": harness.last_heartbeat_at,
                    "checkpoint_sequence": harness.checkpoint_sequence,
                })),
            },
        )
        .await?;
        updated += 1;
    }
    Ok(updated)
}

#[tauri::command]
pub async fn team_v4_mark_expired_harness_runs(
    db: DbState<'_>,
    conversation_id: Option<String>,
) -> Result<u64, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    mark_expired_harnesses(&runtime_pool, conversation_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

async fn run_ids_for_conversation(
    runtime_pool: &DatabasePool,
    conversation_id: &str,
) -> Result<Vec<String>> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                "SELECT id FROM team_v4_runs WHERE conversation_id = ? ORDER BY updated_at DESC",
            )
            .bind(conversation_id)
            .fetch_all(pool)
            .await?;
            Ok(rows.into_iter().map(|row| row.get("id")).collect())
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                "SELECT id FROM team_v4_runs WHERE conversation_id = $1 ORDER BY updated_at DESC",
            )
            .bind(conversation_id)
            .fetch_all(pool)
            .await?;
            Ok(rows.into_iter().map(|row| row.get("id")).collect())
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V4 does not support MySQL")),
    }
}

async fn list_agents_for_run(
    runtime_pool: &DatabasePool,
    run_id: &str,
) -> Result<Vec<TeamV4Agent>> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, profile_id, role_type, name, status, model, context_mode,
                          tool_policy_json, metadata, created_at, updated_at
                   FROM team_v4_agents WHERE run_id = ? ORDER BY created_at ASC"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await?;
            rows.into_iter().map(map_agent).collect()
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, profile_id, role_type, name, status, model, context_mode,
                          tool_policy_json::text as tool_policy_json, metadata::text as metadata,
                          created_at::text as created_at, updated_at::text as updated_at
                   FROM team_v4_agents WHERE run_id = $1 ORDER BY created_at ASC"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await?;
            rows.into_iter().map(map_agent_pg).collect()
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V4 does not support MySQL")),
    }
}

async fn list_tasks_for_run(runtime_pool: &DatabasePool, run_id: &str) -> Result<Vec<TeamV4Task>> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, parent_task_id, task_key, title, instruction, status, priority,
                          assigned_agent_id, depends_on, acceptance_criteria, context_snapshot_id,
                          metadata, created_at, updated_at
                   FROM team_v4_tasks WHERE run_id = ? ORDER BY priority ASC, created_at ASC"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await?;
            rows.into_iter().map(map_task).collect()
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, parent_task_id, task_key, title, instruction, status, priority,
                          assigned_agent_id, depends_on::text as depends_on, acceptance_criteria,
                          context_snapshot_id, metadata::text as metadata,
                          created_at::text as created_at, updated_at::text as updated_at
                   FROM team_v4_tasks WHERE run_id = $1 ORDER BY priority ASC, created_at ASC"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await?;
            rows.into_iter().map(map_task_pg).collect()
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V4 does not support MySQL")),
    }
}

async fn list_events_for_run(
    runtime_pool: &DatabasePool,
    run_id: &str,
) -> Result<Vec<TeamV4Event>> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, sequence, actor_id, task_id, event_type, visibility, payload, created_at
                   FROM team_v4_events WHERE run_id = ? ORDER BY sequence ASC LIMIT 500"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await?;
            rows.into_iter().map(map_event).collect()
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, sequence, actor_id, task_id, event_type, visibility,
                          payload::text as payload, created_at::text as created_at
                   FROM team_v4_events WHERE run_id = $1 ORDER BY sequence ASC LIMIT 500"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await?;
            rows.into_iter().map(map_event_pg).collect()
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V4 does not support MySQL")),
    }
}

async fn list_harness_for_run(
    runtime_pool: &DatabasePool,
    run_id: &str,
) -> Result<Vec<TeamV4HarnessRun>> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, actor_id, task_id, status, lease_expires_at,
                          last_heartbeat_at, checkpoint_sequence, metadata, created_at, updated_at
                   FROM team_v4_harness_runs WHERE run_id = ? ORDER BY created_at ASC"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await?;
            rows.into_iter().map(map_harness_run).collect()
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, actor_id, task_id, status,
                          lease_expires_at::text as lease_expires_at,
                          last_heartbeat_at::text as last_heartbeat_at,
                          checkpoint_sequence, metadata::text as metadata,
                          created_at::text as created_at, updated_at::text as updated_at
                   FROM team_v4_harness_runs WHERE run_id = $1 ORDER BY created_at ASC"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await?;
            rows.into_iter().map(map_harness_run_pg).collect()
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V4 does not support MySQL")),
    }
}

#[tauri::command]
pub async fn team_v4_conversation_harness_snapshot(
    db: DbState<'_>,
    conversation_id: String,
) -> Result<TeamV4ConversationHarnessSnapshot, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    mark_expired_harnesses(&runtime_pool, Some(&conversation_id))
        .await
        .map_err(|e| e.to_string())?;
    let run_ids = run_ids_for_conversation(&runtime_pool, &conversation_id)
        .await
        .map_err(|e| e.to_string())?;
    let mut agents = Vec::new();
    let mut events = Vec::new();
    let mut harness_runs = Vec::new();
    let mut tasks = Vec::new();
    for run_id in run_ids {
        agents.extend(
            list_agents_for_run(&runtime_pool, &run_id)
                .await
                .map_err(|e| e.to_string())?,
        );
        tasks.extend(
            list_tasks_for_run(&runtime_pool, &run_id)
                .await
                .map_err(|e| e.to_string())?,
        );
        events.extend(
            list_events_for_run(&runtime_pool, &run_id)
                .await
                .map_err(|e| e.to_string())?,
        );
        harness_runs.extend(
            list_harness_for_run(&runtime_pool, &run_id)
                .await
                .map_err(|e| e.to_string())?,
        );
    }
    Ok(TeamV4ConversationHarnessSnapshot {
        agents,
        events,
        harness_runs,
        tasks,
    })
}

#[tauri::command]
pub async fn prune_team_v4_runs_after(
    db: DbState<'_>,
    conversation_id: String,
    after_timestamp_ms: i64,
) -> Result<u64, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let timestamp = chrono::Utc
        .timestamp_millis_opt(after_timestamp_ms)
        .single()
        .ok_or_else(|| "Invalid replay timestamp".to_string())?
        .to_rfc3339();
    let result = match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query("DELETE FROM team_v4_runs WHERE conversation_id = ? AND created_at > ?")
                .bind(&conversation_id)
                .bind(&timestamp)
                .execute(pool)
                .await
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query("DELETE FROM team_v4_runs WHERE conversation_id = $1 AND created_at > $2")
                .bind(&conversation_id)
                .bind(&timestamp)
                .execute(pool)
                .await
        }
        DatabasePool::MySQL(_) => return Err("Team V4 does not support MySQL".to_string()),
    }
    .map_err(|e| e.to_string())?;
    Ok(result.rows_affected())
}

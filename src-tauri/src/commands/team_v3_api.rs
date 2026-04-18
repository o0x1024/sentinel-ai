use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Utc;
use sentinel_db::{database_service::connection_manager::DatabasePool, DatabaseService};
use serde_json::{json, Value};
use sqlx::Row;
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::agents::tool_router::ToolConfig;
use crate::services::ai::AiServiceManager;

use super::team_v3_commands::{
    cancel_team_execution, clear_team_execution_cancellation, create_team_execution_cancellation,
    is_team_execution_cancelled, list_team_v3_blackboard_entries, list_team_v3_tasks_internal,
    run_team_v3_execution_orchestrator, TeamV3BlackboardEntry, TeamV3ClaimTaskRequest,
    TeamV3CreateSessionRequest, TeamV3CreateTaskRequest, TeamV3PlanRevision,
    TeamV3ReviewPlanRevisionRequest, TeamV3RunStatus, TeamV3SendMessageRequest, TeamV3Session,
    TeamV3SubmitPlanRevisionRequest, TeamV3Task, TeamV3TaskActionResult, TeamV3ThreadMessage,
    TeamV3UpdateSessionRequest, TeamV3UpdateTaskStatusRequest,
};
use super::team_v3_schema::ensure_team_v3_schema;
use super::team_v3_session_state::{
    append_team_v3_status_message, apply_team_v3_execution_outcome, build_team_state_data, first_member_id,
    get_team_v3_latest_human_message_content, get_team_v3_session_context,
    get_team_v3_session_state_data, parse_state_data_text, set_team_v3_session_conversation_id,
    set_team_v3_session_state, set_team_v3_session_state_data,
};
use super::team_v3_task_notices::append_team_v3_dependency_ready_notices;
use super::team_v3_task_state::{get_team_v3_task_action_result, set_team_v3_task_execution_state};

type DbState<'r> = State<'r, Arc<DatabaseService>>;
type AiState<'r> = State<'r, Arc<AiServiceManager>>;

#[tauri::command]
pub async fn team_v3_create_session(
    db: DbState<'_>,
    request: TeamV3CreateSessionRequest,
) -> Result<TeamV3Session, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    tracing::info!(
        "team_v3_create_session: conversation_id={:?}, name={}",
        request.conversation_id,
        request.name
    );
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let state = "PLAN_DRAFT".to_string();
    let state_data = build_team_state_data(None, None);
    let state_data_text = serde_json::to_string(&state_data).map_err(|e| e.to_string())?;

    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_sessions
                   (id, conversation_id, name, goal, state, state_data, created_at, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(&request.conversation_id)
            .bind(&request.name)
            .bind(&request.goal)
            .bind(&state)
            .bind(&state_data_text)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_sessions
                   (id, conversation_id, name, goal, state, state_data, created_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $8)"#,
            )
            .bind(&id)
            .bind(&request.conversation_id)
            .bind(&request.name)
            .bind(&request.goal)
            .bind(&state)
            .bind(&state_data_text)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => return Err("Team V3 does not support MySQL".to_string()),
    }

    Ok(TeamV3Session {
        id,
        conversation_id: request.conversation_id,
        name: request.name,
        goal: request.goal,
        state,
        state_data,
        created_at: now.clone(),
        updated_at: now,
    })
}

#[tauri::command]
pub async fn team_v3_get_session(
    db: DbState<'_>,
    session_id: String,
) -> Result<Option<TeamV3Session>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT id, conversation_id, name, goal, state, state_data, created_at, updated_at
                   FROM team_v3_sessions WHERE id = ?"#,
            )
            .bind(&session_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;
            Ok(row.map(|r| {
                let state_data_text: String = r.get("state_data");
                TeamV3Session {
                    id: r.get("id"),
                    conversation_id: r.get("conversation_id"),
                    name: r.get("name"),
                    goal: r.get("goal"),
                    state: r.get("state"),
                    state_data: parse_state_data_text(&state_data_text),
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                }
            }))
        }
        DatabasePool::PostgreSQL(pool) => {
            let row = sqlx::query(
                r#"SELECT id, conversation_id, name, goal, state,
                          state_data::text as state_data,
                          created_at::text as created_at, updated_at::text as updated_at
                   FROM team_v3_sessions WHERE id = $1"#,
            )
            .bind(&session_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;
            Ok(row.map(|r| {
                let state_data_text: String = r.get("state_data");
                TeamV3Session {
                    id: r.get("id"),
                    conversation_id: r.get("conversation_id"),
                    name: r.get("name"),
                    goal: r.get("goal"),
                    state: r.get("state"),
                    state_data: parse_state_data_text(&state_data_text),
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                }
            }))
        }
        DatabasePool::MySQL(_) => Err("Team V3 does not support MySQL".to_string()),
    }
}

#[tauri::command]
pub async fn team_v3_list_sessions(
    db: DbState<'_>,
    conversation_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<TeamV3Session>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let lim = limit.unwrap_or(20).max(1);
    let off = offset.unwrap_or(0).max(0);
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = if let Some(cid) = conversation_id.as_ref() {
                sqlx::query(
                    r#"SELECT id, conversation_id, name, goal, state, state_data, created_at, updated_at
                       FROM team_v3_sessions
                       WHERE conversation_id = ?
                       ORDER BY updated_at DESC
                       LIMIT ? OFFSET ?"#,
                )
                .bind(cid)
                .bind(lim)
                .bind(off)
                .fetch_all(pool)
                .await
                .map_err(|e| e.to_string())?
            } else {
                sqlx::query(
                    r#"SELECT id, conversation_id, name, goal, state, state_data, created_at, updated_at
                       FROM team_v3_sessions
                       ORDER BY updated_at DESC
                       LIMIT ? OFFSET ?"#,
                )
                .bind(lim)
                .bind(off)
                .fetch_all(pool)
                .await
                .map_err(|e| e.to_string())?
            };
            Ok(rows
                .into_iter()
                .map(|r| {
                    let state_data_text: String = r.get("state_data");
                    TeamV3Session {
                        id: r.get("id"),
                        conversation_id: r.get("conversation_id"),
                        name: r.get("name"),
                        goal: r.get("goal"),
                        state: r.get("state"),
                        state_data: parse_state_data_text(&state_data_text),
                        created_at: r.get("created_at"),
                        updated_at: r.get("updated_at"),
                    }
                })
                .collect())
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = if let Some(cid) = conversation_id.as_ref() {
                sqlx::query(
                    r#"SELECT id, conversation_id, name, goal, state,
                              state_data::text as state_data,
                              created_at::text as created_at, updated_at::text as updated_at
                       FROM team_v3_sessions
                       WHERE conversation_id = $1
                       ORDER BY updated_at DESC
                       LIMIT $2 OFFSET $3"#,
                )
                .bind(cid)
                .bind(lim)
                .bind(off)
                .fetch_all(pool)
                .await
                .map_err(|e| e.to_string())?
            } else {
                sqlx::query(
                    r#"SELECT id, conversation_id, name, goal, state,
                              state_data::text as state_data,
                              created_at::text as created_at, updated_at::text as updated_at
                       FROM team_v3_sessions
                       ORDER BY updated_at DESC
                       LIMIT $1 OFFSET $2"#,
                )
                .bind(lim)
                .bind(off)
                .fetch_all(pool)
                .await
                .map_err(|e| e.to_string())?
            };
            Ok(rows
                .into_iter()
                .map(|r| {
                    let state_data_text: String = r.get("state_data");
                    TeamV3Session {
                        id: r.get("id"),
                        conversation_id: r.get("conversation_id"),
                        name: r.get("name"),
                        goal: r.get("goal"),
                        state: r.get("state"),
                        state_data: parse_state_data_text(&state_data_text),
                        created_at: r.get("created_at"),
                        updated_at: r.get("updated_at"),
                    }
                })
                .collect())
        }
        DatabasePool::MySQL(_) => Err("Team V3 does not support MySQL".to_string()),
    }
}

#[tauri::command]
pub async fn team_v3_update_session(
    db: DbState<'_>,
    session_id: String,
    request: TeamV3UpdateSessionRequest,
) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    let state_data_text = request
        .state_data
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()));
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET name = COALESCE(?, name),
                       goal = COALESCE(?, goal),
                       state = COALESCE(?, state),
                       state_data = COALESCE(?, state_data),
                       updated_at = ?
                   WHERE id = ?"#,
            )
            .bind(&request.name)
            .bind(&request.goal)
            .bind(&request.state)
            .bind(&state_data_text)
            .bind(&now)
            .bind(&session_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET name = COALESCE($1, name),
                       goal = COALESCE($2, goal),
                       state = COALESCE($3, state),
                       state_data = COALESCE($4::jsonb, state_data),
                       updated_at = $5
                   WHERE id = $6"#,
            )
            .bind(&request.name)
            .bind(&request.goal)
            .bind(&request.state)
            .bind(&state_data_text)
            .bind(&now)
            .bind(&session_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => return Err("Team V3 does not support MySQL".to_string()),
    }
    Ok(())
}

#[tauri::command]
pub async fn team_v3_start_execution(
    db: DbState<'_>,
    session_id: String,
    conversation_id: Option<String>,
    rag_enabled: Option<bool>,
    tool_config: Option<ToolConfig>,
    app_handle: AppHandle,
    ai_manager: AiState<'_>,
) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;

    let now = Utc::now().to_rfc3339();
    set_team_v3_session_state(&runtime_pool, &session_id, "EXECUTING", &now)
        .await
        .map_err(|e| e.to_string())?;

    let current_state_data = get_team_v3_session_state_data(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())?;
    let lead_member_id = first_member_id(&current_state_data);
    let next_state_data = build_team_state_data(Some(&current_state_data), Some(&lead_member_id));
    set_team_v3_session_state_data(&runtime_pool, &session_id, &next_state_data, &now)
        .await
        .map_err(|e| e.to_string())?;

    let (stored_conversation_id_opt, goal_opt) =
        get_team_v3_session_context(&runtime_pool, &session_id)
            .await
            .map_err(|e| e.to_string())?;
    let conversation_id = conversation_id
        .filter(|value| !value.trim().is_empty())
        .or(stored_conversation_id_opt)
        .unwrap_or_else(|| session_id.clone());
    if let Err(e) =
        set_team_v3_session_conversation_id(&runtime_pool, &session_id, &conversation_id).await
    {
        tracing::warn!(
            "team_v3_start_execution: failed to backfill conversation_id for session {}: {}",
            session_id,
            e
        );
    }
    let task = get_team_v3_latest_human_message_content(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())?
        .or_else(|| goal_opt.clone().filter(|g| !g.trim().is_empty()))
        .unwrap_or_else(|| "请继续当前 Team 任务并输出最新进展与结论。".to_string());

    let (generation, cancellation_token) = create_team_execution_cancellation(&session_id);
    let ai_manager = ai_manager.inner().clone();
    let runtime_pool_for_spawn = runtime_pool.clone();
    let app_handle_for_spawn = app_handle.clone();
    let session_id_for_spawn = session_id.clone();
    let goal_for_spawn = goal_opt
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| task.clone());
    let user_input_for_spawn = task.clone();
    let state_data_for_spawn = next_state_data.clone();
    let rag_enabled_for_spawn = rag_enabled.unwrap_or(true);
    let tool_config_for_spawn = tool_config.clone();
    tokio::spawn(async move {
        let run_result = run_team_v3_execution_orchestrator(
            runtime_pool_for_spawn.clone(),
            app_handle_for_spawn,
            ai_manager,
            session_id_for_spawn.clone(),
            generation,
            cancellation_token.clone(),
            goal_for_spawn,
            user_input_for_spawn,
            state_data_for_spawn,
            rag_enabled_for_spawn,
            tool_config_for_spawn,
        )
        .await;

        if is_team_execution_cancelled(&session_id_for_spawn, generation)
            || cancellation_token.is_cancelled()
        {
            clear_team_execution_cancellation(&session_id_for_spawn, generation);
            return;
        }

        match run_result {
            Ok(summary) => {
                if let Err(e) = apply_team_v3_execution_outcome(
                    &runtime_pool_for_spawn,
                    &session_id_for_spawn,
                    true,
                    Some(summary.as_str()),
                )
                .await
                {
                    tracing::warn!(
                        "team_v3_start_execution: failed to apply success outcome for session {}: {}",
                        session_id_for_spawn,
                        e
                    );
                }
            }
            Err(e) => {
                let error_summary = e.to_string();
                if let Err(outcome_err) = apply_team_v3_execution_outcome(
                    &runtime_pool_for_spawn,
                    &session_id_for_spawn,
                    false,
                    Some(error_summary.as_str()),
                )
                .await
                {
                    tracing::warn!(
                        "team_v3_start_execution: failed to apply failure outcome for session {}: {}",
                        session_id_for_spawn,
                        outcome_err
                    );
                }
            }
        }

        clear_team_execution_cancellation(&session_id_for_spawn, generation);
    });

    tracing::info!(
        "team_v3_start_execution started orchestrator: session_id={}, conversation_id={}, generation={}",
        session_id,
        conversation_id,
        generation
    );
    Ok(())
}

#[tauri::command]
pub async fn team_v3_stop_execution(db: DbState<'_>, session_id: String) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    cancel_team_execution(&session_id);

    let (conversation_id_opt, _) = get_team_v3_session_context(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(conversation_id) = conversation_id_opt {
        crate::commands::ai::cancel_conversation_stream(&conversation_id);
    }

    if let Err(e) = apply_team_v3_execution_outcome(
        &runtime_pool,
        &session_id,
        false,
        Some("已由用户手动停止。"),
    )
    .await
    {
        tracing::warn!(
            "team_v3_stop_execution: failed to apply stop outcome for session {}: {}",
            session_id,
            e
        );
    }

    Ok(())
}

#[tauri::command]
pub async fn team_v3_finalize_execution(
    db: DbState<'_>,
    session_id: String,
    success: bool,
    summary: Option<String>,
) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    apply_team_v3_execution_outcome(&runtime_pool, &session_id, success, summary.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn team_v3_get_run_status(
    db: DbState<'_>,
    session_id: String,
) -> Result<Option<TeamV3RunStatus>, String> {
    Ok(team_v3_get_session(db, session_id)
        .await?
        .map(|s| TeamV3RunStatus {
            session_id: s.id,
            state: s.state,
        }))
}

#[tauri::command]
pub async fn team_v3_create_task(
    db: DbState<'_>,
    session_id: String,
    request: TeamV3CreateTaskRequest,
) -> Result<TeamV3Task, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let status = "pending".to_string();
    let priority = request.priority.unwrap_or(100);
    let metadata = request.metadata.unwrap_or_else(|| json!({}));
    let metadata_text = serde_json::to_string(&metadata).map_err(|e| e.to_string())?;

    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_tasks
                   (id, session_id, task_key, title, instruction, status, priority,
                    owner_agent_id, acceptance_criteria, metadata, created_at, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(&session_id)
            .bind(&request.task_key)
            .bind(&request.title)
            .bind(&request.instruction)
            .bind(&status)
            .bind(priority)
            .bind(&request.owner_agent_id)
            .bind(&request.acceptance_criteria)
            .bind(&metadata_text)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_tasks
                   (id, session_id, task_key, title, instruction, status, priority,
                    owner_agent_id, acceptance_criteria, metadata, created_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb, $11, $12)"#,
            )
            .bind(&id)
            .bind(&session_id)
            .bind(&request.task_key)
            .bind(&request.title)
            .bind(&request.instruction)
            .bind(&status)
            .bind(priority)
            .bind(&request.owner_agent_id)
            .bind(&request.acceptance_criteria)
            .bind(&metadata_text)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => return Err("Team V3 does not support MySQL".to_string()),
    }

    Ok(TeamV3Task {
        id,
        session_id,
        task_key: request.task_key,
        title: request.title,
        instruction: request.instruction,
        status,
        priority,
        owner_agent_id: request.owner_agent_id,
        claimed_by_agent_id: None,
        claim_expires_at: None,
        acceptance_criteria: request.acceptance_criteria,
        metadata,
        created_at: now.clone(),
        updated_at: now,
    })
}

#[tauri::command]
pub async fn team_v3_list_tasks(
    db: DbState<'_>,
    session_id: String,
) -> Result<Vec<TeamV3Task>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    list_team_v3_tasks_internal(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())
}

async fn get_task_for_claim(
    pool: &DatabasePool,
    session_id: &str,
    task_id: &str,
) -> Result<(String, Option<String>, Option<String>)> {
    match pool {
        DatabasePool::SQLite(sqlite) => {
            let row = sqlx::query(
                r#"SELECT status, claimed_by_agent_id, claim_expires_at
                   FROM team_v3_tasks WHERE session_id = ? AND id = ?"#,
            )
            .bind(session_id)
            .bind(task_id)
            .fetch_optional(sqlite)
            .await?;
            let row = row.ok_or_else(|| anyhow!("task not found"))?;
            Ok((
                row.get("status"),
                row.get("claimed_by_agent_id"),
                row.get("claim_expires_at"),
            ))
        }
        DatabasePool::PostgreSQL(pg) => {
            let row = sqlx::query(
                r#"SELECT status, claimed_by_agent_id, claim_expires_at::text as claim_expires_at
                   FROM team_v3_tasks WHERE session_id = $1 AND id = $2"#,
            )
            .bind(session_id)
            .bind(task_id)
            .fetch_optional(pg)
            .await?;
            let row = row.ok_or_else(|| anyhow!("task not found"))?;
            Ok((
                row.get("status"),
                row.get("claimed_by_agent_id"),
                row.get("claim_expires_at"),
            ))
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V3 does not support MySQL")),
    }
}

#[tauri::command]
pub async fn team_v3_claim_task(
    db: DbState<'_>,
    session_id: String,
    task_id: String,
    request: TeamV3ClaimTaskRequest,
) -> Result<TeamV3TaskActionResult, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let ttl = request.ttl_secs.unwrap_or(600).max(30);
    let now = Utc::now();
    let expires_at = now + chrono::Duration::seconds(ttl);
    let expires_text = expires_at.to_rfc3339();

    let (status, claimed_by, claim_expires_at) =
        get_task_for_claim(&runtime_pool, &session_id, &task_id)
            .await
            .map_err(|e| e.to_string())?;

    let claim_is_active = claim_expires_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc) > now)
        .unwrap_or(false);

    if status == "completed" || status == "failed" || status == "cancelled" {
        return Err("Task is terminal and cannot be claimed".to_string());
    }
    if claim_is_active && claimed_by.as_deref() != Some(request.agent_id.as_str()) {
        return Err("Task is already claimed by another agent".to_string());
    }

    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_tasks
                   SET status = 'claimed',
                       claimed_by_agent_id = ?,
                       claim_expires_at = ?,
                       lock_version = lock_version + 1,
                       updated_at = ?
                   WHERE session_id = ? AND id = ?"#,
            )
            .bind(&request.agent_id)
            .bind(&expires_text)
            .bind(now.to_rfc3339())
            .bind(&session_id)
            .bind(&task_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
            sqlx::query(
                r#"INSERT INTO team_v3_task_claims
                   (id, session_id, task_id, agent_id, action, ttl_secs, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&session_id)
            .bind(&task_id)
            .bind(&request.agent_id)
            .bind("claim")
            .bind(ttl)
            .bind(now.to_rfc3339())
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_tasks
                   SET status = 'claimed',
                       claimed_by_agent_id = $1,
                       claim_expires_at = $2,
                       lock_version = lock_version + 1,
                       updated_at = $3
                   WHERE session_id = $4 AND id = $5"#,
            )
            .bind(&request.agent_id)
            .bind(&expires_text)
            .bind(now.to_rfc3339())
            .bind(&session_id)
            .bind(&task_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
            sqlx::query(
                r#"INSERT INTO team_v3_task_claims
                   (id, session_id, task_id, agent_id, action, ttl_secs, created_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&session_id)
            .bind(&task_id)
            .bind(&request.agent_id)
            .bind("claim")
            .bind(ttl)
            .bind(now.to_rfc3339())
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => return Err("Team V3 does not support MySQL".to_string()),
    }

    get_team_v3_task_action_result(&runtime_pool, &session_id, &task_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn team_v3_release_task_claim(
    db: DbState<'_>,
    session_id: String,
    task_id: String,
    agent_id: String,
) -> Result<TeamV3TaskActionResult, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            let updated = sqlx::query(
                r#"UPDATE team_v3_tasks
                   SET status = 'ready_for_claim',
                       claimed_by_agent_id = NULL,
                       claim_expires_at = NULL,
                       lock_version = lock_version + 1,
                       updated_at = ?
                   WHERE session_id = ? AND id = ? AND claimed_by_agent_id = ?"#,
            )
            .bind(&now)
            .bind(&session_id)
            .bind(&task_id)
            .bind(&agent_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
            if updated.rows_affected() == 0 {
                return Err("Task is not currently claimed by this agent".to_string());
            }
            sqlx::query(
                r#"INSERT INTO team_v3_task_claims
                   (id, session_id, task_id, agent_id, action, created_at)
                   VALUES (?, ?, ?, ?, ?, ?)"#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&session_id)
            .bind(&task_id)
            .bind(&agent_id)
            .bind("release")
            .bind(&now)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            let updated = sqlx::query(
                r#"UPDATE team_v3_tasks
                   SET status = 'ready_for_claim',
                       claimed_by_agent_id = NULL,
                       claim_expires_at = NULL,
                       lock_version = lock_version + 1,
                       updated_at = $1
                   WHERE session_id = $2 AND id = $3 AND claimed_by_agent_id = $4"#,
            )
            .bind(&now)
            .bind(&session_id)
            .bind(&task_id)
            .bind(&agent_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
            if updated.rows_affected() == 0 {
                return Err("Task is not currently claimed by this agent".to_string());
            }
            sqlx::query(
                r#"INSERT INTO team_v3_task_claims
                   (id, session_id, task_id, agent_id, action, created_at)
                   VALUES ($1, $2, $3, $4, $5, $6)"#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&session_id)
            .bind(&task_id)
            .bind(&agent_id)
            .bind("release")
            .bind(&now)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => return Err("Team V3 does not support MySQL".to_string()),
    }

    get_team_v3_task_action_result(&runtime_pool, &session_id, &task_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn team_v3_update_task_status(
    db: DbState<'_>,
    session_id: String,
    task_id: String,
    request: TeamV3UpdateTaskStatusRequest,
) -> Result<TeamV3TaskActionResult, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;

    let next_status = request.status.trim().to_lowercase();
    if !matches!(next_status.as_str(), "completed" | "failed" | "blocked") {
        return Err("Unsupported task status transition".to_string());
    }

    let previous_tasks = list_team_v3_tasks_internal(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())?;
    if !previous_tasks.iter().any(|task| task.id == task_id) {
        return Err("task not found".to_string());
    }

    set_team_v3_task_execution_state(
        &runtime_pool,
        &session_id,
        &task_id,
        next_status.as_str(),
        None,
        request.last_error.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())?;

    let next_tasks = list_team_v3_tasks_internal(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())?;
    append_team_v3_dependency_ready_notices(
        &runtime_pool,
        &session_id,
        &previous_tasks,
        &next_tasks,
    )
    .await
    .map_err(|e| e.to_string())?;

    let result = get_team_v3_task_action_result(&runtime_pool, &session_id, &task_id)
        .await
        .map_err(|e| e.to_string())?;

    let status_text = match next_status.as_str() {
        "completed" => "已标记为已完成",
        "failed" => "已标记为已失败",
        "blocked" => "已标记为等待依赖",
        _ => "状态已更新",
    };
    let task_label = result
        .title
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or(result.task_key.as_deref())
        .unwrap_or(task_id.as_str());
    let message = match next_status.as_str() {
        "failed" | "blocked" => {
            let detail = request
                .last_error
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| format!("：{}", value))
                .unwrap_or_default();
            format!("任务 {} {}{}", task_label, status_text, detail)
        }
        _ => format!("任务 {} {}", task_label, status_text),
    };
    append_team_v3_status_message(&runtime_pool, &session_id, message.as_str())
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub async fn team_v3_send_message(
    db: DbState<'_>,
    session_id: String,
    request: TeamV3SendMessageRequest,
) -> Result<TeamV3ThreadMessage, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    tracing::info!(
        "team_v3_send_message: session_id={}, thread_id={}, type={}",
        session_id,
        request.thread_id,
        request.message_type.as_deref().unwrap_or("chat")
    );
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let message_type = request.message_type.unwrap_or_else(|| "chat".to_string());
    let payload_text = serde_json::to_string(&request.payload).map_err(|e| e.to_string())?;

    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_messages
                   (id, session_id, thread_id, from_agent_id, to_agent_id, message_type, message_kind, payload, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(&session_id)
            .bind(&request.thread_id)
            .bind(&request.from_agent_id)
            .bind(&request.to_agent_id)
            .bind(&message_type)
            .bind("chat")
            .bind(&payload_text)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_messages
                   (id, session_id, thread_id, from_agent_id, to_agent_id, message_type, message_kind, payload, created_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9)"#,
            )
            .bind(&id)
            .bind(&session_id)
            .bind(&request.thread_id)
            .bind(&request.from_agent_id)
            .bind(&request.to_agent_id)
            .bind(&message_type)
            .bind("chat")
            .bind(&payload_text)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => return Err("Team V3 does not support MySQL".to_string()),
    }

    Ok(TeamV3ThreadMessage {
        id,
        session_id,
        thread_id: request.thread_id,
        from_agent_id: request.from_agent_id,
        to_agent_id: request.to_agent_id,
        message_type,
        payload: request.payload,
        created_at: now,
        sequence: None,
    })
}

#[tauri::command]
pub async fn team_v3_list_thread_messages(
    db: DbState<'_>,
    session_id: String,
    thread_id: String,
) -> Result<Vec<TeamV3ThreadMessage>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, session_id, thread_id, from_agent_id, to_agent_id, message_type, payload, created_at,
                          ROW_NUMBER() OVER (ORDER BY created_at ASC, id ASC) as sequence
                   FROM team_v3_messages
                   WHERE session_id = ? AND thread_id = ?
                   ORDER BY created_at ASC, id ASC"#,
            )
            .bind(&session_id)
            .bind(&thread_id)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;

            rows.into_iter()
                .map(|r| {
                    let payload_text: String = r.get("payload");
                    let payload: Value =
                        serde_json::from_str(&payload_text).unwrap_or_else(|_| json!({}));
                    Ok(TeamV3ThreadMessage {
                        id: r.get("id"),
                        session_id: r.get("session_id"),
                        thread_id: r.get("thread_id"),
                        from_agent_id: r.get("from_agent_id"),
                        to_agent_id: r.get("to_agent_id"),
                        message_type: r.get("message_type"),
                        payload,
                        created_at: r.get("created_at"),
                        sequence: Some(r.get("sequence")),
                    })
                })
                .collect()
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, session_id, thread_id, from_agent_id, to_agent_id, message_type,
                          payload::text as payload, created_at::text as created_at,
                          ROW_NUMBER() OVER (ORDER BY created_at ASC, id ASC) as sequence
                   FROM team_v3_messages
                   WHERE session_id = $1 AND thread_id = $2
                   ORDER BY created_at ASC, id ASC"#,
            )
            .bind(&session_id)
            .bind(&thread_id)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;

            rows.into_iter()
                .map(|r| {
                    let payload_text: String = r.get("payload");
                    let payload: Value =
                        serde_json::from_str(&payload_text).unwrap_or_else(|_| json!({}));
                    Ok(TeamV3ThreadMessage {
                        id: r.get("id"),
                        session_id: r.get("session_id"),
                        thread_id: r.get("thread_id"),
                        from_agent_id: r.get("from_agent_id"),
                        to_agent_id: r.get("to_agent_id"),
                        message_type: r.get("message_type"),
                        payload,
                        created_at: r.get("created_at"),
                        sequence: Some(r.get("sequence")),
                    })
                })
                .collect()
        }
        DatabasePool::MySQL(_) => Err("Team V3 does not support MySQL".to_string()),
    }
}

#[tauri::command]
pub async fn team_v3_list_messages(
    db: DbState<'_>,
    session_id: String,
) -> Result<Vec<TeamV3ThreadMessage>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, session_id, thread_id, from_agent_id, to_agent_id, message_type, payload, created_at,
                          ROW_NUMBER() OVER (ORDER BY created_at ASC, id ASC) as sequence
                   FROM team_v3_messages
                   WHERE session_id = ?
                   ORDER BY created_at ASC, id ASC"#,
            )
            .bind(&session_id)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            Ok(rows
                .into_iter()
                .map(|r| {
                    let payload_text: String = r.get("payload");
                    let payload: Value =
                        serde_json::from_str(&payload_text).unwrap_or_else(|_| json!({}));
                    TeamV3ThreadMessage {
                        id: r.get("id"),
                        session_id: r.get("session_id"),
                        thread_id: r.get("thread_id"),
                        from_agent_id: r.get("from_agent_id"),
                        to_agent_id: r.get("to_agent_id"),
                        message_type: r.get("message_type"),
                        payload,
                        created_at: r.get("created_at"),
                        sequence: Some(r.get("sequence")),
                    }
                })
                .collect())
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, session_id, thread_id, from_agent_id, to_agent_id, message_type,
                          payload::text as payload, created_at::text as created_at,
                          ROW_NUMBER() OVER (ORDER BY created_at ASC, id ASC) as sequence
                   FROM team_v3_messages
                   WHERE session_id = $1
                   ORDER BY created_at ASC, id ASC"#,
            )
            .bind(&session_id)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            Ok(rows
                .into_iter()
                .map(|r| {
                    let payload_text: String = r.get("payload");
                    let payload: Value =
                        serde_json::from_str(&payload_text).unwrap_or_else(|_| json!({}));
                    TeamV3ThreadMessage {
                        id: r.get("id"),
                        session_id: r.get("session_id"),
                        thread_id: r.get("thread_id"),
                        from_agent_id: r.get("from_agent_id"),
                        to_agent_id: r.get("to_agent_id"),
                        message_type: r.get("message_type"),
                        payload,
                        created_at: r.get("created_at"),
                        sequence: Some(r.get("sequence")),
                    }
                })
                .collect())
        }
        DatabasePool::MySQL(_) => Err("Team V3 does not support MySQL".to_string()),
    }
}

#[tauri::command]
pub async fn team_v3_list_blackboard_entries(
    db: DbState<'_>,
    session_id: String,
    limit: Option<i64>,
) -> Result<Vec<TeamV3BlackboardEntry>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    list_team_v3_blackboard_entries(&runtime_pool, &session_id, limit.unwrap_or(100))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn team_v3_submit_plan_revision(
    db: DbState<'_>,
    session_id: String,
    request: TeamV3SubmitPlanRevisionRequest,
) -> Result<TeamV3PlanRevision, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let status = "waiting_approval".to_string();
    let plan_text = serde_json::to_string(&request.plan_json).map_err(|e| e.to_string())?;

    let revision_no: i32 = match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT COALESCE(MAX(revision_no), 0) + 1 AS next_rev
                   FROM team_v3_plan_revisions WHERE session_id = ?"#,
            )
            .bind(&session_id)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;
            row.get("next_rev")
        }
        DatabasePool::PostgreSQL(pool) => {
            let row = sqlx::query(
                r#"SELECT COALESCE(MAX(revision_no), 0) + 1 AS next_rev
                   FROM team_v3_plan_revisions WHERE session_id = $1"#,
            )
            .bind(&session_id)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;
            row.get("next_rev")
        }
        DatabasePool::MySQL(_) => return Err("Team V3 does not support MySQL".to_string()),
    };

    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_plan_revisions
                   (id, session_id, revision_no, plan_json, summary, status, requested_by, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(&session_id)
            .bind(revision_no)
            .bind(&plan_text)
            .bind(&request.summary)
            .bind(&status)
            .bind(&request.requested_by)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;

            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET state = 'WAITING_PLAN_APPROVAL', updated_at = ?
                   WHERE id = ?"#,
            )
            .bind(&now)
            .bind(&session_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_plan_revisions
                   (id, session_id, revision_no, plan_json, summary, status, requested_by, created_at)
                   VALUES ($1, $2, $3, $4::jsonb, $5, $6, $7, $8)"#,
            )
            .bind(&id)
            .bind(&session_id)
            .bind(revision_no)
            .bind(&plan_text)
            .bind(&request.summary)
            .bind(&status)
            .bind(&request.requested_by)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;

            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET state = 'WAITING_PLAN_APPROVAL', updated_at = $1
                   WHERE id = $2"#,
            )
            .bind(&now)
            .bind(&session_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => return Err("Team V3 does not support MySQL".to_string()),
    }

    Ok(TeamV3PlanRevision {
        id,
        session_id,
        revision_no,
        plan_json: request.plan_json,
        summary: request.summary,
        status,
        requested_by: request.requested_by,
        reviewed_by: None,
        review_note: None,
        created_at: now,
        reviewed_at: None,
    })
}

#[tauri::command]
pub async fn team_v3_review_plan_revision(
    db: DbState<'_>,
    session_id: String,
    revision_id: String,
    request: TeamV3ReviewPlanRevisionRequest,
) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    let new_status = if request.approve {
        "approved"
    } else {
        "rejected"
    };
    let session_state = if request.approve {
        "EXECUTING"
    } else {
        "PLAN_DRAFT"
    };

    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_plan_revisions
                   SET status = ?, reviewed_by = ?, review_note = ?, reviewed_at = ?
                   WHERE id = ? AND session_id = ?"#,
            )
            .bind(new_status)
            .bind(&request.reviewed_by)
            .bind(&request.review_note)
            .bind(&now)
            .bind(&revision_id)
            .bind(&session_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET state = ?, updated_at = ?
                   WHERE id = ?"#,
            )
            .bind(session_state)
            .bind(&now)
            .bind(&session_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_plan_revisions
                   SET status = $1, reviewed_by = $2, review_note = $3, reviewed_at = $4
                   WHERE id = $5 AND session_id = $6"#,
            )
            .bind(new_status)
            .bind(&request.reviewed_by)
            .bind(&request.review_note)
            .bind(&now)
            .bind(&revision_id)
            .bind(&session_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET state = $1, updated_at = $2
                   WHERE id = $3"#,
            )
            .bind(session_state)
            .bind(&now)
            .bind(&session_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => return Err("Team V3 does not support MySQL".to_string()),
    }
    Ok(())
}

use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use sentinel_db::{database_service::connection_manager::DatabasePool, DatabaseService};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use tauri::{AppHandle, State};
use uuid::Uuid;

use super::team_v4_mapping::*;
use super::team_v4_schema::ensure_team_v4_schema;

type DbState<'r> = State<'r, Arc<DatabaseService>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV4Run {
    pub id: String,
    pub conversation_id: Option<String>,
    pub profile_id: Option<String>,
    pub goal: String,
    pub state: String,
    pub policy_json: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV4Agent {
    pub id: String,
    pub run_id: String,
    pub profile_id: Option<String>,
    pub role_type: String,
    pub name: String,
    pub status: String,
    pub model: Option<String>,
    pub context_mode: Option<String>,
    pub tool_policy_json: Value,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV4Task {
    pub id: String,
    pub run_id: String,
    pub parent_task_id: Option<String>,
    pub task_key: String,
    pub title: String,
    pub instruction: String,
    pub status: String,
    pub priority: i32,
    pub assigned_agent_id: Option<String>,
    pub depends_on: Value,
    pub acceptance_criteria: Option<String>,
    pub context_snapshot_id: Option<String>,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV4Event {
    pub id: String,
    pub run_id: String,
    pub sequence: i64,
    pub actor_id: Option<String>,
    pub task_id: Option<String>,
    pub event_type: String,
    pub visibility: String,
    pub payload: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV4ContextSnapshot {
    pub id: String,
    pub run_id: String,
    pub actor_id: Option<String>,
    pub task_id: Option<String>,
    pub role_type: String,
    pub source_sequence: i64,
    pub policy_json: Value,
    pub sections_json: Value,
    pub token_estimate: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV4Memory {
    pub id: String,
    pub run_id: String,
    pub task_id: Option<String>,
    pub kind: String,
    pub content: String,
    pub confidence: f64,
    pub source_event_ids: Value,
    pub accepted_by_orchestrator: bool,
    pub promoted_to_long_term: bool,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV4HarnessRun {
    pub id: String,
    pub run_id: String,
    pub actor_id: Option<String>,
    pub task_id: Option<String>,
    pub status: String,
    pub lease_expires_at: Option<String>,
    pub last_heartbeat_at: Option<String>,
    pub checkpoint_sequence: i64,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamV4CreateRunRequest {
    pub conversation_id: Option<String>,
    pub profile_id: Option<String>,
    pub goal: String,
    pub policy_json: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamV4RegisterAgentRequest {
    pub profile_id: Option<String>,
    pub role_type: String,
    pub name: String,
    pub model: Option<String>,
    pub context_mode: Option<String>,
    pub tool_policy_json: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamV4CreateTaskRequest {
    pub parent_task_id: Option<String>,
    pub task_key: String,
    pub title: String,
    pub instruction: String,
    pub priority: Option<i32>,
    pub assigned_agent_id: Option<String>,
    pub depends_on: Option<Value>,
    pub acceptance_criteria: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamV4AppendEventRequest {
    pub actor_id: Option<String>,
    pub task_id: Option<String>,
    pub event_type: String,
    pub visibility: Option<String>,
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamV4CreateContextSnapshotRequest {
    pub actor_id: Option<String>,
    pub task_id: Option<String>,
    pub role_type: String,
    pub source_sequence: Option<i64>,
    pub policy_json: Option<Value>,
    pub sections_json: Value,
    pub token_estimate: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamV4CreateMemoryRequest {
    pub task_id: Option<String>,
    pub kind: String,
    pub content: String,
    pub confidence: Option<f64>,
    pub source_event_ids: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamV4StartHarnessRequest {
    pub actor_id: Option<String>,
    pub task_id: Option<String>,
    pub lease_secs: Option<i64>,
    pub metadata: Option<Value>,
}

fn json_text(value: &Value) -> Result<String> {
    serde_json::to_string(value).map_err(Into::into)
}

fn normalize_required(input: &str, field: &str) -> Result<String> {
    let value = input.trim();
    if value.is_empty() {
        Err(anyhow!("{} cannot be empty", field))
    } else {
        Ok(value.to_string())
    }
}

fn normalize_visibility(value: Option<String>) -> Result<String> {
    let visibility = value.unwrap_or_else(|| "workspace".to_string());
    match visibility.as_str() {
        "user" | "workspace" | "internal" => Ok(visibility),
        _ => Err(anyhow!("unsupported Team V4 event visibility")),
    }
}

fn normalize_role_type(value: &str) -> Result<String> {
    let role_type = value.trim().to_lowercase();
    match role_type.as_str() {
        "orchestrator" | "specialist" | "monitor" | "harness" => Ok(role_type),
        _ => Err(anyhow!("unsupported Team V4 role type")),
    }
}

fn normalize_memory_kind(value: &str) -> Result<String> {
    let kind = value.trim().to_lowercase();
    match kind.as_str() {
        "evidence" | "decision" | "risk" | "blocker" | "checkpoint" | "artifact_summary" => {
            Ok(kind)
        }
        _ => Err(anyhow!("unsupported Team V4 memory kind")),
    }
}

async fn allocate_event_sequence(runtime_pool: &DatabasePool, run_id: &str) -> Result<i64> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT OR IGNORE INTO team_v4_sequences (run_id, last_sequence)
                   VALUES (?, 0)"#,
            )
            .bind(run_id)
            .execute(pool)
            .await?;
            sqlx::query(
                r#"UPDATE team_v4_sequences
                   SET last_sequence = last_sequence + 1
                   WHERE run_id = ?"#,
            )
            .bind(run_id)
            .execute(pool)
            .await?;
            let row =
                sqlx::query(r#"SELECT last_sequence FROM team_v4_sequences WHERE run_id = ?"#)
                    .bind(run_id)
                    .fetch_one(pool)
                    .await?;
            Ok(row.get("last_sequence"))
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_sequences (run_id, last_sequence)
                   VALUES ($1, 0)
                   ON CONFLICT (run_id) DO NOTHING"#,
            )
            .bind(run_id)
            .execute(pool)
            .await?;
            let row = sqlx::query(
                r#"UPDATE team_v4_sequences
                   SET last_sequence = last_sequence + 1
                   WHERE run_id = $1
                   RETURNING last_sequence"#,
            )
            .bind(run_id)
            .fetch_one(pool)
            .await?;
            Ok(row.get("last_sequence"))
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V4 does not support MySQL")),
    }
}

pub(crate) async fn append_team_v4_event_internal(
    runtime_pool: &DatabasePool,
    run_id: &str,
    request: TeamV4AppendEventRequest,
) -> Result<TeamV4Event> {
    let event_type = normalize_required(&request.event_type, "event_type")?;
    let visibility = normalize_visibility(request.visibility)?;
    let payload = request.payload.unwrap_or_else(|| json!({}));
    let payload_text = json_text(&payload)?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let sequence = allocate_event_sequence(runtime_pool, run_id).await?;

    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_events
                   (id, run_id, sequence, actor_id, task_id, event_type, visibility, payload, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(run_id)
            .bind(sequence)
            .bind(&request.actor_id)
            .bind(&request.task_id)
            .bind(&event_type)
            .bind(&visibility)
            .bind(&payload_text)
            .bind(&now)
            .execute(pool)
            .await?;
            sqlx::query("UPDATE team_v4_runs SET updated_at = ? WHERE id = ?")
                .bind(&now)
                .bind(run_id)
                .execute(pool)
                .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_events
                   (id, run_id, sequence, actor_id, task_id, event_type, visibility, payload, created_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9)"#,
            )
            .bind(&id)
            .bind(run_id)
            .bind(sequence)
            .bind(&request.actor_id)
            .bind(&request.task_id)
            .bind(&event_type)
            .bind(&visibility)
            .bind(&payload_text)
            .bind(&now)
            .execute(pool)
            .await?;
            sqlx::query("UPDATE team_v4_runs SET updated_at = $1 WHERE id = $2")
                .bind(&now)
                .bind(run_id)
                .execute(pool)
                .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V4 does not support MySQL")),
    }

    Ok(TeamV4Event {
        id,
        run_id: run_id.to_string(),
        sequence,
        actor_id: request.actor_id,
        task_id: request.task_id,
        event_type,
        visibility,
        payload,
        created_at: now,
    })
}

async fn get_team_v4_run_internal(
    runtime_pool: &DatabasePool,
    run_id: &str,
) -> Result<Option<TeamV4Run>> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT id, conversation_id, profile_id, goal, state, policy_json, created_at, updated_at
                   FROM team_v4_runs WHERE id = ?"#,
            )
            .bind(run_id)
            .fetch_optional(pool)
            .await?;
            row.map(map_run).transpose()
        }
        DatabasePool::PostgreSQL(pool) => {
            let row = sqlx::query(
                r#"SELECT id, conversation_id, profile_id, goal, state, policy_json::text as policy_json,
                          created_at::text as created_at, updated_at::text as updated_at
                   FROM team_v4_runs WHERE id = $1"#,
            )
            .bind(run_id)
            .fetch_optional(pool)
            .await?;
            row.map(map_run_pg).transpose()
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V4 does not support MySQL")),
    }
}

pub(crate) async fn create_team_v4_run_internal(
    runtime_pool: &DatabasePool,
    request: TeamV4CreateRunRequest,
) -> Result<TeamV4Run> {
    let goal = normalize_required(&request.goal, "goal")?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let state = "draft".to_string();
    let policy_json = request.policy_json.unwrap_or_else(|| json!({}));
    let policy_text = json_text(&policy_json)?;

    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_runs
                   (id, conversation_id, profile_id, goal, state, policy_json, created_at, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(&request.conversation_id)
            .bind(&request.profile_id)
            .bind(&goal)
            .bind(&state)
            .bind(&policy_text)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
            sqlx::query(
                r#"INSERT INTO team_v4_sequences (run_id, last_sequence)
                   VALUES (?, 0)"#,
            )
            .bind(&id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_runs
                   (id, conversation_id, profile_id, goal, state, policy_json, created_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $8)"#,
            )
            .bind(&id)
            .bind(&request.conversation_id)
            .bind(&request.profile_id)
            .bind(&goal)
            .bind(&state)
            .bind(&policy_text)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
            sqlx::query(
                r#"INSERT INTO team_v4_sequences (run_id, last_sequence)
                   VALUES ($1, 0)"#,
            )
            .bind(&id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V4 does not support MySQL")),
    }

    Ok(TeamV4Run {
        id,
        conversation_id: request.conversation_id,
        profile_id: request.profile_id,
        goal,
        state,
        policy_json,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub(crate) async fn register_team_v4_agent_internal(
    runtime_pool: &DatabasePool,
    run_id: &str,
    request: TeamV4RegisterAgentRequest,
) -> Result<TeamV4Agent> {
    let role_type = normalize_role_type(&request.role_type)?;
    let name = normalize_required(&request.name, "name")?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let status = "idle".to_string();
    let tool_policy_json = request.tool_policy_json.unwrap_or_else(|| json!({}));
    let metadata = request.metadata.unwrap_or_else(|| json!({}));
    let tool_policy_text = json_text(&tool_policy_json)?;
    let metadata_text = json_text(&metadata)?;

    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_agents
                   (id, run_id, profile_id, role_type, name, status, model, context_mode,
                    tool_policy_json, metadata, created_at, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(run_id)
            .bind(&request.profile_id)
            .bind(&role_type)
            .bind(&name)
            .bind(&status)
            .bind(&request.model)
            .bind(&request.context_mode)
            .bind(&tool_policy_text)
            .bind(&metadata_text)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_agents
                   (id, run_id, profile_id, role_type, name, status, model, context_mode,
                    tool_policy_json, metadata, created_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb, $10::jsonb, $11, $12)"#,
            )
            .bind(&id)
            .bind(run_id)
            .bind(&request.profile_id)
            .bind(&role_type)
            .bind(&name)
            .bind(&status)
            .bind(&request.model)
            .bind(&request.context_mode)
            .bind(&tool_policy_text)
            .bind(&metadata_text)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V4 does not support MySQL")),
    }

    Ok(TeamV4Agent {
        id,
        run_id: run_id.to_string(),
        profile_id: request.profile_id,
        role_type,
        name,
        status,
        model: request.model,
        context_mode: request.context_mode,
        tool_policy_json,
        metadata,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub(crate) async fn create_team_v4_task_internal(
    runtime_pool: &DatabasePool,
    run_id: &str,
    request: TeamV4CreateTaskRequest,
) -> Result<TeamV4Task> {
    let task_key = normalize_required(&request.task_key, "task_key")?;
    let title = normalize_required(&request.title, "title")?;
    let instruction = normalize_required(&request.instruction, "instruction")?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let status = "pending".to_string();
    let priority = request.priority.unwrap_or(100);
    let depends_on = request.depends_on.unwrap_or_else(|| json!([]));
    let metadata = request.metadata.unwrap_or_else(|| json!({}));
    let depends_on_text = json_text(&depends_on)?;
    let metadata_text = json_text(&metadata)?;

    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_tasks
                   (id, run_id, parent_task_id, task_key, title, instruction, status, priority,
                    assigned_agent_id, depends_on, acceptance_criteria, metadata, created_at, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(run_id)
            .bind(&request.parent_task_id)
            .bind(&task_key)
            .bind(&title)
            .bind(&instruction)
            .bind(&status)
            .bind(priority)
            .bind(&request.assigned_agent_id)
            .bind(&depends_on_text)
            .bind(&request.acceptance_criteria)
            .bind(&metadata_text)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_tasks
                   (id, run_id, parent_task_id, task_key, title, instruction, status, priority,
                    assigned_agent_id, depends_on, acceptance_criteria, metadata, created_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb, $11, $12::jsonb, $13, $14)"#,
            )
            .bind(&id)
            .bind(run_id)
            .bind(&request.parent_task_id)
            .bind(&task_key)
            .bind(&title)
            .bind(&instruction)
            .bind(&status)
            .bind(priority)
            .bind(&request.assigned_agent_id)
            .bind(&depends_on_text)
            .bind(&request.acceptance_criteria)
            .bind(&metadata_text)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V4 does not support MySQL")),
    }

    Ok(TeamV4Task {
        id,
        run_id: run_id.to_string(),
        parent_task_id: request.parent_task_id,
        task_key,
        title,
        instruction,
        status,
        priority,
        assigned_agent_id: request.assigned_agent_id,
        depends_on,
        acceptance_criteria: request.acceptance_criteria,
        context_snapshot_id: None,
        metadata,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub(crate) async fn create_team_v4_context_snapshot_internal(
    runtime_pool: &DatabasePool,
    run_id: &str,
    request: TeamV4CreateContextSnapshotRequest,
) -> Result<TeamV4ContextSnapshot> {
    let role_type = normalize_role_type(&request.role_type)?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let policy_json = request.policy_json.unwrap_or_else(|| json!({}));
    let source_sequence = request.source_sequence.unwrap_or(0).max(0);
    let token_estimate = request.token_estimate.unwrap_or(0).max(0);
    let policy_text = json_text(&policy_json)?;
    let sections_text = json_text(&request.sections_json)?;

    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_context_snapshots
                   (id, run_id, actor_id, task_id, role_type, source_sequence, policy_json,
                    sections_json, token_estimate, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(run_id)
            .bind(&request.actor_id)
            .bind(&request.task_id)
            .bind(&role_type)
            .bind(source_sequence)
            .bind(&policy_text)
            .bind(&sections_text)
            .bind(token_estimate)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_context_snapshots
                   (id, run_id, actor_id, task_id, role_type, source_sequence, policy_json,
                    sections_json, token_estimate, created_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8::jsonb, $9, $10)"#,
            )
            .bind(&id)
            .bind(run_id)
            .bind(&request.actor_id)
            .bind(&request.task_id)
            .bind(&role_type)
            .bind(source_sequence)
            .bind(&policy_text)
            .bind(&sections_text)
            .bind(token_estimate)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V4 does not support MySQL")),
    }

    Ok(TeamV4ContextSnapshot {
        id,
        run_id: run_id.to_string(),
        actor_id: request.actor_id,
        task_id: request.task_id,
        role_type,
        source_sequence,
        policy_json,
        sections_json: request.sections_json,
        token_estimate,
        created_at: now,
    })
}

pub(crate) async fn start_team_v4_harness_internal(
    runtime_pool: &DatabasePool,
    run_id: &str,
    request: TeamV4StartHarnessRequest,
) -> Result<TeamV4HarnessRun> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let now_text = now.to_rfc3339();
    let lease_secs = request.lease_secs.unwrap_or(600).max(30);
    let lease_expires_at = (now + Duration::seconds(lease_secs)).to_rfc3339();
    let metadata = request.metadata.unwrap_or_else(|| json!({}));
    let metadata_text = json_text(&metadata)?;
    let status = "running".to_string();

    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_harness_runs
                   (id, run_id, actor_id, task_id, status, lease_expires_at, last_heartbeat_at,
                    checkpoint_sequence, metadata, created_at, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(run_id)
            .bind(&request.actor_id)
            .bind(&request.task_id)
            .bind(&status)
            .bind(&lease_expires_at)
            .bind(&now_text)
            .bind(0_i64)
            .bind(&metadata_text)
            .bind(&now_text)
            .bind(&now_text)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_harness_runs
                   (id, run_id, actor_id, task_id, status, lease_expires_at, last_heartbeat_at,
                    checkpoint_sequence, metadata, created_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb, $10, $11)"#,
            )
            .bind(&id)
            .bind(run_id)
            .bind(&request.actor_id)
            .bind(&request.task_id)
            .bind(&status)
            .bind(&lease_expires_at)
            .bind(&now_text)
            .bind(0_i64)
            .bind(&metadata_text)
            .bind(&now_text)
            .bind(&now_text)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V4 does not support MySQL")),
    }

    Ok(TeamV4HarnessRun {
        id,
        run_id: run_id.to_string(),
        actor_id: request.actor_id,
        task_id: request.task_id,
        status,
        lease_expires_at: Some(lease_expires_at),
        last_heartbeat_at: Some(now_text.clone()),
        checkpoint_sequence: 0,
        metadata,
        created_at: now_text.clone(),
        updated_at: now_text,
    })
}

#[tauri::command]
pub async fn team_v4_create_run(
    db: DbState<'_>,
    request: TeamV4CreateRunRequest,
) -> Result<TeamV4Run, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let run = create_team_v4_run_internal(&runtime_pool, request)
        .await
        .map_err(|e| e.to_string())?;
    append_team_v4_event_internal(
        &runtime_pool,
        &run.id,
        TeamV4AppendEventRequest {
            actor_id: None,
            task_id: None,
            event_type: "run_created".to_string(),
            visibility: Some("workspace".to_string()),
            payload: Some(json!({
                "goal": run.goal,
                "profile_id": run.profile_id,
            })),
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(run)
}

#[tauri::command]
pub async fn team_v4_get_run(db: DbState<'_>, run_id: String) -> Result<Option<TeamV4Run>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    get_team_v4_run_internal(&runtime_pool, &run_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn team_v4_update_run_state(
    db: DbState<'_>,
    run_id: String,
    state: String,
) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let normalized = state.trim().to_lowercase();
    if !matches!(
        normalized.as_str(),
        "draft"
            | "planning"
            | "running"
            | "waiting_human"
            | "completed"
            | "failed"
            | "cancelled"
            | "archived"
    ) {
        return Err("unsupported Team V4 run state".to_string());
    }
    update_run_state_internal(&runtime_pool, &run_id, &normalized)
        .await
        .map_err(|e| e.to_string())?;
    append_team_v4_event_internal(
        &runtime_pool,
        &run_id,
        TeamV4AppendEventRequest {
            actor_id: None,
            task_id: None,
            event_type: "run_state_changed".to_string(),
            visibility: Some("workspace".to_string()),
            payload: Some(json!({ "state": normalized })),
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn team_v4_register_agent(
    db: DbState<'_>,
    run_id: String,
    request: TeamV4RegisterAgentRequest,
) -> Result<TeamV4Agent, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let agent = register_team_v4_agent_internal(&runtime_pool, &run_id, request)
        .await
        .map_err(|e| e.to_string())?;
    append_team_v4_event_internal(
        &runtime_pool,
        &run_id,
        TeamV4AppendEventRequest {
            actor_id: Some(agent.id.clone()),
            task_id: None,
            event_type: "agent_registered".to_string(),
            visibility: Some("workspace".to_string()),
            payload: Some(json!({
                "role_type": agent.role_type,
                "name": agent.name,
                "profile_id": agent.profile_id,
            })),
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(agent)
}

#[tauri::command]
pub async fn team_v4_create_task(
    db: DbState<'_>,
    run_id: String,
    request: TeamV4CreateTaskRequest,
) -> Result<TeamV4Task, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let task = create_team_v4_task_internal(&runtime_pool, &run_id, request)
        .await
        .map_err(|e| e.to_string())?;
    append_team_v4_event_internal(
        &runtime_pool,
        &run_id,
        TeamV4AppendEventRequest {
            actor_id: task.assigned_agent_id.clone(),
            task_id: Some(task.id.clone()),
            event_type: "task_created".to_string(),
            visibility: Some("workspace".to_string()),
            payload: Some(json!({
                "task_key": task.task_key,
                "title": task.title,
                "priority": task.priority,
            })),
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(task)
}

#[tauri::command]
pub async fn team_v4_append_event(
    db: DbState<'_>,
    run_id: String,
    request: TeamV4AppendEventRequest,
) -> Result<TeamV4Event, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    append_team_v4_event_internal(&runtime_pool, &run_id, request)
        .await
        .map_err(|e| e.to_string())
}

pub(crate) async fn update_run_state_internal(
    runtime_pool: &DatabasePool,
    run_id: &str,
    state: &str,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query("UPDATE team_v4_runs SET state = ?, updated_at = ? WHERE id = ?")
                .bind(state)
                .bind(&now)
                .bind(run_id)
                .execute(pool)
                .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query("UPDATE team_v4_runs SET state = $1, updated_at = $2 WHERE id = $3")
                .bind(state)
                .bind(&now)
                .bind(run_id)
                .execute(pool)
                .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V4 does not support MySQL")),
    }
    Ok(())
}

pub(crate) async fn attach_task_context_snapshot_internal(
    runtime_pool: &DatabasePool,
    task_id: &str,
    snapshot_id: &str,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                "UPDATE team_v4_tasks SET context_snapshot_id = ?, updated_at = ? WHERE id = ?",
            )
            .bind(snapshot_id)
            .bind(&now)
            .bind(task_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                "UPDATE team_v4_tasks SET context_snapshot_id = $1, updated_at = $2 WHERE id = $3",
            )
            .bind(snapshot_id)
            .bind(&now)
            .bind(task_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V4 does not support MySQL")),
    }
    Ok(())
}

async fn create_team_v4_memory_internal(
    runtime_pool: &DatabasePool,
    run_id: &str,
    request: TeamV4CreateMemoryRequest,
) -> Result<TeamV4Memory> {
    let kind = normalize_memory_kind(&request.kind)?;
    let content = normalize_required(&request.content, "content")?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let confidence = request.confidence.unwrap_or(0.5).clamp(0.0, 1.0);
    let source_event_ids = request.source_event_ids.unwrap_or_else(|| json!([]));
    let metadata = request.metadata.unwrap_or_else(|| json!({}));
    let source_event_ids_text = json_text(&source_event_ids)?;
    let metadata_text = json_text(&metadata)?;

    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_memories
                   (id, run_id, task_id, kind, content, confidence, source_event_ids,
                    accepted_by_orchestrator, promoted_to_long_term, metadata, created_at, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(run_id)
            .bind(&request.task_id)
            .bind(&kind)
            .bind(&content)
            .bind(confidence)
            .bind(&source_event_ids_text)
            .bind(false)
            .bind(false)
            .bind(&metadata_text)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v4_memories
                   (id, run_id, task_id, kind, content, confidence, source_event_ids,
                    accepted_by_orchestrator, promoted_to_long_term, metadata, created_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8, $9, $10::jsonb, $11, $12)"#,
            )
            .bind(&id)
            .bind(run_id)
            .bind(&request.task_id)
            .bind(&kind)
            .bind(&content)
            .bind(confidence)
            .bind(&source_event_ids_text)
            .bind(false)
            .bind(false)
            .bind(&metadata_text)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V4 does not support MySQL")),
    }

    Ok(TeamV4Memory {
        id,
        run_id: run_id.to_string(),
        task_id: request.task_id,
        kind,
        content,
        confidence,
        source_event_ids,
        accepted_by_orchestrator: false,
        promoted_to_long_term: false,
        metadata,
        created_at: now.clone(),
        updated_at: now,
    })
}

async fn load_harness_run_internal(
    runtime_pool: &DatabasePool,
    harness_run_id: &str,
) -> Result<TeamV4HarnessRun> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT id, run_id, actor_id, task_id, status, lease_expires_at,
                          last_heartbeat_at, checkpoint_sequence, metadata, created_at, updated_at
                   FROM team_v4_harness_runs WHERE id = ?"#,
            )
            .bind(harness_run_id)
            .fetch_one(pool)
            .await?;
            map_harness_run(row)
        }
        DatabasePool::PostgreSQL(pool) => {
            let row = sqlx::query(
                r#"SELECT id, run_id, actor_id, task_id, status,
                          lease_expires_at::text as lease_expires_at,
                          last_heartbeat_at::text as last_heartbeat_at,
                          checkpoint_sequence, metadata::text as metadata,
                          created_at::text as created_at, updated_at::text as updated_at
                   FROM team_v4_harness_runs WHERE id = $1"#,
            )
            .bind(harness_run_id)
            .fetch_one(pool)
            .await?;
            map_harness_run_pg(row)
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V4 does not support MySQL")),
    }
}

async fn promote_team_v4_memory_to_long_term(
    app_handle: &AppHandle,
    memory: &TeamV4Memory,
) -> Result<String> {
    let outcome = crate::memory::store_memory(
        app_handle,
        memory.content.clone(),
        Some(format!("Team v4 {} memory", memory.kind)),
        vec![
            "team_v4".to_string(),
            memory.kind.clone(),
            format!("run:{}", memory.run_id),
        ],
    )
    .await?;
    Ok(outcome.memory_id)
}

#[tauri::command]
pub async fn team_v4_list_runs(
    db: DbState<'_>,
    conversation_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<TeamV4Run>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(20).clamp(1, 100);
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = if let Some(conversation_id) = conversation_id {
                sqlx::query(
                    r#"SELECT id, conversation_id, profile_id, goal, state, policy_json, created_at, updated_at
                       FROM team_v4_runs WHERE conversation_id = ?
                       ORDER BY updated_at DESC LIMIT ?"#,
                )
                .bind(conversation_id)
                .bind(limit)
                .fetch_all(pool)
                .await
                .map_err(|e| e.to_string())?
            } else {
                sqlx::query(
                    r#"SELECT id, conversation_id, profile_id, goal, state, policy_json, created_at, updated_at
                       FROM team_v4_runs ORDER BY updated_at DESC LIMIT ?"#,
                )
                .bind(limit)
                .fetch_all(pool)
                .await
                .map_err(|e| e.to_string())?
            };
            rows.into_iter()
                .map(map_run)
                .collect::<Result<Vec<_>>>()
                .map_err(|e| e.to_string())
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = if let Some(conversation_id) = conversation_id {
                sqlx::query(
                    r#"SELECT id, conversation_id, profile_id, goal, state, policy_json::text as policy_json,
                              created_at::text as created_at, updated_at::text as updated_at
                       FROM team_v4_runs WHERE conversation_id = $1
                       ORDER BY updated_at DESC LIMIT $2"#,
                )
                .bind(conversation_id)
                .bind(limit)
                .fetch_all(pool)
                .await
                .map_err(|e| e.to_string())?
            } else {
                sqlx::query(
                    r#"SELECT id, conversation_id, profile_id, goal, state, policy_json::text as policy_json,
                              created_at::text as created_at, updated_at::text as updated_at
                       FROM team_v4_runs ORDER BY updated_at DESC LIMIT $1"#,
                )
                .bind(limit)
                .fetch_all(pool)
                .await
                .map_err(|e| e.to_string())?
            };
            rows.into_iter()
                .map(map_run_pg)
                .collect::<Result<Vec<_>>>()
                .map_err(|e| e.to_string())
        }
        DatabasePool::MySQL(_) => Err("Team V4 does not support MySQL".to_string()),
    }
}

#[tauri::command]
pub async fn team_v4_list_agents(
    db: DbState<'_>,
    run_id: String,
) -> Result<Vec<TeamV4Agent>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, profile_id, role_type, name, status, model, context_mode,
                          tool_policy_json, metadata, created_at, updated_at
                   FROM team_v4_agents WHERE run_id = ?
                   ORDER BY created_at ASC"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            rows.into_iter()
                .map(map_agent)
                .collect::<Result<Vec<_>>>()
                .map_err(|e| e.to_string())
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, profile_id, role_type, name, status, model, context_mode,
                          tool_policy_json::text as tool_policy_json, metadata::text as metadata,
                          created_at::text as created_at, updated_at::text as updated_at
                   FROM team_v4_agents WHERE run_id = $1
                   ORDER BY created_at ASC"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            rows.into_iter()
                .map(map_agent_pg)
                .collect::<Result<Vec<_>>>()
                .map_err(|e| e.to_string())
        }
        DatabasePool::MySQL(_) => Err("Team V4 does not support MySQL".to_string()),
    }
}

#[tauri::command]
pub async fn team_v4_list_tasks(
    db: DbState<'_>,
    run_id: String,
) -> Result<Vec<TeamV4Task>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, parent_task_id, task_key, title, instruction, status, priority,
                          assigned_agent_id, depends_on, acceptance_criteria, context_snapshot_id,
                          metadata, created_at, updated_at
                   FROM team_v4_tasks WHERE run_id = ?
                   ORDER BY priority ASC, created_at ASC"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            rows.into_iter()
                .map(map_task)
                .collect::<Result<Vec<_>>>()
                .map_err(|e| e.to_string())
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, parent_task_id, task_key, title, instruction, status, priority,
                          assigned_agent_id, depends_on::text as depends_on, acceptance_criteria,
                          context_snapshot_id, metadata::text as metadata,
                          created_at::text as created_at, updated_at::text as updated_at
                   FROM team_v4_tasks WHERE run_id = $1
                   ORDER BY priority ASC, created_at ASC"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            rows.into_iter()
                .map(map_task_pg)
                .collect::<Result<Vec<_>>>()
                .map_err(|e| e.to_string())
        }
        DatabasePool::MySQL(_) => Err("Team V4 does not support MySQL".to_string()),
    }
}

#[tauri::command]
pub async fn team_v4_list_events(
    db: DbState<'_>,
    run_id: String,
    after_sequence: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<TeamV4Event>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let after_sequence = after_sequence.unwrap_or(0).max(0);
    let limit = limit.unwrap_or(100).clamp(1, 500);
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, sequence, actor_id, task_id, event_type, visibility, payload, created_at
                   FROM team_v4_events
                   WHERE run_id = ? AND sequence > ?
                   ORDER BY sequence ASC LIMIT ?"#,
            )
            .bind(run_id)
            .bind(after_sequence)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            rows.into_iter()
                .map(map_event)
                .collect::<Result<Vec<_>>>()
                .map_err(|e| e.to_string())
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, sequence, actor_id, task_id, event_type, visibility,
                          payload::text as payload, created_at::text as created_at
                   FROM team_v4_events
                   WHERE run_id = $1 AND sequence > $2
                   ORDER BY sequence ASC LIMIT $3"#,
            )
            .bind(run_id)
            .bind(after_sequence)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            rows.into_iter()
                .map(map_event_pg)
                .collect::<Result<Vec<_>>>()
                .map_err(|e| e.to_string())
        }
        DatabasePool::MySQL(_) => Err("Team V4 does not support MySQL".to_string()),
    }
}

#[tauri::command]
pub async fn team_v4_list_memories(
    db: DbState<'_>,
    run_id: String,
) -> Result<Vec<TeamV4Memory>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, task_id, kind, content, confidence, source_event_ids,
                          accepted_by_orchestrator, promoted_to_long_term, metadata, created_at, updated_at
                   FROM team_v4_memories WHERE run_id = ?
                   ORDER BY created_at ASC"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            rows.into_iter()
                .map(map_memory)
                .collect::<Result<Vec<_>>>()
                .map_err(|e| e.to_string())
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, task_id, kind, content, confidence,
                          source_event_ids::text as source_event_ids,
                          accepted_by_orchestrator, promoted_to_long_term,
                          metadata::text as metadata,
                          created_at::text as created_at, updated_at::text as updated_at
                   FROM team_v4_memories WHERE run_id = $1
                   ORDER BY created_at ASC"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            rows.into_iter()
                .map(map_memory_pg)
                .collect::<Result<Vec<_>>>()
                .map_err(|e| e.to_string())
        }
        DatabasePool::MySQL(_) => Err("Team V4 does not support MySQL".to_string()),
    }
}

#[tauri::command]
pub async fn team_v4_list_harness_runs(
    db: DbState<'_>,
    run_id: String,
) -> Result<Vec<TeamV4HarnessRun>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, actor_id, task_id, status, lease_expires_at,
                          last_heartbeat_at, checkpoint_sequence, metadata, created_at, updated_at
                   FROM team_v4_harness_runs WHERE run_id = ?
                   ORDER BY created_at ASC"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            rows.into_iter()
                .map(map_harness_run)
                .collect::<Result<Vec<_>>>()
                .map_err(|e| e.to_string())
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, run_id, actor_id, task_id, status,
                          lease_expires_at::text as lease_expires_at,
                          last_heartbeat_at::text as last_heartbeat_at,
                          checkpoint_sequence, metadata::text as metadata,
                          created_at::text as created_at, updated_at::text as updated_at
                   FROM team_v4_harness_runs WHERE run_id = $1
                   ORDER BY created_at ASC"#,
            )
            .bind(run_id)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;
            rows.into_iter()
                .map(map_harness_run_pg)
                .collect::<Result<Vec<_>>>()
                .map_err(|e| e.to_string())
        }
        DatabasePool::MySQL(_) => Err("Team V4 does not support MySQL".to_string()),
    }
}

#[tauri::command]
pub async fn team_v4_create_context_snapshot(
    db: DbState<'_>,
    run_id: String,
    request: TeamV4CreateContextSnapshotRequest,
) -> Result<TeamV4ContextSnapshot, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let snapshot = create_team_v4_context_snapshot_internal(&runtime_pool, &run_id, request)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(task_id) = snapshot.task_id.as_deref() {
        attach_task_context_snapshot_internal(&runtime_pool, task_id, &snapshot.id)
            .await
            .map_err(|e| e.to_string())?;
    }
    append_team_v4_event_internal(
        &runtime_pool,
        &run_id,
        TeamV4AppendEventRequest {
            actor_id: snapshot.actor_id.clone(),
            task_id: snapshot.task_id.clone(),
            event_type: "context_snapshot_created".to_string(),
            visibility: Some("internal".to_string()),
            payload: Some(json!({
                "snapshot_id": snapshot.id,
                "role_type": snapshot.role_type,
                "source_sequence": snapshot.source_sequence,
                "token_estimate": snapshot.token_estimate,
            })),
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(snapshot)
}

#[tauri::command]
pub async fn team_v4_create_memory_candidate(
    db: DbState<'_>,
    run_id: String,
    request: TeamV4CreateMemoryRequest,
) -> Result<TeamV4Memory, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let memory = create_team_v4_memory_internal(&runtime_pool, &run_id, request)
        .await
        .map_err(|e| e.to_string())?;
    append_team_v4_event_internal(
        &runtime_pool,
        &run_id,
        TeamV4AppendEventRequest {
            actor_id: None,
            task_id: memory.task_id.clone(),
            event_type: "monitor_memory_candidate".to_string(),
            visibility: Some("workspace".to_string()),
            payload: Some(json!({
                "memory_id": memory.id,
                "kind": memory.kind,
                "confidence": memory.confidence,
                "content": memory.content,
            })),
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(memory)
}

#[tauri::command]
pub async fn team_v4_accept_memory(
    app_handle: AppHandle,
    db: DbState<'_>,
    run_id: String,
    memory_id: String,
    promoted_to_long_term: Option<bool>,
) -> Result<TeamV4Memory, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    let promote = promoted_to_long_term.unwrap_or(false);
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v4_memories
                   SET accepted_by_orchestrator = ?, promoted_to_long_term = ?, updated_at = ?
                   WHERE id = ? AND run_id = ?"#,
            )
            .bind(true)
            .bind(promote)
            .bind(&now)
            .bind(&memory_id)
            .bind(&run_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
            let row = sqlx::query(
                r#"SELECT id, run_id, task_id, kind, content, confidence, source_event_ids,
                          accepted_by_orchestrator, promoted_to_long_term, metadata, created_at, updated_at
                   FROM team_v4_memories WHERE id = ?"#,
            )
            .bind(&memory_id)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;
            let memory = map_memory(row).map_err(|e| e.to_string())?;
            let long_term_memory_id = if promote {
                Some(
                    promote_team_v4_memory_to_long_term(&app_handle, &memory)
                        .await
                        .map_err(|e| e.to_string())?,
                )
            } else {
                None
            };
            append_team_v4_event_internal(
                &runtime_pool,
                &run_id,
                TeamV4AppendEventRequest {
                    actor_id: None,
                    task_id: memory.task_id.clone(),
                    event_type: "orchestrator_memory_accepted".to_string(),
                    visibility: Some("workspace".to_string()),
                    payload: Some(json!({
                        "memory_id": memory.id,
                        "kind": memory.kind,
                        "promoted_to_long_term": memory.promoted_to_long_term,
                        "long_term_memory_id": long_term_memory_id,
                    })),
                },
            )
            .await
            .map_err(|e| e.to_string())?;
            Ok(memory)
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v4_memories
                   SET accepted_by_orchestrator = $1, promoted_to_long_term = $2, updated_at = $3
                   WHERE id = $4 AND run_id = $5"#,
            )
            .bind(true)
            .bind(promote)
            .bind(&now)
            .bind(&memory_id)
            .bind(&run_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
            let row = sqlx::query(
                r#"SELECT id, run_id, task_id, kind, content, confidence,
                          source_event_ids::text as source_event_ids,
                          accepted_by_orchestrator, promoted_to_long_term,
                          metadata::text as metadata,
                          created_at::text as created_at, updated_at::text as updated_at
                   FROM team_v4_memories WHERE id = $1"#,
            )
            .bind(&memory_id)
            .fetch_one(pool)
            .await
            .map_err(|e| e.to_string())?;
            let memory = map_memory_pg(row).map_err(|e| e.to_string())?;
            let long_term_memory_id = if promote {
                Some(
                    promote_team_v4_memory_to_long_term(&app_handle, &memory)
                        .await
                        .map_err(|e| e.to_string())?,
                )
            } else {
                None
            };
            append_team_v4_event_internal(
                &runtime_pool,
                &run_id,
                TeamV4AppendEventRequest {
                    actor_id: None,
                    task_id: memory.task_id.clone(),
                    event_type: "orchestrator_memory_accepted".to_string(),
                    visibility: Some("workspace".to_string()),
                    payload: Some(json!({
                        "memory_id": memory.id,
                        "kind": memory.kind,
                        "promoted_to_long_term": memory.promoted_to_long_term,
                        "long_term_memory_id": long_term_memory_id,
                    })),
                },
            )
            .await
            .map_err(|e| e.to_string())?;
            Ok(memory)
        }
        DatabasePool::MySQL(_) => Err("Team V4 does not support MySQL".to_string()),
    }
}

#[tauri::command]
pub async fn team_v4_start_harness_run(
    db: DbState<'_>,
    run_id: String,
    request: TeamV4StartHarnessRequest,
) -> Result<TeamV4HarnessRun, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let harness = start_team_v4_harness_internal(&runtime_pool, &run_id, request)
        .await
        .map_err(|e| e.to_string())?;
    append_team_v4_event_internal(
        &runtime_pool,
        &run_id,
        TeamV4AppendEventRequest {
            actor_id: harness.actor_id.clone(),
            task_id: harness.task_id.clone(),
            event_type: "harness_started".to_string(),
            visibility: Some("workspace".to_string()),
            payload: Some(json!({
                "harness_run_id": harness.id,
                "lease_expires_at": harness.lease_expires_at,
            })),
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(harness)
}

#[tauri::command]
pub async fn team_v4_heartbeat_harness_run(
    db: DbState<'_>,
    harness_run_id: String,
    lease_secs: Option<i64>,
) -> Result<TeamV4HarnessRun, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let now = Utc::now();
    let now_text = now.to_rfc3339();
    let lease_expires_at =
        (now + Duration::seconds(lease_secs.unwrap_or(600).max(30))).to_rfc3339();
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v4_harness_runs
                   SET last_heartbeat_at = ?, lease_expires_at = ?, updated_at = ?
                   WHERE id = ?"#,
            )
            .bind(&now_text)
            .bind(&lease_expires_at)
            .bind(&now_text)
            .bind(&harness_run_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v4_harness_runs
                   SET last_heartbeat_at = $1, lease_expires_at = $2, updated_at = $3
                   WHERE id = $4"#,
            )
            .bind(&now_text)
            .bind(&lease_expires_at)
            .bind(&now_text)
            .bind(&harness_run_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => return Err("Team V4 does not support MySQL".to_string()),
    }
    load_harness_run_internal(&runtime_pool, &harness_run_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn team_v4_checkpoint_harness_run(
    db: DbState<'_>,
    harness_run_id: String,
    checkpoint_sequence: i64,
) -> Result<TeamV4HarnessRun, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v4_harness_runs
                   SET checkpoint_sequence = ?, updated_at = ?
                   WHERE id = ?"#,
            )
            .bind(checkpoint_sequence.max(0))
            .bind(&now)
            .bind(&harness_run_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v4_harness_runs
                   SET checkpoint_sequence = $1, updated_at = $2
                   WHERE id = $3"#,
            )
            .bind(checkpoint_sequence.max(0))
            .bind(&now)
            .bind(&harness_run_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => return Err("Team V4 does not support MySQL".to_string()),
    }
    load_harness_run_internal(&runtime_pool, &harness_run_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn team_v4_cancel_harness_run(
    db: DbState<'_>,
    harness_run_id: String,
) -> Result<TeamV4HarnessRun, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v4_harness_runs
                   SET status = ?, updated_at = ?
                   WHERE id = ?"#,
            )
            .bind("cancelled")
            .bind(&now)
            .bind(&harness_run_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v4_harness_runs
                   SET status = $1, updated_at = $2
                   WHERE id = $3"#,
            )
            .bind("cancelled")
            .bind(&now)
            .bind(&harness_run_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => return Err("Team V4 does not support MySQL".to_string()),
    }
    let harness = load_harness_run_internal(&runtime_pool, &harness_run_id)
        .await
        .map_err(|e| e.to_string())?;
    append_team_v4_event_internal(
        &runtime_pool,
        &harness.run_id,
        TeamV4AppendEventRequest {
            actor_id: harness.actor_id.clone(),
            task_id: harness.task_id.clone(),
            event_type: "harness_cancelled".to_string(),
            visibility: Some("workspace".to_string()),
            payload: Some(json!({
                "harness_run_id": harness.id,
                "checkpoint_sequence": harness.checkpoint_sequence,
            })),
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(harness)
}

#[tauri::command]
pub async fn team_v4_resume_harness_run(
    db: DbState<'_>,
    harness_run_id: String,
    lease_secs: Option<i64>,
) -> Result<TeamV4HarnessRun, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let now = Utc::now();
    let now_text = now.to_rfc3339();
    let lease_expires_at =
        (now + Duration::seconds(lease_secs.unwrap_or(600).max(30))).to_rfc3339();
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v4_harness_runs
                   SET status = ?, lease_expires_at = ?, last_heartbeat_at = ?, updated_at = ?
                   WHERE id = ?"#,
            )
            .bind("running")
            .bind(&lease_expires_at)
            .bind(&now_text)
            .bind(&now_text)
            .bind(&harness_run_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v4_harness_runs
                   SET status = $1, lease_expires_at = $2, last_heartbeat_at = $3, updated_at = $4
                   WHERE id = $5"#,
            )
            .bind("running")
            .bind(&lease_expires_at)
            .bind(&now_text)
            .bind(&now_text)
            .bind(&harness_run_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => return Err("Team V4 does not support MySQL".to_string()),
    }
    let harness = load_harness_run_internal(&runtime_pool, &harness_run_id)
        .await
        .map_err(|e| e.to_string())?;
    append_team_v4_event_internal(
        &runtime_pool,
        &harness.run_id,
        TeamV4AppendEventRequest {
            actor_id: harness.actor_id.clone(),
            task_id: harness.task_id.clone(),
            event_type: "harness_resumed".to_string(),
            visibility: Some("workspace".to_string()),
            payload: Some(json!({
                "harness_run_id": harness.id,
                "lease_expires_at": harness.lease_expires_at,
                "checkpoint_sequence": harness.checkpoint_sequence,
            })),
        },
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(harness)
}

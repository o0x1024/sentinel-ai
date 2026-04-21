use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use crate::agents::tool_router::{ToolConfig, ToolSelectionStrategy};
use crate::commands::team_v3_agent_runtime::{
    execute_team_wave_tasks, TeamWaveTaskExecutionRequest, TeamWaveTaskExecutionResult,
};
use crate::commands::team_v3_artifact_store::{
    persist_team_v3_task_output_artifact, TeamV3ArtifactFileRef,
};
use crate::commands::team_v3_blackboard_context::{
    build_blackboard_checkpoint_context, build_blackboard_context, latest_blackboard_revision,
    truncate_chars, TeamV3PromptQuery,
};
use crate::commands::team_v3_memory::{
    append_team_v3_task_memory_layers, build_task_artifact_ref_content, build_task_artifact_summary,
};
use crate::commands::team_v3_planner::{
    prepare_team_v3_execution_tasks_with_main_agent, select_team_member_for_task,
    team_member_profiles,
};
use crate::commands::team_v3_prompting::{
    build_team_member_execution_system_prompt, build_team_member_runtime_context,
    build_team_v3_task_prompt, is_team_v3_summary_task,
};
use crate::commands::team_v3_session_state::{
    append_team_v3_status_message, parse_state_data_text, parse_task_dependencies,
};
use crate::commands::team_v3_task_notices::append_team_v3_dependency_ready_notices;
use crate::commands::team_v3_task_state::set_team_v3_task_execution_state;
use crate::services::ai::AiServiceManager;
use anyhow::{anyhow, Result};
use chrono::Utc;
use sentinel_db::{database_service::connection_manager::DatabasePool, Database, DatabaseService};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use tauri::{AppHandle, Manager};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

static TEAM_EXECUTION_CANCELLATIONS: LazyLock<Mutex<HashMap<String, (u64, CancellationToken)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static TEAM_EXECUTION_GENERATION: AtomicU64 = AtomicU64::new(0);
static TEAM_V3_BLACKBOARD_SESSION_LOCKS: LazyLock<
    Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
> = LazyLock::new(|| Mutex::new(HashMap::new()));

const TEAM_V3_BLACKBOARD_INLINE_CHAR_LIMIT: usize = 2_000;
pub(crate) const TEAM_V3_STRUCTURED_BACKFILL_SCAN_LIMIT: i64 = 2_000;

pub(crate) fn create_team_execution_cancellation(session_id: &str) -> (u64, CancellationToken) {
    let generation = TEAM_EXECUTION_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    let token = CancellationToken::new();
    if let Ok(mut guard) = TEAM_EXECUTION_CANCELLATIONS.lock() {
        if let Some((_, old_token)) =
            guard.insert(session_id.to_string(), (generation, token.clone()))
        {
            old_token.cancel();
        }
    }
    (generation, token)
}

pub(crate) fn cancel_team_execution(session_id: &str) {
    if let Ok(guard) = TEAM_EXECUTION_CANCELLATIONS.lock() {
        if let Some((_, token)) = guard.get(session_id) {
            token.cancel();
        }
    }
}

pub(crate) fn is_team_execution_cancelled(session_id: &str, generation: u64) -> bool {
    if let Ok(guard) = TEAM_EXECUTION_CANCELLATIONS.lock() {
        if let Some((current_generation, token)) = guard.get(session_id) {
            return *current_generation != generation || token.is_cancelled();
        }
    }
    true
}

pub(crate) fn clear_team_execution_cancellation(session_id: &str, generation: u64) {
    if let Ok(mut guard) = TEAM_EXECUTION_CANCELLATIONS.lock() {
        if let Some((current_generation, _)) = guard.get(session_id) {
            if *current_generation == generation {
                guard.remove(session_id);
                clear_team_v3_blackboard_session_lock(session_id);
            }
        }
    }
}

fn team_v3_blackboard_session_lock(session_id: &str) -> Arc<tokio::sync::Mutex<()>> {
    if let Ok(mut guard) = TEAM_V3_BLACKBOARD_SESSION_LOCKS.lock() {
        return guard
            .entry(session_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone();
    }
    Arc::new(tokio::sync::Mutex::new(()))
}

fn clear_team_v3_blackboard_session_lock(session_id: &str) {
    if let Ok(mut guard) = TEAM_V3_BLACKBOARD_SESSION_LOCKS.lock() {
        guard.remove(session_id);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3Session {
    pub id: String,
    pub conversation_id: Option<String>,
    pub name: String,
    pub goal: Option<String>,
    pub state: String,
    pub state_data: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3RunStatus {
    pub session_id: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3Task {
    pub id: String,
    pub session_id: String,
    pub task_key: String,
    pub title: String,
    pub instruction: String,
    pub status: String,
    pub priority: i32,
    pub owner_agent_id: Option<String>,
    pub claimed_by_agent_id: Option<String>,
    pub claim_expires_at: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3TaskActionResult {
    pub task_id: String,
    pub task_key: Option<String>,
    pub title: Option<String>,
    pub status: String,
    pub owner_agent_id: Option<String>,
    pub claimed_by_agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3ThreadMessage {
    pub id: String,
    pub session_id: String,
    pub thread_id: String,
    pub from_agent_id: Option<String>,
    pub to_agent_id: Option<String>,
    pub message_type: String,
    pub payload: Value,
    pub created_at: String,
    pub sequence: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3PlanRevision {
    pub id: String,
    pub session_id: String,
    pub revision_no: i32,
    pub plan_json: Value,
    pub summary: String,
    pub status: String,
    pub requested_by: Option<String>,
    pub reviewed_by: Option<String>,
    pub review_note: Option<String>,
    pub created_at: String,
    pub reviewed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3BlackboardEntry {
    pub id: String,
    pub session_id: String,
    pub task_id: Option<String>,
    pub agent_id: Option<String>,
    pub entry_type: String,
    pub content: String,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3CreateSessionRequest {
    pub conversation_id: Option<String>,
    pub name: String,
    pub goal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3UpdateSessionRequest {
    pub name: Option<String>,
    pub goal: Option<String>,
    pub state: Option<String>,
    pub state_data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3CreateTaskRequest {
    pub task_key: String,
    pub title: String,
    pub instruction: String,
    pub priority: Option<i32>,
    pub owner_agent_id: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3ClaimTaskRequest {
    pub agent_id: String,
    pub ttl_secs: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3SendMessageRequest {
    pub thread_id: String,
    pub from_agent_id: Option<String>,
    pub to_agent_id: Option<String>,
    pub message_type: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3UpdateTaskStatusRequest {
    pub status: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3SubmitPlanRevisionRequest {
    pub plan_json: Value,
    pub summary: String,
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamV3ReviewPlanRevisionRequest {
    pub approve: bool,
    pub reviewed_by: Option<String>,
    pub review_note: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct TeamV3MemberProfile {
    pub(crate) name: String,
    pub(crate) responsibility: Option<String>,
    pub(crate) system_prompt: Option<String>,
    pub(crate) decision_style: Option<String>,
    pub(crate) risk_preference: Option<String>,
}

pub(crate) fn blackboard_revision_from_metadata_value(metadata: &Value) -> Option<i64> {
    let raw = metadata.get("revision")?;
    if let Some(value) = raw.as_i64() {
        return Some(value);
    }
    if let Some(value) = raw.as_u64() {
        return i64::try_from(value).ok();
    }
    if let Some(text) = raw.as_str() {
        return text.trim().parse::<i64>().ok();
    }
    None
}

fn normalize_blackboard_dedupe_content(content: &str) -> String {
    collapse_whitespace(content).to_lowercase()
}

fn should_dedupe_blackboard_entry_type(entry_type: &str) -> bool {
    matches!(entry_type, "goal" | "plan")
}

async fn has_recent_duplicate_blackboard_entry(
    runtime_pool: &DatabasePool,
    session_id: &str,
    entry_type: &str,
    task_id: Option<&str>,
    agent_id: Option<&str>,
    content: &str,
    scan_limit: i64,
) -> Result<bool> {
    let safe_limit = scan_limit.max(1);
    let rows = match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"SELECT task_id, agent_id, content
                   FROM team_v3_blackboard_entries
                   WHERE session_id = ? AND entry_type = ?
                   ORDER BY created_at DESC
                   LIMIT ?"#,
            )
            .bind(session_id)
            .bind(entry_type)
            .bind(safe_limit)
            .fetch_all(pool)
            .await?
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"SELECT task_id, agent_id, content
                   FROM team_v3_blackboard_entries
                   WHERE session_id = $1 AND entry_type = $2
                   ORDER BY created_at DESC
                   LIMIT $3"#,
            )
            .bind(session_id)
            .bind(entry_type)
            .bind(safe_limit)
            .fetch_all(pool)
            .await?
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    };

    let normalized_input = normalize_blackboard_dedupe_content(content);
    let normalized_task_id = task_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("-");
    let normalized_agent_id = agent_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("-");

    for row in rows {
        let row_task_id = row
            .try_get::<Option<String>, _>("task_id")
            .ok()
            .flatten()
            .unwrap_or_default();
        let row_agent_id = row
            .try_get::<Option<String>, _>("agent_id")
            .ok()
            .flatten()
            .unwrap_or_default();
        let row_content = row.try_get::<String, _>("content").unwrap_or_default();
        let normalized_row_task_id = row_task_id.trim();
        let normalized_row_agent_id = row_agent_id.trim();
        if normalized_row_task_id.is_empty() && normalized_task_id != "-" {
            continue;
        }
        if normalized_row_task_id != normalized_task_id && normalized_task_id != "-" {
            continue;
        }
        if normalized_row_agent_id.is_empty() && normalized_agent_id != "-" {
            continue;
        }
        if normalized_row_agent_id != normalized_agent_id && normalized_agent_id != "-" {
            continue;
        }
        if normalize_blackboard_dedupe_content(row_content.as_str()) == normalized_input {
            return Ok(true);
        }
    }

    Ok(false)
}

async fn get_team_v3_latest_blackboard_revision(
    runtime_pool: &DatabasePool,
    session_id: &str,
) -> Result<i64> {
    let metadata_text_opt: Option<String> = match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT metadata
                   FROM team_v3_blackboard_entries
                   WHERE session_id = ?
                   ORDER BY created_at DESC
                   LIMIT 1"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            row.map(|record| record.get("metadata"))
        }
        DatabasePool::PostgreSQL(pool) => {
            let row = sqlx::query(
                r#"SELECT metadata::text as metadata
                   FROM team_v3_blackboard_entries
                   WHERE session_id = $1
                   ORDER BY created_at DESC
                   LIMIT 1"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            row.map(|record| record.get("metadata"))
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    };

    let Some(metadata_text) = metadata_text_opt else {
        return Ok(0);
    };
    let metadata = parse_state_data_text(&metadata_text);
    Ok(blackboard_revision_from_metadata_value(&metadata).unwrap_or(0))
}

pub(crate) async fn append_team_v3_blackboard_entry(
    runtime_pool: &DatabasePool,
    session_id: &str,
    task_id: Option<&str>,
    agent_id: Option<&str>,
    entry_type: &str,
    content: &str,
    metadata: Option<&Value>,
) -> Result<()> {
    let blackboard_lock = team_v3_blackboard_session_lock(session_id);
    let _guard = blackboard_lock.lock().await;
    if should_dedupe_blackboard_entry_type(entry_type)
        && has_recent_duplicate_blackboard_entry(
            runtime_pool,
            session_id,
            entry_type,
            task_id,
            agent_id,
            content,
            12,
        )
        .await?
    {
        tracing::info!(
            "Team V3 blackboard duplicate skipped: session={} type={} agent={} task={}",
            session_id,
            entry_type,
            agent_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("-"),
            task_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("-"),
        );
        return Ok(());
    }
    let next_revision = get_team_v3_latest_blackboard_revision(runtime_pool, session_id).await? + 1;

    let mut metadata_json = metadata.cloned().unwrap_or_else(|| json!({}));
    if !metadata_json.is_object() {
        metadata_json = json!({ "payload": metadata_json });
    }
    if let Some(meta_obj) = metadata_json.as_object_mut() {
        meta_obj.insert("revision".to_string(), json!(next_revision));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let metadata_text = serde_json::to_string(&metadata_json)?;
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_blackboard_entries
                   (id, session_id, task_id, agent_id, entry_type, content, metadata, created_at, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(session_id)
            .bind(task_id)
            .bind(agent_id)
            .bind(entry_type)
            .bind(content)
            .bind(&metadata_text)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_blackboard_entries
                   (id, session_id, task_id, agent_id, entry_type, content, metadata, created_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8, $9)"#,
            )
            .bind(&id)
            .bind(session_id)
            .bind(task_id)
            .bind(agent_id)
            .bind(entry_type)
            .bind(content)
            .bind(&metadata_text)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }
    Ok(())
}

pub(crate) async fn list_team_v3_blackboard_entries(
    runtime_pool: &DatabasePool,
    session_id: &str,
    limit: i64,
) -> Result<Vec<TeamV3BlackboardEntry>> {
    let safe_limit = limit.max(1);
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, session_id, task_id, agent_id, entry_type, content, metadata, created_at, updated_at
                   FROM team_v3_blackboard_entries
                   WHERE session_id = ?
                   ORDER BY created_at DESC
                   LIMIT ?"#,
            )
            .bind(session_id)
            .bind(safe_limit)
            .fetch_all(pool)
            .await?;
            Ok(rows
                .into_iter()
                .map(|row| {
                    let metadata_text: String = row.get("metadata");
                    TeamV3BlackboardEntry {
                        id: row.get("id"),
                        session_id: row.get("session_id"),
                        task_id: row.get("task_id"),
                        agent_id: row.get("agent_id"),
                        entry_type: row.get("entry_type"),
                        content: row.get("content"),
                        metadata: parse_state_data_text(&metadata_text),
                        created_at: row.get("created_at"),
                        updated_at: row.get("updated_at"),
                    }
                })
                .collect())
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, session_id, task_id, agent_id, entry_type, content, metadata::text as metadata,
                          created_at::text as created_at, updated_at::text as updated_at
                   FROM team_v3_blackboard_entries
                   WHERE session_id = $1
                   ORDER BY created_at DESC
                   LIMIT $2"#,
            )
            .bind(session_id)
            .bind(safe_limit)
            .fetch_all(pool)
            .await?;
            Ok(rows
                .into_iter()
                .map(|row| {
                    let metadata_text: String = row.get("metadata");
                    TeamV3BlackboardEntry {
                        id: row.get("id"),
                        session_id: row.get("session_id"),
                        task_id: row.get("task_id"),
                        agent_id: row.get("agent_id"),
                        entry_type: row.get("entry_type"),
                        content: row.get("content"),
                        metadata: parse_state_data_text(&metadata_text),
                        created_at: row.get("created_at"),
                        updated_at: row.get("updated_at"),
                    }
                })
                .collect())
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V3 does not support MySQL")),
    }
}

pub(crate) fn collapse_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn clip_to_sentence_boundary(input: &str, max_chars: usize) -> Option<String> {
    if max_chars == 0 {
        return None;
    }
    let collapsed = collapse_whitespace(input);
    if collapsed.is_empty() {
        return None;
    }
    if collapsed.chars().count() <= max_chars {
        return Some(collapsed);
    }
    let mut count = 0usize;
    let mut best_end: Option<usize> = None;
    for (index, ch) in collapsed.char_indices() {
        count += 1;
        if count > max_chars {
            break;
        }
        if matches!(ch, '。' | '！' | '？' | '.' | '!' | '?' | ';' | '；' | '\n') {
            best_end = Some(index + ch.len_utf8());
        }
    }
    best_end
        .and_then(|end| collapsed.get(..end))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

pub(crate) fn normalize_fact_for_prompt(input: &str, max_chars: usize) -> Option<String> {
    let collapsed = collapse_whitespace(input);
    if collapsed.is_empty() {
        return None;
    }
    if collapsed.chars().count() <= max_chars {
        return Some(collapsed);
    }
    clip_to_sentence_boundary(collapsed.as_str(), max_chars)
}
pub(crate) async fn list_team_v3_tasks_internal(
    runtime_pool: &DatabasePool,
    session_id: &str,
) -> Result<Vec<TeamV3Task>> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, session_id, task_key, title, instruction, status, priority,
                          owner_agent_id, claimed_by_agent_id, claim_expires_at,
                          acceptance_criteria, metadata, created_at, updated_at
                   FROM team_v3_tasks WHERE session_id = ?
                   ORDER BY priority ASC, created_at ASC"#,
            )
            .bind(session_id)
            .fetch_all(pool)
            .await?;
            rows.into_iter()
                .map(|r| {
                    let metadata_text: String = r.get("metadata");
                    let metadata: Value =
                        serde_json::from_str(&metadata_text).unwrap_or_else(|_| json!({}));
                    Ok(TeamV3Task {
                        id: r.get("id"),
                        session_id: r.get("session_id"),
                        task_key: r.get("task_key"),
                        title: r.get("title"),
                        instruction: r.get("instruction"),
                        status: r.get("status"),
                        priority: r.get("priority"),
                        owner_agent_id: r.get("owner_agent_id"),
                        claimed_by_agent_id: r.get("claimed_by_agent_id"),
                        claim_expires_at: r.get("claim_expires_at"),
                        acceptance_criteria: r.get("acceptance_criteria"),
                        metadata,
                        created_at: r.get("created_at"),
                        updated_at: r.get("updated_at"),
                    })
                })
                .collect()
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, session_id, task_key, title, instruction, status, priority,
                          owner_agent_id, claimed_by_agent_id,
                          claim_expires_at::text as claim_expires_at,
                          acceptance_criteria, metadata::text as metadata,
                          created_at::text as created_at, updated_at::text as updated_at
                   FROM team_v3_tasks WHERE session_id = $1
                   ORDER BY priority ASC, created_at ASC"#,
            )
            .bind(session_id)
            .fetch_all(pool)
            .await?;
            rows.into_iter()
                .map(|r| {
                    let metadata_text: String = r.get("metadata");
                    let metadata: Value =
                        serde_json::from_str(&metadata_text).unwrap_or_else(|_| json!({}));
                    Ok(TeamV3Task {
                        id: r.get("id"),
                        session_id: r.get("session_id"),
                        task_key: r.get("task_key"),
                        title: r.get("title"),
                        instruction: r.get("instruction"),
                        status: r.get("status"),
                        priority: r.get("priority"),
                        owner_agent_id: r.get("owner_agent_id"),
                        claimed_by_agent_id: r.get("claimed_by_agent_id"),
                        claim_expires_at: r.get("claim_expires_at"),
                        acceptance_criteria: r.get("acceptance_criteria"),
                        metadata,
                        created_at: r.get("created_at"),
                        updated_at: r.get("updated_at"),
                    })
                })
                .collect()
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V3 does not support MySQL")),
    }
}

async fn append_team_v3_member_message(
    runtime_pool: &DatabasePool,
    session_id: &str,
    member_id: &str,
    task_id: &str,
    task_key: &str,
    content: &str,
) -> Result<()> {
    let message_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let payload = json!({
        "content": content,
        "task_id": task_id,
        "task_key": task_key
    });
    let payload_text = serde_json::to_string(&payload)?;
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_messages
                   (id, session_id, thread_id, from_agent_id, to_agent_id, message_type, message_kind, payload, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&message_id)
            .bind(session_id)
            .bind(session_id)
            .bind(member_id)
            .bind(Option::<String>::None)
            .bind("assistant")
            .bind("chat")
            .bind(&payload_text)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_messages
                   (id, session_id, thread_id, from_agent_id, to_agent_id, message_type, message_kind, payload, created_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9)"#,
            )
            .bind(&message_id)
            .bind(session_id)
            .bind(session_id)
            .bind(member_id)
            .bind(Option::<String>::None)
            .bind("assistant")
            .bind("chat")
            .bind(&payload_text)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }
    Ok(())
}

async fn resolve_team_v3_provider_config(
    ai_manager: &AiServiceManager,
) -> Result<crate::services::AiConfig> {
    let (provider, model) = ai_manager
        .get_default_llm_model()
        .await?
        .ok_or_else(|| anyhow!("Default chat model is not configured"))?;
    let mut provider_config = ai_manager
        .get_provider_config(&provider)
        .await?
        .ok_or_else(|| anyhow!("Provider configuration not found: {}", provider))?;
    provider_config.model = model;
    Ok(provider_config)
}

fn team_v3_default_tool_config() -> ToolConfig {
    ToolConfig {
        enabled: true,
        selection_strategy: ToolSelectionStrategy::Keyword,
        max_tools: 5,
        fixed_tools: vec!["interactive_shell".to_string()],
        disabled_tools: Vec::new(),
        allowed_tools: Vec::new(),
    }
}

async fn load_team_v3_tool_config(app_handle: &AppHandle) -> ToolConfig {
    let default = team_v3_default_tool_config();
    let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() else {
        tracing::info!(
            "Team V3 tool config: database state unavailable, using default config (strategy={:?}, enabled={})",
            default.selection_strategy,
            default.enabled
        );
        return default;
    };

    match db.get_config("agent", "tool_config").await {
        Ok(Some(config_str)) => match serde_json::from_str::<ToolConfig>(&config_str) {
            Ok(config) => {
                tracing::info!(
                    "Team V3 tool config loaded from DB (strategy={:?}, enabled={}, max_tools={})",
                    config.selection_strategy,
                    config.enabled,
                    config.max_tools
                );
                config
            }
            Err(err) => {
                tracing::warn!(
                    "Team V3 tool config parse failed, using default config: {}",
                    err
                );
                default
            }
        },
        Ok(None) => {
            tracing::info!(
                "Team V3 tool config missing in DB, using default config (strategy={:?}, enabled={})",
                default.selection_strategy,
                default.enabled
            );
            default
        }
        Err(err) => {
            tracing::warn!(
                "Team V3 tool config query failed, using default config: {}",
                err
            );
            default
        }
    }
}

pub(crate) async fn run_team_v3_execution_orchestrator(
    runtime_pool: DatabasePool,
    app_handle: AppHandle,
    ai_manager: Arc<AiServiceManager>,
    session_id: String,
    generation: u64,
    cancellation_token: CancellationToken,
    goal_text: String,
    user_input: String,
    state_data: Value,
    _rag_enabled: bool,
    tool_config_override: Option<ToolConfig>,
) -> Result<String> {
    let provider_config = resolve_team_v3_provider_config(ai_manager.as_ref()).await?;
    let team_tool_config = if let Some(config) = tool_config_override {
        config
    } else {
        load_team_v3_tool_config(&app_handle).await
    };
    let rig_provider = provider_config
        .rig_provider
        .clone()
        .unwrap_or_else(|| provider_config.provider.clone());
    let model = provider_config.model.clone();
    let max_iterations = provider_config.max_turns.unwrap_or(24).max(8) as usize;
    let timeout_secs = 1800_u64;
    let mut output_by_task_id: HashMap<String, String> = HashMap::new();
    let mut summary: Option<String> = None;
    let (members, execution_state_data) = prepare_team_v3_execution_tasks_with_main_agent(
        &runtime_pool,
        &app_handle,
        &session_id,
        goal_text.as_str(),
        user_input.as_str(),
        &state_data,
        &provider_config,
        rig_provider.as_str(),
        model.as_str(),
        &cancellation_token,
        &team_tool_config,
    )
    .await?;
    let member_profiles = team_member_profiles(&execution_state_data);

    loop {
        if is_team_execution_cancelled(&session_id, generation) || cancellation_token.is_cancelled()
        {
            return Err(anyhow!("Team execution cancelled"));
        }

        let tasks = list_team_v3_tasks_internal(&runtime_pool, &session_id).await?;
        if tasks.is_empty() {
            return Ok("Team 执行完成。".to_string());
        }

        let mut status_by_id: HashMap<String, String> = HashMap::new();
        let mut id_by_task_key: HashMap<String, String> = HashMap::new();
        let mut task_key_by_id: HashMap<String, String> = HashMap::new();
        for task in tasks.iter() {
            status_by_id.insert(task.id.clone(), task.status.clone());
            id_by_task_key.insert(task.task_key.clone(), task.id.clone());
            task_key_by_id.insert(task.id.clone(), task.task_key.clone());
        }

        let runnable_tasks = tasks
            .iter()
            .filter(|task| {
                matches!(
                    task.status.as_str(),
                    "pending" | "ready_for_claim" | "claimed" | "running"
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        if runnable_tasks.is_empty() {
            break;
        }

        let mut ready_tasks: Vec<(TeamV3Task, Vec<String>, Vec<String>)> = Vec::new();
        for task in runnable_tasks {
            let raw_dependencies = parse_task_dependencies(&task.metadata);
            let dependency_task_keys = raw_dependencies
                .iter()
                .filter_map(|dependency| {
                    if id_by_task_key.contains_key(dependency) {
                        return Some(dependency.clone());
                    }
                    task_key_by_id.get(dependency).cloned()
                })
                .collect::<Vec<_>>();
            let dependencies = raw_dependencies
                .into_iter()
                .filter_map(|dependency| {
                    if status_by_id.contains_key(&dependency) {
                        Some(dependency)
                    } else {
                        id_by_task_key.get(&dependency).cloned()
                    }
                })
                .collect::<Vec<_>>();
            let all_dependencies_done = dependencies.iter().all(|dependency_id| {
                status_by_id
                    .get(dependency_id)
                    .map(|status| status == "completed" || status == "cancelled")
                    .unwrap_or(false)
            });
            if all_dependencies_done {
                ready_tasks.push((task, dependencies, dependency_task_keys));
            }
        }

        if ready_tasks.is_empty() {
            return Err(anyhow!("Team 任务存在循环依赖或未满足依赖，无法继续执行"));
        }

        let mut wave_requests = Vec::new();
        for (index, (task, dependencies, dependency_task_keys)) in
            ready_tasks.into_iter().enumerate()
        {
            let member_id = select_team_member_for_task(&task, &members, index);
            let member_system_prompt = build_team_member_execution_system_prompt(
                member_id.as_str(),
                member_profiles.get(member_id.as_str()),
            );
            let member_runtime_context = build_team_member_runtime_context(
                member_id.as_str(),
                member_profiles.get(member_id.as_str()),
            );
            set_team_v3_task_execution_state(
                &runtime_pool,
                &session_id,
                &task.id,
                "running",
                Some(member_id.as_str()),
                None,
            )
            .await?;
            let start_note = format!("开始执行任务：{}", task.title);
            let start_meta = json!({ "task_key": task.task_key });
            append_team_v3_blackboard_entry(
                &runtime_pool,
                &session_id,
                Some(task.id.as_str()),
                Some(member_id.as_str()),
                "task_start",
                start_note.as_str(),
                Some(&start_meta),
            )
            .await?;

            let is_summary_task = is_team_v3_summary_task(&task);
            let dependency_context = if is_summary_task {
                String::new()
            } else {
                dependencies
                    .iter()
                    .filter_map(|dependency_id| {
                        output_by_task_id.get(dependency_id).map(|output| {
                            let compact_output =
                                truncate_chars(collapse_whitespace(output.as_str()).as_str(), 900);
                            format!("依赖任务 {} 输出：\n{}", dependency_id, compact_output)
                        })
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n")
            };
            let blackboard_entries = list_team_v3_blackboard_entries(
                &runtime_pool,
                &session_id,
                if is_summary_task { 96 } else { 48 },
            )
            .await?;
            let task_query = TeamV3PromptQuery::for_task(
                goal_text.as_str(),
                user_input.as_str(),
                &task,
                dependencies.as_slice(),
                dependency_task_keys.as_slice(),
            );
            let blackboard_context_result = if is_summary_task {
                build_blackboard_checkpoint_context(&blackboard_entries, &task_query)
            } else {
                build_blackboard_context(&blackboard_entries, &task_query)
            };
            tracing::info!(
                "Team V3 task blackboard context: session={} task_key={} mode={} terms={} chars={} structured={}/{} task_outputs={}/{} artifacts={}/{} events={}/{} checkpoints={}/{} dropped_total={}",
                session_id,
                task.task_key,
                blackboard_context_result.diagnostics.mode,
                blackboard_context_result.diagnostics.query_terms,
                blackboard_context_result.diagnostics.context_chars,
                blackboard_context_result.diagnostics.structured_memory.selected,
                blackboard_context_result.diagnostics.structured_memory.total,
                blackboard_context_result.diagnostics.task_outputs.selected,
                blackboard_context_result.diagnostics.task_outputs.total,
                blackboard_context_result.diagnostics.artifacts.selected,
                blackboard_context_result.diagnostics.artifacts.total,
                blackboard_context_result.diagnostics.raw_events.selected,
                blackboard_context_result.diagnostics.raw_events.total,
                blackboard_context_result.diagnostics.checkpoints.selected,
                blackboard_context_result.diagnostics.checkpoints.total,
                blackboard_context_result
                    .diagnostics
                    .structured_memory
                    .dropped()
                    + blackboard_context_result.diagnostics.task_outputs.dropped()
                    + blackboard_context_result.diagnostics.artifacts.dropped()
                    + blackboard_context_result.diagnostics.raw_events.dropped()
                    + blackboard_context_result.diagnostics.checkpoints.dropped(),
            );
            let blackboard_context = blackboard_context_result.context;
            let blackboard_snapshot_revision = latest_blackboard_revision(&blackboard_entries);
            let prompt = build_team_v3_task_prompt(
                goal_text.as_str(),
                user_input.as_str(),
                &task,
                member_runtime_context.as_str(),
                dependency_context.as_str(),
                blackboard_context.as_str(),
                blackboard_snapshot_revision,
                is_summary_task,
            );
            wave_requests.push(TeamWaveTaskExecutionRequest {
                task_id: task.id.clone(),
                task_key: task.task_key.clone(),
                task_title: task.title.clone(),
                member_id: member_id.clone(),
                prompt,
                system_prompt: member_system_prompt,
                dependency_task_ids: dependencies.clone(),
                dependency_task_keys: dependency_task_keys.clone(),
                is_summary_task,
                max_iterations,
                timeout_secs,
                tool_config: team_tool_config.clone(),
            });
        }

        let wave_results = execute_team_wave_tasks(
            &app_handle,
            &session_id,
            rig_provider.as_str(),
            model.as_str(),
            provider_config.api_key.clone(),
            provider_config.api_base.clone(),
            &cancellation_token,
            wave_requests,
        )
        .await?;

        let mut wave_error: Option<String> = None;
        for result in wave_results {
            let TeamWaveTaskExecutionResult {
                task_id,
                task_key,
                task_title,
                member_id,
                dependency_task_ids,
                dependency_task_keys,
                is_summary_task,
                execution_result,
            } = result;
            match execution_result {
                Ok(output) => {
                    let content = output.trim().to_string();
                    let normalized_content = if content.is_empty() {
                        "任务已执行完成，但未返回文本结果。".to_string()
                    } else {
                        content
                    };
                    let mut output_artifact: Option<TeamV3ArtifactFileRef> = None;
                    output_by_task_id.insert(task_id.clone(), normalized_content.clone());
                    set_team_v3_task_execution_state(
                        &runtime_pool,
                        &session_id,
                        &task_id,
                        "completed",
                        Some(member_id.as_str()),
                        None,
                    )
                    .await?;
                    append_team_v3_member_message(
                        &runtime_pool,
                        &session_id,
                        member_id.as_str(),
                        task_id.as_str(),
                        task_key.as_str(),
                        normalized_content.as_str(),
                    )
                    .await?;
                    if normalized_content.chars().count() > TEAM_V3_BLACKBOARD_INLINE_CHAR_LIMIT {
                        let artifact_result = persist_team_v3_task_output_artifact(
                            session_id.as_str(),
                            task_id.as_str(),
                            task_key.as_str(),
                            member_id.as_str(),
                            task_title.as_str(),
                            normalized_content.as_str(),
                        )
                        .await;
                        match artifact_result {
                            Ok(artifact_ref) => {
                                output_artifact = Some(artifact_ref.clone());
                                let summary_text =
                                    build_task_artifact_summary(normalized_content.as_str());
                                let artifact_content = build_task_artifact_ref_content(
                                    summary_text.as_str(),
                                    artifact_ref.path.as_str(),
                                    artifact_ref.bytes,
                                );
                                let output_meta = json!({
                                    "task_key": task_key,
                                    "summary": summary_text,
                                    "artifact": {
                                        "path": artifact_ref.path,
                                        "bytes": artifact_ref.bytes,
                                        "host_path": artifact_ref.host_path,
                                        "container_path": artifact_ref.container_path
                                    }
                                });
                                append_team_v3_blackboard_entry(
                                    &runtime_pool,
                                    &session_id,
                                    Some(task_id.as_str()),
                                    Some(member_id.as_str()),
                                    "artifact_ref",
                                    artifact_content.as_str(),
                                    Some(&output_meta),
                                )
                                .await?;
                            }
                            Err(error) => {
                                tracing::warn!(
                                    "Team task {} artifact persist failed, fallback to inline blackboard entry: {}",
                                    task_key,
                                    error
                                );
                                let output_meta = json!({
                                    "task_key": task_key,
                                    "artifact_fallback": "inline",
                                    "artifact_error": error.to_string(),
                                });
                                append_team_v3_blackboard_entry(
                                    &runtime_pool,
                                    &session_id,
                                    Some(task_id.as_str()),
                                    Some(member_id.as_str()),
                                    "task_output",
                                    normalized_content.as_str(),
                                    Some(&output_meta),
                                )
                                .await?;
                            }
                        }
                    } else {
                        let output_meta = json!({ "task_key": task_key });
                        append_team_v3_blackboard_entry(
                            &runtime_pool,
                            &session_id,
                            Some(task_id.as_str()),
                            Some(member_id.as_str()),
                            "task_output",
                            normalized_content.as_str(),
                            Some(&output_meta),
                        )
                        .await?;
                    }
                    append_team_v3_task_memory_layers(
                        &runtime_pool,
                        &session_id,
                        task_id.as_str(),
                        member_id.as_str(),
                        task_key.as_str(),
                        task_title.as_str(),
                        normalized_content.as_str(),
                        dependency_task_ids.as_slice(),
                        dependency_task_keys.as_slice(),
                        output_artifact.as_ref(),
                    )
                    .await?;
                    if is_summary_task || summary.is_none() {
                        summary = Some(normalized_content.chars().take(300).collect());
                    }
                }
                Err(e) => {
                    let error_text = e.to_string();
                    set_team_v3_task_execution_state(
                        &runtime_pool,
                        &session_id,
                        &task_id,
                        "failed",
                        Some(member_id.as_str()),
                        Some(error_text.as_str()),
                    )
                    .await?;
                    let status_message = format!("任务 {} 执行失败：{}", task_key, error_text);
                    append_team_v3_status_message(
                        &runtime_pool,
                        &session_id,
                        status_message.as_str(),
                    )
                    .await?;
                    let error_meta = json!({ "task_key": task_key });
                    append_team_v3_blackboard_entry(
                        &runtime_pool,
                        &session_id,
                        Some(task_id.as_str()),
                        Some(member_id.as_str()),
                        "task_error",
                        error_text.as_str(),
                        Some(&error_meta),
                    )
                    .await?;
                    if wave_error.is_none() {
                        wave_error = Some(error_text);
                    }
                }
            }
        }

        let tasks_after_wave = list_team_v3_tasks_internal(&runtime_pool, &session_id).await?;
        append_team_v3_dependency_ready_notices(
            &runtime_pool,
            &session_id,
            &tasks,
            &tasks_after_wave,
        )
        .await?;

        if let Some(error) = wave_error {
            return Err(anyhow!(error));
        }
    }

    Ok(summary.unwrap_or_else(|| "Team 执行完成。".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::team_v3_blackboard_context::{
        build_blackboard_context, dedupe_event_entries_for_prompt, split_blackboard_layers,
        TeamV3PromptQuery,
    };
    use crate::commands::team_v3_memory::{
        build_structured_fact_payload_from_checkpoint_entry, build_task_checkpoint_payload,
    };
    use crate::commands::team_v3_planner::parse_execution_plan;

    fn build_checkpoint_entry(
        revision: i64,
        task_key: &str,
        title: &str,
        highlights: Vec<&str>,
    ) -> TeamV3BlackboardEntry {
        TeamV3BlackboardEntry {
            id: format!("entry-{}", revision),
            session_id: "session-1".to_string(),
            task_id: Some(format!("task-{}", revision)),
            agent_id: Some("agent-a".to_string()),
            entry_type: "checkpoint".to_string(),
            content: format!(
                "task_key={}\ntitle={}\nkey_points:\n{}",
                task_key,
                title,
                highlights
                    .iter()
                    .map(|item| format!("- {}", item))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            metadata: json!({
                "revision": revision,
                "task_key": task_key,
                "task_title": title,
                "facts": {
                    "highlights": highlights,
                }
            }),
            created_at: format!("2026-03-04T00:00:{:02}Z", revision.min(59)),
            updated_at: format!("2026-03-04T00:00:{:02}Z", revision.min(59)),
        }
    }

    #[test]
    fn parse_execution_plan_accepts_wrapped_json_text() {
        let raw = r#"
分析说明：先给出简短描述。
{
  "summary": "router 审计",
  "agents": [
    {
      "id": "security-engineer",
      "name": "安全工程师"
    }
  ],
  "tasks": [
    {
      "task_key": "audit-router",
      "title": "审计 router",
      "instruction": "检查 source-sink"
    }
  ]
}
补充说明：以上为计划。
"#;
        let plan = parse_execution_plan(raw);
        assert!(plan.is_some(), "planner wrapped JSON should be parsed");
        let plan = plan.unwrap();
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.tasks[0].task_key, "audit-router");
    }

    #[test]
    fn build_task_checkpoint_payload_keeps_structured_facts() {
        let output = r#"
- 结论: 已发现 338 个 Controller，可生成全量路由映射。
- 依据: 通过批量解析 public 方法并关联模块命名空间。
- 风险: 多数路由未限制 HTTP 方法，存在 CSRF 与未授权访问风险。
- 下一步: 对高危路由执行 source-sink 审计，重点检查 SQL 注入和 RCE。
"#;
        let payload = build_task_checkpoint_payload("analyze-router", "分析路由", output);
        assert!(payload.content.contains("task_key=analyze-router"));
        assert_eq!(
            payload
                .facts
                .get("conclusion")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            "结论: 已发现 338 个 Controller，可生成全量路由映射。"
        );
        assert!(payload
            .facts
            .get("risk")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .contains("风险:"));
        assert!(payload
            .facts
            .get("next_step")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .contains("下一步:"));
    }

    #[test]
    fn blackboard_context_prioritizes_relevant_checkpoint() {
        let mut entries = (1..=18)
            .map(|rev| {
                build_checkpoint_entry(
                    rev,
                    format!("checkpoint-{}", rev).as_str(),
                    "常规阶段总结",
                    vec!["结论: 常规检查完成。", "下一步: 继续扫描。"],
                )
            })
            .collect::<Vec<_>>();
        entries.push(build_checkpoint_entry(
            19,
            "router-source-sink-audit",
            "router source-sink 安全审计",
            vec![
                "结论: 已完成 router path source-sink 建模。",
                "风险: 发现 SQL 注入与 RCE 风险点。",
                "下一步: 对未授权访问风险做 PoC 验证。",
            ],
        ));

        let query = TeamV3PromptQuery {
            goal: "针对 router path 做安全审计".to_string(),
            user_input: "关注 source-sink、sql注入、rce、未授权访问".to_string(),
            task_title: "审计 router".to_string(),
            task_instruction: "复用已有 router 清单，不要重复扫描".to_string(),
            dependency_task_ids: Vec::new(),
            dependency_task_keys: vec!["router-source-sink-audit".to_string()],
        };
        let context = build_blackboard_context(&entries, &query);
        assert!(context.context.contains("router-source-sink-audit"));
        assert!(context.context.contains("SQL 注入与 RCE 风险点"));
        assert!(
            context.diagnostics.checkpoints.selected <= 12,
            "checkpoint selection should keep bounded prompt size"
        );
        assert!(
            context.diagnostics.checkpoints.dropped() > 0,
            "when entries exceed limit, diagnostics should report dropped checkpoints"
        );
    }

    #[test]
    fn structured_backfill_skips_placeholder_only_checkpoint() {
        let checkpoint = build_checkpoint_entry(
            21,
            "placeholder-checkpoint",
            "空摘要",
            vec!["结论", "依据", "风险", "下一步"],
        );
        let structured = build_structured_fact_payload_from_checkpoint_entry(&checkpoint);
        assert!(
            structured.is_none(),
            "placeholder-only checkpoint should not be backfilled into structured memory"
        );
    }

    #[test]
    fn structured_backfill_extracts_meaningful_checkpoint() {
        let checkpoint = build_checkpoint_entry(
            22,
            "router-security-audit",
            "router 安全审计",
            vec![
                "结论: 已梳理关键 router path 与调用链。",
                "依据: 基于 Controller public 方法与路由映射关系。",
                "风险: 存在 SQL 注入与未授权访问风险点。",
                "下一步: 对高危路径进行 source-sink 深度验证。",
            ],
        );
        let structured = build_structured_fact_payload_from_checkpoint_entry(&checkpoint);
        assert!(
            structured.is_some(),
            "meaningful checkpoint should be backfilled"
        );
        let structured = structured.unwrap();
        assert!(structured.content.contains("router-security-audit"));
        assert!(structured.tags.iter().any(|tag| tag == "sql-injection"));
    }

    #[test]
    fn team_events_dedupe_removes_resend_duplicate_goal() {
        let duplicate_text = "针对所有router进行安全审计，关注router path 的source-sink";
        let entries = vec![
            TeamV3BlackboardEntry {
                id: "goal-1".to_string(),
                session_id: "session-1".to_string(),
                task_id: None,
                agent_id: Some("human".to_string()),
                entry_type: "goal".to_string(),
                content: duplicate_text.to_string(),
                metadata: json!({ "revision": 19 }),
                created_at: "2026-03-04T00:00:19Z".to_string(),
                updated_at: "2026-03-04T00:00:19Z".to_string(),
            },
            TeamV3BlackboardEntry {
                id: "goal-2".to_string(),
                session_id: "session-1".to_string(),
                task_id: None,
                agent_id: Some("human".to_string()),
                entry_type: "goal".to_string(),
                content: duplicate_text.to_string(),
                metadata: json!({ "revision": 23 }),
                created_at: "2026-03-04T00:00:23Z".to_string(),
                updated_at: "2026-03-04T00:00:23Z".to_string(),
            },
            TeamV3BlackboardEntry {
                id: "plan-1".to_string(),
                session_id: "session-1".to_string(),
                task_id: None,
                agent_id: Some("planner".to_string()),
                entry_type: "plan".to_string(),
                content: "基于已扫描路由清单执行安全审计".to_string(),
                metadata: json!({ "revision": 24 }),
                created_at: "2026-03-04T00:00:24Z".to_string(),
                updated_at: "2026-03-04T00:00:24Z".to_string(),
            },
        ];
        let layers = split_blackboard_layers(&entries);
        let deduped = dedupe_event_entries_for_prompt(&layers.raw_events);
        let goal_count = deduped
            .iter()
            .filter(|entry| entry.entry_type == "goal")
            .count();
        assert_eq!(goal_count, 1, "duplicated resend goals should be deduped");
    }
}

use crate::commands::ai::{
    cancel_conversation_stream, emit_agent_execution_finished, AgentExecutionOutcome,
};
use crate::commands::ai_runtime_commands::{agent_execute, AgentExecuteConfig};
use crate::models::database::AiMessage;
use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use crate::utils::ai_generation_settings::apply_generation_settings_from_db;
use chrono::Utc;
use sentinel_db::{database_service::connection_manager::DatabasePool, Database};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

static PARALLEL_RUN_CHILDREN: std::sync::LazyLock<Mutex<HashMap<String, Vec<String>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));
static PARALLEL_CHILD_TO_RUN: std::sync::LazyLock<Mutex<HashMap<String, String>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

const PARALLEL_RUN_SELECT_COLUMNS: &str = concat!(
    "id, parent_conversation_id, task, status, aggregation_mode, ",
    "judge_provider, judge_model, judge_status, judge_content, judge_error, ",
    "created_at_ms, updated_at_ms"
);

const PARALLEL_ITEM_SELECT_COLUMNS: &str = concat!(
    "id, model_run_id, message_id, provider, model, status, content, error, ",
    "tool_count, input_tokens, output_tokens, first_response_ms, cost_usd, started_at_ms, completed_at_ms"
);

const PARALLEL_EVENT_SELECT_COLUMNS: &str = concat!(
    "id, run_id, model_run_id, event_type, title, content, success, ",
    "tool_name, tool_args, tool_result, tool_status, tool_call_id, timestamp_ms"
);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParallelModelTarget {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParallelAggregationMode {
    Manual,
    Judge,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentExecuteParallelRequest {
    pub task: String,
    pub config: Option<AgentExecuteConfig>,
    pub models: Vec<ParallelModelTarget>,
    #[serde(default)]
    pub aggregation_mode: Option<ParallelAggregationMode>,
    #[serde(default)]
    pub judge_model: Option<ParallelModelTarget>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParallelModelRunInfo {
    pub id: String,
    pub model_run_id: String,
    pub message_id: String,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub content: String,
    pub error: Option<String>,
    pub tool_count: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub first_response_ms: Option<i64>,
    pub cost_usd: Option<f64>,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub events: Vec<ParallelRunEventInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelRunEventInfo {
    pub id: String,
    pub run_id: String,
    pub model_run_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub title: String,
    pub content: String,
    pub success: Option<bool>,
    pub tool_name: Option<String>,
    pub tool_args: Option<serde_json::Value>,
    pub tool_result: Option<serde_json::Value>,
    pub tool_status: Option<String>,
    pub tool_call_id: Option<String>,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParallelRunDetail {
    pub id: String,
    pub parent_conversation_id: String,
    pub task: String,
    pub status: String,
    pub aggregation_mode: String,
    pub judge_provider: Option<String>,
    pub judge_model: Option<String>,
    pub judge_status: String,
    pub judge_content: Option<String>,
    pub judge_error: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub items: Vec<ParallelModelRunInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentExecuteParallelResponse {
    pub parallel_run_id: String,
    pub parent_execution_id: String,
    pub items: Vec<ParallelModelRunInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CancelAiParallelRunRequest {
    pub parallel_run_id: String,
    pub parent_execution_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetAiParallelRunRequest {
    pub parallel_run_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecordAiParallelRunEventRequest {
    pub id: String,
    pub parallel_run_id: String,
    pub model_run_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub title: String,
    pub content: String,
    pub success: Option<bool>,
    pub tool_name: Option<String>,
    pub tool_args: Option<serde_json::Value>,
    pub tool_result: Option<serde_json::Value>,
    pub tool_status: Option<String>,
    pub tool_call_id: Option<String>,
    pub timestamp_ms: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CancelAiParallelModelRunRequest {
    pub parallel_run_id: String,
    pub model_run_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetryAiParallelModelRunRequest {
    pub parallel_run_id: String,
    pub model_run_id: String,
    pub provider: String,
    pub model: String,
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

fn normalize_model_targets(
    models: Vec<ParallelModelTarget>,
) -> Result<Vec<ParallelModelTarget>, String> {
    let mut seen = std::collections::HashSet::new();
    let mut normalized = Vec::new();
    for item in models {
        let provider = item.provider.trim().to_lowercase();
        let model = item.model.trim().to_string();
        if provider.is_empty() || model.is_empty() {
            return Err("Parallel model target requires provider and model".to_string());
        }
        if seen.insert(format!("{}/{}", provider, model)) {
            normalized.push(ParallelModelTarget { provider, model });
        }
    }
    if normalized.len() < 2 {
        return Err("Parallel execution requires at least two distinct models".to_string());
    }
    Ok(normalized)
}

async fn ensure_parallel_schema(db: &DatabaseService) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let ddl = [
                r#"CREATE TABLE IF NOT EXISTS ai_parallel_runs (
                    id TEXT PRIMARY KEY,
                    parent_conversation_id TEXT NOT NULL,
                    task TEXT NOT NULL,
                    status TEXT NOT NULL,
                    aggregation_mode TEXT NOT NULL,
                    judge_provider TEXT,
                    judge_model TEXT,
                    judge_status TEXT NOT NULL DEFAULT 'not_requested',
                    judge_content TEXT,
                    judge_error TEXT,
                    created_at_ms INTEGER NOT NULL,
                    updated_at_ms INTEGER NOT NULL
                )"#,
                r#"CREATE TABLE IF NOT EXISTS ai_parallel_run_items (
                    id TEXT PRIMARY KEY,
                    run_id TEXT NOT NULL,
                    model_run_id TEXT NOT NULL UNIQUE,
                    message_id TEXT NOT NULL,
                    provider TEXT NOT NULL,
                    model TEXT NOT NULL,
                    status TEXT NOT NULL,
                    content TEXT NOT NULL DEFAULT '',
                    error TEXT,
                    tool_count INTEGER NOT NULL DEFAULT 0,
                    input_tokens INTEGER NOT NULL DEFAULT 0,
                    output_tokens INTEGER NOT NULL DEFAULT 0,
                    first_response_ms INTEGER,
                    cost_usd REAL,
                    started_at_ms INTEGER,
                    completed_at_ms INTEGER,
                    updated_at_ms INTEGER NOT NULL
                )"#,
                r#"CREATE TABLE IF NOT EXISTS ai_parallel_run_events (
                    id TEXT PRIMARY KEY,
                    run_id TEXT NOT NULL,
                    model_run_id TEXT NOT NULL,
                    event_type TEXT NOT NULL,
                    title TEXT NOT NULL,
                    content TEXT NOT NULL DEFAULT '',
                    success INTEGER,
                    tool_name TEXT,
                    tool_args TEXT,
                    tool_result TEXT,
                    tool_status TEXT,
                    tool_call_id TEXT,
                    timestamp_ms INTEGER NOT NULL
                )"#,
                "CREATE INDEX IF NOT EXISTS idx_ai_parallel_runs_parent ON ai_parallel_runs(parent_conversation_id, updated_at_ms)",
                "CREATE INDEX IF NOT EXISTS idx_ai_parallel_items_run ON ai_parallel_run_items(run_id)",
                "CREATE INDEX IF NOT EXISTS idx_ai_parallel_items_model_run ON ai_parallel_run_items(model_run_id)",
                "CREATE INDEX IF NOT EXISTS idx_ai_parallel_events_run ON ai_parallel_run_events(run_id, timestamp_ms)",
                "CREATE INDEX IF NOT EXISTS idx_ai_parallel_events_model_run ON ai_parallel_run_events(model_run_id, timestamp_ms)",
            ];
            for sql in ddl {
                sqlx::query(sql)
                    .execute(&pool)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            ensure_sqlite_column(
                &pool,
                "ai_parallel_run_items",
                "first_response_ms",
                "INTEGER",
            )
            .await?;
            ensure_sqlite_column(&pool, "ai_parallel_run_items", "cost_usd", "REAL").await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            let ddl = [
                r#"CREATE TABLE IF NOT EXISTS ai_parallel_runs (
                    id TEXT PRIMARY KEY,
                    parent_conversation_id TEXT NOT NULL,
                    task TEXT NOT NULL,
                    status TEXT NOT NULL,
                    aggregation_mode TEXT NOT NULL,
                    judge_provider TEXT,
                    judge_model TEXT,
                    judge_status TEXT NOT NULL DEFAULT 'not_requested',
                    judge_content TEXT,
                    judge_error TEXT,
                    created_at_ms BIGINT NOT NULL,
                    updated_at_ms BIGINT NOT NULL
                )"#,
                r#"CREATE TABLE IF NOT EXISTS ai_parallel_run_items (
                    id TEXT PRIMARY KEY,
                    run_id TEXT NOT NULL,
                    model_run_id TEXT NOT NULL UNIQUE,
                    message_id TEXT NOT NULL,
                    provider TEXT NOT NULL,
                    model TEXT NOT NULL,
                    status TEXT NOT NULL,
                    content TEXT NOT NULL DEFAULT '',
                    error TEXT,
                    tool_count BIGINT NOT NULL DEFAULT 0,
                    input_tokens BIGINT NOT NULL DEFAULT 0,
                    output_tokens BIGINT NOT NULL DEFAULT 0,
                    first_response_ms BIGINT,
                    cost_usd DOUBLE PRECISION,
                    started_at_ms BIGINT,
                    completed_at_ms BIGINT,
                    updated_at_ms BIGINT NOT NULL
                )"#,
                r#"CREATE TABLE IF NOT EXISTS ai_parallel_run_events (
                    id TEXT PRIMARY KEY,
                    run_id TEXT NOT NULL,
                    model_run_id TEXT NOT NULL,
                    event_type TEXT NOT NULL,
                    title TEXT NOT NULL,
                    content TEXT NOT NULL DEFAULT '',
                    success BIGINT,
                    tool_name TEXT,
                    tool_args TEXT,
                    tool_result TEXT,
                    tool_status TEXT,
                    tool_call_id TEXT,
                    timestamp_ms BIGINT NOT NULL
                )"#,
                "CREATE INDEX IF NOT EXISTS idx_ai_parallel_runs_parent ON ai_parallel_runs(parent_conversation_id, updated_at_ms)",
                "CREATE INDEX IF NOT EXISTS idx_ai_parallel_items_run ON ai_parallel_run_items(run_id)",
                "CREATE INDEX IF NOT EXISTS idx_ai_parallel_items_model_run ON ai_parallel_run_items(model_run_id)",
                "CREATE INDEX IF NOT EXISTS idx_ai_parallel_events_run ON ai_parallel_run_events(run_id, timestamp_ms)",
                "CREATE INDEX IF NOT EXISTS idx_ai_parallel_events_model_run ON ai_parallel_run_events(model_run_id, timestamp_ms)",
            ];
            for sql in ddl {
                sqlx::query(sql)
                    .execute(&pool)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            sqlx::query("ALTER TABLE ai_parallel_run_items ADD COLUMN IF NOT EXISTS first_response_ms BIGINT")
                .execute(&pool)
                .await
                .map_err(|e| e.to_string())?;
            sqlx::query("ALTER TABLE ai_parallel_run_items ADD COLUMN IF NOT EXISTS cost_usd DOUBLE PRECISION")
                .execute(&pool)
                .await
                .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => {
            return Err("Parallel execution does not support MySQL".to_string())
        }
    }
    Ok(())
}

async fn ensure_sqlite_column(
    pool: &sqlx::SqlitePool,
    table: &str,
    column: &str,
    column_type: &str,
) -> Result<(), String> {
    let rows = sqlx::query(&format!("PRAGMA table_info({})", table))
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
    let exists = rows.iter().any(|row| {
        row.try_get::<String, _>("name")
            .map(|name| name == column)
            .unwrap_or(false)
    });
    if exists {
        return Ok(());
    }
    sqlx::query(&format!(
        "ALTER TABLE {} ADD COLUMN {} {}",
        table, column, column_type
    ))
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

async fn insert_parallel_run(db: &DatabaseService, run: &ParallelRunDetail) -> Result<(), String> {
    ensure_parallel_schema(db).await?;
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO ai_parallel_runs
                   (id, parent_conversation_id, task, status, aggregation_mode, judge_provider, judge_model, judge_status, created_at_ms, updated_at_ms)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&run.id)
            .bind(&run.parent_conversation_id)
            .bind(&run.task)
            .bind(&run.status)
            .bind(&run.aggregation_mode)
            .bind(&run.judge_provider)
            .bind(&run.judge_model)
            .bind(&run.judge_status)
            .bind(run.created_at_ms)
            .bind(run.updated_at_ms)
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO ai_parallel_runs
                   (id, parent_conversation_id, task, status, aggregation_mode, judge_provider, judge_model, judge_status, created_at_ms, updated_at_ms)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"#,
            )
            .bind(&run.id)
            .bind(&run.parent_conversation_id)
            .bind(&run.task)
            .bind(&run.status)
            .bind(&run.aggregation_mode)
            .bind(&run.judge_provider)
            .bind(&run.judge_model)
            .bind(&run.judge_status)
            .bind(run.created_at_ms)
            .bind(run.updated_at_ms)
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => {
            return Err("Parallel execution does not support MySQL".to_string())
        }
    }
    Ok(())
}

async fn insert_parallel_item(
    db: &DatabaseService,
    item: &ParallelModelRunInfo,
    run_id: &str,
) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO ai_parallel_run_items
                   (id, run_id, model_run_id, message_id, provider, model, status, content, tool_count, input_tokens, output_tokens, first_response_ms, started_at_ms, updated_at_ms)
                   VALUES (?, ?, ?, ?, ?, ?, ?, '', 0, 0, 0, NULL, ?, ?)"#,
            )
            .bind(&item.id)
            .bind(run_id)
            .bind(&item.model_run_id)
            .bind(&item.message_id)
            .bind(&item.provider)
            .bind(&item.model)
            .bind(&item.status)
            .bind(item.started_at_ms)
            .bind(now_ms())
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO ai_parallel_run_items
                   (id, run_id, model_run_id, message_id, provider, model, status, content, tool_count, input_tokens, output_tokens, first_response_ms, started_at_ms, updated_at_ms)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, '', 0, 0, 0, NULL, $8, $9)"#,
            )
            .bind(&item.id)
            .bind(run_id)
            .bind(&item.model_run_id)
            .bind(&item.message_id)
            .bind(&item.provider)
            .bind(&item.model)
            .bind(&item.status)
            .bind(item.started_at_ms)
            .bind(now_ms())
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => {
            return Err("Parallel execution does not support MySQL".to_string())
        }
    }
    Ok(())
}

async fn insert_parallel_event(
    db: &DatabaseService,
    event: &ParallelRunEventInfo,
) -> Result<(), String> {
    ensure_parallel_schema(db).await?;
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT OR REPLACE INTO ai_parallel_run_events
                   (id, run_id, model_run_id, event_type, title, content, success, tool_name, tool_args, tool_result, tool_status, tool_call_id, timestamp_ms)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&event.id)
            .bind(&event.run_id)
            .bind(&event.model_run_id)
            .bind(&event.event_type)
            .bind(&event.title)
            .bind(&event.content)
            .bind(event.success.map(|value| if value { 1 } else { 0 }))
            .bind(&event.tool_name)
            .bind(event.tool_args.as_ref().map(|value| value.to_string()))
            .bind(event.tool_result.as_ref().map(|value| value.to_string()))
            .bind(&event.tool_status)
            .bind(&event.tool_call_id)
            .bind(event.timestamp_ms)
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO ai_parallel_run_events
                   (id, run_id, model_run_id, event_type, title, content, success, tool_name, tool_args, tool_result, tool_status, tool_call_id, timestamp_ms)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                   ON CONFLICT (id) DO UPDATE SET
                     event_type = EXCLUDED.event_type,
                     title = EXCLUDED.title,
                     content = EXCLUDED.content,
                     success = EXCLUDED.success,
                     tool_name = EXCLUDED.tool_name,
                     tool_args = EXCLUDED.tool_args,
                     tool_result = EXCLUDED.tool_result,
                     tool_status = EXCLUDED.tool_status,
                     tool_call_id = EXCLUDED.tool_call_id,
                     timestamp_ms = EXCLUDED.timestamp_ms"#,
            )
            .bind(&event.id)
            .bind(&event.run_id)
            .bind(&event.model_run_id)
            .bind(&event.event_type)
            .bind(&event.title)
            .bind(&event.content)
            .bind(event.success)
            .bind(&event.tool_name)
            .bind(event.tool_args.as_ref().map(|value| value.to_string()))
            .bind(event.tool_result.as_ref().map(|value| value.to_string()))
            .bind(&event.tool_status)
            .bind(&event.tool_call_id)
            .bind(event.timestamp_ms)
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => {
            return Err("Parallel execution does not support MySQL".to_string())
        }
    }
    Ok(())
}

async fn load_parallel_run(
    db: &DatabaseService,
    run_id: &str,
) -> Result<Option<ParallelRunDetail>, String> {
    ensure_parallel_schema(db).await?;
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let Some(row) = sqlx::query(&format!(
                "SELECT {} FROM ai_parallel_runs WHERE id = ?",
                PARALLEL_RUN_SELECT_COLUMNS,
            ))
            .bind(run_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| e.to_string())?
            else {
                return Ok(None);
            };
            let item_rows = sqlx::query(&format!(
                "SELECT {} FROM ai_parallel_run_items WHERE run_id = ? ORDER BY started_at_ms ASC, id ASC",
                PARALLEL_ITEM_SELECT_COLUMNS,
            ))
                .bind(run_id)
                .fetch_all(&pool)
                .await
                .map_err(|e| e.to_string())?;
            let event_rows = sqlx::query(&format!(
                "SELECT {} FROM ai_parallel_run_events WHERE run_id = ? ORDER BY timestamp_ms ASC, id ASC",
                PARALLEL_EVENT_SELECT_COLUMNS,
            ))
                .bind(run_id)
                .fetch_all(&pool)
                .await
                .map_err(|e| e.to_string())?;
            Ok(Some(map_run_row(row, item_rows, event_rows)?))
        }
        DatabasePool::PostgreSQL(pool) => {
            let Some(row) = sqlx::query(&format!(
                "SELECT {} FROM ai_parallel_runs WHERE id = $1",
                PARALLEL_RUN_SELECT_COLUMNS,
            ))
            .bind(run_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| e.to_string())?
            else {
                return Ok(None);
            };
            let item_rows = sqlx::query(&format!(
                "SELECT {} FROM ai_parallel_run_items WHERE run_id = $1 ORDER BY started_at_ms ASC, id ASC",
                PARALLEL_ITEM_SELECT_COLUMNS,
            ))
                .bind(run_id)
                .fetch_all(&pool)
                .await
                .map_err(|e| e.to_string())?;
            let event_rows = sqlx::query(&format!(
                "SELECT {} FROM ai_parallel_run_events WHERE run_id = $1 ORDER BY timestamp_ms ASC, id ASC",
                PARALLEL_EVENT_SELECT_COLUMNS,
            ))
                .bind(run_id)
                .fetch_all(&pool)
                .await
                .map_err(|e| e.to_string())?;
            Ok(Some(map_run_row(row, item_rows, event_rows)?))
        }
        DatabasePool::MySQL(_) => Err("Parallel execution does not support MySQL".to_string()),
    }
}

fn map_run_row<R, I, E>(
    row: R,
    item_rows: Vec<I>,
    event_rows: Vec<E>,
) -> Result<ParallelRunDetail, String>
where
    R: Row,
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    String: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    i64: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    f64: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    I: Row<Database = R::Database>,
    for<'r> &'r str: sqlx::ColumnIndex<I>,
    E: Row<Database = R::Database>,
    for<'r> &'r str: sqlx::ColumnIndex<E>,
{
    let mut events_by_model: HashMap<String, Vec<ParallelRunEventInfo>> = HashMap::new();
    for event in event_rows.into_iter().map(map_event_row) {
        let event = event?;
        events_by_model
            .entry(event.model_run_id.clone())
            .or_default()
            .push(event);
    }
    let mut items = item_rows
        .into_iter()
        .map(map_item_row)
        .collect::<Result<Vec<_>, _>>()?;
    for item in &mut items {
        item.events = events_by_model
            .remove(&item.model_run_id)
            .unwrap_or_default();
    }
    Ok(ParallelRunDetail {
        id: row.try_get("id").map_err(|e| e.to_string())?,
        parent_conversation_id: row
            .try_get("parent_conversation_id")
            .map_err(|e| e.to_string())?,
        task: row.try_get("task").map_err(|e| e.to_string())?,
        status: row.try_get("status").map_err(|e| e.to_string())?,
        aggregation_mode: row.try_get("aggregation_mode").map_err(|e| e.to_string())?,
        judge_provider: row.try_get("judge_provider").ok(),
        judge_model: row.try_get("judge_model").ok(),
        judge_status: row.try_get("judge_status").map_err(|e| e.to_string())?,
        judge_content: row.try_get("judge_content").ok(),
        judge_error: row.try_get("judge_error").ok(),
        created_at_ms: row.try_get("created_at_ms").map_err(|e| e.to_string())?,
        updated_at_ms: row.try_get("updated_at_ms").map_err(|e| e.to_string())?,
        items,
    })
}

fn map_item_row<R>(row: R) -> Result<ParallelModelRunInfo, String>
where
    R: Row,
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    String: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    i64: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    f64: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    Ok(ParallelModelRunInfo {
        id: row.try_get("id").map_err(|e| e.to_string())?,
        model_run_id: row.try_get("model_run_id").map_err(|e| e.to_string())?,
        message_id: row.try_get("message_id").map_err(|e| e.to_string())?,
        provider: row.try_get("provider").map_err(|e| e.to_string())?,
        model: row.try_get("model").map_err(|e| e.to_string())?,
        status: row.try_get("status").map_err(|e| e.to_string())?,
        content: row.try_get("content").unwrap_or_default(),
        error: row.try_get("error").ok(),
        tool_count: row.try_get("tool_count").unwrap_or(0),
        input_tokens: row.try_get("input_tokens").unwrap_or(0),
        output_tokens: row.try_get("output_tokens").unwrap_or(0),
        first_response_ms: row.try_get("first_response_ms").ok(),
        cost_usd: row.try_get("cost_usd").ok(),
        started_at_ms: row.try_get("started_at_ms").ok(),
        completed_at_ms: row.try_get("completed_at_ms").ok(),
        events: Vec::new(),
    })
}

fn map_event_row<R>(row: R) -> Result<ParallelRunEventInfo, String>
where
    R: Row,
    for<'r> &'r str: sqlx::ColumnIndex<R>,
    String: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    i64: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    i64: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    let tool_args_raw: Option<String> = row.try_get("tool_args").ok();
    let tool_result_raw: Option<String> = row.try_get("tool_result").ok();
    let success_raw: Option<i64> = row.try_get("success").ok();
    Ok(ParallelRunEventInfo {
        id: row.try_get("id").map_err(|e| e.to_string())?,
        run_id: row.try_get("run_id").map_err(|e| e.to_string())?,
        model_run_id: row.try_get("model_run_id").map_err(|e| e.to_string())?,
        event_type: row.try_get("event_type").map_err(|e| e.to_string())?,
        title: row.try_get("title").unwrap_or_default(),
        content: row.try_get("content").unwrap_or_default(),
        success: success_raw.map(|value| value != 0),
        tool_name: row.try_get("tool_name").ok(),
        tool_args: tool_args_raw.and_then(|value| serde_json::from_str(&value).ok()),
        tool_result: tool_result_raw.and_then(|value| serde_json::from_str(&value).ok()),
        tool_status: row.try_get("tool_status").ok(),
        tool_call_id: row.try_get("tool_call_id").ok(),
        timestamp_ms: row.try_get("timestamp_ms").map_err(|e| e.to_string())?,
    })
}

#[derive(Default)]
struct ParallelSessionStats {
    input_tokens: i64,
    output_tokens: i64,
    first_response_ms: Option<i64>,
    cost_usd: Option<f64>,
}

fn extract_session_stats(messages: &[AiMessage]) -> ParallelSessionStats {
    let Some(metadata) = messages
        .iter()
        .rev()
        .find(|message| message.role == "assistant")
        .and_then(|message| message.metadata.as_ref())
    else {
        return ParallelSessionStats::default();
    };
    let Ok(metadata_json) = serde_json::from_str::<serde_json::Value>(metadata) else {
        return ParallelSessionStats::default();
    };
    let Some(stats) = metadata_json.get("session_stats") else {
        return ParallelSessionStats::default();
    };
    ParallelSessionStats {
        input_tokens: stats
            .get("input_tokens")
            .and_then(|value| value.as_i64())
            .unwrap_or(0),
        output_tokens: stats
            .get("output_tokens")
            .and_then(|value| value.as_i64())
            .unwrap_or(0),
        first_response_ms: stats
            .get("first_response_ms")
            .and_then(|value| value.as_i64()),
        cost_usd: None,
    }
}

async fn update_item_finished(
    db: &DatabaseService,
    model_run_id: &str,
    status: &str,
    content: &str,
    error: Option<&str>,
    tool_count: i64,
    stats: &ParallelSessionStats,
) -> Result<(), String> {
    ensure_parallel_schema(db).await?;
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    let completed_at = now_ms();
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE ai_parallel_run_items
                   SET status = ?, content = ?, error = ?, tool_count = ?, input_tokens = ?, output_tokens = ?, first_response_ms = ?, cost_usd = ?, completed_at_ms = ?, updated_at_ms = ?
                   WHERE model_run_id = ?"#,
            )
            .bind(status)
            .bind(content)
            .bind(error)
            .bind(tool_count)
            .bind(stats.input_tokens)
            .bind(stats.output_tokens)
            .bind(stats.first_response_ms)
            .bind(stats.cost_usd)
            .bind(completed_at)
            .bind(completed_at)
            .bind(model_run_id)
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE ai_parallel_run_items
                   SET status = $1, content = $2, error = $3, tool_count = $4, input_tokens = $5, output_tokens = $6, first_response_ms = $7, cost_usd = $8, completed_at_ms = $9, updated_at_ms = $10
                   WHERE model_run_id = $11"#,
            )
            .bind(status)
            .bind(content)
            .bind(error)
            .bind(tool_count)
            .bind(stats.input_tokens)
            .bind(stats.output_tokens)
            .bind(stats.first_response_ms)
            .bind(stats.cost_usd)
            .bind(completed_at)
            .bind(completed_at)
            .bind(model_run_id)
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => {
            return Err("Parallel execution does not support MySQL".to_string())
        }
    }
    Ok(())
}

async fn update_run_status(db: &DatabaseService, run_id: &str, status: &str) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    let updated_at = now_ms();
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query("UPDATE ai_parallel_runs SET status = ?, updated_at_ms = ? WHERE id = ?")
                .bind(status)
                .bind(updated_at)
                .bind(run_id)
                .execute(&pool)
                .await
                .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                "UPDATE ai_parallel_runs SET status = $1, updated_at_ms = $2 WHERE id = $3",
            )
            .bind(status)
            .bind(updated_at)
            .bind(run_id)
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => {
            return Err("Parallel execution does not support MySQL".to_string())
        }
    }
    Ok(())
}

async fn update_judge_result(
    db: &DatabaseService,
    run_id: &str,
    status: &str,
    content: Option<&str>,
    error: Option<&str>,
) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    let updated_at = now_ms();
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                "UPDATE ai_parallel_runs SET judge_status = ?, judge_content = ?, judge_error = ?, updated_at_ms = ? WHERE id = ?",
            )
            .bind(status)
            .bind(content)
            .bind(error)
            .bind(updated_at)
            .bind(run_id)
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                "UPDATE ai_parallel_runs SET judge_status = $1, judge_content = $2, judge_error = $3, updated_at_ms = $4 WHERE id = $5",
            )
            .bind(status)
            .bind(content)
            .bind(error)
            .bind(updated_at)
            .bind(run_id)
            .execute(&pool)
            .await
            .map_err(|e| e.to_string())?;
        }
        DatabasePool::MySQL(_) => {
            return Err("Parallel execution does not support MySQL".to_string())
        }
    }
    Ok(())
}

async fn emit_parent_user_message(
    app_handle: &AppHandle,
    db: &DatabaseService,
    parent_execution_id: &str,
    task: &str,
    config: &AgentExecuteConfig,
) {
    let message_id = Uuid::new_v4().to_string();
    let display_text = config.display_content.as_deref().unwrap_or(task);
    let msg = AiMessage {
        id: message_id.clone(),
        conversation_id: parent_execution_id.to_string(),
        role: "user".to_string(),
        content: task.to_string(),
        metadata: None,
        token_count: Some(task.len() as i32),
        cost: None,
        tool_calls: None,
        attachments: config
            .attachments
            .as_ref()
            .and_then(|value| serde_json::to_string(value).ok()),
        reasoning_content: None,
        timestamp: Utc::now(),
        architecture_type: None,
        architecture_meta: None,
        structured_data: config
            .display_content
            .as_ref()
            .map(|content| serde_json::json!({ "display_content": content }).to_string()),
    };
    if let Err(err) = db.create_ai_message(&msg).await {
        tracing::warn!("Failed to persist parallel parent user message: {}", err);
    }
    let _ = app_handle.emit(
        "agent:user_message",
        &serde_json::json!({
            "execution_id": parent_execution_id,
            "message_id": message_id,
            "content": display_text,
            "timestamp": msg.timestamp.timestamp_millis(),
            "document_attachments": config.document_attachments,
            "image_attachments": config.attachments,
            "referenced_files": config.referenced_files,
            "referenced_messages": config.referenced_messages,
            "referenced_assets": config.referenced_assets,
            "referenced_traffic": config.referenced_traffic,
        }),
    );
}

async fn persist_parent_parallel_message(
    db: &DatabaseService,
    parent_execution_id: &str,
    parallel_run_id: &str,
) -> Result<(), String> {
    let message_id = format!("parallel-summary:{}", parallel_run_id);
    let metadata = serde_json::json!({
        "kind": "parallel_model_execution",
        "parallel_run_id": parallel_run_id,
    });
    let msg = AiMessage {
        id: message_id,
        conversation_id: parent_execution_id.to_string(),
        role: "system".to_string(),
        content: "Parallel model execution".to_string(),
        metadata: Some(metadata.to_string()),
        token_count: None,
        cost: None,
        tool_calls: None,
        attachments: None,
        reasoning_content: None,
        timestamp: Utc::now(),
        architecture_type: None,
        architecture_meta: None,
        structured_data: None,
    };
    db.upsert_ai_message_append(&msg)
        .await
        .map_err(|e| e.to_string())
}

fn clone_config_for_child(
    base: &AgentExecuteConfig,
    child_execution_id: &str,
    child_message_id: &str,
    model: &ParallelModelTarget,
) -> AgentExecuteConfig {
    let mut child = base.clone();
    child.conversation_id = Some(child_execution_id.to_string());
    child.message_id = Some(child_message_id.to_string());
    child.model_override = Some(format!("{}/{}", model.provider, model.model));
    child.persist_messages = Some(true);
    child
}

fn extract_tool_count(messages: &[AiMessage]) -> i64 {
    messages
        .iter()
        .filter(|message| message.role == "tool")
        .count() as i64
}

async fn run_judge_summary(
    app_handle: &AppHandle,
    ai_manager: &Arc<AiServiceManager>,
    run: &ParallelRunDetail,
) -> Result<String, String> {
    let Some(provider) = run.judge_provider.as_deref() else {
        return Err("Judge provider is not configured".to_string());
    };
    let Some(model) = run.judge_model.as_deref() else {
        return Err("Judge model is not configured".to_string());
    };
    let provider_config = ai_manager
        .get_provider_config(provider)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Provider '{}' configuration not found", provider))?;
    let mut config = provider_config;
    config.model = model.to_string();
    let db = app_handle
        .try_state::<Arc<DatabaseService>>()
        .ok_or_else(|| "Database service not initialized".to_string())?;
    let llm_config = apply_generation_settings_from_db(
        db.inner().as_ref(),
        sentinel_llm::AiService::new(config).to_llm_config(),
    )
    .await;
    let client = sentinel_llm::LlmClient::new(llm_config);
    let mut model_sections = String::new();
    for item in &run.items {
        model_sections.push_str(&format!(
            "\n\n[{} / {} / status={}]\n{}",
            item.provider, item.model, item.status, item.content
        ));
        if let Some(error) = item.error.as_deref() {
            model_sections.push_str(&format!("\nError: {}", error));
        }
    }
    let prompt = format!(
        "Task:\n{}\n\nModel outputs:{}\n\nReturn strict JSON with keys: best_model, confidence (0-1), final_answer, disagreements (array), model_notes (array of objects with model, strengths, weaknesses), and recommendation. Do not include markdown fences. Do not invent facts not present in the outputs.",
        run.task, model_sections
    );
    client
        .completion(
            Some("You are an impartial judge for parallel AI assistant executions."),
            &prompt,
        )
        .await
        .map_err(|e| e.to_string())
}

async fn finish_parallel_run_if_ready(
    app_handle: AppHandle,
    ai_manager: Arc<AiServiceManager>,
    run_id: String,
) -> Result<(), String> {
    let db = app_handle
        .try_state::<Arc<DatabaseService>>()
        .ok_or_else(|| "Database service not initialized".to_string())?;
    let Some(mut run) = load_parallel_run(db.inner(), &run_id).await? else {
        return Ok(());
    };
    let all_done = run
        .items
        .iter()
        .all(|item| matches!(item.status.as_str(), "succeeded" | "failed" | "cancelled"));
    if !all_done {
        emit_parallel_run_updated(&app_handle, &run);
        return Ok(());
    }
    let has_success = run.items.iter().any(|item| item.status == "succeeded");
    let final_status = if has_success { "succeeded" } else { "failed" };
    update_run_status(db.inner(), &run_id, final_status).await?;
    run.status = final_status.to_string();
    emit_parallel_run_updated(&app_handle, &run);

    if run.aggregation_mode == "judge" && run.judge_status == "pending" {
        update_judge_result(db.inner(), &run_id, "running", None, None).await?;
        match run_judge_summary(&app_handle, &ai_manager, &run).await {
            Ok(summary) => {
                update_judge_result(db.inner(), &run_id, "succeeded", Some(&summary), None).await?;
            }
            Err(err) => {
                update_judge_result(db.inner(), &run_id, "failed", None, Some(&err)).await?;
            }
        }
        if let Some(updated) = load_parallel_run(db.inner(), &run_id).await? {
            emit_parallel_run_updated(&app_handle, &updated);
        }
    }
    Ok(())
}

fn emit_parallel_run_updated(app_handle: &AppHandle, run: &ParallelRunDetail) {
    let _ = app_handle.emit("agent:parallel_run_updated", run);
}

pub fn record_parallel_child_finished(
    app_handle: &AppHandle,
    ai_manager: Arc<AiServiceManager>,
    model_run_id: &str,
    outcome: AgentExecutionOutcome,
    error: Option<String>,
    response: Option<String>,
) {
    let run_id = PARALLEL_CHILD_TO_RUN
        .lock()
        .ok()
        .and_then(|guard| guard.get(model_run_id).cloned());
    let Some(run_id) = run_id else {
        return;
    };
    let app = app_handle.clone();
    let model_run_id = model_run_id.to_string();
    tauri::async_runtime::spawn(async move {
        let Some(db) = app.try_state::<Arc<DatabaseService>>() else {
            return;
        };
        let messages = db
            .get_ai_messages_by_conversation(&model_run_id)
            .await
            .unwrap_or_default();
        let content = response.unwrap_or_else(|| {
            messages
                .iter()
                .rev()
                .find(|message| message.role == "assistant")
                .map(|message| message.content.clone())
                .unwrap_or_default()
        });
        let status = match outcome {
            AgentExecutionOutcome::Succeeded => "succeeded",
            AgentExecutionOutcome::Failed => "failed",
            AgentExecutionOutcome::Cancelled => "cancelled",
        };
        let mut stats = extract_session_stats(&messages);
        if stats.input_tokens > 0 || stats.output_tokens > 0 {
            if let Ok(Some(run)) = load_parallel_run(db.inner(), &run_id).await {
                if let Some(item) = run
                    .items
                    .iter()
                    .find(|item| item.model_run_id == model_run_id)
                {
                    stats.cost_usd = Some(sentinel_llm::calculate_cost(
                        &item.provider,
                        &item.model,
                        stats.input_tokens as u32,
                        stats.output_tokens as u32,
                    ));
                }
            }
        }
        if let Err(err) = update_item_finished(
            db.inner(),
            &model_run_id,
            status,
            &content,
            error.as_deref(),
            extract_tool_count(&messages),
            &stats,
        )
        .await
        {
            tracing::warn!("Failed to update parallel child {}: {}", model_run_id, err);
            return;
        }
        if let Err(err) = finish_parallel_run_if_ready(app, ai_manager, run_id).await {
            tracing::warn!("Failed to finish parallel run: {}", err);
        }
    });
}

#[tauri::command]
pub async fn agent_execute_parallel(
    request: AgentExecuteParallelRequest,
    app_handle: AppHandle,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<AgentExecuteParallelResponse, String> {
    sentinel_license::ensure_feature_access(sentinel_license::LicensedFeature::AiRuntime)?;
    let models = normalize_model_targets(request.models)?;
    let mode = request
        .aggregation_mode
        .unwrap_or(ParallelAggregationMode::Manual);
    let judge_model = match mode {
        ParallelAggregationMode::Manual => None,
        ParallelAggregationMode::Judge => Some(
            request
                .judge_model
                .clone()
                .ok_or_else(|| "Judge aggregation requires judge_model".to_string())?,
        ),
    };
    for model in models.iter().chain(judge_model.iter()) {
        ai_manager
            .get_provider_config(&model.provider)
            .await
            .map_err(|err| {
                format!(
                    "Failed to load provider config '{}': {}",
                    model.provider, err
                )
            })?
            .ok_or_else(|| format!("Provider '{}' configuration not found", model.provider))?;
    }

    let mut base_config = request.config.unwrap_or_else(empty_agent_config);
    let parent_execution_id = base_config
        .conversation_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    base_config.conversation_id = Some(parent_execution_id.clone());
    let parallel_run_id = Uuid::new_v4().to_string();
    let db = app_handle.state::<Arc<DatabaseService>>();
    ensure_parallel_schema(db.inner()).await?;

    emit_parent_user_message(
        &app_handle,
        db.inner(),
        &parent_execution_id,
        &request.task,
        &base_config,
    )
    .await;
    persist_parent_parallel_message(db.inner(), &parent_execution_id, &parallel_run_id).await?;

    let aggregation_mode = match mode {
        ParallelAggregationMode::Manual => "manual",
        ParallelAggregationMode::Judge => "judge",
    };
    let run = ParallelRunDetail {
        id: parallel_run_id.clone(),
        parent_conversation_id: parent_execution_id.clone(),
        task: request.task.clone(),
        status: "running".to_string(),
        aggregation_mode: aggregation_mode.to_string(),
        judge_provider: judge_model.as_ref().map(|item| item.provider.clone()),
        judge_model: judge_model.as_ref().map(|item| item.model.clone()),
        judge_status: if aggregation_mode == "judge" {
            "pending".to_string()
        } else {
            "not_requested".to_string()
        },
        judge_content: None,
        judge_error: None,
        created_at_ms: now_ms(),
        updated_at_ms: now_ms(),
        items: Vec::new(),
    };
    insert_parallel_run(db.inner(), &run).await?;

    let items = models
        .iter()
        .map(|model| ParallelModelRunInfo {
            id: Uuid::new_v4().to_string(),
            model_run_id: format!("parallel:{}:{}", parallel_run_id, Uuid::new_v4()),
            message_id: Uuid::new_v4().to_string(),
            provider: model.provider.clone(),
            model: model.model.clone(),
            status: "running".to_string(),
            content: String::new(),
            error: None,
            tool_count: 0,
            input_tokens: 0,
            output_tokens: 0,
            first_response_ms: None,
            cost_usd: None,
            started_at_ms: Some(now_ms()),
            completed_at_ms: None,
            events: Vec::new(),
        })
        .collect::<Vec<_>>();
    for item in &items {
        insert_parallel_item(db.inner(), item, &parallel_run_id).await?;
    }
    if let Ok(mut guard) = PARALLEL_RUN_CHILDREN.lock() {
        guard.insert(
            parallel_run_id.clone(),
            items.iter().map(|item| item.model_run_id.clone()).collect(),
        );
    }
    if let Ok(mut guard) = PARALLEL_CHILD_TO_RUN.lock() {
        for item in &items {
            guard.insert(item.model_run_id.clone(), parallel_run_id.clone());
        }
    }

    let mut event_run = run.clone();
    event_run.items = items.clone();
    let _ = app_handle.emit("agent:parallel_run_started", &event_run);

    for (model, item) in models.iter().zip(items.iter()) {
        let child_config =
            clone_config_for_child(&base_config, &item.model_run_id, &item.message_id, model);
        if let Err(err) = agent_execute(
            request.task.clone(),
            Some(child_config),
            app_handle.clone(),
            ai_manager.inner().clone(),
        )
        .await
        {
            update_item_finished(
                db.inner(),
                &item.model_run_id,
                "failed",
                "",
                Some(&err),
                0,
                &ParallelSessionStats::default(),
            )
            .await?;
            if let Some(updated) = load_parallel_run(db.inner(), &parallel_run_id).await? {
                emit_parallel_run_updated(&app_handle, &updated);
            }
            finish_parallel_run_if_ready(
                app_handle.clone(),
                ai_manager.inner().clone(),
                parallel_run_id.clone(),
            )
            .await?;
        }
    }

    Ok(AgentExecuteParallelResponse {
        parallel_run_id,
        parent_execution_id,
        items,
    })
}

fn empty_agent_config() -> AgentExecuteConfig {
    AgentExecuteConfig {
        conversation_id: None,
        execution_id: None,
        message_id: None,
        enable_rag: Some(false),
        attachments: None,
        document_attachments: None,
        tool_config: None,
        traffic_context: None,
        display_content: None,
        current_browser_shell_direct_write_enabled: None,
        current_browser_shell_session_id: None,
        current_terminal_session_fingerprint: None,
        current_terminal_session_id: None,
        working_directory: None,
        referenced_files: None,
        referenced_messages: None,
        referenced_assets: None,
        referenced_traffic: None,
        model_override: None,
        context_mode: None,
        max_iterations: None,
        timeout_secs: None,
        force_tasks: None,
        harness_mode: None,
        harness_max_continuations: None,
        enable_tenth_man_rule: None,
        tenth_man_config: None,
        persist_messages: None,
    }
}

#[tauri::command]
pub async fn get_ai_parallel_run(
    request: GetAiParallelRunRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<ParallelRunDetail, String> {
    load_parallel_run(db.inner(), &request.parallel_run_id)
        .await?
        .ok_or_else(|| format!("Parallel run '{}' not found", request.parallel_run_id))
}

#[tauri::command]
pub async fn record_ai_parallel_run_event(
    request: RecordAiParallelRunEventRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<ParallelRunEventInfo, String> {
    let event = ParallelRunEventInfo {
        id: request.id,
        run_id: request.parallel_run_id,
        model_run_id: request.model_run_id,
        event_type: request.event_type,
        title: request.title,
        content: request.content,
        success: request.success,
        tool_name: request.tool_name,
        tool_args: request.tool_args,
        tool_result: request.tool_result,
        tool_status: request.tool_status,
        tool_call_id: request.tool_call_id,
        timestamp_ms: request.timestamp_ms.unwrap_or_else(now_ms),
    };
    insert_parallel_event(db.inner(), &event).await?;
    Ok(event)
}

#[tauri::command]
pub async fn cancel_ai_parallel_model_run(
    request: CancelAiParallelModelRunRequest,
    app_handle: AppHandle,
) -> Result<(), String> {
    cancel_conversation_stream(&request.model_run_id);
    let _ = crate::managers::cancellation_manager::cancel_execution(&request.model_run_id).await;
    let _ =
        sentinel_tools::buildin_tools::shell::cancel_shell_execution(&request.model_run_id).await;
    if let Err(err) =
        sentinel_tools::buildin_tools::shell_background::stop_background_shell_tasks_for_execution(
            &request.model_run_id,
        )
        .await
    {
        tracing::warn!(
            "Failed to stop background shell tasks for {}: {}",
            request.model_run_id,
            err
        );
    }
    if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
        update_item_finished(
            db.inner(),
            &request.model_run_id,
            "cancelled",
            "",
            Some("Cancelled by user"),
            0,
            &ParallelSessionStats::default(),
        )
        .await?;
        if let Ok(Some(run)) = load_parallel_run(db.inner(), &request.parallel_run_id).await {
            emit_parallel_run_updated(&app_handle, &run);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn retry_ai_parallel_model_run(
    request: RetryAiParallelModelRunRequest,
    app_handle: AppHandle,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<ParallelModelRunInfo, String> {
    let db = app_handle.state::<Arc<DatabaseService>>();
    let Some(run) = load_parallel_run(db.inner(), &request.parallel_run_id).await? else {
        return Err(format!(
            "Parallel run '{}' not found",
            request.parallel_run_id
        ));
    };
    let model = ParallelModelTarget {
        provider: request.provider.trim().to_lowercase(),
        model: request.model.trim().to_string(),
    };
    if model.provider.is_empty() || model.model.is_empty() {
        return Err("Retry model requires provider and model".to_string());
    }
    ai_manager
        .get_provider_config(&model.provider)
        .await
        .map_err(|err| {
            format!(
                "Failed to load provider config '{}': {}",
                model.provider, err
            )
        })?
        .ok_or_else(|| format!("Provider '{}' configuration not found", model.provider))?;

    let item = ParallelModelRunInfo {
        id: Uuid::new_v4().to_string(),
        model_run_id: format!("parallel:{}:{}", request.parallel_run_id, Uuid::new_v4()),
        message_id: Uuid::new_v4().to_string(),
        provider: model.provider.clone(),
        model: model.model.clone(),
        status: "running".to_string(),
        content: String::new(),
        error: None,
        tool_count: 0,
        input_tokens: 0,
        output_tokens: 0,
        first_response_ms: None,
        cost_usd: None,
        started_at_ms: Some(now_ms()),
        completed_at_ms: None,
        events: Vec::new(),
    };
    insert_parallel_item(db.inner(), &item, &request.parallel_run_id).await?;
    if let Ok(mut guard) = PARALLEL_RUN_CHILDREN.lock() {
        guard
            .entry(request.parallel_run_id.clone())
            .or_default()
            .push(item.model_run_id.clone());
    }
    if let Ok(mut guard) = PARALLEL_CHILD_TO_RUN.lock() {
        guard.insert(item.model_run_id.clone(), request.parallel_run_id.clone());
    }
    update_run_status(db.inner(), &request.parallel_run_id, "running").await?;
    if let Ok(Some(updated)) = load_parallel_run(db.inner(), &request.parallel_run_id).await {
        emit_parallel_run_updated(&app_handle, &updated);
    }

    let mut base_config = empty_agent_config();
    base_config.conversation_id = Some(run.parent_conversation_id.clone());
    let child_config =
        clone_config_for_child(&base_config, &item.model_run_id, &item.message_id, &model);
    if let Err(err) = agent_execute(
        run.task.clone(),
        Some(child_config),
        app_handle.clone(),
        ai_manager.inner().clone(),
    )
    .await
    {
        update_item_finished(
            db.inner(),
            &item.model_run_id,
            "failed",
            "",
            Some(&err),
            0,
            &ParallelSessionStats::default(),
        )
        .await?;
        if let Ok(Some(updated)) = load_parallel_run(db.inner(), &request.parallel_run_id).await {
            emit_parallel_run_updated(&app_handle, &updated);
        }
    }
    Ok(item)
}

#[tauri::command]
pub async fn cancel_ai_parallel_run(
    request: CancelAiParallelRunRequest,
    app_handle: AppHandle,
) -> Result<(), String> {
    let child_ids = PARALLEL_RUN_CHILDREN
        .lock()
        .ok()
        .and_then(|mut guard| guard.remove(&request.parallel_run_id))
        .unwrap_or_default();
    for child_id in child_ids {
        cancel_conversation_stream(&child_id);
        let _ = crate::managers::cancellation_manager::cancel_execution(&child_id).await;
        let _ = sentinel_tools::buildin_tools::shell::cancel_shell_execution(&child_id).await;
        if let Err(err) =
            sentinel_tools::buildin_tools::shell_background::stop_background_shell_tasks_for_execution(
                &child_id,
            )
            .await
        {
            tracing::warn!("Failed to stop background shell tasks for {}: {}", child_id, err);
        }
    }
    if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
        let _ = update_run_status(db.inner(), &request.parallel_run_id, "cancelled").await;
        if let Ok(Some(run)) = load_parallel_run(db.inner(), &request.parallel_run_id).await {
            emit_parallel_run_updated(&app_handle, &run);
        }
    }
    if let Some(parent_execution_id) = request.parent_execution_id.as_deref() {
        emit_agent_execution_finished(
            &app_handle,
            parent_execution_id,
            AgentExecutionOutcome::Cancelled,
            None,
            None,
            Some("Parallel execution cancelled by user".to_string()),
        );
    }
    Ok(())
}

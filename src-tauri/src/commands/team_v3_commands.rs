use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use crate::commands::team_v3_artifact_store::{
    persist_team_v3_task_output_artifact, TeamV3ArtifactFileRef,
};
use crate::agents::executor::{execute_agent as execute_team_agent, AgentExecuteParams};
use crate::agents::tool_router::{ToolConfig, ToolSelectionStrategy};
use crate::agents::ContextPolicy;
use crate::services::ai::AiServiceManager;
use anyhow::{anyhow, Result};
use chrono::Utc;
use sentinel_db::{database_service::connection_manager::DatabasePool, Database, DatabaseService};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use tauri::{AppHandle, Manager, State};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

type DbState<'r> = State<'r, Arc<DatabaseService>>;
type AiState<'r> = State<'r, Arc<AiServiceManager>>;

static TEAM_EXECUTION_CANCELLATIONS: LazyLock<Mutex<HashMap<String, (u64, CancellationToken)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static TEAM_EXECUTION_GENERATION: AtomicU64 = AtomicU64::new(0);
static TEAM_V3_BLACKBOARD_SESSION_LOCKS: LazyLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const TEAM_V3_BLACKBOARD_INLINE_CHAR_LIMIT: usize = 2_000;
const TEAM_V3_STRUCTURED_BACKFILL_SCAN_LIMIT: i64 = 2_000;

fn create_team_execution_cancellation(session_id: &str) -> (u64, CancellationToken) {
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

fn cancel_team_execution(session_id: &str) {
    if let Ok(guard) = TEAM_EXECUTION_CANCELLATIONS.lock() {
        if let Some((_, token)) = guard.get(session_id) {
            token.cancel();
        }
    }
}

fn is_team_execution_cancelled(session_id: &str, generation: u64) -> bool {
    if let Ok(guard) = TEAM_EXECUTION_CANCELLATIONS.lock() {
        if let Some((current_generation, token)) = guard.get(session_id) {
            return *current_generation != generation || token.is_cancelled();
        }
    }
    true
}

fn clear_team_execution_cancellation(session_id: &str, generation: u64) {
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

#[derive(Debug, Clone, Deserialize)]
struct TeamV3PlannedAgent {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    responsibility: Option<String>,
    #[serde(default)]
    system_prompt: Option<String>,
    #[serde(default)]
    decision_style: Option<String>,
    #[serde(default)]
    risk_preference: Option<String>,
    #[serde(default)]
    weight: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct TeamV3PlannedTask {
    task_key: String,
    title: String,
    instruction: String,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    owner_agent_id: Option<String>,
    #[serde(default)]
    priority: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
struct TeamV3ExecutionPlan {
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    agents: Vec<TeamV3PlannedAgent>,
    tasks: Vec<TeamV3PlannedTask>,
}

async fn reset_schema_sqlite(pool: &sqlx::SqlitePool) -> Result<()> {
    let ddl = [
        "DROP TABLE IF EXISTS team_v3_task_events",
        "DROP TABLE IF EXISTS team_v3_blackboard_entries",
        "DROP TABLE IF EXISTS team_v3_messages",
        "DROP TABLE IF EXISTS team_v3_plan_revisions",
        "DROP TABLE IF EXISTS team_v3_task_claims",
        "DROP TABLE IF EXISTS team_v3_tasks",
        "DROP TABLE IF EXISTS team_v3_sessions",
        "DROP TABLE IF EXISTS team_v3_templates",
        "DROP TABLE IF EXISTS agent_team_task_events",
        "DROP TABLE IF EXISTS agent_team_mailbox",
        "DROP TABLE IF EXISTS agent_team_task_attempts",
        "DROP TABLE IF EXISTS agent_team_tasks",
        "DROP TABLE IF EXISTS agent_team_artifacts",
        "DROP TABLE IF EXISTS agent_team_decisions",
        "DROP TABLE IF EXISTS agent_team_messages",
        "DROP TABLE IF EXISTS agent_team_rounds",
        "DROP TABLE IF EXISTS agent_team_blackboard_entries",
        "DROP TABLE IF EXISTS agent_team_members",
        "DROP TABLE IF EXISTS agent_team_sessions",
        "DROP TABLE IF EXISTS agent_team_template_members",
        "DROP TABLE IF EXISTS agent_team_templates",
        r#"CREATE TABLE IF NOT EXISTS team_v3_templates (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            domain TEXT NOT NULL DEFAULT 'custom',
            spec_json TEXT NOT NULL DEFAULT '{}',
            is_system BOOLEAN NOT NULL DEFAULT FALSE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_sessions (
            id TEXT PRIMARY KEY,
            conversation_id TEXT,
            name TEXT NOT NULL,
            goal TEXT,
            state TEXT NOT NULL CHECK (state IN ('PLAN_DRAFT','WAITING_PLAN_APPROVAL','EXECUTING','COMPLETED','FAILED','ARCHIVED')),
            state_data TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_tasks (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_key TEXT NOT NULL,
            title TEXT NOT NULL,
            instruction TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('pending','ready_for_claim','claimed','running','waiting_review','waiting_handoff_ack','completed','failed','blocked','cancelled')),
            priority INTEGER NOT NULL DEFAULT 100,
            owner_agent_id TEXT,
            claimed_by_agent_id TEXT,
            claim_expires_at DATETIME,
            lock_version INTEGER NOT NULL DEFAULT 0,
            acceptance_criteria TEXT,
            task_kind TEXT NOT NULL DEFAULT 'execution',
            is_handoff_required BOOLEAN NOT NULL DEFAULT FALSE,
            parent_task_id TEXT,
            plan_data TEXT,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(session_id, task_key)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_task_claims (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_id TEXT NOT NULL REFERENCES team_v3_tasks(id) ON DELETE CASCADE,
            agent_id TEXT NOT NULL,
            action TEXT NOT NULL CHECK (action IN ('claim','renew','release','expire')),
            ttl_secs INTEGER,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_plan_revisions (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            revision_no INTEGER NOT NULL,
            plan_json TEXT NOT NULL DEFAULT '{}',
            summary TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('draft','waiting_approval','approved','rejected')),
            requested_by TEXT,
            reviewed_by TEXT,
            review_note TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            reviewed_at DATETIME
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            thread_id TEXT NOT NULL,
            in_reply_to TEXT,
            from_agent_id TEXT,
            to_agent_id TEXT,
            message_type TEXT NOT NULL DEFAULT 'chat',
            message_kind TEXT NOT NULL DEFAULT 'chat',
            payload TEXT NOT NULL DEFAULT '{}',
            requires_response BOOLEAN NOT NULL DEFAULT FALSE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_blackboard_entries (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_id TEXT REFERENCES team_v3_tasks(id) ON DELETE SET NULL,
            agent_id TEXT,
            entry_type TEXT NOT NULL DEFAULT 'note',
            content TEXT NOT NULL,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_task_events (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_id TEXT REFERENCES team_v3_tasks(id) ON DELETE SET NULL,
            event_type TEXT NOT NULL,
            payload TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_team_v3_tasks_session_status_priority ON team_v3_tasks(session_id, status, priority, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_tasks_claim_expiry ON team_v3_tasks(claimed_by_agent_id, claim_expires_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_messages_session_thread_created ON team_v3_messages(session_id, thread_id, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_blackboard_session_created ON team_v3_blackboard_entries(session_id, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_plan_revisions_session_rev ON team_v3_plan_revisions(session_id, revision_no)",
    ];

    for sql in ddl {
        sqlx::query(sql).execute(pool).await?;
    }
    Ok(())
}

async fn reset_schema_pg(pool: &sentinel_db::sqlx_compat::PgPool) -> Result<()> {
    let ddl = [
        "DROP TABLE IF EXISTS team_v3_task_events CASCADE",
        "DROP TABLE IF EXISTS team_v3_blackboard_entries CASCADE",
        "DROP TABLE IF EXISTS team_v3_messages CASCADE",
        "DROP TABLE IF EXISTS team_v3_plan_revisions CASCADE",
        "DROP TABLE IF EXISTS team_v3_task_claims CASCADE",
        "DROP TABLE IF EXISTS team_v3_tasks CASCADE",
        "DROP TABLE IF EXISTS team_v3_sessions CASCADE",
        "DROP TABLE IF EXISTS team_v3_templates CASCADE",
        "DROP TABLE IF EXISTS agent_team_task_events CASCADE",
        "DROP TABLE IF EXISTS agent_team_mailbox CASCADE",
        "DROP TABLE IF EXISTS agent_team_task_attempts CASCADE",
        "DROP TABLE IF EXISTS agent_team_tasks CASCADE",
        "DROP TABLE IF EXISTS agent_team_artifacts CASCADE",
        "DROP TABLE IF EXISTS agent_team_decisions CASCADE",
        "DROP TABLE IF EXISTS agent_team_messages CASCADE",
        "DROP TABLE IF EXISTS agent_team_rounds CASCADE",
        "DROP TABLE IF EXISTS agent_team_blackboard_entries CASCADE",
        "DROP TABLE IF EXISTS agent_team_members CASCADE",
        "DROP TABLE IF EXISTS agent_team_sessions CASCADE",
        "DROP TABLE IF EXISTS agent_team_template_members CASCADE",
        "DROP TABLE IF EXISTS agent_team_templates CASCADE",
        r#"CREATE TABLE IF NOT EXISTS team_v3_templates (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            domain TEXT NOT NULL DEFAULT 'custom',
            spec_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            is_system BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_sessions (
            id TEXT PRIMARY KEY,
            conversation_id TEXT,
            name TEXT NOT NULL,
            goal TEXT,
            state TEXT NOT NULL CHECK (state IN ('PLAN_DRAFT','WAITING_PLAN_APPROVAL','EXECUTING','COMPLETED','FAILED','ARCHIVED')),
            state_data JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_tasks (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_key TEXT NOT NULL,
            title TEXT NOT NULL,
            instruction TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('pending','ready_for_claim','claimed','running','waiting_review','waiting_handoff_ack','completed','failed','blocked','cancelled')),
            priority INTEGER NOT NULL DEFAULT 100,
            owner_agent_id TEXT,
            claimed_by_agent_id TEXT,
            claim_expires_at TIMESTAMPTZ,
            lock_version INTEGER NOT NULL DEFAULT 0,
            acceptance_criteria TEXT,
            task_kind TEXT NOT NULL DEFAULT 'execution',
            is_handoff_required BOOLEAN NOT NULL DEFAULT FALSE,
            parent_task_id TEXT,
            plan_data JSONB,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(session_id, task_key)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_task_claims (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_id TEXT NOT NULL REFERENCES team_v3_tasks(id) ON DELETE CASCADE,
            agent_id TEXT NOT NULL,
            action TEXT NOT NULL CHECK (action IN ('claim','renew','release','expire')),
            ttl_secs BIGINT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_plan_revisions (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            revision_no INTEGER NOT NULL,
            plan_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            summary TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('draft','waiting_approval','approved','rejected')),
            requested_by TEXT,
            reviewed_by TEXT,
            review_note TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            reviewed_at TIMESTAMPTZ
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            thread_id TEXT NOT NULL,
            in_reply_to TEXT,
            from_agent_id TEXT,
            to_agent_id TEXT,
            message_type TEXT NOT NULL DEFAULT 'chat',
            message_kind TEXT NOT NULL DEFAULT 'chat',
            payload JSONB NOT NULL DEFAULT '{}'::jsonb,
            requires_response BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_blackboard_entries (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_id TEXT REFERENCES team_v3_tasks(id) ON DELETE SET NULL,
            agent_id TEXT,
            entry_type TEXT NOT NULL DEFAULT 'note',
            content TEXT NOT NULL,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_task_events (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_id TEXT REFERENCES team_v3_tasks(id) ON DELETE SET NULL,
            event_type TEXT NOT NULL,
            payload JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_team_v3_tasks_session_status_priority ON team_v3_tasks(session_id, status, priority, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_tasks_claim_expiry ON team_v3_tasks(claimed_by_agent_id, claim_expires_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_messages_session_thread_created ON team_v3_messages(session_id, thread_id, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_blackboard_session_created ON team_v3_blackboard_entries(session_id, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_plan_revisions_session_rev ON team_v3_plan_revisions(session_id, revision_no)",
    ];

    for sql in ddl {
        sqlx::query(sql).execute(pool).await?;
    }
    Ok(())
}

async fn ensure_schema_sqlite(pool: &sqlx::SqlitePool) -> Result<()> {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS team_v3_templates (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            domain TEXT NOT NULL DEFAULT 'custom',
            spec_json TEXT NOT NULL DEFAULT '{}',
            is_system BOOLEAN NOT NULL DEFAULT FALSE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_sessions (
            id TEXT PRIMARY KEY,
            conversation_id TEXT,
            name TEXT NOT NULL,
            goal TEXT,
            state TEXT NOT NULL CHECK (state IN ('PLAN_DRAFT','WAITING_PLAN_APPROVAL','EXECUTING','COMPLETED','FAILED','ARCHIVED')),
            state_data TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_tasks (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_key TEXT NOT NULL,
            title TEXT NOT NULL,
            instruction TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('pending','ready_for_claim','claimed','running','waiting_review','waiting_handoff_ack','completed','failed','blocked','cancelled')),
            priority INTEGER NOT NULL DEFAULT 100,
            owner_agent_id TEXT,
            claimed_by_agent_id TEXT,
            claim_expires_at DATETIME,
            lock_version INTEGER NOT NULL DEFAULT 0,
            acceptance_criteria TEXT,
            task_kind TEXT NOT NULL DEFAULT 'execution',
            is_handoff_required BOOLEAN NOT NULL DEFAULT FALSE,
            parent_task_id TEXT,
            plan_data TEXT,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(session_id, task_key)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_task_claims (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_id TEXT NOT NULL REFERENCES team_v3_tasks(id) ON DELETE CASCADE,
            agent_id TEXT NOT NULL,
            action TEXT NOT NULL CHECK (action IN ('claim','renew','release','expire')),
            ttl_secs INTEGER,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_plan_revisions (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            revision_no INTEGER NOT NULL,
            plan_json TEXT NOT NULL DEFAULT '{}',
            summary TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('draft','waiting_approval','approved','rejected')),
            requested_by TEXT,
            reviewed_by TEXT,
            review_note TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            reviewed_at DATETIME
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            thread_id TEXT NOT NULL,
            in_reply_to TEXT,
            from_agent_id TEXT,
            to_agent_id TEXT,
            message_type TEXT NOT NULL DEFAULT 'chat',
            message_kind TEXT NOT NULL DEFAULT 'chat',
            payload TEXT NOT NULL DEFAULT '{}',
            requires_response BOOLEAN NOT NULL DEFAULT FALSE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_blackboard_entries (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_id TEXT REFERENCES team_v3_tasks(id) ON DELETE SET NULL,
            agent_id TEXT,
            entry_type TEXT NOT NULL DEFAULT 'note',
            content TEXT NOT NULL,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_task_events (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_id TEXT REFERENCES team_v3_tasks(id) ON DELETE SET NULL,
            event_type TEXT NOT NULL,
            payload TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_team_v3_tasks_session_status_priority ON team_v3_tasks(session_id, status, priority, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_tasks_claim_expiry ON team_v3_tasks(claimed_by_agent_id, claim_expires_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_messages_session_thread_created ON team_v3_messages(session_id, thread_id, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_blackboard_session_created ON team_v3_blackboard_entries(session_id, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_plan_revisions_session_rev ON team_v3_plan_revisions(session_id, revision_no)",
    ];

    for sql in ddl {
        sqlx::query(sql).execute(pool).await?;
    }
    Ok(())
}

async fn ensure_schema_pg(pool: &sentinel_db::sqlx_compat::PgPool) -> Result<()> {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS team_v3_templates (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            domain TEXT NOT NULL DEFAULT 'custom',
            spec_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            is_system BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_sessions (
            id TEXT PRIMARY KEY,
            conversation_id TEXT,
            name TEXT NOT NULL,
            goal TEXT,
            state TEXT NOT NULL CHECK (state IN ('PLAN_DRAFT','WAITING_PLAN_APPROVAL','EXECUTING','COMPLETED','FAILED','ARCHIVED')),
            state_data JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_tasks (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_key TEXT NOT NULL,
            title TEXT NOT NULL,
            instruction TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('pending','ready_for_claim','claimed','running','waiting_review','waiting_handoff_ack','completed','failed','blocked','cancelled')),
            priority INTEGER NOT NULL DEFAULT 100,
            owner_agent_id TEXT,
            claimed_by_agent_id TEXT,
            claim_expires_at TIMESTAMPTZ,
            lock_version INTEGER NOT NULL DEFAULT 0,
            acceptance_criteria TEXT,
            task_kind TEXT NOT NULL DEFAULT 'execution',
            is_handoff_required BOOLEAN NOT NULL DEFAULT FALSE,
            parent_task_id TEXT,
            plan_data JSONB,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(session_id, task_key)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_task_claims (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_id TEXT NOT NULL REFERENCES team_v3_tasks(id) ON DELETE CASCADE,
            agent_id TEXT NOT NULL,
            action TEXT NOT NULL CHECK (action IN ('claim','renew','release','expire')),
            ttl_secs BIGINT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_plan_revisions (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            revision_no INTEGER NOT NULL,
            plan_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            summary TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('draft','waiting_approval','approved','rejected')),
            requested_by TEXT,
            reviewed_by TEXT,
            review_note TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            reviewed_at TIMESTAMPTZ
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            thread_id TEXT NOT NULL,
            in_reply_to TEXT,
            from_agent_id TEXT,
            to_agent_id TEXT,
            message_type TEXT NOT NULL DEFAULT 'chat',
            message_kind TEXT NOT NULL DEFAULT 'chat',
            payload JSONB NOT NULL DEFAULT '{}'::jsonb,
            requires_response BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_blackboard_entries (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_id TEXT REFERENCES team_v3_tasks(id) ON DELETE SET NULL,
            agent_id TEXT,
            entry_type TEXT NOT NULL DEFAULT 'note',
            content TEXT NOT NULL,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v3_task_events (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES team_v3_sessions(id) ON DELETE CASCADE,
            task_id TEXT REFERENCES team_v3_tasks(id) ON DELETE SET NULL,
            event_type TEXT NOT NULL,
            payload JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_team_v3_tasks_session_status_priority ON team_v3_tasks(session_id, status, priority, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_tasks_claim_expiry ON team_v3_tasks(claimed_by_agent_id, claim_expires_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_messages_session_thread_created ON team_v3_messages(session_id, thread_id, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_blackboard_session_created ON team_v3_blackboard_entries(session_id, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v3_plan_revisions_session_rev ON team_v3_plan_revisions(session_id, revision_no)",
    ];

    for sql in ddl {
        sqlx::query(sql).execute(pool).await?;
    }
    Ok(())
}

async fn ensure_team_v3_schema(runtime_pool: &DatabasePool) -> Result<()> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => ensure_schema_sqlite(pool).await,
        DatabasePool::PostgreSQL(pool) => ensure_schema_pg(pool).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Team V3 does not support MySQL")),
    }
}

fn parse_state_data_text(raw: &str) -> Value {
    serde_json::from_str::<Value>(raw).unwrap_or_else(|_| json!({}))
}

fn default_team_members() -> Vec<Value> {
    vec![json!({
        "id": "agent-1",
        "name": "Team Agent",
        "responsibility": "负责执行分配任务并输出可复用结论",
        "sort_order": 0,
        "weight": 1.0,
        "token_usage": 0,
        "tool_calls_count": 0,
        "is_active": false
    })]
}

fn normalize_team_member_values(mut members: Vec<Value>) -> Vec<Value> {
    if members.is_empty() {
        members = default_team_members();
    }

    members
        .into_iter()
        .enumerate()
        .map(|(index, mut member)| {
            if !member.is_object() {
                member = json!({});
            }
            let member_obj = member.as_object_mut().expect("member object");
            let default_id = format!("member-{}", index + 1);
            let id = member_obj
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or(default_id);
            let default_name = format!("Agent {}", index + 1);
            let name = member_obj
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or(default_name);

            member_obj.insert("id".to_string(), json!(id));
            member_obj.insert("name".to_string(), json!(name));
            member_obj.insert("sort_order".to_string(), json!(index as i64));
            member_obj
                .entry("weight".to_string())
                .or_insert_with(|| json!(1.0));
            member_obj
                .entry("token_usage".to_string())
                .or_insert_with(|| json!(0));
            member_obj
                .entry("tool_calls_count".to_string())
                .or_insert_with(|| json!(0));
            member_obj
                .entry("is_active".to_string())
                .or_insert_with(|| json!(false));
            member
        })
        .collect()
}

fn normalize_team_members(state_data: &Value) -> Vec<Value> {
    let members = state_data
        .get("members")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    normalize_team_member_values(members)
}

fn normalize_agent_id(input: &str, fallback_index: usize) -> String {
    let mut normalized = input
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() {
                ch
            } else if ch == '-' || ch == '_' || ch == ' ' {
                '-'
            } else {
                '-'
            }
        })
        .collect::<String>();
    while normalized.contains("--") {
        normalized = normalized.replace("--", "-");
    }
    normalized = normalized.trim_matches('-').to_string();
    if normalized.is_empty() {
        format!("agent-{}", fallback_index + 1)
    } else {
        normalized
    }
}

fn derive_plan_members(plan: &TeamV3ExecutionPlan) -> Vec<Value> {
    if !plan.agents.is_empty() {
        let members = plan
            .agents
            .iter()
            .enumerate()
            .map(|(index, agent)| {
                let id = normalize_agent_id(agent.id.as_str(), index);
                let name = if agent.name.trim().is_empty() {
                    format!("Agent {}", index + 1)
                } else {
                    agent.name.trim().to_string()
                };
                let mut member = json!({
                    "id": id,
                    "name": name,
                    "responsibility": agent
                        .responsibility
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .unwrap_or("负责执行分配任务并输出可复用结论"),
                    "sort_order": index as i64,
                    "weight": agent.weight.unwrap_or(1.0),
                    "token_usage": 0,
                    "tool_calls_count": 0,
                    "is_active": false
                });
                if let Some(obj) = member.as_object_mut() {
                    if let Some(system_prompt) = agent
                        .system_prompt
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                    {
                        obj.insert("system_prompt".to_string(), json!(system_prompt));
                    }
                    if let Some(decision_style) = agent
                        .decision_style
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                    {
                        obj.insert("decision_style".to_string(), json!(decision_style));
                    }
                    if let Some(risk_preference) = agent
                        .risk_preference
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                    {
                        obj.insert("risk_preference".to_string(), json!(risk_preference));
                    }
                }
                member
            })
            .collect::<Vec<_>>();
        return normalize_team_member_values(members);
    }

    let mut owners = plan
        .tasks
        .iter()
        .filter_map(|task| {
            task.owner_agent_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
        .collect::<Vec<_>>();
    owners.sort();
    owners.dedup();
    if owners.is_empty() {
        return default_team_members();
    }
    let members = owners
        .into_iter()
        .enumerate()
        .map(|(index, owner)| {
            let id = normalize_agent_id(owner.as_str(), index);
            json!({
                "id": id,
                "name": format!("Agent {}", index + 1),
                "responsibility": "负责执行分配任务并输出可复用结论",
                "sort_order": index as i64,
                "weight": 1.0,
                "token_usage": 0,
                "tool_calls_count": 0,
                "is_active": false
            })
        })
        .collect::<Vec<_>>();
    normalize_team_member_values(members)
}

fn build_team_state_data_with_members(
    current: Option<&Value>,
    members: Vec<Value>,
    active_member_id: Option<&str>,
) -> Value {
    let mut state_data = current.cloned().unwrap_or_else(|| json!({}));
    if !state_data.is_object() {
        state_data = json!({});
    }
    let mut normalized_members = normalize_team_member_values(members);
    for member in normalized_members.iter_mut() {
        if let Some(member_obj) = member.as_object_mut() {
            let member_id = member_obj
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let is_active = active_member_id.map(|id| id == member_id).unwrap_or(false);
            member_obj.insert("is_active".to_string(), json!(is_active));
        }
    }

    if let Some(state_obj) = state_data.as_object_mut() {
        state_obj.insert("members".to_string(), Value::Array(normalized_members));
        let runtime = state_obj.entry("runtime").or_insert_with(|| json!({}));
        if let Some(runtime_obj) = runtime.as_object_mut() {
            runtime_obj.insert(
                "last_updated_at".to_string(),
                json!(Utc::now().to_rfc3339()),
            );
            if let Some(active_id) = active_member_id {
                runtime_obj.insert("active_member_id".to_string(), json!(active_id));
            } else {
                runtime_obj.remove("active_member_id");
            }
        }
    }
    state_data
}

fn build_team_state_data(current: Option<&Value>, active_member_id: Option<&str>) -> Value {
    let current_state = current.cloned().unwrap_or_else(|| json!({}));
    let members = normalize_team_members(&current_state);
    build_team_state_data_with_members(Some(&current_state), members, active_member_id)
}

fn first_member_id(state_data: &Value) -> String {
    state_data
        .get("members")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|m| m.get("id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "agent-1".to_string())
}

fn team_member_ids(state_data: &Value) -> Vec<String> {
    let mut ids = normalize_team_members(state_data)
        .into_iter()
        .filter_map(|member| {
            member
                .get("id")
                .and_then(|v| v.as_str())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        })
        .collect::<Vec<_>>();
    if ids.is_empty() {
        ids.push("agent-1".to_string());
    }
    ids
}

fn team_member_catalog_lines(state_data: &Value) -> Vec<String> {
    state_data
        .get("members")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|member| {
            let id = member
                .get("id")
                .and_then(|v| v.as_str())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())?;
            let name = member
                .get("name")
                .and_then(|v| v.as_str())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| id.clone());
            let responsibility = member
                .get("responsibility")
                .and_then(|v| v.as_str())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "负责通用问题求解".to_string());
            Some(format!("- {} ({})：{}", id, name, responsibility))
        })
        .collect()
}

#[derive(Debug, Clone)]
struct TeamV3MemberProfile {
    name: String,
    responsibility: Option<String>,
    system_prompt: Option<String>,
    decision_style: Option<String>,
    risk_preference: Option<String>,
}

fn team_member_profiles(state_data: &Value) -> HashMap<String, TeamV3MemberProfile> {
    normalize_team_members(state_data)
        .into_iter()
        .filter_map(|member| {
            let id = member
                .get("id")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())?;
            let name = member
                .get("name")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| id.clone());
            let responsibility = member
                .get("responsibility")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let system_prompt = member
                .get("system_prompt")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let decision_style = member
                .get("decision_style")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let risk_preference = member
                .get("risk_preference")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            Some((
                id,
                TeamV3MemberProfile {
                    name,
                    responsibility,
                    system_prompt,
                    decision_style,
                    risk_preference,
                },
            ))
        })
        .collect()
}

fn canonical_agent_id(input: &str) -> String {
    input
        .trim()
        .to_lowercase()
        .replace('_', "-")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

fn resolve_member_id(owner_candidate: Option<&str>, members: &[String], index: usize) -> String {
    let fallback = members
        .get(index % members.len().max(1))
        .cloned()
        .unwrap_or_else(|| "agent-1".to_string());

    let Some(owner_raw) = owner_candidate
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return fallback;
    };

    if members.iter().any(|member| member == owner_raw) {
        return owner_raw.to_string();
    }

    let canonical_owner = canonical_agent_id(owner_raw);
    if !canonical_owner.is_empty() {
        if let Some(matched) = members
            .iter()
            .find(|member| canonical_agent_id(member.as_str()) == canonical_owner)
        {
            return matched.clone();
        }
    }

    let normalized_owner = normalize_agent_id(owner_raw, index);
    if let Some(matched) = members
        .iter()
        .find(|member| member.as_str() == normalized_owner.as_str())
    {
        return matched.clone();
    }

    fallback
}

fn build_team_member_execution_system_prompt(
    _member_id: &str,
    _profile: Option<&TeamV3MemberProfile>,
) -> String {
    let lines = vec![
        "你是 Team 成员，请以严谨、可执行的方式完成分配任务。".to_string(),
        "输出必须可追溯、可验证，避免空泛表述。".to_string(),
        "优先依据当前用户任务与共享上下文，不要编造未提供事实。\n".to_string(),
    ];
    lines.join("\n")
}

fn build_team_member_runtime_context(
    member_id: &str,
    profile: Option<&TeamV3MemberProfile>,
) -> String {
    let mut lines = vec![format!("成员 ID：{}", member_id)];
    if let Some(profile) = profile {
        if !profile.name.trim().is_empty() {
            lines.push(format!("成员名称：{}", profile.name.trim()));
        }
        if let Some(responsibility) = profile
            .responsibility
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("职责边界：{}", responsibility));
        }
        if let Some(decision_style) = profile
            .decision_style
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("决策风格：{}", decision_style));
        }
        if let Some(risk_preference) = profile
            .risk_preference
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("风险偏好：{}", risk_preference));
        }
        if let Some(system_prompt) = profile
            .system_prompt
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("额外执行约束：{}", system_prompt));
        }
    }
    lines.join("\n")
}

fn parse_task_dependencies(metadata: &Value) -> Vec<String> {
    metadata
        .get("depends_on")
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(|v| v.trim().to_string()))
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn extract_json_candidate(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
    }

    if let Some(fence_start) = trimmed.find("```json").or_else(|| trimmed.find("```")) {
        let after_start = &trimmed[fence_start..];
        if let Some(first_line_end) = after_start.find('\n') {
            let body = &after_start[first_line_end + 1..];
            if let Some(fence_end) = body.find("```") {
                let fenced = body[..fence_end].trim();
                if fenced.starts_with('{') && fenced.ends_with('}') {
                    return Some(fenced.to_string());
                }
                if let Some(candidate) = extract_first_json_object(fenced) {
                    return Some(candidate);
                }
            }
        }
    }
    extract_first_json_object(trimmed)
}

fn extract_first_json_object(raw: &str) -> Option<String> {
    let mut start_index: Option<usize> = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in raw.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start_index = Some(index);
                }
                depth += 1;
            }
            '}' => {
                if depth == 0 {
                    continue;
                }
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = start_index {
                        return Some(raw[start..=index].trim().to_string());
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn normalize_task_key(input: &str, fallback_index: usize) -> String {
    let mut normalized = input
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch
            } else if ch == '-' || ch == '_' || ch == ' ' {
                '-'
            } else {
                '-'
            }
        })
        .collect::<String>();
    while normalized.contains("--") {
        normalized = normalized.replace("--", "-");
    }
    normalized = normalized.trim_matches('-').to_string();
    if normalized.is_empty() {
        format!("task-{}", fallback_index + 1)
    } else {
        normalized
    }
}

fn parse_execution_plan(raw: &str) -> Option<TeamV3ExecutionPlan> {
    let candidate = extract_json_candidate(raw)?;
    serde_json::from_str::<TeamV3ExecutionPlan>(&candidate).ok()
}

async fn set_team_v3_session_state(
    runtime_pool: &DatabasePool,
    session_id: &str,
    state: &str,
    now: &str,
) -> Result<()> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET state = ?, updated_at = ?
                   WHERE id = ?"#,
            )
            .bind(state)
            .bind(now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET state = $1, updated_at = $2
                   WHERE id = $3"#,
            )
            .bind(state)
            .bind(now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }
    Ok(())
}

async fn get_team_v3_session_state_data(
    runtime_pool: &DatabasePool,
    session_id: &str,
) -> Result<Value> {
    let raw_state_data: Option<String> = match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT state_data
                   FROM team_v3_sessions
                   WHERE id = ?"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            row.map(|r| r.get("state_data"))
        }
        DatabasePool::PostgreSQL(pool) => {
            let row = sqlx::query(
                r#"SELECT state_data::text as state_data
                   FROM team_v3_sessions
                   WHERE id = $1"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            row.map(|r| r.get("state_data"))
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    };

    Ok(raw_state_data
        .as_deref()
        .map(parse_state_data_text)
        .unwrap_or_else(|| json!({})))
}

async fn set_team_v3_session_state_data(
    runtime_pool: &DatabasePool,
    session_id: &str,
    state_data: &Value,
    now: &str,
) -> Result<()> {
    let state_data_text = serde_json::to_string(state_data)?;
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET state_data = ?, updated_at = ?
                   WHERE id = ?"#,
            )
            .bind(&state_data_text)
            .bind(now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET state_data = $1::jsonb, updated_at = $2
                   WHERE id = $3"#,
            )
            .bind(&state_data_text)
            .bind(now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }
    Ok(())
}

async fn ensure_team_v3_execution_tasks(
    runtime_pool: &DatabasePool,
    session_id: &str,
    goal: Option<&str>,
    state_data: &Value,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let members = team_member_ids(state_data);
    let goal_text = goal
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .unwrap_or("当前 Team 任务");

    let existing_count: i64 = match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT COUNT(1) as count
                   FROM team_v3_tasks
                   WHERE session_id = ?"#,
            )
            .bind(session_id)
            .fetch_one(pool)
            .await?;
            row.get("count")
        }
        DatabasePool::PostgreSQL(pool) => {
            let row = sqlx::query(
                r#"SELECT COUNT(1) as count
                   FROM team_v3_tasks
                   WHERE session_id = $1"#,
            )
            .bind(session_id)
            .fetch_one(pool)
            .await?;
            row.get("count")
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    };

    if existing_count == 0 {
        let tasks = vec![
            (
                "collect-context",
                "收集上下文与事实",
                format!("围绕目标「{}」收集关键上下文、约束与事实依据。", goal_text),
                10,
                members
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "agent-1".to_string()),
                json!({
                    "team_generated": true,
                    "depends_on": []
                }),
            ),
            (
                "analyze-options",
                "并行分析方案与风险",
                format!(
                    "围绕目标「{}」从产品、架构与风险角度并行提出候选方案。",
                    goal_text
                ),
                20,
                members
                    .get(1)
                    .cloned()
                    .or_else(|| members.first().cloned())
                    .unwrap_or_else(|| "agent-2".to_string()),
                json!({
                    "team_generated": true,
                    "depends_on": []
                }),
            ),
            (
                "deliver-summary",
                "汇总结论与行动建议",
                "整合前置任务结果，输出最终结论、风险清单和可执行下一步。".to_string(),
                30,
                members
                    .get(2)
                    .cloned()
                    .or_else(|| members.first().cloned())
                    .unwrap_or_else(|| "agent-3".to_string()),
                json!({
                    "team_generated": true,
                    "depends_on": ["collect-context", "analyze-options"]
                }),
            ),
        ];

        match runtime_pool {
            DatabasePool::SQLite(pool) => {
                for (task_key, title, instruction, priority, owner_agent_id, metadata) in tasks {
                    sqlx::query(
                        r#"INSERT INTO team_v3_tasks
                           (id, session_id, task_key, title, instruction, status, priority,
                            owner_agent_id, claimed_by_agent_id, claim_expires_at, metadata, created_at, updated_at)
                           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(session_id)
                    .bind(task_key)
                    .bind(title)
                    .bind(instruction)
                    .bind("pending")
                    .bind(priority)
                    .bind(owner_agent_id)
                    .bind(Option::<String>::None)
                    .bind(Option::<String>::None)
                    .bind(serde_json::to_string(&metadata)?)
                    .bind(&now)
                    .bind(&now)
                    .execute(pool)
                    .await?;
                }
            }
            DatabasePool::PostgreSQL(pool) => {
                for (task_key, title, instruction, priority, owner_agent_id, metadata) in tasks {
                    sqlx::query(
                        r#"INSERT INTO team_v3_tasks
                           (id, session_id, task_key, title, instruction, status, priority,
                            owner_agent_id, claimed_by_agent_id, claim_expires_at, metadata, created_at, updated_at)
                           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::jsonb, $12, $13)"#,
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(session_id)
                    .bind(task_key)
                    .bind(title)
                    .bind(instruction)
                    .bind("pending")
                    .bind(priority)
                    .bind(owner_agent_id)
                    .bind(Option::<String>::None)
                    .bind(Option::<String>::None)
                    .bind(serde_json::to_string(&metadata)?)
                    .bind(&now)
                    .bind(&now)
                    .execute(pool)
                    .await?;
                }
            }
            DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
        }
        return Ok(());
    }

    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_tasks
                   SET status = CASE
                       WHEN status = 'cancelled' THEN status
                       ELSE 'pending'
                   END,
                   claimed_by_agent_id = NULL,
                   claim_expires_at = NULL,
                   updated_at = ?
                   WHERE session_id = ?"#,
            )
            .bind(&now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_tasks
                   SET status = CASE
                       WHEN status = 'cancelled' THEN status
                       ELSE 'pending'
                   END,
                   claimed_by_agent_id = NULL,
                   claim_expires_at = NULL,
                   updated_at = $1
                   WHERE session_id = $2"#,
            )
            .bind(&now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }

    Ok(())
}

async fn apply_team_v3_execution_outcome(
    runtime_pool: &DatabasePool,
    session_id: &str,
    success: bool,
    _summary: Option<&str>,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            if success {
                sqlx::query(
                    r#"UPDATE team_v3_tasks
                       SET status = CASE
                           WHEN status = 'cancelled' THEN status
                           ELSE 'completed'
                       END,
                       claimed_by_agent_id = NULL,
                       claim_expires_at = NULL,
                       updated_at = ?
                       WHERE session_id = ?"#,
                )
                .bind(&now)
                .bind(session_id)
                .execute(pool)
                .await?;
            } else {
                sqlx::query(
                    r#"UPDATE team_v3_tasks
                       SET status = CASE
                           WHEN status IN ('running','claimed','waiting_review','waiting_handoff_ack') THEN 'failed'
                           WHEN status IN ('pending','ready_for_claim') THEN 'blocked'
                           ELSE status
                       END,
                       claimed_by_agent_id = NULL,
                       claim_expires_at = NULL,
                       updated_at = ?
                       WHERE session_id = ?"#,
                )
                .bind(&now)
                .bind(session_id)
                .execute(pool)
                .await?;
            }
        }
        DatabasePool::PostgreSQL(pool) => {
            if success {
                sqlx::query(
                    r#"UPDATE team_v3_tasks
                       SET status = CASE
                           WHEN status = 'cancelled' THEN status
                           ELSE 'completed'
                       END,
                       claimed_by_agent_id = NULL,
                       claim_expires_at = NULL,
                       updated_at = $1
                       WHERE session_id = $2"#,
                )
                .bind(&now)
                .bind(session_id)
                .execute(pool)
                .await?;
            } else {
                sqlx::query(
                    r#"UPDATE team_v3_tasks
                       SET status = CASE
                           WHEN status IN ('running','claimed','waiting_review','waiting_handoff_ack') THEN 'failed'
                           WHEN status IN ('pending','ready_for_claim') THEN 'blocked'
                           ELSE status
                       END,
                       claimed_by_agent_id = NULL,
                       claim_expires_at = NULL,
                       updated_at = $1
                       WHERE session_id = $2"#,
                )
                .bind(&now)
                .bind(session_id)
                .execute(pool)
                .await?;
            }
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }

    let current_state_data = get_team_v3_session_state_data(runtime_pool, session_id).await?;
    let next_state_data = build_team_state_data(Some(&current_state_data), None);
    set_team_v3_session_state_data(runtime_pool, session_id, &next_state_data, &now).await?;
    set_team_v3_session_state(
        runtime_pool,
        session_id,
        if success { "PLAN_DRAFT" } else { "FAILED" },
        &now,
    )
    .await?;
    Ok(())
}

async fn get_team_v3_latest_human_message_content(
    runtime_pool: &DatabasePool,
    session_id: &str,
) -> Result<Option<String>> {
    let payload_opt: Option<String> = match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT payload FROM team_v3_messages
                   WHERE session_id = ? AND (message_type = 'human_input' OR message_type = 'user')
                   ORDER BY created_at DESC, id DESC
                   LIMIT 1"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            row.map(|r| r.get("payload"))
        }
        DatabasePool::PostgreSQL(pool) => {
            let row = sqlx::query(
                r#"SELECT payload::text as payload FROM team_v3_messages
                   WHERE session_id = $1 AND (message_type = 'human_input' OR message_type = 'user')
                   ORDER BY created_at DESC, id DESC
                   LIMIT 1"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            row.map(|r| r.get("payload"))
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    };

    let content = payload_opt
        .and_then(|payload| serde_json::from_str::<Value>(&payload).ok())
        .and_then(|value| {
            value
                .get("content")
                .and_then(|c| c.as_str())
                .map(|s| s.trim().to_string())
        })
        .filter(|s| !s.is_empty());

    Ok(content)
}

async fn append_team_v3_status_message(
    runtime_pool: &DatabasePool,
    session_id: &str,
    content: &str,
) -> Result<()> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let payload = json!({ "content": content });
    let payload_text = serde_json::to_string(&payload)?;

    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_messages
                   (id, session_id, thread_id, from_agent_id, to_agent_id, message_type, message_kind, payload, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(session_id)
            .bind(session_id)
            .bind("team_system")
            .bind(Option::<String>::None)
            .bind("status")
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
            .bind(&id)
            .bind(session_id)
            .bind(session_id)
            .bind("team_system")
            .bind(Option::<String>::None)
            .bind("status")
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

async fn get_team_v3_session_context(
    runtime_pool: &DatabasePool,
    session_id: &str,
) -> Result<(Option<String>, Option<String>)> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT conversation_id, goal
                   FROM team_v3_sessions
                   WHERE id = ?"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            Ok(row
                .map(|r| (r.get("conversation_id"), r.get("goal")))
                .unwrap_or((None, None)))
        }
        DatabasePool::PostgreSQL(pool) => {
            let row = sqlx::query(
                r#"SELECT conversation_id, goal
                   FROM team_v3_sessions
                   WHERE id = $1"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            Ok(row
                .map(|r| (r.get("conversation_id"), r.get("goal")))
                .unwrap_or((None, None)))
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V3 does not support MySQL")),
    }
}

async fn set_team_v3_session_conversation_id(
    runtime_pool: &DatabasePool,
    session_id: &str,
    conversation_id: &str,
) -> Result<()> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET conversation_id = COALESCE(conversation_id, ?),
                       updated_at = ?
                   WHERE id = ?"#,
            )
            .bind(conversation_id)
            .bind(Utc::now().to_rfc3339())
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET conversation_id = COALESCE(conversation_id, $1),
                       updated_at = $2
                   WHERE id = $3"#,
            )
            .bind(conversation_id)
            .bind(Utc::now().to_rfc3339())
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }
    Ok(())
}

fn blackboard_revision_from_metadata_value(metadata: &Value) -> Option<i64> {
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

async fn append_team_v3_blackboard_entry(
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

async fn list_team_v3_blackboard_entries(
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

#[derive(Debug, Clone, Default)]
struct TeamV3PromptQuery {
    goal: String,
    user_input: String,
    task_title: String,
    task_instruction: String,
    dependency_task_ids: Vec<String>,
    dependency_task_keys: Vec<String>,
}

impl TeamV3PromptQuery {
    fn for_planner(goal: &str, user_input: &str) -> Self {
        Self {
            goal: collapse_whitespace(goal),
            user_input: collapse_whitespace(user_input),
            task_title: String::new(),
            task_instruction: String::new(),
            dependency_task_ids: Vec::new(),
            dependency_task_keys: Vec::new(),
        }
    }

    fn for_task(
        goal: &str,
        user_input: &str,
        task: &TeamV3Task,
        dependency_task_ids: &[String],
        dependency_task_keys: &[String],
    ) -> Self {
        Self {
            goal: collapse_whitespace(goal),
            user_input: collapse_whitespace(user_input),
            task_title: collapse_whitespace(task.title.as_str()),
            task_instruction: collapse_whitespace(task.instruction.as_str()),
            dependency_task_ids: dependency_task_ids.to_vec(),
            dependency_task_keys: dependency_task_keys.to_vec(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct TeamV3BlackboardSectionDiagnostics {
    total: usize,
    selected: usize,
}

impl TeamV3BlackboardSectionDiagnostics {
    fn dropped(&self) -> usize {
        self.total.saturating_sub(self.selected)
    }
}

#[derive(Debug, Clone, Default)]
struct TeamV3BlackboardContextDiagnostics {
    mode: String,
    query_terms: usize,
    structured_memory: TeamV3BlackboardSectionDiagnostics,
    task_outputs: TeamV3BlackboardSectionDiagnostics,
    raw_events: TeamV3BlackboardSectionDiagnostics,
    raw_log: TeamV3BlackboardSectionDiagnostics,
    artifacts: TeamV3BlackboardSectionDiagnostics,
    checkpoints: TeamV3BlackboardSectionDiagnostics,
    context_chars: usize,
}

#[derive(Debug, Clone, Default)]
struct TeamV3BlackboardContextBuildResult {
    context: String,
    diagnostics: TeamV3BlackboardContextDiagnostics,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TeamV3BlackboardContextMode {
    Standard,
    CheckpointOnly,
}

#[derive(Default)]
struct TeamV3BlackboardLayers<'a> {
    structured_memory: Vec<&'a TeamV3BlackboardEntry>,
    task_outputs: Vec<&'a TeamV3BlackboardEntry>,
    raw_events: Vec<&'a TeamV3BlackboardEntry>,
    raw_log: Vec<&'a TeamV3BlackboardEntry>,
    checkpoints: Vec<&'a TeamV3BlackboardEntry>,
    artifacts: Vec<&'a TeamV3BlackboardEntry>,
}

fn split_blackboard_layers<'a>(entries: &'a [TeamV3BlackboardEntry]) -> TeamV3BlackboardLayers<'a> {
    let mut layers = TeamV3BlackboardLayers::default();
    for entry in entries {
        match entry.entry_type.as_str() {
            "structured_fact" => layers.structured_memory.push(entry),
            "task_output" => layers.task_outputs.push(entry),
            "working_memory" => layers.raw_events.push(entry),
            "checkpoint" => layers.checkpoints.push(entry),
            "artifact_ref" => layers.artifacts.push(entry),
            "goal" | "plan" | "task_start" | "task_error" | "plan_fallback" => {
                layers.raw_events.push(entry)
            }
            _ => layers.raw_log.push(entry),
        }
    }
    layers
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let total = input.chars().count();
    if total <= max_chars {
        return input.to_string();
    }
    if max_chars <= 8 {
        return input.chars().take(max_chars).collect::<String>();
    }
    let head = max_chars.saturating_mul(3) / 4;
    let tail = max_chars.saturating_sub(head + 5);
    let prefix = input.chars().take(head).collect::<String>();
    let suffix = input
        .chars()
        .rev()
        .take(tail)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{} ... {}", prefix.trim_end(), suffix.trim_start())
}

fn collapse_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clip_to_sentence_boundary(input: &str, max_chars: usize) -> Option<String> {
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

fn normalize_fact_for_prompt(input: &str, max_chars: usize) -> Option<String> {
    let collapsed = collapse_whitespace(input);
    if collapsed.is_empty() {
        return None;
    }
    if collapsed.chars().count() <= max_chars {
        return Some(collapsed);
    }
    clip_to_sentence_boundary(collapsed.as_str(), max_chars)
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x4E00..=0x9FFF
            | 0x3400..=0x4DBF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0xF900..=0xFAFF
    )
}

fn extract_query_terms(text: &str) -> Vec<String> {
    const MAX_TERMS: usize = 36;
    let stop_words = [
        "the", "and", "for", "with", "that", "this", "from", "into", "then", "have", "need",
        "task", "team", "用户", "针对", "进行", "继续", "然后", "主要", "关注", "所有", "一个",
    ];
    let mut terms = Vec::new();
    let mut seen = HashSet::new();
    for token in text
        .to_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric() && !is_cjk(ch))
    {
        let normalized = token.trim();
        if normalized.is_empty() {
            continue;
        }
        let token_len = normalized.chars().count();
        if token_len < 2 {
            continue;
        }
        if token_len < 4 && normalized.chars().all(|ch| ch.is_ascii()) {
            continue;
        }
        if stop_words.contains(&normalized) {
            continue;
        }
        if seen.insert(normalized.to_string()) {
            terms.push(normalized.to_string());
            if terms.len() >= MAX_TERMS {
                break;
            }
        }
    }
    terms
}

fn build_query_terms(query: &TeamV3PromptQuery) -> Vec<String> {
    let mut raw = Vec::new();
    if !query.goal.trim().is_empty() {
        raw.push(query.goal.as_str());
    }
    if !query.user_input.trim().is_empty() {
        raw.push(query.user_input.as_str());
    }
    if !query.task_title.trim().is_empty() {
        raw.push(query.task_title.as_str());
    }
    if !query.task_instruction.trim().is_empty() {
        raw.push(query.task_instruction.as_str());
    }
    for dep in query.dependency_task_keys.iter().take(24) {
        raw.push(dep.as_str());
    }
    extract_query_terms(raw.join(" ").as_str())
}

fn entry_search_text(entry: &TeamV3BlackboardEntry) -> String {
    let mut segments = vec![
        entry.entry_type.as_str().to_string(),
        entry.content.as_str().to_string(),
    ];
    if let Some(task_key) = entry.metadata.get("task_key").and_then(Value::as_str) {
        segments.push(task_key.to_string());
    }
    if let Some(task_title) = entry.metadata.get("task_title").and_then(Value::as_str) {
        segments.push(task_title.to_string());
    }
    if let Some(facts) = entry.metadata.get("facts").and_then(Value::as_object) {
        for value in facts.values() {
            match value {
                Value::String(text) => segments.push(text.clone()),
                Value::Array(items) => {
                    for item in items {
                        if let Some(text) = item.as_str() {
                            segments.push(text.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    collapse_whitespace(segments.join(" ").as_str()).to_lowercase()
}

fn entry_base_weight(entry_type: &str) -> i64 {
    match entry_type {
        "structured_fact" => 70,
        "checkpoint" => 55,
        "artifact_ref" => 36,
        "task_output" => 30,
        "working_memory" => 26,
        "plan" => 20,
        "goal" => 12,
        _ => 8,
    }
}

fn score_blackboard_entry(
    entry: &TeamV3BlackboardEntry,
    terms: &[String],
    query: &TeamV3PromptQuery,
    recency_index: usize,
) -> i64 {
    let search_text = entry_search_text(entry);
    let mut score = entry_base_weight(entry.entry_type.as_str());
    for term in terms {
        if search_text.contains(term) {
            score += (term.chars().count().min(18) as i64) * 2;
        }
    }
    let task_id_match = entry
        .task_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| query.dependency_task_ids.iter().any(|dep| dep == value))
        .unwrap_or(false);
    let task_key_match = entry
        .metadata
        .get("task_key")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| query.dependency_task_keys.iter().any(|dep| dep == value))
        .unwrap_or(false);
    if task_id_match || task_key_match {
        score += 95;
    }
    if search_text.contains("sql注入")
        || search_text.contains("rce")
        || search_text.contains("未授权")
        || search_text.contains("source-sink")
        || search_text.contains("风险")
        || search_text.contains("漏洞")
    {
        score += 22;
    }
    if search_text.contains("最终结论")
        || search_text.contains("汇总")
        || search_text.contains("清单")
        || search_text.contains("基线")
    {
        score += 14;
    }
    score + ((120usize.saturating_sub(recency_index)) as i64 / 6)
}

fn select_blackboard_entries<'a>(
    entries: &[&'a TeamV3BlackboardEntry],
    limit: usize,
    always_keep_recent: usize,
    terms: &[String],
    query: &TeamV3PromptQuery,
) -> Vec<&'a TeamV3BlackboardEntry> {
    if entries.is_empty() || limit == 0 {
        return Vec::new();
    }

    let mut selected = HashSet::new();
    for index in 0..entries.len().min(always_keep_recent) {
        selected.insert(index);
    }

    let mut scored = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (index, score_blackboard_entry(entry, terms, query, index)))
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    for (index, _) in scored {
        if selected.len() >= limit {
            break;
        }
        selected.insert(index);
    }

    if selected.len() < limit {
        for index in 0..entries.len() {
            if selected.len() >= limit {
                break;
            }
            selected.insert(index);
        }
    }

    let mut selected_entries = selected
        .into_iter()
        .map(|index| entries[index])
        .collect::<Vec<_>>();
    selected_entries.sort_by(|a, b| {
        let a_revision = blackboard_revision_from_metadata_value(&a.metadata).unwrap_or(0);
        let b_revision = blackboard_revision_from_metadata_value(&b.metadata).unwrap_or(0);
        a_revision
            .cmp(&b_revision)
            .then_with(|| a.created_at.cmp(&b.created_at))
    });
    selected_entries
}

fn dedupe_event_entries_for_prompt<'a>(
    entries: &[&'a TeamV3BlackboardEntry],
) -> Vec<&'a TeamV3BlackboardEntry> {
    if entries.is_empty() {
        return Vec::new();
    }
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for entry in entries {
        let task = entry
            .task_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("-");
        let agent = entry
            .agent_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("-");
        let signature = format!(
            "{}|{}|{}|{}",
            entry.entry_type,
            agent,
            task,
            normalize_blackboard_dedupe_content(entry.content.as_str())
        );
        if seen.insert(signature) {
            deduped.push(*entry);
        }
    }
    deduped
}

fn format_blackboard_entry(entry: &TeamV3BlackboardEntry, max_chars: usize) -> String {
    let agent = entry
        .agent_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown-agent");
    let task = entry
        .task_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("-");
    if entry.entry_type == "artifact_ref" {
        let summary = entry
            .metadata
            .get("summary")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(entry.content.as_str());
        let file_path = entry
            .metadata
            .get("artifact")
            .and_then(|value| value.get("path"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("-");
        let file_bytes = entry
            .metadata
            .get("artifact")
            .and_then(|value| value.get("bytes"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let revision = blackboard_revision_from_metadata_value(&entry.metadata).unwrap_or(0);
        let summary_content =
            truncate_chars(collapse_whitespace(summary).as_str(), max_chars.saturating_sub(40));
        let path_content = truncate_chars(file_path, 140);
        return format!(
            "- [{}][rev={}][agent={}][task={}][bytes={}] {} | file={}",
            entry.entry_type, revision, agent, task, file_bytes, summary_content, path_content
        );
    }
    let content = truncate_chars(
        collapse_whitespace(entry.content.as_str()).as_str(),
        max_chars,
    );
    let revision = blackboard_revision_from_metadata_value(&entry.metadata).unwrap_or(0);
    format!(
        "- [{}][rev={}][agent={}][task={}] {}",
        entry.entry_type, revision, agent, task, content
    )
}

fn latest_blackboard_revision(entries: &[TeamV3BlackboardEntry]) -> i64 {
    entries
        .iter()
        .filter_map(|entry| blackboard_revision_from_metadata_value(&entry.metadata))
        .max()
        .unwrap_or(0)
}

fn build_blackboard_section_with_formatter<'a, F>(
    title: &str,
    entries: &[&'a TeamV3BlackboardEntry],
    limit: usize,
    always_keep_recent: usize,
    terms: &[String],
    query: &TeamV3PromptQuery,
    formatter: F,
) -> (Option<String>, TeamV3BlackboardSectionDiagnostics)
where
    F: Fn(&TeamV3BlackboardEntry) -> String,
{
    let total = entries.len();
    if total == 0 || limit == 0 {
        return (
            None,
            TeamV3BlackboardSectionDiagnostics { total, selected: 0 },
        );
    }
    let selected = select_blackboard_entries(entries, limit, always_keep_recent, terms, query);
    if selected.is_empty() {
        return (
            None,
            TeamV3BlackboardSectionDiagnostics { total, selected: 0 },
        );
    }
    let rows = selected
        .iter()
        .map(|entry| formatter(entry))
        .collect::<Vec<_>>();
    (
        Some(format!("{}\n{}", title, rows.join("\n"))),
        TeamV3BlackboardSectionDiagnostics {
            total,
            selected: selected.len(),
        },
    )
}

#[derive(Debug, Clone, Default)]
struct TeamV3CheckpointFacts {
    task_key: String,
    title: String,
    conclusion: Option<String>,
    evidence: Option<String>,
    risk: Option<String>,
    next_step: Option<String>,
    highlights: Vec<String>,
}

fn parse_checkpoint_content_fields(
    content: &str,
) -> (Option<String>, Option<String>, Vec<String>) {
    let mut task_key = None;
    let mut title = None;
    let mut points = Vec::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if let Some(value) = line.strip_prefix("task_key=") {
            let normalized = collapse_whitespace(value);
            if !normalized.is_empty() {
                task_key = Some(normalized);
            }
            continue;
        }
        if let Some(value) = line.strip_prefix("title=") {
            let normalized = collapse_whitespace(value);
            if !normalized.is_empty() {
                title = Some(normalized);
            }
            continue;
        }
        if let Some(value) = line.strip_prefix("- ") {
            let normalized = collapse_whitespace(value);
            if !normalized.is_empty() {
                points.push(normalized);
            }
        }
    }
    (task_key, title, points)
}

fn infer_checkpoint_structured_fields(
    points: &[String],
) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let mut conclusion = None;
    let mut evidence = None;
    let mut risk = None;
    let mut next_step = None;
    for point in points {
        let lower = point.to_lowercase();
        let is_conclusion = point.contains("结论")
            || point.contains("总结")
            || lower.contains("conclusion")
            || lower.contains("summary");
        let is_evidence =
            point.contains("依据") || point.contains("证据") || lower.contains("evidence");
        let is_risk = point.contains("风险")
            || point.contains("漏洞")
            || point.contains("隐患")
            || lower.contains("risk")
            || lower.contains("impact");
        let is_next = point.contains("下一步")
            || point.contains("建议")
            || point.contains("行动")
            || lower.contains("next")
            || lower.contains("recommend");
        if conclusion.is_none() && is_conclusion {
            conclusion = Some(point.clone());
            continue;
        }
        if evidence.is_none() && is_evidence {
            evidence = Some(point.clone());
            continue;
        }
        if risk.is_none() && is_risk {
            risk = Some(point.clone());
            continue;
        }
        if next_step.is_none() && is_next {
            next_step = Some(point.clone());
            continue;
        }
    }
    (conclusion, evidence, risk, next_step)
}

fn is_placeholder_fact(input: &str) -> bool {
    let normalized = collapse_whitespace(input)
        .trim_matches(|ch: char| matches!(ch, ':' | '：' | '-' | '|' | '*' | '#' | '。'))
        .to_string();
    matches!(
        normalized.as_str(),
        "结论" | "依据" | "证据" | "风险" | "下一步" | "建议" | "行动" | "Conclusion"
            | "Evidence" | "Risk" | "Next" | "Recommendation"
    )
}

fn sanitize_structured_fact_value(value: Option<String>) -> Option<String> {
    value
        .map(|item| collapse_whitespace(item.as_str()))
        .filter(|item| !item.is_empty())
        .filter(|item| !is_placeholder_fact(item.as_str()))
}

fn extract_checkpoint_facts(entry: &TeamV3BlackboardEntry) -> TeamV3CheckpointFacts {
    let (content_task_key, content_title, content_points) =
        parse_checkpoint_content_fields(entry.content.as_str());
    let metadata_task_key = entry
        .metadata
        .get("task_key")
        .and_then(Value::as_str)
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty());
    let metadata_title = entry
        .metadata
        .get("task_title")
        .and_then(Value::as_str)
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty());
    let metadata_facts = entry.metadata.get("facts");

    let metadata_points = metadata_facts
        .and_then(|facts| facts.get("highlights"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(collapse_whitespace)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let points = if metadata_points.is_empty() {
        content_points
    } else {
        metadata_points
    };

    let mut conclusion = sanitize_structured_fact_value(
        metadata_facts
        .and_then(|facts| facts.get("conclusion"))
        .and_then(Value::as_str)
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty()),
    );
    let mut evidence = sanitize_structured_fact_value(
        metadata_facts
        .and_then(|facts| facts.get("evidence"))
        .and_then(Value::as_str)
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty()),
    );
    let mut risk = sanitize_structured_fact_value(
        metadata_facts
        .and_then(|facts| facts.get("risk"))
        .and_then(Value::as_str)
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty()),
    );
    let mut next_step = sanitize_structured_fact_value(
        metadata_facts
        .and_then(|facts| facts.get("next_step"))
        .and_then(Value::as_str)
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty()),
    );

    let (inferred_conclusion, inferred_evidence, inferred_risk, inferred_next_step) =
        infer_checkpoint_structured_fields(&points);
    if conclusion.is_none() {
        conclusion = sanitize_structured_fact_value(inferred_conclusion);
    }
    if evidence.is_none() {
        evidence = sanitize_structured_fact_value(inferred_evidence);
    }
    if risk.is_none() {
        risk = sanitize_structured_fact_value(inferred_risk);
    }
    if next_step.is_none() {
        next_step = sanitize_structured_fact_value(inferred_next_step);
    }

    let mut highlights = points
        .iter()
        .filter(|point| {
            Some(*point) != conclusion.as_ref()
                && Some(*point) != evidence.as_ref()
                && Some(*point) != risk.as_ref()
                && Some(*point) != next_step.as_ref()
        })
        .take(4)
        .cloned()
        .collect::<Vec<_>>();
    if highlights.is_empty() && conclusion.is_none() && evidence.is_none() && risk.is_none() {
        highlights = points.into_iter().take(4).collect::<Vec<_>>();
    }

    TeamV3CheckpointFacts {
        task_key: metadata_task_key
            .or(content_task_key)
            .unwrap_or_else(|| "-".to_string()),
        title: metadata_title
            .or(content_title)
            .unwrap_or_else(|| "未命名任务".to_string()),
        conclusion,
        evidence,
        risk,
        next_step,
        highlights,
    }
}

fn format_checkpoint_entry_for_prompt(entry: &TeamV3BlackboardEntry) -> String {
    let revision = blackboard_revision_from_metadata_value(&entry.metadata).unwrap_or(0);
    let agent = entry
        .agent_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown-agent");
    let task = entry
        .task_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("-");
    let facts = extract_checkpoint_facts(entry);
    let mut fields = vec![
        format!("- [checkpoint][rev={}][agent={}][task={}]", revision, agent, task),
        format!("task_key={}", facts.task_key),
        format!("title={}", facts.title),
    ];

    if let Some(value) = facts
        .conclusion
        .and_then(|value| normalize_fact_for_prompt(value.as_str(), 260))
    {
        fields.push(format!("结论={}", value));
    }
    if let Some(value) = facts
        .evidence
        .and_then(|value| normalize_fact_for_prompt(value.as_str(), 260))
    {
        fields.push(format!("依据={}", value));
    }
    if let Some(value) = facts
        .risk
        .and_then(|value| normalize_fact_for_prompt(value.as_str(), 260))
    {
        fields.push(format!("风险={}", value));
    }
    if let Some(value) = facts
        .next_step
        .and_then(|value| normalize_fact_for_prompt(value.as_str(), 260))
    {
        fields.push(format!("下一步={}", value));
    }

    if !facts.highlights.is_empty() {
        let highlights = facts
            .highlights
            .iter()
            .filter_map(|value| normalize_fact_for_prompt(value.as_str(), 220))
            .collect::<Vec<_>>();
        if !highlights.is_empty() {
            fields.push(format!("要点={}", highlights.join(" | ")));
        }
    }
    fields.join(" | ")
}

fn format_structured_memory_entry_for_prompt(entry: &TeamV3BlackboardEntry) -> String {
    let revision = blackboard_revision_from_metadata_value(&entry.metadata).unwrap_or(0);
    let agent = entry
        .agent_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown-agent");
    let task = entry
        .task_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("-");
    let task_key = entry
        .metadata
        .get("task_key")
        .and_then(Value::as_str)
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".to_string());
    let title = entry
        .metadata
        .get("task_title")
        .and_then(Value::as_str)
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "未命名任务".to_string());
    let conclusion = entry
        .metadata
        .get("facts")
        .and_then(|value| value.get("conclusion"))
        .and_then(Value::as_str)
        .and_then(|value| normalize_fact_for_prompt(value, 260));
    let evidence = entry
        .metadata
        .get("facts")
        .and_then(|value| value.get("evidence"))
        .and_then(Value::as_str)
        .and_then(|value| normalize_fact_for_prompt(value, 260));
    let risk = entry
        .metadata
        .get("facts")
        .and_then(|value| value.get("risk"))
        .and_then(Value::as_str)
        .and_then(|value| normalize_fact_for_prompt(value, 260));
    let next_step = entry
        .metadata
        .get("facts")
        .and_then(|value| value.get("next_step"))
        .and_then(Value::as_str)
        .and_then(|value| normalize_fact_for_prompt(value, 260));
    let tags = entry
        .metadata
        .get("tags")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(collapse_whitespace)
                .filter(|value| !value.is_empty())
                .take(6)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let artifact_path = entry
        .metadata
        .get("artifact")
        .and_then(|value| value.get("path"))
        .and_then(Value::as_str)
        .map(collapse_whitespace)
        .filter(|value| !value.is_empty());

    let mut fields = vec![
        format!("- [structured_fact][rev={}][agent={}][task={}]", revision, agent, task),
        format!("task_key={}", task_key),
        format!("title={}", title),
    ];
    if let Some(value) = conclusion {
        fields.push(format!("结论={}", value));
    }
    if let Some(value) = evidence {
        fields.push(format!("依据={}", value));
    }
    if let Some(value) = risk {
        fields.push(format!("风险={}", value));
    }
    if let Some(value) = next_step {
        fields.push(format!("下一步={}", value));
    }
    if !tags.is_empty() {
        fields.push(format!("tags={}", tags.join(",")));
    }
    if let Some(path) = artifact_path {
        fields.push(format!("artifact={}", path));
    }
    if fields.len() <= 3 {
        fields.push(format!(
            "摘要={}",
            normalize_fact_for_prompt(entry.content.as_str(), 220)
                .unwrap_or_else(|| "暂无结构化摘要".to_string())
        ));
    }
    fields.join(" | ")
}

fn build_blackboard_context(
    entries: &[TeamV3BlackboardEntry],
    query: &TeamV3PromptQuery,
) -> TeamV3BlackboardContextBuildResult {
    build_blackboard_context_with_mode(entries, TeamV3BlackboardContextMode::Standard, query)
}

fn build_blackboard_checkpoint_context(
    entries: &[TeamV3BlackboardEntry],
    query: &TeamV3PromptQuery,
) -> TeamV3BlackboardContextBuildResult {
    build_blackboard_context_with_mode(entries, TeamV3BlackboardContextMode::CheckpointOnly, query)
}

fn build_blackboard_context_with_mode(
    entries: &[TeamV3BlackboardEntry],
    mode: TeamV3BlackboardContextMode,
    query: &TeamV3PromptQuery,
) -> TeamV3BlackboardContextBuildResult {
    let mut diagnostics = TeamV3BlackboardContextDiagnostics {
        mode: if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            "checkpoint_only".to_string()
        } else {
            "standard".to_string()
        },
        ..TeamV3BlackboardContextDiagnostics::default()
    };
    if entries.is_empty() {
        return TeamV3BlackboardContextBuildResult {
            context: String::new(),
            diagnostics,
        };
    }

    let query_terms = build_query_terms(query);
    diagnostics.query_terms = query_terms.len();
    let layers = split_blackboard_layers(entries);
    let mut sections: Vec<String> = Vec::new();

    let (structured, structured_diag) = build_blackboard_section_with_formatter(
        "Structured Memory（结构化记忆）:",
        &layers.structured_memory,
        if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            18
        } else {
            14
        },
        if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            8
        } else {
            5
        },
        &query_terms,
        query,
        format_structured_memory_entry_for_prompt,
    );
    diagnostics.structured_memory = structured_diag;
    if let Some(structured) = structured {
        sections.push(structured);
    }

    let (task_outputs, task_output_diag) = build_blackboard_section_with_formatter(
        "Task Output Evidence（关键执行证据）:",
        &layers.task_outputs,
        if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            5
        } else {
            8
        },
        2,
        &query_terms,
        query,
        |entry| format_blackboard_entry(entry, 320),
    );
    diagnostics.task_outputs = task_output_diag;
    if let Some(task_outputs) = task_outputs {
        sections.push(task_outputs);
    }

    let (artifacts, artifacts_diag) = build_blackboard_section_with_formatter(
        "Artifacts（关键产物摘要）:",
        &layers.artifacts,
        if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            6
        } else {
            8
        },
        2,
        &query_terms,
        query,
        |entry| format_blackboard_entry(entry, 280),
    );
    diagnostics.artifacts = artifacts_diag;
    if let Some(artifacts) = artifacts {
        sections.push(artifacts);
    }

    let deduped_events = dedupe_event_entries_for_prompt(&layers.raw_events);
    if deduped_events.len() < layers.raw_events.len() {
        tracing::info!(
            "Team V3 prompt event dedupe applied: total={} deduped={}",
            layers.raw_events.len(),
            deduped_events.len()
        );
    }
    let (events, events_diag) = build_blackboard_section_with_formatter(
        "Team Events（关键事件）:",
        &deduped_events,
        6,
        2,
        &query_terms,
        query,
        |entry| format_blackboard_entry(entry, 220),
    );
    diagnostics.raw_events = events_diag;
    if let Some(events) = events {
        sections.push(events);
    }

    if mode == TeamV3BlackboardContextMode::Standard {
        let (raw, raw_diag) = build_blackboard_section_with_formatter(
            "Raw Log（全量事件节选）:",
            &layers.raw_log,
            4,
            1,
            &query_terms,
            query,
            |entry| format_blackboard_entry(entry, 220),
        );
        diagnostics.raw_log = raw_diag;
        if let Some(raw) = raw {
            sections.push(raw);
        }
    }

    let (checkpoints, checkpoints_diag) = build_blackboard_section_with_formatter(
        "Checkpoint（阶段总结）:",
        &layers.checkpoints,
        if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            16
        } else {
            12
        },
        if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            6
        } else {
            3
        },
        &query_terms,
        query,
        format_checkpoint_entry_for_prompt,
    );
    diagnostics.checkpoints = checkpoints_diag;
    if diagnostics.structured_memory.selected == 0 {
        if let Some(checkpoints) = checkpoints {
            sections.push(checkpoints);
        } else if mode == TeamV3BlackboardContextMode::CheckpointOnly {
            sections.push(
                "Checkpoint（阶段总结）:\n- 暂无 checkpoint，可基于已知依赖先产出汇总草稿。"
                    .to_string(),
            );
        }
    }

    let context = if sections.is_empty() {
        String::new()
    } else {
        format!("Team 黑板（分层共享）:\n{}", sections.join("\n\n"))
    };
    diagnostics.context_chars = context.chars().count();

    TeamV3BlackboardContextBuildResult {
        context,
        diagnostics,
    }
}

fn normalize_checkpoint_line(line: &str) -> Option<String> {
    let trimmed = line
        .trim()
        .trim_start_matches(|ch: char| {
            matches!(
                ch,
                '-' | '*' | '#' | '>' | '•' | '·' | '1'..='9' | '0' | '.' | '、' | ')' | '('
            )
        })
        .trim();
    if trimmed.is_empty() {
        return None;
    }
    let collapsed = collapse_whitespace(trimmed);
    if collapsed.is_empty() {
        None
    } else {
        Some(collapsed)
    }
}

fn collect_checkpoint_points(
    output: &str,
    max_points: usize,
    max_chars_per_point: usize,
) -> Vec<String> {
    let priority_terms = [
        "结论",
        "依据",
        "风险",
        "下一步",
        "建议",
        "行动",
        "结论:",
        "Conclusion",
        "Evidence",
        "Risk",
        "Next",
        "Recommendation",
    ];
    let mut seen = HashSet::new();
    let mut prioritized = Vec::new();
    let mut fallback = Vec::new();
    for raw_line in output.lines() {
        let Some(line) = normalize_checkpoint_line(raw_line) else {
            continue;
        };
        if !seen.insert(line.clone()) {
            continue;
        }
        let Some(clipped) = normalize_fact_for_prompt(line.as_str(), max_chars_per_point) else {
            continue;
        };
        let lower = clipped.to_lowercase();
        let has_priority = priority_terms.iter().any(|term| {
            if term.chars().all(|ch| ch.is_ascii()) {
                lower.contains(&term.to_lowercase())
            } else {
                clipped.contains(term)
            }
        });
        if has_priority {
            prioritized.push(clipped);
        } else {
            fallback.push(clipped);
        }
    }
    let mut points = Vec::new();
    points.extend(prioritized.into_iter().take(max_points));
    if points.len() < max_points {
        points.extend(fallback.into_iter().take(max_points - points.len()));
    }
    if points.is_empty() {
        let fallback_point = clip_to_sentence_boundary(output, max_chars_per_point)
            .unwrap_or_else(|| "任务完成，待进一步提炼。".to_string());
        points.push(fallback_point);
    }
    points
}

fn build_task_working_memory_content(task_key: &str, output: &str) -> String {
    let first_point = collect_checkpoint_points(output, 1, 220)
        .into_iter()
        .next()
        .unwrap_or_else(|| "任务完成，待进一步提炼。".to_string());
    format!("task_key={}: {}", task_key, first_point)
}

#[derive(Debug, Clone)]
struct TeamV3TaskCheckpointPayload {
    content: String,
    facts: Value,
}

#[derive(Debug, Clone)]
struct TeamV3StructuredFactPayload {
    task_key: String,
    task_title: String,
    content: String,
    facts: Value,
    tags: Vec<String>,
}

fn parse_fact_fields_from_value(
    facts: &Value,
) -> (Option<String>, Option<String>, Option<String>, Option<String>, Vec<String>) {
    let conclusion = sanitize_structured_fact_value(
        facts
            .get("conclusion")
            .and_then(Value::as_str)
            .map(|value| value.to_string()),
    );
    let evidence = sanitize_structured_fact_value(
        facts
            .get("evidence")
            .and_then(Value::as_str)
            .map(|value| value.to_string()),
    );
    let risk = sanitize_structured_fact_value(
        facts
            .get("risk")
            .and_then(Value::as_str)
            .map(|value| value.to_string()),
    );
    let next_step = sanitize_structured_fact_value(
        facts
            .get("next_step")
            .and_then(Value::as_str)
            .map(|value| value.to_string()),
    );
    let highlights = facts
        .get("highlights")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|value| collapse_whitespace(value))
                .filter(|value| !value.is_empty())
                .filter(|value| !is_placeholder_fact(value.as_str()))
                .take(5)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    (conclusion, evidence, risk, next_step, highlights)
}

fn derive_security_tags_from_texts(texts: &[&str]) -> Vec<String> {
    let merged = texts
        .iter()
        .map(|value| value.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let mut tags = Vec::new();
    if merged.contains("sql") || merged.contains("注入") {
        tags.push("sql-injection");
    }
    if merged.contains("rce") || merged.contains("远程代码执行") {
        tags.push("rce");
    }
    if merged.contains("未授权")
        || merged.contains("越权")
        || merged.contains("unauthor")
        || merged.contains("privilege")
    {
        tags.push("unauthorized-access");
    }
    if merged.contains("source-sink") || merged.contains("source sink") {
        tags.push("source-sink");
    }
    if merged.contains("csrf") {
        tags.push("csrf");
    }
    if tags.is_empty() {
        tags.push("general");
    }
    tags
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
}

fn build_structured_fact_content(
    task_key: &str,
    task_title: &str,
    facts: &Value,
    fallback: &str,
) -> Option<String> {
    let (conclusion, evidence, risk, next_step, highlights) = parse_fact_fields_from_value(facts);
    if conclusion.is_none()
        && evidence.is_none()
        && risk.is_none()
        && next_step.is_none()
        && highlights.is_empty()
    {
        let fallback_line = normalize_fact_for_prompt(fallback, 220)?;
        if is_placeholder_fact(fallback_line.as_str()) {
            return None;
        }
        return Some(format!(
            "task_key={}\ntitle={}\nkey_points:\n- {}",
            task_key,
            collapse_whitespace(task_title),
            fallback_line
        ));
    }

    let mut lines = vec![
        format!("task_key={}", task_key),
        format!("title={}", collapse_whitespace(task_title)),
        "key_points:".to_string(),
    ];
    if let Some(value) = conclusion {
        lines.push(format!("- 结论: {}", value));
    }
    if let Some(value) = evidence {
        lines.push(format!("- 依据: {}", value));
    }
    if let Some(value) = risk {
        lines.push(format!("- 风险: {}", value));
    }
    if let Some(value) = next_step {
        lines.push(format!("- 下一步: {}", value));
    }
    lines.extend(highlights.into_iter().map(|value| format!("- {}", value)));
    Some(lines.join("\n"))
}

fn build_structured_fact_payload(
    task_key: &str,
    task_title: &str,
    facts: &Value,
    fallback: &str,
) -> Option<TeamV3StructuredFactPayload> {
    let content = build_structured_fact_content(task_key, task_title, facts, fallback)?;
    let (conclusion, evidence, risk, next_step, highlights) = parse_fact_fields_from_value(facts);
    let facts_value = json!({
        "conclusion": conclusion,
        "evidence": evidence,
        "risk": risk,
        "next_step": next_step,
        "highlights": highlights,
    });
    let tags = derive_security_tags_from_texts(&[content.as_str(), fallback]);
    Some(TeamV3StructuredFactPayload {
        task_key: task_key.to_string(),
        task_title: collapse_whitespace(task_title),
        content,
        facts: facts_value,
        tags,
    })
}

fn build_task_checkpoint_payload(
    task_key: &str,
    task_title: &str,
    output: &str,
) -> TeamV3TaskCheckpointPayload {
    let points = collect_checkpoint_points(output, 6, 220);
    let (raw_conclusion, raw_evidence, raw_risk, raw_next_step) =
        infer_checkpoint_structured_fields(&points);
    let conclusion = sanitize_structured_fact_value(raw_conclusion);
    let evidence = sanitize_structured_fact_value(raw_evidence);
    let risk = sanitize_structured_fact_value(raw_risk);
    let next_step = sanitize_structured_fact_value(raw_next_step);
    let highlights = points
        .iter()
        .filter(|point| {
            Some(*point) != conclusion.as_ref()
                && Some(*point) != evidence.as_ref()
                && Some(*point) != risk.as_ref()
                && Some(*point) != next_step.as_ref()
                && !is_placeholder_fact(point.as_str())
        })
        .take(4)
        .cloned()
        .collect::<Vec<_>>();
    let mut lines = vec![
        format!("task_key={}", task_key),
        format!("title={}", collapse_whitespace(task_title)),
        "key_points:".to_string(),
    ];
    lines.extend(points.iter().map(|point| format!("- {}", point)));
    TeamV3TaskCheckpointPayload {
        content: lines.join("\n"),
        facts: json!({
            "conclusion": conclusion,
            "evidence": evidence,
            "risk": risk,
            "next_step": next_step,
            "highlights": highlights,
        }),
    }
}

fn build_task_artifact_summary(output: &str) -> String {
    let points = collect_checkpoint_points(output, 4, 220);
    if points.is_empty() {
        return "未提取到摘要，请直接阅读 artifact 文件。".to_string();
    }
    points
        .into_iter()
        .map(|point| format!("- {}", point))
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_task_artifact_ref_content(summary: &str, path: &str, bytes: usize) -> String {
    format!(
        "任务产出较长，已落地为 artifact。\n摘要：\n{}\n\nartifact_path: {}\nartifact_bytes: {}",
        summary, path, bytes
    )
}

fn build_structured_fact_payload_from_checkpoint_entry(
    checkpoint: &TeamV3BlackboardEntry,
) -> Option<TeamV3StructuredFactPayload> {
    let facts = extract_checkpoint_facts(checkpoint);
    let (_, _, point_candidates) = parse_checkpoint_content_fields(checkpoint.content.as_str());
    let fallback = point_candidates
        .into_iter()
        .map(|value| collapse_whitespace(value.as_str()))
        .find(|value| !value.is_empty() && !is_placeholder_fact(value.as_str()))
        .unwrap_or_default();
    let facts_value = json!({
        "conclusion": facts.conclusion,
        "evidence": facts.evidence,
        "risk": facts.risk,
        "next_step": facts.next_step,
        "highlights": facts.highlights,
    });
    build_structured_fact_payload(
        facts.task_key.as_str(),
        facts.title.as_str(),
        &facts_value,
        fallback.as_str(),
    )
}

async fn backfill_team_v3_structured_memory_from_checkpoints(
    runtime_pool: &DatabasePool,
    session_id: &str,
    scan_limit: i64,
) -> Result<usize> {
    let entries = list_team_v3_blackboard_entries(runtime_pool, session_id, scan_limit).await?;
    if entries.is_empty() {
        return Ok(0);
    }

    let mut existing_source_checkpoint_ids = entries
        .iter()
        .filter(|entry| entry.entry_type == "structured_fact")
        .filter_map(|entry| {
            entry
                .metadata
                .get("source_checkpoint_entry_id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string())
        })
        .collect::<HashSet<_>>();
    let mut existing_structured_task_keys = entries
        .iter()
        .filter(|entry| entry.entry_type == "structured_fact")
        .filter_map(|entry| {
            let source_type = entry
                .metadata
                .get("source_type")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            if source_type != "task_output" {
                return None;
            }
            let task_key = entry
                .metadata
                .get("task_key")
                .and_then(Value::as_str)
                .map(collapse_whitespace)
                .filter(|value| !value.is_empty())?;
            let task_id = entry
                .task_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("-");
            Some(format!("{}::{}", task_id, task_key))
        })
        .collect::<HashSet<_>>();

    let mut appended = 0usize;
    for checkpoint in entries
        .iter()
        .rev()
        .filter(|entry| entry.entry_type == "checkpoint")
    {
        if existing_source_checkpoint_ids.contains(checkpoint.id.as_str()) {
            continue;
        }
        let Some(structured) = build_structured_fact_payload_from_checkpoint_entry(checkpoint) else {
            continue;
        };
        let task_id_for_key = checkpoint
            .task_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("-");
        let task_key = structured.task_key.clone();
        let dedupe_key = format!("{}::{}", task_id_for_key, task_key);
        if existing_structured_task_keys.contains(dedupe_key.as_str()) {
            continue;
        }
        let source_revision = blackboard_revision_from_metadata_value(&checkpoint.metadata).unwrap_or(0);
        let structured_meta = json!({
            "task_key": task_key,
            "task_title": structured.task_title,
            "source_type": "checkpoint_backfill",
            "source_checkpoint_entry_id": checkpoint.id,
            "source_checkpoint_revision": source_revision,
            "facts": structured.facts,
            "tags": structured.tags,
        });
        append_team_v3_blackboard_entry(
            runtime_pool,
            session_id,
            checkpoint.task_id.as_deref(),
            checkpoint.agent_id.as_deref(),
            "structured_fact",
            structured.content.as_str(),
            Some(&structured_meta),
        )
        .await?;
        existing_source_checkpoint_ids.insert(checkpoint.id.clone());
        existing_structured_task_keys.insert(dedupe_key);
        appended += 1;
    }
    Ok(appended)
}

async fn append_team_v3_task_memory_layers(
    runtime_pool: &DatabasePool,
    session_id: &str,
    task_id: &str,
    member_id: &str,
    task_key: &str,
    task_title: &str,
    output: &str,
    dependency_task_ids: &[String],
    dependency_task_keys: &[String],
    artifact_ref: Option<&TeamV3ArtifactFileRef>,
) -> Result<()> {
    let working_memory = build_task_working_memory_content(task_key, output);
    let working_meta = json!({
        "task_key": task_key,
        "source_type": "task_output",
    });
    append_team_v3_blackboard_entry(
        runtime_pool,
        session_id,
        Some(task_id),
        Some(member_id),
        "working_memory",
        working_memory.as_str(),
        Some(&working_meta),
    )
    .await?;

    let checkpoint = build_task_checkpoint_payload(task_key, task_title, output);
    let checkpoint_meta = json!({
        "task_key": task_key,
        "task_title": task_title,
        "source_type": "task_output",
        "facts": checkpoint.facts,
    });
    append_team_v3_blackboard_entry(
        runtime_pool,
        session_id,
        Some(task_id),
        Some(member_id),
        "checkpoint",
        checkpoint.content.as_str(),
        Some(&checkpoint_meta),
    )
    .await?;
    if let Some(structured) =
        build_structured_fact_payload(task_key, task_title, &checkpoint.facts, output)
    {
        let mut structured_meta = json!({
            "task_key": task_key,
            "task_title": task_title,
            "source_type": "task_output",
            "facts": structured.facts,
            "depends_on_task_ids": dependency_task_ids,
            "depends_on_task_keys": dependency_task_keys,
            "tags": structured.tags,
        });
        if let Some(artifact) = artifact_ref {
            if let Some(obj) = structured_meta.as_object_mut() {
                obj.insert(
                    "artifact".to_string(),
                    json!({
                        "path": artifact.path.as_str(),
                        "bytes": artifact.bytes,
                        "host_path": artifact.host_path.as_deref(),
                        "container_path": artifact.container_path.as_deref(),
                    }),
                );
            }
        }
        append_team_v3_blackboard_entry(
            runtime_pool,
            session_id,
            Some(task_id),
            Some(member_id),
            "structured_fact",
            structured.content.as_str(),
            Some(&structured_meta),
        )
        .await?;
    }
    Ok(())
}

fn is_team_v3_summary_task(task: &TeamV3Task) -> bool {
    let key = task.task_key.to_lowercase();
    let title = task.title.to_lowercase();
    let instruction = task.instruction.to_lowercase();
    key.contains("summary")
        || key.contains("final")
        || title.contains("总结")
        || title.contains("汇总")
        || title.contains("summary")
        || instruction.contains("总结")
        || instruction.contains("汇总")
        || instruction.contains("synthes")
}

async fn replace_team_v3_tasks_with_plan(
    runtime_pool: &DatabasePool,
    session_id: &str,
    plan: &TeamV3ExecutionPlan,
    members: &[String],
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"DELETE FROM team_v3_tasks
                   WHERE session_id = ?"#,
            )
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"DELETE FROM team_v3_tasks
                   WHERE session_id = $1"#,
            )
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }

    let mut seen_task_keys = HashSet::new();
    let mut normalized_keys: Vec<String> = Vec::with_capacity(plan.tasks.len());
    for (index, raw_task) in plan.tasks.iter().enumerate() {
        let mut task_key = normalize_task_key(raw_task.task_key.as_str(), index);
        if seen_task_keys.contains(&task_key) {
            task_key = format!("{}-{}", task_key, index + 1);
        }
        seen_task_keys.insert(task_key.clone());
        normalized_keys.push(task_key);
    }
    let normalized_key_set = normalized_keys.iter().cloned().collect::<HashSet<String>>();

    for (index, raw_task) in plan.tasks.iter().enumerate() {
        let task_key = normalized_keys
            .get(index)
            .cloned()
            .unwrap_or_else(|| normalize_task_key(raw_task.task_key.as_str(), index));
        let owner_agent_id = Some(resolve_member_id(
            raw_task.owner_agent_id.as_deref(),
            members,
            index,
        ));
        let depends_on = raw_task
            .depends_on
            .iter()
            .map(|value| normalize_task_key(value, 0))
            .filter(|value| normalized_key_set.contains(value))
            .collect::<Vec<_>>();
        let metadata = json!({
            "team_generated": true,
            "planned_by": "main_agent",
            "depends_on": depends_on,
            "attempt": 0,
            "max_attempts": 1
        });
        let priority = raw_task.priority.unwrap_or(((index as i32) + 1) * 10);
        match runtime_pool {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"INSERT INTO team_v3_tasks
                       (id, session_id, task_key, title, instruction, status, priority,
                        owner_agent_id, claimed_by_agent_id, claim_expires_at, metadata, created_at, updated_at)
                       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                )
                .bind(Uuid::new_v4().to_string())
                .bind(session_id)
                .bind(task_key)
                .bind(raw_task.title.trim())
                .bind(raw_task.instruction.trim())
                .bind("pending")
                .bind(priority)
                .bind(owner_agent_id)
                .bind(Option::<String>::None)
                .bind(Option::<String>::None)
                .bind(serde_json::to_string(&metadata)?)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO team_v3_tasks
                       (id, session_id, task_key, title, instruction, status, priority,
                        owner_agent_id, claimed_by_agent_id, claim_expires_at, metadata, created_at, updated_at)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::jsonb, $12, $13)"#,
                )
                .bind(Uuid::new_v4().to_string())
                .bind(session_id)
                .bind(task_key)
                .bind(raw_task.title.trim())
                .bind(raw_task.instruction.trim())
                .bind("pending")
                .bind(priority)
                .bind(owner_agent_id)
                .bind(Option::<String>::None)
                .bind(Option::<String>::None)
                .bind(serde_json::to_string(&metadata)?)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
        }
    }

    Ok(())
}

async fn list_team_v3_tasks_internal(
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

async fn set_team_v3_task_execution_state(
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
        enabled: false,
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

fn build_team_v3_task_prompt(
    goal: &str,
    user_input: &str,
    task: &TeamV3Task,
    member_runtime_context: &str,
    dependency_context: &str,
    blackboard_context: &str,
    blackboard_snapshot_revision: i64,
    checkpoint_only: bool,
) -> String {
    let mut context_sections: Vec<String> = Vec::new();
    if !member_runtime_context.trim().is_empty() {
        context_sections.push(format!("成员运行时上下文：\n{}", member_runtime_context));
    }
    if !dependency_context.trim().is_empty() {
        context_sections.push(format!("依赖任务输出：\n{}", dependency_context));
    }
    if !blackboard_context.trim().is_empty() {
        context_sections.push(blackboard_context.to_string());
    }

    if context_sections.is_empty() {
        format!(
            "Team 总目标：{}\n用户输入：{}\nTeam 黑板快照版本：{}\n\n当前子任务：{}\n任务说明：{}\n\n请直接执行该子任务，并输出结构化结论（结论、依据、风险、下一步）。",
            goal,
            user_input,
            blackboard_snapshot_revision,
            task.title,
            task.instruction
        )
    } else {
        if checkpoint_only {
            format!(
                "Team 总目标：{}\n用户输入：{}\nTeam 黑板快照版本：{}\n\n当前子任务：{}\n任务说明：{}\n\n{}\n\n你必须优先基于 Structured Memory（结构化记忆）完成收敛，必要时再参考 Task Output Evidence 与 Artifact 摘要。输出结构化结论（最终结论、关键证据、风险、下一步行动）。",
                goal,
                user_input,
                blackboard_snapshot_revision,
                task.title,
                task.instruction,
                context_sections.join("\n\n")
            )
        } else {
            format!(
                "Team 总目标：{}\n用户输入：{}\nTeam 黑板快照版本：{}\n\n当前子任务：{}\n任务说明：{}\n\n{}\n\n请优先基于 Structured Memory（结构化记忆）继续执行，必要时引用 Task Output Evidence 与 Artifact 摘要，并输出结构化结论（结论、依据、风险、下一步）。",
                goal,
                user_input,
                blackboard_snapshot_revision,
                task.title,
                task.instruction,
                context_sections.join("\n\n")
            )
        }
    }
}

fn select_team_member_for_task(task: &TeamV3Task, members: &[String], index: usize) -> String {
    resolve_member_id(task.owner_agent_id.as_deref(), members, index)
}

fn build_team_v3_planner_prompt(
    goal: &str,
    user_input: &str,
    member_catalog: &[String],
    blackboard_context: &str,
) -> String {
    let mut sections = vec![
        format!("Team 目标：{}", goal),
        format!("用户输入：{}", user_input),
        format!(
            "当前成员（可复用、可重命名、可新增）：\n{}",
            if member_catalog.is_empty() {
                "- 暂无预设成员，请按任务需要自行设计成员".to_string()
            } else {
                member_catalog.join("\n")
            }
        ),
    ];
    if !blackboard_context.trim().is_empty() {
        sections.push(blackboard_context.to_string());
    }
    sections.push(
        "请基于当前目标生成 Team 执行计划，并仅输出合法 JSON 不要输出 Markdown、解释文字或代码块围栏。字段要求：\
        summary: 简短拆解说明；\
        agents: 成员数组（每个成员包含 id/name/responsibility/system_prompt/decision_style/risk_preference/weight）；\
        id: 稳定成员标识，建议使用小写英文和连字符；\
        name: 成员显示名；\
        responsibility/system_prompt/decision_style/risk_preference: 成员角色信息；\
        weight: 成员权重（可选）；\
        tasks: 任务数组；\
        task_key: 英文短键；\
        title: 人类可读标题；\
        instruction: 可直接执行的指令；\
        depends_on: 依赖 task_key 数组；\
        owner_agent_id: 必须引用 agents.id；\
        priority: 数字，越小越先执行。"
            .to_string(),
    );
    sections.join("\n\n")
}

fn build_team_v3_planner_system_prompt(main_agent_id: &str) -> String {
    format!(
        r#"你是 Team 主调度代理 {main_agent_id}，负责把用户目标分解为可执行任务图并定义执行成员。

输出规则：
1) 仅输出 JSON，不要输出 Markdown、解释文字或代码块围栏。
2) JSON 结构必须为：{{"summary":"...","agents":[{{...}}],"tasks":[{{...}}]}}
3) agents 数量 1-8；tasks 数量 2-12。
4) task.owner_agent_id 必须严格使用 agents.id，不得引用未定义成员。
5) 能并行的任务不要相互依赖；必须有至少一个收敛任务依赖关键前置任务。
6) 如果输入已有成员，可复用其 id；也可按任务需要新增成员。

Few-shot 示例 1（双成员并行后收敛）：
输入目标：分析仓库并给出改造建议。
输出：
{{"summary":"双线分析后统一汇总","agents":[
  {{"id":"product-analyst","name":"产品分析师","responsibility":"聚焦用户价值与业务目标","system_prompt":"优先解释用户场景、价值与优先级。","decision_style":"evidence-first","risk_preference":"balanced","weight":1.0}},
  {{"id":"architecture-analyst","name":"架构分析师","responsibility":"评估实现路径与扩展性","system_prompt":"优先分析模块边界、依赖与扩展成本。","decision_style":"structured","risk_preference":"conservative","weight":1.0}}
],"tasks":[
  {{"task_key":"analyze-product","title":"分析产品价值","instruction":"提炼目标用户、核心价值与关键用例。","depends_on":[],"owner_agent_id":"product-analyst","priority":10}},
  {{"task_key":"analyze-architecture","title":"分析技术架构","instruction":"识别架构模式、关键模块与技术风险。","depends_on":[],"owner_agent_id":"architecture-analyst","priority":10}},
  {{"task_key":"deliver-summary","title":"汇总结论","instruction":"合并前置分析并输出行动建议。","depends_on":["analyze-product","analyze-architecture"],"owner_agent_id":"product-analyst","priority":30}}
]}}

Few-shot 示例 2（三成员串行链路）：
输入目标：排查线上故障并给出修复方案。
输出：
{{"summary":"先定位根因，再修复并验证","agents":[
  {{"id":"incident-investigator","name":"故障定位","responsibility":"定位根因与影响范围","system_prompt":"强调证据链完整性。","decision_style":"diagnostic","risk_preference":"balanced","weight":1.0}},
  {{"id":"fix-designer","name":"修复设计","responsibility":"制定修复与回滚策略","system_prompt":"优先最小化风险与变更面。","decision_style":"structured","risk_preference":"conservative","weight":1.0}},
  {{"id":"verifier","name":"验证负责人","responsibility":"设计验证与观测方案","system_prompt":"强调可观测性与验收标准。","decision_style":"checklist","risk_preference":"conservative","weight":1.0}}
],"tasks":[
  {{"task_key":"locate-root-cause","title":"定位根因","instruction":"分析现象、日志与变更定位问题根因。","depends_on":[],"owner_agent_id":"incident-investigator","priority":10}},
  {{"task_key":"propose-fix","title":"制定修复方案","instruction":"基于根因给出可执行修复方案和回滚策略。","depends_on":["locate-root-cause"],"owner_agent_id":"fix-designer","priority":20}},
  {{"task_key":"verify-fix","title":"设计验证步骤","instruction":"制定验证清单和观测指标，确认修复有效。","depends_on":["propose-fix"],"owner_agent_id":"verifier","priority":30}}
]}}"#
    )
}

fn validate_execution_plan(plan: &TeamV3ExecutionPlan) -> bool {
    if plan.tasks.is_empty() || plan.tasks.len() > 12 || plan.agents.len() > 12 {
        return false;
    }
    let mut raw_agent_ids = HashSet::new();
    let mut normalized_agent_ids = HashSet::new();
    for (index, agent) in plan.agents.iter().enumerate() {
        let raw_id = agent.id.trim();
        if raw_id.is_empty() {
            return false;
        }
        if !raw_agent_ids.insert(raw_id.to_string()) {
            return false;
        }
        let normalized_id = normalize_agent_id(raw_id, index);
        if !normalized_agent_ids.insert(normalized_id) {
            return false;
        }
    }

    let mut keys = HashSet::new();
    for (index, task) in plan.tasks.iter().enumerate() {
        let task_key = normalize_task_key(task.task_key.as_str(), index);
        if task_key.trim().is_empty() {
            return false;
        }
        if !keys.insert(task_key) {
            return false;
        }
        if task.title.trim().is_empty() || task.instruction.trim().is_empty() {
            return false;
        }
        if !plan.agents.is_empty() {
            let Some(owner_agent_id) = task
                .owner_agent_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                return false;
            };
            let owner_exists_in_agents = raw_agent_ids.contains(owner_agent_id)
                || normalized_agent_ids.contains(&normalize_agent_id(owner_agent_id, index));
            if !owner_exists_in_agents {
                return false;
            }
        }
    }

    for (index, task) in plan.tasks.iter().enumerate() {
        let task_key = normalize_task_key(task.task_key.as_str(), index);
        for dependency in task.depends_on.iter() {
            let dependency_key = normalize_task_key(dependency.as_str(), 0);
            if dependency_key == task_key || !keys.contains(&dependency_key) {
                return false;
            }
        }
    }
    true
}

async fn generate_team_v3_execution_plan_with_main_agent(
    app_handle: &AppHandle,
    provider_config: &crate::services::AiConfig,
    rig_provider: &str,
    model: &str,
    session_id: &str,
    main_agent_id: &str,
    goal_text: &str,
    user_input: &str,
    member_catalog: &[String],
    blackboard_context: &str,
    cancellation_token: &CancellationToken,
) -> Result<Option<TeamV3ExecutionPlan>> {
    let planner_prompt =
        build_team_v3_planner_prompt(goal_text, user_input, member_catalog, blackboard_context);
    let execution_id = format!("team-v3-planner:{}:{}", session_id, Uuid::new_v4());
    let planner_tool_config = load_team_v3_tool_config(app_handle).await;
    let planner_params = AgentExecuteParams {
        execution_id,
        model: model.to_string(),
        system_prompt: build_team_v3_planner_system_prompt(main_agent_id),
        task: planner_prompt,
        rig_provider: rig_provider.to_string(),
        api_key: provider_config.api_key.clone(),
        api_base: provider_config.api_base.clone(),
        max_iterations: 8,
        timeout_secs: 180,
        tool_config: Some(planner_tool_config),
        enable_tenth_man_rule: false,
        tenth_man_config: None,
        document_attachments: None,
        image_attachments: None,
        persist_messages: false,
        subagent_run_id: None,
        context_policy: Some(ContextPolicy {
            include_working_dir: false,
            include_context_storage: false,
            include_task_mainline: false,
            include_run_state: false,
            include_skill_instructions: false,
            include_stuck_resolution_rule: false,
            ..ContextPolicy::default()
        }),
        recursion_depth: 0,
    };
    let planner_output = tokio::select! {
        _ = cancellation_token.cancelled() => Err(anyhow!("Team execution cancelled")),
        result = execute_team_agent(app_handle, planner_params) => result,
    }?;
    let Some(plan) = parse_execution_plan(&planner_output) else {
        let preview = truncate_chars(collapse_whitespace(planner_output.as_str()).as_str(), 320);
        tracing::warn!(
            "Team V3 planner output is not valid JSON-only payload (session={}): {}",
            session_id,
            preview
        );
        return Ok(None);
    };
    if !validate_execution_plan(&plan) {
        return Ok(None);
    }
    Ok(Some(plan))
}

async fn prepare_team_v3_execution_tasks_with_main_agent(
    runtime_pool: &DatabasePool,
    app_handle: &AppHandle,
    session_id: &str,
    goal_text: &str,
    user_input: &str,
    state_data: &Value,
    provider_config: &crate::services::AiConfig,
    rig_provider: &str,
    model: &str,
    cancellation_token: &CancellationToken,
) -> Result<(Vec<String>, Value)> {
    let members = team_member_ids(state_data);
    let main_agent_id = members
        .first()
        .cloned()
        .unwrap_or_else(|| "agent-1".to_string());
    let goal_meta = json!({ "goal": goal_text });
    append_team_v3_blackboard_entry(
        runtime_pool,
        session_id,
        None,
        Some("human"),
        "goal",
        user_input,
        Some(&goal_meta),
    )
    .await?;
    let backfilled = backfill_team_v3_structured_memory_from_checkpoints(
        runtime_pool,
        session_id,
        TEAM_V3_STRUCTURED_BACKFILL_SCAN_LIMIT,
    )
    .await?;
    if backfilled > 0 {
        tracing::info!(
            "Team V3 backfilled structured memory from historical checkpoints: session={} appended={}",
            session_id,
            backfilled
        );
    }
    let historical_board = list_team_v3_blackboard_entries(runtime_pool, session_id, 200).await?;
    let planner_query = TeamV3PromptQuery::for_planner(goal_text, user_input);
    let blackboard_context_result = build_blackboard_context(&historical_board, &planner_query);
    tracing::info!(
        "Team V3 planner blackboard context: session={} mode={} terms={} chars={} structured={}/{} task_outputs={}/{} artifacts={}/{} events={}/{} checkpoints={}/{} dropped_total={}",
        session_id,
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
    let member_catalog = team_member_catalog_lines(state_data);

    let plan_opt = generate_team_v3_execution_plan_with_main_agent(
        app_handle,
        provider_config,
        rig_provider,
        model,
        session_id,
        main_agent_id.as_str(),
        goal_text,
        user_input,
        &member_catalog,
        blackboard_context.as_str(),
        cancellation_token,
    )
    .await?;

    if let Some(plan) = plan_opt {
        let planned_members = derive_plan_members(&plan);
        let lead_member_id = planned_members
            .first()
            .and_then(|member| member.get("id"))
            .and_then(|value| value.as_str())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| main_agent_id.clone());
        let next_state_data = build_team_state_data_with_members(
            Some(state_data),
            planned_members,
            Some(lead_member_id.as_str()),
        );
        let now = Utc::now().to_rfc3339();
        set_team_v3_session_state_data(runtime_pool, session_id, &next_state_data, &now).await?;
        let member_ids = team_member_ids(&next_state_data);
        replace_team_v3_tasks_with_plan(runtime_pool, session_id, &plan, &member_ids).await?;
        let summary = plan
            .summary
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("主 agent 已完成任务拆解。");
        let plan_meta = json!({
            "task_count": plan.tasks.len(),
            "tasks": plan.tasks.iter().map(|task| task.task_key.clone()).collect::<Vec<_>>()
        });
        append_team_v3_blackboard_entry(
            runtime_pool,
            session_id,
            None,
            Some(lead_member_id.as_str()),
            "plan",
            summary,
            Some(&plan_meta),
        )
        .await?;
        return Ok((member_ids, next_state_data));
    }

    let fallback_state_data = build_team_state_data(Some(state_data), Some(main_agent_id.as_str()));
    let now = Utc::now().to_rfc3339();
    set_team_v3_session_state_data(runtime_pool, session_id, &fallback_state_data, &now).await?;
    ensure_team_v3_execution_tasks(
        runtime_pool,
        session_id,
        Some(user_input),
        &fallback_state_data,
    )
    .await?;
    append_team_v3_status_message(
        runtime_pool,
        session_id,
        "主 agent 拆解失败，已回退到默认任务拆解。",
    )
    .await?;
    append_team_v3_blackboard_entry(
        runtime_pool,
        session_id,
        None,
        Some(main_agent_id.as_str()),
        "plan_fallback",
        "主 agent 拆解失败，使用默认任务图。",
        None,
    )
    .await?;
    Ok((team_member_ids(&fallback_state_data), fallback_state_data))
}

async fn run_team_v3_execution_orchestrator(
    runtime_pool: DatabasePool,
    app_handle: AppHandle,
    ai_manager: Arc<AiServiceManager>,
    session_id: String,
    generation: u64,
    cancellation_token: CancellationToken,
    goal_text: String,
    user_input: String,
    state_data: Value,
    rag_enabled: bool,
) -> Result<String> {
    let provider_config = resolve_team_v3_provider_config(ai_manager.as_ref()).await?;
    let team_tool_config = load_team_v3_tool_config(&app_handle).await;
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

        let mut join_set = JoinSet::new();
        for (index, (task, dependencies, dependency_task_keys)) in ready_tasks.into_iter().enumerate() {
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
            let task_query =
                TeamV3PromptQuery::for_task(
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

            let app_handle = app_handle.clone();
            let provider_config = provider_config.clone();
            let rig_provider = rig_provider.clone();
            let model = model.clone();
            let system_prompt = member_system_prompt.clone();
            let cancel_token = cancellation_token.clone();
            let session_id_for_task = session_id.clone();
            let member_id_for_task = member_id.clone();
            let task_id = task.id.clone();
            let task_key = task.task_key.clone();
            let task_title = task.title.clone();
            let dependency_task_ids_for_task = dependencies.clone();
            let dependency_task_keys_for_task = dependency_task_keys.clone();
            let task_key_for_timeout = task.task_key.clone();
            let is_summary_task_for_task = is_summary_task;
            let rag_enabled_for_task = rag_enabled;
            let task_tool_config = team_tool_config.clone();
            join_set.spawn(async move {
                let execution_id = format!(
                    "team-v3:{}:{}:{}:{}",
                    session_id_for_task,
                    task_id,
                    member_id_for_task,
                    Uuid::new_v4()
                );
                let executor_params = AgentExecuteParams {
                    execution_id,
                    model,
                    system_prompt,
                    task: prompt,
                    rig_provider,
                    api_key: provider_config.api_key.clone(),
                    api_base: provider_config.api_base.clone(),
                    max_iterations,
                    timeout_secs,
                    tool_config: Some(task_tool_config),
                    enable_tenth_man_rule: false,
                    tenth_man_config: None,
                    document_attachments: None,
                    image_attachments: None,
                    persist_messages: false,
                    subagent_run_id: None,
                    context_policy: Some(ContextPolicy {
                        include_working_dir: true,
                        include_context_storage: true,
                        include_task_mainline: false,
                        include_run_state: true,
                        include_document_attachments: false,
                        include_skill_instructions: true,
                        feature_context_packet_v2: rag_enabled_for_task,
                        ..ContextPolicy::default()
                    }),
                    recursion_depth: 0,
                };
                let execution_result = tokio::select! {
                    _ = cancel_token.cancelled() => Err(anyhow!("Team execution cancelled")),
                    result = tokio::time::timeout(
                        std::time::Duration::from_secs(timeout_secs),
                        execute_team_agent(&app_handle, executor_params),
                    ) => match result {
                        Ok(inner) => inner,
                        Err(_) => Err(anyhow!(
                            "Team task '{}' timed out after {} seconds",
                            task_key_for_timeout,
                            timeout_secs,
                        )),
                    },
                };
                (
                    task_id,
                    task_key,
                    task_title,
                    dependency_task_ids_for_task,
                    dependency_task_keys_for_task,
                    member_id_for_task,
                    is_summary_task_for_task,
                    execution_result,
                )
            });
        }

        let mut wave_error: Option<String> = None;
        while let Some(joined) = join_set.join_next().await {
            let (
                task_id,
                task_key,
                task_title,
                dependency_task_ids,
                dependency_task_keys,
                member_id,
                is_summary_task,
                execution_result,
            ) =
                match joined {
                    Ok(result) => result,
                    Err(e) => {
                        if wave_error.is_none() {
                            wave_error = Some(format!("Team 并行执行任务失败: {}", e));
                        }
                        continue;
                    }
                };

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

        if let Some(error) = wave_error {
            return Err(anyhow!(error));
        }
    }

    Ok(summary.unwrap_or_else(|| "Team 执行完成。".to_string()))
}

#[tauri::command]
pub async fn team_v3_ensure_schema(db: DbState<'_>) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn team_v3_reset_schema(db: DbState<'_>) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    match &runtime_pool {
        DatabasePool::SQLite(pool) => reset_schema_sqlite(pool).await.map_err(|e| e.to_string()),
        DatabasePool::PostgreSQL(pool) => reset_schema_pg(pool).await.map_err(|e| e.to_string()),
        DatabasePool::MySQL(_) => Err("Team V3 does not support MySQL".to_string()),
    }
}

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
) -> Result<(), String> {
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
    Ok(())
}

#[tauri::command]
pub async fn team_v3_release_task_claim(
    db: DbState<'_>,
    session_id: String,
    task_id: String,
    agent_id: String,
) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    match &runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
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
            sqlx::query(
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
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(
            payload
                .facts
                .get("risk")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .contains("风险:")
        );
        assert!(
            payload
                .facts
                .get("next_step")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .contains("下一步:")
        );
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
        assert!(structured.is_some(), "meaningful checkpoint should be backfilled");
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

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use crate::agents::ContextPolicy;
use crate::agents::executor::{execute_agent as execute_team_agent, AgentExecuteParams};
use crate::agents::tool_router::{ToolConfig, ToolSelectionStrategy};
use crate::services::ai::AiServiceManager;
use anyhow::{anyhow, Result};
use chrono::Utc;
use sentinel_db::{database_service::connection_manager::DatabasePool, DatabaseService};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use tauri::{AppHandle, State};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

type DbState<'r> = State<'r, Arc<DatabaseService>>;
type AiState<'r> = State<'r, Arc<AiServiceManager>>;

static TEAM_EXECUTION_CANCELLATIONS: LazyLock<Mutex<HashMap<String, (u64, CancellationToken)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static TEAM_EXECUTION_GENERATION: AtomicU64 = AtomicU64::new(0);

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
            }
        }
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

async fn reset_schema_pg(pool: &sqlx::PgPool) -> Result<()> {
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

async fn ensure_schema_pg(pool: &sqlx::PgPool) -> Result<()> {
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
    vec![
        json!({
            "id": "agent-1",
            "name": "Team Agent 1",
            "responsibility": "负责执行分配子任务并沉淀可复用结论",
            "sort_order": 0,
            "weight": 1.0,
            "token_usage": 0,
            "tool_calls_count": 0,
            "is_active": false
        }),
        json!({
            "id": "agent-2",
            "name": "Team Agent 2",
            "responsibility": "负责执行分配子任务并沉淀可复用结论",
            "sort_order": 1,
            "weight": 1.0,
            "token_usage": 0,
            "tool_calls_count": 0,
            "is_active": false
        }),
        json!({
            "id": "agent-3",
            "name": "Team Agent 3",
            "responsibility": "负责执行分配子任务并沉淀可复用结论",
            "sort_order": 2,
            "weight": 1.0,
            "token_usage": 0,
            "tool_calls_count": 0,
            "is_active": false
        }),
    ]
}

fn normalize_team_members(state_data: &Value) -> Vec<Value> {
    let mut members = state_data
        .get("members")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

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

fn build_team_state_data(current: Option<&Value>, active_member_id: Option<&str>) -> Value {
    let mut state_data = current.cloned().unwrap_or_else(|| json!({}));
    if !state_data.is_object() {
        state_data = json!({});
    }

    let mut members = normalize_team_members(&state_data);
    for member in members.iter_mut() {
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
        state_obj.insert("members".to_string(), Value::Array(members));
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
    normalize_team_members(state_data)
        .into_iter()
        .map(|member| {
            let id = member
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("member");
            let name = member
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(id);
            let responsibility = member
                .get("responsibility")
                .and_then(|v| v.as_str())
                .unwrap_or("负责通用问题求解");
            format!("- {} ({})：{}", id, name, responsibility)
        })
        .collect()
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

    let fence_start = trimmed.find("```json").or_else(|| trimmed.find("```"))?;
    let after_start = &trimmed[fence_start..];
    let first_line_end = after_start.find('\n')?;
    let body = &after_start[first_line_end + 1..];
    let fence_end = body.find("```")?;
    Some(body[..fence_end].trim().to_string())
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
    summary: Option<&str>,
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

    let status_text = if success {
        let prefix = "Team 执行完成。";
        if let Some(s) = summary.map(str::trim).filter(|s| !s.is_empty()) {
            format!("{} {}", prefix, s)
        } else {
            prefix.to_string()
        }
    } else {
        let prefix = "Team 执行失败。";
        if let Some(s) = summary.map(str::trim).filter(|s| !s.is_empty()) {
            format!("{} {}", prefix, s)
        } else {
            prefix.to_string()
        }
    };
    append_team_v3_status_message(runtime_pool, session_id, &status_text).await?;
    Ok(())
}

async fn get_team_v3_latest_human_message_preview(
    runtime_pool: &DatabasePool,
    session_id: &str,
) -> Result<Option<String>> {
    let payload_opt: Option<String> = match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT payload FROM team_v3_messages
                   WHERE session_id = ?
                   ORDER BY created_at DESC
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
                   WHERE session_id = $1
                   ORDER BY created_at DESC
                   LIMIT 1"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            row.map(|r| r.get("payload"))
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    };

    let preview = payload_opt
        .and_then(|payload| serde_json::from_str::<Value>(&payload).ok())
        .and_then(|value| {
            value
                .get("content")
                .and_then(|c| c.as_str())
                .map(|s| s.trim().to_string())
        })
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut chars = s.chars();
            let short: String = chars.by_ref().take(140).collect();
            if chars.next().is_some() {
                format!("{}...", short)
            } else {
                short
            }
        });

    Ok(preview)
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
                   ORDER BY created_at DESC
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
                   ORDER BY created_at DESC
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

async fn clear_team_v3_blackboard_entries(runtime_pool: &DatabasePool, session_id: &str) -> Result<()> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"DELETE FROM team_v3_blackboard_entries
                   WHERE session_id = ?"#,
            )
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"DELETE FROM team_v3_blackboard_entries
                   WHERE session_id = $1"#,
            )
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }
    Ok(())
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
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let metadata_text = serde_json::to_string(metadata.unwrap_or(&json!({})))?;
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

fn build_blackboard_context(entries: &[TeamV3BlackboardEntry]) -> String {
    let mut lines = entries
        .iter()
        .rev()
        .map(|entry| {
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
            format!(
                "[{}][agent={}][task={}] {}",
                entry.entry_type, agent, task, entry.content
            )
        })
        .collect::<Vec<_>>();
    if lines.is_empty() {
        String::new()
    } else {
        lines.insert(0, "Team 白板（最新共享信息）:".to_string());
        lines.join("\n")
    }
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
    let normalized_key_set = normalized_keys
        .iter()
        .cloned()
        .collect::<HashSet<String>>();

    for (index, raw_task) in plan.tasks.iter().enumerate() {
        let task_key = normalized_keys
            .get(index)
            .cloned()
            .unwrap_or_else(|| normalize_task_key(raw_task.task_key.as_str(), index));
        let owner_agent_id = raw_task
            .owner_agent_id
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty() && members.contains(value))
            .or_else(|| members.get(index % members.len().max(1)).cloned());
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

fn build_team_v3_task_prompt(
    goal: &str,
    user_input: &str,
    task: &TeamV3Task,
    dependency_context: &str,
    blackboard_context: &str,
) -> String {
    if dependency_context.trim().is_empty() && blackboard_context.trim().is_empty() {
        format!(
            "Team 总目标：{}\n用户输入：{}\n\n当前子任务：{}\n任务说明：{}\n\n请直接执行该子任务，并输出结构化结论（结论、依据、风险、下一步）。",
            goal,
            user_input,
            task.title,
            task.instruction
        )
    } else {
        let mut context_sections: Vec<String> = Vec::new();
        if !dependency_context.trim().is_empty() {
            context_sections.push(format!("依赖任务输出：\n{}", dependency_context));
        }
        if !blackboard_context.trim().is_empty() {
            context_sections.push(blackboard_context.to_string());
        }
        format!(
            "Team 总目标：{}\n用户输入：{}\n\n当前子任务：{}\n任务说明：{}\n\n{}\n\n请基于共享信息继续执行，并输出结构化结论（结论、依据、风险、下一步）。",
            goal,
            user_input,
            task.title,
            task.instruction,
            context_sections.join("\n\n")
        )
    }
}

fn select_team_member_for_task(task: &TeamV3Task, members: &[String], index: usize) -> String {
    if let Some(owner) = task
        .owner_agent_id
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return owner;
    }
    members
        .get(index % members.len().max(1))
        .cloned()
        .unwrap_or_else(|| "agent-1".to_string())
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
            "可分配成员：\n{}",
            if member_catalog.is_empty() {
                "- agent-1 (Team Agent 1)：通用任务执行".to_string()
            } else {
                member_catalog.join("\n")
            }
        ),
    ];
    if !blackboard_context.trim().is_empty() {
        sections.push(blackboard_context.to_string());
    }
    sections.push(
        "请基于当前目标拆解任务，并仅输出合法 JSON。字段要求：\
        summary: 简短拆解说明；\
        tasks: 任务数组；\
        task_key: 英文短键；\
        title: 人类可读标题；\
        instruction: 可直接执行的指令；\
        depends_on: 依赖 task_key 数组；\
        owner_agent_id: 必须来自可分配成员；\
        priority: 数字，越小越先执行。"
            .to_string(),
    );
    sections.join("\n\n")
}

fn build_team_v3_planner_system_prompt(main_agent_id: &str) -> String {
    format!(
        r#"你是 Team 主调度代理 {main_agent_id}，负责把用户目标分解为可执行任务图。

输出规则：
1) 仅输出 JSON，不要输出 Markdown、解释文字或代码块围栏。
2) JSON 结构必须为：{{"summary":"...","tasks":[{{...}}]}}
3) tasks 数量 2-8。
4) 能并行的任务不要相互依赖；最终必须有收敛任务（如 deliver-summary）依赖关键前置任务。
5) owner_agent_id 只能使用输入中提供的可分配成员 id，不得臆造固定角色名。

Few-shot 示例 1（并行后收敛）：
输入目标：分析仓库并给出改造建议。
输出：
{{"summary":"先并行收集信息，再统一产出建议","tasks":[
  {{"task_key":"collect-architecture","title":"收集架构信息","instruction":"阅读核心目录并提炼模块边界与职责。","depends_on":[],"owner_agent_id":"agent-1","priority":10}},
  {{"task_key":"collect-risks","title":"收集风险与缺陷","instruction":"识别潜在风险点与实现缺口。","depends_on":[],"owner_agent_id":"agent-2","priority":10}},
  {{"task_key":"deliver-summary","title":"汇总结论","instruction":"整合前置发现，输出分级建议和下一步计划。","depends_on":["collect-architecture","collect-risks"],"owner_agent_id":"agent-3","priority":30}}
]}}

Few-shot 示例 2（串行链路）：
输入目标：排查线上故障并给出修复方案。
输出：
{{"summary":"先定位问题，再给出修复和验证步骤","tasks":[
  {{"task_key":"locate-root-cause","title":"定位根因","instruction":"分析现象、日志与变更定位问题根因。","depends_on":[],"owner_agent_id":"agent-1","priority":10}},
  {{"task_key":"propose-fix","title":"制定修复方案","instruction":"基于根因给出可执行修复方案和回滚策略。","depends_on":["locate-root-cause"],"owner_agent_id":"agent-2","priority":20}},
  {{"task_key":"verify-fix","title":"设计验证步骤","instruction":"制定验证清单和观测指标，确认修复有效。","depends_on":["propose-fix"],"owner_agent_id":"agent-3","priority":30}}
]}}"#
    )
}

fn validate_execution_plan(plan: &TeamV3ExecutionPlan) -> bool {
    if plan.tasks.is_empty() || plan.tasks.len() > 12 {
        return false;
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
    let planner_prompt = build_team_v3_planner_prompt(
        goal_text,
        user_input,
        member_catalog,
        blackboard_context,
    );
    let execution_id = format!("team-v3-planner:{}:{}", session_id, Uuid::new_v4());
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
        tool_config: Some(ToolConfig {
            selection_strategy: ToolSelectionStrategy::None,
            max_tools: 0,
            fixed_tools: Vec::new(),
            disabled_tools: Vec::new(),
            allowed_tools: Vec::new(),
            enabled: false,
        }),
        enable_tenth_man_rule: false,
        tenth_man_config: None,
        document_attachments: None,
        image_attachments: None,
        persist_messages: false,
        subagent_run_id: None,
        context_policy: None,
        recursion_depth: 0,
        audit_mode: false,
        audit_verification_level: None,
    };
    let planner_output = tokio::select! {
        _ = cancellation_token.cancelled() => Err(anyhow!("Team execution cancelled")),
        result = execute_team_agent(app_handle, planner_params) => result,
    }?;
    let Some(plan) = parse_execution_plan(&planner_output) else {
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
) -> Result<()> {
    let members = team_member_ids(state_data);
    let main_agent_id = members
        .first()
        .cloned()
        .unwrap_or_else(|| "agent-1".to_string());
    clear_team_v3_blackboard_entries(runtime_pool, session_id).await?;
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
    let historical_board = list_team_v3_blackboard_entries(runtime_pool, session_id, 20).await?;
    let blackboard_context = build_blackboard_context(&historical_board);
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
        replace_team_v3_tasks_with_plan(runtime_pool, session_id, &plan, &members).await?;
        let summary = plan
            .summary
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("主 agent 已完成任务拆解。");
        let planner_status = format!("主 agent 已拆解任务，共 {} 项。{}", plan.tasks.len(), summary);
        append_team_v3_status_message(runtime_pool, session_id, planner_status.as_str()).await?;
        let plan_meta = json!({
            "task_count": plan.tasks.len(),
            "tasks": plan.tasks.iter().map(|task| task.task_key.clone()).collect::<Vec<_>>()
        });
        append_team_v3_blackboard_entry(
            runtime_pool,
            session_id,
            None,
            Some(main_agent_id.as_str()),
            "plan",
            summary,
            Some(&plan_meta),
        )
        .await?;
        return Ok(());
    }

    ensure_team_v3_execution_tasks(runtime_pool, session_id, Some(user_input), state_data).await?;
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
    Ok(())
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
    let rig_provider = provider_config
        .rig_provider
        .clone()
        .unwrap_or_else(|| provider_config.provider.clone());
    let model = provider_config.model.clone();
    let max_iterations = provider_config.max_turns.unwrap_or(24).max(8) as usize;
    let timeout_secs = 1800_u64;
    let members = team_member_ids(&state_data);
    let mut output_by_task_id: HashMap<String, String> = HashMap::new();
    let mut summary: Option<String> = None;
    prepare_team_v3_execution_tasks_with_main_agent(
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
        for task in tasks.iter() {
            status_by_id.insert(task.id.clone(), task.status.clone());
            id_by_task_key.insert(task.task_key.clone(), task.id.clone());
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

        let mut ready_tasks: Vec<(TeamV3Task, Vec<String>)> = Vec::new();
        for task in runnable_tasks {
            let dependencies = parse_task_dependencies(&task.metadata)
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
                ready_tasks.push((task, dependencies));
            }
        }

        if ready_tasks.is_empty() {
            return Err(anyhow!("Team 任务存在循环依赖或未满足依赖，无法继续执行"));
        }

        let mut join_set = JoinSet::new();
        for (index, (task, dependencies)) in ready_tasks.into_iter().enumerate() {
            let member_id = select_team_member_for_task(&task, &members, index);
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

            let dependency_context = dependencies
                .iter()
                .filter_map(|dependency_id| {
                    output_by_task_id
                        .get(dependency_id)
                        .map(|output| format!("依赖任务 {} 输出：\n{}", dependency_id, output))
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            let blackboard_entries =
                list_team_v3_blackboard_entries(&runtime_pool, &session_id, 32).await?;
            let blackboard_context = build_blackboard_context(&blackboard_entries);
            let prompt = build_team_v3_task_prompt(
                goal_text.as_str(),
                user_input.as_str(),
                &task,
                dependency_context.as_str(),
                blackboard_context.as_str(),
            );

            let app_handle = app_handle.clone();
            let provider_config = provider_config.clone();
            let rig_provider = rig_provider.clone();
            let model = model.clone();
            let cancel_token = cancellation_token.clone();
            let session_id_for_task = session_id.clone();
            let member_id_for_task = member_id.clone();
            let task_id = task.id.clone();
            let task_key = task.task_key.clone();
            let task_key_for_timeout = task.task_key.clone();
            let rag_enabled_for_task = rag_enabled;
            join_set.spawn(async move {
                let execution_id = format!(
                    "team-v3:{}:{}:{}",
                    session_id_for_task,
                    task_id,
                    Uuid::new_v4()
                );
                let executor_params = AgentExecuteParams {
                    execution_id,
                    model,
                    system_prompt: format!(
                        "你是 Team 成员 {}，请以严谨、可执行的方式完成分配任务。",
                        member_id_for_task
                    ),
                    task: prompt,
                    rig_provider,
                    api_key: provider_config.api_key.clone(),
                    api_base: provider_config.api_base.clone(),
                    max_iterations,
                    timeout_secs,
                    tool_config: Some(ToolConfig {
                        selection_strategy: ToolSelectionStrategy::Keyword,
                        max_tools: 8,
                        fixed_tools: Vec::new(),
                        disabled_tools: Vec::new(),
                        allowed_tools: Vec::new(),
                        enabled: true,
                    }),
                    enable_tenth_man_rule: false,
                    tenth_man_config: None,
                    document_attachments: None,
                    image_attachments: None,
                    persist_messages: false,
                    subagent_run_id: None,
                    context_policy: Some(ContextPolicy {
                        feature_context_packet_v2: rag_enabled_for_task,
                        ..ContextPolicy::default()
                    }),
                    recursion_depth: 0,
                    audit_mode: false,
                    audit_verification_level: None,
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
                (task_id, task_key, member_id_for_task, execution_result)
            });
        }

        let mut wave_error: Option<String> = None;
        while let Some(joined) = join_set.join_next().await {
            let (task_id, task_key, member_id, execution_result) = match joined {
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
                    if task_key == "deliver-summary" || summary.is_none() {
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

    let preview = get_team_v3_latest_human_message_preview(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())?;
    let status_text = match preview.clone() {
        Some(text) => format!("已触发 Team 执行：{}", text),
        None => "已触发 Team 执行。".to_string(),
    };
    if let Err(e) = append_team_v3_status_message(&runtime_pool, &session_id, &status_text).await {
        tracing::warn!(
            "team_v3_start_execution: failed to append start message for session {}: {}",
            session_id,
            e
        );
    }

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
                r#"SELECT id, session_id, thread_id, from_agent_id, to_agent_id, message_type, payload, created_at
                   FROM team_v3_messages
                   WHERE session_id = ? AND thread_id = ?
                   ORDER BY created_at ASC"#,
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
                    })
                })
                .collect()
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, session_id, thread_id, from_agent_id, to_agent_id, message_type,
                          payload::text as payload, created_at::text as created_at
                   FROM team_v3_messages
                   WHERE session_id = $1 AND thread_id = $2
                   ORDER BY created_at ASC"#,
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
                r#"SELECT id, session_id, thread_id, from_agent_id, to_agent_id, message_type, payload, created_at
                   FROM team_v3_messages
                   WHERE session_id = ?
                   ORDER BY created_at ASC"#,
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
                    }
                })
                .collect())
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT id, session_id, thread_id, from_agent_id, to_agent_id, message_type,
                          payload::text as payload, created_at::text as created_at
                   FROM team_v3_messages
                   WHERE session_id = $1
                   ORDER BY created_at ASC"#,
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

use std::sync::Arc;

use anyhow::{anyhow, Result};
use sentinel_db::{database_service::connection_manager::DatabasePool, DatabaseService};
use tauri::State;

pub(crate) async fn reset_schema_sqlite(pool: &sqlx::SqlitePool) -> Result<()> {
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

pub(crate) async fn reset_schema_pg(pool: &sentinel_db::sqlx_compat::PgPool) -> Result<()> {
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

pub(crate) async fn ensure_schema_sqlite(pool: &sqlx::SqlitePool) -> Result<()> {
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

pub(crate) async fn ensure_schema_pg(pool: &sentinel_db::sqlx_compat::PgPool) -> Result<()> {
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

pub(crate) async fn ensure_team_v3_schema(runtime_pool: &DatabasePool) -> Result<()> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => ensure_schema_sqlite(pool).await,
        DatabasePool::PostgreSQL(pool) => ensure_schema_pg(pool).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Team V3 does not support MySQL")),
    }
}

#[tauri::command]
pub async fn team_v3_ensure_schema(db: State<'_, Arc<DatabaseService>>) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v3_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn team_v3_reset_schema(db: State<'_, Arc<DatabaseService>>) -> Result<(), String> {
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

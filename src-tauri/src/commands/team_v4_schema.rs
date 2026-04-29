use std::sync::Arc;

use anyhow::{anyhow, Result};
use sentinel_db::{database_service::connection_manager::DatabasePool, DatabaseService};
use tauri::State;

pub(crate) async fn ensure_team_v4_schema(runtime_pool: &DatabasePool) -> Result<()> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => ensure_schema_sqlite(pool).await,
        DatabasePool::PostgreSQL(pool) => ensure_schema_pg(pool).await,
        DatabasePool::MySQL(_) => Err(anyhow!("Team V4 does not support MySQL")),
    }
}

async fn ensure_schema_sqlite(pool: &sqlx::SqlitePool) -> Result<()> {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS team_v4_runs (
            id TEXT PRIMARY KEY,
            conversation_id TEXT,
            profile_id TEXT,
            goal TEXT NOT NULL,
            state TEXT NOT NULL CHECK (state IN ('draft','planning','running','waiting_human','completed','failed','cancelled','archived')),
            policy_json TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v4_sequences (
            run_id TEXT PRIMARY KEY REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            last_sequence INTEGER NOT NULL DEFAULT 0
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v4_agents (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            profile_id TEXT,
            role_type TEXT NOT NULL CHECK (role_type IN ('commander','solver','observer','harness')),
            name TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('idle','running','waiting','completed','failed','cancelled')),
            model TEXT,
            context_mode TEXT,
            tool_policy_json TEXT NOT NULL DEFAULT '{}',
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v4_tasks (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            parent_task_id TEXT REFERENCES team_v4_tasks(id) ON DELETE SET NULL,
            task_key TEXT NOT NULL,
            title TEXT NOT NULL,
            instruction TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('pending','ready','running','blocked','completed','failed','cancelled')),
            priority INTEGER NOT NULL DEFAULT 100,
            assigned_agent_id TEXT REFERENCES team_v4_agents(id) ON DELETE SET NULL,
            depends_on TEXT NOT NULL DEFAULT '[]',
            acceptance_criteria TEXT,
            context_snapshot_id TEXT,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(run_id, task_key)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v4_events (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            sequence INTEGER NOT NULL,
            actor_id TEXT,
            task_id TEXT,
            event_type TEXT NOT NULL,
            visibility TEXT NOT NULL CHECK (visibility IN ('user','workspace','internal')),
            payload TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(run_id, sequence)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v4_context_snapshots (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            actor_id TEXT REFERENCES team_v4_agents(id) ON DELETE SET NULL,
            task_id TEXT REFERENCES team_v4_tasks(id) ON DELETE SET NULL,
            role_type TEXT NOT NULL,
            source_sequence INTEGER NOT NULL DEFAULT 0,
            policy_json TEXT NOT NULL DEFAULT '{}',
            sections_json TEXT NOT NULL DEFAULT '[]',
            token_estimate INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v4_memories (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            task_id TEXT REFERENCES team_v4_tasks(id) ON DELETE SET NULL,
            kind TEXT NOT NULL CHECK (kind IN ('evidence','decision','risk','blocker','checkpoint','artifact_summary')),
            content TEXT NOT NULL,
            confidence REAL NOT NULL DEFAULT 0,
            source_event_ids TEXT NOT NULL DEFAULT '[]',
            accepted_by_commander BOOLEAN NOT NULL DEFAULT FALSE,
            promoted_to_long_term BOOLEAN NOT NULL DEFAULT FALSE,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v4_harness_runs (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            actor_id TEXT REFERENCES team_v4_agents(id) ON DELETE SET NULL,
            task_id TEXT REFERENCES team_v4_tasks(id) ON DELETE SET NULL,
            status TEXT NOT NULL CHECK (status IN ('queued','running','paused','completed','failed','cancelled')),
            lease_expires_at DATETIME,
            last_heartbeat_at DATETIME,
            checkpoint_sequence INTEGER NOT NULL DEFAULT 0,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_team_v4_runs_conversation ON team_v4_runs(conversation_id, updated_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_events_run_sequence ON team_v4_events(run_id, sequence)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_tasks_run_status_priority ON team_v4_tasks(run_id, status, priority, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_agents_run_role ON team_v4_agents(run_id, role_type, status)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_memories_run_kind ON team_v4_memories(run_id, kind, accepted_by_commander)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_harness_run_status ON team_v4_harness_runs(run_id, status)",
    ];

    for sql in ddl {
        sqlx::query(sql).execute(pool).await?;
    }
    Ok(())
}

async fn ensure_schema_pg(pool: &sentinel_db::sqlx_compat::PgPool) -> Result<()> {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS team_v4_runs (
            id TEXT PRIMARY KEY,
            conversation_id TEXT,
            profile_id TEXT,
            goal TEXT NOT NULL,
            state TEXT NOT NULL CHECK (state IN ('draft','planning','running','waiting_human','completed','failed','cancelled','archived')),
            policy_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v4_sequences (
            run_id TEXT PRIMARY KEY REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            last_sequence BIGINT NOT NULL DEFAULT 0
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v4_agents (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            profile_id TEXT,
            role_type TEXT NOT NULL CHECK (role_type IN ('commander','solver','observer','harness')),
            name TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('idle','running','waiting','completed','failed','cancelled')),
            model TEXT,
            context_mode TEXT,
            tool_policy_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v4_tasks (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            parent_task_id TEXT REFERENCES team_v4_tasks(id) ON DELETE SET NULL,
            task_key TEXT NOT NULL,
            title TEXT NOT NULL,
            instruction TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('pending','ready','running','blocked','completed','failed','cancelled')),
            priority INTEGER NOT NULL DEFAULT 100,
            assigned_agent_id TEXT REFERENCES team_v4_agents(id) ON DELETE SET NULL,
            depends_on JSONB NOT NULL DEFAULT '[]'::jsonb,
            acceptance_criteria TEXT,
            context_snapshot_id TEXT,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(run_id, task_key)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v4_events (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            sequence BIGINT NOT NULL,
            actor_id TEXT,
            task_id TEXT,
            event_type TEXT NOT NULL,
            visibility TEXT NOT NULL CHECK (visibility IN ('user','workspace','internal')),
            payload JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(run_id, sequence)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v4_context_snapshots (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            actor_id TEXT REFERENCES team_v4_agents(id) ON DELETE SET NULL,
            task_id TEXT REFERENCES team_v4_tasks(id) ON DELETE SET NULL,
            role_type TEXT NOT NULL,
            source_sequence BIGINT NOT NULL DEFAULT 0,
            policy_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            sections_json JSONB NOT NULL DEFAULT '[]'::jsonb,
            token_estimate INTEGER NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v4_memories (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            task_id TEXT REFERENCES team_v4_tasks(id) ON DELETE SET NULL,
            kind TEXT NOT NULL CHECK (kind IN ('evidence','decision','risk','blocker','checkpoint','artifact_summary')),
            content TEXT NOT NULL,
            confidence DOUBLE PRECISION NOT NULL DEFAULT 0,
            source_event_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
            accepted_by_commander BOOLEAN NOT NULL DEFAULT FALSE,
            promoted_to_long_term BOOLEAN NOT NULL DEFAULT FALSE,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS team_v4_harness_runs (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            actor_id TEXT REFERENCES team_v4_agents(id) ON DELETE SET NULL,
            task_id TEXT REFERENCES team_v4_tasks(id) ON DELETE SET NULL,
            status TEXT NOT NULL CHECK (status IN ('queued','running','paused','completed','failed','cancelled')),
            lease_expires_at TIMESTAMPTZ,
            last_heartbeat_at TIMESTAMPTZ,
            checkpoint_sequence BIGINT NOT NULL DEFAULT 0,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_team_v4_runs_conversation ON team_v4_runs(conversation_id, updated_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_events_run_sequence ON team_v4_events(run_id, sequence)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_tasks_run_status_priority ON team_v4_tasks(run_id, status, priority, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_agents_run_role ON team_v4_agents(run_id, role_type, status)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_memories_run_kind ON team_v4_memories(run_id, kind, accepted_by_commander)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_harness_run_status ON team_v4_harness_runs(run_id, status)",
    ];

    for sql in ddl {
        sqlx::query(sql).execute(pool).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn team_v4_ensure_schema(db: State<'_, Arc<DatabaseService>>) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())
}

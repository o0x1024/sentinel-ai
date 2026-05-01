use std::sync::Arc;

use anyhow::{anyhow, Result};
use sentinel_db::{database_service::connection_manager::DatabasePool, DatabaseService};
use sqlx::Row;
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
            role_type TEXT NOT NULL CHECK (role_type IN ('orchestrator','specialist','monitor','harness')),
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
            accepted_by_orchestrator BOOLEAN NOT NULL DEFAULT FALSE,
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
    ];

    for sql in ddl {
        sqlx::query(sql).execute(pool).await?;
    }
    migrate_schema_sqlite(pool).await?;
    ensure_indexes_sqlite(pool).await?;
    Ok(())
}

async fn ensure_indexes_sqlite(pool: &sqlx::SqlitePool) -> Result<()> {
    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_team_v4_runs_conversation ON team_v4_runs(conversation_id, updated_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_events_run_sequence ON team_v4_events(run_id, sequence)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_tasks_run_status_priority ON team_v4_tasks(run_id, status, priority, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_agents_run_role ON team_v4_agents(run_id, role_type, status)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_memories_run_kind ON team_v4_memories(run_id, kind, accepted_by_orchestrator)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_harness_run_status ON team_v4_harness_runs(run_id, status)",
    ];
    for sql in indexes {
        sqlx::query(sql).execute(pool).await?;
    }
    Ok(())
}

async fn migrate_schema_sqlite(pool: &sqlx::SqlitePool) -> Result<()> {
    let agent_schema: Option<(String,)> = sqlx::query_as(
        "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'team_v4_agents'",
    )
    .fetch_optional(pool)
    .await?;
    if agent_schema
        .as_ref()
        .map(|(sql,)| {
            sql.contains("'commander'") || sql.contains("'solver'") || sql.contains("'observer'")
        })
        .unwrap_or(false)
    {
        sqlx::query("ALTER TABLE team_v4_agents RENAME TO team_v4_agents_legacy_roles")
            .execute(pool)
            .await?;
        sqlx::query(
            r#"CREATE TABLE team_v4_agents (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
                profile_id TEXT,
                role_type TEXT NOT NULL CHECK (role_type IN ('orchestrator','specialist','monitor','harness')),
                name TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('idle','running','waiting','completed','failed','cancelled')),
                model TEXT,
                context_mode TEXT,
                tool_policy_json TEXT NOT NULL DEFAULT '{}',
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(pool)
        .await?;
        sqlx::query(
            r#"INSERT INTO team_v4_agents (
                id, run_id, profile_id, role_type, name, status, model, context_mode,
                tool_policy_json, metadata, created_at, updated_at
            )
            SELECT id, run_id, profile_id,
                   CASE role_type
                       WHEN 'commander' THEN 'orchestrator'
                       WHEN 'solver' THEN 'specialist'
                       WHEN 'observer' THEN 'monitor'
                       ELSE role_type
                   END,
                   CASE name
                       WHEN 'Commander' THEN 'Orchestrator'
                       WHEN 'Observer' THEN 'Monitor'
                       ELSE REPLACE(name, 'Solver', 'Specialist')
                   END,
                   status, model, context_mode, tool_policy_json, metadata, created_at, updated_at
            FROM team_v4_agents_legacy_roles"#,
        )
        .execute(pool)
        .await?;
        sqlx::query("DROP TABLE team_v4_agents_legacy_roles")
            .execute(pool)
            .await?;
    }

    let memory_columns = sqlx::query("PRAGMA table_info(team_v4_memories)")
        .fetch_all(pool)
        .await?;
    let has_old_column = memory_columns.iter().any(|row| {
        let name: String = row.get("name");
        name == "accepted_by_commander"
    });
    let has_new_column = memory_columns.iter().any(|row| {
        let name: String = row.get("name");
        name == "accepted_by_orchestrator"
    });
    if has_old_column && !has_new_column {
        sqlx::query(
            "ALTER TABLE team_v4_memories RENAME COLUMN accepted_by_commander TO accepted_by_orchestrator",
        )
        .execute(pool)
        .await?;
    }
    repair_sqlite_legacy_agent_foreign_keys(pool).await?;
    Ok(())
}

async fn sqlite_table_ddl(pool: &sqlx::SqlitePool, table_name: &str) -> Result<Option<String>> {
    Ok(
        sqlx::query("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table_name)
            .fetch_optional(pool)
            .await?
            .map(|row| row.get("sql")),
    )
}

async fn sqlite_table_references_legacy_agents(
    pool: &sqlx::SqlitePool,
    table_name: &str,
) -> Result<bool> {
    Ok(sqlite_table_ddl(pool, table_name)
        .await?
        .map(|sql| sql.contains("team_v4_agents_legacy_roles"))
        .unwrap_or(false))
}

async fn rebuild_sqlite_table(
    pool: &sqlx::SqlitePool,
    table_name: &str,
    legacy_table_name: &str,
    create_sql: &str,
    copy_columns: &str,
) -> Result<()> {
    let table_exists = sqlite_table_ddl(pool, table_name).await?.is_some();
    if !table_exists {
        return Ok(());
    }

    let mut conn = pool.acquire().await?;
    let conn = conn.as_mut();
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *conn)
        .await?;
    sqlx::query(&format!("DROP TABLE IF EXISTS {legacy_table_name}"))
        .execute(&mut *conn)
        .await?;
    sqlx::query(&format!(
        "ALTER TABLE {table_name} RENAME TO {legacy_table_name}"
    ))
    .execute(&mut *conn)
    .await?;
    sqlx::query(create_sql).execute(&mut *conn).await?;
    sqlx::query(&format!(
        "INSERT INTO {table_name} ({copy_columns}) SELECT {copy_columns} FROM {legacy_table_name}"
    ))
    .execute(&mut *conn)
    .await?;
    sqlx::query(&format!("DROP TABLE {legacy_table_name}"))
        .execute(&mut *conn)
        .await?;
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&mut *conn)
        .await?;
    Ok(())
}

async fn repair_sqlite_legacy_agent_foreign_keys(pool: &sqlx::SqlitePool) -> Result<()> {
    let affected_tables = [
        "team_v4_tasks",
        "team_v4_context_snapshots",
        "team_v4_memories",
        "team_v4_harness_runs",
    ];
    let mut needs_repair = false;
    for table_name in affected_tables {
        needs_repair =
            sqlite_table_references_legacy_agents(pool, table_name).await? || needs_repair;
    }
    if !needs_repair {
        return Ok(());
    }

    rebuild_sqlite_table(
        pool,
        "team_v4_tasks",
        "team_v4_tasks_legacy_agent_fk",
        r#"CREATE TABLE team_v4_tasks (
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
        "id, run_id, parent_task_id, task_key, title, instruction, status, priority, assigned_agent_id, depends_on, acceptance_criteria, context_snapshot_id, metadata, created_at, updated_at",
    )
    .await?;

    rebuild_sqlite_table(
        pool,
        "team_v4_context_snapshots",
        "team_v4_context_snapshots_legacy_agent_fk",
        r#"CREATE TABLE team_v4_context_snapshots (
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
        "id, run_id, actor_id, task_id, role_type, source_sequence, policy_json, sections_json, token_estimate, created_at",
    )
    .await?;

    rebuild_sqlite_table(
        pool,
        "team_v4_memories",
        "team_v4_memories_legacy_agent_fk",
        r#"CREATE TABLE team_v4_memories (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
            task_id TEXT REFERENCES team_v4_tasks(id) ON DELETE SET NULL,
            kind TEXT NOT NULL CHECK (kind IN ('evidence','decision','risk','blocker','checkpoint','artifact_summary')),
            content TEXT NOT NULL,
            confidence REAL NOT NULL DEFAULT 0,
            source_event_ids TEXT NOT NULL DEFAULT '[]',
            accepted_by_orchestrator BOOLEAN NOT NULL DEFAULT FALSE,
            promoted_to_long_term BOOLEAN NOT NULL DEFAULT FALSE,
            metadata TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        "id, run_id, task_id, kind, content, confidence, source_event_ids, accepted_by_orchestrator, promoted_to_long_term, metadata, created_at, updated_at",
    )
    .await?;

    rebuild_sqlite_table(
        pool,
        "team_v4_harness_runs",
        "team_v4_harness_runs_legacy_agent_fk",
        r#"CREATE TABLE team_v4_harness_runs (
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
        "id, run_id, actor_id, task_id, status, lease_expires_at, last_heartbeat_at, checkpoint_sequence, metadata, created_at, updated_at",
    )
    .await?;

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
            role_type TEXT NOT NULL CHECK (role_type IN ('orchestrator','specialist','monitor','harness')),
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
            accepted_by_orchestrator BOOLEAN NOT NULL DEFAULT FALSE,
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
    ];

    for sql in ddl {
        sqlx::query(sql).execute(pool).await?;
    }
    migrate_schema_pg(pool).await?;
    ensure_indexes_pg(pool).await?;
    Ok(())
}

async fn ensure_indexes_pg(pool: &sentinel_db::sqlx_compat::PgPool) -> Result<()> {
    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_team_v4_runs_conversation ON team_v4_runs(conversation_id, updated_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_events_run_sequence ON team_v4_events(run_id, sequence)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_tasks_run_status_priority ON team_v4_tasks(run_id, status, priority, created_at)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_agents_run_role ON team_v4_agents(run_id, role_type, status)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_memories_run_kind ON team_v4_memories(run_id, kind, accepted_by_orchestrator)",
        "CREATE INDEX IF NOT EXISTS idx_team_v4_harness_run_status ON team_v4_harness_runs(run_id, status)",
    ];
    for sql in indexes {
        sqlx::query(sql).execute(pool).await?;
    }
    Ok(())
}

async fn migrate_schema_pg(pool: &sentinel_db::sqlx_compat::PgPool) -> Result<()> {
    sqlx::query(
        r#"DO $$
        BEGIN
            IF EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_name = 'team_v4_memories'
                  AND column_name = 'accepted_by_commander'
            ) AND NOT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_name = 'team_v4_memories'
                  AND column_name = 'accepted_by_orchestrator'
            ) THEN
                ALTER TABLE team_v4_memories
                  RENAME COLUMN accepted_by_commander TO accepted_by_orchestrator;
            END IF;
        END $$"#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"ALTER TABLE team_v4_agents
           DROP CONSTRAINT IF EXISTS team_v4_agents_role_type_check"#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"UPDATE team_v4_agents
           SET role_type = CASE role_type
               WHEN 'commander' THEN 'orchestrator'
               WHEN 'solver' THEN 'specialist'
               WHEN 'observer' THEN 'monitor'
               ELSE role_type
           END,
           name = CASE name
               WHEN 'Commander' THEN 'Orchestrator'
               WHEN 'Observer' THEN 'Monitor'
               ELSE REPLACE(name, 'Solver', 'Specialist')
           END"#,
    )
    .execute(pool)
    .await?;
    sqlx::query(
        r#"ALTER TABLE team_v4_agents
           ADD CONSTRAINT team_v4_agents_role_type_check
           CHECK (role_type IN ('orchestrator','specialist','monitor','harness'))"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[tauri::command]
pub async fn team_v4_ensure_schema(db: State<'_, Arc<DatabaseService>>) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    ensure_team_v4_schema(&runtime_pool)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn repairs_sqlite_foreign_keys_left_on_legacy_agent_table() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            r#"CREATE TABLE team_v4_runs (
                id TEXT PRIMARY KEY,
                conversation_id TEXT,
                profile_id TEXT,
                goal TEXT NOT NULL,
                state TEXT NOT NULL CHECK (state IN ('draft','planning','running','waiting_human','completed','failed','cancelled','archived')),
                policy_json TEXT NOT NULL DEFAULT '{}',
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"CREATE TABLE team_v4_agents (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
                profile_id TEXT,
                role_type TEXT NOT NULL CHECK (role_type IN ('orchestrator','specialist','monitor','harness')),
                name TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('idle','running','waiting','completed','failed','cancelled')),
                model TEXT,
                context_mode TEXT,
                tool_policy_json TEXT NOT NULL DEFAULT '{}',
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"CREATE TABLE team_v4_tasks (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL REFERENCES team_v4_runs(id) ON DELETE CASCADE,
                parent_task_id TEXT REFERENCES team_v4_tasks(id) ON DELETE SET NULL,
                task_key TEXT NOT NULL,
                title TEXT NOT NULL,
                instruction TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('pending','ready','running','blocked','completed','failed','cancelled')),
                priority INTEGER NOT NULL DEFAULT 100,
                assigned_agent_id TEXT REFERENCES team_v4_agents_legacy_roles(id) ON DELETE SET NULL,
                depends_on TEXT NOT NULL DEFAULT '[]',
                acceptance_criteria TEXT,
                context_snapshot_id TEXT,
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(run_id, task_key)
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();

        ensure_schema_sqlite(&pool).await.unwrap();

        let task_ddl = sqlite_table_ddl(&pool, "team_v4_tasks")
            .await
            .unwrap()
            .unwrap();
        assert!(!task_ddl.contains("team_v4_agents_legacy_roles"));
        assert!(task_ddl.contains("REFERENCES team_v4_agents(id)"));

        sqlx::query(
            "INSERT INTO team_v4_runs (id, goal, state) VALUES ('run-1', 'goal', 'running')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"INSERT INTO team_v4_agents
               (id, run_id, role_type, name, status)
               VALUES ('agent-1', 'run-1', 'specialist', 'Specialist 1', 'idle')"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"INSERT INTO team_v4_tasks
               (id, run_id, task_key, title, instruction, status, assigned_agent_id)
               VALUES ('task-1', 'run-1', 'root', 'Root', 'Do it', 'pending', 'agent-1')"#,
        )
        .execute(&pool)
        .await
        .unwrap();
    }
}

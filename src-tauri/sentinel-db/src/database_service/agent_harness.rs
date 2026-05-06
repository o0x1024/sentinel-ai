use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHarnessRunInput {
    pub id: String,
    pub conversation_id: String,
    pub generation: i64,
    pub task: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub metadata: Option<Value>,
}

impl DatabaseService {
    async fn ensure_agent_harness_schema(&self) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                for sql in [
                    r#"CREATE TABLE IF NOT EXISTS agent_harness_runs (
                        id VARCHAR(191) PRIMARY KEY,
                        conversation_id VARCHAR(191) NOT NULL,
                        generation BIGINT NOT NULL,
                        state TEXT NOT NULL,
                        task TEXT NOT NULL,
                        model TEXT,
                        provider TEXT,
                        error TEXT,
                        metadata TEXT,
                        started_at TIMESTAMPTZ NOT NULL,
                        last_heartbeat_at TIMESTAMPTZ NOT NULL,
                        completed_at TIMESTAMPTZ,
                        created_at TIMESTAMPTZ NOT NULL,
                        updated_at TIMESTAMPTZ NOT NULL
                    )"#,
                    r#"CREATE TABLE IF NOT EXISTS agent_harness_events (
                        id VARCHAR(191) PRIMARY KEY,
                        run_id VARCHAR(191) NOT NULL,
                        conversation_id VARCHAR(191) NOT NULL,
                        generation BIGINT NOT NULL,
                        event_type TEXT NOT NULL,
                        payload TEXT,
                        created_at TIMESTAMPTZ NOT NULL
                    )"#,
                    r#"CREATE TABLE IF NOT EXISTS agent_harness_checkpoints (
                        id VARCHAR(191) PRIMARY KEY,
                        run_id VARCHAR(191) NOT NULL,
                        checkpoint_type TEXT NOT NULL,
                        payload TEXT,
                        created_at TIMESTAMPTZ NOT NULL
                    )"#,
                    "CREATE INDEX IF NOT EXISTS idx_agent_harness_runs_conversation ON agent_harness_runs(conversation_id, generation)",
                    "CREATE INDEX IF NOT EXISTS idx_agent_harness_runs_state ON agent_harness_runs(state)",
                    "CREATE INDEX IF NOT EXISTS idx_agent_harness_events_run ON agent_harness_events(run_id, created_at)",
                    "CREATE INDEX IF NOT EXISTS idx_agent_harness_checkpoints_run ON agent_harness_checkpoints(run_id, created_at)",
                ] {
                    sqlx::query(sql).execute(pool).await?;
                }
            }
            DatabasePool::SQLite(pool) => {
                for sql in [
                    r#"CREATE TABLE IF NOT EXISTS agent_harness_runs (
                        id TEXT PRIMARY KEY,
                        conversation_id TEXT NOT NULL,
                        generation INTEGER NOT NULL,
                        state TEXT NOT NULL,
                        task TEXT NOT NULL,
                        model TEXT,
                        provider TEXT,
                        error TEXT,
                        metadata TEXT,
                        started_at DATETIME NOT NULL,
                        last_heartbeat_at DATETIME NOT NULL,
                        completed_at DATETIME,
                        created_at DATETIME NOT NULL,
                        updated_at DATETIME NOT NULL
                    )"#,
                    r#"CREATE TABLE IF NOT EXISTS agent_harness_events (
                        id TEXT PRIMARY KEY,
                        run_id TEXT NOT NULL,
                        conversation_id TEXT NOT NULL,
                        generation INTEGER NOT NULL,
                        event_type TEXT NOT NULL,
                        payload TEXT,
                        created_at DATETIME NOT NULL
                    )"#,
                    r#"CREATE TABLE IF NOT EXISTS agent_harness_checkpoints (
                        id TEXT PRIMARY KEY,
                        run_id TEXT NOT NULL,
                        checkpoint_type TEXT NOT NULL,
                        payload TEXT,
                        created_at DATETIME NOT NULL
                    )"#,
                    "CREATE INDEX IF NOT EXISTS idx_agent_harness_runs_conversation ON agent_harness_runs(conversation_id, generation)",
                    "CREATE INDEX IF NOT EXISTS idx_agent_harness_runs_state ON agent_harness_runs(state)",
                    "CREATE INDEX IF NOT EXISTS idx_agent_harness_events_run ON agent_harness_events(run_id, created_at)",
                    "CREATE INDEX IF NOT EXISTS idx_agent_harness_checkpoints_run ON agent_harness_checkpoints(run_id, created_at)",
                ] {
                    sqlx::query(sql).execute(pool).await?;
                }
            }
            DatabasePool::MySQL(pool) => {
                for sql in [
                    r#"CREATE TABLE IF NOT EXISTS agent_harness_runs (
                        id TEXT PRIMARY KEY,
                        conversation_id TEXT NOT NULL,
                        generation BIGINT NOT NULL,
                        state TEXT NOT NULL,
                        task TEXT NOT NULL,
                        model TEXT,
                        provider TEXT,
                        error TEXT,
                        metadata TEXT,
                        started_at DATETIME NOT NULL,
                        last_heartbeat_at DATETIME NOT NULL,
                        completed_at DATETIME,
                        created_at DATETIME NOT NULL,
                        updated_at DATETIME NOT NULL
                    )"#,
                    r#"CREATE TABLE IF NOT EXISTS agent_harness_events (
                        id TEXT PRIMARY KEY,
                        run_id TEXT NOT NULL,
                        conversation_id TEXT NOT NULL,
                        generation BIGINT NOT NULL,
                        event_type TEXT NOT NULL,
                        payload TEXT,
                        created_at DATETIME NOT NULL
                    )"#,
                    r#"CREATE TABLE IF NOT EXISTS agent_harness_checkpoints (
                        id TEXT PRIMARY KEY,
                        run_id TEXT NOT NULL,
                        checkpoint_type TEXT NOT NULL,
                        payload TEXT,
                        created_at DATETIME NOT NULL
                    )"#,
                ] {
                    sqlx::query(sql).execute(pool).await?;
                }
                for sql in [
                    "CREATE INDEX idx_agent_harness_runs_conversation ON agent_harness_runs(conversation_id, generation)",
                    "CREATE INDEX idx_agent_harness_runs_state ON agent_harness_runs(state(64))",
                    "CREATE INDEX idx_agent_harness_events_run ON agent_harness_events(run_id, created_at)",
                    "CREATE INDEX idx_agent_harness_checkpoints_run ON agent_harness_checkpoints(run_id, created_at)",
                ] {
                    let _ = sqlx::query(sql).execute(pool).await;
                }
            }
        }

        Ok(())
    }

    pub async fn create_agent_harness_run(&self, input: AgentHarnessRunInput) -> Result<()> {
        self.ensure_agent_harness_schema().await?;
        let _permit = self
            .write_semaphore
            .acquire()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now();
        let metadata = input.metadata.map(|value| value.to_string());

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO agent_harness_runs (
                        id, conversation_id, generation, state, task, model, provider, metadata,
                        started_at, last_heartbeat_at, created_at, updated_at
                    ) VALUES ($1, $2, $3, 'running', $4, $5, $6, $7, $8, $8, $8, $8)
                    ON CONFLICT(id) DO UPDATE SET
                        state = excluded.state,
                        task = excluded.task,
                        model = excluded.model,
                        provider = excluded.provider,
                        metadata = excluded.metadata,
                        last_heartbeat_at = excluded.last_heartbeat_at,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&input.id)
                .bind(&input.conversation_id)
                .bind(input.generation)
                .bind(&input.task)
                .bind(&input.model)
                .bind(&input.provider)
                .bind(&metadata)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"INSERT INTO agent_harness_runs (
                        id, conversation_id, generation, state, task, model, provider, metadata,
                        started_at, last_heartbeat_at, created_at, updated_at
                    ) VALUES (?, ?, ?, 'running', ?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                        state = excluded.state,
                        task = excluded.task,
                        model = excluded.model,
                        provider = excluded.provider,
                        metadata = excluded.metadata,
                        last_heartbeat_at = excluded.last_heartbeat_at,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&input.id)
                .bind(&input.conversation_id)
                .bind(input.generation)
                .bind(&input.task)
                .bind(&input.model)
                .bind(&input.provider)
                .bind(&metadata)
                .bind(now)
                .bind(now)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO agent_harness_runs (
                        id, conversation_id, generation, state, task, model, provider, metadata,
                        started_at, last_heartbeat_at, created_at, updated_at
                    ) VALUES (?, ?, ?, 'running', ?, ?, ?, ?, ?, ?, ?, ?)
                    ON DUPLICATE KEY UPDATE
                        state = VALUES(state),
                        task = VALUES(task),
                        model = VALUES(model),
                        provider = VALUES(provider),
                        metadata = VALUES(metadata),
                        last_heartbeat_at = VALUES(last_heartbeat_at),
                        updated_at = VALUES(updated_at)"#,
                )
                .bind(&input.id)
                .bind(&input.conversation_id)
                .bind(input.generation)
                .bind(&input.task)
                .bind(&input.model)
                .bind(&input.provider)
                .bind(&metadata)
                .bind(now)
                .bind(now)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn update_agent_harness_state(
        &self,
        run_id: &str,
        state: &str,
        error: Option<&str>,
        completed_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        self.ensure_agent_harness_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now();

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE agent_harness_runs SET state = $1, error = $2, completed_at = $3, last_heartbeat_at = $4, updated_at = $4 WHERE id = $5",
                )
                .bind(state)
                .bind(error)
                .bind(completed_at)
                .bind(now)
                .bind(run_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE agent_harness_runs SET state = ?, error = ?, completed_at = ?, last_heartbeat_at = ?, updated_at = ? WHERE id = ?",
                )
                .bind(state)
                .bind(error)
                .bind(completed_at)
                .bind(now)
                .bind(now)
                .bind(run_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE agent_harness_runs SET state = ?, error = ?, completed_at = ?, last_heartbeat_at = ?, updated_at = ? WHERE id = ?",
                )
                .bind(state)
                .bind(error)
                .bind(completed_at)
                .bind(now)
                .bind(now)
                .bind(run_id)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn append_agent_harness_event(
        &self,
        run_id: &str,
        conversation_id: &str,
        generation: i64,
        event_type: &str,
        payload: Option<Value>,
    ) -> Result<()> {
        self.ensure_agent_harness_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let payload = payload.map(|value| value.to_string());

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO agent_harness_events (id, run_id, conversation_id, generation, event_type, payload, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                )
                .bind(&id)
                .bind(run_id)
                .bind(conversation_id)
                .bind(generation)
                .bind(event_type)
                .bind(&payload)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO agent_harness_events (id, run_id, conversation_id, generation, event_type, payload, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(run_id)
                .bind(conversation_id)
                .bind(generation)
                .bind(event_type)
                .bind(&payload)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO agent_harness_events (id, run_id, conversation_id, generation, event_type, payload, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(run_id)
                .bind(conversation_id)
                .bind(generation)
                .bind(event_type)
                .bind(&payload)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn append_agent_harness_checkpoint(
        &self,
        run_id: &str,
        checkpoint_type: &str,
        payload: Option<Value>,
    ) -> Result<()> {
        self.ensure_agent_harness_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let payload = payload.map(|value| value.to_string());

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO agent_harness_checkpoints (id, run_id, checkpoint_type, payload, created_at) VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(&id)
                .bind(run_id)
                .bind(checkpoint_type)
                .bind(&payload)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO agent_harness_checkpoints (id, run_id, checkpoint_type, payload, created_at) VALUES (?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(run_id)
                .bind(checkpoint_type)
                .bind(&payload)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO agent_harness_checkpoints (id, run_id, checkpoint_type, payload, created_at) VALUES (?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(run_id)
                .bind(checkpoint_type)
                .bind(&payload)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }
}

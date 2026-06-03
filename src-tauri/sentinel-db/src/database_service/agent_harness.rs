use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHarnessRunInput {
    pub id: String,
    pub conversation_id: String,
    pub generation: i64,
    pub task: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub metadata: Option<Value>,
    pub lease_secs: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHarnessRunRecord {
    pub id: String,
    pub conversation_id: String,
    pub generation: i64,
    pub state: String,
    pub task: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub error: Option<String>,
    pub metadata: Option<Value>,
    pub started_at: String,
    pub last_heartbeat_at: String,
    pub lease_expires_at: Option<String>,
    pub checkpoint_sequence: i64,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHarnessEventRecord {
    pub id: String,
    pub run_id: String,
    pub conversation_id: String,
    pub generation: i64,
    pub event_type: String,
    pub payload: Option<Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHarnessCheckpointRecord {
    pub id: String,
    pub run_id: String,
    pub checkpoint_type: String,
    pub payload: Option<Value>,
    pub created_at: String,
}

fn parse_optional_json(raw: Option<String>) -> Option<Value> {
    raw.and_then(|value| serde_json::from_str(&value).ok())
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
                        lease_expires_at TIMESTAMPTZ,
                        checkpoint_sequence BIGINT NOT NULL DEFAULT 0,
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
                for sql in [
                    "ALTER TABLE agent_harness_runs ADD COLUMN IF NOT EXISTS lease_expires_at TIMESTAMPTZ",
                    "ALTER TABLE agent_harness_runs ADD COLUMN IF NOT EXISTS checkpoint_sequence BIGINT NOT NULL DEFAULT 0",
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
                        lease_expires_at DATETIME,
                        checkpoint_sequence INTEGER NOT NULL DEFAULT 0,
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
                let columns: Vec<String> = sqlx::query("PRAGMA table_info(agent_harness_runs)")
                    .fetch_all(pool)
                    .await?
                    .into_iter()
                    .filter_map(|row| row.try_get::<String, _>("name").ok())
                    .collect();
                if !columns.iter().any(|name| name == "lease_expires_at") {
                    sqlx::query(
                        "ALTER TABLE agent_harness_runs ADD COLUMN lease_expires_at DATETIME",
                    )
                    .execute(pool)
                    .await?;
                }
                if !columns.iter().any(|name| name == "checkpoint_sequence") {
                    sqlx::query(
                        "ALTER TABLE agent_harness_runs ADD COLUMN checkpoint_sequence INTEGER NOT NULL DEFAULT 0",
                    )
                    .execute(pool)
                    .await?;
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
                        lease_expires_at DATETIME,
                        checkpoint_sequence BIGINT NOT NULL DEFAULT 0,
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
                for sql in [
                    "ALTER TABLE agent_harness_runs ADD COLUMN IF NOT EXISTS lease_expires_at DATETIME",
                    "ALTER TABLE agent_harness_runs ADD COLUMN IF NOT EXISTS checkpoint_sequence BIGINT NOT NULL DEFAULT 0",
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
        let lease_expires_at = input
            .lease_secs
            .map(|secs| now + Duration::seconds(secs.max(30)));
        let metadata = input.metadata.map(|value| value.to_string());

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO agent_harness_runs (
                        id, conversation_id, generation, state, task, model, provider, metadata,
                        started_at, last_heartbeat_at, lease_expires_at, checkpoint_sequence,
                        created_at, updated_at
                    ) VALUES ($1, $2, $3, 'running', $4, $5, $6, $7, $8, $8, $9, 0, $8, $8)
                    ON CONFLICT(id) DO UPDATE SET
                        state = excluded.state,
                        generation = excluded.generation,
                        task = excluded.task,
                        model = excluded.model,
                        provider = excluded.provider,
                        error = NULL,
                        metadata = excluded.metadata,
                        started_at = excluded.started_at,
                        last_heartbeat_at = excluded.last_heartbeat_at,
                        lease_expires_at = excluded.lease_expires_at,
                        completed_at = NULL,
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
                .bind(lease_expires_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"INSERT INTO agent_harness_runs (
                        id, conversation_id, generation, state, task, model, provider, metadata,
                        started_at, last_heartbeat_at, lease_expires_at, checkpoint_sequence,
                        created_at, updated_at
                    ) VALUES (?, ?, ?, 'running', ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                        state = excluded.state,
                        generation = excluded.generation,
                        task = excluded.task,
                        model = excluded.model,
                        provider = excluded.provider,
                        error = NULL,
                        metadata = excluded.metadata,
                        started_at = excluded.started_at,
                        last_heartbeat_at = excluded.last_heartbeat_at,
                        lease_expires_at = excluded.lease_expires_at,
                        completed_at = NULL,
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
                .bind(lease_expires_at)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO agent_harness_runs (
                        id, conversation_id, generation, state, task, model, provider, metadata,
                        started_at, last_heartbeat_at, lease_expires_at, checkpoint_sequence,
                        created_at, updated_at
                    ) VALUES (?, ?, ?, 'running', ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)
                    ON DUPLICATE KEY UPDATE
                        state = VALUES(state),
                        generation = VALUES(generation),
                        task = VALUES(task),
                        model = VALUES(model),
                        provider = VALUES(provider),
                        error = NULL,
                        metadata = VALUES(metadata),
                        started_at = VALUES(started_at),
                        last_heartbeat_at = VALUES(last_heartbeat_at),
                        lease_expires_at = VALUES(lease_expires_at),
                        completed_at = NULL,
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
                .bind(lease_expires_at)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn heartbeat_agent_harness_run(&self, run_id: &str, lease_secs: i64) -> Result<()> {
        self.ensure_agent_harness_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now();
        let lease_expires_at = now + Duration::seconds(lease_secs.max(30));

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE agent_harness_runs SET last_heartbeat_at = $1, lease_expires_at = $2, updated_at = $1 WHERE id = $3 AND completed_at IS NULL",
                )
                .bind(now)
                .bind(lease_expires_at)
                .bind(run_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE agent_harness_runs SET last_heartbeat_at = ?, lease_expires_at = ?, updated_at = ? WHERE id = ? AND completed_at IS NULL",
                )
                .bind(now)
                .bind(lease_expires_at)
                .bind(now)
                .bind(run_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE agent_harness_runs SET last_heartbeat_at = ?, lease_expires_at = ?, updated_at = ? WHERE id = ? AND completed_at IS NULL",
                )
                .bind(now)
                .bind(lease_expires_at)
                .bind(now)
                .bind(run_id)
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
                sqlx::query(
                    "UPDATE agent_harness_runs SET checkpoint_sequence = checkpoint_sequence + 1, updated_at = $1 WHERE id = $2",
                )
                .bind(now)
                .bind(run_id)
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
                sqlx::query(
                    "UPDATE agent_harness_runs SET checkpoint_sequence = checkpoint_sequence + 1, updated_at = ? WHERE id = ?",
                )
                .bind(now)
                .bind(run_id)
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
                sqlx::query(
                    "UPDATE agent_harness_runs SET checkpoint_sequence = checkpoint_sequence + 1, updated_at = ? WHERE id = ?",
                )
                .bind(now)
                .bind(run_id)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn list_agent_harness_runs(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<AgentHarnessRunRecord>> {
        self.ensure_agent_harness_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let rows = sqlx::query(
                    r#"SELECT id, conversation_id, generation, state, task, model, provider, error,
                              metadata, started_at::text AS started_at,
                              last_heartbeat_at::text AS last_heartbeat_at,
                              lease_expires_at::text AS lease_expires_at,
                              checkpoint_sequence, completed_at::text AS completed_at,
                              created_at::text AS created_at, updated_at::text AS updated_at
                       FROM agent_harness_runs
                       WHERE conversation_id = $1
                       ORDER BY created_at DESC, generation DESC"#,
                )
                .bind(conversation_id)
                .fetch_all(pool)
                .await?;
                rows.into_iter().map(map_agent_harness_run).collect()
            }
            DatabasePool::SQLite(pool) => {
                let rows = sqlx::query(
                    r#"SELECT id, conversation_id, generation, state, task, model, provider, error,
                              metadata, started_at, last_heartbeat_at, lease_expires_at,
                              checkpoint_sequence, completed_at, created_at, updated_at
                       FROM agent_harness_runs
                       WHERE conversation_id = ?
                       ORDER BY created_at DESC, generation DESC"#,
                )
                .bind(conversation_id)
                .fetch_all(pool)
                .await?;
                rows.into_iter().map(map_agent_harness_run).collect()
            }
            DatabasePool::MySQL(pool) => {
                let rows = sqlx::query(
                    r#"SELECT id, conversation_id, generation, state, task, model, provider, error,
                              metadata, CAST(started_at AS CHAR) AS started_at,
                              CAST(last_heartbeat_at AS CHAR) AS last_heartbeat_at,
                              CAST(lease_expires_at AS CHAR) AS lease_expires_at,
                              checkpoint_sequence, CAST(completed_at AS CHAR) AS completed_at,
                              CAST(created_at AS CHAR) AS created_at, CAST(updated_at AS CHAR) AS updated_at
                       FROM agent_harness_runs
                       WHERE conversation_id = ?
                       ORDER BY created_at DESC, generation DESC"#,
                )
                .bind(conversation_id)
                .fetch_all(pool)
                .await?;
                rows.into_iter().map(map_agent_harness_run).collect()
            }
        }
    }

    pub async fn delete_agent_harness_runs_after(
        &self,
        conversation_id: &str,
        after: DateTime<Utc>,
    ) -> Result<u64> {
        self.ensure_agent_harness_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let deleted = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"DELETE FROM agent_harness_checkpoints
                       WHERE run_id IN (
                         SELECT id FROM agent_harness_runs
                         WHERE conversation_id = $1 AND created_at > $2
                       )"#,
                )
                .bind(conversation_id)
                .bind(after)
                .execute(pool)
                .await?;
                sqlx::query(
                    r#"DELETE FROM agent_harness_events
                       WHERE run_id IN (
                         SELECT id FROM agent_harness_runs
                         WHERE conversation_id = $1 AND created_at > $2
                       )"#,
                )
                .bind(conversation_id)
                .bind(after)
                .execute(pool)
                .await?;
                sqlx::query(
                    "DELETE FROM agent_harness_runs WHERE conversation_id = $1 AND created_at > $2",
                )
                .bind(conversation_id)
                .bind(after)
                .execute(pool)
                .await?
                .rows_affected()
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"DELETE FROM agent_harness_checkpoints
                       WHERE run_id IN (
                         SELECT id FROM agent_harness_runs
                         WHERE conversation_id = ? AND created_at > ?
                       )"#,
                )
                .bind(conversation_id)
                .bind(after)
                .execute(pool)
                .await?;
                sqlx::query(
                    r#"DELETE FROM agent_harness_events
                       WHERE run_id IN (
                         SELECT id FROM agent_harness_runs
                         WHERE conversation_id = ? AND created_at > ?
                       )"#,
                )
                .bind(conversation_id)
                .bind(after)
                .execute(pool)
                .await?;
                sqlx::query(
                    "DELETE FROM agent_harness_runs WHERE conversation_id = ? AND created_at > ?",
                )
                .bind(conversation_id)
                .bind(after)
                .execute(pool)
                .await?
                .rows_affected()
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"DELETE FROM agent_harness_checkpoints
                       WHERE run_id IN (
                         SELECT id FROM agent_harness_runs
                         WHERE conversation_id = ? AND created_at > ?
                       )"#,
                )
                .bind(conversation_id)
                .bind(after)
                .execute(pool)
                .await?;
                sqlx::query(
                    r#"DELETE FROM agent_harness_events
                       WHERE run_id IN (
                         SELECT id FROM agent_harness_runs
                         WHERE conversation_id = ? AND created_at > ?
                       )"#,
                )
                .bind(conversation_id)
                .bind(after)
                .execute(pool)
                .await?;
                sqlx::query(
                    "DELETE FROM agent_harness_runs WHERE conversation_id = ? AND created_at > ?",
                )
                .bind(conversation_id)
                .bind(after)
                .execute(pool)
                .await?
                .rows_affected()
            }
        };

        Ok(deleted)
    }

    pub async fn list_agent_harness_events(
        &self,
        run_id: &str,
    ) -> Result<Vec<AgentHarnessEventRecord>> {
        self.ensure_agent_harness_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let rows = sqlx::query(
                    r#"SELECT id, run_id, conversation_id, generation, event_type, payload,
                              created_at::text AS created_at
                       FROM agent_harness_events
                       WHERE run_id = $1
                       ORDER BY created_at ASC"#,
                )
                .bind(run_id)
                .fetch_all(pool)
                .await?;
                rows.into_iter().map(map_agent_harness_event).collect()
            }
            DatabasePool::SQLite(pool) => {
                let rows = sqlx::query(
                    r#"SELECT id, run_id, conversation_id, generation, event_type, payload, created_at
                       FROM agent_harness_events
                       WHERE run_id = ?
                       ORDER BY created_at ASC"#,
                )
                .bind(run_id)
                .fetch_all(pool)
                .await?;
                rows.into_iter().map(map_agent_harness_event).collect()
            }
            DatabasePool::MySQL(pool) => {
                let rows = sqlx::query(
                    r#"SELECT id, run_id, conversation_id, generation, event_type, payload,
                              CAST(created_at AS CHAR) AS created_at
                       FROM agent_harness_events
                       WHERE run_id = ?
                       ORDER BY created_at ASC"#,
                )
                .bind(run_id)
                .fetch_all(pool)
                .await?;
                rows.into_iter().map(map_agent_harness_event).collect()
            }
        }
    }

    pub async fn list_agent_harness_checkpoints(
        &self,
        run_id: &str,
    ) -> Result<Vec<AgentHarnessCheckpointRecord>> {
        self.ensure_agent_harness_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let rows = sqlx::query(
                    r#"SELECT id, run_id, checkpoint_type, payload, created_at::text AS created_at
                       FROM agent_harness_checkpoints
                       WHERE run_id = $1
                       ORDER BY created_at ASC"#,
                )
                .bind(run_id)
                .fetch_all(pool)
                .await?;
                rows.into_iter().map(map_agent_harness_checkpoint).collect()
            }
            DatabasePool::SQLite(pool) => {
                let rows = sqlx::query(
                    r#"SELECT id, run_id, checkpoint_type, payload, created_at
                       FROM agent_harness_checkpoints
                       WHERE run_id = ?
                       ORDER BY created_at ASC"#,
                )
                .bind(run_id)
                .fetch_all(pool)
                .await?;
                rows.into_iter().map(map_agent_harness_checkpoint).collect()
            }
            DatabasePool::MySQL(pool) => {
                let rows = sqlx::query(
                    r#"SELECT id, run_id, checkpoint_type, payload,
                              CAST(created_at AS CHAR) AS created_at
                       FROM agent_harness_checkpoints
                       WHERE run_id = ?
                       ORDER BY created_at ASC"#,
                )
                .bind(run_id)
                .fetch_all(pool)
                .await?;
                rows.into_iter().map(map_agent_harness_checkpoint).collect()
            }
        }
    }
}

fn map_agent_harness_run<R>(row: R) -> Result<AgentHarnessRunRecord>
where
    R: Row,
    for<'c> &'c str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> i64: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    Ok(AgentHarnessRunRecord {
        id: row.try_get("id")?,
        conversation_id: row.try_get("conversation_id")?,
        generation: row.try_get("generation")?,
        state: row.try_get("state")?,
        task: row.try_get("task")?,
        model: row.try_get("model")?,
        provider: row.try_get("provider")?,
        error: row.try_get("error")?,
        metadata: parse_optional_json(row.try_get("metadata")?),
        started_at: row.try_get("started_at")?,
        last_heartbeat_at: row.try_get("last_heartbeat_at")?,
        lease_expires_at: row.try_get("lease_expires_at")?,
        checkpoint_sequence: row.try_get("checkpoint_sequence")?,
        completed_at: row.try_get("completed_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn map_agent_harness_event<R>(row: R) -> Result<AgentHarnessEventRecord>
where
    R: Row,
    for<'c> &'c str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> i64: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    Ok(AgentHarnessEventRecord {
        id: row.try_get("id")?,
        run_id: row.try_get("run_id")?,
        conversation_id: row.try_get("conversation_id")?,
        generation: row.try_get("generation")?,
        event_type: row.try_get("event_type")?,
        payload: parse_optional_json(row.try_get("payload")?),
        created_at: row.try_get("created_at")?,
    })
}

fn map_agent_harness_checkpoint<R>(row: R) -> Result<AgentHarnessCheckpointRecord>
where
    R: Row,
    for<'c> &'c str: sqlx::ColumnIndex<R>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Option<String>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    Ok(AgentHarnessCheckpointRecord {
        id: row.try_get("id")?,
        run_id: row.try_get("run_id")?,
        checkpoint_type: row.try_get("checkpoint_type")?,
        payload: parse_optional_json(row.try_get("payload")?),
        created_at: row.try_get("created_at")?,
    })
}

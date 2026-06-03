use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentExecutionTurnRecord {
    pub turn_id: String,
    pub conversation_id: String,
    pub user_message_id: Option<String>,
    pub parent_turn_id: Option<String>,
    pub task: String,
    pub status: String,
    pub harness_mode: Option<String>,
    pub interruption_reason: Option<String>,
    pub last_checkpoint_type: Option<String>,
    pub last_checkpoint_payload: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentExecutionTurnStartInput {
    pub turn_id: String,
    pub conversation_id: String,
    pub user_message_id: Option<String>,
    pub parent_turn_id: Option<String>,
    pub task: String,
    pub harness_mode: Option<String>,
}

impl DatabaseService {
    async fn ensure_agent_execution_turns_schema(&self) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                for sql in [
                    r#"CREATE TABLE IF NOT EXISTS agent_execution_turns (
                        turn_id TEXT PRIMARY KEY,
                        conversation_id TEXT NOT NULL,
                        user_message_id TEXT,
                        parent_turn_id TEXT,
                        task TEXT NOT NULL,
                        status TEXT NOT NULL,
                        harness_mode TEXT,
                        interruption_reason TEXT,
                        last_checkpoint_type TEXT,
                        last_checkpoint_payload TEXT,
                        created_at TEXT NOT NULL,
                        updated_at TEXT NOT NULL,
                        completed_at TEXT
                    )"#,
                    "CREATE INDEX IF NOT EXISTS idx_agent_execution_turns_conversation ON agent_execution_turns(conversation_id, updated_at DESC)",
                    "CREATE INDEX IF NOT EXISTS idx_agent_execution_turns_status ON agent_execution_turns(status)",
                ] {
                    sqlx::query(sql).execute(pool).await?;
                }
            }
            DatabasePool::SQLite(pool) => {
                for sql in [
                    r#"CREATE TABLE IF NOT EXISTS agent_execution_turns (
                        turn_id TEXT PRIMARY KEY,
                        conversation_id TEXT NOT NULL,
                        user_message_id TEXT,
                        parent_turn_id TEXT,
                        task TEXT NOT NULL,
                        status TEXT NOT NULL,
                        harness_mode TEXT,
                        interruption_reason TEXT,
                        last_checkpoint_type TEXT,
                        last_checkpoint_payload TEXT,
                        created_at TEXT NOT NULL,
                        updated_at TEXT NOT NULL,
                        completed_at TEXT
                    )"#,
                    "CREATE INDEX IF NOT EXISTS idx_agent_execution_turns_conversation ON agent_execution_turns(conversation_id, updated_at DESC)",
                    "CREATE INDEX IF NOT EXISTS idx_agent_execution_turns_status ON agent_execution_turns(status)",
                ] {
                    sqlx::query(sql).execute(pool).await?;
                }
            }
            DatabasePool::MySQL(pool) => {
                for sql in [
                    r#"CREATE TABLE IF NOT EXISTS agent_execution_turns (
                        turn_id VARCHAR(191) PRIMARY KEY,
                        conversation_id TEXT NOT NULL,
                        user_message_id TEXT,
                        parent_turn_id TEXT,
                        task TEXT NOT NULL,
                        status TEXT NOT NULL,
                        harness_mode TEXT,
                        interruption_reason TEXT,
                        last_checkpoint_type TEXT,
                        last_checkpoint_payload TEXT,
                        created_at TEXT NOT NULL,
                        updated_at TEXT NOT NULL,
                        completed_at TEXT
                    )"#,
                    "CREATE INDEX idx_agent_execution_turns_conversation ON agent_execution_turns(conversation_id(191), updated_at(64))",
                    "CREATE INDEX idx_agent_execution_turns_status ON agent_execution_turns(status(64))",
                ] {
                    let _ = sqlx::query(sql).execute(pool).await;
                }
            }
        }

        Ok(())
    }

    pub async fn upsert_agent_execution_turn_started(
        &self,
        input: AgentExecutionTurnStartInput,
    ) -> Result<()> {
        self.ensure_agent_execution_turns_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now().to_rfc3339();

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO agent_execution_turns (
                        turn_id, conversation_id, user_message_id, parent_turn_id, task, status,
                        harness_mode, interruption_reason, last_checkpoint_type,
                        last_checkpoint_payload, created_at, updated_at, completed_at
                    ) VALUES ($1, $2, $3, $4, $5, 'running', $6, NULL, NULL, NULL, $7, $7, NULL)
                    ON CONFLICT(turn_id) DO UPDATE SET
                        conversation_id = excluded.conversation_id,
                        user_message_id = excluded.user_message_id,
                        task = excluded.task,
                        status = 'running',
                        harness_mode = excluded.harness_mode,
                        interruption_reason = NULL,
                        updated_at = excluded.updated_at,
                        completed_at = NULL"#,
                )
                .bind(&input.turn_id)
                .bind(&input.conversation_id)
                .bind(&input.user_message_id)
                .bind(&input.parent_turn_id)
                .bind(&input.task)
                .bind(&input.harness_mode)
                .bind(&now)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"INSERT INTO agent_execution_turns (
                        turn_id, conversation_id, user_message_id, parent_turn_id, task, status,
                        harness_mode, interruption_reason, last_checkpoint_type,
                        last_checkpoint_payload, created_at, updated_at, completed_at
                    ) VALUES (?, ?, ?, ?, ?, 'running', ?, NULL, NULL, NULL, ?, ?, NULL)
                    ON CONFLICT(turn_id) DO UPDATE SET
                        conversation_id = excluded.conversation_id,
                        user_message_id = excluded.user_message_id,
                        task = excluded.task,
                        status = 'running',
                        harness_mode = excluded.harness_mode,
                        interruption_reason = NULL,
                        updated_at = excluded.updated_at,
                        completed_at = NULL"#,
                )
                .bind(&input.turn_id)
                .bind(&input.conversation_id)
                .bind(&input.user_message_id)
                .bind(&input.parent_turn_id)
                .bind(&input.task)
                .bind(&input.harness_mode)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO agent_execution_turns (
                        turn_id, conversation_id, user_message_id, parent_turn_id, task, status,
                        harness_mode, interruption_reason, last_checkpoint_type,
                        last_checkpoint_payload, created_at, updated_at, completed_at
                    ) VALUES (?, ?, ?, ?, ?, 'running', ?, NULL, NULL, NULL, ?, ?, NULL)
                    ON DUPLICATE KEY UPDATE
                        conversation_id = VALUES(conversation_id),
                        user_message_id = VALUES(user_message_id),
                        task = VALUES(task),
                        status = 'running',
                        harness_mode = VALUES(harness_mode),
                        interruption_reason = NULL,
                        updated_at = VALUES(updated_at),
                        completed_at = NULL"#,
                )
                .bind(&input.turn_id)
                .bind(&input.conversation_id)
                .bind(&input.user_message_id)
                .bind(&input.parent_turn_id)
                .bind(&input.task)
                .bind(&input.harness_mode)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn update_agent_execution_turn_checkpoint(
        &self,
        turn_id: &str,
        checkpoint_type: &str,
        payload: Option<Value>,
    ) -> Result<()> {
        self.ensure_agent_execution_turns_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now().to_rfc3339();
        let payload = payload.map(|value| value.to_string());

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE agent_execution_turns SET last_checkpoint_type = $1, last_checkpoint_payload = $2, updated_at = $3 WHERE turn_id = $4",
                )
                .bind(checkpoint_type)
                .bind(&payload)
                .bind(&now)
                .bind(turn_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE agent_execution_turns SET last_checkpoint_type = ?, last_checkpoint_payload = ?, updated_at = ? WHERE turn_id = ?",
                )
                .bind(checkpoint_type)
                .bind(&payload)
                .bind(&now)
                .bind(turn_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE agent_execution_turns SET last_checkpoint_type = ?, last_checkpoint_payload = ?, updated_at = ? WHERE turn_id = ?",
                )
                .bind(checkpoint_type)
                .bind(&payload)
                .bind(&now)
                .bind(turn_id)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn finish_agent_execution_turn(
        &self,
        turn_id: &str,
        status: &str,
        interruption_reason: Option<&str>,
    ) -> Result<()> {
        self.ensure_agent_execution_turns_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now().to_rfc3339();

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE agent_execution_turns SET status = $1, interruption_reason = $2, updated_at = $3, completed_at = $3 WHERE turn_id = $4",
                )
                .bind(status)
                .bind(interruption_reason)
                .bind(&now)
                .bind(turn_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE agent_execution_turns SET status = ?, interruption_reason = ?, updated_at = ?, completed_at = ? WHERE turn_id = ?",
                )
                .bind(status)
                .bind(interruption_reason)
                .bind(&now)
                .bind(&now)
                .bind(turn_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE agent_execution_turns SET status = ?, interruption_reason = ?, updated_at = ?, completed_at = ? WHERE turn_id = ?",
                )
                .bind(status)
                .bind(interruption_reason)
                .bind(&now)
                .bind(&now)
                .bind(turn_id)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn get_latest_agent_execution_turn(
        &self,
        conversation_id: &str,
    ) -> Result<Option<AgentExecutionTurnRecord>> {
        self.ensure_agent_execution_turns_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let row = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, AgentExecutionTurnRecord>(
                    "SELECT * FROM agent_execution_turns WHERE conversation_id = $1 ORDER BY updated_at DESC LIMIT 1",
                )
                .bind(conversation_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, AgentExecutionTurnRecord>(
                    "SELECT * FROM agent_execution_turns WHERE conversation_id = ? ORDER BY updated_at DESC LIMIT 1",
                )
                .bind(conversation_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, AgentExecutionTurnRecord>(
                    "SELECT * FROM agent_execution_turns WHERE conversation_id = ? ORDER BY updated_at DESC LIMIT 1",
                )
                .bind(conversation_id)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(row)
    }

    pub async fn get_agent_execution_turn(
        &self,
        turn_id: &str,
    ) -> Result<Option<AgentExecutionTurnRecord>> {
        self.ensure_agent_execution_turns_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let row = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, AgentExecutionTurnRecord>(
                    "SELECT * FROM agent_execution_turns WHERE turn_id = $1",
                )
                .bind(turn_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, AgentExecutionTurnRecord>(
                    "SELECT * FROM agent_execution_turns WHERE turn_id = ?",
                )
                .bind(turn_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, AgentExecutionTurnRecord>(
                    "SELECT * FROM agent_execution_turns WHERE turn_id = ?",
                )
                .bind(turn_id)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(row)
    }

    pub async fn list_agent_execution_turns_for_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<AgentExecutionTurnRecord>> {
        self.ensure_agent_execution_turns_schema().await?;
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, AgentExecutionTurnRecord>(
                    "SELECT * FROM agent_execution_turns WHERE conversation_id = $1 ORDER BY updated_at DESC",
                )
                .bind(conversation_id)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, AgentExecutionTurnRecord>(
                    "SELECT * FROM agent_execution_turns WHERE conversation_id = ? ORDER BY updated_at DESC",
                )
                .bind(conversation_id)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, AgentExecutionTurnRecord>(
                    "SELECT * FROM agent_execution_turns WHERE conversation_id = ? ORDER BY updated_at DESC",
                )
                .bind(conversation_id)
                .fetch_all(pool)
                .await?
            }
        };

        Ok(rows)
    }
}

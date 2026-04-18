use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::core::models::database::{
    DurableMemoryProjectionState, DurableMemoryRecord, MemoryExecution,
};
use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;

impl DatabaseService {
    pub async fn upsert_durable_memory_record_internal(
        &self,
        record: &DurableMemoryRecord,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO memory_records (
                        id, title, text, kind, tier, scope, stability, source,
                        confidence, importance, tags_json, origin_execution_id,
                        supersedes_memory_id, status, created_at_ms, updated_at_ms
                    ) VALUES (
                        $1, $2, $3, $4, $5, $6, $7, $8,
                        $9, $10, $11, $12,
                        $13, $14, $15, $16
                    )
                    ON CONFLICT(id) DO UPDATE SET
                        title = excluded.title,
                        text = excluded.text,
                        kind = excluded.kind,
                        tier = excluded.tier,
                        scope = excluded.scope,
                        stability = excluded.stability,
                        source = excluded.source,
                        confidence = excluded.confidence,
                        importance = excluded.importance,
                        tags_json = excluded.tags_json,
                        origin_execution_id = excluded.origin_execution_id,
                        supersedes_memory_id = excluded.supersedes_memory_id,
                        status = excluded.status,
                        updated_at_ms = excluded.updated_at_ms
                    "#,
                )
                .bind(&record.id)
                .bind(&record.title)
                .bind(&record.text)
                .bind(&record.kind)
                .bind(&record.tier)
                .bind(&record.scope)
                .bind(&record.stability)
                .bind(&record.source)
                .bind(record.confidence)
                .bind(record.importance)
                .bind(&record.tags_json)
                .bind(&record.origin_execution_id)
                .bind(&record.supersedes_memory_id)
                .bind(&record.status)
                .bind(record.created_at_ms)
                .bind(record.updated_at_ms)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO memory_records (
                        id, title, text, kind, tier, scope, stability, source,
                        confidence, importance, tags_json, origin_execution_id,
                        supersedes_memory_id, status, created_at_ms, updated_at_ms
                    ) VALUES (
                        ?, ?, ?, ?, ?, ?, ?, ?,
                        ?, ?, ?, ?,
                        ?, ?, ?, ?
                    )
                    ON CONFLICT(id) DO UPDATE SET
                        title = excluded.title,
                        text = excluded.text,
                        kind = excluded.kind,
                        tier = excluded.tier,
                        scope = excluded.scope,
                        stability = excluded.stability,
                        source = excluded.source,
                        confidence = excluded.confidence,
                        importance = excluded.importance,
                        tags_json = excluded.tags_json,
                        origin_execution_id = excluded.origin_execution_id,
                        supersedes_memory_id = excluded.supersedes_memory_id,
                        status = excluded.status,
                        updated_at_ms = excluded.updated_at_ms
                    "#,
                )
                .bind(&record.id)
                .bind(&record.title)
                .bind(&record.text)
                .bind(&record.kind)
                .bind(&record.tier)
                .bind(&record.scope)
                .bind(&record.stability)
                .bind(&record.source)
                .bind(record.confidence)
                .bind(record.importance)
                .bind(&record.tags_json)
                .bind(&record.origin_execution_id)
                .bind(&record.supersedes_memory_id)
                .bind(&record.status)
                .bind(record.created_at_ms)
                .bind(record.updated_at_ms)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO memory_records (
                        id, title, text, kind, tier, scope, stability, source,
                        confidence, importance, tags_json, origin_execution_id,
                        supersedes_memory_id, status, created_at_ms, updated_at_ms
                    ) VALUES (
                        ?, ?, ?, ?, ?, ?, ?, ?,
                        ?, ?, ?, ?,
                        ?, ?, ?, ?
                    )
                    ON DUPLICATE KEY UPDATE
                        title = VALUES(title),
                        text = VALUES(text),
                        kind = VALUES(kind),
                        tier = VALUES(tier),
                        scope = VALUES(scope),
                        stability = VALUES(stability),
                        source = VALUES(source),
                        confidence = VALUES(confidence),
                        importance = VALUES(importance),
                        tags_json = VALUES(tags_json),
                        origin_execution_id = VALUES(origin_execution_id),
                        supersedes_memory_id = VALUES(supersedes_memory_id),
                        status = VALUES(status),
                        updated_at_ms = VALUES(updated_at_ms)
                    "#,
                )
                .bind(&record.id)
                .bind(&record.title)
                .bind(&record.text)
                .bind(&record.kind)
                .bind(&record.tier)
                .bind(&record.scope)
                .bind(&record.stability)
                .bind(&record.source)
                .bind(record.confidence)
                .bind(record.importance)
                .bind(&record.tags_json)
                .bind(&record.origin_execution_id)
                .bind(&record.supersedes_memory_id)
                .bind(&record.status)
                .bind(record.created_at_ms)
                .bind(record.updated_at_ms)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn upsert_durable_memory_projection_state_internal(
        &self,
        state: &DurableMemoryProjectionState,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO memory_projection_state (
                        memory_id, lexical_indexed, vector_indexed, skill_projected, last_error, updated_at_ms
                    ) VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT(memory_id) DO UPDATE SET
                        lexical_indexed = excluded.lexical_indexed,
                        vector_indexed = excluded.vector_indexed,
                        skill_projected = excluded.skill_projected,
                        last_error = excluded.last_error,
                        updated_at_ms = excluded.updated_at_ms
                    "#,
                )
                .bind(&state.memory_id)
                .bind(state.lexical_indexed)
                .bind(state.vector_indexed)
                .bind(state.skill_projected)
                .bind(&state.last_error)
                .bind(state.updated_at_ms)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO memory_projection_state (
                        memory_id, lexical_indexed, vector_indexed, skill_projected, last_error, updated_at_ms
                    ) VALUES (?, ?, ?, ?, ?, ?)
                    ON CONFLICT(memory_id) DO UPDATE SET
                        lexical_indexed = excluded.lexical_indexed,
                        vector_indexed = excluded.vector_indexed,
                        skill_projected = excluded.skill_projected,
                        last_error = excluded.last_error,
                        updated_at_ms = excluded.updated_at_ms
                    "#,
                )
                .bind(&state.memory_id)
                .bind(state.lexical_indexed)
                .bind(state.vector_indexed)
                .bind(state.skill_projected)
                .bind(&state.last_error)
                .bind(state.updated_at_ms)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO memory_projection_state (
                        memory_id, lexical_indexed, vector_indexed, skill_projected, last_error, updated_at_ms
                    ) VALUES (?, ?, ?, ?, ?, ?)
                    ON DUPLICATE KEY UPDATE
                        lexical_indexed = VALUES(lexical_indexed),
                        vector_indexed = VALUES(vector_indexed),
                        skill_projected = VALUES(skill_projected),
                        last_error = VALUES(last_error),
                        updated_at_ms = VALUES(updated_at_ms)
                    "#,
                )
                .bind(&state.memory_id)
                .bind(state.lexical_indexed)
                .bind(state.vector_indexed)
                .bind(state.skill_projected)
                .bind(&state.last_error)
                .bind(state.updated_at_ms)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn list_recent_durable_memory_records_internal(
        &self,
        limit: i64,
    ) -> Result<Vec<DurableMemoryRecord>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, DurableMemoryRecord>(
                    r#"
                    SELECT * FROM memory_records
                    WHERE status = 'active' AND tier = 'durable'
                    ORDER BY updated_at_ms DESC
                    LIMIT $1
                    "#,
                )
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, DurableMemoryRecord>(
                    r#"
                    SELECT * FROM memory_records
                    WHERE status = 'active' AND tier = 'durable'
                    ORDER BY updated_at_ms DESC
                    LIMIT ?
                    "#,
                )
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, DurableMemoryRecord>(
                    r#"
                    SELECT * FROM memory_records
                    WHERE status = 'active' AND tier = 'durable'
                    ORDER BY updated_at_ms DESC
                    LIMIT ?
                    "#,
                )
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
        };

        Ok(rows)
    }

    pub async fn get_durable_memory_projection_state_internal(
        &self,
        memory_id: &str,
    ) -> Result<Option<DurableMemoryProjectionState>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let row = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, DurableMemoryProjectionState>(
                    r#"
                    SELECT * FROM memory_projection_state
                    WHERE memory_id = $1
                    "#,
                )
                .bind(memory_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, DurableMemoryProjectionState>(
                    r#"
                    SELECT * FROM memory_projection_state
                    WHERE memory_id = ?
                    "#,
                )
                .bind(memory_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, DurableMemoryProjectionState>(
                    r#"
                    SELECT * FROM memory_projection_state
                    WHERE memory_id = ?
                    "#,
                )
                .bind(memory_id)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(row)
    }

    pub async fn get_durable_memory_record_internal(
        &self,
        memory_id: &str,
    ) -> Result<Option<DurableMemoryRecord>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let row = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, DurableMemoryRecord>(
                    r#"
                    SELECT * FROM memory_records
                    WHERE id = $1
                    "#,
                )
                .bind(memory_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, DurableMemoryRecord>(
                    r#"
                    SELECT * FROM memory_records
                    WHERE id = ?
                    "#,
                )
                .bind(memory_id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, DurableMemoryRecord>(
                    r#"
                    SELECT * FROM memory_records
                    WHERE id = ?
                    "#,
                )
                .bind(memory_id)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(row)
    }

    pub async fn create_memory_execution_internal(&self, record: &MemoryExecution) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO memory_executions (
                        id, task, environment, tool_calls, success, error, response_excerpt, created_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    ON CONFLICT(id) DO UPDATE SET
                        task = excluded.task,
                        environment = excluded.environment,
                        tool_calls = excluded.tool_calls,
                        success = excluded.success,
                        error = excluded.error,
                        response_excerpt = excluded.response_excerpt,
                        created_at = excluded.created_at
                    "#,
                )
                .bind(&record.id)
                .bind(&record.task)
                .bind(&record.environment)
                .bind(&record.tool_calls)
                .bind(record.success)
                .bind(&record.error)
                .bind(&record.response_excerpt)
                .bind(record.created_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO memory_executions (
                        id, task, environment, tool_calls, success, error, response_excerpt, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    ON CONFLICT(id) DO UPDATE SET
                        task = excluded.task,
                        environment = excluded.environment,
                        tool_calls = excluded.tool_calls,
                        success = excluded.success,
                        error = excluded.error,
                        response_excerpt = excluded.response_excerpt,
                        created_at = excluded.created_at
                    "#,
                )
                .bind(&record.id)
                .bind(&record.task)
                .bind(&record.environment)
                .bind(&record.tool_calls)
                .bind(record.success)
                .bind(&record.error)
                .bind(&record.response_excerpt)
                .bind(record.created_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO memory_executions (
                        id, task, environment, tool_calls, success, error, response_excerpt, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                    ON DUPLICATE KEY UPDATE
                        task = VALUES(task),
                        environment = VALUES(environment),
                        tool_calls = VALUES(tool_calls),
                        success = VALUES(success),
                        error = VALUES(error),
                        response_excerpt = VALUES(response_excerpt),
                        created_at = VALUES(created_at)
                    "#,
                )
                .bind(&record.id)
                .bind(&record.task)
                .bind(&record.environment)
                .bind(&record.tool_calls)
                .bind(record.success)
                .bind(&record.error)
                .bind(&record.response_excerpt)
                .bind(record.created_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn get_memory_executions_since_internal(
        &self,
        since: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<MemoryExecution>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                if let Some(since) = since {
                    sqlx::query_as::<_, MemoryExecution>(
                        "SELECT * FROM memory_executions WHERE created_at > $1 ORDER BY created_at ASC LIMIT $2",
                    )
                    .bind(since)
                    .bind(limit)
                    .fetch_all(pool)
                    .await?
                } else {
                    sqlx::query_as::<_, MemoryExecution>(
                        "SELECT * FROM memory_executions ORDER BY created_at ASC LIMIT $1",
                    )
                    .bind(limit)
                    .fetch_all(pool)
                    .await?
                }
            }
            DatabasePool::SQLite(pool) => {
                if let Some(since) = since {
                    sqlx::query_as::<_, MemoryExecution>(
                        "SELECT * FROM memory_executions WHERE created_at > ? ORDER BY created_at ASC LIMIT ?",
                    )
                    .bind(since)
                    .bind(limit)
                    .fetch_all(pool)
                    .await?
                } else {
                    sqlx::query_as::<_, MemoryExecution>(
                        "SELECT * FROM memory_executions ORDER BY created_at ASC LIMIT ?",
                    )
                    .bind(limit)
                    .fetch_all(pool)
                    .await?
                }
            }
            DatabasePool::MySQL(pool) => {
                if let Some(since) = since {
                    sqlx::query_as::<_, MemoryExecution>(
                        "SELECT * FROM memory_executions WHERE created_at > ? ORDER BY created_at ASC LIMIT ?",
                    )
                    .bind(since)
                    .bind(limit)
                    .fetch_all(pool)
                    .await?
                } else {
                    sqlx::query_as::<_, MemoryExecution>(
                        "SELECT * FROM memory_executions ORDER BY created_at ASC LIMIT ?",
                    )
                    .bind(limit)
                    .fetch_all(pool)
                    .await?
                }
            }
        };

        Ok(rows)
    }
}

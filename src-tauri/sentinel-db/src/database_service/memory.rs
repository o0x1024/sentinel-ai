use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::core::models::database::MemoryExecution;
use crate::database_service::service::DatabaseService;

impl DatabaseService {
    pub async fn create_memory_execution_internal(&self, record: &MemoryExecution) -> Result<()> {
        let pool = self.get_pool()?;
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

        Ok(())
    }

    pub async fn get_memory_executions_since_internal(
        &self,
        since: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<MemoryExecution>> {
        let pool = self.get_pool()?;
        let rows = if let Some(since) = since {
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
        };

        Ok(rows)
    }
}

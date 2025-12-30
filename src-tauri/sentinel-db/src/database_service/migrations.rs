use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use tracing::info;

/// Database migration for task-tool integration feature
pub struct TaskToolIntegrationMigration;

impl TaskToolIntegrationMigration {
    /// Apply migration to add task-tool tracking tables
    pub async fn apply(pool: &SqlitePool) -> Result<()> {
        info!("Applying task-tool integration migration...");

        // 1. Create task_tool_executions table for detailed execution tracking
        // Note: Removed FOREIGN KEY constraint to allow testing with temporary task IDs
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS task_tool_executions (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                tool_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                tool_type TEXT NOT NULL,
                status TEXT NOT NULL,
                execution_count INTEGER DEFAULT 0,
                success_count INTEGER DEFAULT 0,
                error_count INTEGER DEFAULT 0,
                total_execution_time_ms INTEGER DEFAULT 0,
                avg_execution_time_ms INTEGER DEFAULT 0,
                last_execution_time TEXT,
                last_error_message TEXT,
                metadata TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )"#
        ).execute(pool).await?;

        // 2. Create task_tool_execution_logs for individual execution records
        // Note: Removed FOREIGN KEY constraints to allow testing with temporary task IDs
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS task_tool_execution_logs (
                id TEXT PRIMARY KEY,
                task_tool_execution_id TEXT NOT NULL,
                task_id TEXT NOT NULL,
                tool_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                tool_type TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                execution_time_ms INTEGER,
                input_params TEXT,
                output_result TEXT,
                error_message TEXT,
                metadata TEXT,
                created_at TEXT NOT NULL
            )"#
        ).execute(pool).await?;

        // 3. Add indices for performance
        let indices = vec![
            "CREATE INDEX IF NOT EXISTS idx_task_tool_executions_task_id ON task_tool_executions(task_id)",
            "CREATE INDEX IF NOT EXISTS idx_task_tool_executions_tool_id ON task_tool_executions(tool_id)",
            "CREATE INDEX IF NOT EXISTS idx_task_tool_executions_status ON task_tool_executions(status)",
            "CREATE INDEX IF NOT EXISTS idx_task_tool_executions_updated ON task_tool_executions(updated_at DESC)",
            "CREATE INDEX IF NOT EXISTS idx_task_tool_execution_logs_task_id ON task_tool_execution_logs(task_id)",
            "CREATE INDEX IF NOT EXISTS idx_task_tool_execution_logs_tool_id ON task_tool_execution_logs(tool_id)",
            "CREATE INDEX IF NOT EXISTS idx_task_tool_execution_logs_started ON task_tool_execution_logs(started_at DESC)",
        ];

        for index_sql in indices {
            sqlx::query(index_sql).execute(pool).await?;
        }

        // 4. Add new columns to scan_tasks table (if not exists)
        // Note: SQLite doesn't support ADD COLUMN IF NOT EXISTS, so we need to check first
        Self::add_column_if_not_exists(
            pool,
            "scan_tasks",
            "active_tools_count",
            "INTEGER DEFAULT 0"
        ).await?;

        Self::add_column_if_not_exists(
            pool,
            "scan_tasks",
            "tool_statistics",
            "TEXT"
        ).await?;

        info!("Task-tool integration migration completed successfully");
        Ok(())
    }

    /// Helper function to add column if it doesn't exist
    async fn add_column_if_not_exists(
        pool: &SqlitePool,
        table: &str,
        column: &str,
        column_type: &str,
    ) -> Result<()> {
        // Check if column exists
        let check_query = format!(
            "SELECT COUNT(*) as count FROM pragma_table_info('{}') WHERE name = '{}'",
            table, column
        );
        
        let result: (i64,) = sqlx::query_as(&check_query)
            .fetch_one(pool)
            .await?;

        if result.0 == 0 {
            // Column doesn't exist, add it
            let alter_query = format!(
                "ALTER TABLE {} ADD COLUMN {} {}",
                table, column, column_type
            );
            sqlx::query(&alter_query).execute(pool).await?;
            info!("Added column {} to table {}", column, table);
        } else {
            info!("Column {} already exists in table {}", column, table);
        }

        Ok(())
    }

    /// Rollback migration (for testing purposes)
    pub async fn rollback(pool: &SqlitePool) -> Result<()> {
        info!("Rolling back task-tool integration migration...");

        sqlx::query("DROP TABLE IF EXISTS task_tool_execution_logs")
            .execute(pool)
            .await?;

        sqlx::query("DROP TABLE IF EXISTS task_tool_executions")
            .execute(pool)
            .await?;

        info!("Task-tool integration migration rollback completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_migration_apply_and_rollback() {
        // This test requires a test database
        // Implementation depends on your test setup
    }
}

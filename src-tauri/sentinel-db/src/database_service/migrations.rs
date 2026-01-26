use anyhow::Result;
use sqlx::postgres::PgPool;
use tracing::info;

/// Database migration for ASM (Attack Surface Management) enhancements
pub struct AsmEnhancementMigration;

impl AsmEnhancementMigration {
    /// Apply migration to add ASM fields to bounty_assets table
    pub async fn apply(pool: &PgPool) -> Result<()> {
        info!("Applying ASM enhancement migration...");

        // Add IP Asset Attributes
        Self::add_column_if_not_exists(pool, "bounty_assets", "ip_version", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "asn", "INTEGER").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "asn_org", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "isp", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "country", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "city", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "latitude", "DOUBLE PRECISION").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "longitude", "DOUBLE PRECISION").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "is_cloud", "BOOLEAN").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "cloud_provider", "TEXT").await?;

        // Add Port/Service Attributes
        Self::add_column_if_not_exists(pool, "bounty_assets", "service_name", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "service_version", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "service_product", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "banner", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "transport_protocol", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "cpe", "TEXT").await?;

        // Add Domain Attributes
        Self::add_column_if_not_exists(pool, "bounty_assets", "domain_registrar", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "registration_date", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "expiration_date", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "nameservers_json", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "mx_records_json", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "txt_records_json", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "whois_data_json", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "is_wildcard", "BOOLEAN").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "parent_domain", "TEXT").await?;

        // Add Web/URL Attributes
        Self::add_column_if_not_exists(pool, "bounty_assets", "http_status", "INTEGER").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "response_time_ms", "INTEGER").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "content_length", "INTEGER").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "content_type", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "title", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "favicon_hash", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "headers_json", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "waf_detected", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "cdn_detected", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "screenshot_path", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "body_hash", "TEXT").await?;

        // Add Certificate Attributes
        Self::add_column_if_not_exists(pool, "bounty_assets", "certificate_id", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "ssl_enabled", "BOOLEAN").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "certificate_subject", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "certificate_issuer", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "certificate_valid_from", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "certificate_valid_to", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "certificate_san_json", "TEXT").await?;

        // Add Attack Surface & Risk
        Self::add_column_if_not_exists(pool, "bounty_assets", "exposure_level", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "attack_surface_score", "DOUBLE PRECISION").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "vulnerability_count", "INTEGER").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "cvss_max_score", "DOUBLE PRECISION").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "exploit_available", "BOOLEAN").await?;

        // Add Asset Classification
        Self::add_column_if_not_exists(pool, "bounty_assets", "asset_category", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "asset_owner", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "business_unit", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "criticality", "TEXT").await?;

        // Add Discovery & Monitoring
        Self::add_column_if_not_exists(pool, "bounty_assets", "discovery_method", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "data_sources_json", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "confidence_score", "DOUBLE PRECISION").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "monitoring_enabled", "BOOLEAN").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "scan_frequency", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "last_scan_type", "TEXT").await?;

        // Add Asset Relationships
        Self::add_column_if_not_exists(pool, "bounty_assets", "parent_asset_id", "TEXT").await?;
        Self::add_column_if_not_exists(pool, "bounty_assets", "related_assets_json", "TEXT").await?;

        // Add indices for new columns
        let indices = vec![
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_asset_type ON bounty_assets(asset_type)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_ip_version ON bounty_assets(ip_version)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_asn ON bounty_assets(asn)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_country ON bounty_assets(country)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_service_name ON bounty_assets(service_name)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_exposure_level ON bounty_assets(exposure_level)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_criticality ON bounty_assets(criticality)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_parent_asset_id ON bounty_assets(parent_asset_id)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_attack_surface_score ON bounty_assets(attack_surface_score DESC)",
            "CREATE INDEX IF NOT EXISTS idx_bounty_assets_vulnerability_count ON bounty_assets(vulnerability_count DESC)",
        ];

        for index_sql in indices {
            sqlx::query(index_sql).execute(pool).await?;
        }

        info!("ASM enhancement migration completed successfully");
        Ok(())
    }

    /// Helper function to add column if it doesn't exist
    async fn add_column_if_not_exists(
        pool: &PgPool,
        table: &str,
        column: &str,
        column_type: &str,
    ) -> Result<()> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = $1 AND column_name = $2)"
        )
        .bind(table)
        .bind(column)
        .fetch_one(pool)
        .await?;

        if !exists {
            info!("Adding column '{}' to table '{}'", column, table);
            let alter_query = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, column_type);
            sqlx::query(&alter_query).execute(pool).await?;
        }

        Ok(())
    }
}

/// Database migration for task-tool integration feature
pub struct TaskToolIntegrationMigration;

impl TaskToolIntegrationMigration {
    /// Apply migration to add task-tool tracking tables
    pub async fn apply(pool: &PgPool) -> Result<()> {
        info!("Applying task-tool integration migration...");

        // 1. Create task_tool_executions table for detailed execution tracking
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
                total_execution_time_ms BIGINT DEFAULT 0,
                avg_execution_time_ms BIGINT DEFAULT 0,
                last_execution_time TIMESTAMPTZ,
                last_error_message TEXT,
                metadata TEXT,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )"#
        ).execute(pool).await?;

        // 2. Create task_tool_execution_logs for individual execution records
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS task_tool_execution_logs (
                id TEXT PRIMARY KEY,
                task_tool_execution_id TEXT NOT NULL,
                task_id TEXT NOT NULL,
                tool_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                tool_type TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TIMESTAMPTZ NOT NULL,
                completed_at TIMESTAMPTZ,
                execution_time_ms BIGINT,
                input_params TEXT,
                output_result TEXT,
                error_message TEXT,
                metadata TEXT,
                created_at TIMESTAMPTZ NOT NULL
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
        AsmEnhancementMigration::add_column_if_not_exists(
            pool,
            "scan_tasks",
            "active_tools_count",
            "INTEGER DEFAULT 0"
        ).await?;

        AsmEnhancementMigration::add_column_if_not_exists(
            pool,
            "scan_tasks",
            "tool_statistics",
            "TEXT"
        ).await?;

        info!("Task-tool integration migration completed successfully");
        Ok(())
    }

    /// Rollback migration (for testing purposes)
    pub async fn rollback(pool: &PgPool) -> Result<()> {
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

/// Database migration for subagent runs table
pub struct SubagentRunsMigration;

impl SubagentRunsMigration {
    pub async fn apply(pool: &PgPool) -> Result<()> {
        info!("Applying subagent runs migration...");

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS ai_subagent_runs (
                id TEXT PRIMARY KEY,
                parent_execution_id TEXT NOT NULL,
                role TEXT,
                task TEXT NOT NULL,
                status TEXT NOT NULL,
                output TEXT,
                error TEXT,
                model_name TEXT,
                model_provider TEXT,
                started_at TIMESTAMPTZ NOT NULL,
                completed_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )"#
        ).execute(pool).await?;

        let indices = vec![
            "CREATE INDEX IF NOT EXISTS idx_subagent_runs_parent ON ai_subagent_runs(parent_execution_id)",
            "CREATE INDEX IF NOT EXISTS idx_subagent_runs_status ON ai_subagent_runs(status)",
            "CREATE INDEX IF NOT EXISTS idx_subagent_runs_updated ON ai_subagent_runs(updated_at DESC)",
        ];

        for index_sql in indices {
            sqlx::query(index_sql).execute(pool).await?;
        }

        info!("Subagent runs migration completed successfully");
        Ok(())
    }
}

/// Database migration for subagent messages table
pub struct SubagentMessagesMigration;

impl SubagentMessagesMigration {
    pub async fn apply(pool: &PgPool) -> Result<()> {
        info!("Applying subagent messages migration...");

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS ai_subagent_messages (
                id TEXT PRIMARY KEY,
                subagent_run_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                tool_calls TEXT,
                attachments TEXT,
                reasoning_content TEXT,
                timestamp TIMESTAMPTZ NOT NULL,
                structured_data TEXT
            )"#
        ).execute(pool).await?;

        let indices = vec![
            "CREATE INDEX IF NOT EXISTS idx_subagent_messages_run ON ai_subagent_messages(subagent_run_id)",
            "CREATE INDEX IF NOT EXISTS idx_subagent_messages_time ON ai_subagent_messages(timestamp DESC)",
        ];

        for index_sql in indices {
            sqlx::query(index_sql).execute(pool).await?;
        }

        info!("Subagent messages migration completed successfully");
        Ok(())
    }
}

/// Database migration for agent todos persistence
pub struct AgentTodosMigration;

impl AgentTodosMigration {
    pub async fn apply(pool: &PgPool) -> Result<()> {
        info!("Applying agent todos migration...");

        // Create agent_todos table for persistent todo storage
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_todos (
                id TEXT PRIMARY KEY,
                execution_id TEXT NOT NULL,
                item_index INTEGER NOT NULL,
                description TEXT NOT NULL,
                status TEXT NOT NULL,
                result TEXT,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )"#
        ).execute(pool).await?;

        let indices = vec![
            "CREATE INDEX IF NOT EXISTS idx_agent_todos_execution ON agent_todos(execution_id)",
            "CREATE INDEX IF NOT EXISTS idx_agent_todos_execution_index ON agent_todos(execution_id, item_index)",
            "CREATE INDEX IF NOT EXISTS idx_agent_todos_updated ON agent_todos(updated_at DESC)",
        ];

        for index_sql in indices {
            sqlx::query(index_sql).execute(pool).await?;
        }

        info!("Agent todos migration completed successfully");
        Ok(())
    }
}

/// Database migration to fix FLOAT4/REAL to FLOAT8/DOUBLE PRECISION type mismatch
/// This fixes sqlx type compatibility issues where code uses f64 but database uses REAL
pub struct FloatTypeMigration;

impl FloatTypeMigration {
    pub async fn apply(pool: &PgPool) -> Result<()> {
        info!("Applying float type migration (REAL -> DOUBLE PRECISION)...");

        // List of (table, column) pairs that need to be migrated from REAL to DOUBLE PRECISION
        let columns_to_migrate = vec![
            // bounty_assets table
            ("bounty_assets", "latitude"),
            ("bounty_assets", "longitude"),
            ("bounty_assets", "attack_surface_score"),
            ("bounty_assets", "cvss_max_score"),
            ("bounty_assets", "confidence_score"),
            ("bounty_assets", "priority_score"),
            ("bounty_assets", "risk_score"),
            // bounty_programs table
            ("bounty_programs", "priority_score"),
            ("bounty_programs", "total_earnings"),
            // bounty_findings table
            ("bounty_findings", "cvss_score"),
            // bounty_submissions table
            ("bounty_submissions", "cvss_score"),
            ("bounty_submissions", "reward_amount"),
            ("bounty_submissions", "bonus_amount"),
            // plugin_registry table
            ("plugin_registry", "quality_score"),
            // ai_memories table
            ("ai_memories", "cost"),
            ("ai_memories", "confidence"),
            // agent_todos table
            ("agent_todos", "confidence"),
            // workflow_runs table
            ("workflow_runs", "progress"),
            // tasks table
            ("tasks", "progress"),
            // dictionaries table
            ("dictionaries", "weight"),
            // knowledge_graph_edges table
            ("knowledge_graph_edges", "confidence"),
            // ai_executions table
            ("ai_executions", "cost"),
            // model_stats table
            ("model_stats", "cost"),
        ];

        for (table, column) in columns_to_migrate {
            Self::migrate_column_to_double(pool, table, column).await?;
        }

        info!("Float type migration completed successfully");
        Ok(())
    }

    /// Migrate a single column from REAL to DOUBLE PRECISION
    async fn migrate_column_to_double(
        pool: &PgPool,
        table: &str,
        column: &str,
    ) -> Result<()> {
        // Check if column exists and is REAL type
        let column_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_name = $1 AND column_name = $2
            )"
        )
        .bind(table)
        .bind(column)
        .fetch_one(pool)
        .await?;

        if !column_exists {
            return Ok(());
        }

        // Check if column is already DOUBLE PRECISION
        let is_double: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_name = $1 AND column_name = $2
                AND data_type = 'double precision'
            )"
        )
        .bind(table)
        .bind(column)
        .fetch_one(pool)
        .await?;

        if is_double {
            info!("Column {}.{} is already DOUBLE PRECISION, skipping", table, column);
            return Ok(());
        }

        // Check if column is REAL or FLOAT4 type
        let is_real_or_float4: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_name = $1 AND column_name = $2
                AND (data_type = 'real' OR data_type = 'double precision')
            )"
        )
        .bind(table)
        .bind(column)
        .fetch_one(pool)
        .await?;

        // Check if column is REAL or FLOAT4 (numeric_precision = 24 for REAL/FLOAT4)
        let numeric_precision: Option<i32> = sqlx::query_scalar(
            "SELECT numeric_precision FROM information_schema.columns
             WHERE table_name = $1 AND column_name = $2"
        )
        .bind(table)
        .bind(column)
        .fetch_optional(pool)
        .await?;

        let needs_migration = if let Some(precision) = numeric_precision {
            // REAL/FLOAT4 has precision of 24, DOUBLE PRECISION has precision of 53
            precision == 24
        } else {
            // Fallback: check data_type directly
            is_real_or_float4 && !is_double
        };

        if !needs_migration {
            info!("Column {}.{} does not need migration (not REAL/FLOAT4), skipping", table, column);
            return Ok(());
        }

        // Alter column type from REAL/FLOAT4 to DOUBLE PRECISION
        info!("Migrating column {}.{} from REAL/FLOAT4 to DOUBLE PRECISION", table, column);
        let alter_sql = format!(
            "ALTER TABLE {} ALTER COLUMN {} TYPE DOUBLE PRECISION USING {}::DOUBLE PRECISION",
            table, column, column
        );

        sqlx::query(&alter_sql)
            .execute(pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to migrate column {}.{}: {}", table, column, e))?;

        Ok(())
    }
}

/// Database migration to fix TEXT to TIMESTAMP WITH TIME ZONE type mismatch
/// This fixes sqlx type compatibility issues where code expects TIMESTAMP WITH TIME ZONE but database uses TEXT
pub struct TimestampTypeMigration;

impl TimestampTypeMigration {
    pub async fn apply(pool: &PgPool) -> Result<()> {
        info!("Applying timestamp type migration (TEXT -> TIMESTAMP WITH TIME ZONE)...");

        // List of (table, column) pairs that need to be migrated from TEXT to TIMESTAMP WITH TIME ZONE
        let columns_to_migrate = vec![
            // bounty_programs table
            ("bounty_programs", "created_at"),
            ("bounty_programs", "updated_at"),
            ("bounty_programs", "last_activity_at"),
            // bounty_scopes table
            ("bounty_scopes", "created_at"),
            ("bounty_scopes", "updated_at"),
            // bounty_findings table
            ("bounty_findings", "first_seen_at"),
            ("bounty_findings", "last_seen_at"),
            ("bounty_findings", "verified_at"),
            ("bounty_findings", "created_at"),
            ("bounty_findings", "updated_at"),
            // bounty_submissions table
            ("bounty_submissions", "created_at"),
            ("bounty_submissions", "submitted_at"),
            ("bounty_submissions", "updated_at"),
            ("bounty_submissions", "closed_at"),
            ("bounty_submissions", "retest_at"),
            ("bounty_submissions", "last_retest_at"),
            // bounty_evidence table
            ("bounty_evidence", "created_at"),
            ("bounty_evidence", "updated_at"),
            // bounty_change_events table
            ("bounty_change_events", "created_at"),
            ("bounty_change_events", "updated_at"),
            ("bounty_change_events", "resolved_at"),
            // bounty_assets table
            ("bounty_assets", "last_checked_at"),
            ("bounty_assets", "first_seen_at"),
            ("bounty_assets", "last_seen_at"),
            ("bounty_assets", "created_at"),
            ("bounty_assets", "updated_at"),
            // bounty_workflow_templates table
            ("bounty_workflow_templates", "created_at"),
            ("bounty_workflow_templates", "updated_at"),
            // bounty_workflow_bindings table
            ("bounty_workflow_bindings", "last_run_at"),
            ("bounty_workflow_bindings", "created_at"),
            ("bounty_workflow_bindings", "updated_at"),
            // repeater_tabs table
            ("repeater_tabs", "created_at"),
            ("repeater_tabs", "updated_at"),
        ];

        for (table, column) in columns_to_migrate {
            Self::migrate_column_to_timestamp(pool, table, column).await?;
        }

        info!("Timestamp type migration completed successfully");
        Ok(())
    }

    /// Migrate a single column from TEXT to TIMESTAMP WITH TIME ZONE
    async fn migrate_column_to_timestamp(
        pool: &PgPool,
        table: &str,
        column: &str,
    ) -> Result<()> {
        // Check if column exists
        let column_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_name = $1 AND column_name = $2
            )"
        )
        .bind(table)
        .bind(column)
        .fetch_one(pool)
        .await?;

        if !column_exists {
            return Ok(());
        }

        // Check if column is already TIMESTAMP WITH TIME ZONE
        let is_timestamp: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_name = $1 AND column_name = $2
                AND data_type = 'timestamp with time zone'
            )"
        )
        .bind(table)
        .bind(column)
        .fetch_one(pool)
        .await?;

        if is_timestamp {
            info!("Column {}.{} is already TIMESTAMP WITH TIME ZONE, skipping", table, column);
            return Ok(());
        }

        // Check if column is TEXT type
        let is_text: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_name = $1 AND column_name = $2
                AND data_type = 'text'
            )"
        )
        .bind(table)
        .bind(column)
        .fetch_one(pool)
        .await?;

        if !is_text {
            info!("Column {}.{} is not TEXT type, skipping", table, column);
            return Ok(());
        }

        // Alter column type from TEXT to TIMESTAMP WITH TIME ZONE
        info!("Migrating column {}.{} from TEXT to TIMESTAMP WITH TIME ZONE", table, column);
        let alter_sql = format!(
            "ALTER TABLE {} ALTER COLUMN {} TYPE TIMESTAMP WITH TIME ZONE USING {}::TIMESTAMP WITH TIME ZONE",
            table, column, column
        );

        sqlx::query(&alter_sql)
            .execute(pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to migrate column {}.{}: {}", table, column, e))?;

        Ok(())
    }
}

/// Database migration to fix INTEGER to BIGINT type mismatch
/// This fixes sqlx type compatibility issues where code uses i64 but database uses INTEGER
pub struct IntegerTypeMigration;

impl IntegerTypeMigration {
    pub async fn apply(pool: &PgPool) -> Result<()> {
        info!("Applying integer type migration (INTEGER -> BIGINT)...");

        // List of (table, column) pairs that need to be migrated from INTEGER to BIGINT
        let columns_to_migrate = vec![
            // dictionaries table
            ("dictionaries", "word_count"),
            ("dictionaries", "file_size"),
            // rag_collections table
            ("rag_collections", "document_count"),
            ("rag_collections", "chunk_count"),
            // rag_document_sources table
            ("rag_document_sources", "chunk_count"),
            ("rag_document_sources", "file_size"),
            // bounty_evidence table
            ("bounty_evidence", "file_size"),
        ];

        for (table, column) in columns_to_migrate {
            Self::migrate_column_to_bigint(pool, table, column).await?;
        }

        info!("Integer type migration completed successfully");
        Ok(())
    }

    /// Migrate a single column from INTEGER to BIGINT
    async fn migrate_column_to_bigint(
        pool: &PgPool,
        table: &str,
        column: &str,
    ) -> Result<()> {
        // Check if column exists
        let column_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_name = $1 AND column_name = $2
            )"
        )
        .bind(table)
        .bind(column)
        .fetch_one(pool)
        .await?;

        if !column_exists {
            return Ok(());
        }

        // Check if column is already BIGINT
        let is_bigint: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_name = $1 AND column_name = $2
                AND data_type = 'bigint'
            )"
        )
        .bind(table)
        .bind(column)
        .fetch_one(pool)
        .await?;

        if is_bigint {
            info!("Column {}.{} is already BIGINT, skipping", table, column);
            return Ok(());
        }

        // Check if column is INTEGER type
        let is_integer: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_name = $1 AND column_name = $2
                AND data_type = 'integer'
            )"
        )
        .bind(table)
        .bind(column)
        .fetch_one(pool)
        .await?;

        if !is_integer {
            info!("Column {}.{} is not INTEGER type, skipping", table, column);
            return Ok(());
        }

        // Alter column type from INTEGER to BIGINT
        info!("Migrating column {}.{} from INTEGER to BIGINT", table, column);
        let alter_sql = format!(
            "ALTER TABLE {} ALTER COLUMN {} TYPE BIGINT USING {}::BIGINT",
            table, column, column
        );

        sqlx::query(&alter_sql)
            .execute(pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to migrate column {}.{}: {}", table, column, e))?;

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

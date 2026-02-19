use anyhow::Result;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Column, Row, TypeInfo};
use std::path::PathBuf;
use serde_json::Value;
use crate::core::models::database::DatabaseStats;
use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::db_config::{
    db_config_toml_path, load_db_config_from_disk, DatabaseConfig, DatabaseType,
};
use crate::database_service::migration::DatabaseMigration;
use std::sync::Arc;
use tokio::sync::Semaphore;
use std::time::Duration;

#[derive(Debug, Clone)]
/// 数据库服务
pub struct DatabaseService {
    pub(crate) pool: Option<PgPool>,
    pub(crate) runtime_pool: Option<DatabasePool>,
    pub(crate) config: Option<DatabaseConfig>,
    pub(crate) write_semaphore: Arc<Semaphore>,
}

impl DatabaseService {
    pub fn get_db_config(&self) -> Option<&DatabaseConfig> {
        self.config.as_ref()
    }

    pub fn new() -> Self {
        // Limit concurrent writes if necessary, though PG handles concurrency well.
        // We keep the semaphore for compatibility/throttling if needed.
        Self {
            pool: None,
            runtime_pool: None,
            config: None,
            write_semaphore: Arc::new(Semaphore::new(10)), // Higher limit for PG
        }
    }

    pub fn get_pool(&self) -> Result<&PgPool> {
        if let Some(pool) = self.pool.as_ref() {
            return Ok(pool);
        }

        match self.runtime_pool.as_ref() {
            Some(DatabasePool::PostgreSQL(pool)) => Ok(pool),
            Some(other) => Err(anyhow::anyhow!(
                "当前数据库类型为 {:?}，该能力仅支持 PostgreSQL",
                other.db_type()
            )),
            None => Err(anyhow::anyhow!("数据库未初始化")),
        }
    }

    /// Get database pool (public method for external use)
    pub fn pool(&self) -> &PgPool {
        self.get_pool().expect("Database not initialized")
    }
    
    // Deprecated exact match for SQLite but kept for interface compatibility if generic
    pub fn get_postgres_pool(&self) -> Result<&PgPool> {
        self.get_pool()
    }

    pub fn get_db(&self) -> Result<crate::client::DatabaseClient> {
        match self.runtime_pool.as_ref() {
            Some(DatabasePool::PostgreSQL(pool)) => Ok(crate::client::DatabaseClient::new(pool.clone())),
            Some(other) => Err(anyhow::anyhow!(
                "DatabaseClient 当前仅支持 PostgreSQL，当前数据库类型: {:?}",
                other.db_type()
            )),
            None => Err(anyhow::anyhow!("数据库未初始化")),
        }
    }

    pub fn get_sqlite_pool(&self) -> Result<sqlx::SqlitePool> {
        match self.runtime_pool.as_ref() {
            Some(DatabasePool::SQLite(pool)) => Ok(pool.clone()),
            _ => Err(anyhow::anyhow!(
                "Current runtime database is not SQLite; SQLite pool unavailable"
            )),
        }
    }

    pub fn get_db_path(&self) -> PathBuf {
        if let Some(config) = self.config.as_ref() {
            if matches!(config.db_type, DatabaseType::SQLite) {
                if let Some(path) = config.path.as_ref() {
                    return PathBuf::from(path);
                }
            }
        }

        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sentinel-ai")
            .join("database.db")
    }

    pub fn get_skills_root_dir(&self) -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sentinel-ai")
            .join("skills")
    }

    pub async fn backup(&self, path: Option<PathBuf>) -> Result<PathBuf> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let ext = match runtime {
            DatabasePool::SQLite(_) => "db",
            DatabasePool::PostgreSQL(_) | DatabasePool::MySQL(_) => "sql",
        };
        let filename = format!("backup_{}.{}", chrono::Utc::now().timestamp(), ext);
        let default_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sentinel-ai");
        let mut backup_path = path.unwrap_or_else(|| default_dir.join(&filename));
        if backup_path.extension().is_none() {
            backup_path = backup_path.join(&filename);
        }

        if let Some(parent) = backup_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        match runtime {
            DatabasePool::SQLite(_) => {
                let source = self.get_db_path();
                if source == backup_path {
                    return Err(anyhow::anyhow!("备份路径不能与当前数据库文件相同"));
                }
                tokio::fs::copy(&source, &backup_path).await?;
            }
            DatabasePool::PostgreSQL(_) | DatabasePool::MySQL(_) => {
                let migration = DatabaseMigration::new(runtime.clone());
                migration.export_to_sql(&backup_path).await?;
            }
        }

        Ok(backup_path)
    }

    pub async fn restore(&self, path: PathBuf) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::SQLite(_) => {
                let target = self.get_db_path();
                if path == target {
                    return Err(anyhow::anyhow!("备份文件路径不能与数据库路径相同"));
                }
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                tokio::fs::copy(&path, &target).await?;
                Ok(())
            }
            DatabasePool::PostgreSQL(_) | DatabasePool::MySQL(_) => {
                let sql = tokio::fs::read_to_string(&path).await?;
                self.execute_sql_script(runtime, &sql).await
            }
        }
    }

    /// 初始化数据库
    pub async fn initialize(&mut self) -> Result<()> {
        // Try to load config from file
        let config: DatabaseConfig = match load_db_config_from_disk() {
            Ok(Some(c)) => c,
            Ok(None) => self.default_pg_config(),
            Err(e) => {
                tracing::warn!(
                    "Failed to parse {}: {}, using default",
                    db_config_toml_path().display(),
                    e
                );
                self.default_pg_config()
            }
        };

        self.config = Some(config.clone());

        // Non-PostgreSQL databases use runtime pool for generic commands and migrations.
        if !matches!(config.db_type, DatabaseType::PostgreSQL) {
            let runtime = DatabasePool::connect(&config).await?;
            self.ensure_compat_schema(&runtime).await?;
            self.runtime_pool = Some(runtime);
            self.pool = None;
            self.ensure_runtime_default_data().await?;
            tracing::warn!(
                "Database initialized in {:?} compatibility mode; PostgreSQL-specific features may be unavailable",
                config.db_type
            );
            return Ok(());
        }

        // Construct PostgreSQL connection string
        let conn_str = format!(
            "postgres://{}:{}@{}:{}/{}",
            config.username.as_deref().unwrap_or("postgres"),
            config.password.as_deref().unwrap_or("postgres"),
            config.host.as_deref().unwrap_or("localhost"),
            config.port.unwrap_or(5432),
            config.database.as_deref().unwrap_or("sentinel_ai")
        );

        tracing::info!("Connecting to database: {}", conn_str);

        let db_name = config.database.as_deref().unwrap_or("sentinel_ai");
        let pool = match PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(Duration::from_secs(config.query_timeout as u64))
            .connect(&conn_str)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("does not exist") {
                    tracing::info!("Database \"{}\" not found, creating it", db_name);
                    let maint_conn = format!(
                        "postgres://{}:{}@{}:{}/postgres",
                        config.username.as_deref().unwrap_or("postgres"),
                        config.password.as_deref().unwrap_or("postgres"),
                        config.host.as_deref().unwrap_or("localhost"),
                        config.port.unwrap_or(5432),
                    );
                    let maint_pool = PgPoolOptions::new()
                        .max_connections(1)
                        .connect(&maint_conn)
                        .await?;
                    let quoted = db_name.replace('"', "\"\"");
                    sqlx::query(&format!("CREATE DATABASE \"{}\"", quoted))
                        .execute(&maint_pool)
                        .await?;
                    drop(maint_pool);
                    PgPoolOptions::new()
                        .max_connections(config.max_connections)
                        .acquire_timeout(Duration::from_secs(config.query_timeout as u64))
                        .connect(&conn_str)
                        .await?
                } else {
                    return Err(e.into());
                }
            }
        };

        self.create_database_schema(&pool).await?;
        self.ensure_migrations(&pool).await?;
        self.insert_default_data(&pool).await?;
        
        self.runtime_pool = Some(DatabasePool::PostgreSQL(pool.clone()));
        self.pool = Some(pool);
        self.ensure_runtime_default_data().await?;
        Ok(())
    }

    fn default_pg_config(&self) -> DatabaseConfig {
        DatabaseConfig {
            db_type: DatabaseType::PostgreSQL,
            path: None,
            enable_wal: false,
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: Some("sentinel_ai".to_string()),
            username: Some("postgres".to_string()),
            password: Some("postgres".to_string()),
            enable_ssl: false,
            max_connections: 50,
            query_timeout: 30,
        }
    }

    async fn ensure_migrations(&self, pool: &PgPool) -> Result<()> {
        use tracing::info;

        // 确保 workflow_definitions 表有 category 和 tags 字段
        let has_category: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'workflow_definitions' AND column_name = 'category')"
        ).fetch_one(pool).await?;

        if !has_category {
            sqlx::query("ALTER TABLE workflow_definitions ADD COLUMN category TEXT").execute(pool).await?;
        }

        let has_tags: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'workflow_definitions' AND column_name = 'tags')"
        ).fetch_one(pool).await?;

        if !has_tags {
            sqlx::query("ALTER TABLE workflow_definitions ADD COLUMN tags TEXT").execute(pool).await?;
        }

        let has_is_tool: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'workflow_definitions' AND column_name = 'is_tool')"
        ).fetch_one(pool).await?;

        if !has_is_tool {
            sqlx::query("ALTER TABLE workflow_definitions ADD COLUMN is_tool BOOLEAN DEFAULT FALSE").execute(pool).await?;
        }

        // 确保 ai_messages 表有 reasoning_content 字段
        let has_reasoning_content: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'ai_messages' AND column_name = 'reasoning_content')"
        ).fetch_one(pool).await?;

        if !has_reasoning_content {
            info!("Adding reasoning_content column to ai_messages table");
            sqlx::query("ALTER TABLE ai_messages ADD COLUMN reasoning_content TEXT").execute(pool).await?;
        }

        // Ensure memory_executions table exists
        let memory_table_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'memory_executions')"
        ).fetch_one(pool).await?;

        if !memory_table_exists {
            info!("Creating memory_executions table...");
            sqlx::query(
                r#"CREATE TABLE IF NOT EXISTS memory_executions (
                    id TEXT PRIMARY KEY,
                    task TEXT NOT NULL,
                    environment TEXT,
                    tool_calls TEXT,
                    success BOOLEAN NOT NULL,
                    error TEXT,
                    response_excerpt TEXT,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
                )"#,
            )
            .execute(pool)
            .await?;

            sqlx::query(
                r#"CREATE INDEX IF NOT EXISTS idx_memory_executions_created_at
                   ON memory_executions(created_at)"#,
            )
            .execute(pool)
            .await?;
        }

        let agent_run_states_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'agent_run_states')"
        ).fetch_one(pool).await?;
        if !agent_run_states_exists {
            info!("Creating agent_run_states table...");
            sqlx::query(
                r#"CREATE TABLE IF NOT EXISTS agent_run_states (
                    execution_id TEXT PRIMARY KEY,
                    state_json TEXT NOT NULL,
                    updated_at BIGINT NOT NULL
                )"#,
            )
            .execute(pool)
            .await?;
        }

        // 检查并创建字典相关表
        let dict_table_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'dictionaries')"
        ).fetch_one(pool).await?;

        if !dict_table_exists {
            info!("Creating dictionaries tables...");
            
            sqlx::query(
                r#"CREATE TABLE IF NOT EXISTS dictionaries (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT,
                    dict_type TEXT NOT NULL,
                    service_type TEXT,
                    category TEXT,
                    is_builtin BOOLEAN DEFAULT FALSE,
                    is_active BOOLEAN DEFAULT TRUE,
                    word_count BIGINT DEFAULT 0,
                    file_size BIGINT DEFAULT 0,
                    checksum TEXT,
                    version TEXT DEFAULT '1.0.0',
                    author TEXT,
                    source_url TEXT,
                    tags TEXT,
                    metadata TEXT,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
                )"#
            ).execute(pool).await?;

            sqlx::query(
                r#"CREATE TABLE IF NOT EXISTS dictionary_words (
                    id TEXT PRIMARY KEY,
                    dictionary_id TEXT NOT NULL,
                    word TEXT NOT NULL,
                    weight REAL DEFAULT 1.0,
                    category TEXT,
                    metadata TEXT,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(dictionary_id) REFERENCES dictionaries(id) ON DELETE CASCADE
                )"#
            ).execute(pool).await?;

            sqlx::query(
                r#"CREATE INDEX IF NOT EXISTS idx_dictionary_words_dict_id 
                   ON dictionary_words(dictionary_id)"#
            ).execute(pool).await?;

            sqlx::query(
                r#"CREATE INDEX IF NOT EXISTS idx_dictionary_words_word 
                   ON dictionary_words(word)"#
            ).execute(pool).await?;

            sqlx::query(
                r#"CREATE TABLE IF NOT EXISTS dictionary_sets (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT,
                    service_type TEXT,
                    scenario TEXT,
                    is_active BOOLEAN DEFAULT TRUE,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
                )"#
            ).execute(pool).await?;

            sqlx::query(
                r#"CREATE TABLE IF NOT EXISTS dictionary_set_relations (
                    id TEXT PRIMARY KEY,
                    set_id TEXT NOT NULL,
                    dictionary_id TEXT NOT NULL,
                    priority INTEGER DEFAULT 0,
                    is_enabled BOOLEAN DEFAULT TRUE,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(set_id) REFERENCES dictionary_sets(id) ON DELETE CASCADE,
                    FOREIGN KEY(dictionary_id) REFERENCES dictionaries(id) ON DELETE CASCADE
                )"#
            ).execute(pool).await?;

            info!("Dictionaries tables created successfully");
        }

        // Ensure skills table exists and includes required columns
        let skills_table_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'skills')"
        ).fetch_one(pool).await?;

        if !skills_table_exists {
            info!("Creating skills table...");
            sqlx::query(
                r#"CREATE TABLE IF NOT EXISTS skills (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    description TEXT NOT NULL DEFAULT '',
                    content TEXT NOT NULL DEFAULT '',
                    argument_hint TEXT NOT NULL DEFAULT '',
                    disable_model_invocation BOOLEAN NOT NULL DEFAULT FALSE,
                    user_invocable BOOLEAN NOT NULL DEFAULT TRUE,
                    allowed_tools TEXT NOT NULL DEFAULT '[]',
                    model TEXT NOT NULL DEFAULT '',
                    context TEXT NOT NULL DEFAULT '',
                    agent TEXT NOT NULL DEFAULT '',
                    hooks TEXT NOT NULL DEFAULT '{}',
                    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
                    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
                )"#
            ).execute(pool).await?;
        } else {
            let required_columns = [
                ("source_path", "TEXT NOT NULL DEFAULT ''"),
                ("content", "TEXT NOT NULL DEFAULT ''"),
                ("argument_hint", "TEXT NOT NULL DEFAULT ''"),
                ("disable_model_invocation", "BOOLEAN NOT NULL DEFAULT FALSE"),
                ("user_invocable", "BOOLEAN NOT NULL DEFAULT TRUE"),
                ("allowed_tools", "TEXT NOT NULL DEFAULT '[]'"),
                ("model", "TEXT NOT NULL DEFAULT ''"),
                ("context", "TEXT NOT NULL DEFAULT ''"),
                ("agent", "TEXT NOT NULL DEFAULT ''"),
                ("hooks", "TEXT NOT NULL DEFAULT '{}'"),
            ];

            for (column, column_type) in required_columns {
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'skills' AND column_name = $1)"
                )
                .bind(column)
                .fetch_one(pool)
                .await?;

                if !exists {
                    let alter_query = format!("ALTER TABLE skills ADD COLUMN {} {}", column, column_type);
                    info!("Adding column '{}' to skills table", column);
                    sqlx::query(&alter_query).execute(pool).await?;
                }
            }
        }

        // Drop legacy ability_groups table if it exists
        let ability_groups_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'ability_groups')"
        ).fetch_one(pool).await?;

        if ability_groups_exists {
            info!("Dropping legacy ability_groups table");
            sqlx::query("DROP TABLE IF EXISTS ability_groups").execute(pool).await?;
        }

        // 检查并创建缓存表
        let cache_table_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'cache_storage')"
        ).fetch_one(pool).await?;

        if !cache_table_exists {
            info!("Creating cache_storage table...");
            
            sqlx::query(
                r#"CREATE TABLE IF NOT EXISTS cache_storage (
                    cache_key TEXT PRIMARY KEY,
                    cache_value TEXT NOT NULL,
                    cache_type TEXT NOT NULL,
                    version TEXT DEFAULT '1.0',
                    expires_at TIMESTAMP WITH TIME ZONE,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
                    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
                )"#
            ).execute(pool).await?;

            sqlx::query(
                r#"CREATE INDEX IF NOT EXISTS idx_cache_storage_type 
                   ON cache_storage(cache_type)"#
            ).execute(pool).await?;

            sqlx::query(
                r#"CREATE INDEX IF NOT EXISTS idx_cache_storage_expires 
                   ON cache_storage(expires_at)"#
            ).execute(pool).await?;

            info!("Cache storage table created successfully");
        }

        Ok(())
    }

    async fn ensure_compat_schema(&self, runtime: &DatabasePool) -> Result<()> {
        // Minimal cross-database schema to keep core flows available in non-PostgreSQL mode.
        let statements = [
            r#"CREATE TABLE IF NOT EXISTS configurations (
                id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT,
                description TEXT,
                is_encrypted BOOLEAN DEFAULT FALSE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#,
            r#"CREATE TABLE IF NOT EXISTS ai_conversations (
                id TEXT PRIMARY KEY,
                title TEXT,
                service_name TEXT NOT NULL,
                model_name TEXT NOT NULL,
                model_provider TEXT,
                context_type TEXT,
                project_id TEXT,
                vulnerability_id TEXT,
                scan_task_id TEXT,
                conversation_data TEXT,
                summary TEXT,
                total_messages INTEGER DEFAULT 0,
                total_tokens INTEGER DEFAULT 0,
                cost DOUBLE DEFAULT 0.0,
                tags TEXT,
                tool_config TEXT,
                is_archived BOOLEAN DEFAULT FALSE,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS ai_messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                token_count INTEGER,
                cost DOUBLE,
                tool_calls TEXT,
                attachments TEXT,
                reasoning_content TEXT,
                timestamp DATETIME NOT NULL,
                architecture_type TEXT,
                architecture_meta TEXT,
                structured_data TEXT
            )"#,
            r#"CREATE TABLE IF NOT EXISTS scan_tasks (
                id TEXT PRIMARY KEY,
                project_id TEXT,
                name TEXT NOT NULL,
                description TEXT,
                target_type TEXT NOT NULL,
                targets TEXT NOT NULL,
                scan_type TEXT NOT NULL,
                tools_config TEXT,
                status TEXT NOT NULL,
                progress DOUBLE DEFAULT 0.0,
                priority INTEGER DEFAULT 1,
                scheduled_at DATETIME,
                started_at DATETIME,
                completed_at DATETIME,
                execution_time INTEGER,
                results_summary TEXT,
                error_message TEXT,
                created_by TEXT NOT NULL,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS vulnerabilities (
                id TEXT PRIMARY KEY,
                project_id TEXT,
                asset_id TEXT,
                scan_task_id TEXT,
                title TEXT NOT NULL,
                description TEXT,
                vulnerability_type TEXT,
                severity TEXT NOT NULL,
                cvss_score DOUBLE,
                cvss_vector TEXT,
                cwe_id TEXT,
                owasp_category TEXT,
                proof_of_concept TEXT,
                impact TEXT,
                remediation TEXT,
                references_json TEXT,
                status TEXT NOT NULL,
                verification_status TEXT NOT NULL,
                resolution_date DATETIME,
                tags TEXT,
                attachments TEXT,
                notes TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS assets (
                id TEXT PRIMARY KEY,
                project_id TEXT,
                asset_type TEXT NOT NULL,
                name TEXT NOT NULL,
                value TEXT NOT NULL,
                description TEXT,
                confidence DOUBLE DEFAULT 1.0,
                status TEXT NOT NULL,
                source TEXT,
                source_scan_id TEXT,
                metadata TEXT,
                tags TEXT,
                risk_level TEXT,
                last_seen DATETIME NOT NULL,
                first_seen DATETIME NOT NULL,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                created_by TEXT NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS workflow_definitions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                graph_data TEXT NOT NULL,
                is_template BOOLEAN DEFAULT FALSE,
                is_tool BOOLEAN DEFAULT FALSE,
                category TEXT,
                tags TEXT,
                version TEXT NOT NULL,
                created_by TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#,
            r#"CREATE TABLE IF NOT EXISTS plugin_registry (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL DEFAULT '',
                version TEXT NOT NULL DEFAULT '1.0.0',
                author TEXT,
                main_category TEXT NOT NULL DEFAULT 'traffic',
                category TEXT NOT NULL DEFAULT 'vulnerability',
                description TEXT,
                default_severity TEXT NOT NULL DEFAULT 'medium',
                tags TEXT,
                enabled BOOLEAN NOT NULL DEFAULT FALSE,
                metadata TEXT NOT NULL DEFAULT '{}',
                code TEXT NOT NULL DEFAULT '',
                plugin_code TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                quality_score DOUBLE,
                validation_status TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#,
            r#"CREATE TABLE IF NOT EXISTS skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT NOT NULL DEFAULT '',
                source_path TEXT NOT NULL DEFAULT '',
                content TEXT NOT NULL DEFAULT '',
                argument_hint TEXT NOT NULL DEFAULT '',
                disable_model_invocation BOOLEAN NOT NULL DEFAULT FALSE,
                user_invocable BOOLEAN NOT NULL DEFAULT TRUE,
                allowed_tools TEXT NOT NULL DEFAULT '[]',
                model TEXT NOT NULL DEFAULT '',
                context TEXT NOT NULL DEFAULT '',
                agent TEXT NOT NULL DEFAULT '',
                hooks TEXT NOT NULL DEFAULT '{}',
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS traffic_vulnerabilities (
                id TEXT PRIMARY KEY,
                plugin_id TEXT NOT NULL,
                vuln_type TEXT NOT NULL,
                severity TEXT NOT NULL,
                confidence TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                cwe TEXT,
                owasp TEXT,
                remediation TEXT,
                status TEXT NOT NULL DEFAULT 'open',
                signature TEXT NOT NULL,
                first_seen_at DATETIME NOT NULL,
                last_seen_at DATETIME NOT NULL,
                hit_count INTEGER NOT NULL DEFAULT 1,
                session_id TEXT,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#,
            r#"CREATE TABLE IF NOT EXISTS agent_audit_findings (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                finding_id TEXT NOT NULL,
                signature TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                severity TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'open',
                confidence DOUBLE,
                cwe TEXT,
                files_json TEXT,
                source_json TEXT,
                sink_json TEXT,
                trace_path_json TEXT,
                evidence_json TEXT,
                fix TEXT,
                description TEXT NOT NULL,
                severity_raw TEXT,
                source_message_id TEXT,
                hit_count INTEGER NOT NULL DEFAULT 1,
                first_seen_at DATETIME NOT NULL,
                last_seen_at DATETIME NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#,
            r#"CREATE TABLE IF NOT EXISTS traffic_evidence (
                id TEXT PRIMARY KEY,
                vuln_id TEXT NOT NULL,
                url TEXT NOT NULL,
                method TEXT NOT NULL,
                location TEXT,
                evidence_snippet TEXT,
                request_headers TEXT,
                request_body TEXT,
                response_status INTEGER,
                response_headers TEXT,
                response_body TEXT,
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#,
            r#"CREATE TABLE IF NOT EXISTS traffic_dedupe_index (
                signature TEXT PRIMARY KEY,
                vuln_id TEXT NOT NULL,
                first_hit DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_hit DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#,
            r#"CREATE TABLE IF NOT EXISTS mcp_server_configs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                url TEXT NOT NULL,
                connection_type TEXT NOT NULL,
                command TEXT NOT NULL,
                args TEXT NOT NULL,
                is_enabled BOOLEAN DEFAULT TRUE,
                auto_connect BOOLEAN DEFAULT FALSE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#,
            r#"CREATE TABLE IF NOT EXISTS notification_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                channel TEXT NOT NULL,
                config TEXT,
                is_encrypted BOOLEAN DEFAULT FALSE,
                enabled BOOLEAN DEFAULT TRUE,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS memory_executions (
                id TEXT PRIMARY KEY,
                task TEXT NOT NULL,
                environment TEXT,
                tool_calls TEXT,
                success BOOLEAN NOT NULL,
                error TEXT,
                response_excerpt TEXT,
                created_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS agent_sessions (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                agent_name TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#,
            r#"CREATE TABLE IF NOT EXISTS agent_session_logs (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                level TEXT NOT NULL,
                message TEXT NOT NULL,
                source TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#,
            r#"CREATE TABLE IF NOT EXISTS dictionaries (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                dict_type TEXT NOT NULL,
                service_type TEXT,
                category TEXT,
                is_builtin BOOLEAN DEFAULT FALSE,
                is_active BOOLEAN DEFAULT TRUE,
                word_count BIGINT DEFAULT 0,
                file_size BIGINT DEFAULT 0,
                checksum TEXT,
                version TEXT DEFAULT '1.0.0',
                author TEXT,
                source_url TEXT,
                tags TEXT,
                metadata TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#,
            r#"CREATE TABLE IF NOT EXISTS dictionary_words (
                id TEXT PRIMARY KEY,
                dictionary_id TEXT NOT NULL,
                word TEXT NOT NULL,
                weight DOUBLE DEFAULT 1.0,
                category TEXT,
                metadata TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )"#,
            r#"CREATE TABLE IF NOT EXISTS rag_collections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                is_active BOOLEAN DEFAULT FALSE,
                document_count BIGINT DEFAULT 0,
                chunk_count BIGINT DEFAULT 0,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS rag_document_sources (
                id TEXT PRIMARY KEY,
                collection_id TEXT NOT NULL,
                file_path TEXT,
                file_name TEXT NOT NULL,
                file_type TEXT,
                file_size BIGINT,
                file_hash TEXT,
                content_hash TEXT,
                status TEXT DEFAULT 'Pending',
                chunk_count BIGINT DEFAULT 0,
                metadata TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
            r#"CREATE TABLE IF NOT EXISTS rag_chunks (
                id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                collection_id TEXT NOT NULL,
                content TEXT NOT NULL,
                content_hash TEXT,
                chunk_index INTEGER,
                char_count INTEGER,
                embedding BLOB,
                metadata TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL
            )"#,
        ];

        for sql in statements {
            self.execute_runtime_ddl(runtime, sql).await?;
        }

        self.execute_runtime_ddl(
            runtime,
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_configurations_category_key ON configurations(category, key)",
        )
        .await?;
        self.execute_runtime_ddl(
            runtime,
            "CREATE INDEX IF NOT EXISTS idx_memory_executions_created_at ON memory_executions(created_at)",
        )
        .await?;
        self.execute_runtime_ddl(
            runtime,
            "CREATE INDEX IF NOT EXISTS idx_traffic_vulnerabilities_signature ON traffic_vulnerabilities(signature)",
        )
        .await?;
        self.execute_runtime_ddl(runtime, "ALTER TABLE agent_audit_findings ADD COLUMN source_json TEXT")
            .await
            .ok();
        self.execute_runtime_ddl(runtime, "ALTER TABLE agent_audit_findings ADD COLUMN sink_json TEXT")
            .await
            .ok();
        self.execute_runtime_ddl(runtime, "ALTER TABLE agent_audit_findings ADD COLUMN trace_path_json TEXT")
            .await
            .ok();
        self.execute_runtime_ddl(runtime, "ALTER TABLE agent_audit_findings ADD COLUMN evidence_json TEXT")
            .await
            .ok();
        self.execute_runtime_ddl(runtime, "ALTER TABLE agent_audit_findings ADD COLUMN severity_raw TEXT")
            .await
            .ok();
        self.execute_runtime_ddl(
            runtime,
            "CREATE INDEX IF NOT EXISTS idx_agent_audit_findings_signature ON agent_audit_findings(signature)",
        )
        .await?;
        self.execute_runtime_ddl(
            runtime,
            "CREATE INDEX IF NOT EXISTS idx_agent_audit_findings_conversation ON agent_audit_findings(conversation_id)",
        )
        .await?;
        self.execute_runtime_ddl(
            runtime,
            "CREATE INDEX IF NOT EXISTS idx_agent_audit_findings_last_seen ON agent_audit_findings(last_seen_at)",
        )
        .await?;
        self.execute_runtime_ddl(
            runtime,
            "CREATE INDEX IF NOT EXISTS idx_traffic_evidence_vuln_id ON traffic_evidence(vuln_id)",
        )
        .await?;

        Ok(())
    }

    async fn ensure_runtime_default_data(&self) -> Result<()> {
        if self
            .get_config_internal("ai", "providers_config")
            .await?
            .is_none()
        {
            self.set_config_internal(
                "ai",
                "providers_config",
                "{}",
                Some("AI provider configurations"),
            )
            .await?;
        }

        Ok(())
    }

    async fn execute_runtime_ddl(&self, runtime: &DatabasePool, sql: &str) -> Result<()> {
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(sql).execute(pool).await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(sql).execute(pool).await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(sql).execute(pool).await?;
            }
        }
        Ok(())
    }

    async fn execute_sql_script(&self, runtime: &DatabasePool, sql_script: &str) -> Result<()> {
        for statement in split_sql_statements(sql_script) {
            if statement.trim().is_empty() {
                continue;
            }
            self.execute_runtime_ddl(runtime, &statement).await?;
        }
        Ok(())
    }

    /// 执行自定义查询
    pub async fn execute_query(&self, query: &str) -> Result<Vec<Value>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let rows = sqlx::query(query).fetch_all(pool).await?;
                Ok(rows_to_json_pg(rows))
            }
            DatabasePool::SQLite(pool) => {
                let rows = sqlx::query(query).fetch_all(pool).await?;
                Ok(rows_to_json_sqlite(rows))
            }
            DatabasePool::MySQL(pool) => {
                let rows = sqlx::query(query).fetch_all(pool).await?;
                Ok(rows_to_json_mysql(rows))
            }
        }
    }

    /// 获取数据库统计信息
    pub async fn get_stats_internal(&self) -> Result<DatabaseStats> {
        let scan_tasks_count = self.safe_count_query("SELECT COUNT(*) as cnt FROM scan_tasks").await;
        let vulnerabilities_count = self
            .safe_count_query("SELECT COUNT(*) as cnt FROM vulnerabilities")
            .await;
        let assets_count = self.safe_count_query("SELECT COUNT(*) as cnt FROM assets").await;
        let conversations_count = self
            .safe_count_query("SELECT COUNT(*) as cnt FROM ai_conversations")
            .await;

        let db_size: i64 = match self.config.as_ref().map(|c| &c.db_type) {
            Some(DatabaseType::PostgreSQL) => count_from_result(
                self.execute_query("SELECT pg_database_size(current_database()) as cnt")
                    .await
                    .unwrap_or_default(),
            ),
            Some(DatabaseType::MySQL) => count_from_result(
                self.execute_query("SELECT COALESCE(SUM(data_length + index_length),0) as cnt FROM information_schema.tables WHERE table_schema = DATABASE()")
                    .await
                    .unwrap_or_default(),
            ),
            _ => 0,
        };

        Ok(DatabaseStats {
            scan_tasks_count: scan_tasks_count as f64,
            vulnerabilities_count: vulnerabilities_count as f64,
            assets_count: assets_count as f64,
            conversations_count: conversations_count as f64,
            db_size_bytes: db_size as u64,
            last_backup: None,
        })
    }

    async fn safe_count_query(&self, sql: &str) -> i64 {
        match self.execute_query(sql).await {
            Ok(rows) => count_from_result(rows),
            Err(e) => {
                tracing::debug!("Count query skipped for '{}': {}", sql, e);
                0
            }
        }
    }
}

fn count_from_result(rows: Vec<Value>) -> i64 {
    rows.first()
        .and_then(|row| row.as_object())
        .and_then(|obj| obj.get("cnt"))
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok())))
        .unwrap_or(0)
}

fn rows_to_json_pg(rows: Vec<sqlx::postgres::PgRow>) -> Vec<Value> {
    rows_to_json_internal(rows)
}

fn rows_to_json_sqlite(rows: Vec<sqlx::sqlite::SqliteRow>) -> Vec<Value> {
    rows_to_json_internal(rows)
}

fn rows_to_json_mysql(rows: Vec<sqlx::mysql::MySqlRow>) -> Vec<Value> {
    rows_to_json_internal(rows)
}

fn rows_to_json_internal<R>(rows: Vec<R>) -> Vec<Value>
where
    R: Row,
    usize: sqlx::ColumnIndex<R>,
    String: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    i64: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    f64: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    bool: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    chrono::DateTime<chrono::Utc>: for<'r> sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    let mut results = Vec::new();
    for row in rows {
        let mut obj = serde_json::Map::new();

        for (i, column) in row.columns().iter().enumerate() {
            let column_name = column.name();
            let type_name = column.type_info().name().to_uppercase();

            let value: Value = match type_name.as_str() {
                "TEXT" | "VARCHAR" | "CHAR" | "NAME" | "BPCHAR" => {
                    let val: Option<String> = row.try_get(i).ok();
                    val.map(Value::String).unwrap_or(Value::Null)
                }
                "INT8" | "BIGINT" | "INT4" | "INTEGER" | "INT2" | "SMALLINT" | "INT" | "TINYINT" | "MEDIUMINT" => {
                    let val: Option<i64> = row.try_get(i).ok();
                    val.map(|v| Value::Number(v.into())).unwrap_or(Value::Null)
                }
                "FLOAT8" | "DOUBLE PRECISION" | "FLOAT4" | "REAL" | "NUMERIC" | "DOUBLE" | "FLOAT" | "DECIMAL" => {
                    let val: Option<f64> = row.try_get(i).ok();
                    val.map(|v| Value::Number(serde_json::Number::from_f64(v).unwrap_or_else(|| 0.into())))
                        .unwrap_or(Value::Null)
                }
                "BOOL" | "BOOLEAN" => {
                    let val: Option<bool> = row.try_get(i).ok();
                    val.map(Value::Bool).unwrap_or(Value::Null)
                }
                "TIMESTAMPTZ" | "TIMESTAMP" | "DATETIME" | "DATE" => {
                    let val: Option<chrono::DateTime<chrono::Utc>> = row.try_get(i).ok();
                    val.map(|v| Value::String(v.to_rfc3339())).unwrap_or(Value::Null)
                }
                _ => {
                    if let Ok(val) = row.try_get::<i64, _>(i) {
                        Value::Number(val.into())
                    } else if let Ok(val) = row.try_get::<f64, _>(i) {
                        Value::Number(serde_json::Number::from_f64(val).unwrap_or_else(|| 0.into()))
                    } else if let Ok(val) = row.try_get::<String, _>(i) {
                        Value::String(val)
                    } else if let Ok(val) = row.try_get::<bool, _>(i) {
                        Value::Bool(val)
                    } else {
                        Value::Null
                    }
                }
            };

            obj.insert(column_name.to_string(), value);
        }

        results.push(Value::Object(obj));
    }
    results
}

impl Default for DatabaseService {
    fn default() -> Self {
        Self::new()
    }
}

fn split_sql_statements(script: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = script.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
                current.push(ch);
            }
            '"' if !in_single => {
                in_double = !in_double;
                current.push(ch);
            }
            ';' if !in_single && !in_double => {
                let stmt = current.trim();
                if !stmt.is_empty() && !stmt.starts_with("--") {
                    out.push(stmt.to_string());
                }
                current.clear();
            }
            '-' if !in_single && !in_double && chars.peek() == Some(&'-') => {
                let _ = chars.next();
                for next in chars.by_ref() {
                    if next == '\n' {
                        break;
                    }
                }
            }
            _ => current.push(ch),
        }
    }

    let tail = current.trim();
    if !tail.is_empty() && !tail.starts_with("--") {
        out.push(tail.to_string());
    }
    out
}

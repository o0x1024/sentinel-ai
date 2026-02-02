use anyhow::Result;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Column, Row, TypeInfo};
use std::path::PathBuf;
use serde_json::Value;
use crate::core::models::database::DatabaseStats;
use crate::database_service::db_config::{DatabaseConfig, DatabaseType};
use std::sync::Arc;
use tokio::sync::Semaphore;
use std::time::Duration;

#[derive(Debug, Clone)]
/// 数据库服务
pub struct DatabaseService {
    pub(crate) pool: Option<PgPool>,
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
            config: None,
            write_semaphore: Arc::new(Semaphore::new(10)), // Higher limit for PG
        }
    }

    pub fn get_pool(&self) -> Result<&PgPool> {
        self.pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))
    }

    /// Get database pool (public method for external use)
    pub fn pool(&self) -> &PgPool {
        self.pool.as_ref().expect("Database not initialized")
    }
    
    // Deprecated exact match for SQLite but kept for interface compatibility if generic
    pub fn get_postgres_pool(&self) -> Result<&PgPool> {
        self.get_pool()
    }

    pub fn get_db(&self) -> Result<crate::client::DatabaseClient> {
        let pool = self.get_pool()?.clone();
        Ok(crate::client::DatabaseClient::new(pool))
    }

    pub fn get_sqlite_pool(&self) -> Result<sqlx::SqlitePool> {
        // This is a bit of a hack for the migration commands.
        // If we are in PG mode, we might not have a SQLite pool.
        // For now, let's try to connect to the default SQLite path if requested.
        Err(anyhow::anyhow!("SQLite pool not available in PostgreSQL mode. Please use migration tools with explicit source config."))
    }

    pub fn get_db_path(&self) -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sentinel-ai")
            .join("database.db")
    }

    pub async fn backup(&self, path: Option<PathBuf>) -> Result<PathBuf> {
        let backup_path = path.unwrap_or_else(|| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("sentinel-ai")
                .join(format!("backup_{}.sql", chrono::Utc::now().timestamp()))
        });
        // Implementation for PG backup would go here
        Ok(backup_path)
    }

    pub async fn restore(&self, _path: PathBuf) -> Result<()> {
        // Implementation for PG restore would go here
        Ok(())
    }

    /// 初始化数据库
    pub async fn initialize(&mut self) -> Result<()> {
        // Try to load config from file
        let config_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sentinel-ai")
            .join("db_config.json");

        let config: DatabaseConfig = if config_path.exists() {
             let content = std::fs::read_to_string(&config_path)?;
             match serde_json::from_str(&content) {
                 Ok(c) => c,
                 Err(e) => {
                     tracing::warn!("Failed to parse db_config.json: {}, using default", e);
                     self.default_pg_config()
                 }
             }
        } else {
             self.default_pg_config()
        };

        self.config = Some(config.clone());

        // Construct connection string based on config
        let conn_str = match config.db_type {
            DatabaseType::PostgreSQL => {
                format!(
                    "postgres://{}:{}@{}:{}/{}",
                    config.username.as_deref().unwrap_or("postgres"),
                    config.password.as_deref().unwrap_or("postgres"),
                    config.host.as_deref().unwrap_or("localhost"),
                    config.port.unwrap_or(5432),
                    config.database.as_deref().unwrap_or("sentinel_ai")
                )
            }
            DatabaseType::MySQL => {
                format!(
                    "mysql://{}:{}@{}:{}/{}",
                    config.username.as_deref().unwrap_or("root"),
                    config.password.as_deref().unwrap_or(""),
                    config.host.as_deref().unwrap_or("localhost"),
                    config.port.unwrap_or(3306),
                    config.database.as_deref().unwrap_or("sentinel_ai")
                )
            }
            DatabaseType::SQLite => {
                // Not supported by PgPool, but keeping structure for future
                return Err(anyhow::anyhow!("SQLite not supported by PgPool implementation"));
            }
        };

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
        
        self.pool = Some(pool);
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

        // 确保 ability_groups 表有 additional_notes 字段
        let has_additional_notes: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'ability_groups' AND column_name = 'additional_notes')"
        ).fetch_one(pool).await?;

        if !has_additional_notes {
            info!("Adding additional_notes column to ability_groups table");
            sqlx::query("ALTER TABLE ability_groups ADD COLUMN additional_notes TEXT DEFAULT ''").execute(pool).await?;
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

    /// 执行自定义查询
    pub async fn execute_query(&self, query: &str) -> Result<Vec<Value>> {
        let pool = self.get_pool()?;

        let rows = sqlx::query(query).fetch_all(pool).await?;

        let mut results = Vec::new();
        for row in rows {
            let mut obj = serde_json::Map::new();

            for (i, column) in row.columns().iter().enumerate() {
                let column_name = column.name();
                let type_name = column.type_info().name();
                
                // Postgres types mapping
                let value: Value = match type_name {
                    "TEXT" | "VARCHAR" | "CHAR" | "NAME" | "bpchar" => {
                        let val: Option<String> = row.try_get(i).ok();
                        val.map(Value::String).unwrap_or(Value::Null)
                    }
                    "INT8" | "BIGINT" | "INT4" | "INTEGER" | "INT2" | "SMALLINT" => {
                        let val: Option<i64> = row.try_get(i).ok();
                        val.map(|v| Value::Number(v.into())).unwrap_or(Value::Null)
                    }
                    "FLOAT8" | "DOUBLE PRECISION" | "FLOAT4" | "REAL" | "NUMERIC" => {
                        let val: Option<f64> = row.try_get(i).ok();
                        val.map(|v| {
                            Value::Number(
                                serde_json::Number::from_f64(v).unwrap_or_else(|| 0.into()),
                            )
                        })
                        .unwrap_or(Value::Null)
                    }
                    "BOOL" | "BOOLEAN" => {
                        let val: Option<bool> = row.try_get(i).ok();
                        val.map(Value::Bool).unwrap_or(Value::Null)
                    }
                    "TIMESTAMPTZ" | "TIMESTAMP" => {
                        let val: Option<chrono::DateTime<chrono::Utc>> = row.try_get(i).ok();
                        val.map(|v| Value::String(v.to_rfc3339())).unwrap_or(Value::Null)
                    }
                    "JSON" | "JSONB" => {
                        let val: Option<serde_json::Value> = row.try_get(i).ok();
                        val.unwrap_or(Value::Null)
                    }
                    "UUID" => {
                        let val: Option<uuid::Uuid> = row.try_get(i).ok();
                        val.map(|v| Value::String(v.to_string())).unwrap_or(Value::Null)
                    }
                    _ => {
                         // Fallback attempts
                        if let Ok(val) = row.try_get::<i64, _>(i) {
                            Value::Number(val.into())
                        } else if let Ok(val) = row.try_get::<f64, _>(i) {
                            Value::Number(serde_json::Number::from_f64(val).unwrap_or_else(|| 0.into()))
                        } else if let Ok(val) = row.try_get::<String, _>(i) {
                            Value::String(val)
                        } else if let Ok(val) = row.try_get::<bool, _>(i) {
                            Value::Bool(val)
                        } else if let Ok(val) = row.try_get::<serde_json::Value, _>(i) {
                            val
                        } else {
                            Value::Null
                        }
                    }
                };
                obj.insert(column_name.to_string(), value);
            }

            results.push(Value::Object(obj));
        }

        Ok(results)
    }

    /// 获取数据库统计信息
    pub async fn get_stats_internal(&self) -> Result<DatabaseStats> {
        let pool = self.get_pool()?;

        let scan_tasks_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scan_tasks")
            .fetch_one(pool)
            .await?;

        let vulnerabilities_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vulnerabilities")
            .fetch_one(pool)
            .await?;

        let assets_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assets")
            .fetch_one(pool)
            .await?;

        let conversations_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ai_conversations")
            .fetch_one(pool)
            .await?;

        // DB size usually requires system admin perms in PG, return 0 for now or query a simple estimate
        let db_size: i64 = sqlx::query_scalar("SELECT pg_database_size(current_database())")
             .fetch_one(pool)
             .await
             .unwrap_or(0);

        Ok(DatabaseStats {
            scan_tasks_count: scan_tasks_count as f64,
            vulnerabilities_count: vulnerabilities_count as f64,
            assets_count: assets_count as f64,
            conversations_count: conversations_count as f64,
            db_size_bytes: db_size as u64,
            last_backup: None,
        })
    }
}

impl Default for DatabaseService {
    fn default() -> Self {
        Self::new()
    }
}

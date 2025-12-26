use anyhow::Result;
use sqlx::{sqlite::SqlitePool, Column, Row, TypeInfo};
use std::path::PathBuf;
use serde_json::Value;
use crate::core::models::database::DatabaseStats;
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Debug, Clone)]
/// 数据库服务
pub struct DatabaseService {
    pub(crate) pool: Option<SqlitePool>,
    pub(crate) db_path: PathBuf,
    pub(crate) write_semaphore: Arc<Semaphore>,
}

impl DatabaseService {
    pub fn new() -> Self {
        let db_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sentinel-ai")
            .join("database.db");

        // Limit concurrent writes to 1 to prevent database locking
        Self {
            pool: None,
            db_path,
            write_semaphore: Arc::new(Semaphore::new(1)),
        }
    }

    pub fn get_pool(&self) -> Result<&SqlitePool> {
        self.pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))
    }

    pub fn get_db(&self) -> Result<crate::client::DatabaseClient> {
        let pool = self.get_pool()?.clone();
        Ok(crate::client::DatabaseClient::new(pool))
    }

    pub fn get_db_path(&self) -> PathBuf {
        self.db_path.clone()
    }

    /// 初始化数据库
    pub async fn initialize(&mut self) -> Result<()> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db_url = format!("sqlite:{}?mode=rwc", self.db_path.to_string_lossy());
        let pool = SqlitePool::connect(&db_url).await?;
        
        // Enable WAL mode for better concurrent write handling
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;
        
        // Set busy timeout to 5 seconds for locked database retry
        sqlx::query("PRAGMA busy_timeout=5000")
            .execute(&pool)
            .await?;
        
        self.create_database_schema(&pool).await?;
        self.ensure_migrations(&pool).await?;
        self.insert_default_data(&pool).await?;
        
        self.pool = Some(pool);
        Ok(())
    }

    async fn ensure_migrations(&self, pool: &SqlitePool) -> Result<()> {
        use tracing::info;

        // 确保 workflow_definitions 表有 category 和 tags 字段
        let rows = sqlx::query("PRAGMA table_info(workflow_definitions)")
            .fetch_all(pool)
            .await?;
        
        let mut has_category = false;
        let mut has_tags = false;
        let mut has_is_tool = false;

        for row in rows {
            let name: String = sqlx::Row::get(&row, "name");
            if name == "category" { has_category = true; }
            if name == "tags" { has_tags = true; }
            if name == "is_tool" { has_is_tool = true; }
        }

        if !has_category {
            sqlx::query("ALTER TABLE workflow_definitions ADD COLUMN category TEXT")
                .execute(pool)
                .await?;
        }
        if !has_tags {
            sqlx::query("ALTER TABLE workflow_definitions ADD COLUMN tags TEXT")
                .execute(pool)
                .await?;
        }
        if !has_is_tool {
            sqlx::query("ALTER TABLE workflow_definitions ADD COLUMN is_tool BOOLEAN DEFAULT 0")
                .execute(pool)
                .await?;
        }

        // 确保 ai_messages 表有 reasoning_content 字段
        let ai_messages_rows = sqlx::query("PRAGMA table_info(ai_messages)")
            .fetch_all(pool)
            .await?;
        
        let mut has_reasoning_content = false;
        for row in ai_messages_rows {
            let name: String = sqlx::Row::get(&row, "name");
            if name == "reasoning_content" { 
                has_reasoning_content = true;
                break;
            }
        }

        if !has_reasoning_content {
            info!("Adding reasoning_content column to ai_messages table");
            sqlx::query("ALTER TABLE ai_messages ADD COLUMN reasoning_content TEXT")
                .execute(pool)
                .await?;
        }

        // 检查并创建字典相关表
        let table_exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='dictionaries'"
        )
        .fetch_one(pool)
        .await?;

        if table_exists == 0 {
            info!("Creating dictionaries tables...");
            
            sqlx::query(
                r#"CREATE TABLE IF NOT EXISTS dictionaries (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT,
                    dict_type TEXT NOT NULL,
                    service_type TEXT,
                    category TEXT,
                    is_builtin BOOLEAN DEFAULT 0,
                    is_active BOOLEAN DEFAULT 1,
                    word_count INTEGER DEFAULT 0,
                    file_size INTEGER DEFAULT 0,
                    checksum TEXT,
                    version TEXT DEFAULT '1.0.0',
                    author TEXT,
                    source_url TEXT,
                    tags TEXT,
                    metadata TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
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
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
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
                    is_active BOOLEAN DEFAULT 1,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                )"#
            ).execute(pool).await?;

            sqlx::query(
                r#"CREATE TABLE IF NOT EXISTS dictionary_set_relations (
                    id TEXT PRIMARY KEY,
                    set_id TEXT NOT NULL,
                    dictionary_id TEXT NOT NULL,
                    priority INTEGER DEFAULT 0,
                    is_enabled BOOLEAN DEFAULT 1,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(set_id) REFERENCES dictionary_sets(id) ON DELETE CASCADE,
                    FOREIGN KEY(dictionary_id) REFERENCES dictionaries(id) ON DELETE CASCADE
                )"#
            ).execute(pool).await?;

            info!("Dictionaries tables created successfully");
        }

        // 检查并创建缓存表
        let cache_table_exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='cache_storage'"
        )
        .fetch_one(pool)
        .await?;

        if cache_table_exists == 0 {
            info!("Creating cache_storage table...");
            
            sqlx::query(
                r#"CREATE TABLE IF NOT EXISTS cache_storage (
                    cache_key TEXT PRIMARY KEY,
                    cache_value TEXT NOT NULL,
                    cache_type TEXT NOT NULL,
                    version TEXT DEFAULT '1.0',
                    expires_at DATETIME,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
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
                
                let value: Value = match type_name {
                    "TEXT" | "VARCHAR" | "CHAR" | "CLOB" => {
                        let val: Option<String> = row.try_get(i)?;
                        val.map(Value::String).unwrap_or(Value::Null)
                    }
                    "INTEGER" | "INT" | "BIGINT" | "SMALLINT" | "TINYINT" => {
                        let val: Option<i64> = row.try_get(i)?;
                        val.map(|v| Value::Number(v.into())).unwrap_or(Value::Null)
                    }
                    "REAL" | "FLOAT" | "DOUBLE" | "NUMERIC" | "DECIMAL" => {
                        let val: Option<f64> = row.try_get(i)?;
                        val.map(|v| {
                            Value::Number(
                                serde_json::Number::from_f64(v).unwrap_or_else(|| 0.into()),
                            )
                        })
                        .unwrap_or(Value::Null)
                    }
                    "BOOLEAN" | "BOOL" => {
                        let val: Option<bool> = row.try_get(i)?;
                        val.map(Value::Bool).unwrap_or(Value::Null)
                    }
                    "NULL" => Value::Null,
                    _ => {
                        if let Ok(Some(val)) = row.try_get::<Option<i64>, _>(i) {
                            Value::Number(val.into())
                        } else if let Ok(Some(val)) = row.try_get::<Option<f64>, _>(i) {
                            Value::Number(serde_json::Number::from_f64(val).unwrap_or_else(|| 0.into()))
                        } else if let Ok(Some(val)) = row.try_get::<Option<String>, _>(i) {
                            Value::String(val)
                        } else if let Ok(Some(val)) = row.try_get::<Option<bool>, _>(i) {
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

        let db_size = std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(DatabaseStats {
            scan_tasks_count: scan_tasks_count as f64,
            vulnerabilities_count: vulnerabilities_count as f64,
            assets_count: assets_count as f64,
            conversations_count: conversations_count as f64,
            db_size_bytes: db_size,
            last_backup: None,
        })
    }

    /// 备份数据库
    pub async fn backup(&self, backup_path: Option<PathBuf>) -> Result<PathBuf> {
        let backup_path = backup_path.unwrap_or_else(|| {
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            self.db_path
                .parent()
                .unwrap_or(&PathBuf::from("."))
                .join(format!("backup_{}.db", timestamp))
        });

        std::fs::copy(&self.db_path, &backup_path)?;

        tracing::info!("Database backup completed: {}", backup_path.display());
        Ok(backup_path)
    }

    /// 恢复数据库
    pub async fn restore(&self, backup_path: PathBuf) -> Result<()> {
        std::fs::copy(&backup_path, &self.db_path)?;
        tracing::info!("Database restoration completed: {}", backup_path.display());
        Ok(())
    }

    /// Retry database operation with exponential backoff for locked database
    pub async fn retry_on_locked<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        const MAX_RETRIES: u32 = 5;
        const INITIAL_DELAY_MS: u64 = 10;
        
        let mut retries = 0;
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    let err_msg = e.to_string();
                    let is_locked = err_msg.contains("database is locked") 
                        || err_msg.contains("SQLITE_BUSY")
                        || err_msg.contains("code: 5");
                    
                    if is_locked && retries < MAX_RETRIES {
                        retries += 1;
                        let delay_ms = INITIAL_DELAY_MS * 2u64.pow(retries - 1);
                        tracing::debug!(
                            "Database locked, retry {}/{} after {}ms",
                            retries,
                            MAX_RETRIES,
                            delay_ms
                        );
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }
}

impl Default for DatabaseService {
    fn default() -> Self {
        Self::new()
    }
}

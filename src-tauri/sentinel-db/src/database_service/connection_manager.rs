use anyhow::{Result, Context};
use sqlx::postgres::PgPool;
use sqlx::mysql::MySqlPool;
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;

use super::db_config::{DatabaseConfig, DatabaseType};

#[derive(Debug, Clone)]
pub enum DatabasePool {
    PostgreSQL(PgPool),
    SQLite(sqlx::SqlitePool),
    MySQL(MySqlPool),
}

impl DatabasePool {
    pub async fn connect(config: &DatabaseConfig) -> Result<Self> {
        info!("Connecting to database: {:?}", config.db_type);
        
        let connection_string = config.build_connection_string();
        
        match config.db_type {
            DatabaseType::PostgreSQL => {
                let pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(config.max_connections)
                    .acquire_timeout(Duration::from_secs(config.query_timeout))
                    .connect(&connection_string)
                    .await
                    .context("Failed to connect to PostgreSQL database")?;
                
                Ok(DatabasePool::PostgreSQL(pool))
            }
            DatabaseType::SQLite => {
                let sqlite_path = config.path.clone().unwrap_or_else(|| {
                    dirs::data_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join("sentinel-ai")
                        .join("database.db")
                        .to_string_lossy()
                        .to_string()
                });
                if let Some(parent) = PathBuf::from(&sqlite_path).parent() {
                    std::fs::create_dir_all(parent)
                        .context("Failed to create SQLite database directory")?;
                }

                let pool = sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(config.max_connections)
                    .acquire_timeout(Duration::from_secs(config.query_timeout))
                    .connect(&connection_string)
                    .await
                    .context("Failed to connect to SQLite database")?;

                if config.enable_wal {
                    sqlx::query("PRAGMA journal_mode = WAL")
                        .execute(&pool)
                        .await
                        .context("Failed to enable SQLite WAL mode")?;
                }
                
                Ok(DatabasePool::SQLite(pool))
            }
            DatabaseType::MySQL => {
                let pool = sqlx::mysql::MySqlPoolOptions::new()
                    .max_connections(config.max_connections)
                    .acquire_timeout(Duration::from_secs(config.query_timeout))
                    .connect(&connection_string)
                    .await
                    .context("Failed to connect to MySQL database")?;

                Ok(DatabasePool::MySQL(pool))
            }
        }
    }
    
    pub async fn test_connection(&self) -> Result<()> {
        match self {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
        }
        Ok(())
    }
    
    pub fn db_type(&self) -> DatabaseType {
        match self {
            DatabasePool::PostgreSQL(_) => DatabaseType::PostgreSQL,
            DatabasePool::SQLite(_) => DatabaseType::SQLite,
            DatabasePool::MySQL(_) => DatabaseType::MySQL,
        }
    }
}

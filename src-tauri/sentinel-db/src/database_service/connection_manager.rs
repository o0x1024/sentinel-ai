use anyhow::{Result, Context};
use sqlx::{
    sqlite::SqlitePool,
    postgres::PgPool,
    mysql::MySqlPool,
};
use std::time::Duration;
use tracing::info;

use super::db_config::{DatabaseConfig, DatabaseType};

#[derive(Debug, Clone)]
pub enum DatabasePool {
    SQLite(SqlitePool),
    PostgreSQL(PgPool),
    MySQL(MySqlPool),
}

impl DatabasePool {
    pub async fn connect(config: &DatabaseConfig) -> Result<Self> {
        info!("Connecting to database: {:?}", config.db_type);
        
        let connection_string = config.build_connection_string();
        
        match config.db_type {
            DatabaseType::SQLite => {
                let pool = sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(config.max_connections)
                    .acquire_timeout(Duration::from_secs(config.query_timeout))
                    .connect(&connection_string)
                    .await
                    .context("Failed to connect to SQLite database")?;
                
                // Enable WAL mode
                if config.enable_wal {
                    sqlx::query("PRAGMA journal_mode=WAL")
                        .execute(&pool)
                        .await?;
                }
                
                // Set busy timeout
                sqlx::query("PRAGMA busy_timeout=5000")
                    .execute(&pool)
                    .await?;
                
                Ok(DatabasePool::SQLite(pool))
            }
            DatabaseType::PostgreSQL => {
                let pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(config.max_connections)
                    .acquire_timeout(Duration::from_secs(config.query_timeout))
                    .connect(&connection_string)
                    .await
                    .context("Failed to connect to PostgreSQL database")?;
                
                Ok(DatabasePool::PostgreSQL(pool))
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
            DatabasePool::SQLite(pool) => {
                sqlx::query("SELECT 1").execute(pool).await?;
            }
            DatabasePool::PostgreSQL(pool) => {
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
            DatabasePool::SQLite(_) => DatabaseType::SQLite,
            DatabasePool::PostgreSQL(_) => DatabaseType::PostgreSQL,
            DatabasePool::MySQL(_) => DatabaseType::MySQL,
        }
    }
}

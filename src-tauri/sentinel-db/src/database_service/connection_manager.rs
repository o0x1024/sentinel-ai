use anyhow::{Result, Context};
use sqlx::postgres::PgPool;
use std::time::Duration;
use tracing::info;

use super::db_config::{DatabaseConfig, DatabaseType};

#[derive(Debug, Clone)]
pub enum DatabasePool {
    PostgreSQL(PgPool),
    SQLite(sqlx::SqlitePool),
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
                let pool = sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(config.max_connections)
                    .acquire_timeout(Duration::from_secs(config.query_timeout))
                    .connect(&connection_string)
                    .await
                    .context("Failed to connect to SQLite database")?;
                
                Ok(DatabasePool::SQLite(pool))
            }
            _ => Err(anyhow::anyhow!("Database type {:?} is not supported in this build", config.db_type)),
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
        }
        Ok(())
    }
    
    pub fn db_type(&self) -> DatabaseType {
        match self {
            DatabasePool::PostgreSQL(_) => DatabaseType::PostgreSQL,
            DatabasePool::SQLite(_) => DatabaseType::SQLite,
        }
    }
}

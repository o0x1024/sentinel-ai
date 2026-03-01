use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;

impl DatabaseService {
    pub async fn get_cache_internal(&self, key: &str) -> Result<Option<String>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now();

        let value = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let row = sqlx::query(
                    "SELECT cache_value, expires_at FROM cache_storage 
                     WHERE cache_key = $1
                     AND (expires_at IS NULL OR expires_at > $2)",
                )
                .bind(key)
                .bind(now)
                .fetch_optional(pool)
                .await?;
                row.map(|r| r.get("cache_value"))
            }
            DatabasePool::SQLite(pool) => {
                let row = sqlx::query(
                    "SELECT cache_value, expires_at FROM cache_storage 
                     WHERE cache_key = ?
                     AND (expires_at IS NULL OR expires_at > ?)",
                )
                .bind(key)
                .bind(now)
                .fetch_optional(pool)
                .await?;
                row.map(|r| r.get("cache_value"))
            }
            DatabasePool::MySQL(pool) => {
                let row = sqlx::query(
                    "SELECT cache_value, expires_at FROM cache_storage 
                     WHERE cache_key = ?
                     AND (expires_at IS NULL OR expires_at > ?)",
                )
                .bind(key)
                .bind(now)
                .fetch_optional(pool)
                .await?;
                row.map(|r| r.get("cache_value"))
            }
        };

        Ok(value)
    }

    pub async fn set_cache_internal(
        &self,
        key: &str,
        value: &str,
        cache_type: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now();

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO cache_storage (cache_key, cache_value, cache_type, expires_at, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6)
                     ON CONFLICT(cache_key) DO UPDATE SET
                        cache_value = excluded.cache_value,
                        cache_type = excluded.cache_type,
                        expires_at = excluded.expires_at,
                        updated_at = excluded.updated_at",
                )
                .bind(key)
                .bind(value)
                .bind(cache_type)
                .bind(expires_at)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO cache_storage (cache_key, cache_value, cache_type, expires_at, created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?)
                     ON CONFLICT(cache_key) DO UPDATE SET
                        cache_value = excluded.cache_value,
                        cache_type = excluded.cache_type,
                        expires_at = excluded.expires_at,
                        updated_at = excluded.updated_at",
                )
                .bind(key)
                .bind(value)
                .bind(cache_type)
                .bind(expires_at)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO cache_storage (cache_key, cache_value, cache_type, expires_at, created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?)
                     ON DUPLICATE KEY UPDATE
                        cache_value = VALUES(cache_value),
                        cache_type = VALUES(cache_type),
                        expires_at = VALUES(expires_at),
                        updated_at = VALUES(updated_at)",
                )
                .bind(key)
                .bind(value)
                .bind(cache_type)
                .bind(expires_at)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn delete_cache_internal(&self, key: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM cache_storage WHERE cache_key = $1")
                    .bind(key)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM cache_storage WHERE cache_key = ?")
                    .bind(key)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM cache_storage WHERE cache_key = ?")
                    .bind(key)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn cleanup_expired_cache_internal(&self) -> Result<u64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let now = Utc::now();

        let affected = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let result = sqlx::query(
                    "DELETE FROM cache_storage WHERE expires_at IS NOT NULL AND expires_at <= $1",
                )
                .bind(now)
                .execute(pool)
                .await?;
                result.rows_affected()
            }
            DatabasePool::SQLite(pool) => {
                let result = sqlx::query(
                    "DELETE FROM cache_storage WHERE expires_at IS NOT NULL AND expires_at <= ?",
                )
                .bind(now)
                .execute(pool)
                .await?;
                result.rows_affected()
            }
            DatabasePool::MySQL(pool) => {
                let result = sqlx::query(
                    "DELETE FROM cache_storage WHERE expires_at IS NOT NULL AND expires_at <= ?",
                )
                .bind(now)
                .execute(pool)
                .await?;
                result.rows_affected()
            }
        };

        Ok(affected)
    }

    pub async fn get_all_cache_keys_internal(
        &self,
        cache_type: Option<String>,
    ) -> Result<Vec<String>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        let keys = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                if let Some(t) = cache_type {
                    let rows =
                        sqlx::query("SELECT cache_key FROM cache_storage WHERE cache_type = $1")
                            .bind(t)
                            .fetch_all(pool)
                            .await?;
                    rows.iter().map(|r| r.get("cache_key")).collect()
                } else {
                    let rows = sqlx::query("SELECT cache_key FROM cache_storage")
                        .fetch_all(pool)
                        .await?;
                    rows.iter().map(|r| r.get("cache_key")).collect()
                }
            }
            DatabasePool::SQLite(pool) => {
                if let Some(t) = cache_type {
                    let rows =
                        sqlx::query("SELECT cache_key FROM cache_storage WHERE cache_type = ?")
                            .bind(t)
                            .fetch_all(pool)
                            .await?;
                    rows.iter().map(|r| r.get("cache_key")).collect()
                } else {
                    let rows = sqlx::query("SELECT cache_key FROM cache_storage")
                        .fetch_all(pool)
                        .await?;
                    rows.iter().map(|r| r.get("cache_key")).collect()
                }
            }
            DatabasePool::MySQL(pool) => {
                if let Some(t) = cache_type {
                    let rows =
                        sqlx::query("SELECT cache_key FROM cache_storage WHERE cache_type = ?")
                            .bind(t)
                            .fetch_all(pool)
                            .await?;
                    rows.iter().map(|r| r.get("cache_key")).collect()
                } else {
                    let rows = sqlx::query("SELECT cache_key FROM cache_storage")
                        .fetch_all(pool)
                        .await?;
                    rows.iter().map(|r| r.get("cache_key")).collect()
                }
            }
        };

        Ok(keys)
    }
}

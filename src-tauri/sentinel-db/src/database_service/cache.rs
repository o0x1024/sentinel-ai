use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::Row;
use crate::database_service::service::DatabaseService;

impl DatabaseService {
    pub async fn get_cache_internal(&self, key: &str) -> Result<Option<String>> {
        let pool = self.get_pool()?;
        let now = Utc::now();

        let row = sqlx::query(
            "SELECT cache_value, expires_at FROM cache_storage 
             WHERE cache_key = ?
             AND (expires_at IS NULL OR expires_at > ?)",
        )
        .bind(key)
        .bind(now)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| r.get("cache_value")))
    }

    pub async fn set_cache_internal(&self, key: &str, value: &str, cache_type: &str, expires_at: Option<DateTime<Utc>>) -> Result<()> {
        let pool = self.get_pool()?;
        let now = Utc::now();

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

        Ok(())
    }

    pub async fn delete_cache_internal(&self, key: &str) -> Result<()> {
        let pool = self.get_pool()?;
        sqlx::query("DELETE FROM cache_storage WHERE cache_key = ?")
            .bind(key)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn cleanup_expired_cache_internal(&self) -> Result<u64> {
        let pool = self.get_pool()?;
        let now = Utc::now();

        let result = sqlx::query("DELETE FROM cache_storage WHERE expires_at IS NOT NULL AND expires_at <= ?")
            .bind(now)
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }

    pub async fn get_all_cache_keys_internal(&self, cache_type: Option<String>) -> Result<Vec<String>> {
        let pool = self.get_pool()?;
        
        let rows = if let Some(t) = cache_type {
            sqlx::query("SELECT cache_key FROM cache_storage WHERE cache_type = ?")
                .bind(t)
                .fetch_all(pool)
                .await?
        } else {
            sqlx::query("SELECT cache_key FROM cache_storage")
                .fetch_all(pool)
                .await?
        };

        let keys: Vec<String> = rows.iter().map(|r| r.get("cache_key")).collect();
        Ok(keys)
    }
}

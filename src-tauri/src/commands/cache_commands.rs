use crate::AppState;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheData {
    pub cache_key: String,
    pub cache_value: String,
    pub cache_type: String,
    pub version: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetCacheRequest {
    pub key: String,
    pub value: String,
    pub cache_type: String,
    pub ttl_minutes: Option<i64>, // 过期时间（分钟）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCacheResponse {
    pub success: bool,
    pub data: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetCacheResponse {
    pub success: bool,
    pub error: Option<String>,
}

/// 获取缓存
#[tauri::command]
pub async fn get_cache(
    state: State<'_, Arc<AppState>>,
    key: String,
) -> Result<GetCacheResponse, String> {
    let db_service = state.db_service.lock().await;
    let pool = db_service
        .get_pool()
        .ok_or_else(|| "Database not initialized".to_string())?;

    match get_cache_internal(pool, &key).await {
        Ok(Some(value)) => Ok(GetCacheResponse {
            success: true,
            data: Some(value),
            error: None,
        }),
        Ok(None) => Ok(GetCacheResponse {
            success: true,
            data: None,
            error: None,
        }),
        Err(e) => Ok(GetCacheResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        }),
    }
}

/// 设置缓存
#[tauri::command]
pub async fn set_cache(
    state: State<'_, Arc<AppState>>,
    request: SetCacheRequest,
) -> Result<SetCacheResponse, String> {
    let db_service = state.db_service.lock().await;
    let pool = db_service
        .get_pool()
        .ok_or_else(|| "Database not initialized".to_string())?;

    match set_cache_internal(pool, request).await {
        Ok(_) => Ok(SetCacheResponse {
            success: true,
            error: None,
        }),
        Err(e) => Ok(SetCacheResponse {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}

/// 删除缓存
#[tauri::command]
pub async fn delete_cache(
    state: State<'_, Arc<AppState>>,
    key: String,
) -> Result<SetCacheResponse, String> {
    let db_service = state.db_service.lock().await;
    let pool = db_service
        .get_pool()
        .ok_or_else(|| "Database not initialized".to_string())?;

    match delete_cache_internal(pool, &key).await {
        Ok(_) => Ok(SetCacheResponse {
            success: true,
            error: None,
        }),
        Err(e) => Ok(SetCacheResponse {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}

/// 清理过期缓存
#[tauri::command]
pub async fn cleanup_expired_cache(
    state: State<'_, Arc<AppState>>,
) -> Result<SetCacheResponse, String> {
    let db_service = state.db_service.lock().await;
    let pool = db_service
        .get_pool()
        .ok_or_else(|| "Database not initialized".to_string())?;

    match cleanup_expired_cache_internal(pool).await {
        Ok(count) => {
            tracing::info!("Cleaned up {} expired cache entries", count);
            Ok(SetCacheResponse {
                success: true,
                error: None,
            })
        }
        Err(e) => Ok(SetCacheResponse {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}

/// 获取缓存（内部实现）
async fn get_cache_internal(pool: &sqlx::SqlitePool, key: &str) -> Result<Option<String>> {
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

/// 设置缓存（内部实现）
async fn set_cache_internal(pool: &sqlx::SqlitePool, request: SetCacheRequest) -> Result<()> {
    let now = Utc::now();
    let expires_at = request
        .ttl_minutes
        .map(|ttl| now + Duration::minutes(ttl));

    sqlx::query(
        "INSERT INTO cache_storage (cache_key, cache_value, cache_type, expires_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(cache_key) DO UPDATE SET
            cache_value = excluded.cache_value,
            cache_type = excluded.cache_type,
            expires_at = excluded.expires_at,
            updated_at = excluded.updated_at",
    )
    .bind(&request.key)
    .bind(&request.value)
    .bind(&request.cache_type)
    .bind(expires_at)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

/// 删除缓存（内部实现）
async fn delete_cache_internal(pool: &sqlx::SqlitePool, key: &str) -> Result<()> {
    sqlx::query("DELETE FROM cache_storage WHERE cache_key = ?")
        .bind(key)
        .execute(pool)
        .await?;

    Ok(())
}

/// 清理过期缓存（内部实现）
async fn cleanup_expired_cache_internal(pool: &sqlx::SqlitePool) -> Result<u64> {
    let now = Utc::now();

    let result = sqlx::query("DELETE FROM cache_storage WHERE expires_at IS NOT NULL AND expires_at <= ?")
        .bind(now)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

/// 获取所有缓存键（用于调试）
#[tauri::command]
pub async fn get_all_cache_keys(
    state: State<'_, Arc<AppState>>,
    cache_type: Option<String>,
) -> Result<Vec<String>, String> {
    let db_service = state.db_service.lock().await;
    let pool = db_service
        .get_pool()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let query = if let Some(t) = cache_type {
        sqlx::query("SELECT cache_key FROM cache_storage WHERE cache_type = ?")
            .bind(t)
            .fetch_all(pool)
            .await
    } else {
        sqlx::query("SELECT cache_key FROM cache_storage")
            .fetch_all(pool)
            .await
    };

    match query {
        Ok(rows) => {
            let keys: Vec<String> = rows.iter().map(|r| r.get("cache_key")).collect();
            Ok(keys)
        }
        Err(e) => Err(e.to_string()),
    }
}


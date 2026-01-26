use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use sentinel_db::Database;

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
    db_service: State<'_, Arc<dyn Database>>,
    key: String,
) -> Result<GetCacheResponse, String> {
    match db_service.get_cache(&key).await {
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
    db_service: State<'_, Arc<dyn Database>>,
    request: SetCacheRequest,
) -> Result<SetCacheResponse, String> {
    let now = Utc::now();
    let expires_at = request
        .ttl_minutes
        .map(|ttl| now + Duration::minutes(ttl));

    match db_service.set_cache(&request.key, &request.value, &request.cache_type, expires_at).await {
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
    db_service: State<'_, Arc<dyn Database>>,
    key: String,
) -> Result<SetCacheResponse, String> {
    match db_service.delete_cache(&key).await {
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
    db_service: State<'_, Arc<dyn Database>>,
) -> Result<SetCacheResponse, String> {
    match db_service.cleanup_expired_cache().await {
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

/// 获取所有缓存键（用于调试）
#[tauri::command]
pub async fn get_all_cache_keys(
    db_service: State<'_, Arc<dyn Database>>,
    cache_type: Option<String>,
) -> Result<Vec<String>, String> {
    db_service.get_all_cache_keys(cache_type)
        .await
        .map_err(|e| e.to_string())
}


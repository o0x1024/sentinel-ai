use sentinel_db::core::models::scan_session::*;
use crate::services::DatabaseService;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;
use sentinel_db::Database;

#[derive(Debug, Serialize, Deserialize)]
pub struct ListSessionsRequest {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub status_filter: Option<ScanSessionStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionResponse {
    pub success: bool,
    pub data: Option<ScanSession>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionsListResponse {
    pub success: bool,
    pub data: Vec<ScanSession>,
    pub total: usize,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProgressResponse {
    pub success: bool,
    pub data: Option<ScanProgress>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StagesResponse {
    pub success: bool,
    pub data: Vec<ScanStage>,
    pub message: Option<String>,
}

/// 创建扫描会话
#[tauri::command]
pub async fn create_scan_session(
    request: CreateScanSessionRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<SessionResponse, String> {
    match db.inner().create_scan_session(request).await {
        Ok(session) => Ok(SessionResponse {
            success: true,
            data: Some(session),
            message: None,
        }),
        Err(e) => Ok(SessionResponse {
            success: false,
            data: None,
            message: Some(format!("创建扫描会话失败: {}", e)),
        }),
    }
}

/// 获取扫描会话详情
#[tauri::command]
pub async fn get_scan_session(
    session_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<SessionResponse, String> {
    let uuid = Uuid::parse_str(&session_id).map_err(|e| format!("无效的会话ID: {}", e))?;

    match db.inner().get_scan_session(uuid).await {
        Ok(session) => Ok(SessionResponse {
            success: true,
            data: session,
            message: None,
        }),
        Err(e) => Ok(SessionResponse {
            success: false,
            data: None,
            message: Some(format!("获取扫描会话失败: {}", e)),
        }),
    }
}

/// 更新扫描会话
#[tauri::command]
pub async fn update_scan_session(
    session_id: String,
    request: UpdateScanSessionRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<SessionResponse, String> {
    let uuid = Uuid::parse_str(&session_id).map_err(|e| format!("无效的会话ID: {}", e))?;

    match db.inner().update_scan_session(uuid, request).await {
        Ok(_) => {
            // 获取更新后的会话
            match db.inner().get_scan_session(uuid).await {
                Ok(session) => Ok(SessionResponse {
                    success: true,
                    data: session,
                    message: Some("扫描会话更新成功".to_string()),
                }),
                Err(e) => Ok(SessionResponse {
                    success: false,
                    data: None,
                    message: Some(format!("获取更新后的会话失败: {}", e)),
                }),
            }
        }
        Err(e) => Ok(SessionResponse {
            success: false,
            data: None,
            message: Some(format!("更新扫描会话失败: {}", e)),
        }),
    }
}

/// 列出扫描会话
#[tauri::command]
pub async fn list_scan_sessions(
    request: ListSessionsRequest,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<SessionsListResponse, String> {
    match db.inner()
        .list_scan_sessions(request.limit, request.offset, request.status_filter)
        .await
    {
        Ok(sessions) => {
            let total = sessions.len();
            Ok(SessionsListResponse {
                success: true,
                data: sessions,
                total,
                message: None,
            })
        }
        Err(e) => Ok(SessionsListResponse {
            success: false,
            data: vec![],
            total: 0,
            message: Some(format!("获取扫描会话列表失败: {}", e)),
        }),
    }
}

/// 删除扫描会话
#[tauri::command]
pub async fn delete_scan_session(
    session_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<SessionResponse, String> {
    let uuid = Uuid::parse_str(&session_id).map_err(|e| format!("无效的会话ID: {}", e))?;

    match db.inner().delete_scan_session(uuid).await {
        Ok(_) => Ok(SessionResponse {
            success: true,
            data: None,
            message: Some("扫描会话删除成功".to_string()),
        }),
        Err(e) => Ok(SessionResponse {
            success: false,
            data: None,
            message: Some(format!("删除扫描会话失败: {}", e)),
        }),
    }
}

/// 获取扫描进度
#[tauri::command]
pub async fn get_scan_progress(
    session_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<ProgressResponse, String> {
    let uuid = Uuid::parse_str(&session_id).map_err(|e| format!("无效的会话ID: {}", e))?;

    match db.inner().get_scan_progress(uuid).await {
        Ok(progress) => Ok(ProgressResponse {
            success: true,
            data: progress,
            message: None,
        }),
        Err(e) => Ok(ProgressResponse {
            success: false,
            data: None,
            message: Some(format!("获取扫描进度失败: {}", e)),
        }),
    }
}

/// 获取会话的扫描阶段
#[tauri::command]
pub async fn get_session_stages(
    session_id: String,
    db: State<'_, Arc<DatabaseService>>,
) -> Result<StagesResponse, String> {
    let uuid = Uuid::parse_str(&session_id).map_err(|e| format!("无效的会话ID: {}", e))?;

    match db.inner().get_scan_session_stages(uuid).await {
        Ok(stages) => Ok(StagesResponse {
            success: true,
            data: stages,
            message: None,
        }),
        Err(e) => Ok(StagesResponse {
            success: false,
            data: vec![],
            message: Some(format!("获取扫描阶段失败: {}", e)),
        }),
    }
}

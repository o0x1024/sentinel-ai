use crate::models::scan::{ScanConfig, ScanResult, ScanTask, TaskStats};
use crate::services::scan::ScanService;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

/// 创建扫描任务
#[tauri::command]
pub async fn create_scan_task(
    target: String,
    config: ScanConfig,
    scan_service: State<'_, Arc<ScanService>>,
) -> Result<ScanTask, String> {
    scan_service
        .create_task(target, config)
        .await
        .map_err(|e| e.to_string())
}

/// 启动扫描任务
#[tauri::command]
pub async fn start_scan_task(
    task_id: String,
    scan_service: State<'_, Arc<ScanService>>,
) -> Result<(), String> {
    let task_uuid = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    scan_service
        .start_task(task_uuid)
        .await
        .map_err(|e| e.to_string())
}

/// 停止扫描任务
#[tauri::command]
pub async fn stop_scan_task(
    task_id: String,
    scan_service: State<'_, Arc<ScanService>>,
) -> Result<(), String> {
    let task_uuid = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    scan_service
        .stop_task(task_uuid)
        .await
        .map_err(|e| e.to_string())
}

/// 获取扫描任务列表
#[tauri::command]
pub async fn get_scan_tasks(
    scan_service: State<'_, Arc<ScanService>>,
) -> Result<Vec<ScanTask>, String> {
    scan_service.list_tasks().await.map_err(|e| e.to_string())
}

/// 获取扫描任务详情
#[tauri::command]
pub async fn get_scan_task(
    task_id: String,
    scan_service: State<'_, Arc<ScanService>>,
) -> Result<ScanTask, String> {
    let task_uuid = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    scan_service
        .get_task(task_uuid)
        .await
        .map_err(|e| e.to_string())
}

/// 获取扫描结果
#[tauri::command]
pub async fn get_scan_results(
    task_id: String,
    scan_service: State<'_, Arc<ScanService>>,
) -> Result<Vec<ScanResult>, String> {
    let task_uuid = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    scan_service
        .get_results(task_uuid)
        .await
        .map_err(|e| e.to_string())
}

/// 删除扫描任务
#[tauri::command]
pub async fn delete_scan_task(
    task_id: String,
    scan_service: State<'_, Arc<ScanService>>,
) -> Result<(), String> {
    let task_uuid = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    scan_service
        .delete_task(task_uuid)
        .await
        .map_err(|e| e.to_string())
}

/// 获取扫描任务统计信息
#[tauri::command]
pub async fn get_scan_task_stats(
    scan_service: State<'_, Arc<ScanService>>,
) -> Result<TaskStats, String> {
    scan_service
        .get_task_stats()
        .await
        .map_err(|e| e.to_string())
}

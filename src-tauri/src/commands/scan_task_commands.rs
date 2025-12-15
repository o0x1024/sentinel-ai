//! Scan task commands module

use crate::services::database::DatabaseService;
use sentinel_core::models::database::ScanTask;
use std::sync::Arc;
use tauri::State;

/// Get scan tasks
#[tauri::command]
pub async fn get_scan_tasks(
    project_id: Option<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<ScanTask>, String> {
    let pool = db_service
        .get_pool()
        .map_err(|e| format!("Database pool not available: {}", e))?;
    
    sentinel_db::database::scan_task_dao::get_scan_tasks(pool, project_id.as_deref())
        .await
        .map_err(|e| format!("Failed to get scan tasks: {}", e))
}

/// Create scan task
#[tauri::command]
pub async fn create_scan_task(
    task: ScanTask,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    let pool = db_service
        .get_pool()
        .map_err(|e| format!("Database pool not available: {}", e))?;
    
    sentinel_db::database::scan_task_dao::create_scan_task(pool, &task)
        .await
        .map_err(|e| format!("Failed to create scan task: {}", e))
}

/// Update scan task status
#[tauri::command]
pub async fn update_scan_task_status(
    task_id: String,
    status: String,
    progress: Option<f64>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    let pool = db_service
        .get_pool()
        .map_err(|e| format!("Database pool not available: {}", e))?;
    
    sentinel_db::database::scan_task_dao::update_scan_task_status(pool, &task_id, &status, progress)
        .await
        .map_err(|e| format!("Failed to update scan task status: {}", e))
}

/// Delete scan task
#[tauri::command]
pub async fn delete_scan_task(
    task_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    let pool = db_service
        .get_pool()
        .map_err(|e| format!("Database pool not available: {}", e))?;
    
    sentinel_db::database::scan_task_dao::delete_scan_task(pool, &task_id)
        .await
        .map_err(|e| format!("Failed to delete scan task: {}", e))
}

/// Stop scan task
#[tauri::command]
pub async fn stop_scan_task(
    task_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    let pool = db_service
        .get_pool()
        .map_err(|e| format!("Database pool not available: {}", e))?;
    
    sentinel_db::database::scan_task_dao::stop_scan_task(pool, &task_id)
        .await
        .map_err(|e| format!("Failed to stop scan task: {}", e))
}
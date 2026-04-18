use std::sync::Arc;

use tauri::{AppHandle, Manager};

use crate::memory::DurableMemoryDiagnosticsItem;
use sentinel_db::core::models::database::DurableMemoryProjectionState;

#[tauri::command]
pub async fn list_durable_memory_diagnostics(
    app_handle: AppHandle,
    limit: Option<usize>,
    only_issues: Option<bool>,
) -> Result<Vec<DurableMemoryDiagnosticsItem>, String> {
    let db_service = app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
        .ok_or_else(|| "DatabaseService not available".to_string())?;
    let db = db_service.inner().clone();
    let effective_limit = limit.unwrap_or(20).clamp(1, 100) as i64;
    let issue_only = only_issues.unwrap_or(false);

    let records = db
        .list_recent_durable_memory_records_internal(effective_limit)
        .await
        .map_err(|e| e.to_string())?;
    let mut diagnostics = Vec::with_capacity(records.len());

    for record in records {
        let projection = db
            .get_durable_memory_projection_state_internal(&record.id)
            .await
            .map_err(|e| e.to_string())?;
        let item = DurableMemoryDiagnosticsItem::from_parts(record, projection);
        if !issue_only || item.projection_issue {
            diagnostics.push(item);
        }
    }

    Ok(diagnostics)
}

#[tauri::command]
pub async fn get_durable_memory_projection_states(
    app_handle: AppHandle,
    memory_ids: Vec<String>,
) -> Result<Vec<DurableMemoryProjectionState>, String> {
    let db_service = app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
        .ok_or_else(|| "DatabaseService not available".to_string())?;
    let db = db_service.inner().clone();

    let mut states = Vec::new();
    for memory_id in memory_ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
    {
        if let Some(state) = db
            .get_durable_memory_projection_state_internal(&memory_id)
            .await
            .map_err(|e| e.to_string())?
        {
            states.push(state);
        }
    }

    Ok(states)
}

#[tauri::command]
pub async fn get_durable_memory_diagnostics_by_ids(
    app_handle: AppHandle,
    memory_ids: Vec<String>,
) -> Result<Vec<DurableMemoryDiagnosticsItem>, String> {
    let db_service = app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
        .ok_or_else(|| "DatabaseService not available".to_string())?;
    let db = db_service.inner().clone();

    let mut diagnostics = Vec::new();
    for memory_id in memory_ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
    {
        let Some(record) = db
            .get_durable_memory_record_internal(&memory_id)
            .await
            .map_err(|e| e.to_string())?
        else {
            continue;
        };
        let projection = db
            .get_durable_memory_projection_state_internal(&memory_id)
            .await
            .map_err(|e| e.to_string())?;
        diagnostics.push(DurableMemoryDiagnosticsItem::from_parts(record, projection));
    }

    Ok(diagnostics)
}

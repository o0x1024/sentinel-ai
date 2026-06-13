use std::sync::Arc;

use tauri::{AppHandle, Manager};

use crate::memory::{
    delete_durable_memory, preview_memory_retrieval, set_durable_memory_auto_inject,
    update_durable_memory, DurableMemoryAutoInjectResult, DurableMemoryDeleteResult,
    DurableMemoryDiagnosticsItem, DurableMemoryUpdateResult, MemoryRetrieveOutcome,
};
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

#[tauri::command]
pub async fn update_durable_memory_command(
    app_handle: AppHandle,
    memory_id: String,
    text: String,
    title: Option<String>,
    kind: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<DurableMemoryUpdateResult, String> {
    update_durable_memory(&app_handle, memory_id, text, title, kind, tags)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn delete_durable_memory_command(
    app_handle: AppHandle,
    memory_id: String,
) -> Result<DurableMemoryDeleteResult, String> {
    delete_durable_memory(&app_handle, memory_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn set_durable_memory_auto_inject_command(
    app_handle: AppHandle,
    memory_id: String,
    enabled: bool,
) -> Result<DurableMemoryAutoInjectResult, String> {
    set_durable_memory_auto_inject(&app_handle, memory_id, enabled)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn preview_memory_retrieval_command(
    app_handle: AppHandle,
    query: String,
    top_k: Option<usize>,
) -> Result<MemoryRetrieveOutcome, String> {
    preview_memory_retrieval(&app_handle, query, top_k.unwrap_or(8))
        .await
        .map_err(|error| error.to_string())
}

use std::sync::Arc;

use sentinel_db::{
    DatabaseService, SurfaceAssetDetailResponse, SurfaceAssetFilter, SurfaceDiscoveryRunRow,
    SurfaceObservationRow, SurfaceOverview, SurfaceRelationFilter, SurfaceRelationRow,
    SurfaceTopologyResponse,
};
use tauri::State;

#[tauri::command]
pub async fn surface_get_overview(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
) -> Result<SurfaceOverview, String> {
    db_service
        .get_surface_overview(program_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_list_assets(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: SurfaceAssetFilter,
) -> Result<Vec<sentinel_db::SurfaceAssetRow>, String> {
    db_service
        .list_surface_assets(&filter)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_list_relations(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: SurfaceRelationFilter,
) -> Result<Vec<SurfaceRelationRow>, String> {
    db_service
        .list_surface_relations(&filter)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_list_discovery_runs(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<SurfaceDiscoveryRunRow>, String> {
    db_service
        .list_surface_discovery_runs(program_id.as_deref(), limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_get_topology(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
    limit: Option<i64>,
) -> Result<SurfaceTopologyResponse, String> {
    db_service
        .get_surface_topology(program_id.as_deref(), limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_get_asset_detail(
    db_service: State<'_, Arc<DatabaseService>>,
    asset_id: String,
) -> Result<Option<SurfaceAssetDetailResponse>, String> {
    db_service
        .get_surface_asset_detail(&asset_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_create_observation(
    db_service: State<'_, Arc<DatabaseService>>,
    observation: SurfaceObservationRow,
) -> Result<(), String> {
    db_service
        .create_surface_observation(&observation)
        .await
        .map_err(|e| e.to_string())
}

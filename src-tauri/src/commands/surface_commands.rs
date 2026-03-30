use std::sync::Arc;

use sentinel_db::{
    DatabaseService, SurfaceAssetDetailResponse, SurfaceAssetFilter,
    SurfaceDiscoveryRunDetailResponse, SurfaceDiscoveryRunRow, SurfaceFingerprintAssetFilter,
    SurfaceFingerprintAssetInventoryResponse, SurfaceFingerprintCategoryAggregation,
    SurfaceInventoryFacetsResponse, SurfaceInventoryResponse, SurfaceObservationRow,
    SurfaceOverview, SurfaceRelationFilter, SurfaceRelationRow, SurfaceTopologyResponse,
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
pub async fn surface_get_fingerprint_category_aggregation(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
    category_limit: Option<i64>,
    product_limit: Option<i64>,
) -> Result<SurfaceFingerprintCategoryAggregation, String> {
    db_service
        .get_surface_fingerprint_category_aggregation(
            program_id.as_deref(),
            category_limit,
            product_limit,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_list_fingerprint_assets(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: SurfaceFingerprintAssetFilter,
) -> Result<SurfaceFingerprintAssetInventoryResponse, String> {
    db_service
        .list_surface_fingerprint_assets(&filter)
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
pub async fn surface_list_inventory(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: SurfaceAssetFilter,
) -> Result<SurfaceInventoryResponse, String> {
    db_service
        .list_surface_inventory(&filter)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn surface_get_inventory_facets(
    db_service: State<'_, Arc<DatabaseService>>,
    filter: SurfaceAssetFilter,
) -> Result<SurfaceInventoryFacetsResponse, String> {
    db_service
        .get_surface_inventory_facets(&filter)
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
    node_limit: Option<i64>,
    node_offset: Option<i64>,
    edge_limit: Option<i64>,
    edge_offset: Option<i64>,
) -> Result<SurfaceTopologyResponse, String> {
    db_service
        .get_surface_topology(
            program_id.as_deref(),
            node_limit,
            node_offset,
            edge_limit,
            edge_offset,
        )
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
pub async fn surface_get_discovery_run_detail(
    db_service: State<'_, Arc<DatabaseService>>,
    run_id: String,
) -> Result<Option<SurfaceDiscoveryRunDetailResponse>, String> {
    db_service
        .get_surface_discovery_run_detail(&run_id)
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

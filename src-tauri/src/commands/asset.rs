use crate::models::asset::*;
use crate::services::AssetService;
use crate::database::AssetDao;
use sqlx::SqlitePool;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

/// 初始化资产服务
#[tauri::command]
pub async fn init_asset_service() -> Result<(), String> {
    // 资产服务现在通过依赖注入在应用启动时初始化
    println!("Asset service initialized");
    Ok(())
}

/// 创建资产
#[tauri::command]
pub async fn create_asset(
    asset_service: State<'_, AssetService>,
    request: CreateAssetRequest,
    created_by: String,
) -> Result<Asset, String> {
    asset_service.create_asset(request, created_by).await
}

/// 获取资产详情
#[tauri::command]
pub async fn get_asset_detail(
    asset_service: State<'_, AssetService>,
    id: String,
) -> Result<Option<AssetDetail>, String> {
    asset_service.get_asset_detail(&id).await
}

/// 更新资产
#[tauri::command]
pub async fn update_asset(
    asset_service: State<'_, AssetService>,
    id: String,
    request: UpdateAssetRequest,
) -> Result<bool, String> {
    asset_service.update_asset(&id, request).await
}

/// 删除资产
#[tauri::command]
pub async fn delete_asset(
    asset_service: State<'_, AssetService>,
    id: String,
) -> Result<bool, String> {
    asset_service.delete_asset(&id).await
}

/// 查询资产列表
#[tauri::command]
pub async fn list_assets(
    asset_service: State<'_, AssetService>,
    filter: Option<AssetFilter>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<Asset>, String> {
    asset_service.list_assets(filter, limit, offset).await
}

/// 获取资产统计信息
#[tauri::command]
pub async fn get_asset_stats(
    asset_service: State<'_, AssetService>,
) -> Result<AssetStats, String> {
    asset_service.get_asset_stats().await
}

/// 创建资产关系
#[tauri::command]
pub async fn create_asset_relationship(
    asset_service: State<'_, AssetService>,
    source_asset_id: String,
    target_asset_id: String,
    relationship_type: RelationshipType,
    created_by: String,
) -> Result<AssetRelationship, String> {
    asset_service.create_relationship(source_asset_id, target_asset_id, relationship_type, created_by).await
}

/// 批量导入资产
#[tauri::command]
pub async fn import_assets(
    asset_service: State<'_, AssetService>,
    request: ImportAssetsRequest,
    created_by: String,
) -> Result<ImportResult, String> {
    asset_service.import_assets(request, created_by).await
}

/// 从扫描结果中提取资产
#[tauri::command]
pub async fn extract_assets_from_scan(
    asset_service: State<'_, AssetService>,
    scan_id: String,
    scan_results: HashMap<String, Value>,
    created_by: String,
) -> Result<Vec<Asset>, String> {
    asset_service.extract_assets_from_scan(&scan_id, &scan_results, created_by).await
}

/// 搜索资产
#[tauri::command]
pub async fn search_assets(
    asset_service: State<'_, AssetService>,
    query: String,
    asset_types: Option<Vec<AssetType>>,
    limit: Option<u32>,
) -> Result<Vec<Asset>, String> {
    asset_service.search_assets(&query, asset_types, limit).await
}

/// 获取相关资产
#[tauri::command]
pub async fn get_related_assets(
    asset_service: State<'_, AssetService>,
    asset_id: String,
) -> Result<Vec<Asset>, String> {
    asset_service.get_related_assets(&asset_id).await
}

/// 验证资产
#[tauri::command]
pub async fn verify_asset(
    asset_service: State<'_, AssetService>,
    asset_id: String,
) -> Result<bool, String> {
    asset_service.verify_asset(&asset_id).await
}

/// 更新资产最后发现时间
#[tauri::command]
pub async fn update_asset_last_seen(
    asset_service: State<'_, AssetService>,
    asset_id: String,
) -> Result<bool, String> {
    asset_service.update_last_seen(&asset_id).await
}

/// 获取资产类型列表
#[tauri::command]
pub async fn get_asset_types() -> Result<Vec<String>, String> {
    Ok(vec![
        "domain".to_string(),
        "subdomain".to_string(),
        "ip".to_string(),
        "port".to_string(),
        "service".to_string(),
        "website".to_string(),
        "api".to_string(),
        "certificate".to_string(),
        "fingerprint".to_string(),
        "vulnerability".to_string(),
        "technology".to_string(),
        "email".to_string(),
        "phone".to_string(),
        "file".to_string(),
        "directory".to_string(),
    ])
}

/// 获取风险等级列表
#[tauri::command]
pub async fn get_risk_levels() -> Result<Vec<String>, String> {
    Ok(vec![
        "low".to_string(),
        "medium".to_string(),
        "high".to_string(),
        "critical".to_string(),
        "unknown".to_string(),
    ])
}

/// 获取资产状态列表
#[tauri::command]
pub async fn get_asset_statuses() -> Result<Vec<String>, String> {
    Ok(vec![
        "active".to_string(),
        "inactive".to_string(),
        "verified".to_string(),
        "unverified".to_string(),
    ])
}

/// 获取关系类型列表
#[tauri::command]
pub async fn get_relationship_types() -> Result<Vec<String>, String> {
    Ok(vec![
        "belongs_to".to_string(),
        "contains".to_string(),
        "connects_to".to_string(),
        "depends_on".to_string(),
        "resolves_to".to_string(),
        "hosts".to_string(),
        "uses".to_string(),
        "exposes".to_string(),
    ])
}
//! Ability group CRUD commands

use std::sync::Arc;

use sentinel_db::database::ability_group_dao::{
    AbilityGroup, AbilityGroupDao, AbilityGroupSummary, CreateAbilityGroup, UpdateAbilityGroup,
};

/// List all ability groups (summary only)
pub async fn list_ability_groups(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<AbilityGroupSummary>, String> {
    let pool = db_service.get_pool().map_err(|e| e.to_string())?;
    AbilityGroupDao::list_summary(pool)
        .await
        .map_err(|e| e.to_string())
}

/// List all ability groups (full details)
pub async fn list_ability_groups_full(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<AbilityGroup>, String> {
    let pool = db_service.get_pool().map_err(|e| e.to_string())?;
    AbilityGroupDao::list_all(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Get a single ability group by ID
pub async fn get_ability_group(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Option<AbilityGroup>, String> {
    let pool = db_service.get_pool().map_err(|e| e.to_string())?;
    AbilityGroupDao::get(pool, &id)
        .await
        .map_err(|e| e.to_string())
}

/// Create a new ability group
pub async fn create_ability_group(
    payload: CreateAbilityGroup,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<AbilityGroup, String> {
    let pool = db_service.get_pool().map_err(|e| e.to_string())?;
    AbilityGroupDao::create(pool, &payload)
        .await
        .map_err(|e| e.to_string())
}

/// Update an existing ability group
pub async fn update_ability_group(
    id: String,
    payload: UpdateAbilityGroup,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    let pool = db_service.get_pool().map_err(|e| e.to_string())?;
    AbilityGroupDao::update(pool, &id, &payload)
        .await
        .map_err(|e| e.to_string())
}

/// Delete an ability group
pub async fn delete_ability_group(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    let pool = db_service.get_pool().map_err(|e| e.to_string())?;
    AbilityGroupDao::delete(pool, &id)
        .await
        .map_err(|e| e.to_string())
}



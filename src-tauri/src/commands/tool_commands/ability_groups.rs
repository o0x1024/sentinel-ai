//! Ability group CRUD commands

use std::sync::Arc;

use sentinel_db::{
    AbilityGroup, AbilityGroupSummary, CreateAbilityGroup, Database, UpdateAbilityGroup,
};

/// List all ability groups (summary only)
pub async fn list_ability_groups(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<AbilityGroupSummary>, String> {
    db_service
        .list_ability_groups_summary()
        .await
        .map_err(|e| e.to_string())
}

/// List all ability groups (full details)
pub async fn list_ability_groups_full(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<AbilityGroup>, String> {
    db_service
        .list_all_ability_groups()
        .await
        .map_err(|e| e.to_string())
}

/// Get a single ability group by ID
pub async fn get_ability_group(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Option<AbilityGroup>, String> {
    db_service
        .get_ability_group(&id)
        .await
        .map_err(|e| e.to_string())
}

/// Create a new ability group
pub async fn create_ability_group(
    payload: CreateAbilityGroup,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<AbilityGroup, String> {
    db_service
        .create_ability_group(&payload)
        .await
        .map_err(|e| e.to_string())
}

/// Update an existing ability group
pub async fn update_ability_group(
    id: String,
    payload: UpdateAbilityGroup,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    db_service
        .update_ability_group(&id, &payload)
        .await
        .map_err(|e| e.to_string())
}

/// Delete an ability group
pub async fn delete_ability_group(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    db_service
        .delete_ability_group(&id)
        .await
        .map_err(|e| e.to_string())
}



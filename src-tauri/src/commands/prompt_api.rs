use std::sync::Arc;
use tauri::State;
use anyhow::Result;
use crate::services::{DatabaseService};
use crate::services::prompt_db::PromptRepository;
use crate::models::prompt::{PromptTemplate, UserPromptConfig, ArchitectureType, StageType, PromptGroup, PromptGroupItem};

#[tauri::command]
pub async fn list_prompt_templates_api(db: State<'_, Arc<DatabaseService>>) -> Result<Vec<PromptTemplate>, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.list_templates().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_prompt_template_api(db: State<'_, Arc<DatabaseService>>, id: i64) -> Result<Option<PromptTemplate>, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.get_template(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_prompt_template_api(db: State<'_, Arc<DatabaseService>>, template: PromptTemplate) -> Result<i64, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.create_template(&template).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_prompt_template_api(db: State<'_, Arc<DatabaseService>>, id: i64, template: PromptTemplate) -> Result<(), String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.update_template(id, &template).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_prompt_template_api(db: State<'_, Arc<DatabaseService>>, id: i64) -> Result<(), String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.delete_template(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_user_prompt_configs_api(db: State<'_, Arc<DatabaseService>>) -> Result<Vec<UserPromptConfig>, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.get_user_configs().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_user_prompt_config_api(
    db: State<'_, Arc<DatabaseService>>,
    architecture: ArchitectureType,
    stage: StageType,
    template_id: i64,
) -> Result<(), String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.update_user_config(architecture, stage, template_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_active_prompt_api(
    db: State<'_, Arc<DatabaseService>>,
    architecture: ArchitectureType,
    stage: StageType,
) -> Result<Option<String>, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.get_active_prompt(architecture, stage).await.map_err(|e| e.to_string())
}

// ===== Prompt Groups APIs =====
#[tauri::command]
pub async fn list_prompt_groups_api(
    db: State<'_, Arc<DatabaseService>>,
    architecture: Option<ArchitectureType>,
) -> Result<Vec<PromptGroup>, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.list_groups(architecture).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_prompt_group_api(
    db: State<'_, Arc<DatabaseService>>,
    group: PromptGroup,
) -> Result<i64, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.create_group(&group).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_prompt_group_api(
    db: State<'_, Arc<DatabaseService>>,
    id: i64,
    group: PromptGroup,
) -> Result<(), String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.update_group(id, &group).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_prompt_group_api(
    db: State<'_, Arc<DatabaseService>>,
    id: i64,
) -> Result<(), String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.delete_group(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_arch_default_group_api(
    db: State<'_, Arc<DatabaseService>>,
    architecture: ArchitectureType,
    group_id: i64,
) -> Result<(), String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.set_arch_default_group(architecture, group_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upsert_prompt_group_item_api(
    db: State<'_, Arc<DatabaseService>>,
    group_id: i64,
    stage: StageType,
    template_id: i64,
) -> Result<(), String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.upsert_group_item(group_id, stage, template_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_prompt_group_items_api(
    db: State<'_, Arc<DatabaseService>>,
    group_id: i64,
) -> Result<Vec<PromptGroupItem>, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.list_group_items(group_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_prompt_group_item_api(
    db: State<'_, Arc<DatabaseService>>,
    group_id: i64,
    stage: StageType,
) -> Result<(), String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.remove_group_item(group_id, stage).await.map_err(|e| e.to_string())
}



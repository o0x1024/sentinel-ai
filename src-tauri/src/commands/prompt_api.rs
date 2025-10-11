use std::sync::Arc;
use tauri::State;
use anyhow::Result;
use crate::services::{DatabaseService};
use crate::services::prompt_db::PromptRepository;
use crate::models::prompt::{PromptTemplate, UserPromptConfig, ArchitectureType, StageType, PromptGroup, PromptGroupItem, PromptCategory, TemplateType};
use crate::utils::prompt_resolver::{PromptResolver, AgentPromptConfig, CanonicalStage};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

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

// ===== Extended APIs for Unified Prompt System =====

#[tauri::command]
pub async fn list_prompt_templates_filtered_api(
    db: State<'_, Arc<DatabaseService>>,
    category: Option<PromptCategory>,
    template_type: Option<TemplateType>,
    architecture: Option<ArchitectureType>,
    is_system: Option<bool>,
) -> Result<Vec<PromptTemplate>, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.list_templates_filtered(category, template_type, architecture, is_system)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn duplicate_prompt_template_api(
    db: State<'_, Arc<DatabaseService>>,
    id: i64,
    new_name: Option<String>,
) -> Result<i64, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.duplicate_template(id, new_name).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn evaluate_prompt_api(
    db: State<'_, Arc<DatabaseService>>,
    template_id: i64,
    context: serde_json::Value,
) -> Result<String, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.evaluate_prompt(template_id, context).await.map_err(|e| e.to_string())
}

// ===== Preview resolved prompt for AgentManager =====

fn map_engine_to_arch(engine: &str) -> ArchitectureType {
    match engine {
        "rewoo" => ArchitectureType::ReWOO,
        "llm-compiler" => ArchitectureType::LLMCompiler,
        _ => ArchitectureType::PlanExecute,
    }
}

fn map_stage_to_canonical(stage: &str) -> Option<CanonicalStage> {
    match stage {
        "system" => Some(CanonicalStage::System),
        "intent_classifier" => Some(CanonicalStage::IntentClassifier),
        "planner" => Some(CanonicalStage::Planner),
        "executor" => Some(CanonicalStage::Executor),
        "replanner" => Some(CanonicalStage::Replanner),
        "evaluator" => Some(CanonicalStage::Evaluator),
        _ => None,
    }
}

#[tauri::command]
pub async fn preview_resolved_prompt_api(
    db: State<'_, Arc<DatabaseService>>,
    engine: String,
    stage: String,
    agent_config: JsonValue,
) -> Result<String, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);

    let resolver = PromptResolver::new(repo);
    let arch = map_engine_to_arch(&engine);
    let canonical = map_stage_to_canonical(&stage).ok_or_else(|| format!("Invalid stage: {}", stage))?;

    let params: HashMap<String, JsonValue> = agent_config.as_object()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect();

    let cfg = AgentPromptConfig::parse_agent_config(&params);

    resolver
        .resolve_prompt(&cfg, arch, canonical, None)
        .await
        .map_err(|e| e.to_string())
}



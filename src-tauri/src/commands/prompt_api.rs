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
    
    // If activating this template, deactivate other templates of the same type
    if template.is_active && template.template_type.is_some() {
        let template_type = template.template_type.as_ref().unwrap();
        
        // Get all templates of the same type
        let all_templates = repo.list_templates_filtered(
            template.category.clone(),
            Some(template_type.clone()),
            None,
            None
        ).await.map_err(|e| e.to_string())?;
        
        // Deactivate other active templates of the same type
        for other_template in all_templates {
            if other_template.id.is_some() && other_template.id.unwrap() != id && other_template.is_active {
                let mut deactivated = other_template.clone();
                deactivated.is_active = false;
                repo.update_template(other_template.id.unwrap(), &deactivated)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    
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

/// Get plugin generation prompt template by type
#[tauri::command]
pub async fn get_plugin_generation_prompt_api(
    db: State<'_, Arc<DatabaseService>>,
    template_type: String, // "passive" or "agent"
) -> Result<String, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    
    // 根据类型获取对应的模板
    let t_type = match template_type.as_str() {
        "passive" => TemplateType::PluginGeneration,
        "agent" => TemplateType::AgentPluginGeneration,
        _ => return Err(format!("Unknown template type: {}", template_type)),
    };
    
    // 获取该类型的模板（优先获取激活的，否则获取第一个）
    let templates = repo.list_templates_filtered(
        Some(PromptCategory::Application),
        Some(t_type),
        None,
        None
    ).await.map_err(|e| e.to_string())?;
    
    if templates.is_empty() {
        return Err("No plugin generation template found in database".to_string());
    }
    
    // 返回第一个激活的模板，如果没有激活的则返回第一个
    let template = templates.iter()
        .find(|t| t.is_active)
        .or_else(|| templates.first())
        .ok_or_else(|| "No suitable template found".to_string())?;
    
    Ok(template.content.clone())
}

/// Get combined plugin generation prompt (generation + interface + output format)
#[tauri::command]
pub async fn get_combined_plugin_prompt_api(
    db: State<'_, Arc<DatabaseService>>,
    plugin_type: String, // "passive" or "agent"
    vuln_type: String,
    severity: String,
) -> Result<String, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    
    // 根据插件类型选择对应的模板类型
    let (generation_template_type, interface_template_type, output_template_type) = match plugin_type.as_str() {
        "passive" => (
            TemplateType::PluginGeneration,
            TemplateType::PluginInterface,
            TemplateType::PluginOutputFormat
        ),
        "agent" => (
            TemplateType::AgentPluginGeneration,
            TemplateType::PluginInterface, // Agent 也使用 PluginInterface，但可以有自己的
            TemplateType::AgentPluginOutputFormat
        ),
        _ => (
            TemplateType::PluginGeneration,
            TemplateType::PluginInterface,
            TemplateType::PluginOutputFormat
        ),
    };
    
    // 获取三个模板：生成、接口、输出格式
    let generation_templates = repo.list_templates_filtered(
        Some(PromptCategory::Application),
        Some(generation_template_type),
        None,
        None
    ).await.map_err(|e| e.to_string())?;
    
    let interface_templates = repo.list_templates_filtered(
        Some(PromptCategory::Application),
        Some(interface_template_type),
        None,
        None
    ).await.map_err(|e| e.to_string())?;
    
    let output_templates = repo.list_templates_filtered(
        Some(PromptCategory::Application),
        Some(output_template_type),
        None,
        None
    ).await.map_err(|e| e.to_string())?;
    
    // 获取激活的模板，如果没有激活的则使用第一个
    let get_active_or_first = |templates: &[PromptTemplate], template_name: &str| -> Result<String, String> {
        if templates.is_empty() {
            return Err(format!("No {} template found in database", template_name));
        }
        
        // 优先选择激活的模板
        templates.iter()
            .find(|t| t.is_active)
            .or_else(|| templates.first())
            .map(|t| t.content.clone())
            .ok_or_else(|| format!("No suitable {} template found", template_name))
    };
    
    let generation_content = get_active_or_first(&generation_templates, "generation")?;
    let interface_content = get_active_or_first(&interface_templates, "interface")?;
    let output_content = get_active_or_first(&output_templates, "output format")?;
    
    // 组合模板，进行变量替换
    let mut combined = format!(
        "{}\n\n{}\n\n{}",
        generation_content,
        interface_content,
        output_content
    );
    
    // 简单的变量替换
    combined = combined.replace("{plugin_type}", &plugin_type);
    combined = combined.replace("{vuln_type}", &vuln_type);
    combined = combined.replace("{severity}", &severity);
    
    Ok(combined)
}



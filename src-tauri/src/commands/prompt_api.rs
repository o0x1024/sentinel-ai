use std::sync::Arc;
use tauri::State;
use anyhow::Result;
use crate::services::{DatabaseService};
use crate::services::prompt_db::PromptRepository;
use crate::models::prompt::{PromptTemplate, PromptCategory, TemplateType};
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
pub async fn list_prompt_templates_filtered_api(
    db: State<'_, Arc<DatabaseService>>,
    category: Option<PromptCategory>,
    template_type: Option<TemplateType>,
    is_system: Option<bool>,
) -> Result<Vec<PromptTemplate>, String> {
    let pool = db.get_pool().map_err(|e| e.to_string())?.clone();
    let repo = PromptRepository::new(pool);
    repo.list_templates_filtered(category, template_type, is_system)
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

fn map_stage_to_canonical(stage: &str) -> Option<CanonicalStage> {
    match stage {
        "system" => Some(CanonicalStage::System),
        "intent_classifier" => Some(CanonicalStage::IntentClassifier),
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
    let canonical = map_stage_to_canonical(&stage).ok_or_else(|| format!("Invalid stage: {}", stage))?;

    let params: HashMap<String, JsonValue> = agent_config.as_object()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect();

    let cfg = AgentPromptConfig::parse_agent_config(&params);

    resolver
        .resolve_prompt(&cfg, canonical, None)
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
    
    // 获取该类型的模板（只获取激活的模板）
    let templates = repo.list_templates_filtered(
        Some(PromptCategory::Application),
        Some(t_type),
        None
    ).await.map_err(|e| e.to_string())?;
    
    // 只返回激活的模板，如果没有激活的则返回错误
    let template = templates.iter()
        .find(|t| t.is_active)
        .ok_or_else(|| format!("No active template found for type: {}", template_type))?;
    
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
    
    println!("plugin_type: {}", plugin_type);
    println!("vuln_type: {}", vuln_type);
    println!("severity: {}", severity);
    
    // 根据插件类型选择对应的模板类型（合并后的完整模板）
    let template_type = match plugin_type.as_str() {
        "passive" => TemplateType::PluginGeneration,
        "agent" => TemplateType::AgentPluginGeneration,
        _ => TemplateType::PluginGeneration,
    };
    
    // 获取合并后的完整模板
    let templates = repo.list_templates_filtered(
        Some(PromptCategory::Application),
        Some(template_type),
        None
    ).await.map_err(|e| e.to_string())?;
    
    // 只获取激活的模板，如果没有激活的模板则返回错误
    let template = templates.iter()
        .find(|t| t.is_active)
        .map(|t| t.content.clone())
        .ok_or_else(|| format!("No active template found for type: {}", plugin_type))?;
    
    // 进行变量替换
    let mut content = template;
    content = content.replace("{plugin_type}", &plugin_type);
    content = content.replace("{vuln_type}", &vuln_type);
    content = content.replace("{severity}", &severity);
    
    Ok(content)
}

/// Get default prompt content from prompt.md files in app data directory
#[tauri::command]
pub async fn get_default_prompt_content() -> Result<String, String> {
    use std::fs;
    use std::path::PathBuf;
    
    // Use default react directory
    let arch_dir = "react";
    
    // Get app data directory
    let app_data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sentinel-ai")
        .join("prompts");
    
    // Construct path to prompt.md in app data directory
    let prompt_path = app_data_dir.join(arch_dir).join("prompt.md");
    
    // Read the file
    let content = fs::read_to_string(&prompt_path)
        .map_err(|e| format!("Failed to read prompt file for {} at {:?}: {}", arch_dir, prompt_path, e))?;
    
    // Return the full content
    Ok(content)
}

/// Initialize default prompt files by copying from source to app data directory
#[tauri::command]
pub async fn initialize_default_prompts() -> Result<String, String> {
    use std::fs;
    use std::path::PathBuf;
    
    // Get app data directory
    let app_data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("sentinel-ai")
        .join("prompts");
    
    // Create prompts directory if it doesn't exist
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create prompts directory: {}", e))?;
    
    let architectures = vec![
        ("rewoo", "rewoo"),
        ("llm_compiler", "llm_compiler"),
        ("plan_and_execute", "plan_and_execute"),
        ("react", "react"),
        ("travel", "travel"),
    ];
    
    let mut copied_count = 0;
    let mut skipped_count = 0;
    
    for (arch_name, arch_dir) in architectures {
        // Source path (from compiled binary resources)
        let source_path: PathBuf = [
            env!("CARGO_MANIFEST_DIR"),
            "src",
            "engines",
            arch_dir,
            "prompt.md"
        ].iter().collect();
        
        // Destination path (in app data directory)
        let dest_dir = app_data_dir.join(arch_name);
        let dest_path = dest_dir.join("prompt.md");
        
        // Create architecture directory
        fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("Failed to create directory for {}: {}", arch_name, e))?;
        
        // Only copy if destination doesn't exist (don't overwrite user modifications)
        if !dest_path.exists() {
            if source_path.exists() {
                fs::copy(&source_path, &dest_path)
                    .map_err(|e| format!("Failed to copy prompt file for {}: {}", arch_name, e))?;
                copied_count += 1;
            }
        } else {
            skipped_count += 1;
        }
    }
    
    Ok(format!("Initialized prompts: {} copied, {} skipped (already exist)", copied_count, skipped_count))
}



use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use sentinel_db::DatabaseClient;
use crate::models::prompt::{PromptTemplate, UserPromptConfig, ArchitectureType, StageType, PromptGroup, PromptGroupItem, PromptCategory, TemplateType};

#[derive(Clone, Debug)]
pub struct PromptRepository {
    db: DatabaseClient,
}

impl PromptRepository {
    pub fn new(pool: SqlitePool) -> Self { Self { db: DatabaseClient::new(pool) } }

    pub fn pool(&self) -> &SqlitePool { self.db.pool() }

    pub async fn list_templates(&self) -> Result<Vec<PromptTemplate>> {
        Ok(self.db.list_templates().await?)
    }

    pub async fn get_template(&self, id: i64) -> Result<Option<PromptTemplate>> {
        self.db.get_template(id).await
    }

    pub async fn get_template_by_arch_stage(&self, arch: ArchitectureType, stage: StageType) -> Result<Option<PromptTemplate>> {
        self.db.get_template_by_arch_stage(arch, stage).await
    }

    pub async fn create_template(&self, t: &PromptTemplate) -> Result<i64> {
        self.db.create_template(t).await
    }

    pub async fn update_template(&self, id: i64, t: &PromptTemplate) -> Result<()> {
        self.db.update_template(id, t).await
    }

    pub async fn delete_template(&self, id: i64) -> Result<()> {
        self.db.delete_template(id).await
    }

    pub async fn get_user_configs(&self) -> Result<Vec<UserPromptConfig>> {
        self.db.get_user_configs().await
    }

    pub async fn update_user_config(&self, arch: ArchitectureType, stage: StageType, template_id: i64) -> Result<()> {
        self.db.update_user_config(arch, stage, template_id).await
    }

    pub async fn get_active_prompt(&self, arch: ArchitectureType, stage: StageType) -> Result<Option<String>> {
        // 优先使用用户配置
        self.db.get_active_prompt(arch, stage).await
    }

    // ===== Prompt Groups =====
    pub async fn list_groups(&self, arch: Option<ArchitectureType>) -> Result<Vec<PromptGroup>> {
        self.db.list_groups(arch).await
    }

    pub async fn create_group(&self, g: &PromptGroup) -> Result<i64> {
        self.db.create_group(g).await
    }

    pub async fn update_group(&self, id: i64, g: &PromptGroup) -> Result<()> {
        self.db.update_group(id, g).await
    }

    pub async fn delete_group(&self, id: i64) -> Result<()> {
        self.db.delete_group(id).await
    }

    pub async fn set_arch_default_group(&self, arch: ArchitectureType, group_id: i64) -> Result<()> {
        self.db.set_arch_default_group(arch, group_id).await
    }

    pub async fn upsert_group_item(&self, group_id: i64, stage: StageType, template_id: i64) -> Result<()> {
        self.db.upsert_group_item(group_id, stage, template_id).await
    }

    pub async fn list_group_items(&self, group_id: i64) -> Result<Vec<PromptGroupItem>> {
        self.db.list_group_items(group_id).await
    }

    pub async fn remove_group_item(&self, group_id: i64, stage: StageType) -> Result<()> {
        self.db.remove_group_item(group_id, stage).await
    }

    /// List templates with filters
    pub async fn list_templates_filtered(&self, 
        category: Option<PromptCategory>, 
        template_type: Option<TemplateType>,
        architecture: Option<ArchitectureType>,
        is_system: Option<bool>
    ) -> Result<Vec<PromptTemplate>> {
        self.db.list_templates_filtered(category, template_type, architecture, is_system).await
    }

    /// Duplicate a template
    pub async fn duplicate_template(&self, id: i64, new_name: Option<String>) -> Result<i64> {
        self.db.duplicate_template(id, new_name).await
    }

    /// Get active template by type (returns the first active template of the given type)
    pub async fn get_active_template_by_type(&self, template_type: TemplateType) -> Result<Option<PromptTemplate>> {
        let templates = self.list_templates_filtered(
            None,
            Some(template_type),
            None,
            None
        ).await?;
        
        // Find first active template
        Ok(templates.into_iter().find(|t| t.is_active))
    }

    /// Evaluate prompt with variables
    pub async fn evaluate_prompt(&self, template_id: i64, context: serde_json::Value) -> Result<String> {
        if let Some(template) = self.get_template(template_id).await? {
            let mut content = template.content;
            
            // Simple variable replacement supporting {var} and {{VAR}} syntax
            if let Some(context_obj) = context.as_object() {
                for (key, value) in context_obj {
                    let placeholder_curly = format!("{{{}}}", key);
                    let placeholder_double = format!("{{{{{}}}}}", key.to_uppercase());
                    let replacement = match value {
                        serde_json::Value::String(s) => s.clone(),
                        _ => value.to_string(),
                    };
                    content = content.replace(&placeholder_curly, &replacement);
                    content = content.replace(&placeholder_double, &replacement);
                }
            }
            
            Ok(content)
        } else {
            Err(anyhow::anyhow!("Template not found"))
        }
    }
    
    /// Evaluate template content directly with variables (without needing template_id)
    pub fn evaluate_content(&self, content: &str, context: &serde_json::Value) -> String {
        let mut result = content.to_string();
        
        // Simple variable replacement supporting {var} and {{VAR}} syntax
        if let Some(context_obj) = context.as_object() {
            for (key, value) in context_obj {
                let placeholder_curly = format!("{{{}}}", key);
                let placeholder_double = format!("{{{{{}}}}}", key.to_uppercase());
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };
                result = result.replace(&placeholder_curly, &replacement);
                result = result.replace(&placeholder_double, &replacement);
            }
        }
        
        result
    }

    fn row_to_template(row: sqlx::sqlite::SqliteRow) -> PromptTemplate {
        let id_opt: Option<i64> = row.try_get("id").ok();
        let name: String = row.get("name");
        let description: Option<String> = row.try_get("description").ok();
        let architecture: String = row.get("architecture");
        let stage: String = row.get("stage");
        let content: String = row.get("content");
        let is_default_i: i64 = row.try_get("is_default").unwrap_or(0);
        let is_active_i: i64 = row.try_get("is_active").unwrap_or(1);
        let created_at: Option<String> = row.try_get("created_at").ok();
        let updated_at: Option<String> = row.try_get("updated_at").ok();
        
        // Extended fields
        let category: Option<String> = row.try_get("category").ok();
        let template_type: Option<String> = row.try_get("template_type").ok();
        let target_architecture: Option<String> = row.try_get("target_architecture").ok();
        let is_system_i: i64 = row.try_get("is_system").unwrap_or(0);
        let priority: i32 = row.try_get("priority").unwrap_or(50);
        let tags_json: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
        let variables_json: String = row.try_get("variables").unwrap_or_else(|_| "[]".to_string());
        let version: String = row.try_get("version").unwrap_or_else(|_| "1.0.0".to_string());
        
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let variables: Vec<String> = serde_json::from_str(&variables_json).unwrap_or_default();

        PromptTemplate {
            id: id_opt,
            name,
            description,
            architecture: Self::parse_arch(&architecture),
            stage: Self::parse_stage(&stage),
            content,
            is_default: is_default_i != 0,
            is_active: is_active_i != 0,
            created_at,
            updated_at,
            category: category.and_then(|c| Self::parse_category(&c)),
            template_type: template_type.and_then(|t| Self::parse_template_type(&t)),
            target_architecture: target_architecture.and_then(|a| Some(Self::parse_arch(&a))),
            is_system: is_system_i != 0,
            priority,
            tags,
            variables,
            version,
        }
    }

    fn row_to_group(row: sqlx::sqlite::SqliteRow) -> PromptGroup {
        let id_opt: Option<i64> = row.try_get("id").ok();
        let architecture: String = row.get("architecture");
        let name: String = row.get("name");
        let description: Option<String> = row.try_get("description").ok();
        let is_default_i: i64 = row.try_get("is_default").unwrap_or(0);
        let created_at: Option<String> = row.try_get("created_at").ok();
        let updated_at: Option<String> = row.try_get("updated_at").ok();

        PromptGroup {
            id: id_opt,
            architecture: Self::parse_arch(&architecture),
            name,
            description,
            is_default: is_default_i != 0,
            created_at,
            updated_at,
        }
    }

    fn row_to_group_item(row: sqlx::sqlite::SqliteRow) -> PromptGroupItem {
        let id_opt: Option<i64> = row.try_get("id").ok();
        let group_id: i64 = row.get("group_id");
        let stage: String = row.get("stage");
        let template_id: i64 = row.get("template_id");
        let created_at: Option<String> = row.try_get("created_at").ok();
        let updated_at: Option<String> = row.try_get("updated_at").ok();

        PromptGroupItem {
            id: id_opt,
            group_id,
            stage: Self::parse_stage(&stage),
            template_id,
            created_at,
            updated_at,
        }
    }

    fn arch_str(a: &ArchitectureType) -> &'static str {
        match a {
            ArchitectureType::ReAct => "react",
            ArchitectureType::ReWOO => "rewoo",
            ArchitectureType::LLMCompiler => "llmcompiler",
            ArchitectureType::PlanExecute => "planexecute",
        }
    }

    fn stage_str(s: &StageType) -> &'static str {
        match s {
            StageType::System => "system",
            StageType::Planning => "planning",
            StageType::Execution => "execution",
            StageType::Planner => "planner",
            StageType::Worker => "worker",
            StageType::Solver => "solver",
            StageType::Evaluation => "evaluation",
            StageType::Replan => "replan",
        }
    }

    fn parse_arch(s: &str) -> ArchitectureType {
        match s.to_lowercase().as_str() {
            "rewoo" => ArchitectureType::ReWOO,
            "llmcompiler" => ArchitectureType::LLMCompiler,
            "react" => ArchitectureType::ReAct,
            _ => ArchitectureType::PlanExecute,
        }
    }

    fn parse_stage(s: &str) -> StageType {
        match s.to_lowercase().as_str() {
            "system" => StageType::System,
            "planning" => StageType::Planning,
            "execution" => StageType::Execution,
            "planner" => StageType::Planner,
            "worker" => StageType::Worker,
            "solver" => StageType::Solver,
            "evaluation" => StageType::Evaluation,
            "replan" => StageType::Replan,
            _ => StageType::Planning, // Default to Planning for unknown stages
        }
    }
    
    fn category_str(c: &PromptCategory) -> &'static str {
        match c {
            PromptCategory::System => "System",
            PromptCategory::LlmArchitecture => "LlmArchitecture",
            PromptCategory::Application => "Application",
            PromptCategory::UserDefined => "UserDefined",
        }
    }
    
    fn parse_category(s: &str) -> Option<PromptCategory> {
        match s {
            "System" => Some(PromptCategory::System),
            "LlmArchitecture" => Some(PromptCategory::LlmArchitecture),
            "Application" => Some(PromptCategory::Application),
            "UserDefined" => Some(PromptCategory::UserDefined),
            _ => None,
        }
    }
    
    fn template_type_str(t: &TemplateType) -> &'static str {
        match t {
            TemplateType::SystemPrompt => "SystemPrompt",
            TemplateType::IntentClassifier => "IntentClassifier",
            TemplateType::Planner => "Planner",
            TemplateType::Executor => "Executor",
            TemplateType::Replanner => "Replanner",
            TemplateType::Evaluator => "Evaluator",
            TemplateType::ReportGenerator => "ReportGenerator",
            TemplateType::Domain => "Domain",
            TemplateType::Custom => "Custom",
            TemplateType::PluginGeneration => "PluginGeneration",
            TemplateType::AgentPluginGeneration => "AgentPluginGeneration",
            TemplateType::PluginFix => "PluginFix",
            TemplateType::AgentPluginFix => "AgentPluginFix",
            TemplateType::PluginVulnSpecific => "PluginVulnSpecific",
            TemplateType::VisionExplorerSystem => "VisionExplorerSystem",
        }
    }
    
    fn parse_template_type(s: &str) -> Option<TemplateType> {
        match s {
            "SystemPrompt" => Some(TemplateType::SystemPrompt),
            "IntentClassifier" => Some(TemplateType::IntentClassifier),
            "Planner" => Some(TemplateType::Planner),
            "Executor" => Some(TemplateType::Executor),
            "Replanner" => Some(TemplateType::Replanner),
            "Evaluator" => Some(TemplateType::Evaluator),
            "ReportGenerator" => Some(TemplateType::ReportGenerator),
            "Domain" => Some(TemplateType::Domain),
            "Custom" => Some(TemplateType::Custom),
            "PluginGeneration" => Some(TemplateType::PluginGeneration),
            "AgentPluginGeneration" => Some(TemplateType::AgentPluginGeneration),
            "PluginFix" => Some(TemplateType::PluginFix),
            "AgentPluginFix" => Some(TemplateType::AgentPluginFix),
            "PluginVulnSpecific" => Some(TemplateType::PluginVulnSpecific),
            "VisionExplorerSystem" => Some(TemplateType::VisionExplorerSystem),
            _ => None,
        }
    }
}

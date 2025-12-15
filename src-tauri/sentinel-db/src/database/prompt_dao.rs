use anyhow::Result;
use sqlx::{sqlite::SqlitePool, Row};
use sentinel_core::models::prompt::{
    PromptTemplate,
    PromptCategory, TemplateType,
};

fn category_str(c: &PromptCategory) -> &'static str {
    match c {
        PromptCategory::System => "System",
        PromptCategory::Application => "Application",
        PromptCategory::UserDefined => "UserDefined",
    }
}

fn parse_category(s: &str) -> Option<PromptCategory> {
    match s {
        "System" => Some(PromptCategory::System),
        "LlmArchitecture" => Some(PromptCategory::System),
        "Application" => Some(PromptCategory::Application),
        "UserDefined" => Some(PromptCategory::UserDefined),
        _ => None,
    }
}

fn template_type_str(t: &TemplateType) -> &'static str {
    match t {
        TemplateType::SystemPrompt => "SystemPrompt",
        TemplateType::IntentClassifier => "IntentClassifier",
        TemplateType::Domain => "Domain",
        TemplateType::Custom => "Custom",
        TemplateType::PluginGeneration => "PluginGeneration",
        TemplateType::AgentPluginGeneration => "AgentPluginGeneration",
        TemplateType::PluginFix => "PluginFix",
        TemplateType::AgentPluginFix => "AgentPluginFix",
        TemplateType::PluginVulnSpecific => "PluginVulnSpecific",
        TemplateType::VisionExplorerVision => "VisionExplorerVision",
        TemplateType::VisionExplorerText => "VisionExplorerText",
    }
}

fn parse_template_type(s: &str) -> Option<TemplateType> {
    match s {
        "SystemPrompt" => Some(TemplateType::SystemPrompt),
        "IntentClassifier" => Some(TemplateType::IntentClassifier),
        "Domain" => Some(TemplateType::Domain),
        "Custom" => Some(TemplateType::Custom),
        "PluginGeneration" => Some(TemplateType::PluginGeneration),
        "AgentPluginGeneration" => Some(TemplateType::AgentPluginGeneration),
        "PluginFix" => Some(TemplateType::PluginFix),
        "AgentPluginFix" => Some(TemplateType::AgentPluginFix),
        "PluginVulnSpecific" => Some(TemplateType::PluginVulnSpecific),
        "VisionExplorerVision" => Some(TemplateType::VisionExplorerVision),
        "VisionExplorerText" => Some(TemplateType::VisionExplorerText),
        _ => None,
    }
}

fn row_to_template(row: sqlx::sqlite::SqliteRow) -> PromptTemplate {
    let id_opt: Option<i64> = row.try_get("id").ok();
    let name: String = row.get("name");
    let description: Option<String> = row.try_get("description").ok();
    let content: String = row.get("content");
    let is_default_i: i64 = row.try_get("is_default").unwrap_or(0);
    let is_active_i: i64 = row.try_get("is_active").unwrap_or(1);
    let created_at: Option<String> = row.try_get("created_at").ok();
    let updated_at: Option<String> = row.try_get("updated_at").ok();

    let category: Option<String> = row.try_get("category").ok();
    let template_type: Option<String> = row.try_get("template_type").ok();
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
        content,
        is_default: is_default_i != 0,
        is_active: is_active_i != 0,
        created_at,
        updated_at,
        category: category.and_then(|c| parse_category(&c)),
        template_type: template_type.and_then(|t| parse_template_type(&t)),
        is_system: is_system_i != 0,
        priority,
        tags,
        variables,
        version,
    }
}

pub async fn list_templates(pool: &SqlitePool) -> Result<Vec<PromptTemplate>> {
    let rows = sqlx::query(
        r#"SELECT id, name, description, content, is_default, is_active, created_at, updated_at,
           category, template_type, is_system, priority, tags, variables, version 
           FROM prompt_templates ORDER BY created_at DESC"#
    ).fetch_all(pool).await?;

    Ok(rows.into_iter().map(row_to_template).collect())
}

pub async fn get_template(pool: &SqlitePool, id: i64) -> Result<Option<PromptTemplate>> {
    let r = sqlx::query(
        r#"SELECT id, name, description, stage, content, is_default, is_active, created_at, updated_at,
           category, template_type, is_system, priority, tags, variables, version 
           FROM prompt_templates WHERE id = ?"#
    ).bind(id).fetch_optional(pool).await?;
    Ok(r.map(row_to_template))
}

pub async fn create_template(pool: &SqlitePool, t: &PromptTemplate) -> Result<i64> {
    let category_s = t.category.as_ref().map(category_str);
    let template_type_s = t.template_type.as_ref().map(template_type_str);
    let tags_json = serde_json::to_string(&t.tags).unwrap_or_else(|_| "[]".to_string());
    let variables_json = serde_json::to_string(&t.variables).unwrap_or_else(|_| "[]".to_string());

    let res = sqlx::query(
        r#"INSERT INTO prompt_templates (name, description, content, is_default, is_active,
           category, template_type, is_system, priority, tags, variables, version) 
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(&t.name)
    .bind(&t.description)
    .bind(&t.content)
    .bind(if t.is_default { 1 } else { 0 })
    .bind(if t.is_active { 1 } else { 0 })
    .bind(category_s)
    .bind(template_type_s)
    .bind(if t.is_system { 1 } else { 0 })
    .bind(t.priority)
    .bind(tags_json)
    .bind(variables_json)
    .bind(&t.version)
    .execute(pool).await?;
    Ok(res.last_insert_rowid())
}

pub async fn update_template(pool: &SqlitePool, id: i64, t: &PromptTemplate) -> Result<()> {
    let category_s = t.category.as_ref().map(category_str);
    let template_type_s = t.template_type.as_ref().map(template_type_str);
    let tags_json = serde_json::to_string(&t.tags).unwrap_or_else(|_| "[]".to_string());
    let variables_json = serde_json::to_string(&t.variables).unwrap_or_else(|_| "[]".to_string());

    sqlx::query(
        r#"UPDATE prompt_templates SET name = ?, description = ?, content = ?, 
           is_default = ?, is_active = ?, category = ?, template_type = ?, 
           is_system = ?, priority = ?, tags = ?, variables = ?, version = ?, updated_at = CURRENT_TIMESTAMP 
           WHERE id = ?"#
    )
    .bind(&t.name)
    .bind(&t.description)
    .bind(&t.content)
    .bind(if t.is_default { 1 } else { 0 })
    .bind(if t.is_active { 1 } else { 0 })
    .bind(category_s)
    .bind(template_type_s)
    .bind(if t.is_system { 1 } else { 0 })
    .bind(t.priority)
    .bind(tags_json)
    .bind(variables_json)
    .bind(&t.version)
    .bind(id)
    .execute(pool).await?;
    Ok(())
}

pub async fn delete_template(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM prompt_templates WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn list_templates_filtered(
    pool: &SqlitePool,
    category: Option<PromptCategory>,
    template_type: Option<TemplateType>,
    is_system: Option<bool>,
) -> Result<Vec<PromptTemplate>> {
    let mut query = r#"SELECT id, name, description, content, is_default, is_active, created_at, updated_at,
       category, template_type, is_system, priority, tags, variables, version 
       FROM prompt_templates WHERE 1=1"#.to_string();

    if let Some(cat) = &category { query.push_str(&format!(" AND category = '{}'", category_str(cat))); }
    if let Some(tt) = &template_type { query.push_str(&format!(" AND template_type = '{}'", template_type_str(tt))); }
    if let Some(sys) = is_system { query.push_str(&format!(" AND is_system = {}", if sys { 1 } else { 0 })); }
    query.push_str(" ORDER BY priority DESC, updated_at DESC");

    let rows = sqlx::query(&query).fetch_all(pool).await?;
    Ok(rows.into_iter().map(row_to_template).collect())
}

pub async fn duplicate_template(pool: &SqlitePool, id: i64, new_name: Option<String>) -> Result<i64> {
    if let Some(template) = get_template(pool, id).await? {
        let mut t = template;
        t.id = None;
        t.name = new_name.unwrap_or_else(|| format!("{} (Copy)", t.name));
        t.is_default = false;
        t.created_at = None;
        t.updated_at = None;
        create_template(pool, &t).await
    } else {
        Err(anyhow::anyhow!("Template not found"))
    }
}



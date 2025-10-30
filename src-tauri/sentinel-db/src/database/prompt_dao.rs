use anyhow::Result;
use sqlx::{sqlite::SqlitePool, Row};
use sentinel_core::models::prompt::{
    PromptTemplate, UserPromptConfig, ArchitectureType, StageType, PromptGroup, PromptGroupItem,
    PromptCategory, TemplateType,
};

fn arch_str(a: &ArchitectureType) -> &'static str {
    match a {
        ArchitectureType::ReWOO => "rewoo",
        ArchitectureType::LLMCompiler => "llmcompiler",
        ArchitectureType::PlanExecute => "planexecute",
    }
}

fn stage_str(s: &StageType) -> &'static str {
    match s {
        StageType::Planner => "planner",
        StageType::Worker => "worker",
        StageType::Solver => "solver",
        StageType::Planning => "planning",
        StageType::Execution => "execution",
        StageType::Replan => "replan",
    }
}

fn parse_arch(s: &str) -> ArchitectureType {
    match s.to_lowercase().as_str() {
        "rewoo" => ArchitectureType::ReWOO,
        "llmcompiler" => ArchitectureType::LLMCompiler,
        _ => ArchitectureType::PlanExecute,
    }
}

fn parse_stage(s: &str) -> StageType {
    match s.to_lowercase().as_str() {
        "planner" => StageType::Planner,
        "worker" => StageType::Worker,
        "solver" => StageType::Solver,
        "planning" => StageType::Planning,
        "execution" => StageType::Execution,
        _ => StageType::Replan,
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
        _ => None,
    }
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
        architecture: parse_arch(&architecture),
        stage: parse_stage(&stage),
        content,
        is_default: is_default_i != 0,
        is_active: is_active_i != 0,
        created_at,
        updated_at,
        category: category.and_then(|c| parse_category(&c)),
        template_type: template_type.and_then(|t| parse_template_type(&t)),
        target_architecture: target_architecture.map(|a| parse_arch(&a)),
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
        architecture: parse_arch(&architecture),
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

    PromptGroupItem { id: id_opt, group_id, stage: parse_stage(&stage), template_id, created_at, updated_at }
}

pub async fn list_templates(pool: &SqlitePool) -> Result<Vec<PromptTemplate>> {
    let rows = sqlx::query(
        r#"SELECT id, name, description, architecture, stage, content, is_default, is_active, created_at, updated_at,
           category, template_type, target_architecture, is_system, priority, tags, variables, version 
           FROM prompt_templates ORDER BY created_at DESC"#
    ).fetch_all(pool).await?;

    Ok(rows.into_iter().map(row_to_template).collect())
}

pub async fn get_template(pool: &SqlitePool, id: i64) -> Result<Option<PromptTemplate>> {
    let r = sqlx::query(
        r#"SELECT id, name, description, architecture, stage, content, is_default, is_active, created_at, updated_at,
           category, template_type, target_architecture, is_system, priority, tags, variables, version 
           FROM prompt_templates WHERE id = ?"#
    ).bind(id).fetch_optional(pool).await?;
    Ok(r.map(row_to_template))
}

pub async fn get_template_by_arch_stage(pool: &SqlitePool, arch: ArchitectureType, stage: StageType) -> Result<Option<PromptTemplate>> {
    let arch_s = arch_str(&arch);
    let stage_s = stage_str(&stage);
    let r = sqlx::query(
        r#"SELECT id, name, description, architecture, stage, content, is_default, is_active, created_at, updated_at,
           category, template_type, target_architecture, is_system, priority, tags, variables, version
           FROM prompt_templates WHERE architecture = ? AND stage = ? AND is_active = 1
           ORDER BY is_default DESC, priority DESC, updated_at DESC LIMIT 1"#
    ).bind(arch_s).bind(stage_s).fetch_optional(pool).await?;
    Ok(r.map(row_to_template))
}

pub async fn create_template(pool: &SqlitePool, t: &PromptTemplate) -> Result<i64> {
    let arch_s = arch_str(&t.architecture);
    let stage_s = stage_str(&t.stage);
    let category_s = t.category.as_ref().map(category_str);
    let template_type_s = t.template_type.as_ref().map(template_type_str);
    let target_arch_s = t.target_architecture.as_ref().map(arch_str);
    let tags_json = serde_json::to_string(&t.tags).unwrap_or_else(|_| "[]".to_string());
    let variables_json = serde_json::to_string(&t.variables).unwrap_or_else(|_| "[]".to_string());

    let res = sqlx::query(
        r#"INSERT INTO prompt_templates (name, description, architecture, stage, content, is_default, is_active,
           category, template_type, target_architecture, is_system, priority, tags, variables, version) 
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(&t.name)
    .bind(&t.description)
    .bind(arch_s)
    .bind(stage_s)
    .bind(&t.content)
    .bind(if t.is_default { 1 } else { 0 })
    .bind(if t.is_active { 1 } else { 0 })
    .bind(category_s)
    .bind(template_type_s)
    .bind(target_arch_s)
    .bind(if t.is_system { 1 } else { 0 })
    .bind(t.priority)
    .bind(tags_json)
    .bind(variables_json)
    .bind(&t.version)
    .execute(pool).await?;
    Ok(res.last_insert_rowid())
}

pub async fn update_template(pool: &SqlitePool, id: i64, t: &PromptTemplate) -> Result<()> {
    let arch_s = arch_str(&t.architecture);
    let stage_s = stage_str(&t.stage);
    let category_s = t.category.as_ref().map(category_str);
    let template_type_s = t.template_type.as_ref().map(template_type_str);
    let target_arch_s = t.target_architecture.as_ref().map(arch_str);
    let tags_json = serde_json::to_string(&t.tags).unwrap_or_else(|_| "[]".to_string());
    let variables_json = serde_json::to_string(&t.variables).unwrap_or_else(|_| "[]".to_string());

    sqlx::query(
        r#"UPDATE prompt_templates SET name = ?, description = ?, architecture = ?, stage = ?, content = ?, 
           is_default = ?, is_active = ?, category = ?, template_type = ?, target_architecture = ?, 
           is_system = ?, priority = ?, tags = ?, variables = ?, version = ?, updated_at = CURRENT_TIMESTAMP 
           WHERE id = ?"#
    )
    .bind(&t.name)
    .bind(&t.description)
    .bind(arch_s)
    .bind(stage_s)
    .bind(&t.content)
    .bind(if t.is_default { 1 } else { 0 })
    .bind(if t.is_active { 1 } else { 0 })
    .bind(category_s)
    .bind(template_type_s)
    .bind(target_arch_s)
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

pub async fn get_user_configs(pool: &SqlitePool) -> Result<Vec<UserPromptConfig>> {
    let rows = sqlx::query(
        r#"SELECT id, architecture, stage, template_id, created_at, updated_at FROM user_prompt_configs"#
    ).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|row| UserPromptConfig {
        id: row.try_get::<i64, _>("id").ok(),
        architecture: parse_arch(&row.get::<String, _>("architecture")),
        stage: parse_stage(&row.get::<String, _>("stage")),
        template_id: row.get::<i64, _>("template_id"),
        created_at: row.try_get::<String, _>("created_at").ok(),
        updated_at: row.try_get::<String, _>("updated_at").ok(),
    }).collect())
}

pub async fn update_user_config(pool: &SqlitePool, arch: ArchitectureType, stage: StageType, template_id: i64) -> Result<()> {
    let arch_s = arch_str(&arch);
    let stage_s = stage_str(&stage);
    sqlx::query(
        r#"INSERT INTO user_prompt_configs (architecture, stage, template_id) VALUES (?, ?, ?)
           ON CONFLICT(architecture, stage) DO UPDATE SET template_id = excluded.template_id, updated_at = CURRENT_TIMESTAMP"#
    )
    .bind(arch_s)
    .bind(stage_s)
    .bind(template_id)
    .execute(pool).await?;
    Ok(())
}

pub async fn get_active_prompt(pool: &SqlitePool, arch: ArchitectureType, stage: StageType) -> Result<Option<String>> {
    let arch_s = arch_str(&arch);
    let stage_s = stage_str(&stage);
    if let Some(row) = sqlx::query(
        r#"SELECT t.content as content
            FROM user_prompt_configs c JOIN prompt_templates t ON c.template_id = t.id
            WHERE c.architecture = ? AND c.stage = ?"#
    )
    .bind(arch_s)
    .bind(stage_s)
    .fetch_optional(pool).await? {
        let content: String = row.get::<String, _>("content");
        return Ok(Some(content));
    }
    if let Some(row) = sqlx::query(
        r#"SELECT t.content as content
            FROM prompt_groups g
            JOIN prompt_group_items gi ON gi.group_id = g.id
            JOIN prompt_templates t ON t.id = gi.template_id
            WHERE g.architecture = ? AND g.is_default = 1 AND gi.stage = ?"#
    )
    .bind(arch_s)
    .bind(stage_s)
    .fetch_optional(pool).await? {
        let content: String = row.get::<String, _>("content");
        return Ok(Some(content));
    }
    if let Some(t) = get_template_by_arch_stage(pool, arch, stage).await? {
        return Ok(Some(t.content));
    }
    Ok(None)
}

pub async fn list_groups(pool: &SqlitePool, arch: Option<ArchitectureType>) -> Result<Vec<PromptGroup>> {
    let rows = if let Some(a) = arch {
        let arch_s = arch_str(&a);
        sqlx::query(r#"SELECT id, architecture, name, description, is_default, created_at, updated_at FROM prompt_groups WHERE architecture = ? ORDER BY is_default DESC, updated_at DESC"#)
            .bind(arch_s)
            .fetch_all(pool).await?
    } else {
        sqlx::query(r#"SELECT id, architecture, name, description, is_default, created_at, updated_at FROM prompt_groups ORDER BY is_default DESC, updated_at DESC"#)
            .fetch_all(pool).await?
    };
    Ok(rows.into_iter().map(row_to_group).collect())
}

pub async fn create_group(pool: &SqlitePool, g: &PromptGroup) -> Result<i64> {
    let arch_s = arch_str(&g.architecture);
    let res = sqlx::query(r#"INSERT INTO prompt_groups (architecture, name, description, is_default) VALUES (?, ?, ?, ?)"#)
        .bind(arch_s)
        .bind(&g.name)
        .bind(&g.description)
        .bind(if g.is_default { 1 } else { 0 })
        .execute(pool).await?;
    Ok(res.last_insert_rowid())
}

pub async fn update_group(pool: &SqlitePool, id: i64, g: &PromptGroup) -> Result<()> {
    let arch_s = arch_str(&g.architecture);
    sqlx::query(r#"UPDATE prompt_groups SET architecture = ?, name = ?, description = ?, is_default = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"#)
        .bind(arch_s)
        .bind(&g.name)
        .bind(&g.description)
        .bind(if g.is_default { 1 } else { 0 })
        .bind(id)
        .execute(pool).await?;
    Ok(())
}

pub async fn delete_group(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM prompt_groups WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn set_arch_default_group(pool: &SqlitePool, arch: ArchitectureType, group_id: i64) -> Result<()> {
    let arch_s = arch_str(&arch);
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE prompt_groups SET is_default = 0 WHERE architecture = ?")
        .bind(arch_s)
        .execute(&mut *tx).await?;
    sqlx::query("UPDATE prompt_groups SET is_default = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(group_id)
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn upsert_group_item(pool: &SqlitePool, group_id: i64, stage: StageType, template_id: i64) -> Result<()> {
    let stage_s = stage_str(&stage);
    sqlx::query(r#"INSERT INTO prompt_group_items (group_id, stage, template_id) VALUES (?, ?, ?)
        ON CONFLICT(group_id, stage) DO UPDATE SET template_id = excluded.template_id, updated_at = CURRENT_TIMESTAMP"#)
        .bind(group_id)
        .bind(stage_s)
        .bind(template_id)
        .execute(pool).await?;
    Ok(())
}

pub async fn list_group_items(pool: &SqlitePool, group_id: i64) -> Result<Vec<PromptGroupItem>> {
    let rows = sqlx::query(r#"SELECT id, group_id, stage, template_id, created_at, updated_at FROM prompt_group_items WHERE group_id = ?"#)
        .bind(group_id)
        .fetch_all(pool).await?;
    Ok(rows.into_iter().map(row_to_group_item).collect())
}

pub async fn remove_group_item(pool: &SqlitePool, group_id: i64, stage: StageType) -> Result<()> {
    let stage_s = stage_str(&stage);
    sqlx::query("DELETE FROM prompt_group_items WHERE group_id = ? AND stage = ?")
        .bind(group_id)
        .bind(stage_s)
        .execute(pool).await?;
    Ok(())
}

pub async fn list_templates_filtered(
    pool: &SqlitePool,
    category: Option<PromptCategory>,
    template_type: Option<TemplateType>,
    architecture: Option<ArchitectureType>,
    is_system: Option<bool>,
) -> Result<Vec<PromptTemplate>> {
    let mut query = r#"SELECT id, name, description, architecture, stage, content, is_default, is_active, created_at, updated_at,
       category, template_type, target_architecture, is_system, priority, tags, variables, version 
       FROM prompt_templates WHERE 1=1"#.to_string();

    if let Some(cat) = &category { query.push_str(&format!(" AND category = '{}'", category_str(cat))); }
    if let Some(tt) = &template_type { query.push_str(&format!(" AND template_type = '{}'", template_type_str(tt))); }
    if let Some(arch) = &architecture { query.push_str(&format!(" AND architecture = '{}'", arch_str(arch))); }
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



use anyhow::Result;
use sqlx::{Row};
use sqlx::sqlite::{SqlitePool, SqliteRow};
use crate::models::prompt::{PromptTemplate, UserPromptConfig, ArchitectureType, StageType, PromptGroup, PromptGroupItem};

#[derive(Clone, Debug)]
pub struct PromptRepository {
    pool: SqlitePool,
}

impl PromptRepository {
    pub fn new(pool: SqlitePool) -> Self { Self { pool } }

    pub fn pool(&self) -> &SqlitePool { &self.pool }

    pub async fn list_templates(&self) -> Result<Vec<PromptTemplate>> {
        let rows = sqlx::query(
            r#"SELECT id, name, description, architecture, stage, content, is_default, is_active, created_at, updated_at FROM prompt_templates ORDER BY created_at DESC"#
        ).fetch_all(&self.pool).await?;

        Ok(rows.into_iter().map(Self::row_to_template).collect())
    }

    pub async fn get_template(&self, id: i64) -> Result<Option<PromptTemplate>> {
        let r = sqlx::query(
            r#"SELECT id, name, description, architecture, stage, content, is_default, is_active, created_at, updated_at FROM prompt_templates WHERE id = ?"#
        ).bind(id).fetch_optional(&self.pool).await?;
        Ok(r.map(|row| Self::row_to_template(row)))
    }

    pub async fn get_template_by_arch_stage(&self, arch: ArchitectureType, stage: StageType) -> Result<Option<PromptTemplate>> {
        let arch_s = Self::arch_str(&arch);
        let stage_s = Self::stage_str(&stage);
        let r = sqlx::query(
            r#"SELECT id, name, description, architecture, stage, content, is_default, is_active, created_at, updated_at
                FROM prompt_templates WHERE architecture = ? AND stage = ? AND is_active = 1
                ORDER BY is_default DESC, updated_at DESC LIMIT 1"#
        ).bind(arch_s).bind(stage_s).fetch_optional(&self.pool).await?;
        Ok(r.map(|row| Self::row_to_template(row)))
    }

    pub async fn create_template(&self, t: &PromptTemplate) -> Result<i64> {
        let arch_s = Self::arch_str(&t.architecture);
        let stage_s = Self::stage_str(&t.stage);
        let res = sqlx::query(
            r#"INSERT INTO prompt_templates (name, description, architecture, stage, content, is_default, is_active) VALUES (?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&t.name)
        .bind(&t.description)
        .bind(arch_s)
        .bind(stage_s)
        .bind(&t.content)
        .bind(if t.is_default { 1 } else { 0 })
        .bind(if t.is_active { 1 } else { 0 })
        .execute(&self.pool).await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn update_template(&self, id: i64, t: &PromptTemplate) -> Result<()> {
        let arch_s = Self::arch_str(&t.architecture);
        let stage_s = Self::stage_str(&t.stage);
        sqlx::query(
            r#"UPDATE prompt_templates SET name = ?, description = ?, architecture = ?, stage = ?, content = ?, is_default = ?, is_active = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"#
        )
        .bind(&t.name)
        .bind(&t.description)
        .bind(arch_s)
        .bind(stage_s)
        .bind(&t.content)
        .bind(if t.is_default { 1 } else { 0 })
        .bind(if t.is_active { 1 } else { 0 })
        .bind(id)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn delete_template(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM prompt_templates WHERE id = ?").bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_user_configs(&self) -> Result<Vec<UserPromptConfig>> {
        let rows = sqlx::query(
            r#"SELECT id, architecture, stage, template_id, created_at, updated_at FROM user_prompt_configs"#
        ).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|row| UserPromptConfig {
            id: row.try_get::<i64, _>("id").ok(),
            architecture: Self::parse_arch(&row.get::<String, _>("architecture")),
            stage: Self::parse_stage(&row.get::<String, _>("stage")),
            template_id: row.get::<i64, _>("template_id"),
            created_at: row.try_get::<String, _>("created_at").ok(),
            updated_at: row.try_get::<String, _>("updated_at").ok(),
        }).collect())
    }

    pub async fn update_user_config(&self, arch: ArchitectureType, stage: StageType, template_id: i64) -> Result<()> {
        let arch_s = Self::arch_str(&arch);
        let stage_s = Self::stage_str(&stage);
        sqlx::query(
            r#"INSERT INTO user_prompt_configs (architecture, stage, template_id) VALUES (?, ?, ?)
               ON CONFLICT(architecture, stage) DO UPDATE SET template_id = excluded.template_id, updated_at = CURRENT_TIMESTAMP"#
        )
        .bind(arch_s)
        .bind(stage_s)
        .bind(template_id)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_active_prompt(&self, arch: ArchitectureType, stage: StageType) -> Result<Option<String>> {
        // 优先使用用户配置
        let arch_s = Self::arch_str(&arch);
        let stage_s = Self::stage_str(&stage);
        if let Some(row) = sqlx::query(
            r#"SELECT t.content as content
                FROM user_prompt_configs c JOIN prompt_templates t ON c.template_id = t.id
                WHERE c.architecture = ? AND c.stage = ?"#
        )
        .bind(arch_s)
        .bind(stage_s)
        .fetch_optional(&self.pool).await? {
            let content: String = row.get::<String, _>("content");
            return Ok(Some(content));
        }
        // 否则使用架构默认组的映射
        if let Some(row) = sqlx::query(
            r#"SELECT t.content as content
                FROM prompt_groups g
                JOIN prompt_group_items gi ON gi.group_id = g.id
                JOIN prompt_templates t ON t.id = gi.template_id
                WHERE g.architecture = ? AND g.is_default = 1 AND gi.stage = ?"#
        )
        .bind(arch_s)
        .bind(stage_s)
        .fetch_optional(&self.pool).await? {
            let content: String = row.get::<String, _>("content");
            return Ok(Some(content));
        }
        // 否则取默认/最新活动模板
        if let Some(t) = self.get_template_by_arch_stage(arch, stage).await? {
            return Ok(Some(t.content));
        }
        Ok(None)
    }

    // ===== Prompt Groups =====
    pub async fn list_groups(&self, arch: Option<ArchitectureType>) -> Result<Vec<PromptGroup>> {
        let rows = if let Some(a) = arch {
            let arch_s = Self::arch_str(&a);
            sqlx::query(r#"SELECT id, architecture, name, description, is_default, created_at, updated_at FROM prompt_groups WHERE architecture = ? ORDER BY is_default DESC, updated_at DESC"#)
                .bind(arch_s)
                .fetch_all(&self.pool).await?
        } else {
            sqlx::query(r#"SELECT id, architecture, name, description, is_default, created_at, updated_at FROM prompt_groups ORDER BY is_default DESC, updated_at DESC"#)
                .fetch_all(&self.pool).await?
        };
        Ok(rows.into_iter().map(Self::row_to_group).collect())
    }

    pub async fn create_group(&self, g: &PromptGroup) -> Result<i64> {
        let arch_s = Self::arch_str(&g.architecture);
        let res = sqlx::query(r#"INSERT INTO prompt_groups (architecture, name, description, is_default) VALUES (?, ?, ?, ?)"#)
            .bind(arch_s)
            .bind(&g.name)
            .bind(&g.description)
            .bind(if g.is_default { 1 } else { 0 })
            .execute(&self.pool).await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn update_group(&self, id: i64, g: &PromptGroup) -> Result<()> {
        let arch_s = Self::arch_str(&g.architecture);
        sqlx::query(r#"UPDATE prompt_groups SET architecture = ?, name = ?, description = ?, is_default = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"#)
            .bind(arch_s)
            .bind(&g.name)
            .bind(&g.description)
            .bind(if g.is_default { 1 } else { 0 })
            .bind(id)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn delete_group(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM prompt_groups WHERE id = ?").bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_arch_default_group(&self, arch: ArchitectureType, group_id: i64) -> Result<()> {
        let arch_s = Self::arch_str(&arch);
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE prompt_groups SET is_default = 0 WHERE architecture = ?")
            .bind(arch_s)
            .execute(&mut *tx).await?;
        sqlx::query("UPDATE prompt_groups SET is_default = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(group_id)
            .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn upsert_group_item(&self, group_id: i64, stage: StageType, template_id: i64) -> Result<()> {
        let stage_s = Self::stage_str(&stage);
        sqlx::query(r#"INSERT INTO prompt_group_items (group_id, stage, template_id) VALUES (?, ?, ?)
            ON CONFLICT(group_id, stage) DO UPDATE SET template_id = excluded.template_id, updated_at = CURRENT_TIMESTAMP"#)
            .bind(group_id)
            .bind(stage_s)
            .bind(template_id)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn list_group_items(&self, group_id: i64) -> Result<Vec<PromptGroupItem>> {
        let rows = sqlx::query(r#"SELECT id, group_id, stage, template_id, created_at, updated_at FROM prompt_group_items WHERE group_id = ?"#)
            .bind(group_id)
            .fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(Self::row_to_group_item).collect())
    }

    pub async fn remove_group_item(&self, group_id: i64, stage: StageType) -> Result<()> {
        let stage_s = Self::stage_str(&stage);
        sqlx::query("DELETE FROM prompt_group_items WHERE group_id = ? AND stage = ?")
            .bind(group_id)
            .bind(stage_s)
            .execute(&self.pool).await?;
        Ok(())
    }

    fn row_to_template(row: SqliteRow) -> PromptTemplate {
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
        }
    }

    fn row_to_group(row: SqliteRow) -> PromptGroup {
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

    fn row_to_group_item(row: SqliteRow) -> PromptGroupItem {
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
            "replan" => StageType::Replan,
            _ => StageType::Replan, // Default to Replan for unknown stages
        }
    }
}



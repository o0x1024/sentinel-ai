use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, Row};

/// Level 1: Basic metadata (for LLM selection phase 1 - initial discovery)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSummary {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// Level 2: Detailed information (for LLM selection phase 2 - understanding purpose)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDetail {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_path: String,
    pub argument_hint: String,
    pub disable_model_invocation: bool,
    pub user_invocable: bool,
    pub model: String,
    pub context: String,
    pub agent: String,
    pub hooks: Option<Value>,
    pub allowed_tools_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Level 3: Full details (for phase 3 - execution and management)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_path: String,
    pub argument_hint: String,
    pub disable_model_invocation: bool,
    pub user_invocable: bool,
    pub allowed_tools: Vec<String>,
    pub model: String,
    pub context: String,
    pub agent: String,
    pub hooks: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_path: String,
    pub argument_hint: String,
    pub disable_model_invocation: bool,
    pub user_invocable: bool,
    pub allowed_tools: Vec<String>,
    pub model: String,
    pub context: String,
    pub agent: String,
    pub hooks: Option<Value>,
}

/// Update payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSkill {
    pub name: Option<String>,
    pub description: Option<String>,
    pub source_path: Option<String>,
    pub argument_hint: Option<String>,
    pub disable_model_invocation: Option<bool>,
    pub user_invocable: Option<bool>,
    pub allowed_tools: Option<Vec<String>>,
    pub model: Option<String>,
    pub context: Option<String>,
    pub agent: Option<String>,
    pub hooks: Option<Value>,
}

#[derive(Debug, Clone, FromRow)]
struct SkillDbRow {
    id: String,
    name: String,
    description: String,
    source_path: String,
    argument_hint: String,
    disable_model_invocation: bool,
    user_invocable: bool,
    allowed_tools: String,
    model: String,
    context: String,
    agent: String,
    hooks: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SkillDbRow {
    fn into_skill(self) -> Skill {
        let allowed_tools: Vec<String> =
            serde_json::from_str(&self.allowed_tools).unwrap_or_default();
        let hooks: Option<Value> = serde_json::from_str(&self.hooks).ok();
        Skill {
            id: self.id,
            name: self.name,
            description: self.description,
            source_path: self.source_path,
            argument_hint: self.argument_hint,
            disable_model_invocation: self.disable_model_invocation,
            user_invocable: self.user_invocable,
            allowed_tools,
            model: self.model,
            context: self.context,
            agent: self.agent,
            hooks,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl DatabaseService {
    /// List all skills (summary only)
    pub async fn list_skills_summary_internal(&self) -> Result<Vec<SkillSummary>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let skills = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let rows = sqlx::query("SELECT id, name, description FROM skills ORDER BY name")
                    .fetch_all(pool)
                    .await?;
                rows.into_iter()
                    .map(|row| SkillSummary {
                        id: row.get("id"),
                        name: row.get("name"),
                        description: row.get("description"),
                    })
                    .collect()
            }
            DatabasePool::SQLite(pool) => {
                let rows = sqlx::query("SELECT id, name, description FROM skills ORDER BY name")
                    .fetch_all(pool)
                    .await?;
                rows.into_iter()
                    .map(|row| SkillSummary {
                        id: row.get("id"),
                        name: row.get("name"),
                        description: row.get("description"),
                    })
                    .collect()
            }
            DatabasePool::MySQL(pool) => {
                let rows = sqlx::query("SELECT id, name, description FROM skills ORDER BY name")
                    .fetch_all(pool)
                    .await?;
                rows.into_iter()
                    .map(|row| SkillSummary {
                        id: row.get("id"),
                        name: row.get("name"),
                        description: row.get("description"),
                    })
                    .collect()
            }
        };

        Ok(skills)
    }

    /// List skills by allowed IDs (summary only)
    pub async fn list_skills_summary_by_ids_internal(
        &self,
        ids: &[String],
    ) -> Result<Vec<SkillSummary>> {
        if ids.is_empty() {
            return self.list_skills_summary_internal().await;
        }

        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let skills = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let placeholders = (1..=ids.len())
                    .map(|i| format!("${}", i))
                    .collect::<Vec<_>>()
                    .join(",");
                let query = format!(
                    "SELECT id, name, description FROM skills WHERE id IN ({}) ORDER BY name",
                    placeholders
                );
                let mut q = sqlx::query(&query);
                for id in ids {
                    q = q.bind(id);
                }
                let rows = q.fetch_all(pool).await?;
                rows.into_iter()
                    .map(|row| SkillSummary {
                        id: row.get("id"),
                        name: row.get("name"),
                        description: row.get("description"),
                    })
                    .collect()
            }
            DatabasePool::SQLite(pool) => {
                let placeholders = std::iter::repeat_n("?", ids.len())
                    .collect::<Vec<_>>()
                    .join(",");
                let query = format!(
                    "SELECT id, name, description FROM skills WHERE id IN ({}) ORDER BY name",
                    placeholders
                );
                let mut q = sqlx::query(&query);
                for id in ids {
                    q = q.bind(id);
                }
                let rows = q.fetch_all(pool).await?;
                rows.into_iter()
                    .map(|row| SkillSummary {
                        id: row.get("id"),
                        name: row.get("name"),
                        description: row.get("description"),
                    })
                    .collect()
            }
            DatabasePool::MySQL(pool) => {
                let placeholders = std::iter::repeat_n("?", ids.len())
                    .collect::<Vec<_>>()
                    .join(",");
                let query = format!(
                    "SELECT id, name, description FROM skills WHERE id IN ({}) ORDER BY name",
                    placeholders
                );
                let mut q = sqlx::query(&query);
                for id in ids {
                    q = q.bind(id);
                }
                let rows = q.fetch_all(pool).await?;
                rows.into_iter()
                    .map(|row| SkillSummary {
                        id: row.get("id"),
                        name: row.get("name"),
                        description: row.get("description"),
                    })
                    .collect()
            }
        };

        Ok(skills)
    }

    /// Get Level 2 detail by ID (without allowed_tools)
    pub async fn get_skill_detail_internal(&self, id: &str) -> Result<Option<SkillDetail>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let detail = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let row = sqlx::query(
                    "SELECT id, name, description, source_path, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at FROM skills WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?;
                row.map(|r| {
                    let allowed_tools_str: String = r.get("allowed_tools");
                    let allowed_tools: Vec<String> =
                        serde_json::from_str(&allowed_tools_str).unwrap_or_default();
                    let hooks_str: String = r.get("hooks");
                    let hooks: Option<Value> = serde_json::from_str(&hooks_str).ok();
                    SkillDetail {
                        id: r.get("id"),
                        name: r.get("name"),
                        description: r.get("description"),
                        source_path: r.get("source_path"),
                        argument_hint: r.get("argument_hint"),
                        disable_model_invocation: r.get("disable_model_invocation"),
                        user_invocable: r.get("user_invocable"),
                        model: r.get("model"),
                        context: r.get("context"),
                        agent: r.get("agent"),
                        hooks,
                        allowed_tools_count: allowed_tools.len(),
                        created_at: r.get("created_at"),
                        updated_at: r.get("updated_at"),
                    }
                })
            }
            DatabasePool::SQLite(pool) => {
                let row = sqlx::query(
                    "SELECT id, name, description, source_path, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at FROM skills WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?;
                row.map(|r| {
                    let allowed_tools_str: String = r.get("allowed_tools");
                    let allowed_tools: Vec<String> =
                        serde_json::from_str(&allowed_tools_str).unwrap_or_default();
                    let hooks_str: String = r.get("hooks");
                    let hooks: Option<Value> = serde_json::from_str(&hooks_str).ok();
                    SkillDetail {
                        id: r.get("id"),
                        name: r.get("name"),
                        description: r.get("description"),
                        source_path: r.get("source_path"),
                        argument_hint: r.get("argument_hint"),
                        disable_model_invocation: r.get("disable_model_invocation"),
                        user_invocable: r.get("user_invocable"),
                        model: r.get("model"),
                        context: r.get("context"),
                        agent: r.get("agent"),
                        hooks,
                        allowed_tools_count: allowed_tools.len(),
                        created_at: r.get("created_at"),
                        updated_at: r.get("updated_at"),
                    }
                })
            }
            DatabasePool::MySQL(pool) => {
                let row = sqlx::query(
                    "SELECT id, name, description, source_path, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at FROM skills WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?;
                row.map(|r| {
                    let allowed_tools_str: String = r.get("allowed_tools");
                    let allowed_tools: Vec<String> =
                        serde_json::from_str(&allowed_tools_str).unwrap_or_default();
                    let hooks_str: String = r.get("hooks");
                    let hooks: Option<Value> = serde_json::from_str(&hooks_str).ok();
                    SkillDetail {
                        id: r.get("id"),
                        name: r.get("name"),
                        description: r.get("description"),
                        source_path: r.get("source_path"),
                        argument_hint: r.get("argument_hint"),
                        disable_model_invocation: r.get("disable_model_invocation"),
                        user_invocable: r.get("user_invocable"),
                        model: r.get("model"),
                        context: r.get("context"),
                        agent: r.get("agent"),
                        hooks,
                        allowed_tools_count: allowed_tools.len(),
                        created_at: r.get("created_at"),
                        updated_at: r.get("updated_at"),
                    }
                })
            }
        };

        Ok(detail)
    }

    /// Get Level 3 full skill by ID (with allowed_tools)
    pub async fn get_skill_internal(&self, id: &str) -> Result<Option<Skill>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let row = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, SkillDbRow>(
                    "SELECT id, name, description, source_path, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at FROM skills WHERE id = $1"
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SkillDbRow>(
                    "SELECT id, name, description, source_path, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at FROM skills WHERE id = ?"
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, SkillDbRow>(
                    "SELECT id, name, description, source_path, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at FROM skills WHERE id = ?"
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(row.map(|r| r.into_skill()))
    }

    /// Get full skill by name
    pub async fn get_skill_by_name_internal(&self, name: &str) -> Result<Option<Skill>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let row = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, SkillDbRow>(
                    "SELECT id, name, description, source_path, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at FROM skills WHERE name = $1"
                )
                .bind(name)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SkillDbRow>(
                    "SELECT id, name, description, source_path, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at FROM skills WHERE name = ?"
                )
                .bind(name)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, SkillDbRow>(
                    "SELECT id, name, description, source_path, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at FROM skills WHERE name = ?"
                )
                .bind(name)
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(row.map(|r| r.into_skill()))
    }

    /// List all skills (full)
    pub async fn list_all_skills_internal(&self) -> Result<Vec<Skill>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, SkillDbRow>(
                    "SELECT id, name, description, source_path, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at FROM skills ORDER BY name"
                )
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SkillDbRow>(
                    "SELECT id, name, description, source_path, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at FROM skills ORDER BY name"
                )
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, SkillDbRow>(
                    "SELECT id, name, description, source_path, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at FROM skills ORDER BY name"
                )
                .fetch_all(pool)
                .await?
            }
        };

        Ok(rows.into_iter().map(|r| r.into_skill()).collect())
    }

    /// Create a new skill
    pub async fn create_skill_internal(&self, payload: &CreateSkill) -> Result<Skill> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let id = payload.id.clone();
        let now = Utc::now();
        let allowed_tools_json = serde_json::to_string(&payload.allowed_tools)?;
        let hooks_json = serde_json::to_string(
            &payload
                .hooks
                .clone()
                .unwrap_or_else(|| Value::Object(Default::default())),
        )?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "INSERT INTO skills (id, name, description, source_path, content, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, '', $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
                )
                .bind(&id)
                .bind(&payload.name)
                .bind(&payload.description)
                .bind(&payload.source_path)
                .bind(&payload.argument_hint)
                .bind(payload.disable_model_invocation)
                .bind(payload.user_invocable)
                .bind(&allowed_tools_json)
                .bind(&payload.model)
                .bind(&payload.context)
                .bind(&payload.agent)
                .bind(&hooks_json)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "INSERT INTO skills (id, name, description, source_path, content, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at)
                     VALUES (?, ?, ?, ?, '', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(&payload.name)
                .bind(&payload.description)
                .bind(&payload.source_path)
                .bind(&payload.argument_hint)
                .bind(payload.disable_model_invocation)
                .bind(payload.user_invocable)
                .bind(&allowed_tools_json)
                .bind(&payload.model)
                .bind(&payload.context)
                .bind(&payload.agent)
                .bind(&hooks_json)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "INSERT INTO skills (id, name, description, source_path, content, argument_hint, disable_model_invocation, user_invocable, allowed_tools, model, context, agent, hooks, created_at, updated_at)
                     VALUES (?, ?, ?, ?, '', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(&payload.name)
                .bind(&payload.description)
                .bind(&payload.source_path)
                .bind(&payload.argument_hint)
                .bind(payload.disable_model_invocation)
                .bind(payload.user_invocable)
                .bind(&allowed_tools_json)
                .bind(&payload.model)
                .bind(&payload.context)
                .bind(&payload.agent)
                .bind(&hooks_json)
                .bind(now)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        Ok(Skill {
            id,
            name: payload.name.clone(),
            description: payload.description.clone(),
            source_path: payload.source_path.clone(),
            argument_hint: payload.argument_hint.clone(),
            disable_model_invocation: payload.disable_model_invocation,
            user_invocable: payload.user_invocable,
            allowed_tools: payload.allowed_tools.clone(),
            model: payload.model.clone(),
            context: payload.context.clone(),
            agent: payload.agent.clone(),
            hooks: payload.hooks.clone(),
            created_at: now,
            updated_at: now,
        })
    }

    /// Update an existing skill
    pub async fn update_skill_internal(&self, id: &str, payload: &UpdateSkill) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let existing = self.get_skill_internal(id).await?;
        if existing.is_none() {
            return Ok(false);
        }
        let existing = existing.unwrap();

        let name = payload.name.as_ref().unwrap_or(&existing.name);
        let description = payload
            .description
            .as_ref()
            .unwrap_or(&existing.description);
        let source_path = payload
            .source_path
            .as_ref()
            .unwrap_or(&existing.source_path);
        let argument_hint = payload
            .argument_hint
            .as_ref()
            .unwrap_or(&existing.argument_hint);
        let disable_model_invocation = payload
            .disable_model_invocation
            .unwrap_or(existing.disable_model_invocation);
        let user_invocable = payload.user_invocable.unwrap_or(existing.user_invocable);
        let allowed_tools = payload
            .allowed_tools
            .as_ref()
            .unwrap_or(&existing.allowed_tools);
        let model = payload.model.as_ref().unwrap_or(&existing.model);
        let context = payload.context.as_ref().unwrap_or(&existing.context);
        let agent = payload.agent.as_ref().unwrap_or(&existing.agent);
        let hooks = payload.hooks.as_ref().or(existing.hooks.as_ref());

        let allowed_tools_json = serde_json::to_string(allowed_tools)?;
        let hooks_json = serde_json::to_string(
            &hooks
                .cloned()
                .unwrap_or_else(|| Value::Object(Default::default())),
        )?;
        let now = Utc::now();

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    "UPDATE skills SET name = $1, description = $2, source_path = $3, argument_hint = $4, disable_model_invocation = $5, user_invocable = $6, allowed_tools = $7, model = $8, context = $9, agent = $10, hooks = $11, updated_at = $12 WHERE id = $13",
                )
                .bind(name)
                .bind(description)
                .bind(source_path)
                .bind(argument_hint)
                .bind(disable_model_invocation)
                .bind(user_invocable)
                .bind(&allowed_tools_json)
                .bind(model)
                .bind(context)
                .bind(agent)
                .bind(&hooks_json)
                .bind(now)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    "UPDATE skills SET name = ?, description = ?, source_path = ?, argument_hint = ?, disable_model_invocation = ?, user_invocable = ?, allowed_tools = ?, model = ?, context = ?, agent = ?, hooks = ?, updated_at = ? WHERE id = ?",
                )
                .bind(name)
                .bind(description)
                .bind(source_path)
                .bind(argument_hint)
                .bind(disable_model_invocation)
                .bind(user_invocable)
                .bind(&allowed_tools_json)
                .bind(model)
                .bind(context)
                .bind(agent)
                .bind(&hooks_json)
                .bind(now)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    "UPDATE skills SET name = ?, description = ?, source_path = ?, argument_hint = ?, disable_model_invocation = ?, user_invocable = ?, allowed_tools = ?, model = ?, context = ?, agent = ?, hooks = ?, updated_at = ? WHERE id = ?",
                )
                .bind(name)
                .bind(description)
                .bind(source_path)
                .bind(argument_hint)
                .bind(disable_model_invocation)
                .bind(user_invocable)
                .bind(&allowed_tools_json)
                .bind(model)
                .bind(context)
                .bind(agent)
                .bind(&hooks_json)
                .bind(now)
                .bind(id)
                .execute(pool)
                .await?;
            }
        }

        Ok(true)
    }

    /// Delete a skill
    pub async fn delete_skill_internal(&self, id: &str) -> Result<bool> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let rows_affected = match runtime {
            DatabasePool::PostgreSQL(pool) => sqlx::query("DELETE FROM skills WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await?
                .rows_affected(),
            DatabasePool::SQLite(pool) => sqlx::query("DELETE FROM skills WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?
                .rows_affected(),
            DatabasePool::MySQL(pool) => sqlx::query("DELETE FROM skills WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?
                .rows_affected(),
        };

        Ok(rows_affected > 0)
    }
}

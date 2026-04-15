use crate::database_service::connection_manager::DatabasePool;
use crate::database_service::service::DatabaseService;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemAgentProfileRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    pub mode: String,
    pub capability: String,
    pub enabled: bool,
    pub trigger_mode: String,
    pub llm_provider_override: Option<String>,
    pub llm_model_override: Option<String>,
    pub base_prompt_id: Option<String>,
    pub prompt_patch: Option<String>,
    pub sop_definitions_json: String,
    pub input_schema_json: String,
    pub output_schema_json: String,
    pub required_tools_json: String,
    pub optional_tools_json: String,
    pub forbidden_tools_json: String,
    pub trigger_events_json: String,
    pub budget_json: String,
    pub safety_policy_json: String,
    pub cooldown_secs: i64,
    pub max_concurrency: i64,
    pub risk_level: String,
    pub visibility: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemAgentBindingRecord {
    pub id: String,
    pub profile_id: String,
    pub event_name: String,
    pub filter_json: String,
    pub priority: i64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemAgentRunRecord {
    pub id: String,
    pub profile_id: String,
    pub trigger_event: Option<String>,
    pub status: String,
    pub input_summary_json: String,
    pub tool_calls: Option<String>,
    pub output_json: Option<String>,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemAgentProfileVersionRecord {
    pub id: String,
    pub profile_id: String,
    pub snapshot_json: String,
    pub created_at: DateTime<Utc>,
}

impl DatabaseService {
    pub async fn list_system_agent_profiles_internal(
        &self,
    ) -> Result<Vec<SystemAgentProfileRecord>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let profiles = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, SystemAgentProfileRecord>(
                    "SELECT * FROM system_agent_profiles ORDER BY updated_at DESC, name ASC",
                )
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SystemAgentProfileRecord>(
                    "SELECT * FROM system_agent_profiles ORDER BY updated_at DESC, name ASC",
                )
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, SystemAgentProfileRecord>(
                    "SELECT * FROM system_agent_profiles ORDER BY updated_at DESC, name ASC",
                )
                .fetch_all(pool)
                .await?
            }
        };
        Ok(profiles)
    }

    pub async fn get_system_agent_profile_internal(
        &self,
        id: &str,
    ) -> Result<Option<SystemAgentProfileRecord>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let profile = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, SystemAgentProfileRecord>(
                    "SELECT * FROM system_agent_profiles WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SystemAgentProfileRecord>(
                    "SELECT * FROM system_agent_profiles WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, SystemAgentProfileRecord>(
                    "SELECT * FROM system_agent_profiles WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
        };
        Ok(profile)
    }

    pub async fn save_system_agent_profile_internal(
        &self,
        profile: &SystemAgentProfileRecord,
        bindings: &[SystemAgentBindingRecord],
    ) -> Result<()> {
        let exists = self
            .get_system_agent_profile_internal(&profile.id)
            .await?
            .is_some();
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                if exists {
                    sqlx::query(
                        r#"
                        UPDATE system_agent_profiles
                        SET name = $1,
                            description = $2,
                            mode = $3,
                            capability = $4,
                            enabled = $5,
                            trigger_mode = $6,
                            llm_provider_override = $7,
                            llm_model_override = $8,
                            base_prompt_id = $9,
                            prompt_patch = $10,
                            sop_definitions_json = $11,
                            input_schema_json = $12,
                            output_schema_json = $13,
                            required_tools_json = $14,
                            optional_tools_json = $15,
                            forbidden_tools_json = $16,
                            trigger_events_json = $17,
                            budget_json = $18,
                            safety_policy_json = $19,
                            cooldown_secs = $20,
                            max_concurrency = $21,
                            risk_level = $22,
                            visibility = $23,
                            updated_at = CURRENT_TIMESTAMP
                        WHERE id = $24
                        "#,
                    )
                    .bind(&profile.name)
                    .bind(&profile.description)
                    .bind(&profile.mode)
                    .bind(&profile.capability)
                    .bind(profile.enabled)
                    .bind(&profile.trigger_mode)
                    .bind(&profile.llm_provider_override)
                    .bind(&profile.llm_model_override)
                    .bind(&profile.base_prompt_id)
                    .bind(&profile.prompt_patch)
                    .bind(&profile.sop_definitions_json)
                    .bind(&profile.input_schema_json)
                    .bind(&profile.output_schema_json)
                    .bind(&profile.required_tools_json)
                    .bind(&profile.optional_tools_json)
                    .bind(&profile.forbidden_tools_json)
                    .bind(&profile.trigger_events_json)
                    .bind(&profile.budget_json)
                    .bind(&profile.safety_policy_json)
                    .bind(profile.cooldown_secs)
                    .bind(profile.max_concurrency)
                    .bind(&profile.risk_level)
                    .bind(&profile.visibility)
                    .bind(&profile.id)
                    .execute(pool)
                    .await?;
                } else {
                    sqlx::query(
                        r#"
                        INSERT INTO system_agent_profiles (
                            id, name, description, mode, capability, enabled, trigger_mode,
                            llm_provider_override, llm_model_override,
                            base_prompt_id, prompt_patch, sop_definitions_json,
                            input_schema_json, output_schema_json, required_tools_json,
                            optional_tools_json, forbidden_tools_json, trigger_events_json,
                            budget_json, safety_policy_json, cooldown_secs, max_concurrency,
                            risk_level, visibility
                        ) VALUES (
                            $1, $2, $3, $4, $5, $6, $7,
                            $8, $9, $10, $11, $12, $13, $14,
                            $15, $16, $17, $18, $19, $20, $21,
                            $22, $23, $24
                        )
                        "#,
                    )
                    .bind(&profile.id)
                    .bind(&profile.name)
                    .bind(&profile.description)
                    .bind(&profile.mode)
                    .bind(&profile.capability)
                    .bind(profile.enabled)
                    .bind(&profile.trigger_mode)
                    .bind(&profile.llm_provider_override)
                    .bind(&profile.llm_model_override)
                    .bind(&profile.base_prompt_id)
                    .bind(&profile.prompt_patch)
                    .bind(&profile.sop_definitions_json)
                    .bind(&profile.input_schema_json)
                    .bind(&profile.output_schema_json)
                    .bind(&profile.required_tools_json)
                    .bind(&profile.optional_tools_json)
                    .bind(&profile.forbidden_tools_json)
                    .bind(&profile.trigger_events_json)
                    .bind(&profile.budget_json)
                    .bind(&profile.safety_policy_json)
                    .bind(profile.cooldown_secs)
                    .bind(profile.max_concurrency)
                    .bind(&profile.risk_level)
                    .bind(&profile.visibility)
                    .execute(pool)
                    .await?;
                }

                sqlx::query("DELETE FROM system_agent_bindings WHERE profile_id = $1")
                    .bind(&profile.id)
                    .execute(pool)
                    .await?;

                for binding in bindings {
                    sqlx::query(
                        r#"
                        INSERT INTO system_agent_bindings (
                            id, profile_id, event_name, filter_json, priority, enabled
                        ) VALUES ($1, $2, $3, $4, $5, $6)
                        "#,
                    )
                    .bind(&binding.id)
                    .bind(&binding.profile_id)
                    .bind(&binding.event_name)
                    .bind(&binding.filter_json)
                    .bind(binding.priority)
                    .bind(binding.enabled)
                    .execute(pool)
                    .await?;
                }
            }
            DatabasePool::SQLite(pool) => {
                if exists {
                    sqlx::query(
                        r#"
                        UPDATE system_agent_profiles
                        SET name = ?,
                            description = ?,
                            mode = ?,
                            capability = ?,
                            enabled = ?,
                            trigger_mode = ?,
                            llm_provider_override = ?,
                            llm_model_override = ?,
                            base_prompt_id = ?,
                            prompt_patch = ?,
                            sop_definitions_json = ?,
                            input_schema_json = ?,
                            output_schema_json = ?,
                            required_tools_json = ?,
                            optional_tools_json = ?,
                            forbidden_tools_json = ?,
                            trigger_events_json = ?,
                            budget_json = ?,
                            safety_policy_json = ?,
                            cooldown_secs = ?,
                            max_concurrency = ?,
                            risk_level = ?,
                            visibility = ?,
                            updated_at = CURRENT_TIMESTAMP
                        WHERE id = ?
                        "#,
                    )
                    .bind(&profile.name)
                    .bind(&profile.description)
                    .bind(&profile.mode)
                    .bind(&profile.capability)
                    .bind(profile.enabled)
                    .bind(&profile.trigger_mode)
                    .bind(&profile.llm_provider_override)
                    .bind(&profile.llm_model_override)
                    .bind(&profile.base_prompt_id)
                    .bind(&profile.prompt_patch)
                    .bind(&profile.sop_definitions_json)
                    .bind(&profile.input_schema_json)
                    .bind(&profile.output_schema_json)
                    .bind(&profile.required_tools_json)
                    .bind(&profile.optional_tools_json)
                    .bind(&profile.forbidden_tools_json)
                    .bind(&profile.trigger_events_json)
                    .bind(&profile.budget_json)
                    .bind(&profile.safety_policy_json)
                    .bind(profile.cooldown_secs)
                    .bind(profile.max_concurrency)
                    .bind(&profile.risk_level)
                    .bind(&profile.visibility)
                    .bind(&profile.id)
                    .execute(pool)
                    .await?;
                } else {
                    sqlx::query(
                        r#"
                        INSERT INTO system_agent_profiles (
                            id, name, description, mode, capability, enabled, trigger_mode,
                            llm_provider_override, llm_model_override,
                            base_prompt_id, prompt_patch, sop_definitions_json,
                            input_schema_json, output_schema_json, required_tools_json,
                            optional_tools_json, forbidden_tools_json, trigger_events_json,
                            budget_json, safety_policy_json, cooldown_secs, max_concurrency,
                            risk_level, visibility
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        "#,
                    )
                    .bind(&profile.id)
                    .bind(&profile.name)
                    .bind(&profile.description)
                    .bind(&profile.mode)
                    .bind(&profile.capability)
                    .bind(profile.enabled)
                    .bind(&profile.trigger_mode)
                    .bind(&profile.llm_provider_override)
                    .bind(&profile.llm_model_override)
                    .bind(&profile.base_prompt_id)
                    .bind(&profile.prompt_patch)
                    .bind(&profile.sop_definitions_json)
                    .bind(&profile.input_schema_json)
                    .bind(&profile.output_schema_json)
                    .bind(&profile.required_tools_json)
                    .bind(&profile.optional_tools_json)
                    .bind(&profile.forbidden_tools_json)
                    .bind(&profile.trigger_events_json)
                    .bind(&profile.budget_json)
                    .bind(&profile.safety_policy_json)
                    .bind(profile.cooldown_secs)
                    .bind(profile.max_concurrency)
                    .bind(&profile.risk_level)
                    .bind(&profile.visibility)
                    .execute(pool)
                    .await?;
                }

                sqlx::query("DELETE FROM system_agent_bindings WHERE profile_id = ?")
                    .bind(&profile.id)
                    .execute(pool)
                    .await?;

                for binding in bindings {
                    sqlx::query(
                        r#"
                        INSERT INTO system_agent_bindings (
                            id, profile_id, event_name, filter_json, priority, enabled
                        ) VALUES (?, ?, ?, ?, ?, ?)
                        "#,
                    )
                    .bind(&binding.id)
                    .bind(&binding.profile_id)
                    .bind(&binding.event_name)
                    .bind(&binding.filter_json)
                    .bind(binding.priority)
                    .bind(binding.enabled)
                    .execute(pool)
                    .await?;
                }
            }
            DatabasePool::MySQL(pool) => {
                if exists {
                    sqlx::query(
                        r#"
                        UPDATE system_agent_profiles
                        SET name = ?,
                            description = ?,
                            mode = ?,
                            capability = ?,
                            enabled = ?,
                            trigger_mode = ?,
                            llm_provider_override = ?,
                            llm_model_override = ?,
                            base_prompt_id = ?,
                            prompt_patch = ?,
                            sop_definitions_json = ?,
                            input_schema_json = ?,
                            output_schema_json = ?,
                            required_tools_json = ?,
                            optional_tools_json = ?,
                            forbidden_tools_json = ?,
                            trigger_events_json = ?,
                            budget_json = ?,
                            safety_policy_json = ?,
                            cooldown_secs = ?,
                            max_concurrency = ?,
                            risk_level = ?,
                            visibility = ?,
                            updated_at = CURRENT_TIMESTAMP
                        WHERE id = ?
                        "#,
                    )
                    .bind(&profile.name)
                    .bind(&profile.description)
                    .bind(&profile.mode)
                    .bind(&profile.capability)
                    .bind(profile.enabled)
                    .bind(&profile.trigger_mode)
                    .bind(&profile.llm_provider_override)
                    .bind(&profile.llm_model_override)
                    .bind(&profile.base_prompt_id)
                    .bind(&profile.prompt_patch)
                    .bind(&profile.sop_definitions_json)
                    .bind(&profile.input_schema_json)
                    .bind(&profile.output_schema_json)
                    .bind(&profile.required_tools_json)
                    .bind(&profile.optional_tools_json)
                    .bind(&profile.forbidden_tools_json)
                    .bind(&profile.trigger_events_json)
                    .bind(&profile.budget_json)
                    .bind(&profile.safety_policy_json)
                    .bind(profile.cooldown_secs)
                    .bind(profile.max_concurrency)
                    .bind(&profile.risk_level)
                    .bind(&profile.visibility)
                    .bind(&profile.id)
                    .execute(pool)
                    .await?;
                } else {
                    sqlx::query(
                        r#"
                        INSERT INTO system_agent_profiles (
                            id, name, description, mode, capability, enabled, trigger_mode,
                            llm_provider_override, llm_model_override,
                            base_prompt_id, prompt_patch, sop_definitions_json,
                            input_schema_json, output_schema_json, required_tools_json,
                            optional_tools_json, forbidden_tools_json, trigger_events_json,
                            budget_json, safety_policy_json, cooldown_secs, max_concurrency,
                            risk_level, visibility
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        "#,
                    )
                    .bind(&profile.id)
                    .bind(&profile.name)
                    .bind(&profile.description)
                    .bind(&profile.mode)
                    .bind(&profile.capability)
                    .bind(profile.enabled)
                    .bind(&profile.trigger_mode)
                    .bind(&profile.llm_provider_override)
                    .bind(&profile.llm_model_override)
                    .bind(&profile.base_prompt_id)
                    .bind(&profile.prompt_patch)
                    .bind(&profile.sop_definitions_json)
                    .bind(&profile.input_schema_json)
                    .bind(&profile.output_schema_json)
                    .bind(&profile.required_tools_json)
                    .bind(&profile.optional_tools_json)
                    .bind(&profile.forbidden_tools_json)
                    .bind(&profile.trigger_events_json)
                    .bind(&profile.budget_json)
                    .bind(&profile.safety_policy_json)
                    .bind(profile.cooldown_secs)
                    .bind(profile.max_concurrency)
                    .bind(&profile.risk_level)
                    .bind(&profile.visibility)
                    .execute(pool)
                    .await?;
                }

                sqlx::query("DELETE FROM system_agent_bindings WHERE profile_id = ?")
                    .bind(&profile.id)
                    .execute(pool)
                    .await?;

                for binding in bindings {
                    sqlx::query(
                        r#"
                        INSERT INTO system_agent_bindings (
                            id, profile_id, event_name, filter_json, priority, enabled
                        ) VALUES (?, ?, ?, ?, ?, ?)
                        "#,
                    )
                    .bind(&binding.id)
                    .bind(&binding.profile_id)
                    .bind(&binding.event_name)
                    .bind(&binding.filter_json)
                    .bind(binding.priority)
                    .bind(binding.enabled)
                    .execute(pool)
                    .await?;
                }
            }
        }

        let snapshot_json = serde_json::to_string(&json!({
            "profile": profile,
            "bindings": bindings,
        }))?;
        let version = SystemAgentProfileVersionRecord {
            id: format!("sapv-{}", uuid::Uuid::new_v4()),
            profile_id: profile.id.clone(),
            snapshot_json,
            created_at: Utc::now(),
        };
        self.insert_system_agent_profile_version_internal(&version)
            .await?;

        Ok(())
    }

    pub async fn list_system_agent_profile_versions_internal(
        &self,
        profile_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<SystemAgentProfileVersionRecord>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let limit = limit.unwrap_or(20) as i64;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                let records = sqlx::query_as::<_, SystemAgentProfileVersionRecord>(
                    r#"
                    SELECT id, profile_id, snapshot_json, created_at
                    FROM system_agent_profile_versions
                    WHERE profile_id = $1
                    ORDER BY created_at DESC
                    LIMIT $2
                    "#,
                )
                .bind(profile_id)
                .bind(limit)
                .fetch_all(pool)
                .await?;
                Ok(records)
            }
            DatabasePool::SQLite(pool) => {
                let records = sqlx::query_as::<_, SystemAgentProfileVersionRecord>(
                    r#"
                    SELECT id, profile_id, snapshot_json, created_at
                    FROM system_agent_profile_versions
                    WHERE profile_id = ?
                    ORDER BY created_at DESC
                    LIMIT ?
                    "#,
                )
                .bind(profile_id)
                .bind(limit)
                .fetch_all(pool)
                .await?;
                Ok(records)
            }
            DatabasePool::MySQL(pool) => {
                let records = sqlx::query_as::<_, SystemAgentProfileVersionRecord>(
                    r#"
                    SELECT id, profile_id, snapshot_json, created_at
                    FROM system_agent_profile_versions
                    WHERE profile_id = ?
                    ORDER BY created_at DESC
                    LIMIT ?
                    "#,
                )
                .bind(profile_id)
                .bind(limit)
                .fetch_all(pool)
                .await?;
                Ok(records)
            }
        }
    }

    pub async fn insert_system_agent_profile_version_internal(
        &self,
        version: &SystemAgentProfileVersionRecord,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO system_agent_profile_versions (
                        id, profile_id, snapshot_json, created_at
                    ) VALUES ($1, $2, $3, $4)
                    "#,
                )
                .bind(&version.id)
                .bind(&version.profile_id)
                .bind(&version.snapshot_json)
                .bind(version.created_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO system_agent_profile_versions (
                        id, profile_id, snapshot_json, created_at
                    ) VALUES (?, ?, ?, ?)
                    "#,
                )
                .bind(&version.id)
                .bind(&version.profile_id)
                .bind(&version.snapshot_json)
                .bind(version.created_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO system_agent_profile_versions (
                        id, profile_id, snapshot_json, created_at
                    ) VALUES (?, ?, ?, ?)
                    "#,
                )
                .bind(&version.id)
                .bind(&version.profile_id)
                .bind(&version.snapshot_json)
                .bind(version.created_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn delete_system_agent_profile_internal(&self, id: &str) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;

        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM system_agent_bindings WHERE profile_id = $1")
                    .bind(id)
                    .execute(pool)
                    .await?;
                sqlx::query("DELETE FROM system_agent_profiles WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query("DELETE FROM system_agent_bindings WHERE profile_id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
                sqlx::query("DELETE FROM system_agent_profiles WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query("DELETE FROM system_agent_bindings WHERE profile_id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
                sqlx::query("DELETE FROM system_agent_profiles WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn list_system_agent_bindings_internal(
        &self,
        profile_id: Option<&str>,
    ) -> Result<Vec<SystemAgentBindingRecord>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let bindings = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                if let Some(profile_id) = profile_id {
                    sqlx::query_as::<_, SystemAgentBindingRecord>(
                        "SELECT * FROM system_agent_bindings WHERE profile_id = $1 ORDER BY priority DESC, created_at ASC",
                    )
                    .bind(profile_id)
                    .fetch_all(pool)
                    .await?
                } else {
                    sqlx::query_as::<_, SystemAgentBindingRecord>(
                        "SELECT * FROM system_agent_bindings ORDER BY priority DESC, created_at ASC",
                    )
                    .fetch_all(pool)
                    .await?
                }
            }
            DatabasePool::SQLite(pool) => {
                if let Some(profile_id) = profile_id {
                    sqlx::query_as::<_, SystemAgentBindingRecord>(
                        "SELECT * FROM system_agent_bindings WHERE profile_id = ? ORDER BY priority DESC, created_at ASC",
                    )
                    .bind(profile_id)
                    .fetch_all(pool)
                    .await?
                } else {
                    sqlx::query_as::<_, SystemAgentBindingRecord>(
                        "SELECT * FROM system_agent_bindings ORDER BY priority DESC, created_at ASC",
                    )
                    .fetch_all(pool)
                    .await?
                }
            }
            DatabasePool::MySQL(pool) => {
                if let Some(profile_id) = profile_id {
                    sqlx::query_as::<_, SystemAgentBindingRecord>(
                        "SELECT * FROM system_agent_bindings WHERE profile_id = ? ORDER BY priority DESC, created_at ASC",
                    )
                    .bind(profile_id)
                    .fetch_all(pool)
                    .await?
                } else {
                    sqlx::query_as::<_, SystemAgentBindingRecord>(
                        "SELECT * FROM system_agent_bindings ORDER BY priority DESC, created_at ASC",
                    )
                    .fetch_all(pool)
                    .await?
                }
            }
        };
        Ok(bindings)
    }

    pub async fn list_system_agent_runs_internal(
        &self,
        profile_id: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<SystemAgentRunRecord>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let limit = limit.unwrap_or(50) as i64;

        let runs = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                if let Some(profile_id) = profile_id {
                    sqlx::query_as::<_, SystemAgentRunRecord>(
                        "SELECT * FROM system_agent_runs WHERE profile_id = $1 ORDER BY started_at DESC LIMIT $2",
                    )
                    .bind(profile_id)
                    .bind(limit)
                    .fetch_all(pool)
                    .await?
                } else {
                    sqlx::query_as::<_, SystemAgentRunRecord>(
                        "SELECT * FROM system_agent_runs ORDER BY started_at DESC LIMIT $1",
                    )
                    .bind(limit)
                    .fetch_all(pool)
                    .await?
                }
            }
            DatabasePool::SQLite(pool) => {
                if let Some(profile_id) = profile_id {
                    sqlx::query_as::<_, SystemAgentRunRecord>(
                        "SELECT * FROM system_agent_runs WHERE profile_id = ? ORDER BY started_at DESC LIMIT ?",
                    )
                    .bind(profile_id)
                    .bind(limit)
                    .fetch_all(pool)
                    .await?
                } else {
                    sqlx::query_as::<_, SystemAgentRunRecord>(
                        "SELECT * FROM system_agent_runs ORDER BY started_at DESC LIMIT ?",
                    )
                    .bind(limit)
                    .fetch_all(pool)
                    .await?
                }
            }
            DatabasePool::MySQL(pool) => {
                if let Some(profile_id) = profile_id {
                    sqlx::query_as::<_, SystemAgentRunRecord>(
                        "SELECT * FROM system_agent_runs WHERE profile_id = ? ORDER BY started_at DESC LIMIT ?",
                    )
                    .bind(profile_id)
                    .bind(limit)
                    .fetch_all(pool)
                    .await?
                } else {
                    sqlx::query_as::<_, SystemAgentRunRecord>(
                        "SELECT * FROM system_agent_runs ORDER BY started_at DESC LIMIT ?",
                    )
                    .bind(limit)
                    .fetch_all(pool)
                    .await?
                }
            }
        };
        Ok(runs)
    }

    pub async fn delete_system_agent_run_internal(&self, id: &str) -> Result<u64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let affected_rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query("DELETE FROM system_agent_runs WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
            DatabasePool::SQLite(pool) => sqlx::query("DELETE FROM system_agent_runs WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?
                .rows_affected(),
            DatabasePool::MySQL(pool) => sqlx::query("DELETE FROM system_agent_runs WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?
                .rows_affected(),
        };

        Ok(affected_rows)
    }

    pub async fn clear_system_agent_runs_internal(&self, profile_id: Option<&str>) -> Result<u64> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let affected_rows = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                if let Some(profile_id) = profile_id {
                    sqlx::query("DELETE FROM system_agent_runs WHERE profile_id = $1")
                        .bind(profile_id)
                        .execute(pool)
                        .await?
                        .rows_affected()
                } else {
                    sqlx::query("DELETE FROM system_agent_runs")
                        .execute(pool)
                        .await?
                        .rows_affected()
                }
            }
            DatabasePool::SQLite(pool) => {
                if let Some(profile_id) = profile_id {
                    sqlx::query("DELETE FROM system_agent_runs WHERE profile_id = ?")
                        .bind(profile_id)
                        .execute(pool)
                        .await?
                        .rows_affected()
                } else {
                    sqlx::query("DELETE FROM system_agent_runs")
                        .execute(pool)
                        .await?
                        .rows_affected()
                }
            }
            DatabasePool::MySQL(pool) => {
                if let Some(profile_id) = profile_id {
                    sqlx::query("DELETE FROM system_agent_runs WHERE profile_id = ?")
                        .bind(profile_id)
                        .execute(pool)
                        .await?
                        .rows_affected()
                } else {
                    sqlx::query("DELETE FROM system_agent_runs")
                        .execute(pool)
                        .await?
                        .rows_affected()
                }
            }
        };

        Ok(affected_rows)
    }

    pub async fn get_system_agent_run_internal(
        &self,
        id: &str,
    ) -> Result<Option<SystemAgentRunRecord>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let run = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, SystemAgentRunRecord>(
                    "SELECT * FROM system_agent_runs WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SystemAgentRunRecord>(
                    "SELECT * FROM system_agent_runs WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, SystemAgentRunRecord>(
                    "SELECT * FROM system_agent_runs WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?
            }
        };
        Ok(run)
    }

    pub async fn list_incomplete_system_agent_runs_internal(
        &self,
        limit: Option<u32>,
    ) -> Result<Vec<SystemAgentRunRecord>> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        let limit = limit.unwrap_or(200) as i64;

        let runs = match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query_as::<_, SystemAgentRunRecord>(
                    "SELECT * FROM system_agent_runs WHERE status IN ('queued', 'retrying', 'running') ORDER BY created_at ASC LIMIT $1",
                )
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query_as::<_, SystemAgentRunRecord>(
                    "SELECT * FROM system_agent_runs WHERE status IN ('queued', 'retrying', 'running') ORDER BY created_at ASC LIMIT ?",
                )
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query_as::<_, SystemAgentRunRecord>(
                    "SELECT * FROM system_agent_runs WHERE status IN ('queued', 'retrying', 'running') ORDER BY created_at ASC LIMIT ?",
                )
                .bind(limit)
                .fetch_all(pool)
                .await?
            }
        };
        Ok(runs)
    }

    pub async fn create_system_agent_run_internal(&self, run: &SystemAgentRunRecord) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO system_agent_runs (
                        id, profile_id, trigger_event, status, input_summary_json,
                        tool_calls, output_json, error_message, started_at, finished_at, created_at, updated_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                    "#,
                )
                .bind(&run.id)
                .bind(&run.profile_id)
                .bind(&run.trigger_event)
                .bind(&run.status)
                .bind(&run.input_summary_json)
                .bind(&run.tool_calls)
                .bind(&run.output_json)
                .bind(&run.error_message)
                .bind(run.started_at)
                .bind(run.finished_at)
                .bind(run.created_at)
                .bind(run.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO system_agent_runs (
                        id, profile_id, trigger_event, status, input_summary_json,
                        tool_calls, output_json, error_message, started_at, finished_at, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&run.id)
                .bind(&run.profile_id)
                .bind(&run.trigger_event)
                .bind(&run.status)
                .bind(&run.input_summary_json)
                .bind(&run.tool_calls)
                .bind(&run.output_json)
                .bind(&run.error_message)
                .bind(run.started_at)
                .bind(run.finished_at)
                .bind(run.created_at)
                .bind(run.updated_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO system_agent_runs (
                        id, profile_id, trigger_event, status, input_summary_json,
                        tool_calls, output_json, error_message, started_at, finished_at, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(&run.id)
                .bind(&run.profile_id)
                .bind(&run.trigger_event)
                .bind(&run.status)
                .bind(&run.input_summary_json)
                .bind(&run.tool_calls)
                .bind(&run.output_json)
                .bind(&run.error_message)
                .bind(run.started_at)
                .bind(run.finished_at)
                .bind(run.created_at)
                .bind(run.updated_at)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn update_system_agent_run_internal(
        &self,
        id: &str,
        status: &str,
        tool_calls: Option<&str>,
        output_json: Option<&str>,
        error_message: Option<&str>,
        finished_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    UPDATE system_agent_runs
                    SET status = $1,
                        tool_calls = $2,
                        output_json = $3,
                        error_message = $4,
                        finished_at = $5,
                        updated_at = CURRENT_TIMESTAMP
                    WHERE id = $6
                    "#,
                )
                .bind(status)
                .bind(tool_calls)
                .bind(output_json)
                .bind(error_message)
                .bind(finished_at)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    UPDATE system_agent_runs
                    SET status = ?,
                        tool_calls = ?,
                        output_json = ?,
                        error_message = ?,
                        finished_at = ?,
                        updated_at = CURRENT_TIMESTAMP
                    WHERE id = ?
                    "#,
                )
                .bind(status)
                .bind(tool_calls)
                .bind(output_json)
                .bind(error_message)
                .bind(finished_at)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    UPDATE system_agent_runs
                    SET status = ?,
                        tool_calls = ?,
                        output_json = ?,
                        error_message = ?,
                        finished_at = ?,
                        updated_at = CURRENT_TIMESTAMP
                    WHERE id = ?
                    "#,
                )
                .bind(status)
                .bind(tool_calls)
                .bind(output_json)
                .bind(error_message)
                .bind(finished_at)
                .bind(id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn set_system_agent_run_state_internal(
        &self,
        id: &str,
        status: &str,
        output_json: Option<&str>,
        error_message: Option<&str>,
        started_at: Option<DateTime<Utc>>,
        finished_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let runtime = self
            .runtime_pool
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("数据库未初始化"))?;
        match runtime {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"
                    UPDATE system_agent_runs
                    SET status = $1,
                        output_json = $2,
                        error_message = $3,
                        started_at = COALESCE($4, started_at),
                        finished_at = $5,
                        updated_at = CURRENT_TIMESTAMP
                    WHERE id = $6
                    "#,
                )
                .bind(status)
                .bind(output_json)
                .bind(error_message)
                .bind(started_at)
                .bind(finished_at)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"
                    UPDATE system_agent_runs
                    SET status = ?,
                        output_json = ?,
                        error_message = ?,
                        started_at = COALESCE(?, started_at),
                        finished_at = ?,
                        updated_at = CURRENT_TIMESTAMP
                    WHERE id = ?
                    "#,
                )
                .bind(status)
                .bind(output_json)
                .bind(error_message)
                .bind(started_at)
                .bind(finished_at)
                .bind(id)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(
                    r#"
                    UPDATE system_agent_runs
                    SET status = ?,
                        output_json = ?,
                        error_message = ?,
                        started_at = COALESCE(?, started_at),
                        finished_at = ?,
                        updated_at = CURRENT_TIMESTAMP
                    WHERE id = ?
                    "#,
                )
                .bind(status)
                .bind(output_json)
                .bind(error_message)
                .bind(started_at)
                .bind(finished_at)
                .bind(id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }
}

//! Agent Team 数据库操作层（SQLite 兼容）

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde_json::json;
use sqlx::Row;
use sqlx::SqlitePool;
use tracing::info;
use uuid::Uuid;

use super::models::*;
use super::repository::{
    build_template_spec_v2, build_template_spec_v2_from_legacy, builtin_templates_seed,
    make_member_requests_from_agents, AGENT_TEAM_SCHEMA_V2,
};

pub async fn create_agent_team_template(
    pool: &SqlitePool,
    req: &CreateAgentTeamTemplateRequest,
    created_by: Option<&str>,
) -> Result<AgentTeamTemplate> {
    ensure_schema(pool).await?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let spec_v2 = build_template_spec_v2(req)?;
    let members_from_agents = make_member_requests_from_agents(&spec_v2.agents);

    sqlx::query(
        r#"INSERT INTO agent_team_templates
           (id, name, description, domain, default_rounds_config, default_tool_policy,
            schema_version, template_spec_v2, upgrade_failed, upgrade_error,
            is_system, created_by, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.domain)
    .bind(req.default_rounds_config.as_ref().map(|v| v.to_string()))
    .bind(req.default_tool_policy.as_ref().map(|v| v.to_string()))
    .bind(AGENT_TEAM_SCHEMA_V2)
    .bind(serde_json::to_string(&spec_v2).ok())
    .bind(false)
    .bind(Option::<String>::None)
    .bind(false)
    .bind(created_by)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    for (i, member_req) in members_from_agents.iter().enumerate() {
        let member_id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"INSERT INTO agent_team_template_members
               (id, template_id, name, responsibility, system_prompt, decision_style,
                risk_preference, weight, tool_policy, output_schema, sort_order, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&member_id)
        .bind(&id)
        .bind(&member_req.name)
        .bind(&member_req.responsibility)
        .bind(&member_req.system_prompt)
        .bind(&member_req.decision_style)
        .bind(&member_req.risk_preference)
        .bind(member_req.weight.unwrap_or(1.0))
        .bind(member_req.tool_policy.as_ref().map(|v| v.to_string()))
        .bind(member_req.output_schema.as_ref().map(|v| v.to_string()))
        .bind(member_req.sort_order.unwrap_or(i as i32))
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
    }

    get_agent_team_template_detail(pool, &id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Template not found after creation"))
}

pub async fn update_agent_team_template(
    pool: &SqlitePool,
    id: &str,
    req: &UpdateAgentTeamTemplateRequest,
) -> Result<()> {
    ensure_schema(pool).await?;

    let now = Utc::now();
    let schema_version = req
        .schema_version
        .unwrap_or(AGENT_TEAM_SCHEMA_V2)
        .max(AGENT_TEAM_SCHEMA_V2);
    let template_spec_v2 = if req.agents.is_some() || req.task_graph.is_some() || req.hook_policy.is_some() {
        let existing_row = sqlx::query("SELECT template_spec_v2 FROM agent_team_templates WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        let existing_row =
            existing_row.ok_or_else(|| anyhow!("template not found: {}", id))?;
        let mut merged_spec = existing_row
            .try_get::<Option<String>, _>("template_spec_v2")?
            .and_then(|s| serde_json::from_str::<TeamTemplateSpecV2>(&s).ok())
            .unwrap_or(TeamTemplateSpecV2 {
                schema_version: AGENT_TEAM_SCHEMA_V2,
                agents: vec![],
                task_graph: TeamTaskGraph {
                    version: Some(1),
                    nodes: vec![],
                },
                hook_policy: None,
            });
        if let Some(agents) = req.agents.clone() {
            merged_spec.agents = agents;
        }
        if let Some(task_graph) = req.task_graph.clone() {
            merged_spec.task_graph = task_graph;
        }
        if req.hook_policy.is_some() {
            merged_spec.hook_policy = req.hook_policy.clone();
        }
        merged_spec.schema_version = AGENT_TEAM_SCHEMA_V2;
        if merged_spec.agents.is_empty() {
            return Err(anyhow!("template agents cannot be empty in schema v2"));
        }
        if merged_spec.task_graph.nodes.is_empty() {
            return Err(anyhow!(
                "template task_graph.nodes cannot be empty in schema v2"
            ));
        }
        Some(json!(merged_spec))
    } else {
        None
    };
    sqlx::query(
        r#"UPDATE agent_team_templates
           SET name = COALESCE(?, name),
               description = COALESCE(?, description),
               domain = COALESCE(?, domain),
               default_rounds_config = COALESCE(?, default_rounds_config),
               default_tool_policy = COALESCE(?, default_tool_policy),
               schema_version = COALESCE(?, schema_version),
               template_spec_v2 = COALESCE(?, template_spec_v2),
               upgrade_failed = FALSE,
               upgrade_error = NULL,
               updated_at = ?
           WHERE id = ?"#,
    )
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.domain)
    .bind(req.default_rounds_config.as_ref().map(|v| v.to_string()))
    .bind(req.default_tool_policy.as_ref().map(|v| v.to_string()))
    .bind(Some(schema_version))
    .bind(template_spec_v2.as_ref().map(|v| v.to_string()))
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    // 若传入 agents，则同步替换模板 Agent 快照
    let members_to_use = req
        .agents
        .as_ref()
        .map(|agents| make_member_requests_from_agents(agents));
    if let Some(members) = members_to_use {
        sqlx::query("DELETE FROM agent_team_template_members WHERE template_id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        for (i, member_req) in members.into_iter().enumerate() {
            let member_id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                r#"INSERT INTO agent_team_template_members
                (id, template_id, name, responsibility, system_prompt, decision_style,
                 risk_preference, weight, tool_policy, output_schema, sort_order, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(member_id)
            .bind(id)
            .bind(&member_req.name)
            .bind(&member_req.responsibility)
            .bind(&member_req.system_prompt)
            .bind(&member_req.decision_style)
            .bind(&member_req.risk_preference)
            .bind(member_req.weight.unwrap_or(1.0))
            .bind(member_req.tool_policy.as_ref().map(|v| v.to_string()))
            .bind(member_req.output_schema.as_ref().map(|v| v.to_string()))
            .bind(member_req.sort_order.unwrap_or(i as i32))
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

pub async fn delete_agent_team_template(pool: &SqlitePool, id: &str) -> Result<()> {
    ensure_schema(pool).await?;

    sqlx::query("DELETE FROM agent_team_template_members WHERE template_id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM agent_team_templates WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_agent_team_templates(
    pool: &SqlitePool,
    domain: Option<&str>,
) -> Result<Vec<AgentTeamTemplate>> {
    ensure_schema(pool).await?;

    let rows = if let Some(domain) = domain {
        sqlx::query(
            r#"SELECT id, name, description, domain, default_rounds_config,
                      default_tool_policy, schema_version, template_spec_v2,
                      upgrade_failed, upgrade_error, is_system, created_by, created_at, updated_at
               FROM agent_team_templates WHERE domain = ? ORDER BY created_at DESC"#,
        )
        .bind(domain)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT id, name, description, domain, default_rounds_config,
                      default_tool_policy, schema_version, template_spec_v2,
                      upgrade_failed, upgrade_error, is_system, created_by, created_at, updated_at
               FROM agent_team_templates ORDER BY created_at DESC"#,
        )
        .fetch_all(pool)
        .await?
    };

    let mut templates: Vec<AgentTeamTemplate> = rows
        .into_iter()
        .map(|row| {
            Ok(AgentTeamTemplate {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                domain: row.try_get("domain")?,
                default_rounds_config: row
                    .try_get::<Option<String>, _>("default_rounds_config")?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                default_tool_policy: row
                    .try_get::<Option<String>, _>("default_tool_policy")?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                schema_version: row.try_get::<i32, _>("schema_version").unwrap_or(1),
                template_spec_v2: row
                    .try_get::<Option<String>, _>("template_spec_v2")?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                upgrade_failed: row.try_get::<bool, _>("upgrade_failed").unwrap_or(false),
                upgrade_error: row.try_get("upgrade_error")?,
                is_system: row.try_get("is_system")?,
                created_by: row.try_get("created_by")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                members: vec![],
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // 为前端列表补充 Agent 快照信息（避免计数显示为 0）
    for template in &mut templates {
        let members = get_template_members(pool, &template.id).await?;
        template.members = members;
    }

    Ok(templates)
}

pub async fn get_agent_team_template_detail(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<AgentTeamTemplate>> {
    ensure_schema(pool).await?;

    let row_opt = sqlx::query(
        r#"SELECT id, name, description, domain, default_rounds_config,
                  default_tool_policy, schema_version, template_spec_v2,
                  upgrade_failed, upgrade_error, is_system, created_by, created_at, updated_at
           FROM agent_team_templates WHERE id = ?"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row_opt else {
        return Ok(None);
    };

    let mut template = AgentTeamTemplate {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        domain: row.try_get("domain")?,
        default_rounds_config: row
            .try_get::<Option<String>, _>("default_rounds_config")?
            .and_then(|s| serde_json::from_str(&s).ok()),
        default_tool_policy: row
            .try_get::<Option<String>, _>("default_tool_policy")?
            .and_then(|s| serde_json::from_str(&s).ok()),
        schema_version: row.try_get::<i32, _>("schema_version").unwrap_or(1),
        template_spec_v2: row
            .try_get::<Option<String>, _>("template_spec_v2")?
            .and_then(|s| serde_json::from_str(&s).ok()),
        upgrade_failed: row.try_get::<bool, _>("upgrade_failed").unwrap_or(false),
        upgrade_error: row.try_get("upgrade_error")?,
        is_system: row.try_get("is_system")?,
        created_by: row.try_get("created_by")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        members: vec![],
    };

    template.members = get_template_members(pool, id).await?;
    Ok(Some(template))
}

pub async fn seed_builtin_templates(pool: &SqlitePool) -> Result<()> {
    ensure_schema(pool).await?;

    let count_row =
        sqlx::query("SELECT COUNT(*) as cnt FROM agent_team_templates WHERE is_system = true")
            .fetch_one(pool)
            .await?;
    let count: i64 = count_row.try_get::<i64, _>("cnt").unwrap_or(0);

    if count > 0 {
        info!("Built-in agent team templates already exist, skipping seed");
        return Ok(());
    }

    info!("Seeding built-in agent team templates for SQLite...");

    for template_req in builtin_templates_seed() {
        let id = template_req.id.to_string();
        let now = Utc::now();
        let legacy_req = CreateAgentTeamTemplateRequest {
            name: template_req.name.to_string(),
            description: Some(template_req.description.to_string()),
            domain: template_req.domain.to_string(),
            default_rounds_config: None,
            default_tool_policy: None,
            schema_version: Some(AGENT_TEAM_SCHEMA_V2),
            agents: super::repository::build_agents_from_members(&template_req.members),
            task_graph: super::repository::convert_legacy_to_task_graph(None, &template_req.members),
            hook_policy: None,
        };
        let spec_v2 = build_template_spec_v2(&legacy_req).ok();

        sqlx::query(
            r#"INSERT OR IGNORE INTO agent_team_templates
               (id, name, description, domain, schema_version, template_spec_v2, upgrade_failed, upgrade_error, is_system, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(template_req.name)
        .bind(template_req.description)
        .bind(template_req.domain)
        .bind(AGENT_TEAM_SCHEMA_V2)
        .bind(spec_v2.as_ref().and_then(|v| serde_json::to_string(v).ok()))
        .bind(false)
        .bind(Option::<String>::None)
        .bind(true)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        for (i, member) in template_req.members.iter().enumerate() {
            let member_id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"INSERT INTO agent_team_template_members
                   (id, template_id, name, responsibility, system_prompt, decision_style,
                    risk_preference, weight, sort_order, created_at, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&member_id)
            .bind(&id)
            .bind(&member.name)
            .bind(&member.responsibility)
            .bind(&member.system_prompt)
            .bind(&member.decision_style)
            .bind(&member.risk_preference)
            .bind(member.weight.unwrap_or(1.0))
            .bind(member.sort_order.unwrap_or(i as i32))
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

// ==================== 会话操作 ====================

pub async fn create_agent_team_session(
    pool: &SqlitePool,
    req: &CreateAgentTeamSessionRequest,
) -> Result<AgentTeamSession> {
    ensure_schema(pool).await?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let mut runtime_spec_v2 = req.runtime_spec_v2.clone();
    let schema_version = req.schema_version.unwrap_or(AGENT_TEAM_SCHEMA_V2);

    if let Some(template_id) = req.template_id.as_ref() {
        let tpl_row = sqlx::query(
            r#"SELECT schema_version, template_spec_v2, upgrade_failed, upgrade_error
               FROM agent_team_templates WHERE id = ?"#,
        )
        .bind(template_id)
        .fetch_optional(pool)
        .await?;
        let Some(tpl) = tpl_row else {
            return Err(anyhow!("Template not found: {}", template_id));
        };
        let tpl_schema_version = tpl.try_get::<i32, _>("schema_version").unwrap_or(1);
        let upgrade_failed = tpl.try_get::<bool, _>("upgrade_failed").unwrap_or(false);
        let upgrade_error: Option<String> = tpl.try_get("upgrade_error").ok().flatten();
        if upgrade_failed {
            return Err(anyhow!(
                "Template upgrade failed, cannot create session: {}",
                upgrade_error.unwrap_or_else(|| "unknown reason".to_string())
            ));
        }
        if tpl_schema_version < AGENT_TEAM_SCHEMA_V2 {
            return Err(anyhow!(
                "Template schema_version={} is not supported. Please upgrade templates to v2 first.",
                tpl_schema_version
            ));
        }
        if runtime_spec_v2.is_none() {
            runtime_spec_v2 = tpl
                .try_get::<Option<String>, _>("template_spec_v2")?
                .and_then(|s| serde_json::from_str(&s).ok());
        }
    }
    if runtime_spec_v2.is_none() {
        return Err(anyhow!(
            "runtime_spec_v2 is required for Agent Teams V2 sessions"
        ));
    }

    sqlx::query(
        r#"INSERT INTO agent_team_sessions
           (id, conversation_id, template_id, name, goal, orchestration_plan, schema_version, runtime_spec_v2, plan_version,
            state, state_machine, current_round, max_rounds, total_tokens, estimated_cost, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&req.conversation_id)
    .bind(&req.template_id)
    .bind(&req.name)
    .bind(&req.goal)
    .bind(Option::<String>::None)
    .bind(schema_version.max(AGENT_TEAM_SCHEMA_V2))
    .bind(runtime_spec_v2.as_ref().map(|v| v.to_string()))
    .bind(1i32)
    .bind(TeamSessionState::Pending.to_string())
    .bind(req.state_machine.as_ref().map(|v| v.to_string()))
    .bind(0i32)
    .bind(req.max_rounds.unwrap_or(5))
    .bind(0i64)
    .bind(0.0f64)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    if let Some(ref template_id) = req.template_id {
        snapshot_members_from_template(pool, &id, template_id).await?;
    } else if let Some(runtime_spec) = runtime_spec_v2.as_ref() {
        snapshot_members_from_runtime_spec(pool, &id, runtime_spec).await?;
    }

    if let Some(runtime_spec) = runtime_spec_v2.as_ref() {
        ensure_session_tasks_from_runtime_spec(pool, &id, runtime_spec).await?;
    }

    get_agent_team_session(pool, &id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session not found after creation"))
}

async fn snapshot_members_from_runtime_spec(
    pool: &SqlitePool,
    session_id: &str,
    runtime_spec_v2: &serde_json::Value,
) -> Result<()> {
    let spec: TeamTemplateSpecV2 = serde_json::from_value(runtime_spec_v2.clone())
        .map_err(|e| anyhow!("invalid runtime_spec_v2 when snapshotting session agents: {e}"))?;
    if spec.agents.is_empty() {
        return Err(anyhow!("runtime_spec_v2.agents cannot be empty"));
    }
    for (i, agent) in spec.agents.iter().enumerate() {
        create_agent_team_member_from_profile(pool, session_id, agent, i as i32).await?;
    }
    Ok(())
}

fn build_member_output_schema_from_model(model: Option<&str>) -> Option<serde_json::Value> {
    let model = model?.trim();
    if model.is_empty() {
        return None;
    }
    let (provider, model_name) = if let Some((p, m)) = model.split_once('/') {
        (p.trim().to_lowercase(), m.trim().to_string())
    } else {
        ("".to_string(), model.to_string())
    };
    Some(json!({
        "llm_model": model,
        "model_provider": provider,
        "model_name": model_name,
    }))
}

async fn create_agent_team_member_from_profile(
    pool: &SqlitePool,
    session_id: &str,
    agent: &AgentProfile,
    sort_order: i32,
) -> Result<()> {
    let member_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO agent_team_members
           (id, session_id, name, responsibility, system_prompt, decision_style,
            risk_preference, weight, tool_policy, output_schema, sort_order,
            token_usage, tool_calls_count, is_active, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&member_id)
    .bind(session_id)
    .bind(&agent.name)
    .bind(Option::<String>::None)
    .bind(&agent.system_prompt)
    .bind(Some("balanced".to_string()))
    .bind(Some("medium".to_string()))
    .bind(1.0f64)
    .bind(agent.tool_policy.as_ref().map(|v| v.to_string()))
    .bind(
        build_member_output_schema_from_model(agent.model.as_deref())
            .as_ref()
            .map(|v| v.to_string()),
    )
    .bind(sort_order)
    .bind(0i64)
    .bind(0i32)
    .bind(true)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_agent_team_session(
    pool: &SqlitePool,
    id: &str,
    req: &UpdateAgentTeamSessionRequest,
) -> Result<()> {
    ensure_schema(pool).await?;

    let now = Utc::now();
    let schema_version = req.schema_version.map(|v| v.max(AGENT_TEAM_SCHEMA_V2));
    sqlx::query(
        r#"UPDATE agent_team_sessions
           SET name = COALESCE(?, name),
               goal = COALESCE(?, goal),
               schema_version = COALESCE(?, schema_version),
               runtime_spec_v2 = COALESCE(?, runtime_spec_v2),
               state = COALESCE(?, state),
               max_rounds = COALESCE(?, max_rounds),
               state_machine = COALESCE(?, state_machine),
               error_message = COALESCE(?, error_message),
               updated_at = ?
           WHERE id = ?"#,
    )
    .bind(&req.name)
    .bind(&req.goal)
    .bind(schema_version)
    .bind(req.runtime_spec_v2.as_ref().map(|v| v.to_string()))
    .bind(&req.state)
    .bind(req.max_rounds)
    .bind(req.state_machine.as_ref().map(|v| v.to_string()))
    .bind(&req.error_message)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_agent_team_session(pool: &SqlitePool, id: &str) -> Result<()> {
    ensure_schema(pool).await?;

    // Child tables are configured with ON DELETE CASCADE, so deleting session is sufficient.
    sqlx::query("DELETE FROM agent_team_sessions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_session_state(pool: &SqlitePool, session_id: &str, state: &str) -> Result<()> {
    ensure_schema(pool).await?;

    let now = Utc::now();
    sqlx::query("UPDATE agent_team_sessions SET state = ?, updated_at = ? WHERE id = ?")
        .bind(state)
        .bind(now)
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_agent_team_session(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<AgentTeamSession>> {
    ensure_schema(pool).await?;

    let row_opt = sqlx::query(
        r#"SELECT id, conversation_id, template_id, name, goal, orchestration_plan, schema_version, runtime_spec_v2, plan_version, state, state_machine,
                  current_round, max_rounds, blackboard_state, divergence_scores,
                  total_tokens, estimated_cost, suspended_reason, started_at, completed_at,
                  error_message, created_at, updated_at
           FROM agent_team_sessions WHERE id = ?"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row_opt else {
        return Ok(None);
    };

    let mut session = AgentTeamSession {
        id: row.try_get("id")?,
        conversation_id: row.try_get("conversation_id")?,
        template_id: row.try_get("template_id")?,
        name: row.try_get("name")?,
        goal: row.try_get("goal")?,
        orchestration_plan: row
            .try_get::<Option<String>, _>("orchestration_plan")?
            .and_then(|s| serde_json::from_str(&s).ok()),
        schema_version: row.try_get::<i32, _>("schema_version").unwrap_or(1),
        runtime_spec_v2: row
            .try_get::<Option<String>, _>("runtime_spec_v2")?
            .and_then(|s| serde_json::from_str(&s).ok()),
        plan_version: row.try_get::<i32, _>("plan_version").unwrap_or(1),
        state: row.try_get("state")?,
        state_machine: row
            .try_get::<Option<String>, _>("state_machine")?
            .and_then(|s| serde_json::from_str(&s).ok()),
        current_round: row.try_get::<i32, _>("current_round").unwrap_or(0),
        max_rounds: row.try_get::<i32, _>("max_rounds").unwrap_or(5),
        blackboard_state: row
            .try_get::<Option<String>, _>("blackboard_state")?
            .and_then(|s| serde_json::from_str(&s).ok()),
        divergence_scores: row
            .try_get::<Option<String>, _>("divergence_scores")?
            .and_then(|s| serde_json::from_str(&s).ok()),
        total_tokens: row.try_get::<i64, _>("total_tokens").unwrap_or(0),
        estimated_cost: row.try_get::<f64, _>("estimated_cost").unwrap_or(0.0),
        suspended_reason: row.try_get("suspended_reason")?,
        started_at: row.try_get("started_at")?,
        completed_at: row.try_get("completed_at")?,
        error_message: row.try_get("error_message")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        members: vec![],
    };

    session.members = get_agent_team_members(pool, id).await?;
    Ok(Some(session))
}

pub async fn list_agent_team_sessions(
    pool: &SqlitePool,
    conversation_id: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<AgentTeamSession>> {
    ensure_schema(pool).await?;

    let rows = if let Some(conv_id) = conversation_id {
        sqlx::query(
            r#"SELECT id
               FROM agent_team_sessions
               WHERE conversation_id = ?
               ORDER BY updated_at DESC
               LIMIT ? OFFSET ?"#,
        )
        .bind(conv_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT id
               FROM agent_team_sessions
               ORDER BY updated_at DESC
               LIMIT ? OFFSET ?"#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };

    let mut sessions = Vec::with_capacity(rows.len());
    for row in rows {
        let id: String = row.try_get("id")?;
        if let Some(session) = get_agent_team_session(pool, &id).await? {
            sessions.push(session);
        }
    }
    Ok(sessions)
}

pub async fn get_agent_team_members(
    pool: &SqlitePool,
    session_id: &str,
) -> Result<Vec<AgentTeamMember>> {
    ensure_schema(pool).await?;

    let rows = sqlx::query(
        r#"SELECT id, session_id, name, responsibility, system_prompt, decision_style,
                  risk_preference, weight, tool_policy, output_schema, sort_order,
                  token_usage, tool_calls_count, is_active, created_at, updated_at
           FROM agent_team_members WHERE session_id = ? AND is_active = true ORDER BY sort_order"#,
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(AgentTeamMember {
                id: row.try_get("id")?,
                session_id: row.try_get("session_id")?,
                name: row.try_get("name")?,
                responsibility: row.try_get("responsibility")?,
                system_prompt: row.try_get("system_prompt")?,
                decision_style: row.try_get("decision_style")?,
                risk_preference: row.try_get("risk_preference")?,
                weight: row.try_get::<f64, _>("weight").unwrap_or(1.0),
                tool_policy: row
                    .try_get::<Option<String>, _>("tool_policy")?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                output_schema: row
                    .try_get::<Option<String>, _>("output_schema")?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                sort_order: row.try_get::<i32, _>("sort_order").unwrap_or(0),
                token_usage: row.try_get::<i64, _>("token_usage").unwrap_or(0),
                tool_calls_count: row.try_get::<i32, _>("tool_calls_count").unwrap_or(0),
                is_active: row.try_get::<bool, _>("is_active").unwrap_or(true),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        })
        .collect()
}

// ==================== 轮次操作 ====================

pub async fn create_round(
    pool: &SqlitePool,
    session_id: &str,
    round_number: i32,
    phase: &str,
) -> Result<AgentTeamRound> {
    ensure_schema(pool).await?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        r#"INSERT INTO agent_team_rounds (id, session_id, round_number, phase, status, started_at, created_at)
           VALUES (?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(session_id)
    .bind(round_number)
    .bind(phase)
    .bind("running")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    sqlx::query("UPDATE agent_team_sessions SET current_round = ?, updated_at = ? WHERE id = ?")
        .bind(round_number)
        .bind(now)
        .bind(session_id)
        .execute(pool)
        .await?;

    Ok(AgentTeamRound {
        id,
        session_id: session_id.to_string(),
        round_number,
        phase: phase.to_string(),
        status: "running".to_string(),
        divergence_score: None,
        started_at: Some(now),
        completed_at: None,
        created_at: now,
    })
}

pub async fn complete_round(
    pool: &SqlitePool,
    round_id: &str,
    divergence_score: Option<f64>,
) -> Result<()> {
    ensure_schema(pool).await?;

    let now = Utc::now();
    sqlx::query(
        "UPDATE agent_team_rounds SET status = 'completed', completed_at = ?, divergence_score = ? WHERE id = ?",
    )
    .bind(now)
    .bind(divergence_score)
    .bind(round_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_rounds(pool: &SqlitePool, session_id: &str) -> Result<Vec<AgentTeamRound>> {
    ensure_schema(pool).await?;

    let rows = sqlx::query(
        r#"SELECT id, session_id, round_number, phase, status, divergence_score,
                  started_at, completed_at, created_at
           FROM agent_team_rounds
           WHERE session_id = ?
           ORDER BY created_at ASC"#,
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(AgentTeamRound {
                id: row.try_get("id")?,
                session_id: row.try_get("session_id")?,
                round_number: row.try_get::<i32, _>("round_number").unwrap_or(0),
                phase: row.try_get("phase")?,
                status: row.try_get("status")?,
                divergence_score: row.try_get("divergence_score")?,
                started_at: row.try_get("started_at")?,
                completed_at: row.try_get("completed_at")?,
                created_at: row.try_get("created_at")?,
            })
        })
        .collect()
}

// ==================== 消息操作 ====================

pub async fn create_message(
    pool: &SqlitePool,
    session_id: &str,
    round_id: Option<&str>,
    member_id: Option<&str>,
    member_name: Option<&str>,
    role: &str,
    content: &str,
    token_count: Option<i32>,
) -> Result<AgentTeamMessage> {
    ensure_schema(pool).await?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        r#"INSERT INTO agent_team_messages
           (id, session_id, round_id, member_id, member_name, role, content, token_count, timestamp)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(session_id)
    .bind(round_id)
    .bind(member_id)
    .bind(member_name)
    .bind(role)
    .bind(content)
    .bind(token_count)
    .bind(now)
    .execute(pool)
    .await?;

    if let (Some(mid), Some(tc)) = (member_id, token_count) {
        sqlx::query(
            "UPDATE agent_team_members SET token_usage = token_usage + ?, updated_at = ? WHERE id = ?",
        )
        .bind(tc as i64)
        .bind(now)
        .bind(mid)
        .execute(pool)
        .await?;
    }

    Ok(AgentTeamMessage {
        id,
        session_id: session_id.to_string(),
        round_id: round_id.map(|s| s.to_string()),
        member_id: member_id.map(|s| s.to_string()),
        member_name: member_name.map(|s| s.to_string()),
        role: role.to_string(),
        content: content.to_string(),
        tool_calls: None,
        token_count,
        timestamp: now,
    })
}

pub async fn update_message_tool_calls(
    pool: &SqlitePool,
    message_id: &str,
    tool_calls: &serde_json::Value,
) -> Result<()> {
    ensure_schema(pool).await?;
    sqlx::query(
        r#"UPDATE agent_team_messages
           SET tool_calls = ?
           WHERE id = ?"#,
    )
    .bind(tool_calls.to_string())
    .bind(message_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_messages(pool: &SqlitePool, session_id: &str) -> Result<Vec<AgentTeamMessage>> {
    ensure_schema(pool).await?;

    let rows = sqlx::query(
        r#"SELECT id, session_id, round_id, member_id, member_name, role, content,
                  tool_calls, token_count, timestamp
           FROM agent_team_messages WHERE session_id = ? ORDER BY timestamp ASC"#,
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(AgentTeamMessage {
                id: row.try_get("id")?,
                session_id: row.try_get("session_id")?,
                round_id: row.try_get("round_id")?,
                member_id: row.try_get("member_id")?,
                member_name: row.try_get("member_name")?,
                role: row.try_get("role")?,
                content: row.try_get("content")?,
                tool_calls: row
                    .try_get::<Option<String>, _>("tool_calls")?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                token_count: row.try_get("token_count")?,
                timestamp: row.try_get("timestamp")?,
            })
        })
        .collect()
}

// ==================== 白板操作 ====================

pub async fn upsert_blackboard_entry(
    pool: &SqlitePool,
    req: &UpdateBlackboardRequest,
) -> Result<AgentTeamBlackboardEntry> {
    ensure_schema(pool).await?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        r#"INSERT INTO agent_team_blackboard_entries
           (id, session_id, round_id, entry_type, title, content, contributed_by, is_resolved, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&req.session_id)
    .bind(&req.round_id)
    .bind(&req.entry_type)
    .bind(&req.title)
    .bind(&req.content)
    .bind(&req.contributed_by)
    .bind(false)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(AgentTeamBlackboardEntry {
        id,
        session_id: req.session_id.clone(),
        round_id: req.round_id.clone(),
        entry_type: req.entry_type.clone(),
        title: req.title.clone(),
        content: req.content.clone(),
        contributed_by: req.contributed_by.clone(),
        is_resolved: false,
        created_at: now,
        updated_at: now,
    })
}

pub async fn get_blackboard_entries(
    pool: &SqlitePool,
    session_id: &str,
) -> Result<Vec<AgentTeamBlackboardEntry>> {
    ensure_schema(pool).await?;

    let rows = sqlx::query(
        r#"SELECT id, session_id, round_id, entry_type, title, content,
                  contributed_by, is_resolved, created_at, updated_at
           FROM agent_team_blackboard_entries WHERE session_id = ? ORDER BY created_at ASC"#,
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(AgentTeamBlackboardEntry {
                id: row.try_get("id")?,
                session_id: row.try_get("session_id")?,
                round_id: row.try_get("round_id")?,
                entry_type: row.try_get("entry_type")?,
                title: row.try_get("title")?,
                content: row.try_get("content")?,
                contributed_by: row.try_get("contributed_by")?,
                is_resolved: row.try_get::<bool, _>("is_resolved").unwrap_or(false),
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        })
        .collect()
}

pub async fn resolve_blackboard_entry(
    pool: &SqlitePool,
    session_id: &str,
    entry_id: &str,
) -> Result<AgentTeamBlackboardEntry> {
    ensure_schema(pool).await?;

    let now = Utc::now();
    let updated = sqlx::query(
        r#"UPDATE agent_team_blackboard_entries
           SET is_resolved = 1, updated_at = ?
           WHERE session_id = ? AND id = ?"#,
    )
    .bind(now)
    .bind(session_id)
    .bind(entry_id)
    .execute(pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(anyhow::anyhow!("Blackboard entry not found"));
    }

    let row = sqlx::query(
        r#"SELECT id, session_id, round_id, entry_type, title, content,
                  contributed_by, is_resolved, created_at, updated_at
           FROM agent_team_blackboard_entries
           WHERE session_id = ? AND id = ?"#,
    )
    .bind(session_id)
    .bind(entry_id)
    .fetch_one(pool)
    .await?;

    Ok(AgentTeamBlackboardEntry {
        id: row.try_get("id")?,
        session_id: row.try_get("session_id")?,
        round_id: row.try_get("round_id")?,
        entry_type: row.try_get("entry_type")?,
        title: row.try_get("title")?,
        content: row.try_get("content")?,
        contributed_by: row.try_get("contributed_by")?,
        is_resolved: row.try_get::<bool, _>("is_resolved").unwrap_or(false),
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

pub async fn get_blackboard_entry_archive(
    pool: &SqlitePool,
    session_id: &str,
    entry_id: &str,
    limit: i64,
) -> Result<AgentTeamBlackboardArchive> {
    ensure_schema(pool).await?;

    let safe_limit = limit.clamp(10, 400);
    let entry_row = sqlx::query(
        r#"SELECT id, session_id, round_id, entry_type, title, content,
                  contributed_by, is_resolved, created_at, updated_at
           FROM agent_team_blackboard_entries
           WHERE session_id = ? AND id = ?"#,
    )
    .bind(session_id)
    .bind(entry_id)
    .fetch_one(pool)
    .await?;

    let entry = AgentTeamBlackboardEntry {
        id: entry_row.try_get("id")?,
        session_id: entry_row.try_get("session_id")?,
        round_id: entry_row.try_get("round_id")?,
        entry_type: entry_row.try_get("entry_type")?,
        title: entry_row.try_get("title")?,
        content: entry_row.try_get("content")?,
        contributed_by: entry_row.try_get("contributed_by")?,
        is_resolved: entry_row.try_get::<bool, _>("is_resolved").unwrap_or(false),
        created_at: entry_row.try_get("created_at")?,
        updated_at: entry_row.try_get("updated_at")?,
    };

    let mut retrieval_scope = if entry.round_id.is_some() {
        "round".to_string()
    } else {
        "session_recent".to_string()
    };

    let mut messages = if let Some(round_id) = entry.round_id.as_deref() {
        let rows = sqlx::query(
            r#"SELECT id, session_id, round_id, member_id, member_name, role, content,
                      tool_calls, token_count, timestamp
               FROM agent_team_messages
               WHERE session_id = ? AND round_id = ?
               ORDER BY timestamp ASC
               LIMIT ?"#,
        )
        .bind(session_id)
        .bind(round_id)
        .bind(safe_limit)
        .fetch_all(pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(AgentTeamMessage {
                    id: row.try_get("id")?,
                    session_id: row.try_get("session_id")?,
                    round_id: row.try_get("round_id")?,
                    member_id: row.try_get("member_id")?,
                    member_name: row.try_get("member_name")?,
                    role: row.try_get("role")?,
                    content: row.try_get("content")?,
                    tool_calls: row
                        .try_get::<Option<String>, _>("tool_calls")?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    token_count: row.try_get("token_count")?,
                    timestamp: row.try_get("timestamp")?,
                })
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };

    if messages.is_empty() {
        if entry.round_id.is_some() {
            retrieval_scope = "session_recent_fallback".to_string();
        }
        let rows = sqlx::query(
            r#"SELECT id, session_id, round_id, member_id, member_name, role, content,
                      tool_calls, token_count, timestamp
               FROM (
                   SELECT id, session_id, round_id, member_id, member_name, role, content,
                          tool_calls, token_count, timestamp
                   FROM agent_team_messages
                   WHERE session_id = ?
                   ORDER BY timestamp DESC
                   LIMIT ?
               ) recent
               ORDER BY timestamp ASC"#,
        )
        .bind(session_id)
        .bind(safe_limit)
        .fetch_all(pool)
        .await?;

        messages = rows
            .into_iter()
            .map(|row| {
                Ok(AgentTeamMessage {
                    id: row.try_get("id")?,
                    session_id: row.try_get("session_id")?,
                    round_id: row.try_get("round_id")?,
                    member_id: row.try_get("member_id")?,
                    member_name: row.try_get("member_name")?,
                    role: row.try_get("role")?,
                    content: row.try_get("content")?,
                    tool_calls: row
                        .try_get::<Option<String>, _>("tool_calls")?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    token_count: row.try_get("token_count")?,
                    timestamp: row.try_get("timestamp")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
    }

    Ok(AgentTeamBlackboardArchive {
        entry,
        messages,
        retrieval_scope,
    })
}

// ==================== 产物操作 ====================

pub async fn create_artifact(
    pool: &SqlitePool,
    session_id: &str,
    artifact_type: &str,
    title: &str,
    content: &str,
    created_by: Option<&str>,
    parent_artifact_id: Option<&str>,
    diff_summary: Option<&str>,
) -> Result<AgentTeamArtifact> {
    ensure_schema(pool).await?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let version_row = sqlx::query(
        "SELECT COALESCE(MAX(version), 0) as max_version FROM agent_team_artifacts WHERE session_id = ? AND artifact_type = ?",
    )
    .bind(session_id)
    .bind(artifact_type)
    .fetch_one(pool)
    .await?;
    let version: i32 = version_row.try_get::<i32, _>("max_version").unwrap_or(0) + 1;

    sqlx::query(
        r#"INSERT INTO agent_team_artifacts
           (id, session_id, artifact_type, title, content, version, parent_artifact_id,
            diff_summary, created_by, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(session_id)
    .bind(artifact_type)
    .bind(title)
    .bind(content)
    .bind(version)
    .bind(parent_artifact_id)
    .bind(diff_summary)
    .bind(created_by)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(AgentTeamArtifact {
        id,
        session_id: session_id.to_string(),
        artifact_type: artifact_type.to_string(),
        title: title.to_string(),
        content: content.to_string(),
        version,
        parent_artifact_id: parent_artifact_id.map(|s| s.to_string()),
        diff_summary: diff_summary.map(|s| s.to_string()),
        created_by: created_by.map(|s| s.to_string()),
        created_at: now,
        updated_at: now,
    })
}

pub async fn list_artifacts(pool: &SqlitePool, session_id: &str) -> Result<Vec<AgentTeamArtifact>> {
    ensure_schema(pool).await?;

    let rows = sqlx::query(
        r#"SELECT id, session_id, artifact_type, title, content, version,
                  parent_artifact_id, diff_summary, created_by, created_at, updated_at
           FROM agent_team_artifacts WHERE session_id = ? ORDER BY created_at DESC"#,
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(AgentTeamArtifact {
                id: row.try_get("id")?,
                session_id: row.try_get("session_id")?,
                artifact_type: row.try_get("artifact_type")?,
                title: row.try_get("title")?,
                content: row.try_get("content")?,
                version: row.try_get::<i32, _>("version").unwrap_or(1),
                parent_artifact_id: row.try_get("parent_artifact_id")?,
                diff_summary: row.try_get("diff_summary")?,
                created_by: row.try_get("created_by")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        })
        .collect()
}

pub async fn get_artifact_detail(
    pool: &SqlitePool,
    artifact_id: &str,
) -> Result<Option<AgentTeamArtifact>> {
    ensure_schema(pool).await?;

    let row_opt = sqlx::query(
        r#"SELECT id, session_id, artifact_type, title, content, version,
                  parent_artifact_id, diff_summary, created_by, created_at, updated_at
           FROM agent_team_artifacts WHERE id = ?"#,
    )
    .bind(artifact_id)
    .fetch_optional(pool)
    .await?;

    Ok(row_opt.map(|row| AgentTeamArtifact {
        id: row.try_get("id").unwrap_or_default(),
        session_id: row.try_get("session_id").unwrap_or_default(),
        artifact_type: row.try_get("artifact_type").unwrap_or_default(),
        title: row.try_get("title").unwrap_or_default(),
        content: row.try_get("content").unwrap_or_default(),
        version: row.try_get::<i32, _>("version").unwrap_or(1),
        parent_artifact_id: row.try_get("parent_artifact_id").ok().flatten(),
        diff_summary: row.try_get("diff_summary").ok().flatten(),
        created_by: row.try_get("created_by").ok().flatten(),
        created_at: row.try_get("created_at").unwrap_or_else(|_| Utc::now()),
        updated_at: row.try_get("updated_at").unwrap_or_else(|_| Utc::now()),
    }))
}

pub async fn ensure_session_tasks_from_runtime_spec(
    pool: &SqlitePool,
    session_id: &str,
    runtime_spec_v2: &serde_json::Value,
) -> Result<()> {
    ensure_schema(pool).await?;

    let task_graph = runtime_spec_v2
        .get("task_graph")
        .ok_or_else(|| anyhow!("runtime_spec_v2.task_graph missing"))?;
    let nodes = task_graph
        .get("nodes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("runtime_spec_v2.task_graph.nodes missing"))?;

    for node in nodes {
        let task_id = node
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("task node id missing"))?;
        let title = node
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(task_id);
        let instruction = node
            .get("instruction")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let depends_on = node
            .get("depends_on")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let max_attempts = node
            .get("retry")
            .and_then(|v| v.get("max_attempts"))
            .and_then(|v| v.as_i64())
            .unwrap_or(1)
            .clamp(1, 10) as i32;
        let assignee_agent_id = node
            .get("assignee_strategy")
            .and_then(|v| v.get("agent_id").or_else(|| v.get("agent_name")))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM agent_team_tasks WHERE session_id = ? AND task_id = ?",
        )
        .bind(session_id)
        .bind(task_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        if exists > 0 {
            continue;
        }

        sqlx::query(
            r#"INSERT INTO agent_team_tasks
               (id, session_id, task_id, title, instruction, status, assignee_agent_id, depends_on, attempt, max_attempts, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, 'pending', ?, ?, 0, ?, ?, ?)"#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(session_id)
        .bind(task_id)
        .bind(title)
        .bind(instruction)
        .bind(assignee_agent_id)
        .bind(serde_json::Value::Array(depends_on).to_string())
        .bind(max_attempts)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn list_tasks(pool: &SqlitePool, session_id: &str) -> Result<Vec<TeamTask>> {
    ensure_schema(pool).await?;
    let rows = sqlx::query(
        r#"SELECT id, session_id, task_id, title, instruction, status, assignee_agent_id, depends_on,
                  attempt, max_attempts, last_error, started_at, completed_at, created_at, updated_at
           FROM agent_team_tasks
           WHERE session_id = ?
           ORDER BY created_at ASC"#,
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(TeamTask {
                id: row.try_get("id")?,
                session_id: row.try_get("session_id")?,
                task_id: row.try_get("task_id")?,
                title: row.try_get("title")?,
                instruction: row.try_get("instruction")?,
                status: row.try_get("status")?,
                assignee_agent_id: row.try_get("assignee_agent_id")?,
                depends_on: row
                    .try_get::<String, _>("depends_on")
                    .ok()
                    .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
                    .unwrap_or_default(),
                attempt: row.try_get::<i32, _>("attempt").unwrap_or(0),
                max_attempts: row.try_get::<i32, _>("max_attempts").unwrap_or(1),
                last_error: row.try_get("last_error")?,
                started_at: row.try_get("started_at")?,
                completed_at: row.try_get("completed_at")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        })
        .collect()
}

pub async fn update_task(
    pool: &SqlitePool,
    session_id: &str,
    patch: &UpdateTaskRequest,
) -> Result<()> {
    ensure_schema(pool).await?;
    let now = Utc::now();
    sqlx::query(
        r#"UPDATE agent_team_tasks
           SET status = COALESCE(?, status),
               assignee_agent_id = COALESCE(?, assignee_agent_id),
               last_error = CASE
                   WHEN ? IS NOT NULL THEN ?
                   WHEN LOWER(COALESCE(?, '')) IN ('running', 'completed') THEN NULL
                   ELSE last_error
               END,
               attempt = CASE
                   WHEN LOWER(COALESCE(?, '')) = 'running' THEN attempt + 1
                   ELSE attempt
               END,
               started_at = CASE
                   WHEN LOWER(COALESCE(?, '')) = 'running' THEN COALESCE(started_at, ?)
                   ELSE started_at
               END,
               completed_at = CASE
                   WHEN LOWER(COALESCE(?, '')) = 'running' THEN NULL
                   WHEN LOWER(COALESCE(?, '')) IN ('completed', 'failed', 'cancelled', 'blocked') THEN ?
                   ELSE completed_at
               END,
               updated_at = ?
           WHERE session_id = ? AND task_id = ?"#,
    )
    .bind(&patch.status)
    .bind(&patch.assignee_agent_id)
    .bind(&patch.last_error)
    .bind(&patch.last_error)
    .bind(&patch.status)
    .bind(&patch.status)
    .bind(&patch.status)
    .bind(now.clone())
    .bind(&patch.status)
    .bind(&patch.status)
    .bind(now.clone())
    .bind(now.clone())
    .bind(session_id)
    .bind(&patch.task_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_mailbox(
    pool: &SqlitePool,
    session_id: &str,
    agent_id: Option<&str>,
) -> Result<Vec<MailboxMessage>> {
    ensure_schema(pool).await?;
    let rows = if let Some(agent_id) = agent_id {
        sqlx::query(
            r#"SELECT id, session_id, from_agent_id, to_agent_id, task_record_id, message_type, payload, is_acknowledged, created_at, acknowledged_at
               FROM agent_team_mailbox
               WHERE session_id = ? AND (to_agent_id = ? OR to_agent_id IS NULL)
               ORDER BY created_at DESC"#,
        )
        .bind(session_id)
        .bind(agent_id)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT id, session_id, from_agent_id, to_agent_id, task_record_id, message_type, payload, is_acknowledged, created_at, acknowledged_at
               FROM agent_team_mailbox
               WHERE session_id = ?
               ORDER BY created_at DESC"#,
        )
        .bind(session_id)
        .fetch_all(pool)
        .await?
    };

    rows.into_iter()
        .map(|row| {
            Ok(MailboxMessage {
                id: row.try_get("id")?,
                session_id: row.try_get("session_id")?,
                from_agent_id: row.try_get("from_agent_id")?,
                to_agent_id: row.try_get("to_agent_id")?,
                task_record_id: row.try_get("task_record_id")?,
                message_type: row.try_get("message_type")?,
                payload: row
                    .try_get::<String, _>("payload")
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_else(|| serde_json::json!({})),
                is_acknowledged: row.try_get::<bool, _>("is_acknowledged").unwrap_or(false),
                created_at: row.try_get("created_at")?,
                acknowledged_at: row.try_get("acknowledged_at")?,
            })
        })
        .collect()
}

pub async fn ack_mailbox_message(pool: &SqlitePool, message_id: &str) -> Result<()> {
    ensure_schema(pool).await?;
    sqlx::query(
        r#"UPDATE agent_team_mailbox
           SET is_acknowledged = 1, acknowledged_at = ?
           WHERE id = ?"#,
    )
    .bind(Utc::now())
    .bind(message_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn create_mailbox_message(
    pool: &SqlitePool,
    session_id: &str,
    from_agent_id: Option<&str>,
    to_agent_id: Option<&str>,
    task_record_id: Option<&str>,
    message_type: &str,
    payload: &serde_json::Value,
) -> Result<()> {
    ensure_schema(pool).await?;
    sqlx::query(
        r#"INSERT INTO agent_team_mailbox
           (id, session_id, from_agent_id, to_agent_id, task_record_id, message_type, payload, is_acknowledged, created_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?)"#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(session_id)
    .bind(from_agent_id)
    .bind(to_agent_id)
    .bind(task_record_id)
    .bind(message_type)
    .bind(payload.to_string())
    .bind(Utc::now())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn append_task_attempt(
    pool: &SqlitePool,
    session_id: &str,
    task_record_id: &str,
    attempt: i32,
    status: &str,
    error: Option<&str>,
    duration_ms: Option<i64>,
) -> Result<()> {
    ensure_schema(pool).await?;
    sqlx::query(
        r#"INSERT INTO agent_team_task_attempts
           (id, session_id, task_record_id, attempt, status, error, duration_ms, created_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(session_id)
    .bind(task_record_id)
    .bind(attempt)
    .bind(status)
    .bind(error)
    .bind(duration_ms)
    .bind(Utc::now())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn append_task_event(
    pool: &SqlitePool,
    session_id: &str,
    task_record_id: Option<&str>,
    event_type: &str,
    payload: &serde_json::Value,
) -> Result<()> {
    ensure_schema(pool).await?;
    sqlx::query(
        r#"INSERT INTO agent_team_task_events
           (id, session_id, task_record_id, event_type, payload, created_at)
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(session_id)
    .bind(task_record_id)
    .bind(event_type)
    .bind(payload.to_string())
    .bind(Utc::now())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upgrade_templates_to_v2(pool: &SqlitePool, force: bool) -> Result<i64> {
    ensure_schema(pool).await?;
    let rows = sqlx::query(
        r#"SELECT id, name, domain, default_rounds_config, default_tool_policy
           FROM agent_team_templates
           WHERE schema_version < ?
              OR schema_version IS NULL
              OR (upgrade_failed = 1 AND ? = 1)"#,
    )
    .bind(AGENT_TEAM_SCHEMA_V2)
    .bind(if force { 1 } else { 0 })
    .fetch_all(pool)
    .await?;

    let mut upgraded = 0i64;
    for row in rows {
        let template_id: String = row.try_get("id")?;
        let default_rounds_config = row
            .try_get::<Option<String>, _>("default_rounds_config")?
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
        let members = sqlx::query(
            r#"SELECT name, responsibility, system_prompt, decision_style, risk_preference, weight, tool_policy, output_schema, sort_order
               FROM agent_team_template_members
               WHERE template_id = ?
               ORDER BY sort_order ASC"#,
        )
        .bind(&template_id)
        .fetch_all(pool)
        .await?;

        let member_reqs: Vec<CreateAgentTeamTemplateMemberRequest> = members
            .into_iter()
            .map(|m| CreateAgentTeamTemplateMemberRequest {
                name: m.try_get("name").unwrap_or_default(),
                responsibility: m.try_get("responsibility").ok().flatten(),
                system_prompt: m.try_get("system_prompt").ok().flatten(),
                decision_style: m.try_get("decision_style").ok().flatten(),
                risk_preference: m.try_get("risk_preference").ok().flatten(),
                weight: Some(m.try_get::<f64, _>("weight").unwrap_or(1.0)),
                tool_policy: m
                    .try_get::<Option<String>, _>("tool_policy")
                    .ok()
                    .flatten()
                    .and_then(|s| serde_json::from_str(&s).ok()),
                output_schema: m
                    .try_get::<Option<String>, _>("output_schema")
                    .ok()
                    .flatten()
                    .and_then(|s| serde_json::from_str(&s).ok()),
                sort_order: Some(m.try_get::<i32, _>("sort_order").unwrap_or(0)),
            })
            .collect();

        match build_template_spec_v2_from_legacy(default_rounds_config.as_ref(), &member_reqs, None)
        {
            Ok(spec_v2) => {
                sqlx::query(
                    r#"UPDATE agent_team_templates
                       SET schema_version = ?,
                           template_spec_v2 = ?,
                           upgrade_failed = 0,
                           upgrade_error = NULL,
                           updated_at = ?
                       WHERE id = ?"#,
                )
                .bind(AGENT_TEAM_SCHEMA_V2)
                .bind(serde_json::to_string(&spec_v2).ok())
                .bind(Utc::now())
                .bind(&template_id)
                .execute(pool)
                .await?;
                upgraded += 1;
            }
            Err(e) => {
                sqlx::query(
                    r#"UPDATE agent_team_templates
                       SET upgrade_failed = 1,
                           upgrade_error = ?,
                           updated_at = ?
                       WHERE id = ?"#,
                )
                .bind(e.to_string())
                .bind(Utc::now())
                .bind(&template_id)
                .execute(pool)
                .await?;
            }
        }
    }

    Ok(upgraded)
}

async fn get_template_members(
    pool: &SqlitePool,
    template_id: &str,
) -> Result<Vec<AgentTeamTemplateMember>> {
    let rows = sqlx::query(
        r#"SELECT id, template_id, name, responsibility, system_prompt, decision_style,
                  risk_preference, weight, tool_policy, output_schema, sort_order,
                  created_at, updated_at
           FROM agent_team_template_members
           WHERE template_id = ?
           ORDER BY sort_order ASC"#,
    )
    .bind(template_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| {
            Ok(AgentTeamTemplateMember {
                id: r.try_get("id")?,
                template_id: r.try_get("template_id")?,
                name: r.try_get("name")?,
                responsibility: r.try_get("responsibility")?,
                system_prompt: r.try_get("system_prompt")?,
                decision_style: r.try_get("decision_style")?,
                risk_preference: r.try_get("risk_preference")?,
                weight: r.try_get("weight")?,
                tool_policy: r
                    .try_get::<Option<String>, _>("tool_policy")?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                output_schema: r
                    .try_get::<Option<String>, _>("output_schema")?
                    .and_then(|s| serde_json::from_str(&s).ok()),
                sort_order: r.try_get::<i32, _>("sort_order").unwrap_or(0),
                created_at: r.try_get("created_at")?,
                updated_at: r.try_get("updated_at")?,
            })
        })
        .collect::<Result<Vec<_>>>()
}

async fn snapshot_members_from_template(
    pool: &SqlitePool,
    session_id: &str,
    template_id: &str,
) -> Result<()> {
    let now = Utc::now();
    let template_members = sqlx::query(
        r#"SELECT name, responsibility, system_prompt, decision_style, risk_preference,
                  weight, tool_policy, output_schema, sort_order
           FROM agent_team_template_members WHERE template_id = ? ORDER BY sort_order"#,
    )
    .bind(template_id)
    .fetch_all(pool)
    .await?;

    for m in template_members {
        let member_id = Uuid::new_v4().to_string();
        let name: String = m.try_get("name")?;
        let responsibility: Option<String> = m.try_get("responsibility")?;
        let system_prompt: Option<String> = m.try_get("system_prompt")?;
        let decision_style: Option<String> = m.try_get("decision_style")?;
        let risk_preference: Option<String> = m.try_get("risk_preference")?;
        let weight: f64 = m.try_get::<f64, _>("weight").unwrap_or(1.0);
        let tool_policy: Option<String> = m.try_get("tool_policy")?;
        let output_schema: Option<String> = m.try_get("output_schema")?;
        let sort_order: i32 = m.try_get::<i32, _>("sort_order").unwrap_or(0);

        sqlx::query(
            r#"INSERT INTO agent_team_members
               (id, session_id, name, responsibility, system_prompt, decision_style,
                risk_preference, weight, tool_policy, output_schema, sort_order,
                token_usage, tool_calls_count, is_active, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&member_id)
        .bind(session_id)
        .bind(&name)
        .bind(&responsibility)
        .bind(&system_prompt)
        .bind(&decision_style)
        .bind(&risk_preference)
        .bind(weight)
        .bind(&tool_policy)
        .bind(&output_schema)
        .bind(sort_order)
        .bind(0i64)
        .bind(0i32)
        .bind(true)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn ensure_schema(pool: &SqlitePool) -> Result<()> {
    // agent_team_templates
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS agent_team_templates (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            domain TEXT NOT NULL DEFAULT 'product',
            default_rounds_config TEXT,
            default_tool_policy TEXT,
            schema_version INTEGER NOT NULL DEFAULT 1,
            template_spec_v2 TEXT,
            upgrade_failed BOOLEAN NOT NULL DEFAULT FALSE,
            upgrade_error TEXT,
            is_system BOOLEAN NOT NULL DEFAULT FALSE,
            created_by TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await?;
    ensure_column_if_not_exists(
        pool,
        "agent_team_templates",
        "schema_version",
        "INTEGER NOT NULL DEFAULT 1",
    )
    .await?;
    ensure_column_if_not_exists(pool, "agent_team_templates", "template_spec_v2", "TEXT").await?;
    ensure_column_if_not_exists(
        pool,
        "agent_team_templates",
        "upgrade_failed",
        "BOOLEAN NOT NULL DEFAULT FALSE",
    )
    .await?;
    ensure_column_if_not_exists(pool, "agent_team_templates", "upgrade_error", "TEXT").await?;

    // agent_team_template_members
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS agent_team_template_members (
            id TEXT PRIMARY KEY,
            template_id TEXT NOT NULL REFERENCES agent_team_templates(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            responsibility TEXT,
            system_prompt TEXT,
            decision_style TEXT DEFAULT 'balanced',
            risk_preference TEXT DEFAULT 'medium',
            weight REAL NOT NULL DEFAULT 1.0,
            tool_policy TEXT,
            output_schema TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await?;

    // agent_team_sessions
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS agent_team_sessions (
            id TEXT PRIMARY KEY,
            conversation_id TEXT,
            template_id TEXT,
            name TEXT NOT NULL,
            goal TEXT,
            orchestration_plan TEXT,
            schema_version INTEGER NOT NULL DEFAULT 1,
            runtime_spec_v2 TEXT,
            plan_version INTEGER NOT NULL DEFAULT 1,
            state TEXT NOT NULL DEFAULT 'PENDING',
            state_machine TEXT,
            current_round INTEGER DEFAULT 0,
            max_rounds INTEGER DEFAULT 5,
            blackboard_state TEXT,
            divergence_scores TEXT,
            total_tokens INTEGER DEFAULT 0,
            estimated_cost REAL DEFAULT 0.0,
            suspended_reason TEXT,
            started_at DATETIME,
            completed_at DATETIME,
            error_message TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await?;
    ensure_column_if_not_exists(pool, "agent_team_sessions", "orchestration_plan", "TEXT").await?;
    ensure_column_if_not_exists(
        pool,
        "agent_team_sessions",
        "plan_version",
        "INTEGER NOT NULL DEFAULT 1",
    )
    .await?;
    ensure_column_if_not_exists(
        pool,
        "agent_team_sessions",
        "schema_version",
        "INTEGER NOT NULL DEFAULT 1",
    )
    .await?;
    ensure_column_if_not_exists(pool, "agent_team_sessions", "runtime_spec_v2", "TEXT").await?;

    // agent_team_members
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS agent_team_members (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES agent_team_sessions(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            responsibility TEXT,
            system_prompt TEXT,
            decision_style TEXT DEFAULT 'balanced',
            risk_preference TEXT DEFAULT 'medium',
            weight REAL NOT NULL DEFAULT 1.0,
            tool_policy TEXT,
            output_schema TEXT,
            sort_order INTEGER DEFAULT 0,
            token_usage INTEGER DEFAULT 0,
            tool_calls_count INTEGER DEFAULT 0,
            is_active BOOLEAN NOT NULL DEFAULT TRUE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await?;

    // agent_team_blackboard_entries
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS agent_team_blackboard_entries (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES agent_team_sessions(id) ON DELETE CASCADE,
            round_id TEXT,
            entry_type TEXT NOT NULL CHECK (entry_type IN ('consensus', 'dispute', 'action_item')),
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            contributed_by TEXT,
            is_resolved BOOLEAN NOT NULL DEFAULT FALSE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await?;

    // agent_team_rounds
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS agent_team_rounds (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES agent_team_sessions(id) ON DELETE CASCADE,
            round_number INTEGER NOT NULL,
            phase TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'running',
            divergence_score REAL,
            started_at DATETIME,
            completed_at DATETIME,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await?;

    // agent_team_messages
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS agent_team_messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES agent_team_sessions(id) ON DELETE CASCADE,
            round_id TEXT REFERENCES agent_team_rounds(id),
            member_id TEXT,
            member_name TEXT,
            role TEXT NOT NULL DEFAULT 'assistant',
            content TEXT NOT NULL,
            tool_calls TEXT,
            token_count INTEGER,
            timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await?;

    // agent_team_decisions
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS agent_team_decisions (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES agent_team_sessions(id) ON DELETE CASCADE,
            round_id TEXT,
            decision_type TEXT NOT NULL DEFAULT 'final',
            content TEXT NOT NULL,
            decided_by TEXT,
            confidence_score REAL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await?;

    // agent_team_artifacts
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS agent_team_artifacts (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES agent_team_sessions(id) ON DELETE CASCADE,
            artifact_type TEXT NOT NULL,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            parent_artifact_id TEXT,
            diff_summary TEXT,
            created_by TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await?;

    // agent_team_tasks
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS agent_team_tasks (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES agent_team_sessions(id) ON DELETE CASCADE,
            task_id TEXT NOT NULL,
            title TEXT NOT NULL,
            instruction TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            assignee_agent_id TEXT,
            depends_on TEXT NOT NULL DEFAULT '[]',
            attempt INTEGER NOT NULL DEFAULT 0,
            max_attempts INTEGER NOT NULL DEFAULT 1,
            last_error TEXT,
            started_at DATETIME,
            completed_at DATETIME,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await?;

    // agent_team_task_attempts
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS agent_team_task_attempts (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES agent_team_sessions(id) ON DELETE CASCADE,
            task_record_id TEXT NOT NULL REFERENCES agent_team_tasks(id) ON DELETE CASCADE,
            attempt INTEGER NOT NULL,
            status TEXT NOT NULL,
            error TEXT,
            duration_ms INTEGER,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await?;

    // agent_team_mailbox
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS agent_team_mailbox (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES agent_team_sessions(id) ON DELETE CASCADE,
            from_agent_id TEXT,
            to_agent_id TEXT,
            task_record_id TEXT REFERENCES agent_team_tasks(id) ON DELETE SET NULL,
            message_type TEXT NOT NULL DEFAULT 'handoff',
            payload TEXT NOT NULL DEFAULT '{}',
            is_acknowledged BOOLEAN NOT NULL DEFAULT FALSE,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            acknowledged_at DATETIME
        )"#,
    )
    .execute(pool)
    .await?;

    // agent_team_task_events
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS agent_team_task_events (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES agent_team_sessions(id) ON DELETE CASCADE,
            task_record_id TEXT REFERENCES agent_team_tasks(id) ON DELETE SET NULL,
            event_type TEXT NOT NULL,
            payload TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await?;

    // Key indices
    let index_sqls = [
        "CREATE INDEX IF NOT EXISTS idx_agent_team_templates_domain ON agent_team_templates(domain)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_templates_is_system ON agent_team_templates(is_system)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_templates_schema_version ON agent_team_templates(schema_version)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_template_members_template_id ON agent_team_template_members(template_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_sessions_state ON agent_team_sessions(state)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_sessions_conversation_id ON agent_team_sessions(conversation_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_sessions_updated ON agent_team_sessions(updated_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_sessions_schema_version ON agent_team_sessions(schema_version)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_members_session_id ON agent_team_members(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_blackboard_session_id ON agent_team_blackboard_entries(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_rounds_session_id ON agent_team_rounds(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_messages_session_id ON agent_team_messages(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_artifacts_session_id ON agent_team_artifacts(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_tasks_session_status ON agent_team_tasks(session_id, status)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_tasks_assignee_status ON agent_team_tasks(assignee_agent_id, status)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_tasks_created_at ON agent_team_tasks(created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_task_attempts_task_record_id ON agent_team_task_attempts(task_record_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_mailbox_session_to_ack ON agent_team_mailbox(session_id, to_agent_id, is_acknowledged)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_mailbox_created_at ON agent_team_mailbox(created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_task_events_session_created_at ON agent_team_task_events(session_id, created_at ASC)",
    ];

    for sql in index_sqls {
        sqlx::query(sql).execute(pool).await?;
    }

    Ok(())
}

async fn ensure_column_if_not_exists(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    column_def: &str,
) -> Result<()> {
    let pragma_sql = format!("PRAGMA table_info({})", table);
    let rows = sqlx::query(&pragma_sql).fetch_all(pool).await?;
    let exists = rows.into_iter().any(|row| {
        row.try_get::<String, _>("name")
            .map(|name| name == column)
            .unwrap_or(false)
    });
    if exists {
        return Ok(());
    }

    let alter_sql = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, column_def);
    sqlx::query(&alter_sql).execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create sqlite memory pool")
    }

    async fn seed_legacy_template(
        pool: &SqlitePool,
        template_id: &str,
        members: &[&str],
    ) -> Result<()> {
        ensure_schema(pool).await?;
        let now = Utc::now();
        sqlx::query(
            r#"INSERT INTO agent_team_templates
               (id, name, description, domain, default_rounds_config, schema_version, upgrade_failed, is_system, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(template_id)
        .bind(format!("Legacy {}", template_id))
        .bind(Option::<String>::None)
        .bind("product")
        .bind(Some(json!({"max_rounds": 3}).to_string()))
        .bind(1i32)
        .bind(false)
        .bind(false)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        for (idx, member_name) in members.iter().enumerate() {
            sqlx::query(
                r#"INSERT INTO agent_team_template_members
                   (id, template_id, name, weight, sort_order, created_at, updated_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(template_id)
            .bind(member_name.to_string())
            .bind(1.0f64)
            .bind(idx as i32)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn agent_team_v2_e2e_upgrade_and_recovery_flow() -> Result<()> {
        let pool = test_pool().await;
        seed_legacy_template(&pool, "tpl-upgrade-ok", &["产品经理", "架构师"]).await?;

        let upgraded = upgrade_templates_to_v2(&pool, false).await?;
        assert_eq!(upgraded, 1, "expected one template upgraded");

        let template = get_agent_team_template_detail(&pool, "tpl-upgrade-ok")
            .await?
            .expect("template should exist");
        assert_eq!(template.schema_version, AGENT_TEAM_SCHEMA_V2);
        assert!(!template.upgrade_failed);
        let spec = template
            .template_spec_v2
            .clone()
            .expect("template_spec_v2 should be filled after upgrade");
        let agents_len = spec
            .get("agents")
            .and_then(|v| v.as_array())
            .map(|v| v.len())
            .unwrap_or(0);
        let nodes_len = spec
            .get("task_graph")
            .and_then(|v| v.get("nodes"))
            .and_then(|v| v.as_array())
            .map(|v| v.len())
            .unwrap_or(0);
        assert!(agents_len >= 2);
        assert!(nodes_len >= 1);

        let session = create_agent_team_session(
            &pool,
            &CreateAgentTeamSessionRequest {
                name: "V2 E2E Session".to_string(),
                goal: Some("验证升级与恢复路径".to_string()),
                template_id: Some("tpl-upgrade-ok".to_string()),
                conversation_id: None,
                max_rounds: Some(4),
                schema_version: Some(AGENT_TEAM_SCHEMA_V2),
                runtime_spec_v2: None,
                state_machine: Some(json!({})),
            },
        )
        .await?;
        assert_eq!(session.schema_version, AGENT_TEAM_SCHEMA_V2);

        let runtime_spec = session
            .runtime_spec_v2
            .clone()
            .expect("session should snapshot runtime_spec_v2");

        let mut tasks = list_tasks(&pool, &session.id).await?;
        assert!(tasks.len() >= 2, "task graph should create task rows");
        let initial_count = tasks.len();

        ensure_session_tasks_from_runtime_spec(&pool, &session.id, &runtime_spec).await?;
        let tasks_after_ensure = list_tasks(&pool, &session.id).await?;
        assert_eq!(initial_count, tasks_after_ensure.len(), "ensure should be idempotent");

        let first_task = tasks
            .iter()
            .find(|t| t.depends_on.is_empty())
            .cloned()
            .expect("should have one root task");
        let second_task = tasks
            .iter()
            .find(|t| t.task_id != first_task.task_id)
            .cloned()
            .expect("should have another dependent task");

        update_task(
            &pool,
            &session.id,
            &UpdateTaskRequest {
                task_id: first_task.task_id.clone(),
                status: Some("completed".to_string()),
                assignee_agent_id: Some("agent-product".to_string()),
                last_error: None,
            },
        )
        .await?;
        update_task(
            &pool,
            &session.id,
            &UpdateTaskRequest {
                task_id: second_task.task_id.clone(),
                status: Some("failed".to_string()),
                assignee_agent_id: Some("agent-arch".to_string()),
                last_error: Some("simulated failure".to_string()),
            },
        )
        .await?;

        create_mailbox_message(
            &pool,
            &session.id,
            Some("agent-product"),
            Some("agent-arch"),
            Some(&second_task.id),
            "task_failed",
            &json!({"task_id": second_task.task_id, "error": "simulated failure"}),
        )
        .await?;
        let inbox_before = list_mailbox(&pool, &session.id, Some("agent-arch")).await?;
        assert!(!inbox_before.is_empty());
        assert!(!inbox_before[0].is_acknowledged);
        let message_id = inbox_before[0].id.clone();

        ack_mailbox_message(&pool, &message_id).await?;
        let inbox_after = list_mailbox(&pool, &session.id, Some("agent-arch")).await?;
        assert!(inbox_after[0].is_acknowledged);

        update_task(
            &pool,
            &session.id,
            &UpdateTaskRequest {
                task_id: second_task.task_id.clone(),
                status: Some("pending".to_string()),
                assignee_agent_id: Some("agent-arch".to_string()),
                last_error: None,
            },
        )
        .await?;

        tasks = list_tasks(&pool, &session.id).await?;
        let first_after = tasks
            .iter()
            .find(|t| t.task_id == first_task.task_id)
            .expect("first task should still exist");
        let second_after = tasks
            .iter()
            .find(|t| t.task_id == second_task.task_id)
            .expect("second task should still exist");
        assert_eq!(first_after.status.to_lowercase(), "completed");
        assert_eq!(second_after.status.to_lowercase(), "pending");

        Ok(())
    }

    #[tokio::test]
    async fn agent_team_v2_upgrade_failure_blocks_session_creation() -> Result<()> {
        let pool = test_pool().await;
        seed_legacy_template(&pool, "tpl-upgrade-fail", &[]).await?;

        let upgraded = upgrade_templates_to_v2(&pool, false).await?;
        assert_eq!(
            upgraded, 0,
            "templates without members should not be upgraded"
        );

        let template = get_agent_team_template_detail(&pool, "tpl-upgrade-fail")
            .await?
            .expect("template should exist");
        assert!(template.upgrade_failed, "template should be marked as upgrade_failed");
        assert!(
            template
                .upgrade_error
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false),
            "upgrade error should be recorded"
        );

        let create_result = create_agent_team_session(
            &pool,
            &CreateAgentTeamSessionRequest {
                name: "Blocked Session".to_string(),
                goal: Some("should fail".to_string()),
                template_id: Some("tpl-upgrade-fail".to_string()),
                conversation_id: None,
                max_rounds: Some(3),
                schema_version: Some(AGENT_TEAM_SCHEMA_V2),
                runtime_spec_v2: None,
                state_machine: None,
            },
        )
        .await;
        assert!(create_result.is_err());
        let err_text = create_result
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default()
            .to_lowercase();
        assert!(err_text.contains("upgrade failed"));

        Ok(())
    }
}

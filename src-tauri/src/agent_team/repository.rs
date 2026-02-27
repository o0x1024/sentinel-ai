//! Agent Team 数据库操作层（纯运行时查询，无需 DATABASE_URL）

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::postgres::PgPool;
use sqlx::Row;
use std::collections::HashSet;
use tracing::info;
use uuid::Uuid;

use super::models::*;

pub(crate) const AGENT_TEAM_SCHEMA_V2: i32 = 2;

fn normalize_agent_id(name: &str, index: usize) -> String {
    let mut id = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>();
    id = id.trim_matches('-').to_string();
    if id.is_empty() {
        format!("agent-{}", index + 1)
    } else {
        format!("agent-{}", id)
    }
}

pub(crate) fn build_agents_from_members(
    members: &[CreateAgentTeamTemplateMemberRequest],
) -> Vec<AgentProfile> {
    members
        .iter()
        .enumerate()
        .map(|(idx, member)| AgentProfile {
            id: normalize_agent_id(&member.name, idx),
            name: member.name.clone(),
            system_prompt: member.system_prompt.clone(),
            model: member
                .output_schema
                .as_ref()
                .and_then(|schema| schema.get("llm_model"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            tool_policy: member.tool_policy.clone(),
            skills: vec![],
            max_parallel_tasks: Some(1),
        })
        .collect()
}

pub(crate) fn make_member_requests_from_agents(
    agents: &[AgentProfile],
) -> Vec<CreateAgentTeamTemplateMemberRequest> {
    agents
        .iter()
        .enumerate()
        .map(|(idx, agent)| {
            let output_schema = agent.model.as_ref().map(|m| {
                let mut provider = String::new();
                let mut model = m.clone();
                if let Some((p, mm)) = m.split_once('/') {
                    provider = p.to_string();
                    model = mm.to_string();
                }
                json!({
                    "llm_model": m,
                    "model_provider": provider,
                    "model_name": model,
                })
            });
            CreateAgentTeamTemplateMemberRequest {
                name: agent.name.clone(),
                responsibility: None,
                system_prompt: agent.system_prompt.clone(),
                decision_style: Some("balanced".to_string()),
                risk_preference: Some("medium".to_string()),
                weight: Some(1.0),
                tool_policy: agent.tool_policy.clone(),
                output_schema,
                sort_order: Some(idx as i32),
            }
        })
        .collect()
}

fn parse_legacy_orchestration_plan(config: Option<&Value>) -> Option<Value> {
    let obj = config?.as_object()?;
    obj.get("orchestration_plan")
        .or_else(|| obj.get("orchestrationPlan"))
        .or_else(|| obj.get("plan"))
        .cloned()
}

fn flatten_step_to_nodes(
    step: &Value,
    incoming_deps: &[String],
    nodes: &mut Vec<TeamTaskNode>,
    generated_ids: &mut HashSet<String>,
    idx_seed: &mut usize,
) -> Result<Vec<String>> {
    let step_obj = step
        .as_object()
        .ok_or_else(|| anyhow!("legacy step must be object"))?;
    let step_type = step_obj
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("agent");
    let step_id = step_obj
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            let id = format!("task-{}", *idx_seed);
            *idx_seed += 1;
            id
        });
    if generated_ids.contains(&step_id) {
        return Err(anyhow!("duplicate task id in legacy plan: {}", step_id));
    }

    match step_type {
        "agent" => {
            generated_ids.insert(step_id.clone());
            let member = step_obj
                .get("member")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let mut assignee_strategy = None;
            if !member.is_empty() {
                assignee_strategy = Some(json!({
                    "mode": "fixed_agent",
                    "agent_name": member,
                }));
            }
            let retry = step_obj.get("retry").and_then(|v| {
                v.as_object().map(|obj| TeamTaskRetryPolicy {
                    max_attempts: obj
                        .get("max_attempts")
                        .and_then(|v| v.as_i64())
                        .map(|n| n as i32),
                    backoff_ms: obj.get("backoff_ms").and_then(|v| v.as_i64()),
                })
            });
            let node = TeamTaskNode {
                id: step_id.clone(),
                title: step_obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&step_id)
                    .to_string(),
                instruction: step_obj
                    .get("instruction")
                    .or_else(|| step_obj.get("prompt"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                depends_on: incoming_deps.to_vec(),
                assignee_strategy,
                retry,
                sla: None,
                input_schema: None,
                output_schema: None,
                phase: step_obj
                    .get("phase")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            };
            nodes.push(node);
            Ok(vec![step_id])
        }
        "serial" => {
            let children = step_obj
                .get("children")
                .and_then(|v| v.as_array())
                .ok_or_else(|| anyhow!("serial step missing children"))?;
            let mut deps = incoming_deps.to_vec();
            let mut latest = deps.clone();
            for child in children {
                latest = flatten_step_to_nodes(child, &deps, nodes, generated_ids, idx_seed)?;
                deps = latest.clone();
            }
            Ok(latest)
        }
        "parallel" => {
            let children = step_obj
                .get("children")
                .and_then(|v| v.as_array())
                .ok_or_else(|| anyhow!("parallel step missing children"))?;
            let mut endings: Vec<String> = Vec::new();
            for child in children {
                let child_endings =
                    flatten_step_to_nodes(child, incoming_deps, nodes, generated_ids, idx_seed)?;
                endings.extend(child_endings);
            }
            endings.sort();
            endings.dedup();
            Ok(endings)
        }
        other => Err(anyhow!("unsupported legacy step type: {}", other)),
    }
}

pub(crate) fn convert_legacy_to_task_graph(
    default_rounds_config: Option<&Value>,
    members: &[CreateAgentTeamTemplateMemberRequest],
) -> TeamTaskGraph {
    let mut nodes: Vec<TeamTaskNode> = Vec::new();
    if let Some(plan) = parse_legacy_orchestration_plan(default_rounds_config) {
        if let Some(steps) = plan.get("steps").and_then(|v| v.as_array()) {
            let mut idx_seed = 1usize;
            let mut generated_ids = HashSet::new();
            let mut deps: Vec<String> = Vec::new();
            for step in steps {
                if let Ok(endings) = flatten_step_to_nodes(
                    step,
                    &deps,
                    &mut nodes,
                    &mut generated_ids,
                    &mut idx_seed,
                ) {
                    deps = endings;
                }
            }
        }
    }

    if nodes.is_empty() {
        let mut prev: Option<String> = None;
        for (idx, member) in members.iter().enumerate() {
            let id = format!("task-{}", idx + 1);
            let mut depends_on = Vec::new();
            if let Some(prev_id) = prev.as_ref() {
                depends_on.push(prev_id.clone());
            }
            nodes.push(TeamTaskNode {
                id: id.clone(),
                title: format!("{} 执行任务", member.name),
                instruction: member
                    .responsibility
                    .clone()
                    .unwrap_or_else(|| "请基于目标完成该节点任务。".to_string()),
                depends_on,
                assignee_strategy: Some(json!({
                    "mode": "fixed_agent",
                    "agent_name": member.name,
                })),
                retry: Some(TeamTaskRetryPolicy {
                    max_attempts: Some(1),
                    backoff_ms: Some(800),
                }),
                sla: None,
                input_schema: None,
                output_schema: None,
                phase: Some("orchestrating".to_string()),
            });
            prev = Some(id);
        }
    }

    TeamTaskGraph {
        version: Some(1),
        nodes,
    }
}

pub(crate) fn build_template_spec_v2(
    req: &CreateAgentTeamTemplateRequest,
) -> Result<TeamTemplateSpecV2> {
    let agents = req.agents.clone();
    if agents.is_empty() {
        return Err(anyhow!("agents cannot be empty for schema v2 template"));
    }

    let task_graph = req.task_graph.clone();
    if task_graph.nodes.is_empty() {
        return Err(anyhow!(
            "task_graph.nodes cannot be empty for schema v2 template"
        ));
    }

    Ok(TeamTemplateSpecV2 {
        schema_version: AGENT_TEAM_SCHEMA_V2,
        agents,
        task_graph,
        hook_policy: req.hook_policy.clone(),
    })
}

pub(crate) fn build_template_spec_v2_from_legacy(
    default_rounds_config: Option<&Value>,
    members: &[CreateAgentTeamTemplateMemberRequest],
    hook_policy: Option<serde_json::Value>,
) -> Result<TeamTemplateSpecV2> {
    let agents = build_agents_from_members(members);
    if agents.is_empty() {
        return Err(anyhow!("template must include at least one agent"));
    }

    let task_graph = convert_legacy_to_task_graph(default_rounds_config, members);
    if task_graph.nodes.is_empty() {
        return Err(anyhow!(
            "task_graph.nodes cannot be empty for schema v2 template"
        ));
    }

    Ok(TeamTemplateSpecV2 {
        schema_version: AGENT_TEAM_SCHEMA_V2,
        agents,
        task_graph,
        hook_policy,
    })
}

// ==================== 模板操作 ====================

pub async fn create_agent_team_template(
    pool: &PgPool,
    req: &CreateAgentTeamTemplateRequest,
    created_by: Option<&str>,
) -> Result<AgentTeamTemplate> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let spec_v2 = build_template_spec_v2(req)?;
    let spec_v2_json = serde_json::to_value(&spec_v2).ok();
    let members_from_agents = make_member_requests_from_agents(&spec_v2.agents);

    sqlx::query(
        r#"INSERT INTO agent_team_templates
           (id, name, description, domain, default_rounds_config, default_tool_policy,
            schema_version, template_spec_v2, upgrade_failed, upgrade_error,
            is_system, created_by, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)"#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.domain)
    .bind(req.default_rounds_config.as_ref().map(|v| v.to_string()))
    .bind(req.default_tool_policy.as_ref().map(|v| v.to_string()))
    .bind(AGENT_TEAM_SCHEMA_V2)
    .bind(spec_v2_json.as_ref().map(|v| v.to_string()))
    .bind(false)
    .bind(Option::<String>::None)
    .bind(false)
    .bind(created_by)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .context("Failed to create agent_team_template")?;

    for (i, member_req) in members_from_agents.iter().enumerate() {
        let member_id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"INSERT INTO agent_team_template_members
               (id, template_id, name, responsibility, system_prompt, decision_style,
                risk_preference, weight, tool_policy, output_schema, sort_order, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#,
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
        .await
        .context("Failed to create agent_team_template_member")?;
    }

    get_agent_team_template_detail(pool, &id)
        .await?
        .context("Template not found after creation")
}

pub async fn update_agent_team_template(
    pool: &PgPool,
    id: &str,
    req: &UpdateAgentTeamTemplateRequest,
) -> Result<()> {
    let now = Utc::now();
    let schema_version = req
        .schema_version
        .unwrap_or(AGENT_TEAM_SCHEMA_V2)
        .max(AGENT_TEAM_SCHEMA_V2);
    let template_spec_v2 = if req.agents.is_some() || req.task_graph.is_some() || req.hook_policy.is_some() {
        let existing_row = sqlx::query("SELECT template_spec_v2 FROM agent_team_templates WHERE id = $1")
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
           SET name = COALESCE($1, name),
               description = COALESCE($2, description),
               domain = COALESCE($3, domain),
               default_rounds_config = COALESCE($4, default_rounds_config),
               default_tool_policy = COALESCE($5, default_tool_policy),
               schema_version = COALESCE($6, schema_version),
               template_spec_v2 = COALESCE($7, template_spec_v2),
               upgrade_failed = FALSE,
               upgrade_error = NULL,
               updated_at = $8
           WHERE id = $9"#,
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
    .await
    .context("Failed to update agent_team_template")?;

    // 若传入 agents，则同步替换模板 Agent 快照
    let members_to_use = req
        .agents
        .as_ref()
        .map(|agents| make_member_requests_from_agents(agents));
    if let Some(members) = members_to_use {
        sqlx::query("DELETE FROM agent_team_template_members WHERE template_id = $1")
            .bind(id)
            .execute(pool)
            .await
            .context("Failed to clear template members")?;

        for (i, member_req) in members.into_iter().enumerate() {
            let member_id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                r#"INSERT INTO agent_team_template_members
                (id, template_id, name, responsibility, system_prompt, decision_style,
                 risk_preference, weight, tool_policy, output_schema, sort_order, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#,
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
            .await
            .context("Failed to insert template member during update")?;
        }
    }

    Ok(())
}

pub async fn delete_agent_team_template(pool: &PgPool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM agent_team_template_members WHERE template_id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM agent_team_templates WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to delete agent_team_template")?;
    Ok(())
}

pub async fn list_agent_team_templates(
    pool: &PgPool,
    domain: Option<&str>,
) -> Result<Vec<AgentTeamTemplate>> {
    let rows = if let Some(domain) = domain {
        sqlx::query(
            r#"SELECT id, name, description, domain, default_rounds_config,
                      default_tool_policy, schema_version, template_spec_v2,
                      upgrade_failed, upgrade_error, is_system, created_by, created_at, updated_at
               FROM agent_team_templates WHERE domain = $1 ORDER BY created_at DESC"#,
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

    rows.into_iter()
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
        .collect()
}

pub async fn get_agent_team_template_detail(
    pool: &PgPool,
    id: &str,
) -> Result<Option<AgentTeamTemplate>> {
    let row_opt = sqlx::query(
        r#"SELECT id, name, description, domain, default_rounds_config,
                  default_tool_policy, schema_version, template_spec_v2,
                  upgrade_failed, upgrade_error, is_system, created_by, created_at, updated_at
           FROM agent_team_templates WHERE id = $1"#,
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

    let member_rows = sqlx::query(
        r#"SELECT id, template_id, name, responsibility, system_prompt, decision_style,
                  risk_preference, weight, tool_policy, output_schema, sort_order,
                  created_at, updated_at
           FROM agent_team_template_members WHERE template_id = $1 ORDER BY sort_order ASC"#,
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    template.members = member_rows
        .into_iter()
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
        .collect::<Result<Vec<_>>>()?;

    Ok(Some(template))
}

// ==================== 会话操作 ====================

pub async fn create_agent_team_session(
    pool: &PgPool,
    req: &CreateAgentTeamSessionRequest,
) -> Result<AgentTeamSession> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let mut runtime_spec_v2 = req.runtime_spec_v2.clone();
    let schema_version = req.schema_version.unwrap_or(AGENT_TEAM_SCHEMA_V2);

    if let Some(template_id) = req.template_id.as_ref() {
        let tpl_row = sqlx::query(
            r#"SELECT schema_version, template_spec_v2, upgrade_failed, upgrade_error
               FROM agent_team_templates WHERE id = $1"#,
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
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)"#,
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
    .await
    .context("Failed to create agent_team_session")?;

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
        .context("Session not found after creation")
}

async fn snapshot_members_from_template(
    pool: &PgPool,
    session_id: &str,
    template_id: &str,
) -> Result<()> {
    let now = Utc::now();
    let template_members = sqlx::query(
        r#"SELECT name, responsibility, system_prompt, decision_style, risk_preference,
                  weight, tool_policy, output_schema, sort_order
           FROM agent_team_template_members WHERE template_id = $1 ORDER BY sort_order"#,
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
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)"#,
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

async fn snapshot_members_from_runtime_spec(
    pool: &PgPool,
    session_id: &str,
    runtime_spec_v2: &Value,
) -> Result<()> {
    let spec: TeamTemplateSpecV2 = serde_json::from_value(runtime_spec_v2.clone())
        .context("invalid runtime_spec_v2 when snapshotting session agents")?;
    if spec.agents.is_empty() {
        return Err(anyhow!("runtime_spec_v2.agents cannot be empty"));
    }
    for (i, agent) in spec.agents.iter().enumerate() {
        create_agent_team_member_from_profile(pool, session_id, agent, i as i32).await?;
    }
    Ok(())
}

fn build_member_output_schema_from_model(model: Option<&str>) -> Option<Value> {
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
    pool: &PgPool,
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
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)"#,
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
    pool: &PgPool,
    id: &str,
    req: &UpdateAgentTeamSessionRequest,
) -> Result<()> {
    let now = Utc::now();
    let schema_version = req.schema_version.map(|v| v.max(AGENT_TEAM_SCHEMA_V2));
    sqlx::query(
        r#"UPDATE agent_team_sessions
           SET name = COALESCE($1, name),
               goal = COALESCE($2, goal),
               schema_version = COALESCE($3, schema_version),
               runtime_spec_v2 = COALESCE($4, runtime_spec_v2),
               state = COALESCE($5, state),
               max_rounds = COALESCE($6, max_rounds),
               state_machine = COALESCE($7, state_machine),
               error_message = COALESCE($8, error_message),
               updated_at = $9
           WHERE id = $10"#,
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

pub async fn delete_agent_team_session(pool: &PgPool, id: &str) -> Result<()> {
    // Child tables are configured with ON DELETE CASCADE, so deleting session is sufficient.
    sqlx::query("DELETE FROM agent_team_sessions WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_session_state(pool: &PgPool, session_id: &str, state: &str) -> Result<()> {
    let now = Utc::now();
    sqlx::query("UPDATE agent_team_sessions SET state = $1, updated_at = $2 WHERE id = $3")
        .bind(state)
        .bind(now)
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_session_blackboard(
    pool: &PgPool,
    session_id: &str,
    blackboard_state: &Value,
) -> Result<()> {
    let now = Utc::now();
    sqlx::query(
        "UPDATE agent_team_sessions SET blackboard_state = $1, updated_at = $2 WHERE id = $3",
    )
    .bind(blackboard_state.to_string())
    .bind(now)
    .bind(session_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_agent_team_session(pool: &PgPool, id: &str) -> Result<Option<AgentTeamSession>> {
    let row_opt = sqlx::query(
        r#"SELECT id, conversation_id, template_id, name, goal, orchestration_plan, schema_version, runtime_spec_v2, plan_version, state, state_machine,
                  current_round, max_rounds, blackboard_state, divergence_scores,
                  total_tokens, estimated_cost, suspended_reason, started_at, completed_at,
                  error_message, created_at, updated_at
           FROM agent_team_sessions WHERE id = $1"#,
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
    pool: &PgPool,
    conversation_id: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<AgentTeamSession>> {
    let rows = if let Some(conv_id) = conversation_id {
        sqlx::query(
            r#"SELECT id
               FROM agent_team_sessions
               WHERE conversation_id = $1
               ORDER BY updated_at DESC
               LIMIT $2 OFFSET $3"#,
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
               LIMIT $1 OFFSET $2"#,
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
    pool: &PgPool,
    session_id: &str,
) -> Result<Vec<AgentTeamMember>> {
    let rows = sqlx::query(
        r#"SELECT id, session_id, name, responsibility, system_prompt, decision_style,
                  risk_preference, weight, tool_policy, output_schema, sort_order,
                  token_usage, tool_calls_count, is_active, created_at, updated_at
           FROM agent_team_members WHERE session_id = $1 AND is_active = true ORDER BY sort_order"#,
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
    pool: &PgPool,
    session_id: &str,
    round_number: i32,
    phase: &str,
) -> Result<AgentTeamRound> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        r#"INSERT INTO agent_team_rounds (id, session_id, round_number, phase, status, started_at, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
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

    sqlx::query("UPDATE agent_team_sessions SET current_round = $1, updated_at = $2 WHERE id = $3")
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
    pool: &PgPool,
    round_id: &str,
    divergence_score: Option<f64>,
) -> Result<()> {
    let now = Utc::now();
    sqlx::query(
        "UPDATE agent_team_rounds SET status = 'completed', completed_at = $1, divergence_score = $2 WHERE id = $3",
    )
    .bind(now)
    .bind(divergence_score)
    .bind(round_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_rounds(pool: &PgPool, session_id: &str) -> Result<Vec<AgentTeamRound>> {
    let rows = sqlx::query(
        r#"SELECT id, session_id, round_number, phase, status, divergence_score,
                  started_at, completed_at, created_at
           FROM agent_team_rounds
           WHERE session_id = $1
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
    pool: &PgPool,
    session_id: &str,
    round_id: Option<&str>,
    member_id: Option<&str>,
    member_name: Option<&str>,
    role: &str,
    content: &str,
    token_count: Option<i32>,
) -> Result<AgentTeamMessage> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        r#"INSERT INTO agent_team_messages
           (id, session_id, round_id, member_id, member_name, role, content, token_count, timestamp)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
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
            "UPDATE agent_team_members SET token_usage = token_usage + $1, updated_at = $2 WHERE id = $3",
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
    pool: &PgPool,
    message_id: &str,
    tool_calls: &serde_json::Value,
) -> Result<()> {
    sqlx::query(
        r#"UPDATE agent_team_messages
           SET tool_calls = $1
           WHERE id = $2"#,
    )
    .bind(tool_calls.to_string())
    .bind(message_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_messages(pool: &PgPool, session_id: &str) -> Result<Vec<AgentTeamMessage>> {
    let rows = sqlx::query(
        r#"SELECT id, session_id, round_id, member_id, member_name, role, content,
                  tool_calls, token_count, timestamp
           FROM agent_team_messages WHERE session_id = $1 ORDER BY timestamp ASC"#,
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
    pool: &PgPool,
    req: &UpdateBlackboardRequest,
) -> Result<AgentTeamBlackboardEntry> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        r#"INSERT INTO agent_team_blackboard_entries
           (id, session_id, round_id, entry_type, title, content, contributed_by, is_resolved, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"#,
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
    pool: &PgPool,
    session_id: &str,
) -> Result<Vec<AgentTeamBlackboardEntry>> {
    let rows = sqlx::query(
        r#"SELECT id, session_id, round_id, entry_type, title, content,
                  contributed_by, is_resolved, created_at, updated_at
           FROM agent_team_blackboard_entries WHERE session_id = $1 ORDER BY created_at ASC"#,
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
    pool: &PgPool,
    session_id: &str,
    entry_id: &str,
) -> Result<AgentTeamBlackboardEntry> {
    let now = Utc::now();
    let row = sqlx::query(
        r#"UPDATE agent_team_blackboard_entries
           SET is_resolved = TRUE, updated_at = $1
           WHERE session_id = $2 AND id = $3
           RETURNING id, session_id, round_id, entry_type, title, content,
                     contributed_by, is_resolved, created_at, updated_at"#,
    )
    .bind(now)
    .bind(session_id)
    .bind(entry_id)
    .fetch_one(pool)
    .await
    .context("Failed to resolve blackboard entry")?;

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
    pool: &PgPool,
    session_id: &str,
    entry_id: &str,
    limit: i64,
) -> Result<AgentTeamBlackboardArchive> {
    let safe_limit = limit.clamp(10, 400);
    let entry_row = sqlx::query(
        r#"SELECT id, session_id, round_id, entry_type, title, content,
                  contributed_by, is_resolved, created_at, updated_at
           FROM agent_team_blackboard_entries
           WHERE session_id = $1 AND id = $2"#,
    )
    .bind(session_id)
    .bind(entry_id)
    .fetch_one(pool)
    .await
    .context("Failed to fetch blackboard entry for archive")?;

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
               WHERE session_id = $1 AND round_id = $2
               ORDER BY timestamp ASC
               LIMIT $3"#,
        )
        .bind(session_id)
        .bind(round_id)
        .bind(safe_limit)
        .fetch_all(pool)
        .await
        .context("Failed to fetch round messages for blackboard archive")?;

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
                   WHERE session_id = $1
                   ORDER BY timestamp DESC
                   LIMIT $2
               ) recent
               ORDER BY timestamp ASC"#,
        )
        .bind(session_id)
        .bind(safe_limit)
        .fetch_all(pool)
        .await
        .context("Failed to fetch session messages for blackboard archive")?;

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
    pool: &PgPool,
    session_id: &str,
    artifact_type: &str,
    title: &str,
    content: &str,
    created_by: Option<&str>,
    parent_artifact_id: Option<&str>,
    diff_summary: Option<&str>,
) -> Result<AgentTeamArtifact> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    // Get current version number using runtime query
    let version_row = sqlx::query(
        "SELECT COALESCE(MAX(version), 0) as max_version FROM agent_team_artifacts WHERE session_id = $1 AND artifact_type = $2",
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
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#,
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

pub async fn list_artifacts(pool: &PgPool, session_id: &str) -> Result<Vec<AgentTeamArtifact>> {
    let rows = sqlx::query(
        r#"SELECT id, session_id, artifact_type, title, content, version,
                  parent_artifact_id, diff_summary, created_by, created_at, updated_at
           FROM agent_team_artifacts WHERE session_id = $1 ORDER BY created_at DESC"#,
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
    pool: &PgPool,
    artifact_id: &str,
) -> Result<Option<AgentTeamArtifact>> {
    let row_opt = sqlx::query(
        r#"SELECT id, session_id, artifact_type, title, content, version,
                  parent_artifact_id, diff_summary, created_by, created_at, updated_at
           FROM agent_team_artifacts WHERE id = $1"#,
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

// ==================== V2 Task / Mailbox ====================

pub async fn ensure_session_tasks_from_runtime_spec(
    pool: &PgPool,
    session_id: &str,
    runtime_spec_v2: &Value,
) -> Result<()> {
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
            "SELECT COUNT(*) FROM agent_team_tasks WHERE session_id = $1 AND task_id = $2",
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
               VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7, 0, $8, $9, $10)"#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(session_id)
        .bind(task_id)
        .bind(title)
        .bind(instruction)
        .bind(assignee_agent_id)
        .bind(Value::Array(depends_on).to_string())
        .bind(max_attempts)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn list_tasks(pool: &PgPool, session_id: &str) -> Result<Vec<TeamTask>> {
    let rows = sqlx::query(
        r#"SELECT id, session_id, task_id, title, instruction, status, assignee_agent_id, depends_on,
                  attempt, max_attempts, last_error, started_at, completed_at, created_at, updated_at
           FROM agent_team_tasks
           WHERE session_id = $1
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

pub async fn update_task(pool: &PgPool, session_id: &str, patch: &UpdateTaskRequest) -> Result<()> {
    let now = Utc::now();
    sqlx::query(
        r#"UPDATE agent_team_tasks
           SET status = COALESCE($1, status),
               assignee_agent_id = COALESCE($2, assignee_agent_id),
               last_error = CASE
                   WHEN $3 IS NOT NULL THEN $3
                   WHEN LOWER(COALESCE($1, '')) IN ('running', 'completed') THEN NULL
                   ELSE last_error
               END,
               attempt = CASE
                   WHEN LOWER(COALESCE($1, '')) = 'running' THEN attempt + 1
                   ELSE attempt
               END,
               started_at = CASE
                   WHEN LOWER(COALESCE($1, '')) = 'running' THEN COALESCE(started_at, $4)
                   ELSE started_at
               END,
               completed_at = CASE
                   WHEN LOWER(COALESCE($1, '')) = 'running' THEN NULL
                   WHEN LOWER(COALESCE($1, '')) IN ('completed', 'failed', 'cancelled', 'blocked') THEN $4
                   ELSE completed_at
               END,
               updated_at = $4
           WHERE session_id = $5 AND task_id = $6"#,
    )
    .bind(&patch.status)
    .bind(&patch.assignee_agent_id)
    .bind(&patch.last_error)
    .bind(now)
    .bind(session_id)
    .bind(&patch.task_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_mailbox(
    pool: &PgPool,
    session_id: &str,
    agent_id: Option<&str>,
) -> Result<Vec<MailboxMessage>> {
    let rows = if let Some(agent_id) = agent_id {
        sqlx::query(
            r#"SELECT id, session_id, from_agent_id, to_agent_id, task_record_id, message_type, payload, is_acknowledged, created_at, acknowledged_at
               FROM agent_team_mailbox
               WHERE session_id = $1 AND (to_agent_id = $2 OR to_agent_id IS NULL)
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
               WHERE session_id = $1
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
                    .unwrap_or_else(|| json!({})),
                is_acknowledged: row.try_get::<bool, _>("is_acknowledged").unwrap_or(false),
                created_at: row.try_get("created_at")?,
                acknowledged_at: row.try_get("acknowledged_at")?,
            })
        })
        .collect()
}

pub async fn ack_mailbox_message(pool: &PgPool, message_id: &str) -> Result<()> {
    sqlx::query(
        r#"UPDATE agent_team_mailbox
           SET is_acknowledged = TRUE,
               acknowledged_at = $1
           WHERE id = $2"#,
    )
    .bind(Utc::now())
    .bind(message_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn create_mailbox_message(
    pool: &PgPool,
    session_id: &str,
    from_agent_id: Option<&str>,
    to_agent_id: Option<&str>,
    task_record_id: Option<&str>,
    message_type: &str,
    payload: &Value,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO agent_team_mailbox
           (id, session_id, from_agent_id, to_agent_id, task_record_id, message_type, payload, is_acknowledged, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, FALSE, $8)"#,
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
    pool: &PgPool,
    session_id: &str,
    task_record_id: &str,
    attempt: i32,
    status: &str,
    error: Option<&str>,
    duration_ms: Option<i64>,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO agent_team_task_attempts
           (id, session_id, task_record_id, attempt, status, error, duration_ms, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
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
    pool: &PgPool,
    session_id: &str,
    task_record_id: Option<&str>,
    event_type: &str,
    payload: &Value,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO agent_team_task_events
           (id, session_id, task_record_id, event_type, payload, created_at)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
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

pub async fn upgrade_templates_to_v2(pool: &PgPool, force: bool) -> Result<i64> {
    let rows = sqlx::query(
        r#"SELECT id, name, domain, default_rounds_config, default_tool_policy
           FROM agent_team_templates
           WHERE schema_version < $1
              OR schema_version IS NULL
              OR (upgrade_failed = TRUE AND $2 = TRUE)"#,
    )
    .bind(AGENT_TEAM_SCHEMA_V2)
    .bind(force)
    .fetch_all(pool)
    .await?;

    let mut upgraded = 0i64;
    for row in rows {
        let template_id: String = row.try_get("id")?;
        let default_rounds_config = row
            .try_get::<Option<String>, _>("default_rounds_config")?
            .and_then(|s| serde_json::from_str::<Value>(&s).ok());
        let members = sqlx::query(
            r#"SELECT name, responsibility, system_prompt, decision_style, risk_preference, weight, tool_policy, output_schema, sort_order
               FROM agent_team_template_members
               WHERE template_id = $1
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
                       SET schema_version = $1,
                           template_spec_v2 = $2,
                           upgrade_failed = FALSE,
                           upgrade_error = NULL,
                           updated_at = $3
                       WHERE id = $4"#,
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
                       SET upgrade_failed = TRUE,
                           upgrade_error = $1,
                           updated_at = $2
                       WHERE id = $3"#,
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

// ==================== 种子数据（内置模板） ====================

pub async fn seed_builtin_templates(pool: &PgPool) -> Result<()> {
    let count_row =
        sqlx::query("SELECT COUNT(*) as cnt FROM agent_team_templates WHERE is_system = true")
            .fetch_one(pool)
            .await?;
    let count: i64 = count_row.try_get::<i64, _>("cnt").unwrap_or(0);

    if count > 0 {
        info!("Built-in agent team templates already exist, skipping seed");
        return Ok(());
    }

    info!("Seeding built-in agent team templates...");

    let templates = builtin_templates_seed();
    for template_req in templates {
        let id = template_req.id.to_string();
        let now = Utc::now();
        let legacy_req = CreateAgentTeamTemplateRequest {
            name: template_req.name.to_string(),
            description: Some(template_req.description.to_string()),
            domain: template_req.domain.to_string(),
            default_rounds_config: None,
            default_tool_policy: None,
            schema_version: Some(AGENT_TEAM_SCHEMA_V2),
            agents: build_agents_from_members(&template_req.members),
            task_graph: convert_legacy_to_task_graph(None, &template_req.members),
            hook_policy: None,
        };
        let spec_v2 = build_template_spec_v2(&legacy_req).ok();

        sqlx::query(
            r#"INSERT INTO agent_team_templates
               (id, name, description, domain, schema_version, template_spec_v2, upgrade_failed, upgrade_error, is_system, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
               ON CONFLICT (id) DO NOTHING"#,
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
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#,
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

    info!("Built-in agent team templates seeded successfully");
    Ok(())
}

pub(crate) struct BuiltinTemplateSeed {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub domain: &'static str,
    pub members: Vec<CreateAgentTeamTemplateMemberRequest>,
}

pub(crate) fn builtin_templates_seed() -> Vec<BuiltinTemplateSeed> {
    vec![BuiltinTemplateSeed {
        id: "builtin-product-dev-team",
        name: "产品开发团队（4-Agent）",
        description: "覆盖产品、架构、后端、测试四个核心 Agent 的默认团队",
        domain: "product",
        members: vec![
            CreateAgentTeamTemplateMemberRequest {
                name: "产品经理".to_string(),
                responsibility: Some("负责需求分析、产品路线图和功能优先级".to_string()),
                system_prompt: Some("你是一位资深产品经理，专注于用户价值和商业目标，以 PRD 格式输出需求。".to_string()),
                decision_style: Some("user-centric".to_string()),
                risk_preference: Some("balanced".to_string()),
                weight: Some(1.2),
                tool_policy: None,
                output_schema: None,
                sort_order: Some(0),
            },
            CreateAgentTeamTemplateMemberRequest {
                name: "架构师".to_string(),
                responsibility: Some("负责系统架构设计、技术选型和非功能需求".to_string()),
                system_prompt: Some("你是一位系统架构师，从可扩展性、可维护性、安全性角度审视方案，输出架构设计文档。".to_string()),
                decision_style: Some("technical".to_string()),
                risk_preference: Some("conservative".to_string()),
                weight: Some(1.5),
                tool_policy: None,
                output_schema: None,
                sort_order: Some(1),
            },
            CreateAgentTeamTemplateMemberRequest {
                name: "后端开发".to_string(),
                responsibility: Some("负责服务端实现细节、API 设计和数据模型".to_string()),
                system_prompt: Some("你是一位后端开发工程师，关注实现可行性、性能和代码质量，给出具体的技术实现建议。".to_string()),
                decision_style: Some("pragmatic".to_string()),
                risk_preference: Some("medium".to_string()),
                weight: Some(1.0),
                tool_policy: None,
                output_schema: None,
                sort_order: Some(2),
            },
            CreateAgentTeamTemplateMemberRequest {
                name: "QA工程师".to_string(),
                responsibility: Some("负责测试策略、质量保障和风险识别".to_string()),
                system_prompt: Some("你是一位质量保障工程师，从测试覆盖、边界条件和风险角度评审方案，提出质量改进建议。".to_string()),
                decision_style: Some("risk-aware".to_string()),
                risk_preference: Some("conservative".to_string()),
                weight: Some(0.8),
                tool_policy: None,
                output_schema: None,
                sort_order: Some(3),
            },
        ],
    }]
}

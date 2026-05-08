use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use chrono::Utc;
use sentinel_db::database_service::connection_manager::DatabasePool;
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::agents::executor::{execute_agent as execute_team_agent, AgentExecuteParams};
use crate::agents::tool_router::ToolConfig;
use crate::agents::ContextPolicy;
use crate::commands::team_v3_blackboard_context::{
    build_blackboard_context, truncate_chars, TeamV3PromptQuery,
};
use crate::commands::team_v3_memory::backfill_team_v3_structured_memory_from_checkpoints;
use crate::commands::team_v3_prompting::{
    build_team_v3_planner_prompt, build_team_v3_planner_system_prompt,
};

use super::team_v3_commands::{
    append_team_v3_blackboard_entry, collapse_whitespace, list_team_v3_blackboard_entries,
    TeamV3MemberProfile, TEAM_V3_STRUCTURED_BACKFILL_SCAN_LIMIT,
};
use super::team_v3_session_state::{
    append_team_v3_status_message, build_team_state_data, build_team_state_data_with_members,
    ensure_team_v3_execution_tasks, extract_json_candidate, normalize_team_member_values,
    normalize_team_members, set_team_v3_session_state_data, team_member_ids,
};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TeamV3PlannedAgent {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) responsibility: Option<String>,
    #[serde(default)]
    pub(crate) system_prompt: Option<String>,
    #[serde(default)]
    pub(crate) decision_style: Option<String>,
    #[serde(default)]
    pub(crate) risk_preference: Option<String>,
    #[serde(default)]
    pub(crate) weight: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TeamV3PlannedTask {
    pub(crate) task_key: String,
    pub(crate) title: String,
    pub(crate) instruction: String,
    #[serde(default)]
    pub(crate) depends_on: Vec<String>,
    #[serde(default)]
    pub(crate) owner_agent_id: Option<String>,
    #[serde(default)]
    pub(crate) priority: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TeamV3ExecutionPlan {
    #[serde(default)]
    pub(crate) summary: Option<String>,
    #[serde(default)]
    pub(crate) agents: Vec<TeamV3PlannedAgent>,
    pub(crate) tasks: Vec<TeamV3PlannedTask>,
}

fn normalize_agent_id(input: &str, fallback_index: usize) -> String {
    let mut normalized = input
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    while normalized.contains("--") {
        normalized = normalized.replace("--", "-");
    }
    normalized = normalized.trim_matches('-').to_string();
    if normalized.is_empty() {
        format!("agent-{}", fallback_index + 1)
    } else {
        normalized
    }
}

fn normalize_task_key(input: &str, fallback_index: usize) -> String {
    let mut normalized = input
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    while normalized.contains("--") {
        normalized = normalized.replace("--", "-");
    }
    normalized = normalized.trim_matches('-').to_string();
    if normalized.is_empty() {
        format!("task-{}", fallback_index + 1)
    } else {
        normalized
    }
}

fn derive_plan_members(plan: &TeamV3ExecutionPlan) -> Vec<Value> {
    if !plan.agents.is_empty() {
        let members = plan
            .agents
            .iter()
            .enumerate()
            .map(|(index, agent)| {
                let id = normalize_agent_id(agent.id.as_str(), index);
                let name = if agent.name.trim().is_empty() {
                    format!("Agent {}", index + 1)
                } else {
                    agent.name.trim().to_string()
                };
                let mut member = json!({
                    "id": id,
                    "name": name,
                    "responsibility": agent
                        .responsibility
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .unwrap_or("负责执行分配任务并输出可复用结论"),
                    "sort_order": index as i64,
                    "weight": agent.weight.unwrap_or(1.0),
                    "token_usage": 0,
                    "tool_calls_count": 0,
                    "is_active": false
                });
                if let Some(obj) = member.as_object_mut() {
                    if let Some(system_prompt) = agent
                        .system_prompt
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                    {
                        obj.insert("system_prompt".to_string(), json!(system_prompt));
                    }
                    if let Some(decision_style) = agent
                        .decision_style
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                    {
                        obj.insert("decision_style".to_string(), json!(decision_style));
                    }
                    if let Some(risk_preference) = agent
                        .risk_preference
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                    {
                        obj.insert("risk_preference".to_string(), json!(risk_preference));
                    }
                }
                member
            })
            .collect::<Vec<_>>();
        return normalize_team_member_values(members);
    }

    let mut owners = plan
        .tasks
        .iter()
        .filter_map(|task| {
            task.owner_agent_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
        .collect::<Vec<_>>();
    owners.sort();
    owners.dedup();
    if owners.is_empty() {
        return normalize_team_member_values(Vec::new());
    }
    let members = owners
        .into_iter()
        .enumerate()
        .map(|(index, owner)| {
            let id = normalize_agent_id(owner.as_str(), index);
            json!({
                "id": id,
                "name": format!("Agent {}", index + 1),
                "responsibility": "负责执行分配任务并输出可复用结论",
                "sort_order": index as i64,
                "weight": 1.0,
                "token_usage": 0,
                "tool_calls_count": 0,
                "is_active": false
            })
        })
        .collect::<Vec<_>>();
    normalize_team_member_values(members)
}

pub(crate) fn team_member_catalog_lines(state_data: &Value) -> Vec<String> {
    state_data
        .get("members")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|member| {
            let id = member
                .get("id")
                .and_then(|v| v.as_str())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())?;
            let name = member
                .get("name")
                .and_then(|v| v.as_str())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| id.clone());
            let responsibility = member
                .get("responsibility")
                .and_then(|v| v.as_str())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "负责通用问题求解".to_string());
            Some(format!("- {} ({})：{}", id, name, responsibility))
        })
        .collect()
}

pub(crate) fn team_member_profiles(state_data: &Value) -> HashMap<String, TeamV3MemberProfile> {
    normalize_team_members(state_data)
        .into_iter()
        .filter_map(|member| {
            let id = member
                .get("id")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())?;
            let name = member
                .get("name")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| id.clone());
            let responsibility = member
                .get("responsibility")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let system_prompt = member
                .get("system_prompt")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let decision_style = member
                .get("decision_style")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let risk_preference = member
                .get("risk_preference")
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            Some((
                id,
                TeamV3MemberProfile {
                    name,
                    responsibility,
                    system_prompt,
                    decision_style,
                    risk_preference,
                },
            ))
        })
        .collect()
}

fn canonical_agent_id(input: &str) -> String {
    input
        .trim()
        .to_lowercase()
        .replace('_', "-")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

pub(crate) fn resolve_member_id(
    owner_candidate: Option<&str>,
    members: &[String],
    index: usize,
) -> String {
    let fallback = members
        .get(index % members.len().max(1))
        .cloned()
        .unwrap_or_else(|| "agent-1".to_string());

    let Some(owner_raw) = owner_candidate
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return fallback;
    };

    if members.iter().any(|member| member == owner_raw) {
        return owner_raw.to_string();
    }

    let canonical_owner = canonical_agent_id(owner_raw);
    if !canonical_owner.is_empty() {
        if let Some(matched) = members
            .iter()
            .find(|member| canonical_agent_id(member.as_str()) == canonical_owner)
        {
            return matched.clone();
        }
    }

    let normalized_owner = normalize_agent_id(owner_raw, index);
    if let Some(matched) = members
        .iter()
        .find(|member| member.as_str() == normalized_owner.as_str())
    {
        return matched.clone();
    }

    fallback
}

pub(crate) fn select_team_member_for_task(
    task: &super::team_v3_commands::TeamV3Task,
    members: &[String],
    index: usize,
) -> String {
    resolve_member_id(task.owner_agent_id.as_deref(), members, index)
}

pub(crate) fn parse_execution_plan(raw: &str) -> Option<TeamV3ExecutionPlan> {
    let candidate = extract_json_candidate(raw)?;
    serde_json::from_str::<TeamV3ExecutionPlan>(&candidate).ok()
}

fn validate_execution_plan(plan: &TeamV3ExecutionPlan) -> bool {
    if plan.tasks.is_empty() || plan.tasks.len() > 12 || plan.agents.len() > 12 {
        return false;
    }
    let mut raw_agent_ids = HashSet::new();
    let mut normalized_agent_ids = HashSet::new();
    for (index, agent) in plan.agents.iter().enumerate() {
        let raw_id = agent.id.trim();
        if raw_id.is_empty() {
            return false;
        }
        if !raw_agent_ids.insert(raw_id.to_string()) {
            return false;
        }
        let normalized_id = normalize_agent_id(raw_id, index);
        if !normalized_agent_ids.insert(normalized_id) {
            return false;
        }
    }

    let mut keys = HashSet::new();
    for (index, task) in plan.tasks.iter().enumerate() {
        let task_key = normalize_task_key(task.task_key.as_str(), index);
        if task_key.trim().is_empty() {
            return false;
        }
        if !keys.insert(task_key) {
            return false;
        }
        if task.title.trim().is_empty() || task.instruction.trim().is_empty() {
            return false;
        }
        if !plan.agents.is_empty() {
            let Some(owner_agent_id) = task
                .owner_agent_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                return false;
            };
            let owner_exists_in_agents = raw_agent_ids.contains(owner_agent_id)
                || normalized_agent_ids.contains(&normalize_agent_id(owner_agent_id, index));
            if !owner_exists_in_agents {
                return false;
            }
        }
    }

    for (index, task) in plan.tasks.iter().enumerate() {
        let task_key = normalize_task_key(task.task_key.as_str(), index);
        for dependency in task.depends_on.iter() {
            let dependency_key = normalize_task_key(dependency.as_str(), 0);
            if dependency_key == task_key || !keys.contains(&dependency_key) {
                return false;
            }
        }
    }
    true
}

async fn replace_team_v3_tasks_with_plan(
    runtime_pool: &DatabasePool,
    session_id: &str,
    plan: &TeamV3ExecutionPlan,
    members: &[String],
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"DELETE FROM team_v3_tasks
                   WHERE session_id = ?"#,
            )
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"DELETE FROM team_v3_tasks
                   WHERE session_id = $1"#,
            )
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }

    let mut seen_task_keys = HashSet::new();
    let mut normalized_keys: Vec<String> = Vec::with_capacity(plan.tasks.len());
    for (index, raw_task) in plan.tasks.iter().enumerate() {
        let mut task_key = normalize_task_key(raw_task.task_key.as_str(), index);
        if seen_task_keys.contains(&task_key) {
            task_key = format!("{}-{}", task_key, index + 1);
        }
        seen_task_keys.insert(task_key.clone());
        normalized_keys.push(task_key);
    }
    let normalized_key_set = normalized_keys.iter().cloned().collect::<HashSet<String>>();

    for (index, raw_task) in plan.tasks.iter().enumerate() {
        let task_key = normalized_keys
            .get(index)
            .cloned()
            .unwrap_or_else(|| normalize_task_key(raw_task.task_key.as_str(), index));
        let owner_agent_id = Some(resolve_member_id(
            raw_task.owner_agent_id.as_deref(),
            members,
            index,
        ));
        let depends_on = raw_task
            .depends_on
            .iter()
            .map(|value| normalize_task_key(value, 0))
            .filter(|value| normalized_key_set.contains(value))
            .collect::<Vec<_>>();
        let metadata = json!({
            "team_generated": true,
            "planned_by": "main_agent",
            "depends_on": depends_on,
            "attempt": 0,
            "max_attempts": 1
        });
        let priority = raw_task.priority.unwrap_or(((index as i32) + 1) * 10);
        match runtime_pool {
            DatabasePool::SQLite(pool) => {
                sqlx::query(
                    r#"INSERT INTO team_v3_tasks
                       (id, session_id, task_key, title, instruction, status, priority,
                        owner_agent_id, claimed_by_agent_id, claim_expires_at, metadata, created_at, updated_at)
                       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                )
                .bind(Uuid::new_v4().to_string())
                .bind(session_id)
                .bind(task_key)
                .bind(raw_task.title.trim())
                .bind(raw_task.instruction.trim())
                .bind("pending")
                .bind(priority)
                .bind(owner_agent_id)
                .bind(Option::<String>::None)
                .bind(Option::<String>::None)
                .bind(serde_json::to_string(&metadata)?)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await?;
            }
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(
                    r#"INSERT INTO team_v3_tasks
                       (id, session_id, task_key, title, instruction, status, priority,
                        owner_agent_id, claimed_by_agent_id, claim_expires_at, metadata, created_at, updated_at)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::jsonb, $12, $13)"#,
                )
                .bind(Uuid::new_v4().to_string())
                .bind(session_id)
                .bind(task_key)
                .bind(raw_task.title.trim())
                .bind(raw_task.instruction.trim())
                .bind("pending")
                .bind(priority)
                .bind(owner_agent_id)
                .bind(Option::<String>::None)
                .bind(Option::<String>::None)
                .bind(serde_json::to_string(&metadata)?)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await?;
            }
            DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
        }
    }

    Ok(())
}

async fn generate_team_v3_execution_plan_with_main_agent(
    app_handle: &AppHandle,
    provider_config: &crate::services::AiConfig,
    rig_provider: &str,
    model: &str,
    session_id: &str,
    main_agent_id: &str,
    goal_text: &str,
    user_input: &str,
    member_catalog: &[String],
    blackboard_context: &str,
    cancellation_token: &CancellationToken,
    tool_config: &ToolConfig,
) -> Result<Option<TeamV3ExecutionPlan>> {
    let planner_prompt =
        build_team_v3_planner_prompt(goal_text, user_input, member_catalog, blackboard_context);
    let execution_id = format!("team-v3-planner:{}:{}", session_id, Uuid::new_v4());
    let planner_params = AgentExecuteParams {
        execution_id,
        conversation_id: None,
        cancellation_generation: None,
        model: model.to_string(),
        system_prompt: build_team_v3_planner_system_prompt(main_agent_id),
        task: planner_prompt,
        active_browser_shell_direct_write_enabled: false,
        active_browser_shell_session_id: None,
        active_terminal_session_fingerprint: None,
        active_terminal_session_id: None,
        working_directory: None,
        provider_config_key: provider_config.provider.clone(),
        rig_provider: rig_provider.to_string(),
        api_key: provider_config.api_key.clone(),
        api_base: provider_config.api_base.clone(),
        max_iterations: 8,
        timeout_secs: 180,
        tool_config: Some(tool_config.clone()),
        enable_tenth_man_rule: false,
        tenth_man_config: None,
        document_attachments: None,
        image_attachments: None,
        referenced_traffic: None,
        persist_messages: false,
        subagent_run_id: None,
        harness_run_id: None,
        context_policy: Some(ContextPolicy {
            include_working_dir: false,
            include_context_storage: false,
            include_task_mainline: false,
            include_run_state: false,
            include_skill_instructions: false,
            include_stuck_resolution_rule: false,
            ..ContextPolicy::default()
        }),
        context_engine_mode: Some(crate::agents::ContextEngineMode::CodexLike),
        recursion_depth: 0,
    };
    let planner_output = tokio::select! {
        _ = cancellation_token.cancelled() => Err(anyhow!("Team execution cancelled")),
        result = execute_team_agent(app_handle, planner_params) => result,
    }?;
    let Some(plan) = parse_execution_plan(&planner_output) else {
        let preview = truncate_chars(collapse_whitespace(planner_output.as_str()).as_str(), 320);
        tracing::warn!(
            "Team V3 planner output is not valid JSON-only payload (session={}): {}",
            session_id,
            preview
        );
        return Ok(None);
    };
    if !validate_execution_plan(&plan) {
        return Ok(None);
    }
    Ok(Some(plan))
}

pub(crate) async fn prepare_team_v3_execution_tasks_with_main_agent(
    runtime_pool: &DatabasePool,
    app_handle: &AppHandle,
    session_id: &str,
    goal_text: &str,
    user_input: &str,
    state_data: &Value,
    provider_config: &crate::services::AiConfig,
    rig_provider: &str,
    model: &str,
    cancellation_token: &CancellationToken,
    tool_config: &ToolConfig,
) -> Result<(Vec<String>, Value)> {
    let members = team_member_ids(state_data);
    let main_agent_id = members
        .first()
        .cloned()
        .unwrap_or_else(|| "agent-1".to_string());
    let goal_meta = json!({ "goal": goal_text });
    append_team_v3_blackboard_entry(
        runtime_pool,
        session_id,
        None,
        Some("human"),
        "goal",
        user_input,
        Some(&goal_meta),
    )
    .await?;
    let backfilled = backfill_team_v3_structured_memory_from_checkpoints(
        runtime_pool,
        session_id,
        TEAM_V3_STRUCTURED_BACKFILL_SCAN_LIMIT,
    )
    .await?;
    if backfilled > 0 {
        tracing::info!(
            "Team V3 backfilled structured memory from historical checkpoints: session={} appended={}",
            session_id,
            backfilled
        );
    }
    let historical_board = list_team_v3_blackboard_entries(runtime_pool, session_id, 200).await?;
    let planner_query = TeamV3PromptQuery::for_planner(goal_text, user_input);
    let blackboard_context_result = build_blackboard_context(&historical_board, &planner_query);
    tracing::info!(
        "Team V3 planner blackboard context: session={} mode={} terms={} chars={} structured={}/{} task_outputs={}/{} artifacts={}/{} events={}/{} checkpoints={}/{} dropped_total={}",
        session_id,
        blackboard_context_result.diagnostics.mode,
        blackboard_context_result.diagnostics.query_terms,
        blackboard_context_result.diagnostics.context_chars,
        blackboard_context_result
            .diagnostics
            .structured_memory
            .selected,
        blackboard_context_result
            .diagnostics
            .structured_memory
            .total,
        blackboard_context_result.diagnostics.task_outputs.selected,
        blackboard_context_result.diagnostics.task_outputs.total,
        blackboard_context_result.diagnostics.artifacts.selected,
        blackboard_context_result.diagnostics.artifacts.total,
        blackboard_context_result.diagnostics.raw_events.selected,
        blackboard_context_result.diagnostics.raw_events.total,
        blackboard_context_result.diagnostics.checkpoints.selected,
        blackboard_context_result.diagnostics.checkpoints.total,
        blackboard_context_result
            .diagnostics
            .structured_memory
            .dropped()
            + blackboard_context_result.diagnostics.task_outputs.dropped()
            + blackboard_context_result.diagnostics.artifacts.dropped()
            + blackboard_context_result.diagnostics.raw_events.dropped()
            + blackboard_context_result.diagnostics.checkpoints.dropped(),
    );
    let blackboard_context = blackboard_context_result.context;
    let member_catalog = team_member_catalog_lines(state_data);

    let plan_opt = generate_team_v3_execution_plan_with_main_agent(
        app_handle,
        provider_config,
        rig_provider,
        model,
        session_id,
        main_agent_id.as_str(),
        goal_text,
        user_input,
        &member_catalog,
        blackboard_context.as_str(),
        cancellation_token,
        tool_config,
    )
    .await?;

    if let Some(plan) = plan_opt {
        let planned_members = derive_plan_members(&plan);
        let lead_member_id = planned_members
            .first()
            .and_then(|member| member.get("id"))
            .and_then(Value::as_str)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| main_agent_id.clone());
        let next_state_data = build_team_state_data_with_members(
            Some(state_data),
            planned_members,
            Some(lead_member_id.as_str()),
        );
        let now = Utc::now().to_rfc3339();
        set_team_v3_session_state_data(runtime_pool, session_id, &next_state_data, &now).await?;
        let member_ids = team_member_ids(&next_state_data);
        replace_team_v3_tasks_with_plan(runtime_pool, session_id, &plan, &member_ids).await?;
        let summary = plan
            .summary
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("主 agent 已完成任务拆解。");
        let plan_meta = json!({
            "task_count": plan.tasks.len(),
            "tasks": plan.tasks.iter().map(|task| task.task_key.clone()).collect::<Vec<_>>()
        });
        append_team_v3_blackboard_entry(
            runtime_pool,
            session_id,
            None,
            Some(lead_member_id.as_str()),
            "plan",
            summary,
            Some(&plan_meta),
        )
        .await?;
        return Ok((member_ids, next_state_data));
    }

    let fallback_state_data = build_team_state_data(Some(state_data), Some(main_agent_id.as_str()));
    let now = Utc::now().to_rfc3339();
    set_team_v3_session_state_data(runtime_pool, session_id, &fallback_state_data, &now).await?;
    ensure_team_v3_execution_tasks(
        runtime_pool,
        session_id,
        Some(user_input),
        &fallback_state_data,
    )
    .await?;
    append_team_v3_status_message(
        runtime_pool,
        session_id,
        "主 agent 拆解失败，已回退到默认任务拆解。",
    )
    .await?;
    append_team_v3_blackboard_entry(
        runtime_pool,
        session_id,
        None,
        Some(main_agent_id.as_str()),
        "plan_fallback",
        "主 agent 拆解失败，使用默认任务图。",
        None,
    )
    .await?;
    Ok((team_member_ids(&fallback_state_data), fallback_state_data))
}

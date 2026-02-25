//! Agent Team 数据库操作层（纯运行时查询，无需 DATABASE_URL）

use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::Value;
use sqlx::postgres::PgPool;
use sqlx::Row;
use tracing::info;
use uuid::Uuid;

use super::models::*;

// ==================== 模板操作 ====================

pub async fn create_agent_team_template(
    pool: &PgPool,
    req: &CreateAgentTeamTemplateRequest,
    created_by: Option<&str>,
) -> Result<AgentTeamTemplate> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        r#"INSERT INTO agent_team_templates
           (id, name, description, domain, default_rounds_config, default_tool_policy,
            is_system, created_by, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"#,
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.domain)
    .bind(req.default_rounds_config.as_ref().map(|v| v.to_string()))
    .bind(req.default_tool_policy.as_ref().map(|v| v.to_string()))
    .bind(false)
    .bind(created_by)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .context("Failed to create agent_team_template")?;

    for (i, member_req) in req.members.iter().enumerate() {
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
    sqlx::query(
        r#"UPDATE agent_team_templates
           SET name = COALESCE($1, name),
               description = COALESCE($2, description),
               domain = COALESCE($3, domain),
               default_rounds_config = COALESCE($4, default_rounds_config),
               default_tool_policy = COALESCE($5, default_tool_policy),
               updated_at = $6
           WHERE id = $7"#,
    )
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.domain)
    .bind(req.default_rounds_config.as_ref().map(|v| v.to_string()))
    .bind(req.default_tool_policy.as_ref().map(|v| v.to_string()))
    .bind(now)
    .bind(id)
    .execute(pool)
    .await
    .context("Failed to update agent_team_template")?;

    // 若传入 members，则整体替换模板成员
    if let Some(members) = &req.members {
        sqlx::query("DELETE FROM agent_team_template_members WHERE template_id = $1")
            .bind(id)
            .execute(pool)
            .await
            .context("Failed to clear template members")?;

        for (i, member_req) in members.iter().enumerate() {
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
                      default_tool_policy, is_system, created_by, created_at, updated_at
               FROM agent_team_templates WHERE domain = $1 ORDER BY created_at DESC"#,
        )
        .bind(domain)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT id, name, description, domain, default_rounds_config,
                      default_tool_policy, is_system, created_by, created_at, updated_at
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
                  default_tool_policy, is_system, created_by, created_at, updated_at
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

    sqlx::query(
        r#"INSERT INTO agent_team_sessions
           (id, conversation_id, template_id, name, goal, state, current_round, max_rounds,
            total_tokens, estimated_cost, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"#,
    )
    .bind(&id)
    .bind(&req.conversation_id)
    .bind(&req.template_id)
    .bind(&req.name)
    .bind(&req.goal)
    .bind(TeamSessionState::Pending.to_string())
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
    } else if let Some(ref members) = req.members {
        for (i, m) in members.iter().enumerate() {
            create_agent_team_member_internal(pool, &id, m, i as i32).await?;
        }
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

async fn create_agent_team_member_internal(
    pool: &PgPool,
    session_id: &str,
    req: &CreateAgentTeamTemplateMemberRequest,
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
    .bind(&req.name)
    .bind(&req.responsibility)
    .bind(&req.system_prompt)
    .bind(&req.decision_style)
    .bind(&req.risk_preference)
    .bind(req.weight.unwrap_or(1.0))
    .bind(req.tool_policy.as_ref().map(|v| v.to_string()))
    .bind(req.output_schema.as_ref().map(|v| v.to_string()))
    .bind(req.sort_order.unwrap_or(sort_order))
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
    sqlx::query(
        r#"UPDATE agent_team_sessions
           SET name = COALESCE($1, name),
               goal = COALESCE($2, goal),
               state = COALESCE($3, state),
               max_rounds = COALESCE($4, max_rounds),
               error_message = COALESCE($5, error_message),
               updated_at = $6
           WHERE id = $7"#,
    )
    .bind(&req.name)
    .bind(&req.goal)
    .bind(&req.state)
    .bind(req.max_rounds)
    .bind(&req.error_message)
    .bind(now)
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
        r#"SELECT id, conversation_id, template_id, name, goal, state, state_machine,
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
) -> Result<Vec<AgentTeamSession>> {
    let rows = if let Some(conv_id) = conversation_id {
        sqlx::query(
            r#"SELECT id
               FROM agent_team_sessions
               WHERE conversation_id = $1
               ORDER BY updated_at DESC
               LIMIT $2"#,
        )
        .bind(conv_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT id
               FROM agent_team_sessions
               ORDER BY updated_at DESC
               LIMIT $1"#,
        )
        .bind(limit)
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

// ==================== 种子数据（内置模板） ====================

pub async fn seed_builtin_templates(pool: &PgPool) -> Result<()> {
    let count_row = sqlx::query(
        "SELECT COUNT(*) as cnt FROM agent_team_templates WHERE is_system = true",
    )
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

        sqlx::query(
            r#"INSERT INTO agent_team_templates
               (id, name, description, domain, is_system, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT (id) DO NOTHING"#,
        )
        .bind(&id)
        .bind(template_req.name)
        .bind(template_req.description)
        .bind(template_req.domain)
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
        name: "产品开发团队（4角色）",
        description: "覆盖产品、架构、后端、测试四个核心角色的默认团队",
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

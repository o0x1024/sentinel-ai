//! Agent Team 数据库操作层（SQLite 兼容）

use anyhow::Result;
use chrono::Utc;
use sqlx::Row;
use sqlx::SqlitePool;
use tracing::info;
use uuid::Uuid;

use super::models::*;
use super::repository::builtin_templates_seed;

pub async fn create_agent_team_template(
    pool: &SqlitePool,
    req: &CreateAgentTeamTemplateRequest,
    created_by: Option<&str>,
) -> Result<AgentTeamTemplate> {
    ensure_schema(pool).await?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        r#"INSERT INTO agent_team_templates
           (id, name, description, domain, default_rounds_config, default_tool_policy,
            is_system, created_by, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
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
    .await?;

    for (i, member_req) in req.members.iter().enumerate() {
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
    sqlx::query(
        r#"UPDATE agent_team_templates
           SET name = COALESCE(?, name),
               description = COALESCE(?, description),
               domain = COALESCE(?, domain),
               default_rounds_config = COALESCE(?, default_rounds_config),
               default_tool_policy = COALESCE(?, default_tool_policy),
               updated_at = ?
           WHERE id = ?"#,
    )
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.domain)
    .bind(req.default_rounds_config.as_ref().map(|v| v.to_string()))
    .bind(req.default_tool_policy.as_ref().map(|v| v.to_string()))
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    // 若传入 members，则整体替换模板成员
    if let Some(members) = &req.members {
        sqlx::query("DELETE FROM agent_team_template_members WHERE template_id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        for (i, member_req) in members.iter().enumerate() {
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
                      default_tool_policy, is_system, created_by, created_at, updated_at
               FROM agent_team_templates WHERE domain = ? ORDER BY created_at DESC"#,
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
                is_system: row.try_get("is_system")?,
                created_by: row.try_get("created_by")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                members: vec![],
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // 为前端列表补充成员信息（避免成员数显示为 0）
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
                  default_tool_policy, is_system, created_by, created_at, updated_at
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

    info!("Seeding built-in agent team templates for SQLite...");

    for template_req in builtin_templates_seed() {
        let id = template_req.id.to_string();
        let now = Utc::now();

        sqlx::query(
            r#"INSERT OR IGNORE INTO agent_team_templates
               (id, name, description, domain, is_system, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
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

    sqlx::query(
        r#"INSERT INTO agent_team_sessions
           (id, conversation_id, template_id, name, goal, state, state_machine, current_round, max_rounds,
            total_tokens, estimated_cost, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&req.conversation_id)
    .bind(&req.template_id)
    .bind(&req.name)
    .bind(&req.goal)
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
    } else if let Some(ref members) = req.members {
        for (i, m) in members.iter().enumerate() {
            create_agent_team_member_internal(pool, &id, m, i as i32).await?;
        }
    }

    get_agent_team_session(pool, &id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Session not found after creation"))
}

pub async fn update_agent_team_session(
    pool: &SqlitePool,
    id: &str,
    req: &UpdateAgentTeamSessionRequest,
) -> Result<()> {
    ensure_schema(pool).await?;

    let now = Utc::now();
    sqlx::query(
        r#"UPDATE agent_team_sessions
           SET name = COALESCE(?, name),
               goal = COALESCE(?, goal),
               state = COALESCE(?, state),
               max_rounds = COALESCE(?, max_rounds),
               state_machine = COALESCE(?, state_machine),
               error_message = COALESCE(?, error_message),
               updated_at = ?
           WHERE id = ?"#,
    )
    .bind(&req.name)
    .bind(&req.goal)
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

pub async fn get_agent_team_session(pool: &SqlitePool, id: &str) -> Result<Option<AgentTeamSession>> {
    ensure_schema(pool).await?;

    let row_opt = sqlx::query(
        r#"SELECT id, conversation_id, template_id, name, goal, state, state_machine,
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

async fn get_template_members(pool: &SqlitePool, template_id: &str) -> Result<Vec<AgentTeamTemplateMember>> {
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

async fn create_agent_team_member_internal(
    pool: &SqlitePool,
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
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
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
            is_system BOOLEAN NOT NULL DEFAULT FALSE,
            created_by TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await?;

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

    // Key indices
    let index_sqls = [
        "CREATE INDEX IF NOT EXISTS idx_agent_team_templates_domain ON agent_team_templates(domain)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_templates_is_system ON agent_team_templates(is_system)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_template_members_template_id ON agent_team_template_members(template_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_sessions_state ON agent_team_sessions(state)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_sessions_conversation_id ON agent_team_sessions(conversation_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_sessions_updated ON agent_team_sessions(updated_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_members_session_id ON agent_team_members(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_blackboard_session_id ON agent_team_blackboard_entries(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_rounds_session_id ON agent_team_rounds(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_messages_session_id ON agent_team_messages(session_id)",
        "CREATE INDEX IF NOT EXISTS idx_agent_team_artifacts_session_id ON agent_team_artifacts(session_id)",
    ];

    for sql in index_sqls {
        sqlx::query(sql).execute(pool).await?;
    }

    Ok(())
}

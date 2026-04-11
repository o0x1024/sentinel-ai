use anyhow::{anyhow, Result};
use chrono::Utc;
use sentinel_db::database_service::connection_manager::DatabasePool;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

pub(crate) fn parse_state_data_text(raw: &str) -> Value {
    serde_json::from_str::<Value>(raw).unwrap_or_else(|_| json!({}))
}

fn default_team_members() -> Vec<Value> {
    vec![json!({
        "id": "agent-1",
        "name": "Team Agent",
        "responsibility": "负责执行分配任务并输出可复用结论",
        "sort_order": 0,
        "weight": 1.0,
        "token_usage": 0,
        "tool_calls_count": 0,
        "is_active": false
    })]
}

pub(crate) fn normalize_team_member_values(mut members: Vec<Value>) -> Vec<Value> {
    if members.is_empty() {
        members = default_team_members();
    }

    members
        .into_iter()
        .enumerate()
        .map(|(index, mut member)| {
            if !member.is_object() {
                member = json!({});
            }
            let member_obj = member.as_object_mut().expect("member object");
            let default_id = format!("member-{}", index + 1);
            let id = member_obj
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or(default_id);
            let default_name = format!("Agent {}", index + 1);
            let name = member_obj
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or(default_name);

            member_obj.insert("id".to_string(), json!(id));
            member_obj.insert("name".to_string(), json!(name));
            member_obj.insert("sort_order".to_string(), json!(index as i64));
            member_obj
                .entry("weight".to_string())
                .or_insert_with(|| json!(1.0));
            member_obj
                .entry("token_usage".to_string())
                .or_insert_with(|| json!(0));
            member_obj
                .entry("tool_calls_count".to_string())
                .or_insert_with(|| json!(0));
            member_obj
                .entry("is_active".to_string())
                .or_insert_with(|| json!(false));
            member
        })
        .collect()
}

pub(crate) fn normalize_team_members(state_data: &Value) -> Vec<Value> {
    let members = state_data
        .get("members")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    normalize_team_member_values(members)
}

pub(crate) fn build_team_state_data_with_members(
    current: Option<&Value>,
    members: Vec<Value>,
    active_member_id: Option<&str>,
) -> Value {
    let mut state_data = current.cloned().unwrap_or_else(|| json!({}));
    if !state_data.is_object() {
        state_data = json!({});
    }
    let mut normalized_members = normalize_team_member_values(members);
    for member in &mut normalized_members {
        if let Some(member_obj) = member.as_object_mut() {
            let member_id = member_obj
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let is_active = active_member_id.map(|id| id == member_id).unwrap_or(false);
            member_obj.insert("is_active".to_string(), json!(is_active));
        }
    }

    if let Some(state_obj) = state_data.as_object_mut() {
        state_obj.insert("members".to_string(), Value::Array(normalized_members));
        let runtime = state_obj.entry("runtime").or_insert_with(|| json!({}));
        if let Some(runtime_obj) = runtime.as_object_mut() {
            runtime_obj.insert(
                "last_updated_at".to_string(),
                json!(Utc::now().to_rfc3339()),
            );
            if let Some(active_id) = active_member_id {
                runtime_obj.insert("active_member_id".to_string(), json!(active_id));
            } else {
                runtime_obj.remove("active_member_id");
            }
        }
    }
    state_data
}

pub(crate) fn build_team_state_data(
    current: Option<&Value>,
    active_member_id: Option<&str>,
) -> Value {
    let current_state = current.cloned().unwrap_or_else(|| json!({}));
    let members = normalize_team_members(&current_state);
    build_team_state_data_with_members(Some(&current_state), members, active_member_id)
}

pub(crate) fn first_member_id(state_data: &Value) -> String {
    state_data
        .get("members")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|m| m.get("id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "agent-1".to_string())
}

pub(crate) fn team_member_ids(state_data: &Value) -> Vec<String> {
    let mut ids = normalize_team_members(state_data)
        .into_iter()
        .filter_map(|member| {
            member
                .get("id")
                .and_then(|v| v.as_str())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        })
        .collect::<Vec<_>>();
    if ids.is_empty() {
        ids.push("agent-1".to_string());
    }
    ids
}

pub(crate) fn parse_task_dependencies(metadata: &Value) -> Vec<String> {
    metadata
        .get("depends_on")
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(|v| v.trim().to_string()))
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub(crate) fn extract_json_candidate(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
    }

    if let Some(fence_start) = trimmed.find("```json").or_else(|| trimmed.find("```")) {
        let after_start = &trimmed[fence_start..];
        if let Some(first_line_end) = after_start.find('\n') {
            let body = &after_start[first_line_end + 1..];
            if let Some(fence_end) = body.find("```") {
                let fenced = body[..fence_end].trim();
                if fenced.starts_with('{') && fenced.ends_with('}') {
                    return Some(fenced.to_string());
                }
                if let Some(candidate) = extract_first_json_object(fenced) {
                    return Some(candidate);
                }
            }
        }
    }
    extract_first_json_object(trimmed)
}

fn extract_first_json_object(raw: &str) -> Option<String> {
    let mut start_index: Option<usize> = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in raw.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start_index = Some(index);
                }
                depth += 1;
            }
            '}' => {
                if depth == 0 {
                    continue;
                }
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = start_index {
                        return Some(raw[start..=index].trim().to_string());
                    }
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) async fn set_team_v3_session_state(
    runtime_pool: &DatabasePool,
    session_id: &str,
    state: &str,
    now: &str,
) -> Result<()> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET state = ?, updated_at = ?
                   WHERE id = ?"#,
            )
            .bind(state)
            .bind(now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET state = $1, updated_at = $2
                   WHERE id = $3"#,
            )
            .bind(state)
            .bind(now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }
    Ok(())
}

pub(crate) async fn get_team_v3_session_state_data(
    runtime_pool: &DatabasePool,
    session_id: &str,
) -> Result<Value> {
    let raw_state_data: Option<String> = match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT state_data
                   FROM team_v3_sessions
                   WHERE id = ?"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            row.map(|r| r.get("state_data"))
        }
        DatabasePool::PostgreSQL(pool) => {
            let row = sqlx::query(
                r#"SELECT state_data::text as state_data
                   FROM team_v3_sessions
                   WHERE id = $1"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            row.map(|r| r.get("state_data"))
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    };

    Ok(raw_state_data
        .as_deref()
        .map(parse_state_data_text)
        .unwrap_or_else(|| json!({})))
}

pub(crate) async fn set_team_v3_session_state_data(
    runtime_pool: &DatabasePool,
    session_id: &str,
    state_data: &Value,
    now: &str,
) -> Result<()> {
    let state_data_text = serde_json::to_string(state_data)?;
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET state_data = ?, updated_at = ?
                   WHERE id = ?"#,
            )
            .bind(&state_data_text)
            .bind(now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET state_data = $1::jsonb, updated_at = $2
                   WHERE id = $3"#,
            )
            .bind(&state_data_text)
            .bind(now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }
    Ok(())
}

pub(crate) async fn ensure_team_v3_execution_tasks(
    runtime_pool: &DatabasePool,
    session_id: &str,
    goal: Option<&str>,
    state_data: &Value,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let members = team_member_ids(state_data);
    let goal_text = goal
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .unwrap_or("当前 Team 任务");

    let existing_count: i64 = match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT COUNT(1) as count
                   FROM team_v3_tasks
                   WHERE session_id = ?"#,
            )
            .bind(session_id)
            .fetch_one(pool)
            .await?;
            row.get("count")
        }
        DatabasePool::PostgreSQL(pool) => {
            let row = sqlx::query(
                r#"SELECT COUNT(1) as count
                   FROM team_v3_tasks
                   WHERE session_id = $1"#,
            )
            .bind(session_id)
            .fetch_one(pool)
            .await?;
            row.get("count")
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    };

    if existing_count == 0 {
        let tasks = vec![
            (
                "collect-context",
                "收集上下文与事实",
                format!("围绕目标「{}」收集关键上下文、约束与事实依据。", goal_text),
                10,
                members
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "agent-1".to_string()),
                json!({
                    "team_generated": true,
                    "depends_on": []
                }),
            ),
            (
                "analyze-options",
                "并行分析方案与风险",
                format!(
                    "围绕目标「{}」从产品、架构与风险角度并行提出候选方案。",
                    goal_text
                ),
                20,
                members
                    .get(1)
                    .cloned()
                    .or_else(|| members.first().cloned())
                    .unwrap_or_else(|| "agent-2".to_string()),
                json!({
                    "team_generated": true,
                    "depends_on": []
                }),
            ),
            (
                "deliver-summary",
                "汇总结论与行动建议",
                "整合前置任务结果，输出最终结论、风险清单和可执行下一步。".to_string(),
                30,
                members
                    .get(2)
                    .cloned()
                    .or_else(|| members.first().cloned())
                    .unwrap_or_else(|| "agent-3".to_string()),
                json!({
                    "team_generated": true,
                    "depends_on": ["collect-context", "analyze-options"]
                }),
            ),
        ];

        match runtime_pool {
            DatabasePool::SQLite(pool) => {
                for (task_key, title, instruction, priority, owner_agent_id, metadata) in tasks {
                    sqlx::query(
                        r#"INSERT INTO team_v3_tasks
                           (id, session_id, task_key, title, instruction, status, priority,
                            owner_agent_id, claimed_by_agent_id, claim_expires_at, metadata, created_at, updated_at)
                           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(session_id)
                    .bind(task_key)
                    .bind(title)
                    .bind(instruction)
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
            }
            DatabasePool::PostgreSQL(pool) => {
                for (task_key, title, instruction, priority, owner_agent_id, metadata) in tasks {
                    sqlx::query(
                        r#"INSERT INTO team_v3_tasks
                           (id, session_id, task_key, title, instruction, status, priority,
                            owner_agent_id, claimed_by_agent_id, claim_expires_at, metadata, created_at, updated_at)
                           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::jsonb, $12, $13)"#,
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(session_id)
                    .bind(task_key)
                    .bind(title)
                    .bind(instruction)
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
            }
            DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
        }
        return Ok(());
    }

    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_tasks
                   SET status = CASE
                       WHEN status = 'cancelled' THEN status
                       ELSE 'pending'
                   END,
                   claimed_by_agent_id = NULL,
                   claim_expires_at = NULL,
                   updated_at = ?
                   WHERE session_id = ?"#,
            )
            .bind(&now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_tasks
                   SET status = CASE
                       WHEN status = 'cancelled' THEN status
                       ELSE 'pending'
                   END,
                   claimed_by_agent_id = NULL,
                   claim_expires_at = NULL,
                   updated_at = $1
                   WHERE session_id = $2"#,
            )
            .bind(&now)
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }

    Ok(())
}

pub(crate) async fn apply_team_v3_execution_outcome(
    runtime_pool: &DatabasePool,
    session_id: &str,
    success: bool,
    _summary: Option<&str>,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            if success {
                sqlx::query(
                    r#"UPDATE team_v3_tasks
                       SET status = CASE
                           WHEN status = 'cancelled' THEN status
                           ELSE 'completed'
                       END,
                       claimed_by_agent_id = NULL,
                       claim_expires_at = NULL,
                       updated_at = ?
                       WHERE session_id = ?"#,
                )
                .bind(&now)
                .bind(session_id)
                .execute(pool)
                .await?;
            } else {
                sqlx::query(
                    r#"UPDATE team_v3_tasks
                       SET status = CASE
                           WHEN status IN ('running','claimed','waiting_review','waiting_handoff_ack') THEN 'failed'
                           WHEN status IN ('pending','ready_for_claim') THEN 'blocked'
                           ELSE status
                       END,
                       claimed_by_agent_id = NULL,
                       claim_expires_at = NULL,
                       updated_at = ?
                       WHERE session_id = ?"#,
                )
                .bind(&now)
                .bind(session_id)
                .execute(pool)
                .await?;
            }
        }
        DatabasePool::PostgreSQL(pool) => {
            if success {
                sqlx::query(
                    r#"UPDATE team_v3_tasks
                       SET status = CASE
                           WHEN status = 'cancelled' THEN status
                           ELSE 'completed'
                       END,
                       claimed_by_agent_id = NULL,
                       claim_expires_at = NULL,
                       updated_at = $1
                       WHERE session_id = $2"#,
                )
                .bind(&now)
                .bind(session_id)
                .execute(pool)
                .await?;
            } else {
                sqlx::query(
                    r#"UPDATE team_v3_tasks
                       SET status = CASE
                           WHEN status IN ('running','claimed','waiting_review','waiting_handoff_ack') THEN 'failed'
                           WHEN status IN ('pending','ready_for_claim') THEN 'blocked'
                           ELSE status
                       END,
                       claimed_by_agent_id = NULL,
                       claim_expires_at = NULL,
                       updated_at = $1
                       WHERE session_id = $2"#,
                )
                .bind(&now)
                .bind(session_id)
                .execute(pool)
                .await?;
            }
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }

    let current_state_data = get_team_v3_session_state_data(runtime_pool, session_id).await?;
    let next_state_data = build_team_state_data(Some(&current_state_data), None);
    set_team_v3_session_state_data(runtime_pool, session_id, &next_state_data, &now).await?;
    set_team_v3_session_state(
        runtime_pool,
        session_id,
        if success { "PLAN_DRAFT" } else { "FAILED" },
        &now,
    )
    .await?;
    Ok(())
}

pub(crate) async fn get_team_v3_latest_human_message_content(
    runtime_pool: &DatabasePool,
    session_id: &str,
) -> Result<Option<String>> {
    let payload_opt: Option<String> = match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT payload FROM team_v3_messages
                   WHERE session_id = ? AND (message_type = 'human_input' OR message_type = 'user')
                   ORDER BY created_at DESC, id DESC
                   LIMIT 1"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            row.map(|r| r.get("payload"))
        }
        DatabasePool::PostgreSQL(pool) => {
            let row = sqlx::query(
                r#"SELECT payload::text as payload FROM team_v3_messages
                   WHERE session_id = $1 AND (message_type = 'human_input' OR message_type = 'user')
                   ORDER BY created_at DESC, id DESC
                   LIMIT 1"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            row.map(|r| r.get("payload"))
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    };

    let content = payload_opt
        .and_then(|payload| serde_json::from_str::<Value>(&payload).ok())
        .and_then(|value| {
            value
                .get("content")
                .and_then(|c| c.as_str())
                .map(|s| s.trim().to_string())
        })
        .filter(|s| !s.is_empty());

    Ok(content)
}

pub(crate) async fn append_team_v3_status_message(
    runtime_pool: &DatabasePool,
    session_id: &str,
    content: &str,
) -> Result<()> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let payload = json!({ "content": content });
    let payload_text = serde_json::to_string(&payload)?;

    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_messages
                   (id, session_id, thread_id, from_agent_id, to_agent_id, message_type, message_kind, payload, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&id)
            .bind(session_id)
            .bind(session_id)
            .bind("team_system")
            .bind(Option::<String>::None)
            .bind("status")
            .bind("chat")
            .bind(&payload_text)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_messages
                   (id, session_id, thread_id, from_agent_id, to_agent_id, message_type, message_kind, payload, created_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9)"#,
            )
            .bind(&id)
            .bind(session_id)
            .bind(session_id)
            .bind("team_system")
            .bind(Option::<String>::None)
            .bind("status")
            .bind("chat")
            .bind(&payload_text)
            .bind(&now)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }
    Ok(())
}

pub(crate) async fn get_team_v3_session_context(
    runtime_pool: &DatabasePool,
    session_id: &str,
) -> Result<(Option<String>, Option<String>)> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let row = sqlx::query(
                r#"SELECT conversation_id, goal
                   FROM team_v3_sessions
                   WHERE id = ?"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            Ok(row
                .map(|r| (r.get("conversation_id"), r.get("goal")))
                .unwrap_or((None, None)))
        }
        DatabasePool::PostgreSQL(pool) => {
            let row = sqlx::query(
                r#"SELECT conversation_id, goal
                   FROM team_v3_sessions
                   WHERE id = $1"#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
            Ok(row
                .map(|r| (r.get("conversation_id"), r.get("goal")))
                .unwrap_or((None, None)))
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V3 does not support MySQL")),
    }
}

pub(crate) async fn set_team_v3_session_conversation_id(
    runtime_pool: &DatabasePool,
    session_id: &str,
    conversation_id: &str,
) -> Result<()> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET conversation_id = COALESCE(conversation_id, ?),
                       updated_at = ?
                   WHERE id = ?"#,
            )
            .bind(conversation_id)
            .bind(Utc::now().to_rfc3339())
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::PostgreSQL(pool) => {
            sqlx::query(
                r#"UPDATE team_v3_sessions
                   SET conversation_id = COALESCE(conversation_id, $1),
                       updated_at = $2
                   WHERE id = $3"#,
            )
            .bind(conversation_id)
            .bind(Utc::now().to_rfc3339())
            .bind(session_id)
            .execute(pool)
            .await?;
        }
        DatabasePool::MySQL(_) => return Err(anyhow!("Team V3 does not support MySQL")),
    }
    Ok(())
}

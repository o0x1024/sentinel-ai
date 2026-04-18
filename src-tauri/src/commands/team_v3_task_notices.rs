use anyhow::{anyhow, Result};
use chrono::Utc;
use sentinel_db::database_service::connection_manager::DatabasePool;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::team_v3_commands::TeamV3Task;
use super::team_v3_session_state::parse_task_dependencies;

const TEAM_DEPENDENCY_READY_KIND: &str = "team_dependency_ready";

#[derive(Debug, Clone, PartialEq, Eq)]
struct TeamDependencyReadyNotice {
    notice_id: String,
    task_id: String,
    task_key: String,
    task_title: String,
    content: String,
}

fn effective_task_section(task: &TeamV3Task, status_by_key: &HashMap<String, String>) -> &'static str {
    if matches!(task.status.as_str(), "pending" | "ready_for_claim") {
        let dependencies = parse_task_dependencies(&task.metadata);
        let has_unresolved_dependencies = dependencies.iter().any(|dependency| {
            !matches!(
                status_by_key.get(dependency).map(String::as_str),
                Some("completed" | "cancelled")
            )
        });
        return if has_unresolved_dependencies {
            "blocked"
        } else {
            "executable"
        };
    }

    match task.status.as_str() {
        "claimed" | "running" => "active",
        "waiting_review" => "review",
        "completed" => "completed",
        "failed" | "cancelled" => "attention",
        _ => "other",
    }
}

fn build_status_by_task_key(tasks: &[TeamV3Task]) -> HashMap<String, String> {
    tasks.iter()
        .map(|task| (task.task_key.clone(), task.status.clone()))
        .collect()
}

fn build_task_by_id<'a>(tasks: &'a [TeamV3Task]) -> HashMap<&'a str, &'a TeamV3Task> {
    tasks.iter().map(|task| (task.id.as_str(), task)).collect()
}

fn build_notice_id(session_id: &str, task: &TeamV3Task) -> String {
    let stamp = if task.updated_at.trim().is_empty() {
        task.created_at.trim()
    } else {
        task.updated_at.trim()
    };
    format!(
        "{}:{}:{}:{}",
        TEAM_DEPENDENCY_READY_KIND,
        session_id.trim(),
        task.id.trim(),
        stamp
    )
}

fn collect_existing_notice_ids_from_payloads(payloads: &[Value]) -> HashSet<String> {
    payloads
        .iter()
        .filter_map(|payload| {
            payload
                .get("metadata")
                .and_then(|metadata| metadata.get("notice_id"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn build_dependency_ready_notices(
    session_id: &str,
    previous_tasks: &[TeamV3Task],
    next_tasks: &[TeamV3Task],
    existing_notice_ids: &HashSet<String>,
) -> Vec<TeamDependencyReadyNotice> {
    if session_id.trim().is_empty() || previous_tasks.is_empty() || next_tasks.is_empty() {
        return Vec::new();
    }

    let previous_by_id = build_task_by_id(previous_tasks);
    let previous_status_by_key = build_status_by_task_key(previous_tasks);
    let next_status_by_key = build_status_by_task_key(next_tasks);
    let next_titles_by_key = next_tasks
        .iter()
        .map(|task| (task.task_key.as_str(), task.title.as_str()))
        .collect::<HashMap<_, _>>();

    next_tasks
        .iter()
        .filter_map(|task| {
            let previous = previous_by_id.get(task.id.as_str())?;
            let previous_section = effective_task_section(previous, &previous_status_by_key);
            let next_section = effective_task_section(task, &next_status_by_key);
            if previous_section != "blocked" || next_section != "executable" {
                return None;
            }

            let notice_id = build_notice_id(session_id, task);
            if existing_notice_ids.contains(&notice_id) {
                return None;
            }

            let satisfied_dependencies = parse_task_dependencies(&task.metadata)
                .into_iter()
                .filter(|dependency| {
                    matches!(
                        next_status_by_key.get(dependency).map(String::as_str),
                        Some("completed" | "cancelled")
                    )
                })
                .filter_map(|dependency| {
                    next_titles_by_key
                        .get(dependency.as_str())
                        .map(|title| (*title).to_string())
                        .or(Some(dependency))
                })
                .collect::<Vec<_>>();

            let detail = if satisfied_dependencies.is_empty() {
                "前置依赖已满足。".to_string()
            } else {
                format!("已完成前置任务：{}", satisfied_dependencies.join("、"))
            };

            Some(TeamDependencyReadyNotice {
                notice_id,
                task_id: task.id.clone(),
                task_key: task.task_key.clone(),
                task_title: task.title.clone(),
                content: format!("任务现在可开始执行：{}。{}", task.title, detail),
            })
        })
        .collect()
}

async fn list_existing_notice_payloads(
    runtime_pool: &DatabasePool,
    session_id: &str,
) -> Result<Vec<Value>> {
    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            let rows = sqlx::query(
                r#"SELECT payload
                   FROM team_v3_messages
                   WHERE session_id = ? AND message_type IN ('system', 'status')
                   ORDER BY created_at ASC, id ASC"#,
            )
            .bind(session_id)
            .fetch_all(pool)
            .await?;
            Ok(rows
                .into_iter()
                .filter_map(|row| {
                    let payload_text: String = row.get("payload");
                    serde_json::from_str::<Value>(&payload_text).ok()
                })
                .collect())
        }
        DatabasePool::PostgreSQL(pool) => {
            let rows = sqlx::query(
                r#"SELECT payload::text as payload
                   FROM team_v3_messages
                   WHERE session_id = $1 AND message_type IN ('system', 'status')
                   ORDER BY created_at ASC, id ASC"#,
            )
            .bind(session_id)
            .fetch_all(pool)
            .await?;
            Ok(rows
                .into_iter()
                .filter_map(|row| {
                    let payload_text: String = row.get("payload");
                    serde_json::from_str::<Value>(&payload_text).ok()
                })
                .collect())
        }
        DatabasePool::MySQL(_) => Err(anyhow!("Team V3 does not support MySQL")),
    }
}

async fn insert_dependency_ready_notice(
    runtime_pool: &DatabasePool,
    session_id: &str,
    notice: &TeamDependencyReadyNotice,
) -> Result<()> {
    let message_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let payload = json!({
        "content": notice.content,
        "message": notice.content,
        "metadata": {
            "kind": TEAM_DEPENDENCY_READY_KIND,
            "notice_id": notice.notice_id,
            "task_record_id": notice.task_id,
            "task_key": notice.task_key,
            "task_title": notice.task_title,
            "action_label": "查看任务"
        }
    });
    let payload_text = serde_json::to_string(&payload)?;

    match runtime_pool {
        DatabasePool::SQLite(pool) => {
            sqlx::query(
                r#"INSERT INTO team_v3_messages
                   (id, session_id, thread_id, from_agent_id, to_agent_id, message_type, message_kind, payload, created_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(&message_id)
            .bind(session_id)
            .bind(session_id)
            .bind("team_system")
            .bind(Option::<String>::None)
            .bind("system")
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
            .bind(&message_id)
            .bind(session_id)
            .bind(session_id)
            .bind("team_system")
            .bind(Option::<String>::None)
            .bind("system")
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

pub(crate) async fn append_team_v3_dependency_ready_notices(
    runtime_pool: &DatabasePool,
    session_id: &str,
    previous_tasks: &[TeamV3Task],
    next_tasks: &[TeamV3Task],
) -> Result<usize> {
    let payloads = list_existing_notice_payloads(runtime_pool, session_id).await?;
    let existing_notice_ids = collect_existing_notice_ids_from_payloads(&payloads);
    let notices = build_dependency_ready_notices(
        session_id,
        previous_tasks,
        next_tasks,
        &existing_notice_ids,
    );

    for notice in &notices {
        insert_dependency_ready_notice(runtime_pool, session_id, notice).await?;
    }

    Ok(notices.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::team_v3_schema::ensure_schema_sqlite;

    fn build_task(
        id: &str,
        task_key: &str,
        title: &str,
        status: &str,
        depends_on: &[&str],
        updated_at: &str,
    ) -> TeamV3Task {
        TeamV3Task {
            id: id.to_string(),
            session_id: "session-1".to_string(),
            task_key: task_key.to_string(),
            title: title.to_string(),
            instruction: "instruction".to_string(),
            status: status.to_string(),
            priority: 10,
            owner_agent_id: None,
            claimed_by_agent_id: None,
            claim_expires_at: None,
            acceptance_criteria: None,
            metadata: json!({ "depends_on": depends_on }),
            created_at: "2026-04-18T00:00:00Z".to_string(),
            updated_at: updated_at.to_string(),
        }
    }

    #[test]
    fn build_dependency_ready_notices_detects_blocked_to_executable_transition() {
        let previous_tasks = vec![
            build_task(
                "task-1",
                "collect-context",
                "收集上下文",
                "running",
                &[],
                "2026-04-18T00:00:00Z",
            ),
            build_task(
                "task-2",
                "deliver-summary",
                "汇总结论",
                "pending",
                &["collect-context"],
                "2026-04-18T00:00:00Z",
            ),
        ];
        let next_tasks = vec![
            build_task(
                "task-1",
                "collect-context",
                "收集上下文",
                "completed",
                &[],
                "2026-04-18T00:01:00Z",
            ),
            build_task(
                "task-2",
                "deliver-summary",
                "汇总结论",
                "pending",
                &["collect-context"],
                "2026-04-18T00:01:00Z",
            ),
        ];

        let notices = build_dependency_ready_notices(
            "session-1",
            &previous_tasks,
            &next_tasks,
            &HashSet::new(),
        );

        assert_eq!(notices.len(), 1);
        assert!(notices[0].content.contains("任务现在可开始执行：汇总结论"));
        assert!(notices[0].content.contains("收集上下文"));
    }

    #[tokio::test]
    async fn append_team_v3_dependency_ready_notices_persists_system_message_once() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        ensure_schema_sqlite(&pool).await.unwrap();
        let runtime_pool = DatabasePool::SQLite(pool.clone());

        sqlx::query(
            r#"INSERT INTO team_v3_sessions
               (id, name, state, state_data, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?)"#,
        )
        .bind("session-1")
        .bind("Session")
        .bind("EXECUTING")
        .bind("{}")
        .bind("2026-04-18T00:00:00Z")
        .bind("2026-04-18T00:00:00Z")
        .execute(&pool)
        .await
        .unwrap();

        let previous_tasks = vec![
            build_task(
                "task-1",
                "collect-context",
                "收集上下文",
                "running",
                &[],
                "2026-04-18T00:00:00Z",
            ),
            build_task(
                "task-2",
                "deliver-summary",
                "汇总结论",
                "pending",
                &["collect-context"],
                "2026-04-18T00:00:00Z",
            ),
        ];
        let next_tasks = vec![
            build_task(
                "task-1",
                "collect-context",
                "收集上下文",
                "completed",
                &[],
                "2026-04-18T00:01:00Z",
            ),
            build_task(
                "task-2",
                "deliver-summary",
                "汇总结论",
                "pending",
                &["collect-context"],
                "2026-04-18T00:01:00Z",
            ),
        ];

        let inserted = append_team_v3_dependency_ready_notices(
            &runtime_pool,
            "session-1",
            &previous_tasks,
            &next_tasks,
        )
        .await
        .unwrap();
        assert_eq!(inserted, 1);

        let inserted_again = append_team_v3_dependency_ready_notices(
            &runtime_pool,
            "session-1",
            &previous_tasks,
            &next_tasks,
        )
        .await
        .unwrap();
        assert_eq!(inserted_again, 0);

        let row = sqlx::query(
            r#"SELECT payload
               FROM team_v3_messages
               WHERE session_id = ?"#,
        )
        .bind("session-1")
        .fetch_one(&pool)
        .await
        .unwrap();
        let payload_text: String = row.get("payload");
        let payload: Value = serde_json::from_str(&payload_text).unwrap();
        assert_eq!(
            payload
                .get("metadata")
                .and_then(|value| value.get("kind"))
                .and_then(Value::as_str),
            Some("team_dependency_ready")
        );
    }
}

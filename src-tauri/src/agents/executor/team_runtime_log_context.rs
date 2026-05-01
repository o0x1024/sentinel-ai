use std::sync::Arc;

use sentinel_db::database_service::connection_manager::DatabasePool;
use sentinel_db::DatabaseService;
use sqlx::Row;

#[derive(Debug, Clone)]
pub(crate) struct TeamRuntimeLogContext {
    pub architecture: String,
    pub run_id: String,
    pub task_id: Option<String>,
    pub task_title: Option<String>,
    pub agent_id: Option<String>,
    pub agent_name: Option<String>,
    pub role_type: Option<String>,
    pub phase: String,
    pub attempt_index: Option<u32>,
}

fn parse_team_v4_execution_id(execution_id: &str) -> Option<TeamRuntimeLogContext> {
    if !execution_id.starts_with("team-v4:") {
        return None;
    }
    let parts = execution_id.split(':').collect::<Vec<_>>();
    let run_id = parts.get(1)?.trim();
    if run_id.is_empty() {
        return None;
    }
    let task_id = parts
        .get(2)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let agent_id = parts
        .windows(2)
        .find(|window| window[0] == "specialist" || window[0] == "agent")
        .map(|window| window[1].trim().to_string())
        .filter(|value| !value.is_empty());
    let attempt_index = parts
        .windows(2)
        .find(|window| window[0] == "attempt")
        .and_then(|window| window[1].parse::<u32>().ok());
    let phase = if parts.iter().any(|part| *part == "monitor-review") {
        "monitor_review"
    } else if parts.iter().any(|part| *part == "orchestrator-replan") {
        "orchestrator_replan"
    } else {
        "specialist_execution"
    };

    Some(TeamRuntimeLogContext {
        architecture: "team_v4".to_string(),
        run_id: run_id.to_string(),
        task_id,
        task_title: None,
        agent_id,
        agent_name: None,
        role_type: None,
        phase: phase.to_string(),
        attempt_index,
    })
}

fn parse_team_v3_execution_id(execution_id: &str) -> Option<TeamRuntimeLogContext> {
    if !execution_id.starts_with("team-v3:") {
        return None;
    }
    let parts = execution_id.split(':').collect::<Vec<_>>();
    let run_id = parts.get(1)?.trim();
    if run_id.is_empty() {
        return None;
    }
    let agent_id = if parts.len() >= 5 {
        parts
            .get(3)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    } else {
        None
    };
    Some(TeamRuntimeLogContext {
        architecture: "team_v3".to_string(),
        run_id: run_id.to_string(),
        task_id: None,
        task_title: None,
        agent_id,
        agent_name: None,
        role_type: None,
        phase: "task_execution".to_string(),
        attempt_index: None,
    })
}

async fn enrich_team_v4_context(
    mut context: TeamRuntimeLogContext,
    db: &Arc<DatabaseService>,
) -> TeamRuntimeLogContext {
    let Some(task_id) = context.task_id.clone() else {
        return context;
    };
    let explicit_agent_id = context.agent_id.clone().unwrap_or_default();
    let Ok(pool) = db.get_runtime_pool() else {
        return context;
    };

    let row = match pool {
        DatabasePool::SQLite(pool) => sqlx::query(
            r#"SELECT t.title AS task_title,
                      COALESCE(explicit_agent.id, assigned_agent.id) AS agent_id,
                      COALESCE(explicit_agent.name, assigned_agent.name) AS agent_name,
                      COALESCE(explicit_agent.role_type, assigned_agent.role_type) AS role_type
               FROM team_v4_tasks t
               LEFT JOIN team_v4_agents assigned_agent ON assigned_agent.id = t.assigned_agent_id
               LEFT JOIN team_v4_agents explicit_agent ON explicit_agent.id = ?
               WHERE t.run_id = ? AND t.id = ?"#,
        )
        .bind(&explicit_agent_id)
        .bind(&context.run_id)
        .bind(&task_id)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten()
        .map(|row| {
            (
                row.get::<Option<String>, _>("task_title"),
                row.get::<Option<String>, _>("agent_id"),
                row.get::<Option<String>, _>("agent_name"),
                row.get::<Option<String>, _>("role_type"),
            )
        }),
        DatabasePool::PostgreSQL(pool) => sqlx::query(
            r#"SELECT t.title AS task_title,
                      COALESCE(explicit_agent.id, assigned_agent.id) AS agent_id,
                      COALESCE(explicit_agent.name, assigned_agent.name) AS agent_name,
                      COALESCE(explicit_agent.role_type, assigned_agent.role_type) AS role_type
               FROM team_v4_tasks t
               LEFT JOIN team_v4_agents assigned_agent ON assigned_agent.id = t.assigned_agent_id
               LEFT JOIN team_v4_agents explicit_agent ON explicit_agent.id = $1
               WHERE t.run_id = $2 AND t.id = $3"#,
        )
        .bind(&explicit_agent_id)
        .bind(&context.run_id)
        .bind(&task_id)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten()
        .map(|row| {
            (
                row.get::<Option<String>, _>("task_title"),
                row.get::<Option<String>, _>("agent_id"),
                row.get::<Option<String>, _>("agent_name"),
                row.get::<Option<String>, _>("role_type"),
            )
        }),
        DatabasePool::MySQL(_) => None,
    };

    if let Some((task_title, agent_id, agent_name, role_type)) = row {
        context.task_title = task_title;
        context.agent_id = agent_id.or(context.agent_id);
        context.agent_name = agent_name;
        context.role_type = role_type;
    }
    context
}

pub(crate) async fn resolve_team_runtime_log_context(
    execution_id: &str,
    db: Option<&Arc<DatabaseService>>,
) -> Option<TeamRuntimeLogContext> {
    let context = parse_team_v4_execution_id(execution_id)
        .or_else(|| parse_team_v3_execution_id(execution_id))?;
    if context.architecture == "team_v4" {
        if let Some(db) = db {
            return Some(enrich_team_v4_context(context, db).await);
        }
    }
    Some(context)
}

pub(crate) fn is_toolset_error_result(result: &str) -> bool {
    let lower = result.to_lowercase();
    lower.contains("toolset error")
        || lower.contains("toolnotfounderror")
        || lower.contains("tool not found")
}

pub(crate) fn log_toolset_error_with_context(
    context: Option<&TeamRuntimeLogContext>,
    execution_id: &str,
    tool_name: &str,
    tool_call_id: &str,
    result: &str,
) {
    let preview = result.chars().take(500).collect::<String>();
    if let Some(context) = context {
        tracing::warn!(
            "Team toolset error: architecture={}, run_id={}, task_id={:?}, task_title={:?}, role={:?}, agent_id={:?}, agent_name={:?}, phase={}, attempt={:?}, execution_id={}, tool_name={}, tool_call_id={}, result={}",
            context.architecture,
            context.run_id,
            context.task_id,
            context.task_title,
            context.role_type,
            context.agent_id,
            context.agent_name,
            context.phase,
            context.attempt_index,
            execution_id,
            tool_name,
            tool_call_id,
            preview
        );
    } else {
        tracing::warn!(
            "Agent toolset error: execution_id={}, tool_name={}, tool_call_id={}, result={}",
            execution_id,
            tool_name,
            tool_call_id,
            preview
        );
    }
}

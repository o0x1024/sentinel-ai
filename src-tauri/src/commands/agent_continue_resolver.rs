use crate::services::database::DatabaseService;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentContinueResolution {
    pub execution_id: String,
    pub resumed: bool,
    pub reason: &'static str,
}

fn compact_continue_text(input: &str) -> String {
    input
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| {
            !ch.is_whitespace()
                && !matches!(
                    ch,
                    '.' | ','
                        | '!'
                        | '?'
                        | ';'
                        | ':'
                        | '"'
                        | '\''
                        | '。'
                        | '，'
                        | '！'
                        | '？'
                        | '；'
                        | '：'
                )
        })
        .collect()
}

pub(crate) fn is_continue_only_input(input: &str) -> bool {
    matches!(
        compact_continue_text(input).as_str(),
        "continue"
            | "continueplease"
            | "goon"
            | "proceed"
            | "next"
            | "nextstep"
            | "继续"
            | "继续吧"
            | "继续执行"
            | "继续任务"
            | "继续当前任务"
            | "继续下一步"
            | "下一步"
            | "接着"
            | "接着做"
            | "接着执行"
    )
}

fn task_blocks_completion(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "pending" | "in_progress" | "failed"
    )
}

fn is_success_state(state: &str) -> bool {
    matches!(
        state.trim().to_ascii_lowercase().as_str(),
        "succeeded" | "completed"
    )
}

fn is_running_state(state: &str) -> bool {
    matches!(
        state.trim().to_ascii_lowercase().as_str(),
        "running" | "in_progress"
    )
}

fn parse_lease_active(raw: Option<&str>) -> bool {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    DateTime::parse_from_rfc3339(value)
        .map(|expires_at| expires_at.with_timezone(&Utc) > Utc::now())
        .unwrap_or(false)
}

pub(crate) async fn resolve_agent_continue_execution(
    app_handle: &AppHandle,
    conversation_id: &str,
    requested_execution_id: &str,
    task: &str,
) -> Result<AgentContinueResolution, String> {
    if !is_continue_only_input(task) {
        return Ok(AgentContinueResolution {
            execution_id: requested_execution_id.to_string(),
            resumed: false,
            reason: "not_continue_only_input",
        });
    }

    let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() else {
        return Err("Database service is not available for continue resolution".to_string());
    };

    if let Some(latest_turn) = db
        .get_latest_agent_execution_turn(conversation_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to resolve latest turn for conversation {}: {}",
                conversation_id, error
            )
        })?
    {
        let tasks = db
            .get_execution_tasks(&latest_turn.turn_id)
            .await
            .map_err(|error| {
                format!(
                    "Failed to inspect task ledger for turn {}: {}",
                    latest_turn.turn_id, error
                )
            })?;
        let blocked_tasks = tasks
            .iter()
            .filter(|task| task_blocks_completion(&task.status))
            .count();

        if is_success_state(&latest_turn.status) && blocked_tasks == 0 {
            return Ok(AgentContinueResolution {
                execution_id: requested_execution_id.to_string(),
                resumed: false,
                reason: "previous_turn_completed",
            });
        }

        if blocked_tasks > 0 {
            return Ok(AgentContinueResolution {
                execution_id: latest_turn.turn_id,
                resumed: true,
                reason: "unfinished_turn_task_ledger",
            });
        }
    }

    let latest = db
        .list_agent_harness_runs(conversation_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to resolve latest execution for conversation {}: {}",
                conversation_id, error
            )
        })?
        .into_iter()
        .next();

    let Some(latest) = latest else {
        return Ok(AgentContinueResolution {
            execution_id: requested_execution_id.to_string(),
            resumed: false,
            reason: "no_previous_execution",
        });
    };

    let tasks = db.get_execution_tasks(&latest.id).await.map_err(|error| {
        format!(
            "Failed to inspect task ledger for execution {}: {}",
            latest.id, error
        )
    })?;
    let blocked_tasks = tasks
        .iter()
        .filter(|task| task_blocks_completion(&task.status))
        .count();

    if is_success_state(&latest.state) && blocked_tasks == 0 {
        return Ok(AgentContinueResolution {
            execution_id: requested_execution_id.to_string(),
            resumed: false,
            reason: "previous_execution_completed",
        });
    }

    if is_running_state(&latest.state) && parse_lease_active(latest.lease_expires_at.as_deref()) {
        return Err(format!(
            "Previous execution {} is still running; wait for it to finish or stop it before sending continue.",
            latest.id
        ));
    }

    if blocked_tasks > 0 {
        return Ok(AgentContinueResolution {
            execution_id: latest.id,
            resumed: true,
            reason: "unfinished_task_ledger",
        });
    }

    Err(format!(
        "Cannot safely resolve continue for execution {}: state={} and no unfinished task ledger was found.",
        latest.id, latest.state
    ))
}

#[cfg(test)]
mod tests {
    use super::is_continue_only_input;

    #[test]
    fn detects_continue_only_inputs() {
        for input in ["继续", "继续下一步", "continue", "go on", "next step"] {
            assert!(is_continue_only_input(input), "{input}");
        }
    }

    #[test]
    fn ignores_continue_with_new_goal() {
        for input in ["继续修复 login timeout", "continue fixing login timeout"] {
            assert!(!is_continue_only_input(input), "{input}");
        }
    }
}

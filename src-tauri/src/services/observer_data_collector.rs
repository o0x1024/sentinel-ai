use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use sentinel_core::models::mission::{ListMissionsFilter, Mission};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sentinel_llm::log::turn_log_jsonl_paths_for_date;

use crate::services::database::DatabaseService;
use crate::services::mission_stateful_runtime::MissionRuntimeContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverDataSnapshot {
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub execution_summary: ExecutionSummary,
    pub account_health: Vec<AccountHealthEntry>,
    pub mission_progress: Vec<MissionProgressEntry>,
    pub failure_details: Vec<FailureDetail>,
    pub token_usage: TokenUsageSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub total_runs: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub running: i64,
    pub cancelled: i64,
    pub avg_duration_ms: f64,
    pub failure_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountHealthEntry {
    pub transport: String,
    pub account_id: String,
    pub display_name: String,
    pub status: String,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub peer_count: i64,
    pub execution_count_in_window: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionProgressEntry {
    pub mission_id: String,
    pub title: String,
    pub status: String,
    pub last_run_status: Option<String>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub consecutive_failures: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureDetail {
    pub execution_run_id: String,
    pub peer_display_name: String,
    pub task_text: String,
    pub error_message: String,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTokenUsage {
    pub provider: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub tool_calls: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageSummary {
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_tool_calls: i64,
    pub by_model: Vec<ModelTokenUsage>,
}

#[derive(Debug, Deserialize)]
struct TurnLogLine {
    timestamp: String,
    provider: String,
    model: String,
    summary: Value,
}

pub fn is_observer_mission(mission: &Mission) -> bool {
    mission.owner_kind == "observer"
        || mission_spec_kind(mission).as_deref() == Some("observer_mission")
}

fn mission_spec_kind(mission: &Mission) -> Option<String> {
    mission.mission_spec_json.as_ref().and_then(|json| {
        serde_json::from_str::<Value>(json)
            .ok()
            .and_then(|value| value.get("kind").and_then(|kind| kind.as_str()).map(str::to_string))
    })
}

pub fn resolve_observer_window_start(previous_state_json: &str, end: DateTime<Utc>) -> DateTime<Utc> {
    if let Ok(state) = serde_json::from_str::<Value>(previous_state_json) {
        if let Some(raw) = state.get("last_observation_at").and_then(|value| value.as_str()) {
            if let Ok(parsed) = DateTime::parse_from_rfc3339(raw) {
                return parsed.with_timezone(&Utc);
            }
        }
    }
    end - Duration::hours(24)
}

pub async fn collect_observer_snapshot(
    db: &Arc<DatabaseService>,
    previous_state_json: &str,
) -> Result<ObserverDataSnapshot, String> {
    let window_end = Utc::now();
    let window_start = resolve_observer_window_start(previous_state_json, window_end);

    let execution_summary = build_execution_summary(db, window_start).await;
    let account_health = build_account_health(db, window_start).await;
    let mission_progress = build_mission_progress(db).await;
    let failure_details = build_failure_details(db, window_start).await;
    let token_usage = aggregate_token_usage_from_turn_logs(window_start, window_end);

    Ok(ObserverDataSnapshot {
        window_start,
        window_end,
        execution_summary,
        account_health,
        mission_progress,
        failure_details,
        token_usage,
    })
}

pub fn inject_observer_data(
    runtime_context: &mut MissionRuntimeContext,
    snapshot: &ObserverDataSnapshot,
) -> Result<(), String> {
    let mut ctx: Value = serde_json::from_str(&runtime_context.execution_context_json)
        .map_err(|e| format!("Invalid execution context JSON: {e}"))?;
    ctx["observer_data"] = serde_json::to_value(snapshot)
        .map_err(|e| format!("Failed to serialize observer snapshot: {e}"))?;
    runtime_context.execution_context_json = serde_json::to_string_pretty(&ctx)
        .map_err(|e| format!("Failed to serialize execution context: {e}"))?;
    Ok(())
}

async fn build_execution_summary(db: &Arc<DatabaseService>, since: DateTime<Utc>) -> ExecutionSummary {
    let counts = db
        .count_bot_execution_runs_by_status(since)
        .await
        .unwrap_or_default();
    let succeeded = counts
        .iter()
        .filter(|(status, _)| matches!(status.as_str(), "completed" | "succeeded"))
        .map(|(_, count)| *count)
        .sum();
    let failed = counts
        .iter()
        .filter(|(status, _)| matches!(status.as_str(), "failed" | "error"))
        .map(|(_, count)| *count)
        .sum();
    let running = counts
        .iter()
        .filter(|(status, _)| matches!(status.as_str(), "running" | "in_progress"))
        .map(|(_, count)| *count)
        .sum();
    let cancelled = counts
        .iter()
        .filter(|(status, _)| status.as_str() == "cancelled")
        .map(|(_, count)| *count)
        .sum();
    let total_runs = counts.values().copied().sum();
    let avg_duration_ms = db.avg_bot_execution_duration_ms(since).await.unwrap_or(0.0);
    let failure_rate = if total_runs > 0 {
        failed as f64 / total_runs as f64
    } else {
        0.0
    };

    ExecutionSummary {
        total_runs,
        succeeded,
        failed,
        running,
        cancelled,
        avg_duration_ms,
        failure_rate,
    }
}

async fn build_account_health(
    db: &Arc<DatabaseService>,
    since: DateTime<Utc>,
) -> Vec<AccountHealthEntry> {
    let accounts = db.list_bot_accounts(None).await.unwrap_or_default();
    let peers = db.list_bot_peers(None, None, None, 10_000).await.unwrap_or_default();
    let mut peer_counts: HashMap<(String, String), i64> = HashMap::new();
    for peer in peers {
        *peer_counts
            .entry((peer.transport.clone(), peer.account_id.clone()))
            .or_insert(0) += 1;
    }

    let mut entries = Vec::new();
    for account in accounts {
        let execution_count = db
            .count_bot_executions_for_account_in_window(
                &account.transport,
                &account.account_id,
                since,
            )
            .await
            .unwrap_or(0);
        entries.push(AccountHealthEntry {
            transport: account.transport.clone(),
            account_id: account.account_id.clone(),
            display_name: account
                .display_name
                .clone()
                .unwrap_or_else(|| account.account_id.clone()),
            status: account.status.unwrap_or_else(|| "unknown".to_string()),
            last_seen_at: account.last_seen_at,
            peer_count: peer_counts
                .get(&(account.transport.clone(), account.account_id.clone()))
                .copied()
                .unwrap_or(0),
            execution_count_in_window: execution_count,
        });
    }
    entries
}

async fn build_mission_progress(db: &Arc<DatabaseService>) -> Vec<MissionProgressEntry> {
    let filter = ListMissionsFilter {
        owner_kind: None,
        owner_ref: None,
        status: None,
        limit: 200,
        offset: 0,
    };
    let missions = db.list_missions(&filter).await.unwrap_or_default();
    let mut entries = Vec::new();
    for mission in missions {
        if mission.owner_kind == "observer" {
            continue;
        }
        let runs = db
            .list_mission_runs(&mission.id, 1, 0)
            .await
            .unwrap_or_default();
        let last_run = runs.first();
        let consecutive_failures = db
            .count_consecutive_mission_failures(&mission.id)
            .await
            .unwrap_or(0);
        entries.push(MissionProgressEntry {
            mission_id: mission.id.clone(),
            title: mission.title.clone(),
            status: mission.status.clone(),
            last_run_status: last_run.map(|run| run.status.clone()),
            last_run_at: mission.last_run_at,
            next_run_at: mission.next_run_at,
            consecutive_failures,
        });
    }
    entries
}

async fn build_failure_details(db: &Arc<DatabaseService>, since: DateTime<Utc>) -> Vec<FailureDetail> {
    let failed_runs = db
        .list_failed_bot_execution_runs(since, 20)
        .await
        .unwrap_or_default();
    let peers = db.list_bot_peers(None, None, None, 10_000).await.unwrap_or_default();
    let peer_names: HashMap<(String, String, String, String), String> = peers
        .into_iter()
        .map(|peer| {
            (
                (
                    peer.transport,
                    peer.account_id,
                    peer.peer_type,
                    peer.peer_id,
                ),
                peer.display_name.unwrap_or_default(),
            )
        })
        .collect();

    failed_runs
        .into_iter()
        .map(|run| {
            let peer_display_name = peer_names
                .get(&(
                    run.transport.clone(),
                    run.account_id.clone(),
                    run.peer_type.clone(),
                    run.peer_id.clone(),
                ))
                .cloned()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| run.peer_id.clone());
            let error_message = run
                .error_message
                .unwrap_or_else(|| "unknown error".to_string());
            let truncated = truncate_chars(&error_message, 500);
            FailureDetail {
                execution_run_id: run.id,
                peer_display_name,
                task_text: run.task_text,
                error_message: truncated,
                started_at: run.started_at,
            }
        })
        .collect()
}

pub fn aggregate_token_usage_from_turn_logs(
    since: DateTime<Utc>,
    until: DateTime<Utc>,
) -> TokenUsageSummary {
    let mut total_input_tokens = 0_i64;
    let mut total_output_tokens = 0_i64;
    let mut total_tool_calls = 0_i64;
    let mut by_model: HashMap<(String, String), ModelTokenUsage> = HashMap::new();

    let mut date = since.date_naive();
    let end_date = until.date_naive();
    while date <= end_date {
        let date_str = date.format("%Y-%m-%d").to_string();
        for path in turn_log_jsonl_paths_for_date(&date_str) {
            if !path.exists() {
                continue;
            }
            if let Err(error) = scan_turn_log_file(&path, since, until, |entry| {
                total_input_tokens += entry.input_tokens;
                total_output_tokens += entry.output_tokens;
                total_tool_calls += entry.tool_calls;
                let key = (entry.provider.clone(), entry.model.clone());
                let usage = by_model.entry(key).or_insert_with(|| ModelTokenUsage {
                    provider: entry.provider.clone(),
                    model: entry.model.clone(),
                    input_tokens: 0,
                    output_tokens: 0,
                    tool_calls: 0,
                });
                usage.input_tokens += entry.input_tokens;
                usage.output_tokens += entry.output_tokens;
                usage.tool_calls += entry.tool_calls;
            }) {
                tracing::warn!("Failed to scan turn log {}: {error}", path.display());
            }
        }
        date = next_date(date);
    }

    TokenUsageSummary {
        total_input_tokens,
        total_output_tokens,
        total_tool_calls,
        by_model: by_model.into_values().collect(),
    }
}

struct ParsedTurnUsage {
    provider: String,
    model: String,
    input_tokens: i64,
    output_tokens: i64,
    tool_calls: i64,
}

fn scan_turn_log_file<F>(path: &Path, since: DateTime<Utc>, until: DateTime<Utc>, mut visit: F) -> Result<(), String>
where
    F: FnMut(ParsedTurnUsage),
{
    let file = File::open(path).map_err(|e| format!("open failed: {e}"))?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.map_err(|e| format!("read failed: {e}"))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let entry: TurnLogLine = serde_json::from_str(trimmed)
            .map_err(|e| format!("parse failed: {e}"))?;
        let timestamp = match DateTime::parse_from_rfc3339(&entry.timestamp) {
            Ok(value) => value.with_timezone(&Utc),
            Err(_) => continue,
        };
        if timestamp < since || timestamp > until {
            continue;
        }
        let input_tokens = entry
            .summary
            .get("usage")
            .and_then(|value| value.get("input_tokens"))
            .and_then(|value| value.as_i64())
            .unwrap_or(0);
        let output_tokens = entry
            .summary
            .get("usage")
            .and_then(|value| value.get("output_tokens"))
            .and_then(|value| value.as_i64())
            .unwrap_or(0);
        let tool_calls = entry
            .summary
            .get("tool_calls")
            .and_then(|value| value.as_array())
            .map(|value| value.len() as i64)
            .unwrap_or(0);
        visit(ParsedTurnUsage {
            provider: entry.provider,
            model: entry.model,
            input_tokens,
            output_tokens,
            tool_calls,
        });
    }
    Ok(())
}

fn next_date(date: NaiveDate) -> NaiveDate {
    date.succ_opt().unwrap_or(date)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    format!("{}...", value.chars().take(max_chars).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn resolves_window_from_state_or_defaults_to_24h() {
        let end = Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap();
        let start = resolve_observer_window_start("{}", end);
        assert!(end - start >= Duration::hours(23));

        let state = r#"{"last_observation_at":"2026-06-05T09:00:00Z"}"#;
        let start2 = resolve_observer_window_start(state, end);
        assert_eq!(start2.to_rfc3339(), "2026-06-05T09:00:00+00:00");
    }

    #[test]
    fn detects_observer_mission_by_owner_or_spec_kind() {
        let mut mission = Mission {
            id: "m1".to_string(),
            title: "Observer".to_string(),
            objective: "Observe".to_string(),
            status: "active".to_string(),
            owner_kind: "observer".to_string(),
            owner_ref: "global".to_string(),
            source_json: None,
            delivery_policy_json: None,
            assistant_profile_id: None,
            trigger_json: None,
            mission_spec_json: None,
            step_plan_json: None,
            success_criteria_json: None,
            context_strategy_json: None,
            budget_json: None,
            failure_policy_json: None,
            missed_run_policy: "skip".to_string(),
            next_run_at: None,
            last_run_at: None,
            last_error: None,
            run_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(is_observer_mission(&mission));
        mission.owner_kind = "user".to_string();
        mission.mission_spec_json =
            Some(r#"{"kind":"observer_mission"}"#.to_string());
        assert!(is_observer_mission(&mission));
    }

    #[test]
    fn injects_observer_data_into_runtime_context() {
        let mut context = MissionRuntimeContext {
            mission_spec_json: "{}".to_string(),
            previous_state_json: "{}".to_string(),
            execution_context_json: r#"{"mission":{"id":"m1"}}"#.to_string(),
        };
        let snapshot = ObserverDataSnapshot {
            window_start: Utc::now(),
            window_end: Utc::now(),
            execution_summary: ExecutionSummary {
                total_runs: 0,
                succeeded: 0,
                failed: 0,
                running: 0,
                cancelled: 0,
                avg_duration_ms: 0.0,
                failure_rate: 0.0,
            },
            account_health: vec![],
            mission_progress: vec![],
            failure_details: vec![],
            token_usage: TokenUsageSummary {
                total_input_tokens: 0,
                total_output_tokens: 0,
                total_tool_calls: 0,
                by_model: vec![],
            },
        };
        inject_observer_data(&mut context, &snapshot).expect("inject");
        let parsed: Value = serde_json::from_str(&context.execution_context_json).expect("json");
        assert!(parsed.get("observer_data").is_some());
    }
}

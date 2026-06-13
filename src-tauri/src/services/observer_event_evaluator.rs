use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use sentinel_core::models::mission::{ListMissionsFilter, Mission};
use serde::Deserialize;
use serde_json::{Map, Value};
use tauri::{AppHandle, Manager};

use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use crate::services::mission_run_worker::spawn_mission_run_worker;

#[derive(Debug, Deserialize)]
struct ObserverMissionSpec {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    event_triggers: ObserverEventTriggers,
}

#[derive(Debug, Default, Deserialize)]
struct ObserverEventTriggers {
    #[serde(default)]
    on_failure: ObserverFailureTrigger,
}

#[derive(Debug, Default, Deserialize)]
struct ObserverFailureTrigger {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_failure_threshold")]
    threshold: i64,
    #[serde(default = "default_window_minutes")]
    window_minutes: i64,
    #[serde(default = "default_cooldown_minutes")]
    cooldown_minutes: i64,
}

fn default_failure_threshold() -> i64 {
    3
}

fn default_window_minutes() -> i64 {
    30
}

fn default_cooldown_minutes() -> i64 {
    60
}

pub async fn evaluate_after_bot_execution(
    app_handle: &AppHandle,
    db: &Arc<DatabaseService>,
    failed: bool,
) {
    if !failed {
        return;
    }

    let Some(ai_manager) = app_handle.try_state::<Arc<AiServiceManager>>() else {
        tracing::warn!("Observer event evaluator skipped: AiServiceManager unavailable");
        return;
    };

    let filter = ListMissionsFilter {
        owner_kind: Some("observer".to_string()),
        owner_ref: None,
        status: Some("active".to_string()),
        limit: 20,
        offset: 0,
    };

    let missions = match db.list_missions(&filter).await {
        Ok(items) => items,
        Err(error) => {
            tracing::warn!("Observer event evaluator failed to list missions: {error}");
            return;
        }
    };

    for mission in missions {
        if let Err(error) = maybe_trigger_failure_observer_run(
            app_handle,
            db,
            &ai_manager,
            &mission,
        )
        .await
        {
            tracing::warn!(
                "Observer event evaluator failed for mission {}: {error}",
                mission.id
            );
        }
    }
}

async fn maybe_trigger_failure_observer_run(
    app_handle: &AppHandle,
    db: &Arc<DatabaseService>,
    ai_manager: &Arc<AiServiceManager>,
    mission: &Mission,
) -> Result<(), String> {
    let spec = parse_observer_spec(mission)?;
    if spec.kind != "observer_mission" || !spec.event_triggers.on_failure.enabled {
        return Ok(());
    }

    let trigger = &spec.event_triggers.on_failure;
    let since = Utc::now() - Duration::minutes(trigger.window_minutes.max(1));
    let counts = db
        .count_bot_execution_runs_by_status(since)
        .await
        .map_err(|e| format!("Failed to count bot failures: {e}"))?;
    let failed_count = counts
        .iter()
        .filter(|(status, _)| matches!(status.as_str(), "failed" | "error"))
        .map(|(_, count)| *count)
        .sum::<i64>();

    if failed_count < trigger.threshold {
        return Ok(());
    }

    let state = db
        .get_latest_mission_state_snapshot(&mission.id)
        .await
        .map_err(|e| format!("Failed to load observer state: {e}"))?;
    if let Some(snapshot) = state {
        if let Ok(parsed) = serde_json::from_str::<Value>(&snapshot.state_json) {
            if is_in_cooldown(
                parsed.get("last_event_trigger_at"),
                trigger.cooldown_minutes,
            ) {
                return Ok(());
            }
        }
    }

    let run = db
        .create_mission_run(&mission.id, "event")
        .await
        .map_err(|e| format!("Failed to create observer event run: {e}"))?;

    record_event_trigger_timestamp(db, &mission.id).await?;

    spawn_mission_run_worker(
        app_handle.clone(),
        db.clone(),
        ai_manager.clone(),
        mission.clone(),
        run.id.clone(),
        None,
    );

    Ok(())
}

async fn record_event_trigger_timestamp(
    db: &Arc<DatabaseService>,
    mission_id: &str,
) -> Result<(), String> {
    let mut state = db
        .get_latest_mission_state_snapshot(mission_id)
        .await
        .map_err(|e| format!("Failed to load observer state: {e}"))?
        .map(|snapshot| {
            serde_json::from_str::<Value>(&snapshot.state_json)
                .unwrap_or_else(|_| Value::Object(Map::new()))
        })
        .unwrap_or_else(|| Value::Object(Map::new()));

    if let Value::Object(ref mut map) = state {
        map.insert(
            "last_event_trigger_at".to_string(),
            Value::String(Utc::now().to_rfc3339()),
        );
    }

    let state_json = serde_json::to_string(&state)
        .map_err(|e| format!("Failed to serialize observer event state: {e}"))?;
    db.save_mission_state_snapshot(mission_id, None, &state_json)
        .await
        .map_err(|e| format!("Failed to save observer event state: {e}"))?;
    Ok(())
}

fn parse_observer_spec(mission: &Mission) -> Result<ObserverMissionSpec, String> {
    let raw = mission
        .mission_spec_json
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("{}");
    serde_json::from_str(raw).map_err(|e| format!("Invalid observer mission spec: {e}"))
}

fn is_in_cooldown(last_trigger: Option<&Value>, cooldown_minutes: i64) -> bool {
    let Some(raw) = last_trigger.and_then(|value| value.as_str()) else {
        return false;
    };
    let Ok(parsed) = DateTime::parse_from_rfc3339(raw) else {
        return false;
    };
    let elapsed = Utc::now().signed_duration_since(parsed.with_timezone(&Utc));
    elapsed < Duration::minutes(cooldown_minutes.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_blocks_recent_event_triggers() {
        let recent = Value::String((Utc::now() - Duration::minutes(5)).to_rfc3339());
        assert!(is_in_cooldown(Some(&recent), 60));
    }
}

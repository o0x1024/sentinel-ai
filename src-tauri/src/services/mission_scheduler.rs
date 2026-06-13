use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use crate::services::ensure_bot_console_access;
use chrono_tz::Tz;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

const SCHEDULER_TICK_SECS: u64 = 30;
const DEFAULT_MAX_CONCURRENT_RUNS: usize = 1;
const DEFAULT_LOCK_TTL_SECS: i64 = 600; // 10 minutes

pub struct MissionSchedulerState {
    pub running_mission_ids: Arc<RwLock<std::collections::HashSet<String>>>,
    pub max_concurrent_runs: usize,
}

impl Default for MissionSchedulerState {
    fn default() -> Self {
        Self {
            running_mission_ids: Arc::new(RwLock::new(std::collections::HashSet::new())),
            max_concurrent_runs: DEFAULT_MAX_CONCURRENT_RUNS,
        }
    }
}

/// Run startup recovery: release expired locks, handle stale runs, handle missed runs.
pub async fn run_startup_recovery(db: &Arc<DatabaseService>) {
    if let Err(e) = recover_expired_locks(db).await {
        tracing::warn!("Failed to release expired mission locks on startup: {e}");
    }
    if let Err(e) = recover_stale_runs(db).await {
        tracing::warn!("Failed to recover stale mission runs on startup: {e}");
    }
    if let Err(e) = handle_missed_runs(db).await {
        tracing::warn!("Failed to handle missed mission runs on startup: {e}");
    }
}

/// Start the background mission scheduler loop.
pub fn spawn_mission_scheduler(
    db: Arc<DatabaseService>,
    ai_manager: Arc<AiServiceManager>,
    app_handle: AppHandle,
    cancel: CancellationToken,
    state: Arc<MissionSchedulerState>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!("Mission scheduler started (tick={}s)", SCHEDULER_TICK_SECS);
        let mut tick = tokio::time::interval(Duration::from_secs(SCHEDULER_TICK_SECS));

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    tracing::info!("Mission scheduler cancelled, shutting down");
                    break;
                }
                _ = tick.tick() => {
                    if let Err(e) = dispatch_due_missions(&db, &ai_manager, &app_handle, &state).await {
                        tracing::error!("Mission scheduler tick error: {e}");
                    }
                }
            }
        }
    })
}

async fn dispatch_due_missions(
    db: &Arc<DatabaseService>,
    ai_manager: &Arc<AiServiceManager>,
    app_handle: &AppHandle,
    state: &Arc<MissionSchedulerState>,
) -> Result<(), String> {
    if ensure_bot_console_access().is_err() {
        tracing::debug!("Mission scheduler skipped because Bot Console is not activated");
        return Ok(());
    }

    let due = db
        .list_due_missions()
        .await
        .map_err(|e| format!("Failed to list due missions: {e}"))?;

    if due.is_empty() {
        return Ok(());
    }

    let active_runs = state.running_mission_ids.read().await;
    let available_slots = state.max_concurrent_runs.saturating_sub(active_runs.len());

    if available_slots == 0 {
        tracing::debug!(
            "Mission scheduler: no available slots ({} running, max {})",
            active_runs.len(),
            state.max_concurrent_runs
        );
        return Ok(());
    }
    drop(active_runs);

    for mission in due.iter().take(available_slots) {
        let mission_id = mission.id.clone();
        let already_running = state.running_mission_ids.read().await.contains(&mission_id);
        if already_running {
            continue;
        }

        let acquired = db
            .acquire_mission_lock(
                &mission_id,
                "scheduler",
                "mission_scheduler",
                DEFAULT_LOCK_TTL_SECS,
            )
            .await
            .unwrap_or(false);

        if !acquired {
            tracing::debug!("Mission {mission_id}: could not acquire lock, skipping");
            continue;
        }

        state
            .running_mission_ids
            .write()
            .await
            .insert(mission_id.clone());

        let run_result = db.create_mission_run(&mission_id, "scheduled").await;

        match run_result {
            Ok(run) => {
                tracing::info!(
                    "Mission scheduler: created run {} for mission {} (run_index={})",
                    run.id,
                    mission_id,
                    run.run_index
                );

                if let Some(ref trigger_json) = mission.trigger_json {
                    if let Ok(next) = calculate_next_run_from_trigger(trigger_json) {
                        let _ = db.update_mission_next_run(&mission_id, next).await;
                    }
                }

                let mission_for_run = mission.clone();
                let app_for_run = app_handle.clone();
                crate::services::mission_run_worker::spawn_mission_run_worker(
                    app_for_run,
                    db.clone(),
                    ai_manager.clone(),
                    mission_for_run,
                    run.id.clone(),
                    Some(state.clone()),
                );
            }
            Err(e) => {
                tracing::error!("Mission scheduler: failed to create run for {mission_id}: {e}");
                let _ = db.release_mission_lock(&mission_id).await;
                state.running_mission_ids.write().await.remove(&mission_id);
            }
        }
    }

    Ok(())
}

async fn recover_expired_locks(db: &Arc<DatabaseService>) -> Result<(), String> {
    let released = db
        .release_expired_mission_locks()
        .await
        .map_err(|e| e.to_string())?;
    if released > 0 {
        tracing::info!("Released {released} expired mission locks on startup");
    }
    Ok(())
}

async fn recover_stale_runs(db: &Arc<DatabaseService>) -> Result<(), String> {
    let stale = db
        .list_stale_running_runs()
        .await
        .map_err(|e| e.to_string())?;

    for run in &stale {
        tracing::warn!(
            "Marking stale mission run {} (mission={}) as failed",
            run.id,
            run.mission_id
        );
        let _ = db
            .update_mission_run_status(
                &run.id,
                "failed",
                Some("Application restarted while run was in progress"),
                None,
            )
            .await;
    }

    if !stale.is_empty() {
        tracing::info!("Recovered {} stale mission runs on startup", stale.len());
    }
    Ok(())
}

async fn handle_missed_runs(db: &Arc<DatabaseService>) -> Result<(), String> {
    let missed = db.list_missed_missions().await.map_err(|e| e.to_string())?;

    for mission in &missed {
        let policy = &mission.missed_run_policy;
        match policy.as_str() {
            "skip" => {
                tracing::info!(
                    "Mission {} missed run, policy=skip, advancing next_run_at",
                    mission.id
                );
                if let Some(ref trigger_json) = mission.trigger_json {
                    if let Ok(next) = calculate_next_run_from_trigger(trigger_json) {
                        let _ = db.update_mission_next_run(&mission.id, next).await;
                    }
                }
            }
            "run_once_on_startup" => {
                tracing::info!(
                    "Mission {} missed run, policy=run_once_on_startup, creating run",
                    mission.id
                );
                match db.create_mission_run(&mission.id, "missed_recovery").await {
                    Ok(run) => {
                        tracing::info!(
                            "Created missed-recovery run {} for mission {}",
                            run.id,
                            mission.id
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to create missed-recovery run for mission {}: {e}",
                            mission.id
                        );
                    }
                }
                if let Some(ref trigger_json) = mission.trigger_json {
                    if let Ok(next) = calculate_next_run_from_trigger(trigger_json) {
                        let _ = db.update_mission_next_run(&mission.id, next).await;
                    }
                }
            }
            "run_all_missed" => {
                tracing::info!(
                    "Mission {} missed run, policy=run_all_missed (treating as run_once for safety)",
                    mission.id
                );
                match db.create_mission_run(&mission.id, "missed_recovery").await {
                    Ok(run) => {
                        tracing::info!(
                            "Created missed-recovery run {} for mission {}",
                            run.id,
                            mission.id
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to create missed-recovery run for mission {}: {e}",
                            mission.id
                        );
                    }
                }
                if let Some(ref trigger_json) = mission.trigger_json {
                    if let Ok(next) = calculate_next_run_from_trigger(trigger_json) {
                        let _ = db.update_mission_next_run(&mission.id, next).await;
                    }
                }
            }
            _ => {
                tracing::warn!(
                    "Mission {} has unknown missed_run_policy: {policy}",
                    mission.id
                );
            }
        }
    }

    Ok(())
}

/// Public version for use from other modules (e.g. weixin gateway mission creation).
pub fn calculate_next_run_from_trigger_public(
    trigger_json: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, String> {
    calculate_next_run_from_trigger(trigger_json)
}

fn calculate_next_run_from_trigger(
    trigger_json: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, String> {
    let trigger: serde_json::Value =
        serde_json::from_str(trigger_json).map_err(|e| format!("Invalid trigger JSON: {e}"))?;

    let kind = trigger
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("manual");

    match kind {
        "cron" => {
            let cron_expr = trigger
                .get("cron_expr")
                .and_then(|v| v.as_str())
                .ok_or("Missing cron_expr in trigger")?;
            let timezone = trigger
                .get("timezone")
                .and_then(|v| v.as_str())
                .unwrap_or("Asia/Shanghai")
                .parse::<Tz>()
                .map_err(|e| format!("Invalid trigger timezone: {e}"))?;

            let schedule = cron_expr
                .parse::<cron::Schedule>()
                .map_err(|e| format!("Invalid cron expression: {e}"))?;

            let next = schedule
                .upcoming(timezone)
                .next()
                .ok_or("No upcoming cron time")?
                .with_timezone(&chrono::Utc);

            Ok(Some(next))
        }
        "interval" => {
            let interval_secs = trigger
                .get("interval_seconds")
                .and_then(|v| v.as_i64())
                .unwrap_or(3600);
            let next = chrono::Utc::now() + chrono::Duration::seconds(interval_secs);
            Ok(Some(next))
        }
        "multi_cron" => {
            let ticks = trigger
                .get("ticks")
                .and_then(|v| v.as_array())
                .ok_or("multi_cron trigger requires ticks")?;
            let mut next_runs = Vec::new();
            for tick in ticks {
                let name = tick
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(unnamed)");
                let cron_expr = tick
                    .get("cron_expr")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| format!("multi_cron tick {name} missing cron_expr"))?;
                let timezone = tick
                    .get("timezone")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Asia/Shanghai")
                    .parse::<Tz>()
                    .map_err(|e| format!("Invalid trigger timezone for tick {name}: {e}"))?;
                let schedule = cron_expr
                    .parse::<cron::Schedule>()
                    .map_err(|e| format!("Invalid cron expression for tick {name}: {e}"))?;
                if let Some(next) = schedule.upcoming(timezone).next() {
                    next_runs.push(next.with_timezone(&chrono::Utc));
                }
            }
            next_runs.sort_unstable();
            Ok(next_runs.into_iter().next())
        }
        "manual" | "event" => Ok(None),
        _ => {
            tracing::warn!("Unknown trigger kind: {kind}");
            Ok(None)
        }
    }
}

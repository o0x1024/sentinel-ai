use std::sync::Arc;

use sentinel_core::models::mission::Mission;
use tauri::AppHandle;

use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use crate::services::mission_delivery::deliver_mission_result;
use crate::services::mission_runner::execute_mission_run;
use crate::services::mission_scheduler::MissionSchedulerState;

pub fn spawn_mission_run_worker(
    app_handle: AppHandle,
    db: Arc<DatabaseService>,
    ai_manager: Arc<AiServiceManager>,
    mission: Mission,
    run_id: String,
    scheduler_state: Option<Arc<MissionSchedulerState>>,
) {
    tokio::spawn(async move {
        let outcome =
            execute_mission_run(&app_handle, &db, &ai_manager, &mission, &run_id).await;

        if let Ok(Some(finished_run)) = db.get_mission_run(&outcome.run_id).await {
            if let Err(error) =
                deliver_mission_result(&app_handle, &db, &mission, &finished_run, true).await
            {
                tracing::warn!(
                    "Mission {} run {} delivery failed: {}",
                    mission.id,
                    outcome.run_id,
                    error
                );
            }
        }

        if outcome.status == "failed" {
            let _ = db
                .update_mission_last_error(&mission.id, outcome.error_message.as_deref())
                .await;
        } else {
            let _ = db.update_mission_last_error(&mission.id, None).await;
        }

        if let Some(state) = scheduler_state {
            let _ = db.release_mission_lock(&mission.id).await;
            state
                .running_mission_ids
                .write()
                .await
                .remove(&mission.id);
        }
    });
}

pub async fn try_track_mission_run_start(
    scheduler_state: &MissionSchedulerState,
    mission_id: &str,
) -> bool {
    let mut running = scheduler_state.running_mission_ids.write().await;
    if running.contains(mission_id) {
        return false;
    }
    running.insert(mission_id.to_string());
    true
}

use std::sync::Arc;

use sentinel_core::models::mission::{
    CreateMissionRequest, ListMissionsFilter, Mission, MissionDelivery, MissionRun,
    MissionRuntimeDetail, UpdateMissionFieldsRequest,
};
use tauri::State;

use crate::commands::assistant_profile_commands::load_assistant_profile_by_id_or_default;
use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use crate::services::ensure_bot_console_access;
use crate::services::mission_planner::{
    plan_mission_from_text, validate_mission_draft, MissionDraft, PlannerResult,
};
use crate::services::mission_run_worker::spawn_mission_run_worker;
use crate::services::mission_scheduler::calculate_next_run_from_trigger_public;

const DEFAULT_RUN_LIMIT: i64 = 50;
const DEFAULT_RUNTIME_DETAIL_LIMIT: i64 = 50;

#[tauri::command]
pub async fn mission_create(
    request: CreateMissionRequest,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Mission, String> {
    ensure_bot_console_access()?;

    if let Some(ref profile_id) = request.assistant_profile_id {
        let profile = load_assistant_profile_by_id_or_default(&db_service, Some(profile_id))
            .await?
            .ok_or_else(|| format!("Assistant profile not found: {profile_id}"))?;

        if profile.run_mode == "team" {
            return Err("Mission does not support team mode profiles".to_string());
        }
    }

    db_service
        .create_mission(request)
        .await
        .map_err(|e| format!("Failed to create mission: {e}"))
}

#[tauri::command]
pub async fn mission_list(
    filter: ListMissionsFilter,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<Mission>, String> {
    db_service
        .list_missions(&filter)
        .await
        .map_err(|e| format!("Failed to list missions: {e}"))
}

#[tauri::command]
pub async fn mission_get(
    id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Option<Mission>, String> {
    db_service
        .get_mission(&id)
        .await
        .map_err(|e| format!("Failed to get mission: {e}"))
}

#[tauri::command]
pub async fn mission_update_fields(
    request: UpdateMissionFieldsRequest,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Mission, String> {
    ensure_bot_console_access()?;

    if let Some(ref profile_id) = request.assistant_profile_id {
        let profile = load_assistant_profile_by_id_or_default(&db_service, Some(profile_id))
            .await?
            .ok_or_else(|| format!("Assistant profile not found: {profile_id}"))?;
        if profile.run_mode == "team" {
            return Err("Mission does not support team mode profiles".to_string());
        }
    }

    let trigger_updated = request.trigger_json.is_some();
    let mission = db_service
        .update_mission_fields(request)
        .await
        .map_err(|e| format!("Failed to update mission: {e}"))?;

    if trigger_updated && mission.status == "active" {
        return reload_mission_with_schedule(&db_service, &mission).await;
    }

    Ok(mission)
}

#[tauri::command]
pub async fn mission_pause(
    id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Mission, String> {
    ensure_bot_console_access()?;

    db_service
        .update_mission_status(&id, "paused")
        .await
        .map_err(|e| format!("Failed to pause mission: {e}"))
}

#[tauri::command]
pub async fn mission_resume(
    id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Mission, String> {
    ensure_bot_console_access()?;

    let mission = db_service
        .update_mission_status(&id, "active")
        .await
        .map_err(|e| format!("Failed to resume mission: {e}"))?;

    reload_mission_with_schedule(&db_service, &mission).await
}

#[tauri::command]
pub async fn mission_archive(
    id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Mission, String> {
    ensure_bot_console_access()?;

    db_service
        .update_mission_status(&id, "archived")
        .await
        .map_err(|e| format!("Failed to archive mission: {e}"))
}

#[tauri::command]
pub async fn mission_activate(
    id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Mission, String> {
    ensure_bot_console_access()?;

    let mission = db_service
        .update_mission_status(&id, "active")
        .await
        .map_err(|e| format!("Failed to activate mission: {e}"))?;

    reload_mission_with_schedule(&db_service, &mission).await
}

#[tauri::command]
pub async fn mission_delete(
    id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<(), String> {
    ensure_bot_console_access()?;

    db_service
        .delete_mission(&id)
        .await
        .map_err(|e| format!("Failed to delete mission: {e}"))
}

#[tauri::command]
pub async fn mission_run_now(
    id: String,
    app_handle: tauri::AppHandle,
    db_service: State<'_, Arc<DatabaseService>>,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<MissionRun, String> {
    ensure_bot_console_access()?;

    let mission = db_service
        .get_mission(&id)
        .await
        .map_err(|e| format!("Failed to get mission: {e}"))?
        .ok_or_else(|| format!("Mission not found: {id}"))?;

    let (profile_snapshot, tool_config_snapshot) =
        build_profile_snapshot(&db_service, mission.assistant_profile_id.as_deref()).await?;

    let run = db_service
        .create_mission_run_with_snapshot(
            &id,
            "manual",
            profile_snapshot.as_deref(),
            tool_config_snapshot.as_deref(),
        )
        .await
        .map_err(|e| format!("Failed to trigger mission run: {e}"))?;

    spawn_mission_run_worker(
        app_handle,
        db_service.inner().clone(),
        ai_manager.inner().clone(),
        mission,
        run.id.clone(),
        None,
    );

    Ok(run)
}

#[tauri::command]
pub async fn mission_list_runs(
    mission_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<MissionRun>, String> {
    db_service
        .list_mission_runs(
            &mission_id,
            limit.unwrap_or(DEFAULT_RUN_LIMIT),
            offset.unwrap_or(0),
        )
        .await
        .map_err(|e| format!("Failed to list mission runs: {e}"))
}

#[tauri::command]
pub async fn mission_get_run(
    run_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Option<MissionRun>, String> {
    db_service
        .get_mission_run(&run_id)
        .await
        .map_err(|e| format!("Failed to get mission run: {e}"))
}

#[tauri::command]
pub async fn mission_get_runtime_detail(
    mission_id: String,
    run_id: Option<String>,
    limit: Option<i64>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<MissionRuntimeDetail, String> {
    db_service
        .get_mission_runtime_detail(
            &mission_id,
            run_id.as_deref(),
            limit.unwrap_or(DEFAULT_RUNTIME_DETAIL_LIMIT),
        )
        .await
        .map_err(|e| format!("Failed to get mission runtime detail: {e}"))
}

#[tauri::command]
pub async fn mission_list_deliveries(
    mission_id: String,
    run_id: Option<String>,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<MissionDelivery>, String> {
    db_service
        .list_mission_deliveries(&mission_id, run_id.as_deref())
        .await
        .map_err(|e| format!("Failed to list mission deliveries: {e}"))
}

/// Load the assistant profile and serialize a snapshot for the mission run.
async fn sync_scheduled_next_run(
    db_service: &DatabaseService,
    mission_id: &str,
    trigger_json: Option<&str>,
) -> Result<(), String> {
    let Some(trigger_json) = trigger_json.filter(|value| !value.trim().is_empty()) else {
        return Ok(());
    };

    let next_run_at = calculate_next_run_from_trigger_public(trigger_json)?;
    if let Some(next) = next_run_at {
        db_service
            .set_mission_next_run_at(mission_id, Some(next))
            .await
            .map_err(|e| format!("Failed to set mission next_run_at: {e}"))?;
    }
    Ok(())
}

async fn reload_mission_with_schedule(
    db_service: &DatabaseService,
    mission: &Mission,
) -> Result<Mission, String> {
    sync_scheduled_next_run(
        db_service,
        &mission.id,
        mission.trigger_json.as_deref(),
    )
    .await?;

    db_service
        .get_mission(&mission.id)
        .await
        .map_err(|e| format!("Failed to reload mission: {e}"))?
        .ok_or_else(|| format!("Mission not found: {}", mission.id))
}

async fn build_profile_snapshot(
    db_service: &DatabaseService,
    profile_id: Option<&str>,
) -> Result<(Option<String>, Option<String>), String> {
    let profile = match profile_id {
        Some(pid) => load_assistant_profile_by_id_or_default(db_service, Some(pid)).await?,
        None => load_assistant_profile_by_id_or_default(db_service, None).await?,
    };

    match profile {
        Some(p) => {
            let tool_config = serde_json::json!({
                "tools_enabled": p.default_tools_enabled,
                "web_search_enabled": p.default_web_search_enabled,
                "rag_enabled": p.default_rag_enabled,
                "tool_selection_strategy": p.default_tool_selection_strategy,
                "max_tools": p.default_max_tools,
                "preselected_tools": p.default_preselected_tools,
                "disabled_tools": p.default_disabled_tools,
                "manual_tools": p.default_manual_tools,
            });
            let profile_snapshot = serde_json::to_string(&p)
                .map_err(|e| format!("Failed to serialize profile: {e}"))?;
            let tool_config_snapshot = serde_json::to_string(&tool_config)
                .map_err(|e| format!("Failed to serialize tool config: {e}"))?;
            Ok((Some(profile_snapshot), Some(tool_config_snapshot)))
        }
        None => Ok((None, None)),
    }
}

#[tauri::command]
pub async fn mission_plan_from_text(
    text: String,
    ai_manager: State<'_, Arc<AiServiceManager>>,
) -> Result<PlannerResult, String> {
    ensure_bot_console_access()?;

    plan_mission_from_text(&ai_manager, &text).await
}

#[tauri::command]
pub async fn mission_validate_draft(draft: MissionDraft) -> Result<Vec<String>, String> {
    Ok(validate_mission_draft(&draft))
}

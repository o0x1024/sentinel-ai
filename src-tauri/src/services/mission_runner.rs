use std::sync::Arc;

use chrono::Utc;
use sentinel_core::models::mission::Mission;
use tauri::AppHandle;
use uuid::Uuid;

use crate::agents::executor::{execute_agent_turn, AgentExecuteParams};
use crate::agents::ToolConfig;
use crate::commands::assistant_profile_commands::{
    load_assistant_profile_by_id_or_default, AssistantProfilePayload,
};
use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;

pub struct MissionRunOutcome {
    pub run_id: String,
    pub status: String,
    pub result_summary: Option<String>,
    pub error_message: Option<String>,
}

/// Execute a single mission run via the existing agent runtime.
pub async fn execute_mission_run(
    app_handle: &AppHandle,
    db: &Arc<DatabaseService>,
    ai_manager: &Arc<AiServiceManager>,
    mission: &Mission,
    run_id: &str,
) -> MissionRunOutcome {
    let run_id = run_id.to_string();

    if let Err(e) = db
        .update_mission_run_status(&run_id, "running", None, None)
        .await
    {
        return MissionRunOutcome {
            run_id,
            status: "failed".to_string(),
            result_summary: None,
            error_message: Some(format!("Failed to mark run as running: {e}")),
        };
    }

    let profile = match load_profile_for_run(db, mission).await {
        Ok(p) => p,
        Err(e) => {
            let _ = db
                .update_mission_run_status(&run_id, "failed", Some(&e), None)
                .await;
            return MissionRunOutcome {
                run_id,
                status: "failed".to_string(),
                result_summary: None,
                error_message: Some(e),
            };
        }
    };

    let service = match ai_manager.resolve_generation_service(None).await {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("No AI service available: {e}");
            let _ = db
                .update_mission_run_status(&run_id, "failed", Some(&msg), None)
                .await;
            return MissionRunOutcome {
                run_id,
                status: "failed".to_string(),
                result_summary: None,
                error_message: Some(msg),
            };
        }
    };

    let ai_config = service.get_config();
    let model = profile.default_model.as_deref().unwrap_or(&ai_config.model);

    let execution_id = Uuid::new_v4().to_string();
    let conversation_id = format!("mission:{}:run:{}", mission.id, run_id);

    let _ = db
        .update_mission_run_agent_execution_id(&run_id, &execution_id)
        .await;

    let system_prompt = build_mission_system_prompt(mission);
    let task = build_mission_task(mission);
    let tool_config = build_tool_config_from_profile(&profile);

    let provider_key = ai_config
        .rig_provider
        .clone()
        .unwrap_or_else(|| ai_config.provider.clone());

    let params = AgentExecuteParams {
        execution_id: execution_id.clone(),
        conversation_id: Some(conversation_id),
        cancellation_generation: None,
        model: model.to_string(),
        system_prompt,
        task,
        active_browser_shell_direct_write_enabled: false,
        active_browser_shell_session_id: None,
        active_terminal_session_fingerprint: None,
        active_terminal_session_id: None,
        working_directory: None,
        provider_config_key: provider_key.clone(),
        rig_provider: provider_key,
        api_key: ai_config.api_key.clone(),
        api_base: ai_config.api_base.clone(),
        max_iterations: 20,
        timeout_secs: 300,
        tool_config: Some(tool_config),
        enable_tenth_man_rule: false,
        tenth_man_config: None,
        document_attachments: None,
        image_attachments: None,
        referenced_traffic: None,
        persist_messages: true,
        subagent_run_id: None,
        harness_run_id: None,
        context_policy: None,
        context_engine_mode: None,
        recursion_depth: 0,
    };

    match execute_agent_turn(app_handle, params).await {
        Ok(outcome) => {
            let summary = if outcome.final_response.len() > 500 {
                format!("{}...", &outcome.final_response[..500])
            } else {
                outcome.final_response.clone()
            };

            let checkpoint = serde_json::json!({
                "completed_at": Utc::now().to_rfc3339(),
                "agent_execution_id": execution_id,
                "response_length": outcome.final_response.len(),
            });
            let _ = db
                .update_mission_run_checkpoint(&run_id, &checkpoint.to_string())
                .await;

            let _ = db
                .update_mission_run_status(&run_id, "succeeded", None, Some(&summary))
                .await;

            MissionRunOutcome {
                run_id,
                status: "succeeded".to_string(),
                result_summary: Some(summary),
                error_message: None,
            }
        }
        Err(e) => {
            let error_msg = format!("Agent execution failed: {e}");
            let _ = db
                .update_mission_run_status(&run_id, "failed", Some(&error_msg), None)
                .await;

            MissionRunOutcome {
                run_id,
                status: "failed".to_string(),
                result_summary: None,
                error_message: Some(error_msg),
            }
        }
    }
}

async fn load_profile_for_run(
    db: &Arc<DatabaseService>,
    mission: &Mission,
) -> Result<AssistantProfilePayload, String> {
    load_assistant_profile_by_id_or_default(db, mission.assistant_profile_id.as_deref())
        .await?
        .ok_or_else(|| "No assistant profile available for mission execution".to_string())
}

fn build_mission_system_prompt(mission: &Mission) -> String {
    format!(
        "You are executing a mission task. Your objective: {}\n\n\
         Complete the task accurately and thoroughly. \
         Report your findings clearly.",
        mission.objective
    )
}

fn build_mission_task(mission: &Mission) -> String {
    if let Some(ref step_plan) = mission.step_plan_json {
        if let Ok(plan) = serde_json::from_str::<serde_json::Value>(step_plan) {
            if let Some(steps) = plan.as_array() {
                let steps_text: Vec<String> = steps
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        let desc = s
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("(no description)");
                        format!("{}. {}", i + 1, desc)
                    })
                    .collect();
                return format!(
                    "Mission: {}\n\nObjective: {}\n\nSteps:\n{}",
                    mission.title,
                    mission.objective,
                    steps_text.join("\n")
                );
            }
        }
    }

    format!(
        "Mission: {}\n\nObjective: {}",
        mission.title, mission.objective
    )
}

fn build_tool_config_from_profile(profile: &AssistantProfilePayload) -> ToolConfig {
    let json = serde_json::json!({
        "enabled": profile.default_tools_enabled,
        "selection_strategy": profile.default_tool_selection_strategy,
        "max_tools": profile.default_max_tools,
        "preselected_tools": profile.default_preselected_tools,
        "disabled_tools": profile.default_disabled_tools,
        "allowed_tools": [],
    });

    ToolConfig::from_json_value(json).unwrap_or_default()
}

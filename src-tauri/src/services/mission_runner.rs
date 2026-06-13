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
use crate::models::database::AiConversation;
use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use crate::services::mission_stateful_runtime::{
    build_runtime_context, build_system_prompt, build_task, parse_runtime_output,
    persist_runtime_output, run_status_from_completion, runtime_response_preview, summarize_output,
};
use crate::services::observer_data_collector::{
    collect_observer_snapshot, inject_observer_data, is_observer_mission,
};
use sentinel_db::Database;

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
    let max_iterations = ai_config.max_turns.unwrap_or(50).max(1);

    let execution_id = Uuid::new_v4().to_string();
    let conversation_id = mission_run_conversation_id(mission, &run_id);

    let _ = db
        .update_mission_run_agent_execution_id(&run_id, &execution_id)
        .await;

    let mut runtime_context = match build_runtime_context(db, mission, &run_id).await {
        Ok(context) => context,
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

    if is_observer_mission(mission) {
        match collect_observer_snapshot(db, &runtime_context.previous_state_json).await {
            Ok(snapshot) => {
                if let Err(error) = inject_observer_data(&mut runtime_context, &snapshot) {
                    tracing::warn!("Failed to inject observer data: {error}");
                }
            }
            Err(error) => {
                tracing::warn!("Observer data collection failed: {error}");
            }
        }
    }

    let system_prompt = build_system_prompt(mission);
    let task = build_task(mission, &runtime_context);
    let tool_config = build_tool_config_from_profile(&profile);

    let provider_key = ai_config
        .rig_provider
        .clone()
        .unwrap_or_else(|| ai_config.provider.clone());
    if let Err(e) = ensure_mission_conversation_exists(
        db,
        mission,
        &run_id,
        &conversation_id,
        model,
        &provider_key,
    )
    .await
    {
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
        max_iterations,
        timeout_secs: 0,
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
            let runtime_output = match parse_runtime_output(&outcome.final_response) {
                Ok(output) => output,
                Err(e) => {
                    let checkpoint = serde_json::json!({
                        "failed_at": Utc::now().to_rfc3339(),
                        "failure_stage": "parse_runtime_output",
                        "error_message": e,
                        "response_length": outcome.final_response.len(),
                        "response_preview": runtime_response_preview(&outcome.final_response),
                    });
                    let _ = db
                        .update_mission_run_checkpoint(&run_id, &checkpoint.to_string())
                        .await;
                    let _ = record_mission_tick_failed_event(
                        db,
                        mission,
                        &run_id,
                        "parse_runtime_output",
                        &e,
                        Some(&checkpoint),
                    )
                    .await;
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

            if let Err(e) =
                persist_runtime_output(db, mission, &run_id, &runtime_context, &runtime_output)
                    .await
            {
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

            let run_status = match run_status_from_completion(&runtime_output.completion.status) {
                Ok(status) => status,
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
            let summary = summarize_output(&runtime_output);

            let checkpoint = serde_json::json!({
                "completed_at": Utc::now().to_rfc3339(),
                "agent_execution_id": execution_id,
                "response_length": outcome.final_response.len(),
                "completion": &runtime_output.completion,
            });
            let _ = db
                .update_mission_run_checkpoint(&run_id, &checkpoint.to_string())
                .await;

            let _ = db
                .update_mission_run_status(&run_id, run_status, None, Some(&summary))
                .await;

            if runtime_output.completion.status == "completed" {
                let _ = db.update_mission_status(&mission.id, "completed").await;
            } else if runtime_output.completion.status == "blocked" {
                let _ = db.update_mission_status(&mission.id, "blocked").await;
            }

            MissionRunOutcome {
                run_id,
                status: run_status.to_string(),
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

fn mission_run_conversation_id(mission: &Mission, run_id: &str) -> String {
    if mission.owner_kind == "bot_peer" {
        let owner_ref = mission.owner_ref.trim();
        if !owner_ref.is_empty() {
            return owner_ref.to_string();
        }
    }
    format!("mission:{}:run:{}", mission.id, run_id)
}

async fn ensure_mission_conversation_exists(
    db: &Arc<DatabaseService>,
    mission: &Mission,
    run_id: &str,
    conversation_id: &str,
    model: &str,
    provider: &str,
) -> Result<(), String> {
    let is_bot_peer = mission.owner_kind == "bot_peer";
    let context_type = if is_bot_peer {
        None
    } else {
        Some("mission_run".to_string())
    };
    let fallback_title = if is_bot_peer {
        bot_conversation_title(&mission.owner_ref)
    } else {
        Some(format!("{} / run {}", mission.title, run_id))
    };

    if let Some(mut conversation) = db
        .get_ai_conversation(conversation_id)
        .await
        .map_err(|e| format!("Failed to query mission conversation: {e}"))?
    {
        let mut changed = false;
        if conversation
            .title
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            conversation.title = fallback_title;
            changed = true;
        }
        if conversation.model_name != model {
            conversation.model_name = model.to_string();
            changed = true;
        }
        if conversation.model_provider.as_deref() != Some(provider) {
            conversation.model_provider = Some(provider.to_string());
            changed = true;
        }
        if !is_bot_peer && conversation.context_type.as_deref() != Some("mission_run") {
            conversation.context_type = context_type;
            changed = true;
        }
        if changed {
            conversation.updated_at = Utc::now();
            db.update_ai_conversation(&conversation)
                .await
                .map_err(|e| format!("Failed to update mission conversation: {e}"))?;
        }
        return Ok(());
    }

    let now = Utc::now();
    let conversation = AiConversation {
        id: conversation_id.to_string(),
        title: fallback_title,
        service_name: if provider.trim().is_empty() {
            "default".to_string()
        } else {
            provider.to_string()
        },
        model_name: if model.trim().is_empty() {
            "default".to_string()
        } else {
            model.to_string()
        },
        model_provider: Some(provider.to_string()),
        context_type,
        project_id: None,
        vulnerability_id: None,
        scan_task_id: None,
        conversation_data: None,
        summary: None,
        total_messages: 0,
        total_tokens: 0,
        cost: 0.0,
        tags: if is_bot_peer {
            Some(r#"["bot","bot-peer"]"#.to_string())
        } else {
            Some(r#"["mission","mission-run"]"#.to_string())
        },
        tool_config: None,
        is_archived: false,
        created_at: now,
        updated_at: now,
    };
    db.create_ai_conversation(&conversation)
        .await
        .map_err(|e| format!("Failed to create mission conversation: {e}"))
}

fn bot_conversation_title(owner_ref: &str) -> Option<String> {
    let parts: Vec<&str> = owner_ref.split(':').collect();
    if parts.len() >= 4 {
        return Some(format!("{} {} {}", parts[0], parts[2], parts[3]));
    }
    Some("Bot Conversation".to_string())
}

async fn record_mission_tick_failed_event(
    db: &Arc<DatabaseService>,
    mission: &Mission,
    run_id: &str,
    failure_stage: &str,
    error_message: &str,
    diagnostics: Option<&serde_json::Value>,
) -> Result<(), String> {
    let payload = serde_json::json!({
        "failure_stage": failure_stage,
        "error_message": error_message,
        "diagnostics": diagnostics,
        "failed_at": Utc::now().to_rfc3339(),
    });
    let payload_json = serde_json::to_string(&payload)
        .map_err(|e| format!("Failed to serialize mission failure event: {e}"))?;
    db.save_mission_event(
        &mission.id,
        Some(run_id),
        "mission_tick_failed",
        "Mission tick failed",
        Some(&payload_json),
    )
    .await
    .map(|_| ())
    .map_err(|e| format!("Failed to save mission failure event: {e}"))
}

async fn load_profile_for_run(
    db: &Arc<DatabaseService>,
    mission: &Mission,
) -> Result<AssistantProfilePayload, String> {
    load_assistant_profile_by_id_or_default(db, mission.assistant_profile_id.as_deref())
        .await?
        .ok_or_else(|| "No assistant profile available for mission execution".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mission(owner_kind: &str, owner_ref: &str) -> Mission {
        Mission {
            id: "mission-1".to_string(),
            title: "Daily check".to_string(),
            objective: "Check status".to_string(),
            status: "active".to_string(),
            owner_kind: owner_kind.to_string(),
            owner_ref: owner_ref.to_string(),
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
        }
    }

    #[test]
    fn bot_peer_mission_reuses_owner_ref_as_conversation_id() {
        let mission = mission("bot_peer", "weixin:acct:group:peer-1");
        assert_eq!(
            mission_run_conversation_id(&mission, "run-1"),
            "weixin:acct:group:peer-1"
        );
    }

    #[test]
    fn non_bot_mission_uses_internal_run_conversation_id() {
        let mission = mission("user", "default");
        assert_eq!(
            mission_run_conversation_id(&mission, "run-1"),
            "mission:mission-1:run:run-1"
        );
    }
}

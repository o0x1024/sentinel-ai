use std::sync::Arc;

use chrono::Utc;
use tauri::AppHandle;

use crate::agents::executor::AgentExecuteParams;
use crate::commands::agent_turn_completion::assess_agent_turn_completion;
use crate::commands::ai::{
    create_cancellation_token, emit_agent_execution_finished_for_generation_with_conversation,
    is_conversation_generation_cancelled, AgentExecutionOutcome, CancellationGuard,
};
use crate::commands::ai_runtime_harness::{
    agent_harness_completion_payload, agent_harness_continuation_payload,
    append_agent_harness_event, assess_agent_harness_success,
    build_agent_harness_continuation_task, checkpoint_agent_harness,
    create_agent_harness_run_with_metadata, final_agent_harness_assessment, finish_agent_harness,
    mark_agent_harness_ledger_stalled, observe_agent_harness_ledger_progress,
    resolve_agent_harness_max_continuations, should_continue_agent_harness,
    snapshot_agent_harness_ledger, start_agent_harness_heartbeat, AgentHarnessLedgerWatchdog,
    AgentHarnessMode,
};
use crate::models::database::BotExecutionRun;
use crate::services::database::DatabaseService;

const BOT_HARNESS_RUNTIME: &str = "bot_gateway";

pub struct BotExecutionRequest {
    pub run_id: String,
    pub transport: String,
    pub account_id: String,
    pub peer_type: String,
    pub peer_id: String,
    pub sender_id: String,
    pub conversation_id: String,
    pub assistant_profile_id: Option<String>,
    pub trigger_kind: String,
    pub trigger_bot_message_id: Option<String>,
    pub trigger_ai_message_id: Option<String>,
    pub task_text: String,
    pub agent_params: AgentExecuteParams,
}

pub struct BotExecutionOutcome {
    pub run_id: String,
    pub result: Result<String, String>,
}

pub async fn execute_bot_execution(
    app_handle: &AppHandle,
    db: &Arc<DatabaseService>,
    request: BotExecutionRequest,
) -> Result<BotExecutionOutcome, String> {
    if request.run_id != request.agent_params.execution_id {
        return Err("Bot execution request run_id does not match agent execution_id".to_string());
    }
    if request.conversation_id != request.agent_params.storage_conversation_id() {
        return Err(
            "Bot execution request conversation_id does not match agent conversation_id"
                .to_string(),
        );
    }

    let now = Utc::now();
    let harness_mode = AgentHarnessMode::resolve(
        None,
        false,
        request
            .agent_params
            .tool_config
            .as_ref()
            .map(|config| config.enabled)
            .unwrap_or(false),
    )?;

    let run = BotExecutionRun {
        id: request.run_id.clone(),
        transport: request.transport.clone(),
        account_id: request.account_id.clone(),
        peer_type: request.peer_type.clone(),
        peer_id: request.peer_id.clone(),
        sender_id: request.sender_id.clone(),
        conversation_id: request.conversation_id.clone(),
        ai_execution_id: request.agent_params.execution_id.clone(),
        assistant_profile_id: request.assistant_profile_id.clone(),
        trigger_kind: request.trigger_kind.clone(),
        trigger_bot_message_id: request.trigger_bot_message_id.clone(),
        trigger_ai_message_id: request.trigger_ai_message_id.clone(),
        task_text: request.task_text.clone(),
        status: "running".to_string(),
        result_text: None,
        error_message: None,
        started_at: now,
        completed_at: None,
        created_at: now,
        updated_at: now,
    };
    db.create_bot_execution_run(&run)
        .await
        .map_err(|e| format!("Failed to create bot execution run: {e}"))?;

    db.upsert_agent_execution_turn_started(sentinel_db::AgentExecutionTurnStartInput {
        turn_id: request.run_id.clone(),
        conversation_id: request.conversation_id.clone(),
        user_message_id: request.trigger_ai_message_id.clone(),
        parent_turn_id: None,
        task: request.task_text.clone(),
        harness_mode: Some(harness_mode.as_str().to_string()),
    })
    .await
    .map_err(|e| format!("Failed to persist bot execution turn: {e}"))?;

    let (_cancellation_token, generation) = create_cancellation_token(&request.run_id);
    let _guard = CancellationGuard(request.run_id.clone(), generation);

    create_agent_harness_run_with_metadata(
        app_handle,
        &request.run_id,
        &request.conversation_id,
        generation,
        &request.task_text,
        &request.agent_params.model,
        &request.agent_params.rig_provider,
        harness_mode,
        BOT_HARNESS_RUNTIME,
        Some(serde_json::json!({
            "transport": run.transport.clone(),
            "account_id": run.account_id.clone(),
            "peer_type": run.peer_type.clone(),
            "peer_id": run.peer_id.clone(),
            "sender_id": run.sender_id.clone(),
            "trigger_kind": run.trigger_kind.clone(),
            "trigger_bot_message_id": run.trigger_bot_message_id.clone(),
            "trigger_ai_message_id": run.trigger_ai_message_id.clone(),
        })),
    )
    .await;
    let _heartbeat = start_agent_harness_heartbeat(app_handle.clone(), request.run_id.clone());
    append_agent_harness_event(
        app_handle,
        &request.run_id,
        &request.conversation_id,
        generation,
        "generation_started",
        Some(serde_json::json!({
            "execution_id": run.ai_execution_id.clone(),
            "conversation_id": run.conversation_id.clone(),
            "trigger_kind": run.trigger_kind.clone(),
            "transport": run.transport.clone(),
            "persist_messages": request.agent_params.persist_messages,
        })),
    )
    .await;
    checkpoint_agent_harness(
        app_handle,
        &run.id,
        "generation_created",
        Some(serde_json::json!({
            "conversation_id": run.conversation_id.clone(),
            "execution_id": run.ai_execution_id.clone(),
            "generation": generation,
            "transport": run.transport.clone(),
            "account_id": run.account_id.clone(),
        })),
    )
    .await;

    let mut agent_params = request.agent_params;
    agent_params.cancellation_generation = Some(generation);
    agent_params.harness_run_id = Some(run.id.clone());
    let original_task = request.task_text.clone();
    let max_continuations = resolve_agent_harness_max_continuations(None);
    let mut continuation_count = 0usize;
    let mut ledger_watchdog = AgentHarnessLedgerWatchdog::default();

    loop {
        let execution_result =
            crate::agents::execute_agent_turn(app_handle, agent_params.clone()).await;

        if is_conversation_generation_cancelled(&run.ai_execution_id, generation) {
            db.update_bot_execution_run_result(&run.id, "cancelled", None, None, Utc::now())
                .await
                .map_err(|e| format!("Failed to finalize cancelled bot execution run: {e}"))?;
            finish_agent_harness(
                app_handle,
                &run.id,
                &run.conversation_id,
                generation,
                "cancelled",
                None,
            )
            .await;
            emit_agent_execution_finished_for_generation_with_conversation(
                app_handle,
                &run.ai_execution_id,
                &run.conversation_id,
                generation,
                AgentExecutionOutcome::Cancelled,
                None,
                None,
                Some("Execution cancelled by bot operator".to_string()),
            );
            return Ok(BotExecutionOutcome {
                run_id: run.id,
                result: Err("Execution cancelled by bot operator".to_string()),
            });
        }

        match execution_result {
            Ok(turn_outcome) => {
                let response = turn_outcome.final_response.clone();
                append_agent_harness_event(
                    app_handle,
                    &run.id,
                    &run.conversation_id,
                    generation,
                    "model_turn_finished",
                    Some(serde_json::json!({
                        "response_chars": response.chars().count(),
                    })),
                )
                .await;
                let task_assessment =
                    assess_agent_harness_success(app_handle, &run.ai_execution_id, harness_mode)
                        .await;
                let mut harness_success =
                    assess_agent_turn_completion(&turn_outcome, task_assessment, harness_mode);
                let ledger_snapshot =
                    snapshot_agent_harness_ledger(app_handle, &run.ai_execution_id).await;
                let ledger_decision = observe_agent_harness_ledger_progress(
                    harness_mode,
                    &harness_success,
                    ledger_snapshot.as_ref(),
                    &response,
                    &mut ledger_watchdog,
                );
                if matches!(
                    ledger_decision,
                    crate::commands::ai_runtime_harness::AgentHarnessLedgerWatchdogDecision::Stalled
                ) {
                    harness_success = mark_agent_harness_ledger_stalled(
                        harness_success,
                        ledger_watchdog.no_progress_count(),
                    );
                }
                if should_continue_agent_harness(
                    harness_mode,
                    &harness_success,
                    continuation_count,
                    max_continuations,
                ) {
                    checkpoint_agent_harness(
                        app_handle,
                        &run.id,
                        "generation_continuing",
                        Some(agent_harness_continuation_payload(
                            response.chars().count(),
                            harness_mode,
                            &harness_success,
                            continuation_count,
                            max_continuations,
                            ledger_decision.continuation_kind(),
                            ledger_watchdog.no_progress_count(),
                        )),
                    )
                    .await;
                    append_agent_harness_event(
                        app_handle,
                        &run.id,
                        &run.conversation_id,
                        generation,
                        "harness_continuation_scheduled",
                        Some(agent_harness_continuation_payload(
                            response.chars().count(),
                            harness_mode,
                            &harness_success,
                            continuation_count,
                            max_continuations,
                            ledger_decision.continuation_kind(),
                            ledger_watchdog.no_progress_count(),
                        )),
                    )
                    .await;
                    agent_params.task = build_agent_harness_continuation_task(
                        app_handle,
                        &run.ai_execution_id,
                        &original_task,
                        &harness_success,
                        continuation_count,
                        max_continuations,
                        ledger_decision.continuation_kind(),
                    )
                    .await;
                    continuation_count += 1;
                    continue;
                }

                let harness_success = final_agent_harness_assessment(
                    harness_success,
                    continuation_count,
                    max_continuations,
                );
                checkpoint_agent_harness(
                    app_handle,
                    &run.id,
                    if harness_success.succeeded() {
                        "generation_succeeded"
                    } else {
                        "generation_incomplete"
                    },
                    Some(agent_harness_completion_payload(
                        response.chars().count(),
                        harness_mode,
                        &harness_success,
                        continuation_count,
                        max_continuations,
                    )),
                )
                .await;
                finish_agent_harness(
                    app_handle,
                    &run.id,
                    &run.conversation_id,
                    generation,
                    harness_success.state,
                    harness_success.error.as_deref(),
                )
                .await;
                db.update_bot_execution_run_result(
                    &run.id,
                    if harness_success.succeeded() {
                        "completed"
                    } else {
                        harness_success.state
                    },
                    Some(&response),
                    harness_success.error.as_deref(),
                    Utc::now(),
                )
                .await
                .map_err(|e| format!("Failed to finalize bot execution run: {e}"))?;
                emit_agent_execution_finished_for_generation_with_conversation(
                    app_handle,
                    &run.ai_execution_id,
                    &run.conversation_id,
                    generation,
                    if harness_success.succeeded() {
                        AgentExecutionOutcome::Succeeded
                    } else {
                        AgentExecutionOutcome::Failed
                    },
                    harness_success.error.clone(),
                    Some(response.clone()),
                    None,
                );
                return Ok(BotExecutionOutcome {
                    run_id: run.id,
                    result: if harness_success.succeeded() {
                        Ok(response)
                    } else {
                        Err(harness_success.error.unwrap_or_else(|| {
                            "Bot execution finished without a complete task ledger".to_string()
                        }))
                    },
                });
            }
            Err(error) => {
                let error_text = error.to_string();
                finish_agent_harness(
                    app_handle,
                    &run.id,
                    &run.conversation_id,
                    generation,
                    "failed",
                    Some(&error_text),
                )
                .await;
                db.update_bot_execution_run_result(
                    &run.id,
                    "failed",
                    None,
                    Some(&error_text),
                    Utc::now(),
                )
                .await
                .map_err(|e| format!("Failed to finalize bot execution run: {e}"))?;
                emit_agent_execution_finished_for_generation_with_conversation(
                    app_handle,
                    &run.ai_execution_id,
                    &run.conversation_id,
                    generation,
                    AgentExecutionOutcome::Failed,
                    Some(error_text.clone()),
                    None,
                    None,
                );
                return Ok(BotExecutionOutcome {
                    run_id: run.id,
                    result: Err(error_text),
                });
            }
        }
    }
}

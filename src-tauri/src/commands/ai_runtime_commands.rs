use crate::commands::ai::{
    create_cancellation_token, emit_agent_execution_finished_for_generation,
    is_conversation_cancelled, is_conversation_generation_cancelled, perform_rag_enhancement,
    stream_chat_with_llm, AgentExecutionOutcome, CancellationGuard,
    MAX_SAFE_OUTPUT_STORAGE_THRESHOLD, USER_FORCED_RULES_CONFIG_CATEGORY,
    USER_FORCED_RULES_CONFIG_KEY,
};
use crate::commands::ai_task_support::{
    build_virtual_tool_context, complete_external_profile_run_failure,
    complete_external_profile_run_success, load_external_profile_context,
    merge_external_profile_prompt, run_external_text_task, start_external_profile_run,
};
use crate::commands::traffic::TrafficAnalysisState;
use crate::models::attachment::{load_image_from_path, MessageAttachment};
use crate::models::database::{SubagentMessage, SubagentRun};
use crate::services::ai::AiServiceManager;
use crate::services::database::DatabaseService;
use crate::services::model_capabilities::{
    classify_model_vision_capability_error, resolve_model_vision_capability,
    save_cached_model_vision_capability, ModelVisionCapabilityStatus,
};
use crate::services::SystemAgentRuntime;
use chrono::Utc;
use sentinel_db::Database;
use sentinel_workflow::WorkflowGraph;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HandleTaskExecutionStreamRequest {
    pub user_input: String,
    pub conversation_id: String,
    pub message_id: String,
    pub execution_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentExecuteConfig {
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
    pub enable_rag: Option<bool>,
    pub attachments: Option<serde_json::Value>,
    #[serde(default)]
    pub document_attachments:
        Option<Vec<crate::commands::document_commands::ProcessedDocumentResult>>,
    #[serde(default)]
    pub tool_config: Option<crate::agents::ToolConfig>,
    #[serde(default)]
    pub traffic_context: Option<String>,
    #[serde(default)]
    pub display_content: Option<String>,
    #[serde(default)]
    pub current_browser_shell_direct_write_enabled: Option<bool>,
    #[serde(default)]
    pub current_browser_shell_session_id: Option<String>,
    #[serde(default)]
    pub current_terminal_session_fingerprint: Option<String>,
    #[serde(default)]
    pub current_terminal_session_id: Option<String>,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub referenced_files: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub referenced_messages: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub referenced_assets: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub referenced_traffic: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub model_override: Option<String>,
    #[serde(default)]
    pub context_mode: Option<String>,
    #[serde(default)]
    pub max_iterations: Option<usize>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub force_tasks: Option<bool>,
    #[serde(default)]
    pub enable_tenth_man_rule: Option<bool>,
    #[serde(default)]
    pub tenth_man_config: Option<crate::agents::tenth_man::TenthManConfig>,
    #[serde(default)]
    pub persist_messages: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentExecuteRequest {
    pub task: String,
    pub config: Option<AgentExecuteConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectiveImageAttachmentMode {
    Auto,
    LocalOcr,
    ModelVision,
}

fn build_vision_unsupported_message(provider: &str, model_name: &str) -> String {
    format!(
        "Current model does not support image understanding: {}/{}. Switch to a vision-capable model in the conversation work config, or explicitly change image handling to local OCR.",
        provider, model_name
    )
}

fn append_force_tasks_contract(system_prompt: &str, force_tasks: bool) -> String {
    const TASK_COMPLETION_CONTRACT: &str = "[TaskCompletionContract]
- For multi-step work, create and maintain tasks.
- Task status must reflect live progress: when you begin a step, mark it in_progress; when that step finishes or fails, immediately call tasks update_status with completed or failed and record the result.
- Do not batch task updates at the end. Update one step as soon as its work is done, before moving to unrelated work or composing the final answer.
- Do not claim completion or end the task while any task remains pending or in_progress.
- Before the final answer, verify every task is completed or failed.
- If unfinished tasks remain, continue the task instead of ending the response.";

    if !force_tasks || system_prompt.contains("[TaskCompletionContract]") {
        return system_prompt.to_string();
    }

    let trimmed = system_prompt.trim();
    if trimmed.is_empty() {
        TASK_COMPLETION_CONTRACT.to_string()
    } else {
        format!("{}\n\n{}", trimmed, TASK_COMPLETION_CONTRACT)
    }
}

const MAX_UNFINISHED_TASK_RECOVERY_ATTEMPTS: usize = 2;
const MAX_TASK_RECOVERY_RESPONSE_SNIPPET_CHARS: usize = 1200;

fn is_agent_execution_cancelled(execution_id: &str, generation: Option<u64>) -> bool {
    generation
        .map(|value| is_conversation_generation_cancelled(execution_id, value))
        .unwrap_or_else(|| is_conversation_cancelled(execution_id))
}

async fn settle_running_tool_messages(
    app_handle: &AppHandle,
    execution_id: &str,
    terminal_status: &str,
    reason: &str,
) {
    let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() else {
        return;
    };
    if let Err(error) = db
        .settle_running_ai_tool_messages(execution_id, terminal_status, reason)
        .await
    {
        tracing::warn!(
            "Failed to settle running tool messages for {} as {}: {}",
            execution_id,
            terminal_status,
            error
        );
    }
}

async fn create_agent_harness_run(
    app_handle: &AppHandle,
    run_id: &str,
    conversation_id: &str,
    generation: u64,
    task: &str,
    model: &str,
    provider: &str,
) {
    let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() else {
        return;
    };
    if let Err(error) = db
        .create_agent_harness_run(sentinel_db::AgentHarnessRunInput {
            id: run_id.to_string(),
            conversation_id: conversation_id.to_string(),
            generation: generation as i64,
            task: task.to_string(),
            model: Some(model.to_string()),
            provider: Some(provider.to_string()),
            metadata: Some(serde_json::json!({
                "runtime": "ai_assistant",
                "generation": generation,
            })),
        })
        .await
    {
        tracing::warn!(
            "Failed to create agent harness run {} for {}: {}",
            run_id,
            conversation_id,
            error
        );
    }
}

async fn append_agent_harness_event(
    app_handle: &AppHandle,
    run_id: &str,
    conversation_id: &str,
    generation: u64,
    event_type: &str,
    payload: Option<serde_json::Value>,
) {
    let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() else {
        return;
    };
    if let Err(error) = db
        .append_agent_harness_event(
            run_id,
            conversation_id,
            generation as i64,
            event_type,
            payload,
        )
        .await
    {
        tracing::warn!(
            "Failed to append agent harness event {} for {}: {}",
            event_type,
            conversation_id,
            error
        );
    }
}

async fn update_agent_harness_state(
    app_handle: &AppHandle,
    run_id: &str,
    state: &str,
    error: Option<&str>,
    completed: bool,
) {
    let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() else {
        return;
    };
    if let Err(update_error) = db
        .update_agent_harness_state(run_id, state, error, completed.then(chrono::Utc::now))
        .await
    {
        tracing::warn!(
            "Failed to update agent harness run {} to {}: {}",
            run_id,
            state,
            update_error
        );
    }
}

async fn checkpoint_agent_harness(
    app_handle: &AppHandle,
    run_id: &str,
    checkpoint_type: &str,
    payload: Option<serde_json::Value>,
) {
    let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() else {
        return;
    };
    if let Err(error) = db
        .append_agent_harness_checkpoint(run_id, checkpoint_type, payload)
        .await
    {
        tracing::warn!(
            "Failed to append agent harness checkpoint {} for {}: {}",
            checkpoint_type,
            run_id,
            error
        );
    }
}

async fn finish_agent_harness(
    app_handle: &AppHandle,
    run_id: &str,
    conversation_id: &str,
    generation: u64,
    state: &str,
    error: Option<&str>,
) {
    append_agent_harness_event(
        app_handle,
        run_id,
        conversation_id,
        generation,
        "generation_finished",
        Some(serde_json::json!({
            "state": state,
            "error": error,
        })),
    )
    .await;
    update_agent_harness_state(app_handle, run_id, state, error, true).await;
}

#[derive(Debug, Clone)]
struct UnfinishedExecutionTasksState {
    tasks: Vec<sentinel_db::ExecutionTaskItem>,
}

#[derive(Debug, Clone)]
struct UnfinishedTaskRecoveryOutcome {
    response: String,
    unfinished_error: Option<String>,
    stalled: bool,
    remaining_tasks: Vec<serde_json::Value>,
}

fn truncate_for_task_recovery(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let char_count = trimmed.chars().count();
    if char_count <= max_chars {
        return trimmed.to_string();
    }

    let mut truncated = trimmed.chars().take(max_chars).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn build_unfinished_tasks_continuation_prompt(
    execution_id: &str,
    previous_response: &str,
    tasks: &[sentinel_db::ExecutionTaskItem],
    attempt: usize,
    previous_attempt_stalled: bool,
) -> String {
    let remaining = tasks
        .iter()
        .filter(|task| !matches!(task.status.as_str(), "completed" | "failed"))
        .map(|task| {
            let result = task
                .result
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| {
                    format!(
                        " - current_result: {}",
                        truncate_for_task_recovery(value, 240)
                    )
                })
                .unwrap_or_default();
            format!(
                "#{} {} ({}){}",
                task.item_index + 1,
                task.description,
                task.status,
                result
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let response_snippet =
        truncate_for_task_recovery(previous_response, MAX_TASK_RECOVERY_RESPONSE_SNIPPET_CHARS);

    let previous_response_block = if response_snippet.is_empty() {
        String::new()
    } else {
        format!("\n\n[Previous Assistant Response]\n{}", response_snippet)
    };

    let stalled_rules = if previous_attempt_stalled {
        "\n\n[Recovery Escalation]\n\
- The previous harness continuation attempt made no task-state progress.\n\
- A plain analysis-only reply is invalid.\n\
- Before any explanation, you must either perform real tool work that advances the current task and then update `tasks`, or mark the current in_progress task `failed` with a concrete blocker."
    } else {
        ""
    };

    format!(
        "[Runtime Task Recovery]\n\
Execution {} ended a generation turn before all tasks reached a terminal state.\n\
This is harness continuation attempt {}.\n\
\n\
Rules:\n\
- Do not restart from scratch.\n\
- Do not redo tasks already marked completed.\n\
- Continue from the remaining tasks only.\n\
- You may call `tasks` with `action: \"get_list\"` if needed, but do not stop after reading it.\n\
- Immediately call `tasks` with `action: \"update_status\"` when a step finishes or fails.\n\
- If the current step cannot be completed, mark it `failed` with a concise reason instead of leaving it `in_progress`.\n\
- Before ending, verify every task is `completed` or `failed`.\n\
\n\
[Remaining Tasks]\n\
{}\n\
{}\n\
{}\
\n\
\n\
Continue the work now. Prefer tool execution and real task-state updates over re-explaining the plan.",
        execution_id, attempt, remaining, previous_response_block, stalled_rules
    )
}

fn did_unfinished_tasks_progress(
    before: &[sentinel_db::ExecutionTaskItem],
    after: &[sentinel_db::ExecutionTaskItem],
) -> bool {
    if before.len() != after.len() {
        return true;
    }

    before
        .iter()
        .zip(after.iter())
        .any(|(before_task, after_task)| {
            before_task.status != after_task.status || before_task.result != after_task.result
        })
}

#[cfg(test)]
fn build_unfinished_execution_tasks_error(
    execution_id: &str,
    tasks: &[sentinel_db::ExecutionTaskItem],
) -> String {
    let remaining = tasks
        .iter()
        .filter(|task| !matches!(task.status.as_str(), "completed" | "failed"))
        .map(|task| {
            format!(
                "#{} {} ({})",
                task.item_index + 1,
                task.description,
                task.status
            )
        })
        .collect::<Vec<_>>();

    format!(
        "Execution ended before all tasks reached a terminal state for {}. Remaining tasks: {}",
        execution_id,
        remaining.join("; ")
    )
}

fn remaining_execution_tasks_json(
    tasks: &[sentinel_db::ExecutionTaskItem],
) -> Vec<serde_json::Value> {
    tasks
        .iter()
        .filter(|task| !matches!(task.status.as_str(), "completed" | "failed"))
        .map(|task| {
            serde_json::json!({
                "item_index": task.item_index + 1,
                "description": task.description,
                "status": task.status,
                "result": task.result,
            })
        })
        .collect()
}

fn build_unfinished_tasks_stalled_message(
    execution_id: &str,
    tasks: &[sentinel_db::ExecutionTaskItem],
    attempts: usize,
    last_attempt_stalled: bool,
) -> String {
    let remaining = tasks
        .iter()
        .filter(|task| !matches!(task.status.as_str(), "completed" | "failed"))
        .map(|task| {
            format!(
                "#{} {} ({})",
                task.item_index + 1,
                task.description,
                task.status
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let stalled_detail = if last_attempt_stalled {
        " The last continuation made no task-state progress."
    } else {
        ""
    };
    format!(
        "Harness stalled for {} with open execution tasks after {} continuation attempt(s). Remaining tasks: {}.{}",
        execution_id, attempts, remaining, stalled_detail
    )
}

async fn unfinished_execution_tasks_state(
    app_handle: &AppHandle,
    execution_id: &str,
) -> Option<UnfinishedExecutionTasksState> {
    let db = app_handle.try_state::<Arc<DatabaseService>>()?;
    let tasks = match db.get_execution_tasks(execution_id).await {
        Ok(tasks) => tasks,
        Err(error) => {
            tracing::warn!(
                "Failed to verify execution tasks before finishing {}: {}",
                execution_id,
                error
            );
            return None;
        }
    };

    if tasks
        .iter()
        .all(|task| matches!(task.status.as_str(), "completed" | "failed"))
    {
        return None;
    }

    Some(UnfinishedExecutionTasksState { tasks })
}

async fn recover_unfinished_tasks_after_response(
    app_handle: &AppHandle,
    params: &crate::agents::executor::AgentExecuteParams,
    response: String,
) -> UnfinishedTaskRecoveryOutcome {
    let mut latest_response = response;
    let mut previous_attempt_stalled = false;
    let mut last_attempt_stalled = false;

    for attempt in 1..=MAX_UNFINISHED_TASK_RECOVERY_ATTEMPTS {
        let Some(state) = unfinished_execution_tasks_state(app_handle, &params.execution_id).await
        else {
            return UnfinishedTaskRecoveryOutcome {
                response: latest_response,
                unfinished_error: None,
                stalled: false,
                remaining_tasks: Vec::new(),
            };
        };

        tracing::warn!(
            "Detected unfinished tasks after model response for {}. Starting harness continuation attempt {}",
            params.execution_id,
            attempt
        );

        let remaining_tasks = remaining_execution_tasks_json(&state.tasks);

        let _ = app_handle.emit(
            "ai_meta_info",
            &serde_json::json!({
                "conversation_id": params.execution_id,
                "unfinished_task_recovery": {
                    "attempt": attempt,
                    "max_attempts": MAX_UNFINISHED_TASK_RECOVERY_ATTEMPTS,
                    "remaining_tasks": remaining_tasks,
                }
            }),
        );

        if is_agent_execution_cancelled(&params.execution_id, params.cancellation_generation) {
            return UnfinishedTaskRecoveryOutcome {
                response: latest_response,
                unfinished_error: Some("Execution cancelled by user".to_string()),
                stalled: false,
                remaining_tasks,
            };
        }

        let mut continuation_params = params.clone();
        continuation_params.task = build_unfinished_tasks_continuation_prompt(
            &params.execution_id,
            &latest_response,
            &state.tasks,
            attempt,
            previous_attempt_stalled,
        );

        match crate::agents::executor::execute_agent(app_handle, continuation_params).await {
            Ok(next_response) => {
                if !next_response.trim().is_empty() {
                    latest_response = next_response;
                }

                let state_after =
                    unfinished_execution_tasks_state(app_handle, &params.execution_id).await;
                let Some(after_state) = state_after else {
                    return UnfinishedTaskRecoveryOutcome {
                        response: latest_response,
                        unfinished_error: None,
                        stalled: false,
                        remaining_tasks: Vec::new(),
                    };
                };

                last_attempt_stalled =
                    !did_unfinished_tasks_progress(&state.tasks, &after_state.tasks);
                previous_attempt_stalled = last_attempt_stalled;

                if last_attempt_stalled {
                    let _ = app_handle.emit(
                        "ai_meta_info",
                        &serde_json::json!({
                            "conversation_id": params.execution_id,
                            "generation": params.cancellation_generation,
                            "unfinished_task_recovery": {
                                "attempt": attempt,
                                "stalled": true,
                                "message": "Harness continuation attempt made no task-state progress."
                            }
                        }),
                    );
                }
            }
            Err(error) => {
                return UnfinishedTaskRecoveryOutcome {
                    response: latest_response,
                    unfinished_error: Some(format!(
                        "Harness continuation attempt {} failed for {}: {}",
                        attempt, params.execution_id, error
                    )),
                    stalled: false,
                    remaining_tasks: remaining_execution_tasks_json(&state.tasks),
                };
            }
        }
    }

    let unfinished_state = unfinished_execution_tasks_state(app_handle, &params.execution_id).await;
    let (unfinished_error, remaining_tasks) = unfinished_state
        .map(|state| {
            (
                build_unfinished_tasks_stalled_message(
                    &params.execution_id,
                    &state.tasks,
                    MAX_UNFINISHED_TASK_RECOVERY_ATTEMPTS,
                    last_attempt_stalled,
                ),
                remaining_execution_tasks_json(&state.tasks),
            )
        })
        .map_or((None, Vec::new()), |(message, remaining)| {
            (Some(message), remaining)
        });

    let stalled = unfinished_error.is_some();
    UnfinishedTaskRecoveryOutcome {
        response: latest_response,
        unfinished_error,
        stalled,
        remaining_tasks,
    }
}

async fn emit_success_outcome(
    app_handle: &AppHandle,
    execution_id: &str,
    generation: u64,
    response: String,
) {
    emit_agent_execution_finished_for_generation(
        app_handle,
        execution_id,
        generation,
        AgentExecutionOutcome::Succeeded,
        None,
        Some(response),
        None,
    );
}

pub async fn get_subagent_runs(
    parent_execution_id: String,
    db_service: Arc<DatabaseService>,
) -> Result<Vec<SubagentRun>, String> {
    db_service
        .get_subagent_runs_by_parent_internal(&parent_execution_id)
        .await
        .map_err(|e| {
            format!(
                "Failed to get subagent runs for {}: {}",
                parent_execution_id, e
            )
        })
}

pub async fn get_subagent_messages(
    subagent_run_id: String,
    db_service: Arc<DatabaseService>,
) -> Result<Vec<SubagentMessage>, String> {
    db_service
        .get_subagent_messages_by_run_internal(&subagent_run_id)
        .await
        .map_err(|e| {
            format!(
                "Failed to get subagent messages for {}: {}",
                subagent_run_id, e
            )
        })
}

pub async fn delete_subagent_runs_after(
    parent_execution_id: String,
    after_timestamp_ms: i64,
    db_service: Arc<DatabaseService>,
) -> Result<u64, String> {
    use chrono::TimeZone;

    let timestamp = Utc
        .timestamp_millis_opt(after_timestamp_ms)
        .single()
        .ok_or_else(|| "Invalid timestamp".to_string())?;

    db_service
        .delete_subagent_runs_after_internal(&parent_execution_id, timestamp)
        .await
        .map_err(|e| format!("Failed to delete subagent runs: {}", e))
}

pub async fn clear_conversation_messages(
    conversation_id: String,
    db_service: Arc<DatabaseService>,
) -> Result<(), String> {
    db_service
        .delete_ai_messages_by_conversation(&conversation_id)
        .await
        .map_err(|e: anyhow::Error| {
            format!(
                "Failed to clear messages for conversation {}: {}",
                conversation_id, e
            )
        })?;

    if let Err(e) = db_service
        .delete_sliding_window_summaries(&conversation_id)
        .await
    {
        tracing::warn!(
            "Failed to clear sliding window summaries for {}: {}",
            conversation_id,
            e
        );
    }

    if let Err(e) = db_service.delete_agent_run_state(&conversation_id).await {
        tracing::warn!(
            "Failed to clear agent run_state for {}: {}",
            conversation_id,
            e
        );
    }

    if let Err(e) = db_service.delete_execution_tasks(&conversation_id).await {
        tracing::warn!(
            "Failed to clear execution_tasks for {}: {}",
            conversation_id,
            e
        );
    }

    Ok(())
}

pub async fn save_tool_config(
    tool_config: crate::agents::ToolConfig,
    app_handle: AppHandle,
) -> Result<(), String> {
    save_tool_config_to_db(&app_handle, &tool_config).await
}

pub async fn get_tool_config(
    app_handle: AppHandle,
) -> Result<Option<crate::agents::ToolConfig>, String> {
    Ok(load_tool_config_from_db(&app_handle).await)
}

pub async fn generate_workflow_from_nl(
    description: String,
    ai_manager: Arc<AiServiceManager>,
    traffic_state: &TrafficAnalysisState,
    system_agent_runtime: Arc<SystemAgentRuntime>,
) -> Result<WorkflowGraph, String> {
    let desc = description.trim();
    if desc.is_empty() {
        return Err("description is empty".to_string());
    }

    let agent_context = load_external_profile_context(
        &system_agent_runtime,
        "workflow_designer_agent",
        &["tool_catalog_reader", "workflow_catalog_reader"],
    )
    .await
    .map_err(|e| e.to_string())?;

    let virtual_tool_sections = build_virtual_tool_context(&agent_context, Some(traffic_state))
        .await
        .map_err(|e| e.to_string())?;
    let virtual_tool_context = if virtual_tool_sections.is_empty() {
        String::new()
    } else {
        format!("\n{}\n", virtual_tool_sections.join("\n\n"))
    };

    let tracked_run = start_external_profile_run(
        &system_agent_runtime,
        "workflow_designer_agent",
        serde_json::json!({
            "description": desc,
            "declaredTools": agent_context
                .tool_policy
                .as_ref()
                .map(|policy| policy.declared_tools())
                .unwrap_or_default(),
        }),
    )
    .await;

    let system_prompt = format!(
        r#"You are a workflow design assistant for Sentinel AI.
Based on the user's natural language description, output a WorkflowGraph that strictly conforms to the following JSON Schema.
Only output JSON, do not explain, do not include Markdown.

{}

Schema:
{{
  "id": "string",
  "name": "string",
  "version": "string",
  "nodes": [
    {{
      "id": "string",
      "node_type": "string",
      "node_name": "string",
      "x": number,
      "y": number,
      "params": {{...actual parameters for this node type...}},
      "input_ports": [{{"id":"in","name":"输入","port_type":"String","required":false}}],
      "output_ports": [{{"id":"out","name":"输出","port_type":"String","required":false}}]
    }}
  ],
  "edges": [
    {{
      "id":"string",
      "from_node":"string",
      "from_port":"out",
      "to_node":"string",
      "to_port":"in",
      "source_scope":"output",
      "source_path":"response",
      "target_path":"prompt",
      "merge_mode":"replace"
    }}
  ],
  "variables": [],
  "credentials": []
}}

CRITICAL RULES:
1) node_type selection:
   - Use "trigger_schedule" for scheduled/timed triggers
   - Use "tool::browser" for opening URLs and web scraping
   - Use "tool::http_request" for HTTP API calls
   - Use "ai_chat" for AI text generation/summarization
   - Use "notify" for sending notifications/emails
   - Use "raw" for static JSON/text input data
   - Use "start" for manual trigger entry point

2) params MUST contain actual values extracted from user description:
   - For "trigger_schedule": {{"trigger_type":"daily","hour":8,"minute":0,"second":0,"weekdays":"1,2,3,4,5"}}
   - For "tool::browser": {{"url":"https://example.com","action":"navigate","wait_until":"networkidle"}}
   - For "tool::http_request": {{"url":"https://api.example.com","method":"GET"}}
   - For "ai_chat": {{"prompt":"Summarize the following content: {{{{input}}}}","system_prompt":"You are a helpful assistant"}}
   - For "notify": {{"title":"Notification","content":"{{{{input}}}}","use_input_as_content":true}}
   - For "raw": {{"raw_type":"json","value":"{{\"query\":\"漏洞情报\"}}"}}

3) Extract specific values from user description:
   - Times like "8点" -> hour:8, minute:0
   - URLs mentioned -> put in url parameter
   - Email/notification requirements -> use notify node

4) Layout: x increases left-to-right (0, 250, 500...), y for parallel branches

5) Keep variables and credentials as empty arrays []

6) Every node MUST have meaningful params filled based on its purpose in the workflow
7) Every edge MUST include a non-empty target_path that maps data into the downstream node params
8) source_scope must be "output" or "input"; merge_mode must be "replace", "deep_merge", or "append"
"#,
        virtual_tool_context
    );
    let system_prompt =
        merge_external_profile_prompt(Some(system_prompt), &agent_context).unwrap_or_default();

    let user_prompt = format!("用户描述：{}\n请生成 WorkflowGraph JSON。", desc);
    let run_id = tracked_run
        .as_ref()
        .map(|run| run.run_id.as_str())
        .unwrap_or("workflow-designer-ad-hoc");
    let raw = match run_external_text_task(
        &system_agent_runtime.app_handle(),
        &ai_manager,
        "workflow_designer_agent",
        run_id,
        &agent_context,
        Some(system_prompt.clone()),
        user_prompt,
    )
    .await
    {
        Ok(raw) => raw,
        Err(e) => {
            complete_external_profile_run_failure(
                &system_agent_runtime,
                &tracked_run,
                e.to_string(),
            )
            .await;
            return Err(e.to_string());
        }
    };

    let json_str = raw.trim();
    let parsed_value: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e0) => {
            if let (Some(s), Some(e)) = (json_str.find('{'), json_str.rfind('}')) {
                serde_json::from_str(&json_str[s..=e])
                    .map_err(|e| format!("Failed to parse extracted JSON: {}", e))?
            } else {
                let error = format!("Failed to parse LLM output as JSON: {}", e0);
                complete_external_profile_run_failure(&system_agent_runtime, &tracked_run, &error)
                    .await;
                return Err(error);
            }
        }
    };

    let mut graph: WorkflowGraph = match serde_json::from_value(parsed_value) {
        Ok(graph) => graph,
        Err(e) => {
            let error = format!("Failed to parse workflow graph: {}", e);
            complete_external_profile_run_failure(&system_agent_runtime, &tracked_run, &error)
                .await;
            return Err(error);
        }
    };

    if graph.id.trim().is_empty() {
        graph.id = format!("wf_{}", Utc::now().timestamp_millis());
    }
    if graph.name.trim().is_empty() {
        graph.name = "AI生成工作流".to_string();
    }
    if graph.version.trim().is_empty() {
        graph.version = "0.1.0".to_string();
    }
    if graph.variables.is_empty() {
        graph.variables = vec![];
    }
    if graph.credentials.is_empty() {
        graph.credentials = vec![];
    }

    complete_external_profile_run_success(
        &system_agent_runtime,
        &tracked_run,
        serde_json::json!({ "workflowGraph": &graph }),
    )
    .await;

    Ok(graph)
}

pub async fn save_scheduler_config(
    config: crate::services::ai::SchedulerConfig,
    db: Arc<DatabaseService>,
) -> Result<(), String> {
    tracing::info!("Saving scheduler configuration");

    db.set_config(
        "scheduler",
        "intent_analysis_model",
        &config.intent_analysis_model,
        Some("Intent analysis model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "intent_analysis_provider",
        &config.intent_analysis_provider,
        Some("Intent analysis provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "planner_model",
        &config.planner_model,
        Some("Planner model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "planner_provider",
        &config.planner_provider,
        Some("Planner provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "replanner_model",
        &config.replanner_model,
        Some("Replanner model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "replanner_provider",
        &config.replanner_provider,
        Some("Replanner provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "executor_model",
        &config.executor_model,
        Some("Executor model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "executor_provider",
        &config.executor_provider,
        Some("Executor provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "evaluator_model",
        &config.evaluator_model,
        Some("Evaluator model for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "evaluator_provider",
        &config.evaluator_provider,
        Some("Evaluator provider for scheduler"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "default_strategy",
        &config.default_strategy,
        Some("Default replanning strategy"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "enabled",
        &config.enabled.to_string(),
        Some("Scheduler enabled status"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "max_retries",
        &config.max_retries.to_string(),
        Some("Maximum retry attempts"),
    )
    .await
    .map_err(|e| e.to_string())?;

    db.set_config(
        "scheduler",
        "timeout_seconds",
        &config.timeout_seconds.to_string(),
        Some("Timeout in seconds"),
    )
    .await
    .map_err(|e| e.to_string())?;

    let scenarios_str = serde_json::to_string(&config.scenarios)
        .map_err(|e| format!("Failed to serialize scenarios: {}", e))?;
    db.set_config(
        "scheduler",
        "scenarios",
        &scenarios_str,
        Some("Scenario configurations"),
    )
    .await
    .map_err(|e| e.to_string())?;

    tracing::info!("Successfully saved scheduler configuration");
    Ok(())
}

pub async fn refresh_lm_studio_models(
    _api_base: Option<String>,
    _api_key: Option<String>,
) -> Result<Vec<String>, String> {
    Err("LM Studio model refresh disabled - ai_adapter removed, use Rig instead".to_string())
}

pub async fn get_lm_studio_status(
    _api_base: Option<String>,
    _api_key: Option<String>,
) -> Result<serde_json::Value, String> {
    Err("LM Studio status check disabled - ai_adapter removed, use Rig instead".to_string())
}

pub async fn test_lm_studio_provider_connection(
    _api_base: Option<String>,
    _api_key: Option<String>,
) -> Result<crate::commands::aisettings::TestConnectionResponse, String> {
    Err("LM Studio provider test disabled - ai_adapter removed, use Rig instead".to_string())
}

pub async fn upload_image_attachment(file_path: String) -> Result<serde_json::Value, String> {
    tracing::info!("上传图片附件: {}", file_path);

    match load_image_from_path(&file_path).await {
        Ok(image_attachment) => {
            let attachment = MessageAttachment::Image(image_attachment);
            serde_json::to_value(&attachment).map_err(|e| format!("序列化图片附件失败: {}", e))
        }
        Err(e) => {
            tracing::error!("加载图片失败: {}", e);
            Err(format!("加载图片失败: {}", e))
        }
    }
}

pub async fn upload_multiple_images(
    file_paths: Vec<String>,
) -> Result<Vec<serde_json::Value>, String> {
    tracing::info!("批量上传 {} 个图片", file_paths.len());

    let mut attachments = Vec::new();
    let mut errors = Vec::new();

    for file_path in file_paths {
        match load_image_from_path(&file_path).await {
            Ok(image_attachment) => {
                let attachment = MessageAttachment::Image(image_attachment);
                if let Ok(value) = serde_json::to_value(&attachment) {
                    attachments.push(value);
                } else {
                    errors.push(format!("序列化失败: {}", file_path));
                }
            }
            Err(e) => errors.push(format!("{}: {}", file_path, e)),
        }
    }

    if !errors.is_empty() {
        tracing::warn!("部分图片上传失败: {:?}", errors);
    }

    if attachments.is_empty() {
        Err(format!("所有图片上传失败: {:?}", errors))
    } else {
        Ok(attachments)
    }
}

pub async fn agent_execute(
    task: String,
    config: Option<AgentExecuteConfig>,
    app_handle: AppHandle,
    ai_manager: Arc<AiServiceManager>,
) -> Result<String, String> {
    sentinel_license::ensure_feature_access(sentinel_license::LicensedFeature::AiRuntime)?;

    let config = config.unwrap_or(AgentExecuteConfig {
        conversation_id: None,
        message_id: None,
        enable_rag: Some(false),
        attachments: None,
        document_attachments: None,
        tool_config: None,
        traffic_context: None,
        display_content: None,
        current_browser_shell_direct_write_enabled: None,
        current_browser_shell_session_id: None,
        current_terminal_session_fingerprint: None,
        current_terminal_session_id: None,
        working_directory: None,
        referenced_files: None,
        referenced_messages: None,
        referenced_assets: None,
        referenced_traffic: None,
        model_override: None,
        context_mode: None,
        max_iterations: None,
        timeout_secs: None,
        force_tasks: None,
        enable_tenth_man_rule: None,
        tenth_man_config: None,
        persist_messages: None,
    });

    let conversation_id = config
        .conversation_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let message_id = config
        .message_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let enable_rag = config.enable_rag.unwrap_or(false);
    let force_tasks = config.force_tasks.unwrap_or(false);
    let persist_messages = config.persist_messages.unwrap_or(true);
    let requested_working_directory = config
        .working_directory
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    let effective_working_directory = if let Some(working_directory) = requested_working_directory {
        Some(working_directory)
    } else if let Some(db_service) = app_handle.try_state::<Arc<DatabaseService>>() {
        crate::commands::ai_conversation_binding_support::resolve_effective_conversation_working_directory(
            db_service.inner().as_ref(),
            Some(conversation_id.as_str()),
        )
        .await?
    } else {
        None
    };
    let raw_attachments = config.attachments.clone();
    let attachments_for_save = raw_attachments.as_ref().map(sanitize_image_attachments);
    let document_attachments_for_save = config.document_attachments.clone();
    let referenced_files_for_save = config.referenced_files.clone();
    let referenced_messages_for_save = config.referenced_messages.clone();
    let referenced_assets_for_save = config.referenced_assets.clone();
    let referenced_traffic_for_save = config.referenced_traffic.clone();

    let effective_tool_config = if config.tool_config.is_some() {
        tracing::info!("Using tool config from frontend request");
        config.tool_config.clone()
    } else {
        load_tool_config_from_db(&app_handle).await
    };

    tracing::info!(
        "Agent execute: conv={}, msg={}, rag={}, tools={}",
        conversation_id,
        message_id,
        enable_rag,
        effective_tool_config
            .as_ref()
            .map(|c| c.enabled)
            .unwrap_or(false)
    );

    if let Err(e) =
        crate::commands::ai_execution_state_support::clear_persisted_agent_execution_state(
            &app_handle,
            &conversation_id,
        )
        .await
    {
        tracing::warn!(
            "Failed to clear persisted execution state for conversation {}: {}",
            conversation_id,
            e
        );
    }
    if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
        if let Err(e) = db
            .settle_running_ai_tool_messages(
                &conversation_id,
                "timed_out",
                "Previous generation ended before the tool result was recorded",
            )
            .await
        {
            tracing::warn!(
                "Failed to settle stale running tool messages for {}: {}",
                conversation_id,
                e
            );
        }
    }

    let requested_override = config
        .model_override
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| {
            v.split_once('/')
                .map(|(p, m)| (p.trim().to_string(), m.trim().to_string()))
        });

    let (provider, model_name) = match requested_override {
        Some(Some((provider, model))) if !provider.is_empty() && !model.is_empty() => {
            tracing::info!("Using assistant model override: {}/{}", provider, model);
            (provider, model)
        }
        Some(_) => {
            return Err("Invalid model_override format, expected 'provider/model_name'".to_string())
        }
        None => match ai_manager.get_default_llm_model().await {
            Ok(Some((p, m))) => {
                tracing::info!("Using default chat model: {}/{}", p, m);
                (p, m)
            }
            Ok(None) => return Err("Default chat model is not configured".to_string()),
            Err(e) => return Err(format!("Failed to read default chat model: {}", e)),
        },
    };

    let provider_config = ai_manager
        .get_provider_config(&provider)
        .await
        .map_err(|e| format!("Failed to load provider config '{}': {}", provider, e))?
        .ok_or_else(|| format!("Provider '{}' configuration not found", provider))?;

    let mut dynamic_config = provider_config.clone();
    dynamic_config.model = model_name.clone();

    let db_service = app_handle.state::<Arc<DatabaseService>>();
    let (configured_image_mode, allow_image_upload_to_model) =
        load_image_attachment_settings(db_service.inner()).await;
    let resolved_model_vision_capability = resolve_model_vision_capability(
        db_service.inner(),
        &provider,
        &model_name,
        provider_config.api_base.as_deref(),
        provider_config.rig_provider.as_deref(),
    )
    .await;
    let mut service = crate::services::ai::AiService::new(
        dynamic_config,
        db_service.inner().clone(),
        Some(app_handle.clone()),
    );
    service.set_app_handle(app_handle.clone());

    if let Ok(threshold_str_opt) = db_service
        .get_config_internal("ai", "output_storage_threshold")
        .await
    {
        if let Some(threshold_str) = threshold_str_opt {
            if let Ok(threshold) = threshold_str.parse::<usize>() {
                let effective_threshold = threshold.min(MAX_SAFE_OUTPUT_STORAGE_THRESHOLD);
                if effective_threshold != threshold {
                    tracing::warn!(
                        "Configured output storage threshold {} is too high, clamped to {} bytes for stream stability",
                        threshold,
                        effective_threshold
                    );
                } else {
                    tracing::info!(
                        "Setting output storage threshold to {} bytes (Dynamic Context Discovery)",
                        effective_threshold
                    );
                }
                sentinel_tools::set_storage_threshold(effective_threshold);
            }
        }
    }

    let (_cancellation_token, cancel_gen) = create_cancellation_token(&conversation_id);
    let harness_run_id = Uuid::new_v4().to_string();

    let service_clone = service.clone();
    let conv_id = conversation_id.clone();
    let msg_id = message_id.clone();
    let task_clone = task.clone();
    let display_content_clone = config.display_content.clone();
    let mut base_system_prompt: Option<String> = None;

    let provider_for_closure = if provider_config.rig_provider.is_some() {
        provider_config.rig_provider.clone().unwrap()
    } else {
        provider_config.provider.clone()
    };

    let model_name_for_closure = model_name.clone();
    let provider_config_for_closure = provider_config.clone();
    let provider_for_capability_cache = provider.clone();
    let model_for_capability_cache = model_name.clone();
    let api_base_for_capability_cache = provider_config.api_base.clone();
    let rig_provider_for_capability_cache = provider_config.rig_provider.clone();

    tokio::spawn(async move {
        let _guard = CancellationGuard(conv_id.clone(), cancel_gen);
        create_agent_harness_run(
            &app_handle,
            &harness_run_id,
            &conv_id,
            cancel_gen,
            &task_clone,
            &model_name_for_closure,
            &provider_for_closure,
        )
        .await;
        append_agent_harness_event(
            &app_handle,
            &harness_run_id,
            &conv_id,
            cancel_gen,
            "generation_started",
            Some(serde_json::json!({
                "message_id": msg_id.clone(),
                "persist_messages": persist_messages,
            })),
        )
        .await;
        checkpoint_agent_harness(
            &app_handle,
            &harness_run_id,
            "generation_created",
            Some(serde_json::json!({
                "conversation_id": conv_id.clone(),
                "generation": cancel_gen,
            })),
        )
        .await;
        if persist_messages {
            if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
                let conversation_exists = db
                    .get_ai_conversation(&conv_id)
                    .await
                    .map(|c| c.is_some())
                    .unwrap_or(false);

                if !conversation_exists {
                    use sentinel_core::models::database as core_db;
                    let new_conv = core_db::AiConversation {
                        id: conv_id.clone(),
                        title: Some(task_clone.chars().take(50).collect::<String>()),
                        service_name: "default".to_string(),
                        model_name: "default".to_string(),
                        model_provider: None,
                        context_type: None,
                        project_id: None,
                        vulnerability_id: None,
                        scan_task_id: None,
                        conversation_data: None,
                        summary: None,
                        total_messages: 0,
                        total_tokens: 0,
                        cost: 0.0,
                        tags: None,
                        tool_config: None,
                        is_archived: false,
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                    };
                    if let Err(e) = db.create_ai_conversation(&new_conv).await {
                        tracing::warn!("Failed to create conversation: {}", e);
                    }
                }

                use sentinel_core::models::database as core_db;
                let user_msg_id = Uuid::new_v4().to_string();
                let display_text = display_content_clone.as_ref().unwrap_or(&task_clone);

                let structured_data = {
                    let mut data = serde_json::json!({});
                    if let Some(ref content) = display_content_clone {
                        data["display_content"] = serde_json::json!(content);
                    }
                    if let Some(ref doc_atts) = document_attachments_for_save {
                        if !doc_atts.is_empty() {
                            data["document_attachments"] =
                                serde_json::to_value(doc_atts).unwrap_or_default();
                        }
                    }
                    if let Some(ref files) = referenced_files_for_save {
                        if !files.is_empty() {
                            data["referenced_files"] = serde_json::json!(files);
                        }
                    }
                    if let Some(ref messages) = referenced_messages_for_save {
                        if !messages.is_empty() {
                            data["referenced_messages"] = serde_json::json!(messages);
                        }
                    }
                    if let Some(ref assets) = referenced_assets_for_save {
                        if !assets.is_empty() {
                            data["referenced_assets"] = serde_json::json!(assets);
                        }
                    }
                    if let Some(ref traffic) = referenced_traffic_for_save {
                        if !traffic.is_empty() {
                            data["referenced_traffic"] = serde_json::json!(traffic);
                        }
                    }
                    if data.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                        None
                    } else {
                        Some(data.to_string())
                    }
                };

                let metadata = {
                    let mut meta = serde_json::json!({});
                    if let Some(ref atts) = attachments_for_save {
                        meta["image_attachments"] = atts.clone();
                    }
                    if let Some(ref doc_atts) = document_attachments_for_save {
                        if !doc_atts.is_empty() {
                            meta["document_attachments"] =
                                serde_json::to_value(doc_atts).unwrap_or_default();
                        }
                    }
                    if let Some(ref files) = referenced_files_for_save {
                        if !files.is_empty() {
                            meta["referenced_files"] = serde_json::json!(files);
                        }
                    }
                    if let Some(ref messages) = referenced_messages_for_save {
                        if !messages.is_empty() {
                            meta["referenced_messages"] = serde_json::json!(messages);
                        }
                    }
                    if let Some(ref assets) = referenced_assets_for_save {
                        if !assets.is_empty() {
                            meta["referenced_assets"] = serde_json::json!(assets);
                        }
                    }
                    if let Some(ref traffic) = referenced_traffic_for_save {
                        if !traffic.is_empty() {
                            meta["referenced_traffic"] = serde_json::json!(traffic);
                        }
                    }
                    if meta.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                        None
                    } else {
                        Some(meta.to_string())
                    }
                };

                let user_msg = core_db::AiMessage {
                    id: user_msg_id.clone(),
                    conversation_id: conv_id.clone(),
                    role: "user".to_string(),
                    content: task_clone.clone(),
                    metadata,
                    token_count: Some(task_clone.len() as i32),
                    cost: None,
                    tool_calls: None,
                    attachments: attachments_for_save
                        .as_ref()
                        .and_then(|v| serde_json::to_string(v).ok()),
                    reasoning_content: None,
                    timestamp: chrono::Utc::now(),
                    architecture_type: None,
                    architecture_meta: None,
                    structured_data,
                };
                if let Err(e) = db.create_ai_message(&user_msg).await {
                    tracing::warn!("Failed to save user message: {}", e);
                } else {
                    let _ = app_handle.emit(
                        "agent:user_message",
                        &serde_json::json!({
                            "execution_id": conv_id,
                            "generation": cancel_gen,
                            "message_id": user_msg_id,
                            "content": display_text,
                            "timestamp": user_msg.timestamp.timestamp_millis(),
                            "document_attachments": document_attachments_for_save,
                            "image_attachments": attachments_for_save,
                            "referenced_files": referenced_files_for_save,
                            "referenced_messages": referenced_messages_for_save,
                            "referenced_assets": referenced_assets_for_save,
                            "referenced_traffic": referenced_traffic_for_save,
                        }),
                    );
                }
            }
        }

        let mut role_prompt = String::new();
        if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
            if let Ok(Some(current_role)) = db.get_current_ai_role().await {
                if !current_role.prompt.trim().is_empty() {
                    role_prompt = current_role.prompt;
                    tracing::info!("Using role prompt: {}", current_role.title);
                }
            }
        }

        if !role_prompt.is_empty() {
            base_system_prompt = match base_system_prompt {
                Some(existing) if !existing.trim().is_empty() => {
                    Some(format!("{}\n\n{}", role_prompt, existing))
                }
                _ => Some(role_prompt),
            };
        }

        if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
            if let Ok(Some(raw_rules)) = db
                .get_config(
                    USER_FORCED_RULES_CONFIG_CATEGORY,
                    USER_FORCED_RULES_CONFIG_KEY,
                )
                .await
            {
                let forced_rules = raw_rules.trim();
                if !forced_rules.is_empty() {
                    let rules_block = format!(
                        "[User Forced Rules]\n{}\n\n[Rule Priority]\n- The above rules are user-defined mandatory instructions. Follow them unless they conflict with higher-priority safety/system constraints.",
                        forced_rules
                    );
                    base_system_prompt = match base_system_prompt {
                        Some(existing) if !existing.trim().is_empty() => {
                            Some(format!("{}\n\n{}", existing, rules_block))
                        }
                        _ => Some(rules_block),
                    };
                    tracing::info!("Injected user forced rules into system prompt");
                }
            }
        }

        base_system_prompt = Some(append_force_tasks_contract(
            base_system_prompt.as_deref().unwrap_or_default(),
            force_tasks,
        ));

        let mut augmented_task = task_clone.clone();

        if let Some(ref raw) = raw_attachments {
            let shell_cfg = sentinel_tools::buildin_tools::shell::get_shell_config().await;
            if shell_cfg.default_execution_mode
                == sentinel_tools::buildin_tools::shell::ShellExecutionMode::Docker
            {
                match crate::utils::image_ocr::stage_images_to_docker_context(raw).await {
                    Ok(paths) => {
                        if !paths.is_empty() {
                            let mut lines = Vec::new();
                            for p in paths {
                                let name = p
                                    .filename
                                    .as_deref()
                                    .filter(|s| !s.trim().is_empty())
                                    .unwrap_or("image");
                                lines.push(format!("- {}: {}", name, p.container_path));
                            }
                            augmented_task = format!(
                                "[Image Files in Docker]\n{}\n\n{}",
                                lines.join("\n"),
                                augmented_task
                            );
                        }
                    }
                    Err(e) => tracing::warn!("Failed to stage images to docker context: {}", e),
                }
            }
        }

        let effective_mode = match configured_image_mode {
            EffectiveImageAttachmentMode::Auto => {
                if allow_image_upload_to_model
                    && resolved_model_vision_capability.status
                        != ModelVisionCapabilityStatus::Unsupported
                {
                    EffectiveImageAttachmentMode::ModelVision
                } else {
                    EffectiveImageAttachmentMode::LocalOcr
                }
            }
            EffectiveImageAttachmentMode::LocalOcr => EffectiveImageAttachmentMode::LocalOcr,
            EffectiveImageAttachmentMode::ModelVision => {
                if allow_image_upload_to_model
                    && resolved_model_vision_capability.status
                        != ModelVisionCapabilityStatus::Unsupported
                {
                    EffectiveImageAttachmentMode::ModelVision
                } else {
                    EffectiveImageAttachmentMode::LocalOcr
                }
            }
        };

        if raw_attachments
            .as_ref()
            .and_then(|value| value.as_array())
            .map(|items| !items.is_empty())
            .unwrap_or(false)
        {
            tracing::info!(
                "Image attachment routing: provider={}, model={}, configured_mode={:?}, vision_status={:?}, vision_source={:?}, allow_upload_to_model={}, effective_mode={:?}",
                provider,
                model_name,
                configured_image_mode,
                resolved_model_vision_capability.status,
                resolved_model_vision_capability.source,
                allow_image_upload_to_model,
                effective_mode
            );
        }

        if raw_attachments
            .as_ref()
            .and_then(|value| value.as_array())
            .map(|items| !items.is_empty())
            .unwrap_or(false)
            && configured_image_mode != EffectiveImageAttachmentMode::LocalOcr
            && resolved_model_vision_capability.status == ModelVisionCapabilityStatus::Unsupported
        {
            let error_message = build_vision_unsupported_message(&provider, &model_name);
            tracing::warn!(
                "Rejecting image request for unsupported vision model {} / {}",
                provider,
                model_name
            );
            finish_agent_harness(
                &app_handle,
                &harness_run_id,
                &conv_id,
                cancel_gen,
                "failed",
                Some(&error_message),
            )
            .await;
            emit_agent_execution_finished_for_generation(
                &app_handle,
                &conv_id,
                cancel_gen,
                AgentExecutionOutcome::Failed,
                Some(error_message),
                None,
                None,
            );
            return;
        }

        let image_attachments_for_execution: Option<serde_json::Value> = match effective_mode {
            EffectiveImageAttachmentMode::Auto => None,
            EffectiveImageAttachmentMode::LocalOcr => {
                if let Some(ref raw) = raw_attachments {
                    match crate::utils::image_ocr::ocr_images_from_attachments(raw).await {
                        Ok(results) => {
                            let ctx = crate::utils::image_ocr::format_ocr_context(&results, 8000);
                            if !ctx.trim().is_empty() {
                                augmented_task =
                                    format!("[Image OCR]\n{}\n\n{}", ctx, augmented_task);
                            }
                        }
                        Err(e) => tracing::warn!("Image OCR failed: {}", e),
                    }
                }
                None
            }
            EffectiveImageAttachmentMode::ModelVision => attachments_for_save.clone(),
        };
        let attempted_model_vision =
            matches!(effective_mode, EffectiveImageAttachmentMode::ModelVision)
                && image_attachments_for_execution.is_some();

        if enable_rag {
            if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
                let history_messages = match db.get_ai_messages_by_conversation(&conv_id).await {
                    Ok(msgs) => msgs,
                    Err(_) => Vec::new(),
                };

                let rag_query = display_content_clone.as_ref().unwrap_or(&task_clone);
                match perform_rag_enhancement(
                    &app_handle,
                    &conv_id,
                    rag_query,
                    &history_messages,
                    None,
                )
                .await
                {
                    Ok((context, citations)) => {
                        if !context.trim().is_empty() {
                            let base = base_system_prompt.unwrap_or_default();
                            let policy = "you must strictly answer the question based on the evidence. When citing evidence in your response, use the [SOURCE n] format. If the evidence is insufficient, please answer directly and avoid fabricating. ";
                            let augmented = if base.trim().is_empty() {
                                format!(
                                    "[rule of knowledge]\n{}\n\n[Source Evidence Block]\n{}",
                                    policy, context
                                )
                            } else {
                                format!(
                                    "{}\n\n[rule of knowledge]\n{}\n\n[Source Evidence Block]\n{}",
                                    base, policy, context
                                )
                            };
                            base_system_prompt = Some(augmented);

                            let _ = app_handle.emit(
                                "ai_meta_info",
                                &serde_json::json!({
                                    "conversation_id": conv_id,
                                    "generation": cancel_gen,
                                    "message_id": msg_id,
                                    "rag_applied": true,
                                    "rag_sources_used": !citations.is_empty(),
                                    "source_count": citations.len(),
                                    "citations": citations
                                }),
                            );
                        }
                    }
                    Err(e) => tracing::warn!("RAG enhancement failed in agent_execute: {}", e),
                }
            }
        }

        if let Err(e) = app_handle.emit(
            "ai_stream_start",
            &serde_json::json!({
                "conversation_id": conv_id,
                "generation": cancel_gen,
                "message_id": msg_id
            }),
        ) {
            tracing::error!("Failed to emit stream start event: {}", e);
        }

        if let Some(ref tool_cfg) = effective_tool_config {
            if tool_cfg.enabled {
                tracing::info!(
                    "Using tool-enabled agent executor for conversation: {}",
                    conv_id
                );

                let doc_attachments = if let Some(docs) = config.document_attachments.as_ref() {
                    let mut resolved = Vec::with_capacity(docs.len());
                    for d in docs {
                        let runtime_path = match crate::commands::document_commands::resolve_uploaded_file_for_execution_by_id(
                            &app_handle,
                            &d.file_id,
                        )
                        .await {
                            Ok(path) => path,
                            Err(e) => {
                                let error_message = format!(
                                    "Failed to resolve uploaded file {}: {}",
                                    d.file_id, e
                                );
                                finish_agent_harness(
                                    &app_handle,
                                    &harness_run_id,
                                    &conv_id,
                                    cancel_gen,
                                    "failed",
                                    Some(&error_message),
                                )
                                .await;
                                emit_agent_execution_finished_for_generation(
                                    &app_handle,
                                    &conv_id,
                                    cancel_gen,
                                    AgentExecutionOutcome::Failed,
                                    Some(error_message),
                                    None,
                                    None,
                                );
                                return;
                            }
                        };
                        resolved.push(crate::agents::DocumentAttachmentInfo {
                            id: d.file_id.clone(),
                            original_filename: d.original_filename.clone(),
                            file_size: d.file_size,
                            mime_type: d.mime_type.clone(),
                            file_path: Some(runtime_path),
                        });
                    }
                    Some(resolved)
                } else {
                    None
                };

                let executor_params = crate::agents::executor::AgentExecuteParams {
                    execution_id: conv_id.clone(),
                    cancellation_generation: Some(cancel_gen),
                    model: model_name_for_closure.clone(),
                    system_prompt: base_system_prompt.unwrap_or_default(),
                    task: augmented_task.clone(),
                    active_browser_shell_direct_write_enabled: config
                        .current_browser_shell_direct_write_enabled
                        .unwrap_or(false),
                    active_browser_shell_session_id: config
                        .current_browser_shell_session_id
                        .clone(),
                    active_terminal_session_fingerprint: config
                        .current_terminal_session_fingerprint
                        .clone(),
                    active_terminal_session_id: config.current_terminal_session_id.clone(),
                    working_directory: effective_working_directory.clone(),
                    rig_provider: provider_for_closure.clone(),
                    api_key: provider_config_for_closure.api_key.clone(),
                    api_base: provider_config_for_closure.api_base.clone(),
                    max_iterations: config
                        .max_iterations
                        .unwrap_or(provider_config.max_turns.unwrap_or(50))
                        .max(1),
                    timeout_secs: config.timeout_secs.unwrap_or(300),
                    tool_config: effective_tool_config.clone(),
                    enable_tenth_man_rule: config.enable_tenth_man_rule.unwrap_or(false),
                    tenth_man_config: config.tenth_man_config.clone(),
                    document_attachments: doc_attachments,
                    image_attachments: image_attachments_for_execution.clone(),
                    referenced_traffic: config.referenced_traffic.clone(),
                    persist_messages,
                    subagent_run_id: None,
                    context_policy: None,
                    context_engine_mode: config
                        .context_mode
                        .as_deref()
                        .and_then(crate::agents::ContextEngineMode::from_str),
                    recursion_depth: 0,
                };

                match crate::agents::executor::execute_agent(&app_handle, executor_params.clone())
                    .await
                {
                    Ok(response) => {
                        if is_agent_execution_cancelled(&conv_id, Some(cancel_gen)) {
                            tracing::info!(
                                "Agent execution ended after cancellation for conversation: {}",
                                conv_id
                            );
                            finish_agent_harness(
                                &app_handle,
                                &harness_run_id,
                                &conv_id,
                                cancel_gen,
                                "cancelled",
                                None,
                            )
                            .await;
                            emit_agent_execution_finished_for_generation(
                                &app_handle,
                                &conv_id,
                                cancel_gen,
                                AgentExecutionOutcome::Cancelled,
                                None,
                                None,
                                Some("Execution cancelled by user".to_string()),
                            );
                            return;
                        }
                        if attempted_model_vision {
                            if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
                                if let Err(e) = save_cached_model_vision_capability(
                                    db.inner(),
                                    &provider_for_capability_cache,
                                    &model_for_capability_cache,
                                    api_base_for_capability_cache.as_deref(),
                                    rig_provider_for_capability_cache.as_deref(),
                                    ModelVisionCapabilityStatus::Supported,
                                    Some("Vision request completed successfully".to_string()),
                                )
                                .await
                                {
                                    tracing::warn!(
                                        "Failed to persist supported model vision capability: {}",
                                        e
                                    );
                                }
                            }
                        }
                        tracing::info!("Agent with tools completed for conversation: {}", conv_id);
                        update_agent_harness_state(
                            &app_handle,
                            &harness_run_id,
                            "verifying_completion",
                            None,
                            false,
                        )
                        .await;
                        append_agent_harness_event(
                            &app_handle,
                            &harness_run_id,
                            &conv_id,
                            cancel_gen,
                            "model_turn_finished",
                            Some(serde_json::json!({
                                "response_chars": response.chars().count(),
                            })),
                        )
                        .await;
                        let recovery_outcome = recover_unfinished_tasks_after_response(
                            &app_handle,
                            &executor_params,
                            response,
                        )
                        .await;
                        if is_agent_execution_cancelled(&conv_id, Some(cancel_gen)) {
                            tracing::info!(
                                "Agent execution cancelled during unfinished-task recovery for conversation: {}",
                                conv_id
                            );
                            finish_agent_harness(
                                &app_handle,
                                &harness_run_id,
                                &conv_id,
                                cancel_gen,
                                "cancelled",
                                None,
                            )
                            .await;
                            emit_agent_execution_finished_for_generation(
                                &app_handle,
                                &conv_id,
                                cancel_gen,
                                AgentExecutionOutcome::Cancelled,
                                None,
                                None,
                                Some("Execution cancelled by user".to_string()),
                            );
                            return;
                        }
                        if let Some(error_message) = recovery_outcome.unfinished_error {
                            let harness_state = if recovery_outcome.stalled {
                                "stalled"
                            } else {
                                "failed"
                            };
                            let checkpoint_type = if recovery_outcome.stalled {
                                "unfinished_tasks_stalled"
                            } else {
                                "unfinished_tasks_failed"
                            };
                            settle_running_tool_messages(
                                &app_handle,
                                &conv_id,
                                if recovery_outcome.stalled {
                                    "timed_out"
                                } else {
                                    "failed"
                                },
                                if recovery_outcome.stalled {
                                    "Harness stalled while unfinished tasks remained open"
                                } else {
                                    "Execution failed while unfinished tasks remained open"
                                },
                            )
                            .await;
                            checkpoint_agent_harness(
                                &app_handle,
                                &harness_run_id,
                                checkpoint_type,
                                Some(serde_json::json!({
                                    "error": error_message.clone(),
                                    "remaining_tasks": recovery_outcome.remaining_tasks.clone(),
                                    "stalled": recovery_outcome.stalled,
                                })),
                            )
                            .await;
                            finish_agent_harness(
                                &app_handle,
                                &harness_run_id,
                                &conv_id,
                                cancel_gen,
                                harness_state,
                                Some(&error_message),
                            )
                            .await;
                            emit_agent_execution_finished_for_generation(
                                &app_handle,
                                &conv_id,
                                cancel_gen,
                                AgentExecutionOutcome::Failed,
                                Some(error_message),
                                Some(recovery_outcome.response),
                                None,
                            );
                            return;
                        }
                        settle_running_tool_messages(
                            &app_handle,
                            &conv_id,
                            "timed_out",
                            "Execution finished before the tool result was recorded",
                        )
                        .await;
                        checkpoint_agent_harness(
                            &app_handle,
                            &harness_run_id,
                            "generation_succeeded",
                            Some(serde_json::json!({
                                "response_chars": recovery_outcome.response.chars().count(),
                            })),
                        )
                        .await;
                        finish_agent_harness(
                            &app_handle,
                            &harness_run_id,
                            &conv_id,
                            cancel_gen,
                            "succeeded",
                            None,
                        )
                        .await;
                        emit_success_outcome(
                            &app_handle,
                            &conv_id,
                            cancel_gen,
                            recovery_outcome.response,
                        )
                        .await;
                    }
                    Err(e) => {
                        if is_agent_execution_cancelled(&conv_id, Some(cancel_gen)) {
                            tracing::info!(
                                "Agent execution failed after cancellation for conversation: {}",
                                conv_id
                            );
                            finish_agent_harness(
                                &app_handle,
                                &harness_run_id,
                                &conv_id,
                                cancel_gen,
                                "cancelled",
                                None,
                            )
                            .await;
                            emit_agent_execution_finished_for_generation(
                                &app_handle,
                                &conv_id,
                                cancel_gen,
                                AgentExecutionOutcome::Cancelled,
                                None,
                                None,
                                Some("Execution cancelled by user".to_string()),
                            );
                            return;
                        }
                        if attempted_model_vision {
                            let error_text = e.to_string();
                            if matches!(
                                classify_model_vision_capability_error(&error_text),
                                Some(ModelVisionCapabilityStatus::Unsupported)
                            ) {
                                if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
                                    if let Err(save_error) = save_cached_model_vision_capability(
                                        db.inner(),
                                        &provider_for_capability_cache,
                                        &model_for_capability_cache,
                                        api_base_for_capability_cache.as_deref(),
                                        rig_provider_for_capability_cache.as_deref(),
                                        ModelVisionCapabilityStatus::Unsupported,
                                        Some(error_text.clone()),
                                    )
                                    .await
                                    {
                                        tracing::warn!(
                                            "Failed to persist unsupported model vision capability: {}",
                                            save_error
                                        );
                                    }
                                }
                            }
                        }
                        tracing::error!("Agent with tools execution failed: {}", e);
                        settle_running_tool_messages(
                            &app_handle,
                            &conv_id,
                            "failed",
                            "Execution failed before the tool result was recorded",
                        )
                        .await;
                        let error_message = e.to_string();
                        finish_agent_harness(
                            &app_handle,
                            &harness_run_id,
                            &conv_id,
                            cancel_gen,
                            "failed",
                            Some(&error_message),
                        )
                        .await;
                        emit_agent_execution_finished_for_generation(
                            &app_handle,
                            &conv_id,
                            cancel_gen,
                            AgentExecutionOutcome::Failed,
                            Some(error_message),
                            None,
                            None,
                        );
                    }
                }

                return;
            }
        }

        match stream_chat_with_llm(
            &service_clone,
            &app_handle,
            &conv_id,
            cancel_gen,
            &msg_id,
            &augmented_task,
            base_system_prompt.as_deref(),
            image_attachments_for_execution,
            persist_messages,
        )
        .await
        {
            Ok(response) => {
                if is_agent_execution_cancelled(&conv_id, Some(cancel_gen)) {
                    tracing::info!(
                        "Stream chat ended after cancellation for conversation: {}",
                        conv_id
                    );
                    finish_agent_harness(
                        &app_handle,
                        &harness_run_id,
                        &conv_id,
                        cancel_gen,
                        "cancelled",
                        None,
                    )
                    .await;
                    emit_agent_execution_finished_for_generation(
                        &app_handle,
                        &conv_id,
                        cancel_gen,
                        AgentExecutionOutcome::Cancelled,
                        None,
                        None,
                        Some("Execution cancelled by user".to_string()),
                    );
                    return;
                }
                if attempted_model_vision {
                    if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
                        if let Err(e) = save_cached_model_vision_capability(
                            db.inner(),
                            &provider_for_capability_cache,
                            &model_for_capability_cache,
                            api_base_for_capability_cache.as_deref(),
                            rig_provider_for_capability_cache.as_deref(),
                            ModelVisionCapabilityStatus::Supported,
                            Some("Vision request completed successfully".to_string()),
                        )
                        .await
                        {
                            tracing::warn!(
                                "Failed to persist supported model vision capability: {}",
                                e
                            );
                        }
                    }
                }
                tracing::info!("Stream chat completed for conversation: {}", conv_id);
                checkpoint_agent_harness(
                    &app_handle,
                    &harness_run_id,
                    "generation_succeeded",
                    Some(serde_json::json!({
                        "response_chars": response.chars().count(),
                    })),
                )
                .await;
                finish_agent_harness(
                    &app_handle,
                    &harness_run_id,
                    &conv_id,
                    cancel_gen,
                    "succeeded",
                    None,
                )
                .await;
                emit_success_outcome(&app_handle, &conv_id, cancel_gen, response).await;
            }
            Err(e) => {
                if is_agent_execution_cancelled(&conv_id, Some(cancel_gen)) {
                    tracing::info!(
                        "Stream chat failed after cancellation for conversation: {}",
                        conv_id
                    );
                    finish_agent_harness(
                        &app_handle,
                        &harness_run_id,
                        &conv_id,
                        cancel_gen,
                        "cancelled",
                        None,
                    )
                    .await;
                    emit_agent_execution_finished_for_generation(
                        &app_handle,
                        &conv_id,
                        cancel_gen,
                        AgentExecutionOutcome::Cancelled,
                        None,
                        None,
                        Some("Execution cancelled by user".to_string()),
                    );
                    return;
                }
                if attempted_model_vision {
                    if matches!(
                        classify_model_vision_capability_error(&e),
                        Some(ModelVisionCapabilityStatus::Unsupported)
                    ) {
                        if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
                            if let Err(save_error) = save_cached_model_vision_capability(
                                db.inner(),
                                &provider_for_capability_cache,
                                &model_for_capability_cache,
                                api_base_for_capability_cache.as_deref(),
                                rig_provider_for_capability_cache.as_deref(),
                                ModelVisionCapabilityStatus::Unsupported,
                                Some(e.clone()),
                            )
                            .await
                            {
                                tracing::warn!(
                                    "Failed to persist unsupported model vision capability: {}",
                                    save_error
                                );
                            }
                        }
                    }
                }
                tracing::error!("Stream chat failed: {}", e);
                finish_agent_harness(
                    &app_handle,
                    &harness_run_id,
                    &conv_id,
                    cancel_gen,
                    "failed",
                    Some(&e),
                )
                .await;
                emit_agent_execution_finished_for_generation(
                    &app_handle,
                    &conv_id,
                    cancel_gen,
                    AgentExecutionOutcome::Failed,
                    Some(e),
                    None,
                    None,
                );
            }
        }
    });

    Ok(message_id)
}

fn sanitize_image_attachments(attachments: &serde_json::Value) -> serde_json::Value {
    fn sanitize_one(v: &mut serde_json::Value) {
        let img = if v.get("type").and_then(|t| t.as_str()) == Some("image") {
            Some(v)
        } else {
            v.get_mut("image")
        };
        let Some(img) = img else { return };
        if let Some(obj) = img.as_object_mut() {
            obj.remove("source_path");
        }
    }

    let mut cloned = attachments.clone();
    if let Some(arr) = cloned.as_array_mut() {
        for item in arr.iter_mut() {
            sanitize_one(item);
        }
    } else if cloned.is_object() {
        sanitize_one(&mut cloned);
    }
    cloned
}

async fn load_image_attachment_settings(
    db: &DatabaseService,
) -> (EffectiveImageAttachmentMode, bool) {
    let mode_str = db
        .get_config("agent", "image_attachment_mode")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "auto".to_string());
    let mode = match mode_str.as_str() {
        "auto" => EffectiveImageAttachmentMode::Auto,
        "model_vision" => EffectiveImageAttachmentMode::ModelVision,
        _ => EffectiveImageAttachmentMode::LocalOcr,
    };

    let allow_upload = db
        .get_config("agent", "allow_image_upload_to_model")
        .await
        .ok()
        .flatten()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    (mode, allow_upload)
}

async fn load_tool_config_from_db(app_handle: &AppHandle) -> Option<crate::agents::ToolConfig> {
    if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
        if let Ok(Some(config_str)) = db.get_config("agent", "tool_config").await {
            if let Ok(config) = crate::agents::ToolConfig::from_json_str(&config_str) {
                tracing::info!("Loaded global tool config from database");
                return Some(config);
            }
        }
    }

    tracing::info!("No global tool config found, using default");
    None
}

async fn save_tool_config_to_db(
    app_handle: &AppHandle,
    config: &crate::agents::ToolConfig,
) -> Result<(), String> {
    if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
        let config_str = serde_json::to_string(config)
            .map_err(|e| format!("Failed to serialize tool config: {}", e))?;

        db.set_config(
            "agent",
            "tool_config",
            &config_str,
            Some("Global tool configuration"),
        )
        .await
        .map_err(|e| format!("Failed to save tool config: {}", e))?;

        tracing::info!("Saved global tool config to database");
        Ok(())
    } else {
        Err("Database service not available".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_unfinished_execution_tasks_error, build_unfinished_tasks_continuation_prompt,
    };

    #[test]
    fn unfinished_execution_tasks_error_lists_only_non_terminal_tasks() {
        let tasks = vec![
            sentinel_db::ExecutionTaskItem {
                id: "task-1".to_string(),
                execution_id: "exec-1".to_string(),
                item_index: 0,
                description: "done".to_string(),
                status: "completed".to_string(),
                result: None,
                created_at: "2026-05-05T00:00:00Z".to_string(),
                updated_at: "2026-05-05T00:00:00Z".to_string(),
            },
            sentinel_db::ExecutionTaskItem {
                id: "task-2".to_string(),
                execution_id: "exec-1".to_string(),
                item_index: 1,
                description: "building page".to_string(),
                status: "in_progress".to_string(),
                result: None,
                created_at: "2026-05-05T00:00:00Z".to_string(),
                updated_at: "2026-05-05T00:00:00Z".to_string(),
            },
            sentinel_db::ExecutionTaskItem {
                id: "task-3".to_string(),
                execution_id: "exec-1".to_string(),
                item_index: 2,
                description: "write tests".to_string(),
                status: "pending".to_string(),
                result: None,
                created_at: "2026-05-05T00:00:00Z".to_string(),
                updated_at: "2026-05-05T00:00:00Z".to_string(),
            },
            sentinel_db::ExecutionTaskItem {
                id: "task-4".to_string(),
                execution_id: "exec-1".to_string(),
                item_index: 3,
                description: "known issue".to_string(),
                status: "failed".to_string(),
                result: None,
                created_at: "2026-05-05T00:00:00Z".to_string(),
                updated_at: "2026-05-05T00:00:00Z".to_string(),
            },
        ];

        let message = build_unfinished_execution_tasks_error("exec-1", &tasks);

        assert!(message.contains("exec-1"));
        assert!(message.contains("#2 building page (in_progress)"));
        assert!(message.contains("#3 write tests (pending)"));
        assert!(!message.contains("done"));
        assert!(!message.contains("known issue"));
    }

    #[test]
    fn unfinished_task_recovery_prompt_focuses_on_remaining_tasks() {
        let tasks = vec![
            sentinel_db::ExecutionTaskItem {
                id: "task-1".to_string(),
                execution_id: "exec-1".to_string(),
                item_index: 0,
                description: "done".to_string(),
                status: "completed".to_string(),
                result: Some("already handled".to_string()),
                created_at: "2026-05-05T00:00:00Z".to_string(),
                updated_at: "2026-05-05T00:00:00Z".to_string(),
            },
            sentinel_db::ExecutionTaskItem {
                id: "task-2".to_string(),
                execution_id: "exec-1".to_string(),
                item_index: 1,
                description: "building page".to_string(),
                status: "in_progress".to_string(),
                result: Some("editing current page".to_string()),
                created_at: "2026-05-05T00:00:00Z".to_string(),
                updated_at: "2026-05-05T00:00:00Z".to_string(),
            },
            sentinel_db::ExecutionTaskItem {
                id: "task-3".to_string(),
                execution_id: "exec-1".to_string(),
                item_index: 2,
                description: "write tests".to_string(),
                status: "pending".to_string(),
                result: None,
                created_at: "2026-05-05T00:00:00Z".to_string(),
                updated_at: "2026-05-05T00:00:00Z".to_string(),
            },
        ];

        let prompt = build_unfinished_tasks_continuation_prompt(
            "exec-1",
            "Previous answer ended too early.",
            &tasks,
            1,
            false,
        );

        assert!(prompt.contains("harness continuation attempt 1"));
        assert!(prompt.contains("#2 building page (in_progress)"));
        assert!(prompt.contains("#3 write tests (pending)"));
        assert!(!prompt.contains("#1 done (completed)"));
        assert!(prompt.contains("Previous answer ended too early."));
    }

    #[test]
    fn unfinished_task_recovery_prompt_escalates_after_stalled_attempt() {
        let tasks = vec![sentinel_db::ExecutionTaskItem {
            id: "task-2".to_string(),
            execution_id: "exec-1".to_string(),
            item_index: 1,
            description: "building page".to_string(),
            status: "in_progress".to_string(),
            result: Some("still analyzing".to_string()),
            created_at: "2026-05-05T00:00:00Z".to_string(),
            updated_at: "2026-05-05T00:00:00Z".to_string(),
        }];

        let prompt = build_unfinished_tasks_continuation_prompt(
            "exec-1",
            "No concrete progress yet.",
            &tasks,
            2,
            true,
        );

        assert!(prompt.contains("harness continuation attempt 2"));
        assert!(prompt.contains("[Recovery Escalation]"));
        assert!(prompt.contains("plain analysis-only reply is invalid"));
    }
}

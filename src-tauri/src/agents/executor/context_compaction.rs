use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use serde_json::json;
use tauri::{AppHandle, Emitter};

use sentinel_llm::{ChatMessage, LlmConfig};

use super::context_pressure::{
    append_request_context_pressure_harness_event, emit_request_context_pressure,
    RequestContextPressure,
};
use super::run_with_tools_support::build_retry_history;
use super::types::ToolCallRecord;
use super::AgentExecuteParams;
use crate::agents::context_engineering::{
    ContextBudgetAnalyzer, ContextPressure, ContextPressure::Blocking,
};
use crate::agents::{build_context, ContextBuildInput, ContextPolicy};
use crate::commands::ai_runtime_harness::{append_agent_harness_event, checkpoint_agent_harness};

pub(super) struct ContextCompactionOrchestrator;

pub(super) struct ContextCompactionOutcome {
    pub pressure: RequestContextPressure,
    pub compacted: bool,
}

pub(super) struct ContextCompactionRequest<'a> {
    pub app_handle: &'a AppHandle,
    pub params: &'a AgentExecuteParams,
    pub phase: &'a str,
    pub analyzer: &'a mut ContextBudgetAnalyzer,
    pub system_prompt: &'a mut Option<String>,
    pub base_history_messages: &'a mut Vec<ChatMessage>,
    pub history_for_retry: &'a mut Vec<ChatMessage>,
    pub compaction_attempted: &'a mut bool,
    pub force_compaction: bool,
    pub injected_skill_prompt: &'a Option<String>,
    pub injected_runtime_context: &'a Option<String>,
    pub rig_provider: &'a str,
    pub llm_config: &'a LlmConfig,
    pub current_tool_ids: &'a [String],
    pub context_policy: &'a ContextPolicy,
    pub accumulated_tool_calls: &'a Arc<Mutex<Vec<ToolCallRecord>>>,
    pub accumulated_assistant_output: &'a Arc<Mutex<String>>,
    pub retries: u32,
    pub include_accumulated: bool,
}

impl ContextCompactionOrchestrator {
    pub(super) async fn assess_and_rebuild(
        request: ContextCompactionRequest<'_>,
    ) -> Result<ContextCompactionOutcome> {
        let pressure = emit_and_persist_pressure(&request).await;
        log_pressure(&request.params.execution_id, &pressure);

        if !pressure.should_compact && !request.force_compaction {
            return Ok(ContextCompactionOutcome {
                pressure,
                compacted: false,
            });
        }

        if *request.compaction_attempted {
            if pressure.should_block || request.force_compaction {
                append_unresolved_blocking_event(&request, &pressure).await;
                return Err(anyhow!(
                    "Context remains over the blocking threshold after compaction attempt: used_tokens={}, remaining_tokens={}, pressure={:?}",
                    pressure.analysis.used_tokens,
                    pressure.analysis.remaining_tokens,
                    pressure.analysis.pressure
                ));
            }
            return Ok(ContextCompactionOutcome {
                pressure,
                compacted: false,
            });
        }

        *request.compaction_attempted = true;
        emit_compaction_requested(&request, &pressure).await;

        let before = pressure;
        let rebuilt_context = build_context(ContextBuildInput {
            app_handle: request.app_handle.clone(),
            execution_id: request.params.execution_id.clone(),
            conversation_id: request.params.storage_conversation_id().to_string(),
            generation: request.params.cancellation_generation,
            active_browser_shell_direct_write_enabled: request
                .params
                .active_browser_shell_direct_write_enabled,
            active_browser_shell_session_id: request.params.active_browser_shell_session_id.clone(),
            active_terminal_session_fingerprint: request
                .params
                .active_terminal_session_fingerprint
                .clone(),
            active_terminal_session_id: request.params.active_terminal_session_id.clone(),
            working_directory: request.params.working_directory.clone(),
            base_system_prompt: request.params.system_prompt.clone(),
            injected_skill_prompt: request.injected_skill_prompt.clone(),
            injected_runtime_context: request.injected_runtime_context.clone(),
            task: request.params.task.clone(),
            provider_config_key: request.params.provider_config_key.clone(),
            rig_provider: request.rig_provider.to_string(),
            llm_config: request.llm_config.clone(),
            selected_tool_ids: request.current_tool_ids.to_vec(),
            document_attachments: request.params.document_attachments.clone(),
            engine_mode: request.params.context_engine_mode.unwrap_or_default(),
            policy: request.context_policy.clone(),
        })
        .await?;

        *request.analyzer = rebuilt_context.budget_analyzer;
        *request.system_prompt = Some(rebuilt_context.system_prompt);
        let mut rebuilt_history_messages = rebuilt_context.history_messages;
        if let Some(last) = rebuilt_history_messages.last() {
            if last.role == "user" {
                rebuilt_history_messages.pop();
            }
        }
        *request.base_history_messages = rebuilt_history_messages;
        *request.history_for_retry = build_retry_history(
            request.base_history_messages,
            request
                .accumulated_tool_calls
                .lock()
                .map(|calls| calls.clone())
                .unwrap_or_default(),
            request
                .accumulated_assistant_output
                .lock()
                .map(|s| s.clone())
                .unwrap_or_default(),
            request.retries,
            request.include_accumulated,
        );
        let state_digest = build_post_compact_state_digest_from_request(&request);
        request
            .history_for_retry
            .push(ChatMessage::user(state_digest));

        let after = emit_request_context_pressure(
            request.app_handle,
            &request.params.execution_id,
            request.params.cancellation_generation,
            rebuilt_phase(request.phase),
            request.analyzer,
            request.system_prompt.as_deref(),
            &request.params.task,
            request.history_for_retry,
        );
        append_request_context_pressure_harness_event(
            request.app_handle,
            request.params.harness_run_id.as_deref(),
            &request.params.execution_id,
            request.params.cancellation_generation,
            "context_pressure",
            rebuilt_phase(request.phase),
            request.analyzer,
            &after,
            request.history_for_retry.len(),
        )
        .await;
        checkpoint_compaction_rebuilt(&request, &before, &after).await;

        tracing::info!(
            "Context compaction rebuild completed - execution_id: {}, phase: {}, before_used_tokens: {}, after_used_tokens: {}, after_pressure: {:?}",
            request.params.execution_id,
            request.phase,
            before.analysis.used_tokens,
            after.analysis.used_tokens,
            after.analysis.pressure
        );

        if matches!(after.analysis.pressure, Blocking) {
            append_unresolved_blocking_event(&request, &after).await;
            return Err(anyhow!(
                "Context remains over the blocking threshold after rebuild: used_tokens={}, remaining_tokens={}, pressure={:?}",
                after.analysis.used_tokens,
                after.analysis.remaining_tokens,
                after.analysis.pressure
            ));
        }

        Ok(ContextCompactionOutcome {
            pressure: after,
            compacted: true,
        })
    }
}

fn rebuilt_phase(phase: &str) -> &'static str {
    if phase == "reactive_context_length_error" {
        "post_reactive_compaction_rebuild_pre_stream_request"
    } else {
        "post_compaction_rebuild_pre_stream_request"
    }
}

async fn emit_and_persist_pressure(
    request: &ContextCompactionRequest<'_>,
) -> RequestContextPressure {
    let pressure = emit_request_context_pressure(
        request.app_handle,
        &request.params.execution_id,
        request.params.cancellation_generation,
        request.phase,
        request.analyzer,
        request.system_prompt.as_deref(),
        &request.params.task,
        request.history_for_retry,
    );
    append_request_context_pressure_harness_event(
        request.app_handle,
        request.params.harness_run_id.as_deref(),
        &request.params.execution_id,
        request.params.cancellation_generation,
        "context_pressure",
        request.phase,
        request.analyzer,
        &pressure,
        request.history_for_retry.len(),
    )
    .await;
    pressure
}

async fn emit_compaction_requested(
    request: &ContextCompactionRequest<'_>,
    pressure: &RequestContextPressure,
) {
    let _ = request.app_handle.emit(
        "agent:context_compaction_requested",
        &json!({
            "execution_id": request.params.execution_id,
            "generation": request.params.cancellation_generation,
            "phase": request.phase,
            "used_tokens": pressure.analysis.used_tokens,
            "remaining_tokens": pressure.analysis.remaining_tokens,
            "context_pressure": format!("{:?}", pressure.analysis.pressure),
        }),
    );
    append_request_context_pressure_harness_event(
        request.app_handle,
        request.params.harness_run_id.as_deref(),
        &request.params.execution_id,
        request.params.cancellation_generation,
        "context_compaction_requested",
        request.phase,
        request.analyzer,
        pressure,
        request.history_for_retry.len(),
    )
    .await;
}

async fn checkpoint_compaction_rebuilt(
    request: &ContextCompactionRequest<'_>,
    before: &RequestContextPressure,
    after: &RequestContextPressure,
) {
    let Some(run_id) = request.params.harness_run_id.as_deref() else {
        return;
    };
    let accumulated_tool_call_count = request
        .accumulated_tool_calls
        .lock()
        .map(|calls| calls.len())
        .unwrap_or(0);
    let accumulated_output_chars = request
        .accumulated_assistant_output
        .lock()
        .map(|value| value.chars().count())
        .unwrap_or(0);
    checkpoint_agent_harness(
        request.app_handle,
        run_id,
        "context_compaction_rebuilt",
        Some(json!({
            "phase": request.phase,
            "rebuilt_phase": rebuilt_phase(request.phase),
            "before_used_tokens": before.analysis.used_tokens,
            "after_used_tokens": after.analysis.used_tokens,
            "token_delta": before.analysis.used_tokens as i64 - after.analysis.used_tokens as i64,
            "before_pressure": format!("{:?}", before.analysis.pressure),
            "after_pressure": format!("{:?}", after.analysis.pressure),
            "before_remaining_tokens": before.analysis.remaining_tokens,
            "after_remaining_tokens": after.analysis.remaining_tokens,
            "still_blocking": matches!(after.analysis.pressure, ContextPressure::Blocking),
            "base_history_count": request.base_history_messages.len(),
            "retry_history_count": request.history_for_retry.len(),
            "accumulated_tool_call_count": accumulated_tool_call_count,
            "accumulated_output_chars": accumulated_output_chars,
            "document_attachment_count": request.params.document_attachments.as_ref().map(|items| items.len()).unwrap_or(0),
            "active_tool_ids": request.current_tool_ids,
        })),
    )
    .await;
}

async fn append_unresolved_blocking_event(
    request: &ContextCompactionRequest<'_>,
    pressure: &RequestContextPressure,
) {
    let (Some(run_id), Some(generation)) = (
        request.params.harness_run_id.as_deref(),
        request.params.cancellation_generation,
    ) else {
        return;
    };
    append_agent_harness_event(
        request.app_handle,
        run_id,
        &request.params.execution_id,
        generation,
        "context_blocking_unresolved",
        Some(json!({
            "phase": request.phase,
            "used_tokens": pressure.analysis.used_tokens,
            "remaining_tokens": pressure.analysis.remaining_tokens,
            "context_pressure": format!("{:?}", pressure.analysis.pressure),
            "blocking_threshold_tokens": request.analyzer.blocking_threshold_tokens,
        })),
    )
    .await;
}

fn build_post_compact_state_digest_from_request(request: &ContextCompactionRequest<'_>) -> String {
    let tool_call_count = request
        .accumulated_tool_calls
        .lock()
        .map(|calls| calls.len())
        .unwrap_or(0);
    let output_chars = request
        .accumulated_assistant_output
        .lock()
        .map(|value| value.chars().count())
        .unwrap_or(0);
    let attachment_count = request
        .params
        .document_attachments
        .as_ref()
        .map(|items| items.len())
        .unwrap_or(0);
    build_post_compact_state_digest(
        request.phase,
        request.current_tool_ids,
        attachment_count,
        tool_call_count,
        output_chars,
    )
}

fn build_post_compact_state_digest(
    phase: &str,
    current_tool_ids: &[String],
    attachment_count: usize,
    tool_call_count: usize,
    output_chars: usize,
) -> String {
    format!(
        "[ContextCompaction]\nphase: {}\nactive_tools: {}\ndocument_attachments: {}\nrecovered_tool_calls: {}\nrecovered_assistant_output_chars: {}\nContinue the current task from this recovered state. Do not restart completed work.",
        phase,
        current_tool_ids.join(", "),
        attachment_count,
        tool_call_count,
        output_chars
    )
}

fn log_pressure(execution_id: &str, pressure: &RequestContextPressure) {
    if pressure.should_block {
        tracing::warn!(
            "Context pressure blocking threshold reached before stream request - execution_id: {}, used_tokens: {}, remaining_tokens: {}",
            execution_id,
            pressure.analysis.used_tokens,
            pressure.analysis.remaining_tokens
        );
    } else if pressure.should_compact {
        tracing::info!(
            "Context pressure auto-compact threshold reached before stream request - execution_id: {}, used_tokens: {}, remaining_tokens: {}",
            execution_id,
            pressure.analysis.used_tokens,
            pressure.analysis.remaining_tokens
        );
    }
}

#[cfg(test)]
mod tests {
    use super::build_post_compact_state_digest;

    #[test]
    fn post_compact_state_digest_restores_current_execution_state() {
        let tools = vec!["shell".to_string()];
        let digest = build_post_compact_state_digest("test", &tools, 2, 3, 128);

        assert!(digest.contains("recovered_assistant_output_chars: 128"));
        assert!(digest.contains("document_attachments: 2"));
        assert!(digest.contains("recovered_tool_calls: 3"));
        assert!(digest.contains("shell"));
    }
}

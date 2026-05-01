//! Tool-enabled execution path.

use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use sentinel_db::Database;
use sentinel_llm::{
    normalize_tool_call_arguments_str, parse_images_from_json, ChatMessage, StreamContent,
    StreamingLlmClient,
};
use sentinel_memory::{get_global_memory, ExecutionRecord, ToolCallSummary};
use sentinel_tools::buildin_tools::{ShellTool, ToolSearchTool};
use sentinel_tools::ToolServer;

use super::run_with_tools_support::{
    accumulate_retry_progress, apply_allowed_tools_policy, build_retry_history,
    clear_retry_turn_state, collect_all_tool_calls, ensure_ai_conversation_exists_for_persistence,
    final_response_needs_evidence_review, final_response_needs_verification_review,
    finalize_response_state, infer_tool_result_success, is_empty_response_error,
    is_high_risk_tool_call, is_retryable_error, is_side_effectful_tool_call,
    looks_like_verification_tool_call, parse_team_stream_context, patch_builtin_dynamic_tools,
    persist_ai_message_with_retry, register_skills_tool_guard,
    streaming_content_needs_evidence_review, tool_loop_fingerprint, trailing_failed_tool_calls,
};
use super::AgentExecuteParams;
use crate::agents::context_engineering::reflection::{
    record_execution_reflection, ExecutionOutcome,
};
use crate::agents::executor::message_store::{
    build_assistant_session_stats_metadata, mark_first_response_ms, save_assistant_message,
};
use crate::agents::executor::skill_loaded_events::emit_and_persist_skill_loaded;
use crate::agents::executor::team_runtime_log_context::{
    is_toolset_error_result, log_toolset_error_with_context, resolve_team_runtime_log_context,
};
use crate::agents::executor::tenth_man_hypothesis::HypothesisTracker;
use crate::agents::executor::terminal_session_store::scope_active_terminal_session;
use crate::agents::executor::tool_activation_events::{
    emit_and_persist_tool_activation, emit_initial_tool_selection,
};
use crate::agents::executor::tool_bias::bias_tool_ids_for_recent_file_changes;
use crate::agents::executor::tool_feedback::{emit_retry_event, spawn_tenth_man_warning};
use crate::agents::executor::tool_progress::accumulate_progress;
use crate::agents::executor::tool_trace_store::{
    append_execution_tool_trace, clear_execution_tool_trace,
};
use crate::agents::executor::types::ToolCallRecord;
use crate::agents::executor::utils::{cleanup_container_context_async, truncate_for_memory};
use crate::agents::tenth_man::{
    InterventionContext, InterventionMode, TenthMan, TenthManTriggerPolicy, TriggerReason,
};
use crate::agents::tool_router::ToolRouter;
use crate::agents::{
    append_tool_digests, apply_sentinel_execution_outcome, apply_tool_digest_to_tracked_artifacts,
    build_context, build_tool_digest, load_run_state, resolve_context_policy, ContextBuildInput,
    TrackedArtifact,
};
use crate::utils::ai_generation_settings::apply_generation_settings_from_db;

type PendingToolCalls = std::collections::HashMap<String, (String, String, i64, u32)>;

pub async fn execute_agent_with_tools(
    app_handle: &AppHandle,
    params: AgentExecuteParams,
    tool_server: &ToolServer,
) -> Result<String> {
    let execution_started_at_ms = chrono::Utc::now().timestamp_millis();
    clear_execution_tool_trace(&params.execution_id);
    let _active_terminal_session_guard = scope_active_terminal_session(
        &params.execution_id,
        params.active_terminal_session_id.as_deref(),
    );
    let tool_config = params.tool_config.clone().unwrap_or_default();
    let tenth_man_trigger_policy: TenthManTriggerPolicy = params
        .tenth_man_config
        .as_ref()
        .map(|config| config.trigger_policy.clone())
        .unwrap_or_default();

    use tauri::Manager;
    let db_service = app_handle.state::<std::sync::Arc<sentinel_db::DatabaseService>>();

    let tool_router = ToolRouter::new_with_all_tools(Some(db_service.inner())).await;

    let rig_provider = params.rig_provider.to_lowercase();
    let mut llm_config = sentinel_llm::LlmConfig::new(&rig_provider, &params.model)
        .with_timeout(params.timeout_secs)
        .with_max_turns(params.max_iterations)
        .with_rig_provider(&rig_provider)
        .with_conversation_id(&params.execution_id);

    if let Some(ref api_key) = params.api_key {
        llm_config = llm_config.with_api_key(api_key);
    }

    if let Some(ref api_base) = params.api_base {
        llm_config = llm_config.with_base_url(api_base);
    }

    if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        llm_config = apply_generation_settings_from_db(db.as_ref(), llm_config).await;
    }

    let selection_plan = tool_router
        .plan_tools(&params.task, &tool_config, Some(&llm_config))
        .await?;

    let selected_tool_ids = bias_tool_ids_for_recent_file_changes(
        app_handle,
        &params.execution_id,
        &params.task,
        apply_allowed_tools_policy(selection_plan.tool_ids.clone(), &tool_config.allowed_tools),
        &tool_config,
        params.active_browser_shell_session_id.as_deref(),
    )
    .await;
    let usage_data = Arc::new(std::sync::Mutex::new(None::<(u32, u32)>));
    let usage_data_for_stream = usage_data.clone();
    let first_response_ms = Arc::new(std::sync::Mutex::new(None::<i64>));

    tracing::info!(
        "Selected {} tools for execution_id {}: {:?} (strategy={:?})",
        selected_tool_ids.len(),
        params.execution_id,
        selected_tool_ids,
        tool_config.selection_strategy
    );

    emit_initial_tool_selection(
        app_handle,
        &params.execution_id,
        selection_plan
            .selected_skill
            .as_ref()
            .map(|skill| (skill.id.as_str(), skill.name.as_str())),
        &selected_tool_ids,
    );

    let mut current_tool_ids = selected_tool_ids.clone();

    let context_policy = resolve_context_policy(
        params.context_policy.clone(),
        params.context_engine_mode.unwrap_or_default(),
    );
    let context_result = build_context(ContextBuildInput {
        app_handle: app_handle.clone(),
        execution_id: params.execution_id.clone(),
        active_browser_shell_direct_write_enabled: params.active_browser_shell_direct_write_enabled,
        active_browser_shell_session_id: params.active_browser_shell_session_id.clone(),
        active_terminal_session_fingerprint: params.active_terminal_session_fingerprint.clone(),
        active_terminal_session_id: params.active_terminal_session_id.clone(),
        base_system_prompt: params.system_prompt.clone(),
        injected_skill_prompt: selection_plan.injected_system_prompt.clone(),
        task: params.task.clone(),
        rig_provider: rig_provider.clone(),
        llm_config: llm_config.clone(),
        selected_tool_ids: selected_tool_ids.clone(),
        document_attachments: params.document_attachments.clone(),
        engine_mode: params.context_engine_mode.unwrap_or_default(),
        policy: context_policy.clone(),
    })
    .await?;

    let final_system_prompt_content = Some(context_result.system_prompt);
    let mut history_chat_messages = context_result.history_messages;

    if let Some(last) = history_chat_messages.last() {
        if last.role == "user" {
            history_chat_messages.pop();
        }
    }

    let image_attachments = parse_images_from_json(params.image_attachments.as_ref());

    let client = StreamingLlmClient::new(llm_config);
    let execution_id = params.execution_id.clone();
    let team_stream_context = parse_team_stream_context(&execution_id);
    let team_log_context =
        resolve_team_runtime_log_context(&execution_id, Some(db_service.inner())).await;
    let app = app_handle.clone();
    let db_for_stream: Option<std::sync::Arc<sentinel_db::DatabaseService>> =
        if params.persist_messages {
            app_handle
                .try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
                .map(|s| s.inner().clone())
        } else {
            None
        };
    if let Some(db) = db_for_stream.as_ref() {
        ensure_ai_conversation_exists_for_persistence(
            db.as_ref(),
            &execution_id,
            &params.model,
            &params.rig_provider,
        )
        .await;
    }

    // 用于收集工具调用信息
    use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
    use std::sync::Mutex;
    let tool_calls_collector: Arc<Mutex<Vec<ToolCallRecord>>> = Arc::new(Mutex::new(Vec::new()));
    let pending_calls: Arc<Mutex<PendingToolCalls>> = Arc::new(Mutex::new(PendingToolCalls::new()));
    let tool_seq: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));
    let tool_call_counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let loop_break_requested: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let loop_guard_prompt_needed: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let last_tool_fingerprint: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));
    let repeated_tool_fingerprint_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let low_evidence_warning_issued: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let hypothesis_tracker: Arc<Mutex<HypothesisTracker>> =
        Arc::new(Mutex::new(HypothesisTracker::default()));
    let assistant_segment_buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    // Track how many assistant text segments have been flushed (persisted) to the database
    // at tool-call boundaries. When > 0, the final save_assistant_message should only save
    // the last turn's response to avoid duplicating earlier segments.
    let persisted_segment_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let reasoning_content_buf: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let pending_tool_digests: Arc<Mutex<Vec<crate::agents::ToolDigest>>> =
        Arc::new(Mutex::new(Vec::new()));
    let tracked_artifacts_state: Arc<Mutex<Vec<TrackedArtifact>>> = Arc::new(Mutex::new(
        load_run_state(app_handle, &params.execution_id)
            .await
            .ok()
            .flatten()
            .map(|state| state.tracked_artifacts)
            .unwrap_or_default(),
    ));
    let context_policy_for_stream = context_policy.clone();

    let collector = tool_calls_collector.clone();
    let pending = pending_calls.clone();
    let seq_counter = tool_seq.clone();
    let tool_counter = tool_call_counter.clone();
    let loop_break_flag = loop_break_requested.clone();
    let loop_prompt_flag = loop_guard_prompt_needed.clone();
    let last_tool_fp = last_tool_fingerprint.clone();
    let repeated_tool_fp_count = repeated_tool_fingerprint_count.clone();
    let low_evidence_warning_flag = low_evidence_warning_issued.clone();
    let hypothesis_tracker_for_stream = hypothesis_tracker.clone();
    let segment_buf = assistant_segment_buf.clone();
    let reasoning_buf = reasoning_content_buf.clone();
    let first_response_ms_for_stream = first_response_ms.clone();
    let pending_digests = pending_tool_digests.clone();
    let tracked_artifacts_for_stream = tracked_artifacts_state.clone();
    let persisted_seg_count = persisted_segment_count.clone();
    let team_log_context_for_stream = team_log_context.clone();

    // Ensure skills tool enforces per-skill enable flags at execution time.
    if let Some(db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        register_skills_tool_guard(tool_server, db.inner().clone()).await?;
    }

    // 7. 调用带动态工具的流式方法，增加重试机制以应对模型抖动或解析错误
    let mut retries = 0;
    let max_retries = 2; // 最多重试 2 次
    let mut empty_response_retries = 0;
    let max_empty_response_retries = 2;
    let mut silent_retry_pending = false;
    let mut last_error: Option<anyhow::Error> = None;
    let mut skill_reload_count = 0;
    let max_skill_reload = 3;
    let mut tool_activation_reload_count = 0;
    let max_tool_activation_reload = 6;

    // 累积的工具调用记录（跨重试保留）
    let accumulated_tool_calls: Arc<Mutex<Vec<ToolCallRecord>>> = Arc::new(Mutex::new(Vec::new()));
    // 累积的助手输出（跨重试保留）
    let accumulated_assistant_output: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let base_history_messages = history_chat_messages.clone();

    let skill_reload_requested = Arc::new(AtomicBool::new(false));
    let loaded_skill_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let tool_activation_reload_requested = Arc::new(AtomicBool::new(false));
    let activated_tool_ids: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let activated_tool_query: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let activated_tool_runtime_hint: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let team_stream_started = Arc::new(AtomicBool::new(false));
    let team_stream_had_delta = Arc::new(AtomicBool::new(false));

    let mut force_history_with_tools = false;
    while retries <= max_retries {
        // Early exit if cancelled before starting a new stream turn
        if crate::commands::ai::is_conversation_cancelled(&params.execution_id) {
            tracing::info!(
                "Execution cancelled before new stream turn: {}",
                params.execution_id
            );
            return Ok(String::new());
        }

        let mut dynamic_tools = tool_server.get_dynamic_tools(&current_tool_ids).await;
        let referenced_traffic = params.referenced_traffic.as_deref().unwrap_or(&[]);
        dynamic_tools = patch_builtin_dynamic_tools(
            dynamic_tools,
            &current_tool_ids,
            app_handle,
            tool_server,
            &params.execution_id,
            params.active_terminal_session_id.as_deref(),
            referenced_traffic,
        )
        .await;

        tracing::info!(
            "Got {} dynamic tool instances for rig-core native tool calling",
            dynamic_tools.len()
        );

        if retries > 0 {
            // 保存当前已完成的工具调用到累积记录
            accumulate_progress(
                &tool_calls_collector,
                &accumulated_tool_calls,
                &assistant_segment_buf,
                &accumulated_assistant_output,
                None,
            );

            tracing::warn!(
                "Retrying agent execution (attempt {}/{}) due to error: {}. Accumulated {} tool calls and {} chars output.",
                retries,
                max_retries,
                last_error.as_ref().map(|e| e.to_string()).unwrap_or_default(),
                accumulated_tool_calls.lock().map(|c| c.len()).unwrap_or(0),
                accumulated_assistant_output.lock().map(|s| s.len()).unwrap_or(0)
            );

            if !silent_retry_pending {
                emit_retry_event(
                    app_handle,
                    &params.execution_id,
                    retries,
                    max_retries,
                    last_error.as_ref(),
                    &accumulated_tool_calls,
                    &accumulated_assistant_output,
                );
            } else {
                tracing::info!(
                    "Silent retry scheduled (user-transparent) - execution_id: {}",
                    params.execution_id
                );
            }
            silent_retry_pending = false;

            // 重试前稍作延迟
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }

        let include_accumulated = retries > 0 || force_history_with_tools;
        let tool_calls_snapshot = accumulated_tool_calls
            .lock()
            .map(|calls| calls.clone())
            .unwrap_or_default();
        let output_snapshot = accumulated_assistant_output
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();
        let mut history_for_retry = build_retry_history(
            &base_history_messages,
            tool_calls_snapshot,
            output_snapshot,
            retries as u32,
            include_accumulated,
        );
        if loop_guard_prompt_needed.swap(false, Ordering::SeqCst) {
            history_for_retry.push(ChatMessage::user(
                "[LoopGuard] You are repeating the same tool call arguments and getting the same result. Do not repeat identical probes. First summarize what has been learned, then change strategy (different endpoint/input/query) or explicitly conclude insufficient evidence.",
            ));
        }
        force_history_with_tools = false;
        let result = client
            .stream_chat_with_dynamic_tools(
                final_system_prompt_content.as_deref(),
                &params.task,
                &history_for_retry,
                image_attachments.as_slice(),
                dynamic_tools.clone(),
                |content| {
                    if crate::commands::ai::is_conversation_cancelled(&execution_id) {
                        return false;
                    }
                    match content {
                        StreamContent::Text(text) => {
                            mark_first_response_ms(first_response_ms_for_stream.as_ref(), execution_started_at_ms);
                            if let Some(ctx) = team_stream_context.as_ref() {
                                if !team_stream_started.swap(true, Ordering::SeqCst) {
                                    let _ = app.emit(
                                        "agent_team:message_stream_start",
                                        &json!({
                                            "session_id": ctx.session_id.clone(),
                                            "stream_id": ctx.stream_id.clone(),
                                            "member_id": ctx.member_id.clone(),
                                            "member_name": ctx.member_id.clone(),
                                            "phase": ctx.phase.clone(),
                                        }),
                                    );
                                }
                                team_stream_had_delta.store(true, Ordering::SeqCst);
                                let _ = app.emit(
                                    "agent_team:message_stream_delta",
                                    &json!({
                                        "session_id": ctx.session_id.clone(),
                                        "stream_id": ctx.stream_id.clone(),
                                        "member_id": ctx.member_id.clone(),
                                        "member_name": ctx.member_id.clone(),
                                        "phase": ctx.phase.clone(),
                                        "delta": text.clone(),
                                    }),
                                );
                            }
                            // Accumulate assistant text into a segment buffer.
                            let _ = segment_buf.lock().map(|mut buf| buf.push_str(&text));

                            if params.enable_tenth_man_rule
                                && tenth_man_trigger_policy.review_low_evidence_high_confidence
                                && !low_evidence_warning_flag.load(Ordering::SeqCst)
                            {
                                let current_segment = segment_buf
                                    .lock()
                                    .map(|buf| buf.clone())
                                    .unwrap_or_default();
                                let current_focus_hint = hypothesis_tracker_for_stream
                                    .lock()
                                    .map(|mut tracker| {
                                        tracker.observe_text(&current_segment);
                                        tracker.focus_hint().map(str::to_string)
                                    })
                                    .unwrap_or(None);
                                let all_known_tool_calls = collect_all_tool_calls(
                                    &accumulated_tool_calls,
                                    &tool_calls_collector,
                                );

                                if streaming_content_needs_evidence_review(
                                    &current_segment,
                                    current_focus_hint.as_deref(),
                                    &all_known_tool_calls,
                                    tenth_man_trigger_policy.minimum_evidence_score(),
                                ) && !low_evidence_warning_flag.swap(true, Ordering::SeqCst)
                                {
                                    let require_confirmation = params
                                        .tenth_man_config
                                        .as_ref()
                                        .map(|config| config.require_user_confirmation)
                                        .unwrap_or(false);
                                    let has_recent_verification = all_known_tool_calls
                                        .iter()
                                        .rev()
                                        .take(tenth_man_trigger_policy.recent_verification_window())
                                        .any(|call| {
                                            looks_like_verification_tool_call(
                                                &call.name,
                                                &call.arguments,
                                                call.success,
                                            )
                                        });
                                    let last_tool_name = all_known_tool_calls
                                        .last()
                                        .map(|call| call.name.clone());
                                    let has_side_effects = all_known_tool_calls.iter().any(|call| {
                                        is_side_effectful_tool_call(
                                            &call.name,
                                            &call.arguments,
                                        )
                                    });
                                    let context = InterventionContext {
                                        execution_id: execution_id.clone(),
                                        task: params.task.clone(),
                                        tool_call_count: all_known_tool_calls.len(),
                                        recent_failure_count: trailing_failed_tool_calls(
                                            &all_known_tool_calls,
                                        ),
                                        last_tool_name,
                                        has_recent_verification,
                                        has_side_effects,
                                        current_content: Some(match &current_focus_hint {
                                            Some(focus_hint) => format!(
                                                "Current Hypothesis Focus:\n{}\n\nCurrent Content:\n{}",
                                                focus_hint, current_segment
                                            ),
                                            None => current_segment,
                                        }),
                                        trigger_reason: TriggerReason::LowEvidenceHighConfidence,
                                    };
                                    let tenth_man = TenthMan::new(&params);
                                    if tenth_man.should_trigger(&context) {
                                        spawn_tenth_man_warning(
                                            app.clone(),
                                            params.clone(),
                                            context,
                                            "streaming_low_evidence_high_confidence",
                                            require_confirmation,
                                            json!({
                                                "hypothesis_focus": current_focus_hint,
                                                "minimum_evidence_score": tenth_man_trigger_policy.minimum_evidence_score(),
                                            }),
                                        );
                                    }
                                }
                            }

                            let _ = app.emit(
                                "agent:chunk",
                                &json!({
                                    "execution_id": execution_id,
                                    "chunk_type": "text",
                                    "content": text,
                                }),
                            );
                        }
                        StreamContent::Reasoning(reasoning) => {
                            mark_first_response_ms(first_response_ms_for_stream.as_ref(), execution_started_at_ms);
                            // Accumulate reasoning content
                            if let Ok(mut buf) = reasoning_buf.lock() {
                                buf.push_str(&reasoning);
                            }
                            let _ = app.emit(
                                "agent:chunk",
                                &json!({
                                    "execution_id": execution_id,
                                    "chunk_type": "reasoning",
                                    "content": reasoning,
                                }),
                            );
                        }
                        StreamContent::ToolCallStart { id, name } => {
                            tracing::debug!("Tool call started via rig-core: {} ({})", name, id);

                            // Increment tool call counter
                            tool_counter.fetch_add(1, Ordering::SeqCst);

                            // Tenth Man Intervention Point 1: Before Tool Execution
                            if params.enable_tenth_man_rule {
                                let tenth_man = TenthMan::new(&params);
                                let current_count = tool_counter.load(Ordering::SeqCst) as usize;
                                let require_confirmation = params
                                    .tenth_man_config
                                    .as_ref()
                                    .map(|config| config.require_user_confirmation)
                                    .unwrap_or(false);

                                let context = InterventionContext {
                                    execution_id: execution_id.clone(),
                                    task: params.task.clone(),
                                    tool_call_count: current_count,
                                    recent_failure_count: 0,
                                    last_tool_name: Some(name.clone()),
                                    has_recent_verification: false,
                                    has_side_effects: false,
                                    current_content: Some(format!("Preparing to call tool: {}", name)),
                                    trigger_reason: TriggerReason::ToolCallThreshold,
                                };

                                if tenth_man.should_trigger(&context) {
                                    spawn_tenth_man_warning(
                                        app.clone(),
                                        params.clone(),
                                        context,
                                        "before_tool_call_threshold",
                                        require_confirmation,
                                        json!({
                                            "tool_name": name.clone(),
                                        }),
                                    );
                                }
                            }

                            let _ = app.emit(
                                "agent:tool_call_start",
                                &json!({
                                    "execution_id": execution_id,
                                    "tool_call_id": id,
                                    "tool_name": name,
                                }),
                            );
                        }
                        StreamContent::ToolCallDelta { id, delta } => {
                            let _ = app.emit(
                                "agent:tool_call_delta",
                                &json!({
                                    "execution_id": execution_id,
                                    "tool_call_id": id,
                                    "delta": delta,
                                }),
                            );
                        }
                        StreamContent::ToolCallComplete {
                            id,
                            name,
                            arguments,
                        } => {
                            mark_first_response_ms(first_response_ms_for_stream.as_ref(), execution_started_at_ms);
                            low_evidence_warning_flag.store(false, Ordering::SeqCst);
                            if let Ok(mut tracker) = hypothesis_tracker_for_stream.lock() {
                                tracker.clear();
                            }
                            tracing::debug!("Tool call complete via rig-core: {} ({})", name, id);
                            sentinel_llm::log::log_tool_call(
                                &execution_id,
                                Some(&execution_id),
                                &params.rig_provider,
                                &params.model,
                                &name,
                                &id,
                                &arguments,
                            );

                            // 记录 pending 的工具调用，等待结果
                            if let Ok(mut pending_map) = pending.lock() {
                                let seq = seq_counter.fetch_add(1, Ordering::Relaxed);
                                let started_at_ms = chrono::Utc::now().timestamp_millis() + seq as i64;
                                pending_map.insert(
                                    id.clone(),
                                    (name.clone(), arguments.clone(), started_at_ms, seq),
                                );
                            }

                            if params.enable_tenth_man_rule
                                && tenth_man_trigger_policy.review_high_risk_tools
                                && is_high_risk_tool_call(&name, &arguments)
                            {
                                let current_count = tool_counter.load(Ordering::SeqCst) as usize;
                                let existing_records = collector
                                    .lock()
                                    .map(|records| records.clone())
                                    .unwrap_or_default();
                                let require_confirmation = params
                                    .tenth_man_config
                                    .as_ref()
                                    .map(|config| config.require_user_confirmation)
                                    .unwrap_or(false);
                                let has_recent_verification = existing_records
                                    .iter()
                                    .rev()
                                    .take(tenth_man_trigger_policy.recent_verification_window())
                                    .any(|record| {
                                        looks_like_verification_tool_call(
                                            &record.name,
                                            &record.arguments,
                                            record.success,
                                        )
                                    });
                                let context = InterventionContext {
                                    execution_id: execution_id.clone(),
                                    task: params.task.clone(),
                                    tool_call_count: current_count,
                                    recent_failure_count: trailing_failed_tool_calls(
                                        &existing_records,
                                    ),
                                    last_tool_name: Some(name.clone()),
                                    has_recent_verification,
                                    has_side_effects: true,
                                    current_content: Some(format!(
                                        "About to execute high-risk tool call.\nTool: {}\nArguments: {}",
                                        name, arguments
                                    )),
                                    trigger_reason: TriggerReason::HighRiskTool(name.clone()),
                                };
                                let tenth_man = TenthMan::new(&params);
                                if tenth_man.should_trigger(&context) {
                                    spawn_tenth_man_warning(
                                        app.clone(),
                                        params.clone(),
                                        context,
                                        "high_risk_tool_call",
                                        require_confirmation,
                                        json!({
                                            "tool_call_id": id.clone(),
                                            "tool_name": name.clone(),
                                            "arguments": arguments.clone(),
                                        }),
                                    );
                                }
                            }

                            // Flush assistant segment BEFORE inserting tool call message (preserve ordering on reload).
                            if let Some(db) = db_for_stream.clone() {
                                use sentinel_core::models::database as core_db;
                                use chrono::TimeZone;
                                let seg = segment_buf
                                    .lock()
                                    .map(|mut g| std::mem::take(&mut *g))
                                    .unwrap_or_default();
                                let seg_trimmed = seg.trim().to_string();
                                if !seg_trimmed.trim().is_empty() {
                                    persisted_seg_count.fetch_add(1, Ordering::SeqCst);
                                    // Ensure segment timestamp is slightly before tool call timestamp.
                                    let seg_ts_ms = chrono::Utc::now().timestamp_millis() - 1;
                                    let seg_ts = chrono::Utc
                                        .timestamp_millis_opt(seg_ts_ms)
                                        .single()
                                        .unwrap_or_else(chrono::Utc::now);

                                    // Get reasoning content (for deepseek-reasoner with tool calls, always include it)
                                    // 参考：https://api-docs.deepseek.com/zh-cn/guides/thinking_mode#tool-calls
                                    let reasoning = reasoning_buf
                                        .lock()
                                        .map(|g| {
                                            let r = g.clone();
                                            // 即使为空也返回 Some("")，因为 deepseek-reasoner 要求必须有此字段
                                            Some(if r.trim().is_empty() {
                                                String::new()
                                            } else {
                                                r
                                            })
                                        })
                                        .ok()
                                        .flatten();

                                    let seg_msg = core_db::AiMessage {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        conversation_id: execution_id.clone(),
                                        role: "assistant".to_string(),
                                        content: seg_trimmed.clone(),
                                        metadata: None,
                                        token_count: Some(seg_trimmed.len() as i32),
                                        cost: None,
                                        tool_calls: None,
                                        attachments: None,
                                        reasoning_content: reasoning,
                                        timestamp: seg_ts,
                                        architecture_type: None,
                                        architecture_meta: None,
                                        structured_data: None,
                                    };
                                    tauri::async_runtime::spawn(async move {
                                        persist_ai_message_with_retry(
                                            db,
                                            seg_msg,
                                            "assistant segment",
                                        )
                                        .await;
                                    });
                                }
                            }

                            // Persist tool call as a standalone message (role=tool) so history ordering is correct.
                            if let Some(db) = db_for_stream.clone() {
                                use sentinel_core::models::database as core_db;
                                use chrono::TimeZone;
                                let (started_at_ms, seq) = pending
                                    .lock()
                                    .ok()
                                    .and_then(|m| m.get(&id).map(|(_, _, ms, s)| (*ms, *s)))
                                    .unwrap_or((chrono::Utc::now().timestamp_millis(), 0));

                                let started_at = chrono::Utc
                                    .timestamp_millis_opt(started_at_ms)
                                    .single()
                                    .unwrap_or_else(chrono::Utc::now);

                                let tool_args_val =
                                    normalize_tool_call_arguments_str(&name, &arguments);
                                let meta = json!({
                                    "kind": "tool_call",
                                    "tool_name": name,
                                    "tool_args": tool_args_val,
                                    "tool_call_id": id,
                                    "status": "running",
                                    "sequence": seq,
                                    "started_at_ms": started_at_ms,
                                });

                                let tool_msg = core_db::AiMessage {
                                    id: id.clone(),
                                    conversation_id: execution_id.clone(),
                                    role: "tool".to_string(),
                                    content: String::new(),
                                    metadata: Some(meta.to_string()),
                                    token_count: None,
                                    cost: None,
                                    tool_calls: None,
                                    attachments: None,
                                    reasoning_content: None,
                                    timestamp: started_at,
                                    architecture_type: None,
                                    architecture_meta: None,
                                    structured_data: None,
                                };
                                tauri::async_runtime::spawn(async move {
                                    persist_ai_message_with_retry(db, tool_msg, "tool call message")
                                        .await;
                                });
                            }

                            let team_tool_call_id = id.clone();
                            let team_tool_name = name.clone();
                            let team_tool_arguments = arguments.clone();
                            let _ = app.emit(
                                "agent:tool_call_complete",
                                &json!({
                                    "execution_id": execution_id,
                                    "tool_call_id": id,
                                    "tool_name": name,
                                    "arguments": arguments,
                                }),
                            );
                            if let Some(ctx) = team_stream_context.as_ref() {
                                let _ = app.emit(
                                    "agent_team:tool_call",
                                    &json!({
                                        "session_id": ctx.session_id.clone(),
                                        "stream_id": ctx.stream_id.clone(),
                                        "member_id": ctx.member_id.clone(),
                                        "member_name": ctx.member_id.clone(),
                                        "phase": ctx.phase.clone(),
                                        "tool_call_id": team_tool_call_id,
                                        "name": team_tool_name,
                                        "arguments": team_tool_arguments,
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                    }),
                                );
                            }
                        }
                        StreamContent::ToolResult { id, result } => {
                            // tracing::info!(
                            //     "Tool result via rig-core: id={}, result_preview={}",
                //     id,
                //     &result.chars().take(500).collect::<String>()
                // );

                            // 将工具调用完整信息添加到收集器
                            if let Ok(mut pending_map) = pending.lock() {
                                if let Some((name, arguments, started_at_ms, seq)) =
                                    pending_map.remove(&id)
                                {
                                    let completed_at_ms = chrono::Utc::now().timestamp_millis();
                                    let duration_ms = completed_at_ms.saturating_sub(started_at_ms);
                                    let name_for_meta = name.clone();
                                    let args_for_meta = arguments.clone();
                                    let tool_success = infer_tool_result_success(&result);
                                    sentinel_llm::log::log_tool_result(
                                        &execution_id,
                                        Some(&execution_id),
                                        &params.rig_provider,
                                        &params.model,
                                        &name_for_meta,
                                        &id,
                                        Some(duration_ms),
                                        tool_success,
                                        &result,
                                    );
                                    if !tool_success && is_toolset_error_result(&result) {
                                        log_toolset_error_with_context(
                                            team_log_context_for_stream.as_ref(),
                                            &execution_id,
                                            &name_for_meta,
                                            &id,
                                            &result,
                                        );
                                    }
                                    let record = ToolCallRecord {
                                        id: id.clone(),
                                        name,
                                        arguments,
                                        result: Some(result.clone()),
                                        success: tool_success,
                                        sequence: seq,
                                        started_at_ms,
                                        completed_at_ms,
                                        duration_ms,
                                    };
                                    if let Ok(mut records) = collector.lock() {
                                        records.push(record.clone());
                                    }
                                    let current_records = collector
                                        .lock()
                                        .map(|records| records.clone())
                                        .unwrap_or_default();
                                    append_execution_tool_trace(&execution_id, record.clone());
                                    if execution_id.starts_with("sar-") {
                                        let _ = app_handle.emit(
                                            "system-agent:tool-call-recorded",
                                            json!({
                                                "runId": execution_id,
                                                "toolCall": record,
                                            }),
                                        );
                                    }

                                    // Update persisted tool message with result (keep timestamp as started_at to avoid reordering).
                                    if let Some(db) = db_for_stream.clone() {
                                        use sentinel_core::models::database as core_db;
                                        use chrono::TimeZone;
                                        let started_at = chrono::Utc
                                            .timestamp_millis_opt(started_at_ms)
                                            .single()
                                            .unwrap_or_else(chrono::Utc::now);

                                        let tool_args_val = normalize_tool_call_arguments_str(
                                            &name_for_meta,
                                            &args_for_meta,
                                        );
                                        let meta = json!({
                                            "kind": "tool_call",
                                            "tool_name": name_for_meta,
                                            "tool_args": tool_args_val,
                                            "tool_call_id": id,
                                            "status": if tool_success { "completed" } else { "failed" },
                                            "sequence": seq,
                                            "started_at_ms": started_at_ms,
                                            "completed_at_ms": completed_at_ms,
                                            "duration_ms": duration_ms,
                                            "tool_result": result,
                                            "success": tool_success,
                                        });
                                        let tool_msg = core_db::AiMessage {
                                            id: id.clone(),
                                            conversation_id: execution_id.clone(),
                                            role: "tool".to_string(),
                                            content: String::new(),
                                            metadata: Some(meta.to_string()),
                                            token_count: None,
                                            cost: None,
                                            tool_calls: None,
                                            attachments: None,
                                            reasoning_content: None,
                                            timestamp: started_at,
                                            architecture_type: None,
                                            architecture_meta: None,
                                            structured_data: None,
                                        };
                                        tauri::async_runtime::spawn(async move {
                                            persist_ai_message_with_retry(
                                                db,
                                                tool_msg,
                                                "tool result update",
                                            )
                                            .await;
                                        });
                                    }

                                    if name_for_meta == "skills" {
                                        if let Ok(args_json) =
                                            serde_json::from_str::<serde_json::Value>(&args_for_meta)
                                        {
                                            if args_json
                                                .get("action")
                                                .and_then(|v| v.as_str())
                                                .map(|a| a == "load")
                                                .unwrap_or(false)
                                            {
                                                if let Some(skill_id) = args_json
                                                    .get("skill_id")
                                                    .and_then(|v| v.as_str())
                                                {
                                                    if let Ok(mut slot) = loaded_skill_id.lock() {
                                                        *slot = Some(skill_id.to_string());
                                                    }
                                                    skill_reload_requested
                                                        .store(true, Ordering::SeqCst);
                                                }
                                            }
                                        }
                                    }

                                    if name_for_meta == ToolSearchTool::NAME {
                                        let result_json =
                                            serde_json::from_str::<serde_json::Value>(&result)
                                                .unwrap_or_else(|_| json!({}));
                                        let should_reload = result_json
                                            .get("requires_reload")
                                            .and_then(|value| value.as_bool())
                                            .unwrap_or(false);
                                        if should_reload {
                                            let requested = result_json
                                                .get("activated_tool_ids")
                                                .and_then(|value| value.as_array())
                                                .map(|items| {
                                                    items
                                                        .iter()
                                                        .filter_map(|item| {
                                                            item.as_str().map(str::to_string)
                                                        })
                                                        .collect::<Vec<_>>()
                                                })
                                                .unwrap_or_default();
                                            if !requested.is_empty() {
                                                if let Ok(mut slot) = activated_tool_ids.lock() {
                                                    *slot = requested;
                                                }
                                                if let Ok(mut slot) =
                                                    activated_tool_runtime_hint.lock()
                                                {
                                                    *slot = result_json
                                                        .get("runtime_hint")
                                                        .and_then(|value| value.as_str())
                                                        .map(str::to_string);
                                                }
                                                if let Ok(args_json) =
                                                    serde_json::from_str::<serde_json::Value>(
                                                        &args_for_meta,
                                                    )
                                                {
                                                    if let Ok(mut slot) = activated_tool_query.lock()
                                                    {
                                                        *slot = args_json
                                                            .get("query")
                                                            .and_then(|value| value.as_str())
                                                            .map(str::to_string);
                                                    }
                                                }
                                                tool_activation_reload_requested
                                                    .store(true, Ordering::SeqCst);
                                            }
                                        }
                                    }

                                    let args_value: serde_json::Value = serde_json::from_str(&args_for_meta)
                                        .unwrap_or_else(|_| json!({ "raw": args_for_meta }));
                                    let digest = build_tool_digest(&name_for_meta, &args_value, &result);
                                    if let Ok(mut tracked) = tracked_artifacts_for_stream.lock() {
                                        apply_tool_digest_to_tracked_artifacts(&mut tracked, &digest);
                                    }
                                    if let Ok(mut queue) = pending_digests.lock() {
                                        queue.push(digest);
                                    }

                                    let fingerprint =
                                        tool_loop_fingerprint(&name_for_meta, &args_for_meta, &result);
                                    let mut loop_triggered = false;
                                    let mut repeat_hits = 0usize;
                                    if let Ok(mut last_slot) = last_tool_fp.lock() {
                                        let same_as_last = last_slot
                                            .as_ref()
                                            .map(|prev| *prev == fingerprint)
                                            .unwrap_or(false);
                                        if same_as_last {
                                            repeat_hits =
                                                repeated_tool_fp_count.fetch_add(1, Ordering::SeqCst) + 1;
                                            if repeat_hits
                                                >= tenth_man_trigger_policy.loop_repeat_threshold()
                                            {
                                                loop_triggered = true;
                                            }
                                        } else {
                                            *last_slot = Some(fingerprint);
                                            repeated_tool_fp_count.store(0, Ordering::SeqCst);
                                        }
                                    }

                                    if params.enable_tenth_man_rule {
                                        let require_confirmation = params
                                            .tenth_man_config
                                            .as_ref()
                                            .map(|config| config.require_user_confirmation)
                                            .unwrap_or(false);
                                        let recent_failure_count =
                                            trailing_failed_tool_calls(&current_records);
                                        let has_recent_verification = current_records
                                            .iter()
                                            .rev()
                                            .take(tenth_man_trigger_policy.recent_verification_window())
                                            .any(|call| {
                                                looks_like_verification_tool_call(
                                                    &call.name,
                                                    &call.arguments,
                                                    call.success,
                                                )
                                            });

                                        if tenth_man_trigger_policy.review_repeated_failures
                                            && !tool_success
                                            && recent_failure_count
                                                >= tenth_man_trigger_policy
                                                    .repeated_failure_streak()
                                        {
                                            let context = InterventionContext {
                                                execution_id: execution_id.clone(),
                                                task: params.task.clone(),
                                                tool_call_count: current_records.len(),
                                                recent_failure_count,
                                                last_tool_name: Some(name_for_meta.clone()),
                                                has_recent_verification,
                                                has_side_effects: current_records
                                                    .iter()
                                                    .any(|call| {
                                                        is_side_effectful_tool_call(
                                                            &call.name,
                                                            &call.arguments,
                                                        )
                                                    }),
                                                current_content: Some(format!(
                                                    "Recent tools are failing repeatedly.\nLatest tool: {}\nArguments: {}\nResult: {}",
                                                    name_for_meta, args_for_meta, result
                                                )),
                                                trigger_reason: TriggerReason::RepeatedFailurePattern,
                                            };
                                            let tenth_man = TenthMan::new(&params);
                                            if tenth_man.should_trigger(&context) {
                                                spawn_tenth_man_warning(
                                                    app.clone(),
                                                    params.clone(),
                                                    context,
                                                    "repeated_failure_pattern",
                                                    require_confirmation,
                                                    json!({
                                                        "tool_call_id": id.clone(),
                                                        "tool_name": name_for_meta.clone(),
                                                        "failure_streak": recent_failure_count,
                                                    }),
                                                );
                                            }
                                        }
                                    }

                                    if loop_triggered {
                                        loop_break_flag.store(true, Ordering::SeqCst);
                                        loop_prompt_flag.store(true, Ordering::SeqCst);
                                        tracing::warn!(
                                            "Detected repeated tool loop - execution_id: {}, tool: {}, repeats: {}",
                                            execution_id,
                                            name_for_meta,
                                            repeat_hits
                                        );
                                        let _ = app.emit(
                                            "agent:loop_detected",
                                            &json!({
                                                "execution_id": execution_id,
                                                "tool_name": name_for_meta,
                                                "repeat_count": repeat_hits,
                                                "reason": "repeated identical tool arguments and result"
                                            }),
                                        );

                                        if params.enable_tenth_man_rule
                                            && tenth_man_trigger_policy.review_loops
                                        {
                                            let require_confirmation = params
                                                .tenth_man_config
                                                .as_ref()
                                                .map(|config| config.require_user_confirmation)
                                                .unwrap_or(false);
                                            let context = InterventionContext {
                                                execution_id: execution_id.clone(),
                                                task: params.task.clone(),
                                                tool_call_count: current_records.len(),
                                                recent_failure_count: trailing_failed_tool_calls(
                                                    &current_records,
                                                ),
                                                last_tool_name: Some(name_for_meta.clone()),
                                                has_recent_verification: current_records
                                                    .iter()
                                                    .rev()
                                                    .take(
                                                        tenth_man_trigger_policy
                                                            .recent_verification_window(),
                                                    )
                                                    .any(|call| {
                                                        looks_like_verification_tool_call(
                                                            &call.name,
                                                            &call.arguments,
                                                            call.success,
                                                        )
                                                    }),
                                                has_side_effects: current_records
                                                    .iter()
                                                    .any(|call| {
                                                        is_side_effectful_tool_call(
                                                            &call.name,
                                                            &call.arguments,
                                                        )
                                                    }),
                                                current_content: Some(format!(
                                                    "Detected repeated identical tool loop.\nTool: {}\nArguments: {}\nResult: {}",
                                                    name_for_meta, args_for_meta, result
                                                )),
                                                trigger_reason: TriggerReason::LoopDetected,
                                            };
                                            let tenth_man = TenthMan::new(&params);
                                            if tenth_man.should_trigger(&context) {
                                                spawn_tenth_man_warning(
                                                    app.clone(),
                                                    params.clone(),
                                                    context,
                                                    "loop_detected",
                                                    require_confirmation,
                                                    json!({
                                                        "tool_call_id": id.clone(),
                                                        "tool_name": name_for_meta.clone(),
                                                        "repeat_count": repeat_hits,
                                                    }),
                                                );
                                            }
                                        }
                                    }
                                }
                            }

                            let tracked_artifacts_snapshot: Option<Vec<TrackedArtifact>> =
                                tracked_artifacts_for_stream
                                    .lock()
                                    .ok()
                                    .map(|tracked| tracked.clone());
                            let team_tool_call_id = id.clone();
                            let team_result = result.clone();
                            let team_success = infer_tool_result_success(&result);
                            let _ = app.emit(
                                "agent:tool_result",
                                &json!({
                                    "execution_id": execution_id,
                                    "tool_call_id": id,
                                    "result": result,
                                    "success": team_success,
                                    "tracked_artifacts": tracked_artifacts_snapshot,
                                }),
                            );
                            if let Some(ctx) = team_stream_context.as_ref() {
                                let _ = app.emit(
                                    "agent_team:tool_result",
                                    &json!({
                                        "session_id": ctx.session_id.clone(),
                                        "stream_id": ctx.stream_id.clone(),
                                        "member_id": ctx.member_id.clone(),
                                        "member_name": ctx.member_id.clone(),
                                        "phase": ctx.phase.clone(),
                                        "tool_call_id": team_tool_call_id,
                                        "result": team_result,
                                        "success": team_success,
                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                    }),
                                );
                            }
                        }
                StreamContent::Usage {
                    input_tokens,
                    output_tokens,
                } => {
                    if let Ok(mut guard) = usage_data_for_stream.lock() {
                        *guard = Some((input_tokens, output_tokens));
                    }
                    tracing::info!(
                        "Token usage report - execution_id: {}, input: {}, output: {}, total: {}",
                        execution_id,
                        input_tokens,
                        output_tokens,
                        input_tokens + output_tokens
                    );
                    let _ = app.emit(
                                "agent:chunk",
                                &json!({
                                    "execution_id": execution_id,
                                    "chunk_type": "usage",
                                    "input_tokens": input_tokens,
                                    "output_tokens": output_tokens,
                                }),
                            );

                            // 记录 token 使用统计到数据库
                            if input_tokens > 0 || output_tokens > 0 {
                                use tauri::Manager;
                                if let Some(db) =
                                    app.try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
                                {
                                    let provider = params.rig_provider.clone();
                                    let model = params.model.clone();
                                    let cost = sentinel_llm::calculate_cost(
                                        &provider,
                                        &model,
                                        input_tokens,
                                        output_tokens,
                                    );

                                    let db_clone = db.inner().clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = db_clone
                                            .update_ai_usage(
                                                &provider,
                                                &model,
                                                input_tokens as i32,
                                                output_tokens as i32,
                                                cost,
                                            )
                                            .await
                                        {
                                            tracing::warn!("Failed to update AI usage stats: {}", e);
                                        } else {
                                            tracing::info!(
                                                "Updated AI usage: provider={}, model={}, input={}, output={}, cost=${:.4}",
                                                provider, model, input_tokens, output_tokens, cost
                                            );
                                        }
                                    });
                                }
                            }
                        }
                        StreamContent::Done => {
                            tracing::info!("Stream completed - execution_id: {}", execution_id);
                            // Always clear the segment buffer. Persisting the final segment here will duplicate the
                            // assistant message because we also persist the final response in `save_assistant_message`.
                            // The final response includes tool_calls metadata and is the canonical persisted message.
                            let _ = segment_buf
                                .lock()
                                .map(|mut g| std::mem::take(&mut *g))
                                .unwrap_or_default();
                        }
                    }
                    if loop_break_flag.load(Ordering::SeqCst) {
                        return false;
                    }
                    if skill_reload_requested.load(Ordering::SeqCst) {
                        return false;
                    }
                    if tool_activation_reload_requested.load(Ordering::SeqCst) {
                        return false;
                    }
                    true
                },
            )
            .await;

        let digests_to_flush = pending_tool_digests
            .lock()
            .map(|mut queue| std::mem::take(&mut *queue))
            .unwrap_or_default();
        if let Err(e) = append_tool_digests(
            app_handle,
            &params.execution_id,
            digests_to_flush,
            &context_policy_for_stream,
        )
        .await
        {
            tracing::warn!("Failed to flush tool digests: {}", e);
        }

        if tool_activation_reload_requested.load(Ordering::SeqCst) {
            if tool_activation_reload_count >= max_tool_activation_reload {
                tool_activation_reload_requested.store(false, Ordering::SeqCst);
            } else {
                tool_activation_reload_count += 1;

                accumulate_progress(
                    &tool_calls_collector,
                    &accumulated_tool_calls,
                    &assistant_segment_buf,
                    &accumulated_assistant_output,
                    Some(&pending),
                );

                let mut requested_tools = if let Ok(mut slot) = activated_tool_ids.lock() {
                    std::mem::take(&mut *slot)
                } else {
                    Vec::new()
                };
                let activation_query = if let Ok(mut slot) = activated_tool_query.lock() {
                    slot.take()
                } else {
                    None
                };
                let activation_runtime_hint =
                    if let Ok(mut slot) = activated_tool_runtime_hint.lock() {
                        slot.take()
                    } else {
                        None
                    };

                if !requested_tools.is_empty() {
                    let mut next_tools = tool_config.fixed_tools.clone();
                    next_tools.extend(requested_tools.clone());
                    next_tools.extend(current_tool_ids.clone());

                    let available_tools = tool_server
                        .list_tools()
                        .await
                        .into_iter()
                        .map(|t| t.name)
                        .collect::<std::collections::HashSet<_>>();
                    let mut seen = std::collections::HashSet::new();
                    next_tools.retain(|id| seen.insert(id.clone()));
                    next_tools.retain(|id| available_tools.contains(id));
                    next_tools.retain(|id| !tool_config.disabled_tools.contains(id));
                    current_tool_ids =
                        apply_allowed_tools_policy(next_tools, &tool_config.allowed_tools);

                    if current_tool_ids.len() > tool_config.max_tools {
                        current_tool_ids.truncate(tool_config.max_tools);
                    }

                    requested_tools.retain(|id| current_tool_ids.contains(id));

                    emit_and_persist_tool_activation(
                        app_handle,
                        &params.execution_id,
                        &requested_tools,
                        activation_query,
                        activation_runtime_hint,
                        &current_tool_ids,
                        db_for_stream.clone(),
                    );
                }

                tool_activation_reload_requested.store(false, Ordering::SeqCst);
                low_evidence_warning_issued.store(false, Ordering::SeqCst);
                if let Ok(mut tracker) = hypothesis_tracker.lock() {
                    tracker.clear();
                }
                force_history_with_tools = true;
                continue;
            }
        }

        if skill_reload_requested.load(Ordering::SeqCst) {
            if skill_reload_count >= max_skill_reload {
                skill_reload_requested.store(false, Ordering::SeqCst);
            } else {
                skill_reload_count += 1;

                accumulate_progress(
                    &tool_calls_collector,
                    &accumulated_tool_calls,
                    &assistant_segment_buf,
                    &accumulated_assistant_output,
                    Some(&pending),
                );

                let skill_id = if let Ok(mut slot) = loaded_skill_id.lock() {
                    slot.take()
                } else {
                    None
                };
                if let Some(skill_id) = skill_id {
                    if let Some(db) =
                        app_handle.try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
                    {
                        if let Ok(Some(skill)) = db.get_skill(&skill_id).await {
                            let mut next_tools = vec![
                                "skills".to_string(),
                                "tasks".to_string(),
                                "http_request".to_string(),
                                "spawn_agent".to_string(),
                                "wait_agents".to_string(),
                                "list_agents".to_string(),
                                "close_agent".to_string(),
                                "tenth_man_review".to_string(),
                            ];
                            if !tool_config
                                .disabled_tools
                                .contains(&ShellTool::NAME.to_string())
                            {
                                next_tools.push(ShellTool::NAME.to_string());
                            }
                            next_tools.extend(skill.allowed_tools.clone());
                            next_tools.extend(tool_config.fixed_tools.clone());
                            let available_tools = tool_server
                                .list_tools()
                                .await
                                .into_iter()
                                .map(|t| t.name)
                                .collect::<std::collections::HashSet<_>>();
                            let mut seen = std::collections::HashSet::new();
                            next_tools.retain(|id| seen.insert(id.clone()));
                            next_tools.retain(|id| available_tools.contains(id));
                            next_tools.retain(|id| !tool_config.disabled_tools.contains(id));
                            current_tool_ids = apply_allowed_tools_policy(
                                next_tools.clone(),
                                &tool_config.allowed_tools,
                            );
                            let _ = app_handle.emit(
                                "agent:tools_selected",
                                &json!({
                                    "execution_id": params.execution_id,
                                    "tools": current_tool_ids,
                                }),
                            );
                            emit_and_persist_skill_loaded(
                                app_handle,
                                &params.execution_id,
                                &skill.id,
                                &skill.name,
                                db_for_stream.clone(),
                            );
                        }
                    }
                }

                skill_reload_requested.store(false, Ordering::SeqCst);
                low_evidence_warning_issued.store(false, Ordering::SeqCst);
                if let Ok(mut tracker) = hypothesis_tracker.lock() {
                    tracker.clear();
                }
                force_history_with_tools = true;
                continue;
            }
        }

        if loop_break_requested.load(Ordering::SeqCst) {
            loop_break_requested.store(false, Ordering::SeqCst);
            let loop_err = anyhow::anyhow!(
                "Detected repeated identical tool loop; retrying with loop-guard context"
            );
            if retries < max_retries {
                retries += 1;
                last_error = Some(loop_err);

                accumulate_retry_progress(
                    &tool_calls_collector,
                    &accumulated_tool_calls,
                    &assistant_segment_buf,
                    &accumulated_assistant_output,
                );
                clear_retry_turn_state(
                    &assistant_segment_buf,
                    &reasoning_content_buf,
                    &pending,
                    &tool_calls_collector,
                    Some(&last_tool_fingerprint),
                    Some(repeated_tool_fingerprint_count.as_ref()),
                );
                low_evidence_warning_issued.store(false, Ordering::SeqCst);
                if let Ok(mut tracker) = hypothesis_tracker.lock() {
                    tracker.clear();
                }
                force_history_with_tools = true;
                continue;
            }
            return Err(loop_err);
        }

        match result {
            Ok(response) => {
                let (full_response, final_response, seg_count) = finalize_response_state(
                    &response,
                    &accumulated_assistant_output,
                    &persisted_segment_count,
                );
                let all_tool_calls =
                    collect_all_tool_calls(&accumulated_tool_calls, &tool_calls_collector);

                tracing::info!(
                    "Agent with tools completed - execution_id: {}, final_save_length: {}, full_response_length: {}, persisted_segments: {}",
                    params.execution_id,
                    final_response.len(),
                    full_response.len(),
                    seg_count
                );

                let tool_summaries = all_tool_calls
                    .iter()
                    .map(|call| ToolCallSummary {
                        name: call.name.clone(),
                        success: call.success,
                        duration_ms: Some(call.duration_ms),
                    })
                    .collect::<Vec<_>>();

                tracing::info!(
                    "Total tool calls completed: {} (accumulated: {}, current: {})",
                    tool_summaries.len(),
                    accumulated_tool_calls.lock().map(|c| c.len()).unwrap_or(0),
                    tool_calls_collector.lock().map(|c| c.len()).unwrap_or(0)
                );

                let tool_names_for_reflection: Vec<String> = all_tool_calls
                    .iter()
                    .map(|c| c.name.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();

                if let Err(e) = get_global_memory()
                    .record_execution(ExecutionRecord {
                        id: params.execution_id.clone(),
                        task: params.task.clone(),
                        environment: Some(rig_provider.clone()),
                        tool_calls: tool_summaries,
                        success: true,
                        error: None,
                        response_excerpt: Some(truncate_for_memory(&full_response, 400)),
                        created_at: chrono::Utc::now().timestamp(),
                    })
                    .await
                {
                    tracing::warn!("Failed to store memory record: {}", e);
                }

                record_execution_reflection(
                    app_handle,
                    &ExecutionOutcome {
                        execution_id: params.execution_id.clone(),
                        task: params.task.clone(),
                        success: true,
                        error: None,
                        tool_names_used: tool_names_for_reflection,
                        response_excerpt: Some(truncate_for_memory(&full_response, 200)),
                    },
                )
                .await;
                if let Err(err) = apply_sentinel_execution_outcome(
                    app_handle,
                    &params.execution_id,
                    true,
                    Some(&full_response),
                    None,
                )
                .await
                {
                    tracing::warn!("Failed to update sentinel execution outcome: {}", err);
                }

                let reasoning_content = reasoning_content_buf
                    .lock()
                    .ok()
                    .map(|s| s.clone())
                    .filter(|s| !s.trim().is_empty());

                let tool_calls_slice = if all_tool_calls.is_empty() {
                    None
                } else {
                    Some(all_tool_calls.as_slice())
                };
                let (input_tokens, output_tokens) = if let Ok(guard) = usage_data.lock() {
                    guard.unwrap_or((0, 0))
                } else {
                    (0, 0)
                };
                let session_metadata = build_assistant_session_stats_metadata(
                    Some(chrono::Utc::now().timestamp_millis() - execution_started_at_ms),
                    first_response_ms.lock().ok().and_then(|guard| *guard),
                    Some(input_tokens),
                    Some(output_tokens),
                );

                save_assistant_message(
                    app_handle,
                    &params.execution_id,
                    &final_response,
                    tool_calls_slice,
                    reasoning_content,
                    session_metadata,
                    params.persist_messages,
                    params.subagent_run_id.as_deref(),
                )
                .await;

                // Tenth Man Rule: Adversarial Review (System-enforced final check)
                if params.enable_tenth_man_rule {
                    let tenth_man = TenthMan::new(&params);
                    let final_focus_hint = hypothesis_tracker
                        .lock()
                        .ok()
                        .and_then(|tracker| tracker.focus_hint().map(str::to_string));
                    let requires_verification_review = tenth_man_trigger_policy
                        .review_final_response_without_verification
                        && final_response_needs_verification_review(
                            &final_response,
                            &all_tool_calls,
                        );
                    let requires_evidence_review = tenth_man_trigger_policy
                        .review_low_evidence_high_confidence
                        && final_response_needs_evidence_review(
                            &final_response,
                            final_focus_hint.as_deref(),
                            &all_tool_calls,
                            tenth_man_trigger_policy.minimum_evidence_score(),
                        );

                    // Check if we should run final review based on mode
                    let mut final_trigger = "final_review";
                    let should_run_final = if let Some(ref config) = params.tenth_man_config {
                        match &config.mode {
                            InterventionMode::SystemOnly => true,
                            InterventionMode::Hybrid {
                                force_final_review, ..
                            } => {
                                if requires_evidence_review && requires_verification_review {
                                    final_trigger = "final_response_low_evidence_and_unverified";
                                    true
                                } else if requires_evidence_review {
                                    final_trigger = "final_response_low_evidence_high_confidence";
                                    true
                                } else if requires_verification_review {
                                    final_trigger = "final_response_without_verification";
                                    true
                                } else {
                                    *force_final_review
                                }
                            }
                            InterventionMode::ToolOnly => false,
                            _ => {
                                if requires_evidence_review && requires_verification_review {
                                    final_trigger = "final_response_low_evidence_and_unverified";
                                } else if requires_evidence_review {
                                    final_trigger = "final_response_low_evidence_high_confidence";
                                } else if requires_verification_review {
                                    final_trigger = "final_response_without_verification";
                                }
                                true
                            } // Legacy modes default to true
                        }
                    } else {
                        if requires_evidence_review && requires_verification_review {
                            final_trigger = "final_response_low_evidence_and_unverified";
                        } else if requires_evidence_review {
                            final_trigger = "final_response_low_evidence_high_confidence";
                        } else if requires_verification_review {
                            final_trigger = "final_response_without_verification";
                        }
                        true // Default: run final review
                    };

                    if should_run_final {
                        tracing::info!(
                            "Running Tenth Man final review with full history for execution_id: {} (trigger={})",
                            params.execution_id,
                            final_trigger
                        );

                        match tenth_man.review_with_history(&params.execution_id).await {
                            Ok(critique) => {
                                tracing::info!(
                                    "Tenth Man Critique generated ({} chars)",
                                    critique.len()
                                );

                                if params.persist_messages {
                                    if let Some(db) = db_for_stream.clone() {
                                        use sentinel_core::models::database as core_db;
                                        let review_msg = core_db::AiMessage {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            conversation_id: params.execution_id.clone(),
                                            role: "system".to_string(),
                                            content: critique.clone(),
                                            metadata: Some(
                                                json!({
                                                    "kind": "tenth_man_critique",
                                                    "trigger": final_trigger,
                                                    "mode": "system_enforced"
                                                })
                                                .to_string(),
                                            ),
                                            token_count: Some(critique.len() as i32),
                                            cost: None,
                                            tool_calls: None,
                                            attachments: None,
                                            reasoning_content: None,
                                            timestamp: chrono::Utc::now(),
                                            architecture_type: None,
                                            architecture_meta: None,
                                            structured_data: None,
                                        };

                                        if let Err(e) = db.create_ai_message(&review_msg).await {
                                            tracing::warn!(
                                                "Failed to save Tenth Man critique: {}",
                                                e
                                            );
                                        }

                                        // Emit event to frontend
                                        let _ = app.emit(
                                            "agent:tenth_man_critique",
                                            &json!({
                                                "execution_id": params.execution_id,
                                                "critique": critique,
                                                "message_id": review_msg.id,
                                                "trigger": final_trigger,
                                                "mode": "system_enforced"
                                            }),
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Tenth Man Review failed: {}", e);
                            }
                        }
                    } else {
                        tracing::info!("Skipping final Tenth Man review (mode: ToolOnly)");
                    }
                }

                // Cleanup Tenth Man execution context
                if params.enable_tenth_man_rule {
                    use crate::agents::tenth_man_executor;
                    tenth_man_executor::clear_tenth_man_execution(&params.execution_id).await;
                }

                // Cleanup container context files (keep history.txt)
                cleanup_container_context_async(&app, &params.execution_id).await;

                if let Some(ctx) = team_stream_context.as_ref() {
                    let _ = app.emit(
                        "agent_team:message_stream_done",
                        &json!({
                            "session_id": ctx.session_id.clone(),
                            "stream_id": ctx.stream_id.clone(),
                            "member_id": ctx.member_id.clone(),
                            "member_name": ctx.member_id.clone(),
                            "phase": ctx.phase.clone(),
                            "content": final_response.clone(),
                            "had_delta": team_stream_had_delta.load(Ordering::SeqCst),
                        }),
                    );
                }

                return Ok(final_response);
            }
            Err(e) => {
                let err_msg = e.to_string();

                // Cleanup container context files even on error
                cleanup_container_context_async(&app, &params.execution_id).await;

                // 优化错误消息
                let friendly_err = if err_msg.contains("error decoding response body") {
                    if err_msg.contains("UnexpectedEof") || err_msg.contains("unexpected EOF") {
                        anyhow::anyhow!("LLM provider connection closed unexpectedly. This might be a temporary issue with the provider or proxy. (Original error: {})", err_msg)
                    } else {
                        anyhow::anyhow!("Failed to decode LLM response. The provider may have returned an invalid format. (Original error: {})", err_msg)
                    }
                } else {
                    e
                };

                // 检查是否是可重试的错误（主要是解析错误和网络抖动）
                let is_retryable = is_retryable_error(&err_msg);
                let is_empty_response = is_empty_response_error(&err_msg);
                let has_tool_activity = tool_calls_collector
                    .lock()
                    .map(|calls| !calls.is_empty())
                    .unwrap_or(false)
                    || pending_calls
                        .lock()
                        .map(|pending| !pending.is_empty())
                        .unwrap_or(false);

                if is_retryable && retries < max_retries {
                    retries += 1;
                    if is_empty_response {
                        silent_retry_pending = true;
                    }
                    if has_tool_activity {
                        tracing::warn!(
                            "Retrying despite tool activity due to retryable error - execution_id: {}",
                            params.execution_id
                        );
                    }
                    last_error = Some(anyhow::anyhow!("{}", friendly_err));

                    // 保存当前工作到累积记录中（在清理之前）
                    accumulate_retry_progress(
                        &tool_calls_collector,
                        &accumulated_tool_calls,
                        &assistant_segment_buf,
                        &accumulated_assistant_output,
                    );
                    clear_retry_turn_state(
                        &assistant_segment_buf,
                        &reasoning_content_buf,
                        &pending_calls,
                        &tool_calls_collector,
                        None,
                        None,
                    );
                    low_evidence_warning_issued.store(false, Ordering::SeqCst);
                    if let Ok(mut tracker) = hypothesis_tracker.lock() {
                        tracker.clear();
                    }

                    continue;
                } else if is_empty_response && empty_response_retries < max_empty_response_retries {
                    empty_response_retries += 1;
                    silent_retry_pending = true;
                    last_error = Some(anyhow::anyhow!("{}", friendly_err));

                    tracing::warn!(
                        "Retrying empty response with same model (attempt {}/{}) - execution_id: {}",
                        empty_response_retries,
                        max_empty_response_retries,
                        params.execution_id
                    );

                    accumulate_retry_progress(
                        &tool_calls_collector,
                        &accumulated_tool_calls,
                        &assistant_segment_buf,
                        &accumulated_assistant_output,
                    );
                    clear_retry_turn_state(
                        &assistant_segment_buf,
                        &reasoning_content_buf,
                        &pending_calls,
                        &tool_calls_collector,
                        Some(&last_tool_fingerprint),
                        Some(repeated_tool_fingerprint_count.as_ref()),
                    );
                    low_evidence_warning_issued.store(false, Ordering::SeqCst);
                    if let Ok(mut tracker) = hypothesis_tracker.lock() {
                        tracker.clear();
                    }
                    force_history_with_tools = true;
                    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                    continue;
                } else {
                    // Final failure recording
                    let tool_summaries = tool_calls_collector
                        .lock()
                        .map(|calls| {
                            calls
                                .iter()
                                .map(|call| ToolCallSummary {
                                    name: call.name.clone(),
                                    success: call.success,
                                    duration_ms: Some(call.duration_ms),
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    let fail_tool_names: Vec<String> = tool_summaries
                        .iter()
                        .map(|t| t.name.clone())
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .collect();
                    let err_msg_clone = err_msg.clone();

                    if let Err(err) = get_global_memory()
                        .record_execution(ExecutionRecord {
                            id: params.execution_id.clone(),
                            task: params.task.clone(),
                            environment: Some(rig_provider.clone()),
                            tool_calls: tool_summaries,
                            success: false,
                            error: Some(err_msg),
                            response_excerpt: None,
                            created_at: chrono::Utc::now().timestamp(),
                        })
                        .await
                    {
                        tracing::warn!("Failed to store memory record: {}", err);
                    }

                    record_execution_reflection(
                        app_handle,
                        &ExecutionOutcome {
                            execution_id: params.execution_id.clone(),
                            task: params.task.clone(),
                            success: false,
                            error: Some(err_msg_clone.clone()),
                            tool_names_used: fail_tool_names,
                            response_excerpt: None,
                        },
                    )
                    .await;
                    if let Err(update_err) = apply_sentinel_execution_outcome(
                        app_handle,
                        &params.execution_id,
                        false,
                        None,
                        Some(&err_msg_clone),
                    )
                    .await
                    {
                        tracing::warn!(
                            "Failed to update sentinel execution outcome after error: {}",
                            update_err
                        );
                    }

                    if let Some(ctx) = team_stream_context.as_ref() {
                        let _ = app.emit(
                            "agent_team:message_stream_done",
                            &json!({
                                "session_id": ctx.session_id.clone(),
                                "stream_id": ctx.stream_id.clone(),
                                "member_id": ctx.member_id.clone(),
                                "member_name": ctx.member_id.clone(),
                                "phase": ctx.phase.clone(),
                                "error": friendly_err.to_string(),
                                "had_delta": team_stream_had_delta.load(Ordering::SeqCst),
                            }),
                        );
                    }

                    return Err(friendly_err);
                }
            }
        }
    }

    let final_error = last_error.unwrap_or_else(|| anyhow::anyhow!("Max retries reached"));
    if let Some(ctx) = team_stream_context.as_ref() {
        let _ = app.emit(
            "agent_team:message_stream_done",
            &json!({
                "session_id": ctx.session_id.clone(),
                "stream_id": ctx.stream_id.clone(),
                "member_id": ctx.member_id.clone(),
                "member_name": ctx.member_id.clone(),
                "phase": ctx.phase.clone(),
                "error": final_error.to_string(),
                "had_delta": team_stream_had_delta.load(Ordering::SeqCst),
            }),
        );
    }
    Err(final_error)
}

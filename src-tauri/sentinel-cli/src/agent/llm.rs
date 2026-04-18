use super::codex_context::build_codex_like_history;
use super::compaction::compact_history_if_needed;
use super::config::ContestLlmConfig;
use super::prompt::{challenge_turn_prompt, contest_system_prompt};
use super::rate_limit::SharedLlmThrottle;
use super::signal::{extract_candidate_flags, parse_agent_signal, AgentSignalStatus};
use super::tenth_man::{
    clear_review_context, init_tenth_man_executor, run_runner_review, set_review_context,
    ContestReviewContext,
};
use super::tooling::{build_contest_dynamic_tools, ContestToolWindowStats, ContestTraceRecorder};
use super::watchdog::{evaluate_watchdog, WatchdogAction, WatchdogInput};
use crate::arena::{ArenaClient, ChallengeInfo, HintResponse, SubmitFlagResponse};
use crate::runtime::{ChallengeRunState, RuntimeStateStore};
use crate::state::sentinel_state_dir;
use anyhow::{Context, Result};
use reqwest::Client;
use sentinel_llm::{ChatMessage, LlmConfig, StreamingLlmClient};
use sentinel_tools::buildin_tools::browser::close_browser_session;
use sentinel_tools::buildin_tools::shell::{
    set_shell_config, ShellConfig, ShellDefaultPolicy, ShellExecutionMode,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::time::Duration;

const STARTUP_WAIT_ATTEMPTS: usize = 12;
const STARTUP_WAIT_DELAY_MS: u64 = 1_000;

#[derive(Debug, Clone, Serialize)]
pub struct AgentSolveReport {
    pub code: String,
    pub title: String,
    pub entrypoints: Vec<String>,
    pub steps_taken: usize,
    pub hint_used: bool,
    pub discovered_flags: Vec<String>,
    pub submissions: Vec<FlagSubmissionAttempt>,
    pub final_status: String,
    pub final_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlagSubmissionAttempt {
    pub flag: String,
    pub correct: bool,
    pub message: String,
    pub flag_got_count: i32,
    pub flag_count: i32,
}

pub async fn solve_challenge_with_llm(
    arena: &ArenaClient,
    challenge: ChallengeInfo,
    llm: ContestLlmConfig,
    max_steps: usize,
    max_challenge_duration_secs: u64,
    allow_hint: bool,
    stop_when_done: bool,
    llm_throttle: Option<SharedLlmThrottle>,
    failure_cooldown_secs: u64,
) -> Result<AgentSolveReport> {
    configure_shell_for_unattended_mode().await;
    init_tenth_man_executor();

    let store = RuntimeStateStore::new(sentinel_state_dir());
    let code = challenge.code.clone();
    let mut run_state = store.load_challenge(&code).await?;
    let resumed = is_resumable_attempt(&run_state);
    let attempt_id = run_state
        .current_attempt_id
        .clone()
        .filter(|_| resumed)
        .unwrap_or_else(|| build_attempt_id(&code));
    let browser_session_id = format!("browser-{}-{}", code, attempt_id);
    let attempt_deadline_at =
        ensure_attempt_deadline(&store, &mut run_state, resumed, max_challenge_duration_secs)
            .await?;
    let timeout_budget = remaining_timeout_budget(&attempt_deadline_at);
    let timeout_challenge = challenge.clone();

    let solve_result = tokio::time::timeout(timeout_budget, async {
        if resumed {
            let _ = store
                .append_attempt_event(
                    &code,
                    &attempt_id,
                    "attempt_resumed",
                    &serde_json::json!({
                        "step": run_state.current_step,
                        "hint_used": run_state.hint_used,
                        "tool_calls_count": run_state.tool_calls_count,
                    }),
                )
                .await;
        } else {
            prepare_challenge_attempt(
                &store,
                &challenge,
                &attempt_id,
                &mut run_state,
                &attempt_deadline_at,
            )
            .await?;
        }

        let entrypoints = if resumed && !run_state.entrypoints.is_empty() {
            run_state.entrypoints.clone()
        } else {
            let started =
                start_challenge_with_capacity_recovery(arena, &store, &challenge, &attempt_id)
                    .await?;
            activate_challenge_attempt(
                &store,
                &challenge,
                &attempt_id,
                &started.entrypoints,
                &mut run_state,
                &attempt_deadline_at,
            )
            .await?;
            started.entrypoints
        };
        let hint_allowed = allow_hint && !challenge.hint_viewed;
        wait_for_entrypoints(&entrypoints).await;

        let trace_recorder = ContestTraceRecorder::new(store.clone(), &code, &attempt_id);
        let dynamic_tools = build_contest_dynamic_tools(trace_recorder.clone()).await?;

        let client = build_llm_client(&llm);
        let system_prompt = contest_system_prompt();
        let mut history = run_state.recent_history.clone();
        let mut hint_response =
            run_state
                .last_hint_content
                .clone()
                .map(|hint_content| HintResponse {
                    code: code.clone(),
                    hint_content: Some(hint_content),
                });
        let mut hint_used = run_state.hint_used;
        let mut discovered_flags = BTreeSet::new();
        let mut submissions = Vec::new();
        let mut final_status = "give_up".to_string();
        let mut final_reason = Some("step budget exhausted".to_string());
        let mut last_feedback = run_state.last_reason.clone();
        let mut context_summary = run_state.context_summary.clone();
        let start_step = run_state.current_step.unwrap_or(1).max(1);

        for step in start_step..=max_steps {
            let previous_tool_calls = run_state.tool_calls_count;
            let previous_flag_count = run_state.discovered_flags.len();
            run_state.current_step = Some(step);
            run_state.last_reason = last_feedback.clone();
            store.save_challenge(&run_state).await?;
            let _ = store
                .append_attempt_event(
                    &code,
                    &attempt_id,
                    "step_started",
                    &serde_json::json!({
                        "step": step,
                        "hint_used": hint_used,
                        "last_feedback": last_feedback,
                    }),
                )
                .await;

            let user_prompt = challenge_turn_prompt(
                &llm.execution_id,
                &challenge,
                &entrypoints,
                hint_response.as_ref(),
                hint_allowed && !hint_used,
                step,
                max_steps,
            );
            let codex_like_history = build_codex_like_history(
                &history,
                &run_state,
                context_summary.as_deref(),
                last_feedback.as_deref(),
                &trace_recorder.recent_tool_digests(4).await,
            );
            set_review_context(
                llm.execution_id.clone(),
                ContestReviewContext {
                    llm: llm.clone(),
                    challenge: challenge.clone(),
                    entrypoints: entrypoints.clone(),
                    hint: hint_response
                        .as_ref()
                        .and_then(|value| value.hint_content.clone()),
                    context_summary: context_summary.clone(),
                    last_feedback: last_feedback.clone(),
                    current_prompt: user_prompt.clone(),
                    history: history.clone(),
                    step_index: step,
                    max_steps,
                },
            )
            .await;

            if let Some(llm_throttle) = &llm_throttle {
                let waited_secs = llm_throttle.wait_for_clearance().await;
                if waited_secs > 0 {
                    let _ = store
                        .append_attempt_event(
                            &code,
                            &attempt_id,
                            "llm_backoff_waited",
                            &serde_json::json!({
                                "step": step,
                                "waited_secs": waited_secs,
                            }),
                        )
                        .await;
                }
            }

            let response = match client
                .stream_chat_with_dynamic_tools(
                    Some(&system_prompt),
                    &user_prompt,
                    &codex_like_history,
                    None,
                    dynamic_tools.clone(),
                    |_| true,
                )
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    if let Some(llm_throttle) = &llm_throttle {
                        if is_llm_rate_limit_error(&error.to_string()) {
                            let snapshot =
                                llm_throttle.register_rate_limit(error.to_string()).await;
                            let _ = store
                                .append_attempt_event(
                                    &code,
                                    &attempt_id,
                                    "llm_rate_limited",
                                    &serde_json::json!({
                                        "step": step,
                                        "error": error.to_string(),
                                        "throttle": snapshot,
                                    }),
                                )
                                .await;
                        }
                    }
                    let error_message =
                        format!("LLM execution failed for challenge {}: {}", code, error);
                    run_state.tool_calls_count = read_tool_count(&store, &code, &attempt_id).await;
                    run_state.last_reason = Some(error_message.clone());
                    persist_runtime_snapshot(
                        &store,
                        &mut run_state,
                        &history,
                        context_summary.clone(),
                        hint_response.as_ref(),
                        step,
                        Some(error_message.clone()),
                    )
                    .await?;
                    let _ = store
                        .append_attempt_event(
                            &code,
                            &attempt_id,
                            "llm_execution_error",
                            &serde_json::json!({
                                "step": step,
                                "error": error.to_string(),
                                "tool_calls_count": run_state.tool_calls_count,
                            }),
                        )
                        .await;
                    return Err(anyhow::anyhow!(error_message));
                }
            };
            if let Some(llm_throttle) = &llm_throttle {
                let _ = llm_throttle.register_success().await;
            }
            history.push(ChatMessage::user(user_prompt));
            history.push(ChatMessage::assistant(response.clone()));
            run_state.tool_calls_count = read_tool_count(&store, &code, &attempt_id).await;
            if let Some(report) = compact_history_if_needed(&mut history, &mut context_summary) {
                run_state.context_summary = context_summary.clone();
                let _ = store
                    .append_attempt_event(&code, &attempt_id, "history_compacted", &report)
                    .await;
            }
            store.save_challenge(&run_state).await?;

            let fallback_flags = extract_candidate_flags(&response);
            discovered_flags.extend(fallback_flags.clone());
            run_state.discovered_flags.extend(fallback_flags);
            persist_runtime_snapshot(
                &store,
                &mut run_state,
                &history,
                context_summary.clone(),
                hint_response.as_ref(),
                step,
                last_feedback.clone(),
            )
            .await?;

            let signal = match parse_agent_signal(&response) {
                Ok(signal) => signal,
                Err(error) => {
                    let reason = format!(
                    "agent response was not valid JSON signal: {}. Retry with strict JSON only.",
                    error
                );
                    let _ = store
                        .append_attempt_event(
                            &code,
                            &attempt_id,
                            "signal_parse_error",
                            &serde_json::json!({
                                "step": step,
                                "error": error.to_string(),
                                "response": response,
                            }),
                        )
                        .await;
                    history.push(ChatMessage::user(reason.clone()));
                    last_feedback = Some(reason);
                    persist_runtime_snapshot(
                        &store,
                        &mut run_state,
                        &history,
                        context_summary.clone(),
                        hint_response.as_ref(),
                        step + 1,
                        last_feedback.clone(),
                    )
                    .await?;
                    continue;
                }
            };

            let _ = store
                .append_attempt_event(
                    &code,
                    &attempt_id,
                    "llm_signal",
                    &serde_json::json!({
                        "step": step,
                        "signal": signal,
                    }),
                )
                .await;

            match signal.status {
                AgentSignalStatus::Continue => {
                    run_state.last_reason = signal.reason.clone();
                    last_feedback = signal.reason.clone();
                }
                AgentSignalStatus::NeedHint => {
                    if hint_allowed && !hint_used {
                        let hint = arena.view_hint(&code).await?;
                        hint_used = true;
                        run_state.hint_used = true;
                        hint_response = Some(hint.clone());
                        store.save_challenge(&run_state).await?;
                        let _ = store
                            .append_attempt_event(
                                &code,
                                &attempt_id,
                                "hint_obtained",
                                &serde_json::json!({
                                    "step": step,
                                    "hint": hint.hint_content,
                                }),
                            )
                            .await;
                        let feedback = format!(
                            "Hint fetched successfully. Continue solving with this hint: {}",
                            hint.hint_content.unwrap_or_default()
                        );
                        history.push(ChatMessage::user(feedback.clone()));
                        last_feedback = Some(feedback);
                    } else {
                        let feedback =
                        "Hint is unavailable or already used. Continue without requesting hint."
                            .to_string();
                        history.push(ChatMessage::user(feedback.clone()));
                        last_feedback = Some(feedback);
                    }
                }
                AgentSignalStatus::CandidateFlag => {
                    let flag = signal.flag.unwrap_or_default();
                    discovered_flags.insert(flag.clone());
                    run_state.discovered_flags.insert(flag.clone());
                    if run_state.submitted_flags.contains(&flag) {
                        let feedback = format!(
                            "Flag {} was already submitted earlier. Continue searching.",
                            flag
                        );
                        history.push(ChatMessage::user(feedback.clone()));
                        last_feedback = Some(feedback);
                        continue;
                    }

                    let result = arena.submit_flag(&code, &flag).await?;
                    run_state.submitted_flags.insert(flag.clone());
                    if result.correct {
                        run_state.accepted_flags.insert(flag.clone());
                    }
                    submissions.push(map_submission(flag.clone(), &result));
                    let _ = store
                        .append_attempt_event(
                            &code,
                            &attempt_id,
                            "flag_submitted",
                            &serde_json::json!({
                                "flag": flag,
                                "correct": result.correct,
                                "message": result.message,
                            }),
                        )
                        .await;
                    store.save_challenge(&run_state).await?;

                    if result.correct {
                        final_status = "solved".to_string();
                        final_reason = signal.reason.clone().or(Some(result.message));
                        run_state.last_reason = final_reason.clone();
                        break;
                    }

                    let feedback = format!(
                        "Submitted candidate flag but it was incorrect: {}. Continue searching.",
                        result.message
                    );
                    history.push(ChatMessage::user(feedback.clone()));
                    last_feedback = Some(feedback);
                }
                AgentSignalStatus::Done => {
                    final_status = "done".to_string();
                    final_reason = signal.reason.clone();
                    run_state.last_reason = final_reason.clone();
                    break;
                }
                AgentSignalStatus::GiveUp => {
                    final_status = "give_up".to_string();
                    final_reason = signal.reason.clone();
                    run_state.last_reason = final_reason.clone();
                    break;
                }
            }

            let latest_summary = trace_recorder.latest_call_summary().await;
            let latest_tool_signature = latest_summary
                .as_ref()
                .map(|summary| summary.tool_signature.clone());
            let latest_result_signature = latest_summary
                .as_ref()
                .and_then(|summary| summary.result_signature.clone());
            let current_tool_calls = run_state.tool_calls_count;
            let current_flag_count = run_state.discovered_flags.len();
            let (watchdog_snapshot, watchdog_action) = evaluate_watchdog(
                &mut run_state,
                WatchdogInput {
                    previous_tool_calls,
                    current_tool_calls,
                    previous_flag_count,
                    current_flag_count,
                    latest_tool_signature: latest_tool_signature.as_deref(),
                    latest_result_signature: latest_result_signature.as_deref(),
                    hint_allowed,
                    hint_used,
                },
            );
            let _ = store
                .append_attempt_event(
                    &code,
                    &attempt_id,
                    "watchdog_evaluated",
                    &serde_json::json!({
                        "step": step,
                        "snapshot": watchdog_snapshot,
                        "action": watchdog_action,
                    }),
                )
                .await;

            match &watchdog_action {
                WatchdogAction::Continue => {}
                WatchdogAction::InjectFeedback { reason } => {
                    history.push(ChatMessage::user(reason.clone()));
                    last_feedback = Some(reason.clone());
                }
                WatchdogAction::RequestHint { reason } => {
                    if hint_allowed && !hint_used {
                        let hint = arena.view_hint(&code).await?;
                        hint_used = true;
                        run_state.hint_used = true;
                        hint_response = Some(hint.clone());
                        let hint_text = hint.hint_content.clone().unwrap_or_default();
                        let feedback =
                            format!("{} Hint fetched by watchdog: {}", reason, hint_text);
                        let _ = store
                            .append_attempt_event(
                                &code,
                                &attempt_id,
                                "watchdog_hint_obtained",
                                &serde_json::json!({
                                    "step": step,
                                    "reason": reason,
                                    "hint": hint.hint_content,
                                }),
                            )
                            .await;
                        history.push(ChatMessage::user(feedback.clone()));
                        last_feedback = Some(feedback);
                    }
                }
                WatchdogAction::Abort { reason } => {
                    final_status = "give_up".to_string();
                    final_reason = Some(reason.clone());
                    run_state.last_reason = Some(reason.clone());
                    break;
                }
            }

            let tool_window = trace_recorder.recent_tool_window_stats(24).await;
            let runner_review_injected = if let Some(critique) = force_runner_review(
                &store,
                &code,
                &attempt_id,
                &llm.execution_id,
                step,
                &tool_window,
                &watchdog_action,
                last_feedback.as_deref(),
            )
            .await {
                let feedback = format!(
                    "Runner tenth man review: {}\nFollow this critique before retrying the same path.",
                    critique
                );
                history.push(ChatMessage::user(feedback.clone()));
                last_feedback = Some(feedback);
                true
            } else {
                false
            };
            if !runner_review_injected {
                if let Some(reason) = shell_heavy_review_feedback(&tool_window, hint_used) {
                let _ = store
                    .append_attempt_event(
                        &code,
                        &attempt_id,
                        "runner_forced_review_feedback",
                        &serde_json::json!({
                            "step": step,
                            "tool_window": tool_window,
                            "reason": reason,
                        }),
                    )
                    .await;
                history.push(ChatMessage::user(reason.clone()));
                last_feedback = Some(reason);
            }
            }

            persist_runtime_snapshot(
                &store,
                &mut run_state,
                &history,
                context_summary.clone(),
                hint_response.as_ref(),
                step + 1,
                last_feedback.clone(),
            )
            .await?;
        }

        if stop_when_done {
            let _ = arena.stop_challenge(&code).await;
        }

        let _ = store
            .append_attempt_event(
                &code,
                &attempt_id,
                "attempt_finished",
                &serde_json::json!({
                    "status": final_status,
                    "reason": final_reason,
                    "hint_used": hint_used,
                    "tool_calls_count": run_state.tool_calls_count,
                    "discovered_flags": discovered_flags.iter().cloned().collect::<Vec<_>>(),
                }),
            )
            .await;
        finish_challenge_run_state(&store, &mut run_state, &final_status, failure_cooldown_secs)
            .await?;

        Ok(AgentSolveReport {
            code: code.clone(),
            title: challenge.title,
            entrypoints,
            steps_taken: history.iter().filter(|msg| msg.role == "assistant").count(),
            hint_used,
            discovered_flags: discovered_flags.into_iter().collect(),
            submissions,
            final_status,
            final_reason,
        })
    })
    .await;

    let solve_result: Result<AgentSolveReport> = match solve_result {
        Ok(result) => result,
        Err(_) => {
            record_timeout_result(
                arena,
                &store,
                &timeout_challenge,
                &attempt_id,
                &mut run_state,
                failure_cooldown_secs,
                max_challenge_duration_secs,
            )
            .await
        }
    };

    if let Err(error) = &solve_result {
        let _ = store
            .append_attempt_event(
                &code,
                &attempt_id,
                "attempt_failed",
                &serde_json::json!({
                    "error": error.to_string(),
                }),
            )
            .await;
        let _ = store
            .mark_challenge_terminal(&code, "error", Some(error.to_string()))
            .await;
        if let Ok(mut run_state) = store.load_challenge(&code).await {
            apply_retry_cooldown(&mut run_state, failure_cooldown_secs, false);
            let _ = store.save_challenge(&run_state).await;
        }
    }

    let _ = close_browser_session(&browser_session_id).await;
    clear_review_context(&llm.execution_id).await;
    solve_result
}

fn build_llm_client(config: &ContestLlmConfig) -> StreamingLlmClient {
    let mut llm_config = LlmConfig::new(&config.provider, &config.model)
        .with_timeout(config.timeout_secs)
        .with_rig_provider(&config.rig_provider)
        .with_conversation_id(&config.execution_id)
        .with_max_turns(config.max_turns);

    if let Some(api_key) = &config.api_key {
        llm_config = llm_config.with_api_key(api_key.clone());
    }
    if let Some(base_url) = &config.base_url {
        llm_config = llm_config.with_base_url(base_url.clone());
    }

    StreamingLlmClient::new(llm_config)
}

async fn configure_shell_for_unattended_mode() {
    let mut config = ShellConfig {
        default_policy: ShellDefaultPolicy::AlwaysProceed,
        default_execution_mode: ShellExecutionMode::Host,
        default_timeout_secs: 30,
        max_timeout_secs: Some(45),
        ..ShellConfig::default()
    };
    config.denied_commands = vec![
        "rm".to_string(),
        "rm -rf".to_string(),
        "mkfs".to_string(),
        "dd".to_string(),
        "find /".to_string(),
        "find /proc".to_string(),
        "find /sys".to_string(),
        "find /dev".to_string(),
        "grep -r /".to_string(),
        "grep -R /".to_string(),
        "du /".to_string(),
        "ls -R /".to_string(),
    ];
    set_shell_config(config).await;
}

async fn ensure_attempt_deadline(
    store: &RuntimeStateStore,
    run_state: &mut ChallengeRunState,
    resumed: bool,
    max_challenge_duration_secs: u64,
) -> Result<String> {
    let deadline = if resumed {
        run_state
            .attempt_deadline_at
            .clone()
            .or_else(|| deadline_from_started_at(run_state, max_challenge_duration_secs))
            .unwrap_or_else(|| build_attempt_deadline(max_challenge_duration_secs))
    } else {
        build_attempt_deadline(max_challenge_duration_secs)
    };
    run_state.attempt_deadline_at = Some(deadline.clone());
    store.save_challenge(run_state).await?;
    Ok(deadline)
}

fn deadline_from_started_at(
    run_state: &ChallengeRunState,
    max_challenge_duration_secs: u64,
) -> Option<String> {
    let started_at = run_state.last_started_at.as_deref()?;
    let started_at = chrono::DateTime::parse_from_rfc3339(started_at).ok()?;
    Some(
        (started_at.with_timezone(&chrono::Utc)
            + chrono::Duration::seconds(max_challenge_duration_secs as i64))
        .to_rfc3339(),
    )
}

fn build_attempt_deadline(max_challenge_duration_secs: u64) -> String {
    (chrono::Utc::now() + chrono::Duration::seconds(max_challenge_duration_secs as i64))
        .to_rfc3339()
}

fn remaining_timeout_budget(attempt_deadline_at: &str) -> Duration {
    let now = chrono::Utc::now();
    let remaining = chrono::DateTime::parse_from_rfc3339(attempt_deadline_at)
        .map(|deadline| (deadline.with_timezone(&chrono::Utc) - now).num_milliseconds())
        .unwrap_or(0);
    if remaining <= 0 {
        Duration::from_millis(0)
    } else {
        Duration::from_millis(remaining as u64)
    }
}

fn shell_heavy_review_feedback(
    tool_window: &ContestToolWindowStats,
    hint_used: bool,
) -> Option<String> {
    if tool_window.total < 8 {
        return None;
    }

    let shell_heavy = tool_window.trailing_shell >= 6
        || (tool_window.shell >= 12
            && tool_window.browser
                + tool_window.http_request
                + tool_window.route_discovery
                + tool_window.search_exploit
                <= 4);

    if !shell_heavy {
        return None;
    }

    if tool_window.tenth_man_review > 0 {
        return None;
    }

    Some(format!(
        "Runner intervention: the last {} tool calls are shell-heavy (shell={}, trailing_shell={}) with no tenth_man_review. Before any more shell probing, call tenth_man_review to challenge the current path. After that, switch to a higher-leverage action: use browser/http_request for precise verification, use route_discovery for authenticated route enumeration, use search_exploit for product/CVE hypotheses, or install and run a specialized scanner through shell. Do not continue handcrafting repetitive curl/find/grep loops. Hint already used: {}.",
        tool_window.total,
        tool_window.shell,
        tool_window.trailing_shell,
        hint_used
    ))
}

async fn force_runner_review(
    store: &RuntimeStateStore,
    code: &str,
    attempt_id: &str,
    execution_id: &str,
    step: usize,
    tool_window: &ContestToolWindowStats,
    watchdog_action: &WatchdogAction,
    last_feedback: Option<&str>,
) -> Option<String> {
    if tool_window.tenth_man_review > 0 {
        return None;
    }

    if last_feedback
        .map(|feedback| feedback.contains("Runner tenth man review:"))
        .unwrap_or(false)
    {
        return None;
    }

    let focus_area = match watchdog_action {
        WatchdogAction::InjectFeedback { reason } => Some(reason.clone()),
        WatchdogAction::RequestHint { reason } => Some(reason.clone()),
        WatchdogAction::Abort { reason } => Some(reason.clone()),
        WatchdogAction::Continue => shell_heavy_review_feedback(tool_window, false),
    }?;

    let quick = tool_window.shell < 20;
    match run_runner_review(execution_id, Some(focus_area.clone()), quick).await {
        Ok(critique) => {
            let _ = store
                .append_attempt_event(
                    code,
                    attempt_id,
                    "runner_tenth_man_review",
                    &serde_json::json!({
                        "step": step,
                        "tool_window": tool_window,
                        "focus_area": focus_area,
                        "quick": quick,
                        "critique": critique,
                    }),
                )
                .await;
            Some(critique)
        }
        Err(error) => {
            let _ = store
                .append_attempt_event(
                    code,
                    attempt_id,
                    "runner_tenth_man_review_failed",
                    &serde_json::json!({
                        "step": step,
                        "tool_window": tool_window,
                        "focus_area": focus_area,
                        "quick": quick,
                        "error": error.to_string(),
                    }),
                )
                .await;
            None
        }
    }
}

async fn record_timeout_result(
    arena: &ArenaClient,
    store: &RuntimeStateStore,
    challenge: &ChallengeInfo,
    attempt_id: &str,
    run_state: &mut ChallengeRunState,
    failure_cooldown_secs: u64,
    max_challenge_duration_secs: u64,
) -> Result<AgentSolveReport> {
    let final_status = "give_up".to_string();
    let final_reason = Some(format!(
        "challenge wall clock timeout reached after {} seconds",
        max_challenge_duration_secs
    ));
    let steps_taken = run_state.current_step.unwrap_or(0);
    let hint_used = run_state.hint_used;
    let entrypoints = run_state.entrypoints.clone();
    let discovered_flags: Vec<String> = run_state.discovered_flags.iter().cloned().collect();
    run_state.tool_calls_count = read_tool_count(store, &challenge.code, attempt_id).await;
    run_state.last_reason = final_reason.clone();
    let _ = store
        .append_attempt_event(
            &challenge.code,
            attempt_id,
            "attempt_timed_out",
            &serde_json::json!({
                "max_challenge_duration_secs": max_challenge_duration_secs,
                "tool_calls_count": run_state.tool_calls_count,
                "current_step": run_state.current_step,
            }),
        )
        .await;
    let stop_result = arena.stop_challenge(&challenge.code).await;
    let _ = store
        .append_attempt_event(
            &challenge.code,
            attempt_id,
            "attempt_timeout_cleanup",
            &serde_json::json!({
                "stop_ok": stop_result.is_ok(),
                "stop_error": stop_result.err().map(|error| error.to_string()),
            }),
        )
        .await;
    let _ = store
        .append_attempt_event(
            &challenge.code,
            attempt_id,
            "attempt_finished",
            &serde_json::json!({
                "status": final_status,
                "reason": final_reason,
                "hint_used": run_state.hint_used,
                "tool_calls_count": run_state.tool_calls_count,
                "discovered_flags": run_state.discovered_flags.iter().cloned().collect::<Vec<_>>(),
            }),
        )
        .await;
    finish_challenge_run_state(store, run_state, &final_status, failure_cooldown_secs).await?;

    Ok(AgentSolveReport {
        code: challenge.code.clone(),
        title: challenge.title.clone(),
        entrypoints,
        steps_taken,
        hint_used,
        discovered_flags,
        submissions: Vec::new(),
        final_status,
        final_reason,
    })
}

async fn prepare_challenge_attempt(
    store: &RuntimeStateStore,
    challenge: &ChallengeInfo,
    attempt_id: &str,
    run_state: &mut ChallengeRunState,
    attempt_deadline_at: &str,
) -> Result<()> {
    run_state.title = Some(challenge.title.clone());
    run_state.last_started_at = Some(chrono::Utc::now().to_rfc3339());
    run_state.attempt_deadline_at = Some(attempt_deadline_at.to_string());
    run_state.last_status = Some("starting".to_string());
    run_state.current_attempt_id = Some(attempt_id.to_string());
    run_state.current_step = None;
    run_state.hint_used = false;
    run_state.last_reason = None;
    run_state.last_hint_content = None;
    run_state.context_summary = None;
    run_state.recent_history = Vec::new();
    run_state.entrypoints = Vec::new();
    run_state.tool_calls_count = 0;
    run_state.watchdog_stall_count = 0;
    run_state.watchdog_repeat_count = 0;
    run_state.watchdog_last_tool_signature = None;
    run_state.watchdog_last_result_signature = None;
    run_state.watchdog_interventions = 0;
    run_state.total_attempts = run_state.total_attempts.saturating_add(1);
    run_state.next_eligible_at = None;
    run_state.last_progress_at = Some(chrono::Utc::now().to_rfc3339());
    store.save_challenge(run_state).await?;
    let _ = store
        .append_attempt_event(
            &challenge.code,
            attempt_id,
            "attempt_preparing",
            &serde_json::json!({
                "title": challenge.title,
                "difficulty": challenge.difficulty,
                "level": challenge.level,
            }),
        )
        .await;
    Ok(())
}

async fn activate_challenge_attempt(
    store: &RuntimeStateStore,
    challenge: &ChallengeInfo,
    attempt_id: &str,
    entrypoints: &[String],
    run_state: &mut ChallengeRunState,
    attempt_deadline_at: &str,
) -> Result<()> {
    run_state.last_status = Some("running".to_string());
    run_state.current_step = Some(0);
    run_state.entrypoints = entrypoints.to_vec();
    run_state.attempt_deadline_at = Some(attempt_deadline_at.to_string());
    run_state.last_progress_at = Some(chrono::Utc::now().to_rfc3339());
    store.save_challenge(run_state).await?;
    let _ = store
        .append_event(
            "agent_challenge_started",
            &serde_json::json!({
                "code": challenge.code,
                "title": challenge.title,
                "difficulty": challenge.difficulty,
                "level": challenge.level,
                "attempt_id": attempt_id,
                "entrypoints": entrypoints,
            }),
        )
        .await;
    let _ = store
        .append_attempt_event(
            &challenge.code,
            attempt_id,
            "attempt_started",
            &serde_json::json!({
                "entrypoints": entrypoints,
            }),
        )
        .await;
    Ok(())
}

async fn finish_challenge_run_state(
    store: &RuntimeStateStore,
    run_state: &mut ChallengeRunState,
    final_status: &str,
    failure_cooldown_secs: u64,
) -> Result<()> {
    let updated = store
        .mark_challenge_terminal(&run_state.code, final_status, run_state.last_reason.clone())
        .await?;
    *run_state = updated;
    if matches!(final_status, "solved" | "done") {
        run_state.consecutive_failures = 0;
        run_state.next_eligible_at = None;
    } else if final_status == "give_up" {
        apply_retry_cooldown(run_state, failure_cooldown_secs, true);
    } else if final_status == "error" {
        apply_retry_cooldown(run_state, failure_cooldown_secs, false);
    }
    store.save_challenge(run_state).await?;
    Ok(())
}

fn apply_retry_cooldown(
    run_state: &mut ChallengeRunState,
    failure_cooldown_secs: u64,
    count_as_failure: bool,
) {
    if count_as_failure {
        run_state.consecutive_failures = run_state.consecutive_failures.saturating_add(1);
    }
    run_state.next_eligible_at = Some(
        (chrono::Utc::now() + chrono::Duration::seconds(failure_cooldown_secs as i64)).to_rfc3339(),
    );
}

fn map_submission(flag: String, result: &SubmitFlagResponse) -> FlagSubmissionAttempt {
    FlagSubmissionAttempt {
        flag,
        correct: result.correct,
        message: result.message.clone(),
        flag_count: result.flag_count,
        flag_got_count: result.flag_got_count,
    }
}

async fn wait_for_entrypoints(entrypoints: &[String]) {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .build()
        .ok();

    let Some(client) = client else {
        return;
    };

    for entrypoint in entrypoints {
        let url = normalize_entrypoint_to_url(entrypoint);
        for _ in 0..STARTUP_WAIT_ATTEMPTS {
            match client.get(url.clone()).send().await {
                Ok(_) => break,
                Err(_) => tokio::time::sleep(Duration::from_millis(STARTUP_WAIT_DELAY_MS)).await,
            }
        }
    }
}

fn normalize_entrypoint_to_url(entrypoint: &str) -> String {
    if entrypoint.starts_with("http://") || entrypoint.starts_with("https://") {
        entrypoint.to_string()
    } else {
        format!("http://{}", entrypoint)
    }
}

fn is_llm_rate_limit_error(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("429")
        || normalized.contains("rate limit")
        || normalized.contains("too many requests")
        || normalized.contains("请求频率")
        || normalized.contains("请求过于频繁")
}

fn build_attempt_id(code: &str) -> String {
    format!("{}-{}", code, chrono::Utc::now().format("%Y%m%d%H%M%S"))
}

async fn read_tool_count(_store: &RuntimeStateStore, code: &str, attempt_id: &str) -> usize {
    let path = sentinel_state_dir()
        .join("logs")
        .join("tool-calls")
        .join(code)
        .join(format!("{}.jsonl", attempt_id));
    match tokio::fs::read_to_string(path).await {
        Ok(content) => content.lines().count(),
        Err(_) => 0,
    }
}

fn is_resumable_attempt(run_state: &ChallengeRunState) -> bool {
    run_state.last_status.as_deref() == Some("running")
        && run_state.current_attempt_id.is_some()
        && run_state.current_step.is_some()
}

async fn persist_runtime_snapshot(
    store: &RuntimeStateStore,
    run_state: &mut ChallengeRunState,
    history: &[ChatMessage],
    context_summary: Option<String>,
    hint_response: Option<&HintResponse>,
    next_step: usize,
    last_feedback: Option<String>,
) -> Result<()> {
    run_state.current_step = Some(next_step);
    run_state.last_reason = last_feedback;
    run_state.context_summary = context_summary;
    run_state.last_hint_content = hint_response.and_then(|value| value.hint_content.clone());
    run_state.recent_history = history.to_vec();
    run_state.last_progress_at = Some(chrono::Utc::now().to_rfc3339());
    store.save_challenge(run_state).await
}

async fn start_challenge_with_capacity_recovery(
    arena: &ArenaClient,
    store: &RuntimeStateStore,
    challenge: &ChallengeInfo,
    attempt_id: &str,
) -> Result<crate::arena::StartChallengeResponse> {
    match arena.start_challenge(&challenge.code).await {
        Ok(started) => Ok(started),
        Err(error) if is_instance_limit_error(&error.to_string()) => {
            let message = error.to_string();
            let _ = store
                .append_attempt_event(
                    &challenge.code,
                    attempt_id,
                    "start_blocked_by_instance_limit",
                    &serde_json::json!({
                        "error": message,
                    }),
                )
                .await;
            recover_instance_capacity(arena, store, &challenge.code, attempt_id).await;
            arena
                .start_challenge(&challenge.code)
                .await
                .with_context(|| format!("retry start_challenge failed for {}", challenge.code))
        }
        Err(error) => Err(error),
    }
}

async fn recover_instance_capacity(
    arena: &ArenaClient,
    store: &RuntimeStateStore,
    current_code: &str,
    attempt_id: &str,
) {
    let mut stop_codes = BTreeSet::new();

    if let Ok(active_codes) = store.list_active_challenge_codes().await {
        for code in active_codes {
            if code != current_code {
                stop_codes.insert(code);
            }
        }
    }

    if let Ok(list) = arena.list_challenges().await {
        for challenge in list.challenges {
            if challenge.code != current_code
                && matches!(challenge.instance_status.as_str(), "running" | "pending")
            {
                stop_codes.insert(challenge.code);
            }
        }
    }

    for code in stop_codes {
        let stop_result = arena.stop_challenge(&code).await;
        let _ = store
            .append_attempt_event(
                current_code,
                attempt_id,
                "instance_capacity_recovery",
                &serde_json::json!({
                    "stopped_code": code,
                    "ok": stop_result.is_ok(),
                    "error": stop_result.as_ref().err().map(|error| error.to_string()),
                }),
            )
            .await;
        let reason = format!(
            "local run cleared during capacity recovery triggered by {}",
            current_code
        );
        let _ = store
            .mark_challenge_terminal(&code, "interrupted", Some(reason))
            .await;
    }

    tokio::time::sleep(Duration::from_secs(1)).await;
}

fn is_instance_limit_error(message: &str) -> bool {
    message.contains("最多同时运行3个实例")
}

#[cfg(test)]
mod tests {
    use super::{build_attempt_deadline, deadline_from_started_at};
    use crate::runtime::ChallengeRunState;

    #[test]
    fn derives_deadline_from_last_started_at() {
        let run_state = ChallengeRunState {
            last_started_at: Some("2026-04-16T01:00:00+00:00".to_string()),
            ..ChallengeRunState::default()
        };
        let deadline = deadline_from_started_at(&run_state, 900).unwrap();
        assert_eq!(deadline, "2026-04-16T01:15:00+00:00");
    }

    #[test]
    fn fresh_attempt_uses_fresh_deadline_not_old_started_at() {
        let run_state = ChallengeRunState {
            last_started_at: Some("2026-04-16T01:00:00+00:00".to_string()),
            last_status: Some("give_up".to_string()),
            ..ChallengeRunState::default()
        };
        let derived = deadline_from_started_at(&run_state, 900).unwrap();
        let fresh = build_attempt_deadline(900);
        assert_ne!(derived, fresh);
    }
}

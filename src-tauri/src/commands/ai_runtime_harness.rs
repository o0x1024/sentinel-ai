use crate::agents::{AgentToolProtocolSummary, AgentTurnToolSummary};
use crate::services::database::DatabaseService;
use chrono::{TimeZone, Utc};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

const DEFAULT_AGENT_HARNESS_LEASE_SECS: i64 = 600;
const DEFAULT_AGENT_HARNESS_HEARTBEAT_SECS: u64 = 30;
pub const DEFAULT_AGENT_HARNESS_MAX_CONTINUATIONS: usize = 6;
pub const MAX_AGENT_HARNESS_MAX_CONTINUATIONS: usize = 20;
const AGENT_HARNESS_RECONCILE_AFTER_NO_PROGRESS_TURNS: usize = 3;
const AGENT_HARNESS_STALL_AFTER_NO_PROGRESS_TURNS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentHarnessMode {
    Direct,
    ToolRun,
    Planned,
    Team,
}

impl AgentHarnessMode {
    pub fn resolve(
        requested: Option<&str>,
        force_tasks: bool,
        tools_enabled: bool,
    ) -> Result<Self, String> {
        if let Some(raw) = requested.map(str::trim).filter(|value| !value.is_empty()) {
            return match raw {
                "direct" => Ok(Self::Direct),
                "tool_run" => Ok(Self::ToolRun),
                "planned" => Ok(Self::Planned),
                "team" => Ok(Self::Team),
                other => Err(format!("Unsupported harness_mode: {}", other)),
            };
        }

        if force_tasks {
            Ok(Self::Planned)
        } else if tools_enabled {
            Ok(Self::ToolRun)
        } else {
            Ok(Self::Direct)
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::ToolRun => "tool_run",
            Self::Planned => "planned",
            Self::Team => "team",
        }
    }

    fn requires_task_ledger(self) -> bool {
        matches!(self, Self::Planned | Self::Team)
    }
}

pub struct AgentHarnessSuccessAssessment {
    pub state: &'static str,
    pub error: Option<String>,
    pub task_counts: AgentHarnessTaskLedgerCounts,
    pub turn_incomplete_reason: Option<&'static str>,
    pub tool_protocol: AgentToolProtocolSummary,
    pub recent_tools: Vec<AgentTurnToolSummary>,
}

impl AgentHarnessSuccessAssessment {
    pub fn succeeded(&self) -> bool {
        self.state == "succeeded"
    }

    pub fn incomplete(&self) -> bool {
        self.state == "incomplete"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentHarnessContinuationKind {
    Standard,
    LedgerReconcile,
}

impl AgentHarnessContinuationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::LedgerReconcile => "ledger_reconcile",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentHarnessLedgerSnapshot {
    pub fingerprint: String,
    pub task_counts: AgentHarnessTaskLedgerCounts,
}

impl AgentHarnessLedgerSnapshot {
    fn from_tasks(tasks: &[sentinel_db::ExecutionTaskItem]) -> Self {
        let mut fingerprint = String::new();
        for task in tasks {
            fingerprint.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                task.item_index,
                task.status,
                task.description,
                task.result.as_deref().unwrap_or_default()
            ));
        }
        Self {
            fingerprint,
            task_counts: AgentHarnessTaskLedgerCounts::from_tasks(tasks),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct AgentHarnessLedgerWatchdog {
    last_fingerprint: Option<String>,
    last_tool_fingerprint: Option<String>,
    last_response_fingerprint: Option<String>,
    no_progress_count: usize,
    reconcile_active: bool,
}

impl AgentHarnessLedgerWatchdog {
    pub fn no_progress_count(&self) -> usize {
        self.no_progress_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentHarnessLedgerWatchdogDecision {
    Continue,
    Reconcile,
    Stalled,
}

impl AgentHarnessLedgerWatchdogDecision {
    pub fn continuation_kind(self) -> AgentHarnessContinuationKind {
        match self {
            Self::Reconcile => AgentHarnessContinuationKind::LedgerReconcile,
            Self::Continue | Self::Stalled => AgentHarnessContinuationKind::Standard,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct AgentHarnessTaskLedgerCounts {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
    pub unfinished: usize,
}

impl AgentHarnessTaskLedgerCounts {
    fn from_tasks(tasks: &[sentinel_db::ExecutionTaskItem]) -> Self {
        let mut counts = Self {
            total: tasks.len(),
            ..Self::default()
        };
        for task in tasks {
            match task.status.as_str() {
                "pending" => counts.pending += 1,
                "in_progress" => counts.in_progress += 1,
                "completed" => counts.completed += 1,
                "failed" => counts.failed += 1,
                _ => {}
            }
        }
        counts.unfinished = counts.pending + counts.in_progress;
        counts
    }
}

pub fn resolve_agent_harness_max_continuations(configured: Option<usize>) -> usize {
    configured
        .unwrap_or(DEFAULT_AGENT_HARNESS_MAX_CONTINUATIONS)
        .min(MAX_AGENT_HARNESS_MAX_CONTINUATIONS)
}

pub fn should_continue_agent_harness(
    mode: AgentHarnessMode,
    assessment: &AgentHarnessSuccessAssessment,
    continuation_count: usize,
    max_continuations: usize,
) -> bool {
    !matches!(mode, AgentHarnessMode::Direct)
        && assessment.incomplete()
        && continuation_count < max_continuations
}

pub async fn snapshot_agent_harness_ledger(
    app_handle: &AppHandle,
    execution_id: &str,
) -> Option<AgentHarnessLedgerSnapshot> {
    let db = app_handle.try_state::<Arc<DatabaseService>>()?;
    let tasks = db.get_execution_tasks(execution_id).await.ok()?;
    Some(AgentHarnessLedgerSnapshot::from_tasks(&tasks))
}

pub fn observe_agent_harness_ledger_progress(
    mode: AgentHarnessMode,
    assessment: &AgentHarnessSuccessAssessment,
    snapshot: Option<&AgentHarnessLedgerSnapshot>,
    final_response: &str,
    watchdog: &mut AgentHarnessLedgerWatchdog,
) -> AgentHarnessLedgerWatchdogDecision {
    if matches!(mode, AgentHarnessMode::Direct)
        || !assessment.incomplete()
        || assessment.task_counts.total == 0
        || assessment.task_counts.unfinished == 0
    {
        if let Some(snapshot) = snapshot {
            watchdog.last_fingerprint = Some(snapshot.fingerprint.clone());
        }
        watchdog.no_progress_count = 0;
        watchdog.reconcile_active = false;
        return AgentHarnessLedgerWatchdogDecision::Continue;
    }

    let Some(snapshot) = snapshot else {
        return AgentHarnessLedgerWatchdogDecision::Continue;
    };

    let tool_fingerprint = tool_progress_fingerprint(assessment);
    let response_fingerprint = response_progress_fingerprint(final_response);
    let ledger_progress = watchdog
        .last_fingerprint
        .as_deref()
        .is_none_or(|previous| previous != snapshot.fingerprint);
    let tool_progress = tool_fingerprint.as_ref().is_some_and(|fingerprint| {
        watchdog.last_tool_fingerprint.as_deref() != Some(fingerprint.as_str())
    });
    let response_progress = response_fingerprint.as_ref().is_some_and(|fingerprint| {
        watchdog.last_response_fingerprint.as_deref() != Some(fingerprint.as_str())
    });

    watchdog.last_fingerprint = Some(snapshot.fingerprint.clone());
    if let Some(fingerprint) = tool_fingerprint {
        watchdog.last_tool_fingerprint = Some(fingerprint);
    }
    if let Some(fingerprint) = response_fingerprint {
        watchdog.last_response_fingerprint = Some(fingerprint);
    }

    if ledger_progress
        || tool_progress
        || response_progress
        || assessment.tool_protocol.pending_tool_call_count > 0
    {
        watchdog.no_progress_count = 0;
        watchdog.reconcile_active = false;
        return AgentHarnessLedgerWatchdogDecision::Continue;
    }

    watchdog.no_progress_count += 1;
    if watchdog.no_progress_count >= AGENT_HARNESS_STALL_AFTER_NO_PROGRESS_TURNS
        && watchdog.reconcile_active
    {
        AgentHarnessLedgerWatchdogDecision::Stalled
    } else if watchdog.no_progress_count >= AGENT_HARNESS_RECONCILE_AFTER_NO_PROGRESS_TURNS {
        watchdog.reconcile_active = true;
        AgentHarnessLedgerWatchdogDecision::Reconcile
    } else {
        AgentHarnessLedgerWatchdogDecision::Continue
    }
}

fn tool_progress_fingerprint(assessment: &AgentHarnessSuccessAssessment) -> Option<String> {
    let mut fingerprint = String::new();
    for tool in assessment
        .recent_tools
        .iter()
        .filter(|tool| tool.has_result)
    {
        fingerprint.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            tool.id,
            tool.name,
            tool.success,
            tool.result_excerpt.as_deref().unwrap_or_default()
        ));
    }
    if fingerprint.is_empty() {
        None
    } else {
        Some(fingerprint)
    }
}

fn response_progress_fingerprint(final_response: &str) -> Option<String> {
    let trimmed = final_response.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.chars().take(512).collect())
    }
}

pub fn mark_agent_harness_ledger_stalled(
    mut assessment: AgentHarnessSuccessAssessment,
    no_progress_count: usize,
) -> AgentHarnessSuccessAssessment {
    let base_error = assessment
        .error
        .take()
        .unwrap_or_else(|| "Harness stopped with incomplete tasks".to_string());
    assessment.state = "stalled_without_ledger_progress";
    assessment.turn_incomplete_reason = Some("ledger_no_progress_after_reconcile");
    assessment.error = Some(format!(
        "{}; task ledger made no progress after reconcile ({} unchanged turn(s))",
        base_error, no_progress_count
    ));
    assessment
}

pub async fn build_agent_harness_continuation_task(
    app_handle: &AppHandle,
    execution_id: &str,
    original_task: &str,
    assessment: &AgentHarnessSuccessAssessment,
    continuation_count: usize,
    max_continuations: usize,
    continuation_kind: AgentHarnessContinuationKind,
) -> String {
    let mut task_lines = Vec::new();
    if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
        if let Ok(tasks) = db.get_execution_tasks(execution_id).await {
            task_lines = tasks
                .into_iter()
                .map(|task| {
                    format!(
                        "{}. [{}] {}{}",
                        task.item_index + 1,
                        task.status,
                        task.description,
                        task.result
                            .filter(|value| !value.trim().is_empty())
                            .map(|value| format!(" - {}", value))
                            .unwrap_or_default()
                    )
                })
                .collect();
        }
    }

    let task_summary = if task_lines.is_empty() {
        "No execution task snapshot is available.".to_string()
    } else {
        task_lines.join("\n")
    };
    let final_continuation_notice = if continuation_count + 1 >= max_continuations {
        "\nThis is the final Harness continuation allowed by the current Agent configuration. Reconcile the task ledger now before ending the turn."
    } else {
        ""
    };
    let continuation_reason = match assessment.turn_incomplete_reason {
        Some("tool_calls_pending") => {
            "The previous model turn stopped with pending tool calls. Continue from the existing tool state and produce the missing tool result or final answer; do not restart completed work."
        }
        Some("stopped_after_tool_result_without_final_answer") => {
            "The previous model turn completed tool use but stopped before producing a final assistant answer. Use the existing tool results and finish the answer; do not restart completed work."
        }
        Some("empty_final_assistant") => {
            "The previous model turn stopped without producing a final assistant answer. Continue the same task and write the final answer; do not restart completed work."
        }
        _ => "The previous model turn stopped before the task ledger was complete. Continue the same task. Do not restart completed work.",
    };
    let reconcile_instruction = match continuation_kind {
        AgentHarnessContinuationKind::LedgerReconcile => "\n\n[Ledger reconcile required]\nThe execution_tasks ledger did not change in the previous continuation. Before doing any further analysis, update the ledger: choose the current in_progress or pending item, persist a completed or failed status with concrete evidence, then continue only from the remaining unfinished item. Do not restate the plan or restart analysis.",
        AgentHarnessContinuationKind::Standard => "",
    };
    let recent_tool_summary = if assessment.recent_tools.is_empty() {
        "No recent tool results are available.".to_string()
    } else {
        assessment
            .recent_tools
            .iter()
            .map(|tool| {
                format!(
                    "- {} ({}) result={}{}",
                    tool.name,
                    if tool.success { "success" } else { "failed" },
                    if tool.has_result {
                        "present"
                    } else {
                        "missing"
                    },
                    tool.result_excerpt
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                        .map(|value| format!(": {}", value.replace('\n', " ")))
                        .unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "{}\n\n[Harness continuation {} of {}]\n{} Reason: {}.{}\n\nRequired order:\n1. Inspect the current execution_tasks ledger shown below when it exists.\n2. Reuse already completed work and existing tool results.\n3. Continue only pending or in_progress items when a task ledger exists.\n4. Do not end the turn until every unfinished task is completed or explicitly failed with evidence, and the final assistant answer is written.{}\n\nRecent tool protocol:\n{}\n\nCurrent execution_tasks:\n{}",
        original_task,
        continuation_count + 1,
        max_continuations,
        continuation_reason,
        assessment
            .error
            .as_deref()
            .unwrap_or("turn protocol is incomplete"),
        reconcile_instruction,
        final_continuation_notice,
        recent_tool_summary,
        task_summary
    )
}

pub async fn assess_agent_harness_success(
    app_handle: &AppHandle,
    execution_id: &str,
    mode: AgentHarnessMode,
) -> AgentHarnessSuccessAssessment {
    let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() else {
        if !mode.requires_task_ledger() {
            return AgentHarnessSuccessAssessment {
                state: "succeeded",
                error: None,
                task_counts: AgentHarnessTaskLedgerCounts::default(),
                turn_incomplete_reason: None,
                tool_protocol: AgentToolProtocolSummary::default(),
                recent_tools: Vec::new(),
            };
        }
        return AgentHarnessSuccessAssessment {
            state: "failed",
            error: Some("Planned harness could not access the task ledger database".to_string()),
            task_counts: AgentHarnessTaskLedgerCounts::default(),
            turn_incomplete_reason: None,
            tool_protocol: AgentToolProtocolSummary::default(),
            recent_tools: Vec::new(),
        };
    };

    let tasks = match db.get_execution_tasks(execution_id).await {
        Ok(items) => items,
        Err(error) => {
            if !mode.requires_task_ledger() {
                return AgentHarnessSuccessAssessment {
                    state: "succeeded",
                    error: None,
                    task_counts: AgentHarnessTaskLedgerCounts::default(),
                    turn_incomplete_reason: None,
                    tool_protocol: AgentToolProtocolSummary::default(),
                    recent_tools: Vec::new(),
                };
            }
            return AgentHarnessSuccessAssessment {
                state: "failed",
                error: Some(format!("Failed to read execution_tasks: {}", error)),
                task_counts: AgentHarnessTaskLedgerCounts::default(),
                turn_incomplete_reason: None,
                tool_protocol: AgentToolProtocolSummary::default(),
                recent_tools: Vec::new(),
            };
        }
    };

    if tasks.is_empty() {
        if !mode.requires_task_ledger() {
            return AgentHarnessSuccessAssessment {
                state: "succeeded",
                error: None,
                task_counts: AgentHarnessTaskLedgerCounts::default(),
                turn_incomplete_reason: None,
                tool_protocol: AgentToolProtocolSummary::default(),
                recent_tools: Vec::new(),
            };
        }
        return AgentHarnessSuccessAssessment {
            state: "incomplete",
            error: Some(
                "Planned harness expected execution_tasks, but no task ledger was recorded"
                    .to_string(),
            ),
            task_counts: AgentHarnessTaskLedgerCounts::default(),
            turn_incomplete_reason: Some("missing_task_ledger"),
            tool_protocol: AgentToolProtocolSummary::default(),
            recent_tools: Vec::new(),
        };
    }

    assess_task_ledger(mode, &tasks)
}

fn assess_task_ledger(
    mode: AgentHarnessMode,
    tasks: &[sentinel_db::ExecutionTaskItem],
) -> AgentHarnessSuccessAssessment {
    let task_counts = AgentHarnessTaskLedgerCounts::from_tasks(tasks);
    if task_counts.failed > 0 {
        return AgentHarnessSuccessAssessment {
            state: "failed",
            error: Some(format!(
                "{} harness task ledger contains {} failed task(s)",
                mode.as_str(),
                task_counts.failed
            )),
            task_counts,
            turn_incomplete_reason: None,
            tool_protocol: AgentToolProtocolSummary::default(),
            recent_tools: Vec::new(),
        };
    }

    if task_counts.unfinished > 0 {
        return AgentHarnessSuccessAssessment {
            state: "incomplete",
            error: Some(format!(
                "{} harness task ledger still contains {} unfinished task(s)",
                mode.as_str(),
                task_counts.unfinished
            )),
            task_counts,
            turn_incomplete_reason: Some("task_ledger_incomplete"),
            tool_protocol: AgentToolProtocolSummary::default(),
            recent_tools: Vec::new(),
        };
    }

    AgentHarnessSuccessAssessment {
        state: "succeeded",
        error: None,
        task_counts,
        turn_incomplete_reason: None,
        tool_protocol: AgentToolProtocolSummary::default(),
        recent_tools: Vec::new(),
    }
}

pub fn agent_harness_continuation_payload(
    response_chars: usize,
    mode: AgentHarnessMode,
    assessment: &AgentHarnessSuccessAssessment,
    continuation_count: usize,
    max_continuations: usize,
    continuation_kind: AgentHarnessContinuationKind,
    ledger_no_progress_count: usize,
) -> serde_json::Value {
    serde_json::json!({
        "response_chars": response_chars,
        "harness_mode": mode.as_str(),
        "harness_state": assessment.state,
        "error": assessment.error.clone(),
        "continuation": continuation_count + 1,
        "max_continuations": max_continuations,
        "continuation_kind": continuation_kind.as_str(),
        "interruption_source": "llm",
        "interruption_reason": assessment
            .turn_incomplete_reason
            .unwrap_or("llm_stopped_with_incomplete_tasks"),
        "task_counts": assessment.task_counts,
        "ledger_watchdog": {
            "decision": if matches!(continuation_kind, AgentHarnessContinuationKind::LedgerReconcile) {
                "reconcile"
            } else {
                "continue"
            },
            "no_progress_count": ledger_no_progress_count,
        },
        "tool_protocol": assessment.tool_protocol,
        "recent_tools": assessment.recent_tools.clone(),
    })
}

pub fn agent_harness_completion_payload(
    response_chars: usize,
    mode: AgentHarnessMode,
    assessment: &AgentHarnessSuccessAssessment,
    continuation_count: usize,
    max_continuations: usize,
) -> serde_json::Value {
    serde_json::json!({
        "response_chars": response_chars,
        "harness_mode": mode.as_str(),
        "harness_state": assessment.state,
        "error": assessment.error.clone(),
        "continuation_count": continuation_count,
        "max_continuations": max_continuations,
        "stop_reason": agent_harness_stop_reason(assessment, continuation_count, max_continuations),
        "task_counts": assessment.task_counts,
        "turn_incomplete_reason": assessment.turn_incomplete_reason,
        "tool_protocol": assessment.tool_protocol,
        "recent_tools": assessment.recent_tools.clone(),
    })
}

pub enum AgentHarnessStreamEmptyDecision {
    Continue {
        next_task: String,
    },
    Stop {
        assessment: AgentHarnessSuccessAssessment,
    },
}

pub fn is_empty_llm_stream_error(error: &str) -> bool {
    let normalized = error.to_lowercase();
    normalized.contains("llm stream returned empty response")
        || normalized.contains("llm stream error: llm stream returned empty response")
}

pub async fn resolve_empty_stream_harness_decision(
    app_handle: &AppHandle,
    run_id: &str,
    conversation_id: &str,
    generation: u64,
    execution_id: &str,
    original_task: &str,
    error: &str,
    mode: AgentHarnessMode,
    continuation_count: usize,
    max_continuations: usize,
) -> AgentHarnessStreamEmptyDecision {
    let task_assessment = assess_agent_harness_success(app_handle, execution_id, mode).await;
    let assessment = AgentHarnessSuccessAssessment {
        state: "incomplete",
        error: Some(error.to_string()),
        task_counts: task_assessment.task_counts,
        turn_incomplete_reason: Some("empty_final_assistant"),
        tool_protocol: AgentToolProtocolSummary::default(),
        recent_tools: Vec::new(),
    };

    if should_continue_agent_harness(mode, &assessment, continuation_count, max_continuations) {
        let continuation_kind = AgentHarnessContinuationKind::Standard;
        let payload = agent_harness_continuation_payload(
            0,
            mode,
            &assessment,
            continuation_count,
            max_continuations,
            continuation_kind,
            0,
        );
        checkpoint_agent_harness(
            app_handle,
            run_id,
            "generation_continuing",
            Some(payload.clone()),
        )
        .await;
        append_agent_harness_event(
            app_handle,
            run_id,
            conversation_id,
            generation,
            "harness_continuation_scheduled",
            Some(payload),
        )
        .await;
        let next_task = build_agent_harness_continuation_task(
            app_handle,
            execution_id,
            original_task,
            &assessment,
            continuation_count,
            max_continuations,
            continuation_kind,
        )
        .await;
        AgentHarnessStreamEmptyDecision::Continue { next_task }
    } else {
        AgentHarnessStreamEmptyDecision::Stop {
            assessment: final_agent_harness_assessment(
                assessment,
                continuation_count,
                max_continuations,
            ),
        }
    }
}

pub fn final_agent_harness_assessment(
    mut assessment: AgentHarnessSuccessAssessment,
    continuation_count: usize,
    max_continuations: usize,
) -> AgentHarnessSuccessAssessment {
    if assessment.incomplete() && continuation_count >= max_continuations {
        let base_error = assessment
            .error
            .take()
            .unwrap_or_else(|| "Harness stopped with incomplete tasks".to_string());
        assessment.state = "max_continuations_reached";
        assessment.error = Some(format!(
            "{}; max continuations reached: {}/{}",
            base_error, continuation_count, max_continuations
        ));
    }
    assessment
}

fn agent_harness_stop_reason(
    assessment: &AgentHarnessSuccessAssessment,
    continuation_count: usize,
    max_continuations: usize,
) -> &'static str {
    if assessment.succeeded() {
        "task_ledger_completed"
    } else if assessment.state == "max_continuations_reached" {
        "max_continuations_reached"
    } else if assessment.incomplete() && continuation_count >= max_continuations {
        "max_continuations_reached"
    } else if assessment.state == "stalled_without_ledger_progress" {
        "stalled_without_ledger_progress"
    } else if assessment.state == "failed" {
        "task_ledger_failed"
    } else if let Some(reason) = assessment.turn_incomplete_reason {
        reason
    } else {
        "task_ledger_unresolved"
    }
}

pub async fn create_agent_harness_run(
    app_handle: &AppHandle,
    run_id: &str,
    conversation_id: &str,
    generation: u64,
    task: &str,
    model: &str,
    provider: &str,
    mode: AgentHarnessMode,
) {
    create_agent_harness_run_with_metadata(
        app_handle,
        run_id,
        conversation_id,
        generation,
        task,
        model,
        provider,
        mode,
        "ai_assistant",
        None,
    )
    .await;
}

pub async fn create_agent_harness_run_with_metadata(
    app_handle: &AppHandle,
    run_id: &str,
    conversation_id: &str,
    generation: u64,
    task: &str,
    model: &str,
    provider: &str,
    mode: AgentHarnessMode,
    runtime: &str,
    extra_metadata: Option<serde_json::Value>,
) {
    let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() else {
        return;
    };
    let metadata = {
        let mut base = serde_json::json!({
            "runtime": runtime,
            "generation": generation,
            "harness_mode": mode.as_str(),
        });
        if let Some(extra) = extra_metadata {
            if let Some(base_object) = base.as_object_mut() {
                if let Some(extra_object) = extra.as_object() {
                    for (key, value) in extra_object {
                        base_object.insert(key.clone(), value.clone());
                    }
                } else {
                    base_object.insert("extra_metadata".to_string(), extra);
                }
            }
        }
        Some(base)
    };
    if let Err(error) = db
        .create_agent_harness_run(sentinel_db::AgentHarnessRunInput {
            id: run_id.to_string(),
            conversation_id: conversation_id.to_string(),
            generation: generation as i64,
            task: task.to_string(),
            model: Some(model.to_string()),
            provider: Some(provider.to_string()),
            metadata,
            lease_secs: Some(DEFAULT_AGENT_HARNESS_LEASE_SECS),
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

pub struct AgentHarnessHeartbeat {
    handle: tokio::task::JoinHandle<()>,
}

impl Drop for AgentHarnessHeartbeat {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub fn start_agent_harness_heartbeat(
    app_handle: AppHandle,
    run_id: String,
) -> AgentHarnessHeartbeat {
    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            DEFAULT_AGENT_HARNESS_HEARTBEAT_SECS,
        ));
        loop {
            interval.tick().await;
            let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() else {
                continue;
            };
            if let Err(error) = db
                .heartbeat_agent_harness_run(&run_id, DEFAULT_AGENT_HARNESS_LEASE_SECS)
                .await
            {
                tracing::warn!(
                    "Failed to heartbeat agent harness run {}: {}",
                    run_id,
                    error
                );
            }
        }
    });
    AgentHarnessHeartbeat { handle }
}

pub async fn append_agent_harness_event(
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

pub async fn update_agent_harness_state(
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

pub async fn checkpoint_agent_harness(
    app_handle: &AppHandle,
    run_id: &str,
    checkpoint_type: &str,
    payload: Option<serde_json::Value>,
) {
    let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() else {
        return;
    };
    if let Err(error) = db
        .append_agent_harness_checkpoint(run_id, checkpoint_type, payload.clone())
        .await
    {
        tracing::warn!(
            "Failed to append agent harness checkpoint {} for {}: {}",
            checkpoint_type,
            run_id,
            error
        );
    }
    if let Err(error) = db
        .update_agent_execution_turn_checkpoint(run_id, checkpoint_type, payload)
        .await
    {
        tracing::warn!(
            "Failed to update agent execution turn checkpoint {} for {}: {}",
            checkpoint_type,
            run_id,
            error
        );
    }
}

pub async fn finish_agent_harness(
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
    if let Some(db) = app_handle.try_state::<Arc<DatabaseService>>() {
        if let Err(update_error) = db.finish_agent_execution_turn(run_id, state, error).await {
            tracing::warn!(
                "Failed to finish agent execution turn {} as {}: {}",
                run_id,
                state,
                update_error
            );
        }
    }
}

#[tauri::command]
pub async fn get_agent_harness_runs(
    conversation_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<sentinel_db::AgentHarnessRunRecord>, String> {
    db_service
        .list_agent_harness_runs(&conversation_id)
        .await
        .map_err(|e| format!("Failed to load harness runs for {}: {}", conversation_id, e))
}

#[tauri::command]
pub async fn get_agent_harness_events(
    run_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<sentinel_db::AgentHarnessEventRecord>, String> {
    db_service
        .list_agent_harness_events(&run_id)
        .await
        .map_err(|e| format!("Failed to load harness events for {}: {}", run_id, e))
}

#[tauri::command]
pub async fn get_agent_harness_checkpoints(
    run_id: String,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<Vec<sentinel_db::AgentHarnessCheckpointRecord>, String> {
    db_service
        .list_agent_harness_checkpoints(&run_id)
        .await
        .map_err(|e| format!("Failed to load harness checkpoints for {}: {}", run_id, e))
}

#[tauri::command]
pub async fn prune_agent_harness_after(
    conversation_id: String,
    after_timestamp_ms: i64,
    db_service: State<'_, Arc<DatabaseService>>,
) -> Result<u64, String> {
    let timestamp = Utc
        .timestamp_millis_opt(after_timestamp_ms)
        .single()
        .ok_or_else(|| "Invalid timestamp".to_string())?;

    db_service
        .delete_agent_harness_runs_after(&conversation_id, timestamp)
        .await
        .map_err(|e| {
            format!(
                "Failed to prune harness runs for {} after {}: {}",
                conversation_id, after_timestamp_ms, e
            )
        })
}

#[cfg(test)]
mod tests {
    use super::{
        assess_task_ledger, AgentHarnessLedgerSnapshot, AgentHarnessLedgerWatchdog,
        AgentHarnessLedgerWatchdogDecision, AgentHarnessMode, AgentHarnessSuccessAssessment,
        AgentHarnessTaskLedgerCounts,
    };
    use crate::agents::{AgentToolProtocolSummary, AgentTurnToolSummary};

    fn task(status: &str) -> sentinel_db::ExecutionTaskItem {
        sentinel_db::ExecutionTaskItem {
            id: format!("task-{}", status),
            execution_id: "exec-1".to_string(),
            item_index: 0,
            description: "test task".to_string(),
            status: status.to_string(),
            result: None,
            created_at: "2026-05-07T00:00:00Z".to_string(),
            updated_at: "2026-05-07T00:00:00Z".to_string(),
        }
    }

    fn incomplete_assessment() -> AgentHarnessSuccessAssessment {
        AgentHarnessSuccessAssessment {
            state: "incomplete",
            error: Some("unfinished".to_string()),
            task_counts: AgentHarnessTaskLedgerCounts {
                total: 1,
                pending: 0,
                in_progress: 1,
                completed: 0,
                failed: 0,
                unfinished: 1,
            },
            turn_incomplete_reason: Some("task_ledger_incomplete"),
            tool_protocol: AgentToolProtocolSummary::default(),
            recent_tools: Vec::new(),
        }
    }

    fn snapshot(fingerprint: &str) -> AgentHarnessLedgerSnapshot {
        AgentHarnessLedgerSnapshot {
            fingerprint: fingerprint.to_string(),
            task_counts: AgentHarnessTaskLedgerCounts {
                total: 1,
                pending: 0,
                in_progress: 1,
                completed: 0,
                failed: 0,
                unfinished: 1,
            },
        }
    }

    fn with_recent_tool(
        mut assessment: AgentHarnessSuccessAssessment,
        result: &str,
    ) -> AgentHarnessSuccessAssessment {
        assessment.tool_protocol = AgentToolProtocolSummary {
            tool_call_count: 1,
            tool_result_count: 1,
            pending_tool_call_count: 0,
            final_assistant_after_last_tool_result: false,
        };
        assessment.recent_tools = vec![AgentTurnToolSummary {
            id: "tool-1".to_string(),
            name: "file_read".to_string(),
            success: true,
            has_result: true,
            result_excerpt: Some(result.to_string()),
        }];
        assessment
    }

    #[test]
    fn resolves_direct_for_short_requests_without_tools_or_tasks() {
        assert_eq!(
            AgentHarnessMode::resolve(None, false, false).unwrap(),
            AgentHarnessMode::Direct
        );
    }

    #[test]
    fn resolves_tool_run_when_tools_are_enabled_without_forced_tasks() {
        assert_eq!(
            AgentHarnessMode::resolve(None, false, true).unwrap(),
            AgentHarnessMode::ToolRun
        );
    }

    #[test]
    fn resolves_planned_when_task_contract_is_forced() {
        assert_eq!(
            AgentHarnessMode::resolve(None, true, false).unwrap(),
            AgentHarnessMode::Planned
        );
    }

    #[test]
    fn rejects_unknown_explicit_mode() {
        assert!(AgentHarnessMode::resolve(Some("legacy"), false, false).is_err());
    }

    #[test]
    fn tool_run_with_unfinished_task_ledger_is_incomplete() {
        let tasks = vec![task("completed"), task("in_progress"), task("pending")];

        let assessment = assess_task_ledger(AgentHarnessMode::ToolRun, &tasks);

        assert_eq!(assessment.state, "incomplete");
        assert_eq!(
            assessment.error.as_deref(),
            Some("tool_run harness task ledger still contains 2 unfinished task(s)")
        );
    }

    #[test]
    fn tool_run_with_failed_task_ledger_fails() {
        let tasks = vec![task("completed"), task("failed")];

        let assessment = assess_task_ledger(AgentHarnessMode::ToolRun, &tasks);

        assert_eq!(assessment.state, "failed");
        assert_eq!(
            assessment.error.as_deref(),
            Some("tool_run harness task ledger contains 1 failed task(s)")
        );
    }

    #[test]
    fn incomplete_tool_run_schedules_continuation_before_limit() {
        let assessment = incomplete_assessment();

        assert!(super::should_continue_agent_harness(
            AgentHarnessMode::ToolRun,
            &assessment,
            0,
            1
        ));
    }

    #[test]
    fn incomplete_tool_run_stops_at_configured_limit() {
        let assessment = incomplete_assessment();

        assert!(!super::should_continue_agent_harness(
            AgentHarnessMode::ToolRun,
            &assessment,
            2,
            2
        ));
    }

    #[test]
    fn incomplete_direct_run_does_not_schedule_continuation() {
        let assessment = incomplete_assessment();

        assert!(!super::should_continue_agent_harness(
            AgentHarnessMode::Direct,
            &assessment,
            0,
            1
        ));
    }

    #[test]
    fn configured_max_continuations_is_capped() {
        assert_eq!(super::resolve_agent_harness_max_continuations(Some(99)), 20);
        assert_eq!(super::resolve_agent_harness_max_continuations(Some(0)), 0);
        assert_eq!(super::resolve_agent_harness_max_continuations(None), 6);
    }

    #[test]
    fn ledger_watchdog_reconciles_then_stalls_without_progress() {
        let assessment = incomplete_assessment();
        let snapshot = snapshot("same");
        let mut watchdog = AgentHarnessLedgerWatchdog::default();

        assert_eq!(
            super::observe_agent_harness_ledger_progress(
                AgentHarnessMode::ToolRun,
                &assessment,
                Some(&snapshot),
                "",
                &mut watchdog,
            ),
            AgentHarnessLedgerWatchdogDecision::Continue
        );
        assert_eq!(
            super::observe_agent_harness_ledger_progress(
                AgentHarnessMode::ToolRun,
                &assessment,
                Some(&snapshot),
                "",
                &mut watchdog,
            ),
            AgentHarnessLedgerWatchdogDecision::Continue
        );
        assert_eq!(
            super::observe_agent_harness_ledger_progress(
                AgentHarnessMode::ToolRun,
                &assessment,
                Some(&snapshot),
                "",
                &mut watchdog,
            ),
            AgentHarnessLedgerWatchdogDecision::Continue
        );
        for _ in 0..3 {
            assert_eq!(
                super::observe_agent_harness_ledger_progress(
                    AgentHarnessMode::ToolRun,
                    &assessment,
                    Some(&snapshot),
                    "",
                    &mut watchdog,
                ),
                AgentHarnessLedgerWatchdogDecision::Reconcile
            );
        }
        assert_eq!(
            super::observe_agent_harness_ledger_progress(
                AgentHarnessMode::ToolRun,
                &assessment,
                Some(&snapshot),
                "",
                &mut watchdog,
            ),
            AgentHarnessLedgerWatchdogDecision::Stalled
        );
    }

    #[test]
    fn ledger_watchdog_resets_when_fingerprint_changes() {
        let assessment = incomplete_assessment();
        let first_snapshot = snapshot("first");
        let second_snapshot = snapshot("second");
        let mut watchdog = AgentHarnessLedgerWatchdog::default();

        assert_eq!(
            super::observe_agent_harness_ledger_progress(
                AgentHarnessMode::ToolRun,
                &assessment,
                Some(&first_snapshot),
                "",
                &mut watchdog,
            ),
            AgentHarnessLedgerWatchdogDecision::Continue
        );
        assert_eq!(
            super::observe_agent_harness_ledger_progress(
                AgentHarnessMode::ToolRun,
                &assessment,
                Some(&first_snapshot),
                "",
                &mut watchdog,
            ),
            AgentHarnessLedgerWatchdogDecision::Continue
        );
        assert_eq!(
            super::observe_agent_harness_ledger_progress(
                AgentHarnessMode::ToolRun,
                &assessment,
                Some(&first_snapshot),
                "",
                &mut watchdog,
            ),
            AgentHarnessLedgerWatchdogDecision::Continue
        );
        assert_eq!(
            super::observe_agent_harness_ledger_progress(
                AgentHarnessMode::ToolRun,
                &assessment,
                Some(&first_snapshot),
                "",
                &mut watchdog,
            ),
            AgentHarnessLedgerWatchdogDecision::Reconcile
        );
        assert_eq!(
            super::observe_agent_harness_ledger_progress(
                AgentHarnessMode::ToolRun,
                &assessment,
                Some(&second_snapshot),
                "",
                &mut watchdog,
            ),
            AgentHarnessLedgerWatchdogDecision::Continue
        );
        assert_eq!(watchdog.no_progress_count(), 0);
    }

    #[test]
    fn ledger_watchdog_treats_new_tool_results_as_progress() {
        let snapshot = snapshot("same");
        let mut watchdog = AgentHarnessLedgerWatchdog::default();
        let first = with_recent_tool(incomplete_assessment(), "first result");
        let second = with_recent_tool(incomplete_assessment(), "second result");

        assert_eq!(
            super::observe_agent_harness_ledger_progress(
                AgentHarnessMode::ToolRun,
                &first,
                Some(&snapshot),
                "",
                &mut watchdog,
            ),
            AgentHarnessLedgerWatchdogDecision::Continue
        );
        assert_eq!(
            super::observe_agent_harness_ledger_progress(
                AgentHarnessMode::ToolRun,
                &second,
                Some(&snapshot),
                "",
                &mut watchdog,
            ),
            AgentHarnessLedgerWatchdogDecision::Continue
        );
        assert_eq!(watchdog.no_progress_count(), 0);
    }

    #[test]
    fn ledger_watchdog_treats_new_assistant_text_as_progress() {
        let assessment = incomplete_assessment();
        let snapshot = snapshot("same");
        let mut watchdog = AgentHarnessLedgerWatchdog::default();

        assert_eq!(
            super::observe_agent_harness_ledger_progress(
                AgentHarnessMode::ToolRun,
                &assessment,
                Some(&snapshot),
                "first partial answer",
                &mut watchdog,
            ),
            AgentHarnessLedgerWatchdogDecision::Continue
        );
        assert_eq!(
            super::observe_agent_harness_ledger_progress(
                AgentHarnessMode::ToolRun,
                &assessment,
                Some(&snapshot),
                "second partial answer",
                &mut watchdog,
            ),
            AgentHarnessLedgerWatchdogDecision::Continue
        );
        assert_eq!(watchdog.no_progress_count(), 0);
    }

    #[test]
    fn detects_wrapped_empty_stream_errors() {
        assert!(super::is_empty_llm_stream_error(
            "LLM stream returned empty response (provider=openai, model=mimo-v2.5-pro)"
        ));
        assert!(super::is_empty_llm_stream_error(
            "LLM stream error: LLM stream returned empty response (provider=openai, model=mimo-v2.5-pro)"
        ));
        assert!(!super::is_empty_llm_stream_error(
            "LLM stream error: provider rate limit"
        ));
    }
}

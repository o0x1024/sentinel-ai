//! Subagent executor - spawn and manage subagent runs from tool calls
//!
//! Supports execution and collaboration modes:
//! - spawn: enqueue task asynchronously and return task_id immediately
//! - wait: wait for specified tasks to complete (with parent ownership checks)
//! - run: legacy synchronous execution (spawn + wait combined)
//! - shared state: key-value store scoped by parent_execution_id
//! - event bus: publish/poll events scoped by parent_execution_id

use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::{Lazy, OnceCell};
use serde_json::json;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tauri::Manager;
use tokio::sync::{watch, RwLock, Semaphore};
use tokio::task::AbortHandle;

use sentinel_tools::buildin_tools::subagent_tool::{
    set_subagent_await_executor, set_subagent_channel_executor, set_subagent_execute_executor,
    SubagentAwaitArgs, SubagentAwaitOutput, SubagentAwaitPolicy, SubagentChannelArgs,
    SubagentChannelOp, SubagentChannelOutput, SubagentExecuteArgs, SubagentExecuteMode,
    SubagentExecuteOutput,
    SubagentEventItem, SubagentEventPollArgs, SubagentEventPollOutput, SubagentEventPublishArgs,
    SubagentEventPublishOutput, SubagentRunArgs, SubagentRunOutput, SubagentSpawnArgs,
    SubagentSpawnOutput, SubagentStateGetArgs, SubagentStateGetOutput, SubagentStatePutArgs,
    SubagentStatePutOutput, SubagentStatus, SubagentTaskInfo, SubagentTaskResult, SubagentToolError,
    SubagentWaitAnyArgs, SubagentWaitAnyOutput, SubagentWaitArgs, SubagentWaitOutput,
    SubagentWorkflowNodeResult, SubagentWorkflowRunArgs, SubagentWorkflowRunOutput,
};

use super::{condense_text, execute_agent, ContextPolicy, ToolConfig, ToolSelectionStrategy};
use sentinel_core::models::database::{SubagentMessage, SubagentRun};

// ============================================================================
// Global State
// ============================================================================

static APP_HANDLE: OnceCell<tauri::AppHandle> = OnceCell::new();

/// Parent context storage (keyed by parent execution_id)
static PARENT_CONTEXTS: Lazy<Arc<RwLock<HashMap<String, SubagentParentContext>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Task storage (keyed by task_id)
static TASK_REGISTRY: Lazy<Arc<RwLock<HashMap<String, SubagentTaskEntry>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Shared state storage (scoped by parent_execution_id)
static SHARED_STATE: Lazy<Arc<RwLock<HashMap<String, HashMap<String, SharedStateEntry>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Event bus storage (scoped by parent_execution_id + channel)
static EVENT_BUS: Lazy<Arc<RwLock<HashMap<String, HashMap<String, Vec<SubagentEventItem>>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Global concurrency limiter
static GLOBAL_SEMAPHORE: Lazy<Arc<Semaphore>> = Lazy::new(|| Arc::new(Semaphore::new(5)));

/// Per-parent concurrency limiter (max 3 subagents per parent)
static PARENT_SEMAPHORES: Lazy<Arc<RwLock<HashMap<String, Arc<Semaphore>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

const MAX_SUBAGENTS_PER_PARENT: usize = 3;
const MAX_EVENTS_PER_CHANNEL: usize = 500;
const MAX_SUBAGENT_RECURSION_DEPTH: usize = 4;
const SUBAGENT_TOOL_IDS: [&str; 3] = ["subagent_execute", "subagent_await", "subagent_channel"];

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone)]
pub struct SubagentParentContext {
    pub rig_provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub system_prompt: String,
    pub tool_config: ToolConfig,
    pub max_iterations: usize,
    pub timeout_secs: u64,
    pub task_context: String,
    pub recursion_depth: usize,
    pub audit_mode: bool,
    pub audit_verification_level: Option<String>,
}

#[derive(Debug, Clone)]
struct PendingExecutionData {
    parent: SubagentParentContext,
    task: String,
    system_prompt: Option<String>,
    tool_config: Option<serde_json::Value>,
    max_iterations: usize,
    timeout_secs: Option<u64>,
    inherit_parent_tools: bool,
    recursion_depth: usize,
    role: Option<String>,
}

/// Internal task entry with completion channel
struct SubagentTaskEntry {
    info: SubagentTaskInfo,
    completion_tx: watch::Sender<Option<TaskCompletion>>,
    completion_rx: watch::Receiver<Option<TaskCompletion>>,
    abort_handle: Option<AbortHandle>,
    pending_data: PendingExecutionData,
}

#[derive(Debug, Clone)]
struct TaskCompletion {
    success: bool,
    output: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct SharedStateEntry {
    value: serde_json::Value,
    version: u64,
}

// ============================================================================
// Public API
// ============================================================================

pub fn set_app_handle(handle: tauri::AppHandle) {
    let _ = APP_HANDLE.set(handle);
}

pub async fn set_parent_context(execution_id: String, context: SubagentParentContext) {
    let mut contexts = PARENT_CONTEXTS.write().await;
    contexts.insert(execution_id, context);
}

pub async fn clear_parent_context(execution_id: &str) {
    {
        let mut contexts = PARENT_CONTEXTS.write().await;
        contexts.remove(execution_id);
    }

    // Abort all active tasks spawned by this parent
    abort_parent_tasks(execution_id).await;

    cleanup_parent_resources_if_idle(execution_id).await;
}

/// Abort all pending/running tasks belonging to a parent and mark them as failed.
async fn abort_parent_tasks(parent_id: &str) {
    let task_ids: Vec<String> = {
        let tasks = TASK_REGISTRY.read().await;
        tasks
            .values()
            .filter(|e| {
                e.info.parent_execution_id == parent_id
                    && matches!(e.info.status, SubagentStatus::Pending | SubagentStatus::Running)
            })
            .map(|e| {
                if let Some(handle) = &e.abort_handle {
                    handle.abort();
                }
                e.info.task_id.clone()
            })
            .collect()
    };

    for task_id in &task_ids {
        mark_task_terminal(
            task_id,
            TaskCompletion {
                success: false,
                output: None,
                error: Some("Parent agent terminated".to_string()),
            },
        )
        .await;
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn default_subagent_tool_config() -> ToolConfig {
    normalize_tool_config(ToolConfig {
        enabled: true,
        selection_strategy: ToolSelectionStrategy::All,
        max_tools: 50,
        fixed_tools: vec![],
        disabled_tools: vec![],
        allowed_tools: vec![],
    })
}

fn normalize_tool_config(mut config: ToolConfig) -> ToolConfig {
    // Subagents must never be able to spawn or orchestrate other subagents.
    config
        .fixed_tools
        .retain(|tool| !SUBAGENT_TOOL_IDS.contains(&tool.as_str()));
    config
        .allowed_tools
        .retain(|tool| !SUBAGENT_TOOL_IDS.contains(&tool.as_str()));

    if let ToolSelectionStrategy::Manual(ref mut tools) = config.selection_strategy {
        tools.retain(|tool| !SUBAGENT_TOOL_IDS.contains(&tool.as_str()));
    }

    for tool in SUBAGENT_TOOL_IDS {
        if !config.disabled_tools.iter().any(|t| t == tool) {
            config.disabled_tools.push(tool.to_string());
        }
    }

    config
}

fn allowed_tools_for_role(role: &str) -> Option<&'static [&'static str]> {
    match role.trim().to_lowercase().as_str() {
        "planner" => Some(&[
            "todos",
            "skills",
            "read_file",
            "project_overview",
            "audit_coverage",
            "git_diff_scope",
            "audit_report",
        ]),
        "access" | "access_control" => Some(&[
            "read_file",
            "project_overview",
            "code_search",
            "git_diff_scope",
            "call_graph_lite",
            "cross_file_taint",
            "build_cpg",
            "query_cpg",
            "cpg_taint_analysis",
            "audit_finding_upsert",
        ]),
        "auth" | "auth_jwt" | "jwt" => Some(&[
            "read_file",
            "project_overview",
            "code_search",
            "git_diff_scope",
            "call_graph_lite",
            "cross_file_taint",
            "dependency_audit",
            "build_cpg",
            "query_cpg",
            "cpg_security_scan",
            "audit_finding_upsert",
        ]),
        "state" | "state_concurrency" | "concurrency" => Some(&[
            "read_file",
            "project_overview",
            "code_search",
            "git_diff_scope",
            "call_graph_lite",
            "cross_file_taint",
            "build_cpg",
            "query_cpg",
            "cpg_taint_analysis",
            "cpg_security_scan",
            "audit_finding_upsert",
        ]),
        "payment" => Some(&[
            "read_file",
            "project_overview",
            "code_search",
            "git_diff_scope",
            "call_graph_lite",
            "cross_file_taint",
            "dependency_audit",
            "build_cpg",
            "query_cpg",
            "cpg_taint_analysis",
            "cpg_security_scan",
            "audit_finding_upsert",
            "audit_report",
        ]),
        "judge" | "reviewer" => Some(&[
            "read_file",
            "project_overview",
            "code_search",
            "git_diff_scope",
            "call_graph_lite",
            "build_cpg",
            "query_cpg",
            "cpg_taint_analysis",
            "cpg_security_scan",
            "dependency_audit",
            "tenth_man_review",
            "audit_report",
            "audit_finding_upsert",
        ]),
        _ => None,
    }
}

fn apply_role_tool_policy(mut config: ToolConfig, role: Option<&str>) -> ToolConfig {
    let Some(role_name) = role.filter(|v| !v.trim().is_empty()) else {
        return config;
    };
    let Some(allowed_for_role) = allowed_tools_for_role(role_name) else {
        return config;
    };

    let role_allowed = allowed_for_role
        .iter()
        .map(|v| (*v).to_string())
        .collect::<std::collections::HashSet<_>>();

    if config.allowed_tools.is_empty() {
        config.allowed_tools = role_allowed.iter().cloned().collect();
        config.allowed_tools.sort();
    } else {
        config.allowed_tools.retain(|tool| role_allowed.contains(tool));
    }

    config.fixed_tools.retain(|tool| role_allowed.contains(tool));

    if let ToolSelectionStrategy::Manual(ref mut tools) = config.selection_strategy {
        tools.retain(|tool| role_allowed.contains(tool));
    }

    config
}

fn subagent_context_policy(audit_mode: bool, verification_level: Option<&str>) -> ContextPolicy {
    if !audit_mode {
        return ContextPolicy::subagent();
    }
    match verification_level
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_lowercase())
        .as_deref()
    {
        Some("low") => ContextPolicy::subagent(),
        _ => ContextPolicy::subagent_audit(),
    }
}

fn max_iterations_for_verification_level(
    requested: usize,
    audit_mode: bool,
    verification_level: Option<&str>,
) -> usize {
    if !audit_mode {
        return requested.clamp(1, 500);
    }

    let cap = match verification_level
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_lowercase())
        .as_deref()
    {
        Some("low") => 120,
        Some("medium") => 260,
        _ => 500,
    };
    requested.clamp(1, cap)
}

/// Build subagent task with parent context reference
fn build_subagent_task(
    parent_task: &str,
    subagent_task: &str,
    parent_execution_id: &str,
    context_dir: &str,
) -> String {
    let parent = parent_task.trim();
    let subagent = subagent_task.trim();

    if parent.is_empty() {
        return subagent.to_string();
    }
    if subagent.is_empty() {
        return parent.to_string();
    }

    let brief = condense_text(parent, ContextPolicy::subagent().task_brief_max_chars);
    let parent_history_path = format!(
        "{}/history_{}.txt",
        context_dir,
        &parent_execution_id[..12.min(parent_execution_id.len())]
    );

    format!(
        "[Parent Context Summary]\n\
        {}\n\n\
        [Parent Context History Access]\n\
        The parent agent's full conversation history is available at:\n\
        - Path: {}\n\
        - Usage: You can use shell tools (cat, grep, less, etc.) to search this file if you need more context\n\
        - Example: `grep -i \"specific topic\" {}`\n\
        - Note: This contains the complete dialogue history from the parent agent\n\n\
        [Your Subagent Task]\n\
        {}",
        brief, parent_history_path, parent_history_path, subagent
    )
}

fn role_instruction_block(role: &str) -> Option<&'static str> {
    match role.trim().to_lowercase().as_str() {
        "planner" => Some(
            "Role: Planner\n- Break work into explicit phases.\n- Define evidence needed before drawing conclusions.\n- Assign concrete, verifiable sub-tasks.",
        ),
        "access" | "access_control" => Some(
            "Role: Access Control Auditor\n- Focus on authentication, authorization, tenant isolation, and object ownership checks.\n- Prioritize IDOR, privilege escalation, and policy bypass paths.",
        ),
        "auth" | "auth_jwt" | "jwt" => Some(
            "Role: Authentication Auditor\n- Focus on session lifecycle, token validation, refresh/revocation, and MFA downgrade paths.\n- Treat missing claim validation or weak verification as high risk.",
        ),
        "state" | "state_concurrency" | "concurrency" => Some(
            "Role: State and Concurrency Auditor\n- Focus on state transitions, idempotency, race conditions, and replay handling.\n- Explicitly check duplicate side-effects and ordering assumptions.",
        ),
        "payment" => Some(
            "Role: Payment Auditor\n- Focus on amount integrity, signature/verification flow, callback idempotency, and settlement consistency.\n- Validate money and order-state invariants before concluding.",
        ),
        "judge" | "reviewer" => Some(
            "Role: Judge\n- Challenge unsupported claims.\n- Mark findings as confirmed only with concrete evidence and reproducible steps.\n- Output a structured verdict with fields: finding_id, verdict (confirmed|probable|uncertain|rejected), confidence (0-1), evidence_refs (array), rationale.",
        ),
        _ => None,
    }
}

fn build_subagent_system_prompt(
    base_prompt: String,
    role: Option<&str>,
    audit_mode: bool,
) -> String {
    let mut prompt = base_prompt;
    if let Some(role_name) = role.filter(|v| !v.trim().is_empty()) {
        if let Some(block) = role_instruction_block(role_name) {
            prompt.push_str("\n\n");
            prompt.push_str(block);
        }
    }
    if audit_mode {
        prompt.push_str(
            "\n\nAudit mode requirements:\n- Prefer evidence-backed findings over broad claims.\n- If evidence is insufficient, state uncertainty explicitly.\n- Include file/line references whenever possible.",
        );
    }
    prompt
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum JudgeVerdictKind {
    Confirmed,
    Probable,
    Uncertain,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JudgeVerdictPayload {
    #[serde(default)]
    finding_id: Option<String>,
    verdict: JudgeVerdictKind,
    confidence: f64,
    evidence_refs: Vec<String>,
    rationale: String,
}

fn normalize_judge_output(output: String) -> String {
    let raw_excerpt: String = output.chars().take(600).collect();
    let parsed = serde_json::from_str::<JudgeVerdictPayload>(&output);
    let Ok(mut payload) = parsed else {
        return serde_json::json!({
            "verdict": "uncertain",
            "confidence": 0.2,
            "evidence_refs": [],
            "rationale": "Judge output failed strict schema validation; downgraded to uncertain.",
            "raw_excerpt": raw_excerpt,
        })
        .to_string();
    };

    payload.confidence = payload.confidence.clamp(0.0, 1.0);
    payload.evidence_refs = payload
        .evidence_refs
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(20)
        .collect();
    payload.rationale = payload.rationale.trim().to_string();

    if payload.rationale.is_empty() {
        payload.rationale = "No rationale provided.".to_string();
    }

    if matches!(payload.verdict, JudgeVerdictKind::Confirmed | JudgeVerdictKind::Probable)
        && payload.evidence_refs.is_empty()
    {
        payload.verdict = JudgeVerdictKind::Uncertain;
        payload.confidence = payload.confidence.min(0.4);
        payload.rationale = format!(
            "Downgraded to uncertain because evidence_refs is empty. {}",
            payload.rationale
        );
    }

    serde_json::to_string(&payload).unwrap_or_else(|_| {
        serde_json::json!({
            "verdict": "uncertain",
            "confidence": 0.2,
            "evidence_refs": [],
            "rationale": "Judge output serialization failed; downgraded to uncertain.",
            "raw_excerpt": raw_excerpt,
        })
        .to_string()
    })
}

fn normalize_output_for_role(role: Option<&str>, output: String) -> String {
    let Some(role_name) = role.map(str::trim).filter(|v| !v.is_empty()) else {
        return output;
    };
    match role_name.to_lowercase().as_str() {
        "judge" | "reviewer" => normalize_judge_output(output),
        _ => output,
    }
}

async fn persist_judge_lifecycle_decision(
    app_handle: &tauri::AppHandle,
    conversation_id: &str,
    output_json: &str,
) {
    let Some(db) = app_handle
        .try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
        .map(|s| s.inner().clone())
    else {
        return;
    };

    let Ok(payload) = serde_json::from_str::<JudgeVerdictPayload>(output_json) else {
        return;
    };
    let Some(finding_id) = payload
        .finding_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    else {
        return;
    };

    let stage = match payload.verdict {
        JudgeVerdictKind::Confirmed => "confirmed",
        JudgeVerdictKind::Probable => "verified",
        JudgeVerdictKind::Uncertain => "triaged",
        JudgeVerdictKind::Rejected => "rejected",
    };
    let verification_status = match payload.verdict {
        JudgeVerdictKind::Confirmed => "passed",
        JudgeVerdictKind::Probable => "pending",
        JudgeVerdictKind::Uncertain => "needs_more_evidence",
        JudgeVerdictKind::Rejected => "failed",
    };

    let judge_json = serde_json::to_string(&payload).ok();
    let provenance = serde_json::json!({
        "source": "judge_subagent",
        "conversation_id": conversation_id,
    });
    let provenance_json = serde_json::to_string(&provenance).ok();

    let finding = match db
        .get_agent_audit_finding_by_conversation_finding_id(conversation_id, finding_id)
        .await
    {
        Ok(item) => item,
        Err(e) => {
            tracing::warn!(
                "Failed loading finding for judge transition (conv={}, finding_id={}): {}",
                conversation_id,
                finding_id,
                e
            );
            return;
        }
    };

    let Some(finding) = finding else {
        tracing::debug!(
            "Judge output references unknown finding_id={} for conversation={}",
            finding_id,
            conversation_id
        );
        return;
    };

    if let Err(e) = db
        .update_agent_audit_finding_lifecycle(
            &finding.id,
            stage,
            Some(verification_status),
            judge_json.as_deref(),
            None,
            provenance_json.as_deref(),
        )
        .await
    {
        tracing::warn!(
            "Failed applying judge lifecycle transition for finding {}: {}",
            finding.id,
            e
        );
    }
}

async fn get_or_create_parent_semaphore(parent_id: &str) -> Arc<Semaphore> {
    let mut sems = PARENT_SEMAPHORES.write().await;
    sems.entry(parent_id.to_string())
        .or_insert_with(|| Arc::new(Semaphore::new(MAX_SUBAGENTS_PER_PARENT)))
        .clone()
}

async fn cleanup_parent_resources_if_idle(parent_id: &str) {
    // Atomically check for active tasks and remove terminal ones under write lock
    let has_active_tasks = {
        let mut tasks = TASK_REGISTRY.write().await;
        let active = tasks.values().any(|entry| {
            entry.info.parent_execution_id == parent_id
                && matches!(entry.info.status, SubagentStatus::Pending | SubagentStatus::Running)
        });

        if !active {
            tasks.retain(|_, entry| entry.info.parent_execution_id != parent_id);
        }
        active
    };

    if has_active_tasks {
        return;
    }

    let mut parent_sems = PARENT_SEMAPHORES.write().await;
    parent_sems.remove(parent_id);

    let mut state = SHARED_STATE.write().await;
    state.remove(parent_id);

    let mut events = EVENT_BUS.write().await;
    events.remove(parent_id);
}

fn get_app_handle() -> Result<&'static tauri::AppHandle, SubagentToolError> {
    APP_HANDLE
        .get()
        .ok_or_else(|| SubagentToolError::InternalError("AppHandle not initialized".to_string()))
}

async fn get_parent_context(parent_id: &str) -> Result<SubagentParentContext, SubagentToolError> {
    let contexts = PARENT_CONTEXTS.read().await;
    contexts
        .get(parent_id)
        .cloned()
        .ok_or_else(|| SubagentToolError::ParentContextNotFound(parent_id.to_string()))
}

async fn create_subagent_run(app_handle: &tauri::AppHandle, run: &SubagentRun) {
    if let Some(db) = app_handle
        .try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
        .map(|s| s.inner().clone())
    {
        if let Err(e) = db.create_subagent_run_internal(run).await {
            tracing::warn!("Failed to create subagent run record: {}", e);
        }
    }
}

async fn update_subagent_run_result(
    app_handle: &tauri::AppHandle,
    id: &str,
    status: &str,
    output: Option<&str>,
    error: Option<&str>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
) {
    if let Some(db) = app_handle
        .try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
        .map(|s| s.inner().clone())
    {
        if let Err(e) = db
            .update_subagent_run_result_internal(id, status, output, error, completed_at)
            .await
        {
            tracing::warn!("Failed to update subagent run record: {}", e);
        }
    }
}

async fn create_subagent_message(
    app_handle: &tauri::AppHandle,
    subagent_run_id: &str,
    role: &str,
    content: &str,
) {
    if content.trim().is_empty() {
        return;
    }
    if let Some(db) = app_handle
        .try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
        .map(|s| s.inner().clone())
    {
        let msg = SubagentMessage {
            id: uuid::Uuid::new_v4().to_string(),
            subagent_run_id: subagent_run_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            metadata: None,
            tool_calls: None,
            attachments: None,
            reasoning_content: None,
            timestamp: chrono::Utc::now(),
            structured_data: None,
        };
        if let Err(e) = db.create_subagent_message_internal(&msg).await {
            tracing::warn!("Failed to create subagent message: {}", e);
        } else {
            let _ = app_handle.emit(
                "subagent:message",
                &json!({
                    "subagent_run_id": subagent_run_id,
                    "message_id": msg.id,
                    "role": role,
                    "content": content,
                    "tool_calls": null,
                    "reasoning_content": null,
                    "timestamp": msg.timestamp.to_rfc3339(),
                }),
            );
        }
    }
}

async fn mark_task_terminal(task_id: &str, completion: TaskCompletion) {
    let now = chrono::Utc::now().timestamp();
    let mut tasks = TASK_REGISTRY.write().await;
    if let Some(entry) = tasks.get_mut(task_id) {
        entry.info.status = if completion.success {
            SubagentStatus::Completed
        } else {
            SubagentStatus::Failed
        };
        entry.info.output = completion.output.clone();
        entry.info.error = completion.error.clone();
        entry.info.completed_at = Some(now);
        let _ = entry.completion_tx.send(Some(completion));
    }
}

async fn wait_for_dependencies(task_id: &str, timeout: tokio::time::Duration) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + timeout;

    let (parent_id, deps) = {
        let tasks = TASK_REGISTRY.read().await;
        let entry = tasks
            .get(task_id)
            .ok_or_else(|| format!("Task {} not found", task_id))?;
        (
            entry.info.parent_execution_id.clone(),
            entry.info.depends_on_task_ids.clone(),
        )
    };

    if deps.is_empty() {
        return Ok(());
    }

    // Collect watch receivers and validate ownership upfront
    let dep_receivers: Vec<(String, watch::Receiver<Option<TaskCompletion>>)> = {
        let tasks = TASK_REGISTRY.read().await;
        let mut receivers = Vec::with_capacity(deps.len());
        for dep_id in &deps {
            let entry = tasks
                .get(dep_id)
                .ok_or_else(|| format!("Dependency task not found: {}", dep_id))?;
            if entry.info.parent_execution_id != parent_id {
                return Err(format!(
                    "Dependency {} belongs to different parent execution",
                    dep_id
                ));
            }
            receivers.push((dep_id.clone(), entry.completion_rx.clone()));
        }
        receivers
    };

    // Wait for each dependency via watch notification
    for (dep_id, mut rx) in dep_receivers {
        loop {
            if let Some(completion) = rx.borrow().as_ref() {
                if !completion.success {
                    return Err(format!(
                        "Dependency {} failed: {}",
                        dep_id,
                        completion.error.as_deref().unwrap_or("unknown error")
                    ));
                }
                break;
            }

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(format!("Timeout waiting for dependency {}", dep_id));
            }

            match tokio::time::timeout(remaining, rx.changed()).await {
                Ok(Ok(())) => continue,
                Ok(Err(_)) => return Err(format!("Dependency {} channel closed", dep_id)),
                Err(_) => return Err(format!("Timeout waiting for dependency {}", dep_id)),
            }
        }
    }

    Ok(())
}

async fn resolve_context_dir() -> (String, bool) {
    use sentinel_tools::output_storage::{get_host_context_dir, CONTAINER_CONTEXT_DIR};
    use sentinel_tools::shell::get_shell_config;

    let shell_config = get_shell_config().await;
    let docker_available = sentinel_tools::DockerSandbox::is_docker_available().await;
    let docker_enabled = shell_config.default_execution_mode
        == sentinel_tools::shell::ShellExecutionMode::Docker
        && shell_config.docker_config.is_some()
        && docker_available;

    if docker_enabled {
        (CONTAINER_CONTEXT_DIR.to_string(), true)
    } else {
        (get_host_context_dir().display().to_string(), false)
    }
}

async fn export_parent_history(
    app_handle: &tauri::AppHandle,
    parent_execution_id: &str,
    is_docker: bool,
) {
    if app_handle
        .try_state::<Arc<sentinel_db::DatabaseService>>()
        .is_none()
    {
        return;
    }

    use crate::agents::sliding_window::SlidingWindowManager;

    if let Ok(parent_sliding_window) =
        SlidingWindowManager::new(app_handle, parent_execution_id, None).await
    {
        if let Ok(parent_history_content) = parent_sliding_window.export_history().await {
            if is_docker {
                use sentinel_tools::shell::get_shell_config;
                let shell_config = get_shell_config().await;
                if let Some(docker_config) = shell_config.docker_config {
                    let sandbox = sentinel_tools::DockerSandbox::new(docker_config);
                    if let Err(e) = sentinel_tools::output_storage::store_history_in_container_with_id(
                        &sandbox,
                        &parent_history_content,
                        Some(parent_execution_id),
                    )
                    .await
                    {
                        tracing::warn!("Failed to export parent history to container: {}", e);
                    }
                }
            } else if let Err(e) = sentinel_tools::output_storage::store_history_on_host(
                &parent_history_content,
                Some(parent_execution_id),
            )
            .await
            {
                tracing::warn!("Failed to export parent history to host: {}", e);
            }
        }
    }
}

async fn run_task(task_id: String) {
    let app_handle = match get_app_handle() {
        Ok(h) => h.clone(),
        Err(e) => {
            mark_task_terminal(
                &task_id,
                TaskCompletion {
                    success: false,
                    output: None,
                    error: Some(e.to_string()),
                },
            )
            .await;
            return;
        }
    };

    let (parent_execution_id, dep_timeout) = {
        let tasks = TASK_REGISTRY.read().await;
        match tasks.get(&task_id) {
            Some(entry) => {
                let timeout_secs = entry
                    .pending_data
                    .timeout_secs
                    .unwrap_or(entry.pending_data.parent.timeout_secs);
                (
                    entry.info.parent_execution_id.clone(),
                    tokio::time::Duration::from_secs(timeout_secs),
                )
            }
            None => return,
        }
    };

    if let Err(err) = wait_for_dependencies(&task_id, dep_timeout).await {
        let _ = app_handle.emit(
            "subagent:error",
            &json!({"task_id": task_id, "execution_id": task_id, "error": err}),
        );
        mark_task_terminal(
            &task_id,
            TaskCompletion {
                success: false,
                output: None,
                error: Some(err),
            },
        )
        .await;
        return;
    }

    let global_permit = match GLOBAL_SEMAPHORE.clone().acquire_owned().await {
        Ok(p) => p,
        Err(_) => {
            mark_task_terminal(
                &task_id,
                TaskCompletion {
                    success: false,
                    output: None,
                    error: Some("Failed to acquire global concurrency permit".to_string()),
                },
            )
            .await;
            return;
        }
    };

    let parent_sem = get_or_create_parent_semaphore(&parent_execution_id).await;
    let parent_permit = match parent_sem.acquire_owned().await {
        Ok(p) => p,
        Err(_) => {
            drop(global_permit);
            mark_task_terminal(
                &task_id,
                TaskCompletion {
                    success: false,
                    output: None,
                    error: Some("Failed to acquire parent concurrency permit".to_string()),
                },
            )
            .await;
            return;
        }
    };

    {
        let mut tasks = TASK_REGISTRY.write().await;
        if let Some(entry) = tasks.get_mut(&task_id) {
            entry.info.status = SubagentStatus::Running;
        }
    }

    let _keep_permits = (global_permit, parent_permit);

    let pending_data = {
        let tasks = TASK_REGISTRY.read().await;
        match tasks.get(&task_id) {
            Some(entry) => entry.pending_data.clone(),
            None => return,
        }
    };

    let (context_dir, is_docker) = resolve_context_dir().await;
    export_parent_history(&app_handle, &parent_execution_id, is_docker).await;

    let task_with_context = build_subagent_task(
        &pending_data.parent.task_context,
        &pending_data.task,
        &parent_execution_id,
        &context_dir,
    );

    create_subagent_message(&app_handle, &task_id, "user", &task_with_context).await;

    let tool_config_base = if let Some(raw) = pending_data.tool_config {
        match serde_json::from_value::<ToolConfig>(raw) {
            Ok(parsed) => normalize_tool_config(parsed),
            Err(e) => {
                tracing::error!("Invalid tool_config: {}", e);
                default_subagent_tool_config()
            }
        }
    } else if pending_data.inherit_parent_tools {
        normalize_tool_config(pending_data.parent.tool_config.clone())
    } else {
        default_subagent_tool_config()
    };
    let tool_config = apply_role_tool_policy(tool_config_base, pending_data.role.as_deref());

    let system_prompt_base = pending_data
        .system_prompt
        .unwrap_or_else(|| pending_data.parent.system_prompt.clone());
    let system_prompt = build_subagent_system_prompt(
        system_prompt_base,
        pending_data.role.as_deref(),
        pending_data.parent.audit_mode,
    );
    let max_iterations = max_iterations_for_verification_level(
        pending_data.max_iterations,
        pending_data.parent.audit_mode,
        pending_data.parent.audit_verification_level.as_deref(),
    );
    let timeout_secs = pending_data
        .timeout_secs
        .unwrap_or(pending_data.parent.timeout_secs);

    let _ = app_handle.emit(
        "subagent:start",
        &json!({
            "task_id": task_id,
            "execution_id": task_id,
            "parent_execution_id": parent_execution_id,
            "task": task_with_context,
            "mode": "async",
        }),
    );

    update_subagent_run_result(&app_handle, &task_id, "running", None, None, None).await;

    let params = super::AgentExecuteParams {
        execution_id: task_id.clone(),
        model: pending_data.parent.model,
        system_prompt,
        task: task_with_context,
        rig_provider: pending_data.parent.rig_provider,
        api_key: pending_data.parent.api_key,
        api_base: pending_data.parent.api_base,
        max_iterations,
        timeout_secs,
        tool_config: Some(tool_config),
        enable_tenth_man_rule: false,
        tenth_man_config: None,
        document_attachments: None,
        image_attachments: None,
        persist_messages: false,
        subagent_run_id: Some(task_id.clone()),
        context_policy: Some(subagent_context_policy(
            pending_data.parent.audit_mode,
            pending_data.parent.audit_verification_level.as_deref(),
        )),
        recursion_depth: pending_data.recursion_depth,
        audit_mode: pending_data.parent.audit_mode,
        audit_verification_level: pending_data.parent.audit_verification_level.clone(),
    };

    let result = execute_agent(&app_handle, params).await;

    match result {
        Ok(output) => {
            let output = normalize_output_for_role(pending_data.role.as_deref(), output);
            if pending_data.parent.audit_mode
                && matches!(
                    pending_data.role.as_deref().map(|v| v.trim().to_lowercase()),
                    Some(ref role) if role == "judge" || role == "reviewer"
                )
            {
                persist_judge_lifecycle_decision(
                    &app_handle,
                    &parent_execution_id,
                    &output,
                )
                .await;
            }
            let completed_at = chrono::Utc::now();
            let _ = app_handle.emit(
                "subagent:done",
                &json!({
                    "task_id": task_id,
                    "execution_id": task_id,
                    "parent_execution_id": parent_execution_id,
                    "success": true,
                    "output": output,
                }),
            );

            update_subagent_run_result(
                &app_handle,
                &task_id,
                "completed",
                Some(&output),
                None,
                Some(completed_at),
            )
            .await;

            mark_task_terminal(
                &task_id,
                TaskCompletion {
                    success: true,
                    output: Some(output),
                    error: None,
                },
            )
            .await;
        }
        Err(e) => {
            let error = e.to_string();
            let completed_at = chrono::Utc::now();
            let _ = app_handle.emit(
                "subagent:error",
                &json!({
                    "task_id": task_id,
                    "execution_id": task_id,
                    "parent_execution_id": parent_execution_id,
                    "error": error,
                }),
            );

            update_subagent_run_result(
                &app_handle,
                &task_id,
                "failed",
                None,
                Some(&error),
                Some(completed_at),
            )
            .await;

            mark_task_terminal(
                &task_id,
                TaskCompletion {
                    success: false,
                    output: None,
                    error: Some(error),
                },
            )
            .await;
        }
    }

    cleanup_parent_resources_if_idle(&parent_execution_id).await;
}

// ============================================================================
// Executor: spawn (non-blocking)
// ============================================================================

async fn execute_spawn(args: SubagentSpawnArgs) -> Result<SubagentSpawnOutput, SubagentToolError> {
    let app_handle = get_app_handle()?;
    let parent = get_parent_context(&args.parent_execution_id).await?;

    if !args.inherit_parent_llm {
        return Err(SubagentToolError::InvalidArguments(
            "Custom LLM config is not supported yet".to_string(),
        ));
    }

    for dep_id in &args.depends_on_task_ids {
        if dep_id.trim().is_empty() {
            return Err(SubagentToolError::InvalidArguments(
                "depends_on_task_ids cannot contain empty task IDs".to_string(),
            ));
        }
    }

    let task_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let recursion_depth = parent.recursion_depth + 1;
    if recursion_depth > MAX_SUBAGENT_RECURSION_DEPTH {
        return Err(SubagentToolError::InvalidArguments(format!(
            "subagent recursion depth exceeded: {} (max {})",
            recursion_depth, MAX_SUBAGENT_RECURSION_DEPTH
        )));
    }

    let (tx, rx) = watch::channel(None);

    let task_info = SubagentTaskInfo {
        task_id: task_id.clone(),
        parent_execution_id: args.parent_execution_id.clone(),
        role: args.role.clone(),
        task: args.task.clone(),
        status: SubagentStatus::Pending,
        output: None,
        error: None,
        started_at: now.timestamp(),
        completed_at: None,
        depends_on_task_ids: args.depends_on_task_ids.clone(),
    };

    let pending_data = PendingExecutionData {
        parent: parent.clone(),
        task: args.task.clone(),
        system_prompt: args.system_prompt,
        tool_config: args.tool_config,
        max_iterations: args.max_iterations.max(1),
        timeout_secs: args.timeout_secs,
        inherit_parent_tools: args.inherit_parent_tools,
        recursion_depth,
        role: args.role.clone(),
    };

    {
        let mut tasks = TASK_REGISTRY.write().await;
        tasks.insert(
            task_id.clone(),
            SubagentTaskEntry {
                info: task_info,
                completion_tx: tx,
                completion_rx: rx,
                abort_handle: None,
                pending_data,
            },
        );
    }

    let run_record = SubagentRun {
        id: task_id.clone(),
        parent_execution_id: args.parent_execution_id.clone(),
        role: args.role.clone(),
        task: args.task,
        status: "queued".to_string(),
        output: None,
        error: None,
        model_name: Some(parent.model),
        model_provider: Some(parent.rig_provider),
        started_at: now,
        completed_at: None,
        created_at: now,
        updated_at: now,
    };
    create_subagent_run(app_handle, &run_record).await;

    let _ = app_handle.emit(
        "subagent:queued",
        &json!({
            "task_id": task_id,
            "execution_id": task_id,
            "parent_execution_id": args.parent_execution_id,
            "depends_on_task_ids": args.depends_on_task_ids,
        }),
    );

    // Spawn background runner after registry insert to avoid race.
    let runner_handle = tokio::spawn(run_task(task_id.clone()));
    {
        let mut tasks = TASK_REGISTRY.write().await;
        if let Some(entry) = tasks.get_mut(&task_id) {
            entry.abort_handle = Some(runner_handle.abort_handle());
        }
    }

    Ok(SubagentSpawnOutput {
        task_id,
        message: "Subagent task queued. Use subagent_await to get results.".to_string(),
    })
}

// ============================================================================
// Executor: wait (blocking)
// ============================================================================

async fn execute_wait(args: SubagentWaitArgs) -> Result<SubagentWaitOutput, SubagentToolError> {
    if args.task_ids.is_empty() {
        return Err(SubagentToolError::InvalidArguments(
            "task_ids cannot be empty".to_string(),
        ));
    }

    let timeout = tokio::time::Duration::from_secs(args.timeout_secs);
    let deadline = tokio::time::Instant::now() + timeout;

    let mut results = Vec::new();
    let mut success_count = 0;
    let mut fail_count = 0;

    for task_id in &args.task_ids {
        let (rx, role) = {
            let tasks = TASK_REGISTRY.read().await;
            match tasks.get(task_id) {
                Some(entry) => {
                    if entry.info.parent_execution_id != args.parent_execution_id {
                        results.push(SubagentTaskResult {
                            task_id: task_id.clone(),
                            role: entry.info.role.clone(),
                            success: false,
                            output: None,
                            error: Some(format!(
                                "Task {} does not belong to parent_execution_id {}",
                                task_id, args.parent_execution_id
                            )),
                        });
                        fail_count += 1;
                        continue;
                    }
                    (entry.completion_rx.clone(), entry.info.role.clone())
                }
                None => {
                    results.push(SubagentTaskResult {
                        task_id: task_id.clone(),
                        role: None,
                        success: false,
                        output: None,
                        error: Some(format!("Task not found: {}", task_id)),
                    });
                    fail_count += 1;
                    continue;
                }
            }
        };

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            results.push(SubagentTaskResult {
                task_id: task_id.clone(),
                role,
                success: false,
                output: None,
                error: Some("Timeout waiting for task".to_string()),
            });
            fail_count += 1;
            continue;
        }

        let mut rx = rx;
        let wait_result = tokio::time::timeout(remaining, async {
            loop {
                if let Some(completion) = rx.borrow().clone() {
                    return completion;
                }
                if rx.changed().await.is_err() {
                    return TaskCompletion {
                        success: false,
                        output: None,
                        error: Some("Task channel closed".to_string()),
                    };
                }
            }
        })
        .await;

        match wait_result {
            Ok(completion) => {
                if completion.success {
                    success_count += 1;
                } else {
                    fail_count += 1;
                }
                results.push(SubagentTaskResult {
                    task_id: task_id.clone(),
                    role,
                    success: completion.success,
                    output: completion.output,
                    error: completion.error,
                });
            }
            Err(_) => {
                fail_count += 1;
                results.push(SubagentTaskResult {
                    task_id: task_id.clone(),
                    role,
                    success: false,
                    output: None,
                    error: Some("Timeout waiting for task".to_string()),
                });
            }
        }
    }

    let summary = format!(
        "Completed {} tasks: {} succeeded, {} failed",
        results.len(), success_count, fail_count
    );

    Ok(SubagentWaitOutput { results, summary })
}

// ============================================================================
// Executor: wait_any
// ============================================================================

async fn execute_wait_any(args: SubagentWaitAnyArgs) -> Result<SubagentWaitAnyOutput, SubagentToolError> {
    if args.task_ids.is_empty() {
        return Err(SubagentToolError::InvalidArguments(
            "task_ids cannot be empty".to_string(),
        ));
    }

    let timeout = tokio::time::Duration::from_secs(args.timeout_secs);

    // First pass: check already-completed and collect watch receivers for pending
    let mut completed = Vec::new();
    let mut pending_info: Vec<(String, Option<String>, watch::Receiver<Option<TaskCompletion>>)> = Vec::new();

    {
        let tasks = TASK_REGISTRY.read().await;
        for task_id in &args.task_ids {
            match tasks.get(task_id) {
                Some(entry) => {
                    if entry.info.parent_execution_id != args.parent_execution_id {
                        completed.push(SubagentTaskResult {
                            task_id: task_id.clone(),
                            role: entry.info.role.clone(),
                            success: false,
                            output: None,
                            error: Some(format!(
                                "Task {} does not belong to parent_execution_id {}",
                                task_id, args.parent_execution_id
                            )),
                        });
                        continue;
                    }

                    if let Some(completion) = entry.completion_rx.borrow().clone() {
                        completed.push(SubagentTaskResult {
                            task_id: task_id.clone(),
                            role: entry.info.role.clone(),
                            success: completion.success,
                            output: completion.output,
                            error: completion.error,
                        });
                    } else {
                        pending_info.push((
                            task_id.clone(),
                            entry.info.role.clone(),
                            entry.completion_rx.clone(),
                        ));
                    }
                }
                None => completed.push(SubagentTaskResult {
                    task_id: task_id.clone(),
                    role: None,
                    success: false,
                    output: None,
                    error: Some(format!("Task not found: {}", task_id)),
                }),
            }
        }
    }

    if !completed.is_empty() {
        let pending_task_ids = pending_info.iter().map(|(id, _, _)| id.clone()).collect();
        return Ok(SubagentWaitAnyOutput {
            completed,
            pending_task_ids,
        });
    }

    if pending_info.is_empty() {
        return Err(SubagentToolError::InvalidArguments(
            "No tasks to wait for".to_string(),
        ));
    }

    // Use mpsc to get notified when any watcher sees a completion
    let (notify_tx, mut notify_rx) =
        tokio::sync::mpsc::channel::<usize>(pending_info.len().max(1));

    let watcher_handles: Vec<_> = pending_info
        .iter()
        .enumerate()
        .map(|(idx, (_, _, rx))| {
            let tx = notify_tx.clone();
            let mut rx = rx.clone();
            tokio::spawn(async move {
                loop {
                    if rx.borrow().is_some() {
                        let _ = tx.send(idx).await;
                        return;
                    }
                    if rx.changed().await.is_err() {
                        let _ = tx.send(idx).await;
                        return;
                    }
                }
            })
        })
        .collect();
    drop(notify_tx);

    let wait_result = tokio::time::timeout(timeout, notify_rx.recv()).await;

    for h in &watcher_handles {
        h.abort();
    }

    match wait_result {
        Ok(Some(completed_idx)) => {
            let (task_id, role, rx) = &pending_info[completed_idx];
            let completion = rx.borrow().clone().unwrap_or(TaskCompletion {
                success: false,
                output: None,
                error: Some("Task channel closed".to_string()),
            });
            completed.push(SubagentTaskResult {
                task_id: task_id.clone(),
                role: role.clone(),
                success: completion.success,
                output: completion.output,
                error: completion.error,
            });

            let pending_task_ids = pending_info
                .iter()
                .enumerate()
                .filter(|(idx, _)| *idx != completed_idx)
                .map(|(_, (id, _, _))| id.clone())
                .collect();

            Ok(SubagentWaitAnyOutput {
                completed,
                pending_task_ids,
            })
        }
        Ok(None) => Err(SubagentToolError::Timeout),
        Err(_) => Err(SubagentToolError::Timeout),
    }
}

// ============================================================================
// Executor: workflow_run (DAG orchestration)
// ============================================================================

async fn execute_workflow_run(args: SubagentWorkflowRunArgs) -> Result<SubagentWorkflowRunOutput, SubagentToolError> {
    if args.nodes.is_empty() {
        return Err(SubagentToolError::InvalidArguments(
            "workflow nodes cannot be empty".to_string(),
        ));
    }

    let workflow_id = uuid::Uuid::new_v4().to_string();
    let mut remaining = args.nodes.clone();
    let mut node_to_task_id: HashMap<String, String> = HashMap::new();
    let mut spawn_order: Vec<(String, String)> = Vec::new();
    let mut seen_node_ids = std::collections::HashSet::new();

    for node in &args.nodes {
        if node.node_id.trim().is_empty() {
            return Err(SubagentToolError::InvalidArguments(
                "node_id cannot be empty".to_string(),
            ));
        }
        if !seen_node_ids.insert(node.node_id.clone()) {
            return Err(SubagentToolError::InvalidArguments(format!(
                "duplicate workflow node_id: {}",
                node.node_id
            )));
        }
    }

    while !remaining.is_empty() {
        let mut ready_indexes = Vec::new();
        for (idx, node) in remaining.iter().enumerate() {
            let ready = node
                .depends_on_node_ids
                .iter()
                .all(|dep| node_to_task_id.contains_key(dep));
            if ready {
                ready_indexes.push(idx);
            }
        }

        if ready_indexes.is_empty() {
            let unresolved = remaining
                .iter()
                .map(|n| n.node_id.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(SubagentToolError::InvalidArguments(format!(
                "workflow has cyclic or unresolved dependencies among nodes: {}",
                unresolved
            )));
        }

        for idx in ready_indexes.into_iter().rev() {
            let node = remaining.remove(idx);
            let depends_on_task_ids = node
                .depends_on_node_ids
                .iter()
                .map(|dep| {
                    node_to_task_id.get(dep).cloned().ok_or_else(|| {
                        SubagentToolError::InvalidArguments(format!(
                            "dependency node not found: {}",
                            dep
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            let spawn_output = execute_spawn(SubagentSpawnArgs {
                parent_execution_id: args.parent_execution_id.clone(),
                task: node.task,
                role: node.role,
                system_prompt: None,
                tool_config: None,
                max_iterations: node.max_iterations,
                timeout_secs: node.timeout_secs,
                inherit_parent_llm: true,
                inherit_parent_tools: true,
                depends_on_task_ids,
            })
            .await?;

            node_to_task_id.insert(node.node_id.clone(), spawn_output.task_id.clone());
            spawn_order.push((node.node_id, spawn_output.task_id));
        }
    }

    let wait_output = execute_wait(SubagentWaitArgs {
        parent_execution_id: args.parent_execution_id.clone(),
        task_ids: spawn_order.iter().map(|(_, task_id)| task_id.clone()).collect(),
        timeout_secs: args.timeout_secs,
    })
    .await?;

    let by_task = wait_output
        .results
        .into_iter()
        .map(|r| (r.task_id.clone(), r))
        .collect::<HashMap<_, _>>();

    let mut results = Vec::new();
    for (node_id, task_id) in spawn_order {
        if let Some(result) = by_task.get(&task_id) {
            results.push(SubagentWorkflowNodeResult {
                node_id,
                task_id,
                result: result.clone(),
            });
        }
    }

    let success = results.iter().filter(|r| r.result.success).count();
    let failed = results.len().saturating_sub(success);
    let summary = format!(
        "Workflow {} completed: {} nodes, {} succeeded, {} failed",
        workflow_id,
        results.len(),
        success,
        failed
    );

    Ok(SubagentWorkflowRunOutput {
        workflow_id,
        results,
        summary,
    })
}

// ============================================================================
// Executor: run (legacy, blocking)
// ============================================================================

const WAIT_TIMEOUT_BUFFER_SECS: u64 = 30;

async fn execute_run(args: SubagentRunArgs) -> Result<SubagentRunOutput, SubagentToolError> {
    let execution_timeout = args.timeout_secs;
    let parent_execution_id = args.parent_execution_id.clone();

    let spawn_args = SubagentSpawnArgs {
        parent_execution_id,
        role: args.role,
        task: args.task,
        inherit_parent_llm: args.inherit_parent_llm,
        inherit_parent_tools: args.inherit_parent_tools,
        tool_config: args.tool_config,
        system_prompt: args.system_prompt,
        max_iterations: args.max_iterations,
        timeout_secs: execution_timeout,
        depends_on_task_ids: args.depends_on_task_ids,
    };

    let spawn_output = execute_spawn(spawn_args).await?;
    let task_id = spawn_output.task_id;

    // Wait timeout = execution timeout + buffer for cleanup/transition
    let base_timeout = execution_timeout
        .unwrap_or_else(sentinel_tools::buildin_tools::subagent_tool::get_default_subagent_timeout);
    let wait_timeout_secs = base_timeout.saturating_add(WAIT_TIMEOUT_BUFFER_SECS);

    let wait_args = SubagentWaitArgs {
        parent_execution_id: args.parent_execution_id,
        task_ids: vec![task_id.clone()],
        timeout_secs: wait_timeout_secs,
    };

    let wait_output = execute_wait(wait_args).await?;

    if let Some(result) = wait_output.results.first() {
        if result.success {
            Ok(SubagentRunOutput {
                success: true,
                execution_id: task_id,
                output: result.output.clone(),
                error: None,
            })
        } else {
            let err_msg = result
                .error
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            Err(SubagentToolError::ExecutionFailed(err_msg))
        }
    } else {
        Err(SubagentToolError::ExecutionFailed(
            "Task lost during wait".to_string(),
        ))
    }
}

// ============================================================================
// Executor: shared state
// ============================================================================

async fn execute_state_put(
    args: SubagentStatePutArgs,
) -> Result<SubagentStatePutOutput, SubagentToolError> {
    if args.key.trim().is_empty() {
        return Err(SubagentToolError::InvalidArguments(
            "state key cannot be empty".to_string(),
        ));
    }

    let mut state = SHARED_STATE.write().await;
    let parent_state = state
        .entry(args.parent_execution_id)
        .or_insert_with(HashMap::new);

    let current_version = parent_state.get(&args.key).map(|e| e.version).unwrap_or(0);
    if let Some(expected) = args.expected_version {
        if expected != current_version {
            return Err(SubagentToolError::InvalidArguments(format!(
                "version mismatch for key {}: expected {}, current {}",
                args.key, expected, current_version
            )));
        }
    }

    let next_version = current_version + 1;
    parent_state.insert(
        args.key.clone(),
        SharedStateEntry {
            value: args.value,
            version: next_version,
        },
    );

    Ok(SubagentStatePutOutput {
        key: args.key,
        version: next_version,
    })
}

async fn execute_state_get(
    args: SubagentStateGetArgs,
) -> Result<SubagentStateGetOutput, SubagentToolError> {
    if args.key.trim().is_empty() {
        return Err(SubagentToolError::InvalidArguments(
            "state key cannot be empty".to_string(),
        ));
    }

    let state = SHARED_STATE.read().await;
    let Some(parent_state) = state.get(&args.parent_execution_id) else {
        return Ok(SubagentStateGetOutput {
            key: args.key,
            found: false,
            value: None,
            version: None,
        });
    };

    match parent_state.get(&args.key) {
        Some(entry) => Ok(SubagentStateGetOutput {
            key: args.key,
            found: true,
            value: Some(entry.value.clone()),
            version: Some(entry.version),
        }),
        None => Ok(SubagentStateGetOutput {
            key: args.key,
            found: false,
            value: None,
            version: None,
        }),
    }
}

// ============================================================================
// Executor: event bus
// ============================================================================

async fn execute_event_publish(
    args: SubagentEventPublishArgs,
) -> Result<SubagentEventPublishOutput, SubagentToolError> {
    if args.channel.trim().is_empty() {
        return Err(SubagentToolError::InvalidArguments(
            "channel cannot be empty".to_string(),
        ));
    }

    let mut bus = EVENT_BUS.write().await;
    let parent_bus = bus
        .entry(args.parent_execution_id)
        .or_insert_with(HashMap::new);

    let channel_events = parent_bus
        .entry(args.channel.clone())
        .or_insert_with(Vec::new);
    let next_seq = channel_events.last().map(|e| e.seq + 1).unwrap_or(1);

    channel_events.push(SubagentEventItem {
        channel: args.channel.clone(),
        seq: next_seq,
        timestamp: chrono::Utc::now().timestamp(),
        payload: args.payload,
    });

    // Evict oldest events when exceeding capacity
    if channel_events.len() > MAX_EVENTS_PER_CHANNEL {
        let drain_count = channel_events.len() - MAX_EVENTS_PER_CHANNEL;
        channel_events.drain(..drain_count);
    }

    Ok(SubagentEventPublishOutput {
        channel: args.channel,
        seq: next_seq,
    })
}

async fn execute_event_poll(
    args: SubagentEventPollArgs,
) -> Result<SubagentEventPollOutput, SubagentToolError> {
    if args.channel.trim().is_empty() {
        return Err(SubagentToolError::InvalidArguments(
            "channel cannot be empty".to_string(),
        ));
    }

    let limit = args.limit.clamp(1, 200);
    let after_seq = args.after_seq.unwrap_or(0);

    let bus = EVENT_BUS.read().await;
    let Some(parent_bus) = bus.get(&args.parent_execution_id) else {
        return Ok(SubagentEventPollOutput {
            channel: args.channel,
            latest_seq: 0,
            events: vec![],
        });
    };

    let Some(channel_events) = parent_bus.get(&args.channel) else {
        return Ok(SubagentEventPollOutput {
            channel: args.channel,
            latest_seq: 0,
            events: vec![],
        });
    };

    let latest_seq = channel_events.last().map(|e| e.seq).unwrap_or(0);
    let events = channel_events
        .iter()
        .filter(|e| e.seq > after_seq)
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();

    Ok(SubagentEventPollOutput {
        channel: args.channel,
        latest_seq,
        events,
    })
}

// ============================================================================
// Unified executors
// ============================================================================

async fn execute_unified(args: SubagentExecuteArgs) -> Result<SubagentExecuteOutput, SubagentToolError> {
    match args.mode {
        SubagentExecuteMode::Async => {
            let task = args
                .task
                .ok_or_else(|| SubagentToolError::InvalidArguments("task is required for mode=async".to_string()))?;
            let spawn_output = execute_spawn(SubagentSpawnArgs {
                parent_execution_id: args.parent_execution_id,
                task,
                role: args.role,
                system_prompt: args.system_prompt,
                tool_config: args.tool_config,
                max_iterations: args.max_iterations,
                timeout_secs: args.timeout_secs,
                inherit_parent_llm: args.inherit_parent_llm,
                inherit_parent_tools: args.inherit_parent_tools,
                depends_on_task_ids: args.depends_on_task_ids,
            })
            .await?;

            Ok(SubagentExecuteOutput {
                mode: SubagentExecuteMode::Async,
                success: true,
                task_id: Some(spawn_output.task_id),
                execution_id: None,
                workflow_id: None,
                result: None,
                results: None,
                summary: Some(spawn_output.message),
                error: None,
            })
        }
        SubagentExecuteMode::Sync => {
            let task = args
                .task
                .ok_or_else(|| SubagentToolError::InvalidArguments("task is required for mode=sync".to_string()))?;
            let run_output = execute_run(SubagentRunArgs {
                parent_execution_id: args.parent_execution_id,
                task,
                role: args.role.clone(),
                system_prompt: args.system_prompt,
                tool_config: args.tool_config,
                max_iterations: args.max_iterations,
                timeout_secs: args.timeout_secs,
                inherit_parent_llm: args.inherit_parent_llm,
                inherit_parent_tools: args.inherit_parent_tools,
                depends_on_task_ids: args.depends_on_task_ids,
            })
            .await?;

            Ok(SubagentExecuteOutput {
                mode: SubagentExecuteMode::Sync,
                success: run_output.success,
                task_id: None,
                execution_id: Some(run_output.execution_id.clone()),
                workflow_id: None,
                result: Some(SubagentTaskResult {
                    task_id: run_output.execution_id,
                    role: args.role,
                    success: run_output.success,
                    output: run_output.output,
                    error: run_output.error,
                }),
                results: None,
                summary: Some("Synchronous subagent execution completed".to_string()),
                error: None,
            })
        }
        SubagentExecuteMode::Workflow => {
            if args.nodes.is_empty() {
                return Err(SubagentToolError::InvalidArguments(
                    "nodes is required for mode=workflow".to_string(),
                ));
            }
            let timeout_secs =
                args.timeout_secs
                    .unwrap_or_else(sentinel_tools::buildin_tools::subagent_tool::get_default_subagent_timeout);
            let workflow_output = execute_workflow_run(SubagentWorkflowRunArgs {
                parent_execution_id: args.parent_execution_id,
                nodes: args.nodes,
                timeout_secs,
            })
            .await?;

            Ok(SubagentExecuteOutput {
                mode: SubagentExecuteMode::Workflow,
                success: true,
                task_id: None,
                execution_id: None,
                workflow_id: Some(workflow_output.workflow_id),
                result: None,
                results: Some(workflow_output.results),
                summary: Some(workflow_output.summary),
                error: None,
            })
        }
    }
}

async fn await_unified(args: SubagentAwaitArgs) -> Result<SubagentAwaitOutput, SubagentToolError> {
    match args.policy {
        SubagentAwaitPolicy::All => {
            let wait_output = execute_wait(SubagentWaitArgs {
                parent_execution_id: args.parent_execution_id,
                task_ids: args.task_ids,
                timeout_secs: args.timeout_secs,
            })
            .await?;
            Ok(SubagentAwaitOutput {
                policy: SubagentAwaitPolicy::All,
                completed: wait_output.results,
                pending_task_ids: vec![],
                summary: Some(wait_output.summary),
            })
        }
        SubagentAwaitPolicy::Any => {
            let wait_output = execute_wait_any(SubagentWaitAnyArgs {
                parent_execution_id: args.parent_execution_id,
                task_ids: args.task_ids,
                timeout_secs: args.timeout_secs,
            })
            .await?;
            Ok(SubagentAwaitOutput {
                policy: SubagentAwaitPolicy::Any,
                completed: wait_output.completed,
                pending_task_ids: wait_output.pending_task_ids,
                summary: None,
            })
        }
    }
}

async fn channel_unified(args: SubagentChannelArgs) -> Result<SubagentChannelOutput, SubagentToolError> {
    match args.op {
        SubagentChannelOp::StatePut => {
            let key = args.key.ok_or_else(|| {
                SubagentToolError::InvalidArguments("key is required for op=state.put".to_string())
            })?;
            let value = args.value.ok_or_else(|| {
                SubagentToolError::InvalidArguments("value is required for op=state.put".to_string())
            })?;

            let output = execute_state_put(SubagentStatePutArgs {
                parent_execution_id: args.parent_execution_id,
                key,
                value,
                expected_version: args.expected_version,
            })
            .await?;
            Ok(SubagentChannelOutput {
                op: SubagentChannelOp::StatePut,
                key: Some(output.key),
                found: None,
                value: None,
                version: Some(output.version),
                channel: None,
                seq: None,
                latest_seq: None,
                events: None,
            })
        }
        SubagentChannelOp::StateGet => {
            let key = args.key.ok_or_else(|| {
                SubagentToolError::InvalidArguments("key is required for op=state.get".to_string())
            })?;
            let output = execute_state_get(SubagentStateGetArgs {
                parent_execution_id: args.parent_execution_id,
                key,
            })
            .await?;
            Ok(SubagentChannelOutput {
                op: SubagentChannelOp::StateGet,
                key: Some(output.key),
                found: Some(output.found),
                value: output.value,
                version: output.version,
                channel: None,
                seq: None,
                latest_seq: None,
                events: None,
            })
        }
        SubagentChannelOp::EventPublish => {
            let payload = args.payload.ok_or_else(|| {
                SubagentToolError::InvalidArguments(
                    "payload is required for op=event.publish".to_string(),
                )
            })?;
            let output = execute_event_publish(SubagentEventPublishArgs {
                parent_execution_id: args.parent_execution_id,
                channel: args.channel,
                payload,
            })
            .await?;
            Ok(SubagentChannelOutput {
                op: SubagentChannelOp::EventPublish,
                key: None,
                found: None,
                value: None,
                version: None,
                channel: Some(output.channel),
                seq: Some(output.seq),
                latest_seq: None,
                events: None,
            })
        }
        SubagentChannelOp::EventPoll => {
            let output = execute_event_poll(SubagentEventPollArgs {
                parent_execution_id: args.parent_execution_id,
                channel: args.channel,
                after_seq: args.after_seq,
                limit: args.limit,
            })
            .await?;
            Ok(SubagentChannelOutput {
                op: SubagentChannelOp::EventPoll,
                key: None,
                found: None,
                value: None,
                version: None,
                channel: Some(output.channel),
                seq: None,
                latest_seq: Some(output.latest_seq),
                events: Some(output.events),
            })
        }
    }
}

// ============================================================================
// Initialization
// ============================================================================

pub fn init_subagent_executor() {
    let execute_executor = std::sync::Arc::new(|args: SubagentExecuteArgs| {
        Box::pin(execute_unified(args))
            as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>
    });
    set_subagent_execute_executor(execute_executor);

    let await_executor = std::sync::Arc::new(|args: SubagentAwaitArgs| {
        Box::pin(await_unified(args))
            as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>
    });
    set_subagent_await_executor(await_executor);

    let channel_executor = std::sync::Arc::new(|args: SubagentChannelArgs| {
        Box::pin(channel_unified(args))
            as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>
    });
    set_subagent_channel_executor(channel_executor);

    tracing::info!("Subagent executors initialized (execute/await/channel)");
}

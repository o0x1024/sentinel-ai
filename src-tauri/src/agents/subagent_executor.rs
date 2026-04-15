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
use tauri::Emitter;
use tauri::Manager;
use tokio::sync::{watch, RwLock, Semaphore};
use tokio::task::AbortHandle;

use sentinel_tools::buildin_tools::agent_control_tool::{
    set_close_agent_executor, set_list_agents_executor, set_spawn_agent_executor,
    set_wait_agents_executor, AgentHandleItem, CloseAgentArgs, CloseAgentOutput, ListAgentsArgs,
    ListAgentsOutput, SpawnAgentArgs, SpawnAgentOutput, WaitAgentsArgs, WaitAgentsOutput,
};
use sentinel_tools::buildin_tools::subagent_tool::{
    SubagentSpawnArgs, SubagentSpawnOutput, SubagentStatus, SubagentTaskInfo, SubagentTaskResult,
    SubagentToolError, SubagentWaitArgs, SubagentWaitOutput,
};

use super::{condense_text, execute_agent, ContextPolicy, ToolConfig};
use crate::agents::ToolSelectionStrategy;
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

/// Global concurrency limiter
static GLOBAL_SEMAPHORE: Lazy<Arc<Semaphore>> = Lazy::new(|| Arc::new(Semaphore::new(5)));

/// Per-parent concurrency limiter (max 3 subagents per parent)
static PARENT_SEMAPHORES: Lazy<Arc<RwLock<HashMap<String, Arc<Semaphore>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

const MAX_SUBAGENTS_PER_PARENT: usize = 3;
const MAX_SUBAGENT_RECURSION_DEPTH: usize = 4;
const SUBAGENT_TOOL_IDS: [&str; 4] = ["spawn_agent", "wait_agents", "list_agents", "close_agent"];

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
}

#[derive(Debug, Clone)]
pub struct ControlPlaneSpawnRequest {
    pub parent_execution_id: String,
    pub task: String,
    pub role: Option<String>,
    pub system_prompt: Option<String>,
    pub tool_config: Option<serde_json::Value>,
    pub max_iterations: usize,
    pub timeout_secs: Option<u64>,
    pub inherit_parent_tools: bool,
    pub depends_on_task_ids: Vec<String>,
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
                    && matches!(
                        e.info.status,
                        SubagentStatus::Pending | SubagentStatus::Running
                    )
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

fn subagent_context_policy() -> ContextPolicy {
    ContextPolicy::subagent()
}

fn max_iterations_for_verification_level(requested: usize) -> usize {
    requested.clamp(1, 500)
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
            "Role: Reviewer\n- Challenge unsupported claims.\n- Call out evidence gaps explicitly.\n- Provide a concise verdict and confidence score.",
        ),
        _ => None,
    }
}

fn build_subagent_system_prompt(base_prompt: String, role: Option<&str>) -> String {
    let mut prompt = base_prompt;
    if let Some(role_name) = role.filter(|v| !v.trim().is_empty()) {
        if let Some(block) = role_instruction_block(role_name) {
            prompt.push_str("\n\n");
            prompt.push_str(block);
        }
    }
    prompt
}

fn normalize_output_for_role(role: Option<&str>, output: String) -> String {
    let _ = role;
    output
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
                && matches!(
                    entry.info.status,
                    SubagentStatus::Pending | SubagentStatus::Running
                )
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

async fn wait_for_dependencies(
    task_id: &str,
    timeout: tokio::time::Duration,
) -> Result<(), String> {
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
                    if let Err(e) =
                        sentinel_tools::output_storage::store_history_in_container_with_id(
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
    let tool_config = tool_config_base;

    let system_prompt_base = pending_data
        .system_prompt
        .unwrap_or_else(|| pending_data.parent.system_prompt.clone());
    let system_prompt =
        build_subagent_system_prompt(system_prompt_base, pending_data.role.as_deref());
    let max_iterations = max_iterations_for_verification_level(pending_data.max_iterations);
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
        active_terminal_session_fingerprint: None,
        active_terminal_session_id: None,
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
        context_policy: Some(subagent_context_policy()),
        context_engine_mode: Some(crate::agents::ContextEngineMode::CodexLike),
        recursion_depth: pending_data.recursion_depth,
    };

    let result = execute_agent(&app_handle, params).await;

    match result {
        Ok(output) => {
            let output = normalize_output_for_role(pending_data.role.as_deref(), output);
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
        message: "Subagent task queued. Use wait_agents to get results.".to_string(),
    })
}

pub async fn control_plane_spawn_task(request: ControlPlaneSpawnRequest) -> Result<String, String> {
    execute_spawn(SubagentSpawnArgs {
        parent_execution_id: request.parent_execution_id,
        task: request.task,
        role: request.role,
        system_prompt: request.system_prompt,
        tool_config: request.tool_config,
        max_iterations: request.max_iterations.max(1),
        timeout_secs: request.timeout_secs,
        inherit_parent_llm: true,
        inherit_parent_tools: request.inherit_parent_tools,
        depends_on_task_ids: request.depends_on_task_ids,
    })
    .await
    .map(|output| output.task_id)
    .map_err(|error| error.to_string())
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
        results.len(),
        success_count,
        fail_count
    );

    Ok(SubagentWaitOutput { results, summary })
}

pub async fn control_plane_wait_tasks(
    parent_execution_id: String,
    task_ids: Vec<String>,
    timeout_secs: u64,
) -> Result<SubagentWaitOutput, String> {
    execute_wait(SubagentWaitArgs {
        parent_execution_id,
        task_ids,
        timeout_secs,
    })
    .await
    .map_err(|error| error.to_string())
}

pub async fn control_plane_list_tasks(parent_execution_id: &str) -> Vec<SubagentTaskInfo> {
    let tasks = TASK_REGISTRY.read().await;
    let mut items = tasks
        .values()
        .filter(|entry| entry.info.parent_execution_id == parent_execution_id)
        .map(|entry| entry.info.clone())
        .collect::<Vec<_>>();
    items.sort_by(|left, right| right.started_at.cmp(&left.started_at));
    items
}

pub async fn control_plane_close_task(
    parent_execution_id: &str,
    task_id: &str,
) -> Result<(), String> {
    let abort_handle = {
        let tasks = TASK_REGISTRY.read().await;
        let Some(entry) = tasks.get(task_id) else {
            return Err(format!("Task not found: {}", task_id));
        };
        if entry.info.parent_execution_id != parent_execution_id {
            return Err(format!(
                "Task {} does not belong to parent_execution_id {}",
                task_id, parent_execution_id
            ));
        }
        entry.abort_handle.clone()
    };

    if let Some(handle) = abort_handle {
        handle.abort();
    }

    mark_task_terminal(
        task_id,
        TaskCompletion {
            success: false,
            output: None,
            error: Some("Task closed by control plane".to_string()),
        },
    )
    .await;
    cleanup_parent_resources_if_idle(parent_execution_id).await;
    Ok(())
}

// ============================================================================
// Initialization
// ============================================================================

pub fn init_subagent_executor() {
    let spawn_executor = std::sync::Arc::new(|args: SpawnAgentArgs| {
        Box::pin(async move {
            let task_id = super::control_plane::spawn_agent(ControlPlaneSpawnRequest {
                parent_execution_id: args.parent_execution_id,
                task: args.task,
                role: args.role,
                system_prompt: args.system_prompt,
                tool_config: args.tool_config,
                max_iterations: args.max_iterations,
                timeout_secs: args.timeout_secs,
                inherit_parent_tools: args.inherit_parent_tools,
                depends_on_task_ids: args.depends_on_task_ids,
            })
            .await
            .map_err(|message| {
                sentinel_tools::buildin_tools::agent_control_tool::AgentControlToolError { message }
            })?;
            Ok(SpawnAgentOutput {
                task_id,
                status: "queued".to_string(),
            })
        }) as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>
    });
    set_spawn_agent_executor(spawn_executor);

    let wait_executor = std::sync::Arc::new(|args: WaitAgentsArgs| {
        Box::pin(async move {
            let agents = super::control_plane::wait_agents(
                args.parent_execution_id,
                args.task_ids,
                args.timeout_secs,
            )
            .await
            .map_err(|message| {
                sentinel_tools::buildin_tools::agent_control_tool::AgentControlToolError { message }
            })?
            .into_iter()
            .map(|item| AgentHandleItem {
                id: item.id,
                parent_execution_id: item.parent_execution_id,
                kind: format!("{:?}", item.kind).to_ascii_lowercase(),
                role: item.role,
                task: item.task,
                status: item.status,
                started_at: item.started_at,
                completed_at: item.completed_at,
                output: item.output,
                error: item.error,
            })
            .collect();
            Ok(WaitAgentsOutput { agents })
        }) as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>
    });
    set_wait_agents_executor(wait_executor);

    let list_executor = std::sync::Arc::new(|args: ListAgentsArgs| {
        Box::pin(async move {
            let agents = super::control_plane::list_agents(&args.parent_execution_id)
                .await
                .into_iter()
                .map(|item| AgentHandleItem {
                    id: item.id,
                    parent_execution_id: item.parent_execution_id,
                    kind: format!("{:?}", item.kind).to_ascii_lowercase(),
                    role: item.role,
                    task: item.task,
                    status: item.status,
                    started_at: item.started_at,
                    completed_at: item.completed_at,
                    output: item.output,
                    error: item.error,
                })
                .collect();
            Ok(ListAgentsOutput { agents })
        }) as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>
    });
    set_list_agents_executor(list_executor);

    let close_executor = std::sync::Arc::new(|args: CloseAgentArgs| {
        Box::pin(async move {
            super::control_plane::close_agent(&args.parent_execution_id, &args.task_id)
                .await
                .map_err(|message| {
                    sentinel_tools::buildin_tools::agent_control_tool::AgentControlToolError {
                        message,
                    }
                })?;
            Ok(CloseAgentOutput {
                closed: true,
                task_id: args.task_id,
            })
        }) as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>
    });
    set_close_agent_executor(close_executor);

    tracing::info!("Subagent executors initialized (spawn/wait/list/close)");
}

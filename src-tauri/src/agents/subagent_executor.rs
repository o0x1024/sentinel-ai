//! Subagent executor - spawn and manage subagent runs from tool calls
//!
//! Supports three execution modes:
//! - spawn: Start task asynchronously, return task_id immediately
//! - wait: Wait for specified tasks to complete
//! - run: Legacy synchronous execution (spawn + wait combined)

use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::{Lazy, OnceCell};
use serde_json::json;
use tauri::Emitter;
use tauri::Manager;
use tokio::sync::{watch, RwLock, Semaphore};
use tokio::task::AbortHandle;

use sentinel_tools::buildin_tools::subagent_tool::{
    set_subagent_spawn_executor, set_subagent_wait_executor, set_subagent_run_executor,
    SubagentSpawnArgs, SubagentSpawnOutput,
    SubagentWaitArgs, SubagentWaitOutput, SubagentTaskResult,
    SubagentRunArgs, SubagentRunOutput,
    SubagentToolError, SubagentStatus, SubagentTaskInfo,
};

use super::{condense_text, execute_agent, ContextPolicy, ToolConfig, ToolSelectionStrategy};
use sentinel_core::models::database::SubagentRun;
use sentinel_core::models::database::SubagentMessage;

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
const MAX_RECURSION_DEPTH: usize = 3;

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

/// Internal task entry with completion channel
struct SubagentTaskEntry {
    info: SubagentTaskInfo,
    /// Sender to notify completion
    completion_tx: watch::Sender<Option<TaskCompletion>>,
    /// Receiver to wait for completion
    completion_rx: watch::Receiver<Option<TaskCompletion>>,
    /// Handle to abort task on cleanup
    abort_handle: Option<AbortHandle>,
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
    let mut contexts = PARENT_CONTEXTS.write().await;
    contexts.remove(execution_id);
    
    // Also cleanup any orphaned tasks for this parent
    let mut tasks = TASK_REGISTRY.write().await;
    tasks.retain(|_, entry| {
        if entry.info.parent_execution_id == execution_id {
            // Abort running task to prevent zombie processes
            if let Some(handle) = &entry.abort_handle {
                handle.abort();
            }
            false
        } else {
            true
        }
    });
    
    // Remove parent semaphore
    let mut parent_sems = PARENT_SEMAPHORES.write().await;
    parent_sems.remove(execution_id);
}

// ============================================================================
// Helper Functions
// ============================================================================

fn default_subagent_tool_config() -> ToolConfig {
    ToolConfig {
        enabled: true,
        selection_strategy: ToolSelectionStrategy::All,
        max_tools: 50,
        fixed_tools: vec![],
        disabled_tools: vec![],
    }
}

fn normalize_tool_config(mut config: ToolConfig, allow_subagents: bool) -> ToolConfig {
    if !allow_subagents {
        // Disable subagent tools if recursion limit reached
        for tool in ["subagent_run", "subagent_spawn", "subagent_wait"] {
            if !config.disabled_tools.contains(&tool.to_string()) {
                config.disabled_tools.push(tool.to_string());
            }
        }
    }
    config
}

/// Build subagent task with parent context reference
/// This function creates a task description that includes a summary of the parent task
/// and instructions on how to access the full parent context via shell tools.
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
    let parent_history_path = format!("{}/history_{}.txt", context_dir, &parent_execution_id[..12.min(parent_execution_id.len())]);
    
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
        brief,
        parent_history_path,
        parent_history_path,
        subagent
    )
}

async fn get_or_create_parent_semaphore(parent_id: &str) -> Arc<Semaphore> {
    let mut sems = PARENT_SEMAPHORES.write().await;
    sems.entry(parent_id.to_string())
        .or_insert_with(|| Arc::new(Semaphore::new(MAX_SUBAGENTS_PER_PARENT)))
        .clone()
}

fn get_app_handle() -> Result<&'static tauri::AppHandle, SubagentToolError> {
    APP_HANDLE.get().ok_or_else(|| {
        SubagentToolError::InternalError("AppHandle not initialized".to_string())
    })
}

async fn get_parent_context(parent_id: &str) -> Result<SubagentParentContext, SubagentToolError> {
    let contexts = PARENT_CONTEXTS.read().await;
    contexts.get(parent_id).cloned().ok_or_else(|| {
        SubagentToolError::ParentContextNotFound(parent_id.to_string())
    })
}

/// Create subagent run record for traceability
async fn create_subagent_run(
    app_handle: &tauri::AppHandle,
    run: &SubagentRun,
) {
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
            // Emit event for real-time update
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

// ============================================================================
// Executor: spawn (non-blocking)
// ============================================================================

async fn execute_spawn(args: SubagentSpawnArgs) -> Result<SubagentSpawnOutput, SubagentToolError> {
    let app_handle = get_app_handle()?;
    
    // Determine execution context (host or docker) for parent history export
    let (context_dir, is_docker) = {
        use sentinel_tools::shell::get_shell_config;
        use sentinel_tools::output_storage::{get_host_context_dir, CONTAINER_CONTEXT_DIR};
        
        let shell_config = get_shell_config().await;
        let docker_available = sentinel_tools::DockerSandbox::is_docker_available().await;
        let docker_enabled = shell_config.default_execution_mode == sentinel_tools::shell::ShellExecutionMode::Docker
            && shell_config.docker_config.is_some()
            && docker_available;
        
        if docker_enabled {
            (CONTAINER_CONTEXT_DIR.to_string(), true)
        } else {
            (get_host_context_dir().display().to_string(), false)
        }
    };
    let parent = get_parent_context(&args.parent_execution_id).await?;
    
    // Export parent's conversation history to file for subagent access
    if let Some(_db) = app_handle.try_state::<Arc<sentinel_db::DatabaseService>>() {
        // Load parent's sliding window to get full conversation history
        use crate::agents::sliding_window::SlidingWindowManager;
        
        if let Ok(parent_sliding_window) = SlidingWindowManager::new(
            &app_handle,
            &args.parent_execution_id,
            None,
        ).await {
            if let Ok(parent_history_content) = parent_sliding_window.export_history().await {
                // Store parent history based on execution environment
                if is_docker {
                    use sentinel_tools::shell::get_shell_config;
                    let shell_config = get_shell_config().await;
                    if let Some(docker_config) = shell_config.docker_config {
                        let sandbox = sentinel_tools::DockerSandbox::new(docker_config);
                        if let Err(e) = sentinel_tools::output_storage::store_history_in_container_with_id(
                            &sandbox,
                            &parent_history_content,
                            Some(&args.parent_execution_id),
                        ).await {
                            tracing::warn!("Failed to export parent history to container: {}", e);
                        } else {
                            tracing::info!("Parent history exported to container for subagent access");
                        }
                    }
                } else {
                    if let Err(e) = sentinel_tools::output_storage::store_history_on_host(
                        &parent_history_content,
                        Some(&args.parent_execution_id),
                    ).await {
                        tracing::warn!("Failed to export parent history to host: {}", e);
                    } else {
                        tracing::info!("Parent history exported to host for subagent access");
                    }
                }
            }
        }
    }
    
    // Build subagent task with parent context reference
    let task_with_context = build_subagent_task(
        &parent.task_context, 
        &args.task, 
        &args.parent_execution_id,
        &context_dir,
    );
    
    // Check recursion depth
    let new_recursion_depth = parent.recursion_depth + 1;
    let allow_subagents = new_recursion_depth < MAX_RECURSION_DEPTH;

    // Generate task ID first so we can use it for queued event
    let task_id = uuid::Uuid::new_v4().to_string();

    if !args.inherit_parent_llm {
        return Err(SubagentToolError::InvalidArguments(
            "Custom LLM config is not supported yet".to_string(),
        ));
    }
    
    // Emit queued event
    let _ = app_handle.emit("subagent:queued", &json!({
        "task_id": task_id,
        "execution_id": task_id,
        "parent_execution_id": args.parent_execution_id,
    }));

    // Try to acquire semaphores (waiting if necessary)
    let global_permit = GLOBAL_SEMAPHORE.clone().acquire_owned().await
        .map_err(|_| SubagentToolError::ConcurrencyLimitReached)?;
    
    let parent_sem = get_or_create_parent_semaphore(&args.parent_execution_id).await;
    let parent_permit = parent_sem.clone().acquire_owned().await
        .map_err(|_| SubagentToolError::ConcurrencyLimitReached)?;
    
    let now = chrono::Utc::now().timestamp();
    
    // Create completion channel
    let (tx, rx) = watch::channel(None);
    
    // Create task entry info
    let task_info = SubagentTaskInfo {
        task_id: task_id.clone(),
        parent_execution_id: args.parent_execution_id.clone(),
        role: args.role.clone(),
        task: task_with_context.clone(),
        status: SubagentStatus::Pending,
        output: None,
        error: None,
        started_at: now,
        completed_at: None,
    };

    let now_dt = chrono::Utc::now();
    let run_record = SubagentRun {
        id: task_id.clone(),
        parent_execution_id: args.parent_execution_id.clone(),
        role: args.role.clone(),
        task: task_with_context.clone(),
        status: "running".to_string(),
        output: None,
        error: None,
        model_name: Some(parent.model.clone()),
        model_provider: Some(parent.rig_provider.clone()),
        started_at: now_dt,
        completed_at: None,
        created_at: now_dt,
        updated_at: now_dt,
    };
    create_subagent_run(&app_handle, &run_record).await;

    create_subagent_message(
        &app_handle,
        &task_id,
        "user",
        &task_with_context,
    )
    .await;
    
    // Spawn background task ensuring we have an abort handle
    let task_id_clone = task_id.clone();
    let app_clone = app_handle.clone();
    let args_task = task_with_context.clone();
    let parent_execution_id = args.parent_execution_id.clone();
    
    // Build tool config eagerly to pass into closure (avoids clone issues)
    let tool_config = if let Some(raw) = args.tool_config {
        match serde_json::from_value::<ToolConfig>(raw) {
            Ok(parsed) => normalize_tool_config(parsed, allow_subagents),
            Err(e) => {
                tracing::error!("Invalid tool_config: {}", e);
                default_subagent_tool_config()
            }
        }
    } else if args.inherit_parent_tools {
        normalize_tool_config(parent.tool_config.clone(), allow_subagents)
    } else {
        default_subagent_tool_config()
    };
    
    let system_prompt = args.system_prompt.unwrap_or_else(|| parent.system_prompt.clone());
    let max_iterations = args.max_iterations.unwrap_or(parent.max_iterations.max(200));
    let timeout_secs = args.timeout_secs.unwrap_or(parent.timeout_secs);
    
    let task_handle = tokio::spawn(async move {
        // Keep permits alive until task completes
        let _global = global_permit;
        let _parent = parent_permit;
        
        // Update status to running
        {
            let mut tasks = TASK_REGISTRY.write().await;
            if let Some(entry) = tasks.get_mut(&task_id_clone) {
                entry.info.status = SubagentStatus::Running;
            }
        }
        
        let params = super::AgentExecuteParams {
            execution_id: task_id_clone.clone(),
            model: parent.model,
            system_prompt,
            task: args_task,
            rig_provider: parent.rig_provider,
            api_key: parent.api_key,
            api_base: parent.api_base,
            max_iterations,
            timeout_secs,
            tool_config: Some(tool_config),
            enable_tenth_man_rule: false,
            tenth_man_config: None,
            document_attachments: None,
            image_attachments: None,
            persist_messages: false,
            subagent_run_id: Some(task_id_clone.clone()),
            context_policy: Some(ContextPolicy::subagent()),
            recursion_depth: new_recursion_depth,
        };
        
        // Execute agent
        let result = execute_agent(&app_clone, params).await;
        let now = chrono::Utc::now().timestamp();
        
        // Update task registry and notify waiters
        let completion = match result {
            Ok(output) => {
                let completed_at = chrono::Utc::now();
                let _ = app_clone.emit("subagent:done", &json!({
                    "task_id": task_id_clone,
                    "execution_id": task_id_clone,
                    "parent_execution_id": parent_execution_id,
                    "success": true,
                    "output": output,
                }));

                update_subagent_run_result(
                    &app_clone,
                    &task_id_clone,
                    "completed",
                    Some(&output),
                    None,
                    Some(completed_at),
                )
                .await;
                
                TaskCompletion {
                    success: true,
                    output: Some(output),
                    error: None,
                }
            }
            Err(e) => {
                let error = e.to_string();
                let completed_at = chrono::Utc::now();
                let _ = app_clone.emit("subagent:error", &json!({
                    "task_id": task_id_clone,
                    "execution_id": task_id_clone,
                    "parent_execution_id": parent_execution_id,
                    "error": error,
                }));

                update_subagent_run_result(
                    &app_clone,
                    &task_id_clone,
                    "failed",
                    None,
                    Some(&error),
                    Some(completed_at),
                )
                .await;
                
                TaskCompletion {
                    success: false,
                    output: None,
                    error: Some(error),
                }
            }
        };
        
        // Update registry
        {
            let mut tasks = TASK_REGISTRY.write().await;
            if let Some(entry) = tasks.get_mut(&task_id_clone) {
                entry.info.status = if completion.success {
                    SubagentStatus::Completed
                } else {
                    SubagentStatus::Failed
                };
                entry.info.output = completion.output.clone();
                entry.info.error = completion.error.clone();
                entry.info.completed_at = Some(now);
                
                // Notify waiters
                let _ = entry.completion_tx.send(Some(completion));
            }
        }
    });
    
    // Register task with abort handle
    let abort_handle = task_handle.abort_handle();
    {
        let mut tasks = TASK_REGISTRY.write().await;
        tasks.insert(task_id.clone(), SubagentTaskEntry {
            info: task_info,
            completion_tx: tx,
            completion_rx: rx,
            abort_handle: Some(abort_handle),
        });
    }

    // Emit start event
    let _ = app_handle.emit("subagent:start", &json!({
        "task_id": task_id,
        "execution_id": task_id,
        "parent_execution_id": args.parent_execution_id,
        "role": args.role,
        "task": task_with_context,
        "mode": "async",
    }));
    
    Ok(SubagentSpawnOutput {
        task_id,
        message: "Subagent task spawned. Use subagent_wait to get results.".to_string(),
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
        // Get receiver for this task
        let rx = {
            let tasks = TASK_REGISTRY.read().await;
            match tasks.get(task_id) {
                Some(entry) => entry.completion_rx.clone(),
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
        
        // Wait for completion with timeout
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            results.push(SubagentTaskResult {
                task_id: task_id.clone(),
                role: None,
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
        }).await;
        
        // Get task info for role
        let role = {
            let tasks = TASK_REGISTRY.read().await;
            tasks.get(task_id).map(|e| e.info.role.clone()).flatten()
        };
        
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
// Executor: run (legacy, blocking)
// ============================================================================

async fn execute_run(args: SubagentRunArgs) -> Result<SubagentRunOutput, SubagentToolError> {
    // 1. Spawn the task
    let spawn_args = SubagentSpawnArgs {
        parent_execution_id: args.parent_execution_id.clone(),
        role: args.role,
        task: args.task,
        inherit_parent_llm: args.inherit_parent_llm,
        inherit_parent_tools: args.inherit_parent_tools,
        tool_config: args.tool_config,
        system_prompt: args.system_prompt,
        max_iterations: args.max_iterations,
        timeout_secs: args.timeout_secs,
    };
    
    let spawn_output = execute_spawn(spawn_args).await?;
    let task_id = spawn_output.task_id;
    
    // 2. Wait for completion
    let wait_args = SubagentWaitArgs {
        parent_execution_id: args.parent_execution_id.clone(),
        task_ids: vec![task_id.clone()],
        timeout_secs: args.timeout_secs.unwrap_or(300), // Default 5 mins wait
    };
    
    let wait_output = execute_wait(wait_args).await?;
    
    // 3. Process result
    if let Some(result) = wait_output.results.first() {
        if result.success {
            Ok(SubagentRunOutput {
                success: true,
                execution_id: task_id,
                output: result.output.clone(),
                error: None,
            })
        } else {
            let err_msg = result.error.clone().unwrap_or_else(|| "Unknown error".to_string());
            Err(SubagentToolError::ExecutionFailed(err_msg))
        }
    } else {
        Err(SubagentToolError::ExecutionFailed("Task lost during wait".to_string()))
    }
}

// ============================================================================
// Initialization
// ============================================================================

pub fn init_subagent_executor() {
    // Register spawn executor
    let spawn_executor = std::sync::Arc::new(|args: SubagentSpawnArgs| {
        Box::pin(execute_spawn(args))
            as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>
    });
    set_subagent_spawn_executor(spawn_executor);
    
    // Register wait executor
    let wait_executor = std::sync::Arc::new(|args: SubagentWaitArgs| {
        Box::pin(execute_wait(args))
            as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>
    });
    set_subagent_wait_executor(wait_executor);
    
    // Register run executor (legacy)
    let run_executor = std::sync::Arc::new(|args: SubagentRunArgs| {
        Box::pin(execute_run(args))
            as std::pin::Pin<Box<dyn std::future::Future<Output = _> + Send>>
    });
    set_subagent_run_executor(run_executor);
    
    tracing::info!("Subagent executors initialized (spawn/wait/run)");
}

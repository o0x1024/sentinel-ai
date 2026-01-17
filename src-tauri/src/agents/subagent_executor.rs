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

use sentinel_tools::buildin_tools::subagent_tool::{
    set_subagent_spawn_executor, set_subagent_wait_executor, set_subagent_run_executor,
    SubagentSpawnArgs, SubagentSpawnOutput,
    SubagentWaitArgs, SubagentWaitOutput, SubagentTaskResult,
    SubagentRunArgs, SubagentRunOutput,
    SubagentToolError, SubagentStatus, SubagentTaskInfo,
};

use super::{execute_agent, ToolConfig, ToolSelectionStrategy};
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
}

/// Internal task entry with completion channel
struct SubagentTaskEntry {
    info: SubagentTaskInfo,
    /// Sender to notify completion
    completion_tx: watch::Sender<Option<TaskCompletion>>,
    /// Receiver to wait for completion
    completion_rx: watch::Receiver<Option<TaskCompletion>>,
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
    tasks.retain(|_, entry| entry.info.parent_execution_id != execution_id);
    
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
        // Disable all subagent tools in subagent to prevent recursion
        disabled_tools: vec![
            "subagent_run".to_string(),
            "subagent_spawn".to_string(),
            "subagent_wait".to_string(),
        ],
    }
}

fn normalize_tool_config(mut config: ToolConfig) -> ToolConfig {
    // Always disable subagent tools in subagent
    for tool in ["subagent_run", "subagent_spawn", "subagent_wait"] {
        if !config.disabled_tools.contains(&tool.to_string()) {
            config.disabled_tools.push(tool.to_string());
        }
    }
    config
}

fn build_subagent_task(parent_task: &str, subagent_task: &str) -> String {
    let parent = parent_task.trim();
    let subagent = subagent_task.trim();
    if parent.is_empty() {
        return subagent.to_string();
    }
    if subagent.is_empty() {
        return parent.to_string();
    }
    format!(
        "Parent task context:\n{}\n\nSubagent task:\n{}",
        parent,
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
        }
    }
}

// ============================================================================
// Executor: spawn (non-blocking)
// ============================================================================

async fn execute_spawn(args: SubagentSpawnArgs) -> Result<SubagentSpawnOutput, SubagentToolError> {
    let app_handle = get_app_handle()?;
    let parent = get_parent_context(&args.parent_execution_id).await?;
    let task_with_context = build_subagent_task(&parent.task_context, &args.task);
    
    if !args.inherit_parent_llm {
        return Err(SubagentToolError::InvalidArguments(
            "Custom LLM config is not supported yet".to_string(),
        ));
    }
    
    // Try to acquire semaphores (non-blocking check)
    let global_permit = GLOBAL_SEMAPHORE.clone().try_acquire_owned()
        .map_err(|_| SubagentToolError::ConcurrencyLimitReached)?;
    
    let parent_sem = get_or_create_parent_semaphore(&args.parent_execution_id).await;
    let parent_permit = parent_sem.clone().try_acquire_owned()
        .map_err(|_| SubagentToolError::ConcurrencyLimitReached)?;
    
    // Generate task ID
    let task_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();
    
    // Create completion channel
    let (tx, rx) = watch::channel(None);
    
    // Create task entry
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
    
    // Register task
    {
        let mut tasks = TASK_REGISTRY.write().await;
        tasks.insert(task_id.clone(), SubagentTaskEntry {
            info: task_info.clone(),
            completion_tx: tx,
            completion_rx: rx,
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
    
    // Spawn background task
    let task_id_clone = task_id.clone();
    let app_clone = app_handle.clone();
    
    // Clone values needed for the spawned task
    let args_task = task_with_context.clone();
    
    tokio::spawn(async move {
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
        
        // Build tool config
        let tool_config = if let Some(raw) = args.tool_config {
            match serde_json::from_value::<ToolConfig>(raw) {
                Ok(parsed) => normalize_tool_config(parsed),
                Err(e) => {
                    tracing::error!("Invalid tool_config: {}", e);
                    default_subagent_tool_config()
                }
            }
        } else if args.inherit_parent_tools {
            normalize_tool_config(parent.tool_config.clone())
        } else {
            default_subagent_tool_config()
        };
        
        let system_prompt = args.system_prompt.unwrap_or_else(|| parent.system_prompt.clone());
        let max_iterations = args.max_iterations.unwrap_or(6);
        let timeout_secs = args.timeout_secs.unwrap_or(parent.timeout_secs);
        
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
                    "parent_execution_id": args.parent_execution_id,
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
                    "parent_execution_id": args.parent_execution_id,
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
    let app_handle = get_app_handle()?;
    let parent = get_parent_context(&args.parent_execution_id).await?;
    let task_with_context = build_subagent_task(&parent.task_context, &args.task);
    
    if !args.inherit_parent_llm {
        return Err(SubagentToolError::InvalidArguments(
            "Custom LLM config is not supported yet".to_string(),
        ));
    }
    
    // Build tool config
    let tool_config = if let Some(raw) = args.tool_config {
        let parsed: ToolConfig = serde_json::from_value(raw)
            .map_err(|e| SubagentToolError::InvalidArguments(format!("Invalid tool_config: {}", e)))?;
        normalize_tool_config(parsed)
    } else if args.inherit_parent_tools {
        normalize_tool_config(parent.tool_config.clone())
    } else {
        default_subagent_tool_config()
    };
    
    let execution_id = uuid::Uuid::new_v4().to_string();
    let system_prompt = args.system_prompt.unwrap_or_else(|| parent.system_prompt.clone());
    let max_iterations = args.max_iterations.unwrap_or(6);
    let timeout_secs = args.timeout_secs.unwrap_or(parent.timeout_secs);
    
    let now_dt = chrono::Utc::now();
    let run_record = SubagentRun {
        id: execution_id.clone(),
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
    create_subagent_run(app_handle, &run_record).await;

    create_subagent_message(
        app_handle,
        &execution_id,
        "user",
        &task_with_context,
    )
    .await;
    
    // Emit start event
    let _ = app_handle.emit("subagent:start", &json!({
        "task_id": execution_id,
        "execution_id": execution_id,
        "parent_execution_id": args.parent_execution_id,
        "role": args.role,
        "task": task_with_context,
        "mode": "sync",
    }));
    
    let params = super::AgentExecuteParams {
        execution_id: execution_id.clone(),
        model: parent.model,
        system_prompt,
        task: task_with_context,
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
        subagent_run_id: Some(execution_id.clone()),
    };
    
    match execute_agent(app_handle, params).await {
        Ok(response) => {
            let completed_at = chrono::Utc::now();
            let _ = app_handle.emit("subagent:done", &json!({
                "task_id": execution_id,
                "execution_id": execution_id,
                "parent_execution_id": args.parent_execution_id,
                "success": true,
                "output": response,
            }));
            update_subagent_run_result(
                app_handle,
                &execution_id,
                "completed",
                Some(&response),
                None,
                Some(completed_at),
            )
            .await;
            Ok(SubagentRunOutput {
                success: true,
                execution_id,
                output: Some(response),
                error: None,
            })
        }
        Err(e) => {
            let error = e.to_string();
            let completed_at = chrono::Utc::now();
            let _ = app_handle.emit("subagent:error", &json!({
                "task_id": execution_id,
                "execution_id": execution_id,
                "parent_execution_id": args.parent_execution_id,
                "error": error,
            }));
            update_subagent_run_result(
                app_handle,
                &execution_id,
                "failed",
                None,
                Some(&error),
                Some(completed_at),
            )
            .await;
            Err(SubagentToolError::ExecutionFailed(error))
        }
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

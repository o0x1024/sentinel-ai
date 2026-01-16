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
use sentinel_core::models::database::AiConversation;
use sentinel_db::Database;

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

/// Create a temporary conversation record for subagent to satisfy FK constraint
async fn create_subagent_conversation(
    app_handle: &tauri::AppHandle,
    execution_id: &str,
    parent_execution_id: &str,
    role: Option<&str>,
    task: &str,
    model: &str,
    rig_provider: &str,
) -> Result<(), SubagentToolError> {
    let db = app_handle
        .try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
        .map(|s| s.inner().clone());
    
    if let Some(db) = db {
        let now = chrono::Utc::now();
        // Truncate task at char boundary to avoid panic with UTF-8 strings
        let task_preview = if task.chars().count() > 50 {
            task.chars().take(50).collect::<String>()
        } else {
            task.to_string()
        };
        let title = format!(
            "[Subagent{}] {}",
            role.map(|r| format!(": {}", r)).unwrap_or_default(),
            task_preview
        );
        
        let conversation = AiConversation {
            id: execution_id.to_string(),
            title: Some(title),
            service_name: "subagent".to_string(),
            model_name: model.to_string(),
            model_provider: Some(rig_provider.to_string()),
            context_type: Some("subagent".to_string()),
            project_id: None,
            vulnerability_id: None,
            scan_task_id: None,
            conversation_data: Some(json!({
                "parent_execution_id": parent_execution_id,
                "role": role,
                "task": task,
            }).to_string()),
            summary: None,
            total_messages: 0,
            total_tokens: 0,
            cost: 0.0,
            tags: Some(serde_json::to_string(&vec!["subagent"]).unwrap_or_default()),
            tool_config: None,
            is_archived: false,
            created_at: now,
            updated_at: now,
        };
        
        if let Err(e) = db.create_ai_conversation(&conversation).await {
            tracing::warn!("Failed to create subagent conversation record: {}", e);
            // Don't fail the subagent execution, just log the warning
        } else {
            tracing::debug!("Created subagent conversation: {}", execution_id);
        }
    }
    
    Ok(())
}

/// Delete the temporary conversation record after subagent completes
async fn cleanup_subagent_conversation(app_handle: &tauri::AppHandle, execution_id: &str) {
    let db = app_handle
        .try_state::<std::sync::Arc<sentinel_db::DatabaseService>>()
        .map(|s| s.inner().clone());
    
    if let Some(db) = db {
        // Delete messages first (due to FK constraint)
        if let Err(e) = db.delete_ai_messages_by_conversation(execution_id).await {
            tracing::warn!("Failed to delete subagent messages: {}", e);
        }
        // Then delete conversation
        if let Err(e) = db.delete_ai_conversation(execution_id).await {
            tracing::warn!("Failed to delete subagent conversation: {}", e);
        } else {
            tracing::debug!("Cleaned up subagent conversation: {}", execution_id);
        }
    }
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
        task: args.task.clone(),
        status: SubagentStatus::Pending,
        output: None,
        error: None,
        started_at: now,
        completed_at: None,
    };
    
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
        "task": args.task,
        "mode": "async",
    }));
    
    // Spawn background task
    let task_id_clone = task_id.clone();
    let app_clone = app_handle.clone();
    
    // Clone values needed for the spawned task
    let parent_model = parent.model.clone();
    let parent_rig_provider = parent.rig_provider.clone();
    let args_role = args.role.clone();
    let args_task = args.task.clone();
    let args_parent_id = args.parent_execution_id.clone();
    
    tokio::spawn(async move {
        // Keep permits alive until task completes
        let _global = global_permit;
        let _parent = parent_permit;
        
        // Create temporary conversation record for FK constraint
        let _ = create_subagent_conversation(
            &app_clone,
            &task_id_clone,
            &args_parent_id,
            args_role.as_deref(),
            &args_task,
            &parent_model,
            &parent_rig_provider,
        ).await;
        
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
            task: args.task,
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
        };
        
        // Execute agent
        let result = execute_agent(&app_clone, params).await;
        let now = chrono::Utc::now().timestamp();
        
        // Update task registry and notify waiters
        let completion = match result {
            Ok(output) => {
                let _ = app_clone.emit("subagent:done", &json!({
                    "task_id": task_id_clone,
                    "execution_id": task_id_clone,
                    "parent_execution_id": args.parent_execution_id,
                    "success": true,
                    "output": output,
                }));
                
                TaskCompletion {
                    success: true,
                    output: Some(output),
                    error: None,
                }
            }
            Err(e) => {
                let error = e.to_string();
                let _ = app_clone.emit("subagent:error", &json!({
                    "task_id": task_id_clone,
                    "execution_id": task_id_clone,
                    "parent_execution_id": args.parent_execution_id,
                    "error": error,
                }));
                
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
    
    // Create temporary conversation record for FK constraint
    let _ = create_subagent_conversation(
        app_handle,
        &execution_id,
        &args.parent_execution_id,
        args.role.as_deref(),
        &args.task,
        &parent.model,
        &parent.rig_provider,
    ).await;
    
    // Emit start event
    let _ = app_handle.emit("subagent:start", &json!({
        "task_id": execution_id,
        "execution_id": execution_id,
        "parent_execution_id": args.parent_execution_id,
        "role": args.role,
        "task": args.task,
        "mode": "sync",
    }));
    
    let params = super::AgentExecuteParams {
        execution_id: execution_id.clone(),
        model: parent.model,
        system_prompt,
        task: args.task.clone(),
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
    };
    
    match execute_agent(app_handle, params).await {
        Ok(response) => {
            let _ = app_handle.emit("subagent:done", &json!({
                "task_id": execution_id,
                "execution_id": execution_id,
                "parent_execution_id": args.parent_execution_id,
                "success": true,
                "output": response,
            }));
            Ok(SubagentRunOutput {
                success: true,
                execution_id,
                output: Some(response),
                error: None,
            })
        }
        Err(e) => {
            let error = e.to_string();
            let _ = app_handle.emit("subagent:error", &json!({
                "task_id": execution_id,
                "execution_id": execution_id,
                "parent_execution_id": args.parent_execution_id,
                "error": error,
            }));
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

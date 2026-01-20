//! Subagent Tools - LLM-callable tools to spawn and manage subagents
//!
//! Provides three tools for flexible subagent execution:
//! - subagent_spawn: Start a subagent asynchronously (non-blocking)
//! - subagent_wait: Wait for one or more subagents to complete (blocking)
//! - subagent_run: Legacy synchronous execution (blocking, for simple cases)

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Global default timeout for subagent tasks (in seconds)
static DEFAULT_SUBAGENT_TIMEOUT_SECS: AtomicU64 = AtomicU64::new(600); // 10 minutes

/// Set the default timeout for subagent tasks
pub fn set_default_subagent_timeout(timeout_secs: u64) {
    DEFAULT_SUBAGENT_TIMEOUT_SECS.store(timeout_secs, Ordering::SeqCst);
}

/// Get the default timeout for subagent tasks
pub fn get_default_subagent_timeout() -> u64 {
    DEFAULT_SUBAGENT_TIMEOUT_SECS.load(Ordering::SeqCst)
}

// ============================================================================
// Type Definitions
// ============================================================================

/// Subagent task status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubagentStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Subagent task info stored in task manager
#[derive(Debug, Clone, Serialize)]
pub struct SubagentTaskInfo {
    pub task_id: String,
    pub parent_execution_id: String,
    pub role: Option<String>,
    pub task: String,
    pub status: SubagentStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

/// Subagent tool errors
#[derive(Debug, thiserror::Error)]
pub enum SubagentToolError {
    #[error("Parent context not found for execution {0}")]
    ParentContextNotFound(String),
    #[error("Subagent execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    #[error("Concurrency limit reached")]
    ConcurrencyLimitReached,
    #[error("Timeout waiting for tasks")]
    Timeout,
}

// ============================================================================
// Executor Function Types
// ============================================================================

/// Type alias for spawn executor (non-blocking, returns task_id)
pub type SubagentSpawnExecutorFn = std::sync::Arc<
    dyn Fn(SubagentSpawnArgs)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SubagentSpawnOutput, SubagentToolError>> + Send>>
        + Send
        + Sync,
>;

/// Type alias for wait executor (blocking, waits for completion)
pub type SubagentWaitExecutorFn = std::sync::Arc<
    dyn Fn(SubagentWaitArgs)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SubagentWaitOutput, SubagentToolError>> + Send>>
        + Send
        + Sync,
>;

/// Type alias for legacy run executor (blocking, full execution)
pub type SubagentRunExecutorFn = std::sync::Arc<
    dyn Fn(SubagentRunArgs)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SubagentRunOutput, SubagentToolError>> + Send>>
        + Send
        + Sync,
>;

// ============================================================================
// Global Executor Storage
// ============================================================================

static SUBAGENT_SPAWN_EXECUTOR: once_cell::sync::OnceCell<SubagentSpawnExecutorFn> = once_cell::sync::OnceCell::new();
static SUBAGENT_WAIT_EXECUTOR: once_cell::sync::OnceCell<SubagentWaitExecutorFn> = once_cell::sync::OnceCell::new();
static SUBAGENT_RUN_EXECUTOR: once_cell::sync::OnceCell<SubagentRunExecutorFn> = once_cell::sync::OnceCell::new();

pub fn set_subagent_spawn_executor(executor: SubagentSpawnExecutorFn) {
    let _ = SUBAGENT_SPAWN_EXECUTOR.set(executor);
}

pub fn set_subagent_wait_executor(executor: SubagentWaitExecutorFn) {
    let _ = SUBAGENT_WAIT_EXECUTOR.set(executor);
}

pub fn set_subagent_run_executor(executor: SubagentRunExecutorFn) {
    let _ = SUBAGENT_RUN_EXECUTOR.set(executor);
}

// Legacy compatibility
pub type SubagentExecutorFn = SubagentRunExecutorFn;
pub fn set_subagent_executor(executor: SubagentExecutorFn) {
    set_subagent_run_executor(executor);
}

fn get_spawn_executor() -> Result<&'static SubagentSpawnExecutorFn, SubagentToolError> {
    SUBAGENT_SPAWN_EXECUTOR.get().ok_or_else(|| {
        SubagentToolError::InternalError("Subagent spawn executor not initialized".to_string())
    })
}

fn get_wait_executor() -> Result<&'static SubagentWaitExecutorFn, SubagentToolError> {
    SUBAGENT_WAIT_EXECUTOR.get().ok_or_else(|| {
        SubagentToolError::InternalError("Subagent wait executor not initialized".to_string())
    })
}

fn get_run_executor() -> Result<&'static SubagentRunExecutorFn, SubagentToolError> {
    SUBAGENT_RUN_EXECUTOR.get().ok_or_else(|| {
        SubagentToolError::InternalError("Subagent run executor not initialized".to_string())
    })
}

// ============================================================================
// Tool 1: subagent_spawn (Non-blocking)
// ============================================================================

/// Arguments for spawning a subagent
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubagentSpawnArgs {
    /// Parent execution id (required)
    pub parent_execution_id: String,
    /// Task to execute in subagent
    pub task: String,
    /// Optional role name for display (e.g., "Scanner", "Analyzer")
    #[serde(default)]
    pub role: Option<String>,
    /// Optional system prompt override
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Optional tool config override (raw JSON)
    #[serde(default)]
    pub tool_config: Option<serde_json::Value>,
    /// Max iterations for subagent (default: 6)
    #[serde(default)]
    pub max_iterations: Option<usize>,
    /// Timeout seconds for subagent
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    /// Inherit LLM config from parent (default: true)
    #[serde(default = "default_true")]
    pub inherit_parent_llm: bool,
    /// Inherit tool config from parent (default: false)
    #[serde(default)]
    pub inherit_parent_tools: bool,
}

fn default_true() -> bool { true }

/// Output from spawning a subagent
#[derive(Debug, Clone, Serialize)]
pub struct SubagentSpawnOutput {
    /// Unique task ID for tracking
    pub task_id: String,
    /// Status message
    pub message: String,
}

/// Subagent Spawn Tool - starts a subagent without waiting
#[derive(Debug, Clone, Default)]
pub struct SubagentSpawnTool;

impl SubagentSpawnTool {
    pub fn new() -> Self { Self }
    pub const NAME: &'static str = "subagent_spawn";
    pub const DESCRIPTION: &'static str = "Start a subagent task asynchronously (NON-BLOCKING). Returns immediately with a task_id. Use this to launch multiple parallel tasks, then use subagent_wait to collect results. Ideal for: 1) Parallel scanning of multiple targets, 2) Concurrent analysis of independent files, 3) Fan-out tasks that don't depend on each other. After spawning, you MUST call subagent_wait to get results before using them.";
}

impl Tool for SubagentSpawnTool {
    const NAME: &'static str = Self::NAME;
    type Args = SubagentSpawnArgs;
    type Output = SubagentSpawnOutput;
    type Error = SubagentToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SubagentSpawnArgs)).unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!(
            "Subagent spawn requested - parent: {}, role: {:?}, task_preview: {}",
            args.parent_execution_id,
            args.role,
            args.task.chars().take(50).collect::<String>()
        );
        let executor = get_spawn_executor()?;
        executor(args).await
    }
}

// ============================================================================
// Tool 2: subagent_wait (Blocking)
// ============================================================================

/// Arguments for waiting on subagent tasks
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubagentWaitArgs {
    /// Parent execution id
    pub parent_execution_id: String,
    /// Task IDs to wait for (from subagent_spawn)
    pub task_ids: Vec<String>,
    /// Timeout in seconds (default: 300)
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 { get_default_subagent_timeout() }

/// Single task result
#[derive(Debug, Clone, Serialize)]
pub struct SubagentTaskResult {
    pub task_id: String,
    pub role: Option<String>,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// Output from waiting on subagents
#[derive(Debug, Clone, Serialize)]
pub struct SubagentWaitOutput {
    /// Results for all waited tasks
    pub results: Vec<SubagentTaskResult>,
    /// Summary message
    pub summary: String,
}

/// Subagent Wait Tool - waits for spawned tasks to complete
#[derive(Debug, Clone, Default)]
pub struct SubagentWaitTool;

impl SubagentWaitTool {
    pub fn new() -> Self { Self }
    pub const NAME: &'static str = "subagent_wait";
    pub const DESCRIPTION: &'static str = "Wait for one or more spawned subagent tasks to complete (BLOCKING). Provide task_ids from subagent_spawn. Returns all results once all tasks finish or timeout. Use after subagent_spawn to collect results. Supports waiting for multiple tasks simultaneously - they run in parallel while you wait.";
}

impl Tool for SubagentWaitTool {
    const NAME: &'static str = Self::NAME;
    type Args = SubagentWaitArgs;
    type Output = SubagentWaitOutput;
    type Error = SubagentToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SubagentWaitArgs)).unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!(
            "Subagent wait requested - parent: {}, task_ids: {:?}, timeout: {}s",
            args.parent_execution_id,
            args.task_ids,
            args.timeout_secs
        );
        let executor = get_wait_executor()?;
        executor(args).await
    }
}

// ============================================================================
// Tool 3: subagent_run (Legacy, Blocking)
// ============================================================================

/// Arguments for running a subagent synchronously (legacy)
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubagentRunArgs {
    /// Parent execution id
    pub parent_execution_id: String,
    /// Task to execute in subagent
    pub task: String,
    /// Optional role name for display
    #[serde(default)]
    pub role: Option<String>,
    /// Optional system prompt override
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Optional tool config override (raw JSON)
    #[serde(default)]
    pub tool_config: Option<serde_json::Value>,
    /// Max iterations for subagent
    #[serde(default)]
    pub max_iterations: Option<usize>,
    /// Timeout seconds for subagent
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    /// Inherit LLM config from parent (default: true)
    #[serde(default = "default_true")]
    pub inherit_parent_llm: bool,
    /// Inherit tool config from parent (default: false)
    #[serde(default)]
    pub inherit_parent_tools: bool,
}

/// Output from running a subagent synchronously
#[derive(Debug, Clone, Serialize)]
pub struct SubagentRunOutput {
    pub success: bool,
    pub execution_id: String,
    pub output: Option<String>,
    pub error: Option<String>,
}

// Legacy type aliases for backward compatibility
pub type SubagentToolArgs = SubagentRunArgs;
pub type SubagentToolOutput = SubagentRunOutput;

/// Subagent Run Tool - synchronous execution (legacy)
#[derive(Debug, Clone, Default)]
pub struct SubagentRunTool;

impl SubagentRunTool {
    pub fn new() -> Self { Self }
    pub const NAME: &'static str = "subagent_run";
    pub const DESCRIPTION: &'static str = "Execute a subagent task synchronously (BLOCKING). Waits for the subagent to complete before returning. Use for: 1) Sequential dependent tasks where you need result A before starting task B, 2) Simple single-task delegation. For parallel execution, use subagent_spawn + subagent_wait instead.";
}

impl Tool for SubagentRunTool {
    const NAME: &'static str = Self::NAME;
    type Args = SubagentRunArgs;
    type Output = SubagentRunOutput;
    type Error = SubagentToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SubagentRunArgs)).unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!(
            "Subagent run requested - parent: {}, role: {:?}, inherit_llm: {}, inherit_tools: {}",
            args.parent_execution_id,
            args.role,
            args.inherit_parent_llm,
            args.inherit_parent_tools
        );
        let executor = get_run_executor()?;
        executor(args).await
    }
}

// ============================================================================
// Legacy Compatibility: SubagentTool alias
// ============================================================================

pub type SubagentTool = SubagentRunTool;

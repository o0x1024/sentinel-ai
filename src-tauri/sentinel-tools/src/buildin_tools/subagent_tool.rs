//! Subagent Tools - LLM-callable tools to spawn and manage subagents
//!
//! Provides tools for subagent orchestration and collaboration:
//! - subagent_spawn: Start a subagent asynchronously (non-blocking)
//! - subagent_wait: Wait for one or more subagents to complete (blocking)
//! - subagent_run: Legacy synchronous execution (blocking, for simple cases)
//! - subagent_state_put/get: Shared state exchange across subagents
//! - subagent_event_publish/poll: Lightweight event bus across subagents

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Global default timeout for subagent tasks (in seconds)
static DEFAULT_SUBAGENT_TIMEOUT_SECS: AtomicU64 = AtomicU64::new(600); // 10 minutes

/// Default max iterations for subagent tasks
pub const DEFAULT_SUBAGENT_MAX_ITERATIONS: usize = 50;

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
    pub depends_on_task_ids: Vec<String>,
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

/// Type alias for wait-any executor (blocking until any task completes)
pub type SubagentWaitAnyExecutorFn = std::sync::Arc<
    dyn Fn(SubagentWaitAnyArgs)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SubagentWaitAnyOutput, SubagentToolError>> + Send>>
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

pub type SubagentStatePutExecutorFn = std::sync::Arc<
    dyn Fn(SubagentStatePutArgs)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SubagentStatePutOutput, SubagentToolError>> + Send>>
        + Send
        + Sync,
>;

pub type SubagentStateGetExecutorFn = std::sync::Arc<
    dyn Fn(SubagentStateGetArgs)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SubagentStateGetOutput, SubagentToolError>> + Send>>
        + Send
        + Sync,
>;

pub type SubagentEventPublishExecutorFn = std::sync::Arc<
    dyn Fn(SubagentEventPublishArgs)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SubagentEventPublishOutput, SubagentToolError>> + Send>>
        + Send
        + Sync,
>;

pub type SubagentEventPollExecutorFn = std::sync::Arc<
    dyn Fn(SubagentEventPollArgs)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SubagentEventPollOutput, SubagentToolError>> + Send>>
        + Send
        + Sync,
>;

pub type SubagentWorkflowRunExecutorFn = std::sync::Arc<
    dyn Fn(SubagentWorkflowRunArgs)
        -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<SubagentWorkflowRunOutput, SubagentToolError>> + Send>>
        + Send
        + Sync,
>;

// ============================================================================
// Global Executor Storage
// ============================================================================

static SUBAGENT_SPAWN_EXECUTOR: once_cell::sync::OnceCell<SubagentSpawnExecutorFn> = once_cell::sync::OnceCell::new();
static SUBAGENT_WAIT_EXECUTOR: once_cell::sync::OnceCell<SubagentWaitExecutorFn> = once_cell::sync::OnceCell::new();
static SUBAGENT_WAIT_ANY_EXECUTOR: once_cell::sync::OnceCell<SubagentWaitAnyExecutorFn> = once_cell::sync::OnceCell::new();
static SUBAGENT_RUN_EXECUTOR: once_cell::sync::OnceCell<SubagentRunExecutorFn> = once_cell::sync::OnceCell::new();
static SUBAGENT_STATE_PUT_EXECUTOR: once_cell::sync::OnceCell<SubagentStatePutExecutorFn> = once_cell::sync::OnceCell::new();
static SUBAGENT_STATE_GET_EXECUTOR: once_cell::sync::OnceCell<SubagentStateGetExecutorFn> = once_cell::sync::OnceCell::new();
static SUBAGENT_EVENT_PUBLISH_EXECUTOR: once_cell::sync::OnceCell<SubagentEventPublishExecutorFn> = once_cell::sync::OnceCell::new();
static SUBAGENT_EVENT_POLL_EXECUTOR: once_cell::sync::OnceCell<SubagentEventPollExecutorFn> = once_cell::sync::OnceCell::new();
static SUBAGENT_WORKFLOW_RUN_EXECUTOR: once_cell::sync::OnceCell<SubagentWorkflowRunExecutorFn> = once_cell::sync::OnceCell::new();

pub fn set_subagent_spawn_executor(executor: SubagentSpawnExecutorFn) {
    let _ = SUBAGENT_SPAWN_EXECUTOR.set(executor);
}

pub fn set_subagent_wait_executor(executor: SubagentWaitExecutorFn) {
    let _ = SUBAGENT_WAIT_EXECUTOR.set(executor);
}

pub fn set_subagent_wait_any_executor(executor: SubagentWaitAnyExecutorFn) {
    let _ = SUBAGENT_WAIT_ANY_EXECUTOR.set(executor);
}

pub fn set_subagent_run_executor(executor: SubagentRunExecutorFn) {
    let _ = SUBAGENT_RUN_EXECUTOR.set(executor);
}

pub fn set_subagent_state_put_executor(executor: SubagentStatePutExecutorFn) {
    let _ = SUBAGENT_STATE_PUT_EXECUTOR.set(executor);
}

pub fn set_subagent_state_get_executor(executor: SubagentStateGetExecutorFn) {
    let _ = SUBAGENT_STATE_GET_EXECUTOR.set(executor);
}

pub fn set_subagent_event_publish_executor(executor: SubagentEventPublishExecutorFn) {
    let _ = SUBAGENT_EVENT_PUBLISH_EXECUTOR.set(executor);
}

pub fn set_subagent_event_poll_executor(executor: SubagentEventPollExecutorFn) {
    let _ = SUBAGENT_EVENT_POLL_EXECUTOR.set(executor);
}

pub fn set_subagent_workflow_run_executor(executor: SubagentWorkflowRunExecutorFn) {
    let _ = SUBAGENT_WORKFLOW_RUN_EXECUTOR.set(executor);
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

fn get_wait_any_executor() -> Result<&'static SubagentWaitAnyExecutorFn, SubagentToolError> {
    SUBAGENT_WAIT_ANY_EXECUTOR.get().ok_or_else(|| {
        SubagentToolError::InternalError("Subagent wait_any executor not initialized".to_string())
    })
}

fn get_run_executor() -> Result<&'static SubagentRunExecutorFn, SubagentToolError> {
    SUBAGENT_RUN_EXECUTOR.get().ok_or_else(|| {
        SubagentToolError::InternalError("Subagent run executor not initialized".to_string())
    })
}

fn get_state_put_executor() -> Result<&'static SubagentStatePutExecutorFn, SubagentToolError> {
    SUBAGENT_STATE_PUT_EXECUTOR.get().ok_or_else(|| {
        SubagentToolError::InternalError("Subagent state put executor not initialized".to_string())
    })
}

fn get_state_get_executor() -> Result<&'static SubagentStateGetExecutorFn, SubagentToolError> {
    SUBAGENT_STATE_GET_EXECUTOR.get().ok_or_else(|| {
        SubagentToolError::InternalError("Subagent state get executor not initialized".to_string())
    })
}

fn get_event_publish_executor() -> Result<&'static SubagentEventPublishExecutorFn, SubagentToolError> {
    SUBAGENT_EVENT_PUBLISH_EXECUTOR.get().ok_or_else(|| {
        SubagentToolError::InternalError("Subagent event publish executor not initialized".to_string())
    })
}

fn get_event_poll_executor() -> Result<&'static SubagentEventPollExecutorFn, SubagentToolError> {
    SUBAGENT_EVENT_POLL_EXECUTOR.get().ok_or_else(|| {
        SubagentToolError::InternalError("Subagent event poll executor not initialized".to_string())
    })
}

fn get_workflow_run_executor() -> Result<&'static SubagentWorkflowRunExecutorFn, SubagentToolError> {
    SUBAGENT_WORKFLOW_RUN_EXECUTOR.get().ok_or_else(|| {
        SubagentToolError::InternalError("Subagent workflow run executor not initialized".to_string())
    })
}

fn default_true() -> bool {
    true
}

fn default_max_iterations() -> usize {
    DEFAULT_SUBAGENT_MAX_ITERATIONS
}

fn default_timeout() -> u64 {
    get_default_subagent_timeout()
}

fn default_channel() -> String {
    "default".to_string()
}

fn default_limit() -> usize {
    50
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
    /// Max iterations for subagent (default: 50)
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    /// Timeout seconds for subagent
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    /// Inherit LLM config from parent (default: true)
    #[serde(default = "default_true")]
    pub inherit_parent_llm: bool,
    /// Inherit tool config from parent (default: false)
    #[serde(default)]
    pub inherit_parent_tools: bool,
    /// Optional dependency task IDs. This task starts only when dependencies succeed.
    #[serde(default)]
    pub depends_on_task_ids: Vec<String>,
}

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
    pub fn new() -> Self {
        Self
    }
    pub const NAME: &'static str = "subagent_spawn";
    pub const DESCRIPTION: &'static str = "Start a subagent task asynchronously (NON-BLOCKING). Returns immediately with a task_id. Supports dependency constraints via depends_on_task_ids for ordered execution.";
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
            parameters: serde_json::to_value(schemars::schema_for!(SubagentSpawnArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!(
            "Subagent spawn requested - parent: {}, role: {:?}, deps: {}, task_preview: {}",
            args.parent_execution_id,
            args.role,
            args.depends_on_task_ids.len(),
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
    /// Timeout in seconds (default: global subagent timeout)
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

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

/// Arguments for waiting until any task completes
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubagentWaitAnyArgs {
    /// Parent execution id
    pub parent_execution_id: String,
    /// Candidate task IDs
    pub task_ids: Vec<String>,
    /// Timeout in seconds (default: global subagent timeout)
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

/// Output for wait_any
#[derive(Debug, Clone, Serialize)]
pub struct SubagentWaitAnyOutput {
    /// Tasks that completed during this wait call
    pub completed: Vec<SubagentTaskResult>,
    /// Task IDs still pending/running
    pub pending_task_ids: Vec<String>,
}

/// Subagent Wait Tool - waits for spawned tasks to complete
#[derive(Debug, Clone, Default)]
pub struct SubagentWaitTool;

impl SubagentWaitTool {
    pub fn new() -> Self {
        Self
    }
    pub const NAME: &'static str = "subagent_wait";
    pub const DESCRIPTION: &'static str = "Wait for one or more spawned subagent tasks to complete (BLOCKING). Provide task_ids from subagent_spawn. Enforces parent_execution_id ownership checks.";
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
            parameters: serde_json::to_value(schemars::schema_for!(SubagentWaitArgs))
                .unwrap_or_default(),
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

/// Subagent Wait Any Tool - wait until any task completes
#[derive(Debug, Clone, Default)]
pub struct SubagentWaitAnyTool;

impl SubagentWaitAnyTool {
    pub fn new() -> Self {
        Self
    }
    pub const NAME: &'static str = "subagent_wait_any";
    pub const DESCRIPTION: &'static str = "Wait until any spawned subagent task completes (BLOCKING). Returns completed results and pending task_ids.";
}

impl Tool for SubagentWaitAnyTool {
    const NAME: &'static str = Self::NAME;
    type Args = SubagentWaitAnyArgs;
    type Output = SubagentWaitAnyOutput;
    type Error = SubagentToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SubagentWaitAnyArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!(
            "Subagent wait_any requested - parent: {}, task_ids: {:?}, timeout: {}s",
            args.parent_execution_id,
            args.task_ids,
            args.timeout_secs
        );
        let executor = get_wait_any_executor()?;
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
    /// Max iterations for subagent (default: 50)
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    /// Timeout seconds for subagent
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    /// Inherit LLM config from parent (default: true)
    #[serde(default = "default_true")]
    pub inherit_parent_llm: bool,
    /// Inherit tool config from parent (default: false)
    #[serde(default)]
    pub inherit_parent_tools: bool,
    /// Optional dependency task IDs.
    #[serde(default)]
    pub depends_on_task_ids: Vec<String>,
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
    pub fn new() -> Self {
        Self
    }
    pub const NAME: &'static str = "subagent_run";
    pub const DESCRIPTION: &'static str = "Execute a subagent task synchronously (BLOCKING). Waits for completion before returning. Supports depends_on_task_ids for ordered execution.";
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
            parameters: serde_json::to_value(schemars::schema_for!(SubagentRunArgs))
                .unwrap_or_default(),
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
// Tool 4: subagent_workflow_run (DAG orchestration)
// ============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubagentWorkflowNode {
    /// Unique node ID in the workflow
    pub node_id: String,
    /// Task for this node
    pub task: String,
    /// Optional display role
    #[serde(default)]
    pub role: Option<String>,
    /// Optional dependency node IDs
    #[serde(default)]
    pub depends_on_node_ids: Vec<String>,
    /// Optional max iterations override (default: 50)
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    /// Optional timeout override
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubagentWorkflowRunArgs {
    pub parent_execution_id: String,
    pub nodes: Vec<SubagentWorkflowNode>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentWorkflowNodeResult {
    pub node_id: String,
    pub task_id: String,
    pub result: SubagentTaskResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentWorkflowRunOutput {
    pub workflow_id: String,
    pub results: Vec<SubagentWorkflowNodeResult>,
    pub summary: String,
}

#[derive(Debug, Clone, Default)]
pub struct SubagentWorkflowRunTool;

impl SubagentWorkflowRunTool {
    pub fn new() -> Self {
        Self
    }
    pub const NAME: &'static str = "subagent_workflow_run";
    pub const DESCRIPTION: &'static str = "Run a DAG-style subagent workflow. Each node can depend on previous nodes via depends_on_node_ids.";
}

impl Tool for SubagentWorkflowRunTool {
    const NAME: &'static str = Self::NAME;
    type Args = SubagentWorkflowRunArgs;
    type Output = SubagentWorkflowRunOutput;
    type Error = SubagentToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SubagentWorkflowRunArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!(
            "Subagent workflow_run requested - parent: {}, nodes: {}, timeout: {}s",
            args.parent_execution_id,
            args.nodes.len(),
            args.timeout_secs
        );
        let executor = get_workflow_run_executor()?;
        executor(args).await
    }
}

// ============================================================================
// Tool 5/6: Shared State
// ============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubagentStatePutArgs {
    pub parent_execution_id: String,
    pub key: String,
    pub value: serde_json::Value,
    #[serde(default)]
    pub expected_version: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentStatePutOutput {
    pub key: String,
    pub version: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubagentStateGetArgs {
    pub parent_execution_id: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentStateGetOutput {
    pub key: String,
    pub found: bool,
    pub value: Option<serde_json::Value>,
    pub version: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct SubagentStatePutTool;

impl SubagentStatePutTool {
    pub fn new() -> Self {
        Self
    }
    pub const NAME: &'static str = "subagent_state_put";
    pub const DESCRIPTION: &'static str = "Write shared state for subagents under the same parent_execution_id. Supports optimistic locking via expected_version.";
}

impl Tool for SubagentStatePutTool {
    const NAME: &'static str = Self::NAME;
    type Args = SubagentStatePutArgs;
    type Output = SubagentStatePutOutput;
    type Error = SubagentToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SubagentStatePutArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = get_state_put_executor()?;
        executor(args).await
    }
}

#[derive(Debug, Clone, Default)]
pub struct SubagentStateGetTool;

impl SubagentStateGetTool {
    pub fn new() -> Self {
        Self
    }
    pub const NAME: &'static str = "subagent_state_get";
    pub const DESCRIPTION: &'static str = "Read shared state for subagents under the same parent_execution_id.";
}

impl Tool for SubagentStateGetTool {
    const NAME: &'static str = Self::NAME;
    type Args = SubagentStateGetArgs;
    type Output = SubagentStateGetOutput;
    type Error = SubagentToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SubagentStateGetArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = get_state_get_executor()?;
        executor(args).await
    }
}

// ============================================================================
// Tool 6/7: Event Bus
// ============================================================================

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubagentEventPublishArgs {
    pub parent_execution_id: String,
    #[serde(default = "default_channel")]
    pub channel: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentEventPublishOutput {
    pub channel: String,
    pub seq: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentEventItem {
    pub channel: String,
    pub seq: u64,
    pub timestamp: i64,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubagentEventPollArgs {
    pub parent_execution_id: String,
    #[serde(default = "default_channel")]
    pub channel: String,
    #[serde(default)]
    pub after_seq: Option<u64>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentEventPollOutput {
    pub channel: String,
    pub latest_seq: u64,
    pub events: Vec<SubagentEventItem>,
}

#[derive(Debug, Clone, Default)]
pub struct SubagentEventPublishTool;

impl SubagentEventPublishTool {
    pub fn new() -> Self {
        Self
    }
    pub const NAME: &'static str = "subagent_event_publish";
    pub const DESCRIPTION: &'static str = "Publish an event to a shared channel for subagents under the same parent_execution_id.";
}

impl Tool for SubagentEventPublishTool {
    const NAME: &'static str = Self::NAME;
    type Args = SubagentEventPublishArgs;
    type Output = SubagentEventPublishOutput;
    type Error = SubagentToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SubagentEventPublishArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = get_event_publish_executor()?;
        executor(args).await
    }
}

#[derive(Debug, Clone, Default)]
pub struct SubagentEventPollTool;

impl SubagentEventPollTool {
    pub fn new() -> Self {
        Self
    }
    pub const NAME: &'static str = "subagent_event_poll";
    pub const DESCRIPTION: &'static str = "Poll events from a shared channel for subagents under the same parent_execution_id.";
}

impl Tool for SubagentEventPollTool {
    const NAME: &'static str = Self::NAME;
    type Args = SubagentEventPollArgs;
    type Output = SubagentEventPollOutput;
    type Error = SubagentToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SubagentEventPollArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = get_event_poll_executor()?;
        executor(args).await
    }
}

// ============================================================================
// Legacy Compatibility: SubagentTool alias
// ============================================================================

pub type SubagentTool = SubagentRunTool;

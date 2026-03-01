//! Subagent tools (condensed)
//!
//! Provides three LLM-facing tools:
//! - `subagent_execute`: sync/async/workflow execution
//! - `subagent_await`: wait for all/any async tasks
//! - `subagent_channel`: shared state + event bus operations

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
// Shared Types
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

/// Single task result
#[derive(Debug, Clone, Serialize)]
pub struct SubagentTaskResult {
    pub task_id: String,
    pub role: Option<String>,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentEventItem {
    pub channel: String,
    pub seq: u64,
    pub timestamp: i64,
    pub payload: serde_json::Value,
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
// Internal execution primitives used by executor implementation
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct SubagentSpawnArgs {
    pub parent_execution_id: String,
    pub task: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub tool_config: Option<serde_json::Value>,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default = "default_true")]
    pub inherit_parent_llm: bool,
    #[serde(default)]
    pub inherit_parent_tools: bool,
    #[serde(default)]
    pub depends_on_task_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentSpawnOutput {
    pub task_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubagentWaitArgs {
    pub parent_execution_id: String,
    pub task_ids: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentWaitOutput {
    pub results: Vec<SubagentTaskResult>,
    pub summary: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubagentWaitAnyArgs {
    pub parent_execution_id: String,
    pub task_ids: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentWaitAnyOutput {
    pub completed: Vec<SubagentTaskResult>,
    pub pending_task_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubagentRunArgs {
    pub parent_execution_id: String,
    pub task: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub tool_config: Option<serde_json::Value>,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default = "default_true")]
    pub inherit_parent_llm: bool,
    #[serde(default)]
    pub inherit_parent_tools: bool,
    #[serde(default)]
    pub depends_on_task_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentRunOutput {
    pub success: bool,
    pub execution_id: String,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubagentWorkflowNode {
    pub node_id: String,
    pub task: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub depends_on_node_ids: Vec<String>,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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

// ============================================================================
// Unified tool args/output
// ============================================================================

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SubagentExecuteMode {
    Sync,
    Async,
    Workflow,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubagentExecuteArgs {
    pub parent_execution_id: String,
    pub mode: SubagentExecuteMode,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub tool_config: Option<serde_json::Value>,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default = "default_true")]
    pub inherit_parent_llm: bool,
    #[serde(default)]
    pub inherit_parent_tools: bool,
    #[serde(default)]
    pub depends_on_task_ids: Vec<String>,
    #[serde(default)]
    pub nodes: Vec<SubagentWorkflowNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentExecuteOutput {
    pub mode: SubagentExecuteMode,
    pub success: bool,
    pub task_id: Option<String>,
    pub execution_id: Option<String>,
    pub workflow_id: Option<String>,
    pub result: Option<SubagentTaskResult>,
    pub results: Option<Vec<SubagentWorkflowNodeResult>>,
    pub summary: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SubagentAwaitPolicy {
    All,
    Any,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubagentAwaitArgs {
    pub parent_execution_id: String,
    pub policy: SubagentAwaitPolicy,
    pub task_ids: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentAwaitOutput {
    pub policy: SubagentAwaitPolicy,
    pub completed: Vec<SubagentTaskResult>,
    pub pending_task_ids: Vec<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub enum SubagentChannelOp {
    #[serde(rename = "state.put")]
    StatePut,
    #[serde(rename = "state.get")]
    StateGet,
    #[serde(rename = "event.publish")]
    EventPublish,
    #[serde(rename = "event.poll")]
    EventPoll,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SubagentChannelArgs {
    pub parent_execution_id: String,
    pub op: SubagentChannelOp,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    #[serde(default)]
    pub expected_version: Option<u64>,
    #[serde(default = "default_channel")]
    pub channel: String,
    #[serde(default)]
    pub payload: Option<serde_json::Value>,
    #[serde(default)]
    pub after_seq: Option<u64>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentChannelOutput {
    pub op: SubagentChannelOp,
    pub key: Option<String>,
    pub found: Option<bool>,
    pub value: Option<serde_json::Value>,
    pub version: Option<u64>,
    pub channel: Option<String>,
    pub seq: Option<u64>,
    pub latest_seq: Option<u64>,
    pub events: Option<Vec<SubagentEventItem>>,
}

// ============================================================================
// Executor registration
// ============================================================================

pub type SubagentExecuteExecutorFn = std::sync::Arc<
    dyn Fn(
            SubagentExecuteArgs,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<SubagentExecuteOutput, SubagentToolError>>
                    + Send,
            >,
        > + Send
        + Sync,
>;

pub type SubagentAwaitExecutorFn = std::sync::Arc<
    dyn Fn(
            SubagentAwaitArgs,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<SubagentAwaitOutput, SubagentToolError>>
                    + Send,
            >,
        > + Send
        + Sync,
>;

pub type SubagentChannelExecutorFn = std::sync::Arc<
    dyn Fn(
            SubagentChannelArgs,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<SubagentChannelOutput, SubagentToolError>>
                    + Send,
            >,
        > + Send
        + Sync,
>;

static SUBAGENT_EXECUTE_EXECUTOR: once_cell::sync::OnceCell<SubagentExecuteExecutorFn> =
    once_cell::sync::OnceCell::new();
static SUBAGENT_AWAIT_EXECUTOR: once_cell::sync::OnceCell<SubagentAwaitExecutorFn> =
    once_cell::sync::OnceCell::new();
static SUBAGENT_CHANNEL_EXECUTOR: once_cell::sync::OnceCell<SubagentChannelExecutorFn> =
    once_cell::sync::OnceCell::new();

pub fn set_subagent_execute_executor(executor: SubagentExecuteExecutorFn) {
    let _ = SUBAGENT_EXECUTE_EXECUTOR.set(executor);
}

pub fn set_subagent_await_executor(executor: SubagentAwaitExecutorFn) {
    let _ = SUBAGENT_AWAIT_EXECUTOR.set(executor);
}

pub fn set_subagent_channel_executor(executor: SubagentChannelExecutorFn) {
    let _ = SUBAGENT_CHANNEL_EXECUTOR.set(executor);
}

fn get_execute_executor() -> Result<&'static SubagentExecuteExecutorFn, SubagentToolError> {
    SUBAGENT_EXECUTE_EXECUTOR.get().ok_or_else(|| {
        SubagentToolError::InternalError("Subagent execute executor not initialized".to_string())
    })
}

fn get_await_executor() -> Result<&'static SubagentAwaitExecutorFn, SubagentToolError> {
    SUBAGENT_AWAIT_EXECUTOR.get().ok_or_else(|| {
        SubagentToolError::InternalError("Subagent await executor not initialized".to_string())
    })
}

fn get_channel_executor() -> Result<&'static SubagentChannelExecutorFn, SubagentToolError> {
    SUBAGENT_CHANNEL_EXECUTOR.get().ok_or_else(|| {
        SubagentToolError::InternalError("Subagent channel executor not initialized".to_string())
    })
}

// ============================================================================
// Tool implementations
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct SubagentExecuteTool;

impl SubagentExecuteTool {
    pub fn new() -> Self {
        Self
    }

    pub const NAME: &'static str = "subagent_execute";
    pub const DESCRIPTION: &'static str =
        "Execute subagent tasks in unified modes: sync, async, or workflow.";
}

impl Tool for SubagentExecuteTool {
    const NAME: &'static str = Self::NAME;
    type Args = SubagentExecuteArgs;
    type Output = SubagentExecuteOutput;
    type Error = SubagentToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SubagentExecuteArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = get_execute_executor()?;
        executor(args).await
    }
}

#[derive(Debug, Clone, Default)]
pub struct SubagentAwaitTool;

impl SubagentAwaitTool {
    pub fn new() -> Self {
        Self
    }

    pub const NAME: &'static str = "subagent_await";
    pub const DESCRIPTION: &'static str =
        "Wait for subagent tasks to complete with policy all or any.";
}

impl Tool for SubagentAwaitTool {
    const NAME: &'static str = Self::NAME;
    type Args = SubagentAwaitArgs;
    type Output = SubagentAwaitOutput;
    type Error = SubagentToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SubagentAwaitArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = get_await_executor()?;
        executor(args).await
    }
}

#[derive(Debug, Clone, Default)]
pub struct SubagentChannelTool;

impl SubagentChannelTool {
    pub fn new() -> Self {
        Self
    }

    pub const NAME: &'static str = "subagent_channel";
    pub const DESCRIPTION: &'static str =
        "Operate shared state and events for subagents using op: state.put, state.get, event.publish, event.poll.";
}

impl Tool for SubagentChannelTool {
    const NAME: &'static str = Self::NAME;
    type Args = SubagentChannelArgs;
    type Output = SubagentChannelOutput;
    type Error = SubagentToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SubagentChannelArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = get_channel_executor()?;
        executor(args).await
    }
}

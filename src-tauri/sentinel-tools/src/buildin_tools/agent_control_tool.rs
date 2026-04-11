use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

type SpawnAgentExecutorFuture =
    Pin<Box<dyn Future<Output = Result<SpawnAgentOutput, AgentControlToolError>> + Send>>;
type WaitAgentsExecutorFuture =
    Pin<Box<dyn Future<Output = Result<WaitAgentsOutput, AgentControlToolError>> + Send>>;
type ListAgentsExecutorFuture =
    Pin<Box<dyn Future<Output = Result<ListAgentsOutput, AgentControlToolError>> + Send>>;
type CloseAgentExecutorFuture =
    Pin<Box<dyn Future<Output = Result<CloseAgentOutput, AgentControlToolError>> + Send>>;

type SpawnAgentExecutorFn = Arc<dyn Fn(SpawnAgentArgs) -> SpawnAgentExecutorFuture + Send + Sync>;
type WaitAgentsExecutorFn = Arc<dyn Fn(WaitAgentsArgs) -> WaitAgentsExecutorFuture + Send + Sync>;
type ListAgentsExecutorFn = Arc<dyn Fn(ListAgentsArgs) -> ListAgentsExecutorFuture + Send + Sync>;
type CloseAgentExecutorFn = Arc<dyn Fn(CloseAgentArgs) -> CloseAgentExecutorFuture + Send + Sync>;

static SPAWN_AGENT_EXECUTOR: OnceLock<SpawnAgentExecutorFn> = OnceLock::new();
static WAIT_AGENTS_EXECUTOR: OnceLock<WaitAgentsExecutorFn> = OnceLock::new();
static LIST_AGENTS_EXECUTOR: OnceLock<ListAgentsExecutorFn> = OnceLock::new();
static CLOSE_AGENT_EXECUTOR: OnceLock<CloseAgentExecutorFn> = OnceLock::new();
static DEFAULT_AGENT_TIMEOUT_SECS: AtomicU64 = AtomicU64::new(300);

#[derive(Debug, Clone, Serialize)]
pub struct AgentControlToolError {
    pub message: String,
}

impl std::fmt::Display for AgentControlToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AgentControlToolError {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SpawnAgentArgs {
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
    #[serde(default)]
    pub inherit_parent_tools: bool,
    #[serde(default)]
    pub depends_on_task_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpawnAgentOutput {
    pub task_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WaitAgentsArgs {
    pub parent_execution_id: String,
    pub task_ids: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WaitAgentsOutput {
    pub agents: Vec<AgentHandleItem>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ListAgentsArgs {
    pub parent_execution_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListAgentsOutput {
    pub agents: Vec<AgentHandleItem>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CloseAgentArgs {
    pub parent_execution_id: String,
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CloseAgentOutput {
    pub closed: bool,
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentHandleItem {
    pub id: String,
    pub parent_execution_id: String,
    pub kind: String,
    pub role: Option<String>,
    pub task: String,
    pub status: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub output: Option<String>,
    pub error: Option<String>,
}

fn default_max_iterations() -> usize {
    50
}

fn default_timeout() -> u64 {
    get_default_agent_timeout()
}

pub fn set_default_agent_timeout(timeout_secs: u64) {
    DEFAULT_AGENT_TIMEOUT_SECS.store(timeout_secs, Ordering::SeqCst);
}

pub fn get_default_agent_timeout() -> u64 {
    DEFAULT_AGENT_TIMEOUT_SECS.load(Ordering::SeqCst)
}

fn get_spawn_executor() -> Result<SpawnAgentExecutorFn, AgentControlToolError> {
    SPAWN_AGENT_EXECUTOR
        .get()
        .cloned()
        .ok_or_else(|| AgentControlToolError {
            message: "spawn_agent executor not initialized".to_string(),
        })
}

fn get_wait_executor() -> Result<WaitAgentsExecutorFn, AgentControlToolError> {
    WAIT_AGENTS_EXECUTOR
        .get()
        .cloned()
        .ok_or_else(|| AgentControlToolError {
            message: "wait_agents executor not initialized".to_string(),
        })
}

fn get_list_executor() -> Result<ListAgentsExecutorFn, AgentControlToolError> {
    LIST_AGENTS_EXECUTOR
        .get()
        .cloned()
        .ok_or_else(|| AgentControlToolError {
            message: "list_agents executor not initialized".to_string(),
        })
}

fn get_close_executor() -> Result<CloseAgentExecutorFn, AgentControlToolError> {
    CLOSE_AGENT_EXECUTOR
        .get()
        .cloned()
        .ok_or_else(|| AgentControlToolError {
            message: "close_agent executor not initialized".to_string(),
        })
}

pub fn set_spawn_agent_executor(executor: SpawnAgentExecutorFn) {
    let _ = SPAWN_AGENT_EXECUTOR.set(executor);
}

pub fn set_wait_agents_executor(executor: WaitAgentsExecutorFn) {
    let _ = WAIT_AGENTS_EXECUTOR.set(executor);
}

pub fn set_list_agents_executor(executor: ListAgentsExecutorFn) {
    let _ = LIST_AGENTS_EXECUTOR.set(executor);
}

pub fn set_close_agent_executor(executor: CloseAgentExecutorFn) {
    let _ = CLOSE_AGENT_EXECUTOR.set(executor);
}

#[derive(Debug, Clone, Default)]
pub struct SpawnAgentTool;

impl SpawnAgentTool {
    pub const NAME: &'static str = "spawn_agent";
    pub const DESCRIPTION: &'static str =
        "Spawn a bounded background agent under the current execution. Use this when work can continue in parallel or should be delegated to a subagent.";
}

impl Tool for SpawnAgentTool {
    const NAME: &'static str = Self::NAME;
    type Args = SpawnAgentArgs;
    type Output = SpawnAgentOutput;
    type Error = AgentControlToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(SpawnAgentArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = get_spawn_executor()?;
        executor(args).await
    }
}

#[derive(Debug, Clone, Default)]
pub struct WaitAgentsTool;

impl WaitAgentsTool {
    pub const NAME: &'static str = "wait_agents";
    pub const DESCRIPTION: &'static str =
        "Wait for one or more previously spawned agents to reach a terminal state and return their latest summaries.";
}

impl Tool for WaitAgentsTool {
    const NAME: &'static str = Self::NAME;
    type Args = WaitAgentsArgs;
    type Output = WaitAgentsOutput;
    type Error = AgentControlToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(WaitAgentsArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = get_wait_executor()?;
        executor(args).await
    }
}

#[derive(Debug, Clone, Default)]
pub struct ListAgentsTool;

impl ListAgentsTool {
    pub const NAME: &'static str = "list_agents";
    pub const DESCRIPTION: &'static str =
        "List spawned agents for the current parent execution, including status and latest summaries.";
}

impl Tool for ListAgentsTool {
    const NAME: &'static str = Self::NAME;
    type Args = ListAgentsArgs;
    type Output = ListAgentsOutput;
    type Error = AgentControlToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ListAgentsArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = get_list_executor()?;
        executor(args).await
    }
}

#[derive(Debug, Clone, Default)]
pub struct CloseAgentTool;

impl CloseAgentTool {
    pub const NAME: &'static str = "close_agent";
    pub const DESCRIPTION: &'static str =
        "Close a spawned agent handle once its result is no longer needed.";
}

impl Tool for CloseAgentTool {
    const NAME: &'static str = Self::NAME;
    type Args = CloseAgentArgs;
    type Output = CloseAgentOutput;
    type Error = AgentControlToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(CloseAgentArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = get_close_executor()?;
        executor(args).await
    }
}

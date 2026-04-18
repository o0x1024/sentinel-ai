use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

type ToolSearchExecutorFuture =
    Pin<Box<dyn Future<Output = Result<ToolSearchOutput, ToolSearchError>> + Send>>;
type ToolSearchExecutorFn = Arc<dyn Fn(ToolSearchArgs) -> ToolSearchExecutorFuture + Send + Sync>;
type ToolSearchContextProviderFuture =
    Pin<Box<dyn Future<Output = Option<ToolSearchRuntimeContext>> + Send>>;
type ToolSearchContextProviderFn =
    Arc<dyn Fn(String) -> ToolSearchContextProviderFuture + Send + Sync>;

static TOOL_SEARCH_EXECUTOR: OnceLock<ToolSearchExecutorFn> = OnceLock::new();
static TOOL_SEARCH_CONTEXT_PROVIDER: OnceLock<ToolSearchContextProviderFn> = OnceLock::new();

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolSearchAction {
    Search,
    Activate,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ToolSearchArgs {
    pub action: ToolSearchAction,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub tool_ids: Option<Vec<String>>,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default, skip_serializing)]
    #[schemars(skip)]
    pub execution_id: Option<String>,
}

fn default_max_results() -> usize {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolSearchMatch {
    pub tool_id: String,
    pub description: String,
    pub reason: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub search_hint: Option<String>,
    #[serde(default)]
    pub exposure: Option<String>,
    #[serde(default)]
    pub already_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolSearchOutput {
    pub action: String,
    pub matches: Vec<ToolSearchMatch>,
    pub activated_tool_ids: Vec<String>,
    #[serde(default)]
    pub recommended_tool_ids: Vec<String>,
    #[serde(default)]
    pub recommendation_reason: Option<String>,
    #[serde(default)]
    pub runtime_hint: Option<String>,
    pub requires_reload: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolSearchRuntimeContext {
    #[serde(default)]
    pub prefer_file_read_backfill: bool,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolSearchError {
    pub message: String,
}

impl std::fmt::Display for ToolSearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ToolSearchError {}

pub fn set_tool_search_executor(executor: ToolSearchExecutorFn) {
    let _ = TOOL_SEARCH_EXECUTOR.set(executor);
}

pub fn set_tool_search_context_provider(provider: ToolSearchContextProviderFn) {
    let _ = TOOL_SEARCH_CONTEXT_PROVIDER.set(provider);
}

fn get_executor() -> Result<ToolSearchExecutorFn, ToolSearchError> {
    TOOL_SEARCH_EXECUTOR
        .get()
        .cloned()
        .ok_or_else(|| ToolSearchError {
            message: "tool_search executor not initialized".to_string(),
        })
}

pub async fn load_tool_search_runtime_context(
    execution_id: Option<&str>,
) -> Option<ToolSearchRuntimeContext> {
    let execution_id = execution_id
        .map(str::trim)
        .filter(|execution_id| !execution_id.is_empty())?;
    let provider = TOOL_SEARCH_CONTEXT_PROVIDER.get()?.clone();
    provider(execution_id.to_string()).await
}

#[derive(Debug, Clone, Default)]
pub struct ToolSearchTool;

impl ToolSearchTool {
    pub const NAME: &'static str = "tool_search";
    pub const DESCRIPTION: &'static str = concat!(
        "Search the full tool catalog and activate deferred tools on demand. ",
        "Use action='search' to find tools by capability when the current toolset is insufficient. ",
        "Use action='activate' with selected tool IDs to request adding them to the active toolset, ",
        "or reuse the same query and omit tool_ids when the search result provides recommended_tool_ids. ",
        "For file creation or overwrite tasks, prefer the recommended bundle so you can write and then re-read the result. ",
        "Prefer this instead of guessing hidden tool names in deferred-tool mode."
    );
}

impl Tool for ToolSearchTool {
    const NAME: &'static str = Self::NAME;
    type Args = ToolSearchArgs;
    type Output = ToolSearchOutput;
    type Error = ToolSearchError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ToolSearchArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = get_executor()?;
        executor(args).await
    }
}

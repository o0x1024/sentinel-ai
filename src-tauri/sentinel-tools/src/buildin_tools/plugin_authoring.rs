use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

type PluginAuthoringExecutorFuture =
    Pin<Box<dyn Future<Output = Result<PluginAuthoringOutput, PluginAuthoringError>> + Send>>;
type PluginAuthoringExecutorFn =
    Arc<dyn Fn(PluginAuthoringArgs) -> PluginAuthoringExecutorFuture + Send + Sync>;

static PLUGIN_AUTHORING_EXECUTOR: OnceLock<PluginAuthoringExecutorFn> = OnceLock::new();

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginAuthoringAction {
    Generate,
    Improve,
    Validate,
    Test,
    SaveDraft,
    Enable,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct PluginAuthoringArgs {
    pub action: PluginAuthoringAction,
    #[serde(default)]
    pub main_category: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub requirements: Option<String>,
    #[serde(default)]
    pub existing_plugin_id: Option<String>,
    #[serde(default)]
    pub plugin_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub default_severity: Option<String>,
    #[serde(default)]
    pub monitor_type: Option<String>,
    #[serde(default)]
    pub traffic_samples: Option<serde_json::Value>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub enable_after_test: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct PluginAuthoringOutput {
    pub success: bool,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginAuthoringError {
    pub message: String,
}

impl std::fmt::Display for PluginAuthoringError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for PluginAuthoringError {}

pub fn register_plugin_authoring_executor(executor: PluginAuthoringExecutorFn) {
    let _ = PLUGIN_AUTHORING_EXECUTOR.set(executor);
}

fn get_executor() -> Result<PluginAuthoringExecutorFn, PluginAuthoringError> {
    PLUGIN_AUTHORING_EXECUTOR
        .get()
        .cloned()
        .ok_or_else(|| PluginAuthoringError {
            message: "plugin_authoring executor not initialized".to_string(),
        })
}

#[derive(Debug, Clone, Default)]
pub struct PluginAuthoringTool;

impl PluginAuthoringTool {
    pub const NAME: &'static str = "plugin_authoring";
    pub const DESCRIPTION: &'static str = concat!(
        "Author or revise Sentinel plugins through a controlled lifecycle. ",
        "Use this instead of generic file or shell tools when the task is to generate, improve, validate, test, save as draft, or enable a plugin. ",
        "This tool keeps creation and activation as separate steps and only enables when explicitly requested."
    );
}

impl Tool for PluginAuthoringTool {
    const NAME: &'static str = Self::NAME;
    type Args = PluginAuthoringArgs;
    type Output = PluginAuthoringOutput;
    type Error = PluginAuthoringError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(PluginAuthoringArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = get_executor()?;
        executor(args).await
    }
}

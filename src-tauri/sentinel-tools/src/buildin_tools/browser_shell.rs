use once_cell::sync::Lazy;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BrowserShellAction {
    ListSessions,
    ReadFrames,
    QueueWrite,
    ListWriteRequests,
    RespondWrite,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct BrowserShellArgs {
    /// Action to perform on tracked browser shell sessions.
    pub action: BrowserShellAction,
    /// Browser shell session id for read or write actions.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Maximum number of recent frames to return.
    #[serde(default)]
    pub limit: Option<usize>,
    /// Text to send to the selected browser shell session.
    #[serde(default)]
    pub input_text: Option<String>,
    /// Whether the queued write requires explicit user approval.
    #[serde(default = "default_requires_approval")]
    pub requires_approval: bool,
    /// Pending browser shell write request id for approval or rejection.
    #[serde(default)]
    pub request_id: Option<String>,
    /// Approval decision for `respond_write`.
    #[serde(default)]
    pub allowed: Option<bool>,
}

fn default_requires_approval() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct BrowserShellOutput {
    pub action: String,
    pub data: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum BrowserShellError {
    #[error("invalid browser_shell input: {0}")]
    InvalidInput(String),
    #[error("no browser shell handler registered")]
    NoHandler,
    #[error("browser shell request failed: {0}")]
    RequestFailed(String),
}

#[async_trait::async_trait]
pub trait BrowserShellHandler: Send + Sync {
    async fn handle(&self, args: BrowserShellArgs) -> Result<serde_json::Value, BrowserShellError>;
}

static BROWSER_SHELL_HANDLER: Lazy<RwLock<Option<Arc<dyn BrowserShellHandler>>>> =
    Lazy::new(|| RwLock::new(None));

pub async fn set_browser_shell_handler(handler: Arc<dyn BrowserShellHandler>) {
    let mut guard = BROWSER_SHELL_HANDLER.write().await;
    *guard = Some(handler);
}

async fn dispatch_browser_shell(
    args: BrowserShellArgs,
) -> Result<serde_json::Value, BrowserShellError> {
    let guard = BROWSER_SHELL_HANDLER.read().await;
    let Some(handler) = &*guard else {
        return Err(BrowserShellError::NoHandler);
    };
    handler.handle(args).await
}

fn validate_args(args: &BrowserShellArgs) -> Result<(), BrowserShellError> {
    match args.action {
        BrowserShellAction::ListSessions | BrowserShellAction::ListWriteRequests => {}
        BrowserShellAction::ReadFrames => {
            if args
                .session_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                return Err(BrowserShellError::InvalidInput(
                    "read_frames requires session_id".to_string(),
                ));
            }
        }
        BrowserShellAction::QueueWrite => {
            if args
                .session_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                return Err(BrowserShellError::InvalidInput(
                    "queue_write requires session_id".to_string(),
                ));
            }
            if args
                .input_text
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                return Err(BrowserShellError::InvalidInput(
                    "queue_write requires input_text".to_string(),
                ));
            }
        }
        BrowserShellAction::RespondWrite => {
            if args
                .request_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            {
                return Err(BrowserShellError::InvalidInput(
                    "respond_write requires request_id".to_string(),
                ));
            }
            if args.allowed.is_none() {
                return Err(BrowserShellError::InvalidInput(
                    "respond_write requires allowed".to_string(),
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct BrowserShellTool;

impl BrowserShellTool {
    pub const NAME: &'static str = "browser_shell";
    pub const DESCRIPTION: &'static str = concat!(
        "Inspect and control third-party browser WebSocket shell sessions captured by the Sentinel Chrome extension. ",
        "Use list_sessions first, then read_frames to inspect a compacted summary of recent terminal output. ",
        "Use queue_write to request sending input into a browser shell session. ",
        "Writes should normally require approval; only set requires_approval=false when the user has explicitly authorized direct execution. ",
        "Use list_write_requests and respond_write to review or approve pending write requests."
    );
}

impl Tool for BrowserShellTool {
    const NAME: &'static str = Self::NAME;
    type Args = BrowserShellArgs;
    type Output = BrowserShellOutput;
    type Error = BrowserShellError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(BrowserShellArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        validate_args(&args)?;
        let data = dispatch_browser_shell(args.clone()).await?;
        Ok(BrowserShellOutput {
            action: serde_json::to_value(&args.action)
                .ok()
                .and_then(|value| value.as_str().map(ToOwned::to_owned))
                .unwrap_or_else(|| "unknown".to_string()),
            data,
        })
    }
}

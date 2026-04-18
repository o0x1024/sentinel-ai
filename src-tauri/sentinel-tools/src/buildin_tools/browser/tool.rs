use super::args::{BrowserAction, BrowserToolArgs};
use super::output::BrowserToolOutput;
use super::session::execute_browser_action;
use rig::tool::Tool;

#[derive(Debug, thiserror::Error)]
pub enum BrowserToolError {
    #[error("browser tool failed: {0}")]
    Failed(String),
}

#[derive(Debug, Clone, Default)]
pub struct BrowserTool;

impl BrowserTool {
    pub const NAME: &'static str = "browser";
    pub const DESCRIPTION: &'static str = concat!(
        "Interact with a real Playwright-driven browser session for JavaScript-heavy or login-backed targets. ",
        "Use the same session_id across actions to preserve cookies and page state. ",
        "Recommended actions: launch, goto, snapshot, fill, click, eval, cookies, network_log, close."
    );

    fn validate_args(args: &BrowserToolArgs) -> Result<(), BrowserToolError> {
        match args.action {
            BrowserAction::Goto => {
                if args.url.as_deref().unwrap_or_default().trim().is_empty() {
                    return Err(BrowserToolError::Failed("goto requires url".to_string()));
                }
            }
            BrowserAction::Click => {
                if args
                    .selector
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                {
                    return Err(BrowserToolError::Failed(
                        "click requires selector".to_string(),
                    ));
                }
            }
            BrowserAction::Fill => {
                if args
                    .selector
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                {
                    return Err(BrowserToolError::Failed(
                        "fill requires selector".to_string(),
                    ));
                }
            }
            BrowserAction::Eval => {
                if args.script.as_deref().unwrap_or_default().trim().is_empty() {
                    return Err(BrowserToolError::Failed("eval requires script".to_string()));
                }
            }
            BrowserAction::Launch
            | BrowserAction::Snapshot
            | BrowserAction::Cookies
            | BrowserAction::NetworkLog
            | BrowserAction::Close => {}
        }
        Ok(())
    }
}

impl Tool for BrowserTool {
    const NAME: &'static str = Self::NAME;
    type Args = BrowserToolArgs;
    type Output = BrowserToolOutput;
    type Error = BrowserToolError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(BrowserToolArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Self::validate_args(&args)?;
        let action = serde_json::to_value(&args.action)
            .ok()
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| "unknown".to_string());
        let (session_id, data) = execute_browser_action(&args)
            .await
            .map_err(|error| BrowserToolError::Failed(error.to_string()))?;

        Ok(BrowserToolOutput {
            session_id,
            action,
            data,
        })
    }
}

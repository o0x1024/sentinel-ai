use once_cell::sync::Lazy;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

// --- Tool Actions ---

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BrowserAction {
    /// Launch or connect to a browser instance
    Launch,
    /// Navigate to a URL
    Navigate,
    /// Get current page state (accessibility tree, screenshot, or DOM)
    PageState,
    /// Click on an element
    Click,
    /// Type text into an element
    Type,
    /// Scroll the page
    Scroll,
    /// Wait for a condition
    Wait,
    /// Execute JavaScript
    Eval,
    /// Capture a screenshot
    Screenshot,
    /// Manage tabs (list, create, close, switch)
    Tabs,
    /// Manage cookies (get, set, clear)
    Cookies,
    /// Intercept/modify network requests
    Network,
    /// Close the browser session
    Close,
}

// --- Tool Args ---

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct BrowserAutomationArgs {
    /// Action to perform
    pub action: BrowserAction,

    /// Target tab ID (optional, uses active tab if not specified)
    #[serde(default)]
    pub tab_id: Option<String>,

    /// URL for navigate action
    #[serde(default)]
    pub url: Option<String>,

    /// Element selector for click/type actions. Can be:
    /// - ref_id: "e1", "e2" (from accessibility tree)
    /// - CSS selector: "#login-btn", ".submit"
    /// - text: "text:Login" (by visible text)
    #[serde(default)]
    pub selector: Option<String>,

    /// Text to type (for type action)
    #[serde(default)]
    pub text: Option<String>,

    /// Clear existing text before typing
    #[serde(default)]
    pub clear_first: Option<bool>,

    /// Scroll direction: "up", "down", "left", "right"
    #[serde(default)]
    pub direction: Option<String>,

    /// Scroll amount in pixels
    #[serde(default)]
    pub amount: Option<f64>,

    /// JavaScript expression (for eval action)
    #[serde(default)]
    pub expression: Option<String>,

    /// Page state mode: "a11y_tree" (default), "screenshot", "dom", "markdown"
    #[serde(default)]
    pub mode: Option<String>,

    /// Wait condition: "element", "navigation", "network_idle", "time"
    #[serde(default)]
    pub wait_for: Option<String>,

    /// Timeout in milliseconds
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// Tab sub-action: "list", "create", "close", "switch"
    #[serde(default)]
    pub tab_action: Option<String>,

    /// Screenshot format: "png" (default), "jpeg", "webp"
    #[serde(default)]
    pub format: Option<String>,

    /// Full page screenshot
    #[serde(default)]
    pub full_page: Option<bool>,

    /// Browser backend: "chrome" (default), "camoufox"
    #[serde(default)]
    pub backend: Option<String>,

    /// Run headless
    #[serde(default)]
    pub headless: Option<bool>,

    /// Humanization level: "raw", "basic", "human" (default), "stealth"
    #[serde(default)]
    pub humanize: Option<String>,

    /// Proxy URL
    #[serde(default)]
    pub proxy: Option<String>,

    /// Network interception patterns (for network action)
    #[serde(default)]
    pub patterns: Option<Vec<String>>,

    /// Cookies to set (for cookies action)
    #[serde(default)]
    pub cookies: Option<serde_json::Value>,
}

// --- Tool Output ---

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct BrowserAutomationOutput {
    pub action: String,
    pub success: bool,
    pub data: serde_json::Value,
}

// --- Tool Error ---

#[derive(Debug, thiserror::Error)]
pub enum BrowserAutomationError {
    #[error("invalid browser automation input: {0}")]
    InvalidInput(String),
    #[error("no browser automation handler registered")]
    NoHandler,
    #[error("browser automation failed: {0}")]
    ExecutionFailed(String),
    #[error("browser not connected. Ensure Chrome/Chromium is running with: chromium --headless=new --remote-debugging-port=9222 --remote-debugging-address=127.0.0.1 --no-sandbox --disable-gpu --disable-dev-shm-usage")]
    NotConnected,
    #[error("element not found: {0}")]
    ElementNotFound(String),
    #[error("timeout: {0}")]
    Timeout(String),
}

// --- Handler trait (injected at app startup) ---

#[async_trait::async_trait]
pub trait BrowserAutomationHandler: Send + Sync {
    async fn handle(
        &self,
        args: BrowserAutomationArgs,
    ) -> Result<serde_json::Value, BrowserAutomationError>;
}

static BROWSER_AUTOMATION_HANDLER: Lazy<RwLock<Option<Arc<dyn BrowserAutomationHandler>>>> =
    Lazy::new(|| RwLock::new(None));

pub async fn set_browser_automation_handler(handler: Arc<dyn BrowserAutomationHandler>) {
    let mut guard = BROWSER_AUTOMATION_HANDLER.write().await;
    *guard = Some(handler);
}

async fn dispatch_browser_automation(
    args: BrowserAutomationArgs,
) -> Result<serde_json::Value, BrowserAutomationError> {
    let guard = BROWSER_AUTOMATION_HANDLER.read().await;
    let Some(handler) = &*guard else {
        return Err(BrowserAutomationError::NoHandler);
    };
    handler.handle(args).await
}

// --- Validation ---

fn validate_args(args: &BrowserAutomationArgs) -> Result<(), BrowserAutomationError> {
    match args.action {
        BrowserAction::Navigate => {
            if args.url.as_deref().unwrap_or("").trim().is_empty() {
                return Err(BrowserAutomationError::InvalidInput(
                    "navigate requires 'url'".to_string(),
                ));
            }
        }
        BrowserAction::Click => {
            if args.selector.as_deref().unwrap_or("").trim().is_empty() {
                return Err(BrowserAutomationError::InvalidInput(
                    "click requires 'selector'".to_string(),
                ));
            }
        }
        BrowserAction::Type => {
            if args.selector.as_deref().unwrap_or("").trim().is_empty() {
                return Err(BrowserAutomationError::InvalidInput(
                    "type requires 'selector'".to_string(),
                ));
            }
            if args.text.is_none() {
                return Err(BrowserAutomationError::InvalidInput(
                    "type requires 'text'".to_string(),
                ));
            }
        }
        BrowserAction::Eval => {
            if args.expression.as_deref().unwrap_or("").trim().is_empty() {
                return Err(BrowserAutomationError::InvalidInput(
                    "eval requires 'expression'".to_string(),
                ));
            }
        }
        BrowserAction::Scroll => {
            if args.direction.is_none() && args.amount.is_none() {
                return Err(BrowserAutomationError::InvalidInput(
                    "scroll requires 'direction' or 'amount'".to_string(),
                ));
            }
        }
        BrowserAction::Tabs => {
            if args.tab_action.is_none() {
                return Err(BrowserAutomationError::InvalidInput(
                    "tabs requires 'tab_action' (list|create|close|switch)".to_string(),
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

// --- Tool implementation ---

#[derive(Debug, Clone, Default)]
pub struct BrowserAutomationTool;

impl BrowserAutomationTool {
    pub const NAME: &'static str = "browser";
    pub const DESCRIPTION: &'static str = concat!(
        "Control a real browser with human-like behavior for web automation, testing, and data extraction. ",
        "Supports Chrome (CDP) and Camoufox (Juggler) backends with configurable humanization levels. ",
        "WORKFLOW: 1) Use action='launch' to start/connect to a browser. ",
        "2) Use action='navigate' to go to a URL. ",
        "3) Use action='page_state' to read the page (accessibility tree shows interactive elements as [e1], [e2]...). ",
        "4) Use action='click'/'type'/'scroll' with selector='e1' to interact with elements. ",
        "5) Use action='screenshot' to capture visual state. ",
        "6) Use action='eval' to run JavaScript for complex operations. ",
        "Humanization levels: 'raw' (instant), 'basic' (random delays), 'human' (Bezier mouse + typing rhythm), 'stealth' (anti-detection)."
    );
}

impl Tool for BrowserAutomationTool {
    const NAME: &'static str = Self::NAME;
    type Args = BrowserAutomationArgs;
    type Output = BrowserAutomationOutput;
    type Error = BrowserAutomationError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(BrowserAutomationArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        validate_args(&args)?;
        let action_name = serde_json::to_value(&args.action)
            .ok()
            .and_then(|v| v.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| "unknown".to_string());

        let data = dispatch_browser_automation(args).await?;

        Ok(BrowserAutomationOutput {
            action: action_name,
            success: true,
            data,
        })
    }
}

//! Browser automation tools for AI assistant
//!
//! Provides browser_* tools that enable AI to perform web tasks like:
//! - Booking tickets
//! - Information retrieval
//! - Form filling
//! - Web scraping

use crate::agent_browser::{get_browser_service, ScrollDirection, SnapshotOptions};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// Tool name and description constants
pub mod constants {
    pub const BROWSER_OPEN_NAME: &str = "browser_open";
    pub const BROWSER_OPEN_DESC: &str = "Open a URL in browser and get page snapshot. Use this to start web tasks like booking tickets, searching information, or filling forms.";
    
    pub const BROWSER_SNAPSHOT_NAME: &str = "browser_snapshot";
    pub const BROWSER_SNAPSHOT_DESC: &str = "Get current page structure as accessibility tree with refs. Each interactive element has a ref like @e1, @e2 for interaction.";
    
    pub const BROWSER_CLICK_NAME: &str = "browser_click";
    pub const BROWSER_CLICK_DESC: &str = "Click an element by ref (@e1) or CSS selector.";
    
    pub const BROWSER_FILL_NAME: &str = "browser_fill";
    pub const BROWSER_FILL_DESC: &str = "Fill text into an input field by ref or selector.";
    
    pub const BROWSER_TYPE_NAME: &str = "browser_type";
    pub const BROWSER_TYPE_DESC: &str = "Type text character by character into an element.";
    
    pub const BROWSER_SELECT_NAME: &str = "browser_select";
    pub const BROWSER_SELECT_DESC: &str = "Select an option from a dropdown.";
    
    pub const BROWSER_SCROLL_NAME: &str = "browser_scroll";
    pub const BROWSER_SCROLL_DESC: &str = "Scroll the page in a direction.";
    
    pub const BROWSER_WAIT_NAME: &str = "browser_wait";
    pub const BROWSER_WAIT_DESC: &str = "Wait for an element to appear or for a timeout.";
    
    pub const BROWSER_GET_TEXT_NAME: &str = "browser_get_text";
    pub const BROWSER_GET_TEXT_DESC: &str = "Get the text content of an element.";
    
    pub const BROWSER_SCREENSHOT_NAME: &str = "browser_screenshot";
    pub const BROWSER_SCREENSHOT_DESC: &str = "Take a screenshot of the current page.";
    
    pub const BROWSER_BACK_NAME: &str = "browser_back";
    pub const BROWSER_BACK_DESC: &str = "Navigate back to the previous page.";
    
    pub const BROWSER_PRESS_NAME: &str = "browser_press";
    pub const BROWSER_PRESS_DESC: &str = "Press a keyboard key (Enter, Tab, Escape, etc.).";
    
    pub const BROWSER_HOVER_NAME: &str = "browser_hover";
    pub const BROWSER_HOVER_DESC: &str = "Hover over an element.";
    
    pub const BROWSER_EVALUATE_NAME: &str = "browser_evaluate";
    pub const BROWSER_EVALUATE_DESC: &str = "Execute JavaScript code in the browser.";
    
    pub const BROWSER_GET_URL_NAME: &str = "browser_get_url";
    pub const BROWSER_GET_URL_DESC: &str = "Get the current page URL and title.";
    
    pub const BROWSER_CLOSE_NAME: &str = "browser_close";
    pub const BROWSER_CLOSE_DESC: &str = "Close the browser.";
}

/// Browser open arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserOpenArgs {
    pub url: String,
    #[serde(default)]
    pub wait_until: Option<String>,
    /// Whether to run in headless mode (true) or show browser window (false)
    #[serde(default = "default_open_headless")]
    pub headless: bool,
}

fn default_open_headless() -> bool {
    true
}

/// Browser snapshot arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSnapshotArgs {
    #[serde(default = "default_true")]
    pub interactive_only: bool,
    #[serde(default = "default_true")]
    pub compact: bool,
    pub max_depth: Option<u32>,
    pub selector: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Browser click arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserClickArgs {
    /// Element ref (@e1) or CSS selector
    pub target: String,
}

/// Browser fill arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserFillArgs {
    /// Element ref (@e1) or CSS selector
    pub target: String,
    /// Value to fill
    pub value: String,
}

/// Browser type arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTypeArgs {
    /// Element ref (@e1) or CSS selector
    pub target: String,
    /// Text to type
    pub text: String,
    /// Delay between keystrokes in ms
    pub delay_ms: Option<u32>,
}

/// Browser select arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSelectArgs {
    /// Element ref (@e1) or CSS selector
    pub target: String,
    /// Value(s) to select
    pub value: String,
}

/// Browser scroll arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserScrollArgs {
    #[serde(default)]
    pub direction: String,
    pub amount: Option<u32>,
}

/// Browser wait arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserWaitArgs {
    /// Selector to wait for
    pub selector: Option<String>,
    /// Timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    30000
}

/// Browser get text arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserGetTextArgs {
    /// Element ref (@e1) or CSS selector
    pub target: String,
}

/// Browser evaluate arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserEvaluateArgs {
    /// JavaScript code to execute
    pub script: String,
}

/// Browser screenshot arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserScreenshotArgs {
    #[serde(default)]
    pub full_page: bool,
}

/// Browser press key arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserPressArgs {
    /// Key to press (e.g., "Enter", "Tab", "Escape")
    pub key: String,
    /// Optional target element
    pub target: Option<String>,
}

/// Browser hover arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserHoverArgs {
    /// Element ref (@e1) or CSS selector
    pub target: String,
}

// ==================== Tool Executors ====================

/// Normalize URL by adding http:// prefix if missing
fn normalize_url(url: &str) -> String {
    let url = url.trim();
    
    // Check if URL already has a protocol
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }
    
    // Check if it looks like a file path
    if url.starts_with("file://") {
        return url.to_string();
    }
    
    // Add https:// prefix for domain-like URLs
    format!("https://{}", url)
}

/// Execute browser_open
pub async fn execute_browser_open(args: Value) -> Result<Value, String> {
    let args: BrowserOpenArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    // Normalize URL
    let normalized_url = normalize_url(&args.url);

    let service = get_browser_service().await;
    let mut service = service.write().await;

    // Set headless mode
    let headless = args.headless;

    let result = service
        .open(&normalized_url, args.wait_until.as_deref(), Some(headless))
        .await
        .map_err(|e| format!("Failed to open URL: {}", e))?;

    // Also get snapshot for context
    let snapshot = service
        .snapshot(SnapshotOptions::interactive())
        .await
        .map_err(|e| format!("Failed to get snapshot: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "url": result.url,
        "title": result.title,
        "snapshot": snapshot.tree,
        "refs_count": snapshot.refs.len(),
        "hint": "Use @e1, @e2 etc. refs from snapshot with browser_click/browser_fill"
    }))
}

/// Execute browser_snapshot
pub async fn execute_browser_snapshot(args: Value) -> Result<Value, String> {
    let args: BrowserSnapshotArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let service = get_browser_service().await;
    let mut service = service.write().await;

    let options = SnapshotOptions {
        interactive: args.interactive_only,
        compact: args.compact,
        max_depth: args.max_depth,
        selector: args.selector,
    };

    let snapshot = service
        .snapshot(options)
        .await
        .map_err(|e| format!("Failed to get snapshot: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "snapshot": snapshot.tree,
        "refs_count": snapshot.refs.len(),
        "hint": "Use @e1, @e2 etc. refs with browser_click/browser_fill"
    }))
}

/// Execute browser_click
pub async fn execute_browser_click(args: Value) -> Result<Value, String> {
    let args: BrowserClickArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let service = get_browser_service().await;
    let mut service = service.write().await;

    service
        .click(&args.target)
        .await
        .map_err(|e| format!("Failed to click: {}", e))?;

    // Get new snapshot after click
    let snapshot = service
        .snapshot(SnapshotOptions::interactive())
        .await
        .ok();

    let mut result = serde_json::json!({
        "success": true,
        "clicked": args.target
    });

    if let Some(snap) = snapshot {
        result["snapshot"] = serde_json::json!(snap.tree);
        result["refs_count"] = serde_json::json!(snap.refs.len());
    }

    Ok(result)
}

/// Execute browser_fill
pub async fn execute_browser_fill(args: Value) -> Result<Value, String> {
    let args: BrowserFillArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let service = get_browser_service().await;
    let mut service = service.write().await;

    service
        .fill(&args.target, &args.value)
        .await
        .map_err(|e| format!("Failed to fill: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "filled": args.target,
        "value": args.value
    }))
}

/// Execute browser_type
pub async fn execute_browser_type(args: Value) -> Result<Value, String> {
    let args: BrowserTypeArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let service = get_browser_service().await;
    let mut service = service.write().await;

    service
        .type_text(&args.target, &args.text, args.delay_ms)
        .await
        .map_err(|e| format!("Failed to type: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "typed": args.text,
        "target": args.target
    }))
}

/// Execute browser_select
pub async fn execute_browser_select(args: Value) -> Result<Value, String> {
    let args: BrowserSelectArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let service = get_browser_service().await;
    let mut service = service.write().await;

    service
        .select(&args.target, &[args.value.as_str()])
        .await
        .map_err(|e| format!("Failed to select: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "selected": args.value,
        "target": args.target
    }))
}

/// Execute browser_scroll
pub async fn execute_browser_scroll(args: Value) -> Result<Value, String> {
    let args: BrowserScrollArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let direction = match args.direction.to_lowercase().as_str() {
        "up" => ScrollDirection::Up,
        "left" => ScrollDirection::Left,
        "right" => ScrollDirection::Right,
        _ => ScrollDirection::Down,
    };

    let service = get_browser_service().await;
    let mut service = service.write().await;

    service
        .scroll(direction, args.amount)
        .await
        .map_err(|e| format!("Failed to scroll: {}", e))?;

    // Get new snapshot after scroll
    let snapshot = service
        .snapshot(SnapshotOptions::interactive())
        .await
        .ok();

    let mut result = serde_json::json!({
        "success": true,
        "scrolled": args.direction
    });

    if let Some(snap) = snapshot {
        result["snapshot"] = serde_json::json!(snap.tree);
        result["refs_count"] = serde_json::json!(snap.refs.len());
    }

    Ok(result)
}

/// Execute browser_wait
pub async fn execute_browser_wait(args: Value) -> Result<Value, String> {
    let args: BrowserWaitArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let service = get_browser_service().await;
    let mut service = service.write().await;

    service
        .wait(args.selector.as_deref(), Some(args.timeout_ms))
        .await
        .map_err(|e| format!("Failed to wait: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "waited": true
    }))
}

/// Execute browser_get_text
pub async fn execute_browser_get_text(args: Value) -> Result<Value, String> {
    let args: BrowserGetTextArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let service = get_browser_service().await;
    let mut service = service.write().await;

    let text = service
        .get_text(&args.target)
        .await
        .map_err(|e| format!("Failed to get text: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "text": text
    }))
}

/// Execute browser_screenshot
pub async fn execute_browser_screenshot(args: Value) -> Result<Value, String> {
    let args: BrowserScreenshotArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let service = get_browser_service().await;
    let mut service = service.write().await;

    let result = service
        .screenshot(args.full_page)
        .await
        .map_err(|e| format!("Failed to take screenshot: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "base64": result.base64,
        "path": result.path
    }))
}

/// Execute browser_back
pub async fn execute_browser_back(_args: Value) -> Result<Value, String> {
    let service = get_browser_service().await;
    let mut service = service.write().await;

    let url = service
        .back()
        .await
        .map_err(|e| format!("Failed to go back: {}", e))?;

    // Get new snapshot
    let snapshot = service
        .snapshot(SnapshotOptions::interactive())
        .await
        .ok();

    let mut result = serde_json::json!({
        "success": true,
        "url": url
    });

    if let Some(snap) = snapshot {
        result["snapshot"] = serde_json::json!(snap.tree);
        result["refs_count"] = serde_json::json!(snap.refs.len());
    }

    Ok(result)
}

/// Execute browser_evaluate
pub async fn execute_browser_evaluate(args: Value) -> Result<Value, String> {
    let args: BrowserEvaluateArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let service = get_browser_service().await;
    let mut service = service.write().await;

    let result = service
        .evaluate(&args.script)
        .await
        .map_err(|e| format!("Failed to evaluate: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "result": result
    }))
}

/// Execute browser_close
pub async fn execute_browser_close(_args: Value) -> Result<Value, String> {
    let service = get_browser_service().await;
    let mut service = service.write().await;

    service
        .close()
        .await
        .map_err(|e| format!("Failed to close browser: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "closed": true
    }))
}

/// Execute browser_press
pub async fn execute_browser_press(args: Value) -> Result<Value, String> {
    let args: BrowserPressArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let service = get_browser_service().await;
    let mut service = service.write().await;

    service
        .press(&args.key, args.target.as_deref())
        .await
        .map_err(|e| format!("Failed to press key: {}", e))?;

    // Get new snapshot after key press
    let snapshot = service
        .snapshot(SnapshotOptions::interactive())
        .await
        .ok();

    let mut result = serde_json::json!({
        "success": true,
        "pressed": args.key
    });

    if let Some(snap) = snapshot {
        result["snapshot"] = serde_json::json!(snap.tree);
        result["refs_count"] = serde_json::json!(snap.refs.len());
    }

    Ok(result)
}

/// Execute browser_hover
pub async fn execute_browser_hover(args: Value) -> Result<Value, String> {
    let args: BrowserHoverArgs =
        serde_json::from_value(args).map_err(|e| format!("Invalid arguments: {}", e))?;

    let service = get_browser_service().await;
    let mut service = service.write().await;

    service
        .hover(&args.target)
        .await
        .map_err(|e| format!("Failed to hover: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "hovered": args.target
    }))
}

/// Execute browser_get_url
pub async fn execute_browser_get_url(_args: Value) -> Result<Value, String> {
    let service = get_browser_service().await;
    let mut service = service.write().await;

    let url = service
        .get_url()
        .await
        .map_err(|e| format!("Failed to get URL: {}", e))?;

    let title = service.get_title().await.unwrap_or_default();

    Ok(serde_json::json!({
        "success": true,
        "url": url,
        "title": title
    }))
}

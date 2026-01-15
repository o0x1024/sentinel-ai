//! Type definitions for agent-browser integration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Snapshot options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnapshotOptions {
    /// Only show interactive elements (buttons, inputs, links)
    #[serde(default)]
    pub interactive: bool,
    /// Remove empty structural elements
    #[serde(default)]
    pub compact: bool,
    /// Limit tree depth
    pub max_depth: Option<u32>,
    /// CSS selector to scope the snapshot
    pub selector: Option<String>,
}

impl SnapshotOptions {
    pub fn interactive() -> Self {
        Self {
            interactive: true,
            compact: true,
            max_depth: None,
            selector: None,
        }
    }

    pub fn full() -> Self {
        Self {
            interactive: false,
            compact: false,
            max_depth: None,
            selector: None,
        }
    }
}

/// Ref data from snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefData {
    pub role: String,
    pub name: Option<String>,
    pub nth: Option<u32>,
}

/// Snapshot result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// ARIA tree as text
    pub tree: String,
    /// Refs map (e.g., "e1" -> RefData)
    pub refs: HashMap<String, RefData>,
}

/// Navigate result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigateResult {
    pub url: String,
    pub title: String,
}

/// Screenshot result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotResult {
    /// Base64 encoded image
    pub base64: Option<String>,
    /// File path if saved to disk
    pub path: Option<String>,
}

/// Command sent to daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserCommand {
    pub id: String,
    pub action: String,
    #[serde(flatten)]
    pub params: serde_json::Value,
}

/// Response from daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserResponse {
    pub id: String,
    pub success: bool,
    #[serde(default)]
    pub data: serde_json::Value,
    pub error: Option<String>,
}

/// Tab info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub index: u32,
    pub url: String,
    pub title: String,
    pub active: bool,
}

/// Cookie data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub expires: Option<f64>,
    #[serde(rename = "httpOnly")]
    pub http_only: Option<bool>,
    pub secure: Option<bool>,
    #[serde(rename = "sameSite")]
    pub same_site: Option<String>,
}

/// Storage state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageState {
    pub cookies: Vec<Cookie>,
    #[serde(rename = "localStorage", default)]
    pub local_storage: HashMap<String, String>,
    #[serde(rename = "sessionStorage", default)]
    pub session_storage: HashMap<String, String>,
}

/// Browser session config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Session name for isolation
    #[serde(default = "default_session")]
    pub session: String,
    /// Run in headless mode
    #[serde(default = "default_true")]
    pub headless: bool,
    /// Viewport width
    #[serde(default = "default_viewport_width")]
    pub viewport_width: u32,
    /// Viewport height
    #[serde(default = "default_viewport_height")]
    pub viewport_height: u32,
    /// User agent string
    pub user_agent: Option<String>,
    /// Custom executable path
    pub executable_path: Option<String>,
}

fn default_session() -> String {
    "default".to_string()
}

fn default_true() -> bool {
    true
}

fn default_viewport_width() -> u32 {
    1280
}

fn default_viewport_height() -> u32 {
    720
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            session: default_session(),
            headless: false,
            viewport_width: 1280,
            viewport_height: 720,
            user_agent: None,
            executable_path: None,
        }
    }
}

/// Scroll direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

impl Default for ScrollDirection {
    fn default() -> Self {
        Self::Down
    }
}

/// Wait options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WaitOptions {
    /// Selector to wait for
    pub selector: Option<String>,
    /// Text to wait for
    pub text: Option<String>,
    /// URL pattern to wait for
    pub url: Option<String>,
    /// Load state to wait for
    pub load_state: Option<String>,
    /// Timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    30000
}

/// Element bounding box
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Element info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    pub visible: bool,
    pub enabled: bool,
    pub checked: Option<bool>,
    pub text: Option<String>,
    pub value: Option<String>,
    pub bounding_box: Option<BoundingBox>,
}

/// Network request info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub timestamp: u64,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
}

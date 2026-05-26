use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TabId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: TabId,
    pub url: String,
    pub title: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn distance_to(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.width / 2.0,
            y: self.y + self.height / 2.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub x: f64,
    pub y: f64,
    pub button: MouseButton,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseEventKind {
    Moved,
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub kind: KeyEventKind,
    pub key: String,
    pub code: String,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, Copy)]
pub enum KeyEventKind {
    Down,
    Up,
    Char,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

#[derive(Debug, Clone)]
pub struct ScrollEvent {
    pub x: f64,
    pub y: f64,
    pub delta_x: f64,
    pub delta_y: f64,
}

#[derive(Debug, Clone)]
pub struct NavigationResult {
    pub url: String,
    pub status: u16,
    pub load_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ScreenshotOpts {
    pub full_page: bool,
    pub format: ImageFormat,
    pub quality: Option<u8>,
    pub clip: Option<Rect>,
}

#[derive(Debug, Clone, Copy)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<f64>,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RequestMod {
    pub url: Option<String>,
    pub method: Option<String>,
    pub headers: Option<Vec<(String, String)>>,
    pub body: Option<Vec<u8>>,
}

#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    #[error("tab not found: {0}")]
    TabNotFound(String),
    #[error("navigation failed: {0}")]
    NavigationFailed(String),
    #[error("element not found: {0}")]
    ElementNotFound(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("javascript error: {0}")]
    JsError(String),
    #[error("protocol error: {0}")]
    ProtocolError(String),
    #[error("browser not connected")]
    NotConnected,
}

pub type Result<T> = std::result::Result<T, BrowserError>;

use crate::page::accessibility::AccessibilityTree;

/// Unified browser backend trait — abstracts CDP/Juggler protocol differences
#[async_trait]
pub trait BrowserBackend: Send + Sync {
    // --- Lifecycle ---
    async fn connect(&mut self, endpoint: &str) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    fn is_connected(&self) -> bool;

    // --- Navigation ---
    async fn navigate(&self, tab: &TabId, url: &str) -> Result<NavigationResult>;
    async fn reload(&self, tab: &TabId) -> Result<()>;
    async fn go_back(&self, tab: &TabId) -> Result<()>;
    async fn go_forward(&self, tab: &TabId) -> Result<()>;
    async fn current_url(&self, tab: &TabId) -> Result<String>;

    // --- Input (low-level, called by humanization engine) ---
    async fn dispatch_mouse_event(&self, tab: &TabId, event: MouseEvent) -> Result<()>;
    async fn dispatch_key_event(&self, tab: &TabId, event: KeyEvent) -> Result<()>;
    async fn dispatch_scroll_event(&self, tab: &TabId, event: ScrollEvent) -> Result<()>;

    // --- Page State ---
    async fn get_accessibility_tree(&self, tab: &TabId) -> Result<AccessibilityTree>;
    async fn capture_screenshot(&self, tab: &TabId, opts: ScreenshotOpts) -> Result<Vec<u8>>;
    async fn evaluate_js(&self, tab: &TabId, expression: &str) -> Result<serde_json::Value>;

    // --- Tabs ---
    async fn create_tab(&self, url: Option<&str>) -> Result<TabId>;
    async fn close_tab(&self, tab: &TabId) -> Result<()>;
    async fn list_tabs(&self) -> Result<Vec<TabInfo>>;
    async fn activate_tab(&self, tab: &TabId) -> Result<()>;

    // --- Network ---
    async fn intercept_requests(&self, tab: &TabId, patterns: &[String]) -> Result<()>;
    async fn continue_request(
        &self,
        request_id: &str,
        modifications: Option<RequestMod>,
    ) -> Result<()>;
    async fn get_cookies(&self, tab: &TabId) -> Result<Vec<Cookie>>;
    async fn set_cookies(&self, cookies: &[Cookie]) -> Result<()>;
}

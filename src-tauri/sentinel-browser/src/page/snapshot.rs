use serde::{Deserialize, Serialize};

/// Mode for capturing page state
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PageCaptureMode {
    /// Accessibility tree only (token-efficient, preferred)
    AccessibilityTree,
    /// Screenshot (for visual/canvas content)
    Screenshot,
    /// Full DOM snapshot
    Dom,
    /// Extracted readable text as markdown
    Markdown,
}

/// Complete page snapshot combining multiple modalities
#[derive(Debug, Clone, Serialize)]
pub struct PageSnapshot {
    pub url: String,
    pub title: String,
    pub mode: PageCaptureMode,
    /// Accessibility tree text (when mode = AccessibilityTree)
    pub a11y_text: Option<String>,
    /// Base64 screenshot (when mode = Screenshot)
    pub screenshot_base64: Option<String>,
    /// DOM HTML (when mode = Dom)
    pub dom_html: Option<String>,
    /// Markdown text (when mode = Markdown)
    pub markdown: Option<String>,
    /// Viewport dimensions
    pub viewport_width: u32,
    pub viewport_height: u32,
    /// Scroll position
    pub scroll_x: f64,
    pub scroll_y: f64,
    /// Total page height
    pub page_height: f64,
}

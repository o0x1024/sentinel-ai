pub mod browser;
pub mod browser_scripts;
pub mod navigator;
pub mod operator;

use crate::engines::vision_explorer_v2::core::PageContext;
use anyhow::Result;
use async_trait::async_trait;

/// Browser action interface for Vision Explorer V2
///
/// Designed to work with mcp-playwright-security MCP server
/// which provides `playwright_*` prefixed tools.
#[async_trait]
pub trait BrowserActions: Send + Sync {
    /// Navigate to a URL
    async fn goto(&self, url: &str) -> Result<()>;

    /// Type text into an input field by selector
    async fn type_text(&self, selector: &str, text: &str) -> Result<()>;

    /// Click an element by selector
    async fn click(&self, selector: &str) -> Result<()>;

    /// Capture the current page context (URL, title, screenshot, DOM)
    async fn capture_context(&self) -> Result<PageContext>;

    // ============ Extended actions ============

    /// Hover over an element
    async fn hover(&self, selector: &str) -> Result<()> {
        // Default implementation - subclasses can override
        let _ = selector;
        Err(anyhow::anyhow!("hover not implemented"))
    }

    /// Press a keyboard key
    async fn press_key(&self, key: &str, selector: Option<&str>) -> Result<()> {
        // Default implementation
        let _ = (key, selector);
        Err(anyhow::anyhow!("press_key not implemented"))
    }

    /// Click by screen coordinates
    async fn click_coordinate(&self, x: i32, y: i32) -> Result<()> {
        // Default implementation
        let _ = (x, y);
        Err(anyhow::anyhow!("click_coordinate not implemented"))
    }

    /// Annotate all interactive elements and return their info
    async fn annotate(&self) -> Result<serde_json::Value> {
        Err(anyhow::anyhow!("annotate not implemented"))
    }

    /// Click by annotation index (requires annotate first)
    async fn click_by_index(&self, index: usize) -> Result<()> {
        let _ = index;
        Err(anyhow::anyhow!("click_by_index not implemented"))
    }

    /// Fill input by annotation index (requires annotate first)
    async fn fill_by_index(&self, index: usize, value: &str) -> Result<()> {
        let _ = (index, value);
        Err(anyhow::anyhow!("fill_by_index not implemented"))
    }

    /// Get visible text content of the page
    async fn get_visible_text(&self) -> Result<String> {
        Err(anyhow::anyhow!("get_visible_text not implemented"))
    }

    /// Get HTML content of the page
    async fn get_visible_html(&self, selector: Option<&str>) -> Result<String> {
        let _ = selector;
        Err(anyhow::anyhow!("get_visible_html not implemented"))
    }

    /// Navigate back in browser history
    async fn go_back(&self) -> Result<()> {
        Err(anyhow::anyhow!("go_back not implemented"))
    }

    /// Navigate forward in browser history
    async fn go_forward(&self) -> Result<()> {
        Err(anyhow::anyhow!("go_forward not implemented"))
    }

    /// Select option in a dropdown
    async fn select(&self, selector: &str, value: &str) -> Result<()> {
        let _ = (selector, value);
        Err(anyhow::anyhow!("select not implemented"))
    }
}

pub use browser::BrowserDriver;
pub use navigator::NavigatorAgent;
pub use operator::OperatorAgent;

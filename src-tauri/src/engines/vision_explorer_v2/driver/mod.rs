pub mod browser;
pub mod browser_scripts;
pub mod navigator;
pub mod operator;

use crate::engines::vision_explorer_v2::core::PageContext;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait BrowserActions: Send {
    async fn goto(&self, url: &str) -> Result<()>;
    async fn type_text(&self, selector: &str, text: &str) -> Result<()>;
    async fn click(&self, selector: &str) -> Result<()>;
    async fn capture_context(&self) -> Result<PageContext>;
}

pub use browser::BrowserDriver;
pub use navigator::NavigatorAgent;
pub use operator::OperatorAgent;

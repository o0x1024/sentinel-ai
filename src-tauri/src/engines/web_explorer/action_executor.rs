//! Action Executor - Executes actions on the browser using AgentBrowserService
//!
//! This module is responsible for the "Act" step in ReAct loop
//! Uses agent-browser for browser automation with snapshot-based element selection

use super::types::{Action, ActionResult, ScrollDirection};
use anyhow::Result;
use sentinel_tools::agent_browser::{get_browser_service, SnapshotOptions};
use tracing::{debug, info, warn};

/// Action executor using AgentBrowserService
pub struct ActionExecutor;

impl ActionExecutor {
    /// Create a new action executor
    pub fn new() -> Self {
        Self
    }

    // ==================== Core Actions ====================

    /// Execute an action and return the result
    pub async fn execute(&self, action: Action) -> Result<ActionResult> {
        info!("Executing action: {:?}", action);

        match action {
            Action::Navigate { ref url } => self.execute_navigate(url).await,
            Action::Click {
                ref selector,
                ref index,
                x,
                y,
            } => self.execute_click(selector, index, x, y).await,
            Action::Fill {
                ref selector,
                ref index,
                ref value,
            } => self.execute_fill(selector, index, value).await,
            Action::Submit { ref selector } => self.execute_submit(selector).await,
            Action::Scroll { direction, amount } => self.execute_scroll(direction, amount).await,
            Action::Wait { duration_ms } => self.execute_wait(duration_ms).await,
            Action::TakeSnapshot => self.execute_snapshot().await,
            Action::GoBack => self.execute_go_back().await,
            Action::Stop { reason: _ } => Ok(ActionResult {
                success: true,
                new_url: None,
                error: None,
                observation: None,
            }),
        }
    }

    /// Execute navigate action
    async fn execute_navigate(&self, url: &str) -> Result<ActionResult> {
        debug!("Navigating to: {}", url);

        let service = get_browser_service().await;
        let mut service = service.write().await;

        match service.open(url, Some("load")).await {
            Ok(result) => {
                info!("Navigation successful to: {}", result.url);
                
                // Wait briefly for any dynamic content
                let _ = service.wait(None, Some(500)).await;
                debug!("Wait completed after navigation");

                // Observation will be captured in the main observe() loop
                Ok(ActionResult {
                    success: true,
                    new_url: Some(result.url),
                    error: None,
                    observation: None,
                })
            }
            Err(e) => {
                warn!("Navigation failed: {}", e);
                Ok(ActionResult {
                    success: false,
                    new_url: None,
                    error: Some(format!("Navigation failed: {}", e)),
                    observation: None,
                })
            }
        }
    }

    /// Execute click action - supports ref (@e1), selector, or coordinates
    async fn execute_click(
        &self,
        selector: &Option<String>,
        index: &Option<u32>,
        x: Option<i32>,
        y: Option<i32>,
    ) -> Result<ActionResult> {
        debug!(
            "Clicking: index={:?}, selector={:?}, coords=({:?}, {:?})",
            index, selector, x, y
        );

        let service = get_browser_service().await;
        let mut service = service.write().await;

        // NOTE: Do NOT take snapshot here! The index is based on the snapshot
        // that was taken during the observe phase. Taking a new snapshot would
        // regenerate the refMap and the index would point to a different element.

        // Build target: prefer index (as ref), then selector, then coordinates
        let target = if let Some(idx) = index {
            // Convert index to ref format
            format!("@e{}", idx)
        } else if let Some(sel) = selector {
            sel.clone()
        } else if let (Some(x_coord), Some(y_coord)) = (x, y) {
            // For coordinates, use JavaScript click
            let script = format!(
                "document.elementFromPoint({}, {})?.click()",
                x_coord, y_coord
            );
            match service.evaluate(&script).await {
                Ok(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    // Get new URL after click
                    let new_url = service.get_url().await.ok();
                    return Ok(ActionResult {
                        success: true,
                        new_url,
                        error: None,
                        observation: None,
                    });
                }
                Err(e) => {
                    return Ok(ActionResult {
                        success: false,
                        new_url: None,
                        error: Some(format!("Click by coordinates failed: {}", e)),
                        observation: None,
                    });
                }
            }
        } else {
            return Ok(ActionResult {
                success: false,
                new_url: None,
                error: Some("Click requires index, selector, or coordinates".to_string()),
                observation: None,
            });
        };

        match service.click(&target).await {
            Ok(_) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                // Get new URL after click - page may have navigated
                let new_url = service.get_url().await.ok();
                Ok(ActionResult {
                    success: true,
                    new_url,
                    error: None,
                    observation: None,
                })
            }
            Err(e) => Ok(ActionResult {
                success: false,
                new_url: None,
                error: Some(format!("Click failed: {}", e)),
                observation: None,
            }),
        }
    }

    /// Execute fill action - supports ref or selector
    async fn execute_fill(
        &self,
        selector: &Option<String>,
        index: &Option<u32>,
        value: &str,
    ) -> Result<ActionResult> {
        debug!(
            "Filling: index={:?}, selector={:?}, value={}",
            index, selector, value
        );

        let service = get_browser_service().await;
        let mut service = service.write().await;

        let target = if let Some(idx) = index {
            format!("@e{}", idx)
        } else if let Some(sel) = selector {
            sel.clone()
        } else {
            return Ok(ActionResult {
                success: false,
                new_url: None,
                error: Some("Fill requires index or selector".to_string()),
                observation: None,
            });
        };

        match service.fill(&target, value).await {
            Ok(_) => Ok(ActionResult {
                success: true,
                new_url: None,
                error: None,
                observation: None,
            }),
            Err(e) => Ok(ActionResult {
                success: false,
                new_url: None,
                error: Some(format!("Fill failed: {}", e)),
                observation: None,
            }),
        }
    }

    /// Execute submit action (press Enter)
    async fn execute_submit(&self, selector: &str) -> Result<ActionResult> {
        debug!("Submitting form: {}", selector);

        let service = get_browser_service().await;
        let mut service = service.write().await;

        match service.press("Enter", Some(selector)).await {
            Ok(_) => {
                // Wait briefly for form submission
                let _ = service.wait(None, Some(1000)).await;
                // Get new URL after submit - form may have navigated
                let new_url = service.get_url().await.ok();
                Ok(ActionResult {
                    success: true,
                    new_url,
                    error: None,
                    observation: None,
                })
            }
            Err(e) => Ok(ActionResult {
                success: false,
                new_url: None,
                error: Some(format!("Submit failed: {}", e)),
                observation: None,
            }),
        }
    }

    /// Execute scroll action
    async fn execute_scroll(&self, direction: ScrollDirection, amount: u32) -> Result<ActionResult> {
        debug!("Scrolling: {:?} by {}", direction, amount);

        let service = get_browser_service().await;
        let mut service = service.write().await;

        let scroll_dir = match direction {
            ScrollDirection::Down => sentinel_tools::agent_browser::ScrollDirection::Down,
            ScrollDirection::Up => sentinel_tools::agent_browser::ScrollDirection::Up,
            ScrollDirection::Left => sentinel_tools::agent_browser::ScrollDirection::Left,
            ScrollDirection::Right => sentinel_tools::agent_browser::ScrollDirection::Right,
        };

        match service.scroll(scroll_dir, Some(amount)).await {
            Ok(_) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                Ok(ActionResult {
                    success: true,
                    new_url: None,
                    error: None,
                    observation: None,
                })
            }
            Err(e) => Ok(ActionResult {
                success: false,
                new_url: None,
                error: Some(format!("Scroll failed: {}", e)),
                observation: None,
            }),
        }
    }

    /// Execute wait action
    async fn execute_wait(&self, duration_ms: u64) -> Result<ActionResult> {
        debug!("Waiting for {}ms", duration_ms);
        tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
        Ok(ActionResult {
            success: true,
            new_url: None,
            error: None,
            observation: None,
        })
    }

    /// Execute snapshot action
    async fn execute_snapshot(&self) -> Result<ActionResult> {
        debug!("Taking snapshot");
        // Snapshot is taken in observe() loop, this just signals success
        Ok(ActionResult {
            success: true,
            new_url: None,
            error: None,
            observation: None,
        })
    }

    /// Execute go back action
    async fn execute_go_back(&self) -> Result<ActionResult> {
        debug!("Going back");

        let service = get_browser_service().await;
        let mut service = service.write().await;

        match service.back().await {
            Ok(_) => {
                let _ = service.wait(None, Some(500)).await;
                // Get new URL after go back
                let new_url = service.get_url().await.ok();
                Ok(ActionResult {
                    success: true,
                    new_url,
                    error: None,
                    observation: None,
                })
            }
            Err(e) => Ok(ActionResult {
                success: false,
                new_url: None,
                error: Some(format!("Go back failed: {}", e)),
                observation: None,
            }),
        }
    }

    // ==================== Snapshot-based Methods ====================

    /// Get page snapshot (ARIA tree with refs)
    pub async fn get_snapshot(&self) -> Result<sentinel_tools::agent_browser::Snapshot> {
        let service = get_browser_service().await;
        let mut service = service.write().await;
        service.snapshot(SnapshotOptions::interactive()).await
    }

    /// Get full snapshot with all elements
    pub async fn get_full_snapshot(&self) -> Result<sentinel_tools::agent_browser::Snapshot> {
        let service = get_browser_service().await;
        let mut service = service.write().await;
        service.snapshot(SnapshotOptions::full()).await
    }

    // ==================== Network & Storage ====================

    /// Enable network request interception for API discovery
    pub async fn enable_network_interception(&self) -> Result<()> {
        let service = get_browser_service().await;
        let mut service = service.write().await;
        service.start_network_tracking().await?;
        info!("Network interception enabled for API discovery");
        Ok(())
    }

    /// Get discovered API endpoints from network interception
    pub async fn get_discovered_apis(&self) -> Result<Vec<String>> {
        let service = get_browser_service().await;
        let mut service = service.write().await;
        let apis = service.get_discovered_apis().await?;
        debug!("Discovered {} API endpoints", apis.len());
        Ok(apis)
    }

}

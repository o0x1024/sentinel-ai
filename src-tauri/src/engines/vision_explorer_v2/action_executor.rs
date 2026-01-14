//! Action Executor - Executes actions on the browser using MCP Playwright
//!
//! This module is responsible for the "Act" step in ReAct loop
//! Updated for hybrid exploration (Vision + DOM annotation)

use super::perception::PerceptionEngine;
use super::types::{Action, ActionResult, PageContext, ScrollDirection, SiteProfile, StorageState};
use crate::services::mcp::McpService;
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, info};

/// Action executor using MCP Playwright (hybrid mode)
pub struct ActionExecutor {
    mcp_service: Arc<McpService>,
    perception_engine: Arc<PerceptionEngine>,
    mcp_server_name: String,
}

impl ActionExecutor {
    /// Create a new action executor
    pub fn new(
        mcp_service: Arc<McpService>,
        perception_engine: Arc<PerceptionEngine>,
        mcp_server_name: String,
    ) -> Self {
        Self {
            mcp_service,
            perception_engine,
            mcp_server_name,
        }
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

        let result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_navigate",
                json!({
                    "url": url,
                    "waitUntil": "load",
                }),
            )
            .await;

        match result {
            Ok(_) => {
                // Wait for network idle after navigation
                let _ = self.wait_for_network_idle(3000).await;

                let observation = self.capture_observation().await.ok();
                Ok(ActionResult {
                    success: true,
                    new_url: Some(url.to_string()),
                    error: None,
                    observation,
                })
            }
            Err(e) => Ok(ActionResult {
                success: false,
                new_url: None,
                error: Some(format!("Navigation failed: {}", e)),
                observation: None,
            }),
        }
    }

    /// Execute click action - supports index, selector, or coordinates
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

        // Priority: index > selector > coordinates
        let result = if let Some(idx) = index {
            // Click by annotation index (most reliable for hybrid mode)
            self.mcp_service
                .execute_client_tool(
                    &self.mcp_server_name,
                    "playwright_click",
                    json!({ "index": idx }),
                )
                .await
        } else if let Some(sel) = selector {
            // Click by selector
            self.mcp_service
                .execute_client_tool(
                    &self.mcp_server_name,
                    "playwright_click",
                    json!({ "selector": sel }),
                )
                .await
        } else if let (Some(x_coord), Some(y_coord)) = (x, y) {
            // Click by coordinates (vision fallback)
            self.mcp_service
                .execute_client_tool(
                    &self.mcp_server_name,
                    "playwright_click",
                    json!({ "coordinate": [x_coord, y_coord] }),
                )
                .await
        } else {
            return Ok(ActionResult {
                success: false,
                new_url: None,
                error: Some("Click requires index, selector, or coordinates".to_string()),
                observation: None,
            });
        };

        match result {
            Ok(_) => {
                // Wait briefly for any state changes
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                let observation = self.capture_observation().await.ok();
                Ok(ActionResult {
                    success: true,
                    new_url: None,
                    error: None,
                    observation,
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

    /// Execute fill action - supports index or selector
    async fn execute_fill(
        &self,
        selector: &Option<String>,
        index: &Option<u32>,
        value: &str,
    ) -> Result<ActionResult> {
        debug!("Filling: index={:?}, selector={:?}, value={}", index, selector, value);

        // Priority: index > selector
        let result = if let Some(idx) = index {
            self.mcp_service
                .execute_client_tool(
                    &self.mcp_server_name,
                    "playwright_fill",
                    json!({ "index": idx, "value": value }),
                )
                .await
        } else if let Some(sel) = selector {
            self.mcp_service
                .execute_client_tool(
                    &self.mcp_server_name,
                    "playwright_fill",
                    json!({ "selector": sel, "value": value }),
                )
                .await
        } else {
            return Ok(ActionResult {
                success: false,
                new_url: None,
                error: Some("Fill requires index or selector".to_string()),
                observation: None,
            });
        };

        match result {
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

        let result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_press_key",
                json!({
                    "selector": selector,
                    "key": "Enter",
                }),
            )
            .await;

        match result {
            Ok(_) => {
                // Wait for form submission
                let _ = self.wait_for_network_idle(3000).await;
                let observation = self.capture_observation().await.ok();
                Ok(ActionResult {
                    success: true,
                    new_url: None,
                    error: None,
                    observation,
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
    async fn execute_scroll(
        &self,
        direction: ScrollDirection,
        amount: u32,
    ) -> Result<ActionResult> {
        debug!("Scrolling: {:?} by {}", direction, amount);

        let dir = match direction {
            ScrollDirection::Down => "down",
            ScrollDirection::Up => "up",
            ScrollDirection::Left => "down", // Treat as down for simplicity
            ScrollDirection::Right => "down",
        };

        let result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_scroll",
                json!({
                    "direction": dir,
                    "amount": amount,
                }),
            )
            .await;

        match result {
            Ok(_) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                let observation = self.capture_observation().await.ok();
                Ok(ActionResult {
                    success: true,
                    new_url: None,
                    error: None,
                    observation,
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
        let observation = self.capture_observation().await.ok();
        Ok(ActionResult {
            success: true,
            new_url: None,
            error: None,
            observation,
        })
    }

    /// Execute go back action
    async fn execute_go_back(&self) -> Result<ActionResult> {
        debug!("Going back");

        let result = self
            .mcp_service
            .execute_client_tool(&self.mcp_server_name, "playwright_go_back", json!({}))
            .await;

        match result {
            Ok(_) => {
                let _ = self.wait_for_network_idle(2000).await;
                let observation = self.capture_observation().await.ok();
                Ok(ActionResult {
                    success: true,
                    new_url: None,
                    error: None,
                    observation,
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

    // ==================== Hybrid Exploration Methods ====================

    /// Enable network request interception for API discovery
    pub async fn enable_network_interception(&self) -> Result<()> {
        self.mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_intercept_requests",
                json!({ "enabled": true }),
            )
            .await?;
        Ok(())
    }

    /// Get discovered API endpoints from network interception
    pub async fn get_discovered_apis(&self) -> Result<Vec<String>> {
        let result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_get_requests",
                json!({ "limit": 200 }),
            )
            .await?;

        // Parse apis from response
        let apis = self
            .extract_text_from_response(&result)
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .and_then(|v| v.get("apis").cloned())
            .and_then(|apis| apis.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        Ok(apis)
    }

    /// Detect site type (SPA/MPA) and framework
    pub async fn detect_site_type(&self) -> Result<SiteProfile> {
        let result = self
            .mcp_service
            .execute_client_tool(&self.mcp_server_name, "playwright_detect_site", json!({}))
            .await?;

        let text = self
            .extract_text_from_response(&result)
            .unwrap_or_else(|| "{}".to_string());

        serde_json::from_str(&text).context("Failed to parse site profile")
    }

    /// Wait for network to become idle (for SPAs)
    pub async fn wait_for_network_idle(&self, timeout_ms: u64) -> Result<()> {
        let _ = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_wait_for_network_idle",
                json!({ "timeout": timeout_ms }),
            )
            .await;
        Ok(())
    }

    /// Wait for a specific selector to appear
    pub async fn wait_for_selector(&self, selector: &str, timeout_ms: u64) -> Result<bool> {
        let result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_wait_for_selector",
                json!({
                    "selector": selector,
                    "timeout": timeout_ms,
                    "state": "visible"
                }),
            )
            .await;

        Ok(result.is_ok())
    }

    /// Get browser storage (for session detection)
    pub async fn get_storage(&self) -> Result<StorageState> {
        let result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_get_storage",
                json!({ "type": "all" }),
            )
            .await?;

        let text = self
            .extract_text_from_response(&result)
            .unwrap_or_else(|| "{}".to_string());

        Ok(serde_json::from_str(&text).unwrap_or_else(|_| StorageState::default()))
    }

    /// Set browser storage (for session restoration)
    pub async fn set_storage(&self, storage: &StorageState) -> Result<()> {
        self.mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_set_storage",
                json!({
                    "cookies": storage.cookies,
                    "localStorage": storage.local_storage,
                }),
            )
            .await?;
        Ok(())
    }

    /// Get annotated elements (DOM-based element detection)
    pub async fn get_annotated_elements(&self) -> Result<Vec<AnnotatedElement>> {
        let result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_get_annotated_elements",
                json!({}),
            )
            .await?;

        let text = self
            .extract_text_from_response(&result)
            .unwrap_or_else(|| "{}".to_string());

        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();

        let elements = parsed
            .get("elements")
            .and_then(|e| e.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| {
                        Some(AnnotatedElement {
                            index: e.get("i")?.as_u64()? as u32,
                            element_type: e.get("t")?.as_str()?.to_string(),
                            text: e.get("x").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            href: e.get("h").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            name: e.get("n").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            tag: e.get("g").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(elements)
    }

    /// Scroll element into view by index
    pub async fn scroll_into_view(&self, index: u32) -> Result<()> {
        self.mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_scroll",
                json!({ "index": index }),
            )
            .await?;
        Ok(())
    }

    // ==================== Observation Methods ====================

    /// Capture current page observation
    async fn capture_observation(&self) -> Result<super::types::Observation> {
        let context = self.capture_page_context().await?;
        self.perception_engine.analyze(&context).await
    }

    /// Capture current page context (screenshot + HTML)
    pub async fn capture_page_context(&self) -> Result<PageContext> {
        // Get screenshot
        let screenshot_result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_screenshot",
                json!({
                    "name": "vision_explorer",
                    "fullPage": false,
                }),
            )
            .await
            .context("Failed to capture screenshot")?;

        // Parse screenshot from response
        let screenshot_base64 = screenshot_result
            .get("content")
            .and_then(|content| content.as_array())
            .and_then(|arr| {
                arr.iter().find_map(|item| {
                    if item.get("type")?.as_str()? == "text" {
                        let text = item.get("text")?.as_str()?;
                        if let Ok(json_obj) = serde_json::from_str::<serde_json::Value>(text) {
                            return json_obj
                                .get("screenshot_base64")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                    }
                    None
                })
            })
            .or_else(|| {
                screenshot_result
                    .get("content")
                    .and_then(|content| content.as_array())
                    .and_then(|arr| {
                        arr.iter().find_map(|item| {
                            if item.get("type")?.as_str()? == "image" {
                                return item.get("data")?.as_str().map(|s| s.to_string());
                            }
                            None
                        })
                    })
            })
            .context("Screenshot not found in response")?;

        let screenshot =
            BASE64_STANDARD.decode(screenshot_base64).context("Failed to decode screenshot")?;

        // Get HTML
        let html_result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_get_visible_html",
                json!({ "maxLength": 50000 }),
            )
            .await
            .context("Failed to get HTML content")?;

        let html = self
            .extract_text_from_response(&html_result)
            .map(|s| s.strip_prefix("HTML content:\n").unwrap_or(&s).to_string())
            .unwrap_or_default();

        // Get page info
        let info_result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_evaluate",
                json!({
                    "script": "JSON.stringify({ url: window.location.href, title: document.title, width: window.innerWidth, height: window.innerHeight })"
                }),
            )
            .await
            .context("Failed to get page info")?;

        let page_info_str = self
            .extract_text_from_response(&info_result)
            .unwrap_or_else(|| "{}".to_string());

        let page_info: serde_json::Value = serde_json::from_str(&page_info_str).unwrap_or_default();

        Ok(PageContext {
            url: page_info
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            title: page_info
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            screenshot,
            html,
            viewport_width: page_info.get("width").and_then(|v| v.as_u64()).unwrap_or(1280) as u32,
            viewport_height: page_info.get("height").and_then(|v| v.as_u64()).unwrap_or(720) as u32,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        })
    }

    // ==================== Helper Methods ====================

    /// Extract text content from MCP response
    fn extract_text_from_response(&self, result: &serde_json::Value) -> Option<String> {
        result
            .get("content")
            .and_then(|content| content.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
            .map(|s| s.to_string())
    }
}

/// Annotated element from DOM
#[derive(Debug, Clone)]
pub struct AnnotatedElement {
    pub index: u32,
    pub element_type: String,
    pub text: Option<String>,
    pub href: Option<String>,
    pub name: Option<String>,
    pub tag: Option<String>,
}

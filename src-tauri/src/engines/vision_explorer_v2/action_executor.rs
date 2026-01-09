//! Action Executor - Executes actions on the browser using MCP Playwright
//!
//! This module is responsible for the "Act" step in ReAct loop

use super::perception::PerceptionEngine;
use super::types::{Action, ActionResult, PageContext, ScrollDirection};
use crate::services::mcp::McpService;
use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, info};

/// Action executor using MCP Playwright
pub struct ActionExecutor {
    mcp_service: Arc<McpService>,
    perception_engine: Arc<PerceptionEngine>,
    mcp_server_name: String,
}

impl ActionExecutor {
    /// Create a new action executor
    pub fn new(mcp_service: Arc<McpService>, perception_engine: Arc<PerceptionEngine>, mcp_server_name: String) -> Self {
        Self {
            mcp_service,
            perception_engine,
            mcp_server_name,
        }
    }

    /// Execute an action and return the result
    pub async fn execute(&self, action: Action) -> Result<ActionResult> {
        info!("Executing action: {:?}", action);

        match action {
            Action::Navigate { ref url } => self.execute_navigate(url).await,
            Action::Click { ref selector, x, y } => self.execute_click(selector, x, y).await,
            Action::Fill { ref selector, ref value } => self.execute_fill(selector, value).await,
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
                    "waitUntil": "networkidle",
                }),
            )
            .await;

        match result {
            Ok(_) => {
                // Get new page context after navigation
                let observation = self.capture_observation().await.ok();
                let new_url = observation.as_ref().and_then(|_obs| {
                    // Extract URL from observation context
                    Some(url.to_string())
                });

                Ok(ActionResult {
                    success: true,
                    new_url,
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

    /// Execute click action
    async fn execute_click(
        &self,
        selector: &Option<String>,
        x: Option<i32>,
        y: Option<i32>,
    ) -> Result<ActionResult> {
        debug!("Clicking: selector={:?}, coords=({:?}, {:?})", selector, x, y);

        let result = if let Some(sel) = selector {
            // Click by selector
            self.mcp_service
                .execute_client_tool(
                    &self.mcp_server_name,
                    "playwright_click",
                    json!({
                        "selector": sel,
                    }),
                )
                .await
        } else if let (Some(x_coord), Some(y_coord)) = (x, y) {
            // Click by coordinates (pure vision mode)
            self.mcp_service
                .execute_client_tool(
                    &self.mcp_server_name,
                    "playwright_click",
                    json!({
                        "x": x_coord,
                        "y": y_coord,
                    }),
                )
                .await
        } else {
            return Ok(ActionResult {
                success: false,
                new_url: None,
                error: Some("Click requires either selector or coordinates".to_string()),
                observation: None,
            });
        };

        match result {
            Ok(_) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
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

    /// Execute fill action
    async fn execute_fill(&self, selector: &str, value: &str) -> Result<ActionResult> {
        debug!("Filling: {} = {}", selector, value);

        let result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_fill",
                json!({
                    "selector": selector,
                    "value": value,
                }),
            )
            .await;

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

    /// Execute submit action
    async fn execute_submit(&self, selector: &str) -> Result<ActionResult> {
        debug!("Submitting form: {}", selector);

        // Submit by clicking submit button or pressing Enter
        let result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "browser_press_key",
                json!({
                    "selector": selector,
                    "key": "Enter",
                }),
            )
            .await;

        match result {
            Ok(_) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
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
    async fn execute_scroll(&self, direction: ScrollDirection, amount: u32) -> Result<ActionResult> {
        debug!("Scrolling: {:?} by {}", direction, amount);

        let (x, y) = match direction {
            ScrollDirection::Down => (0, amount as i32),
            ScrollDirection::Up => (0, -(amount as i32)),
            ScrollDirection::Right => (amount as i32, 0),
            ScrollDirection::Left => (-(amount as i32), 0),
        };

        let result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "browser_evaluate",
                json!({
                    "script": format!("window.scrollBy({}, {})", x, y),
                }),
            )
            .await;

        match result {
            Ok(_) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
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
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_go_back",
                json!({}),
            )
            .await;

        match result {
            Ok(_) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
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

    /// Capture current page observation
    async fn capture_observation(&self) -> Result<super::types::Observation> {
        let context = self.capture_page_context().await?;
        self.perception_engine.analyze(&context).await
    }

    /// Capture current page context (screenshot + HTML)
    pub async fn capture_page_context(&self) -> Result<PageContext> {
        // Get screenshot using playwright_screenshot
        let screenshot_result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_screenshot",
                json!({
                    "name": "vision_explorer",
                    "storeBase64": true,
                    "fullPage": false,
                }),
            )
            .await
            .context("Failed to capture screenshot")?;

        // Parse screenshot from MCP response
        // The response format is: { "content": [{"type": "text", "text": "..."}, {"type": "text", "text": "{\"screenshot_base64\":\"...\"}"}, {"type": "image", "data": "...", "mimeType": "..."}], "is_error": false }
        let screenshot_base64 = screenshot_result
            .get("content")
            .and_then(|content| content.as_array())
            .and_then(|arr| {
                // Find the text content with screenshot_base64
                arr.iter().find_map(|item| {
                    if item.get("type")?.as_str()? == "text" {
                        let text = item.get("text")?.as_str()?;
                        if let Ok(json_obj) = serde_json::from_str::<serde_json::Value>(text) {
                            return json_obj.get("screenshot_base64")?.as_str().map(|s| s.to_string());
                        }
                    }
                    None
                })
            })
            .or_else(|| {
                // Fallback: try to get from image type directly
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

        let screenshot = BASE64_STANDARD.decode(screenshot_base64)
            .context("Failed to decode screenshot")?;

        // Get page HTML using playwright_get_visible_html
        let html_result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_get_visible_html",
                json!({
                    "cleanHtml": true,
                    "maxLength": 50000,
                }),
            )
            .await
            .context("Failed to get HTML content")?;

        // Parse HTML from response
        let html = html_result
            .get("content")
            .and_then(|content| content.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
            .map(|s| {
                // Remove the "HTML content:\n" prefix if present
                s.strip_prefix("HTML content:\n").unwrap_or(s).to_string()
            })
            .unwrap_or_default();

        // Get page URL and title using playwright_evaluate
        let info_result = self
            .mcp_service
            .execute_client_tool(
                &self.mcp_server_name,
                "playwright_evaluate",
                json!({
                    "script": "JSON.stringify({ url: window.location.href, title: document.title, width: window.innerWidth, height: window.innerHeight })",
                }),
            )
            .await
            .context("Failed to get page info")?;

        // Parse page info from evaluate result
        let page_info_str = info_result
            .get("content")
            .and_then(|content| content.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
            .unwrap_or("{}");

        let page_info: serde_json::Value = serde_json::from_str(page_info_str).unwrap_or_default();

        let url = page_info
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let title = page_info
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let viewport_width = page_info
            .get("width")
            .and_then(|v| v.as_u64())
            .unwrap_or(1280) as u32;

        let viewport_height = page_info
            .get("height")
            .and_then(|v| v.as_u64())
            .unwrap_or(720) as u32;

        Ok(PageContext {
            url,
            title,
            screenshot,
            html,
            viewport_width,
            viewport_height,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        })
    }
}

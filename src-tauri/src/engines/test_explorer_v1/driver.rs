//! Browser driver implementation using Playwright MCP
//!
//! This driver uses McpService to communicate with mcp-playwright-security server
//! which provides `playwright_*` prefixed tools.

use crate::services::mcp::McpService;
use anyhow::{anyhow, Result};
use serde_json::json;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{debug, info};

use super::types::{ApiRequest, InteractiveElement, PageState, TestExplorerV1Config};

/// Browser driver for Test Explorer V1 using MCP
///
/// Communicates with mcp-playwright-security MCP server
/// to perform browser automation actions.
pub struct BrowserDriver {
    mcp_service: Arc<McpService>,
    config: TestExplorerV1Config,
}

impl BrowserDriver {
    /// Create a new browser driver with MCP service
    pub async fn new(config: TestExplorerV1Config) -> Result<Self> {
        info!("Initializing BrowserDriver with MCP backend");

        let mcp_service = Arc::new(McpService::new());

        Ok(Self { mcp_service, config })
    }

    /// Get the name of the connected Playwright MCP server
    async fn get_playwright_conn_name(&self) -> Result<String> {
        let connections = self.mcp_service.get_connection_info().await?;
        let conn = connections
            .iter()
            .find(|c| {
                c.name.to_lowercase().contains("playwright")
                    && c.status.to_lowercase() == "connected"
            })
            .ok_or_else(|| anyhow!("Playwright MCP server not connected"))?;
        Ok(conn.name.clone())
    }

    /// Call a Playwright MCP tool
    async fn call_tool(&self, tool: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let conn_name = self.get_playwright_conn_name().await?;
        debug!(
            "BrowserDriver: Calling tool '{}' with params: {:?}",
            tool, params
        );
        let result = self
            .mcp_service
            .execute_client_tool(&conn_name, tool, params)
            .await?;
        Ok(result)
    }

    /// Execute JavaScript in the browser console
    async fn evaluate_script(&self, script: &str) -> Result<serde_json::Value> {
        let result = self
            .call_tool("playwright_evaluate", json!({ "script": script }))
            .await?;

        // Extract text content from the response
        if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
            for item in content {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
                            return Ok(parsed);
                        }
                        return Ok(serde_json::Value::String(text.to_string()));
                    }
                }
            }
        }

        Ok(result)
    }

    /// Extract text content from MCP tool response
    fn extract_text_response(response: &serde_json::Value) -> Option<String> {
        if let Some(content) = response.get("content").and_then(|c| c.as_array()) {
            for item in content {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        return Some(text.to_string());
                    }
                }
            }
        }
        None
    }

    /// Navigate to a URL and return page state
    pub async fn navigate(&self, url: &str) -> Result<PageState> {
        info!("Navigating to: {}", url);

        self.call_tool("playwright_navigate", json!({ "url": url }))
            .await?;

        // Wait for page load
        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

        // Get page state
        self.get_page_state().await
    }

    /// Get current page state
    pub async fn get_page_state(&self) -> Result<PageState> {
        debug!("Getting page state");

        // 1. Get URL and title
        let info_script = r#"
            JSON.stringify({
                url: window.location.href,
                title: document.title
            })
        "#;
        let info_val = self.evaluate_script(info_script).await?;

        let info: serde_json::Value = if let Some(s) = info_val.as_str() {
            serde_json::from_str(s).unwrap_or(json!({}))
        } else {
            info_val
        };

        let url = info
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let title = info
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        debug!("BrowserDriver: Page URL: {}, Title: {}", url, title);

        // 2. Get visible text
        let visible_text = self.get_visible_text().await.unwrap_or_default();

        // 3. Get simplified HTML
        let simplified_html = self.get_visible_html(None).await.unwrap_or_default();

        // 4. Get interactive elements
        let interactive_elements = self.annotate_elements().await?;

        // 5. Get captured APIs (if network capture is enabled)
        let captured_apis = Vec::new(); // Network capture done through MCP when needed

        Ok(PageState {
            url,
            title,
            visible_text,
            simplified_html,
            interactive_elements,
            captured_apis,
            timestamp: SystemTime::now(),
        })
    }

    /// Get visible text content of the page
    pub async fn get_visible_text(&self) -> Result<String> {
        let result = self
            .call_tool("playwright_get_visible_text", json!({}))
            .await?;

        Ok(Self::extract_text_response(&result).unwrap_or_default())
    }

    /// Get HTML content of the page
    pub async fn get_visible_html(&self, selector: Option<&str>) -> Result<String> {
        let params = if let Some(sel) = selector {
            json!({ "selector": sel })
        } else {
            json!({})
        };
        let result = self
            .call_tool("playwright_get_visible_html", params)
            .await?;

        Ok(Self::extract_text_response(&result).unwrap_or_default())
    }

    /// Annotate interactive elements on the page
    pub async fn annotate_elements(&self) -> Result<Vec<InteractiveElement>> {
        let result = self.call_tool("playwright_annotate", json!({})).await?;

        // Extract the elements from the response
        let elements_json = if let Some(content) = result.get("content").and_then(|c| c.as_array())
        {
            let mut found = serde_json::Value::Null;
            for item in content {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
                            found = parsed;
                            break;
                        }
                    }
                }
            }
            found
        } else {
            result
        };

        // Parse the elements
        let mut elements: Vec<InteractiveElement> =
            serde_json::from_value(elements_json).unwrap_or_default();

        // Add indices
        for (i, elem) in elements.iter_mut().enumerate() {
            elem.index = i;
        }

        debug!("Found {} interactive elements", elements.len());
        Ok(elements)
    }

    /// Click an element by selector
    pub async fn click(&self, selector: &str) -> Result<()> {
        info!("Clicking element: {}", selector);

        self.call_tool("playwright_click", json!({ "selector": selector }))
            .await?;

        // Wait for potential navigation/changes
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

        Ok(())
    }

    /// Click an element by annotation index
    pub async fn click_by_index(&self, index: usize) -> Result<()> {
        info!("Clicking element by index: {}", index);

        self.call_tool("playwright_click_by_index", json!({ "index": index }))
            .await?;

        // Wait for potential navigation/changes
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

        Ok(())
    }

    /// Fill an input field by selector
    pub async fn fill(&self, selector: &str, value: &str) -> Result<()> {
        info!("Filling element {} with value: {}", selector, value);

        // Use evaluate to perform the fill since playwright_fill_by_index requires index
        let script = format!(
            r#"
            (function() {{
                const el = document.querySelector('{}');
                if (el) {{
                    el.value = '{}';
                    el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                    return 'filled';
                }}
                return 'not found';
            }})()
            "#,
            selector.replace('\'', "\\'").replace('\\', "\\\\"),
            value.replace('\'', "\\'").replace('\\', "\\\\")
        );
        self.evaluate_script(&script).await?;
        Ok(())
    }

    /// Fill an input field by annotation index
    pub async fn fill_by_index(&self, index: usize, value: &str) -> Result<()> {
        info!("Filling element at index {} with value: {}", index, value);

        self.call_tool(
            "playwright_fill_by_index",
            json!({ "index": index, "value": value }),
        )
        .await?;

        Ok(())
    }

    /// Go back in browser history
    pub async fn go_back(&self) -> Result<()> {
        info!("Going back in history");

        self.call_tool("playwright_go_back", json!({})).await?;

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

        Ok(())
    }

    /// Go forward in browser history
    pub async fn go_forward(&self) -> Result<()> {
        info!("Going forward in history");

        self.call_tool("playwright_go_forward", json!({})).await?;

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

        Ok(())
    }

    /// Hover over an element
    pub async fn hover(&self, selector: &str) -> Result<()> {
        info!("Hovering over element: {}", selector);

        self.call_tool("playwright_hover", json!({ "selector": selector }))
            .await?;

        Ok(())
    }

    /// Press a keyboard key
    pub async fn press_key(&self, key: &str, selector: Option<&str>) -> Result<()> {
        let params = if let Some(sel) = selector {
            json!({ "key": key, "selector": sel })
        } else {
            json!({ "key": key })
        };
        self.call_tool("playwright_press_key", params).await?;
        Ok(())
    }

    /// Click by screen coordinates
    pub async fn click_coordinate(&self, x: i32, y: i32) -> Result<()> {
        info!("Clicking at coordinates: ({}, {})", x, y);

        self.call_tool("playwright_click", json!({ "coordinate": [x, y] }))
            .await?;

        Ok(())
    }

    /// Select option in a dropdown
    pub async fn select(&self, selector: &str, value: &str) -> Result<()> {
        self.call_tool(
            "playwright_select",
            json!({ "selector": selector, "value": value }),
        )
        .await?;
        Ok(())
    }

    /// Execute JavaScript and return result
    pub async fn evaluate(&self, script: &str) -> Result<serde_json::Value> {
        self.evaluate_script(script).await
    }

    /// Start network capture (if supported by MCP server)
    pub async fn start_network_capture(&self) -> Result<()> {
        info!("Starting network capture via MCP");

        // Try to call the network capture tool if available
        match self
            .call_tool("playwright_start_network_capture", json!({}))
            .await
        {
            Ok(_) => {
                info!("Network capture started");
                Ok(())
            }
            Err(e) => {
                debug!("Network capture not available: {}", e);
                Ok(()) // Not a fatal error, continue without network capture
            }
        }
    }

    /// Get captured API requests
    pub async fn get_captured_requests(&self) -> Vec<ApiRequest> {
        match self
            .call_tool("playwright_get_captured_requests", json!({}))
            .await
        {
            Ok(response) => {
                if let Some(text) = Self::extract_text_response(&response) {
                    if let Ok(requests) = serde_json::from_str::<Vec<ApiRequest>>(&text) {
                        return requests;
                    }
                }
                Vec::new()
            }
            Err(_) => Vec::new(),
        }
    }

    /// Wait for a specific API request matching pattern
    pub async fn wait_for_request(
        &self,
        pattern: &str,
        timeout: std::time::Duration,
    ) -> Result<ApiRequest> {
        let start = std::time::Instant::now();
        let pattern_lower = pattern.to_lowercase();

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow!(
                    "Timeout waiting for request matching pattern: {}",
                    pattern
                ));
            }

            let requests = self.get_captured_requests().await;
            if let Some(request) = requests
                .iter()
                .find(|r| r.url.to_lowercase().contains(&pattern_lower))
            {
                return Ok(request.clone());
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    /// Close the browser
    pub async fn close(&self) -> Result<()> {
        info!("Closing browser");
        // The MCP server manages browser lifecycle
        // We just log the intent
        Ok(())
    }
}

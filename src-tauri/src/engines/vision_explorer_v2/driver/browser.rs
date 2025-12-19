use crate::engines::vision_explorer_v2::core::PageContext;
use crate::services::mcp::McpService;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use base64::Engine;
use serde_json::json;
use std::sync::Arc;

/// A wrapper around the MCP Playwright tool.
/// It performs actions via the McpService.
///
/// This driver is designed to work with mcp-playwright-security server
/// which uses `playwright_*` prefixed tool names.
pub struct BrowserDriver {
    mcp_service: Arc<McpService>,
}

impl BrowserDriver {
    pub fn new() -> Self {
        Self {
            mcp_service: Arc::new(McpService::new()),
        }
    }

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

    async fn call_tool(&self, tool: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let conn_name = self.get_playwright_conn_name().await?;
        log::debug!(
            "BrowserDriver: Calling tool '{}' with params: {:?}",
            tool,
            params
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
        // The evaluate tool returns: { "content": [{ "type": "text", "text": "result" }] }
        if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
            for item in content {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        // Try to parse as JSON first, otherwise return as string value
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

    /// Extract base64 image data from MCP tool response
    fn extract_screenshot_base64(response: &serde_json::Value) -> Option<String> {
        if let Some(content) = response.get("content").and_then(|c| c.as_array()) {
            log::debug!(
                "BrowserDriver: Found content array with {} items",
                content.len()
            );

            for item in content {
                let item_type = item.get("type").and_then(|s| s.as_str());
                log::debug!("BrowserDriver: Content item type: {:?}", item_type);

                // Check for direct image type
                if item_type == Some("image") {
                    if let Some(data) = item.get("data").and_then(|s| s.as_str()) {
                        log::debug!("BrowserDriver: Found image data ({} chars)", data.len());
                        return Some(data.to_string());
                    }
                }

                // Also check for text with embedded JSON (fallback)
                if item_type == Some("text") {
                    if let Some(text) = item.get("text").and_then(|s| s.as_str()) {
                        // Try to parse as JSON to extract screenshot_base64
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
                            if let Some(b64) =
                                parsed.get("screenshot_base64").and_then(|s| s.as_str())
                            {
                                log::debug!("BrowserDriver: Found screenshot_base64 in text JSON ({} chars)", b64.len());
                                return Some(b64.to_string());
                            }
                        }
                    }
                }
            }
        } else {
            log::warn!(
                "BrowserDriver: No 'content' array in response. Keys: {:?}",
                response.as_object().map(|o| o.keys().collect::<Vec<_>>())
            );
        }
        None
    }
}

#[async_trait]
impl crate::engines::vision_explorer_v2::driver::BrowserActions for BrowserDriver {
    async fn goto(&self, url: &str) -> Result<()> {
        self.call_tool("playwright_navigate", json!({ "url": url }))
            .await?;
        Ok(())
    }

    async fn type_text(&self, selector: &str, text: &str) -> Result<()> {
        // Note: mcp-playwright-security uses playwright_fill_by_index for indexed fills
        // For selector-based fills, we need to use iframe_fill or a custom approach
        // Let's use evaluate to perform the fill
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
            text.replace('\'', "\\'").replace('\\', "\\\\")
        );
        self.evaluate_script(&script).await?;
        Ok(())
    }

    async fn click(&self, selector: &str) -> Result<()> {
        self.call_tool("playwright_click", json!({ "selector": selector }))
            .await?;
        Ok(())
    }

    async fn capture_context(&self) -> Result<PageContext> {
        // 1. Get info (URL, Title) using playwright_evaluate
        let info_script = r#"
            JSON.stringify({
                url: window.location.href,
                title: document.title
            })
        "#;
        let info_val = self.evaluate_script(info_script).await?;

        // Handle both string and parsed JSON responses
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

        log::debug!(
            "BrowserDriver: Captured page info - URL: {}, Title: {}",
            url,
            title
        );

        // 2. Screenshot using playwright_screenshot
        // The tool requires a "name" parameter
        let screenshot_val = self
            .call_tool(
                "playwright_screenshot",
                json!({
                    "name": "vision_explorer_capture",
                    "storeBase64": true
                }),
            )
            .await?;

        // Debug: Log the raw screenshot tool response structure
        log::debug!(
            "BrowserDriver: screenshot tool response keys: {:?}",
            screenshot_val
                .as_object()
                .map(|o| o.keys().collect::<Vec<_>>())
        );

        let screenshot_base64 = Self::extract_screenshot_base64(&screenshot_val);

        let screenshot = if let Some(b64) = screenshot_base64 {
            log::debug!(
                "BrowserDriver: Decoding base64 screenshot ({} chars)",
                b64.len()
            );
            match base64::engine::general_purpose::STANDARD.decode(&b64) {
                Ok(bytes) => {
                    log::info!(
                        "BrowserDriver: Screenshot captured successfully ({} bytes)",
                        bytes.len()
                    );
                    Some(bytes)
                }
                Err(e) => {
                    log::warn!("BrowserDriver: Failed to decode base64 screenshot: {}", e);
                    None
                }
            }
        } else {
            log::warn!("BrowserDriver: No screenshot base64 data available");
            None
        };

        // 3. DOM Snapshot using playwright_evaluate
        let dom_val = self
            .evaluate_script(
                crate::engines::vision_explorer_v2::driver::browser_scripts::DOM_SKELETON_SCRIPT,
            )
            .await?;

        let dom_snapshot = if let Some(s) = dom_val.as_str() {
            s.to_string()
        } else {
            // Try to stringify if it's an object
            dom_val.to_string()
        };

        // Fallback for DOM if empty (script block or issue)
        let dom_snapshot =
            if dom_snapshot.is_empty() || dom_snapshot == "null" || dom_snapshot == "\"\"" {
                log::debug!("BrowserDriver: DOM skeleton empty, falling back to body.outerHTML");
                let html_val = self.evaluate_script("document.body.outerHTML").await.ok();
                html_val
                    .and_then(|v| {
                        if let Some(s) = v.as_str() {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default()
            } else {
                dom_snapshot
            };

        log::debug!(
            "BrowserDriver: DOM snapshot length: {} chars",
            dom_snapshot.len()
        );

        Ok(PageContext {
            url,
            title,
            screenshot,
            dom_snapshot,
            accessibility_tree: None,
        })
    }

    // ============ Extended BrowserActions implementations ============

    async fn hover(&self, selector: &str) -> Result<()> {
        self.call_tool("playwright_hover", json!({ "selector": selector }))
            .await?;
        Ok(())
    }

    async fn press_key(&self, key: &str, selector: Option<&str>) -> Result<()> {
        let params = if let Some(sel) = selector {
            json!({ "key": key, "selector": sel })
        } else {
            json!({ "key": key })
        };
        self.call_tool("playwright_press_key", params).await?;
        Ok(())
    }

    async fn click_coordinate(&self, x: i32, y: i32) -> Result<()> {
        self.call_tool("playwright_click", json!({ "coordinate": [x, y] }))
            .await?;
        Ok(())
    }

    async fn annotate(&self) -> Result<serde_json::Value> {
        let result = self.call_tool("playwright_annotate", json!({})).await?;

        // Extract the elements from the response
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

    async fn click_by_index(&self, index: usize) -> Result<()> {
        self.call_tool("playwright_click_by_index", json!({ "index": index }))
            .await?;
        Ok(())
    }

    async fn fill_by_index(&self, index: usize, value: &str) -> Result<()> {
        self.call_tool(
            "playwright_fill_by_index",
            json!({ "index": index, "value": value }),
        )
        .await?;
        Ok(())
    }

    async fn get_visible_text(&self) -> Result<String> {
        let result = self
            .call_tool("playwright_get_visible_text", json!({}))
            .await?;

        // Extract text from response
        if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
            for item in content {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        return Ok(text.to_string());
                    }
                }
            }
        }
        Ok(String::new())
    }

    async fn get_visible_html(&self, selector: Option<&str>) -> Result<String> {
        let params = if let Some(sel) = selector {
            json!({ "selector": sel })
        } else {
            json!({})
        };
        let result = self
            .call_tool("playwright_get_visible_html", params)
            .await?;

        // Extract HTML from response
        if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
            for item in content {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        return Ok(text.to_string());
                    }
                }
            }
        }
        Ok(String::new())
    }

    async fn go_back(&self) -> Result<()> {
        self.call_tool("playwright_go_back", json!({})).await?;
        Ok(())
    }

    async fn go_forward(&self) -> Result<()> {
        self.call_tool("playwright_go_forward", json!({})).await?;
        Ok(())
    }

    async fn select(&self, selector: &str, value: &str) -> Result<()> {
        self.call_tool(
            "playwright_select",
            json!({ "selector": selector, "value": value }),
        )
        .await?;
        Ok(())
    }
}

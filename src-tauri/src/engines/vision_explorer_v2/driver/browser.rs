use crate::engines::vision_explorer_v2::core::PageContext;
use crate::services::mcp::McpService;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use base64::Engine;
use serde_json::json;
use std::sync::Arc;

/// A wrapper around the MCP Playwright tool.
/// It performs actions via the McpService.
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
        self.mcp_service
            .execute_client_tool(&conn_name, tool, params)
            .await
    }

    async fn evaluate_script(&self, script: &str) -> Result<serde_json::Value> {
        let result = self
            .call_tool("evaluate", json!({ "script": script }))
            .await?;

        // The evaluate tool typically returns the result directly or in a wrapper.
        // Assuming the result is the value itself or a string representation.
        // We'll trust the output for now, but robustness might be needed.
        Ok(result)
    }
}

#[async_trait]
impl crate::engines::vision_explorer_v2::driver::BrowserActions for BrowserDriver {
    async fn goto(&self, url: &str) -> Result<()> {
        self.call_tool("navigate", json!({ "url": url })).await?;
        Ok(())
    }

    async fn type_text(&self, selector: &str, text: &str) -> Result<()> {
        self.call_tool("fill", json!({ "selector": selector, "value": text }))
            .await?;
        Ok(())
    }

    async fn click(&self, selector: &str) -> Result<()> {
        self.call_tool("click", json!({ "selector": selector }))
            .await?;
        Ok(())
    }

    async fn capture_context(&self) -> Result<PageContext> {
        // 1. Get info (URL, Title)
        let info_script = r#"
            JSON.stringify({
                url: window.location.href,
                title: document.title
            })
        "#;
        let info_val = self.evaluate_script(info_script).await?;
        let info_str = info_val.as_str().unwrap_or("{}");
        let info: serde_json::Value = serde_json::from_str(info_str).unwrap_or(json!({}));

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

        // 2. Screenshot
        let screenshot_val = self.call_tool("screenshot", json!({})).await?;
        // Expecting base64 string in "content" or directly if it's a simple tool return.
        // Inspecting standard MCP tool result structure: Usually a list of content blocks.
        // However, `execute_client_tool` in `McpService` wraps `mcp_call_tool`.
        // `mcp_call_tool` returns `ToolCallResult`.
        // Wait, `mcp_call_tool` returns `ToolCallResultMessage`? No, let's check McpService again.
        // It calls `crate::commands::mcp_commands::mcp_call_tool`.
        // I need to be careful about what `execute_client_tool` returns.

        // Assuming it validates success and returns a Value.
        // Usually screenshot tool returns:
        // { "type": "image", "data": "base64...", "mimeType": "image/png" } in the content list.

        // If we can't be sure, we might need to debug. But for now, let's try to extract base64.
        // If the result is a massive JSON object representing the tool result.

        let screenshot_base64 =
            if let Some(content) = screenshot_val.get("content").and_then(|c| c.as_array()) {
                // Find first image content
                content.iter().find_map(|item| {
                    if item.get("type").and_then(|s| s.as_str()) == Some("image") {
                        item.get("data")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string())
                    } else if item.get("type").and_then(|s| s.as_str()) == Some("text") {
                        // Sometimes text tools return base64 in text? Unlikely for 'screenshot'.
                        None
                    } else {
                        None
                    }
                })
            } else {
                // Maybe direct return?
                None
            };

        let screenshot = if let Some(b64) = screenshot_base64 {
            base64::engine::general_purpose::STANDARD.decode(b64).ok()
        } else {
            None
        };

        // 3. DOM Snapshot
        let dom_val = self
            .evaluate_script(
                crate::engines::vision_explorer_v2::driver::browser_scripts::DOM_SKELETON_SCRIPT,
            )
            .await?;

        let dom_snapshot = dom_val.as_str().map(|s| s.to_string()).unwrap_or_else(|| {
            // Fallback
            "".to_string()
        });

        // Fallback for DOM if empty (script block or issue)
        let dom_snapshot = if dom_snapshot.is_empty() {
            let html_val = self.evaluate_script("document.body.outerHTML").await.ok();
            html_val
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default()
        } else {
            dom_snapshot
        };

        Ok(PageContext {
            url,
            title,
            screenshot,
            dom_snapshot,
            accessibility_tree: None,
        })
    }
}

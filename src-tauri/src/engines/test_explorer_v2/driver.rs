use crate::engines::test_explorer_v1::types::PageState;
use crate::services::mcp::McpService;
use anyhow::{anyhow, Result};
use serde_json::json;
use std::sync::Arc;

pub struct ExplorerDriver {
    mcp_service: Arc<McpService>,
}

impl Default for ExplorerDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl ExplorerDriver {
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

    pub async fn navigate(&self, url: &str) -> Result<()> {
        self.call_tool("playwright_navigate", json!({ "url": url })).await?;
        Ok(())
    }

    pub async fn click(&self, index: usize) -> Result<()> {
        self.call_tool("playwright_click_by_index", json!({ "index": index })).await?;
        Ok(())
    }

    pub async fn type_text(&self, index: usize, text: &str) -> Result<()> {
        self.call_tool("playwright_fill_by_index", json!({ "index": index, "value": text })).await?;
        Ok(())
    }

    pub async fn scroll(&self, direction: &str) -> Result<()> {
        // Simple scroll using evaluate
        let script = match direction {
            "up" => "window.scrollBy(0, -500)",
            "down" => "window.scrollBy(0, 500)",
            "top" => "window.scrollTo(0, 0)",
            "bottom" => "window.scrollTo(0, document.body.scrollHeight)",
            _ => "window.scrollBy(0, 500)",
        };
        self.call_tool("playwright_evaluate", json!({ "script": script })).await?;
        Ok(())
    }

    pub async fn go_back(&self) -> Result<()> {
        self.call_tool("playwright_go_back", json!({})).await?;
        Ok(())
    }

    pub async fn get_state(&self) -> Result<PageState> {
        // 1. Get info (URL, Title)
        let info_val = self.call_tool("playwright_evaluate", json!({ 
            "script": "JSON.stringify({url: window.location.href, title: document.title})" 
        })).await?;
        
        // Parse info
        let info_str = self.extract_text_result(&info_val)?;
        let info: serde_json::Value = serde_json::from_str(&info_str).unwrap_or(json!({}));
        
        let url = info.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let title = info.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();

        // 2. Annotate elements (get interactive elements map)
        let annotation_val = self.call_tool("playwright_annotate", json!({})).await?;
        let interactive_elements_str = self.extract_text_result(&annotation_val)?;
        let interactive_elements: Option<serde_json::Value> = serde_json::from_str(&interactive_elements_str).ok();

        // 3. Get Visible Text
        let text_val = self.call_tool("playwright_get_visible_text", json!({})).await?;
        let content = self.extract_text_result(&text_val)?;

        Ok(PageState {
            url,
            title,
            content,
            interactive_elements,
        })
    }

    fn extract_text_result(&self, result: &serde_json::Value) -> Result<String> {
        if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
            for item in content {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        return Ok(text.to_string());
                    }
                }
            }
        }
        // Fallback or error?
        Ok(String::new())
    }
}

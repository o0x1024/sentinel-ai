//! Tool definitions for Test Explorer V1

use anyhow::Result;
use serde_json::{json, Value};
use sentinel_tools::{DynamicToolBuilder, ToolSource};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;

use super::driver::BrowserDriver;

/// Shared state for test explorer tools
pub struct TestExplorerToolState {
    pub driver: Arc<RwLock<Option<Arc<BrowserDriver>>>>,
}

impl Default for TestExplorerToolState {
    fn default() -> Self {
        Self::new()
    }
}

impl TestExplorerToolState {
    pub fn new() -> Self {
        Self {
            driver: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_driver(&self, driver: Arc<BrowserDriver>) {
        *self.driver.write().await = Some(driver);
    }

    pub async fn get_driver(&self) -> Option<Arc<BrowserDriver>> {
        self.driver.read().await.clone()
    }
}

/// Register all test explorer tools
pub async fn register_test_explorer_tools(
    tool_server: &sentinel_tools::ToolServer,
    state: Arc<TestExplorerToolState>,
) -> Result<()> {
    info!("Registering Test Explorer V1 tools");

    // 1. navigate_to - Navigate to a URL
    {
        let state = state.clone();
        let tool = DynamicToolBuilder::new("test_explorer_navigate")
            .description("Navigate to a URL and return the page state with visible text, interactive elements, and captured API requests")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to navigate to"
                    }
                },
                "required": ["url"]
            }))
            .source(ToolSource::Builtin)
            .executor(move |args: Value| {
                let state = state.clone();
                async move {
                    let url = args["url"].as_str().ok_or_else(|| "Missing url parameter".to_string())?;
                    
                    let driver = state.get_driver().await.ok_or_else(|| "Browser not initialized".to_string())?;
                    let page_state = driver.navigate(url).await.map_err(|e| e.to_string())?;
                    
                    Ok(serde_json::to_value(&page_state).unwrap_or(json!({})))
                }
            })
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build tool: {}", e))?;
        
        tool_server.register_tool(tool).await;
    }

    // 2. click_element - Click an element by selector or index
    {
        let state = state.clone();
        let tool = DynamicToolBuilder::new("test_explorer_click")
            .description("Click an element on the page by CSS selector or element index")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector of the element to click"
                    },
                    "index": {
                        "type": "number",
                        "description": "Index of the element from annotated elements list"
                    }
                }
            }))
            .source(ToolSource::Builtin)
            .executor(move |args: Value| {
                let state = state.clone();
                async move {
                    let driver = state.get_driver().await.ok_or_else(|| "Browser not initialized".to_string())?;
                    
                    if let Some(index) = args["index"].as_u64() {
                        driver.click_by_index(index as usize).await.map_err(|e| e.to_string())?;
                    } else if let Some(selector) = args["selector"].as_str() {
                        driver.click(selector).await.map_err(|e| e.to_string())?;
                    } else {
                        return Err("Must provide either selector or index".to_string());
                    }
                    
                    let page_state = driver.get_page_state().await.map_err(|e| e.to_string())?;
                    Ok(serde_json::to_value(&page_state).unwrap_or(json!({})))
                }
            })
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build tool: {}", e))?;
        
        tool_server.register_tool(tool).await;
    }

    // 3. fill_input - Fill an input field
    {
        let state = state.clone();
        let tool = DynamicToolBuilder::new("test_explorer_fill")
            .description("Fill an input field with a value")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector of the input element"
                    },
                    "value": {
                        "type": "string",
                        "description": "Value to fill into the input"
                    }
                },
                "required": ["selector", "value"]
            }))
            .source(ToolSource::Builtin)
            .executor(move |args: Value| {
                let state = state.clone();
                async move {
                    let selector = args["selector"].as_str().ok_or_else(|| "Missing selector parameter".to_string())?;
                    let value = args["value"].as_str().ok_or_else(|| "Missing value parameter".to_string())?;
                    
                    let driver = state.get_driver().await.ok_or_else(|| "Browser not initialized".to_string())?;
                    driver.fill(selector, value).await.map_err(|e| e.to_string())?;
                    
                    let page_state = driver.get_page_state().await.map_err(|e| e.to_string())?;
                    Ok(serde_json::to_value(&page_state).unwrap_or(json!({})))
                }
            })
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build tool: {}", e))?;
        
        tool_server.register_tool(tool).await;
    }

    // 4. get_page_content - Get current page state
    {
        let state = state.clone();
        let tool = DynamicToolBuilder::new("test_explorer_get_page")
            .description("Get the current page state including visible text, HTML, and interactive elements")
            .input_schema(json!({
                "type": "object",
                "properties": {}
            }))
            .source(ToolSource::Builtin)
            .executor(move |_args: Value| {
                let state = state.clone();
                async move {
                    let driver = state.get_driver().await.ok_or_else(|| "Browser not initialized".to_string())?;
                    let page_state = driver.get_page_state().await.map_err(|e| e.to_string())?;
                    
                    Ok(serde_json::to_value(&page_state).unwrap_or(json!({})))
                }
            })
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build tool: {}", e))?;
        
        tool_server.register_tool(tool).await;
    }

    // 5. get_captured_apis - Get all captured API requests
    {
        let state = state.clone();
        let tool = DynamicToolBuilder::new("test_explorer_get_apis")
            .description("Get all captured API requests (XHR, Fetch) with request/response details")
            .input_schema(json!({
                "type": "object",
                "properties": {}
            }))
            .source(ToolSource::Builtin)
            .executor(move |_args: Value| {
                let state = state.clone();
                async move {
                    let driver = state.get_driver().await.ok_or_else(|| "Browser not initialized".to_string())?;
                    let requests = driver.get_captured_requests().await;
                    
                    Ok(serde_json::to_value(&requests).unwrap_or(json!([])))
                }
            })
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build tool: {}", e))?;
        
        tool_server.register_tool(tool).await;
    }

    // 6. wait_for_api - Wait for a specific API request
    {
        let state = state.clone();
        let tool = DynamicToolBuilder::new("test_explorer_wait_for_api")
            .description("Wait for an API request matching the given URL pattern")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "URL pattern to match (substring)"
                    },
                    "timeout_ms": {
                        "type": "number",
                        "description": "Timeout in milliseconds (default: 10000)"
                    }
                },
                "required": ["pattern"]
            }))
            .source(ToolSource::Builtin)
            .executor(move |args: Value| {
                let state = state.clone();
                async move {
                    let pattern = args["pattern"].as_str().ok_or_else(|| "Missing pattern parameter".to_string())?;
                    let timeout_ms = args["timeout_ms"].as_u64().unwrap_or(10000);
                    
                    let driver = state.get_driver().await.ok_or_else(|| "Browser not initialized".to_string())?;
                    let request = driver
                        .wait_for_request(pattern, Duration::from_millis(timeout_ms))
                        .await
                        .map_err(|e| e.to_string())?;
                    
                    Ok(serde_json::to_value(&request).unwrap_or(json!({})))
                }
            })
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build tool: {}", e))?;
        
        tool_server.register_tool(tool).await;
    }

    // 7. annotate_page - Get interactive elements
    {
        let state = state.clone();
        let tool = DynamicToolBuilder::new("test_explorer_annotate")
            .description("Get all interactive elements on the page (buttons, links, inputs, etc.) with their indices")
            .input_schema(json!({
                "type": "object",
                "properties": {}
            }))
            .source(ToolSource::Builtin)
            .executor(move |_args: Value| {
                let state = state.clone();
                async move {
                    let driver = state.get_driver().await.ok_or_else(|| "Browser not initialized".to_string())?;
                    let elements = driver.annotate_elements().await.map_err(|e| e.to_string())?;
                    
                    Ok(serde_json::to_value(&elements).unwrap_or(json!([])))
                }
            })
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build tool: {}", e))?;
        
        tool_server.register_tool(tool).await;
    }

    // 8. go_back - Navigate back in history
    {
        let state = state.clone();
        let tool = DynamicToolBuilder::new("test_explorer_back")
            .description("Navigate back in browser history")
            .input_schema(json!({
                "type": "object",
                "properties": {}
            }))
            .source(ToolSource::Builtin)
            .executor(move |_args: Value| {
                let state = state.clone();
                async move {
                    let driver = state.get_driver().await.ok_or_else(|| "Browser not initialized".to_string())?;
                    driver.go_back().await.map_err(|e| e.to_string())?;
                    
                    let page_state = driver.get_page_state().await.map_err(|e| e.to_string())?;
                    Ok(serde_json::to_value(&page_state).unwrap_or(json!({})))
                }
            })
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build tool: {}", e))?;
        
        tool_server.register_tool(tool).await;
    }

    // 9. evaluate_js - Execute JavaScript
    {
        let state = state.clone();
        let tool = DynamicToolBuilder::new("test_explorer_evaluate")
            .description("Execute JavaScript code in the browser and return the result")
            .input_schema(json!({
                "type": "object",
                "properties": {
                    "script": {
                        "type": "string",
                        "description": "JavaScript code to execute"
                    }
                },
                "required": ["script"]
            }))
            .source(ToolSource::Builtin)
            .executor(move |args: Value| {
                let state = state.clone();
                async move {
                    let script = args["script"].as_str().ok_or_else(|| "Missing script parameter".to_string())?;
                    
                    let driver = state.get_driver().await.ok_or_else(|| "Browser not initialized".to_string())?;
                    let result = driver.evaluate(script).await.map_err(|e| e.to_string())?;
                    
                    Ok(result)
                }
            })
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build tool: {}", e))?;
        
        tool_server.register_tool(tool).await;
    }

    info!("Registered 9 Test Explorer V1 tools");
    Ok(())
}


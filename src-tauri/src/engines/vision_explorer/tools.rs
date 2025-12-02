//! 浏览器工具封装
//!
//! 封装Playwright MCP工具为computer_*风格的统一接口
//! 关键特性：每次操作后自动截图返回

use super::types::*;
use crate::services::mcp::McpService;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// 浏览器工具服务
pub struct BrowserTools {
    mcp_service: Arc<McpService>,
    config: VisionExplorerConfig,
}

impl BrowserTools {
    pub fn new(mcp_service: Arc<McpService>, config: VisionExplorerConfig) -> Self {
        Self { mcp_service, config }
    }

    /// 执行浏览器操作并自动截图
    pub async fn execute_action(&self, action: &BrowserAction) -> Result<ActionResult> {
        let start_time = std::time::Instant::now();
        
        info!("Executing browser action: {:?}", action);

        // 执行操作
        let result = match action {
            BrowserAction::Screenshot => self.take_screenshot().await,
            BrowserAction::MoveMouse { coordinates } => {
                self.move_mouse(coordinates).await
            }
            BrowserAction::ClickMouse { coordinates, button, click_count } => {
                self.click_mouse(coordinates.as_ref(), button, *click_count).await
            }
            BrowserAction::Scroll { coordinates, direction, scroll_count } => {
                self.scroll(coordinates.as_ref(), direction, *scroll_count).await
            }
            BrowserAction::TypeText { text } => self.type_text(text).await,
            BrowserAction::PasteText { text } => self.paste_text(text).await,
            BrowserAction::TypeKeys { keys } => self.type_keys(keys).await,
            BrowserAction::Wait { duration_ms } => self.wait(*duration_ms).await,
            BrowserAction::Navigate { url } => self.navigate(url).await,
            BrowserAction::ClickElement { selector } => self.click_element(selector).await,
            BrowserAction::FillInput { selector, value } => {
                self.fill_input(selector, value).await
            }
            BrowserAction::SelectOption { selector, value } => {
                self.select_option(selector, value).await
            }
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(mut action_result) => {
                action_result.duration_ms = duration_ms;
                
                // 对于非截图操作，等待UI稳定后自动截图
                if !matches!(action, BrowserAction::Screenshot) {
                    sleep(Duration::from_millis(self.config.screenshot_delay_ms)).await;
                    
                    match self.take_screenshot_raw().await {
                        Ok(screenshot) => {
                            action_result.screenshot = Some(screenshot);
                        }
                        Err(e) => {
                            warn!("Failed to take screenshot after action: {}", e);
                        }
                    }
                }
                
                Ok(action_result)
            }
            Err(e) => {
                error!("Browser action failed: {}", e);
                Ok(ActionResult {
                    success: false,
                    error: Some(e.to_string()),
                    screenshot: None,
                    duration_ms,
                })
            }
        }
    }

    /// 截图
    async fn take_screenshot(&self) -> Result<ActionResult> {
        let screenshot = self.take_screenshot_raw().await?;
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: Some(screenshot),
            duration_ms: 0,
        })
    }

    /// 截图（返回base64）
    async fn take_screenshot_raw(&self) -> Result<String> {
        let result = self.call_playwright_tool("playwright_screenshot", json!({})).await?;
        
        // 解析截图结果，提取base64数据
        if let Some(content) = result.get("content") {
            if let Some(arr) = content.as_array() {
                for item in arr {
                    if item.get("type").and_then(|t| t.as_str()) == Some("image") {
                        if let Some(data) = item.get("data").and_then(|d| d.as_str()) {
                            return Ok(data.to_string());
                        }
                    }
                }
            }
        }
        
        // 尝试直接获取base64
        if let Some(base64) = result.get("base64").and_then(|b| b.as_str()) {
            return Ok(base64.to_string());
        }
        
        // 尝试从result字段获取
        if let Some(res) = result.get("result") {
            if let Some(base64) = res.as_str() {
                return Ok(base64.to_string());
            }
        }
        
        Err(anyhow!("Failed to extract screenshot data from response"))
    }

    /// 移动鼠标
    async fn move_mouse(&self, coordinates: &Coordinates) -> Result<ActionResult> {
        self.call_playwright_tool("playwright_hover", json!({
            "coordinate": [coordinates.x, coordinates.y]
        })).await?;
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 点击鼠标
    async fn click_mouse(
        &self,
        coordinates: Option<&Coordinates>,
        button: &MouseButton,
        click_count: u32,
    ) -> Result<ActionResult> {
        let button_str = match button {
            MouseButton::Left => "left",
            MouseButton::Right => "right",
            MouseButton::Middle => "middle",
        };
        
        let mut params = json!({
            "button": button_str
        });
        
        if let Some(coords) = coordinates {
            params["coordinate"] = json!([coords.x, coords.y]);
        }
        
        // 执行点击
        for _ in 0..click_count {
            self.call_playwright_tool("playwright_click", params.clone()).await?;
            if click_count > 1 {
                sleep(Duration::from_millis(100)).await;
            }
        }
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 滚动
    async fn scroll(
        &self,
        coordinates: Option<&Coordinates>,
        direction: &ScrollDirection,
        scroll_count: u32,
    ) -> Result<ActionResult> {
        let (delta_x, delta_y) = match direction {
            ScrollDirection::Up => (0, -100),
            ScrollDirection::Down => (0, 100),
            ScrollDirection::Left => (-100, 0),
            ScrollDirection::Right => (100, 0),
        };
        
        for _ in 0..scroll_count {
            let mut params = json!({
                "deltaX": delta_x,
                "deltaY": delta_y
            });
            
            if let Some(coords) = coordinates {
                params["coordinate"] = json!([coords.x, coords.y]);
            }
            
            // 使用evaluate执行滚动
            self.call_playwright_tool("playwright_evaluate", json!({
                "expression": format!("window.scrollBy({}, {})", delta_x * scroll_count as i32, delta_y * scroll_count as i32)
            })).await?;
            
            break; // 一次性滚动
        }
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 输入文本
    async fn type_text(&self, text: &str) -> Result<ActionResult> {
        self.call_playwright_tool("playwright_type", json!({
            "text": text
        })).await?;
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 粘贴文本
    async fn paste_text(&self, text: &str) -> Result<ActionResult> {
        // 使用fill来快速填充
        self.call_playwright_tool("playwright_fill", json!({
            "value": text
        })).await?;
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 按键
    async fn type_keys(&self, keys: &[String]) -> Result<ActionResult> {
        for key in keys {
            self.call_playwright_tool("playwright_press", json!({
                "key": key
            })).await?;
        }
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 等待
    async fn wait(&self, duration_ms: u64) -> Result<ActionResult> {
        sleep(Duration::from_millis(duration_ms)).await;
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms,
        })
    }

    /// 导航
    async fn navigate(&self, url: &str) -> Result<ActionResult> {
        self.call_playwright_tool("playwright_navigate", json!({
            "url": url
        })).await?;
        
        // 等待页面加载
        sleep(Duration::from_millis(1000)).await;
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 点击元素（通过选择器）
    async fn click_element(&self, selector: &str) -> Result<ActionResult> {
        self.call_playwright_tool("playwright_click", json!({
            "selector": selector
        })).await?;
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 填写输入框
    async fn fill_input(&self, selector: &str, value: &str) -> Result<ActionResult> {
        self.call_playwright_tool("playwright_fill", json!({
            "selector": selector,
            "value": value
        })).await?;
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 选择下拉选项
    async fn select_option(&self, selector: &str, value: &str) -> Result<ActionResult> {
        self.call_playwright_tool("playwright_select_option", json!({
            "selector": selector,
            "value": value
        })).await?;
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 获取页面可见HTML
    pub async fn get_page_html(&self) -> Result<String> {
        let result = self.call_playwright_tool("playwright_get_visible_html", json!({})).await?;
        
        if let Some(html) = result.get("html").and_then(|h| h.as_str()) {
            return Ok(html.to_string());
        }
        
        if let Some(content) = result.as_str() {
            return Ok(content.to_string());
        }
        
        Ok(result.to_string())
    }

    /// 执行JavaScript获取页面信息
    pub async fn evaluate_js(&self, expression: &str) -> Result<Value> {
        self.call_playwright_tool("playwright_evaluate", json!({
            "expression": expression
        })).await
    }

    /// 获取页面可交互元素
    pub async fn get_interactable_elements(&self) -> Result<Vec<PageElement>> {
        let js = r#"
            (function() {
                const elements = [];
                const interactables = document.querySelectorAll(
                    'a, button, input, select, textarea, [onclick], [role="button"], [role="link"]'
                );
                
                interactables.forEach((el, idx) => {
                    const rect = el.getBoundingClientRect();
                    if (rect.width > 0 && rect.height > 0) {
                        const computedStyle = window.getComputedStyle(el);
                        if (computedStyle.display !== 'none' && computedStyle.visibility !== 'hidden') {
                            elements.push({
                                id: el.id || `element_${idx}`,
                                tag: el.tagName.toLowerCase(),
                                text: (el.innerText || el.value || el.placeholder || '').slice(0, 100),
                                selector: el.id ? `#${el.id}` : `${el.tagName.toLowerCase()}:nth-of-type(${idx + 1})`,
                                element_type: el.type || null,
                                attributes: {
                                    href: el.href || null,
                                    name: el.name || null,
                                    class: el.className || null
                                },
                                bounding_box: {
                                    x: rect.x,
                                    y: rect.y,
                                    width: rect.width,
                                    height: rect.height
                                },
                                is_visible: true,
                                is_interactable: !el.disabled
                            });
                        }
                    }
                });
                
                return elements;
            })()
        "#;
        
        let result = self.evaluate_js(js).await?;
        
        let elements: Vec<PageElement> = serde_json::from_value(result)
            .unwrap_or_else(|_| Vec::new());
        
        Ok(elements)
    }

    /// 获取页面表单
    pub async fn get_forms(&self) -> Result<Vec<FormInfo>> {
        let js = r#"
            (function() {
                const forms = [];
                document.querySelectorAll('form').forEach((form, idx) => {
                    const fields = [];
                    form.querySelectorAll('input, select, textarea').forEach(field => {
                        fields.push({
                            name: field.name || '',
                            field_type: field.type || field.tagName.toLowerCase(),
                            required: field.required,
                            placeholder: field.placeholder || null,
                            value: field.value || null
                        });
                    });
                    
                    forms.push({
                        id: form.id || `form_${idx}`,
                        action: form.action || null,
                        method: form.method || 'GET',
                        fields: fields
                    });
                });
                return forms;
            })()
        "#;
        
        let result = self.evaluate_js(js).await?;
        
        let forms: Vec<FormInfo> = serde_json::from_value(result)
            .unwrap_or_else(|_| Vec::new());
        
        Ok(forms)
    }

    /// 获取当前URL
    pub async fn get_current_url(&self) -> Result<String> {
        let result = self.evaluate_js("window.location.href").await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// 获取页面标题
    pub async fn get_page_title(&self) -> Result<String> {
        let result = self.evaluate_js("document.title").await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// 采集当前页面状态
    pub async fn capture_page_state(&self) -> Result<PageState> {
        // 并行获取各种信息
        let (url, title, screenshot, elements, forms) = tokio::try_join!(
            self.get_current_url(),
            self.get_page_title(),
            self.take_screenshot_raw(),
            self.get_interactable_elements(),
            self.get_forms()
        )?;
        
        // 提取链接
        let links: Vec<PageElement> = elements.iter()
            .filter(|e| e.tag == "a")
            .cloned()
            .collect();
        
        Ok(PageState {
            url,
            title,
            screenshot: Some(screenshot),
            interactable_elements: elements,
            forms,
            links,
            visible_text_summary: None,
            captured_at: Utc::now(),
        })
    }

    /// 调用Playwright MCP工具
    async fn call_playwright_tool(&self, tool_name: &str, params: Value) -> Result<Value> {
        debug!("Calling Playwright tool: {} with params: {:?}", tool_name, params);
        
        // 查找playwright连接
        let connections = self.mcp_service.get_connection_info().await?;
        let playwright_conn = connections.iter()
            .find(|c| c.name.contains("playwright") && c.status == "connected");
        
        if playwright_conn.is_none() {
            return Err(anyhow!("Playwright MCP server not connected"));
        }
        
        let conn_name = &playwright_conn.unwrap().name;
        
        // 执行工具调用
        let result = self.mcp_service
            .execute_client_tool(conn_name, tool_name, params)
            .await?;
        
        debug!("Playwright tool result: {:?}", result);
        
        Ok(result)
    }
}

// ============================================================================
// 工具定义转换
// ============================================================================

/// 将BrowserAction转换为工具调用格式
pub fn action_to_tool_call(action: &BrowserAction) -> (String, Value) {
    match action {
        BrowserAction::Screenshot => (
            "computer_screenshot".to_string(),
            json!({})
        ),
        BrowserAction::MoveMouse { coordinates } => (
            "computer_move_mouse".to_string(),
            json!({ "coordinates": { "x": coordinates.x, "y": coordinates.y } })
        ),
        BrowserAction::ClickMouse { coordinates, button, click_count } => (
            "computer_click_mouse".to_string(),
            json!({
                "coordinates": coordinates.as_ref().map(|c| json!({ "x": c.x, "y": c.y })),
                "button": match button {
                    MouseButton::Left => "left",
                    MouseButton::Right => "right",
                    MouseButton::Middle => "middle",
                },
                "click_count": click_count
            })
        ),
        BrowserAction::Scroll { coordinates, direction, scroll_count } => (
            "computer_scroll".to_string(),
            json!({
                "coordinates": coordinates.as_ref().map(|c| json!({ "x": c.x, "y": c.y })),
                "direction": match direction {
                    ScrollDirection::Up => "up",
                    ScrollDirection::Down => "down",
                    ScrollDirection::Left => "left",
                    ScrollDirection::Right => "right",
                },
                "scroll_count": scroll_count
            })
        ),
        BrowserAction::TypeText { text } => (
            "computer_type_text".to_string(),
            json!({ "text": text })
        ),
        BrowserAction::PasteText { text } => (
            "computer_paste_text".to_string(),
            json!({ "text": text })
        ),
        BrowserAction::TypeKeys { keys } => (
            "computer_type_keys".to_string(),
            json!({ "keys": keys })
        ),
        BrowserAction::Wait { duration_ms } => (
            "computer_wait".to_string(),
            json!({ "duration_ms": duration_ms })
        ),
        BrowserAction::Navigate { url } => (
            "computer_navigate".to_string(),
            json!({ "url": url })
        ),
        BrowserAction::ClickElement { selector } => (
            "computer_click_element".to_string(),
            json!({ "selector": selector })
        ),
        BrowserAction::FillInput { selector, value } => (
            "computer_fill_input".to_string(),
            json!({ "selector": selector, "value": value })
        ),
        BrowserAction::SelectOption { selector, value } => (
            "computer_select_option".to_string(),
            json!({ "selector": selector, "value": value })
        ),
    }
}

/// 从工具调用解析BrowserAction
pub fn parse_tool_call_to_action(tool_name: &str, params: &Value) -> Result<BrowserAction> {
    match tool_name {
        "computer_screenshot" => Ok(BrowserAction::Screenshot),
        
        "computer_move_mouse" => {
            let coords = params.get("coordinates").ok_or_else(|| anyhow!("Missing coordinates"))?;
            Ok(BrowserAction::MoveMouse {
                coordinates: Coordinates {
                    x: coords.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                    y: coords.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                }
            })
        }
        
        "computer_click_mouse" => {
            let coords = params.get("coordinates").map(|c| Coordinates {
                x: c.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                y: c.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            });
            
            let button = match params.get("button").and_then(|b| b.as_str()).unwrap_or("left") {
                "right" => MouseButton::Right,
                "middle" => MouseButton::Middle,
                _ => MouseButton::Left,
            };
            
            let click_count = params.get("click_count").and_then(|c| c.as_u64()).unwrap_or(1) as u32;
            
            Ok(BrowserAction::ClickMouse {
                coordinates: coords,
                button,
                click_count,
            })
        }
        
        "computer_scroll" => {
            let coords = params.get("coordinates").map(|c| Coordinates {
                x: c.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                y: c.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            });
            
            let direction = match params.get("direction").and_then(|d| d.as_str()).unwrap_or("down") {
                "up" => ScrollDirection::Up,
                "left" => ScrollDirection::Left,
                "right" => ScrollDirection::Right,
                _ => ScrollDirection::Down,
            };
            
            let scroll_count = params.get("scroll_count").and_then(|c| c.as_u64()).unwrap_or(3) as u32;
            
            Ok(BrowserAction::Scroll {
                coordinates: coords,
                direction,
                scroll_count,
            })
        }
        
        "computer_type_text" => {
            let text = params.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string();
            Ok(BrowserAction::TypeText { text })
        }
        
        "computer_paste_text" => {
            let text = params.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string();
            Ok(BrowserAction::PasteText { text })
        }
        
        "computer_type_keys" => {
            let keys = params.get("keys")
                .and_then(|k| k.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            Ok(BrowserAction::TypeKeys { keys })
        }
        
        "computer_wait" => {
            let duration_ms = params.get("duration_ms").and_then(|d| d.as_u64()).unwrap_or(500);
            Ok(BrowserAction::Wait { duration_ms })
        }
        
        "computer_navigate" => {
            let url = params.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string();
            Ok(BrowserAction::Navigate { url })
        }
        
        _ => Err(anyhow!("Unknown tool: {}", tool_name))
    }
}


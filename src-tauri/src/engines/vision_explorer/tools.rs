//! 浏览器工具封装
//!
//! 封装Playwright MCP工具为computer_*风格的统一接口
//! 关键特性：每次操作后自动截图返回

use super::types::*;
use crate::services::mcp::McpService;
use anyhow::{anyhow, Result};
// use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
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
            BrowserAction::TypeKeys { keys } => self.type_keys(keys).await,
            BrowserAction::Wait { duration_ms } => self.wait(*duration_ms).await,
            BrowserAction::Navigate { url } => self.navigate(url).await,
            BrowserAction::SelectOption { selector, value } => {
                self.select_option(selector, value).await
            }
            // 新增：元素标注相关操作
            BrowserAction::AnnotateElements => self.annotate_elements_action().await,
            BrowserAction::ClickByIndex { index } => self.click_by_index(*index).await,
            BrowserAction::SetAutoAnnotation { enabled } => self.set_auto_annotation(*enabled).await,
            BrowserAction::GetAnnotatedElements => self.get_annotated_elements_action().await,
            BrowserAction::FillByIndex { index, value } => self.fill_by_index(*index, value).await,
            BrowserAction::HoverByIndex { index } => self.hover_by_index(*index).await,
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
        // 参数说明：
        // - name: 截图名称（必需）
        // - storeBase64: 存储为 base64 格式（默认 true）
        // - savePng: 保存为 PNG 文件（默认 false）
        let result = self.call_playwright_tool("playwright_screenshot", json!({
            "name": "vision_screenshot",
            "storeBase64": true,
            "savePng": false
        })).await?;
        
        debug!("Screenshot response: {:?}", result);
        
        // 优先从 images 数组提取（MCP 响应中的图片数据）
        if let Some(images) = result.get("images") {
            if let Some(arr) = images.as_array() {
                if let Some(first_image) = arr.first() {
                    if let Some(base64) = first_image.as_str() {
                        info!("Got screenshot from images array, length: {}", base64.len());
                        return Ok(base64.to_string());
                    }
                }
            }
        }
        
        // 尝试从 output 字段提取
        if let Some(output) = result.get("output") {
            if let Some(base64) = extract_base64_from_value(output) {
                return Ok(base64);
            }
        }
        
        // 尝试从整个响应提取
        if let Some(base64) = extract_base64_from_value(&result) {
            return Ok(base64);
        }
        
        // 打印完整响应用于调试
        warn!("Failed to extract screenshot. Response: {:?}", result);
        
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
        
        if let Some(coords) = coordinates {
            info!("Clicking at coordinates ({}, {}) with {} button", coords.x, coords.y, button_str);
            
            // 方法1：使用 JavaScript 找到坐标位置的元素并点击
            // 这样可以确保链接等元素被正确触发
            let click_js = format!(
                r#"(function() {{
                    const el = document.elementFromPoint({}, {});
                    if (el) {{
                        // 如果是链接，直接导航
                        if (el.tagName === 'A' && el.href) {{
                            window.location.href = el.href;
                            return {{ success: true, type: 'navigation', href: el.href }};
                        }}
                        // 否则模拟点击
                        el.click();
                        return {{ success: true, type: 'click', tag: el.tagName }};
                    }}
                    return {{ success: false, error: 'No element at coordinates' }};
                }})()"#,
                coords.x, coords.y
            );
            
            let js_result = self.call_playwright_tool("playwright_evaluate", json!({
                "script": click_js
            })).await;
            
            match js_result {
                Ok(result) => {
                    debug!("JS click result: {:?}", result);
                    // 等待页面响应
                    sleep(Duration::from_millis(300)).await;
                    return Ok(ActionResult {
                        success: true,
                        error: None,
                        screenshot: None,
                        duration_ms: 0,
                    });
                }
                Err(e) => {
                    warn!("JS click failed: {}, falling back to mouse click", e);
                }
            }
            
            // 方法2：回退到 playwright_click 的坐标点击
            let mut params = json!({
                "button": button_str,
                "coordinate": [coords.x, coords.y]
            });
            
            for i in 0..click_count {
                let result = self.call_playwright_tool("playwright_click", params.clone()).await?;
                debug!("Mouse click {} result: {:?}", i + 1, result);
                if click_count > 1 {
                    sleep(Duration::from_millis(100)).await;
                }
            }
        } else {
            warn!("Click without coordinates - this may not work as expected");
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
        info!("Navigating to {} with viewport {}x{}, headless: {}, proxy: {:?}", 
            url, self.config.viewport_width, self.config.viewport_height, 
            self.config.headless, self.config.browser_proxy);
        
        let mut params = json!({
            "url": url,
            "width": self.config.viewport_width,
            "height": self.config.viewport_height,
            "headless": self.config.headless
        });
        
        // 添加代理配置
        if let Some(ref proxy_server) = self.config.browser_proxy {
            params["proxy"] = json!({"server": proxy_server});
        }

        // 添加自定义 header
        if let Some(ref headers) = self.config.headers {
            params["headers"] = json!(headers);
        }

        // 添加自定义 local storage
        if let Some(ref storage) = self.config.local_storage {
            params["localStorage"] = json!(storage);
        }
        
        self.call_playwright_tool("playwright_navigate", params).await?;
        
        // 等待页面加载
        sleep(Duration::from_millis(1000)).await;
        
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

    // ========== 新增：元素标注相关方法 ==========

    /// 标注页面所有可交互元素 (BrowserAction 版本)
    async fn annotate_elements_action(&self) -> Result<ActionResult> {
        info!("Annotating all interactive elements on page");
        let result = self.call_playwright_tool("playwright_annotate", json!({})).await?;
        
        // 解析返回的元素数量
        let element_count = if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
            content.iter()
                .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                .find(|t| t.contains("Found"))
                .and_then(|t| t.split_whitespace().nth(1))
                .and_then(|n| n.parse::<usize>().ok())
                .unwrap_or(0)
        } else {
            0
        };
        
        info!("Annotated {} elements", element_count);
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 通过索引点击元素
    async fn click_by_index(&self, index: u32) -> Result<ActionResult> {
        info!("Clicking element by index: {}", index);
        
        let result = self.call_playwright_tool("playwright_click_by_index", json!({
            "index": index
        })).await?;
        
        // 检查是否有错误
        if let Some(is_error) = result.get("isError").and_then(|e| e.as_bool()) {
            if is_error {
                let error_msg = result.get("content")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|c| c.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("Unknown error");
                return Err(anyhow!("Click by index failed: {}", error_msg));
            }
        }
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 通过索引填充输入框
    async fn fill_by_index(&self, index: u32, value: &str) -> Result<ActionResult> {
        info!("Filling element by index: {} with value: {}", index, value);
        
        let result = self.call_playwright_tool("playwright_fill_by_index", json!({
            "index": index,
            "value": value
        })).await?;
        
        // 检查是否有错误
        if let Some(is_error) = result.get("isError").and_then(|e| e.as_bool()) {
            if is_error {
                let error_msg = result.get("content")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|c| c.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("Unknown error");
                return Err(anyhow!("Fill by index failed: {}", error_msg));
            }
        }
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 通过索引悬停元素（用于发现悬停菜单）
    pub async fn hover_by_index(&self, index: u32) -> Result<ActionResult> {
        info!("Hovering element by index: {}", index);
        
        // 使用 JavaScript 触发悬停事件
        let hover_script = format!(
            r#"(async function() {{
                // 获取标注元素
                const elements = window.__elements_map__ || window.__playwrightAnnotatedElements || {{}};
                const element = elements[{}];
                
                if (!element) {{
                    return {{ success: false, error: 'Element not found at index {}' }};
                }}
                
                // 记录悬停前的可见元素计数
                const beforeCount = document.querySelectorAll(
                    '[role="menu"], [role="listbox"], .dropdown-menu, .submenu, .tooltip, [aria-expanded="true"]'
                ).length;
                
                // 获取元素中心坐标
                const rect = element.getBoundingClientRect();
                const centerX = rect.left + rect.width / 2;
                const centerY = rect.top + rect.height / 2;
                
                // 发送 mouseover 和 mouseenter 事件
                const mouseoverEvent = new MouseEvent('mouseover', {{
                    bubbles: true,
                    cancelable: true,
                    view: window,
                    clientX: centerX,
                    clientY: centerY
                }});
                
                const mouseenterEvent = new MouseEvent('mouseenter', {{
                    bubbles: false,
                    cancelable: true,
                    view: window,
                    clientX: centerX,
                    clientY: centerY
                }});
                
                element.dispatchEvent(mouseenterEvent);
                element.dispatchEvent(mouseoverEvent);
                
                // 也触发 focus 事件（某些元素需要）
                if (element.focus) {{
                    element.focus();
                }}
                
                // 等待动画和 DOM 更新
                await new Promise(resolve => setTimeout(resolve, 350));
                
                // 检测新出现的元素
                const afterElements = document.querySelectorAll(
                    '[role="menu"], [role="listbox"], .dropdown-menu, .submenu, .tooltip, [aria-expanded="true"]'
                );
                
                const newCount = afterElements.length - beforeCount;
                
                // 检测 aria-expanded 状态变化
                const expanded = element.getAttribute('aria-expanded');
                
                return {{
                    success: true,
                    before_count: beforeCount,
                    after_count: afterElements.length,
                    new_elements_count: newCount,
                    aria_expanded: expanded,
                    element_text: (element.textContent || '').substring(0, 50)
                }};
            }})()"#,
            index, index
        );
        
        let result = self.evaluate_js(&hover_script).await?;
        
        // 检查结果
        let success = result.get("success").and_then(|s| s.as_bool()).unwrap_or(false);
        
        if !success {
            let error = result.get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("Unknown error");
            return Err(anyhow!("Hover by index failed: {}", error));
        }
        
        let new_count = result.get("new_elements_count").and_then(|n| n.as_i64()).unwrap_or(0);
        if new_count > 0 {
            info!("Hover revealed {} new elements", new_count);
        }
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 设置自动标注开关
    async fn set_auto_annotation(&self, enabled: bool) -> Result<ActionResult> {
        info!("Setting auto annotation: {}", enabled);
        self.call_playwright_tool("playwright_set_auto_annotation", json!({
            "enabled": enabled
        })).await?;
        
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 获取已标注元素列表 (BrowserAction 版本)
    async fn get_annotated_elements_action(&self) -> Result<ActionResult> {
        let _elements = self.get_annotated_elements().await?;
        Ok(ActionResult {
            success: true,
            error: None,
            screenshot: None,
            duration_ms: 0,
        })
    }

    /// 标注页面元素并返回元素列表
    pub async fn annotate_elements(&self) -> Result<Vec<AnnotatedElement>> {
        info!("Annotating elements and getting list");
        let result = self.call_playwright_tool("playwright_annotate", json!({})).await?;
        
        // 从响应中提取元素列表
        self.parse_annotated_elements(&result)
    }

    /// 获取已标注的元素列表
    pub async fn get_annotated_elements(&self) -> Result<Vec<AnnotatedElement>> {
        let result = self.call_playwright_tool("playwright_get_annotated_elements", json!({})).await?;
        self.parse_annotated_elements(&result)
    }

    /// 解析标注元素响应
    /// 支持多种返回格式：
    /// 1. MCP content 格式: { "content": [{ "text": "{\"annotated_elements\": [...]}" }] }
    /// 2. MCP output 格式: { "success": true, "output": "...{\"annotated_elements\": [...]}..." }
    /// 3. 直接格式: { "annotated_elements": [...] }
    fn parse_annotated_elements(&self, result: &Value) -> Result<Vec<AnnotatedElement>> {
        // 格式1: MCP content 数组格式
        if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
            for item in content {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    if let Some(elements) = self.extract_elements_from_text(text) {
                        return Ok(elements);
                    }
                }
            }
        }
        
        // 格式2: MCP output 字符串格式 { "success": true, "output": "..." }
        if let Some(output) = result.get("output").and_then(|o| o.as_str()) {
            if let Some(elements) = self.extract_elements_from_text(output) {
                return Ok(elements);
            }
        }
        
        // 格式3: 直接包含 annotated_elements
        if let Some(elements_arr) = result.get("annotated_elements").and_then(|e| e.as_array()) {
            let elements = self.convert_elements_array(elements_arr);
            info!("Parsed {} annotated elements (direct format)", elements.len());
            return Ok(elements);
        }
        
        warn!("Could not parse annotated elements from response: {:?}", result);
        Ok(Vec::new())
    }
    
    /// 从文本中提取 annotated_elements JSON
    fn extract_elements_from_text(&self, text: &str) -> Option<Vec<AnnotatedElement>> {
        // 查找 {"annotated_elements": 开始的位置
        if let Some(start) = text.find("{\"annotated_elements\"") {
            // 从这个位置开始查找对应的闭合括号
            let json_start = start;
            let remaining = &text[json_start..];
            
            // 简单的括号匹配找到完整的 JSON 对象
            let mut depth = 0;
            let mut end_pos = 0;
            for (i, ch) in remaining.char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end_pos = i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            
            if end_pos > 0 {
                let json_str = &remaining[..end_pos];
                if let Ok(parsed) = serde_json::from_str::<Value>(json_str) {
                    if let Some(elements_arr) = parsed.get("annotated_elements").and_then(|e| e.as_array()) {
                        let elements = self.convert_elements_array(elements_arr);
                        info!("Parsed {} annotated elements from text", elements.len());
                        return Some(elements);
                    }
                }
            }
        }
        None
    }
    
    /// 将元素数组转换为 AnnotatedElement 列表
    /// 支持完整格式和紧凑格式
    fn convert_elements_array(&self, arr: &[Value]) -> Vec<AnnotatedElement> {
        arr.iter().filter_map(|el| {
            // 尝试检测格式：紧凑格式使用 "i" 而非 "index"
            let is_compact = el.get("i").is_some();
            
            if is_compact {
                // 紧凑格式: { i, t, x?, h?, p? }
                let index = el.get("i").and_then(|v| v.as_u64())? as u32;
                let element_type = el.get("t").and_then(|v| v.as_str())?.to_string();
                let text = el.get("x").and_then(|v| v.as_str()).unwrap_or("").to_string();
                
                let mut attributes = std::collections::HashMap::new();
                if let Some(href) = el.get("h").and_then(|v| v.as_str()) {
                    attributes.insert("href".to_string(), href.to_string());
                }
                if let Some(placeholder) = el.get("p").and_then(|v| v.as_str()) {
                    attributes.insert("placeholder".to_string(), placeholder.to_string());
                }
                
                Some(AnnotatedElement {
                    index,
                    element_type,
                    tag_name: String::new(), // 紧凑格式不包含
                    text,
                    selector: String::new(), // 紧凑格式不包含
                    bounding_box: BoundingBox { x: 0.0, y: 0.0, width: 0.0, height: 0.0 },
                    attributes,
                })
            } else {
                // 完整格式：直接反序列化
                serde_json::from_value(el.clone()).ok()
            }
        }).collect()
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
    pub async fn evaluate_js(&self, script: &str) -> Result<Value> {
        let result = self.call_playwright_tool("playwright_evaluate", json!({
            "script": script
        })).await?;
        
        // MCP 返回格式: { "success": true, "output": "Executed JavaScript:\n...\nResult:\n..." }
        // 需要从 "Result:\n" 后的文本中提取实际结果
        if let Some(output) = result.get("output").and_then(|o| o.as_str()) {
            // 查找 "Result:\n" 后面的内容
            if let Some(result_idx) = output.find("Result:\n") {
                let result_str = &output[result_idx + 8..]; // "Result:\n" 长度为 8
                let result_str = result_str.trim();
                
                // 尝试解析为 JSON
                if let Ok(parsed) = serde_json::from_str::<Value>(result_str) {
                    return Ok(parsed);
                }
                // 如果不是 JSON，返回字符串（去掉可能的引号）
                let cleaned = result_str.trim_matches('"');
                return Ok(Value::String(cleaned.to_string()));
            }
        }
        
        // 旧格式兼容: { "content": [{ "type": "text", "text": "..." }], "isError": false }
        if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
            let mut found_result = false;
            for item in content {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    if found_result {
                        if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                            return Ok(parsed);
                        }
                        return Ok(Value::String(text.to_string()));
                    }
                    if text == "Result:" {
                        found_result = true;
                    }
                }
            }
        }
        
        // 直接返回结果
        Ok(result)
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
                            const attrs = {};
                            if (el.href) attrs.href = el.href;
                            if (el.name) attrs.name = el.name;
                            if (el.className) attrs.class = el.className;
                            if (el.placeholder) attrs.placeholder = el.placeholder;
                            if (el.getAttribute('aria-label')) attrs['aria-label'] = el.getAttribute('aria-label');
                            if (el.getAttribute('role')) attrs.role = el.getAttribute('role');
                            if (el.title) attrs.title = el.title;
                            if (el.getAttribute('type')) attrs.type = el.getAttribute('type'); // Ensure type is in attributes too for consistency
                            
                            elements.push({
                                id: el.id || `element_${idx}`,
                                tag: el.tagName.toLowerCase(),
                                text: (el.innerText || el.value || el.placeholder || '').slice(0, 100),
                                selector: el.id ? `#${el.id}` : `${el.tagName.toLowerCase()}:nth-of-type(${idx + 1})`,
                                element_type: el.type || null,
                                attributes: attrs,
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
        
        // 记录原始返回值用于调试
        debug!("get_interactable_elements raw result: {:?}", result);
        
        let elements: Vec<PageElement> = match serde_json::from_value::<Vec<PageElement>>(result.clone()) {
            Ok(elems) => {
                info!("Parsed {} interactable elements", elems.len());
                elems
            }
            Err(e) => {
                warn!("Failed to parse interactable elements: {}. Raw value: {:?}", e, result);
                Vec::new()
            }
        };
        
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
        
        debug!("get_forms raw result: {:?}", result);
        
        let forms: Vec<FormInfo> = match serde_json::from_value::<Vec<FormInfo>>(result.clone()) {
            Ok(f) => {
                info!("Parsed {} forms", f.len());
                f
            }
            Err(e) => {
                warn!("Failed to parse forms: {}. Raw value: {:?}", e, result);
                Vec::new()
            }
        };
        
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

    /// 采集当前页面状态 (多模态模式 - 包含截图)
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
            annotated_elements: Vec::new(), // 多模态模式不需要标注元素列表
            forms,
            links,
            visible_text_summary: None,
            captured_at: Utc::now(),
        })
    }

    /// 采集当前页面状态 (文本模式 - 不含截图，包含标注元素列表)
    /// 用于非多模态模型，通过元素列表而非截图来理解页面
    pub async fn capture_page_state_text_mode(&self) -> Result<PageState> {
        // 先标注元素，annotate_elements() 会返回标注后的元素列表
        // 同时也会将元素存储到 window.__playwrightAnnotatedElements 供后续操作使用
        let annotated_elements = self.annotate_elements().await?;
        
        // 并行获取其他页面信息（不截图）
        let (url, title, elements, forms) = tokio::try_join!(
            self.get_current_url(),
            self.get_page_title(),
            self.get_interactable_elements(),
            self.get_forms()
        )?;
        
        // 提取链接
        let links: Vec<PageElement> = elements.iter()
            .filter(|e| e.tag == "a")
            .cloned()
            .collect();
        
        info!("Text mode: captured {} annotated elements", annotated_elements.len());
        
        Ok(PageState {
            url,
            title,
            screenshot: None, // 文本模式不需要截图
            interactable_elements: elements,
            annotated_elements,
            forms,
            links,
            visible_text_summary: None,
            captured_at: Utc::now(),
        })
    }

    /// 调用Playwright MCP工具
    async fn call_playwright_tool(&self, tool_name: &str, params: Value) -> Result<Value> {
        debug!("Calling Playwright tool: {} with params: {:?}", tool_name, params);
        
        // 查找playwright连接（大小写不敏感）
        let connections = self.mcp_service.get_connection_info().await?;
        
        // 打印所有可用连接用于调试
        debug!("Available MCP connections: {:?}", 
            connections.iter().map(|c| format!("{}(status={})", c.name, c.status)).collect::<Vec<_>>());
        
        let playwright_conn = connections.iter()
            .find(|c| c.name.to_lowercase().contains("playwright") && c.status.to_lowercase() == "connected");
        
        if playwright_conn.is_none() {
            // 打印更详细的错误信息
            let available = connections.iter()
                .map(|c| format!("{}({})", c.name, c.status))
                .collect::<Vec<_>>()
                .join(", ");
            warn!("Playwright MCP not found. Available connections: [{}]", available);
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

    /// 关闭浏览器
    pub async fn close_browser(&self) -> Result<()> {
        info!("Closing browser via playwright_close");
        
        match self.call_playwright_tool("playwright_close", json!({})).await {
            Ok(_) => {
                info!("Browser closed successfully");
                Ok(())
            }
            Err(e) => {
                warn!("Failed to close browser: {}", e);
                // 不抛出错误，因为浏览器可能已经关闭
                Ok(())
            }
        }
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
        BrowserAction::SelectOption { selector, value } => (
            "computer_select_option".to_string(),
            json!({ "selector": selector, "value": value })
        ),
        // 新增：元素标注相关操作
        BrowserAction::AnnotateElements => (
            "playwright_annotate".to_string(),
            json!({})
        ),
        BrowserAction::ClickByIndex { index } => (
            "playwright_click_by_index".to_string(),
            json!({ "index": index })
        ),
        BrowserAction::SetAutoAnnotation { enabled } => (
            "playwright_set_auto_annotation".to_string(),
            json!({ "enabled": enabled })
        ),
        BrowserAction::GetAnnotatedElements => (
            "playwright_get_annotated_elements".to_string(),
            json!({})
        ),
        BrowserAction::FillByIndex { index, value } => (
            "playwright_fill_by_index".to_string(),
            json!({ "index": index, "value": value })
        ),
        BrowserAction::HoverByIndex { index } => (
            "hover_by_index".to_string(),
            json!({ "index": index })
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
        
        // 新增：元素标注相关工具
        "playwright_annotate" | "annotate" | "annotate_elements" => {
            Ok(BrowserAction::AnnotateElements)
        }
        
        "playwright_click_by_index" | "click_by_index" => {
            let index = params.get("index")
                .and_then(|i| i.as_u64())
                .unwrap_or(0) as u32;
            Ok(BrowserAction::ClickByIndex { index })
        }
        
        "playwright_fill_by_index" | "fill_by_index" => {
            let index = params.get("index")
                .and_then(|i| i.as_u64())
                .unwrap_or(0) as u32;
            let value = params.get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(BrowserAction::FillByIndex { index, value })
        }
        
        "playwright_set_auto_annotation" | "set_auto_annotation" => {
            let enabled = params.get("enabled")
                .and_then(|e| e.as_bool())
                .unwrap_or(true);
            Ok(BrowserAction::SetAutoAnnotation { enabled })
        }
        
        "playwright_get_annotated_elements" | "get_annotated_elements" | "get_elements" => {
            Ok(BrowserAction::GetAnnotatedElements)
        }
        
        _ => Err(anyhow!("Unknown tool: {}", tool_name))
    }
}

/// 从 JSON 值中提取 base64 数据
/// 支持多种可能的响应格式
fn extract_base64_from_value(value: &Value) -> Option<String> {
    // 优先检查 screenshot_base64 字段（MCP playwright 工具返回格式）
    if let Some(base64) = value.get("screenshot_base64").and_then(|v| v.as_str()) {
        if base64.len() > 100 {
            return Some(base64.to_string());
        }
    }
    
    // 直接是字符串
    if let Some(s) = value.as_str() {
        // 检查是否看起来像 base64（长度合理且只包含有效字符）
        if s.len() > 100 && s.chars().all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=') {
            return Some(s.to_string());
        }
        // 尝试从 RawImageContent 格式提取: [Non-text content: Image(RawImageContent { data: "...", ... })]
        if let Some(base64) = extract_base64_from_raw_image_content(s) {
            return Some(base64);
        }
    }
    
    // { "content": [{ "type": "image", "data": "base64..." }] }
    if let Some(content) = value.get("content") {
        if let Some(arr) = content.as_array() {
            for item in arr {
                if item.get("type").and_then(|t| t.as_str()) == Some("image") {
                    if let Some(data) = item.get("data").and_then(|d| d.as_str()) {
                        return Some(data.to_string());
                    }
                }
                // { "source": { "type": "base64", "data": "..." } }
                if let Some(source) = item.get("source") {
                    if let Some(data) = source.get("data").and_then(|d| d.as_str()) {
                        return Some(data.to_string());
                    }
                }
            }
        }
        // content 直接是字符串
        if let Some(data) = content.as_str() {
            if data.len() > 100 {
                return Some(data.to_string());
            }
        }
    }
    
    // 从 output 字段提取
    if let Some(output) = value.get("output").and_then(|v| v.as_str()) {
        // 尝试解析 JSON 格式: {"screenshot_base64": "...", "mimeType": "..."}
        if let Ok(json) = serde_json::from_str::<Value>(output) {
            if let Some(base64) = json.get("screenshot_base64").and_then(|v| v.as_str()) {
                if base64.len() > 100 {
                    return Some(base64.to_string());
                }
            }
        }
        // 尝试从 RawImageContent 格式提取
        if let Some(base64) = extract_base64_from_raw_image_content(output) {
            return Some(base64);
        }
    }
    
    // 常见字段名
    for key in &["base64", "data", "image", "screenshot", "result"] {
        if let Some(val) = value.get(*key) {
            if let Some(s) = val.as_str() {
                if s.len() > 100 {
                    return Some(s.to_string());
                }
            }
            // 递归查找
            if let Some(found) = extract_base64_from_value(val) {
                return Some(found);
            }
        }
    }
    
    None
}

/// 从 RawImageContent 格式字符串中提取 base64 数据
/// 格式: [Non-text content: Image(RawImageContent { data: "...", mime_type: "...", meta: None })]
fn extract_base64_from_raw_image_content(s: &str) -> Option<String> {
    // 查找 data: " 开始位置
    let data_marker = "data: \"";
    let start_idx = s.find(data_marker)?;
    let data_start = start_idx + data_marker.len();
    
    // 从 data_start 查找结束引号
    let remaining = &s[data_start..];
    let end_idx = remaining.find('"')?;
    
    let base64_data = &remaining[..end_idx];
    if base64_data.len() > 100 {
        Some(base64_data.to_string())
    } else {
        None
    }
}


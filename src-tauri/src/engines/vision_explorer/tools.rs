//! 浏览器工具封装
//!
//! 封装Playwright MCP工具为computer_*风格的统一接口
//! 关键特性：每次操作后自动截图返回

use super::types::*;
use crate::commands::passive_scan_commands::PassiveScanState;
use crate::services::mcp::McpService;
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

// Re-export tool conversion functions from playwright_bridge module
pub use super::playwright_bridge::{
    action_to_tool_call, extract_base64_from_value, parse_tool_call_to_action,
};

// ============================================================================
// Text Mode Enhanced Types
// ============================================================================

/// Page state info from JS script
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageStateInfo {
    #[serde(default)]
    pub has_loading: bool,
    #[serde(default)]
    pub has_empty_state: bool,
    #[serde(default)]
    pub has_pagination: bool,
    #[serde(default)]
    pub scroll_position: f64,
    #[serde(default)]
    pub scroll_height: f64,
    #[serde(default)]
    pub client_height: f64,
    #[serde(default)]
    pub can_scroll_more: bool,
}

/// Visible table info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisibleTableInfo {
    #[serde(default)]
    pub headers: String,
    #[serde(default)]
    pub rows: usize,
}

/// Visible list info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisibleListInfo {
    #[serde(default)]
    pub count: usize,
    #[serde(default)]
    pub preview: String,
}

/// Visible text and regions info for text mode
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisibleTextInfo {
    #[serde(default)]
    pub header_text: String,
    #[serde(default)]
    pub nav_items: Vec<String>,
    #[serde(default)]
    pub main_headings: Vec<String>,
    #[serde(default)]
    pub main_content_preview: String,
    #[serde(default)]
    pub sidebar_items: Vec<String>,
    #[serde(default)]
    pub footer_text: String,
    #[serde(default)]
    pub visible_tables: Vec<VisibleTableInfo>,
    #[serde(default)]
    pub visible_lists: Vec<VisibleListInfo>,
    #[serde(default)]
    pub form_labels: Vec<String>,
    #[serde(default)]
    pub alerts_and_messages: Vec<String>,
    #[serde(default)]
    pub page_state: PageStateInfo,
}

/// Form field state info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FormFieldState {
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub tag: String,
    #[serde(default, rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub readonly: bool,
    #[serde(default, rename = "validationError")]
    pub validation_error: String,
    #[serde(default)]
    pub options: Option<Vec<String>>,
}

/// Lazy load detection info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LazyLoadInfo {
    #[serde(default)]
    pub has_infinite_scroll: bool,
    #[serde(default)]
    pub has_load_more_button: bool,
    #[serde(default)]
    pub has_pagination: bool,
    #[serde(default)]
    pub at_bottom: bool,
    #[serde(default)]
    pub visible_item_count: usize,
}

/// Active overlay (modal/drawer/popover) detection info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActiveOverlayInfo {
    /// Whether there's an active overlay blocking interactions
    #[serde(default, rename = "hasActiveOverlay")]
    pub has_active_overlay: bool,
    /// Type of overlay: 'modal', 'drawer', 'popover', 'dropdown'
    #[serde(default, rename = "overlayType")]
    pub overlay_type: Option<String>,
    /// CSS selector to identify the overlay
    #[serde(default, rename = "overlaySelector")]
    pub overlay_selector: Option<String>,
    /// Index of the close button if found in annotated elements
    #[serde(default, rename = "closeButtonIndex")]
    pub close_button_index: Option<u32>,
    /// Element indices that are inside the overlay (can be interacted with)
    #[serde(default, rename = "elementIndicesInOverlay")]
    pub element_indices_in_overlay: Vec<u32>,
    /// Element indices that are blocked by the overlay (cannot be interacted with)
    #[serde(default, rename = "elementIndicesBlocked")]
    pub element_indices_blocked: Vec<u32>,
    /// Suggested dismiss action: 'click_close', 'click_outside', 'press_escape', 'none'
    #[serde(default, rename = "dismissAction")]
    pub dismiss_action: Option<String>,
}

/// Truncate string to max chars
fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

/// 浏览器工具服务
pub struct BrowserTools {
    mcp_service: Arc<McpService>,
    config: VisionExplorerConfig,
    /// 被动扫描状态，用于获取当前运行的代理配置
    passive_scan_state: Option<Arc<PassiveScanState>>,
}

impl BrowserTools {
    pub fn new(mcp_service: Arc<McpService>, config: VisionExplorerConfig) -> Self {
        Self {
            mcp_service,
            config,
            passive_scan_state: None,
        }
    }

    /// 设置被动扫描状态（用于动态获取代理配置）- builder pattern
    pub fn with_passive_scan_state(mut self, state: Arc<PassiveScanState>) -> Self {
        self.passive_scan_state = Some(state);
        self
    }

    /// 设置被动扫描状态（用于动态获取代理配置）- mutable reference
    pub fn set_passive_scan_state(&mut self, state: Arc<PassiveScanState>) {
        self.passive_scan_state = Some(state);
    }

    /// 执行浏览器操作并自动截图
    pub async fn execute_action(&self, action: &BrowserAction) -> Result<ActionResult> {
        let start_time = std::time::Instant::now();

        info!("Executing browser action: {:?}", action);

        // 执行操作
        let result = match action {
            BrowserAction::Screenshot => {
                if !self.config.enable_multimodal {
                    // Text-only mode has no visual capability; avoid taking screenshots to save time/resources.
                    warn!("Text mode: screenshot action requested but multimodal is disabled, skipping");
                    Ok(ActionResult {
                        success: true,
                        error: None,
                        screenshot: None,
                        duration_ms: 0,
                    })
                } else {
                    self.take_screenshot().await
                }
            }
            BrowserAction::MoveMouse { coordinates } => self.move_mouse(coordinates).await,
            BrowserAction::ClickMouse {
                coordinates,
                button,
                click_count,
            } => {
                self.click_mouse(coordinates.as_ref(), button, *click_count)
                    .await
            }
            BrowserAction::Scroll {
                coordinates,
                direction,
                scroll_count,
            } => {
                self.scroll(coordinates.as_ref(), direction, *scroll_count)
                    .await
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
            BrowserAction::SetAutoAnnotation { enabled } => {
                self.set_auto_annotation(*enabled).await
            }
            BrowserAction::GetAnnotatedElements => self.get_annotated_elements_action().await,
            BrowserAction::FillByIndex { index, value } => self.fill_by_index(*index, value).await,
            BrowserAction::HoverByIndex { index } => self.hover_by_index(*index).await,
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(mut action_result) => {
                action_result.duration_ms = duration_ms;

                // 对于非截图操作，等待UI稳定后自动截图
                // 仅多模态模式需要截图；文本模式使用元素标注/列表理解页面
                if self.config.enable_multimodal && !matches!(action, BrowserAction::Screenshot) {
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
        let result = self
            .call_playwright_tool(
                "playwright_screenshot",
                json!({
                    "name": "vision_screenshot",
                    "storeBase64": true,
                    "savePng": false
                }),
            )
            .await?;

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
        self.call_playwright_tool(
            "playwright_hover",
            json!({
                "coordinate": [coordinates.x, coordinates.y]
            }),
        )
        .await?;

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
            info!(
                "Clicking at coordinates ({}, {}) with {} button",
                coords.x, coords.y, button_str
            );

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

            let js_result = self
                .call_playwright_tool(
                    "playwright_evaluate",
                    json!({
                        "script": click_js
                    }),
                )
                .await;

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
                let result = self
                    .call_playwright_tool("playwright_click", params.clone())
                    .await?;
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
            self.call_playwright_tool(
                "playwright_press",
                json!({
                    "key": key
                }),
            )
            .await?;
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
        // 动态获取代理配置：优先从运行中的被动扫描服务获取，否则使用配置中的静态值
        let effective_proxy = if let Some(ref state) = self.passive_scan_state {
            // 尝试从运行中的代理服务获取当前地址
            let running_proxy = state.get_running_proxy_address().await;
            if running_proxy.is_some() {
                running_proxy
            } else {
                // 代理未运行，使用配置中的静态值
                self.config.browser_proxy.clone()
            }
        } else {
            // 没有PassiveScanState，使用配置中的静态值
            self.config.browser_proxy.clone()
        };

        info!(
            "Navigating to {} with viewport {}x{}, headless: {}, proxy: {:?}",
            url,
            self.config.viewport_width,
            self.config.viewport_height,
            self.config.headless,
            effective_proxy
        );

        let mut params = json!({
            "url": url,
            "width": self.config.viewport_width,
            "height": self.config.viewport_height,
            "headless": self.config.headless
        });

        // 添加代理配置（使用动态获取的代理地址）
        if let Some(ref proxy_server) = effective_proxy {
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

        self.call_playwright_tool("playwright_navigate", params)
            .await?;

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
        self.call_playwright_tool(
            "playwright_select_option",
            json!({
                "selector": selector,
                "value": value
            }),
        )
        .await?;

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
        let result = self
            .call_playwright_tool("playwright_annotate", json!({}))
            .await?;

        // 解析返回的元素数量
        let element_count = if let Some(content) = result.get("content").and_then(|c| c.as_array())
        {
            content
                .iter()
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

    /// 获取指定索引元素的中心坐标
    async fn get_element_coordinates(&self, index: u32) -> Result<Option<Coordinates>> {
        let script = format!(
            r#"(function() {{
                const elements = window.__elements_map__ || window.__playwrightAnnotatedElements || {{}};
                const el = elements[{}];
                if (el) {{
                    const rect = el.getBoundingClientRect();
                    return {{ x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 }};
                }}
                return null;
            }})()"#,
            index
        );
        let result = self.evaluate_js(&script).await?;

        if result.is_null() {
            return Ok(None);
        }

        if let (Some(x), Some(y)) = (
            result.get("x").and_then(|v| v.as_f64()),
            result.get("y").and_then(|v| v.as_f64()),
        ) {
            Ok(Some(Coordinates {
                x: x as i32,
                y: y as i32,
            }))
        } else {
            Ok(None)
        }
    }

    /// 通过索引点击元素
    async fn click_by_index(&self, index: u32) -> Result<ActionResult> {
        info!("Clicking element by index: {}", index);

        let result = self
            .call_playwright_tool(
                "playwright_click_by_index",
                json!({
                    "index": index
                }),
            )
            .await?;

        // 检查是否有错误
        if let Some(is_error) = result.get("isError").and_then(|e| e.as_bool()) {
            if is_error {
                let error_msg = result
                    .get("content")
                    .and_then(|c| c.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|c| c.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("Unknown error");

                warn!(
                    "JS click failed: {}. Attempting fallback to coordinate click...",
                    error_msg
                );

                // Fallback: Get coordinates and click
                if let Ok(Some(coords)) = self.get_element_coordinates(index).await {
                    info!(
                        "Fallback: Clicking at coordinates ({}, {})",
                        coords.x, coords.y
                    );
                    return self.click_mouse(Some(&coords), &MouseButton::Left, 1).await;
                } else {
                    return Err(anyhow!(
                        "Click by index failed and fallback coordinates not found: {}",
                        error_msg
                    ));
                }
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

        let result = self
            .call_playwright_tool(
                "playwright_fill_by_index",
                json!({
                    "index": index,
                    "value": value
                }),
            )
            .await?;

        // 检查是否有错误
        if let Some(is_error) = result.get("isError").and_then(|e| e.as_bool()) {
            if is_error {
                let error_msg = result
                    .get("content")
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
        let success = result
            .get("success")
            .and_then(|s| s.as_bool())
            .unwrap_or(false);

        if !success {
            let error = result
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("Unknown error");
            return Err(anyhow!("Hover by index failed: {}", error));
        }

        let new_count = result
            .get("new_elements_count")
            .and_then(|n| n.as_i64())
            .unwrap_or(0);
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
        self.call_playwright_tool(
            "playwright_set_auto_annotation",
            json!({
                "enabled": enabled
            }),
        )
        .await?;

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
        let result = self
            .call_playwright_tool("playwright_annotate", json!({}))
            .await?;

        // 从响应中提取元素列表
        self.parse_annotated_elements(&result)
    }

    /// 获取已标注的元素列表
    pub async fn get_annotated_elements(&self) -> Result<Vec<AnnotatedElement>> {
        let result = self
            .call_playwright_tool("playwright_get_annotated_elements", json!({}))
            .await?;
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
            info!(
                "Parsed {} annotated elements (direct format)",
                elements.len()
            );
            return Ok(elements);
        }

        warn!(
            "Could not parse annotated elements from response: {:?}",
            result
        );
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
                    if let Some(elements_arr) =
                        parsed.get("annotated_elements").and_then(|e| e.as_array())
                    {
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
        arr.iter()
            .filter_map(|el| {
                // 尝试检测格式：紧凑格式使用 "i" 而非 "index"
                let is_compact = el.get("i").is_some();

                if is_compact {
                    // 紧凑格式: { i, t, x?, h?, p? }
                    let index = el.get("i").and_then(|v| v.as_u64())? as u32;
                    let element_type = el.get("t").and_then(|v| v.as_str())?.to_string();
                    let text = el
                        .get("x")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

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
                        bounding_box: BoundingBox {
                            x: 0.0,
                            y: 0.0,
                            width: 0.0,
                            height: 0.0,
                        },
                        attributes,
                        enhanced_attributes: None,
                    })
                } else {
                    // 完整格式：直接反序列化
                    serde_json::from_value(el.clone()).ok()
                }
            })
            .collect()
    }

    /// 获取页面可见HTML
    pub async fn get_page_html(&self) -> Result<String> {
        let result = self
            .call_playwright_tool("playwright_get_visible_html", json!({}))
            .await?;

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
        let result = self
            .call_playwright_tool(
                "playwright_evaluate",
                json!({
                    "script": script
                }),
            )
            .await?;

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

        let elements: Vec<PageElement> =
            match serde_json::from_value::<Vec<PageElement>>(result.clone()) {
                Ok(elems) => {
                    info!("Parsed {} interactable elements", elems.len());
                    elems
                }
                Err(e) => {
                    warn!(
                        "Failed to parse interactable elements: {}. Raw value: {:?}",
                        e, result
                    );
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

    // ========== 元素快照系统方法 - 解决 Index 漂移问题 ==========

    /// 创建元素快照
    /// 在 annotate_elements() 之后调用，将当前元素的 DOM 引用锁定到带唯一 ID 的快照中
    /// 返回 snapshot_id，后续 click_by_index / fill_by_index 操作可以验证此 ID
    pub async fn create_element_snapshot(&self) -> Result<String> {
        use super::browser_scripts::CREATE_ELEMENT_SNAPSHOT_SCRIPT;

        let result = self.evaluate_js(CREATE_ELEMENT_SNAPSHOT_SCRIPT).await?;

        // 检查是否成功
        let success = result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !success {
            let error = result
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            return Err(anyhow!("Failed to create element snapshot: {}", error));
        }

        // 提取 snapshot_id
        let snapshot_id = result
            .get("snapshotId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No snapshotId in response"))?
            .to_string();

        let element_count = result
            .get("elementCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        debug!(
            "Created element snapshot {} with {} elements",
            snapshot_id, element_count
        );

        Ok(snapshot_id)
    }

    /// 验证元素快照是否仍然有效
    /// 在执行 click_by_index / fill_by_index 之前调用，检查：
    /// 1. snapshot_id 是否匹配
    /// 2. 目标 index 的元素是否仍然存在且未发生漂移
    /// 返回 (有效性, 错误类型, 回退坐标)
    pub async fn validate_element_snapshot(
        &self,
        snapshot_id: &str,
        index: u32,
    ) -> Result<(bool, Option<String>, Option<Coordinates>)> {
        use super::browser_scripts::validate_snapshot_script;

        let script = validate_snapshot_script(snapshot_id, index);
        let result = self.evaluate_js(&script).await?;

        let valid = result
            .get("valid")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if valid {
            return Ok((true, None, None));
        }

        // 提取错误类型
        let error_type = result
            .get("error")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let message = result.get("message").and_then(|v| v.as_str()).unwrap_or("");

        warn!("Snapshot validation failed: {:?} - {}", error_type, message);

        // 尝试提取回退坐标 (从原始快照的 fingerprint 中)
        let fallback_coords = result.get("fingerprint").and_then(|fp| {
            let x = fp.get("centerX").and_then(|v| v.as_f64())?;
            let y = fp.get("centerY").and_then(|v| v.as_f64())?;
            Some(Coordinates {
                x: x as i32,
                y: y as i32,
            })
        });

        Ok((false, error_type, fallback_coords))
    }

    /// 通过快照执行点击操作
    /// 使用 snapshot_id + index 定位元素，会先验证快照再执行点击
    /// 如果快照无效，返回错误并提供回退坐标（如果可用）
    pub async fn click_by_snapshot(&self, snapshot_id: &str, index: u32) -> Result<ActionResult> {
        use super::browser_scripts::click_by_snapshot_script;

        info!("Clicking element by snapshot: {}:{}", snapshot_id, index);

        let script = click_by_snapshot_script(snapshot_id, index);
        let result = self.evaluate_js(&script).await?;

        let success = result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if success {
            return Ok(ActionResult {
                success: true,
                error: None,
                screenshot: None,
                duration_ms: 0,
            });
        }

        // 获取错误信息
        let error = result
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        let message = result.get("message").and_then(|v| v.as_str()).unwrap_or("");

        // 检查是否有回退坐标
        if let Some(coords) = result.get("fallbackCoords") {
            if let (Some(x), Some(y)) = (
                coords.get("x").and_then(|v| v.as_f64()),
                coords.get("y").and_then(|v| v.as_f64()),
            ) {
                warn!(
                    "Snapshot click failed ({}), falling back to coordinates ({}, {})",
                    error, x, y
                );
                let fallback_coords = Coordinates {
                    x: x as i32,
                    y: y as i32,
                };
                return self
                    .click_mouse(Some(&fallback_coords), &MouseButton::Left, 1)
                    .await;
            }
        }

        Err(anyhow!("Click by snapshot failed: {} - {}", error, message))
    }

    /// 通过快照执行填充操作
    pub async fn fill_by_snapshot(
        &self,
        snapshot_id: &str,
        index: u32,
        value: &str,
    ) -> Result<ActionResult> {
        use super::browser_scripts::fill_by_snapshot_script;

        info!(
            "Filling element by snapshot: {}:{} with value: {}",
            snapshot_id,
            index,
            if value.len() > 20 {
                format!("{}...", &value[..20])
            } else {
                value.to_string()
            }
        );

        let script = fill_by_snapshot_script(snapshot_id, index, value);
        let result = self.evaluate_js(&script).await?;

        let success = result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if success {
            return Ok(ActionResult {
                success: true,
                error: None,
                screenshot: None,
                duration_ms: 0,
            });
        }

        // 获取错误信息
        let error = result
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        let message = result.get("message").and_then(|v| v.as_str()).unwrap_or("");

        Err(anyhow!("Fill by snapshot failed: {} - {}", error, message))
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

    /// 采集当前页面状态 (多模态模式 - 包含截图和标注元素)
    /// 多模态模型依赖截图理解页面，同时保留 annotated_elements 以支持 fill_by_index / click_by_index 操作
    pub async fn capture_page_state(&self) -> Result<PageState> {
        // 先标注元素，确保 window.__playwrightAnnotatedElements 可用
        let annotated_elements = self.annotate_elements().await?;

        // 创建元素快照 - 解决 index 漂移问题
        // 快照会锁定当前元素的 DOM 引用和指纹，后续操作可以验证是否操作正确的元素
        let snapshot_id = self.create_element_snapshot().await?;
        info!("Created element snapshot: {}", snapshot_id);

        // 并行获取 URL / title / DOM elements / forms
        let (url, title, elements, forms) = tokio::try_join!(
            self.get_current_url(),
            self.get_page_title(),
            self.get_interactable_elements(),
            self.get_forms()
        )?;

        // 截图在 annotate 之后，确保截图包含标注覆盖层
        let screenshot = self.take_screenshot_raw().await?;

        // 提取链接
        let links: Vec<PageElement> = elements.iter().filter(|e| e.tag == "a").cloned().collect();

        Ok(PageState {
            url,
            title,
            screenshot: Some(screenshot),
            interactable_elements: elements,
            annotated_elements,
            forms,
            links,
            visible_text_summary: None,
            captured_at: Utc::now(),
            snapshot_id: Some(snapshot_id),
        })
    }

    /// 采集当前页面状态 (文本模式 - 不含截图，包含标注元素列表)
    /// 用于非多模态模型，通过元素列表而非截图来理解页面
    pub async fn capture_page_state_text_mode(&self) -> Result<PageState> {
        // 先标注元素，annotate_elements() 会返回标注后的元素列表
        // 同时也会将元素存储到 window.__playwrightAnnotatedElements 供后续操作使用
        let mut annotated_elements = self.annotate_elements().await?;

        // 创建元素快照 - 解决 index 漂移问题
        let snapshot_id = self.create_element_snapshot().await?;
        info!("Created element snapshot: {}", snapshot_id);

        // 增强：获取额外属性（颜色、遮挡等）并合并
        if let Ok(enhanced_attrs) = self.get_enhanced_element_attributes().await {
            info!("Fetched {} enhanced attributes", enhanced_attrs.len());
            // Create a map for faster lookup
            let attrs_map: std::collections::HashMap<u32, EnhancedElementAttributes> =
                enhanced_attrs
                    .into_iter()
                    .map(|attr| (attr.index, attr))
                    .collect();

            for el in &mut annotated_elements {
                if let Some(attr) = attrs_map.get(&el.index) {
                    el.enhanced_attributes = Some(attr.clone());
                }
            }
        }

        // 并行获取其他页面信息（不截图）
        let (url, title, elements, forms) = tokio::try_join!(
            self.get_current_url(),
            self.get_page_title(),
            self.get_interactable_elements(),
            self.get_forms()
        )?;

        // 提取链接
        let links: Vec<PageElement> = elements.iter().filter(|e| e.tag == "a").cloned().collect();

        info!(
            "Text mode: captured {} annotated elements",
            annotated_elements.len()
        );

        // Generate enhanced page semantic summary for text mode
        // 如果元素为空，生成更详细的页面状态信息帮助 AI 决策
        let visible_text_summary = if annotated_elements.is_empty() {
            warn!("No annotated elements found, generating fallback page context");
            self.generate_empty_elements_fallback_summary(&url, &title)
                .await
        } else {
            self.generate_enhanced_page_summary(&annotated_elements, &forms, &url, &title)
                .await
        };

        Ok(PageState {
            url,
            title,
            screenshot: None, // 文本模式不需要截图
            interactable_elements: elements,
            annotated_elements,
            forms,
            links,
            visible_text_summary,
            captured_at: Utc::now(),
            snapshot_id: Some(snapshot_id),
        })
    }

    /// Generate semantic summary of the page for text mode (basic version)
    async fn generate_page_semantic_summary(
        &self,
        elements: &[AnnotatedElement],
        forms: &[FormInfo],
        url: &str,
        title: &str,
    ) -> Option<String> {
        // Infer page type from URL and title
        let page_type = Self::infer_page_type(url, title, elements);

        // Count element types
        let link_count = elements.iter().filter(|e| e.element_type == "link").count();
        let button_count = elements
            .iter()
            .filter(|e| e.element_type == "button" || e.element_type == "submit")
            .count();
        let input_count = elements
            .iter()
            .filter(|e| e.element_type == "input" || e.element_type == "textarea")
            .count();
        let select_count = elements
            .iter()
            .filter(|e| e.element_type == "select")
            .count();

        // Identify key regions
        let has_navigation = elements.iter().any(|e| {
            e.attributes
                .get("role")
                .map(|r| r == "navigation")
                .unwrap_or(false)
                || e.attributes
                    .get("class")
                    .map(|c| c.to_lowercase().contains("nav"))
                    .unwrap_or(false)
        });
        let has_sidebar = elements.iter().any(|e| {
            e.attributes
                .get("class")
                .map(|c| {
                    let cl = c.to_lowercase();
                    cl.contains("sidebar") || cl.contains("aside") || cl.contains("menu")
                })
                .unwrap_or(false)
        });
        let has_search = elements.iter().any(|e| {
            e.attributes
                .get("placeholder")
                .map(|p| p.to_lowercase().contains("search"))
                .unwrap_or(false)
                || e.attributes
                    .get("name")
                    .map(|n| n.to_lowercase().contains("search"))
                    .unwrap_or(false)
                || e.text.to_lowercase().contains("search")
        });
        let has_pagination = elements.iter().any(|e| {
            e.text.to_lowercase().contains("next")
                || e.text.to_lowercase().contains("prev")
                || e.attributes
                    .get("class")
                    .map(|c| c.to_lowercase().contains("paginat"))
                    .unwrap_or(false)
        });

        // Identify key features
        let mut features = Vec::new();
        if has_search {
            features.push("search");
        }
        if has_pagination {
            features.push("pagination");
        }
        if !forms.is_empty() {
            features.push("forms");
        }
        if elements.iter().any(|e| {
            e.text.to_lowercase().contains("logout") || e.text.to_lowercase().contains("退出")
        }) {
            features.push("logged_in");
        }

        // Build regions string
        let mut regions = Vec::new();
        if has_navigation {
            regions.push("navigation");
        }
        if has_sidebar {
            regions.push("sidebar");
        }
        regions.push("main_content");

        Some(format!(
            "Type: {} | Regions: {} | Features: {} | Elements: {} links, {} buttons, {} inputs, {} selects, {} forms",
            page_type,
            if regions.is_empty() { "unknown" } else { &regions.join("+") },
            if features.is_empty() { "none" } else { &features.join("+") },
            link_count,
            button_count,
            input_count,
            select_count,
            forms.len()
        ))
    }

    /// Generate enhanced page summary with visible text and state info (text mode)
    async fn generate_enhanced_page_summary(
        &self,
        elements: &[AnnotatedElement],
        forms: &[FormInfo],
        url: &str,
        title: &str,
    ) -> Option<String> {
        // Get basic summary
        let basic_summary = self
            .generate_page_semantic_summary(elements, forms, url, title)
            .await?;

        // Try to get visible text and regions
        let visible_info = self.get_visible_text_and_regions().await;
        let form_state = self.get_form_fields_state().await;
        let lazy_load_info = self.get_lazy_load_detection().await;

        // Use Spatial Tree instead of basic lists
        let spatial_tree = self.generate_spatial_tree(elements, forms, url, title);

        let mut parts = vec![
            basic_summary,
            spatial_tree, // Insert Spatial Tree here
        ];

        // Add visible text info

        if let Ok(info) = visible_info {
            // Main headings
            if !info.main_headings.is_empty() {
                parts.push(format!("Headings: {}", info.main_headings.join(" > ")));
            }

            // Navigation items (compact)
            if !info.nav_items.is_empty() {
                let nav_preview = info
                    .nav_items
                    .iter()
                    .take(8)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                parts.push(format!("Nav: {}", nav_preview));
            }

            // Main content preview
            if !info.main_content_preview.is_empty() {
                parts.push(format!(
                    "Content: {}",
                    truncate(&info.main_content_preview, 150)
                ));
            }

            // Tables
            if !info.visible_tables.is_empty() {
                let table_info: Vec<String> = info
                    .visible_tables
                    .iter()
                    .map(|t| format!("{}cols/{}rows", t.headers.split(',').count(), t.rows))
                    .collect();
                parts.push(format!("Tables: {}", table_info.join(", ")));
            }

            // Lists
            if !info.visible_lists.is_empty() {
                let list_info: Vec<String> = info
                    .visible_lists
                    .iter()
                    .map(|l| format!("{}items", l.count))
                    .collect();
                parts.push(format!("Lists: {}", list_info.join(", ")));
            }

            // Alerts/messages
            if !info.alerts_and_messages.is_empty() {
                parts.push(format!("Alerts: {}", info.alerts_and_messages.join(" | ")));
            }

            // Page state
            if info.page_state.has_loading {
                parts.push("State: LOADING".to_string());
            }
            if info.page_state.has_empty_state {
                parts.push("State: EMPTY/NO_DATA".to_string());
            }
            if info.page_state.can_scroll_more {
                parts.push("Scroll: MORE_CONTENT_BELOW".to_string());
            }
        }

        // Add form field state info
        if let Ok(fields) = form_state {
            let filled_fields: Vec<String> = fields
                .iter()
                .filter(|f| !f.value.is_empty())
                .map(|f| format!("{}={}", f.label, f.value))
                .take(5)
                .collect();
            if !filled_fields.is_empty() {
                parts.push(format!("Filled: {}", filled_fields.join(", ")));
            }

            let errors: Vec<String> = fields
                .iter()
                .filter(|f| !f.validation_error.is_empty())
                .map(|f| format!("{}: {}", f.label, f.validation_error))
                .take(3)
                .collect();
            if !errors.is_empty() {
                parts.push(format!("Errors: {}", errors.join("; ")));
            }
        }

        // Add lazy load info
        if let Ok(lazy_info) = lazy_load_info {
            if lazy_info.has_load_more_button {
                parts.push("HasLoadMore: YES".to_string());
            }
            if lazy_info.has_infinite_scroll {
                parts.push("InfiniteScroll: YES".to_string());
            }
            if lazy_info.at_bottom && lazy_info.visible_item_count > 0 {
                parts.push(format!(
                    "Items: {}, AtBottom: YES",
                    lazy_info.visible_item_count
                ));
            }
        }

        // Add active overlay info (modal/drawer/popover detection)
        if let Ok(overlay_info) = self.get_active_overlay_info().await {
            if overlay_info.has_active_overlay {
                let overlay_type = overlay_info.overlay_type.as_deref().unwrap_or("unknown");
                let dismiss_action = overlay_info.dismiss_action.as_deref().unwrap_or("none");

                parts.push(format!(
                    "\n⚠️ ACTIVE OVERLAY DETECTED: type={}, dismiss_via={}",
                    overlay_type, dismiss_action
                ));

                if let Some(close_idx) = overlay_info.close_button_index {
                    parts.push(format!("  → Close button at index [{}]", close_idx));
                }

                if !overlay_info.element_indices_in_overlay.is_empty() {
                    let in_overlay: Vec<String> = overlay_info
                        .element_indices_in_overlay
                        .iter()
                        .take(10)
                        .map(|i| i.to_string())
                        .collect();
                    let suffix = if overlay_info.element_indices_in_overlay.len() > 10 {
                        format!(
                            "... +{} more",
                            overlay_info.element_indices_in_overlay.len() - 10
                        )
                    } else {
                        String::new()
                    };
                    parts.push(format!(
                        "  → Elements INSIDE overlay (can interact): [{}]{}",
                        in_overlay.join(", "),
                        suffix
                    ));
                }

                if !overlay_info.element_indices_blocked.is_empty() {
                    parts.push(format!(
                        "  → {} elements BLOCKED by overlay (cannot interact until dismissed)",
                        overlay_info.element_indices_blocked.len()
                    ));
                }

                parts.push("  → IMPORTANT: Close the overlay first before interacting with elements behind it!".to_string());
            }
        }

        Some(parts.join("\n"))
    }

    /// Get visible text and regions from page
    async fn get_visible_text_and_regions(&self) -> Result<VisibleTextInfo> {
        let result = self
            .evaluate_js(super::browser_scripts::VISIBLE_TEXT_AND_REGIONS_SCRIPT)
            .await?;

        let info: VisibleTextInfo = serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse visible text info: {}", e))?;
        Ok(info)
    }

    /// Get form fields state
    async fn get_form_fields_state(&self) -> Result<Vec<FormFieldState>> {
        let result = self
            .evaluate_js(super::browser_scripts::FORM_FIELDS_STATE_SCRIPT)
            .await?;

        let fields: Vec<FormFieldState> = serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse form fields state: {}", e))?;
        Ok(fields)
    }

    /// Get lazy load detection info
    async fn get_lazy_load_detection(&self) -> Result<LazyLoadInfo> {
        let result = self
            .evaluate_js(super::browser_scripts::LAZY_LOAD_DETECTION_SCRIPT)
            .await?;

        let info: LazyLoadInfo = serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse lazy load info: {}", e))?;
        Ok(info)
    }

    /// Get active overlay (modal/drawer/popover) info
    /// Returns information about any active overlay that may be blocking element interactions
    pub async fn get_active_overlay_info(&self) -> Result<ActiveOverlayInfo> {
        let result = self
            .evaluate_js(super::browser_scripts::DETECT_ACTIVE_OVERLAY_SCRIPT)
            .await?;

        let info: ActiveOverlayInfo = serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse active overlay info: {}", e))?;

        if info.has_active_overlay {
            info!(
                "Detected active overlay: type={:?}, close_button_index={:?}, elements_in_overlay={}, elements_blocked={}",
                info.overlay_type,
                info.close_button_index,
                info.element_indices_in_overlay.len(),
                info.element_indices_blocked.len()
            );
        }

        Ok(info)
    }

    /// 当元素列表为空时，生成回退摘要帮助 AI 理解页面状态
    /// 这在以下场景特别有用：
    /// 1. 页面正在加载
    /// 2. 空白页面 / 404 / 错误页面
    /// 3. 纯静态内容页面（无可交互元素）
    /// 4. iframe 内容未加载
    async fn generate_empty_elements_fallback_summary(
        &self,
        url: &str,
        title: &str,
    ) -> Option<String> {
        let mut parts = vec![];

        // 基本页面信息
        parts.push(format!("⚠️ NO INTERACTIVE ELEMENTS DETECTED"));
        parts.push(format!("URL: {}", url));
        parts.push(format!("Title: {}", title));

        // 尝试获取页面可见文本
        if let Ok(visible_info) = self.get_visible_text_and_regions().await {
            // 标题
            if !visible_info.main_headings.is_empty() {
                parts.push(format!(
                    "Page Headings: {}",
                    visible_info.main_headings.join(" > ")
                ));
            }

            // 主要内容
            if !visible_info.main_content_preview.is_empty() {
                parts.push(format!(
                    "Visible Text: {}",
                    truncate(&visible_info.main_content_preview, 500)
                ));
            }

            // 警告/提示信息（可能包含错误信息）
            if !visible_info.alerts_and_messages.is_empty() {
                parts.push(format!(
                    "Alerts/Messages: {}",
                    visible_info.alerts_and_messages.join(" | ")
                ));
            }

            // 页面状态检测
            let mut status_hints = vec![];
            if visible_info.page_state.has_loading {
                status_hints.push("LOADING_INDICATOR_PRESENT");
            }
            if visible_info.page_state.has_empty_state {
                status_hints.push("EMPTY_STATE_DETECTED");
            }
            if visible_info.page_state.can_scroll_more {
                status_hints.push("CAN_SCROLL_FOR_MORE");
            }
            if !status_hints.is_empty() {
                parts.push(format!("Page State: {}", status_hints.join(", ")));
            }

            // 表格信息
            if !visible_info.visible_tables.is_empty() {
                let table_desc: Vec<String> = visible_info
                    .visible_tables
                    .iter()
                    .map(|t| format!("Table({}rows, headers: {})", t.rows, t.headers))
                    .collect();
                parts.push(format!("Tables: {}", table_desc.join("; ")));
            }

            // 列表信息
            if !visible_info.visible_lists.is_empty() {
                let list_desc: Vec<String> = visible_info
                    .visible_lists
                    .iter()
                    .map(|l| format!("List({}items: {})", l.count, truncate(&l.preview, 50)))
                    .collect();
                parts.push(format!("Lists: {}", list_desc.join("; ")));
            }
        }

        // 使用 Playwright 工具获取可见文本作为回退
        if let Ok(visible_text) = self.get_visible_text_via_playwright().await {
            if !visible_text.is_empty() && parts.len() < 5 {
                parts.push(format!(
                    "Visible Text (raw): {}",
                    truncate(&visible_text, 800)
                ));
            }
        }

        // 检测页面类型并给出建议
        let suggestion = self.infer_empty_page_suggestion(url, title, &parts);
        parts.push(format!("\n📋 SUGGESTED ACTIONS: {}", suggestion));

        Some(parts.join("\n"))
    }

    /// 使用 Playwright MCP 工具获取页面可见文本
    async fn get_visible_text_via_playwright(&self) -> Result<String> {
        let result = self
            .call_playwright_tool("playwright_get_visible_text", json!({}))
            .await?;

        // 尝试从不同格式提取文本
        // 格式1: { "text": "..." }
        if let Some(text) = result.get("text").and_then(|t| t.as_str()) {
            return Ok(text.to_string());
        }

        // 格式2: { "content": [{ "type": "text", "text": "..." }] }
        if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
            let texts: Vec<&str> = content
                .iter()
                .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                .collect();
            if !texts.is_empty() {
                return Ok(texts.join("\n"));
            }
        }

        // 格式3: { "output": "..." }
        if let Some(output) = result.get("output").and_then(|o| o.as_str()) {
            return Ok(output.to_string());
        }

        // 格式4: 直接是字符串
        if let Some(text) = result.as_str() {
            return Ok(text.to_string());
        }

        Ok(String::new())
    }

    /// 根据页面信息推断建议操作
    fn infer_empty_page_suggestion(&self, url: &str, title: &str, parts: &[String]) -> String {
        let url_lower = url.to_lowercase();
        let title_lower = title.to_lowercase();
        let text = parts.join(" ").to_lowercase();

        // 检测加载中状态
        if text.contains("loading") || text.contains("加载") || text.contains("spinner") {
            return "Page appears to be loading. Wait 2-3 seconds and refresh elements with 'get_elements'.".to_string();
        }

        // 检测登录/认证页面
        if url_lower.contains("login")
            || url_lower.contains("signin")
            || url_lower.contains("auth")
            || title_lower.contains("login")
            || title_lower.contains("登录")
        {
            return "Login page detected but no form elements found. Try scrolling down or check if page is fully loaded.".to_string();
        }

        // 检测错误页面
        if text.contains("404")
            || text.contains("not found")
            || text.contains("error")
            || text.contains("错误")
            || text.contains("找不到")
        {
            return "Error page detected. Consider navigating back or to a different URL."
                .to_string();
        }

        // 检测空白页面
        if text.len() < 100 {
            return "Page appears mostly blank. Try: 1) Wait for load, 2) Scroll to reveal content, 3) Check if JavaScript is blocking.".to_string();
        }

        // 检测 iframe 内容
        if text.contains("iframe") || url_lower.contains("frame") {
            return "Page may contain iframe content. Interactive elements might be inside frames."
                .to_string();
        }

        // 默认建议
        "No interactive elements found. Options: 1) Wait and retry get_elements, 2) Scroll page, 3) Check page HTML structure, 4) Navigate to different URL.".to_string()
    }

    /// 获取增强元素属性
    async fn get_enhanced_element_attributes(&self) -> Result<Vec<EnhancedElementAttributes>> {
        let result = self
            .evaluate_js(super::browser_scripts::ENHANCED_ELEMENT_ATTRIBUTES_SCRIPT)
            .await?;

        let attrs: Vec<EnhancedElementAttributes> = serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to parse enhanced attributes: {}", e))?;
        Ok(attrs)
    }

    /// 生成空间布局树（用于文本模式）
    fn generate_spatial_tree(
        &self,
        elements: &[AnnotatedElement],
        forms: &[FormInfo],
        url: &str,
        title: &str,
    ) -> String {
        // 1. 定义区域 (Header, Sidebar, Main, Footer)
        // 使用配置的 Viewport 大小进行动态划分
        let vw = self.config.viewport_width as f64;
        let vh = self.config.viewport_height as f64;

        // 动态阈值策略:
        // Header: 顶部 15% 区域
        // Sidebar: 左侧 25% 区域 (通常 Sidebar 在左侧)
        // Footer: 底部 10% 区域 (仅当确实位于底部时)

        let header_height = vh * 0.15;
        let sidebar_width = vw * 0.25;
        let footer_threshold = vh * 0.90;

        // 辅助检测：如果页面很宽(>1200px)，Main Content 可能是居中的，Sidebar 可能在左侧
        // 如果页面很窄，Sidebar 可能是隐藏的或者是顶部导航的一部分

        let mut headers: Vec<&AnnotatedElement> = Vec::new();
        let mut sidebars: Vec<&AnnotatedElement> = Vec::new();
        let mut mains: Vec<&AnnotatedElement> = Vec::new();
        let mut footers: Vec<&AnnotatedElement> = Vec::new();

        for el in elements {
            // 跳过被遮挡的元素（如果是纯文本模式且我们信任遮挡检测）
            if let Some(enhanced) = &el.enhanced_attributes {
                if enhanced.is_occluded {
                    continue;
                }
            }

            let y = el.bounding_box.y;
            let x = el.bounding_box.x;

            if y < header_height {
                headers.push(el);
            } else if x < sidebar_width {
                sidebars.push(el);
            } else if y > footer_threshold && title.len() > 10000 {
                // 只有超长页面才轻易归为footer，这里简单化
                footers.push(el);
            } else {
                mains.push(el);
            }
        }

        // 2. 格式化输出
        let mut output = String::new();
        output.push_str("=== SPATIAL LAYOUT TREE ===\n");

        if !headers.is_empty() {
            output.push_str("\n[HEADER REGION] (Top Navigation)\n");
            output.push_str(&self.format_region_elements(&headers));
        }

        if !sidebars.is_empty() {
            output.push_str("\n[SIDEBAR REGION] (Left Navigation/Menu)\n");
            output.push_str(&self.format_region_elements(&sidebars));
        }

        if !mains.is_empty() {
            output.push_str("\n[MAIN CONTENT REGION]\n");
            output.push_str(&self.format_region_elements(&mains));
        }

        if !footers.is_empty() {
            output.push_str("\n[FOOTER REGION]\n");
            output.push_str(&self.format_region_elements(&footers));
        }

        output
    }

    /// 格式化区域内元素
    fn format_region_elements(&self, elements: &[&AnnotatedElement]) -> String {
        // 按 Y 坐标排序，如果 Y 接近（同一行），按 X 排序
        let mut sorted: Vec<&AnnotatedElement> = elements.to_vec();
        sorted.sort_by(|a, b| {
            let y_diff = (a.bounding_box.y - b.bounding_box.y).abs();
            if y_diff < 10.0 {
                a.bounding_box
                    .x
                    .partial_cmp(&b.bounding_box.x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                a.bounding_box
                    .y
                    .partial_cmp(&b.bounding_box.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        let mut output = String::new();
        for el in sorted.iter().take(50) {
            // 限制每个区域最多显示50个元素，防止溢出
            let mut flags = Vec::new();
            let mut extra_info = String::new();

            if let Some(enhanced) = &el.enhanced_attributes {
                // 颜色语义
                if let Some(color) = &enhanced.computed_styles.color_semantic {
                    if color != "transparent" && color != "black" && color != "white" {
                        flags.push(format!("Color:{}", color));
                    }
                }

                // 状态
                for state in &enhanced.derived_state {
                    flags.push(state.clone());
                }

                // 隐式标签
                if el.text.trim().is_empty() {
                    if let Some(label) = &enhanced.inferred_label {
                        extra_info = format!(" (Inferred: {})", label);
                    }
                }
            }

            if let Some(placeholder) = el.attributes.get("placeholder") {
                if !placeholder.is_empty() {
                    extra_info.push_str(&format!(" [ph: {}]", placeholder));
                }
            }

            let flags_str = if flags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", flags.join("|"))
            };

            // 格式: [index] <tag> "text" extra flags
            output.push_str(&format!(
                "  [{}] <{}> \"{}\"{}{}\n",
                el.index,
                el.tag_name,
                el.text.trim().replace('\n', " "),
                extra_info,
                flags_str
            ));
        }

        if sorted.len() > 50 {
            output.push_str(&format!("  ... and {} more elements\n", sorted.len() - 50));
        }

        output
    }

    fn infer_page_type(url: &str, title: &str, elements: &[AnnotatedElement]) -> &'static str {
        let url_lower = url.to_lowercase();
        let title_lower = title.to_lowercase();

        // Login/Auth pages
        if url_lower.contains("login")
            || url_lower.contains("signin")
            || url_lower.contains("auth")
            || title_lower.contains("login")
            || title_lower.contains("登录")
        {
            return "login";
        }

        // Dashboard/Admin pages
        if url_lower.contains("dashboard")
            || url_lower.contains("admin")
            || url_lower.contains("console")
            || title_lower.contains("dashboard")
            || title_lower.contains("控制台")
        {
            return "dashboard";
        }

        // List pages (many similar links)
        let link_texts: Vec<_> = elements
            .iter()
            .filter(|e| e.element_type == "link")
            .map(|e| e.text.len())
            .collect();
        if link_texts.len() > 10 {
            let avg_len = link_texts.iter().sum::<usize>() / link_texts.len();
            let variance: f64 = link_texts
                .iter()
                .map(|&l| (l as f64 - avg_len as f64).powi(2))
                .sum::<f64>()
                / link_texts.len() as f64;
            if variance < 100.0 {
                return "list";
            }
        }

        // Detail pages (few links, more content)
        if url_lower.contains("/detail")
            || url_lower.contains("/view")
            || url_lower.contains("/show")
            || url.split('/').filter(|s| s.parse::<u64>().is_ok()).count() > 0
        {
            return "detail";
        }

        // Form pages
        let input_count = elements
            .iter()
            .filter(|e| e.element_type == "input" || e.element_type == "textarea")
            .count();
        if input_count > 3 {
            return "form";
        }

        // Settings pages
        if url_lower.contains("setting")
            || url_lower.contains("config")
            || url_lower.contains("preference")
            || title_lower.contains("setting")
            || title_lower.contains("设置")
        {
            return "settings";
        }

        // Home/Index pages
        if url_lower.ends_with('/') || url_lower.contains("index") || url_lower.contains("home") {
            return "home";
        }

        "content"
    }

    /// Get simplified DOM skeleton for text mode
    pub async fn get_dom_skeleton(&self) -> Result<String> {
        let result = self
            .evaluate_js(super::browser_scripts::DOM_SKELETON_SCRIPT)
            .await?;

        if let Some(skeleton) = result.as_str() {
            if !skeleton.is_empty() {
                return Ok(skeleton.to_string());
            }
        }

        // Fallback: return empty string if failed
        Ok(String::new())
    }

    /// 调用Playwright MCP工具
    async fn call_playwright_tool(&self, tool_name: &str, params: Value) -> Result<Value> {
        debug!(
            "Calling Playwright tool: {} with params: {:?}",
            tool_name, params
        );

        // 查找playwright连接（大小写不敏感）
        let connections = self.mcp_service.get_connection_info().await?;

        // 打印所有可用连接用于调试
        debug!(
            "Available MCP connections: {:?}",
            connections
                .iter()
                .map(|c| format!("{}(status={})", c.name, c.status))
                .collect::<Vec<_>>()
        );

        let playwright_conn = connections.iter().find(|c| {
            c.name.to_lowercase().contains("playwright") && c.status.to_lowercase() == "connected"
        });

        if playwright_conn.is_none() {
            // 打印更详细的错误信息
            let available = connections
                .iter()
                .map(|c| format!("{}({})", c.name, c.status))
                .collect::<Vec<_>>()
                .join(", ");
            warn!(
                "Playwright MCP not found. Available connections: [{}]",
                available
            );
            return Err(anyhow!("Playwright MCP server not connected"));
        }

        let conn_name = &playwright_conn.unwrap().name;

        // 执行工具调用
        let result = self
            .mcp_service
            .execute_client_tool(conn_name, tool_name, params)
            .await?;

        debug!("Playwright tool result: {:?}", result);

        Ok(result)
    }

    /// 关闭浏览器
    pub async fn close_browser(&self) -> Result<()> {
        info!("Closing browser via playwright_close");

        match self
            .call_playwright_tool("playwright_close", json!({}))
            .await
        {
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
// Tool definition conversion functions moved to playwright_bridge module
// Re-exported at the top of this file for backwards compatibility
// ============================================================================

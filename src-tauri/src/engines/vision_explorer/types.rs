//! 视觉探索引擎类型定义
//!
//! 定义VLM驱动的网站全流量发现所需的核心数据结构

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};

// ============================================================================
// 浏览器操作类型 (参考bytebot的computer_*风格)
// ============================================================================

/// 坐标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coordinates {
    pub x: i32,
    pub y: i32,
}

/// 鼠标按钮
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// 滚动方向
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// 浏览器操作命令
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum BrowserAction {
    /// 截图
    Screenshot,
    /// 移动鼠标
    MoveMouse { coordinates: Coordinates },
    /// 点击鼠标
    ClickMouse {
        coordinates: Option<Coordinates>,
        button: MouseButton,
        click_count: u32,
    },
    /// 滚动
    Scroll {
        coordinates: Option<Coordinates>,
        direction: ScrollDirection,
        scroll_count: u32,
    },
    /// 输入文本
    TypeText { text: String },
    /// 粘贴文本
    PasteText { text: String },
    /// 按键
    TypeKeys { keys: Vec<String> },
    /// 等待
    Wait { duration_ms: u64 },
    /// 导航到URL
    Navigate { url: String },
    /// 点击元素 (通过选择器)
    ClickElement { selector: String },
    /// 填写输入框
    FillInput { selector: String, value: String },
    /// 选择下拉选项
    SelectOption { selector: String, value: String },
}

/// 操作执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
    /// 截图 (base64)
    pub screenshot: Option<String>,
    /// 执行耗时 (ms)
    pub duration_ms: u64,
}

// ============================================================================
// 页面状态类型
// ============================================================================

/// 页面元素信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageElement {
    /// 元素唯一ID
    pub id: String,
    /// 标签名 (button, input, a, select等)
    pub tag: String,
    /// 元素文本内容
    pub text: String,
    /// CSS选择器
    pub selector: String,
    /// 元素类型 (submit, text, password, link等)
    pub element_type: Option<String>,
    /// 属性
    pub attributes: HashMap<String, String>,
    /// 位置信息
    pub bounding_box: Option<BoundingBox>,
    /// 是否可见
    pub is_visible: bool,
    /// 是否可交互
    pub is_interactable: bool,
}

/// 元素边界框
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// 表单信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormInfo {
    /// 表单ID
    pub id: String,
    /// 表单action
    pub action: Option<String>,
    /// 表单method
    pub method: Option<String>,
    /// 表单字段
    pub fields: Vec<FormField>,
}

/// 表单字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    /// 字段名
    pub name: String,
    /// 字段类型
    pub field_type: String,
    /// 是否必填
    pub required: bool,
    /// placeholder
    pub placeholder: Option<String>,
    /// 当前值
    pub value: Option<String>,
}

/// 页面状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageState {
    /// 当前URL
    pub url: String,
    /// 页面标题
    pub title: String,
    /// 截图 (base64)
    pub screenshot: Option<String>,
    /// 可交互元素列表
    pub interactable_elements: Vec<PageElement>,
    /// 表单列表
    pub forms: Vec<FormInfo>,
    /// 链接列表
    pub links: Vec<PageElement>,
    /// 可见文本摘要
    pub visible_text_summary: Option<String>,
    /// 采集时间
    pub captured_at: DateTime<Utc>,
}

// ============================================================================
// 探索状态类型
// ============================================================================

/// 探索状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplorationStatus {
    /// 初始化中
    Initializing,
    /// 探索中
    Exploring,
    /// 等待用户输入 (如验证码)
    WaitingForInput,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 失败
    Failed,
}

/// 操作记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    /// 操作ID
    pub id: String,
    /// 操作类型
    pub action: BrowserAction,
    /// 目标元素
    pub target_element: Option<String>,
    /// 执行时间
    pub executed_at: DateTime<Utc>,
    /// 执行结果
    pub result: ActionResult,
    /// 触发的API (如果有)
    pub triggered_apis: Vec<String>,
}

/// 发现的API端点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    /// 请求方法
    pub method: String,
    /// URL路径
    pub path: String,
    /// 完整URL
    pub full_url: String,
    /// 请求头
    pub headers: HashMap<String, String>,
    /// 请求参数
    pub parameters: HashMap<String, String>,
    /// 请求体
    pub body: Option<String>,
    /// 响应状态码
    pub status_code: Option<u16>,
    /// 发现时间
    pub discovered_at: DateTime<Utc>,
    /// 来源操作ID
    pub source_action_id: Option<String>,
}

/// 探索会话状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationState {
    /// 会话ID
    pub session_id: String,
    /// 目标URL
    pub target_url: String,
    /// 开始时间
    pub start_time: DateTime<Utc>,
    /// 当前状态
    pub status: ExplorationStatus,
    /// 已访问URL
    pub visited_urls: HashSet<String>,
    /// 已交互元素
    pub interacted_elements: HashSet<String>,
    /// 操作历史
    pub action_history: Vec<ActionRecord>,
    /// 发现的API
    pub discovered_apis: Vec<ApiEndpoint>,
    /// 发现的表单
    pub discovered_forms: Vec<FormInfo>,
    /// 当前页面状态
    pub current_page: Option<PageState>,
    /// 探索进度 (0-100)
    pub exploration_progress: f32,
    /// 错误信息
    pub error_message: Option<String>,
    /// 迭代次数
    pub iteration_count: u32,
    /// 最大迭代次数
    pub max_iterations: u32,
}

impl Default for ExplorationState {
    fn default() -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            target_url: String::new(),
            start_time: Utc::now(),
            status: ExplorationStatus::Initializing,
            visited_urls: HashSet::new(),
            interacted_elements: HashSet::new(),
            action_history: Vec::new(),
            discovered_apis: Vec::new(),
            discovered_forms: Vec::new(),
            current_page: None,
            exploration_progress: 0.0,
            error_message: None,
            iteration_count: 0,
            max_iterations: 100,
        }
    }
}

// ============================================================================
// VLM分析结果类型
// ============================================================================

/// VLM输出的下一步操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmNextAction {
    /// 操作类型
    pub action_type: String,
    /// 目标元素ID或选择器
    pub element_id: Option<String>,
    /// 填写内容 (如果是输入操作)
    pub value: Option<String>,
    /// 选择此操作的原因
    pub reason: String,
}

/// VLM分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmAnalysisResult {
    /// 页面功能分析
    pub page_analysis: String,
    /// 下一步操作
    pub next_action: VlmNextAction,
    /// 可能触发的API
    pub estimated_apis: Vec<String>,
    /// 探索进度估计
    pub exploration_progress: f32,
    /// 是否完成探索
    pub is_exploration_complete: bool,
    /// 完成原因 (如果完成)
    pub completion_reason: Option<String>,
}

// ============================================================================
// 配置类型
// ============================================================================

/// 视觉探索引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionExplorerConfig {
    /// 目标URL
    pub target_url: String,
    /// 最大迭代次数
    pub max_iterations: u32,
    /// 操作后等待时间 (ms)
    pub action_delay_ms: u64,
    /// 截图后等待UI稳定时间 (ms)
    pub screenshot_delay_ms: u64,
    /// 是否启用被动代理抓包
    pub enable_passive_proxy: bool,
    /// 被动代理端口
    pub passive_proxy_port: Option<u16>,
    /// 是否无头模式
    pub headless: bool,
    /// 浏览器视口宽度
    pub viewport_width: u32,
    /// 浏览器视口高度
    pub viewport_height: u32,
    /// VLM模型提供商
    pub vlm_provider: String,
    /// VLM模型名称
    pub vlm_model: String,
    /// 登录凭据 (可选)
    pub credentials: Option<LoginCredentials>,
}

/// 登录凭据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginCredentials {
    pub username: String,
    pub password: String,
}

impl Default for VisionExplorerConfig {
    fn default() -> Self {
        Self {
            target_url: String::new(),
            max_iterations: 100,
            action_delay_ms: 500,
            screenshot_delay_ms: 750, // bytebot使用750ms
            enable_passive_proxy: true,
            passive_proxy_port: Some(4201),
            headless: false,
            viewport_width: 1280,
            viewport_height: 960, // bytebot使用1280x960
            vlm_provider: "anthropic".to_string(),
            vlm_model: "claude-sonnet-4-20250514".to_string(),
            credentials: None,
        }
    }
}

// ============================================================================
// 工具定义类型 (用于LLM工具调用)
// ============================================================================

/// 浏览器工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// 获取所有浏览器工具定义
pub fn get_browser_tool_definitions() -> Vec<BrowserToolDefinition> {
    vec![
        BrowserToolDefinition {
            name: "computer_screenshot".to_string(),
            description: "Captures a screenshot of the current browser page".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        BrowserToolDefinition {
            name: "computer_click_mouse".to_string(),
            description: "Performs a mouse click at the specified coordinates or current position".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "coordinates": {
                        "type": "object",
                        "properties": {
                            "x": { "type": "integer", "description": "X coordinate" },
                            "y": { "type": "integer", "description": "Y coordinate" }
                        },
                        "required": ["x", "y"]
                    },
                    "button": {
                        "type": "string",
                        "enum": ["left", "right", "middle"],
                        "default": "left"
                    },
                    "click_count": {
                        "type": "integer",
                        "default": 1,
                        "description": "Number of clicks (2 for double-click)"
                    }
                },
                "required": ["coordinates"]
            }),
        },
        BrowserToolDefinition {
            name: "computer_type_text".to_string(),
            description: "Types text into the currently focused element".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "The text to type"
                    }
                },
                "required": ["text"]
            }),
        },
        BrowserToolDefinition {
            name: "computer_scroll".to_string(),
            description: "Scrolls the page in the specified direction".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "coordinates": {
                        "type": "object",
                        "properties": {
                            "x": { "type": "integer" },
                            "y": { "type": "integer" }
                        }
                    },
                    "direction": {
                        "type": "string",
                        "enum": ["up", "down", "left", "right"]
                    },
                    "scroll_count": {
                        "type": "integer",
                        "default": 3
                    }
                },
                "required": ["direction"]
            }),
        },
        BrowserToolDefinition {
            name: "computer_type_keys".to_string(),
            description: "Presses keyboard keys (for shortcuts like Enter, Tab, etc.)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "keys": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Array of key names to press"
                    }
                },
                "required": ["keys"]
            }),
        },
        BrowserToolDefinition {
            name: "computer_wait".to_string(),
            description: "Waits for a specified duration".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "duration_ms": {
                        "type": "integer",
                        "default": 500,
                        "description": "Duration to wait in milliseconds"
                    }
                }
            }),
        },
        BrowserToolDefinition {
            name: "computer_navigate".to_string(),
            description: "Navigates to a URL".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to navigate to"
                    }
                },
                "required": ["url"]
            }),
        },
        BrowserToolDefinition {
            name: "set_exploration_status".to_string(),
            description: "Sets the exploration status (completed or needs_help)".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["completed", "needs_help"],
                        "description": "The exploration status"
                    },
                    "description": {
                        "type": "string",
                        "description": "Summary or description of help needed"
                    }
                },
                "required": ["status", "description"]
            }),
        },
    ]
}


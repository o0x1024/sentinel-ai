//! 视觉探索引擎类型定义
//!
//! 定义VLM驱动的网站全流量发现所需的核心数据结构

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    /// 按键
    TypeKeys { keys: Vec<String> },
    /// 等待
    Wait { duration_ms: u64 },
    /// 导航到URL
    Navigate { url: String },
    /// 选择下拉选项
    SelectOption { selector: String, value: String },
    
    // ========== 新增：元素标注相关操作 ==========
    
    /// 标注页面所有可交互元素
    AnnotateElements,
    /// 通过标注索引点击元素
    ClickByIndex { index: u32 },
    /// 设置自动标注开关
    SetAutoAnnotation { enabled: bool },
    /// 获取已标注元素列表
    GetAnnotatedElements,
    /// 通过索引填充输入框（使用 fill 而非 type，更可靠）
    FillByIndex { index: u32, value: String },
    /// 通过索引悬停元素（用于发现悬停菜单）
    HoverByIndex { index: u32 },
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

/// 标注的元素信息 (来自 playwright_annotate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotatedElement {
    /// 索引号 (用于 click_by_index)
    pub index: u32,
    /// 元素类型 (link, button, input, select, checkbox, radio, textarea, form, submit, file, clickable)
    #[serde(rename = "type")]
    pub element_type: String,
    /// 标签名
    #[serde(rename = "tagName")]
    pub tag_name: String,
    /// 元素文本内容
    pub text: String,
    /// CSS选择器
    pub selector: String,
    /// 位置信息
    #[serde(rename = "boundingBox")]
    pub bounding_box: BoundingBox,
    /// 属性 (href, type, name, placeholder, value, role, aria-label 等)
    #[serde(default)]
    pub attributes: HashMap<String, String>,
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
    /// 截图 (base64) - 多模态模式使用
    pub screenshot: Option<String>,
    /// 可交互元素列表
    pub interactable_elements: Vec<PageElement>,
    /// 标注元素列表 (带索引) - 文本模式使用
    pub annotated_elements: Vec<AnnotatedElement>,
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
    /// 已访问页面 (URL -> 标题)
    pub visited_pages: HashMap<String, String>,
    /// 已交互元素
    pub interacted_elements: HashMap<String, ()>,
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
            visited_pages: HashMap::new(),
            interacted_elements: HashMap::new(),
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
    /// 目标元素ID或选择器 (兼容旧格式)
    pub element_id: Option<String>,
    /// 目标元素索引号 (新格式，优先使用)
    pub element_index: Option<u32>,
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
    /// 执行 ID (用于前端消息)
    pub execution_id: Option<String>,
    /// 消息 ID (用于前端消息)
    pub message_id: Option<String>,
    /// 会话 ID (用于前端消息)
    pub conversation_id: Option<String>,
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
    /// 浏览器代理服务器 (例如: http://127.0.0.1:8080)
    pub browser_proxy: Option<String>,
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
    /// 是否启用多模态 (图片输入)
    pub enable_multimodal: bool,
    /// 是否启用上下文摘要
    pub enable_context_summary: bool,
    /// 上下文摘要阈值 (token数，超过则触发摘要)
    pub context_summary_threshold: u32,
    /// 是否在 prompt 中包含可交互元素 JSON（用于非多模态模型）
    /// 默认关闭以节省 token，多模态模型通过截图查看元素标注
    pub include_elements_in_prompt: bool,
    /// 是否启用Takeover模式
    pub enable_takeover: bool,
    /// API发现轮询间隔 (ms)
    pub api_poll_interval_ms: u64,
    /// 探索完成时是否发送最终块以终止消息流
    pub finalize_on_complete: bool,
    /// 自定义 HTTP 请求头 (用于 Playwright 导航)
    pub headers: Option<HashMap<String, String>>,
    /// 自定义 Local Storage 数据 (用于 Playwright 导航)
    pub local_storage: Option<HashMap<String, String>>,
}

/// 登录凭据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginCredentials {
    /// 用户名/账号
    pub username: String,
    /// 密码
    pub password: String,
    /// 验证码（可选，如图形验证码、短信验证码等）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_code: Option<String>,
    /// 其他额外字段（如安全码、OTP、动态口令等）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_fields: Option<HashMap<String, String>>,
}

/// 登录字段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginField {
    /// 字段ID (也是提交时的 key)
    pub id: String,
    /// 显示标签
    pub label: String,
    /// 字段类型 (text, password, etc.)
    pub field_type: String,
    /// 是否必填
    pub required: bool,
    /// 占位符
    pub placeholder: Option<String>,
}

impl Default for VisionExplorerConfig {
    fn default() -> Self {
        Self {
            target_url: String::new(),
            max_iterations: 100,
            execution_id: None,
            message_id: None,
            conversation_id: None,
            action_delay_ms: 500,
            screenshot_delay_ms: 750, // bytebot使用750ms
            enable_passive_proxy: true,
            passive_proxy_port: Some(4201),
            headless: false,
            browser_proxy: Some("http://127.0.0.1:8080".to_string()),
            viewport_width: 1920,
            viewport_height: 1080,
            vlm_provider: "anthropic".to_string(),
            vlm_model: "claude-sonnet-4-20250514".to_string(),
            credentials: None,
            enable_multimodal: true, // 默认启用多模态
            enable_context_summary: true, // 默认启用上下文摘要
            context_summary_threshold: 50000, // 50k tokens触发摘要
            include_elements_in_prompt: false, // 默认关闭，多模态模型通过截图查看
            enable_takeover: true, // 默认启用Takeover
            api_poll_interval_ms: 2000, // 2秒轮询一次API
            finalize_on_complete: true,
            headers: None,
            local_storage: None,
        }
    }
}

// ============================================================================
// Takeover模式类型
// ============================================================================

/// Takeover模式状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TakeoverStatus {
    /// 未激活
    Inactive,
    /// 用户已接管
    Active,
    /// 等待用户操作
    WaitingForUser,
    /// 用户已归还控制权
    Returned,
}

/// 用户操作记录 (Takeover模式)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAction {
    /// 操作ID
    pub id: String,
    /// 操作类型
    pub action_type: String,
    /// 操作详情
    pub details: serde_json::Value,
    /// 操作时间
    pub timestamp: DateTime<Utc>,
    /// 触发的截图
    pub screenshot: Option<String>,
}

/// Takeover会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeoverSession {
    /// 会话ID
    pub session_id: String,
    /// 状态
    pub status: TakeoverStatus,
    /// 开始时间
    pub started_at: Option<DateTime<Utc>>,
    /// 结束时间
    pub ended_at: Option<DateTime<Utc>>,
    /// 用户操作记录
    pub user_actions: Vec<UserAction>,
    /// 原因 (为什么需要Takeover)
    pub reason: Option<String>,
    /// 用户提供的凭据 (在Takeover期间由用户输入)
    pub user_credentials: Option<LoginCredentials>,
    /// 是否检测到登录页面
    pub login_detected: bool,
    /// 登录字段定义 (动态)
    pub login_fields: Option<Vec<LoginField>>,
}

impl Default for TakeoverSession {
    fn default() -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            status: TakeoverStatus::Inactive,
            started_at: None,
            ended_at: None,
            user_actions: Vec::new(),
            reason: None,
            user_credentials: None,
            login_detected: false,
            login_fields: None,
        }
    }
}

// ============================================================================
// 上下文摘要类型
// ============================================================================

/// 上下文摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummary {
    /// 摘要ID
    pub id: String,
    /// 摘要内容
    pub content: String,
    /// 摘要覆盖的迭代范围
    pub iteration_range: (u32, u32),
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 摘要前的token估算
    pub tokens_before: u32,
    /// 摘要后的token估算
    pub tokens_after: u32,
}

/// 消息历史项 (用于上下文管理)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// 角色 (user/assistant)
    pub role: String,
    /// 内容
    pub content: String,
    /// 关联的迭代号
    pub iteration: u32,
    /// 是否包含图片
    pub has_image: bool,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
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
        // ========== 观察类工具 ==========
        BrowserToolDefinition {
            name: "screenshot".to_string(),
            description: "截取当前页面截图（包含元素标注）".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        BrowserToolDefinition {
            name: "annotate".to_string(),
            description: "标注页面所有可交互元素，返回带索引号的元素列表".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        BrowserToolDefinition {
            name: "get_elements".to_string(),
            description: "获取已标注元素列表".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        
        // ========== 交互类工具（使用元素索引） ==========
        BrowserToolDefinition {
            name: "click_by_index".to_string(),
            description: "通过元素索引号点击元素（推荐方式）".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "index": {
                        "type": "integer",
                        "description": "元素标注索引号"
                    }
                },
                "required": ["index"]
            }),
        },
        BrowserToolDefinition {
            name: "type_text".to_string(),
            description: "在当前聚焦的输入框中输入文本".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "要输入的文本"
                    }
                },
                "required": ["text"]
            }),
        },
        BrowserToolDefinition {
            name: "scroll".to_string(),
            description: "滚动页面".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "direction": {
                        "type": "string",
                        "enum": ["up", "down", "left", "right"],
                        "description": "滚动方向"
                    }
                },
                "required": ["direction"]
            }),
        },
        BrowserToolDefinition {
            name: "type_keys".to_string(),
            description: "按下键盘按键（如 Enter, Tab, Escape 等）".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "keys": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "按键名称数组"
                    }
                },
                "required": ["keys"]
            }),
        },
        BrowserToolDefinition {
            name: "hover_by_index".to_string(),
            description: "通过元素索引号悬停元素（用于发现悬停菜单）".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "index": {
                        "type": "integer",
                        "description": "元素标注索引号"
                    }
                },
                "required": ["index"]
            }),
        },
        
        // ========== 导航类工具 ==========
        BrowserToolDefinition {
            name: "navigate".to_string(),
            description: "导航到指定URL".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "目标URL"
                    }
                },
                "required": ["url"]
            }),
        },
        BrowserToolDefinition {
            name: "wait".to_string(),
            description: "等待页面稳定".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "duration_ms": {
                        "type": "integer",
                        "default": 500,
                        "description": "等待毫秒数"
                    }
                }
            }),
        },
        
        // ========== 坐标类工具（备用） ==========
        BrowserToolDefinition {
            name: "click_mouse".to_string(),
            description: "在指定坐标点击鼠标（仅当索引点击失效时使用）".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "coordinates": {
                        "type": "object",
                        "properties": {
                            "x": { "type": "integer" },
                            "y": { "type": "integer" }
                        },
                        "required": ["x", "y"]
                    }
                },
                "required": ["coordinates"]
            }),
        },
        
        // ========== 任务管理 ==========
        BrowserToolDefinition {
            name: "set_status".to_string(),
            description: "设置探索状态（完成或需要帮助）".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["completed", "needs_help"],
                        "description": "探索状态"
                    },
                    "reason": {
                        "type": "string",
                        "description": "状态说明"
                    }
                },
                "required": ["status", "reason"]
            }),
        },
    ]
}

use crate::match_replace::MatchReplaceRule;
use crate::scope::ProxyScopeRule;
use crate::{RequestContext, ResponseContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 拦截动作
#[derive(Debug, Clone)]
pub enum InterceptAction {
    Forward(Option<String>), // Forward with optional modified content
    Drop,
}

/// 拦截请求（用于等待用户操作）
pub struct PendingInterceptRequest {
    pub id: String,
    pub method: String,
    pub url: String,
    pub path: String,
    pub protocol: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: i64,
    pub response_tx: tokio::sync::oneshot::Sender<InterceptAction>,
}

/// 拦截响应（用于等待用户操作）
pub struct PendingInterceptResponse {
    pub id: String,
    pub request_id: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: i64,
    pub response_tx: tokio::sync::oneshot::Sender<InterceptAction>,
}

/// WebSocket 消息方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebSocketDirection {
    /// 客户端 -> 服务器
    ClientToServer,
    /// 服务器 -> 客户端
    ServerToClient,
}

/// WebSocket 连接上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConnectionContext {
    pub id: String,
    pub url: String,
    pub host: String,
    pub protocol: String, // "ws" or "wss"
    pub request_headers: HashMap<String, String>,
    pub response_headers: Option<HashMap<String, String>>,
    pub opened_at: chrono::DateTime<chrono::Utc>,
}

/// WebSocket 消息上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessageContext {
    pub id: String,
    pub connection_id: String,
    pub direction: WebSocketDirection,
    pub message_type: String, // "text", "binary", "ping", "pong", "close"
    pub content: Option<String>,
    pub content_length: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 拦截的 WebSocket 消息（用于等待用户操作）
pub struct PendingInterceptWebSocketMessage {
    pub id: String,
    pub connection_id: String,
    pub direction: WebSocketDirection,
    pub message_type: String,
    pub content: Option<String>,
    pub timestamp: i64,
    pub response_tx: tokio::sync::oneshot::Sender<InterceptAction>,
}

/// 拦截过滤规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptFilterRule {
    /// 是否启用
    pub enabled: bool,
    /// 操作符: And, Or
    pub operator: String,
    /// 匹配类型: domain_name, url, http_method, file_extension, etc.
    pub match_type: String,
    /// 匹配关系: matches, does_not_match
    pub relationship: String,
    /// 匹配条件（正则表达式）
    pub condition: String,
}

/// 拦截状态（共享）
#[derive(Clone)]
pub struct InterceptState {
    /// 是否启用主拦截
    pub enabled: Arc<RwLock<bool>>,
    /// 是否启用请求拦截规则
    pub request_enabled: Arc<RwLock<bool>>,
    /// 是否启用响应拦截
    pub response_enabled: Arc<RwLock<bool>>,
    /// 是否启用 WebSocket 拦截
    pub websocket_enabled: Arc<RwLock<bool>>,
    /// 待处理的拦截请求发送端
    pub pending_tx: Option<tokio::sync::mpsc::UnboundedSender<PendingInterceptRequest>>,
    /// 待处理的拦截响应发送端
    pub pending_response_tx: Option<tokio::sync::mpsc::UnboundedSender<PendingInterceptResponse>>,
    /// 待处理的拦截 WebSocket 消息发送端
    pub pending_websocket_tx:
        Option<tokio::sync::mpsc::UnboundedSender<PendingInterceptWebSocketMessage>>,
    /// 请求拦截过滤规则
    pub request_filter_rules: Arc<RwLock<Vec<InterceptFilterRule>>>,
    /// 响应拦截过滤规则
    pub response_filter_rules: Arc<RwLock<Vec<InterceptFilterRule>>>,
    /// 请求/响应匹配替换规则
    pub match_replace_rules: Arc<RwLock<Vec<MatchReplaceRule>>>,
}

/// Upstream proxy 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpstreamProxyConfig {
    /// 是否启用 upstream proxy
    #[serde(default)]
    pub enabled: bool,
    /// 目标主机匹配模式（* 表示所有）
    #[serde(default = "default_destination_host")]
    pub destination_host: String,
    /// upstream proxy 主机地址
    #[serde(default)]
    pub proxy_host: String,
    /// upstream proxy 端口
    #[serde(default)]
    pub proxy_port: u16,
    /// 认证类型（None, Basic）
    #[serde(default)]
    pub auth_type: String,
    /// 用户名（可选）
    #[serde(default)]
    pub username: Option<String>,
    /// 密码（可选）
    #[serde(default)]
    pub password: Option<String>,
}

fn default_destination_host() -> String {
    "*".to_string()
}

/// 代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// 起始端口（默认 4201）
    pub start_port: u16,
    /// 最大端口尝试次数（默认 10）
    pub max_port_attempts: u16,
    /// HTTPS MITM 是否启用（默认 true）
    pub mitm_enabled: bool,
    /// 请求体大小限制（字节，默认 2MB）
    pub max_request_body_size: usize,
    /// 响应体大小限制（字节，默认 2MB）
    pub max_response_body_size: usize,
    /// 对同一域名发生握手/证书错误的次数阈值，超过后自动绕过 MITM
    #[serde(default = "default_bypass_threshold")]
    pub mitm_bypass_fail_threshold: u32,
    /// Upstream proxy 配置
    #[serde(default)]
    pub upstream_proxy: Option<UpstreamProxyConfig>,
    /// 是否排除本应用流量的扫描（默认 true）
    #[serde(default = "default_exclude_self_traffic")]
    pub exclude_self_traffic: bool,
    /// Burp 风格全局范围：仅命中 include 且未命中 exclude 的流量会进入历史和后续处理
    #[serde(default)]
    pub scope_include_rules: Vec<ProxyScopeRule>,
    #[serde(default)]
    pub scope_exclude_rules: Vec<ProxyScopeRule>,
    #[serde(default)]
    pub match_replace_rules: Vec<MatchReplaceRule>,
}

fn default_bypass_threshold() -> u32 {
    1
}

fn default_exclude_self_traffic() -> bool {
    true
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            start_port: 8080,
            max_port_attempts: 10,
            mitm_enabled: true,
            max_request_body_size: 2 * 1024 * 1024,
            max_response_body_size: 2 * 1024 * 1024,
            mitm_bypass_fail_threshold: 3,
            upstream_proxy: None,
            exclude_self_traffic: true,
            scope_include_rules: Vec::new(),
            scope_exclude_rules: Vec::new(),
            match_replace_rules: Vec::new(),
        }
    }
}

/// 扫描任务发送器（用于将请求/响应发送到扫描器）
pub type ScanSender = tokio::sync::mpsc::UnboundedSender<ScanTask>;

/// 失败连接记录
#[derive(Debug, Clone)]
pub struct FailedConnection {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub error: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 扫描任务
#[derive(Debug, Clone)]
pub enum ScanTask {
    Request(RequestContext),
    Response(ResponseContext),
    ReloadPlugin(String),
    RemovePlugin(String),                            // 移除/禁用插件
    FailedConnection(FailedConnection),              // TLS 握手失败的连接
    WebSocketConnection(WebSocketConnectionContext), // WebSocket 连接建立
    WebSocketMessage(WebSocketMessageContext),       // WebSocket 消息
}

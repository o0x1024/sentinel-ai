//! 代理核心模块
//!
//! 基于 Hudsucker 实现 HTTP/HTTPS 拦截代理，支持：
//! - 端口自动递增（4201 → 4202 → ...）
//! - HTTPS MITM（默认启用）
//! - 请求/响应 tee（异步扫描队列）
//! - 忽略上游证书验证（用于抓取证书异常的站点）

use crate::{PassiveError, ProxyStats, RequestContext, ResponseContext, Result};
use brotli::Decompressor;
use flate2::read::GzDecoder;
use http_body_util::{BodyExt, Full};
use hudsucker::{
    hyper::{self, Request, Response},
    tokio_tungstenite::tungstenite::Message,
    Body, HttpContext, HttpHandler, Proxy, RequestOrResponse, WebSocketContext, WebSocketHandler,
};
use hyper::body::{Bytes, Frame};
use rustls::client::danger::{ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{io::Read, net::SocketAddr, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tower::{Service, ServiceBuilder};
use tracing::{debug, error, info, warn};

/// 忽略证书验证的 ServerCertVerifier
/// 用于抓取证书异常的站点（如自签名、过期、版本不支持等）
#[derive(Debug)]
struct InsecureServerCertVerifier;

impl ServerCertVerifier for InsecureServerCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        // 忽略所有证书验证错误
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

/// 代理流封装，统一 HTTP 和 HTTPS 连接
pub enum ProxyStream {
    Http(tokio::net::TcpStream),
    Https(tokio_rustls::client::TlsStream<tokio::net::TcpStream>),
}

impl tokio::io::AsyncRead for ProxyStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ProxyStream::Http(s) => Pin::new(s).poll_read(cx, buf),
            ProxyStream::Https(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl tokio::io::AsyncWrite for ProxyStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            ProxyStream::Http(s) => Pin::new(s).poll_write(cx, buf),
            ProxyStream::Https(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ProxyStream::Http(s) => Pin::new(s).poll_flush(cx),
            ProxyStream::Https(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ProxyStream::Http(s) => Pin::new(s).poll_shutdown(cx),
            ProxyStream::Https(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

impl hyper::rt::Read for ProxyStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<std::io::Result<()>> {
        let n = unsafe {
            let mut tbuf = tokio::io::ReadBuf::uninit(buf.as_mut());
            match tokio::io::AsyncRead::poll_read(self, cx, &mut tbuf) {
                Poll::Ready(Ok(())) => tbuf.filled().len(),
                other => return other,
            }
        };

        unsafe {
            buf.advance(n);
        }
        Poll::Ready(Ok(()))
    }
}

impl hyper::rt::Write for ProxyStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::result::Result<usize, std::io::Error>> {
        tokio::io::AsyncWrite::poll_write(self, cx, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        tokio::io::AsyncWrite::poll_flush(self, cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        tokio::io::AsyncWrite::poll_shutdown(self, cx)
    }
}

impl hyper_util::client::legacy::connect::Connection for ProxyStream {
    fn connected(&self) -> hyper_util::client::legacy::connect::Connected {
        hyper_util::client::legacy::connect::Connected::new()
    }
}

/// 自定义 Proxy Connector，用于处理 upstream proxy 连接
/// 替代 hyper-proxy2，提供更稳定的 CONNECT 隧道处理
#[derive(Clone)]
pub struct CustomProxyConnector {
    proxy_host: String,
    proxy_port: u16,
    tls_connector: tokio_rustls::TlsConnector,
}

impl CustomProxyConnector {
    pub fn new(host: String, port: u16, tls_config: Arc<rustls::ClientConfig>) -> Self {
        Self {
            proxy_host: host,
            proxy_port: port,
            tls_connector: tokio_rustls::TlsConnector::from(tls_config),
        }
    }
}

impl Service<hyper::Uri> for CustomProxyConnector {
    type Response = ProxyStream;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future =
        Pin<Box<dyn Future<Output = std::result::Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, dst: hyper::Uri) -> Self::Future {
        let proxy_host = self.proxy_host.clone();
        let proxy_port = self.proxy_port;
        let tls_connector = self.tls_connector.clone();

        Box::pin(async move {
            debug!(
                "CustomProxyConnector: connecting to proxy {}:{}",
                proxy_host, proxy_port
            );

            // 1. 连接到 Upstream Proxy
            let mut stream = tokio::net::TcpStream::connect((proxy_host.as_str(), proxy_port))
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            // 2. 决定是否使用 CONNECT
            let host = dst.host().unwrap_or("").to_string();
            let port = dst
                .port_u16()
                .unwrap_or(if dst.scheme_str() == Some("https") {
                    443
                } else {
                    80
                });

            let is_https = dst.scheme_str() == Some("https") || port == 443;

            if is_https {
                debug!("CustomProxyConnector: creating tunnel to {}:{}", host, port);
                let connect_req = format!(
                    "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\nProxy-Connection: Keep-Alive\r\n\r\n",
                    host, port, host, port
                );

                stream
                    .write_all(connect_req.as_bytes())
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                // 逐字节读取响应头，避免多读
                let mut header = Vec::new();
                loop {
                    let b = stream
                        .read_u8()
                        .await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                    header.push(b);

                    if header.len() > 4096 {
                        return Err("Proxy response too large".into());
                    }

                    if header.ends_with(b"\r\n\r\n") {
                        let response = String::from_utf8_lossy(&header);
                        if !response.contains(" 200 ") {
                            return Err(format!("Proxy connect failed: {}", response).into());
                        }
                        debug!("CustomProxyConnector: tunnel established");
                        break;
                    }
                }

                // 3. 建立 TLS 连接
                let domain = ServerName::try_from(host.as_str())
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                let tls_stream = tls_connector
                    .connect(domain.to_owned(), stream)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                Ok(ProxyStream::Https(tls_stream))
            } else {
                Ok(ProxyStream::Http(stream))
            }
        })
    }
}

/// 创建忽略证书验证的 rustls ClientConfig
/// 支持弱加密套件和旧版本 TLS，以便抓取证书异常的站点
/// 创建忽略证书验证的 rustls 配置（不含 ALPN，用于 HttpsConnectorBuilder）
fn create_insecure_rustls_config() -> rustls::ClientConfig {
    // 注意：不要在这里设置 ALPN 协议
    // HttpsConnectorBuilder 的 enable_http1/enable_http2 会自动设置
    // 如果这里预设了 ALPN，会导致 panic: "ALPN protocols should not be pre-defined"
    rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(InsecureServerCertVerifier))
        .with_no_client_auth()
}

/// 创建忽略证书验证的 rustls 配置（含 ALPN，用于 tokio-rustls TlsConnector）
fn create_insecure_rustls_config_with_alpn() -> rustls::ClientConfig {
    let mut config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(InsecureServerCertVerifier))
        .with_no_client_auth();

    // 配置 ALPN 协议（用于 tokio-rustls 的直接连接）
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    config
}

/// 创建带 upstream proxy 的 HTTPS connector
fn create_upstream_proxy_connector(
    upstream_config: &UpstreamProxyConfig,
) -> Result<CustomProxyConnector> {
    info!(
        "Creating upstream proxy connector: host={}, port={}, auth_type={}",
        upstream_config.proxy_host, upstream_config.proxy_port, upstream_config.auth_type
    );

    // 使用带 ALPN 的配置，因为 CustomProxyConnector 使用 tokio-rustls
    let rustls_config = create_insecure_rustls_config_with_alpn();
    let proxy_connector = CustomProxyConnector::new(
        upstream_config.proxy_host.clone(),
        upstream_config.proxy_port,
        Arc::new(rustls_config),
    );

    // TODO: Basic 认证支持将在后续版本实现
    if upstream_config.auth_type == "Basic" {
        warn!("Basic authentication for upstream proxy is not yet implemented in CustomProxyConnector");
    }

    info!("Upstream proxy connector created successfully");
    Ok(proxy_connector)
}

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
    /// 是否启用请求拦截
    pub enabled: Arc<RwLock<bool>>,
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
}

fn default_bypass_threshold() -> u32 {
    1
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            start_port: 4201,
            max_port_attempts: 10,
            mitm_enabled: true,
            max_request_body_size: 2 * 1024 * 1024,
            max_response_body_size: 2 * 1024 * 1024,
            mitm_bypass_fail_threshold: 3,
            upstream_proxy: None,
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
    FailedConnection(FailedConnection), // TLS 握手失败的连接
    WebSocketConnection(WebSocketConnectionContext), // WebSocket 连接建立
    WebSocketMessage(WebSocketMessageContext), // WebSocket 消息
}

/// 代理处理器（实现 Hudsucker HttpHandler）
///
/// Note: hudsucker calls `self.clone().proxy(req)` for each request-response pair,
/// so each pair gets its own handler clone. We must ensure `current_request_id` is **NOT shared**
/// across clones, otherwise concurrent requests will overwrite each other and cause request/response
/// mismatch in history.
pub struct PassiveProxyHandler {
    config: ProxyConfig,
    stats: Arc<RwLock<ProxyStats>>,
    scan_tx: Option<ScanSender>,
    /// 请求ID映射（用于关联请求和响应）
    /// 使用请求ID作为键来关联请求和响应
    request_map: Arc<RwLock<std::collections::HashMap<String, RequestContext>>>,
    /// 需要绕过 MITM 的域名集合（目前未使用，因为已配置忽略证书错误）
    #[allow(dead_code)]
    bypass_hosts: Arc<RwLock<HashSet<String>>>,
    /// 域名失败计数（目前未使用，因为不再自动绕过MITM）
    #[allow(dead_code)]
    fail_counts: Arc<RwLock<HashMap<String, u32>>>,
    /// 连接键到 CONNECT host 的映射
    conn_to_host: Arc<RwLock<HashMap<String, String>>>,
    /// 连接键到 WebSocket 连接 ID 的映射（用于消息关联）
    conn_to_ws_id: Arc<RwLock<HashMap<String, String>>>,
    /// WebSocket 消息计数器（用于判断方向）
    ws_message_counters: Arc<RwLock<HashMap<String, usize>>>,
    /// 拦截状态
    intercept_state: Option<InterceptState>,
    /// 当前请求 ID（用于匹配 handle_request/handle_response）
    /// 注意：必须是“每个 clone 独立”的状态，不能用 Arc 共享。
    current_request_id: std::sync::Mutex<Option<String>>,
}

impl Clone for PassiveProxyHandler {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            stats: self.stats.clone(),
            scan_tx: self.scan_tx.clone(),
            request_map: self.request_map.clone(),
            bypass_hosts: self.bypass_hosts.clone(),
            fail_counts: self.fail_counts.clone(),
            conn_to_host: self.conn_to_host.clone(),
            conn_to_ws_id: self.conn_to_ws_id.clone(),
            ws_message_counters: self.ws_message_counters.clone(),
            intercept_state: self.intercept_state.clone(),
            // 每个 clone 新建一份独立的 request_id 槽位，避免并发覆盖
            current_request_id: std::sync::Mutex::new(None),
        }
    }
}

impl PassiveProxyHandler {
    pub fn new(config: ProxyConfig, scan_tx: Option<ScanSender>) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(ProxyStats::default())),
            scan_tx,
            request_map: Arc::new(RwLock::new(std::collections::HashMap::new())),
            bypass_hosts: Arc::new(RwLock::new(HashSet::new())),
            fail_counts: Arc::new(RwLock::new(HashMap::new())),
            conn_to_host: Arc::new(RwLock::new(HashMap::new())),
            conn_to_ws_id: Arc::new(RwLock::new(HashMap::new())),
            ws_message_counters: Arc::new(RwLock::new(HashMap::new())),
            intercept_state: None,
            current_request_id: std::sync::Mutex::new(None),
        }
    }

    /// Create a new handler with intercept support
    pub fn with_intercept(
        config: ProxyConfig,
        scan_tx: Option<ScanSender>,
        intercept_state: InterceptState,
    ) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(ProxyStats::default())),
            scan_tx,
            request_map: Arc::new(RwLock::new(std::collections::HashMap::new())),
            bypass_hosts: Arc::new(RwLock::new(HashSet::new())),
            fail_counts: Arc::new(RwLock::new(HashMap::new())),
            conn_to_host: Arc::new(RwLock::new(HashMap::new())),
            conn_to_ws_id: Arc::new(RwLock::new(HashMap::new())),
            ws_message_counters: Arc::new(RwLock::new(HashMap::new())),
            intercept_state: Some(intercept_state),
            current_request_id: std::sync::Mutex::new(None),
        }
    }

    pub fn stats(&self) -> Arc<RwLock<ProxyStats>> {
        self.stats.clone()
    }

    /// 生成连接关联键（基于连接信息）
    fn generate_connection_key(ctx: &HttpContext) -> String {
        // 尝试直接使用 client_addr
        ctx.client_addr.to_string()
    }

    /// 生成 WebSocket 连接关联键（基于连接信息）
    fn generate_ws_connection_key(ctx: &WebSocketContext) -> String {
        let debug_str = format!("{:?}", ctx);
        // 解析 Debug 字符串提取客户端地址
        // 格式: ClientToServer { src: 127.0.0.1:51838, dst: ... }
        // 或 ServerToClient { src: ..., dst: 127.0.0.1:51838 }

        if debug_str.starts_with("ClientToServer") {
            // 提取 src: 后面的内容
            if let Some(start) = debug_str.find("src: ") {
                let rest = &debug_str[start + 5..];
                if let Some(end) = rest.find(',') {
                    return rest[..end].trim().to_string();
                }
            }
        } else if debug_str.starts_with("ServerToClient") {
            // 提取 dst: 后面的内容
            if let Some(start) = debug_str.find("dst: ") {
                let rest = &debug_str[start + 5..];
                // 可能是结尾的 } 或者逗号
                if let Some(end) = rest.find(|c| c == ',' || c == '}') {
                    return rest[..end].trim().to_string();
                }
            }
        }

        // 如果解析失败，回退到原始字符串（虽然这肯定会失败）
        warn!(
            "Failed to parse WebSocketContext debug string: {}",
            debug_str
        );
        debug_str
    }

    /// 设置当前请求 ID（用于响应匹配）
    fn set_current_request_id(&self, request_id: String) {
        if let Ok(mut guard) = self.current_request_id.lock() {
            *guard = Some(request_id)
        }
    }

    /// 获取并清除当前请求 ID
    fn take_current_request_id(&self) -> Option<String> {
        self.current_request_id
            .lock()
            .ok()
            .and_then(|mut g| g.take())
    }

    /// 从 CONNECT 请求中提取 host（去掉端口）
    fn parse_connect_host(req: &Request<Body>) -> Option<String> {
        // CONNECT 请求的 URI 通常为 authority 形式，如 host:443
        let authority = req
            .uri()
            .authority()
            .map(|a| a.as_str().to_string())
            .or_else(|| {
                req.headers()
                    .get("host")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string())
            });
        authority.map(|auth| auth.split(':').next().unwrap_or(&auth).to_string())
    }

    /// Check if a request should be intercepted based on filter rules
    /// Returns true if the request should be intercepted, false if it should be skipped
    async fn should_intercept_request(
        intercept_state: &InterceptState,
        url: &str,
        method: &str,
        _headers: &HashMap<String, String>,
    ) -> bool {
        let rules = intercept_state.request_filter_rules.read().await;
        if rules.is_empty() {
            return true; // No rules, intercept all
        }

        // Extract domain from URL
        let domain = url
            .split("://")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .and_then(|s| s.split(':').next())
            .unwrap_or("");

        // Extract file extension from URL
        let path = url.split('?').next().unwrap_or(url);
        let file_ext = path.rsplit('.').next().unwrap_or("");

        // Evaluate rules (AND logic by default, rules with "does_not_match" exclude requests)
        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            let value_to_match = match rule.match_type.as_str() {
                "domain_name" => domain,
                "url" => url,
                "http_method" => method,
                "file_extension" => file_ext,
                _ => continue,
            };

            let matches = if rule.condition.is_empty() {
                false
            } else {
                match regex::Regex::new(&rule.condition) {
                    Ok(re) => re.is_match(value_to_match),
                    Err(_) => {
                        // Fallback to simple contains check
                        value_to_match
                            .to_lowercase()
                            .contains(&rule.condition.to_lowercase())
                    }
                }
            };

            // "does_not_match" means: if condition matches, skip interception
            if rule.relationship == "does_not_match" && matches {
                debug!(
                    "Request {} skipped by filter rule: {} does_not_match {}",
                    url, rule.match_type, rule.condition
                );
                return false;
            }

            // "matches" means: if condition doesn't match, skip interception
            if rule.relationship == "matches" && !matches {
                debug!(
                    "Request {} skipped by filter rule: {} matches {}",
                    url, rule.match_type, rule.condition
                );
                return false;
            }
        }

        true
    }

    /// 解析修改后的请求内容并重建 HTTP 请求
    /// 格式: METHOD PATH PROTOCOL\nHeader1: Value1\n...\n\nBODY
    fn parse_and_rebuild_request(
        content: &str,
        original_uri: &hyper::Uri,
    ) -> Result<Request<Body>> {
        let mut lines = content.lines();

        // 解析请求行: METHOD PATH PROTOCOL
        let request_line = lines
            .next()
            .ok_or_else(|| PassiveError::Proxy("Empty request content".to_string()))?;
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(PassiveError::Proxy(format!(
                "Invalid request line: {}",
                request_line
            )));
        }

        let method = parts[0];
        let path = parts[1];
        // parts[2] 是协议版本，我们忽略它

        // 解析请求头
        let mut headers = HashMap::new();
        let mut body_start = false;
        let mut body_lines = Vec::new();

        for line in lines {
            if body_start {
                body_lines.push(line);
            } else if line.is_empty() {
                body_start = true;
            } else if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                headers.insert(key, value);
            }
        }

        // 合并 body
        let body_content = body_lines.join("\n");

        // 构建新的 URI
        let new_uri = if path.starts_with("http://") || path.starts_with("https://") {
            path.parse::<hyper::Uri>()
                .unwrap_or_else(|_| original_uri.clone())
        } else {
            // 使用原始 URI 的 scheme 和 authority，只替换 path
            let scheme = original_uri.scheme_str().unwrap_or("http");
            let authority = original_uri.authority().map(|a| a.as_str()).unwrap_or("");
            format!("{}://{}{}", scheme, authority, path)
                .parse::<hyper::Uri>()
                .unwrap_or_else(|_| original_uri.clone())
        };

        // 构建请求
        let mut builder = Request::builder().method(method).uri(new_uri);

        // 添加头部
        for (key, value) in headers {
            builder = builder.header(&key, &value);
        }

        // 构建带 body 的请求
        let body = if body_content.is_empty() {
            Body::empty()
        } else {
            Body::from(body_content)
        };

        builder
            .body(body)
            .map_err(|e| PassiveError::Proxy(format!("Failed to build request: {}", e)))
    }

    /// 解析修改后的响应内容并重建 HTTP 响应
    /// 格式: HTTP/1.1 STATUS\nHeader1: Value1\n...\n\nBODY
    fn parse_and_rebuild_response(content: &str) -> Result<Response<Body>> {
        let mut lines = content.lines();

        // 解析状态行: HTTP/1.1 STATUS
        let status_line = lines
            .next()
            .ok_or_else(|| PassiveError::Proxy("Empty response content".to_string()))?;
        let parts: Vec<&str> = status_line.split_whitespace().collect();

        // 提取状态码
        let status_code = if parts.len() >= 2 {
            parts[1].parse::<u16>().unwrap_or(200)
        } else {
            200
        };

        // 解析响应头
        let mut headers = HashMap::new();
        let mut body_start = false;
        let mut body_lines = Vec::new();

        for line in lines {
            if body_start {
                body_lines.push(line);
            } else if line.is_empty() {
                body_start = true;
            } else if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                headers.insert(key, value);
            }
        }

        // 合并 body
        let body_content = body_lines.join("\n");

        // 构建响应
        let mut builder = Response::builder().status(status_code);

        // 添加头部
        for (key, value) in headers {
            builder = builder.header(&key, &value);
        }

        // 构建带 body 的响应
        let body = if body_content.is_empty() {
            Body::empty()
        } else {
            Body::from(body_content)
        };

        builder
            .body(body)
            .map_err(|e| PassiveError::Proxy(format!("Failed to build response: {}", e)))
    }

    /// 标记某域名失败一次；到达阈值后加入 bypass 列表
    /// 目前未使用：已配置忽略所有证书错误，不再自动绕过MITM
    #[allow(dead_code)]
    async fn note_fail_and_maybe_bypass(&self, host: &str) {
        let mut counts = self.fail_counts.write().await;
        let c = counts.entry(host.to_string()).or_insert(0);
        *c += 1;
        let threshold = self.config.mitm_bypass_fail_threshold;
        if *c >= threshold {
            let mut bypass = self.bypass_hosts.write().await;
            if bypass.insert(host.to_string()) {
                warn!(
                    "MITM disabled for host {} after {} failures; future CONNECT will be tunneled",
                    host, c
                );
            }
        } else {
            warn!(
                "TLS/MITM failure counted for host {} ({} / {})",
                host, c, threshold
            );
        }
    }

    /// 解压响应体（支持 gzip 和 brotli）
    fn decompress_body(body_bytes: &[u8], encoding: Option<&str>) -> Vec<u8> {
        let encoding = match encoding {
            Some(e) => e.to_lowercase(),
            None => return body_bytes.to_vec(),
        };

        match encoding.as_str() {
            "gzip" => {
                match GzDecoder::new(body_bytes)
                    .bytes()
                    .collect::<std::io::Result<Vec<u8>>>()
                {
                    Ok(decompressed) => {
                        debug!(
                            "Decompressed gzip body: {} -> {} bytes",
                            body_bytes.len(),
                            decompressed.len()
                        );
                        decompressed
                    }
                    Err(e) => {
                        warn!("Failed to decompress gzip body: {}, returning original", e);
                        body_bytes.to_vec()
                    }
                }
            }
            "br" => {
                let mut decompressor = Decompressor::new(body_bytes, 4096);
                let mut decompressed = Vec::new();
                match decompressor.read_to_end(&mut decompressed) {
                    Ok(_) => {
                        debug!(
                            "Decompressed brotli body: {} -> {} bytes",
                            body_bytes.len(),
                            decompressed.len()
                        );
                        decompressed
                    }
                    Err(e) => {
                        warn!(
                            "Failed to decompress brotli body: {}, returning original",
                            e
                        );
                        body_bytes.to_vec()
                    }
                }
            }
            "deflate" => {
                // deflate 也是 zlib 格式
                match flate2::read::DeflateDecoder::new(body_bytes)
                    .bytes()
                    .collect::<std::io::Result<Vec<u8>>>()
                {
                    Ok(decompressed) => {
                        debug!(
                            "Decompressed deflate body: {} -> {} bytes",
                            body_bytes.len(),
                            decompressed.len()
                        );
                        decompressed
                    }
                    Err(e) => {
                        warn!(
                            "Failed to decompress deflate body: {}, returning original",
                            e
                        );
                        body_bytes.to_vec()
                    }
                }
            }
            _ => {
                // 不支持的编码或无编码，返回原始数据
                body_bytes.to_vec()
            }
        }
    }

    /// 从 Hyper Request 构建 RequestContext（读取 body）
    async fn build_request_context(
        &self,
        _ctx: &HttpContext,
        req: Request<Body>,
    ) -> Result<(RequestContext, Request<Body>)> {
        let id = uuid::Uuid::new_v4().to_string();

        // 判断是否是 CONNECT 请求
        let is_connect = req.method() == hyper::Method::CONNECT;

        // 提取 URL
        let uri = req.uri().clone();
        let scheme = if is_connect {
            "https"
        } else {
            uri.scheme_str().unwrap_or("http")
        };

        // 从 Host header 或 URI 中获取 authority
        let authority = req
            .headers()
            .get("host")
            .and_then(|h| h.to_str().ok())
            .or_else(|| uri.authority().map(|a| a.as_str()))
            .unwrap_or("unknown")
            .to_string();

        let path = uri.path();
        let query = uri.query().unwrap_or("");

        let url = if query.is_empty() {
            format!("{}://{}{}", scheme, authority, path)
        } else {
            format!("{}://{}{}?{}", scheme, authority, path, query)
        };

        // 提取方法
        let method = req.method().to_string();

        // 提取请求头
        let mut headers = std::collections::HashMap::new();
        for (name, value) in req.headers().iter() {
            if let Ok(v) = value.to_str() {
                headers.insert(name.to_string(), v.to_string());
            }
        }

        // 提取 Content-Type
        let content_type = headers.get("content-type").cloned();

        // 解析查询参数
        let query_params: std::collections::HashMap<String, String> =
            url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();

        // 读取 body 并创建新的 body 用于转发
        let (parts, body) = req.into_parts();

        // 收集 body 数据
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                warn!("Failed to read request body for {}: {}", url, e);
                Bytes::new()
            }
        };

        // 检查大小限制
        let body_vec = if body_bytes.len() > self.config.max_request_body_size {
            warn!(
                "Request body too large ({} bytes), truncating to {} bytes for {}",
                body_bytes.len(),
                self.config.max_request_body_size,
                url
            );
            body_bytes[..self.config.max_request_body_size].to_vec()
        } else {
            body_bytes.to_vec()
        };

        debug!(
            "Captured request body: {} bytes for {} {}",
            body_vec.len(),
            method,
            url
        );

        // 创建新的请求用于转发（包含原始 body）
        // hudsucker::Body 实现了 From<Full<Bytes>>
        let new_body = Body::from(Full::new(body_bytes.clone()));
        let new_req = Request::from_parts(parts, new_body);

        let req_ctx = RequestContext {
            id,
            method,
            url,
            headers,
            body: body_vec,
            content_type,
            query_params,
            is_https: scheme == "https",
            timestamp: chrono::Utc::now(),
            was_edited: false,
            edited_method: None,
            edited_url: None,
            edited_headers: None,
            edited_body: None,
        };

        Ok((req_ctx, new_req))
    }

    /// 从 Hyper Response 构建 ResponseContext（读取 body）
    async fn build_response_context(
        &self,
        request_id: String,
        res: Response<Body>,
    ) -> Result<(ResponseContext, Response<Body>)> {
        // 提取状态码
        let status = res.status().as_u16();

        // 提取响应头
        let mut headers = std::collections::HashMap::new();
        for (name, value) in res.headers().iter() {
            if let Ok(v) = value.to_str() {
                headers.insert(name.to_string(), v.to_string());
            }
        }

        // 提取 Content-Type
        let content_type = headers.get("content-type").cloned();

        // 提取 Content-Encoding（用于解压）
        let content_encoding = headers
            .get("content-encoding")
            .or_else(|| headers.get("Content-Encoding"))
            .map(|s| s.as_str());

        // 读取 body 并创建新的 body 用于转发
        let (parts, body) = res.into_parts();

        // 收集 body 数据
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                warn!(
                    "Failed to read response body for request {}: {}",
                    request_id, e
                );
                Bytes::new()
            }
        };

        // 检查大小限制（对压缩后的数据）
        let compressed_body_vec = if body_bytes.len() > self.config.max_response_body_size {
            warn!(
                "Response body too large ({} bytes), truncating to {} bytes for request {}",
                body_bytes.len(),
                self.config.max_response_body_size,
                request_id
            );
            body_bytes[..self.config.max_response_body_size].to_vec()
        } else {
            body_bytes.to_vec()
        };

        // 解压响应体（如果有压缩）
        let decompressed_body = if content_encoding.is_some() {
            debug!(
                "Detected content encoding: {:?}, attempting decompression for request {}",
                content_encoding, request_id
            );
            Self::decompress_body(&compressed_body_vec, content_encoding)
        } else {
            compressed_body_vec.clone()
        };

        // 再次检查解压后的大小限制
        let body_vec = if decompressed_body.len() > self.config.max_response_body_size {
            warn!(
                "Decompressed response body too large ({} bytes), truncating to {} bytes for request {}",
                decompressed_body.len(),
                self.config.max_response_body_size,
                request_id
            );
            decompressed_body[..self.config.max_response_body_size].to_vec()
        } else {
            decompressed_body
        };

        debug!(
            "Captured response body: compressed={} bytes, decompressed={} bytes, status={} for request {}",
            compressed_body_vec.len(),
            body_vec.len(),
            status,
            request_id
        );

        // 创建新的响应用于转发（使用压缩后的原始数据，保持原样转发）
        // hudsucker::Body 实现了 From<Full<Bytes>>
        let new_body = Body::from(Full::new(body_bytes.clone()));
        let new_res = Response::from_parts(parts, new_body);

        // 但保存到数据库和扫描器的是解压后的数据
        let resp_ctx = ResponseContext {
            request_id,
            status,
            headers,
            body: body_vec, // 保存解压后的数据
            content_type,
            timestamp: chrono::Utc::now(),
            was_edited: false,
            edited_status: None,
            edited_headers: None,
            edited_body: None,
        };

        Ok((resp_ctx, new_res))
    }

    /// 检查响应是否为流式类型（SSE 或分块传输）
    ///
    /// 支持以下流式响应类型：
    /// - text/event-stream (SSE, Server-Sent Events)
    /// - application/x-ndjson (Newline Delimited JSON, 常用于 LLM 流式输出)
    /// - Transfer-Encoding: chunked (分块传输，当 Content-Type 为上述类型时)
    fn is_streaming_response(headers: &HashMap<String, String>) -> bool {
        let content_type = headers
            .get("content-type")
            .or_else(|| headers.get("Content-Type"))
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        // 检查是否为流式 Content-Type
        let is_stream_content_type = content_type.contains("text/event-stream")
            || content_type.contains("application/x-ndjson")
            || content_type.contains("application/stream+json");

        if is_stream_content_type {
            debug!("Detected streaming response: content-type={}", content_type);
            return true;
        }

        false
    }

    /// 构建流式响应上下文（用于 SSE 等）
    ///
    /// 与 build_response_context 不同，此方法：
    /// 1. 不缓冲整个响应体
    /// 2. 返回一个可以边转发边收集的流式响应
    /// 3. 异步收集数据用于后续扫描
    async fn build_streaming_response(
        &self,
        request_id: String,
        res: Response<Body>,
        scan_tx: Option<ScanSender>,
    ) -> Result<(Response<Body>, tokio::task::JoinHandle<Vec<u8>>)> {
        // 提取状态码
        let status = res.status().as_u16();

        // 提取响应头
        let mut headers = std::collections::HashMap::new();
        for (name, value) in res.headers().iter() {
            if let Ok(v) = value.to_str() {
                headers.insert(name.to_string(), v.to_string());
            }
        }

        // 提取 Content-Type
        let content_type = headers.get("content-type").cloned();
        let headers_clone = headers.clone();
        let request_id_clone = request_id.clone();

        // 分解响应
        let (parts, body) = res.into_parts();

        // 创建用于收集数据的通道
        let (collector_tx, mut collector_rx) = mpsc::channel::<Bytes>(100);

        // 创建 Tee 流：一边转发给客户端，一边收集数据
        let tee_body = TeeBodyStream::new(body, collector_tx);

        // 使用 BodyExt::boxed() 将 TeeBodyStream 转换为 BoxBody
        // 然后通过 Body::from 转换为 hudsucker::Body
        let boxed_body = http_body_util::BodyExt::boxed(tee_body);
        let new_body = Body::from(boxed_body);
        let new_res = Response::from_parts(parts, new_body);

        // 异步任务：收集流式数据并在完成后发送到扫描器
        let max_body_size = self.config.max_response_body_size;
        let collector_handle = tokio::spawn(async move {
            let mut collected_data = Vec::new();
            let mut truncated = false;

            while let Some(chunk) = collector_rx.recv().await {
                if !truncated {
                    if collected_data.len() + chunk.len() > max_body_size {
                        // 达到大小限制，只收集到限制为止
                        let remaining = max_body_size - collected_data.len();
                        collected_data.extend_from_slice(&chunk[..remaining]);
                        truncated = true;
                        warn!(
                            "Streaming response body truncated at {} bytes for request {}",
                            max_body_size, request_id_clone
                        );
                    } else {
                        collected_data.extend_from_slice(&chunk);
                    }
                }
            }

            debug!(
                "Streaming response collected: {} bytes for request {}",
                collected_data.len(),
                request_id_clone
            );

            // 构建 ResponseContext 并发送到扫描器
            if let Some(tx) = scan_tx {
                let resp_ctx = ResponseContext {
                    request_id: request_id_clone,
                    status,
                    headers: headers_clone,
                    body: collected_data.clone(),
                    content_type,
                    timestamp: chrono::Utc::now(),
                    was_edited: false,
                    edited_status: None,
                    edited_headers: None,
                    edited_body: None,
                };

                if let Err(e) = tx.send(ScanTask::Response(resp_ctx)) {
                    warn!("Failed to send streaming response to scanner: {}", e);
                }
            }

            collected_data
        });

        Ok((new_res, collector_handle))
    }
}

/// Tee Body Stream: 同时转发和收集数据的流
///
/// 用于 SSE 等流式响应，实现：
/// 1. 将每个 chunk 转发给客户端（通过 http_body::Body trait）
/// 2. 同时将每个 chunk 发送到收集器（用于后续扫描）
struct TeeBodyStream {
    inner: Body,
    collector_tx: mpsc::Sender<Bytes>,
}

impl TeeBodyStream {
    fn new(body: Body, collector_tx: mpsc::Sender<Bytes>) -> Self {
        Self {
            inner: body,
            collector_tx,
        }
    }
}

impl http_body::Body for TeeBodyStream {
    type Data = Bytes;
    type Error = hudsucker::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<std::result::Result<Frame<Self::Data>, Self::Error>>> {
        // 从内部 body 获取下一个 frame
        let inner = Pin::new(&mut self.inner);
        match http_body::Body::poll_frame(inner, cx) {
            Poll::Ready(Some(Ok(frame))) => {
                // 如果是数据帧，复制一份发送给收集器
                if let Some(data) = frame.data_ref() {
                    let data_clone = data.clone();
                    let tx = self.collector_tx.clone();
                    // 使用 try_send 避免阻塞
                    let _ = tx.try_send(data_clone);
                }
                Poll::Ready(Some(Ok(frame)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }

    fn is_end_stream(&self) -> bool {
        http_body::Body::is_end_stream(&self.inner)
    }

    fn size_hint(&self) -> http_body::SizeHint {
        http_body::Body::size_hint(&self.inner)
    }
}

impl HttpHandler for PassiveProxyHandler {
    fn should_intercept(
        &mut self,
        ctx: &HttpContext,
        req: &Request<Body>,
    ) -> impl std::future::Future<Output = bool> + Send {
        let bypass_hosts = self.bypass_hosts.clone();
        let conn_to_host = self.conn_to_host.clone();
        let conn_key = Self::generate_connection_key(ctx);

        // 提前解析 host（在 async 块外）
        let host_opt = Self::parse_connect_host(&req);
        let is_connect = req.method() == hyper::Method::CONNECT;

        async move {
            // 仅对 CONNECT 生效
            if !is_connect {
                return true;
            }

            if let Some(ref host) = host_opt {
                // 提前记录 host 映射，确保 handle_error 能拿到
                {
                    let mut map = conn_to_host.write().await;
                    map.insert(conn_key.clone(), host.clone());
                }

                let bypass = bypass_hosts.read().await;
                if bypass.contains(host) {
                    debug!("Bypass MITM for host {} (in bypass list)", host);
                    return false;
                }
            }
            true
        }
    }

    async fn handle_request(&mut self, ctx: &HttpContext, req: Request<Body>) -> RequestOrResponse {
        let method = req.method().clone();
        let uri = req.uri().clone();

        // 判断是否是 HTTPS (CONNECT 方法)
        let is_https = method == hyper::Method::CONNECT;

        // 更新统计
        {
            let mut stats = self.stats.write().await;
            if is_https {
                stats.https_requests += 1;
            } else {
                stats.http_requests += 1;
            }
        }

        debug!(
            "Processing request: {} {} (HTTPS: {})",
            method, uri, is_https
        );

        // 记录 host → conn_key 映射以便错误统计
        // 对 CONNECT 请求：从 authority 提取 host
        // 对 MITM 内部请求（非 CONNECT）：从 URI 或 Host header 提取 host
        let conn_key = Self::generate_connection_key(ctx);
        let host_for_mapping = if is_https {
            Self::parse_connect_host(&req)
        } else {
            // MITM 内部请求：从 URI authority 或 Host header 提取
            uri.authority().map(|a| a.host().to_string()).or_else(|| {
                req.headers()
                    .get("host")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.split(':').next().unwrap_or(s).to_string())
            })
        };
        if let Some(host) = host_for_mapping {
            let mut map = self.conn_to_host.write().await;
            map.insert(conn_key, host);
        }

        // 构建上下文并发送到扫描器
        if let Some(tx) = &self.scan_tx {
            match self.build_request_context(ctx, req).await {
                Ok((req_ctx, new_req)) => {
                    let request_id = req_ctx.id.clone();
                    let conn_key = Self::generate_connection_key(ctx);

                    debug!(
                        "Request captured: id={}, method={}, url={}, body_size={}, conn_key={}",
                        request_id,
                        req_ctx.method,
                        req_ctx.url,
                        req_ctx.body.len(),
                        conn_key
                    );

                    // 保存请求上下文供响应关联使用
                    {
                        let mut request_map = self.request_map.write().await;
                        request_map.insert(request_id.clone(), req_ctx.clone());

                        // 限制缓存大小
                        if request_map.len() > 1000 {
                            // 移除最早的一半条目
                            let keys_to_remove: Vec<_> =
                                request_map.keys().take(500).cloned().collect();
                            for key in &keys_to_remove {
                                request_map.remove(key);
                            }
                            debug!("Request map cleaned, current size: {}", request_map.len());
                        }
                    }

                    // 发送到扫描器
                    if let Err(e) = tx.send(ScanTask::Request(req_ctx.clone())) {
                        warn!("Failed to send request to scanner: {}", e);
                    }

                    // 检测 WebSocket 升级请求
                    let is_websocket_upgrade = req_ctx.headers.iter().any(|(k, v)| {
                        k.to_lowercase() == "upgrade" && v.to_lowercase().contains("websocket")
                    });

                    if is_websocket_upgrade {
                        debug!("WebSocket upgrade detected: {}", req_ctx.url);

                        // 从 URL 或 Host header 中提取 host
                        let host = if let Ok(parsed_url) = url::Url::parse(&req_ctx.url) {
                            parsed_url.host_str().unwrap_or("unknown").to_string()
                        } else {
                            req_ctx
                                .headers
                                .get("host")
                                .or_else(|| req_ctx.headers.get("Host"))
                                .cloned()
                                .unwrap_or_else(|| "unknown".to_string())
                        };

                        // 创建 WebSocket 连接上下文
                        let ws_id = uuid::Uuid::new_v4().to_string();
                        let ws_conn = WebSocketConnectionContext {
                            id: ws_id.clone(),
                            url: req_ctx.url.clone(),
                            host,
                            protocol: if req_ctx.is_https {
                                "wss".to_string()
                            } else {
                                "ws".to_string()
                            },
                            request_headers: req_ctx
                                .headers
                                .iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect(),
                            response_headers: None, // 响应头将在 handle_response 中更新
                            opened_at: chrono::Utc::now(),
                        };

                        // 保存连接键到 WebSocket ID 的映射
                        {
                            let mut ws_map = self.conn_to_ws_id.write().await;
                            ws_map.insert(conn_key.clone(), ws_id.clone());
                            debug!(
                                "Saved WebSocket connection mapping: conn_key={}, ws_id={}",
                                conn_key, ws_id
                            );
                        }

                        // 发送 WebSocket 连接到扫描器
                        if let Err(e) = tx.send(ScanTask::WebSocketConnection(ws_conn)) {
                            warn!("Failed to send WebSocket connection to scanner: {}", e);
                        }
                    }

                    // 检查是否启用了拦截模式（跳过 CONNECT 请求）
                    if !is_https {
                        if let Some(intercept_state) = &self.intercept_state {
                            let intercept_enabled = *intercept_state.enabled.read().await;
                            if intercept_enabled {
                                // Check filter rules before intercepting
                                let should_intercept = Self::should_intercept_request(
                                    intercept_state,
                                    &req_ctx.url,
                                    &req_ctx.method,
                                    &req_ctx.headers,
                                )
                                .await;

                                if !should_intercept {
                                    debug!("Request {} skipped by filter rules", req_ctx.url);
                                    self.set_current_request_id(request_id.clone());
                                    return RequestOrResponse::Request(new_req);
                                }

                                if let Some(pending_tx) = &intercept_state.pending_tx {
                                    // 创建 oneshot channel 等待用户操作
                                    let (response_tx, response_rx) =
                                        tokio::sync::oneshot::channel();

                                    // 解析 URL path
                                    let path = uri
                                        .path_and_query()
                                        .map(|pq| pq.to_string())
                                        .unwrap_or_else(|| uri.path().to_string());

                                    let pending_request = PendingInterceptRequest {
                                        id: request_id.clone(),
                                        method: req_ctx.method.clone(),
                                        url: req_ctx.url.clone(),
                                        path,
                                        protocol: if req_ctx.is_https {
                                            "HTTPS".to_string()
                                        } else {
                                            "HTTP/1.1".to_string()
                                        },
                                        headers: req_ctx.headers.clone(),
                                        body: if req_ctx.body.is_empty() {
                                            None
                                        } else {
                                            String::from_utf8(req_ctx.body.clone()).ok()
                                        },
                                        timestamp: chrono::Utc::now().timestamp_millis(),
                                        response_tx,
                                    };

                                    // 发送到待处理队列
                                    if let Err(e) = pending_tx.send(pending_request) {
                                        warn!("Failed to send intercept request: {}", e);
                                        self.set_current_request_id(request_id.clone());
                                        return RequestOrResponse::Request(new_req);
                                    }

                                    info!(
                                        "Request {} intercepted, waiting for user action",
                                        request_id
                                    );

                                    // 等待用户操作（带超时）
                                    match tokio::time::timeout(
                                        std::time::Duration::from_secs(300), // 5 minutes timeout
                                        response_rx,
                                    )
                                    .await
                                    {
                                        Ok(Ok(action)) => {
                                            match action {
                                                InterceptAction::Forward(modified_content) => {
                                                    info!(
                                                        "Request {} forwarded by user",
                                                        request_id
                                                    );
                                                    self.set_current_request_id(request_id.clone());
                                                    if let Some(content) = modified_content {
                                                        // 解析修改后的内容并重建请求
                                                        match Self::parse_and_rebuild_request(
                                                            &content, &uri,
                                                        ) {
                                                            Ok(modified_req) => {
                                                                info!("Request {} modified and forwarded", request_id);

                                                                // 更新 req_ctx 保存修改后的数据
                                                                let mut updated_req_ctx =
                                                                    req_ctx.clone();
                                                                updated_req_ctx.was_edited = true;
                                                                updated_req_ctx.edited_method =
                                                                    Some(
                                                                        modified_req
                                                                            .method()
                                                                            .to_string(),
                                                                    );

                                                                // 构建修改后的 URL
                                                                let edited_uri = modified_req.uri();
                                                                let edited_scheme = edited_uri
                                                                    .scheme_str()
                                                                    .unwrap_or(
                                                                        if req_ctx.is_https {
                                                                            "https"
                                                                        } else {
                                                                            "http"
                                                                        },
                                                                    );
                                                                let edited_authority = modified_req
                                                                    .headers()
                                                                    .get("host")
                                                                    .and_then(|h| h.to_str().ok())
                                                                    .or_else(|| {
                                                                        edited_uri
                                                                            .authority()
                                                                            .map(|a| a.as_str())
                                                                    })
                                                                    .unwrap_or("unknown");
                                                                let edited_path = edited_uri.path();
                                                                let edited_query = edited_uri
                                                                    .query()
                                                                    .unwrap_or("");
                                                                let edited_url =
                                                                    if edited_query.is_empty() {
                                                                        format!(
                                                                            "{}://{}{}",
                                                                            edited_scheme,
                                                                            edited_authority,
                                                                            edited_path
                                                                        )
                                                                    } else {
                                                                        format!(
                                                                            "{}://{}{}?{}",
                                                                            edited_scheme,
                                                                            edited_authority,
                                                                            edited_path,
                                                                            edited_query
                                                                        )
                                                                    };
                                                                updated_req_ctx.edited_url =
                                                                    Some(edited_url);

                                                                // 保存修改后的 headers
                                                                let mut edited_headers =
                                                                    std::collections::HashMap::new(
                                                                    );
                                                                for (name, value) in
                                                                    modified_req.headers().iter()
                                                                {
                                                                    if let Ok(v) = value.to_str() {
                                                                        edited_headers.insert(
                                                                            name.to_string(),
                                                                            v.to_string(),
                                                                        );
                                                                    }
                                                                }
                                                                updated_req_ctx.edited_headers =
                                                                    Some(edited_headers);

                                                                // 更新 request_map
                                                                {
                                                                    let mut request_map = self
                                                                        .request_map
                                                                        .write()
                                                                        .await;
                                                                    request_map.insert(
                                                                        request_id.clone(),
                                                                        updated_req_ctx.clone(),
                                                                    );
                                                                }

                                                                // 发送更新后的 RequestContext 到 scanner
                                                                if let Err(e) =
                                                                    tx.send(ScanTask::Request(
                                                                        updated_req_ctx,
                                                                    ))
                                                                {
                                                                    warn!("Failed to send updated request to scanner: {}", e);
                                                                }

                                                                return RequestOrResponse::Request(
                                                                    modified_req,
                                                                );
                                                            }
                                                            Err(e) => {
                                                                warn!("Failed to parse modified request: {}, forwarding original", e);
                                                                return RequestOrResponse::Request(
                                                                    new_req,
                                                                );
                                                            }
                                                        }
                                                    }
                                                    return RequestOrResponse::Request(new_req);
                                                }
                                                InterceptAction::Drop => {
                                                    info!("Request {} dropped by user", request_id);
                                                    // Return an empty response (connection reset)
                                                    return RequestOrResponse::Response(
                                                        Response::builder()
                                                            .status(444) // Connection Closed Without Response
                                                            .body(Body::empty())
                                                            .unwrap_or_else(|_| {
                                                                Response::new(Body::empty())
                                                            }),
                                                    );
                                                }
                                            }
                                        }
                                        Ok(Err(_)) => {
                                            warn!("Intercept channel closed for request {}, forwarding", request_id);
                                            self.set_current_request_id(request_id.clone());
                                            return RequestOrResponse::Request(new_req);
                                        }
                                        Err(_) => {
                                            warn!(
                                                "Intercept timeout for request {}, forwarding",
                                                request_id
                                            );
                                            self.set_current_request_id(request_id.clone());
                                            return RequestOrResponse::Request(new_req);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 设置当前请求 ID，供 handle_response 使用
                    self.set_current_request_id(request_id);

                    // 返回修改后的请求
                    RequestOrResponse::Request(new_req)
                }
                Err(e) => {
                    error!("Failed to build request context: {}", e);
                    let mut stats = self.stats.write().await;
                    stats.errors += 1;
                    // 由于 req 已经被 move，我们需要创建一个空请求
                    // 但实际上这个分支不应该发生，因为 build_request_context 成功返回了 new_req
                    // 这里返回一个默认的请求，实际应用中可能需要更好的错误处理
                    RequestOrResponse::Request(Request::new(Body::from("")))
                }
            }
        } else {
            // 无扫描器，直接转发
            RequestOrResponse::Request(req)
        }
    }

    async fn handle_response(&mut self, ctx: &HttpContext, res: Response<Body>) -> Response<Body> {
        let status = res.status();
        let conn_key = Self::generate_connection_key(ctx);

        debug!(
            "Processing response: status={}, conn_key={}",
            status, conn_key
        );

        // 提取响应头用于流式检测
        let mut response_headers = std::collections::HashMap::new();
        for (name, value) in res.headers().iter() {
            if let Ok(v) = value.to_str() {
                response_headers.insert(name.to_string(), v.to_string());
            }
        }

        // 检测是否为流式响应（SSE, NDJSON 等）
        let is_streaming = Self::is_streaming_response(&response_headers);

        // 构建上下文并发送到扫描器
        if let Some(tx) = &self.scan_tx {
            // 从当前 handler 实例获取请求 ID（由 handle_request 设置）
            // 这比使用 FIFO 队列更可靠，因为每个请求-响应对使用独立的 handler 克隆
            let request_id_opt = self.take_current_request_id();

            if let Some(request_id) = request_id_opt {
                // 获取请求上下文
                let req_ctx_opt = {
                    let request_map = self.request_map.read().await;
                    request_map.get(&request_id).cloned()
                };

                if let Some(_req_ctx) = req_ctx_opt {
                    // 流式响应：使用 Tee 机制同时转发和收集
                    if is_streaming {
                        info!(
                            "Streaming response detected: request_id={}, status={}, conn_key={}",
                            request_id, status, conn_key
                        );

                        match self
                            .build_streaming_response(request_id.clone(), res, Some(tx.clone()))
                            .await
                        {
                            Ok((streaming_res, _collector_handle)) => {
                                // 清理请求映射
                                {
                                    let mut request_map = self.request_map.write().await;
                                    request_map.remove(&request_id);
                                }

                                debug!(
                                    "Streaming response forwarded: request_id={}, conn_key={}",
                                    request_id, conn_key
                                );

                                // 直接返回流式响应，collector_handle 会在后台收集数据
                                return streaming_res;
                            }
                            Err(e) => {
                                error!("Failed to build streaming response: {}", e);
                                let mut stats = self.stats.write().await;
                                stats.errors += 1;
                                return Response::new(Body::from(""));
                            }
                        }
                    }

                    // 非流式响应：使用原有的全量缓冲逻辑
                    match self.build_response_context(request_id.clone(), res).await {
                        Ok((mut resp_ctx, new_res)) => {
                            debug!(
                                "Response captured: request_id={}, status={}, body_size={}, conn_key={}",
                                request_id,
                                resp_ctx.status,
                                resp_ctx.body.len(),
                                conn_key
                            );

                            // 检查是否需要拦截响应
                            let mut final_response = new_res;
                            if let Some(intercept_state) = &self.intercept_state {
                                let response_intercept_enabled =
                                    *intercept_state.response_enabled.read().await;

                                if response_intercept_enabled {
                                    if let Some(pending_tx) = &intercept_state.pending_response_tx {
                                        let response_id = uuid::Uuid::new_v4().to_string();
                                        let (action_tx, action_rx) =
                                            tokio::sync::oneshot::channel();

                                        // 发送拦截响应到待处理队列
                                        let body_string =
                                            String::from_utf8_lossy(&resp_ctx.body).to_string();
                                        let pending_response = PendingInterceptResponse {
                                            id: response_id.clone(),
                                            request_id: request_id.clone(),
                                            status: resp_ctx.status,
                                            headers: resp_ctx.headers.clone(),
                                            body: Some(body_string),
                                            timestamp: chrono::Utc::now().timestamp_millis(),
                                            response_tx: action_tx,
                                        };

                                        if pending_tx.send(pending_response).is_ok() {
                                            info!(
                                                "Response intercepted: {} (status: {})",
                                                response_id, resp_ctx.status
                                            );

                                            // 等待用户操作（最多30秒）
                                            match tokio::time::timeout(
                                                std::time::Duration::from_secs(30),
                                                action_rx,
                                            )
                                            .await
                                            {
                                                Ok(Ok(InterceptAction::Forward(
                                                    modified_content,
                                                ))) => {
                                                    info!("Response {} forwarded", response_id);
                                                    if let Some(content) = modified_content {
                                                        // 解析修改后的内容并重建响应
                                                        match Self::parse_and_rebuild_response(
                                                            &content,
                                                        ) {
                                                            Ok(modified_resp) => {
                                                                info!("Response {} modified and forwarded", response_id);

                                                                // 更新 resp_ctx 保存修改后的数据
                                                                resp_ctx.was_edited = true;
                                                                resp_ctx.edited_status = Some(
                                                                    modified_resp.status().as_u16(),
                                                                );

                                                                // 保存修改后的 headers
                                                                let mut edited_headers =
                                                                    std::collections::HashMap::new(
                                                                    );
                                                                for (name, value) in
                                                                    modified_resp.headers().iter()
                                                                {
                                                                    if let Ok(v) = value.to_str() {
                                                                        edited_headers.insert(
                                                                            name.to_string(),
                                                                            v.to_string(),
                                                                        );
                                                                    }
                                                                }
                                                                resp_ctx.edited_headers =
                                                                    Some(edited_headers);

                                                                // 注意：修改后的 body 需要从 content 中解析
                                                                // 因为 modified_resp 的 body 已经被消费了，我们从原始 content 中提取
                                                                if let Some(body_start) =
                                                                    content.find("\r\n\r\n")
                                                                {
                                                                    let body_content =
                                                                        &content[body_start + 4..];
                                                                    resp_ctx.edited_body = Some(
                                                                        body_content
                                                                            .as_bytes()
                                                                            .to_vec(),
                                                                    );
                                                                }

                                                                final_response = modified_resp;
                                                            }
                                                            Err(e) => {
                                                                warn!("Failed to parse modified response: {}, using original", e);
                                                            }
                                                        }
                                                    }
                                                }
                                                Ok(Ok(InterceptAction::Drop)) => {
                                                    info!("Response {} dropped", response_id);
                                                    // 返回一个空响应
                                                    return Response::builder()
                                                        .status(204)
                                                        .body(Body::empty())
                                                        .unwrap_or_else(|_| {
                                                            Response::new(Body::from(""))
                                                        });
                                                }
                                                Ok(Err(_)) => {
                                                    warn!(
                                                        "Response intercept channel closed for {}",
                                                        response_id
                                                    );
                                                }
                                                Err(_) => {
                                                    warn!("Response intercept timeout for {}, forwarding", response_id);
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // 发送到扫描器
                            if let Err(e) = tx.send(ScanTask::Response(resp_ctx)) {
                                warn!("Failed to send response to scanner: {}", e);
                            }

                            // 清理请求映射（队列中的请求ID已在上面pop_front时移除）
                            {
                                let mut request_map = self.request_map.write().await;
                                request_map.remove(&request_id);
                            }

                            // 返回响应
                            final_response
                        }
                        Err(e) => {
                            error!("Failed to build response context: {}", e);
                            let mut stats = self.stats.write().await;
                            stats.errors += 1;
                            // 由于 res 已经被 move，返回一个空响应
                            Response::new(Body::from(""))
                        }
                    }
                } else {
                    warn!(
                        "Request context not found for request_id: {} (status: {}, conn_key: {})",
                        request_id, status, conn_key
                    );
                    res
                }
            } else {
                // CONNECT 隧道响应没有 request_id 是正常的，降级为 debug
                debug!(
                    "No request_id for connection (status: {}, conn_key: {}). Expected for CONNECT tunnels.",
                    status, conn_key
                );
                res
            }
        } else {
            // 无扫描器，直接转发
            res
        }
    }

    fn handle_error(
        &mut self,
        ctx: &HttpContext,
        err: hyper_util::client::legacy::Error,
    ) -> impl std::future::Future<Output = Response<Body>> + Send {
        let conn_key = Self::generate_connection_key(ctx);

        // 复制必要的状态用于 async 块
        let stats = self.stats.clone();
        let self_clone = self.clone();
        let scan_tx = self.scan_tx.clone();

        async move {
            use std::error::Error as _;

            // 在异步上下文中获取 host，避免在 Tokio runtime 线程中使用阻塞读
            let host_opt = {
                let map = self_clone.conn_to_host.read().await;
                map.get(&conn_key).cloned()
            };

            let error_msg = err.to_string();
            let error_debug = format!("{:?}", err);
            let error_chain = {
                let mut parts = Vec::new();
                parts.push(error_msg.clone());
                let mut src = err.source();
                while let Some(e) = src {
                    parts.push(e.to_string());
                    src = e.source();
                }
                parts.join(" | ")
            };

            if let Some(host) = &host_opt {
                // 启发式匹配证书/握手错误：
                // - error_msg 可能只有 "client error (Connect)"
                // - 真实原因通常在 error chain / debug 信息里
                let tls_probe = format!("{} {}", error_chain, error_debug).to_lowercase();
                let is_tls_error = tls_probe.contains("certificate")
                    || tls_probe.contains("invalid peer certificate")
                    || tls_probe.contains("unsupportedcertversion")
                    || tls_probe.contains("tls")
                    || tls_probe.contains("alert")
                    || tls_probe.contains("handshake");

                if is_tls_error {
                    // 发送失败连接记录到扫描器（用于统计和展示）
                    if let Some(tx) = &scan_tx {
                        // 解析 host:port
                        let (hostname, port) = if host.contains(':') {
                            let parts: Vec<&str> = host.split(':').collect();
                            (
                                parts[0].to_string(),
                                parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(443),
                            )
                        } else {
                            (host.clone(), 443)
                        };

                        let failed_conn = FailedConnection {
                            id: uuid::Uuid::new_v4().to_string(),
                            host: hostname,
                            port,
                            error: error_msg.clone(),
                            timestamp: chrono::Utc::now(),
                        };

                        if let Err(e) = tx.send(ScanTask::FailedConnection(failed_conn)) {
                            warn!("Failed to send failed connection to scanner: {}", e);
                        }
                    }

                    // 不再自动绕过MITM，因为我们已经配置为忽略证书错误
                    // 只记录警告信息供调试
                    warn!(
                        "TLS error detected for host {}, but continuing with MITM (certificate validation disabled)",
                        host
                    );
                }

                // 清理连接→host 映射
                {
                    let mut map = self_clone.conn_to_host.write().await;
                    map.remove(&conn_key);
                }
            }

            {
                let mut s = stats.write().await;
                s.errors += 1;
            }

            // 区分错误类型：连接错误和超时是常见的，降级为 warn
            let error_lower = error_msg.to_lowercase();
            if error_lower.contains("connect")
                || error_lower.contains("timeout")
                || error_lower.contains("reset")
                || error_lower.contains("refused")
                || error_lower.contains("closed")
            {
                debug!(
                    "Forward request failed (network): host={:?} conn_key={} error={}",
                    host_opt, conn_key, error_chain
                );
                debug!(
                    "Forward request failed debug: host={:?} conn_key={} debug={}",
                    host_opt, conn_key, error_debug
                );
            } else {
                error!(
                    "Forward request failed: host={:?} conn_key={} error={}",
                    host_opt, conn_key, error_chain
                );
                debug!(
                    "Forward request failed debug: host={:?} conn_key={} debug={}",
                    host_opt, conn_key, error_debug
                );
            }

            Response::builder()
                .status(hudsucker::hyper::StatusCode::BAD_GATEWAY)
                .body(Body::empty())
                .expect("Failed to build response")
        }
    }
}

/// WebSocket 处理器实现
///
/// 处理 WebSocket 消息的转发，记录消息到扫描器，
/// 优雅地处理连接关闭等情况，避免产生大量无意义的错误日志。
impl WebSocketHandler for PassiveProxyHandler {
    async fn handle_message(&mut self, ctx: &WebSocketContext, msg: Message) -> Option<Message> {
        // 获取消息类型和内容 (用于记录)
        let (message_type, content, content_length) = match &msg {
            Message::Text(text) => {
                // Utf8Bytes 实现 Deref<str>，需要转换为 String
                let text_str = text.to_string();
                let len = text_str.len();
                ("text".to_string(), Some(text_str), len)
            }
            Message::Binary(data) => {
                // 对于二进制数据，尝试转换为 base64
                use base64::{engine::general_purpose, Engine as _};
                let base64_content = format!("[BASE64]{}", general_purpose::STANDARD.encode(data));
                let len = data.len();
                ("binary".to_string(), Some(base64_content), len)
            }
            Message::Ping(data) => ("ping".to_string(), None, data.len()),
            Message::Pong(data) => ("pong".to_string(), None, data.len()),
            Message::Close(reason) => {
                let reason_str = reason
                    .as_ref()
                    .map(|r| format!("code={}, reason={}", r.code, r.reason));
                ("close".to_string(), reason_str, 0)
            }
            _ => ("unknown".to_string(), None, 0),
        };

        // 发送消息到扫描器 (用于记录到历史缓存)
        if let Some(tx) = &self.scan_tx {
            // 生成唯一的消息 ID
            let message_id = uuid::Uuid::new_v4().to_string();

            // 从连接映射中获取 WebSocket 连接 ID
            let conn_key = Self::generate_ws_connection_key(ctx);
            let connection_id = {
                let ws_map = self.conn_to_ws_id.read().await;
                ws_map.get(&conn_key).cloned()
            };

            // 如果找不到连接 ID，使用连接键作为备用
            let connection_id = connection_id.unwrap_or_else(|| {
                warn!(
                    "WebSocket connection ID not found for conn_key: {}, using fallback",
                    conn_key
                );
                format!("ws-unknown-{}", uuid::Uuid::new_v4().simple())
            });

            // 判断消息方向
            // Hudsucker 的 handle_message 对于 WebSocket 会被调用两次：
            // 1. 客户端 -> 服务器方向的消息
            // 2. 服务器 -> 客户端方向的消息
            // 通过交替计数来判断方向（简单但有效的方法）
            // 更精确的方法需要 Hudsucker 提供更多上下文信息

            // 获取或创建此连接的消息计数器
            let conn_key_for_counter = conn_key.clone();
            let direction = {
                let mut ws_counters = self.ws_message_counters.write().await;
                let counter = ws_counters.entry(conn_key_for_counter).or_insert(0);
                *counter += 1;

                // 假设消息交替出现：奇数为客户端->服务器，偶数为服务器->客户端
                // 这是一个简化假设，可能不完全准确，但对大多数情况有效
                if *counter % 2 == 1 {
                    WebSocketDirection::ClientToServer
                } else {
                    WebSocketDirection::ServerToClient
                }
            };

            // 拦截逻辑
            let mut intercepted = false;
            if let Some(intercept_state) = &self.intercept_state {
                let websocket_enabled = *intercept_state.websocket_enabled.read().await;

                // 只拦截文本和二进制消息
                let should_intercept = websocket_enabled
                    && match &msg {
                        Message::Text(_) | Message::Binary(_) => true,
                        _ => false,
                    };

                if should_intercept {
                    if let Some(pending_tx) = &intercept_state.pending_websocket_tx {
                        intercepted = true;

                        // 创建 oneshot channel
                        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

                        let pending_msg = PendingInterceptWebSocketMessage {
                            id: message_id.clone(),
                            connection_id: connection_id.clone(),
                            direction,
                            message_type: message_type.clone(),
                            content: content.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis(),
                            response_tx,
                        };

                        info!("Intercepting WebSocket message: {}", message_id);

                        // 发送到待处理队列
                        if let Err(e) = pending_tx.send(pending_msg) {
                            warn!("Failed to send intercept websocket message: {}", e);
                            intercepted = false; // 发送失败，回退到正常记录
                        } else {
                            // 等待用户操作
                            match response_rx.await {
                                Ok(action) => match action {
                                    InterceptAction::Forward(modified_content) => {
                                        // 获取最终要发送和记录的内容
                                        let (final_content, final_length) =
                                            if let Some(ref new_content) = modified_content {
                                                (Some(new_content.clone()), new_content.len())
                                            } else {
                                                (content.clone(), content_length)
                                            };

                                        // 记录最终发送的消息到历史
                                        let final_msg_ctx = WebSocketMessageContext {
                                            id: message_id.clone(),
                                            connection_id: connection_id.clone(),
                                            direction,
                                            message_type: message_type.clone(),
                                            content: final_content,
                                            content_length: final_length,
                                            timestamp: chrono::Utc::now(),
                                        };

                                        if let Err(e) =
                                            tx.send(ScanTask::WebSocketMessage(final_msg_ctx))
                                        {
                                            warn!(
                                                "Failed to send WebSocket message to scanner: {}",
                                                e
                                            );
                                        } else {
                                            info!("WebSocket message recorded after intercept: conn_id={}, type={}, modified={}", 
                                                connection_id, message_type, modified_content.is_some());
                                        }

                                        // 如果有修改，发送修改后的消息
                                        if let Some(new_content) = modified_content {
                                            if message_type == "text" {
                                                return Some(Message::Text(new_content.into()));
                                            } else if message_type == "binary" {
                                                // 尝试从 base64 解码
                                                let clean_content =
                                                    if new_content.starts_with("[BASE64]") {
                                                        &new_content[8..]
                                                    } else {
                                                        &new_content
                                                    };

                                                use base64::{
                                                    engine::general_purpose, Engine as _,
                                                };
                                                if let Ok(decoded) =
                                                    general_purpose::STANDARD.decode(clean_content)
                                                {
                                                    return Some(Message::Binary(decoded.into()));
                                                } else {
                                                    warn!("Failed to decode base64 content for modified WebSocket message");
                                                }
                                            }
                                        }
                                        // 无修改，继续处理（走到下面的 match）
                                    }
                                    InterceptAction::Drop => {
                                        info!("Dropped WebSocket message: {}", message_id);
                                        return None;
                                    }
                                },
                                Err(_) => {
                                    warn!(
                                        "Intercept response channel closed for WebSocket message"
                                    );
                                    intercepted = false; // channel 关闭，回退到正常记录
                                }
                            }
                        }
                    }
                }
            }

            // 如果没有被拦截，正常记录消息到历史
            if !intercepted {
                let msg_ctx = WebSocketMessageContext {
                    id: message_id.clone(),
                    connection_id: connection_id.clone(),
                    direction,
                    message_type: message_type.clone(),
                    content: content.clone(),
                    content_length,
                    timestamp: chrono::Utc::now(),
                };

                if let Err(e) = tx.send(ScanTask::WebSocketMessage(msg_ctx)) {
                    warn!("Failed to send WebSocket message to scanner: {}", e);
                } else {
                    debug!(
                        "WebSocket message recorded: conn_id={}, type={}, length={}",
                        connection_id, message_type, content_length
                    );
                }
            }
        }

        // 原有的消息处理逻辑
        match &msg {
            Message::Close(_) => {
                debug!("WebSocket close message received, closing connection gracefully");
                None // 返回 None 优雅关闭，不转发 close 帧
            }
            Message::Ping(data) => {
                // 自动响应 Ping 为 Pong
                Some(Message::Pong(data.clone()))
            }
            _ => {
                // 转发其他所有消息（Text, Binary, Pong 等）
                Some(msg)
            }
        }
    }
}

/// 代理服务
pub struct ProxyService {
    config: ProxyConfig,
    handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    stats: Arc<RwLock<ProxyStats>>,
    actual_port: Arc<RwLock<Option<u16>>>,
    ca_dir: std::path::PathBuf,
    intercept_state: Option<InterceptState>,
}

impl ProxyService {
    pub fn new(config: ProxyConfig) -> Self {
        // 默认使用当前工作目录下的 ./ca 目录
        let ca_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("ca");

        Self {
            config,
            handle: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(ProxyStats::default())),
            actual_port: Arc::new(RwLock::new(None)),
            ca_dir,
            intercept_state: None,
        }
    }

    /// 创建代理服务实例（指定 CA 目录）
    pub fn with_ca_dir(config: ProxyConfig, ca_dir: std::path::PathBuf) -> Self {
        Self {
            config,
            handle: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(ProxyStats::default())),
            actual_port: Arc::new(RwLock::new(None)),
            ca_dir,
            intercept_state: None,
        }
    }

    /// 创建代理服务实例（支持拦截）
    pub fn with_intercept(
        config: ProxyConfig,
        ca_dir: std::path::PathBuf,
        intercept_state: InterceptState,
    ) -> Self {
        Self {
            config,
            handle: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(ProxyStats::default())),
            actual_port: Arc::new(RwLock::new(None)),
            ca_dir,
            intercept_state: Some(intercept_state),
        }
    }

    /// 启动代理服务（端口自动递增）
    pub async fn start(&self, scan_tx: Option<ScanSender>) -> Result<u16> {
        // 检查是否已启动
        {
            let handle = self.handle.read().await;
            if handle.is_some() {
                return Err(PassiveError::Proxy("Proxy already running".to_string()));
            }
        }

        // 尝试绑定端口（自动递增）
        let mut port = self.config.start_port;
        let listener = loop {
            if port >= self.config.start_port + self.config.max_port_attempts {
                return Err(PassiveError::Proxy(format!(
                    "Failed to bind port after {} attempts",
                    self.config.max_port_attempts
                )));
            }

            let addr = SocketAddr::from(([127, 0, 0, 1], port));
            match TcpListener::bind(addr).await {
                Ok(listener) => {
                    info!("Proxy bound to port {}", port);
                    break listener;
                }
                Err(e) => {
                    warn!("Port {} in use, trying next: {}", port, e);
                    port += 1;
                }
            }
        };

        // 保存实际端口
        *self.actual_port.write().await = Some(port);

        // 确保 Root CA 存在
        let ca_service = crate::certificate::CertificateService::new(self.ca_dir.clone());
        ca_service.ensure_root_ca().await?;

        // macOS: 启动前检查是否已受信；未受信则尝试提示或自动导入
        #[cfg(target_os = "macos")]
        {
            match ca_service.is_root_ca_trusted_macos().await {
                Ok(true) => info!("Root CA is trusted in macOS System keychain"),
                Ok(false) => {
                    warn!(
                        "Root CA not trusted in macOS System keychain. Attempting to add trust..."
                    );
                    if let Err(e) = ca_service.trust_root_ca_macos().await {
                        warn!(
                            "Auto trust failed: {}. You may need to import {} into System keychain and set to 'Always Trust'.",
                            e,
                            ca_service.export_root_ca().map(|p| p.display().to_string()).unwrap_or_else(|_| "<unknown>".into())
                        );
                    } else {
                        info!("Root CA added to System keychain");
                    }
                }
                Err(e) => warn!("Failed to check macOS trust status: {}", e),
            }
        }

        // Windows: 启动前检查是否已受信；未受信则尝试自动导入
        #[cfg(target_os = "windows")]
        {
            match ca_service.is_root_ca_trusted_windows().await {
                Ok(true) => info!("Root CA is trusted in Windows Certificate Store"),
                Ok(false) => {
                    warn!("Root CA not trusted in Windows Certificate Store. Attempting to add trust...");
                    if let Err(e) = ca_service.trust_root_ca_windows().await {
                        warn!(
                            "Auto trust failed: {}. You may need to manually import {} into 'Trusted Root Certification Authorities'.",
                            e,
                            ca_service.export_root_ca().map(|p| p.display().to_string()).unwrap_or_else(|_| "<unknown>".into())
                        );
                    } else {
                        info!("Root CA added to Windows Certificate Store");
                    }
                }
                Err(e) => warn!("Failed to check Windows trust status: {}", e),
            }
        }

        // 获取 CA authority（使用完整证书链版本）
        let ca = ca_service.get_chained_ca()?;

        // 创建处理器（如果有拦截状态，则使用支持拦截的构造器）
        let handler = if let Some(intercept_state) = &self.intercept_state {
            PassiveProxyHandler::with_intercept(
                self.config.clone(),
                scan_tx,
                intercept_state.clone(),
            )
        } else {
            PassiveProxyHandler::new(self.config.clone(), scan_tx)
        };
        let stats = handler.stats();

        // 检查是否配置了 upstream proxy
        // 检查并打印 upstream proxy 配置
        info!(
            "Checking upstream proxy config: {:?}",
            self.config.upstream_proxy
        );
        let use_upstream_proxy = self
            .config
            .upstream_proxy
            .as_ref()
            .map(|up| {
                info!(
                    "Upstream proxy found - enabled: {}, host: {}, port: {}",
                    up.enabled, up.proxy_host, up.proxy_port
                );
                up.enabled
            })
            .unwrap_or(false);

        info!("Use upstream proxy decision: {}", use_upstream_proxy);

        // 创建 HTTPS 连接器（根据是否使用 upstream proxy）
        let proxy_task = if use_upstream_proxy {
            if let Some(upstream_config) = &self.config.upstream_proxy {
                info!(
                    "Starting HTTPS MITM proxy on port {} with upstream proxy {}:{} (destination: {})",
                    port, upstream_config.proxy_host, upstream_config.proxy_port, upstream_config.destination_host
                );

                // 创建带 upstream proxy 的连接器
                let proxy_connector = match create_upstream_proxy_connector(upstream_config) {
                    Ok(connector) => connector,
                    Err(e) => {
                        error!("Failed to create upstream proxy connector: {}", e);
                        return Err(e);
                    }
                };

                // 包装 connector 以返回 hyper_util::rt::TokioIo
                let http_connector = ServiceBuilder::new()
                    .map_response(|stream| hyper_util::rt::TokioIo::new(stream))
                    .service(proxy_connector.clone());

                // WebSocket 连接器: tokio-tungstenite 0.28 不再使用 Connector enum
                // 改用 with_rustls_connector，但它需要直接的 TLS stream
                // 由于我们的 ProxyStream 已经处理了 upstream proxy，这里简化处理
                // 暂时不配置 ws_connector，使用默认行为（可能会有问题，但至少能编译）

                tokio::spawn(async move {
                    match Proxy::builder()
                        .with_listener(listener)
                        .with_ca(ca)
                        .with_http_connector(http_connector)
                        .with_http_handler(handler.clone())
                        .with_websocket_handler(handler)
                        .build()
                    {
                        Ok(proxy) => {
                            if let Err(e) = proxy.start().await {
                                error!("Proxy error: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to build proxy: {}", e);
                        }
                    }
                })
            } else {
                return Err(PassiveError::Proxy(
                    "Upstream proxy enabled but not configured".to_string(),
                ));
            }
        } else {
            // 不使用 upstream proxy，创建忽略证书验证的连接器
            info!(
                "Starting HTTPS MITM proxy on port {} (ignoring upstream cert errors)",
                port
            );

            // 创建忽略证书验证的 rustls ClientConfig
            let rustls_config = create_insecure_rustls_config();

            // 使用 hyper-rustls connector with custom TLS config
            use hyper_rustls::HttpsConnectorBuilder;

            let https_connector = HttpsConnectorBuilder::new()
                .with_tls_config(rustls_config)
                .https_or_http()
                .enable_http1()
                .enable_http2()
                .build();

            tokio::spawn(async move {
                match Proxy::builder()
                    .with_listener(listener)
                    .with_ca(ca)
                    .with_http_connector(https_connector)
                    .with_http_handler(handler.clone())
                    .with_websocket_handler(handler)
                    .build()
                {
                    Ok(proxy) => {
                        if let Err(e) = proxy.start().await {
                            error!("Proxy error: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to build proxy: {}", e);
                    }
                }
            })
        };

        // 保存任务句柄和统计
        *self.handle.write().await = Some(proxy_task);
        *self.stats.write().await = stats.read().await.clone();

        Ok(port)
    }

    /// 停止代理服务
    pub async fn stop(&self) -> Result<()> {
        let mut handle = self.handle.write().await;
        if let Some(task) = handle.take() {
            task.abort();
            info!("Proxy stopped");
        }
        *self.actual_port.write().await = None;
        Ok(())
    }

    /// 获取当前端口
    pub async fn get_port(&self) -> Option<u16> {
        *self.actual_port.read().await
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> ProxyStats {
        self.stats.read().await.clone()
    }
}

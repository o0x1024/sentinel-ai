//! 代理核心模块
//!
//! 基于 Hudsucker 实现 HTTP/HTTPS 拦截代理，支持：
//! - 端口自动递增（4201 → 4202 → ...）
//! - HTTPS MITM（默认启用）
//! - 请求/响应 tee（异步扫描队列）

use crate::{PassiveError, ProxyStats, RequestContext, ResponseContext, Result};
use http_body_util::{BodyExt, Full};
use hudsucker::{
    hyper::{self, Request, Response},
    tokio_tungstenite::tungstenite::Message,
    Body, HttpContext, HttpHandler, Proxy, RequestOrResponse, WebSocketContext, WebSocketHandler,
};
use hyper::body::Bytes;
use serde::{Deserialize, Serialize};
use std::{io::Read, net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::RwLock, task::JoinHandle};
use tracing::{debug, error, info, warn};
use flate2::read::GzDecoder;
use brotli::Decompressor;
use std::collections::{HashMap, HashSet};

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

/// 拦截状态（共享）
#[derive(Clone)]
pub struct InterceptState {
    /// 是否启用请求拦截
    pub enabled: Arc<RwLock<bool>>,
    /// 是否启用响应拦截
    pub response_enabled: Arc<RwLock<bool>>,
    /// 待处理的拦截请求发送端
    pub pending_tx: Option<tokio::sync::mpsc::UnboundedSender<PendingInterceptRequest>>,
    /// 待处理的拦截响应发送端
    pub pending_response_tx: Option<tokio::sync::mpsc::UnboundedSender<PendingInterceptResponse>>,
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
}

fn default_bypass_threshold() -> u32 {
    3
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
}

/// 代理处理器（实现 Hudsucker HttpHandler）
#[derive(Clone)]
pub struct PassiveProxyHandler {
    config: ProxyConfig,
    stats: Arc<RwLock<ProxyStats>>,
    scan_tx: Option<ScanSender>,
    /// 请求ID映射（用于关联请求和响应）
    /// 使用请求ID作为键来关联请求和响应
    request_map: Arc<RwLock<std::collections::HashMap<String, RequestContext>>>,
    /// 连接键到请求ID的映射（用于响应匹配）
    conn_to_request: Arc<RwLock<std::collections::HashMap<String, String>>>,
    /// 需要绕过 MITM 的域名集合
    bypass_hosts: Arc<RwLock<HashSet<String>>>,
    /// 域名失败计数
    fail_counts: Arc<RwLock<HashMap<String, u32>>>,
    /// 连接键到 CONNECT host 的映射
    conn_to_host: Arc<RwLock<HashMap<String, String>>>,
    /// 拦截状态
    intercept_state: Option<InterceptState>,
}

impl PassiveProxyHandler {
    pub fn new(config: ProxyConfig, scan_tx: Option<ScanSender>) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(ProxyStats::default())),
            scan_tx,
            request_map: Arc::new(RwLock::new(std::collections::HashMap::new())),
            conn_to_request: Arc::new(RwLock::new(std::collections::HashMap::new())),
            bypass_hosts: Arc::new(RwLock::new(HashSet::new())),
            fail_counts: Arc::new(RwLock::new(HashMap::new())),
            conn_to_host: Arc::new(RwLock::new(HashMap::new())),
            intercept_state: None,
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
            conn_to_request: Arc::new(RwLock::new(std::collections::HashMap::new())),
            bypass_hosts: Arc::new(RwLock::new(HashSet::new())),
            fail_counts: Arc::new(RwLock::new(HashMap::new())),
            conn_to_host: Arc::new(RwLock::new(HashMap::new())),
            intercept_state: Some(intercept_state),
        }
    }

    pub fn stats(&self) -> Arc<RwLock<ProxyStats>> {
        self.stats.clone()
    }

    /// 生成连接关联键（基于连接信息）
    fn generate_connection_key(ctx: &HttpContext) -> String {
        // 使用连接的远程地址作为键
        format!("{:?}", ctx)
    }

    /// 从 CONNECT 请求中提取 host（去掉端口）
    fn parse_connect_host(req: &Request<Body>) -> Option<String> {
        // CONNECT 请求的 URI 通常为 authority 形式，如 host:443
        let authority = req
            .uri()
            .authority()
            .map(|a| a.as_str().to_string())
            .or_else(|| req.headers().get("host").and_then(|h| h.to_str().ok()).map(|s| s.to_string()));
        authority.map(|auth| auth.split(':').next().unwrap_or(&auth).to_string())
    }

    /// 标记某域名失败一次；到达阈值后加入 bypass 列表
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
            warn!("TLS/MITM failure counted for host {} ({} / {})", host, c, threshold);
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
                match GzDecoder::new(body_bytes).bytes().collect::<std::io::Result<Vec<u8>>>() {
                    Ok(decompressed) => {
                        debug!("Decompressed gzip body: {} -> {} bytes", body_bytes.len(), decompressed.len());
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
                        debug!("Decompressed brotli body: {} -> {} bytes", body_bytes.len(), decompressed.len());
                        decompressed
                    }
                    Err(e) => {
                        warn!("Failed to decompress brotli body: {}, returning original", e);
                        body_bytes.to_vec()
                    }
                }
            }
            "deflate" => {
                // deflate 也是 zlib 格式
                match flate2::read::DeflateDecoder::new(body_bytes).bytes().collect::<std::io::Result<Vec<u8>>>() {
                    Ok(decompressed) => {
                        debug!("Decompressed deflate body: {} -> {} bytes", body_bytes.len(), decompressed.len());
                        decompressed
                    }
                    Err(e) => {
                        warn!("Failed to decompress deflate body: {}, returning original", e);
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
        let content_encoding = headers.get("content-encoding")
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
        };

        Ok((resp_ctx, new_res))
    }
}

impl HttpHandler for PassiveProxyHandler {
    fn should_intercept(
        &mut self,
        _ctx: &HttpContext,
        req: &Request<Body>,
    ) -> impl std::future::Future<Output = bool> + Send {
        let bypass_hosts = self.bypass_hosts.clone();
        async move {
            // 仅对 CONNECT 生效
            if req.method() != hyper::Method::CONNECT {
                return true;
            }
            let host_opt = Self::parse_connect_host(&req);
            if let Some(host) = host_opt {
                let bypass = bypass_hosts.read().await;
                if bypass.contains(&host) {
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

        // 若为 CONNECT，记录 host → conn_key 映射以便错误统计
        if is_https {
            if let Some(host) = Self::parse_connect_host(&req) {
                let conn_key = Self::generate_connection_key(ctx);
                let mut map = self.conn_to_host.write().await;
                map.insert(conn_key, host);
            }
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

                        // 同时保存连接键到请求ID的映射
                        let mut conn_map = self.conn_to_request.write().await;
                        conn_map.insert(conn_key, request_id.clone());

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

                        // 清理连接映射
                        if conn_map.len() > 1000 {
                            let keys_to_remove: Vec<_> =
                                conn_map.keys().take(500).cloned().collect();
                            for key in keys_to_remove {
                                conn_map.remove(&key);
                            }
                        }
                    }

                    // 发送到扫描器
                    if let Err(e) = tx.send(ScanTask::Request(req_ctx.clone())) {
                        warn!("Failed to send request to scanner: {}", e);
                    }

                    // 检查是否启用了拦截模式（跳过 CONNECT 请求）
                    if !is_https {
                        if let Some(intercept_state) = &self.intercept_state {
                            let intercept_enabled = *intercept_state.enabled.read().await;
                            if intercept_enabled {
                                if let Some(pending_tx) = &intercept_state.pending_tx {
                                    // 创建 oneshot channel 等待用户操作
                                    let (response_tx, response_rx) = tokio::sync::oneshot::channel();
                                    
                                    // 解析 URL path
                                    let path = uri.path_and_query()
                                        .map(|pq| pq.to_string())
                                        .unwrap_or_else(|| uri.path().to_string());
                                    
                                    let pending_request = PendingInterceptRequest {
                                        id: request_id.clone(),
                                        method: req_ctx.method.clone(),
                                        url: req_ctx.url.clone(),
                                        path,
                                        protocol: if req_ctx.is_https { "HTTPS".to_string() } else { "HTTP/1.1".to_string() },
                                        headers: req_ctx.headers.clone(),
                                        body: if req_ctx.body.is_empty() { None } else { String::from_utf8(req_ctx.body.clone()).ok() },
                                        timestamp: chrono::Utc::now().timestamp_millis(),
                                        response_tx,
                                    };
                                    
                                    // 发送到待处理队列
                                    if let Err(e) = pending_tx.send(pending_request) {
                                        warn!("Failed to send intercept request: {}", e);
                                        return RequestOrResponse::Request(new_req);
                                    }
                                    
                                    info!("Request {} intercepted, waiting for user action", request_id);
                                    
                                    // 等待用户操作（带超时）
                                    match tokio::time::timeout(
                                        std::time::Duration::from_secs(300), // 5 minutes timeout
                                        response_rx
                                    ).await {
                                        Ok(Ok(action)) => {
                                            match action {
                                                InterceptAction::Forward(modified_content) => {
                                                    info!("Request {} forwarded by user", request_id);
                                                    if let Some(_content) = modified_content {
                                                        // TODO: Parse modified content and rebuild request
                                                        // For now, just forward the original request
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
                                                            .unwrap_or_else(|_| Response::new(Body::empty()))
                                                    );
                                                }
                                            }
                                        }
                                        Ok(Err(_)) => {
                                            warn!("Intercept channel closed for request {}, forwarding", request_id);
                                            return RequestOrResponse::Request(new_req);
                                        }
                                        Err(_) => {
                                            warn!("Intercept timeout for request {}, forwarding", request_id);
                                            return RequestOrResponse::Request(new_req);
                                        }
                                    }
                                }
                            }
                        }
                    }

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

        // 构建上下文并发送到扫描器
        if let Some(tx) = &self.scan_tx {
            // 从连接映射中获取请求ID
            let request_id_opt = {
                let conn_map = self.conn_to_request.read().await;
                conn_map.get(&conn_key).cloned()
            };

            if let Some(request_id) = request_id_opt {
                // 获取请求上下文
                let req_ctx_opt = {
                    let request_map = self.request_map.read().await;
                    request_map.get(&request_id).cloned()
                };

                if let Some(_req_ctx) = req_ctx_opt {
                    match self.build_response_context(request_id.clone(), res).await {
                        Ok((resp_ctx, new_res)) => {
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
                                let response_intercept_enabled = *intercept_state.response_enabled.read().await;
                                
                                if response_intercept_enabled {
                                    if let Some(pending_tx) = &intercept_state.pending_response_tx {
                                        let response_id = uuid::Uuid::new_v4().to_string();
                                        let (action_tx, action_rx) = tokio::sync::oneshot::channel();
                                        
                                        // 发送拦截响应到待处理队列
                                        let body_string = String::from_utf8_lossy(&resp_ctx.body).to_string();
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
                                            info!("Response intercepted: {} (status: {})", response_id, resp_ctx.status);
                                            
                                            // 等待用户操作（最多30秒）
                                            match tokio::time::timeout(
                                                std::time::Duration::from_secs(30),
                                                action_rx
                                            ).await {
                                                Ok(Ok(InterceptAction::Forward(modified_content))) => {
                                                    info!("Response {} forwarded", response_id);
                                                    if let Some(content) = modified_content {
                                                        // 用户修改了响应，重新构建
                                                        final_response = Response::builder()
                                                            .status(resp_ctx.status)
                                                            .body(Body::from(content))
                                                            .unwrap_or_else(|_| Response::new(Body::from("")));
                                                    }
                                                }
                                                Ok(Ok(InterceptAction::Drop)) => {
                                                    info!("Response {} dropped", response_id);
                                                    // 返回一个空响应
                                                    return Response::builder()
                                                        .status(204)
                                                        .body(Body::empty())
                                                        .unwrap_or_else(|_| Response::new(Body::from("")));
                                                }
                                                Ok(Err(_)) => {
                                                    warn!("Response intercept channel closed for {}", response_id);
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

                            // 清理映射
                            {
                                let mut request_map = self.request_map.write().await;
                                request_map.remove(&request_id);

                                let mut conn_map = self.conn_to_request.write().await;
                                conn_map.remove(&conn_key);
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
            // 在异步上下文中获取 host，避免在 Tokio runtime 线程中使用阻塞读
            let host_opt = {
                let map = self_clone.conn_to_host.read().await;
                map.get(&conn_key).cloned()
            };

            let error_msg = err.to_string();
            
            if let Some(host) = &host_opt {
                let msg = error_msg.to_lowercase();
                // 启发式匹配证书/握手错误
                let is_tls_error = msg.contains("certificate") 
                    || msg.contains("tls") 
                    || msg.contains("alert")
                    || msg.contains("handshake");
                    
                if is_tls_error {
                    self_clone.note_fail_and_maybe_bypass(host).await;
                    
                    // 发送失败连接记录到扫描器
                    if let Some(tx) = &scan_tx {
                        // 解析 host:port
                        let (hostname, port) = if host.contains(':') {
                            let parts: Vec<&str> = host.split(':').collect();
                            (parts[0].to_string(), parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(443))
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
                warn!("Forward request failed (network): {}", error_msg);
            } else {
                error!("Forward request failed: {}", error_msg);
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
/// 处理 WebSocket 消息的转发，优雅地处理连接关闭等情况，
/// 避免产生大量无意义的错误日志。
impl WebSocketHandler for PassiveProxyHandler {
    async fn handle_message(&mut self, _ctx: &WebSocketContext, msg: Message) -> Option<Message> {
        // 简单转发所有 WebSocket 消息
        // 对于 Close 消息，返回 None 会优雅地关闭连接
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
    pub fn with_intercept(config: ProxyConfig, ca_dir: std::path::PathBuf, intercept_state: InterceptState) -> Self {
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
                    warn!("Root CA not trusted in macOS System keychain. Attempting to add trust...");
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
            PassiveProxyHandler::with_intercept(self.config.clone(), scan_tx, intercept_state.clone())
        } else {
            PassiveProxyHandler::new(self.config.clone(), scan_tx)
        };
        let stats = handler.stats();

        // 启动代理（支持 HTTPS MITM 和 WebSocket）
        info!("Starting HTTPS MITM proxy on port {}", port);
        let proxy_task = tokio::spawn(async move {
            match Proxy::builder()
                .with_listener(listener)
                .with_ca(ca)
                .with_rustls_connector(rustls::crypto::ring::default_provider())
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
        });

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

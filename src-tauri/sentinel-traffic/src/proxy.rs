//! 代理核心模块
//!
//! 基于 Hudsucker 实现 HTTP/HTTPS 拦截代理，支持：
//! - 端口自动递增（4201 → 4202 → ...）
//! - HTTPS MITM（默认启用）
//! - 请求/响应 tee（异步扫描队列）
//! - 忽略上游证书验证（用于抓取证书异常的站点）

use crate::header_utils::{append_request_headers, append_response_headers, merge_header_value};
use crate::intercept_content::{
    parse_intercept_response_content, sanitize_edited_response_headers,
};
use crate::intercept_rules::should_intercept_response;
use crate::intercept_tracking::InterceptTracking;
use crate::match_replace::{apply_request_match_replace_rules, apply_response_match_replace_rules};
use crate::scope::{url_is_in_scope, ProxyScopeRule};
use crate::{ProxyStats, RequestContext, ResponseContext, Result, TrafficError};
use brotli::Decompressor;
use flate2::read::GzDecoder;
use http_body_util::{BodyExt, Full};
use hudsucker::{
    hyper::{self, Request, Response},
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext,
};
use hyper::body::{Bytes, Frame};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

pub use crate::proxy_types::{
    FailedConnection, InterceptAction, InterceptFilterRule, InterceptState,
    PendingInterceptRequest, PendingInterceptResponse, PendingInterceptWebSocketMessage,
    ProxyConfig, ScanSender, ScanTask, UpstreamProxyConfig, WebSocketConnectionContext,
    WebSocketDirection, WebSocketMessageContext,
};

/// 代理处理器（实现 Hudsucker HttpHandler）
///
/// Note: hudsucker calls `self.clone().proxy(req)` for each request-response pair,
/// so each pair gets its own handler clone. We must ensure `current_request_id` is **NOT shared**
/// across clones, otherwise concurrent requests will overwrite each other and cause request/response
/// mismatch in history.
pub struct TrafficProxyHandler {
    config: ProxyConfig,
    stats: Arc<RwLock<ProxyStats>>,
    pub(crate) scan_tx: Option<ScanSender>,
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
    pub(crate) conn_to_ws_id: Arc<RwLock<HashMap<String, String>>>,
    /// WebSocket 消息计数器（用于判断方向）
    pub(crate) ws_message_counters: Arc<RwLock<HashMap<String, usize>>>,
    /// 拦截状态
    pub(crate) intercept_state: Option<InterceptState>,
    /// 当前请求 ID（用于匹配 handle_request/handle_response）
    /// 注意：必须是“每个 clone 独立”的状态，不能用 Arc 共享。
    current_request_id: std::sync::Mutex<Option<String>>,
    /// 记录哪些请求曾进入过请求拦截流程，供响应规则使用。
    intercept_tracking: InterceptTracking,
}

impl Clone for TrafficProxyHandler {
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
            intercept_tracking: self.intercept_tracking.clone(),
        }
    }
}

impl TrafficProxyHandler {
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
            intercept_tracking: InterceptTracking::new(),
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
            intercept_tracking: InterceptTracking::new(),
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

    fn format_http_version(version: hyper::Version) -> Option<String> {
        match version {
            hyper::Version::HTTP_09 => Some("HTTP/0.9".to_string()),
            hyper::Version::HTTP_10 => Some("HTTP/1.0".to_string()),
            hyper::Version::HTTP_11 => Some("HTTP/1.1".to_string()),
            hyper::Version::HTTP_2 => Some("HTTP/2".to_string()),
            hyper::Version::HTTP_3 => Some("HTTP/3".to_string()),
            _ => None,
        }
    }

    /// 生成 WebSocket 连接关联键（基于连接信息）
    pub(crate) fn generate_ws_connection_key(ctx: &WebSocketContext) -> String {
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
                if let Some(end) = rest.find([',', '}']) {
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

    fn is_internal_request(req: &Request<Body>) -> bool {
        req.headers()
            .iter()
            .find(|(key, _)| key.as_str().eq_ignore_ascii_case("x-sentinel-internal"))
            .and_then(|(_, value)| value.to_str().ok())
            .map(|value| value == "true" || value == "1")
            .unwrap_or(false)
    }

    fn normalize_internal_request_uri(req: Request<Body>) -> Request<Body> {
        if req.method() == hyper::Method::CONNECT {
            return req;
        }

        if req.uri().scheme().is_some() && req.uri().authority().is_some() {
            return req;
        }

        let host = req
            .headers()
            .get("host")
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let Some(host) = host else {
            return req;
        };

        let path_and_query = req
            .uri()
            .path_and_query()
            .map(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("/");

        let Some(normalized_uri) = format!("http://{}{}", host, path_and_query)
            .parse::<hyper::Uri>()
            .ok()
        else {
            return req;
        };

        let (mut parts, body) = req.into_parts();
        parts.uri = normalized_uri;
        Request::from_parts(parts, body)
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
        include_rules: &[ProxyScopeRule],
        exclude_rules: &[ProxyScopeRule],
    ) -> bool {
        if !url_is_in_scope(url, include_rules, exclude_rules) {
            debug!("Request {} skipped by global scope rules", url);
            return false;
        }

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
            .ok_or_else(|| TrafficError::Proxy("Empty request content".to_string()))?;
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(TrafficError::Proxy(format!(
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
                merge_header_value(&mut headers, &key, &value);
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
        builder = append_request_headers(builder, &headers);

        // 构建带 body 的请求
        let body = if body_content.is_empty() {
            Body::empty()
        } else {
            Body::from(body_content)
        };

        builder
            .body(body)
            .map_err(|e| TrafficError::Proxy(format!("Failed to build request: {}", e)))
    }

    /// 解析修改后的响应内容并重建 HTTP 响应
    /// 格式: HTTP/1.1 STATUS\nHeader1: Value1\n...\n\nBODY
    fn parse_and_rebuild_response(content: &str) -> Result<Response<Body>> {
        let parsed = parse_intercept_response_content(content)?;
        let mut headers = parsed.headers;
        sanitize_edited_response_headers(&mut headers, parsed.body.len());

        // 构建响应
        let mut builder = Response::builder().status(parsed.status_code);

        // 添加头部
        builder = append_response_headers(builder, &headers);

        // 构建带 body 的响应
        let body = if parsed.body.is_empty() {
            Body::empty()
        } else {
            Body::from(Full::new(Bytes::from(parsed.body)))
        };

        builder
            .body(body)
            .map_err(|e| TrafficError::Proxy(format!("Failed to build response: {}", e)))
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
    /// 返回 (解压后的数据, 是否成功解压)
    fn decompress_body(body_bytes: &[u8], encoding: Option<&str>) -> (Vec<u8>, bool) {
        let encoding = match encoding {
            Some(e) => e.to_lowercase(),
            None => return (body_bytes.to_vec(), true), // 无压缩，视为成功
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
                        (decompressed, true)
                    }
                    Err(e) => {
                        warn!("Failed to decompress gzip body: {}, returning empty", e);
                        (Vec::new(), false) // 解压失败返回空数据，避免插件处理错误数据
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
                        (decompressed, true)
                    }
                    Err(e) => {
                        warn!("Failed to decompress brotli body: {}, returning empty", e);
                        (Vec::new(), false) // 解压失败返回空数据
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
                        (decompressed, true)
                    }
                    Err(e) => {
                        warn!("Failed to decompress deflate body: {}, returning empty", e);
                        (Vec::new(), false)
                    }
                }
            }
            _ => {
                // 不支持的编码或无编码，返回原始数据
                (body_bytes.to_vec(), true)
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

        let http_version = Self::format_http_version(req.version());

        // 提取方法
        let method = req.method().to_string();

        // 提取请求头
        let mut headers = std::collections::HashMap::new();
        for (name, value) in req.headers().iter() {
            if let Ok(v) = value.to_str() {
                merge_header_value(&mut headers, name.as_str(), v);
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

        let mut req_ctx = RequestContext {
            id,
            method: method.clone(),
            url: url.clone(),
            http_version: http_version.clone(),
            headers: headers.clone(),
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

        let match_replace_rules = match &self.intercept_state {
            Some(intercept_state) => intercept_state.match_replace_rules.read().await.clone(),
            None => Vec::new(),
        };

        if let Some(edited) = apply_request_match_replace_rules(
            &match_replace_rules,
            &method,
            &url,
            &headers,
            body_bytes.as_ref(),
            http_version.as_deref(),
            &self.config.scope_include_rules,
            &self.config.scope_exclude_rules,
        ) {
            let mut edited_headers = edited.headers.clone();
            edited_headers.remove("content-length");
            edited_headers.remove("Content-Length");
            edited_headers.remove("transfer-encoding");
            edited_headers.remove("Transfer-Encoding");
            edited_headers.insert("content-length".to_string(), edited.body.len().to_string());

            let edited_body = if edited.body.len() > self.config.max_request_body_size {
                edited.body[..self.config.max_request_body_size].to_vec()
            } else {
                edited.body.clone()
            };

            req_ctx.was_edited = true;
            req_ctx.edited_method = Some(edited.method.clone());
            req_ctx.edited_url = Some(edited.url.clone());
            req_ctx.edited_headers = Some(edited_headers.clone());
            req_ctx.edited_body = Some(edited_body);

            let method = hyper::Method::from_bytes(edited.method.as_bytes()).map_err(|error| {
                TrafficError::Proxy(format!("Invalid rewritten request method: {}", error))
            })?;
            let uri = edited.url.parse::<hyper::Uri>().map_err(|error| {
                TrafficError::Proxy(format!("Invalid rewritten request URI: {}", error))
            })?;

            let mut builder = Request::builder()
                .method(method)
                .uri(uri)
                .version(parts.version);
            builder = append_request_headers(builder, &edited_headers);

            let new_body = Body::from(Full::new(Bytes::from(edited.body)));
            let new_req = builder.body(new_body).map_err(|error| {
                TrafficError::Proxy(format!("Failed to build rewritten request: {}", error))
            })?;

            return Ok((req_ctx, new_req));
        }

        // 创建新的请求用于转发（包含原始 body）
        // hudsucker::Body 实现了 From<Full<Bytes>>
        let new_body = Body::from(Full::new(body_bytes.clone()));
        let new_req = Request::from_parts(parts, new_body);

        Ok((req_ctx, new_req))
    }

    /// 从 Hyper Response 构建 ResponseContext（读取 body）
    async fn build_response_context(
        &self,
        request_id: String,
        request_url: &str,
        res: Response<Body>,
    ) -> Result<(ResponseContext, Response<Body>)> {
        let http_version = Self::format_http_version(res.version());

        // 提取状态码
        let status = res.status().as_u16();

        // 提取响应头
        let mut headers = std::collections::HashMap::new();
        for (name, value) in res.headers().iter() {
            if let Ok(v) = value.to_str() {
                merge_header_value(&mut headers, name.as_str(), v);
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
        let (decompressed_body, decompress_success) = if content_encoding.is_some() {
            debug!(
                "Detected content encoding: {:?}, attempting decompression for request {}",
                content_encoding, request_id
            );
            Self::decompress_body(&compressed_body_vec, content_encoding)
        } else {
            (compressed_body_vec.clone(), true)
        };

        // 如果解压失败，记录警告并跳过此响应的扫描
        if !decompress_success {
            warn!(
                "Failed to decompress response body for request {}, skipping plugin scan",
                request_id
            );
        }

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
            "Captured response body: compressed={} bytes, decompressed={} bytes, decompress_success={}, status={} for request {}",
            compressed_body_vec.len(),
            body_vec.len(),
            decompress_success,
            status,
            request_id
        );

        // 但保存到数据库和扫描器的是解压后的数据
        let mut resp_ctx = ResponseContext {
            request_id,
            status,
            http_version,
            headers: headers.clone(),
            body: body_vec.clone(), // 保存解压后的数据
            content_type,
            timestamp: chrono::Utc::now(),
            was_edited: false,
            edited_status: None,
            edited_headers: None,
            edited_body: None,
        };

        let match_replace_rules = match &self.intercept_state {
            Some(intercept_state) => intercept_state.match_replace_rules.read().await.clone(),
            None => Vec::new(),
        };

        if let Some(edited) = apply_response_match_replace_rules(
            &match_replace_rules,
            request_url,
            &headers,
            &body_vec,
            &self.config.scope_include_rules,
            &self.config.scope_exclude_rules,
        ) {
            let mut edited_headers = edited.headers.clone();
            if edited.body_is_plain_text {
                edited_headers.remove("content-length");
                edited_headers.remove("Content-Length");
                edited_headers.remove("content-encoding");
                edited_headers.remove("Content-Encoding");
                edited_headers.remove("transfer-encoding");
                edited_headers.remove("Transfer-Encoding");
                edited_headers.insert("content-length".to_string(), edited.body.len().to_string());
            }

            let edited_body = if edited.body.len() > self.config.max_response_body_size {
                edited.body[..self.config.max_response_body_size].to_vec()
            } else {
                edited.body.clone()
            };

            resp_ctx.was_edited = true;
            resp_ctx.edited_status = Some(status);
            resp_ctx.edited_headers = Some(edited_headers.clone());
            resp_ctx.edited_body = Some(edited_body);

            let mut builder = Response::builder().status(status).version(parts.version);
            builder = append_response_headers(builder, &edited_headers);

            let response_body = if edited.body_is_plain_text {
                Bytes::from(edited.body)
            } else {
                body_bytes.clone()
            };
            let new_body = Body::from(Full::new(response_body));
            let new_res = builder.body(new_body).map_err(|error| {
                TrafficError::Proxy(format!("Failed to build rewritten response: {}", error))
            })?;

            return Ok((resp_ctx, new_res));
        }

        // 创建新的响应用于转发（使用压缩后的原始数据，保持原样转发）
        // hudsucker::Body 实现了 From<Full<Bytes>>
        let new_body = Body::from(Full::new(body_bytes.clone()));
        let new_res = Response::from_parts(parts, new_body);

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
        let http_version = Self::format_http_version(res.version());

        // 提取状态码
        let status = res.status().as_u16();

        // 提取响应头
        let mut headers = std::collections::HashMap::new();
        for (name, value) in res.headers().iter() {
            if let Ok(v) = value.to_str() {
                merge_header_value(&mut headers, name.as_str(), v);
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
                    http_version: http_version.clone(),
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

impl HttpHandler for TrafficProxyHandler {
    fn should_intercept(
        &mut self,
        ctx: &HttpContext,
        req: &Request<Body>,
    ) -> impl std::future::Future<Output = bool> + Send {
        let bypass_hosts = self.bypass_hosts.clone();
        let conn_to_host = self.conn_to_host.clone();
        let conn_key = Self::generate_connection_key(ctx);

        // 提前解析 host（在 async 块外）
        let host_opt = Self::parse_connect_host(req);
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
        let req = if Self::is_internal_request(&req) {
            let normalized_req = Self::normalize_internal_request_uri(req);
            debug!(
                "Capturing internal request for history only: {} {}",
                normalized_req.method(),
                normalized_req.uri()
            );
            normalized_req
        } else {
            req
        };

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

                    // 始终发送到扫描器（用于保存历史记录）
                    // 扫描器会根据配置决定是否进行插件扫描
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
                            let request_intercept_enabled =
                                *intercept_state.request_enabled.read().await;
                            info!(
                                "Intercept check: url={}, master={}, request_enabled={}",
                                req_ctx.url, intercept_enabled, request_intercept_enabled
                            );
                            if intercept_enabled && request_intercept_enabled {
                                // Check filter rules before intercepting
                                let should_intercept = Self::should_intercept_request(
                                    intercept_state,
                                    &req_ctx.url,
                                    &req_ctx.method,
                                    &req_ctx.headers,
                                    &self.config.scope_include_rules,
                                    &self.config.scope_exclude_rules,
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

                                    self.intercept_tracking
                                        .mark_request_intercepted(&request_id)
                                        .await;

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
                                                                        merge_header_value(
                                                                            &mut edited_headers,
                                                                            name.as_str(),
                                                                            v,
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
                                                    self.intercept_tracking
                                                        .clear_request(&request_id)
                                                        .await;
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
                merge_header_value(&mut response_headers, name.as_str(), v);
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

                if let Some(req_ctx) = req_ctx_opt {
                    let request_was_intercepted = self
                        .intercept_tracking
                        .was_request_intercepted(&request_id)
                        .await;

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
                                self.intercept_tracking.clear_request(&request_id).await;

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
                    match self
                        .build_response_context(request_id.clone(), &req_ctx.url, res)
                        .await
                    {
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
                                let intercept_enabled = *intercept_state.enabled.read().await;
                                let response_intercept_enabled =
                                    *intercept_state.response_enabled.read().await;

                                if intercept_enabled && response_intercept_enabled {
                                    if !url_is_in_scope(
                                        &req_ctx.url,
                                        &self.config.scope_include_rules,
                                        &self.config.scope_exclude_rules,
                                    ) {
                                        debug!(
                                            "Response {} skipped by global scope rules",
                                            request_id
                                        );
                                    } else {
                                        let response_rules = intercept_state
                                            .response_filter_rules
                                            .read()
                                            .await
                                            .clone();
                                        let should_intercept = should_intercept_response(
                                            &response_rules,
                                            &req_ctx,
                                            &resp_ctx,
                                            &self.config.scope_include_rules,
                                            &self.config.scope_exclude_rules,
                                            request_was_intercepted,
                                        );

                                        if !should_intercept {
                                            debug!(
                                                "Response {} skipped by filter rules",
                                                request_id
                                            );
                                        } else if let Some(pending_tx) =
                                            &intercept_state.pending_response_tx
                                        {
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
                                                                        modified_resp
                                                                            .status()
                                                                            .as_u16(),
                                                                    );

                                                                    match parse_intercept_response_content(&content) {
                                                                        Ok(mut parsed_content) => {
                                                                            sanitize_edited_response_headers(
                                                                                &mut parsed_content.headers,
                                                                                parsed_content.body.len(),
                                                                            );
                                                                            resp_ctx.edited_headers =
                                                                                Some(parsed_content.headers);
                                                                            resp_ctx.edited_body =
                                                                                Some(parsed_content.body);
                                                                        }
                                                                        Err(error) => {
                                                                            warn!("Failed to extract edited response content for history: {}", error);
                                                                        }
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
                                                        {
                                                            let mut request_map =
                                                                self.request_map.write().await;
                                                            request_map.remove(&request_id);
                                                        }
                                                        self.intercept_tracking
                                                            .clear_request(&request_id)
                                                            .await;
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
                            }

                            // 始终发送到扫描器（用于保存历史记录）
                            // 扫描器会根据配置决定是否进行插件扫描
                            if let Err(e) = tx.send(ScanTask::Response(resp_ctx)) {
                                warn!("Failed to send response to scanner: {}", e);
                            }

                            // 清理请求映射（队列中的请求ID已在上面pop_front时移除）
                            {
                                let mut request_map = self.request_map.write().await;
                                request_map.remove(&request_id);
                            }
                            self.intercept_tracking.clear_request(&request_id).await;

                            // 返回响应
                            final_response
                        }
                        Err(e) => {
                            self.intercept_tracking.clear_request(&request_id).await;
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

pub use crate::proxy_service::ProxyService;

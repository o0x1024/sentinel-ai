//! HTTP客户端模块
//! 
//! 提供统一的HTTP请求处理功能

use reqwest::{Client, Proxy};
use serde_json::Value;
use tokio_util::bytes;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::types::{HttpRequest, HttpResponse};

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProxyConfig {
    pub enabled: bool,
    pub scheme: Option<String>, // http|https|socks5|socks5h
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub no_proxy: Option<String>,
}

static GLOBAL_PROXY: std::sync::OnceLock<std::sync::RwLock<Option<ProxyConfig>>> = std::sync::OnceLock::new();

fn global_proxy_lock() -> &'static std::sync::RwLock<Option<ProxyConfig>> {
    GLOBAL_PROXY.get_or_init(|| std::sync::RwLock::new(None))
}

pub fn set_global_proxy(config: Option<ProxyConfig>) {
    if let Ok(mut w) = global_proxy_lock().write() {
        *w = config.clone();
    }
    // 同步环境变量，便于其他直接使用 reqwest::Client::new 的路径也走代理
    if let Some(cfg) = config {
        if cfg.enabled {
            if let (Some(host), Some(port)) = (cfg.host, cfg.port) {
                let scheme = cfg.scheme.unwrap_or_else(|| "http".to_string());
                let proxy_url = format!("{}://{}:{}", scheme, host, port);
                std::env::set_var("HTTP_PROXY", &proxy_url);
                std::env::set_var("HTTPS_PROXY", &proxy_url);
                std::env::set_var("ALL_PROXY", &proxy_url);
            }
            if let Some(no_proxy) = cfg.no_proxy {
                std::env::set_var("NO_PROXY", no_proxy);
            }
        } else {
            std::env::remove_var("HTTP_PROXY");
            std::env::remove_var("HTTPS_PROXY");
            std::env::remove_var("ALL_PROXY");
            std::env::remove_var("NO_PROXY");
        }
    } else {
        std::env::remove_var("HTTP_PROXY");
        std::env::remove_var("HTTPS_PROXY");
        std::env::remove_var("ALL_PROXY");
        std::env::remove_var("NO_PROXY");
    }
}

pub fn get_global_proxy() -> Option<ProxyConfig> {
    global_proxy_lock().read().ok().and_then(|g| g.clone())
}

pub fn build_client_with_global_proxy(timeout: Duration) -> Result<Client> {
    let mut builder = Client::builder().timeout(timeout);
    if let Some(cfg) = get_global_proxy() {
        if cfg.enabled {
            if let (Some(host), Some(port)) = (cfg.host.as_ref(), cfg.port) {
                let scheme = cfg.scheme.as_deref().unwrap_or("http");
                let proxy_url = format!("{}://{}:{}", scheme, host, port);
                tracing::info!("Configuring HTTP client with proxy: {}", proxy_url);
                let mut proxy = Proxy::all(&proxy_url)
                    .map_err(|e| AiAdapterError::ConfigurationError(e.to_string()))?;
                if let (Some(u), Some(p)) = (cfg.username.as_ref(), cfg.password.as_ref()) {
                    proxy = proxy.basic_auth(u, p);
                    tracing::info!("Using proxy authentication for user: {}", u);
                }
                builder = builder.proxy(proxy);
            } else {
                tracing::warn!("Proxy is enabled but host or port is missing");
            }
        } else {
            tracing::debug!("Proxy is disabled in configuration");
        }
    } else {
        tracing::debug!("No proxy configuration found");
    }
    builder
        .build()
        .map_err(|e| AiAdapterError::ConfigurationError(e.to_string()))
}

/// HTTP客户端
#[derive(Debug, Clone)]
pub struct HttpClient {
    client: Arc<Mutex<Client>>,
    default_headers: HashMap<String, String>,
    last_request: Arc<Mutex<Option<HttpRequest>>>,
    last_response: Arc<Mutex<Option<HttpResponse>>>,
    timeout: Duration,
    last_proxy_hash: Arc<Mutex<Option<u64>>>,
}

impl HttpClient {
    /// 创建新的HTTP客户端
    pub fn new(timeout: Duration) -> Result<Self> {
        let client: Client = build_client_with_global_proxy(timeout)?;
        
        let mut default_headers = HashMap::new();
        default_headers.insert("Content-Type".to_string(), "application/json".to_string());
        default_headers.insert("User-Agent".to_string(), "sentinel-ai/1.0".to_string());
        
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            default_headers,
            last_request: Arc::new(Mutex::new(None)),
            last_response: Arc::new(Mutex::new(None)),
            timeout,
            last_proxy_hash: Arc::new(Mutex::new(None)),
        })
    }
    
    /// 检查并更新代理配置
    async fn ensure_proxy_updated(&self) -> Result<()> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // 计算当前代理配置的哈希值
        let current_proxy_hash = {
            let proxy = get_global_proxy();
            let mut hasher = DefaultHasher::new();
            proxy.hash(&mut hasher);
            hasher.finish()
        };
        
        // 检查是否需要更新客户端
        let need_update = {
            if let Ok(last_hash) = self.last_proxy_hash.lock() {
                last_hash.map_or(true, |h| h != current_proxy_hash)
            } else {
                true
            }
        };
        
        if need_update {
            // 重新创建客户端
            let new_client = build_client_with_global_proxy(self.timeout)?;
            
            // 更新客户端和哈希值
            if let Ok(mut client) = self.client.lock() {
                *client = new_client;
            }
            if let Ok(mut hash) = self.last_proxy_hash.lock() {
                *hash = Some(current_proxy_hash);
            }
            
            tracing::debug!("HTTP client proxy configuration updated");
        }
        
        Ok(())
    }
    
    /// 设置默认头部
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.default_headers.extend(headers);
        self
    }
    
    /// 发送GET请求
    pub async fn get(
        &self,
        url: &str,
        headers: Option<HashMap<String, String>>,
    ) -> Result<Value> {
        // 确保代理配置是最新的
        self.ensure_proxy_updated().await?;
        
        let mut req_headers = self.default_headers.clone();
        if let Some(extra_headers) = headers {
            req_headers.extend(extra_headers);
        }
        
        // 记录请求信息
        let request_info = HttpRequest {
            method: "GET".to_string(),
            url: url.to_string(),
            headers: req_headers.clone(),
            body: None,
            timestamp: SystemTime::now(),
        };
        
        if let Ok(mut last_req) = self.last_request.lock() {
            *last_req = Some(request_info);
        }
        
        // 构建请求
        let client = {
            let client_guard = self.client.lock().map_err(|_| AiAdapterError::UnknownError("Failed to lock client".to_string()))?;
            client_guard.clone()
        };
        let mut request = client.get(url);
        
        for (key, value) in req_headers {
            request = request.header(&key, &value);
        }
        
        // 发送请求
        let start_time = SystemTime::now();
        let response = request.send().await?;
        let duration = start_time.elapsed().unwrap_or_default();
        
        // 记录响应信息
        let status = response.status().as_u16();
        let response_headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        
        let response_body = response.text().await?;
        
        let response_info = HttpResponse {
            status,
            headers: response_headers,
            body: Some(response_body.clone()),
            timestamp: SystemTime::now(),
            duration,
        };
        
        if let Ok(mut last_resp) = self.last_response.lock() {
            *last_resp = Some(response_info);
        }
        
        // 检查状态码
        if status >= 400 {
            return Err(self.handle_error_response(status, &response_body));
        }
        
        // 解析JSON响应
        serde_json::from_str(&response_body)
            .map_err(|e| AiAdapterError::DeserializationError(e.to_string()))
    }
    
    /// 发送POST请求
    pub async fn post_json<T: serde::Serialize>(
        &self,
        url: &str,
        body: &T,
        headers: Option<HashMap<String, String>>,
    ) -> Result<Value> {
        // 确保代理配置是最新的
        self.ensure_proxy_updated().await?;
        
        let body_str = serde_json::to_string(body)
            .map_err(|e| AiAdapterError::SerializationError(e.to_string()))?;
        
        let mut req_headers = self.default_headers.clone();
        if let Some(extra_headers) = headers {
            req_headers.extend(extra_headers);
        }
        
        // 记录请求信息
        let request_info = HttpRequest {
            method: "POST".to_string(),
            url: url.to_string(),
            headers: req_headers.clone(),
            body: Some(body_str.clone()),
            timestamp: SystemTime::now(),
        };
        
        if let Ok(mut last_req) = self.last_request.lock() {
            *last_req = Some(request_info);
        }
        
        // 构建请求
        let client = {
            let client_guard = self.client.lock().map_err(|_| AiAdapterError::UnknownError("Failed to lock client".to_string()))?;
            client_guard.clone()
        };
        let mut request = client.post(url);
        
        for (key, value) in req_headers {
            request = request.header(&key, &value);
        }
        
        request = request.body(body_str);
        
        // 发送请求
        let start_time = SystemTime::now();
        let response = request.send().await?;

        let duration = start_time.elapsed().unwrap_or_default();
        
        // 记录响应信息
        let status = response.status().as_u16();
        let response_headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        
        let response_body = response.text().await?;
        
        let response_info = HttpResponse {
            status,
            headers: response_headers,
            body: Some(response_body.clone()),
            timestamp: SystemTime::now(),
            duration,
        };
        
        if let Ok(mut last_resp) = self.last_response.lock() {
            *last_resp = Some(response_info);
        }
        
        // 检查状态码
        if status >= 400 {
            return Err(self.handle_error_response(status, &response_body));
        }
        
        // 解析JSON响应
        serde_json::from_str(&response_body)
            .map_err(|e| AiAdapterError::DeserializationError(e.to_string()))
    }
    
    /// 发送流式POST请求
    pub async fn post_stream<T: serde::Serialize>(
        &self,
        url: &str,
        body: &T,
        headers: Option<HashMap<String, String>>,
    ) -> Result<impl futures::Stream<Item = std::result::Result<bytes::Bytes, AiAdapterError>>> {
        // 确保代理配置是最新的
        self.ensure_proxy_updated().await?;
        
        let body_str = serde_json::to_string(body)
            .map_err(|e| AiAdapterError::SerializationError(e.to_string()))?;
        
        let mut req_headers = self.default_headers.clone();
        if let Some(extra_headers) = headers {
            req_headers.extend(extra_headers);
        }
        
        // 记录请求信息
        let request_info = HttpRequest {
            method: "POST".to_string(),
            url: url.to_string(),
            headers: req_headers.clone(),
            body: Some(body_str.clone()),
            timestamp: SystemTime::now(),
        };
        
        if let Ok(mut last_req) = self.last_request.lock() {
            *last_req = Some(request_info);
        }
        
        // 构建请求
        let client = {
            let client_guard = self.client.lock().map_err(|_| AiAdapterError::UnknownError("Failed to lock client".to_string()))?;
            client_guard.clone()
        };
        let mut request = client.post(url);
        
        for (key, value) in req_headers {
            request = request.header(&key, &value);
        }
        
        request = request.body(body_str);
        
        // 发送请求
        let response = request.send().await?;
        let status = response.status().as_u16();
        
        // 检查状态码
        if status >= 400 {
            let error_body = response.text().await.unwrap_or_default();
            return Err(self.handle_error_response(status, &error_body));
        }
        
        // 返回字节流
        use futures::StreamExt;
        let stream = response.bytes_stream().map(|result| {
            result.map_err(|e| AiAdapterError::NetworkError(e.to_string()))
        });
        
        Ok(stream)
    }
    
    /// 获取最后的请求信息
    pub fn last_request(&self) -> Option<HttpRequest> {
        self.last_request.lock().ok()?.clone()
    }
    
    /// 获取最后的响应信息
    pub fn last_response(&self) -> Option<HttpResponse> {
        self.last_response.lock().ok()?.clone()
    }
    
    /// 处理错误响应
    fn handle_error_response(&self, status: u16, body: &str) -> AiAdapterError {
        match status {
            401 => AiAdapterError::AuthenticationError(format!("Unauthorized: {}", body)),
            403 => AiAdapterError::AuthorizationError(format!("Forbidden: {}", body)),
            429 => AiAdapterError::RateLimitError(format!("Rate limit exceeded: {}", body)),
            400..=499 => AiAdapterError::ClientError(format!("Client error {}: {}", status, body)),
            500..=599 => AiAdapterError::ServerError(format!("Server error {}: {}", status, body)),
            _ => AiAdapterError::UnknownError(format!("HTTP error {}: {}", status, body)),
        }
    }
}

/// SSE事件结构
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event_type: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

/// SSE解析器
pub struct SseParser<S> {
    stream: S,
    buffer: String,
}

impl<S> SseParser<S>
where
    S: futures::Stream<Item = std::result::Result<bytes::Bytes, AiAdapterError>> + Unpin,
{
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            buffer: String::new(),
        }
    }
}

impl<S> futures::Stream for SseParser<S>
where
    S: futures::Stream<Item = std::result::Result<bytes::Bytes, AiAdapterError>> + Unpin,
{
    type Item = std::result::Result<SseEvent, AiAdapterError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::pin::Pin;

        loop {
            // 检查缓冲区中是否有完整的事件
            if let Some(event) = self.try_parse_event() {
                return std::task::Poll::Ready(Some(Ok(event)));
            }

            // 从底层流读取更多数据
            match Pin::new(&mut self.stream).poll_next(cx) {
                std::task::Poll::Ready(Some(Ok(chunk))) => {
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    self.buffer.push_str(&chunk_str);
                },
                std::task::Poll::Ready(Some(Err(e))) => {
                    return std::task::Poll::Ready(Some(Err(e)));
                },
                std::task::Poll::Ready(None) => {
                    // 流结束，处理剩余的缓冲区
                    if !self.buffer.is_empty() {
                        let remaining = std::mem::take(&mut self.buffer);
                        return std::task::Poll::Ready(Some(Ok(SseEvent {
                            event_type: None,
                            data: remaining,
                            id: None,
                            retry: None,
                        })));
                    }
                    return std::task::Poll::Ready(None);
                },
                std::task::Poll::Pending => {
                    return std::task::Poll::Pending;
                }
            }
        }
    }
}

impl<S> SseParser<S> {
    fn try_parse_event(&mut self) -> Option<SseEvent> {
        // 查找完整的事件（由空行分隔）
        if let Some(event_end) = self.buffer.find("\n\n") {
            let event_text = self.buffer.drain(..event_end + 2).collect::<String>();
            return Some(self.parse_event_text(&event_text));
        }
        
        // 检查单行事件（data: 行后跟换行符）
        let buffer_clone = self.buffer.clone();
        let lines: Vec<&str> = buffer_clone.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("data: ") {
                // 检查是否是完整的事件
                let next_line_is_empty = lines.get(i + 1).map_or(false, |l| l.is_empty());
                let is_last_line = i == lines.len() - 1 && self.buffer.ends_with('\n');
                
                if next_line_is_empty || is_last_line {
                    // 计算要移除的字符数
                    let mut char_count = 0;
                    let mut event_lines = Vec::new();
                    
                    for (j, &event_line) in lines.iter().enumerate() {
                        char_count += event_line.len() + 1; // +1 for newline
                        if j <= i {
                            event_lines.push(event_line.to_string());
                        } else {
                            break;
                        }
                    }
                    
                    if next_line_is_empty {
                        char_count += 1; // for the empty line
                    }
                    
                    // 现在可以安全地修改缓冲区
                    self.buffer.drain(..char_count);
                    let event_text = event_lines.join("\n") + "\n";
                    return Some(self.parse_event_text(&event_text));
                }
            }
        }
        
        None
    }
    
    fn parse_event_text(&self, text: &str) -> SseEvent {
        let mut event = SseEvent {
            event_type: None,
            data: String::new(),
            id: None,
            retry: None,
        };
        
        for line in text.lines() {
            if line.is_empty() {
                continue;
            }
            
            if line.starts_with("data: ") {
                let data = &line[6..];
                if !event.data.is_empty() {
                    event.data.push('\n');
                }
                event.data.push_str(data);
            } else if line.starts_with("event: ") {
                event.event_type = Some(line[7..].to_string());
            } else if line.starts_with("id: ") {
                event.id = Some(line[4..].to_string());
            } else if line.starts_with("retry: ") {
                if let Ok(retry) = line[7..].parse::<u64>() {
                    event.retry = Some(retry);
                }
            }
        }
        
        event
    }
}

/// 解析服务器发送事件(SSE)数据
pub fn parse_sse_line(line: &str) -> Option<String> {
    if line.starts_with("data: ") {
        let data = &line[6..]; // 跳过 "data: "
        if data == "[DONE]" {
            None
        } else {
            Some(data.to_string())
        }
    } else {
        None
    }
}

/// 解析流式响应
pub async fn parse_stream_response(
    stream: impl futures::Stream<Item = std::result::Result<bytes::Bytes, AiAdapterError>>,
) -> impl futures::Stream<Item = std::result::Result<Value, AiAdapterError>> {
    use futures::StreamExt;
    
    stream
        .map(|chunk_result| {
            chunk_result.and_then(|chunk| {
                let text = String::from_utf8_lossy(&chunk);
                let mut results = Vec::new();
                
                for line in text.lines() {
                    if let Some(data) = parse_sse_line(line) {
                        match serde_json::from_str::<Value>(&data) {
                            Ok(json) => results.push(json),
                            Err(e) => return Err(AiAdapterError::DeserializationError(e.to_string())),
                        }
                    }
                }
                
                Ok(results)
            })
        })
        .map(|result| {
            match result {
                Ok(jsons) => futures::stream::iter(jsons.into_iter().map(|json| Ok(json))).left_stream(),
                Err(e) => futures::stream::once(async move { Err(e) }).right_stream(),
            }
        })
        .flatten()
}
//! HTTP客户端模块
//! 
//! 提供统一的HTTP请求处理功能

use reqwest::{Client, Method, Request, Response};
use serde_json::Value;
use tokio_util::bytes;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use futures::StreamExt;

use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::types::{HttpRequest, HttpResponse};

/// HTTP客户端
#[derive(Debug, Clone)]
pub struct HttpClient {
    client: Client,
    default_headers: HashMap<String, String>,
    last_request: Arc<Mutex<Option<HttpRequest>>>,
    last_response: Arc<Mutex<Option<HttpResponse>>>,
}

impl HttpClient {
    /// 创建新的HTTP客户端
    pub fn new(timeout: Duration) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| AiAdapterError::ConfigurationError(e.to_string()))?;
        
        let mut default_headers = HashMap::new();
        default_headers.insert("Content-Type".to_string(), "application/json".to_string());
        default_headers.insert("User-Agent".to_string(), "sentinel-ai/1.0".to_string());
        
        Ok(Self {
            client,
            default_headers,
            last_request: Arc::new(Mutex::new(None)),
            last_response: Arc::new(Mutex::new(None)),
        })
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
        let mut request = self.client.get(url);
        
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
        let mut request = self.client.post(url);
        
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
        let mut request = self.client.post(url);
        
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
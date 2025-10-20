//! 基础提供商实现
//! 
//! 为所有AI提供商提供通用功能和默认实现

use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::ai_adapter::core::{RetryStrategy, DefaultRetryStrategy};
use crate::ai_adapter::types::*;
use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::http::HttpClient;
use crate::ai_adapter::utils;

/// 基础提供商结构
pub struct BaseProvider {
    pub name: String,
    pub version: String,
    pub config: ProviderConfig,
    pub http_client: HttpClient,
    pub retry_strategy: Box<dyn RetryStrategy>,
    pub last_request_info: Arc<Mutex<Option<HttpRequestInfo>>>,
    pub last_response_info: Arc<Mutex<Option<HttpResponseInfo>>>,
}

impl std::fmt::Debug for BaseProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BaseProvider")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("config", &self.config)
            .field("http_client", &self.http_client)
            .field("retry_strategy", &"<RetryStrategy>")
            .field("last_request_info", &"<Arc<Mutex<Option<HttpRequestInfo>>>>")
            .field("last_response_info", &"<Arc<Mutex<Option<HttpResponseInfo>>>>")
            .finish()
    }
}

impl BaseProvider {
    /// 创建新的基础提供商
    pub fn new(name: String, version: String, config: ProviderConfig) -> Result<Self> {
        // 验证配置
        Self::validate_config(&config)?;
        
        // 创建HTTP客户端
        let timeout = config.timeout.unwrap_or(Duration::from_secs(600));
        let http_client = HttpClient::new(timeout)?.with_provider_name(name.clone());
        
        // 创建重试策略
        let retry_strategy: Box<dyn RetryStrategy> = Box::new(DefaultRetryStrategy::default());
        
        Ok(Self {
            name,
            version,
            config,
            http_client,
            retry_strategy,
            last_request_info: Arc::new(Mutex::new(None)),
            last_response_info: Arc::new(Mutex::new(None)),
        })
    }
    
    /// 验证配置
    fn validate_config(config: &ProviderConfig) -> Result<()> {
        if config.name.is_empty() {
            return Err(AiAdapterError::ConfigurationError(
                "Provider name cannot be empty".to_string()
            ));
        }
        
        utils::validate_api_key(&config.api_key)?;
        
        Ok(())
    }
    
    /// 执行带重试的请求
    pub async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if !self.retry_strategy.should_retry(&error, attempt) {
                        return Err(error);
                    }
                    
                    attempt += 1;
                    let delay = self.retry_strategy.delay(attempt).await;
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    
    /// 构建基础请求头
    pub fn build_base_headers(&self) -> Result<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();
        
        // 添加Content-Type
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        
        // 添加User-Agent
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_str(&format!(
                "sentinel-ai-adapter/{} ({})",
                self.version,
                self.name
            )).map_err(|e| AiAdapterError::ConfigurationError(e.to_string()))?,
        );
        
        // 添加额外的头部
        if let Some(extra_headers) = &self.config.extra_headers {
            for (key, value) in extra_headers {
                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                    .map_err(|e| AiAdapterError::ConfigurationError(e.to_string()))?;
                let header_value = reqwest::header::HeaderValue::from_str(value)
                    .map_err(|e| AiAdapterError::ConfigurationError(e.to_string()))?;
                headers.insert(header_name, header_value);
            }
        }
        
        Ok(headers)
    }
    
    /// 获取API基础URL
    pub fn get_api_base(&self, default_base: &str) -> String {
        self.config.api_base.clone().unwrap_or_else(|| default_base.to_string())
    }
    
    /// 记录请求信息
    pub fn record_request_info(&self, info: HttpRequestInfo) {
        if let Ok(mut last_req) = self.last_request_info.lock() {
            *last_req = Some(info);
        }
    }
    
    /// 记录响应信息
    pub fn record_response_info(&self, info: HttpResponseInfo) {
        if let Ok(mut last_resp) = self.last_response_info.lock() {
            *last_resp = Some(info);
        }
    }
}

#[async_trait]
impl AiProvider for BaseProvider {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    fn supported_models(&self) -> Vec<String> {
        // 基础实现返回空列表，具体提供商应该重写此方法
        vec![]
    }
    
    fn build_chat_request(&self, request: &ChatRequest) -> Result<serde_json::Value> {
        // 基础实现：将消息转换为标准JSON格式
        let mut body = serde_json::json!({
            "model": request.model,
            "messages": request.messages
        });
        
        // 添加可选参数
        if let Some(options) = &request.options {
            if let Some(temperature) = options.temperature {
                body["temperature"] = serde_json::json!(temperature);
            }
            if let Some(max_tokens) = options.max_tokens {
                body["max_tokens"] = serde_json::json!(max_tokens);
            }
            if let Some(top_p) = options.top_p {
                body["top_p"] = serde_json::json!(top_p);
            }
            if let Some(stop) = &options.stop {
                body["stop"] = serde_json::json!(stop);
            }
        }
        
        // 添加工具
        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                body["tools"] = serde_json::json!(tools);
            }
        }
        
        Ok(body)
    }
    
    async fn test_connection(&self) -> Result<bool> {
        // 基础实现总是返回true，具体提供商应该重写此方法
        Ok(true)
    }
    
    async fn send_chat_request(&self, _request: &ChatRequest) -> Result<ChatResponse> {
        // 基础实现抛出错误，具体提供商必须实现此方法
        Err(AiAdapterError::UnknownError(
            "send_chat_request not implemented for base provider".to_string()
        ))
    }
    
    async fn send_chat_stream(&self, _request: &ChatRequest) -> Result<ChatStreamResponse> {
        // 基础实现抛出错误，具体提供商必须实现此方法
        Err(AiAdapterError::UnknownError(
            "send_chat_stream not implemented for base provider".to_string()
        ))
    }
    
    fn parse_stream(&self, _chunk: &str) -> Result<Option<StreamChunk>> {
        // 基础实现抛出错误，具体提供商必须实现此方法
        Err(AiAdapterError::UnknownError(
            "parse_stream not implemented for base provider".to_string()
        ))
    }
    
    fn get_last_request_info(&self) -> Option<HttpRequestInfo> {
        self.last_request_info.lock().ok()?.clone()
    }
    
    fn get_last_response_info(&self) -> Option<HttpResponseInfo> {
        self.last_response_info.lock().ok()?.clone()
    }
    
}
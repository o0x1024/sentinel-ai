//! AI适配器核心模块
//! 
//! 提供统一的AI服务接口和提供商管理

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::ai_adapter::types::*;
use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::http::HttpClient;

// Provider trait 已合并到 AiProvider trait 中，请使用 types::AiProvider

/// AI客户端
#[derive(Debug)]
pub struct AiClient {
    providers: Arc<RwLock<HashMap<String, Arc<dyn AiProvider>>>>,
    default_provider: Option<String>,
}

impl AiClient {
    /// 创建新的AI客户端
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            default_provider: None,
        }
    }
    
    /// 获取全局实例
    pub fn global() -> &'static Self {
        static INSTANCE: std::sync::OnceLock<AiClient> = std::sync::OnceLock::new();
        INSTANCE.get_or_init(|| Self::new())
    }
    
    /// 注册提供商
    pub fn register_provider(&mut self, provider: Arc<dyn AiProvider>) -> Result<()> {
        let name = provider.name().to_string();
        let mut providers = self.providers.write()
            .map_err(|_| AiAdapterError::UnknownError("Failed to acquire write lock".to_string()))?;
        providers.insert(name, provider);
        Ok(())
    }
    
    /// 设置默认提供商
    pub fn set_default_provider(&mut self, name: &str) -> Result<()> {
        let providers = self.providers.read()
            .map_err(|_| AiAdapterError::UnknownError("Failed to acquire read lock".to_string()))?;
        
        if !providers.contains_key(name) {
            return Err(AiAdapterError::ProviderNotFoundError(
                format!("Provider '{}' not found", name)
            ));
        }
        
        self.default_provider = Some(name.to_string());
        Ok(())
    }
    
    /// 发送聊天请求
    pub async fn chat(
        &self,
        provider_name: Option<&str>,
        request: ChatRequest,
    ) -> Result<ChatResponse> {
        let provider = self.get_provider(provider_name.unwrap_or_else(|| "deepseek"))?;
        provider.send_chat_request(&request).await
    }
    
    /// 发送流式聊天请求
    pub async fn chat_stream(
        &self,
        provider_name: Option<&str>,
        request: ChatRequest,
    ) -> Result<ChatStream> {
        let provider = self.get_provider(provider_name.unwrap_or_else(|| "deepseek"))?;
        
        // 注意：AiProvider trait 没有 supports_streaming 方法，直接尝试调用
        provider.send_chat_stream(&request).await
    }
    
    /// 获取提供商信息
    pub fn get_provider_info(&self, name: &str) -> Result<ProviderInfo> {
        let providers = self.providers.read()
            .map_err(|_| AiAdapterError::UnknownError("Failed to acquire read lock".to_string()))?;
        
        let provider = providers.get(name)
            .ok_or_else(|| AiAdapterError::ProviderNotFoundError(
                format!("Provider '{}' not found", name)
            ))?;
        
        Ok(ProviderInfo {
            name: provider.name().to_string(),
            version: provider.version().to_string(),
            models: provider.supported_models(),
            supports_streaming: true, // AiProvider 默认支持流式
            supports_tools: true, // AiProvider 默认支持工具
        })
    }
    
    /// 列出所有提供商
    pub fn list_providers(&self) -> Result<Vec<String>> {
        let providers = self.providers.read()
            .map_err(|_| AiAdapterError::UnknownError("Failed to acquire read lock".to_string()))?;
        Ok(providers.keys().cloned().collect())
    }
    
    /// 移除提供商
    pub fn remove_provider(&mut self, name: &str) -> Result<bool> {
        let mut providers = self.providers.write()
            .map_err(|_| AiAdapterError::UnknownError("Failed to acquire write lock".to_string()))?;
        
        let removed = providers.remove(name).is_some();
        
        // 如果移除的是默认提供商，清除默认设置
        if let Some(ref default) = self.default_provider {
            if default == name {
                self.default_provider = None;
            }
        }
        
        Ok(removed)
    }
    
    /// 测试提供商连接
    pub async fn test_provider(&self, name: &str) -> Result<bool> {
        let provider = self.get_provider_internal(Some(name))?;
        provider.test_connection().await
    }
    
    /// 获取提供商实例（公共方法）
    pub fn get_provider(&self, name: &str) -> Result<Arc<dyn AiProvider>> {
        let providers = self.providers.read()
            .map_err(|_| AiAdapterError::UnknownError("Failed to acquire read lock".to_string()))?;
        
        providers.get(name)
            .cloned()
            .ok_or_else(|| AiAdapterError::ProviderNotFoundError(
                format!("Provider '{}' not found", name)
            ))
    }
    
    /// 获取提供商实例（私有方法）
    fn get_provider_internal(&self, name: Option<&str>) -> Result<Arc<dyn AiProvider>> {
        let providers = self.providers.read()
            .map_err(|_| AiAdapterError::UnknownError("Failed to acquire read lock".to_string()))?;
        
        let provider_name = match name {
            Some(name) => name,
            None => {
                self.default_provider.as_ref()
                    .ok_or_else(|| AiAdapterError::ConfigurationError(
                        "No provider specified and no default provider set".to_string()
                    ))?
            }
        };
        
        providers.get(provider_name)
            .cloned()
            .ok_or_else(|| AiAdapterError::ProviderNotFoundError(
                format!("Provider '{}' not found", provider_name)
            ))
    }
}

impl Default for AiClient {
    fn default() -> Self {
        Self::new()
    }
}

/// 基础提供商实现
#[derive(Debug)]
pub struct BaseProvider {
    pub name: String,
    pub version: String,
    pub config: ProviderConfig,
    pub http_client: HttpClient,
    pub models: Vec<String>,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    last_request_info: Arc<std::sync::Mutex<Option<crate::ai_adapter::types::HttpRequest>>>,
    last_response_info: Arc<std::sync::Mutex<Option<crate::ai_adapter::types::HttpResponse>>>,
}

impl BaseProvider {
    /// 创建新的基础提供商
    pub fn new(
        name: String,
        version: String,
        config: ProviderConfig,
        models: Vec<String>,
        supports_streaming: bool,
        supports_tools: bool,
    ) -> Result<Self> {
        let timeout = Duration::from_secs(300);
        // let timeout = config.timeout.unwrap_or(Duration::from_secs(300));
        let http_client = HttpClient::new(timeout)?
            .with_headers(config.extra_headers.clone().unwrap_or_default());
        
        Ok(Self {
            name,
            version,
            config,
            http_client,
            models,
            supports_streaming,
            supports_tools,
            last_request_info: Arc::new(std::sync::Mutex::new(None)),
            last_response_info: Arc::new(std::sync::Mutex::new(None)),
        })
    }
    
    /// 构建认证头部
    pub fn build_auth_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), format!("Bearer {}", self.config.api_key));
        headers
    }
    
    /// 获取API基础URL
    pub fn get_api_base(&self, default_base: &str) -> String {
        self.config.api_base.clone().unwrap_or_else(|| default_base.to_string())
    }
    
    /// 执行带重试的操作
    pub async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let max_retries = self.config.max_retries.unwrap_or(3);
        let mut attempt = 0;
        
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if attempt >= max_retries || !self.should_retry(&error) {
                        return Err(error);
                    }
                    
                    attempt += 1;
                    let delay = Duration::from_millis(1000 * 2_u64.pow(attempt - 1));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    
    /// 判断是否应该重试
    fn should_retry(&self, error: &AiAdapterError) -> bool {
        matches!(
            error,
            AiAdapterError::NetworkError(_) |
            AiAdapterError::RateLimitError(_) |
            AiAdapterError::ServerError(_)
        )
    }
    
    /// 获取最后一次请求信息
    pub fn get_last_request_info(&self) -> Option<crate::ai_adapter::types::HttpRequest> {
        self.last_request_info.lock().ok()?.clone()
    }
    
    /// 获取最后一次响应信息
    pub fn get_last_response_info(&self) -> Option<crate::ai_adapter::types::HttpResponse> {
        self.last_response_info.lock().ok()?.clone()
    }
    
    /// 记录请求信息
    pub fn record_request_info(&self, info: crate::ai_adapter::types::HttpRequest) {
        if let Ok(mut last_req) = self.last_request_info.lock() {
            *last_req = Some(info);
        }
    }
    
    /// 记录响应信息
    pub fn record_response_info(&self, info: crate::ai_adapter::types::HttpResponse) {
        if let Ok(mut last_resp) = self.last_response_info.lock() {
            *last_resp = Some(info);
        }
    }
    
    /// 发送原始聊天请求
    pub async fn send_raw_chat_request(
        &self,
        _model: &str,
        _request: crate::ai_adapter::raw_message::RawChatRequest,
        _options: Option<&crate::ai_adapter::raw_message::RawChatOptions>,
    ) -> Result<crate::ai_adapter::raw_message::RawChatResponse> {
        Err(AiAdapterError::ProviderNotSupportedError(
            "Base provider does not implement send_raw_chat_request".to_string()
        ))
    }
    
    /// 发送原始流式聊天请求
    pub async fn send_raw_chat_stream(
        &self,
        _model: &str,
        _request: crate::ai_adapter::raw_message::RawChatRequest,
        _options: Option<&crate::ai_adapter::raw_message::RawChatOptions>,
    ) -> Result<crate::ai_adapter::raw_message::RawChatStreamResponse> {
        Err(AiAdapterError::ProviderNotSupportedError(
            "Base provider does not implement send_raw_chat_stream".to_string()
        ))
    }
}

// BaseProvider 的 Provider trait 实现已删除，请使用 AiProvider trait

/// 重试策略trait
#[async_trait]
pub trait RetryStrategy: Send + Sync {
    /// 计算重试延迟
    async fn delay(&self, attempt: u32) -> Duration;
    
    /// 判断是否应该重试
    fn should_retry(&self, error: &AiAdapterError, attempt: u32) -> bool;
    
    /// 最大重试次数
    fn max_retries(&self) -> u32;
}

/// 默认重试策略
#[derive(Debug, Clone)]
pub struct DefaultRetryStrategy {
    pub max_retries: u32,
    pub base_delay: Duration,
}

impl Default for DefaultRetryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(1000),
        }
    }
}

#[async_trait]
impl RetryStrategy for DefaultRetryStrategy {
    async fn delay(&self, attempt: u32) -> Duration {
        // 指数退避策略
        self.base_delay * 2_u32.pow(attempt.saturating_sub(1))
    }
    
    fn should_retry(&self, error: &AiAdapterError, attempt: u32) -> bool {
        if attempt >= self.max_retries {
            return false;
        }
        
        matches!(
            error,
            AiAdapterError::NetworkError(_) |
            AiAdapterError::RateLimitError(_) |
            AiAdapterError::ServerError(_)
        )
    }
    
    fn max_retries(&self) -> u32 {
        self.max_retries
    }
}

/// AI适配器管理器
#[derive(Debug)]
pub struct AiAdapterManager {
    client: Arc<RwLock<AiClient>>,
}

impl AiAdapterManager {
    /// 创建新的管理器
    pub fn new() -> Self {
        Self {
            client: Arc::new(RwLock::new(AiClient::new())),
        }
    }
    
    /// 获取全局实例
    pub fn global() -> &'static Self {
        static INSTANCE: std::sync::OnceLock<AiAdapterManager> = std::sync::OnceLock::new();
        INSTANCE.get_or_init(|| Self::new())
    }
    
    /// 注册提供商
    pub fn register_provider(&self, provider: Arc<dyn AiProvider>) -> Result<()> {
        let mut client = self.client.write()
            .map_err(|_| AiAdapterError::UnknownError("Failed to acquire write lock".to_string()))?;
        client.register_provider(provider)
    }
    
    /// 获取客户端
    pub fn get_client(&self) -> Result<Arc<RwLock<AiClient>>> {
        Ok(self.client.clone())
    }
    
    /// 获取提供商实例
    pub fn get_provider(&self, name: &str) -> Result<Arc<dyn AiProvider>> {
        let client = self.client.read()
            .map_err(|_| AiAdapterError::UnknownError("Failed to acquire read lock".to_string()))?;
        client.get_provider(name)
    }
    
    /// 列出所有提供商
    pub fn list_providers(&self) -> Result<Vec<String>> {
        let client = self.client.read()
            .map_err(|_| AiAdapterError::UnknownError("Failed to acquire read lock".to_string()))?;
        client.list_providers()
    }
    
    /// 测试提供商连接
    pub async fn test_provider(&self, name: &str) -> Result<bool> {
        let client = self.client.read()
            .map_err(|_| AiAdapterError::UnknownError("Failed to acquire read lock".to_string()))?;
        client.test_provider(name).await
    }
}

impl Default for AiAdapterManager {
    fn default() -> Self {
        Self::new()
    }
}
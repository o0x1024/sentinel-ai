//! 提供商适配器模块
//! 
//! 为不同的AI提供商提供统一的适配器接口

use async_trait::async_trait;


use crate::ai_adapter::types::*;
use crate::ai_adapter::error::{AiAdapterError, Result};

/// AI提供商适配器trait
#[async_trait]
pub trait AiProviderAdapter: Send + Sync {
    /// 获取提供商名称
    fn name(&self) -> &str;
    
    /// 获取提供商版本
    fn version(&self) -> &str;
    
    /// 获取支持的模型列表
    fn supported_models(&self) -> Vec<String>;
    
    /// 测试连接
    async fn test_connection(&self) -> Result<bool>;
    
    /// 发送聊天请求
    async fn send_chat_request(&self, request: &ChatRequest) -> Result<ChatResponse>;
    
    /// 发送流式聊天请求
    async fn send_chat_stream(&self, request: &ChatRequest) -> Result<ChatStreamResponse>;
    
    /// 获取最后一次请求信息
    fn get_last_request_info(&self) -> Option<HttpRequestInfo>;
    
    /// 获取最后一次响应信息
    fn get_last_response_info(&self) -> Option<HttpResponseInfo>;
    
    /// 发送原始聊天请求
    async fn send_raw_chat_request(
        &self,
        model: &str,
        request: ChatRequest,
        options: Option<&ChatOptions>,
    ) -> Result<ChatResponse>;
    
    /// 发送原始流式聊天请求
    async fn send_raw_chat_stream(
        &self,
        model: &str,
        request: ChatRequest,
        options: Option<&ChatOptions>,
    ) -> Result<ChatStreamResponse>;
}

/// 基础提供商适配器实现
#[derive(Debug)]
pub struct BaseProviderAdapter {
    pub name: String,
    pub version: String,
    pub config: ProviderConfig,
    pub models: Vec<String>,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    last_request: Option<HttpRequestInfo>,
    last_response: Option<HttpResponseInfo>,
}

impl BaseProviderAdapter {
    /// 创建新的基础适配器
    pub fn new(
        name: String,
        version: String,
        config: ProviderConfig,
        models: Vec<String>,
        supports_streaming: bool,
        supports_tools: bool,
    ) -> Self {
        Self {
            name,
            version,
            config,
            models,
            supports_streaming,
            supports_tools,
            last_request: None,
            last_response: None,
        }
    }
    
    /// 设置最后一次请求信息
    pub fn set_last_request(&mut self, request: HttpRequestInfo) {
        self.last_request = Some(request);
    }
    
    /// 设置最后一次响应信息
    pub fn set_last_response(&mut self, response: HttpResponseInfo) {
        self.last_response = Some(response);
    }
}

#[async_trait]
impl AiProviderAdapter for BaseProviderAdapter {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn version(&self) -> &str {
        &self.version
    }
    
    fn supported_models(&self) -> Vec<String> {
        self.models.clone()
    }
    
    async fn test_connection(&self) -> Result<bool> {
        // 基础实现，具体适配器应该重写
        Ok(true)
    }
    
    async fn send_chat_request(&self, _request: &ChatRequest) -> Result<ChatResponse> {
        Err(AiAdapterError::ProviderNotSupportedError(
            "Base adapter does not implement send_chat_request".to_string()
        ))
    }
    
    async fn send_chat_stream(&self, _request: &ChatRequest) -> Result<ChatStreamResponse> {
        Err(AiAdapterError::ProviderNotSupportedError(
            "Base adapter does not implement send_chat_stream".to_string()
        ))
    }
    
    fn get_last_request_info(&self) -> Option<HttpRequestInfo> {
        self.last_request.clone()
    }
    
    fn get_last_response_info(&self) -> Option<HttpResponseInfo> {
        self.last_response.clone()
    }
    
    async fn send_raw_chat_request(
        &self,
        _model: &str,
        _request: ChatRequest,
        _options: Option<&ChatOptions>,
    ) -> Result<ChatResponse> {
        Err(AiAdapterError::ProviderNotSupportedError(
            "Base adapter does not implement send_raw_chat_request".to_string()
        ))
    }
    
    async fn send_raw_chat_stream(
        &self,
        _model: &str,
        _request: ChatRequest,
        _options: Option<&ChatOptions>,
    ) -> Result<ChatStreamResponse> {
        Err(AiAdapterError::ProviderNotSupportedError(
            "Base adapter does not implement send_raw_chat_stream".to_string()
        ))
    }
}
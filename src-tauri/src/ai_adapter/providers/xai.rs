//! xAI提供商适配器

use async_trait::async_trait;
use crate::ai_adapter::types::AiProvider;
use crate::ai_adapter::types::*;
use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::providers::base::BaseProvider;
use crate::ai_adapter::raw_message::{RawChatRequest, RawChatResponse, RawChatStreamResponse, RawChatOptions};

/// xAI提供商
#[derive(Debug)]
pub struct XaiProvider {
    base: BaseProvider,
}

impl XaiProvider {
    /// 创建新的xAI提供商
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let base = BaseProvider::new(
            "xai".to_string(),
            "1.0.0".to_string(),
            config,
        )?;
        
        Ok(Self { base })
    }
}



#[async_trait]
impl AiProvider for XaiProvider {
    fn name(&self) -> &str {
        self.base.name()
    }
    
    fn version(&self) -> &str {
        self.base.version()
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "grok-beta".to_string(),
            "grok-vision-beta".to_string(),
        ]
    }
    
    async fn test_connection(&self) -> Result<bool> {
        // TODO: 实现xAI连接测试
        Ok(true)
    }
    
    async fn send_chat_request(&self, _request: &ChatRequest) -> Result<ChatResponse> {
        // TODO: 实现xAI聊天请求
        Err(AiAdapterError::UnknownError(
            "xAI provider not yet implemented".to_string()
        ))
    }
    
    async fn send_chat_stream(&self, _request: &ChatRequest) -> Result<ChatStream> {
        // TODO: 实现xAI流式响应
        Err(AiAdapterError::UnknownError(
            "xAI streaming not yet implemented".to_string()
        ))
    }
    
    fn get_last_request_info(&self) -> Option<HttpRequest> {
        self.base.get_last_request_info()
    }
    
    fn get_last_response_info(&self) -> Option<HttpResponse> {
        self.base.get_last_response_info()
    }
  
}
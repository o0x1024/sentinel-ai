//! Cohere提供商适配器

use async_trait::async_trait;
use crate::ai_adapter::types::AiProvider;
use crate::ai_adapter::types::*;
use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::providers::base::BaseProvider;
use crate::ai_adapter::raw_message::{RawChatRequest, RawChatResponse, RawChatStreamResponse, RawChatOptions};

/// Cohere提供商
#[derive(Debug)]
pub struct CohereProvider {
    base: BaseProvider,
}

impl CohereProvider {
    /// 创建新的Cohere提供商
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let base = BaseProvider::new(
            "cohere".to_string(),
            "1.0.0".to_string(),
            config,
        )?;
        
        Ok(Self { base })
    }
}

#[async_trait]
impl AiProvider for CohereProvider {
    fn name(&self) -> &str {
        self.base.name()
    }
    
    fn version(&self) -> &str {
        self.base.version()
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "command-r-plus".to_string(),
            "command-r".to_string(),
            "command".to_string(),
        ]
    }
    
    async fn test_connection(&self) -> Result<bool> {
        // TODO: 实现Cohere连接测试
        Ok(true)
    }
    

    
    fn get_last_request_info(&self) -> Option<HttpRequestInfo> {
        self.base.get_last_request_info()
    }
    
    fn get_last_response_info(&self) -> Option<HttpResponseInfo> {
        self.base.get_last_response_info()
    }
    
    async fn send_raw_chat_request(
        &self,
        prompt: &str,
        options: Option<&RawChatOptions>,
    ) -> Result<RawChatResponse> {
        self.base.send_raw_chat_request(model, request, options).await
    }
    
    async fn send_raw_chat_stream(
        &self,
        model: &str,
        prompt: &str,
        options: Option<&RawChatOptions>,
    ) -> Result<RawChatStreamResponse> {
        self.base.send_raw_chat_stream(model, request, options).await
    }
}
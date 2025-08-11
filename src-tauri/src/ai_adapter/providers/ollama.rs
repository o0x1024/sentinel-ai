//! Ollama提供商适配器

use async_trait::async_trait;
use crate::ai_adapter::types::AiProvider;
use crate::ai_adapter::types::*;
use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::providers::base::BaseProvider;
use crate::ai_adapter::raw_message::{RawChatRequest, RawChatResponse, RawChatStreamResponse, RawChatOptions};

/// Ollama提供商
#[derive(Debug)]
pub struct OllamaProvider {
    base: BaseProvider,
}

impl OllamaProvider {
    /// 创建新的Ollama提供商
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let base = BaseProvider::new(
            "ollama".to_string(),
            "1.0.0".to_string(),
            config,
        )?;
        
        Ok(Self { base })
    }
}

#[async_trait]
impl AiProvider for OllamaProvider {
    fn name(&self) -> &str {
        self.base.name()
    }
    
    fn version(&self) -> &str {
        self.base.version()
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "llama3.2".to_string(),
            "llama3.1".to_string(),
            "codellama".to_string(),
            "mistral".to_string(),
        ]
    }
    
    async fn test_connection(&self) -> Result<bool> {
        // TODO: 实现Ollama连接测试
        Ok(true)
    }
    
    async fn send_chat_request(&self, _request: &ChatRequest) -> Result<ChatResponse> {
        // TODO: 实现Ollama聊天请求
        Err(AiAdapterError::UnknownError(
            "Ollama provider not yet implemented".to_string()
        ))
    }
    
    async fn send_chat_stream(&self, _request: &ChatRequest) -> Result<ChatStreamResponse> {
        // TODO: 实现Ollama流式响应
        Err(AiAdapterError::UnknownError(
            "Ollama streaming not yet implemented".to_string()
        ))
    }
    
    fn get_last_request_info(&self) -> Option<HttpRequestInfo> {
        self.base.get_last_request_info()
    }
    
    fn get_last_response_info(&self) -> Option<HttpResponseInfo> {
        self.base.get_last_response_info()
    }
    
}
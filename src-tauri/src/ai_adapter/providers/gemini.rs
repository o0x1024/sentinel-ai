//! Gemini提供商适配器

use async_trait::async_trait;
use crate::ai_adapter::types::AiProvider;
use crate::ai_adapter::types::*;
use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::providers::base::BaseProvider;

/// Gemini提供商
#[derive(Debug)]
pub struct GeminiProvider {
    base: BaseProvider,
}

impl GeminiProvider {
    /// 创建新的Gemini提供商
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let base = BaseProvider::new(
            "gemini".to_string(),
            "1.0.0".to_string(),
            config,
        )?;
        
        Ok(Self { base })
    }
}

#[async_trait]
impl AiProvider for GeminiProvider {
    fn name(&self) -> &str {
        self.base.name()
    }
    
    fn version(&self) -> &str {
        self.base.version()
    }
    
    fn supported_models(&self) -> Vec<String> {
        vec![
            "gemini-1.5-pro".to_string(),
            "gemini-1.5-flash".to_string(),
            "gemini-pro".to_string(),
        ]
    }
    
    async fn test_connection(&self) -> Result<bool> {
        // TODO: 实现Gemini连接测试
        Ok(true)
    }

    fn build_chat_request(&self, request: &ChatRequest) -> Result<serde_json::Value> {
        // TODO: 实现Gemini聊天请求
        Err(AiAdapterError::UnknownError(
            "Gemini provider not yet implemented".to_string()
        ))
    }
    
    async fn send_chat_request(&self, _request: &ChatRequest) -> Result<ChatResponse> {
        // TODO: 实现Gemini聊天请求
        Err(AiAdapterError::UnknownError(
            "Gemini provider not yet implemented".to_string()
        ))
    }

    
    async fn send_chat_stream(&self, _request: &ChatRequest) -> Result<ChatStreamResponse> {
        // TODO: 实现Gemini流式响应
        Err(AiAdapterError::UnknownError(
            "Gemini streaming not yet implemented".to_string()
        ))
    }
    
    fn get_last_request_info(&self) -> Option<HttpRequestInfo> {
        self.base.get_last_request_info()
    }
    
    fn get_last_response_info(&self) -> Option<HttpResponseInfo> {
        self.base.get_last_response_info()
    }
    
    fn parse_stream(&self, chunk: &str) -> Result<Option<StreamChunk>> {
        // TODO: 实现Gemini流式解析
        // Gemini可能使用不同的流式格式，需要根据实际API文档实现
        for line in chunk.lines() {
            if line.starts_with("data: ") {
                let data = &line[6..]; // 移除"data: "前缀
                
                if data == "[DONE]" {
                    return Ok(None);
                }
                
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    // 这里需要根据Gemini的实际响应格式进行调整
                    let content = json.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string();
                    
                    let chunk = StreamChunk {
                        id: json.get("id").and_then(|i| i.as_str()).unwrap_or("unknown").to_string(),
                        model: "gemini".to_string(),
                        content,
                        finish_reason: None,
                        usage: None,
                    };
                    
                    return Ok(Some(chunk));
                }
            }
        }
        
        Ok(None)
    }
    
}
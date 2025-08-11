//! 原始消息类型定义
//! 
//! 用于与AI提供商进行低级别通信的原始消息类型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 原始聊天请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawChatRequest {
    pub messages: Vec<RawMessage>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub stream: Option<bool>,
    pub tools: Option<serde_json::Value>,
    pub tool_choice: Option<serde_json::Value>,
}

/// 原始消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawMessage {
    pub role: String,
    pub content: String,
    pub name: Option<String>,
    pub tool_calls: Option<serde_json::Value>,
    pub tool_call_id: Option<String>,
}

impl RawMessage {
    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
    
    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
    
    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

/// 原始聊天响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<RawChoice>,
    pub usage: Option<RawUsage>,
    pub system_fingerprint: Option<String>,
}

/// 原始选择项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawChoice {
    pub index: u32,
    pub message: RawMessage,
    pub finish_reason: Option<String>,
    pub logprobs: Option<serde_json::Value>,
}

/// 原始使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 原始流式响应
pub struct RawChatStreamResponse {
    pub stream: Box<dyn futures::Stream<Item = Result<RawStreamChunk, crate::ai_adapter::error::AiAdapterError>> + Send + Unpin>,
}

impl std::fmt::Debug for RawChatStreamResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawChatStreamResponse")
            .field("stream", &"<Stream>")
            .finish()
    }
}

/// 原始流式响应块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawStreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<RawStreamChoice>,
    pub usage: Option<RawUsage>,
}

/// 原始流式选择项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawStreamChoice {
    pub index: u32,
    pub delta: RawMessage,
    pub finish_reason: Option<String>,
    pub logprobs: Option<serde_json::Value>,
}

/// 原始聊天选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawChatOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub stream: Option<bool>,
    pub tools: Option<serde_json::Value>,
    pub tool_choice: Option<serde_json::Value>,
    pub extra_headers: Option<HashMap<String, String>>,
    pub timeout: Option<std::time::Duration>,
}

impl Default for RawChatOptions {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            max_tokens: Some(4096),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            stream: Some(false),
            tools: None,
            tool_choice: None,
            extra_headers: None,
            timeout: None,
        }
    }
}
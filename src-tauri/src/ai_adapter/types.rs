//! AI适配器类型定义
//! 
//! 重构后的简化类型系统，专注于实用性和易用性

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

// ===== 核心消息类型 =====

/// 聊天请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Option<Vec<Tool>>,
    pub tool_choice: Option<String>,
    /// 用户标识符，用于跟踪和审计
    pub user: Option<String>,
    /// 额外的提供商特定参数
    pub extra_params: Option<HashMap<String, serde_json::Value>>,
    /// 聊天选项配置
    pub options: Option<ChatOptions>,
}

/// 聊天响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub message: Message,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
    pub finish_reason: Option<String>,
    pub created_at: SystemTime,
}

/// 消息内容类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Text(String),
    Image(String), // base64 encoded image
    Mixed(Vec<ContentPart>),
}

/// 内容部分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPart {
    pub content_type: String,
    pub content: String,
}

/// 消息角色（重新导出）
pub use crate::models::ai::MessageRole;

/// 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn system(content: &str) -> Self {
        Self {
            role: MessageRole::System,
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
    
    pub fn user(content: &str) -> Self {
        Self {
            role: MessageRole::User,
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
    
    pub fn assistant(content: &str) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
    
    pub fn tool(content: &str, tool_call_id: &str) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
        }
    }
}

impl ChatRequest {
    /// 创建包含system prompt的聊天请求
    pub fn with_system_prompt(model: &str, system_prompt: &str, user_prompt: &str) -> Self {
        Self {
            model: model.to_string(),
            messages: vec![
                Message::system(system_prompt),
                Message::user(user_prompt),
            ],
            tools: None,
            tool_choice: None,
            user: None,
            extra_params: None,
            options: Some(ChatOptions::default()),
        }
    }
    
    /// 创建包含上下文消息的聊天请求
    pub fn with_context(model: &str, system_prompt: Option<&str>, user_prompt: &str, context_messages: &[Message]) -> Self {
        let mut messages = Vec::new();
        
        // 添加系统消息（如果有）
        if let Some(system) = system_prompt {
            messages.push(Message::system(system));
        }
        
        // 添加上下文消息
        messages.extend(context_messages.iter().cloned());
        
        // 添加用户消息
        messages.push(Message::user(user_prompt));
        
        Self {
            model: model.to_string(),
            messages,
            tools: None,
            tool_choice: None,
            user: None,
            extra_params: None,
            options: Some(ChatOptions::default()),
        }
    }
    
    /// 创建简单的用户消息请求
    pub fn simple(model: &str, user_prompt: &str) -> Self {
        Self {
            model: model.to_string(),
            messages: vec![Message::user(user_prompt)],
            tools: None,
            tool_choice: None,
            user: None,
            extra_params: None,
            options: Some(ChatOptions::default()),
        }
    }
}

// ===== 工具相关类型 =====

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub r#type: String,
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

// ===== 配置类型 =====

/// 聊天选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub stream: Option<bool>,
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            max_tokens: Some(4096),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            stream: Some(false),
        }
    }
}

/// 提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: String,
    pub api_base: Option<String>,
    pub api_version: Option<String>,
    pub timeout: Option<Duration>,
    pub max_retries: Option<u32>,
    pub extra_headers: Option<HashMap<String, String>>,
}

// ===== 使用统计 =====

/// 使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ===== HTTP相关类型 =====

/// HTTP请求信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: SystemTime,
}

/// HTTP响应信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: SystemTime,
    pub duration: Duration,
}

// ===== 流式响应类型 =====

/// 流式响应块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub model: String,
    pub content: String,
    pub finish_reason: Option<String>,
    pub usage: Option<Usage>,
}

/// 流式响应
pub struct ChatStream {
    pub stream: Box<dyn futures::Stream<Item = Result<StreamChunk, crate::ai_adapter::error::AiAdapterError>> + Send + Unpin>,
    pub request_info: Option<HttpRequest>,
    pub response_info: Option<HttpResponse>,
}

impl std::fmt::Debug for ChatStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatStream")
            .field("request_info", &self.request_info)
            .field("response_info", &self.response_info)
            .field("stream", &"<Stream>")
            .finish()
    }
}

// ===== 提供商信息 =====

/// 提供商信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub name: String,
    pub version: String,
    pub models: Vec<String>,
    pub supports_streaming: bool,
    pub supports_tools: bool,
}

// ===== 兼容性类型别名 =====

/// 流式聊天响应（兼容性别名）
pub type ChatStreamResponse = ChatStream;

/// HTTP请求信息（兼容性别名）
pub type HttpRequestInfo = HttpRequest;

/// HTTP响应信息（兼容性别名）
pub type HttpResponseInfo = HttpResponse;

/// 选择项（用于响应解析）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

// ===== AI提供商trait =====

/// AI提供商trait
#[async_trait::async_trait]
pub trait AiProvider: Send + Sync + std::fmt::Debug {
    /// 获取提供商名称
    fn name(&self) -> &str;
    
    /// 获取提供商版本
    fn version(&self) -> &str;
    
    /// 获取支持的模型列表
    fn supported_models(&self) -> Vec<String>;
    
    /// 检查是否支持流式响应
    fn supports_streaming(&self) -> bool {
        true // 默认支持流式响应
    }
    
    /// 测试连接
    async fn test_connection(&self) -> crate::ai_adapter::error::Result<bool>;
    
    /// 构建聊天请求 - 将通用ChatRequest转换为提供商特定格式
    fn build_chat_request(&self, request: &ChatRequest) -> crate::ai_adapter::error::Result<serde_json::Value>;
    
    /// 发送聊天请求（保留为兼容性方法，当不支持流式时使用）
    async fn send_chat_request(&self, request: &ChatRequest) -> crate::ai_adapter::error::Result<ChatResponse>;
    
    /// 发送流式聊天请求（现在作为主要方法）
    async fn send_chat_stream(&self, request: &ChatRequest) -> crate::ai_adapter::error::Result<ChatStreamResponse>;
    
    /// 解析流式响应块
    fn parse_stream(&self, chunk: &str) -> crate::ai_adapter::error::Result<Option<StreamChunk>>;
    
    /// 获取最后一次请求信息
    fn get_last_request_info(&self) -> Option<HttpRequestInfo>;
    
    /// 获取最后一次响应信息
    fn get_last_response_info(&self) -> Option<HttpResponseInfo>;
    
}
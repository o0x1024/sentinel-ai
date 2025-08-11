//! 错误处理模块

use serde::{Deserialize, Serialize};
use std::fmt;

/// AI适配器错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiAdapterError {
    /// 网络错误
    NetworkError(String),
    /// 认证错误
    AuthenticationError(String),
    /// 授权错误
    AuthorizationError(String),
    /// 速率限制错误
    RateLimitError(String),
    /// 服务器错误
    ServerError(String),
    /// 客户端错误
    ClientError(String),
    /// 配置错误
    ConfigurationError(String),
    /// 序列化错误
    SerializationError(String),
    /// 反序列化错误
    DeserializationError(String),
    /// 工具调用错误
    ToolCallError(String),
    /// 流处理错误
    StreamError(String),
    /// 超时错误
    TimeoutError(String),
    /// 模型不支持错误
    ModelNotSupportedError(String),
    /// 提供商不支持错误
    ProviderNotSupportedError(String),
    /// 提供商未找到错误
    ProviderNotFoundError(String),
    /// 参数验证错误
    ValidationError(String),
    /// 未知错误
    UnknownError(String),
}

impl fmt::Display for AiAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiAdapterError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            AiAdapterError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            AiAdapterError::AuthorizationError(msg) => write!(f, "Authorization error: {}", msg),
            AiAdapterError::RateLimitError(msg) => write!(f, "Rate limit error: {}", msg),
            AiAdapterError::ServerError(msg) => write!(f, "Server error: {}", msg),
            AiAdapterError::ClientError(msg) => write!(f, "Client error: {}", msg),
            AiAdapterError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            AiAdapterError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            AiAdapterError::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
            AiAdapterError::ToolCallError(msg) => write!(f, "Tool call error: {}", msg),
            AiAdapterError::StreamError(msg) => write!(f, "Stream error: {}", msg),
            AiAdapterError::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
            AiAdapterError::ModelNotSupportedError(msg) => write!(f, "Model not supported: {}", msg),
            AiAdapterError::ProviderNotSupportedError(msg) => write!(f, "Provider not supported: {}", msg),
            AiAdapterError::ProviderNotFoundError(msg) => write!(f, "Provider not found: {}", msg),
            AiAdapterError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AiAdapterError::UnknownError(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for AiAdapterError {}

/// 从reqwest错误转换
impl From<reqwest::Error> for AiAdapterError {
    fn from(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            AiAdapterError::TimeoutError(error.to_string())
        } else if error.is_connect() {
            AiAdapterError::NetworkError(error.to_string())
        } else if let Some(status) = error.status() {
            match status.as_u16() {
                401 => AiAdapterError::AuthenticationError(error.to_string()),
                403 => AiAdapterError::AuthorizationError(error.to_string()),
                429 => AiAdapterError::RateLimitError(error.to_string()),
                400..=499 => AiAdapterError::ClientError(error.to_string()),
                500..=599 => AiAdapterError::ServerError(error.to_string()),
                _ => AiAdapterError::UnknownError(error.to_string()),
            }
        } else {
            AiAdapterError::NetworkError(error.to_string())
        }
    }
}

/// 从serde_json错误转换
impl From<serde_json::Error> for AiAdapterError {
    fn from(error: serde_json::Error) -> Self {
        if error.is_syntax() || error.is_data() {
            AiAdapterError::DeserializationError(error.to_string())
        } else {
            AiAdapterError::SerializationError(error.to_string())
        }
    }
}

/// 从std::io::Error转换
impl From<std::io::Error> for AiAdapterError {
    fn from(error: std::io::Error) -> Self {
        AiAdapterError::NetworkError(error.to_string())
    }
}

/// 错误结果类型别名
pub type Result<T> = std::result::Result<T, AiAdapterError>;

/// 错误详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub error_type: String,
    pub message: String,
    pub code: Option<String>,
    pub details: Option<serde_json::Value>,
    pub timestamp: std::time::SystemTime,
    pub request_id: Option<String>,
}

impl ErrorDetails {
    pub fn new(error: &AiAdapterError) -> Self {
        Self {
            error_type: match error {
                AiAdapterError::NetworkError(_) => "network".to_string(),
                AiAdapterError::AuthenticationError(_) => "authentication".to_string(),
                AiAdapterError::AuthorizationError(_) => "authorization".to_string(),
                AiAdapterError::RateLimitError(_) => "rate_limit".to_string(),
                AiAdapterError::ServerError(_) => "server".to_string(),
                AiAdapterError::ClientError(_) => "client".to_string(),
                AiAdapterError::ConfigurationError(_) => "configuration".to_string(),
                AiAdapterError::SerializationError(_) => "serialization".to_string(),
                AiAdapterError::DeserializationError(_) => "deserialization".to_string(),
                AiAdapterError::ToolCallError(_) => "tool_call".to_string(),
                AiAdapterError::StreamError(_) => "stream".to_string(),
                AiAdapterError::TimeoutError(_) => "timeout".to_string(),
                AiAdapterError::ModelNotSupportedError(_) => "model_not_supported".to_string(),
                AiAdapterError::ProviderNotSupportedError(_) => "provider_not_supported".to_string(),
                AiAdapterError::ProviderNotFoundError(_) => "provider_not_found".to_string(),
                AiAdapterError::ValidationError(_) => "validation".to_string(),
                AiAdapterError::UnknownError(_) => "unknown".to_string(),
            },
            message: error.to_string(),
            code: None,
            details: None,
            timestamp: std::time::SystemTime::now(),
            request_id: None,
        }
    }
    
    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }
    
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
    
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
}
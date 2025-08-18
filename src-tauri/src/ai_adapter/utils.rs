//! 工具模块

use reqwest::Client;
use serde_json::Value;
use std::time::{Duration, SystemTime};

use crate::ai_adapter::error::{AiAdapterError, Result};

/// 创建HTTP客户端
pub fn create_http_client(timeout: Duration) -> Result<Client> {
    Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| AiAdapterError::ConfigurationError(e.to_string()))
}

/// JSON工具函数
pub mod json {
    use super::*;
    
    /// 安全地从JSON值中提取字符串
    pub fn extract_string(value: &Value, key: &str) -> Option<String> {
        value.get(key)?.as_str().map(|s| s.to_string())
    }
    
    /// 安全地从JSON值中提取数字
    pub fn extract_u64(value: &Value, key: &str) -> Option<u64> {
        value.get(key)?.as_u64()
    }
    
    /// 安全地从JSON值中提取布尔值
    pub fn extract_bool(value: &Value, key: &str) -> Option<bool> {
        value.get(key)?.as_bool()
    }
    
    /// 安全地从JSON值中提取数组
    pub fn extract_array<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
        value.get(key)?.as_array()
    }
    
    pub fn extract_object<'a>(value: &'a Value, key: &str) -> Option<&'a serde_json::Map<String, Value>> {
        value.get(key)?.as_object()
    }
}

/// 字符串工具函数
pub mod string {
    /// 截断字符串到指定长度
    pub fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }
    
    /// 清理字符串中的控制字符
    pub fn clean_control_chars(s: &str) -> String {
        s.chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .collect()
    }
    
    /// 转义JSON字符串
    pub fn escape_json_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }
}

/// 时间工具函数
pub mod time {
    use super::*;
    
    /// 获取当前Unix时间戳
    pub fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    
    /// 格式化持续时间
    pub fn format_duration(duration: Duration) -> String {
        let secs = duration.as_secs();
        let millis = duration.subsec_millis();
        
        if secs > 0 {
            format!("{}.{:03}s", secs, millis)
        } else {
            format!("{}ms", millis)
        }
    }
}

/// 验证API密钥格式
pub fn validate_api_key(api_key: &str) -> Result<()> {
    if api_key.is_empty() {
        return Err(AiAdapterError::ValidationError(
            "API key cannot be empty".to_string()
        ));
    }
    Ok(())
}

/// 验证模型名称
pub fn validate_model_name(model: &str) -> Result<()> {
    if model.is_empty() {
        return Err(AiAdapterError::ValidationError(
            "Model name cannot be empty".to_string()
        ));
    }
    Ok(())
}
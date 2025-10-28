//! 通用错误分类器和重连策略
//! 
//! 提供灵活的错误分类和恢复策略，支持配置化的错误处理

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;
use anyhow::Result;
use tracing::{debug, warn, info};

/// 错误分类
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// 连接相关错误
    Connection,
    /// 传输层错误  
    Transport,
    /// 序列化/反序列化错误
    Serialization,
    /// 超时错误
    Timeout,
    /// 资源不可用（如浏览器关闭）
    ResourceUnavailable,
    /// 权限/认证错误
    Authentication,
    /// 配置错误
    Configuration,
    /// 服务端内部错误
    ServerInternal,
    /// 不可恢复错误
    NonRecoverable,
    /// 未知错误
    Unknown,
}

/// 恢复策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// 立即重连
    ImmediateReconnect,
    /// 延迟重连
    DelayedReconnect { delay_ms: u64 },
    /// 指数退避重连
    ExponentialBackoff { initial_delay_ms: u64, max_delay_ms: u64, multiplier: f64 },
    /// 重新初始化
    Reinitialize,
    /// 不重试
    NoRetry,
    /// 自定义策略
    Custom(String),
}

/// 错误匹配规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMatchRule {
    /// 规则名称
    pub name: String,
    /// 错误代码匹配（可选）
    pub error_code: Option<i32>,
    /// 错误消息正则表达式
    pub message_pattern: Option<String>,
    /// 错误类型匹配
    pub error_type: Option<String>,
    /// 优先级（数字越大优先级越高）
    pub priority: u32,
    /// 错误分类
    pub category: ErrorCategory,
    /// 恢复策略
    pub recovery_strategy: RecoveryStrategy,
}

/// 错误上下文
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// 错误消息
    pub error_message: String,
    /// 错误代码（如果有）
    pub error_code: Option<i32>,
    /// 错误类型
    pub error_type: Option<String>,
    /// 工具名称
    pub tool_name: String,
    /// 连接名称
    pub connection_name: String,
    /// 重试次数
    pub retry_count: u32,
    /// 额外元数据
    pub metadata: HashMap<String, String>,
}

/// 错误分类器
#[derive(Debug, Clone)]
pub struct ErrorClassifier {
    /// 错误匹配规则
    rules: Vec<ErrorMatchRule>,
    /// 编译后的正则表达式缓存
    regex_cache: HashMap<String, Regex>,
}

impl ErrorClassifier {
    /// 创建默认的错误分类器
    pub fn new() -> Self {
        Self {
            rules: Self::create_default_rules(),
            regex_cache: HashMap::new(),
        }
    }
    
    /// 创建带自定义规则的分类器
    pub fn with_rules(rules: Vec<ErrorMatchRule>) -> Self {
        Self {
            rules,
            regex_cache: HashMap::new(),
        }
    }
    
    /// 分类错误并返回恢复策略
    pub fn classify_error(&mut self, context: &ErrorContext) -> (ErrorCategory, RecoveryStrategy) {
        debug!("Classifying error: {} for tool: {}", context.error_message, context.tool_name);
        
        // 按优先级排序规则
        let mut sorted_rules = self.rules.clone();
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        for rule in sorted_rules {
            if self.matches_rule(&rule, context) {
                info!(
                    "Error classified as {:?} using rule '{}' for tool '{}'", 
                    rule.category, rule.name, context.tool_name
                );
                return (rule.category, rule.recovery_strategy);
            }
        }
        
        warn!(
            "Error not classified, using default Unknown category for tool '{}'", 
            context.tool_name
        );
        (ErrorCategory::Unknown, RecoveryStrategy::NoRetry)
    }
    
    /// 检查规则是否匹配
    fn matches_rule(&mut self, rule: &ErrorMatchRule, context: &ErrorContext) -> bool {
        // 检查错误代码匹配
        if let Some(expected_code) = rule.error_code {
            if let Some(actual_code) = context.error_code {
                if expected_code != actual_code {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // 检查错误类型匹配
        if let Some(expected_type) = &rule.error_type {
            if let Some(actual_type) = &context.error_type {
                if expected_type != actual_type {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // 检查消息模式匹配
        if let Some(pattern) = &rule.message_pattern {
            let regex = self.get_or_compile_regex(pattern);
            match regex {
                Ok(re) => {
                    if !re.is_match(&context.error_message) {
                        return false;
                    }
                }
                Err(e) => {
                    warn!("Invalid regex pattern '{}' in rule '{}': {}", pattern, rule.name, e);
                    return false;
                }
            }
        }
        
        true
    }
    
    /// 获取或编译正则表达式
    fn get_or_compile_regex(&mut self, pattern: &str) -> Result<&Regex> {
        if !self.regex_cache.contains_key(pattern) {
            let regex = Regex::new(pattern)?;
            self.regex_cache.insert(pattern.to_string(), regex);
        }
        Ok(self.regex_cache.get(pattern).unwrap())
    }
    
    /// 添加自定义规则
    pub fn add_rule(&mut self, rule: ErrorMatchRule) {
        self.rules.push(rule);
    }
    
    /// 创建默认错误分类规则
    fn create_default_rules() -> Vec<ErrorMatchRule> {
        vec![
            // 浏览器相关错误（高优先级）
            ErrorMatchRule {
                name: "Browser Closed".to_string(),
                error_code: Some(-32603),
                message_pattern: Some(r"(?i)(target page|context|browser).*closed|page\.goto.*closed".to_string()),
                error_type: None,
                priority: 100,
                category: ErrorCategory::ResourceUnavailable,
                recovery_strategy: RecoveryStrategy::DelayedReconnect { delay_ms: 2000 },
            },
            
            // 传输层连接错误
            ErrorMatchRule {
                name: "Transport Connection".to_string(),
                error_code: None,
                message_pattern: Some(r"(?i)transport\s+(closed|error|failed)|connection\s+(closed|lost|failed)".to_string()),
                error_type: None,
                priority: 90,
                category: ErrorCategory::Transport,
                recovery_strategy: RecoveryStrategy::ExponentialBackoff { 
                    initial_delay_ms: 1000, 
                    max_delay_ms: 30000, 
                    multiplier: 2.0 
                },
            },
            
            // 序列化错误
            ErrorMatchRule {
                name: "Serialization Error".to_string(),
                error_code: None,
                message_pattern: Some(r"(?i)serde\s+error|serialization|deserialization|json\s+parse".to_string()),
                error_type: None,
                priority: 80,
                category: ErrorCategory::Serialization,
                recovery_strategy: RecoveryStrategy::ImmediateReconnect,
            },
            
            // 管道破损错误
            ErrorMatchRule {
                name: "Broken Pipe".to_string(),
                error_code: None,
                message_pattern: Some(r"(?i)broken\s+pipe|pipe\s+error".to_string()),
                error_type: None,
                priority: 85,
                category: ErrorCategory::Transport,
                recovery_strategy: RecoveryStrategy::DelayedReconnect { delay_ms: 1500 },
            },
            
            // 流读取错误
            ErrorMatchRule {
                name: "Stream Error".to_string(),
                error_code: None,
                message_pattern: Some(r"(?i)error\s+reading\s+from\s+stream|stream\s+error".to_string()),
                error_type: None,
                priority: 75,
                category: ErrorCategory::Transport,
                recovery_strategy: RecoveryStrategy::DelayedReconnect { delay_ms: 1000 },
            },
            
            // 超时错误
            ErrorMatchRule {
                name: "Timeout".to_string(),
                error_code: Some(-32003),
                message_pattern: Some(r"(?i)timeout|timed\s+out".to_string()),
                error_type: None,
                priority: 70,
                category: ErrorCategory::Timeout,
                recovery_strategy: RecoveryStrategy::DelayedReconnect { delay_ms: 2000 },
            },
            
            // 服务器内部错误
            ErrorMatchRule {
                name: "Internal Server Error".to_string(),
                error_code: Some(-32603),
                message_pattern: None,
                error_type: None,
                priority: 60,
                category: ErrorCategory::ServerInternal,
                recovery_strategy: RecoveryStrategy::DelayedReconnect { delay_ms: 3000 },
            },
            
            // 方法未找到
            ErrorMatchRule {
                name: "Method Not Found".to_string(),
                error_code: Some(-32601),
                message_pattern: None,
                error_type: None,
                priority: 50,
                category: ErrorCategory::NonRecoverable,
                recovery_strategy: RecoveryStrategy::NoRetry,
            },
            
            // 无效参数
            ErrorMatchRule {
                name: "Invalid Params".to_string(),
                error_code: Some(-32602),
                message_pattern: None,
                error_type: None,
                priority: 50,
                category: ErrorCategory::NonRecoverable,
                recovery_strategy: RecoveryStrategy::NoRetry,
            },
        ]
    }
}

impl Default for ErrorClassifier {
    fn default() -> Self {
        Self::new()
    }
}

/// 恢复策略执行器
pub struct RecoveryExecutor;

impl RecoveryExecutor {
    /// 计算重连延迟时间
    pub fn calculate_delay(strategy: &RecoveryStrategy, retry_count: u32) -> Option<u64> {
        match strategy {
            RecoveryStrategy::ImmediateReconnect => Some(0),
            RecoveryStrategy::DelayedReconnect { delay_ms } => Some(*delay_ms),
            RecoveryStrategy::ExponentialBackoff { initial_delay_ms, max_delay_ms, multiplier } => {
                let delay = (*initial_delay_ms as f64) * multiplier.powi(retry_count as i32);
                Some(delay.min(*max_delay_ms as f64) as u64)
            },
            RecoveryStrategy::Reinitialize => Some(1000), // 固定1秒延迟用于重新初始化
            RecoveryStrategy::NoRetry => None,
            RecoveryStrategy::Custom(_) => Some(1000), // 自定义策略默认1秒
        }
    }
    
    /// 判断是否应该重试
    pub fn should_retry(strategy: &RecoveryStrategy, retry_count: u32, max_retries: u32) -> bool {
        if retry_count >= max_retries {
            return false;
        }
        
        !matches!(strategy, RecoveryStrategy::NoRetry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_browser_error_classification() {
        let mut classifier = ErrorClassifier::new();
        let context = ErrorContext {
            error_message: "page.goto: Target page, context or browser has been closed".to_string(),
            error_code: Some(-32603),
            error_type: None,
            tool_name: "browser_navigate".to_string(),
            connection_name: "playwright".to_string(),
            retry_count: 0,
            metadata: HashMap::new(),
        };
        
        let (category, strategy) = classifier.classify_error(&context);
        assert_eq!(category, ErrorCategory::ResourceUnavailable);
        assert!(matches!(strategy, RecoveryStrategy::DelayedReconnect { .. }));
    }
    
    #[test]
    fn test_transport_error_classification() {
        let mut classifier = ErrorClassifier::new();
        let context = ErrorContext {
            error_message: "Transport closed unexpectedly".to_string(),
            error_code: None,
            error_type: None,
            tool_name: "test_tool".to_string(),
            connection_name: "test_connection".to_string(),
            retry_count: 0,
            metadata: HashMap::new(),
        };
        
        let (category, strategy) = classifier.classify_error(&context);
        assert_eq!(category, ErrorCategory::Transport);
        assert!(matches!(strategy, RecoveryStrategy::ExponentialBackoff { .. }));
    }
    
    #[test]
    fn test_recovery_delay_calculation() {
        let strategy = RecoveryStrategy::ExponentialBackoff { 
            initial_delay_ms: 1000, 
            max_delay_ms: 10000, 
            multiplier: 2.0 
        };
        
        assert_eq!(RecoveryExecutor::calculate_delay(&strategy, 0), Some(1000));
        assert_eq!(RecoveryExecutor::calculate_delay(&strategy, 1), Some(2000));
        assert_eq!(RecoveryExecutor::calculate_delay(&strategy, 2), Some(4000));
        assert_eq!(RecoveryExecutor::calculate_delay(&strategy, 5), Some(10000)); // 达到最大值
    }
}

//! Error classifier and recovery strategies (migrated)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;
use anyhow::Result;
use tracing::{debug, warn, info};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCategory {
    Connection,
    Transport,
    Serialization,
    Timeout,
    ResourceUnavailable,
    Authentication,
    Configuration,
    ServerInternal,
    NonRecoverable,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    ImmediateReconnect,
    DelayedReconnect { delay_ms: u64 },
    ExponentialBackoff { initial_delay_ms: u64, max_delay_ms: u64, multiplier: f64 },
    Reinitialize,
    NoRetry,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMatchRule {
    pub name: String,
    pub error_code: Option<i32>,
    pub message_pattern: Option<String>,
    pub error_type: Option<String>,
    pub priority: u32,
    pub category: ErrorCategory,
    pub recovery_strategy: RecoveryStrategy,
}

#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub error_message: String,
    pub error_code: Option<i32>,
    pub error_type: Option<String>,
    pub tool_name: String,
    pub connection_name: String,
    pub retry_count: u32,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ErrorClassifier {
    rules: Vec<ErrorMatchRule>,
    regex_cache: HashMap<String, Regex>,
}

impl ErrorClassifier {
    pub fn new() -> Self {
        Self { rules: Self::create_default_rules(), regex_cache: HashMap::new() }
    }

    pub fn with_rules(rules: Vec<ErrorMatchRule>) -> Self {
        Self { rules, regex_cache: HashMap::new() }
    }

    pub fn classify_error(&mut self, context: &ErrorContext) -> (ErrorCategory, RecoveryStrategy) {
        debug!("Classifying error: {} for tool: {}", context.error_message, context.tool_name);
        let mut sorted_rules = self.rules.clone();
        sorted_rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        for rule in sorted_rules {
            if self.matches_rule(&rule, context) {
                info!("Error classified as {:?} using rule '{}' for tool '{}'", rule.category, rule.name, context.tool_name);
                return (rule.category, rule.recovery_strategy);
            }
        }
        warn!("Error not classified, using default Unknown category for tool '{}'", context.tool_name);
        (ErrorCategory::Unknown, RecoveryStrategy::NoRetry)
    }

    fn matches_rule(&mut self, rule: &ErrorMatchRule, context: &ErrorContext) -> bool {
        if let Some(expected_code) = rule.error_code {
            if let Some(actual_code) = context.error_code { if expected_code != actual_code { return false; } } else { return false; }
        }
        if let Some(expected_type) = &rule.error_type { if let Some(actual_type) = &context.error_type { if expected_type != actual_type { return false; } } else { return false; } }
        if let Some(pattern) = &rule.message_pattern {
            let regex = self.get_or_compile_regex(pattern);
            match regex { Ok(re) => { if !re.is_match(&context.error_message) { return false; } } Err(e) => { warn!("Invalid regex pattern '{}' in rule '{}': {}", pattern, rule.name, e); return false; } }
        }
        true
    }

    fn get_or_compile_regex(&mut self, pattern: &str) -> Result<&Regex> {
        if !self.regex_cache.contains_key(pattern) {
            let regex = Regex::new(pattern)?;
            self.regex_cache.insert(pattern.to_string(), regex);
        }
        Ok(self.regex_cache.get(pattern).unwrap())
    }

    pub fn add_rule(&mut self, rule: ErrorMatchRule) { self.rules.push(rule); }

    fn create_default_rules() -> Vec<ErrorMatchRule> {
        vec![
            ErrorMatchRule { name: "Browser Closed".to_string(), error_code: Some(-32603), message_pattern: Some(r"(?i)(target page|context|browser).*closed|page\.goto.*closed".to_string()), error_type: None, priority: 100, category: ErrorCategory::ResourceUnavailable, recovery_strategy: RecoveryStrategy::DelayedReconnect { delay_ms: 2000 } },
            ErrorMatchRule { name: "Transport Connection".to_string(), error_code: None, message_pattern: Some(r"(?i)transport\s+(closed|error|failed)|connection\s+(closed|lost|failed)".to_string()), error_type: None, priority: 90, category: ErrorCategory::Transport, recovery_strategy: RecoveryStrategy::ExponentialBackoff { initial_delay_ms: 1000, max_delay_ms: 30000, multiplier: 2.0 } },
            ErrorMatchRule { name: "Serialization Error".to_string(), error_code: None, message_pattern: Some(r"(?i)serde\s+error|serialization|deserialization|json\s+parse".to_string()), error_type: None, priority: 80, category: ErrorCategory::Serialization, recovery_strategy: RecoveryStrategy::ImmediateReconnect },
            ErrorMatchRule { name: "Broken Pipe".to_string(), error_code: None, message_pattern: Some(r"(?i)broken\s+pipe|pipe\s+error".to_string()), error_type: None, priority: 85, category: ErrorCategory::Transport, recovery_strategy: RecoveryStrategy::DelayedReconnect { delay_ms: 1500 } },
            ErrorMatchRule { name: "Stream Error".to_string(), error_code: None, message_pattern: Some(r"(?i)error\s+reading\s+from\s+stream|stream\s+error".to_string()), error_type: None, priority: 75, category: ErrorCategory::Transport, recovery_strategy: RecoveryStrategy::DelayedReconnect { delay_ms: 1000 } },
            ErrorMatchRule { name: "Timeout".to_string(), error_code: Some(-32003), message_pattern: Some(r"(?i)timeout|timed\s+out".to_string()), error_type: None, priority: 70, category: ErrorCategory::Timeout, recovery_strategy: RecoveryStrategy::DelayedReconnect { delay_ms: 2000 } },
            ErrorMatchRule { name: "Internal Server Error".to_string(), error_code: Some(-32603), message_pattern: None, error_type: None, priority: 60, category: ErrorCategory::ServerInternal, recovery_strategy: RecoveryStrategy::DelayedReconnect { delay_ms: 3000 } },
            ErrorMatchRule { name: "Method Not Found".to_string(), error_code: Some(-32601), message_pattern: None, error_type: None, priority: 50, category: ErrorCategory::NonRecoverable, recovery_strategy: RecoveryStrategy::NoRetry },
            ErrorMatchRule { name: "Invalid Params".to_string(), error_code: Some(-32602), message_pattern: None, error_type: None, priority: 50, category: ErrorCategory::NonRecoverable, recovery_strategy: RecoveryStrategy::NoRetry },
        ]
    }
}

impl Default for ErrorClassifier { fn default() -> Self { Self::new() } }

pub struct RecoveryExecutor;

impl RecoveryExecutor {
    pub fn calculate_delay(strategy: &RecoveryStrategy, retry_count: u32) -> Option<u64> {
        match strategy {
            RecoveryStrategy::ImmediateReconnect => Some(0),
            RecoveryStrategy::DelayedReconnect { delay_ms } => Some(*delay_ms),
            RecoveryStrategy::ExponentialBackoff { initial_delay_ms, max_delay_ms, multiplier } => {
                let delay = (*initial_delay_ms as f64) * multiplier.powi(retry_count as i32);
                Some(delay.min(*max_delay_ms as f64) as u64)
            }
            RecoveryStrategy::Reinitialize => Some(1000),
            RecoveryStrategy::NoRetry => None,
            RecoveryStrategy::Custom(_) => Some(1000),
        }
    }

    pub fn should_retry(strategy: &RecoveryStrategy, retry_count: u32, max_retries: u32) -> bool {
        if retry_count >= max_retries { return false; }
        !matches!(strategy, RecoveryStrategy::NoRetry)
    }
}



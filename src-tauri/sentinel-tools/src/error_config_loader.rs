//! Error classification config loader (migrated)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, Context};
use tracing::{info, warn, debug};

use crate::error_classifier::{ErrorMatchRule, ErrorCategory, RecoveryStrategy, ErrorClassifier};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ConfigRecoveryStrategy {
    Simple(String),
    DelayedReconnect { delay_ms: u64 },
    ExponentialBackoff { initial_delay_ms: u64, max_delay_ms: u64, multiplier: f64 },
}

impl From<ConfigRecoveryStrategy> for RecoveryStrategy {
    fn from(config_strategy: ConfigRecoveryStrategy) -> Self {
        match config_strategy {
            ConfigRecoveryStrategy::Simple(s) => match s.as_str() {
                "ImmediateReconnect" => RecoveryStrategy::ImmediateReconnect,
                "Reinitialize" => RecoveryStrategy::Reinitialize,
                "NoRetry" => RecoveryStrategy::NoRetry,
                custom => RecoveryStrategy::Custom(custom.to_string()),
            },
            ConfigRecoveryStrategy::DelayedReconnect { delay_ms } => RecoveryStrategy::DelayedReconnect { delay_ms },
            ConfigRecoveryStrategy::ExponentialBackoff { initial_delay_ms, max_delay_ms, multiplier } => RecoveryStrategy::ExponentialBackoff { initial_delay_ms, max_delay_ms, multiplier },
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigRule {
    pub name: String,
    pub priority: u32,
    pub error_code: Option<i32>,
    pub message_pattern: Option<String>,
    pub error_type: Option<String>,
    pub category: String,
    pub recovery_strategy: ConfigRecoveryStrategy,
}

impl TryFrom<ConfigRule> for ErrorMatchRule {
    type Error = anyhow::Error;
    fn try_from(config_rule: ConfigRule) -> Result<Self> {
        let category = match config_rule.category.as_str() {
            "Connection" => ErrorCategory::Connection,
            "Transport" => ErrorCategory::Transport,
            "Serialization" => ErrorCategory::Serialization,
            "Timeout" => ErrorCategory::Timeout,
            "ResourceUnavailable" => ErrorCategory::ResourceUnavailable,
            "Authentication" => ErrorCategory::Authentication,
            "Configuration" => ErrorCategory::Configuration,
            "ServerInternal" => ErrorCategory::ServerInternal,
            "NonRecoverable" => ErrorCategory::NonRecoverable,
            "Unknown" => ErrorCategory::Unknown,
            other => return Err(anyhow::anyhow!("Unknown error category: {}", other)),
        };
        Ok(ErrorMatchRule { name: config_rule.name, error_code: config_rule.error_code, message_pattern: config_rule.message_pattern, error_type: config_rule.error_type, priority: config_rule.priority, category, recovery_strategy: config_rule.recovery_strategy.into() })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolSpecificConfig { pub max_retries: Option<u32>, pub rules: Vec<ConfigRule> }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalConfig { pub max_retries: u32, pub default_timeout_ms: u64 }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ErrorConfig { pub global: GlobalConfig, pub rules: Vec<ConfigRule>, pub tool_specific: HashMap<String, ToolSpecificConfig> }

pub struct ErrorConfigLoader;

impl ErrorConfigLoader {
    pub fn load_from_toml_file(path: &str) -> Result<ErrorConfig> {
        let content = std::fs::read_to_string(path).with_context(|| format!("Failed to read config file: {}", path))?;
        Self::load_from_toml(&content)
    }
    pub fn load_from_toml(content: &str) -> Result<ErrorConfig> { toml::from_str(content).with_context(|| "Failed to parse TOML configuration") }
    pub fn load_from_json_file(path: &str) -> Result<ErrorConfig> { let content = std::fs::read_to_string(path).with_context(|| format!("Failed to read config file: {}", path))?; Self::load_from_json(&content) }
    pub fn load_from_json(content: &str) -> Result<ErrorConfig> { serde_json::from_str(content).with_context(|| "Failed to parse JSON configuration") }

    pub fn create_classifier_from_config(config: &ErrorConfig) -> Result<ErrorClassifier> {
        let mut rules = Vec::new();
        for config_rule in &config.rules {
            match ErrorMatchRule::try_from(config_rule.clone()) { Ok(rule) => rules.push(rule), Err(e) => { warn!("Failed to parse rule '{}': {}", config_rule.name, e); continue; } }
        }
        for (tool_name, tool_config) in &config.tool_specific {
            for config_rule in &tool_config.rules {
                match ErrorMatchRule::try_from(config_rule.clone()) { Ok(rule) => { debug!("Added tool-specific rule '{}' for tool '{}'", rule.name, tool_name); rules.push(rule); }, Err(e) => { warn!("Failed to parse tool-specific rule '{}' for tool '{}': {}", config_rule.name, tool_name, e); continue; } }
            }
        }
        info!("Loaded {} error classification rules from configuration", rules.len());
        Ok(ErrorClassifier::with_rules(rules))
    }

    pub fn get_tool_max_retries(config: &ErrorConfig, tool_name: &str) -> u32 {
        config.tool_specific.get(tool_name).and_then(|tool_config| tool_config.max_retries).unwrap_or(config.global.max_retries)
    }

    pub fn create_default_config() -> ErrorConfig {
        ErrorConfig { global: GlobalConfig { max_retries: 3, default_timeout_ms: 30000 }, rules: vec![
            ConfigRule { name: "Browser Closed".to_string(), priority: 100, error_code: Some(-32603), message_pattern: Some(r"(?i)(target page|context|browser).*closed|page\.goto.*closed".to_string()), error_type: None, category: "ResourceUnavailable".to_string(), recovery_strategy: ConfigRecoveryStrategy::DelayedReconnect { delay_ms: 2000 } },
            ConfigRule { name: "Transport Error".to_string(), priority: 90, error_code: None, message_pattern: Some(r"(?i)transport\s+(closed|error|failed)|connection\s+(closed|lost|failed)".to_string()), error_type: None, category: "Transport".to_string(), recovery_strategy: ConfigRecoveryStrategy::ExponentialBackoff { initial_delay_ms: 1000, max_delay_ms: 30000, multiplier: 2.0 } },
            ConfigRule { name: "Serialization Error".to_string(), priority: 80, error_code: None, message_pattern: Some(r"(?i)serde\s+error|serialization|deserialization|json\s+parse".to_string()), error_type: None, category: "Serialization".to_string(), recovery_strategy: ConfigRecoveryStrategy::Simple("ImmediateReconnect".to_string()) },
        ], tool_specific: HashMap::new() }
    }
}



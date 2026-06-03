//! 流量分析核心类型定义
use chrono::{DateTime, Utc};
pub use sentinel_plugins::{
    Confidence, Finding, PluginMetadata, RequestContext, ResponseContext, Severity,
};
use serde::{Deserialize, Serialize};

use crate::InterceptFilterRule;

pub const INTERCEPT_FILTER_RULES_CONFIG_KEY: &str = "intercept_filter_rules";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptRuleConfig {
    pub id: String,
    pub rule_type: String,
    pub match_type: String,
    pub relationship: String,
    pub condition: String,
    pub action: String,
    #[serde(default = "default_intercept_rule_enabled")]
    pub enabled: bool,
}

impl InterceptRuleConfig {
    pub fn with_new_id(mut self) -> Self {
        self.id = uuid::Uuid::new_v4().to_string();
        self
    }

    pub fn from_runtime_rule(rule_type: impl Into<String>, rule: &RuntimeInterceptRule) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            rule_type: rule_type.into(),
            match_type: rule.match_type.clone(),
            relationship: rule.relationship.clone(),
            condition: rule.condition.clone(),
            action: "exclude".to_string(),
            enabled: rule.enabled,
        }
    }

    pub fn to_runtime_rule(&self) -> InterceptFilterRule {
        InterceptFilterRule {
            enabled: self.enabled,
            operator: "And".to_string(),
            match_type: self.match_type.clone(),
            relationship: self.relationship.clone(),
            condition: self.condition.clone(),
        }
    }
}

fn default_intercept_rule_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InterceptRuleConfigSet {
    pub rules: Vec<InterceptRuleConfig>,
}

impl InterceptRuleConfigSet {
    pub fn runtime_rules_for_type(&self, rule_type: &str) -> Vec<InterceptFilterRule> {
        self.rules
            .iter()
            .filter(|rule| rule.rule_type == rule_type)
            .map(InterceptRuleConfig::to_runtime_rule)
            .collect()
    }

    pub fn replace_runtime_rules(&mut self, rule_type: &str, rules: &[RuntimeInterceptRule]) {
        self.rules.retain(|rule| rule.rule_type != rule_type);
        self.rules.extend(
            rules
                .iter()
                .map(|rule| InterceptRuleConfig::from_runtime_rule(rule_type, rule)),
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInterceptRule {
    pub enabled: bool,
    pub operator: String,
    pub match_type: String,
    pub relationship: String,
    pub condition: String,
}

impl From<&RuntimeInterceptRule> for InterceptFilterRule {
    fn from(rule: &RuntimeInterceptRule) -> Self {
        Self {
            enabled: rule.enabled,
            operator: rule.operator.clone(),
            match_type: rule.match_type.clone(),
            relationship: rule.relationship.clone(),
            condition: rule.condition.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptedRequest {
    pub id: String,
    pub method: String,
    pub url: String,
    pub path: String,
    pub protocol: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptedResponse {
    pub id: String,
    pub request_id: String,
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStatus {
    pub running: bool,
    pub port: u16,
    pub mitm_enabled: bool,
    pub stats: ProxyStats,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProxyStats {
    pub http_requests: u64,
    pub https_requests: u64,
    pub errors: u64,
    pub qps: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSession {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub enabled_plugins: Vec<String>,
    pub http_total: u64,
    pub https_total: u64,
    pub findings_total: u64,
}

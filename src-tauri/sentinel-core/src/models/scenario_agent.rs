use serde::{Deserialize, Serialize};
use serde::de::{self, Deserializer};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEngine {
    #[serde(rename = "travel")] Travel,
    #[serde(rename = "plan-execute")] PlanExecute,
    #[serde(rename = "react")] React,
    #[serde(rename = "rewoo")] Rewoo,
    #[serde(rename = "llm-compiler")] LlmCompiler,
    #[serde(rename = "auto")] Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmDefaultConfig {
    pub provider: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig { pub default: LlmDefaultConfig }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptOverrides {
    pub system: Option<String>,
    pub planner: Option<String>,
    pub executor: Option<String>,
    pub replanner: Option<String>,
    pub evaluator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptIds {
    pub system: Option<i64>,
    pub planner: Option<i64>,
    pub executor: Option<i64>,
    pub replanner: Option<i64>,
    pub evaluator: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsPolicy { pub allow: Vec<String>, pub deny: Option<Vec<String>> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub backoff: String,
    #[serde(default, deserialize_with = "deserialize_option_u32_from_number")]
    pub interval_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    #[serde(default, deserialize_with = "deserialize_option_u64_from_number")]
    pub timeout_sec: Option<u64>,
    pub retry: RetryPolicy,
    pub concurrency: Option<u32>,
    pub strict_mode: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioAgentProfile {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub version: Option<String>,
    pub engine: AgentEngine,
    pub llm: LlmConfig,
    pub prompts: PromptOverrides,
    pub prompt_ids: Option<PromptIds>,
    pub prompt_strategy: Option<String>,
    pub group_id: Option<i64>,
    pub pinned_versions: Option<HashMap<i64, String>>,
    pub tools: ToolsPolicy,
    pub execution: ExecutionPolicy,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

fn deserialize_option_u64_from_number<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<JsonValue>::deserialize(deserializer)?;
    match v {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Number(n)) => {
            if let Some(u) = n.as_u64() {
                Ok(Some(u))
            } else if let Some(f) = n.as_f64() {
                Ok(Some(f as u64))
            } else {
                Err(de::Error::custom("invalid number for u64"))
            }
        }
        Some(JsonValue::String(s)) => {
            if let Ok(u) = s.parse::<u64>() {
                Ok(Some(u))
            } else if let Ok(f) = s.parse::<f64>() {
                Ok(Some(f as u64))
            } else {
                Err(de::Error::custom("invalid string for u64"))
            }
        }
        _ => Err(de::Error::custom("invalid type for option u64")),
    }
}

fn deserialize_option_u32_from_number<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<JsonValue>::deserialize(deserializer)?;
    match v {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Number(n)) => {
            if let Some(u) = n.as_u64() {
                Ok(Some(u as u32))
            } else if let Some(f) = n.as_f64() {
                Ok(Some(f as u32))
            } else {
                Err(de::Error::custom("invalid number for u32"))
            }
        }
        Some(JsonValue::String(s)) => {
            if let Ok(u) = s.parse::<u64>() {
                Ok(Some(u as u32))
            } else if let Ok(f) = s.parse::<f64>() {
                Ok(Some(f as u32))
            } else {
                Err(de::Error::custom("invalid string for u32"))
            }
        }
        _ => Err(de::Error::custom("invalid type for option u32")),
    }
}

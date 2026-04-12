use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAgentEvent {
    pub event_name: String,
    pub payload: Value,
    pub source: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAgentRunUpdateEvent {
    pub run_id: String,
    pub profile_id: String,
    pub status: String,
    pub trigger_event: Option<String>,
    pub tool_calls: Option<Value>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAgentDispatchResult {
    pub event_name: String,
    pub matched_profiles: usize,
    pub scheduled_run_ids: Vec<String>,
}

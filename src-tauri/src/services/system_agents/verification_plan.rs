use serde::{Deserialize, Serialize};
use serde_json::Value;

use sentinel_db::{ProxyRequestRecord, TrafficEvidenceRecord};

const SYSTEM_AGENT_CONTEXT_LOCATION: &str = "system_agent_context";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VerificationPlan {
    #[serde(default = "default_strategy")]
    pub preferred_strategy: String,
    #[serde(default)]
    pub target_request_id: Option<i64>,
    #[serde(default)]
    pub candidate_parameters: Vec<String>,
    #[serde(default)]
    pub replay_count: Option<u32>,
    #[serde(default)]
    pub concurrent_requests: Option<u32>,
    #[serde(default)]
    pub sequence_request_ids: Vec<i64>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct VerificationBaseline {
    pub source_request_id: Option<i64>,
    pub url: String,
    pub method: String,
    pub request_headers: Option<String>,
    pub request_body: Option<String>,
    pub response_status: Option<i32>,
    pub response_headers: Option<String>,
    pub response_body: Option<String>,
}

fn default_strategy() -> String {
    "replay_as_is".to_string()
}

pub fn extract_verification_plan(output: &Value) -> Option<VerificationPlan> {
    let raw = output.get("verificationPlan")?;
    let mut plan = serde_json::from_value::<VerificationPlan>(raw.clone()).ok()?;
    if plan.preferred_strategy.trim().is_empty() {
        plan.preferred_strategy = default_strategy();
    }
    Some(plan)
}

pub fn extract_context_output(evidence: &[TrafficEvidenceRecord]) -> Option<Value> {
    evidence
        .iter()
        .find(|item| item.location == SYSTEM_AGENT_CONTEXT_LOCATION)
        .and_then(|item| item.response_body.as_deref())
        .and_then(|body| serde_json::from_str::<Value>(body).ok())
}

pub fn extract_context_payload(evidence: &[TrafficEvidenceRecord]) -> Option<Value> {
    evidence
        .iter()
        .find(|item| item.location == SYSTEM_AGENT_CONTEXT_LOCATION)
        .and_then(|item| item.request_body.as_deref())
        .and_then(|body| serde_json::from_str::<Value>(body).ok())
}

pub fn extract_target_request_id(output: Option<&Value>, payload: Option<&Value>) -> Option<i64> {
    output
        .and_then(extract_verification_plan)
        .and_then(|plan| plan.target_request_id)
        .or_else(|| {
            payload
                .and_then(|value| value.get("dbRequestId"))
                .and_then(Value::as_i64)
        })
}

pub fn build_baseline_from_proxy_request(record: &ProxyRequestRecord) -> VerificationBaseline {
    VerificationBaseline {
        source_request_id: record.id,
        url: record.url.clone(),
        method: record.method.clone(),
        request_headers: record.request_headers.clone(),
        request_body: record.request_body.clone(),
        response_status: Some(record.status_code),
        response_headers: record.response_headers.clone(),
        response_body: record.response_body.clone(),
    }
}

pub fn build_baseline_from_evidence(item: &TrafficEvidenceRecord) -> VerificationBaseline {
    VerificationBaseline {
        source_request_id: None,
        url: item.url.clone(),
        method: item.method.clone(),
        request_headers: item.request_headers.clone(),
        request_body: item.request_body.clone(),
        response_status: item.response_status,
        response_headers: item.response_headers.clone(),
        response_body: item.response_body.clone(),
    }
}

pub fn select_fallback_evidence<'a>(
    evidence: &'a [TrafficEvidenceRecord],
) -> Option<&'a TrafficEvidenceRecord> {
    evidence.iter().find(|item| {
        !matches!(
            item.location.as_str(),
            "system_agent_context" | "system_agent_verification" | "system_agent_feedback"
        ) && item.method != "SYSTEM"
    })
}

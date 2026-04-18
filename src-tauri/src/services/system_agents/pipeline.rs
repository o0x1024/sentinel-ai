use serde_json::{json, Value};

pub const TRAFFIC_CONTEXT_AGENT_PROFILE_ID: &str = "traffic_context_agent";
pub const TRAFFIC_HYPOTHESIS_AGENT_PROFILE_ID: &str = "traffic_hypothesis_agent";
pub const TRAFFIC_VERIFICATION_AGENT_PROFILE_ID: &str = "traffic_verification_agent";
pub const TRAFFIC_DECISION_AGENT_PROFILE_ID: &str = "traffic_decision_agent";

pub const EVENT_TRAFFIC_RAW_READY: &str = "traffic.raw.ready";
pub const EVENT_TRAFFIC_CONTEXT_READY: &str = "traffic.context.ready";
pub const EVENT_TRAFFIC_HYPOTHESIS_READY: &str = "traffic.hypothesis.ready";
pub const EVENT_TRAFFIC_VERIFICATION_REQUESTED: &str = "traffic.verification.requested";
pub const EVENT_TRAFFIC_VERIFICATION_COMPLETED: &str = "traffic.verification.completed";

pub fn build_hypothesis_ready_payload(
    context_payload: Value,
    hypothesis_output: Value,
    source_profile_id: &str,
) -> Value {
    json!({
        "contextPayload": context_payload,
        "hypothesisOutput": hypothesis_output,
        "sourceProfileId": source_profile_id,
    })
}

pub fn extract_scope_url(payload: &Value) -> Option<&str> {
    payload
        .get("url")
        .and_then(Value::as_str)
        .or_else(|| {
            payload
                .get("record")
                .and_then(|value| value.get("url"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            payload
                .get("contextPayload")
                .and_then(|value| value.get("url"))
                .and_then(Value::as_str)
        })
}

pub fn extract_scope_host(payload: &Value) -> Option<&str> {
    payload
        .get("host")
        .and_then(Value::as_str)
        .or_else(|| {
            payload
                .get("record")
                .and_then(|value| value.get("host"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            payload
                .get("contextPayload")
                .and_then(|value| value.get("host"))
                .and_then(Value::as_str)
        })
}

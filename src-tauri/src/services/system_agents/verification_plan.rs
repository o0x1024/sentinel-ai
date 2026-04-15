use serde::{Deserialize, Serialize};
use serde_json::Value;

use sentinel_db::{ProxyRequestRecord, TrafficEvidenceRecord};

pub use crate::services::system_agents::verification_mutation::VerificationParameterMutation;

const SYSTEM_AGENT_CONTEXT_LOCATION: &str = "system_agent_context";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerificationTarget {
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub selector: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VerificationPlan {
    #[serde(default = "default_strategy")]
    pub preferred_strategy: String,
    #[serde(default)]
    pub target_request_id: Option<i64>,
    #[serde(default)]
    pub candidate_targets: Vec<VerificationTarget>,
    // Legacy compatibility field. New planners should use candidateTargets.
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
    #[serde(default)]
    pub parameter_mutations: Vec<VerificationParameterMutation>,
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

fn value_to_payload_string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => serde_json::to_string(other).ok(),
    }
}

pub fn extract_verification_plan(output: &Value) -> Option<VerificationPlan> {
    let raw = output.get("verificationPlan")?;
    let mut plan = serde_json::from_value::<VerificationPlan>(raw.clone()).ok()?;
    normalize_verification_plan(&mut plan);
    Some(plan)
}

pub fn normalize_verification_plan(plan: &mut VerificationPlan) {
    if plan.preferred_strategy.trim().is_empty() {
        plan.preferred_strategy = default_strategy();
    }
    plan.candidate_targets
        .retain(|target| !target.location.trim().is_empty() && !target.selector.trim().is_empty());
    dedupe_candidate_targets(&mut plan.candidate_targets);
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
    let payload_request_id = payload
        .and_then(|value| value.get("dbRequestId"))
        .and_then(Value::as_i64);
    let recent_request_ids = payload
        .and_then(|value| value.get("clusterSummary"))
        .and_then(|value| value.get("recentRequestIds"))
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_i64).collect::<Vec<_>>())
        .unwrap_or_default();
    let planned_request_id = output
        .and_then(extract_verification_plan)
        .and_then(|plan| plan.target_request_id);

    if let Some(target_request_id) = planned_request_id {
        if payload_request_id == Some(target_request_id)
            || recent_request_ids.contains(&target_request_id)
        {
            return Some(target_request_id);
        }
    }

    payload_request_id
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

pub fn build_baseline_from_context_payload(payload: &Value) -> Option<VerificationBaseline> {
    let baseline = payload.get("baselineRequest")?;
    let url = payload.get("url").and_then(Value::as_str)?.to_string();
    let method = payload.get("method").and_then(Value::as_str)?.to_string();

    Some(VerificationBaseline {
        source_request_id: payload.get("dbRequestId").and_then(Value::as_i64),
        url,
        method,
        request_headers: value_to_payload_string(baseline.get("requestHeaders")),
        request_body: value_to_payload_string(baseline.get("requestBody")),
        response_status: baseline
            .get("responseStatus")
            .and_then(Value::as_i64)
            .and_then(|value| i32::try_from(value).ok()),
        response_headers: value_to_payload_string(baseline.get("responseHeaders")),
        response_body: value_to_payload_string(baseline.get("responseBody")),
    })
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

pub fn hydrate_candidate_targets_from_context(
    plan: &mut VerificationPlan,
    payload: Option<&Value>,
) {
    if !plan.candidate_targets.is_empty() {
        dedupe_candidate_targets(&mut plan.candidate_targets);
        return;
    }

    let Some(payload) = payload else {
        return;
    };
    plan.candidate_targets =
        infer_candidate_targets_from_context(payload, &plan.candidate_parameters);
}

pub fn infer_candidate_targets_from_context(
    payload: &Value,
    candidate_parameters: &[String],
) -> Vec<VerificationTarget> {
    let body_location = infer_body_target_location(payload);
    let mut targets = Vec::new();

    if candidate_parameters.is_empty()
        || candidate_parameters
            .iter()
            .any(|item| item.eq_ignore_ascii_case("pathSegments"))
    {
        if let Some(path_segments) = payload
            .get("resourceKeys")
            .and_then(|value| value.get("pathSegments"))
            .and_then(Value::as_array)
        {
            for segment in path_segments.iter().filter_map(Value::as_str) {
                push_candidate_target(
                    &mut targets,
                    VerificationTarget {
                        location: "pathSegment".to_string(),
                        selector: segment.to_string(),
                    },
                );
            }
        }
    }

    let resource_matches = payload
        .get("contextExtraction")
        .and_then(|value| value.get("resourceMatches"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for item in resource_matches {
        let Some(matched_key) = item.get("matchedKey").and_then(Value::as_str) else {
            continue;
        };
        if !candidate_parameters.is_empty()
            && !candidate_parameters
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(matched_key))
        {
            continue;
        }
        let Some(source) = item.get("source").and_then(Value::as_str) else {
            continue;
        };
        let location = match source {
            "query" => "query",
            "body" => body_location.as_str(),
            _ => continue,
        };
        push_candidate_target(
            &mut targets,
            VerificationTarget {
                location: location.to_string(),
                selector: matched_key.to_string(),
            },
        );
    }

    targets
}

fn infer_body_target_location(payload: &Value) -> String {
    let Some(raw_body) = payload
        .get("baselineRequest")
        .and_then(|value| value.get("requestBody"))
    else {
        return "jsonBody".to_string();
    };

    match raw_body {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                "jsonBody".to_string()
            } else if serde_json::from_str::<Value>(trimmed).is_ok() {
                "jsonBody".to_string()
            } else if trimmed.contains('=') {
                "formBody".to_string()
            } else {
                "jsonBody".to_string()
            }
        }
        Value::Object(_) | Value::Array(_) => "jsonBody".to_string(),
        _ => "jsonBody".to_string(),
    }
}

fn push_candidate_target(targets: &mut Vec<VerificationTarget>, target: VerificationTarget) {
    if target.location.trim().is_empty() || target.selector.trim().is_empty() {
        return;
    }
    if targets.iter().any(|existing| {
        existing.location.eq_ignore_ascii_case(&target.location)
            && existing.selector.eq_ignore_ascii_case(&target.selector)
    }) {
        return;
    }
    targets.push(target);
}

fn dedupe_candidate_targets(targets: &mut Vec<VerificationTarget>) {
    let mut deduped = Vec::with_capacity(targets.len());
    for target in targets.drain(..) {
        push_candidate_target(&mut deduped, target);
    }
    *targets = deduped;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn infers_explicit_targets_from_context_payload() {
        let payload = json!({
            "resourceKeys": {
                "pathSegments": ["1524234490"],
                "imageId": "1524234490"
            },
            "contextExtraction": {
                "resourceMatches": [
                    {
                        "matchedKey": "imageId",
                        "source": "query"
                    },
                    {
                        "matchedKey": "pathSegments",
                        "source": "path"
                    }
                ]
            },
            "baselineRequest": {
                "requestBody": null
            }
        });

        let targets = infer_candidate_targets_from_context(
            &payload,
            &["imageId".to_string(), "pathSegments".to_string()],
        );

        assert!(targets
            .iter()
            .any(|item| { item.location == "query" && item.selector == "imageId" }));
        assert!(targets
            .iter()
            .any(|item| { item.location == "pathSegment" && item.selector == "1524234490" }));
    }

    #[test]
    fn hydrates_legacy_plan_without_overwriting_explicit_targets() {
        let payload = json!({
            "resourceKeys": {
                "pathSegments": ["123"]
            },
            "contextExtraction": {
                "resourceMatches": [
                    {
                        "matchedKey": "pathSegments",
                        "source": "path"
                    }
                ]
            },
            "baselineRequest": {
                "requestBody": null
            }
        });

        let mut plan = VerificationPlan {
            preferred_strategy: "swap_resource_reference".to_string(),
            candidate_parameters: vec!["pathSegments".to_string()],
            ..VerificationPlan::default()
        };
        hydrate_candidate_targets_from_context(&mut plan, Some(&payload));
        assert_eq!(plan.candidate_targets.len(), 1);
        assert_eq!(plan.candidate_targets[0].location, "pathSegment");
        assert_eq!(plan.candidate_targets[0].selector, "123");

        let explicit = VerificationTarget {
            location: "query".to_string(),
            selector: "imageId".to_string(),
        };
        plan.candidate_targets = vec![explicit.clone()];
        hydrate_candidate_targets_from_context(&mut plan, Some(&payload));
        assert_eq!(plan.candidate_targets, vec![explicit]);
    }
}

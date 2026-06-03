use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticCandidate {
    pub field: String,
    pub source: String,
    pub confidence: String,
    pub reason: String,
}

impl SemanticCandidate {
    fn to_value(&self) -> Value {
        json!({
            "field": self.field,
            "source": self.source,
            "confidence": self.confidence,
            "reason": self.reason,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticActionCandidate {
    pub kind: String,
    pub source: String,
    pub confidence: String,
    pub reason: String,
}

impl SemanticActionCandidate {
    fn to_value(&self) -> Value {
        json!({
            "kind": self.kind,
            "source": self.source,
            "confidence": self.confidence,
            "reason": self.reason,
        })
    }
}

pub fn semantic_mapper_prompt() -> &'static str {
    r#"You are a semantic abstraction agent for HTTP traffic.
Infer business semantics from sanitized feature names only. Never assume access to real secret values.
Map fields and path features into semantic roles such as principal, resource, credential, state, and action.
Be conservative: if you are unsure, lower confidence instead of inventing certainty.
Return strict JSON only.

Required JSON shape:
{
  "principalCandidates": [{"field": string, "source": string, "confidence": "low" | "medium" | "high", "reason": string}],
  "resourceCandidates": [{"field": string, "source": string, "confidence": "low" | "medium" | "high", "reason": string}],
  "credentialCandidates": [{"field": string, "source": string, "confidence": "low" | "medium" | "high", "reason": string}],
  "stateCandidates": [{"field": string, "source": string, "confidence": "low" | "medium" | "high", "reason": string}],
  "actionCandidates": [{"kind": string, "source": string, "confidence": "low" | "medium" | "high", "reason": string}],
  "notes": [string]
}"#
}

pub fn build_semantic_signature(payload: &Value) -> String {
    let mut hasher = DefaultHasher::new();
    semantic_signature_seed(payload).hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

pub fn build_semantic_feature_payload(payload: &Value) -> Value {
    let body_keys = payload
        .get("requestFingerprint")
        .and_then(|value| value.get("bodySchema"));
    let response_keys = payload
        .get("responseFingerprint")
        .and_then(|value| value.get("topLevelKeys"));

    json!({
        "signature": build_semantic_signature(payload),
        "method": payload.get("method").and_then(Value::as_str).unwrap_or_default(),
        "host": payload.get("host").and_then(Value::as_str).unwrap_or_default(),
        "pathTemplate": payload.get("pathTemplate").and_then(Value::as_str).unwrap_or_default(),
        "pathTokens": path_tokens(payload.get("rawPath").and_then(Value::as_str).unwrap_or_default()),
        "actionKind": payload.get("actionKind").and_then(Value::as_str).unwrap_or_default(),
        "queryKeys": payload
            .get("requestFingerprint")
            .and_then(|value| value.get("queryKeys"))
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "bodyKeys": extract_string_array(body_keys),
        "responseKeys": extract_string_array(response_keys),
        "principalKeys": object_keys(payload.get("principalContext")),
        "resourceKeys": resource_field_names(payload.get("resourceKeys")),
        "authHeaders": payload
            .get("authContext")
            .and_then(|value| value.get("headerSources"))
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "cookieKeys": payload
            .get("authContext")
            .and_then(|value| value.get("cookieKeys"))
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "recentSequence": payload
            .get("recentSequence")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "clusterPrincipalKeys": payload
            .get("clusterSummary")
            .and_then(|value| value.get("principalSignalKeys"))
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "clusterResourceKeys": payload
            .get("clusterSummary")
            .and_then(|value| value.get("resourceFieldNames"))
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        "clusterActions": payload
            .get("clusterSummary")
            .and_then(|value| value.get("actionHistogram"))
            .cloned()
            .unwrap_or_else(|| json!({})),
    })
}

pub fn should_attempt_ai_semantic_mapping(payload: &Value) -> bool {
    let action_kind = payload
        .get("actionKind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let principal_keys = object_key_count(payload.get("principalContext"));
    let resource_keys = resource_field_names(payload.get("resourceKeys"));
    let query_keys = payload
        .get("requestFingerprint")
        .and_then(|value| value.get("queryKeys"))
        .and_then(Value::as_array)
        .map(|items| items.len())
        .unwrap_or(0);
    let body_keys = extract_string_array(
        payload
            .get("requestFingerprint")
            .and_then(|value| value.get("bodySchema")),
    );
    let cookie_count = payload
        .get("authContext")
        .and_then(|value| value.get("cookieKeys"))
        .and_then(Value::as_array)
        .map(|items| items.len())
        .unwrap_or(0);

    action_kind == "invoke"
        || action_kind == "read"
        || principal_keys == 0
        || resource_keys.is_empty()
        || query_keys + body_keys.len() + cookie_count >= 2
}

pub fn deterministic_semantic_abstraction(payload: &Value) -> Value {
    let signature = build_semantic_signature(payload);
    let principal_candidates = object_keys(payload.get("principalContext"))
        .into_iter()
        .map(|field| SemanticCandidate {
            field,
            source: "context_principal".to_string(),
            confidence: "high".to_string(),
            reason: "Field was extracted into principalContext by deterministic traffic parsing."
                .to_string(),
        })
        .collect::<Vec<_>>();
    let resource_candidates = resource_field_names(payload.get("resourceKeys"))
        .into_iter()
        .map(|field| {
            let source = if field == "pathSegments" {
                "path".to_string()
            } else {
                "context_resource".to_string()
            };
            SemanticCandidate {
                field,
                source,
                confidence: "medium".to_string(),
                reason:
                    "Field appears in extracted resourceKeys and is likely part of object identity."
                        .to_string(),
            }
        })
        .collect::<Vec<_>>();

    let auth_headers = extract_string_array(
        payload
            .get("authContext")
            .and_then(|value| value.get("headerSources")),
    );
    let cookie_keys = extract_string_array(
        payload
            .get("authContext")
            .and_then(|value| value.get("cookieKeys")),
    );
    let credential_candidates = auth_headers
        .into_iter()
        .map(|field| SemanticCandidate {
            field,
            source: "auth_header".to_string(),
            confidence: "high".to_string(),
            reason: "Header contributed to authContext fingerprint construction.".to_string(),
        })
        .chain(cookie_keys.into_iter().map(|field| SemanticCandidate {
            field,
            source: "auth_cookie".to_string(),
            confidence: "medium".to_string(),
            reason: "Cookie name matched credential/session heuristics.".to_string(),
        }))
        .collect::<Vec<_>>();

    let state_candidates = state_key_candidates(payload)
        .into_iter()
        .map(|field| SemanticCandidate {
            field,
            source: "schema".to_string(),
            confidence: "medium".to_string(),
            reason: "Field name suggests workflow or lifecycle state.".to_string(),
        })
        .collect::<Vec<_>>();

    let action_candidates = payload
        .get("actionKind")
        .and_then(Value::as_str)
        .filter(|kind| !kind.is_empty())
        .map(|kind| {
            vec![SemanticActionCandidate {
                kind: kind.to_string(),
                source: "deterministic_action_kind".to_string(),
                confidence: if matches!(kind, "invoke" | "read") {
                    "medium".to_string()
                } else {
                    "high".to_string()
                },
                reason:
                    "Action came from deterministic path and method inference in traffic context."
                        .to_string(),
            }]
        })
        .unwrap_or_default();

    json!({
        "source": "fallback",
        "signature": signature,
        "principalCandidates": principal_candidates.iter().map(SemanticCandidate::to_value).collect::<Vec<_>>(),
        "resourceCandidates": resource_candidates.iter().map(SemanticCandidate::to_value).collect::<Vec<_>>(),
        "credentialCandidates": credential_candidates.iter().map(SemanticCandidate::to_value).collect::<Vec<_>>(),
        "stateCandidates": state_candidates.iter().map(SemanticCandidate::to_value).collect::<Vec<_>>(),
        "actionCandidates": action_candidates.iter().map(SemanticActionCandidate::to_value).collect::<Vec<_>>(),
        "notes": build_fallback_notes(payload),
        "featureSummary": build_semantic_feature_payload(payload),
    })
}

pub fn normalize_semantic_abstraction_output(
    raw: &str,
    payload: &Value,
    fallback: &Value,
) -> Value {
    let parsed = normalize_json_output(raw);
    let signature = build_semantic_signature(payload);

    json!({
        "source": "ai_augmented",
        "signature": signature,
        "principalCandidates": merge_candidate_groups(
            fallback.get("principalCandidates"),
            parsed.get("principalCandidates"),
            "field"
        ),
        "resourceCandidates": merge_candidate_groups(
            fallback.get("resourceCandidates"),
            parsed.get("resourceCandidates"),
            "field"
        ),
        "credentialCandidates": merge_candidate_groups(
            fallback.get("credentialCandidates"),
            parsed.get("credentialCandidates"),
            "field"
        ),
        "stateCandidates": merge_candidate_groups(
            fallback.get("stateCandidates"),
            parsed.get("stateCandidates"),
            "field"
        ),
        "actionCandidates": merge_candidate_groups(
            fallback.get("actionCandidates"),
            parsed.get("actionCandidates"),
            "kind"
        ),
        "notes": merge_string_groups(fallback.get("notes"), parsed.get("notes")),
        "featureSummary": build_semantic_feature_payload(payload),
    })
}

fn normalize_json_output(raw: &str) -> Value {
    match serde_json::from_str::<Value>(raw.trim()) {
        Ok(value) => value,
        Err(_) => {
            if let (Some(start), Some(end)) = (raw.find('{'), raw.rfind('}')) {
                serde_json::from_str::<Value>(&raw[start..=end])
                    .unwrap_or_else(|_| json!({ "raw": raw }))
            } else {
                json!({ "raw": raw })
            }
        }
    }
}

fn merge_candidate_groups(
    fallback: Option<&Value>,
    ai_value: Option<&Value>,
    primary_key: &str,
) -> Vec<Value> {
    let mut merged = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for entry in iterate_array(ai_value).chain(iterate_array(fallback)) {
        let Some(object) = entry.as_object() else {
            continue;
        };
        let dedupe_key = format!(
            "{}::{}",
            object
                .get(primary_key)
                .and_then(Value::as_str)
                .unwrap_or_default(),
            object
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or_default()
        );
        if dedupe_key == "::" || !seen.insert(dedupe_key) {
            continue;
        }
        merged.push(Value::Object(object.clone()));
    }

    merged
}

fn merge_string_groups(left: Option<&Value>, right: Option<&Value>) -> Vec<String> {
    let mut values = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for entry in iterate_array(left).chain(iterate_array(right)) {
        let Some(text) = entry.as_str() else {
            continue;
        };
        if text.is_empty() || !seen.insert(text.to_string()) {
            continue;
        }
        values.push(text.to_string());
    }
    values
}

fn iterate_array(value: Option<&Value>) -> impl Iterator<Item = &Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|items| items.iter())
}

fn build_fallback_notes(payload: &Value) -> Vec<String> {
    let mut notes = vec![
        "Fallback semantic abstraction was derived from deterministic traffic parsing.".to_string(),
    ];
    let action_kind = payload
        .get("actionKind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if matches!(action_kind, "invoke" | "read") {
        notes.push(
            "Action kind is still generic; AI semantic mapping is most useful for this request shape."
                .to_string(),
        );
    }
    notes
}

fn state_key_candidates(payload: &Value) -> Vec<String> {
    let mut keys = extract_string_array(
        payload
            .get("responseFingerprint")
            .and_then(|value| value.get("topLevelKeys")),
    );
    keys.extend(extract_string_array(
        payload
            .get("requestFingerprint")
            .and_then(|value| value.get("bodySchema")),
    ));
    keys.into_iter()
        .filter(|key| {
            let normalized = key.to_ascii_lowercase();
            ["status", "state", "phase", "step", "stage", "workflow"]
                .iter()
                .any(|needle| normalized.contains(needle))
        })
        .collect()
}

fn path_tokens(raw_path: &str) -> Vec<String> {
    raw_path
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_ascii_lowercase())
        .filter(|segment| !matches!(segment.as_str(), "api" | "v1" | "v2" | "v3"))
        .filter(|segment| !segment.chars().all(|ch| ch.is_ascii_digit()))
        .filter(|segment| !segment.contains('{'))
        .collect()
}

fn semantic_signature_seed(payload: &Value) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        payload
            .get("host")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        payload
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        payload
            .get("pathTemplate")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        payload
            .get("actionKind")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        serialize_feature(extract_string_array(
            payload
                .get("requestFingerprint")
                .and_then(|value| value.get("queryKeys")),
        )),
        serialize_feature(extract_string_array(
            payload
                .get("requestFingerprint")
                .and_then(|value| value.get("bodySchema")),
        )),
        serialize_feature(object_keys(payload.get("principalContext"))),
        serialize_feature(resource_field_names(payload.get("resourceKeys"))),
    )
}

fn serialize_feature(values: Vec<String>) -> String {
    values.join("|")
}

fn object_keys(value: Option<&Value>) -> Vec<String> {
    let mut keys = value
        .and_then(Value::as_object)
        .map(|items| items.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    keys.sort();
    keys
}

fn object_key_count(value: Option<&Value>) -> usize {
    value
        .and_then(Value::as_object)
        .map(|items| items.len())
        .unwrap_or(0)
}

fn resource_field_names(value: Option<&Value>) -> Vec<String> {
    object_keys(value)
        .into_iter()
        .filter(|key| key != "pathSegments")
        .collect()
}

fn extract_string_array(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        Some(Value::String(text)) if !text.is_empty() => vec![text.to_string()],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_semantic_feature_payload, deterministic_semantic_abstraction,
        should_attempt_ai_semantic_mapping,
    };
    use serde_json::{json, Value};

    #[test]
    fn deterministic_semantic_abstraction_exposes_expected_candidates() {
        let payload = json!({
            "host": "api.example.com",
            "method": "POST",
            "pathTemplate": "/workflow/{id}/finalize",
            "rawPath": "/workflow/123/finalize",
            "actionKind": "invoke",
            "principalContext": {"operator_code": "op-1"},
            "resourceKeys": {"case-ref": "CASE-9"},
            "authContext": {
                "headerSources": ["X-Tenant-Token"],
                "cookieKeys": ["tenant_session"]
            },
            "requestFingerprint": {
                "queryKeys": ["case-ref"],
                "bodySchema": ["operator_code", "workflowState"]
            },
            "responseFingerprint": {
                "topLevelKeys": ["workflowState", "approvedAt"]
            }
        });

        let abstraction = deterministic_semantic_abstraction(&payload);

        assert_eq!(abstraction.get("source"), Some(&json!("fallback")));
        assert_eq!(
            abstraction
                .get("principalCandidates")
                .and_then(Value::as_array)
                .map(|items| items.len()),
            Some(1)
        );
        assert_eq!(
            abstraction
                .get("stateCandidates")
                .and_then(Value::as_array)
                .map(|items| items.len()),
            Some(2)
        );
        assert!(should_attempt_ai_semantic_mapping(&payload));

        let feature_payload = build_semantic_feature_payload(&payload);
        assert_eq!(
            feature_payload.get("pathTokens"),
            Some(&json!(["workflow", "finalize"]))
        );
    }
}

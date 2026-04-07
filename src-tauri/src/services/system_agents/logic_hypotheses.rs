use serde_json::{json, Value};

pub fn build_logic_hypotheses(payload: &Value) -> Value {
    let mut hypotheses = Vec::new();

    let action_kind = payload
        .get("actionKind")
        .and_then(Value::as_str)
        .unwrap_or("inspect");
    let path_template = payload
        .get("pathTemplate")
        .and_then(Value::as_str)
        .unwrap_or("/");
    let total_requests = payload
        .get("clusterSummary")
        .and_then(|value| value.get("totalRequests"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let distinct_auth_contexts = payload
        .get("clusterSummary")
        .and_then(|value| value.get("distinctAuthContexts"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let invariant_ids = payload
        .get("logicInvariants")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("id").and_then(Value::as_str))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let resource_fields = payload
        .get("resourceKeys")
        .and_then(Value::as_object)
        .map(|items| {
            let mut fields = items.keys().cloned().collect::<Vec<_>>();
            fields.sort();
            fields
        })
        .unwrap_or_default();

    if invariant_ids.contains(&"ownership_invariant") {
        let confidence = if distinct_auth_contexts >= 3 || total_requests >= 5 {
            "high"
        } else {
            "medium"
        };
        hypotheses.push(json!({
            "id": "cross_identity_resource_access",
            "riskType": "idor",
            "confidence": confidence,
            "summary": format!(
                "The same resource pattern on '{}' is reachable across multiple auth contexts during '{}'.",
                path_template, action_kind
            ),
            "signals": [
                format!("distinctAuthContexts={distinct_auth_contexts}"),
                format!("resourceFields={}", resource_fields.join(",")),
                format!("actionKind={action_kind}"),
            ],
            "recommendedVerification": {
                "preferredStrategy": "swap_identity",
                "targetRequestId": payload.get("dbRequestId").cloned().unwrap_or(Value::Null),
                "candidateParameters": resource_fields.clone(),
                "concurrentRequests": null,
                "replayCount": null,
                "sequenceRequestIds": payload
                    .get("clusterSummary")
                    .and_then(|value| value.get("recentRequestIds"))
                    .cloned()
                    .unwrap_or_else(|| Value::Array(Vec::new())),
                "notes": [
                    "Replay the same resource request using an alternate authenticated context from the cluster.",
                    "If the request still succeeds with similar response semantics, treat it as strong object authorization evidence."
                ]
            }
        }));
    }

    if invariant_ids.contains(&"role_separation_invariant") {
        hypotheses.push(json!({
            "id": "role_sensitive_function_access",
            "riskType": "bfla",
            "confidence": if distinct_auth_contexts >= 2 { "medium" } else { "low" },
            "summary": format!(
                "The action '{}' looks role-sensitive on '{}' but the observed context lacks explicit role separation.",
                action_kind, path_template
            ),
            "signals": [
                format!("distinctAuthContexts={distinct_auth_contexts}"),
                format!("actionKind={action_kind}"),
            ],
            "recommendedVerification": {
                "preferredStrategy": "swap_identity",
                "targetRequestId": payload.get("dbRequestId").cloned().unwrap_or(Value::Null),
                "candidateParameters": resource_fields.clone(),
                "concurrentRequests": null,
                "replayCount": null,
                "sequenceRequestIds": payload
                    .get("clusterSummary")
                    .and_then(|value| value.get("recentRequestIds"))
                    .cloned()
                    .unwrap_or_else(|| Value::Array(Vec::new())),
                "notes": [
                    "Try the same privileged action with another authenticated identity from the same cluster.",
                    "Focus on approval, refund, delete, confirm, and payment style actions."
                ]
            }
        }));
    }

    if invariant_ids.contains(&"state_prerequisite_invariant") {
        hypotheses.push(json!({
            "id": "missing_prerequisite_transition",
            "riskType": "workflow",
            "confidence": if total_requests >= 4 { "high" } else { "medium" },
            "summary": format!(
                "The action '{}' on '{}' appears without strong prerequisite signals in the recent sequence.",
                action_kind, path_template
            ),
            "signals": [
                format!("recentSequence={}", render_sequence(payload)),
                format!("actionKind={action_kind}"),
            ],
            "recommendedVerification": {
                "preferredStrategy": "skip_prerequisite",
                "targetRequestId": payload.get("dbRequestId").cloned().unwrap_or(Value::Null),
                "candidateParameters": resource_fields.clone(),
                "replayCount": 1,
                "concurrentRequests": null,
                "sequenceRequestIds": payload
                    .get("clusterSummary")
                    .and_then(|value| value.get("recentRequestIds"))
                    .cloned()
                    .unwrap_or_else(|| Value::Array(Vec::new())),
                "notes": [
                    "Replay the target action directly without reproducing the earlier workflow steps.",
                    "If it still succeeds, the server may not enforce required state transitions."
                ]
            }
        }));
    }

    if invariant_ids.contains(&"single_use_invariant") {
        hypotheses.push(json!({
            "id": "repeatable_single_use_action",
            "riskType": "logic",
            "confidence": if total_requests >= 4 { "high" } else { "medium" },
            "summary": format!(
                "The action '{}' on '{}' repeated inside the same cluster and may violate single-use expectations.",
                action_kind, path_template
            ),
            "signals": [
                format!("actionKind={action_kind}"),
                format!("totalRequests={total_requests}"),
            ],
            "recommendedVerification": {
                "preferredStrategy": "repeat_action",
                "targetRequestId": payload.get("dbRequestId").cloned().unwrap_or(Value::Null),
                "candidateParameters": resource_fields.clone(),
                "replayCount": 2,
                "concurrentRequests": null,
                "sequenceRequestIds": payload
                    .get("clusterSummary")
                    .and_then(|value| value.get("recentRequestIds"))
                    .cloned()
                    .unwrap_or_else(|| Value::Array(Vec::new())),
                "notes": [
                    "Repeat the same request without changing the payload.",
                    "If the second replay keeps succeeding, review idempotency and one-time-consumption controls."
                ]
            }
        }));
    }

    if invariant_ids.contains(&"concurrency_invariant") {
        hypotheses.push(json!({
            "id": "concurrent_duplicate_acceptance",
            "riskType": "race",
            "confidence": if total_requests >= 4 { "high" } else { "medium" },
            "summary": format!(
                "The action '{}' on '{}' repeated enough times to justify an idempotency or race-condition hypothesis.",
                action_kind, path_template
            ),
            "signals": [
                format!("actionKind={action_kind}"),
                format!("totalRequests={total_requests}"),
            ],
            "recommendedVerification": {
                "preferredStrategy": "concurrent_submit",
                "targetRequestId": payload.get("dbRequestId").cloned().unwrap_or(Value::Null),
                "candidateParameters": resource_fields,
                "replayCount": null,
                "concurrentRequests": 3,
                "sequenceRequestIds": payload
                    .get("clusterSummary")
                    .and_then(|value| value.get("recentRequestIds"))
                    .cloned()
                    .unwrap_or_else(|| Value::Array(Vec::new())),
                "notes": [
                    "Replay the same request concurrently.",
                    "If duplicate submissions both succeed, treat it as strong race or idempotency evidence."
                ]
            }
        }));
    }

    Value::Array(hypotheses)
}

fn render_sequence(payload: &Value) -> String {
    payload
        .get("recentSequence")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" -> ")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_object_access_hypothesis() {
        let payload = json!({
            "actionKind": "read",
            "pathTemplate": "/api/orders/{id}",
            "dbRequestId": 77,
            "resourceKeys": {
                "orderId": "123"
            },
            "logicInvariants": [
                { "id": "ownership_invariant" }
            ],
            "clusterSummary": {
                "totalRequests": 4,
                "distinctAuthContexts": 2
            }
        });

        let hypotheses = build_logic_hypotheses(&payload);
        let items = hypotheses.as_array().expect("array");
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].get("riskType").and_then(Value::as_str),
            Some("idor")
        );
        assert_eq!(
            items[0]
                .get("recommendedVerification")
                .and_then(|value| value.get("preferredStrategy"))
                .and_then(Value::as_str),
            Some("swap_identity")
        );
    }

    #[test]
    fn builds_race_hypothesis() {
        let payload = json!({
            "actionKind": "submit",
            "pathTemplate": "/api/redeem/{id}",
            "dbRequestId": 88,
            "resourceKeys": {
                "id": "1"
            },
            "logicInvariants": [
                { "id": "concurrency_invariant" }
            ],
            "clusterSummary": {
                "totalRequests": 5,
                "distinctAuthContexts": 1
            }
        });

        let hypotheses = build_logic_hypotheses(&payload);
        let items = hypotheses.as_array().expect("array");
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].get("riskType").and_then(Value::as_str),
            Some("race")
        );
        assert_eq!(
            items[0]
                .get("recommendedVerification")
                .and_then(|value| value.get("preferredStrategy"))
                .and_then(Value::as_str),
            Some("concurrent_submit")
        );
    }
}

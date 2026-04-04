use serde_json::{json, Value};

pub fn evaluate_logic_invariants(payload: &Value) -> Value {
    let mut hits = Vec::new();

    let action_kind = payload
        .get("actionKind")
        .and_then(Value::as_str)
        .unwrap_or("inspect");
    let distinct_auth_contexts = payload
        .get("clusterSummary")
        .and_then(|value| value.get("distinctAuthContexts"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let has_resource_keys = payload
        .get("resourceKeys")
        .and_then(Value::as_object)
        .map(|items| !items.is_empty())
        .unwrap_or(false);

    if distinct_auth_contexts > 1 && has_resource_keys {
        hits.push(json!({
            "id": "ownership_invariant",
            "severity": "medium",
            "reason": "The same resource pattern appears across multiple auth contexts.",
        }));
    }

    let recent_sequence = payload
        .get("recentSequence")
        .and_then(Value::as_array)
        .map(|items| {
            items.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if requires_prerequisite(action_kind) && !has_prerequisite(action_kind, &recent_sequence) {
        hits.push(json!({
            "id": "state_prerequisite_invariant",
            "severity": "high",
            "reason": format!("Action '{}' appeared without an obvious prerequisite action in the recent sequence.", action_kind),
        }));
    }

    let principal_has_role = payload
        .get("principalContext")
        .and_then(Value::as_object)
        .map(|items| items.contains_key("role") || items.contains_key("roleId") || items.contains_key("role_id"))
        .unwrap_or(false);
    if principal_has_role && matches!(action_kind, "approve" | "refund" | "delete" | "administer") {
        hits.push(json!({
            "id": "role_separation_invariant",
            "severity": "medium",
            "reason": format!("Action '{}' looks role-sensitive and should enforce separation of duties.", action_kind),
        }));
    }

    let total_requests = payload
        .get("clusterSummary")
        .and_then(|value| value.get("totalRequests"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if matches!(action_kind, "redeem" | "pay" | "claim" | "refund") && total_requests > 1 {
        hits.push(json!({
            "id": "single_use_invariant",
            "severity": "medium",
            "reason": format!("Action '{}' appears multiple times and may violate single-use expectations.", action_kind),
        }));
    }

    let action_count = payload
        .get("clusterSummary")
        .and_then(|value| value.get("actionHistogram"))
        .and_then(|value| value.get(action_kind))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if action_count > 1 && matches!(action_kind, "submit" | "redeem" | "pay" | "approve") {
        hits.push(json!({
            "id": "concurrency_invariant",
            "severity": "medium",
            "reason": format!("Action '{}' repeats within the same cluster and may need idempotency or concurrency checks.", action_kind),
        }));
    }

    Value::Array(hits)
}

fn requires_prerequisite(action: &str) -> bool {
    matches!(action, "approve" | "refund" | "pay" | "redeem" | "submit")
}

fn has_prerequisite(action: &str, recent_sequence: &[String]) -> bool {
    let candidates: &[&str] = match action {
        "approve" => &["submit", "review", "create"],
        "refund" => &["pay", "capture", "approve"],
        "pay" => &["create", "submit", "confirm"],
        "redeem" => &["claim", "issue", "grant"],
        "submit" => &["create", "draft", "edit"],
        _ => &[],
    };

    recent_sequence
        .iter()
        .any(|item| candidates.iter().any(|candidate| item == candidate))
}

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
    let total_requests = payload
        .get("clusterSummary")
        .and_then(|value| value.get("totalRequests"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let action_count = payload
        .get("clusterSummary")
        .and_then(|value| value.get("actionHistogram"))
        .and_then(|value| value.get(action_kind))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let recent_sequence = payload
        .get("recentSequence")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let principal_has_identity = payload
        .get("principalContext")
        .and_then(Value::as_object)
        .map(has_principal_identity)
        .unwrap_or(false);

    if distinct_auth_contexts > 1
        && has_resource_keys
        && total_requests >= 3
        && action_count >= 1
        && principal_has_identity
        && suggests_object_boundary_action(action_kind)
    {
        hits.push(json!({
            "id": "ownership_invariant",
            "severity": "medium",
            "reason": "The same resource pattern appears across multiple auth contexts during an object-boundary-sensitive action.",
        }));
    }

    if total_requests >= 3
        && has_resource_keys
        && recent_sequence.len() >= 3
        && requires_prerequisite(action_kind)
        && !has_prerequisite(action_kind, &recent_sequence)
    {
        hits.push(json!({
            "id": "state_prerequisite_invariant",
            "severity": "medium",
            "reason": format!("Action '{}' appeared without a strong prerequisite signal in the recent sequence.", action_kind),
        }));
    }

    let principal_has_role = payload
        .get("principalContext")
        .and_then(Value::as_object)
        .map(|items| {
            items.contains_key("role")
                || items.contains_key("roleId")
                || items.contains_key("role_id")
        })
        .unwrap_or(false);
    if !principal_has_role
        && total_requests >= 3
        && distinct_auth_contexts > 1
        && matches!(
            action_kind,
            "approve" | "refund" | "delete" | "administer" | "confirm" | "pay"
        )
    {
        hits.push(json!({
            "id": "role_separation_invariant",
            "severity": "medium",
            "reason": format!("Action '{}' looks role-sensitive but the observed context does not expose explicit role boundaries.", action_kind),
        }));
    }

    if matches!(
        action_kind,
        "redeem" | "pay" | "claim" | "refund" | "confirm"
    ) && total_requests >= 3
        && action_count > 1
        && has_resource_keys
    {
        hits.push(json!({
            "id": "single_use_invariant",
            "severity": "medium",
            "reason": format!("Action '{}' appears multiple times and may violate single-use expectations.", action_kind),
        }));
    }

    if action_count > 1
        && total_requests >= 3
        && matches!(
            action_kind,
            "submit" | "redeem" | "pay" | "approve" | "confirm"
        )
    {
        hits.push(json!({
            "id": "concurrency_invariant",
            "severity": "medium",
            "reason": format!("Action '{}' repeats within the same cluster and may need idempotency or concurrency checks.", action_kind),
        }));
    }

    Value::Array(hits)
}

fn requires_prerequisite(action: &str) -> bool {
    matches!(
        action,
        "approve" | "refund" | "pay" | "redeem" | "submit" | "confirm" | "review"
    )
}

fn has_prerequisite(action: &str, recent_sequence: &[String]) -> bool {
    let candidates: &[&str] = match action {
        "approve" => &["submit", "review", "create"],
        "review" => &["create", "draft", "submit"],
        "confirm" => &["review", "approve", "submit", "create"],
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

fn suggests_object_boundary_action(action: &str) -> bool {
    matches!(action, "read" | "update" | "delete" | "export" | "confirm")
}

fn has_principal_identity(items: &serde_json::Map<String, Value>) -> bool {
    items.contains_key("userId")
        || items.contains_key("user_id")
        || items.contains_key("tenantId")
        || items.contains_key("tenant_id")
        || items.contains_key("orgId")
        || items.contains_key("org_id")
        || items.contains_key("accountId")
        || items.contains_key("account_id")
}

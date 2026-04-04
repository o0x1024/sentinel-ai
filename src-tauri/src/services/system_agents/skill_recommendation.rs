use serde_json::{json, Value};

pub fn recommend_logic_skills(payload: &Value) -> Value {
    let mut recommendations = Vec::new();

    let path = payload
        .get("pathTemplate")
        .and_then(Value::as_str)
        .unwrap_or("/")
        .to_ascii_lowercase();
    let action_kind = payload
        .get("actionKind")
        .and_then(Value::as_str)
        .unwrap_or("inspect")
        .to_ascii_lowercase();
    let resource_fields = payload
        .get("resourceKeys")
        .and_then(Value::as_object)
        .map(|items| items.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let invariant_ids = payload
        .get("logicInvariants")
        .and_then(Value::as_array)
        .map(|items| {
            items.iter()
                .filter_map(|item| item.get("id").and_then(Value::as_str))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if path.contains("pay") || path.contains("refund") || action_kind == "pay" || action_kind == "refund" {
        recommendations.push(json!({
            "id": "payment-flow",
            "score": 0.86,
            "reasons": [
                "path/action strongly suggest a payment or refund workflow",
            ]
        }));
    }

    if path.contains("approve")
        || path.contains("reject")
        || action_kind == "approve"
        || invariant_ids.iter().any(|id| id == "role_separation_invariant")
    {
        recommendations.push(json!({
            "id": "approval-workflow",
            "score": 0.82,
            "reasons": [
                "approval-like action or role-sensitive invariant detected",
            ]
        }));
    }

    if resource_fields.iter().any(|field| field.ends_with("Id") || field.ends_with("_id"))
        || invariant_ids.iter().any(|id| id == "ownership_invariant")
    {
        recommendations.push(json!({
            "id": "resource-ownership",
            "score": 0.8,
            "reasons": [
                "resource identifiers or ownership-style access pattern detected",
            ]
        }));
    }

    if invariant_ids
        .iter()
        .any(|id| id == "single_use_invariant" || id == "concurrency_invariant")
    {
        recommendations.push(json!({
            "id": "single-use-consumption",
            "score": 0.79,
            "reasons": [
                "repeated action pattern indicates single-use or idempotency risk",
            ]
        }));
    }

    Value::Array(recommendations)
}

use serde_json::{json, Value};

pub fn build_process_graph(payload: &Value) -> Option<Value> {
    let action_kind = payload.get("actionKind").and_then(Value::as_str)?;
    let path_template = payload
        .get("pathTemplate")
        .and_then(Value::as_str)
        .unwrap_or("/");
    let principal_fields = payload
        .get("principalContext")
        .and_then(Value::as_object)
        .map(|items| items.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let resource_fields = payload
        .get("resourceKeys")
        .and_then(Value::as_object)
        .map(|items| items.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
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

    let mut nodes = vec![
        json!({"id": "principal", "type": "principal", "labels": principal_fields}),
        json!({"id": "resource", "type": "resource", "labels": resource_fields}),
        json!({"id": "action", "type": "action", "labels": [action_kind]}),
    ];

    if let Some(body_schema) = payload
        .get("requestFingerprint")
        .and_then(|value| value.get("bodySchema"))
    {
        nodes.push(json!({"id": "request-body", "type": "request", "labels": body_schema}));
    }

    let mut edges = vec![
        json!({"from": "principal", "to": "action", "relation": "initiates"}),
        json!({"from": "action", "to": "resource", "relation": "targets"}),
    ];

    if let Some(status_code) = payload.get("statusCode").and_then(Value::as_i64) {
        nodes.push(json!({
            "id": "response",
            "type": "response",
            "labels": [format!("status:{status_code}")]
        }));
        edges.push(json!({"from": "action", "to": "response", "relation": "produces"}));
    }

    Some(json!({
        "graphKey": payload.get("clusterKey").cloned().unwrap_or(Value::Null),
        "pathTemplate": path_template,
        "currentAction": action_kind,
        "sequence": recent_sequence,
        "nodes": nodes,
        "edges": edges,
        "summary": format!(
            "Process graph links principal -> action '{}' -> resource for '{}'.",
            action_kind, path_template
        ),
    }))
}

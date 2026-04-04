use serde_json::{json, Value};

const MAX_BEHAVIOR_STEPS: usize = 8;
const MAX_INTENT_HINTS: usize = 6;

pub fn build_behavior_session(payload: &Value) -> Option<Value> {
    let host = payload.get("host")?.as_str()?;
    let path_template = payload.get("pathTemplate").and_then(Value::as_str).unwrap_or("/");
    let action_kind = payload
        .get("actionKind")
        .and_then(Value::as_str)
        .unwrap_or("inspect");
    let auth_fingerprint = payload
        .get("authContext")
        .and_then(|value| value.get("fingerprint"))
        .and_then(Value::as_str)
        .unwrap_or("anonymous");
    let selected_mode = payload
        .get("behaviorSignal")
        .and_then(|value| value.get("selectedMode"))
        .and_then(Value::as_str)
        .unwrap_or("proxy_inferred");
    let effective_mode = payload
        .get("behaviorSignal")
        .and_then(|value| value.get("effectiveMode"))
        .and_then(Value::as_str)
        .unwrap_or("proxy_inferred");
    let browser_extension_behavior = payload.get("browserExtensionBehavior");

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

    let mut behavior_steps = browser_extension_behavior
        .and_then(|value| value.get("behaviorSteps"))
        .and_then(Value::as_array)
        .map(|items| {
            items.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or(recent_sequence);
    if behavior_steps.last().map(|value| value.as_str()) != Some(action_kind) {
        behavior_steps.push(action_kind.to_string());
    }
    if behavior_steps.len() > MAX_BEHAVIOR_STEPS {
        let overflow = behavior_steps.len() - MAX_BEHAVIOR_STEPS;
        behavior_steps.drain(0..overflow);
    }

    let mut intent_hints = Vec::new();
    push_unique(&mut intent_hints, action_kind.to_string());

    if let Some(raw_path) = payload.get("rawPath").and_then(Value::as_str) {
        for segment in raw_path.split('/') {
            let normalized = segment.trim().to_ascii_lowercase();
            if normalized.len() < 3 || normalized.chars().all(|char| char.is_ascii_digit()) {
                continue;
            }
            push_unique(&mut intent_hints, normalized);
            if intent_hints.len() >= MAX_INTENT_HINTS {
                break;
            }
        }
    }

    let query_keys = payload
        .get("requestFingerprint")
        .and_then(|value| value.get("queryKeys"))
        .and_then(Value::as_array)
        .map(|items| {
            items.iter()
                .filter_map(Value::as_str)
                .take(MAX_INTENT_HINTS)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for key in query_keys {
        push_unique(&mut intent_hints, key);
    }
    if let Some(items) = browser_extension_behavior
        .and_then(|value| value.get("intentHints"))
        .and_then(Value::as_array)
    {
        for item in items.iter().filter_map(Value::as_str) {
            push_unique(&mut intent_hints, item.to_string());
        }
    }

    let resource_keys = payload
        .get("resourceKeys")
        .and_then(Value::as_object)
        .map(|items| items.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    Some(json!({
        "sessionKey": format!("{host}::{auth_fingerprint}"),
        "selectedMode": selected_mode,
        "effectiveMode": effective_mode,
        "kind": if effective_mode == "browser_extension" { "enhanced_behavior_session" } else { "proxy_inferred_behavior_session" },
        "currentAction": action_kind,
        "pathTemplate": path_template,
        "behaviorSteps": behavior_steps,
        "intentHints": intent_hints,
        "resourceFocus": resource_keys,
        "browserExtensionBehavior": browser_extension_behavior.cloned().unwrap_or(Value::Null),
        "summary": format!(
            "Behavior session on host '{}' currently appears to be at action '{}' for '{}'.",
            host, action_kind, path_template
        ),
    }))
}

fn push_unique(items: &mut Vec<String>, value: String) {
    if value.is_empty() || items.iter().any(|item| item == &value) {
        return;
    }
    items.push(value);
}

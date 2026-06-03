use serde_json::Value;

use crate::services::system_agents::types::SystemAgentEvent;

pub fn matches_event_filter(filter: &Value, event: &SystemAgentEvent) -> bool {
    if filter.is_null() {
        return true;
    }

    let envelope = serde_json::json!({
        "eventName": event.event_name,
        "source": event.source,
        "payload": event.payload,
    });

    matches_value(filter, &envelope)
}

fn matches_value(expected: &Value, actual: &Value) -> bool {
    match expected {
        Value::Null => true,
        Value::Object(expected_obj) => {
            if expected_obj.is_empty() {
                return true;
            }

            let actual_obj = match actual.as_object() {
                Some(actual_obj) => actual_obj,
                None => return false,
            };

            expected_obj.iter().all(|(key, expected_value)| {
                actual_obj
                    .get(key)
                    .or_else(|| {
                        actual_obj
                            .get("payload")
                            .and_then(Value::as_object)
                            .and_then(|payload| payload.get(key))
                    })
                    .map(|actual_value| matches_value(expected_value, actual_value))
                    .unwrap_or(false)
            })
        }
        Value::Array(expected_items) => {
            if expected_items.is_empty() {
                return true;
            }

            match actual {
                Value::Array(actual_items) => expected_items.iter().all(|expected_item| {
                    actual_items
                        .iter()
                        .any(|actual_item| matches_value(expected_item, actual_item))
                }),
                _ => expected_items
                    .iter()
                    .any(|expected_item| matches_value(expected_item, actual)),
            }
        }
        _ => expected == actual,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    fn build_event() -> SystemAgentEvent {
        SystemAgentEvent {
            event_name: "traffic.cluster.ready".to_string(),
            source: "traffic_history".to_string(),
            timestamp: Utc::now(),
            payload: json!({
                "host": "example.com",
                "method": "GET",
                "statusCode": 200,
                "tags": ["api", "public"],
            }),
        }
    }

    #[test]
    fn matches_empty_filter() {
        assert!(matches_event_filter(&json!({}), &build_event()));
    }

    #[test]
    fn matches_nested_payload_filter() {
        assert!(matches_event_filter(
            &json!({ "payload": { "host": "example.com", "method": "GET" } }),
            &build_event()
        ));
    }

    #[test]
    fn matches_direct_payload_key_filter() {
        assert!(matches_event_filter(
            &json!({ "host": "example.com" }),
            &build_event()
        ));
    }

    #[test]
    fn matches_scalar_against_expected_array() {
        assert!(matches_event_filter(
            &json!({ "payload": { "method": ["POST", "GET"] } }),
            &build_event()
        ));
    }

    #[test]
    fn rejects_non_matching_filter() {
        assert!(!matches_event_filter(
            &json!({ "payload": { "statusCode": 500 } }),
            &build_event()
        ));
    }
}

//! Helpers for parsing rig-core tool result envelopes and detecting skill outcomes.

use serde_json::Value;

/// Unwrap rig `[{"type":"text","text":"{...}"}]` envelopes to the inner JSON payload.
pub fn unwrap_tool_result_payload(parsed: Value) -> Value {
    if parsed.get("action").is_some() || parsed.get("success").is_some() {
        return parsed;
    }

    if let Some(items) = parsed.as_array() {
        for item in items {
            let Some(obj) = item.as_object() else {
                continue;
            };
            if obj.get("type").and_then(Value::as_str) != Some("text") {
                continue;
            }
            let Some(text) = obj.get("text").and_then(Value::as_str) else {
                continue;
            };
            if let Ok(inner) = serde_json::from_str::<Value>(text) {
                return inner;
            }
        }
    }

    parsed
}

/// Parse a raw tool result string into the inner JSON payload when possible.
pub fn parse_tool_result_payload(raw: &str) -> Option<Value> {
    let parsed = serde_json::from_str::<Value>(raw).ok()?;
    Some(unwrap_tool_result_payload(parsed))
}

fn skills_action_success(payload: &Value) -> Option<bool> {
    let action = payload.get("action")?.as_str()?;
    match action {
        "invoke" => {
            let content = payload.get("content")?.as_str()?;
            Some(content.starts_with("Skill loaded:") && content.contains("<skill>"))
        }
        "fork" => {
            let content = payload.get("content")?.as_str()?;
            Some(content.contains("forked execution") || content.starts_with("Skill \""))
        }
        "read_file" => {
            let content = payload.get("content")?.as_str()?;
            Some(content.contains("<file path="))
        }
        _ => None,
    }
}

/// When the payload is a successful `skills` invoke/fork result, return `Some(true|false)`.
pub fn skills_tool_success_from_value(value: &Value) -> Option<bool> {
    skills_action_success(&unwrap_tool_result_payload(value.clone()))
}

/// When the raw result is a successful `skills` invoke/fork payload, return `Some(true|false)`.
pub fn infer_skills_tool_success_from_result(raw: &str) -> Option<bool> {
    let payload = parse_tool_result_payload(raw)?;
    skills_action_success(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_invoke_content() -> String {
        format!(
            "Skill loaded: penetration-tester\n\n<skill>\n<name>penetration-tester</name>\n{}\n</skill>",
            "timed out error failed ".repeat(20)
        )
    }

    #[test]
    fn unwrap_rig_text_envelope() {
        let inner = serde_json::json!({
            "action": "invoke",
            "content": sample_invoke_content(),
        });
        let wrapped = serde_json::json!([{
            "type": "text",
            "text": inner.to_string(),
        }]);
        let payload = unwrap_tool_result_payload(wrapped);
        assert_eq!(payload.get("action").and_then(|v| v.as_str()), Some("invoke"));
    }

    #[test]
    fn skills_read_file_success() {
        let inner = serde_json::json!({
            "action": "read_file",
            "content": "<file path=\"references/attack_vectors.md\">\n# Attack Vectors\n</file>",
            "referenced_files": ["references/attack_vectors.md"],
        });
        let raw = serde_json::json!([{"type":"text","text": inner.to_string()}]).to_string();
        assert_eq!(infer_skills_tool_success_from_result(&raw), Some(true));
    }

    #[test]
    fn skills_invoke_success_ignores_error_like_content() {
        let inner = serde_json::json!({
            "action": "invoke",
            "content": sample_invoke_content(),
        });
        let wrapped = serde_json::json!([{
            "type": "text",
            "text": inner.to_string(),
        }]);
        let raw = wrapped.to_string();
        assert_eq!(infer_skills_tool_success_from_result(&raw), Some(true));
    }

    #[test]
    fn skills_invoke_without_skill_tag_is_not_success() {
        let inner = serde_json::json!({
            "action": "invoke",
            "content": "Skill loaded: audit\n\nplain text",
        });
        let raw = serde_json::json!([{"type":"text","text": inner.to_string()}]).to_string();
        assert_eq!(infer_skills_tool_success_from_result(&raw), Some(false));
    }
}

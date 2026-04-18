use serde_json::{json, Map, Value};

fn parse_embedded_json_value(value: Value, depth: usize) -> Value {
    if depth >= 4 {
        return value;
    }

    match value {
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Value::String(s);
            }

            let looks_like_json =
                matches!(trimmed.chars().next(), Some('{') | Some('[') | Some('"'));
            if !looks_like_json {
                return Value::String(s);
            }

            match serde_json::from_str::<Value>(trimmed) {
                Ok(parsed) => parse_embedded_json_value(parsed, depth + 1),
                Err(_) => Value::String(s),
            }
        }
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| parse_embedded_json_value(item, depth + 1))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| (key, parse_embedded_json_value(value, depth + 1)))
                .collect(),
        ),
        other => other,
    }
}

fn requires_command_object(tool_name: &str) -> bool {
    matches!(
        tool_name.trim().to_lowercase().as_str(),
        "shell" | "interactive_shell"
    )
}

fn empty_object() -> Value {
    Value::Object(Map::new())
}

fn command_object(command: String) -> Value {
    if command.trim().is_empty() {
        empty_object()
    } else {
        json!({ "command": command })
    }
}

pub fn normalize_tool_call_arguments_value(tool_name: &str, value: Value) -> Value {
    let parsed = parse_embedded_json_value(value, 0);

    if requires_command_object(tool_name) {
        return match parsed {
            Value::Object(_) | Value::Array(_) => parsed,
            Value::String(command) => command_object(command),
            Value::Null => empty_object(),
            other => command_object(other.to_string()),
        };
    }

    match parsed {
        Value::Object(_) | Value::Array(_) => parsed,
        Value::String(raw) => json!({ "raw": raw }),
        Value::Null => empty_object(),
        other => json!({ "raw": other }),
    }
}

pub fn normalize_tool_call_arguments_str(tool_name: &str, raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return empty_object();
    }

    match serde_json::from_str::<Value>(trimmed) {
        Ok(parsed) => normalize_tool_call_arguments_value(tool_name, parsed),
        Err(_) if requires_command_object(tool_name) => command_object(raw.to_string()),
        Err(_) => json!({ "raw": raw }),
    }
}

pub fn normalize_tool_call_arguments_json(tool_name: &str, value: &Value) -> String {
    normalize_tool_call_arguments_value(tool_name, value.clone()).to_string()
}

pub fn tool_call_argument_repair_message(tool_name: &str, raw: &str) -> Option<String> {
    match tool_name.trim().to_lowercase().as_str() {
        "shell" => shell_argument_repair_message(raw),
        "interactive_shell" => interactive_shell_argument_repair_message(raw),
        _ => None,
    }
}

fn shell_argument_repair_message(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Some(
            "Tool call rejected: `shell` requires `function.arguments` to be a JSON object with a non-empty `command` string, for example {\"command\":\"pwd\"}.".to_string(),
        );
    }

    let parsed = serde_json::from_str::<Value>(trimmed).ok();
    match parsed {
        Some(Value::Object(map)) => match map.get("command").and_then(|value| value.as_str()) {
            Some(command) if !command.trim().is_empty() => None,
            _ => Some(
                "Tool call rejected: `shell` requires `function.arguments.command` to be a non-empty string. Retry with a JSON object like {\"command\":\"pwd\"}.".to_string(),
            ),
        },
        Some(Value::String(command)) => Some(format!(
            "Tool call rejected: `shell` does not accept a bare string in `function.arguments`. Retry with this JSON object instead: {}",
            command_object(command)
        )),
        None => Some(format!(
            "Tool call rejected: `shell` does not accept a bare string in `function.arguments`. Retry with this JSON object instead: {}",
            command_object(trimmed.to_string())
        )),
        _ => Some(
            "Tool call rejected: `shell` requires `function.arguments` to be a JSON object like {\"command\":\"pwd\"}.".to_string(),
        ),
    }
}

fn interactive_shell_argument_repair_message(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Some(
            "Tool call rejected: `interactive_shell` requires `function.arguments` to be a JSON object, for example {\"command\":\"top\",\"session_policy\":\"reuse\"}.".to_string(),
        );
    }

    let parsed = serde_json::from_str::<Value>(trimmed).ok();
    match parsed {
        Some(Value::Object(map)) => {
            if let Some(command_value) = map.get("command") {
                match command_value.as_str() {
                    Some(command) if !command.trim().is_empty() => None,
                    Some(_) => Some(
                        "Tool call rejected: `interactive_shell.command` must be a non-empty string when provided.".to_string(),
                    ),
                    None => Some(
                        "Tool call rejected: `interactive_shell.command` must be a string when provided.".to_string(),
                    ),
                }
            } else {
                None
            }
        }
        Some(Value::String(command)) => Some(format!(
            "Tool call rejected: `interactive_shell` does not accept a bare string in `function.arguments`. Retry with a JSON object instead: {}",
            command_object(command)
        )),
        None => Some(format!(
            "Tool call rejected: `interactive_shell` does not accept a bare string in `function.arguments`. Retry with a JSON object instead: {}",
            command_object(trimmed.to_string())
        )),
        _ => Some(
            "Tool call rejected: `interactive_shell` requires `function.arguments` to be a JSON object.".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_tool_call_arguments_str, normalize_tool_call_arguments_value,
        tool_call_argument_repair_message,
    };
    use serde_json::json;

    #[test]
    fn wraps_shell_string_into_command_object() {
        assert_eq!(
            normalize_tool_call_arguments_value("shell", json!("pwd")),
            json!({ "command": "pwd" })
        );
    }

    #[test]
    fn unwraps_stringified_shell_object() {
        assert_eq!(
            normalize_tool_call_arguments_str("shell", r#""{\"command\":\"pwd\"}""#),
            json!({ "command": "pwd" })
        );
    }

    #[test]
    fn wraps_non_shell_string_as_raw_object() {
        assert_eq!(
            normalize_tool_call_arguments_value("web_search", json!("hello")),
            json!({ "raw": "hello" })
        );
    }

    #[test]
    fn preserves_object_arguments() {
        assert_eq!(
            normalize_tool_call_arguments_value("shell", json!({ "command": "ls" })),
            json!({ "command": "ls" })
        );
    }

    #[test]
    fn rejects_bare_shell_string_with_repair_message() {
        let message = tool_call_argument_repair_message("shell", "\"pwd\"")
            .expect("shell bare string should be rejected");
        assert!(message.contains("{\"command\":\"pwd\"}"));
    }

    #[test]
    fn accepts_valid_shell_command_object() {
        assert!(tool_call_argument_repair_message("shell", r#"{"command":"pwd"}"#).is_none());
    }
}

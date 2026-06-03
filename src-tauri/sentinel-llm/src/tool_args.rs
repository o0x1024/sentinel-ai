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
    matches!(tool_name.trim().to_lowercase().as_str(), "shell")
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
        _ => None,
    }
}

fn has_non_empty_string(map: &Map<String, Value>, key: &str) -> bool {
    map.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
}

fn has_valid_shell_call_shape(map: &Map<String, Value>) -> bool {
    has_non_empty_string(map, "command")
        || has_non_empty_string(map, "cmd")
        || has_non_empty_string(map, "session_id")
        || has_non_empty_string(map, "process_id")
        || has_non_empty_string(map, "action")
        || has_shell_stdin_key(map)
}

fn has_shell_stdin_key(map: &Map<String, Value>) -> bool {
    map.contains_key("chars") || map.contains_key("input") || map.contains_key("input_text")
}

fn shell_argument_repair_message(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let parsed = serde_json::from_str::<Value>(trimmed).ok();
    match parsed {
        Some(Value::Object(map)) => {
            if has_valid_shell_call_shape(&map) {
                None
            } else {
                Some(
                    "Tool call rejected: `shell` requires a non-empty `command`/`cmd` to start a command, or `session_id`/`process_id` plus an explicit action to continue one. Retry with {\"command\":\"pwd\"}, {\"session_id\":\"...\",\"action\":\"poll\"}, {\"session_id\":\"...\",\"action\":\"write\",\"chars\":\"...\"}, {\"session_id\":\"...\",\"action\":\"key\",\"key\":\"ArrowDown\"}, or {\"session_id\":\"...\",\"action\":\"submit\"}.".to_string(),
                )
            }
        }
        Some(Value::String(command)) => Some(format!(
            "Tool call rejected: `shell` does not accept a bare string in `function.arguments`. Retry with this JSON object instead: {}",
            command_object(command)
        )),
        None => Some(format!(
            "Tool call rejected: `shell` does not accept a bare string in `function.arguments`. Retry with this JSON object instead: {}",
            command_object(trimmed.to_string())
        )),
        _ => Some(
            "Tool call rejected: `shell` requires `function.arguments` to be a JSON object like {\"command\":\"pwd\"}, {\"session_id\":\"...\",\"action\":\"poll\"}, {\"session_id\":\"...\",\"action\":\"write\",\"chars\":\"...\"}, {\"session_id\":\"...\",\"action\":\"key\",\"key\":\"ArrowDown\"}, or {\"session_id\":\"...\",\"action\":\"submit\"}.".to_string(),
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
    fn shell_repair_message_uses_explicit_session_actions() {
        let message = tool_call_argument_repair_message("shell", r#"{"foo":"bar"}"#)
            .expect("invalid shell object should be rejected");

        assert!(message.contains(r#""action":"poll""#));
        assert!(message.contains(r#""action":"write""#));
        assert!(message.contains(r#""action":"key""#));
        assert!(message.contains(r#""action":"submit""#));
        assert!(!message.contains(r#""chars":"\n""#));
    }

    #[test]
    fn accepts_valid_shell_command_object() {
        assert!(tool_call_argument_repair_message("shell", r#"{"command":"pwd"}"#).is_none());
    }

    #[test]
    fn accepts_valid_shell_cmd_alias_object() {
        assert!(tool_call_argument_repair_message("shell", r#"{"cmd":"pwd"}"#).is_none());
    }

    #[test]
    fn accepts_shell_session_continuation_without_command() {
        assert!(tool_call_argument_repair_message(
            "shell",
            r#"{"session_id":"abc123","chars":"\n"}"#
        )
        .is_none());
    }

    #[test]
    fn accepts_shell_session_cancel_without_command() {
        assert!(tool_call_argument_repair_message(
            "shell",
            r#"{"process_id":"abc123","action":"cancel"}"#
        )
        .is_none());
    }

    #[test]
    fn accepts_shell_action_without_explicit_session_for_active_session_repair() {
        assert!(tool_call_argument_repair_message("shell", r#"{"action":"poll"}"#).is_none());
    }

    #[test]
    fn accepts_shell_stdin_without_explicit_session_for_active_session_repair() {
        assert!(tool_call_argument_repair_message("shell", r#"{"chars":"\n"}"#).is_none());
    }

    #[test]
    fn accepts_empty_shell_arguments_for_active_session_repair() {
        assert!(tool_call_argument_repair_message("shell", "").is_none());
    }
}

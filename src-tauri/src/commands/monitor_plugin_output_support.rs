use serde_json::Value;

pub(crate) fn extract_plugin_failure(output: &Value) -> Option<String> {
    match output.get("success").and_then(Value::as_bool) {
        Some(false) => Some(
            output
                .get("error")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("plugin returned success=false without an error message")
                .to_string(),
        ),
        _ => None,
    }
}

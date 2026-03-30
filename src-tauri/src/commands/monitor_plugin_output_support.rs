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

pub(crate) fn extract_plugin_summary(output: &Value) -> Option<&Value> {
    let data = output.get("data").unwrap_or(output);
    data.get("summary").filter(|summary| summary.is_object())
}

pub(crate) fn extract_service_probe_engine_used(output: &Value) -> Option<String> {
    extract_plugin_summary(output)
        .and_then(|summary| summary.get("probeEngineUsed"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

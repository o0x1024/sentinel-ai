use crate::services::PluginMainCategory;
use sentinel_plugins::MonitorSeedBinding;

fn normalize_optional_monitor_type(monitor_type: Option<String>) -> Result<Option<String>, String> {
    Ok(monitor_type
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty()))
}

fn normalize_optional_input_mode(input_mode: Option<String>) -> Result<Option<String>, String> {
    let Some(value) = input_mode else {
        return Ok(None);
    };

    match value.trim().to_ascii_lowercase().as_str() {
        "asset" => Ok(Some("asset".to_string())),
        "seed" => Ok(Some("seed".to_string())),
        "hybrid" => Ok(Some("hybrid".to_string())),
        other if other.is_empty() => Ok(None),
        other => Err(format!("unsupported input_mode: {other}")),
    }
}

pub fn validate_plugin_monitor_type(
    main_category: PluginMainCategory,
    monitor_type: Option<String>,
) -> Result<Option<String>, String> {
    let monitor_type = normalize_optional_monitor_type(monitor_type)?;

    if main_category == PluginMainCategory::Agent && monitor_type.is_none() {
        return Err("monitor_type is required for agent plugins".to_string());
    }

    Ok(monitor_type)
}

pub fn validate_plugin_input_mode(
    main_category: PluginMainCategory,
    input_mode: Option<String>,
    seed_bindings: &[MonitorSeedBinding],
) -> Result<Option<String>, String> {
    let input_mode = normalize_optional_input_mode(input_mode)?;

    if !seed_bindings.is_empty() {
        if main_category != PluginMainCategory::Agent {
            return Err("seed_bindings are only supported for agent plugins".to_string());
        }

        if matches!(input_mode.as_deref(), None | Some("asset")) {
            return Err(
                "input_mode must be seed or hybrid when seed_bindings are configured".to_string(),
            );
        }
    }

    Ok(input_mode)
}

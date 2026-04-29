use sentinel_db::DatabaseService;

const PLUGIN_DEFAULT_INPUTS_KEY_PREFIX: &str = "plugin_default_inputs::";

pub fn merge_plugin_default_inputs(
    defaults: &serde_json::Value,
    inputs: &serde_json::Value,
) -> serde_json::Value {
    fn merge_maps(
        defaults: &serde_json::Map<String, serde_json::Value>,
        overrides: &serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Value {
        let mut merged = defaults.clone();

        for (key, override_value) in overrides {
            let next_value = match (merged.get(key), override_value) {
                (
                    Some(serde_json::Value::Object(default_map)),
                    serde_json::Value::Object(override_map),
                ) => merge_maps(default_map, override_map),
                _ => override_value.clone(),
            };
            merged.insert(key.clone(), next_value);
        }

        serde_json::Value::Object(merged)
    }

    match (defaults, inputs) {
        (serde_json::Value::Object(default_map), serde_json::Value::Object(input_map)) => {
            merge_maps(default_map, input_map)
        }
        (serde_json::Value::Object(_), serde_json::Value::Null) => defaults.clone(),
        _ => inputs.clone(),
    }
}

pub async fn load_plugin_default_inputs(
    db: &DatabaseService,
    plugin_id: &str,
) -> serde_json::Value {
    let key = format!("{}{}", PLUGIN_DEFAULT_INPUTS_KEY_PREFIX, plugin_id.trim());
    let Ok(Some(raw)) = db.load_proxy_config(&key).await else {
        return serde_json::json!({});
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return serde_json::json!({});
    };
    match parsed {
        serde_json::Value::Object(_) => parsed,
        _ => serde_json::json!({}),
    }
}

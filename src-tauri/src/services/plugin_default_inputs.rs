use sentinel_db::DatabaseService;
use serde_json::{Map, Value};

const PLUGIN_DEFAULT_INPUTS_KEY_PREFIX: &str = "plugin_default_inputs::";

fn plugin_default_inputs_storage_key(plugin_id: &str) -> String {
    format!("{}{}", PLUGIN_DEFAULT_INPUTS_KEY_PREFIX, plugin_id.trim())
}

fn normalize_object_value(value: Value) -> Result<Value, String> {
    if value.is_null() {
        return Ok(Value::Object(Map::new()));
    }

    match value {
        Value::Object(_) => Ok(value),
        _ => Err("Plugin default input config must be a JSON object".to_string()),
    }
}

fn merge_object_values(defaults: &Map<String, Value>, overrides: &Map<String, Value>) -> Value {
    let mut merged = defaults.clone();

    for (key, override_value) in overrides {
        let next_value = match (merged.get(key), override_value) {
            (Some(Value::Object(default_map)), Value::Object(override_map)) => {
                merge_object_values(default_map, override_map)
            }
            _ => override_value.clone(),
        };
        merged.insert(key.clone(), next_value);
    }

    Value::Object(merged)
}

pub fn merge_plugin_input_defaults(defaults: &Value, inputs: &Value) -> Value {
    match (defaults, inputs) {
        (Value::Object(default_map), Value::Object(input_map)) => {
            merge_object_values(default_map, input_map)
        }
        (Value::Object(_), Value::Null) => defaults.clone(),
        _ => inputs.clone(),
    }
}

pub async fn load_plugin_default_inputs(
    db: &DatabaseService,
    plugin_id: &str,
) -> Result<Value, String> {
    let storage_key = plugin_default_inputs_storage_key(plugin_id);
    let raw = db
        .load_proxy_config(&storage_key)
        .await
        .map_err(|error| format!("Failed to load plugin default input config: {error}"))?;

    match raw {
        Some(value) => {
            let parsed = serde_json::from_str::<Value>(&value)
                .map_err(|error| format!("Failed to parse plugin default input config: {error}"))?;
            normalize_object_value(parsed)
        }
        None => Ok(Value::Object(Map::new())),
    }
}

pub async fn save_plugin_default_inputs(
    db: &DatabaseService,
    plugin_id: &str,
    config: &Value,
) -> Result<Value, String> {
    let normalized = normalize_object_value(config.clone())?;
    let storage_key = plugin_default_inputs_storage_key(plugin_id);
    let raw = serde_json::to_string(&normalized)
        .map_err(|error| format!("Failed to serialize plugin default input config: {error}"))?;

    db.save_proxy_config(&storage_key, &raw)
        .await
        .map_err(|error| format!("Failed to save plugin default input config: {error}"))?;

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::merge_plugin_input_defaults;
    use serde_json::json;

    #[test]
    fn merges_nested_objects_without_overwriting_unspecified_defaults() {
        let defaults = json!({
            "config": {
                "limit": 200,
                "dedupe": true,
                "headers": {
                    "x-test": "1"
                }
            },
            "mode": "safe"
        });
        let inputs = json!({
            "config": {
                "dedupe": false,
                "headers": {
                    "x-extra": "2"
                }
            }
        });

        let merged = merge_plugin_input_defaults(&defaults, &inputs);

        assert_eq!(
            merged,
            json!({
                "config": {
                    "limit": 200,
                    "dedupe": false,
                    "headers": {
                        "x-test": "1",
                        "x-extra": "2"
                    }
                },
                "mode": "safe"
            })
        );
    }

    #[test]
    fn returns_defaults_when_runtime_input_is_null() {
        let defaults = json!({
            "config": {
                "limit": 200
            }
        });

        let merged = merge_plugin_input_defaults(&defaults, &serde_json::Value::Null);

        assert_eq!(merged, defaults);
    }
}

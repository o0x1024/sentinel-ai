use serde_json::Value;
use std::collections::HashMap;

use super::{ParameterType, ToolParameters, UnifiedTool};

/// Try to coerce pipeline_input into the expected tool parameters.
/// Rules:
/// - If pipeline_input is an object, copy overlapping keys to inputs
/// - If missing exactly one required param and pipeline_input is a scalar/string/object with a single primitive field,
///   assign it to that required param
/// - Prefer common wrappers: data/result/output -> unwrap to inner content when it's a primitive or a flat map
pub fn map_pipeline_input_to_tool_inputs(
    tool: &dyn UnifiedTool,
    mut inputs: HashMap<String, Value>,
    pipeline_input: Value,
) -> HashMap<String, Value> {
    let params: &ToolParameters = tool.parameters();

    // Helper: choose inner content if wrapped
    fn unwrap_common_wrappers(v: &Value) -> &Value {
        if let Value::Object(map) = v {
            for key in ["data", "result", "output"] {
                if let Some(inner) = map.get(key) {
                    return inner;
                }
            }
        }
        v
    }

    let pip = unwrap_common_wrappers(&pipeline_input);

    // 1) Object-to-object: copy matching keys
    if let Value::Object(obj) = pip {
        for def in &params.parameters {
            if let Some(v) = obj.get(&def.name) {
                inputs.entry(def.name.clone()).or_insert_with(|| v.clone());
            }
        }
        // Early return if we satisfied all required keys
        if all_required_present(params, &inputs) {
            return inputs;
        }
    }

    // 2) Single required param shortcut
    let required: Vec<_> = params.parameters.iter().filter(|p| p.required).collect();
    if required.len() == 1 {
        let key = required[0].name.clone();
        if !inputs.contains_key(&key) {
            let coerced = coerce_scalar(pip, &required[0].param_type);
            inputs.insert(key, coerced);
            return inputs;
        }
    }

    // 3) If no required param, but exactly one parameter total, map to it
    if required.is_empty() && params.parameters.len() == 1 {
        let key = params.parameters[0].name.clone();
        if !inputs.contains_key(&key) {
            let coerced = coerce_scalar(pip, &params.parameters[0].param_type);
            inputs.insert(key, coerced);
            return inputs;
        }
    }

    // 4) Fallback: keep original inputs and inject entire pipeline_input as `pipeline_input`
    inputs.entry("pipeline_input".to_string()).or_insert_with(|| pipeline_input);
    inputs
}

fn all_required_present(params: &ToolParameters, inputs: &HashMap<String, Value>) -> bool {
    params
        .parameters
        .iter()
        .filter(|p| p.required)
        .all(|p| inputs.contains_key(&p.name))
}

fn coerce_scalar(v: &Value, expected: &ParameterType) -> Value {
    match expected {
        ParameterType::String => Value::String(as_string(v)),
        ParameterType::Number => {
            if let Some(n) = v.as_f64() {
                serde_json::Number::from_f64(n).map(Value::Number).unwrap_or_else(|| Value::String(as_string(v)))
            } else {
                // try parse
                if let Ok(n) = as_string(v).parse::<f64>() {
                    serde_json::Number::from_f64(n).map(Value::Number).unwrap_or_else(|| Value::String(as_string(v)))
                } else {
                    Value::String(as_string(v))
                }
            }
        }
        ParameterType::Boolean => {
            if let Some(b) = v.as_bool() {
                Value::Bool(b)
            } else {
                match as_string(v).to_lowercase().as_str() {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    _ => Value::Bool(false),
                }
            }
        }
        ParameterType::Array | ParameterType::Object => v.clone(),
    }
}

fn as_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}



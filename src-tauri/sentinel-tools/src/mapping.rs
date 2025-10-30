use serde_json::Value;
use std::collections::HashMap;

use crate::unified_types::{ParameterType, ToolParameters, UnifiedTool};

pub fn map_pipeline_input_to_tool_inputs(
    tool: &dyn UnifiedTool,
    mut inputs: HashMap<String, Value>,
    pipeline_input: Value,
) -> HashMap<String, Value> {
    let params: &ToolParameters = tool.parameters();

    fn unwrap_common_wrappers(v: &Value) -> &Value {
        if let Value::Object(map) = v {
            for key in ["data", "result", "output"] {
                if let Some(inner) = map.get(key) { return inner; }
            }
        }
        v
    }

    let pip = unwrap_common_wrappers(&pipeline_input);

    if let Value::Object(obj) = pip {
        for def in &params.parameters { if let Some(v) = obj.get(&def.name) { inputs.entry(def.name.clone()).or_insert_with(|| v.clone()); } }
        if all_required_present(params, &inputs) { return inputs; }
    }

    let required: Vec<_> = params.parameters.iter().filter(|p| p.required).collect();
    if required.len() == 1 {
        let key = required[0].name.clone();
        if !inputs.contains_key(&key) { let coerced = coerce_scalar(pip, &required[0].param_type); inputs.insert(key, coerced); return inputs; }
    }

    if required.is_empty() && params.parameters.len() == 1 {
        let key = params.parameters[0].name.clone();
        if !inputs.contains_key(&key) { let coerced = coerce_scalar(pip, &params.parameters[0].param_type); inputs.insert(key, coerced); return inputs; }
    }

    inputs.entry("pipeline_input".to_string()).or_insert_with(|| pipeline_input);
    inputs
}

fn all_required_present(params: &ToolParameters, inputs: &HashMap<String, Value>) -> bool {
    params.parameters.iter().filter(|p| p.required).all(|p| inputs.contains_key(&p.name))
}

fn coerce_scalar(v: &Value, expected: &ParameterType) -> Value {
    match expected {
        ParameterType::String => Value::String(as_string(v)),
        ParameterType::Number => {
            if let Some(n) = v.as_f64() {
                serde_json::Number::from_f64(n).map(Value::Number).unwrap_or_else(|| Value::String(as_string(v)))
            } else {
                if let Ok(n) = as_string(v).parse::<f64>() {
                    serde_json::Number::from_f64(n).map(Value::Number).unwrap_or_else(|| Value::String(as_string(v)))
                } else { Value::String(as_string(v)) }
            }
        }
        ParameterType::Boolean => {
            if let Some(b) = v.as_bool() { Value::Bool(b) } else { match as_string(v).to_lowercase().as_str() { "true" => Value::Bool(true), "false" => Value::Bool(false), _ => Value::Bool(false) } }
        }
        ParameterType::Array | ParameterType::Object => v.clone(),
    }
}

fn as_string(v: &Value) -> String {
    match v { Value::String(s) => s.clone(), Value::Number(n) => n.to_string(), Value::Bool(b) => b.to_string(), other => other.to_string() }
}



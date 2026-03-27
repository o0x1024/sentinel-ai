use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Map, Value};

use crate::commands::EdgeDef;
use crate::engine::{WorkflowEngine, WorkflowStep};

pub const EDGE_SOURCE_SCOPE_OUTPUT: &str = "output";
pub const EDGE_SOURCE_SCOPE_INPUT: &str = "input";
pub const EDGE_MERGE_MODE_REPLACE: &str = "replace";
pub const EDGE_MERGE_MODE_DEEP_MERGE: &str = "deep_merge";
pub const EDGE_MERGE_MODE_APPEND: &str = "append";

pub fn default_edge_source_scope() -> String {
    EDGE_SOURCE_SCOPE_OUTPUT.to_string()
}

pub fn default_edge_merge_mode() -> String {
    EDGE_MERGE_MODE_REPLACE.to_string()
}

fn normalize_path(path: &str) -> Vec<&str> {
    path.trim()
        .trim_start_matches("$.")
        .trim_start_matches('.')
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn extract_value_at_path(value: &Value, path: Option<&str>) -> Option<Value> {
    let Some(path) = path.map(str::trim).filter(|entry| !entry.is_empty()) else {
        return Some(value.clone());
    };

    let mut current = value;
    for segment in normalize_path(path) {
        if let Ok(index) = segment.parse::<usize>() {
            current = current.as_array()?.get(index)?;
        } else {
            current = current.as_object()?.get(segment)?;
        }
    }

    Some(current.clone())
}

fn deep_merge_values(existing: &mut Value, next: Value) {
    match (existing, next) {
        (Value::Object(existing_obj), Value::Object(next_obj)) => {
            for (key, value) in next_obj {
                if let Some(existing_value) = existing_obj.get_mut(&key) {
                    deep_merge_values(existing_value, value);
                } else {
                    existing_obj.insert(key, value);
                }
            }
        }
        (slot, value) => {
            *slot = value;
        }
    }
}

fn merge_value(slot: &mut Value, next: Value, mode: &str) {
    match mode {
        EDGE_MERGE_MODE_APPEND => match slot {
            Value::Array(existing) => existing.push(next),
            Value::Null => *slot = Value::Array(vec![next]),
            _ => {
                let previous = std::mem::replace(slot, Value::Null);
                *slot = Value::Array(vec![previous, next]);
            }
        },
        EDGE_MERGE_MODE_DEEP_MERGE => deep_merge_values(slot, next),
        _ => *slot = next,
    }
}

fn ensure_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    value.as_object_mut().expect("value must be object")
}

fn apply_value_to_target(
    resolved_inputs: &mut Value,
    target_path: &str,
    value: Value,
    merge_mode: &str,
) {
    let segments = normalize_path(target_path);
    if segments.is_empty() {
        return;
    }

    let mut current = resolved_inputs;
    for segment in &segments[..segments.len() - 1] {
        let object = ensure_object(current);
        current = object
            .entry((*segment).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
    }

    let last = segments[segments.len() - 1].to_string();
    let object = ensure_object(current);
    let slot = object.entry(last).or_insert(Value::Null);
    merge_value(slot, value, merge_mode);
}

pub async fn resolve_step_inputs(
    execution_id: &str,
    step: &WorkflowStep,
    edges: &[EdgeDef],
    resolved_inputs_by_step: &HashMap<String, HashMap<String, Value>>,
    engine: &Arc<WorkflowEngine>,
) -> HashMap<String, Value> {
    let mut resolved_inputs = Value::Object(
        step.inputs
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    );

    for edge in edges.iter().filter(|entry| entry.to_node == step.id) {
        let source_root = match edge.source_scope.as_str() {
            EDGE_SOURCE_SCOPE_INPUT => resolved_inputs_by_step
                .get(&edge.from_node)
                .map(|inputs| Value::Object(inputs.clone().into_iter().collect())),
            _ => engine.get_step_result(execution_id, &edge.from_node).await,
        };

        let Some(source_root) = source_root else {
            continue;
        };
        let Some(mapped_value) =
            extract_value_at_path(&source_root, Some(edge.source_path.as_str()))
        else {
            continue;
        };

        let target_path = edge.target_path.trim();
        if target_path.is_empty() {
            tracing::warn!(
                "Skipping edge '{}' because target_path is empty for node '{}'",
                edge.id,
                step.id
            );
            continue;
        }

        apply_value_to_target(
            &mut resolved_inputs,
            target_path,
            mapped_value,
            edge.merge_mode.as_str(),
        );
    }

    resolved_inputs
        .as_object()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect()
}

use serde_json::{json, Value};

use sentinel_db::SystemAgentProfileRecord;

use crate::services::system_agents::sop_registry::{
    resolve_matched_sop_definitions, SystemAgentSopDefinition,
};

const MAX_SOP_CONTEXT_ITEMS: usize = 3;
const MAX_SOP_PROCEDURE_CHARS: usize = 1_600;

pub fn build_logic_sop_context(profile: &SystemAgentProfileRecord, payload: &Value) -> Value {
    if let Some(existing) = extract_existing_logic_sop_context(payload) {
        return Value::Array(existing);
    }

    let definitions = resolve_matched_sop_definitions(profile, payload);
    if definitions.is_empty() {
        return Value::Array(vec![]);
    }

    let contexts = definitions
        .into_iter()
        .take(MAX_SOP_CONTEXT_ITEMS)
        .map(|definition| {
            json!({
                "id": definition.id,
                "name": normalize_name(&definition),
                "description": empty_to_none(&definition.description),
                "procedure": truncate_text(&definition.procedure),
                "updatedAt": empty_to_none(&definition.updated_at),
            })
        })
        .collect::<Vec<_>>();

    Value::Array(contexts)
}

pub fn render_logic_sop_prompt(
    profile: &SystemAgentProfileRecord,
    payload: &Value,
) -> Option<String> {
    let items = build_logic_sop_context(profile, payload);
    let Value::Array(entries) = items else {
        return None;
    };
    if entries.is_empty() {
        return None;
    }

    let blocks = entries
        .into_iter()
        .filter_map(|entry| {
            let id = entry.get("id").and_then(Value::as_str)?.trim().to_string();
            if id.is_empty() {
                return None;
            }

            let name = entry
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(&id)
                .to_string();
            let description = entry
                .get("description")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("");
            let procedure = entry
                .get("procedure")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("");
            if procedure.is_empty() {
                return None;
            }

            let mut block = format!("[SopContentBegin: {} ({})]", name, id);
            if !description.is_empty() {
                block.push_str(&format!("\nDescription: {}", description));
            }
            block.push_str("\nProcedure:");
            block.push_str(&format!("\n{}", procedure));
            block.push_str("\n[SopContentEnd]");
            Some(block)
        })
        .collect::<Vec<_>>();

    if blocks.is_empty() {
        None
    } else {
        Some(blocks.join("\n\n"))
    }
}

fn extract_existing_logic_sop_context(payload: &Value) -> Option<Vec<Value>> {
    let items = payload.get("logicSopContext")?.as_array()?;
    if items.is_empty() {
        return None;
    }
    Some(items.clone())
}

fn normalize_name(definition: &SystemAgentSopDefinition) -> String {
    if definition.name.trim().is_empty() {
        definition.id.clone()
    } else {
        definition.name.trim().to_string()
    }
}

fn empty_to_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn truncate_text(input: &str) -> String {
    input.chars().take(MAX_SOP_PROCEDURE_CHARS).collect()
}

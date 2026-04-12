use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use sentinel_db::SystemAgentProfileRecord;

const MAX_SOP_CONTEXT_ITEMS: usize = 3;
const MAX_SOP_PROCEDURE_CHARS: usize = 1_600;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SystemAgentSopDefinition {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub procedure: String,
    #[serde(default)]
    pub updated_at: String,
}

pub fn parse_system_agent_sop_definitions(raw: &str) -> Vec<SystemAgentSopDefinition> {
    serde_json::from_str::<Vec<SystemAgentSopDefinition>>(raw)
        .unwrap_or_default()
        .into_iter()
        .filter_map(normalize_definition)
        .collect()
}

pub fn serialize_system_agent_sop_definitions(
    definitions: &[SystemAgentSopDefinition],
) -> Result<String, serde_json::Error> {
    let normalized = definitions
        .iter()
        .cloned()
        .filter_map(normalize_definition)
        .collect::<Vec<_>>();
    serde_json::to_string(&normalized)
}

pub fn build_logic_sop_context(profile: &SystemAgentProfileRecord, payload: &Value) -> Value {
    if let Some(existing) = extract_existing_logic_sop_context(payload) {
        return Value::Array(existing);
    }

    let definitions = parse_system_agent_sop_definitions(&profile.sop_definitions_json);
    if definitions.is_empty() {
        return Value::Array(vec![]);
    }

    let matched_ids = extract_matched_sop_ids(payload);
    if matched_ids.is_empty() {
        return Value::Array(vec![]);
    }

    let contexts = definitions
        .into_iter()
        .filter(|definition| matched_ids.contains(&definition.id))
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

pub fn render_logic_sop_prompt(profile: &SystemAgentProfileRecord, payload: &Value) -> Option<String> {
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

fn extract_matched_sop_ids(payload: &Value) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();

    if let Some(items) = payload.get("logicSkillContext").and_then(Value::as_array) {
        for item in items {
            if let Some(id) = item.get("id").and_then(Value::as_str) {
                let normalized = id.trim();
                if !normalized.is_empty() {
                    ids.insert(normalized.to_string());
                }
            }
        }
    }

    if let Some(items) = payload.get("skillRecommendations").and_then(Value::as_array) {
        for item in items {
            if let Some(id) = item.get("id").and_then(Value::as_str) {
                let normalized = id.trim();
                if !normalized.is_empty() {
                    ids.insert(normalized.to_string());
                }
            }
        }
    }

    ids
}

fn normalize_definition(mut definition: SystemAgentSopDefinition) -> Option<SystemAgentSopDefinition> {
    definition.id = definition.id.trim().to_string();
    if definition.id.is_empty() {
        return None;
    }
    definition.name = definition.name.trim().to_string();
    definition.description = definition.description.trim().to_string();
    if definition.updated_at.trim().is_empty() {
        definition.updated_at = chrono::Utc::now().to_rfc3339();
    } else {
        definition.updated_at = definition.updated_at.trim().to_string();
    }
    Some(definition)
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

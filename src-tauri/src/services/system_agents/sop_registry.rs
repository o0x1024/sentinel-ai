use sentinel_db::SystemAgentProfileRecord;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
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

pub fn resolve_matched_sop_definitions(
    profile: &SystemAgentProfileRecord,
    payload: &Value,
) -> Vec<SystemAgentSopDefinition> {
    let matched_ids = collect_matched_sop_ids(payload);
    if matched_ids.is_empty() {
        return Vec::new();
    }

    let profile_definitions = parse_system_agent_sop_definitions(&profile.sop_definitions_json)
        .into_iter()
        .map(|definition| (definition.id.clone(), definition))
        .collect::<std::collections::HashMap<_, _>>();

    matched_ids
        .into_iter()
        .filter_map(|id| profile_definitions.get(&id).cloned())
        .collect()
}

fn collect_matched_sop_ids(payload: &Value) -> Vec<String> {
    let mut ids = Vec::new();

    if let Some(items) = payload.get("logicSkillContext").and_then(Value::as_array) {
        for item in items {
            push_unique_id(&mut ids, item.get("id").and_then(Value::as_str));
        }
    }

    if let Some(items) = payload
        .get("skillRecommendations")
        .and_then(Value::as_array)
    {
        for item in items {
            push_unique_id(&mut ids, item.get("id").and_then(Value::as_str));
        }
    }

    ids
}

fn push_unique_id(ids: &mut Vec<String>, raw: Option<&str>) {
    let Some(raw) = raw else {
        return;
    };
    let normalized = raw.trim();
    if normalized.is_empty() || ids.iter().any(|existing| existing == normalized) {
        return;
    }
    ids.push(normalized.to_string());
}

fn normalize_definition(
    mut definition: SystemAgentSopDefinition,
) -> Option<SystemAgentSopDefinition> {
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

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sentinel_db::SystemAgentProfileRecord;
    use serde_json::json;

    use super::{
        resolve_matched_sop_definitions, serialize_system_agent_sop_definitions,
        SystemAgentSopDefinition,
    };

    fn build_profile(definitions: Vec<SystemAgentSopDefinition>) -> SystemAgentProfileRecord {
        SystemAgentProfileRecord {
            id: "traffic_logic_triage".to_string(),
            name: "Traffic Logic Triage".to_string(),
            description: String::new(),
            mode: "passive".to_string(),
            capability: "triage".to_string(),
            enabled: true,
            trigger_mode: "event".to_string(),
            llm_provider_override: None,
            llm_model_override: None,
            base_prompt_id: None,
            prompt_patch: None,
            sop_definitions_json: serialize_system_agent_sop_definitions(&definitions).unwrap(),
            input_schema_json: "{}".to_string(),
            output_schema_json: "{}".to_string(),
            required_tools_json: "[]".to_string(),
            optional_tools_json: "[]".to_string(),
            forbidden_tools_json: "[]".to_string(),
            trigger_events_json: "[]".to_string(),
            budget_json: "{}".to_string(),
            safety_policy_json: "{}".to_string(),
            cooldown_secs: 0,
            max_concurrency: 1,
            risk_level: "high".to_string(),
            visibility: "system".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn resolves_only_profile_registered_sops() {
        let profile = build_profile(vec![SystemAgentSopDefinition {
            id: "payment-flow".to_string(),
            name: "payment-flow".to_string(),
            description: "profile description".to_string(),
            procedure: "profile procedure".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }]);
        let payload = json!({
            "skillRecommendations": [{"id": "payment-flow"}]
        });

        let resolved = resolve_matched_sop_definitions(&profile, &payload);

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].description, "profile description");
        assert_eq!(resolved[0].procedure, "profile procedure");
    }

    #[test]
    fn does_not_fallback_to_non_registered_sops() {
        let profile = build_profile(Vec::new());
        let payload = json!({
            "logicSkillContext": [{"id": "resource-ownership"}]
        });

        let resolved = resolve_matched_sop_definitions(&profile, &payload);

        assert!(resolved.is_empty());
    }
}

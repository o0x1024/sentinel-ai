use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::services::system_agents::verification_hypothesis_memory::{
    normalize_hypothesis_state, VerificationHypothesisState,
};
use crate::services::system_agents::verification_plan::{normalize_verification_plan, VerificationPlan};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriageBootstrapDecision {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub should_promote: bool,
    #[serde(default)]
    pub risk_type: Option<String>,
    #[serde(default)]
    pub confidence: Option<String>,
    #[serde(default)]
    pub signals: Vec<String>,
    #[serde(default)]
    pub hypothesis_state: VerificationHypothesisState,
    #[serde(default)]
    pub verification_plan: Option<VerificationPlan>,
}

pub fn should_attempt_triage_bootstrap(output: &Value) -> bool {
    output
        .get("verificationPlan")
        .is_none_or(Value::is_null)
        && output
            .get("suggestedNextActions")
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty())
}

pub fn merge_triage_bootstrap_decision(
    output: &Value,
    decision: TriageBootstrapDecision,
) -> Value {
    if !decision.should_promote || decision.verification_plan.is_none() {
        return output.clone();
    }

    let mut enriched = output.clone();
    let Some(object) = enriched.as_object_mut() else {
        return output.clone();
    };

    if should_replace_risk_type(object.get("riskType")) {
        if let Some(risk_type) = decision.risk_type.filter(|value| !value.trim().is_empty()) {
            object.insert("riskType".to_string(), Value::String(risk_type));
        }
    }
    if should_replace_confidence(object.get("confidence")) {
        if let Some(confidence) = decision.confidence.filter(|value| !value.trim().is_empty()) {
            object.insert("confidence".to_string(), Value::String(confidence));
        }
    }
    if object.get("hypothesisState").is_none_or(Value::is_null) {
        let mut hypothesis_state = decision.hypothesis_state;
        normalize_hypothesis_state(&mut hypothesis_state);
        object.insert(
            "hypothesisState".to_string(),
            serde_json::to_value(hypothesis_state).unwrap_or(Value::Null),
        );
    }
    if object.get("verificationPlan").is_none_or(Value::is_null) {
        let mut plan = decision.verification_plan.expect("checked is_some above");
        normalize_verification_plan(&mut plan);
        object.insert(
            "verificationPlan".to_string(),
            serde_json::to_value(plan).unwrap_or(Value::Null),
        );
    }
    merge_string_array_field(object, "signals", &decision.signals);
    if let Some(summary) = append_summary(
        object.get("summary").and_then(Value::as_str),
        &decision.summary,
    ) {
        object.insert("summary".to_string(), Value::String(summary));
    }

    enriched
}

fn append_summary(existing: Option<&str>, addition: &str) -> Option<String> {
    let trimmed = addition.trim();
    if trimmed.is_empty() {
        return existing.map(str::to_string);
    }
    match existing.map(str::trim).filter(|text| !text.is_empty()) {
        Some(summary) if summary.contains(trimmed) => Some(summary.to_string()),
        Some(summary) => Some(format!("{summary}；{trimmed}")),
        None => Some(trimmed.to_string()),
    }
}

fn should_replace_risk_type(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .map(|text| {
            let normalized = text.trim().to_ascii_lowercase();
            normalized.is_empty() || normalized == "none"
        })
        .unwrap_or(true)
}

fn should_replace_confidence(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .map(|text| {
            let normalized = text.trim().to_ascii_lowercase();
            normalized.is_empty() || normalized == "low"
        })
        .unwrap_or(true)
}

fn merge_string_array_field(object: &mut Map<String, Value>, key: &str, additions: &[String]) {
    if additions.is_empty() {
        return;
    }

    let mut merged = object
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for addition in additions {
        if !merged.iter().any(|item| item == addition) {
            merged.push(addition.clone());
        }
    }
    object.insert(
        key.to_string(),
        Value::Array(merged.into_iter().map(Value::String).collect()),
    );
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::services::system_agents::verification_plan::VerificationTarget;

    #[test]
    fn bootstrap_requires_suggestions_and_missing_plan() {
        let output = json!({
            "suggestedNextActions": ["check cart mutation"],
            "verificationPlan": null
        });
        assert!(should_attempt_triage_bootstrap(&output));

        let output = json!({
            "suggestedNextActions": [],
            "verificationPlan": null
        });
        assert!(!should_attempt_triage_bootstrap(&output));

        let output = json!({
            "suggestedNextActions": ["check cart mutation"],
            "verificationPlan": {"preferredStrategy":"repeat_action"}
        });
        assert!(!should_attempt_triage_bootstrap(&output));
    }

    #[test]
    fn merge_promotes_initial_plan_from_llm_decision() {
        let output = json!({
            "summary": "当前返回资金不足。",
            "riskType": "none",
            "confidence": "low",
            "signals": ["baseline denial"],
            "verificationPlan": null
        });
        let decision = TriageBootstrapDecision {
            summary: "已补充为可执行的首个验证计划".to_string(),
            should_promote: true,
            risk_type: Some("logic".to_string()),
            confidence: Some("medium".to_string()),
            signals: vec!["llm selected a business-parameter mutation".to_string()],
            hypothesis_state: VerificationHypothesisState {
                active: vec!["price manipulation".to_string()],
                ..VerificationHypothesisState::default()
            },
            verification_plan: Some(VerificationPlan {
                preferred_strategy: "mutate_business_parameter".to_string(),
                target_request_id: Some(42),
                candidate_targets: vec![VerificationTarget {
                    location: "jsonBody".to_string(),
                    selector: "quantity".to_string(),
                }],
                candidate_parameters: vec!["quantity".to_string()],
                replay_count: None,
                concurrent_requests: None,
                sequence_request_ids: vec![40, 41, 42],
                notes: vec!["llm bootstrap".to_string()],
                parameter_mutations: vec![],
            }),
        };

        let merged = merge_triage_bootstrap_decision(&output, decision);
        assert_eq!(merged.get("riskType").and_then(Value::as_str), Some("logic"));
        assert_eq!(
            merged.get("confidence").and_then(Value::as_str),
            Some("medium")
        );
        assert_eq!(
            merged
                .get("verificationPlan")
                .and_then(|value| value.get("preferredStrategy"))
                .and_then(Value::as_str),
            Some("mutate_business_parameter")
        );
        assert_eq!(
            merged
                .get("hypothesisState")
                .and_then(|value| value.get("active"))
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(Value::as_str),
            Some("price manipulation")
        );
    }

    #[test]
    fn merge_keeps_original_output_when_llm_declines() {
        let output = json!({
            "summary": "待确认",
            "riskType": "none",
            "confidence": "low",
            "signals": [],
            "verificationPlan": null
        });
        let decision = TriageBootstrapDecision {
            summary: "not enough evidence".to_string(),
            should_promote: false,
            risk_type: Some("logic".to_string()),
            confidence: Some("medium".to_string()),
            signals: vec!["ignored".to_string()],
            hypothesis_state: VerificationHypothesisState::default(),
            verification_plan: Some(VerificationPlan::default()),
        };

        assert_eq!(merge_triage_bootstrap_decision(&output, decision), output);
    }
}

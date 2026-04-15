use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VerificationHypothesisState {
    #[serde(default)]
    pub active: Vec<String>,
    #[serde(default)]
    pub strengthened: Vec<String>,
    #[serde(default)]
    pub weakened: Vec<String>,
    #[serde(default)]
    pub exhausted: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

pub fn normalize_hypothesis_state(state: &mut VerificationHypothesisState) {
    normalize_string_list(&mut state.active);
    normalize_string_list(&mut state.strengthened);
    normalize_string_list(&mut state.weakened);
    normalize_string_list(&mut state.exhausted);
    normalize_string_list(&mut state.notes);
}

pub fn extract_hypothesis_state(value: &Value) -> Option<VerificationHypothesisState> {
    let raw = value.get("hypothesisState")?;
    let mut state = serde_json::from_value::<VerificationHypothesisState>(raw.clone()).ok()?;
    normalize_hypothesis_state(&mut state);
    Some(state)
}

pub fn seed_hypothesis_state_from_logic_hypotheses(
    payload: &Value,
) -> Option<VerificationHypothesisState> {
    let hypotheses = payload.get("logicHypotheses")?.as_array()?;
    let mut state = VerificationHypothesisState::default();

    for hypothesis in hypotheses {
        if let Some(label) = logic_hypothesis_label(hypothesis) {
            state.active.push(label);
        }
        if let Some(note) = logic_hypothesis_note(hypothesis) {
            state.notes.push(note);
        }
    }

    normalize_hypothesis_state(&mut state);
    if state.active.is_empty() && state.notes.is_empty() {
        None
    } else {
        Some(state)
    }
}

pub fn derive_triage_hypothesis_state(
    output: &Value,
    payload: &Value,
) -> Option<VerificationHypothesisState> {
    let mut state = extract_hypothesis_state(output)
        .or_else(|| extract_hypothesis_state(payload))
        .or_else(|| seed_hypothesis_state_from_logic_hypotheses(payload))
        .unwrap_or_default();
    let risk_type = output
        .get("riskType")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("none");
    let confidence = output
        .get("confidence")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("low");
    let summary = output
        .get("summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let has_plan = output
        .get("verificationPlan")
        .is_some_and(|value| !value.is_null());
    let has_suggested_actions = output
        .get("suggestedNextActions")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());

    state.notes.push(format!(
        "triage assessment:{}:{}:plan={}:actions={}",
        risk_type, confidence, has_plan, has_suggested_actions
    ));

    match (risk_type.eq_ignore_ascii_case("none"), confidence) {
        (true, "high" | "medium") => {
            if let Some(summary) = summary {
                state.weakened.push(summary.to_string());
            }
            if !has_plan && !has_suggested_actions {
                state.exhausted.push(format!(
                    "triage concluded no actionable hypothesis: {}",
                    summary.unwrap_or("no summary")
                ));
            }
        }
        (true, _) => {
            if let Some(summary) = summary {
                state.notes.push(format!("triage summary: {}", summary));
            }
        }
        (false, "high" | "medium") => {
            if let Some(summary) = summary {
                state.strengthened.push(summary.to_string());
            }
        }
        (false, _) => {
            if let Some(summary) = summary {
                state.notes.push(format!("triage summary: {}", summary));
            }
        }
    }

    if let Some(signals) = output.get("signals").and_then(Value::as_array) {
        for signal in signals.iter().filter_map(Value::as_str).map(str::trim) {
            if signal.is_empty() {
                continue;
            }
            let entry = format!("triage signal: {}", signal);
            if risk_type.eq_ignore_ascii_case("none") {
                if matches!(confidence, "high" | "medium") {
                    state.weakened.push(entry);
                } else {
                    state.notes.push(entry);
                }
            } else if matches!(confidence, "high" | "medium") {
                state.strengthened.push(entry);
            } else {
                state.notes.push(entry);
            }
        }
    }

    normalize_hypothesis_state(&mut state);
    if state.active.is_empty()
        && state.strengthened.is_empty()
        && state.weakened.is_empty()
        && state.exhausted.is_empty()
        && state.notes.is_empty()
    {
        None
    } else {
        Some(state)
    }
}

fn logic_hypothesis_label(hypothesis: &Value) -> Option<String> {
    hypothesis
        .get("summary")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            hypothesis
                .get("id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.replace('_', " "))
        })
}

fn logic_hypothesis_note(hypothesis: &Value) -> Option<String> {
    let id = hypothesis
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let risk_type = hypothesis
        .get("riskType")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("logic");
    let confidence = hypothesis
        .get("confidence")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown");
    Some(format!(
        "seeded from logicHypotheses:{}:{}:{}",
        id, risk_type, confidence
    ))
}

fn normalize_string_list(items: &mut Vec<String>) {
    let mut normalized = Vec::new();
    for item in items.drain(..) {
        let trimmed = item.trim();
        if trimmed.is_empty() || normalized.iter().any(|existing| existing == trimmed) {
            continue;
        }
        normalized.push(trimmed.to_string());
    }
    *items = normalized;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_and_deduplicates_hypothesis_state() {
        let mut state = VerificationHypothesisState {
            active: vec![
                "  price manipulation ".to_string(),
                "price manipulation".to_string(),
            ],
            strengthened: vec!["workflow bypass".to_string(), String::new()],
            weakened: vec!["  ".to_string(), "resource swap".to_string()],
            exhausted: vec!["repeat replay".to_string(), "repeat replay".to_string()],
            notes: vec![" note ".to_string(), "note".to_string()],
        };

        normalize_hypothesis_state(&mut state);

        assert_eq!(state.active, vec!["price manipulation"]);
        assert_eq!(state.strengthened, vec!["workflow bypass"]);
        assert_eq!(state.weakened, vec!["resource swap"]);
        assert_eq!(state.exhausted, vec!["repeat replay"]);
        assert_eq!(state.notes, vec!["note"]);
    }

    #[test]
    fn seeds_hypothesis_state_from_logic_hypotheses() {
        let payload = json!({
            "logicHypotheses": [
                {
                    "id": "price_manipulation",
                    "riskType": "logic",
                    "confidence": "medium",
                    "summary": "Cart total may be manipulable through business parameter tampering."
                },
                {
                    "id": "price_manipulation",
                    "riskType": "logic",
                    "confidence": "medium",
                    "summary": "Cart total may be manipulable through business parameter tampering."
                },
                {
                    "id": "missing_prerequisite_transition",
                    "riskType": "workflow",
                    "confidence": "high"
                }
            ]
        });

        let state = seed_hypothesis_state_from_logic_hypotheses(&payload).expect("seeded");

        assert_eq!(
            state.active,
            vec![
                "Cart total may be manipulable through business parameter tampering.",
                "missing prerequisite transition"
            ]
        );
        assert_eq!(
            state.notes,
            vec![
                "seeded from logicHypotheses:price_manipulation:logic:medium",
                "seeded from logicHypotheses:missing_prerequisite_transition:workflow:high"
            ]
        );
    }

    #[test]
    fn derives_strengthened_memory_from_positive_triage_output() {
        let payload = json!({
            "logicHypotheses": [{
                "id": "missing_prerequisite_transition",
                "riskType": "workflow",
                "confidence": "medium",
                "summary": "Approval may be reachable without prerequisite state."
            }]
        });
        let output = json!({
            "summary": "Observed a workflow gap worth verification.",
            "riskType": "workflow",
            "confidence": "medium",
            "signals": ["target action appears before approval state"]
        });

        let state = derive_triage_hypothesis_state(&output, &payload).expect("state");

        assert!(state
            .strengthened
            .contains(&"Observed a workflow gap worth verification.".to_string()));
        assert!(state
            .strengthened
            .contains(&"triage signal: target action appears before approval state".to_string()));
        assert!(state
            .active
            .contains(&"Approval may be reachable without prerequisite state.".to_string()));
    }

    #[test]
    fn derives_weakened_and_exhausted_memory_from_negative_triage_output() {
        let payload = json!({
            "logicHypotheses": [{
                "id": "cross_identity_resource_access",
                "riskType": "idor",
                "confidence": "high",
                "summary": "A cross-identity object access hypothesis exists."
            }]
        });
        let output = json!({
            "summary": "Current request looks like a normal denial and no actionable follow-up exists.",
            "riskType": "none",
            "confidence": "high",
            "signals": ["denial matches normal business constraint"],
            "suggestedNextActions": [],
            "verificationPlan": null
        });

        let state = derive_triage_hypothesis_state(&output, &payload).expect("state");

        assert!(state.weakened.contains(
            &"Current request looks like a normal denial and no actionable follow-up exists."
                .to_string()
        ));
        assert!(state
            .weakened
            .contains(&"triage signal: denial matches normal business constraint".to_string()));
        assert!(state.exhausted.contains(
            &"triage concluded no actionable hypothesis: Current request looks like a normal denial and no actionable follow-up exists.".to_string()
        ));
    }
}

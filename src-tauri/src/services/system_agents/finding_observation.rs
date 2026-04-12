use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficFindingObservation {
    pub observed_at: DateTime<Utc>,
    pub profile_id: String,
    pub risk_type: String,
    pub confidence: String,
    pub summary: String,
    pub signals: Vec<String>,
    pub verification_plan: Value,
    pub url: String,
    pub method: String,
    pub location: String,
    pub action_kind: String,
    pub total_requests: u64,
    pub distinct_auth_contexts: u64,
    pub response_status: Option<i32>,
    pub has_auth_material: bool,
    pub path_template: Option<String>,
    pub db_request_id: Option<i64>,
    pub verification_plan_executable: bool,
}

impl TrafficFindingObservation {
    pub fn should_promote_to_hypothesis(&self) -> bool {
        if self.risk_type.is_empty() || self.risk_type == "none" {
            return false;
        }

        if self.confidence == "low" {
            return false;
        }

        if is_logic_family(&self.risk_type) {
            if self.verification_plan.is_null() {
                return false;
            }

            if !self.verification_plan_executable {
                return false;
            }

            if self.confidence == "medium" && self.signals.len() < 3 {
                return false;
            }

            if matches!(self.risk_type.as_str(), "bola" | "bfla" | "idor")
                && self.distinct_auth_contexts < 2
                && !self.is_unauthenticated_success_candidate()
            {
                return false;
            }
        }

        true
    }

    fn is_unauthenticated_success_candidate(&self) -> bool {
        !self.has_auth_material
            && self
                .response_status
                .is_some_and(|status| (200..300).contains(&status))
    }
}

pub fn is_logic_family(risk_type: &str) -> bool {
    matches!(
        risk_type,
        "workflow" | "logic" | "bola" | "bfla" | "idor" | "race"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_observation() -> TrafficFindingObservation {
        TrafficFindingObservation {
            observed_at: Utc::now(),
            profile_id: "traffic_logic_triage".to_string(),
            risk_type: "bola".to_string(),
            confidence: "high".to_string(),
            summary: "test".to_string(),
            signals: vec!["signal-a".to_string(), "signal-b".to_string()],
            verification_plan: serde_json::json!({
                "preferredStrategy": "swap_resource_reference",
            }),
            url: "http://example.test/unauthorized_access/?username=bob".to_string(),
            method: "GET".to_string(),
            location: "cluster".to_string(),
            action_kind: "authenticate".to_string(),
            total_requests: 1,
            distinct_auth_contexts: 1,
            response_status: Some(200),
            has_auth_material: false,
            path_template: Some("/unauthorized_access/".to_string()),
            db_request_id: None,
            verification_plan_executable: true,
        }
    }

    #[test]
    fn promotes_single_account_unauthenticated_success_for_object_access_risks() {
        let observation = base_observation();
        assert!(observation.should_promote_to_hypothesis());
    }

    #[test]
    fn keeps_requiring_extra_evidence_when_request_is_not_a_success() {
        let mut observation = base_observation();
        observation.response_status = Some(403);
        assert!(!observation.should_promote_to_hypothesis());
    }

    #[test]
    fn keeps_requiring_extra_evidence_when_auth_material_exists() {
        let mut observation = base_observation();
        observation.has_auth_material = true;
        assert!(!observation.should_promote_to_hypothesis());
    }

    #[test]
    fn blocks_logic_findings_when_plan_cannot_execute_against_baseline() {
        let mut observation = base_observation();
        observation.risk_type = "logic".to_string();
        observation.action_kind = "invoke".to_string();
        observation.response_status = Some(302);
        observation.has_auth_material = true;
        observation.verification_plan_executable = false;
        assert!(!observation.should_promote_to_hypothesis());
    }

    #[test]
    fn promotes_single_request_logic_when_plan_is_executable() {
        let mut observation = base_observation();
        observation.risk_type = "logic".to_string();
        observation.action_kind = "invoke".to_string();
        observation.response_status = Some(302);
        observation.has_auth_material = true;
        observation.verification_plan_executable = true;
        assert!(observation.should_promote_to_hypothesis());
    }
}

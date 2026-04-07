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
    pub path_template: Option<String>,
    pub db_request_id: Option<i64>,
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

            if self.confidence == "medium" && self.signals.len() < 3 {
                return false;
            }

            if matches!(
                self.action_kind.as_str(),
                "create" | "read" | "invoke" | "search" | "synchronize"
            ) && self.total_requests < 3
            {
                return false;
            }

            if matches!(self.risk_type.as_str(), "bola" | "bfla" | "idor")
                && self.distinct_auth_contexts < 2
            {
                return false;
            }
        }

        true
    }
}

pub fn is_logic_family(risk_type: &str) -> bool {
    matches!(
        risk_type,
        "workflow" | "logic" | "bola" | "bfla" | "idor" | "race"
    )
}

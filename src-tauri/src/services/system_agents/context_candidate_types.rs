use serde::{Deserialize, Serialize};

use crate::services::system_agents::TrafficContextExtractionSettings;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ContextDictionaryCandidateCategory {
    Principal,
    Resource,
    AuthHeader,
    AuthToken,
    CookieHint,
    ActionAlias,
}

impl ContextDictionaryCandidateCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Principal => "principal",
            Self::Resource => "resource",
            Self::AuthHeader => "auth_header",
            Self::AuthToken => "auth_token",
            Self::CookieHint => "cookie_hint",
            Self::ActionAlias => "action_alias",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextDictionaryCandidateEvidenceRequest {
    pub request_id: i64,
    pub method: String,
    pub url: String,
    pub matched_locations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextDictionaryCandidate {
    pub key: String,
    pub normalized_key: String,
    pub category: String,
    pub suggested_canonical_action: Option<String>,
    pub confidence: String,
    pub score: i32,
    pub evidence_count: usize,
    pub distinct_value_count: usize,
    pub sources: Vec<String>,
    pub example_values: Vec<String>,
    pub example_locations: Vec<String>,
    pub rule_reasons: Vec<String>,
    pub ai_reason: Option<String>,
    pub behavior_reason: Option<String>,
    pub behavior_evidence: Vec<String>,
    pub evidence_requests: Vec<ContextDictionaryCandidateEvidenceRequest>,
    pub conflict_with_existing: bool,
    pub already_covered_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendTrafficContextDictionaryCandidatesResponse {
    pub candidates: Vec<ContextDictionaryCandidate>,
    pub analyzed_request_count: usize,
    pub skipped_request_count: usize,
    pub existing_settings: TrafficContextExtractionSettings,
}

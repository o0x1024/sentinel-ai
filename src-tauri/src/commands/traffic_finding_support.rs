use serde::Serialize;

use crate::services::system_agents::finding_lifecycle::derive_lifecycle_from_finding;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficFindingView {
    #[serde(flatten)]
    pub finding: sentinel_traffic::VulnerabilityWithEvidence,
    pub analysis_stage: String,
    pub analysis_stage_label: String,
}

impl TrafficFindingView {
    pub fn from_record(finding: sentinel_traffic::VulnerabilityWithEvidence) -> Self {
        let lifecycle = derive_lifecycle_from_finding(&finding);
        Self {
            finding,
            analysis_stage: lifecycle.key().to_string(),
            analysis_stage_label: lifecycle.label().to_string(),
        }
    }
}

pub fn matches_analysis_stage_filters(
    finding: &sentinel_traffic::VulnerabilityWithEvidence,
    analysis_stage_filters: Option<&[String]>,
) -> bool {
    let Some(filters) = analysis_stage_filters else {
        return true;
    };
    if filters.is_empty() {
        return true;
    }

    let lifecycle = derive_lifecycle_from_finding(finding);
    filters
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(lifecycle.key()))
}

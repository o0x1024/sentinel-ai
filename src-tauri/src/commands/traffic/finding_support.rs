use serde::Serialize;

use crate::services::system_agents::finding_lifecycle::derive_lifecycle_from_finding;
use sentinel_traffic::EvidenceRecord;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficFindingView {
    #[serde(flatten)]
    pub finding: sentinel_traffic::VulnerabilityWithEvidence,
    pub analysis_stage: String,
    pub analysis_stage_label: String,
}

impl TrafficFindingView {
    pub fn from_record(mut finding: sentinel_traffic::VulnerabilityWithEvidence) -> Self {
        if let Some(primary_evidence) = select_primary_evidence(&finding.evidence) {
            finding.url = Some(primary_evidence.url.clone());
            finding.method = Some(primary_evidence.method.clone());
        }
        let lifecycle = derive_lifecycle_from_finding(&finding);
        Self {
            finding,
            analysis_stage: lifecycle.key().to_string(),
            analysis_stage_label: lifecycle.label().to_string(),
        }
    }
}

fn select_primary_evidence(evidence: &[EvidenceRecord]) -> Option<&EvidenceRecord> {
    evidence
        .iter()
        .find(|item| !item.location.starts_with("system_agent_"))
        .or_else(|| {
            evidence.iter().find(|item| {
                !matches!(
                    item.location.as_str(),
                    "system_agent_verification" | "system_agent_feedback"
                )
            })
        })
        .or_else(|| evidence.first())
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

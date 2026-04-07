use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrafficFindingLifecycle {
    Hypothesis,
    FormalOpen,
    Verified,
    FalsePositive,
    Fixed,
}

impl TrafficFindingLifecycle {
    pub fn key(self) -> &'static str {
        match self {
            Self::Hypothesis => "hypothesis",
            Self::FormalOpen => "formal_open",
            Self::Verified => "verified",
            Self::FalsePositive => "false_positive",
            Self::Fixed => "fixed",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Hypothesis => "AI 假设",
            Self::FormalOpen => "正式漏洞",
            Self::Verified => "已验证",
            Self::FalsePositive => "误报",
            Self::Fixed => "已修复",
        }
    }

    pub fn vulnerability_status(self) -> &'static str {
        match self {
            Self::Hypothesis => "candidate",
            Self::FormalOpen => "open",
            Self::Verified => "reviewed",
            Self::FalsePositive => "false_positive",
            Self::Fixed => "fixed",
        }
    }

    pub fn formal_statuses() -> Vec<String> {
        vec![
            Self::FormalOpen.vulnerability_status().to_string(),
            Self::Verified.vulnerability_status().to_string(),
            Self::Fixed.vulnerability_status().to_string(),
        ]
    }
}

pub fn initial_lifecycle_for_detection(
    profile_id: &str,
    vuln_type: &str,
) -> TrafficFindingLifecycle {
    if profile_id == "traffic_logic_triage"
        || matches!(
            vuln_type,
            "workflow" | "logic" | "bola" | "bfla" | "idor" | "race"
        )
    {
        return TrafficFindingLifecycle::Hypothesis;
    }

    TrafficFindingLifecycle::FormalOpen
}

pub fn derive_lifecycle_from_finding(
    finding: &sentinel_traffic::VulnerabilityWithEvidence,
) -> TrafficFindingLifecycle {
    let status = finding.vulnerability.status.as_str();
    if status == TrafficFindingLifecycle::FalsePositive.vulnerability_status() {
        return TrafficFindingLifecycle::FalsePositive;
    }
    if status == TrafficFindingLifecycle::Fixed.vulnerability_status() {
        return TrafficFindingLifecycle::Fixed;
    }
    if status == TrafficFindingLifecycle::Verified.vulnerability_status()
        || has_stage(
            finding,
            "system_agent_verification",
            TrafficFindingLifecycle::Verified.key(),
        )
    {
        return TrafficFindingLifecycle::Verified;
    }
    if status == TrafficFindingLifecycle::Hypothesis.vulnerability_status()
        || status == "triaging"
        || has_stage(
            finding,
            "system_agent_context",
            TrafficFindingLifecycle::Hypothesis.key(),
        )
    {
        return TrafficFindingLifecycle::Hypothesis;
    }
    TrafficFindingLifecycle::FormalOpen
}

fn has_stage(
    finding: &sentinel_traffic::VulnerabilityWithEvidence,
    location: &str,
    stage: &str,
) -> bool {
    finding
        .evidence
        .iter()
        .filter(|evidence| evidence.location == location)
        .filter_map(|evidence| evidence.response_headers.as_deref())
        .filter_map(parse_json)
        .any(|meta| meta.get("analysisStage").and_then(Value::as_str) == Some(stage))
}

fn parse_json(raw: &str) -> Option<Value> {
    serde_json::from_str(raw).ok()
}

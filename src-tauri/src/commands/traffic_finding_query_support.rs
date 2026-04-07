use serde_json::Value;

use crate::commands::traffic_finding_support::matches_analysis_stage_filters;

pub fn requires_in_memory_filtering(
    analysis_stage_filters: Option<&[String]>,
    search: Option<&str>,
    semantic_source_filter: Option<&str>,
    hypothesis_risk_type_filter: Option<&str>,
    hypothesis_risk_type_filters: Option<&[String]>,
) -> bool {
    analysis_stage_filters.is_some()
        || normalize_filter(search).is_some()
        || normalize_filter(semantic_source_filter).is_some()
        || normalize_filter(hypothesis_risk_type_filter).is_some()
        || normalize_filters(hypothesis_risk_type_filters).is_some()
}

pub fn matches_finding_filters(
    finding: &sentinel_traffic::VulnerabilityWithEvidence,
    analysis_stage_filters: Option<&[String]>,
    search: Option<&str>,
    semantic_source_filter: Option<&str>,
    hypothesis_risk_type_filter: Option<&str>,
    hypothesis_risk_type_filters: Option<&[String]>,
) -> bool {
    if !matches_analysis_stage_filters(finding, analysis_stage_filters) {
        return false;
    }
    if !matches_search(finding, search) {
        return false;
    }
    if !matches_semantic_source_filter(finding, semantic_source_filter) {
        return false;
    }
    if !matches_hypothesis_filter(
        finding,
        hypothesis_risk_type_filter,
        hypothesis_risk_type_filters,
    ) {
        return false;
    }
    true
}

fn matches_search(finding: &sentinel_traffic::VulnerabilityWithEvidence, search: Option<&str>) -> bool {
    let Some(search) = normalize_filter(search) else {
        return true;
    };
    let needle = search.to_lowercase();
    let lifecycle = crate::services::system_agents::finding_lifecycle::derive_lifecycle_from_finding(
        finding,
    );
    let haystacks = [
        finding.vulnerability.id.as_str(),
        finding.vulnerability.title.as_str(),
        finding.vulnerability.description.as_str(),
        finding.url.as_deref().unwrap_or_default(),
        finding.method.as_deref().unwrap_or_default(),
        finding.vulnerability.status.as_str(),
        finding.vulnerability.severity.as_str(),
        finding.vulnerability.vuln_type.as_str(),
        finding.vulnerability.plugin_id.as_str(),
        lifecycle.key(),
        lifecycle.label(),
    ];

    haystacks
        .iter()
        .any(|value| value.to_lowercase().contains(&needle))
        || finding.evidence.iter().any(|evidence| {
            [
                evidence.url.as_str(),
                evidence.method.as_str(),
                evidence.location.as_str(),
                evidence.evidence_snippet.as_str(),
            ]
            .iter()
            .any(|value| value.to_lowercase().contains(&needle))
        })
}

fn matches_semantic_source_filter(
    finding: &sentinel_traffic::VulnerabilityWithEvidence,
    semantic_source_filter: Option<&str>,
) -> bool {
    let Some(expected) = normalize_filter(semantic_source_filter) else {
        return true;
    };
    let Some(payload) = parse_system_agent_context_payload(finding) else {
        return false;
    };
    payload
        .get("semanticAbstraction")
        .and_then(Value::as_object)
        .and_then(|value| value.get("source"))
        .and_then(Value::as_str)
        .map(|source| source.eq_ignore_ascii_case(expected))
        .unwrap_or(false)
}

fn matches_hypothesis_filter(
    finding: &sentinel_traffic::VulnerabilityWithEvidence,
    hypothesis_risk_type_filter: Option<&str>,
    hypothesis_risk_type_filters: Option<&[String]>,
) -> bool {
    let expected_values = normalize_filters(hypothesis_risk_type_filters)
        .or_else(|| normalize_filter(hypothesis_risk_type_filter).map(|value| vec![value]));
    let Some(expected_values) = expected_values else {
        return true;
    };
    let Some(payload) = parse_system_agent_context_payload(finding) else {
        return false;
    };
    payload
        .get("logicHypotheses")
        .and_then(Value::as_array)
        .map(|items| {
            items.iter().any(|item| {
                item.get("riskType")
                    .and_then(Value::as_str)
                    .map(|risk_type| {
                        expected_values
                            .iter()
                            .any(|expected| risk_type.eq_ignore_ascii_case(expected))
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn parse_system_agent_context_payload(
    finding: &sentinel_traffic::VulnerabilityWithEvidence,
) -> Option<Value> {
    finding
        .evidence
        .iter()
        .find(|evidence| evidence.location == "system_agent_context")
        .and_then(|evidence| evidence.request_body.as_deref())
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
}

fn normalize_filter(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn normalize_filters(values: Option<&[String]>) -> Option<Vec<&str>> {
    values.and_then(|items| {
        let normalized = items
            .iter()
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    fn sample_finding() -> sentinel_traffic::VulnerabilityWithEvidence {
        let now = Utc::now();
        sentinel_traffic::VulnerabilityWithEvidence {
            vulnerability: sentinel_traffic::VulnerabilityRecord {
                id: "vuln-1".to_string(),
                plugin_id: "agent:traffic_logic_triage".to_string(),
                vuln_type: "logic".to_string(),
                severity: "medium".to_string(),
                confidence: "medium".to_string(),
                title: "Order finalize bypass".to_string(),
                description: "Workflow bypass candidate".to_string(),
                cwe: None,
                owasp: None,
                remediation: None,
                status: "candidate".to_string(),
                signature: "sig-1".to_string(),
                first_seen_at: now,
                last_seen_at: now,
                hit_count: 1,
                session_id: None,
                created_at: now,
                updated_at: now,
            },
            evidence: vec![sentinel_traffic::EvidenceRecord {
                id: "e-1".to_string(),
                vuln_id: "vuln-1".to_string(),
                url: "https://target.test/api/orders/123/finalize".to_string(),
                method: "POST".to_string(),
                location: "system_agent_context".to_string(),
                evidence_snippet: "context".to_string(),
                request_headers: None,
                request_body: Some(
                    json!({
                        "semanticAbstraction": {
                            "source": "ai_augmented"
                        },
                        "logicHypotheses": [
                            {
                                "riskType": "workflow",
                                "confidence": "high"
                            }
                        ]
                    })
                    .to_string(),
                ),
                response_status: None,
                response_headers: None,
                response_body: None,
                timestamp: now,
            }],
            url: Some("https://target.test/api/orders/123/finalize".to_string()),
            method: Some("POST".to_string()),
        }
    }

    #[test]
    fn matches_search_across_title_and_url() {
        let finding = sample_finding();
        assert!(matches_finding_filters(
            &finding,
            None,
            Some("finalize"),
            None,
            None,
            None
        ));
        assert!(matches_finding_filters(
            &finding,
            None,
            Some("target.test"),
            None,
            None,
            None
        ));
        assert!(!matches_finding_filters(
            &finding,
            None,
            Some("refund"),
            None,
            None,
            None
        ));
    }

    #[test]
    fn matches_semantic_source_and_hypothesis_filters() {
        let finding = sample_finding();
        assert!(matches_finding_filters(
            &finding,
            None,
            None,
            Some("ai_augmented"),
            Some("workflow"),
            None
        ));
        assert!(!matches_finding_filters(
            &finding,
            None,
            None,
            Some("fallback"),
            Some("workflow"),
            None
        ));
        assert!(!matches_finding_filters(
            &finding,
            None,
            None,
            Some("ai_augmented"),
            Some("race"),
            None
        ));
        assert!(matches_finding_filters(
            &finding,
            None,
            None,
            Some("ai_augmented"),
            None,
            Some(&["race".to_string(), "workflow".to_string()])
        ));
    }
}

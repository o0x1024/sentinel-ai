use chrono::{Duration, Utc};
use serde::Serialize;
use serde_json::Value;
use tauri::State;

use sentinel_traffic::VulnerabilityFilters;

use crate::commands::traffic_analysis_commands::{CommandResponse, TrafficAnalysisState};
use crate::services::system_agents::finding_lifecycle::{
    derive_lifecycle_from_finding, TrafficFindingLifecycle,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficBehaviorEffectStats {
    pub window: String,
    pub total_findings: i64,
    pub behavior_context_findings: i64,
    pub browser_extension_findings: i64,
    pub proxy_inferred_findings: i64,
    pub unknown_mode_findings: i64,
    pub browser_extension_hypotheses: i64,
    pub browser_extension_formal: i64,
    pub browser_extension_verified: i64,
    pub proxy_inferred_hypotheses: i64,
    pub proxy_inferred_formal: i64,
    pub proxy_inferred_verified: i64,
    pub behavior_context_coverage_rate: f64,
    pub browser_extension_share_rate: f64,
}

enum BehaviorEffectMode {
    BrowserExtension,
    ProxyInferred,
    Unknown,
}

#[tauri::command]
pub async fn get_traffic_behavior_effect_stats(
    state: State<'_, TrafficAnalysisState>,
    profile_id: String,
    window: Option<String>,
) -> Result<CommandResponse<TrafficBehaviorEffectStats>, String> {
    let db_service = state.get_db_service();
    let filters = VulnerabilityFilters {
        plugin_id: Some(format!("agent:{profile_id}")),
        ..Default::default()
    };
    let records = db_service
        .list_traffic_vulnerabilities_with_evidence(filters)
        .await
        .map_err(|e| format!("Database error: {e}"))?;

    let window_key = normalize_window(window.as_deref());
    let cutoff = window_cutoff(window_key);

    let mut total_findings = 0;
    let mut behavior_context_findings = 0;
    let mut browser_extension_findings = 0;
    let mut proxy_inferred_findings = 0;
    let mut unknown_mode_findings = 0;
    let mut browser_extension_hypotheses = 0;
    let mut browser_extension_formal = 0;
    let mut browser_extension_verified = 0;
    let mut proxy_inferred_hypotheses = 0;
    let mut proxy_inferred_formal = 0;
    let mut proxy_inferred_verified = 0;

    for record in records {
        if record.vulnerability.last_seen_at < cutoff {
            continue;
        }

        total_findings += 1;
        let lifecycle = derive_lifecycle_from_finding(&record);
        match derive_behavior_mode(&record) {
            BehaviorEffectMode::BrowserExtension => {
                behavior_context_findings += 1;
                browser_extension_findings += 1;
                if lifecycle == TrafficFindingLifecycle::Hypothesis {
                    browser_extension_hypotheses += 1;
                }
                if matches!(
                    lifecycle,
                    TrafficFindingLifecycle::FormalOpen
                        | TrafficFindingLifecycle::Verified
                        | TrafficFindingLifecycle::Fixed
                ) {
                    browser_extension_formal += 1;
                }
                if lifecycle == TrafficFindingLifecycle::Verified {
                    browser_extension_verified += 1;
                }
            }
            BehaviorEffectMode::ProxyInferred => {
                behavior_context_findings += 1;
                proxy_inferred_findings += 1;
                if lifecycle == TrafficFindingLifecycle::Hypothesis {
                    proxy_inferred_hypotheses += 1;
                }
                if matches!(
                    lifecycle,
                    TrafficFindingLifecycle::FormalOpen
                        | TrafficFindingLifecycle::Verified
                        | TrafficFindingLifecycle::Fixed
                ) {
                    proxy_inferred_formal += 1;
                }
                if lifecycle == TrafficFindingLifecycle::Verified {
                    proxy_inferred_verified += 1;
                }
            }
            BehaviorEffectMode::Unknown => {
                unknown_mode_findings += 1;
            }
        }
    }

    let behavior_context_coverage_rate = if total_findings > 0 {
        behavior_context_findings as f64 / total_findings as f64
    } else {
        0.0
    };
    let browser_extension_share_rate = if behavior_context_findings > 0 {
        browser_extension_findings as f64 / behavior_context_findings as f64
    } else {
        0.0
    };

    Ok(CommandResponse::ok(TrafficBehaviorEffectStats {
        window: window_key.to_string(),
        total_findings,
        behavior_context_findings,
        browser_extension_findings,
        proxy_inferred_findings,
        unknown_mode_findings,
        browser_extension_hypotheses,
        browser_extension_formal,
        browser_extension_verified,
        proxy_inferred_hypotheses,
        proxy_inferred_formal,
        proxy_inferred_verified,
        behavior_context_coverage_rate,
        browser_extension_share_rate,
    }))
}

fn normalize_window(value: Option<&str>) -> &'static str {
    match value.unwrap_or("24h").trim() {
        "7d" => "7d",
        "30d" => "30d",
        _ => "24h",
    }
}

fn window_cutoff(window: &str) -> chrono::DateTime<Utc> {
    let now = Utc::now();
    match window {
        "7d" => now - Duration::days(7),
        "30d" => now - Duration::days(30),
        _ => now - Duration::hours(24),
    }
}

fn derive_behavior_mode(
    finding: &sentinel_traffic::VulnerabilityWithEvidence,
) -> BehaviorEffectMode {
    for evidence in &finding.evidence {
        if evidence.location != "system_agent_context" {
            continue;
        }

        let Some(raw_payload) = evidence.request_body.as_deref() else {
            continue;
        };
        let Ok(payload) = serde_json::from_str::<Value>(raw_payload) else {
            continue;
        };

        if let Some(mode) = payload
            .get("behaviorSession")
            .and_then(|value| value.get("effectiveMode"))
            .and_then(Value::as_str)
        {
            return normalize_behavior_mode(mode);
        }
        if let Some(mode) = payload
            .get("behaviorSignal")
            .and_then(|value| value.get("effectiveMode"))
            .and_then(Value::as_str)
        {
            return normalize_behavior_mode(mode);
        }
        if payload
            .get("browserExtensionBehavior")
            .and_then(|value| value.get("events"))
            .and_then(Value::as_array)
            .is_some_and(|events| !events.is_empty())
        {
            return BehaviorEffectMode::BrowserExtension;
        }
    }

    BehaviorEffectMode::Unknown
}

fn normalize_behavior_mode(mode: &str) -> BehaviorEffectMode {
    if mode.eq_ignore_ascii_case("browser_extension") {
        return BehaviorEffectMode::BrowserExtension;
    }
    if mode.eq_ignore_ascii_case("proxy_inferred") {
        return BehaviorEffectMode::ProxyInferred;
    }
    BehaviorEffectMode::Unknown
}

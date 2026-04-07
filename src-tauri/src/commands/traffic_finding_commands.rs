use serde::Serialize;
use tauri::State;

use sentinel_traffic::VulnerabilityFilters;

use crate::commands::traffic_analysis_commands::{CommandResponse, TrafficAnalysisState};
use crate::services::system_agents::finding_lifecycle::{
    derive_lifecycle_from_finding, TrafficFindingLifecycle,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficFindingLifecycleStats {
    pub formal_total: i64,
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
    pub candidate: i64,
    pub verified: i64,
    pub false_positive: i64,
}

#[tauri::command]
pub async fn get_finding_lifecycle_stats(
    state: State<'_, TrafficAnalysisState>,
) -> Result<CommandResponse<TrafficFindingLifecycleStats>, String> {
    let db_service = state.get_db_service();
    let records = db_service
        .list_traffic_vulnerabilities_with_evidence(VulnerabilityFilters::default())
        .await
        .map_err(|e| format!("Database error: {e}"))?;

    let mut formal_total = 0;
    let mut critical = 0;
    let mut high = 0;
    let mut medium = 0;
    let mut low = 0;
    let mut candidate = 0;
    let mut verified = 0;
    let mut false_positive = 0;

    for record in records {
        let lifecycle = derive_lifecycle_from_finding(&record);
        match lifecycle {
            TrafficFindingLifecycle::Hypothesis => {
                candidate += 1;
            }
            TrafficFindingLifecycle::Verified => {
                verified += 1;
                formal_total += 1;
                increment_severity_counter(
                    &record.vulnerability.severity,
                    &mut critical,
                    &mut high,
                    &mut medium,
                    &mut low,
                );
            }
            TrafficFindingLifecycle::FalsePositive => {
                false_positive += 1;
            }
            TrafficFindingLifecycle::Fixed | TrafficFindingLifecycle::FormalOpen => {
                formal_total += 1;
                increment_severity_counter(
                    &record.vulnerability.severity,
                    &mut critical,
                    &mut high,
                    &mut medium,
                    &mut low,
                );
            }
        }
    }

    Ok(CommandResponse::ok(TrafficFindingLifecycleStats {
        formal_total,
        critical,
        high,
        medium,
        low,
        candidate,
        verified,
        false_positive,
    }))
}

fn increment_severity_counter(
    severity: &str,
    critical: &mut i64,
    high: &mut i64,
    medium: &mut i64,
    low: &mut i64,
) {
    match severity {
        "critical" => *critical += 1,
        "high" => *high += 1,
        "medium" => *medium += 1,
        "low" => *low += 1,
        _ => {}
    }
}

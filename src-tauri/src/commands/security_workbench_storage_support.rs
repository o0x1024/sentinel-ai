use serde::Serialize;
use sentinel_traffic::{EvidenceRecord, VulnerabilityRecord};

use crate::commands::traffic::TrafficAnalysisState;

use super::security_workbench_commands::{
    WorkbenchActivityPayload, WorkbenchCasePayload, WorkbenchExecutionDraftPayload,
    WorkbenchExecutionRunPayload, WorkbenchFindingSnapshotPayload, WorkbenchNotePayload,
};

pub(crate) const SECURITY_WORKBENCH_CASES_KEY: &str = "security_workbench_cases_v1";
pub(crate) const SECURITY_WORKBENCH_NOTES_KEY: &str = "security_workbench_notes_v1";
pub(crate) const SECURITY_WORKBENCH_ACTIVITIES_KEY: &str = "security_workbench_activities_v1";
pub(crate) const SECURITY_WORKBENCH_EXECUTION_DRAFTS_KEY: &str =
    "security_workbench_execution_drafts_v1";
pub(crate) const SECURITY_WORKBENCH_EXECUTION_RUNS_KEY: &str =
    "security_workbench_execution_runs_v1";
pub(crate) const SECURITY_WORKBENCH_IGNORED_FINDING_IDS_KEY: &str =
    "security_workbench_ignored_finding_ids_v1";

pub(crate) fn build_workbench_snapshot(
    vulnerability: &VulnerabilityRecord,
    evidence: Vec<EvidenceRecord>,
) -> WorkbenchFindingSnapshotPayload {
    let primary_evidence = select_primary_evidence(&evidence);
    WorkbenchFindingSnapshotPayload {
        id: vulnerability.id.clone(),
        title: vulnerability.title.clone(),
        vuln_type: vulnerability.vuln_type.clone(),
        severity: vulnerability.severity.clone(),
        confidence: vulnerability.confidence.clone(),
        status: vulnerability.status.clone(),
        plugin_id: vulnerability.plugin_id.clone(),
        url: primary_evidence
            .map(|item| item.url.clone())
            .unwrap_or_default(),
        method: primary_evidence.map(|item| item.method.clone()),
        description: vulnerability.description.clone(),
        created_at: vulnerability.created_at.to_rfc3339(),
        updated_at: vulnerability.updated_at.to_rfc3339(),
        first_seen_at: vulnerability.first_seen_at.to_rfc3339(),
        last_seen_at: vulnerability.last_seen_at.to_rfc3339(),
        evidence,
    }
}

pub(crate) async fn load_workbench_cases(
    state: &TrafficAnalysisState,
) -> Result<Vec<WorkbenchCasePayload>, String> {
    match state
        .get_db_service()
        .load_proxy_config(SECURITY_WORKBENCH_CASES_KEY)
        .await
    {
        Ok(Some(raw)) => serde_json::from_str::<Vec<WorkbenchCasePayload>>(&raw)
            .map_err(|error| format!("Failed to parse workbench cases: {error}")),
        Ok(None) => Ok(Vec::new()),
        Err(error) => Err(format!("Failed to load workbench cases: {error}")),
    }
}

pub(crate) async fn save_workbench_cases(
    state: &TrafficAnalysisState,
    cases: &[WorkbenchCasePayload],
) -> Result<(), String> {
    let compacted = cases
        .iter()
        .cloned()
        .map(compact_case_for_storage)
        .collect::<Vec<_>>();
    let raw = serde_json::to_string(&compacted)
        .map_err(|error| format!("Failed to serialize workbench cases: {error}"))?;

    state
        .get_db_service()
        .save_proxy_config(SECURITY_WORKBENCH_CASES_KEY, &raw)
        .await
        .map_err(|error| format!("Failed to save workbench cases: {error}"))
}

pub(crate) async fn load_workbench_notes(
    state: &TrafficAnalysisState,
) -> Result<Vec<WorkbenchNotePayload>, String> {
    match state
        .get_db_service()
        .load_proxy_config(SECURITY_WORKBENCH_NOTES_KEY)
        .await
    {
        Ok(Some(raw)) => serde_json::from_str::<Vec<WorkbenchNotePayload>>(&raw)
            .map_err(|error| format!("Failed to parse workbench notes: {error}")),
        Ok(None) => Ok(Vec::new()),
        Err(error) => Err(format!("Failed to load workbench notes: {error}")),
    }
}

pub(crate) async fn load_workbench_activities(
    state: &TrafficAnalysisState,
) -> Result<Vec<WorkbenchActivityPayload>, String> {
    match state
        .get_db_service()
        .load_proxy_config(SECURITY_WORKBENCH_ACTIVITIES_KEY)
        .await
    {
        Ok(Some(raw)) => serde_json::from_str::<Vec<WorkbenchActivityPayload>>(&raw)
            .map_err(|error| format!("Failed to parse workbench activities: {error}")),
        Ok(None) => Ok(Vec::new()),
        Err(error) => Err(format!("Failed to load workbench activities: {error}")),
    }
}

pub(crate) async fn save_workbench_activities(
    state: &TrafficAnalysisState,
    activities: &[WorkbenchActivityPayload],
) -> Result<(), String> {
    let raw = serde_json::to_string(activities)
        .map_err(|error| format!("Failed to serialize workbench activities: {error}"))?;

    state
        .get_db_service()
        .save_proxy_config(SECURITY_WORKBENCH_ACTIVITIES_KEY, &raw)
        .await
        .map_err(|error| format!("Failed to save workbench activities: {error}"))
}

pub(crate) async fn save_workbench_notes(
    state: &TrafficAnalysisState,
    notes: &[WorkbenchNotePayload],
) -> Result<(), String> {
    let raw = serde_json::to_string(notes)
        .map_err(|error| format!("Failed to serialize workbench notes: {error}"))?;

    state
        .get_db_service()
        .save_proxy_config(SECURITY_WORKBENCH_NOTES_KEY, &raw)
        .await
        .map_err(|error| format!("Failed to save workbench notes: {error}"))
}

pub(crate) async fn load_workbench_execution_drafts(
    state: &TrafficAnalysisState,
) -> Result<Vec<WorkbenchExecutionDraftPayload>, String> {
    match state
        .get_db_service()
        .load_proxy_config(SECURITY_WORKBENCH_EXECUTION_DRAFTS_KEY)
        .await
    {
        Ok(Some(raw)) => serde_json::from_str::<Vec<WorkbenchExecutionDraftPayload>>(&raw)
            .map_err(|error| format!("Failed to parse workbench execution drafts: {error}")),
        Ok(None) => Ok(Vec::new()),
        Err(error) => Err(format!(
            "Failed to load workbench execution drafts: {error}"
        )),
    }
}

pub(crate) async fn save_workbench_execution_drafts(
    state: &TrafficAnalysisState,
    drafts: &[WorkbenchExecutionDraftPayload],
) -> Result<(), String> {
    let raw = serde_json::to_string(drafts)
        .map_err(|error| format!("Failed to serialize workbench execution drafts: {error}"))?;

    state
        .get_db_service()
        .save_proxy_config(SECURITY_WORKBENCH_EXECUTION_DRAFTS_KEY, &raw)
        .await
        .map_err(|error| format!("Failed to save workbench execution drafts: {error}"))
}

pub(crate) async fn load_workbench_execution_runs(
    state: &TrafficAnalysisState,
) -> Result<Vec<WorkbenchExecutionRunPayload>, String> {
    match state
        .get_db_service()
        .load_proxy_config(SECURITY_WORKBENCH_EXECUTION_RUNS_KEY)
        .await
    {
        Ok(Some(raw)) => serde_json::from_str::<Vec<WorkbenchExecutionRunPayload>>(&raw)
            .map_err(|error| format!("Failed to parse workbench execution runs: {error}")),
        Ok(None) => Ok(Vec::new()),
        Err(error) => Err(format!("Failed to load workbench execution runs: {error}")),
    }
}

pub(crate) async fn save_workbench_execution_runs(
    state: &TrafficAnalysisState,
    runs: &[WorkbenchExecutionRunPayload],
) -> Result<(), String> {
    let raw = serde_json::to_string(runs)
        .map_err(|error| format!("Failed to serialize workbench execution runs: {error}"))?;

    state
        .get_db_service()
        .save_proxy_config(SECURITY_WORKBENCH_EXECUTION_RUNS_KEY, &raw)
        .await
        .map_err(|error| format!("Failed to save workbench execution runs: {error}"))
}

pub(crate) async fn load_workbench_ignored_finding_ids(
    state: &TrafficAnalysisState,
) -> Result<Vec<String>, String> {
    match state
        .get_db_service()
        .load_proxy_config(SECURITY_WORKBENCH_IGNORED_FINDING_IDS_KEY)
        .await
    {
        Ok(Some(raw)) => serde_json::from_str::<Vec<String>>(&raw)
            .map_err(|error| format!("Failed to parse workbench ignored finding ids: {error}")),
        Ok(None) => Ok(Vec::new()),
        Err(error) => Err(format!(
            "Failed to load workbench ignored finding ids: {error}"
        )),
    }
}

pub(crate) async fn save_workbench_ignored_finding_ids(
    state: &TrafficAnalysisState,
    finding_ids: &[String],
) -> Result<(), String> {
    let raw = serde_json::to_string(finding_ids)
        .map_err(|error| format!("Failed to serialize workbench ignored finding ids: {error}"))?;

    state
        .get_db_service()
        .save_proxy_config(SECURITY_WORKBENCH_IGNORED_FINDING_IDS_KEY, &raw)
        .await
        .map_err(|error| format!("Failed to save workbench ignored finding ids: {error}"))
}

pub(crate) async fn load_finding_snapshot(
    state: &TrafficAnalysisState,
    finding_id: &str,
) -> Result<Option<WorkbenchFindingSnapshotPayload>, String> {
    let db = state.get_db_service();
    let vulnerability = db
        .get_traffic_vulnerability_by_id(finding_id)
        .await
        .map_err(|error| format!("Failed to fetch finding for workbench: {error}"))?;

    let Some(vulnerability) = vulnerability else {
        return Ok(None);
    };

    let evidence = db
        .get_traffic_evidence_by_vuln_id(finding_id)
        .await
        .map_err(|error| format!("Failed to fetch finding evidence for workbench: {error}"))?;

    Ok(Some(build_workbench_snapshot(&vulnerability, evidence)))
}

pub(crate) async fn refresh_workbench_case_snapshot(
    state: &TrafficAnalysisState,
    case_item: &WorkbenchCasePayload,
) -> Result<WorkbenchCasePayload, String> {
    let Some(snapshot) = load_finding_snapshot(state, &case_item.finding_id).await? else {
        return Ok(case_item.clone());
    };

    let mut next_case = case_item.clone();
    next_case.title = snapshot.title.clone();
    next_case.priority = workbench_priority_for_severity(&snapshot.severity);
    next_case.finding = snapshot.clone();
    if next_case.baseline_evidence_id.is_none() {
        next_case.baseline_evidence_id = default_baseline_evidence_id(&snapshot);
    }
    Ok(next_case)
}

pub(crate) fn create_workbench_id(prefix: &str) -> String {
    format!(
        "{}-{}-{}",
        prefix,
        chrono::Utc::now().timestamp_millis(),
        uuid::Uuid::new_v4().simple()
    )
}

pub(crate) fn payload_changed<T: Serialize>(left: &T, right: &T) -> bool {
    serde_json::to_value(left).ok() != serde_json::to_value(right).ok()
}

pub(crate) fn default_baseline_evidence_id(
    snapshot: &WorkbenchFindingSnapshotPayload,
) -> Option<String> {
    select_primary_evidence(&snapshot.evidence)
        .map(|item| item.id.clone())
        .or_else(|| snapshot.evidence.first().map(|item| item.id.clone()))
}

pub(crate) fn workbench_priority_for_severity(severity: &str) -> String {
    match severity {
        "critical" | "high" => "high".to_string(),
        "medium" => "medium".to_string(),
        _ => "low".to_string(),
    }
}

fn compact_case_for_storage(mut case_item: WorkbenchCasePayload) -> WorkbenchCasePayload {
    case_item.finding.evidence.clear();
    case_item
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

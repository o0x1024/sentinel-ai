use sentinel_traffic::EvidenceRecord;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchFindingSnapshotPayload {
    pub id: String,
    pub title: String,
    pub vuln_type: String,
    pub severity: String,
    pub confidence: String,
    pub status: String,
    pub plugin_id: String,
    pub url: String,
    pub method: Option<String>,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub evidence: Vec<EvidenceRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchCasePayload {
    pub id: String,
    pub finding_id: String,
    pub title: String,
    pub status: String,
    pub current_conclusion: String,
    pub priority: String,
    pub baseline_evidence_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_activity_at: String,
    pub finding: WorkbenchFindingSnapshotPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchNotePayload {
    pub id: String,
    pub case_id: String,
    pub kind: String,
    pub body: String,
    pub created_at: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchActivityPayload {
    pub id: String,
    pub case_id: String,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub created_at: String,
    pub actor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchCaseListItemPayload {
    #[serde(flatten)]
    pub case_item: WorkbenchCasePayload,
    pub note_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchCaseListResponsePayload {
    pub items: Vec<WorkbenchCaseListItemPayload>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchCaseDetailPayload {
    pub case_item: WorkbenchCasePayload,
    pub activities: Vec<WorkbenchActivityPayload>,
    pub notes: Vec<WorkbenchNotePayload>,
    pub execution_drafts: Vec<WorkbenchExecutionDraftPayload>,
    pub execution_runs: Vec<WorkbenchExecutionRunPayload>,
    pub verifier_runs: Vec<WorkbenchVerifierRunPayload>,
    pub assessment_suggestion: Option<WorkbenchAssessmentSuggestionPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchFindingSyncResultPayload {
    pub case_id: String,
    pub finding_id: String,
    pub case_status: String,
    pub previous_finding_status: String,
    pub next_finding_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchReplayPlanStepPayload {
    pub id: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchReplayPlanStopConditionPayload {
    pub id: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchReplayPlanPayload {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub strategy: String,
    pub severity: String,
    pub read_only: bool,
    pub target_evidence_id: String,
    pub target_field: String,
    pub target_method: String,
    pub target_url: String,
    pub candidate_values: Vec<String>,
    pub steps: Vec<WorkbenchReplayPlanStepPayload>,
    pub stop_conditions: Vec<WorkbenchReplayPlanStopConditionPayload>,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchExecutionDraftPayload {
    pub id: String,
    pub case_id: String,
    pub plan_id: String,
    pub title: String,
    pub status: String,
    pub read_only: bool,
    pub severity: String,
    pub target_evidence_id: String,
    pub target_field: String,
    pub target_method: String,
    pub target_url: String,
    pub candidate_values: Vec<String>,
    pub steps: Vec<WorkbenchReplayPlanStepPayload>,
    pub stop_conditions: Vec<WorkbenchReplayPlanStopConditionPayload>,
    pub rationale: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchExecutionAttemptPayload {
    pub id: String,
    pub candidate_value: String,
    pub mutated_url: String,
    pub response_status: Option<i32>,
    pub response_snippet: String,
    pub outcome: String,
    pub diff: WorkbenchExecutionDiffPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchExecutionDiffPayload {
    pub matched_status: bool,
    pub matched_body: bool,
    pub similarity_level: String,
    pub baseline_status: Option<i32>,
    pub baseline_length: usize,
    pub response_length: usize,
    pub changed_signals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchExecutionRunPayload {
    pub id: String,
    pub case_id: String,
    pub draft_id: String,
    pub status: String,
    pub method: String,
    pub baseline_url: String,
    pub target_field: String,
    pub summary: String,
    pub attempts: Vec<WorkbenchExecutionAttemptPayload>,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchVerifierRunPayload {
    pub id: String,
    pub status: String,
    pub trigger_event: Option<String>,
    pub strategy: Option<String>,
    pub verified: bool,
    pub response_status: Option<i32>,
    pub summary: String,
    pub evidence_id: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchAssessmentSuggestionPayload {
    pub title: String,
    pub summary: String,
    pub suggested_status: String,
    pub suggested_conclusion: String,
    pub confidence: String,
    pub signals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListSecurityWorkbenchCasesRequest {
    pub search: Option<String>,
    pub status: Option<String>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOrCreateSecurityWorkbenchCaseForFindingRequest {
    pub finding_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSecurityWorkbenchCaseDetailRequest {
    pub case_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSecurityWorkbenchCasePatch {
    pub status: Option<String>,
    pub current_conclusion: Option<String>,
    pub priority: Option<String>,
    pub baseline_evidence_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSecurityWorkbenchCaseRequest {
    pub case_id: String,
    pub patch: UpdateSecurityWorkbenchCasePatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddSecurityWorkbenchNoteRequest {
    pub case_id: String,
    pub kind: String,
    pub body: String,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSecurityWorkbenchCaseToFindingRequest {
    pub case_id: String,
    pub apply_suggestion_to_case: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSecurityWorkbenchExecutionDraftRequest {
    pub case_id: String,
    pub plan: WorkbenchReplayPlanPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSecurityWorkbenchExecutionDraftRequest {
    pub draft_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteSecurityWorkbenchExecutionDraftRequest {
    pub draft_id: String,
    pub confirm_non_readonly: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSecurityWorkbenchCasesRequest {
    pub case_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSecurityWorkbenchCasesResultPayload {
    pub deleted_case_ids: Vec<String>,
    pub deleted_case_count: usize,
    pub deleted_note_count: usize,
    pub deleted_activity_count: usize,
    pub deleted_execution_draft_count: usize,
    pub deleted_execution_run_count: usize,
}

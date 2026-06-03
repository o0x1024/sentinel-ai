use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::commands::command_response_support::CommandResponse;
use crate::commands::security_workbench_commands::{
    delete_workbench_cases_by_ids, DeleteSecurityWorkbenchCasesResultPayload,
    WorkbenchFindingSnapshotPayload,
};
use crate::commands::security_workbench_storage_support::{
    load_finding_snapshot, load_workbench_cases, load_workbench_ignored_finding_ids,
    save_workbench_ignored_finding_ids,
};
use crate::commands::traffic::TrafficAnalysisState;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListSecurityWorkbenchIgnoredFindingsRequest {
    pub search: Option<String>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreSecurityWorkbenchCasesRequest {
    pub case_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSecurityWorkbenchIgnoredFindingsRequest {
    pub finding_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchIgnoredFindingListResponsePayload {
    pub items: Vec<WorkbenchFindingSnapshotPayload>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreSecurityWorkbenchCasesResultPayload {
    pub ignored_finding_ids: Vec<String>,
    pub ignored_count: usize,
    pub deleted_case_ids: Vec<String>,
    pub deleted_case_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSecurityWorkbenchIgnoredFindingsResultPayload {
    pub restored_finding_ids: Vec<String>,
    pub restored_count: usize,
}

fn emit_workbench_changed(
    app_handle: &AppHandle,
    reason: &str,
    case_ids: &[String],
    finding_ids: &[String],
) {
    let _ = app_handle.emit(
        "security-workbench:changed",
        serde_json::json!({
            "reason": reason,
            "caseIds": case_ids,
            "findingIds": finding_ids,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );
}

fn normalize_ids(ids: &[String]) -> Vec<String> {
    ids.iter()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn matches_ignored_search(snapshot: &WorkbenchFindingSnapshotPayload, search: &str) -> bool {
    if search.is_empty() {
        return true;
    }

    let haystack = [
        snapshot.title.as_str(),
        snapshot.vuln_type.as_str(),
        snapshot.url.as_str(),
        snapshot.description.as_str(),
    ]
    .join("\n")
    .to_lowercase();

    haystack.contains(search)
}

fn sort_snapshots_by_last_seen(items: &mut [WorkbenchFindingSnapshotPayload]) {
    items.sort_by(|left, right| right.last_seen_at.cmp(&left.last_seen_at));
}

fn build_ignore_result(
    delete_result: DeleteSecurityWorkbenchCasesResultPayload,
    ignored_finding_ids: Vec<String>,
) -> IgnoreSecurityWorkbenchCasesResultPayload {
    IgnoreSecurityWorkbenchCasesResultPayload {
        ignored_count: ignored_finding_ids.len(),
        ignored_finding_ids,
        deleted_case_ids: delete_result.deleted_case_ids,
        deleted_case_count: delete_result.deleted_case_count,
    }
}

#[tauri::command]
pub async fn security_workbench_list_ignored_findings(
    state: State<'_, TrafficAnalysisState>,
    request: Option<ListSecurityWorkbenchIgnoredFindingsRequest>,
) -> Result<CommandResponse<WorkbenchIgnoredFindingListResponsePayload>, String> {
    let request = request.unwrap_or_default();
    let page = request.page.unwrap_or(1).max(1);
    let page_size = request.page_size.unwrap_or(20).max(1);
    let search = request.search.unwrap_or_default().trim().to_lowercase();

    let ignored_finding_ids = load_workbench_ignored_finding_ids(&state).await?;
    let mut items = Vec::new();
    for finding_id in ignored_finding_ids {
        let Some(snapshot) = load_finding_snapshot(&state, &finding_id).await? else {
            continue;
        };

        if matches_ignored_search(&snapshot, &search) {
            items.push(snapshot);
        }
    }

    sort_snapshots_by_last_seen(&mut items);

    let total = items.len();
    let start = page_size.saturating_mul(page.saturating_sub(1));
    let paged_items = items
        .into_iter()
        .skip(start)
        .take(page_size)
        .collect::<Vec<_>>();

    Ok(CommandResponse::ok(
        WorkbenchIgnoredFindingListResponsePayload {
            items: paged_items,
            total,
            page,
            page_size,
        },
    ))
}

#[tauri::command]
pub async fn security_workbench_ignore_cases(
    app_handle: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    request: IgnoreSecurityWorkbenchCasesRequest,
) -> Result<CommandResponse<IgnoreSecurityWorkbenchCasesResultPayload>, String> {
    let normalized_case_ids = normalize_ids(&request.case_ids);
    if normalized_case_ids.is_empty() {
        return Ok(CommandResponse::ok(build_ignore_result(
            DeleteSecurityWorkbenchCasesResultPayload {
                deleted_case_ids: Vec::new(),
                deleted_case_count: 0,
                deleted_note_count: 0,
                deleted_activity_count: 0,
                deleted_execution_draft_count: 0,
                deleted_execution_run_count: 0,
            },
            Vec::new(),
        )));
    }

    let cases = load_workbench_cases(&state).await?;
    let ignored_finding_ids = cases
        .iter()
        .filter(|item| {
            normalized_case_ids
                .iter()
                .any(|case_id| case_id == &item.id)
        })
        .map(|item| item.finding_id.clone())
        .collect::<Vec<_>>();
    if ignored_finding_ids.is_empty() {
        return Ok(CommandResponse::ok(build_ignore_result(
            DeleteSecurityWorkbenchCasesResultPayload {
                deleted_case_ids: Vec::new(),
                deleted_case_count: 0,
                deleted_note_count: 0,
                deleted_activity_count: 0,
                deleted_execution_draft_count: 0,
                deleted_execution_run_count: 0,
            },
            Vec::new(),
        )));
    }

    let delete_result = delete_workbench_cases_by_ids(&state, &normalized_case_ids).await?;

    let mut stored_ignored_finding_ids = load_workbench_ignored_finding_ids(&state).await?;
    for finding_id in &ignored_finding_ids {
        if !stored_ignored_finding_ids
            .iter()
            .any(|item| item == finding_id)
        {
            stored_ignored_finding_ids.push(finding_id.clone());
        }
    }
    save_workbench_ignored_finding_ids(&state, &stored_ignored_finding_ids).await?;

    emit_workbench_changed(
        &app_handle,
        "case_ignored",
        &delete_result.deleted_case_ids,
        &ignored_finding_ids,
    );

    Ok(CommandResponse::ok(build_ignore_result(
        delete_result,
        ignored_finding_ids,
    )))
}

#[tauri::command]
pub async fn security_workbench_restore_ignored_findings(
    app_handle: AppHandle,
    state: State<'_, TrafficAnalysisState>,
    request: RestoreSecurityWorkbenchIgnoredFindingsRequest,
) -> Result<CommandResponse<RestoreSecurityWorkbenchIgnoredFindingsResultPayload>, String> {
    let normalized_finding_ids = normalize_ids(&request.finding_ids);
    if normalized_finding_ids.is_empty() {
        return Ok(CommandResponse::ok(
            RestoreSecurityWorkbenchIgnoredFindingsResultPayload {
                restored_finding_ids: Vec::new(),
                restored_count: 0,
            },
        ));
    }

    let mut ignored_finding_ids = load_workbench_ignored_finding_ids(&state).await?;
    let before = ignored_finding_ids.len();
    ignored_finding_ids.retain(|finding_id| {
        !normalized_finding_ids
            .iter()
            .any(|target_finding_id| target_finding_id == finding_id)
    });
    let restored_count = before.saturating_sub(ignored_finding_ids.len());
    if restored_count > 0 {
        save_workbench_ignored_finding_ids(&state, &ignored_finding_ids).await?;
        emit_workbench_changed(
            &app_handle,
            "ignored_restored",
            &[],
            &normalized_finding_ids,
        );
    }

    Ok(CommandResponse::ok(
        RestoreSecurityWorkbenchIgnoredFindingsResultPayload {
            restored_finding_ids: normalized_finding_ids,
            restored_count,
        },
    ))
}

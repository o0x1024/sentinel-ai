//! Bug bounty submission timeline and retest commands.

use super::bounty_commands::ensure_bounty_feature;
use chrono::Utc;
use sentinel_db::{BountySubmissionRow, DatabaseService};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

// ============================================================================
// D3: Submission & Retest Operations
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionTimelineEvent {
    pub id: String,
    pub event_type: String, // "submitted", "triaged", "response", "update", "resolved", "retest"
    pub timestamp: String,
    pub content: String,
    pub actor: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTimelineEventRequest {
    pub submission_id: String,
    pub event_type: String,
    pub content: String,
    pub actor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionWithTimeline {
    pub submission: BountySubmissionRow,
    pub timeline: Vec<SubmissionTimelineEvent>,
    pub days_since_submission: i64,
    pub needs_followup: bool,
    pub next_action: Option<String>,
}

/// Add a timeline event to submission
#[tauri::command]
pub async fn bounty_add_submission_timeline_event(
    db_service: State<'_, Arc<DatabaseService>>,
    request: AddTimelineEventRequest,
) -> Result<BountySubmissionRow, String> {
    ensure_bounty_feature()?;

    let mut submission = db_service
        .get_bounty_submission(&request.submission_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Submission not found".to_string())?;

    let now = Utc::now().to_rfc3339();

    // Parse existing timeline
    let mut timeline: Vec<SubmissionTimelineEvent> = submission
        .timeline_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    // Add new event
    let event = SubmissionTimelineEvent {
        id: Uuid::new_v4().to_string(),
        event_type: request.event_type.clone(),
        timestamp: now.clone(),
        content: request.content,
        actor: request.actor,
        metadata: None,
    };

    timeline.push(event);

    // Update submission
    submission.timeline_json = Some(serde_json::to_string(&timeline).unwrap_or_default());
    submission.updated_at = now;

    // Update status if needed
    match request.event_type.as_str() {
        "response" => {
            if submission.status == "submitted" {
                submission.status = "triaged".to_string();
            }
        }
        "resolved" => {
            submission.status = "accepted".to_string();
        }
        _ => {}
    }

    db_service
        .update_bounty_submission(&submission)
        .await
        .map_err(|e| e.to_string())?;
    Ok(submission)
}

/// Get submission with timeline analysis
#[tauri::command]
pub async fn bounty_get_submission_with_timeline(
    db_service: State<'_, Arc<DatabaseService>>,
    submission_id: String,
) -> Result<SubmissionWithTimeline, String> {
    let submission = db_service
        .get_bounty_submission(&submission_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Submission not found".to_string())?;

    let timeline: Vec<SubmissionTimelineEvent> = submission
        .timeline_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    // Calculate days since submission
    let submitted_at_str = submission
        .submitted_at
        .as_deref()
        .unwrap_or(&submission.created_at);
    let submitted_at = chrono::DateTime::parse_from_rfc3339(submitted_at_str)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| Utc::now());
    let days_since = (Utc::now() - submitted_at).num_days();

    // Determine if followup is needed
    let last_response = timeline
        .iter()
        .filter(|e| e.event_type == "response")
        .last();

    let needs_followup = if let Some(resp) = last_response {
        let resp_time = chrono::DateTime::parse_from_rfc3339(&resp.timestamp)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| Utc::now());
        (Utc::now() - resp_time).num_days() > 7
    } else {
        days_since > 7 && submission.status == "submitted"
    };

    // Suggest next action
    let next_action = match submission.status.as_str() {
        "submitted" if days_since > 7 => Some("Send follow-up message".to_string()),
        "triaged" if days_since > 14 => Some("Request update".to_string()),
        "accepted" => Some("Verify fix and close".to_string()),
        "needs_more_info" => Some("Provide additional information".to_string()),
        _ => None,
    };

    Ok(SubmissionWithTimeline {
        submission,
        timeline,
        days_since_submission: days_since,
        needs_followup,
        next_action,
    })
}

/// Get submissions needing followup
#[tauri::command]
pub async fn bounty_get_submissions_needing_followup(
    db_service: State<'_, Arc<DatabaseService>>,
    program_id: Option<String>,
    days_threshold: Option<i64>,
) -> Result<Vec<SubmissionWithTimeline>, String> {
    let submissions = db_service
        .list_bounty_submissions(
            program_id.as_deref(),
            None, // finding_id
            None, // statuses
            None, // search
            None, // sort_by
            None, // sort_dir
            None, // limit
            None, // offset
        )
        .await
        .map_err(|e| e.to_string())?;

    let threshold = days_threshold.unwrap_or(7);
    let mut needs_followup = Vec::new();

    for submission in submissions {
        if submission.status == "accepted"
            || submission.status == "rejected"
            || submission.status == "closed"
        {
            continue;
        }

        let submitted_at_str = submission
            .submitted_at
            .as_deref()
            .unwrap_or(&submission.created_at);
        let submitted_at = chrono::DateTime::parse_from_rfc3339(submitted_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| Utc::now());
        let days_since = (Utc::now() - submitted_at).num_days();

        let timeline: Vec<SubmissionTimelineEvent> = submission
            .timeline_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let default_timestamp = submitted_at_str.to_string();
        let last_activity = timeline
            .last()
            .map(|e| e.timestamp.as_str())
            .unwrap_or(&default_timestamp);

        let last_activity_time = chrono::DateTime::parse_from_rfc3339(last_activity)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| Utc::now());
        let days_since_activity = (Utc::now() - last_activity_time).num_days();

        if days_since_activity >= threshold {
            let next_action = match submission.status.as_str() {
                "submitted" => Some("Send follow-up message".to_string()),
                "triaged" => Some("Request update".to_string()),
                "needs_more_info" => Some("Provide additional information".to_string()),
                _ => Some("Check status".to_string()),
            };

            needs_followup.push(SubmissionWithTimeline {
                submission,
                timeline,
                days_since_submission: days_since,
                needs_followup: true,
                next_action,
            });
        }
    }

    // Sort by days since submission (oldest first)
    needs_followup.sort_by(|a, b| b.days_since_submission.cmp(&a.days_since_submission));

    Ok(needs_followup)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetestRequest {
    pub submission_id: String,
    pub finding_id: String,
    pub workflow_template_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetestResult {
    pub submission_id: String,
    pub finding_id: String,
    pub is_fixed: bool,
    pub retested_at: String,
    pub notes: Option<String>,
    pub workflow_run_id: Option<String>,
}

/// Schedule a retest for a finding
#[tauri::command]
pub async fn bounty_schedule_retest(
    db_service: State<'_, Arc<DatabaseService>>,
    request: RetestRequest,
) -> Result<RetestResult, String> {
    ensure_bounty_feature()?;

    let now = Utc::now().to_rfc3339();

    // Verify submission and finding exist
    let submission = db_service
        .get_bounty_submission(&request.submission_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Submission not found".to_string())?;

    let _finding = db_service
        .get_bounty_finding(&request.finding_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Finding not found".to_string())?;

    // Add timeline event for retest scheduled
    let mut timeline: Vec<SubmissionTimelineEvent> = submission
        .timeline_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    timeline.push(SubmissionTimelineEvent {
        id: Uuid::new_v4().to_string(),
        event_type: "retest".to_string(),
        timestamp: now.clone(),
        content: format!("Retest scheduled for finding {}", request.finding_id),
        actor: None,
        metadata: request
            .workflow_template_id
            .as_ref()
            .map(|id| serde_json::json!({"workflow_template_id": id})),
    });

    let mut updated_submission = submission.clone();
    updated_submission.timeline_json = Some(serde_json::to_string(&timeline).unwrap_or_default());
    updated_submission.updated_at = now.clone();

    db_service
        .update_bounty_submission(&updated_submission)
        .await
        .map_err(|e| e.to_string())?;

    Ok(RetestResult {
        submission_id: request.submission_id,
        finding_id: request.finding_id,
        is_fixed: false, // Will be determined after actual retest
        retested_at: now,
        notes: Some("Retest scheduled".to_string()),
        workflow_run_id: None, // Would be populated if workflow is triggered
    })
}

/// Record retest result
#[tauri::command]
pub async fn bounty_record_retest_result(
    db_service: State<'_, Arc<DatabaseService>>,
    submission_id: String,
    finding_id: String,
    is_fixed: bool,
    notes: Option<String>,
) -> Result<RetestResult, String> {
    ensure_bounty_feature()?;

    let now = Utc::now().to_rfc3339();

    let submission = db_service
        .get_bounty_submission(&submission_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Submission not found".to_string())?;

    let mut finding = db_service
        .get_bounty_finding(&finding_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Finding not found".to_string())?;

    // Update finding status
    if is_fixed {
        finding.status = "fixed".to_string();
        finding.verified_at = Some(now.clone());
    } else {
        finding.status = "not_fixed".to_string();
    }
    finding.updated_at = now.clone();
    db_service
        .update_bounty_finding(&finding)
        .await
        .map_err(|e| e.to_string())?;

    // Add timeline event
    let mut timeline: Vec<SubmissionTimelineEvent> = submission
        .timeline_json
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let result_text = if is_fixed { "Fixed" } else { "Not Fixed" };
    timeline.push(SubmissionTimelineEvent {
        id: Uuid::new_v4().to_string(),
        event_type: "retest".to_string(),
        timestamp: now.clone(),
        content: format!(
            "Retest completed: {} - {}",
            result_text,
            notes.as_deref().unwrap_or("")
        ),
        actor: None,
        metadata: Some(serde_json::json!({"is_fixed": is_fixed})),
    });

    let mut updated_submission = submission;
    updated_submission.timeline_json = Some(serde_json::to_string(&timeline).unwrap_or_default());
    updated_submission.updated_at = now.clone();

    db_service
        .update_bounty_submission(&updated_submission)
        .await
        .map_err(|e| e.to_string())?;

    Ok(RetestResult {
        submission_id,
        finding_id,
        is_fixed,
        retested_at: now,
        notes,
        workflow_run_id: None,
    })
}

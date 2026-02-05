//! Submission management service

use chrono::Utc;
use sentinel_db::{BountySubmissionRow, DatabaseService};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CreateSubmissionInput {
    pub program_id: String,
    pub finding_id: String,
    pub title: String,
    pub vulnerability_type: String,
    pub severity: Option<String>,
    pub cvss_score: Option<f64>,
    pub cwe_id: Option<String>,
    pub description: String,
    pub reproduction_steps: Option<Vec<String>>,
    pub impact: String,
    pub remediation: Option<String>,
    pub evidence_ids: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct UpdateSubmissionInput {
    pub platform_submission_id: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub vulnerability_type: Option<String>,
    pub severity: Option<String>,
    pub cvss_score: Option<f64>,
    pub cwe_id: Option<String>,
    pub description: Option<String>,
    pub reproduction_steps: Option<Vec<String>>,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub evidence_ids: Option<Vec<String>>,
    pub platform_url: Option<String>,
    pub reward_amount: Option<f64>,
    pub reward_currency: Option<String>,
    pub bonus_amount: Option<f64>,
    pub tags: Option<Vec<String>>,
}

pub struct SubmissionDbService;

impl SubmissionDbService {
    pub async fn create_submission(
        db: &DatabaseService,
        input: CreateSubmissionInput,
    ) -> Result<BountySubmissionRow, String> {
        validate_required(&input.title, "title")?;
        validate_required(&input.vulnerability_type, "vulnerability_type")?;
        validate_required(&input.description, "description")?;
        validate_required(&input.impact, "impact")?;

        let now = Utc::now().to_rfc3339();

        let submission = BountySubmissionRow {
            id: Uuid::new_v4().to_string(),
            program_id: input.program_id,
            finding_id: input.finding_id,
            platform_submission_id: None,
            title: input.title,
            status: "draft".to_string(),
            priority: "medium".to_string(),
            vulnerability_type: input.vulnerability_type,
            severity: input.severity.unwrap_or_else(|| "medium".to_string()),
            cvss_score: input.cvss_score,
            cwe_id: input.cwe_id,
            description: input.description,
            reproduction_steps_json: input
                .reproduction_steps
                .map(|s| serde_json::to_string(&s).unwrap_or_default()),
            impact: input.impact,
            remediation: input.remediation,
            evidence_ids_json: input
                .evidence_ids
                .map(|e| serde_json::to_string(&e).unwrap_or_default()),
            platform_url: None,
            reward_amount: None,
            reward_currency: None,
            bonus_amount: None,
            response_time_hours: None,
            resolution_time_hours: None,
            requires_retest: false,
            retest_at: None,
            last_retest_at: None,
            communications_json: None,
            timeline_json: None,
            tags_json: input.tags.map(|t| serde_json::to_string(&t).unwrap_or_default()),
            metadata_json: None,
            created_at: now.clone(),
            submitted_at: None,
            updated_at: now,
            closed_at: None,
            created_by: "user".to_string(),
        };

        db.create_bounty_submission(&submission)
            .await
            .map_err(|e| e.to_string())?;
        Ok(submission)
    }

    pub async fn update_submission(
        db: &DatabaseService,
        id: &str,
        input: UpdateSubmissionInput,
    ) -> Result<bool, String> {
        let existing = db
            .get_bounty_submission(id)
            .await
            .map_err(|e| e.to_string())?;

        let Some(mut submission) = existing else {
            return Ok(false);
        };

        if input.platform_submission_id.is_some() {
            submission.platform_submission_id = input.platform_submission_id;
        }
        if let Some(title) = input.title { submission.title = title; }
        if let Some(status) = input.status {
            if status == "submitted" && submission.submitted_at.is_none() {
                submission.submitted_at = Some(Utc::now().to_rfc3339());
            }
            if ["accepted", "rejected", "duplicate", "closed"].contains(&status.as_str())
                && submission.closed_at.is_none()
            {
                submission.closed_at = Some(Utc::now().to_rfc3339());
            }
            submission.status = status;
        }
        if let Some(priority) = input.priority { submission.priority = priority; }
        if let Some(vulnerability_type) = input.vulnerability_type { submission.vulnerability_type = vulnerability_type; }
        if let Some(severity) = input.severity { submission.severity = severity; }
        if input.cvss_score.is_some() { submission.cvss_score = input.cvss_score; }
        if input.cwe_id.is_some() { submission.cwe_id = input.cwe_id; }
        if let Some(description) = input.description { submission.description = description; }
        if let Some(steps) = input.reproduction_steps {
            submission.reproduction_steps_json = Some(serde_json::to_string(&steps).unwrap_or_default());
        }
        if let Some(impact) = input.impact { submission.impact = impact; }
        if input.remediation.is_some() { submission.remediation = input.remediation; }
        if let Some(evidence_ids) = input.evidence_ids {
            submission.evidence_ids_json = Some(serde_json::to_string(&evidence_ids).unwrap_or_default());
        }
        if input.platform_url.is_some() { submission.platform_url = input.platform_url; }
        if input.reward_amount.is_some() { submission.reward_amount = input.reward_amount; }
        if input.reward_currency.is_some() { submission.reward_currency = input.reward_currency; }
        if input.bonus_amount.is_some() { submission.bonus_amount = input.bonus_amount; }
        if let Some(tags) = input.tags {
            submission.tags_json = Some(serde_json::to_string(&tags).unwrap_or_default());
        }

        submission.updated_at = Utc::now().to_rfc3339();

        db.update_bounty_submission(&submission)
            .await
            .map_err(|e| e.to_string())
    }
}

fn validate_required(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{} is required", field));
    }
    Ok(())
}

//! Submission tracking for bug bounty programs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Submission status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionStatus {
    /// Draft, not yet submitted
    #[default]
    Draft,
    /// Submitted and pending review
    Pending,
    /// Under review by program
    UnderReview,
    /// Requires more information
    NeedsMoreInfo,
    /// Triaged and validated
    Triaged,
    /// Accepted and valid
    Accepted,
    /// Duplicate of existing report
    Duplicate,
    /// Not applicable or invalid
    NotApplicable,
    /// Out of scope
    OutOfScope,
    /// Informational only
    Informational,
    /// Resolved/fixed
    Resolved,
    /// Closed
    Closed,
    /// Bounty awarded
    Awarded,
}

/// Submission priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionPriority {
    /// Low priority
    Low,
    /// Medium priority
    #[default]
    Medium,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// Bug Bounty Submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    /// Unique identifier
    pub id: String,
    /// Associated program ID
    pub program_id: String,
    /// Associated finding ID
    pub finding_id: String,
    /// Platform submission ID (from HackerOne, etc.)
    pub platform_submission_id: Option<String>,
    /// Submission title
    pub title: String,
    /// Current status
    pub status: SubmissionStatus,
    /// Priority level
    pub priority: SubmissionPriority,
    /// Vulnerability type/category
    pub vulnerability_type: String,
    /// Severity (as reported)
    pub severity: String,
    /// CVSS score (if applicable)
    pub cvss_score: Option<f64>,
    /// CWE ID (if applicable)
    pub cwe_id: Option<String>,
    /// Submission description
    pub description: String,
    /// Steps to reproduce
    pub reproduction_steps: Vec<String>,
    /// Impact description
    pub impact: String,
    /// Remediation suggestions
    pub remediation: Option<String>,
    /// Evidence IDs
    pub evidence_ids: Vec<String>,
    /// Platform URL
    pub platform_url: Option<String>,
    /// Reward amount
    pub reward_amount: Option<f64>,
    /// Reward currency
    pub reward_currency: Option<String>,
    /// Bonus amount
    pub bonus_amount: Option<f64>,
    /// Response time (in hours)
    pub response_time_hours: Option<i32>,
    /// Resolution time (in hours)
    pub resolution_time_hours: Option<i32>,
    /// Retest required
    pub requires_retest: bool,
    /// Retest scheduled at
    pub retest_at: Option<DateTime<Utc>>,
    /// Last retest date
    pub last_retest_at: Option<DateTime<Utc>>,
    /// Communication history
    pub communications: Vec<Communication>,
    /// Tags
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Submitted timestamp
    pub submitted_at: Option<DateTime<Utc>>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Closed timestamp
    pub closed_at: Option<DateTime<Utc>>,
    /// Created by user
    pub created_by: String,
}

impl Submission {
    pub fn new(
        program_id: String,
        finding_id: String,
        title: String,
        vulnerability_type: String,
        created_by: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            program_id,
            finding_id,
            platform_submission_id: None,
            title,
            status: SubmissionStatus::default(),
            priority: SubmissionPriority::default(),
            vulnerability_type,
            severity: "Medium".to_string(),
            cvss_score: None,
            cwe_id: None,
            description: String::new(),
            reproduction_steps: Vec::new(),
            impact: String::new(),
            remediation: None,
            evidence_ids: Vec::new(),
            platform_url: None,
            reward_amount: None,
            reward_currency: None,
            bonus_amount: None,
            response_time_hours: None,
            resolution_time_hours: None,
            requires_retest: false,
            retest_at: None,
            last_retest_at: None,
            communications: Vec::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            submitted_at: None,
            updated_at: now,
            closed_at: None,
            created_by,
        }
    }

    /// Mark as submitted
    pub fn submit(&mut self) {
        self.status = SubmissionStatus::Pending;
        self.submitted_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Calculate total reward
    pub fn total_reward(&self) -> f64 {
        self.reward_amount.unwrap_or(0.0) + self.bonus_amount.unwrap_or(0.0)
    }

    /// Add communication entry
    pub fn add_communication(&mut self, sender: String, message: String) {
        self.communications.push(Communication {
            id: Uuid::new_v4().to_string(),
            sender,
            message,
            timestamp: Utc::now(),
        });
        self.updated_at = Utc::now();
    }
}

/// Communication entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Communication {
    pub id: String,
    pub sender: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Create submission request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubmissionRequest {
    pub program_id: String,
    pub finding_id: String,
    pub title: String,
    pub vulnerability_type: String,
    pub severity: String,
    pub cvss_score: Option<f64>,
    pub cwe_id: Option<String>,
    pub description: String,
    pub reproduction_steps: Vec<String>,
    pub impact: String,
    pub remediation: Option<String>,
    pub evidence_ids: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

/// Update submission request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSubmissionRequest {
    pub title: Option<String>,
    pub status: Option<SubmissionStatus>,
    pub priority: Option<SubmissionPriority>,
    pub vulnerability_type: Option<String>,
    pub severity: Option<String>,
    pub cvss_score: Option<f64>,
    pub cwe_id: Option<String>,
    pub description: Option<String>,
    pub reproduction_steps: Option<Vec<String>>,
    pub impact: Option<String>,
    pub remediation: Option<String>,
    pub evidence_ids: Option<Vec<String>>,
    pub platform_submission_id: Option<String>,
    pub platform_url: Option<String>,
    pub reward_amount: Option<f64>,
    pub reward_currency: Option<String>,
    pub bonus_amount: Option<f64>,
    pub requires_retest: Option<bool>,
    pub retest_at: Option<DateTime<Utc>>,
    pub tags: Option<Vec<String>>,
}

/// Submission filter for queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SubmissionFilter {
    pub program_ids: Option<Vec<String>>,
    pub finding_ids: Option<Vec<String>>,
    pub statuses: Option<Vec<SubmissionStatus>>,
    pub priorities: Option<Vec<SubmissionPriority>>,
    pub vulnerability_types: Option<Vec<String>>,
    pub severities: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub requires_retest: Option<bool>,
    pub has_reward: Option<bool>,
    pub search: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
}

/// Submission statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionStats {
    pub total_submissions: i32,
    pub by_status: HashMap<String, i32>,
    pub by_severity: HashMap<String, i32>,
    pub by_program: HashMap<String, i32>,
    pub total_rewards: f64,
    pub average_reward: f64,
    pub average_response_time_hours: f64,
    pub average_resolution_time_hours: f64,
    pub acceptance_rate: f64,
}

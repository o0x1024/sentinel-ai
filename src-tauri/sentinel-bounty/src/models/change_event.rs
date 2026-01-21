//! Change event tracking for ASM (Attack Surface Management)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Change event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeEventType {
    /// New asset discovered
    AssetDiscovered,
    /// Asset removed or disappeared
    AssetRemoved,
    /// Asset property changed
    AssetModified,
    /// DNS record changed
    DnsChange,
    /// Certificate changed
    CertificateChange,
    /// Technology stack changed
    TechnologyChange,
    /// Port status changed
    PortChange,
    /// Service version changed
    ServiceChange,
    /// Content changed (page fingerprint)
    ContentChange,
    /// API endpoint changed
    ApiChange,
    /// Configuration exposed
    ConfigurationExposed,
}

/// Change severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChangeSeverity {
    /// Low priority change
    Low,
    /// Medium priority change
    #[default]
    Medium,
    /// High priority change
    High,
    /// Critical priority change
    Critical,
}

/// Change event status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChangeEventStatus {
    /// New, not yet processed
    #[default]
    New,
    /// Being analyzed
    Analyzing,
    /// Workflow triggered
    WorkflowTriggered,
    /// Review required
    ReviewRequired,
    /// Acknowledged
    Acknowledged,
    /// Resolved
    Resolved,
    /// Ignored
    Ignored,
}

/// Change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEvent {
    /// Unique identifier
    pub id: String,
    /// Associated program ID (if any)
    pub program_id: Option<String>,
    /// Associated asset ID
    pub asset_id: String,
    /// Event type
    pub event_type: ChangeEventType,
    /// Event severity
    pub severity: ChangeSeverity,
    /// Event status
    pub status: ChangeEventStatus,
    /// Event title
    pub title: String,
    /// Event description
    pub description: String,
    /// Old value (before change)
    pub old_value: Option<String>,
    /// New value (after change)
    pub new_value: Option<String>,
    /// Diff content
    pub diff: Option<String>,
    /// Affected scope (URL, domain, etc.)
    pub affected_scope: Option<String>,
    /// Detection method
    pub detection_method: String,
    /// Triggered workflow IDs
    pub triggered_workflows: Vec<String>,
    /// Generated findings
    pub generated_findings: Vec<String>,
    /// Tags
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Risk score (0-100)
    pub risk_score: f64,
    /// Auto-trigger enabled
    pub auto_trigger_enabled: bool,
    /// Created timestamp (when change was detected)
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Resolved timestamp
    pub resolved_at: Option<DateTime<Utc>>,
}

impl ChangeEvent {
    pub fn new(
        asset_id: String,
        event_type: ChangeEventType,
        title: String,
        detection_method: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            program_id: None,
            asset_id,
            event_type,
            severity: ChangeSeverity::default(),
            status: ChangeEventStatus::default(),
            title,
            description: String::new(),
            old_value: None,
            new_value: None,
            diff: None,
            affected_scope: None,
            detection_method,
            triggered_workflows: Vec::new(),
            generated_findings: Vec::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
            risk_score: 0.0,
            auto_trigger_enabled: false,
            created_at: now,
            updated_at: now,
            resolved_at: None,
        }
    }

    /// Calculate risk score based on event properties
    pub fn calculate_risk_score(&mut self) {
        let mut score: f64 = 0.0;

        // Base score from severity
        score += match self.severity {
            ChangeSeverity::Critical => 40.0,
            ChangeSeverity::High => 30.0,
            ChangeSeverity::Medium => 20.0,
            ChangeSeverity::Low => 10.0,
        };

        // Event type importance
        score += match self.event_type {
            ChangeEventType::AssetDiscovered => 20.0,
            ChangeEventType::CertificateChange => 15.0,
            ChangeEventType::ConfigurationExposed => 25.0,
            ChangeEventType::ApiChange => 15.0,
            _ => 10.0,
        };

        // Bonus for new assets or high-value changes
        if matches!(self.event_type, ChangeEventType::AssetDiscovered) {
            score += 15.0;
        }

        self.risk_score = score.min(100.0);
    }
}

/// Create change event request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChangeEventRequest {
    pub program_id: Option<String>,
    pub asset_id: String,
    pub event_type: ChangeEventType,
    pub severity: Option<ChangeSeverity>,
    pub title: String,
    pub description: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub diff: Option<String>,
    pub affected_scope: Option<String>,
    pub detection_method: String,
    pub tags: Option<Vec<String>>,
    pub auto_trigger_enabled: Option<bool>,
}

/// Update change event request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateChangeEventRequest {
    pub status: Option<ChangeEventStatus>,
    pub severity: Option<ChangeSeverity>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub triggered_workflows: Option<Vec<String>>,
    pub generated_findings: Option<Vec<String>>,
}

/// Change event filter
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangeEventFilter {
    pub program_ids: Option<Vec<String>>,
    pub asset_ids: Option<Vec<String>>,
    pub event_types: Option<Vec<ChangeEventType>>,
    pub severities: Option<Vec<ChangeSeverity>>,
    pub statuses: Option<Vec<ChangeEventStatus>>,
    pub tags: Option<Vec<String>>,
    pub min_risk_score: Option<f64>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
}

/// Change event statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEventStats {
    pub total_events: i32,
    pub by_type: HashMap<String, i32>,
    pub by_severity: HashMap<String, i32>,
    pub by_status: HashMap<String, i32>,
    pub pending_review: i32,
    pub average_risk_score: f64,
}

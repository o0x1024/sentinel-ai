//! Finding model for discovered vulnerabilities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Finding status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    /// Newly discovered, not yet reviewed
    #[default]
    New,
    /// Under review/triage
    Triaging,
    /// Confirmed as valid
    Confirmed,
    /// Ready to submit
    ReadyToSubmit,
    /// Submitted to platform
    Submitted,
    /// Marked as duplicate
    Duplicate,
    /// Invalid/not a vulnerability
    Invalid,
    /// Won't fix (by vendor)
    WontFix,
    /// Fixed by vendor
    Fixed,
    /// Archived
    Archived,
}

/// Severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Critical severity
    Critical,
    /// High severity
    High,
    /// Medium severity
    #[default]
    Medium,
    /// Low severity
    Low,
    /// Informational
    Informational,
}

impl Severity {
    pub fn score(&self) -> f64 {
        match self {
            Severity::Critical => 9.0,
            Severity::High => 7.0,
            Severity::Medium => 5.0,
            Severity::Low => 3.0,
            Severity::Informational => 1.0,
        }
    }
}

/// Finding source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FindingSource {
    /// Manual discovery
    Manual,
    /// Traffic analysis plugin
    TrafficPlugin,
    /// Workflow execution
    Workflow,
    /// External scanner
    Scanner,
    /// AI agent
    Agent,
    /// Import from other tool
    Import,
}

impl Default for FindingSource {
    fn default() -> Self {
        FindingSource::Manual
    }
}

/// Vulnerability finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Unique identifier
    pub id: String,
    /// Associated program ID
    pub program_id: Option<String>,
    /// Associated asset ID (from asset management)
    pub asset_id: Option<String>,
    /// Title/summary
    pub title: String,
    /// Vulnerability type (e.g., XSS, SQLi, IDOR)
    pub vuln_type: String,
    /// CWE ID if applicable
    pub cwe_id: Option<String>,
    /// Severity
    pub severity: Severity,
    /// CVSS score (0.0 - 10.0)
    pub cvss_score: Option<f64>,
    /// CVSS vector string
    pub cvss_vector: Option<String>,
    /// Current status
    pub status: FindingStatus,
    /// Affected URL/endpoint
    pub affected_url: Option<String>,
    /// Affected parameter
    pub affected_parameter: Option<String>,
    /// Detailed description
    pub description: String,
    /// Impact description
    pub impact: Option<String>,
    /// Steps to reproduce
    pub reproduction_steps: Vec<String>,
    /// Proof of concept
    pub poc: Option<String>,
    /// Remediation recommendation
    pub remediation: Option<String>,
    /// Evidence IDs
    pub evidence_ids: Vec<String>,
    /// Discovery source
    pub source: FindingSource,
    /// Source reference (plugin ID, workflow ID, etc.)
    pub source_ref: Option<String>,
    /// Fingerprint for deduplication
    pub fingerprint: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Is this a duplicate of another finding?
    pub duplicate_of: Option<String>,
    /// Tags
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Discovered timestamp
    pub discovered_at: DateTime<Utc>,
    /// Submitted timestamp
    pub submitted_at: Option<DateTime<Utc>>,
    /// Resolved timestamp
    pub resolved_at: Option<DateTime<Utc>>,
}

impl Finding {
    pub fn new(title: String, vuln_type: String, description: String) -> Self {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();
        
        // Generate fingerprint from key fields
        let fingerprint = Self::generate_fingerprint(&title, &vuln_type, None, None);
        
        Self {
            id,
            program_id: None,
            asset_id: None,
            title,
            vuln_type,
            cwe_id: None,
            severity: Severity::default(),
            cvss_score: None,
            cvss_vector: None,
            status: FindingStatus::default(),
            affected_url: None,
            affected_parameter: None,
            description,
            impact: None,
            reproduction_steps: Vec::new(),
            poc: None,
            remediation: None,
            evidence_ids: Vec::new(),
            source: FindingSource::default(),
            source_ref: None,
            fingerprint,
            confidence: 1.0,
            duplicate_of: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            discovered_at: now,
            submitted_at: None,
            resolved_at: None,
        }
    }

    /// Generate fingerprint for deduplication
    pub fn generate_fingerprint(
        _title: &str,
        vuln_type: &str,
        url: Option<&str>,
        param: Option<&str>,
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        
        // Normalize URL for fingerprinting
        let normalized_url = url.map(|u| {
            // Remove query parameters for URL normalization
            u.split('?').next().unwrap_or(u).to_lowercase()
        });

        vuln_type.to_lowercase().hash(&mut hasher);
        if let Some(ref u) = normalized_url {
            u.hash(&mut hasher);
        }
        if let Some(p) = param {
            p.to_lowercase().hash(&mut hasher);
        }
        // Don't include title in fingerprint as it may vary

        format!("{:016x}", hasher.finish())
    }

    /// Update fingerprint based on current fields
    pub fn update_fingerprint(&mut self) {
        self.fingerprint = Self::generate_fingerprint(
            &self.title,
            &self.vuln_type,
            self.affected_url.as_deref(),
            self.affected_parameter.as_deref(),
        );
    }

    /// Check if this finding is potentially a duplicate of another
    pub fn is_potential_duplicate(&self, other: &Finding) -> bool {
        self.fingerprint == other.fingerprint
    }
}

/// Create finding request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFindingRequest {
    pub program_id: Option<String>,
    pub asset_id: Option<String>,
    pub title: String,
    pub vuln_type: String,
    pub cwe_id: Option<String>,
    pub severity: Option<Severity>,
    pub cvss_score: Option<f64>,
    pub cvss_vector: Option<String>,
    pub affected_url: Option<String>,
    pub affected_parameter: Option<String>,
    pub description: String,
    pub impact: Option<String>,
    pub reproduction_steps: Option<Vec<String>>,
    pub poc: Option<String>,
    pub remediation: Option<String>,
    pub source: Option<FindingSource>,
    pub source_ref: Option<String>,
    pub confidence: Option<f64>,
    pub tags: Option<Vec<String>>,
}

/// Update finding request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFindingRequest {
    pub program_id: Option<String>,
    pub asset_id: Option<String>,
    pub title: Option<String>,
    pub vuln_type: Option<String>,
    pub cwe_id: Option<String>,
    pub severity: Option<Severity>,
    pub cvss_score: Option<f64>,
    pub cvss_vector: Option<String>,
    pub status: Option<FindingStatus>,
    pub affected_url: Option<String>,
    pub affected_parameter: Option<String>,
    pub description: Option<String>,
    pub impact: Option<String>,
    pub reproduction_steps: Option<Vec<String>>,
    pub poc: Option<String>,
    pub remediation: Option<String>,
    pub confidence: Option<f64>,
    pub duplicate_of: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Finding filter for queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FindingFilter {
    pub program_ids: Option<Vec<String>>,
    pub asset_ids: Option<Vec<String>>,
    pub vuln_types: Option<Vec<String>>,
    pub severities: Option<Vec<Severity>>,
    pub statuses: Option<Vec<FindingStatus>>,
    pub sources: Option<Vec<FindingSource>>,
    pub tags: Option<Vec<String>>,
    pub search: Option<String>,
    pub min_confidence: Option<f64>,
    pub discovered_after: Option<DateTime<Utc>>,
    pub discovered_before: Option<DateTime<Utc>>,
}

/// Finding statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingStats {
    pub total: i32,
    pub by_status: HashMap<String, i32>,
    pub by_severity: HashMap<String, i32>,
    pub by_vuln_type: HashMap<String, i32>,
    pub by_source: HashMap<String, i32>,
    pub new_today: i32,
    pub submitted_this_week: i32,
}

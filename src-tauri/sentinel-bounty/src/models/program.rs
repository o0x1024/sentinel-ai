//! Bug Bounty Program model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Bug Bounty platform types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BountyPlatform {
    /// HackerOne
    HackerOne,
    /// Bugcrowd
    Bugcrowd,
    /// Synack
    Synack,
    /// Intigriti
    Intigriti,
    /// YesWeHack
    YesWeHack,
    /// Chinese SRC platforms
    Src,
    /// Private program
    Private,
    /// Self-hosted
    SelfHosted,
    /// Other platforms
    Other(String),
}

impl Default for BountyPlatform {
    fn default() -> Self {
        BountyPlatform::Private
    }
}

/// Program status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProgramStatus {
    /// Active and accepting submissions
    #[default]
    Active,
    /// Temporarily paused
    Paused,
    /// Program ended
    Ended,
    /// Archived for reference
    Archived,
}

/// Program type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProgramType {
    /// Public bug bounty
    #[default]
    Public,
    /// Private/invite-only
    Private,
    /// Vulnerability Disclosure Program (no bounty)
    Vdp,
}

/// Reward structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RewardStructure {
    /// Currency (USD, CNY, etc.)
    pub currency: String,
    /// Critical severity reward range
    pub critical: Option<RewardRange>,
    /// High severity reward range
    pub high: Option<RewardRange>,
    /// Medium severity reward range
    pub medium: Option<RewardRange>,
    /// Low severity reward range
    pub low: Option<RewardRange>,
    /// Informational reward (if any)
    pub informational: Option<RewardRange>,
    /// Bonus multipliers or special rewards
    pub bonuses: HashMap<String, String>,
}

/// Reward range (min-max)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardRange {
    pub min: f64,
    pub max: f64,
}

/// Bug Bounty Program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BountyProgram {
    /// Unique identifier
    pub id: String,
    /// Program name
    pub name: String,
    /// Organization/company name
    pub organization: String,
    /// Platform hosting the program
    pub platform: BountyPlatform,
    /// Platform-specific program ID/handle
    pub platform_handle: Option<String>,
    /// Program URL
    pub url: Option<String>,
    /// Program type
    pub program_type: ProgramType,
    /// Current status
    pub status: ProgramStatus,
    /// Program description
    pub description: Option<String>,
    /// Reward structure
    pub rewards: RewardStructure,
    /// Response time SLA (in days)
    pub response_sla_days: Option<i32>,
    /// Resolution time SLA (in days)
    pub resolution_sla_days: Option<i32>,
    /// Special rules or notes
    pub rules: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Priority score (for sorting)
    pub priority_score: f64,
    /// Total submissions count
    pub total_submissions: i32,
    /// Accepted submissions count
    pub accepted_submissions: i32,
    /// Total earnings from this program
    pub total_earnings: f64,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity_at: Option<DateTime<Utc>>,
}

impl BountyProgram {
    pub fn new(name: String, organization: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            organization,
            platform: BountyPlatform::default(),
            platform_handle: None,
            url: None,
            program_type: ProgramType::default(),
            status: ProgramStatus::default(),
            description: None,
            rewards: RewardStructure::default(),
            response_sla_days: None,
            resolution_sla_days: None,
            rules: Vec::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
            priority_score: 0.0,
            total_submissions: 0,
            accepted_submissions: 0,
            total_earnings: 0.0,
            created_at: now,
            updated_at: now,
            last_activity_at: None,
        }
    }

    /// Calculate acceptance rate
    pub fn acceptance_rate(&self) -> f64 {
        if self.total_submissions == 0 {
            0.0
        } else {
            (self.accepted_submissions as f64 / self.total_submissions as f64) * 100.0
        }
    }
}

/// Create program request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProgramRequest {
    pub name: String,
    pub organization: String,
    pub platform: Option<BountyPlatform>,
    pub platform_handle: Option<String>,
    pub url: Option<String>,
    pub program_type: Option<ProgramType>,
    pub description: Option<String>,
    pub rewards: Option<RewardStructure>,
    pub rules: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

/// Update program request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProgramRequest {
    pub name: Option<String>,
    pub organization: Option<String>,
    pub platform: Option<BountyPlatform>,
    pub platform_handle: Option<String>,
    pub url: Option<String>,
    pub program_type: Option<ProgramType>,
    pub status: Option<ProgramStatus>,
    pub description: Option<String>,
    pub rewards: Option<RewardStructure>,
    pub response_sla_days: Option<i32>,
    pub resolution_sla_days: Option<i32>,
    pub rules: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub priority_score: Option<f64>,
}

/// Program filter for queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProgramFilter {
    pub platforms: Option<Vec<BountyPlatform>>,
    pub statuses: Option<Vec<ProgramStatus>>,
    pub program_types: Option<Vec<ProgramType>>,
    pub tags: Option<Vec<String>>,
    pub search: Option<String>,
    pub min_priority: Option<f64>,
}

/// Program statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramStats {
    pub total_programs: i32,
    pub active_programs: i32,
    pub by_platform: HashMap<String, i32>,
    pub by_type: HashMap<String, i32>,
    pub total_submissions: i32,
    pub total_accepted: i32,
    pub total_earnings: f64,
}

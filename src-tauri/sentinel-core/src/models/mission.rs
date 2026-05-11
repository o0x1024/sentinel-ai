use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Mission status enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionStatus {
    Draft,
    Active,
    Paused,
    Blocked,
    Completed,
    Failed,
    Archived,
}

impl MissionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Blocked => "blocked",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Archived => "archived",
        }
    }

    pub fn can_transition_to(&self, target: &MissionStatus) -> bool {
        matches!(
            (self, target),
            (Self::Draft, Self::Active)
                | (Self::Draft, Self::Archived)
                | (Self::Active, Self::Paused)
                | (Self::Active, Self::Blocked)
                | (Self::Active, Self::Completed)
                | (Self::Active, Self::Failed)
                | (Self::Active, Self::Archived)
                | (Self::Paused, Self::Active)
                | (Self::Paused, Self::Archived)
                | (Self::Blocked, Self::Active)
                | (Self::Blocked, Self::Archived)
                | (Self::Failed, Self::Active)
                | (Self::Failed, Self::Archived)
                | (Self::Completed, Self::Archived)
        )
    }
}

impl fmt::Display for MissionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for MissionStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "blocked" => Ok(Self::Blocked),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "archived" => Ok(Self::Archived),
            _ => Err(format!("invalid mission status: {s}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionRunStatus {
    Queued,
    Running,
    Succeeded,
    Partial,
    Failed,
    Cancelled,
    TimedOut,
}

impl MissionRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Partial => "partial",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
        }
    }
}

impl fmt::Display for MissionRunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for MissionRunStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "partial" => Ok(Self::Partial),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "timed_out" => Ok(Self::TimedOut),
            _ => Err(format!("invalid mission run status: {s}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionStepStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Blocked,
}

impl MissionStepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Blocked => "blocked",
        }
    }
}

impl fmt::Display for MissionStepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for MissionStepStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            "blocked" => Ok(Self::Blocked),
            _ => Err(format!("invalid mission step status: {s}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Row structs (map directly to SQLite tables)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Mission {
    pub id: String,
    pub title: String,
    pub objective: String,
    pub status: String,
    pub owner_kind: String,
    pub owner_ref: String,
    pub source_json: Option<String>,
    pub delivery_policy_json: Option<String>,
    pub assistant_profile_id: Option<String>,
    pub trigger_json: Option<String>,
    pub step_plan_json: Option<String>,
    pub success_criteria_json: Option<String>,
    pub context_strategy_json: Option<String>,
    pub budget_json: Option<String>,
    pub failure_policy_json: Option<String>,
    pub missed_run_policy: String,
    pub next_run_at: Option<DateTime<Utc>>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub run_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MissionRun {
    pub id: String,
    pub mission_id: String,
    pub run_index: i64,
    pub status: String,
    pub trigger_kind: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub agent_execution_id: Option<String>,
    pub bot_execution_run_id: Option<String>,
    pub assistant_profile_snapshot_json: Option<String>,
    pub tool_config_snapshot_json: Option<String>,
    pub checkpoint_json: Option<String>,
    pub context_injected_json: Option<String>,
    pub result_summary: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MissionStep {
    pub id: String,
    pub run_id: String,
    pub step_index: i64,
    pub description: String,
    pub status: String,
    pub input_json: Option<String>,
    pub output_json: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MissionArtifact {
    pub id: String,
    pub mission_id: String,
    pub run_id: String,
    pub step_id: Option<String>,
    pub artifact_type: String,
    pub storage_kind: String,
    pub uri: String,
    pub size_bytes: Option<i64>,
    pub content_hash: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MissionObservation {
    pub id: String,
    pub mission_id: String,
    pub run_id: String,
    pub step_id: Option<String>,
    pub observation_type: String,
    pub severity: String,
    pub title: String,
    pub summary: Option<String>,
    pub data_json: Option<String>,
    pub artifact_ids_json: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MissionDelivery {
    pub id: String,
    pub mission_id: String,
    pub run_id: String,
    pub target_json: String,
    pub status: String,
    pub message_id: Option<String>,
    pub payload_json: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MissionLock {
    pub mission_id: String,
    pub run_id: String,
    pub lock_owner: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Request / input types for creating missions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMissionRequest {
    pub title: String,
    pub objective: String,
    pub owner_kind: String,
    pub owner_ref: String,
    #[serde(default)]
    pub source_json: Option<String>,
    #[serde(default)]
    pub delivery_policy_json: Option<String>,
    #[serde(default)]
    pub assistant_profile_id: Option<String>,
    #[serde(default)]
    pub trigger_json: Option<String>,
    #[serde(default)]
    pub step_plan_json: Option<String>,
    #[serde(default)]
    pub success_criteria_json: Option<String>,
    #[serde(default)]
    pub context_strategy_json: Option<String>,
    #[serde(default)]
    pub budget_json: Option<String>,
    #[serde(default)]
    pub failure_policy_json: Option<String>,
    #[serde(default = "default_missed_run_policy")]
    pub missed_run_policy: String,
    #[serde(default)]
    pub next_run_at: Option<DateTime<Utc>>,
}

fn default_missed_run_policy() -> String {
    "skip".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMissionFieldsRequest {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub objective: Option<String>,
    #[serde(default)]
    pub trigger_json: Option<String>,
    #[serde(default)]
    pub delivery_policy_json: Option<String>,
    #[serde(default)]
    pub assistant_profile_id: Option<String>,
    #[serde(default)]
    pub step_plan_json: Option<String>,
    #[serde(default)]
    pub success_criteria_json: Option<String>,
    #[serde(default)]
    pub context_strategy_json: Option<String>,
    #[serde(default)]
    pub budget_json: Option<String>,
    #[serde(default)]
    pub failure_policy_json: Option<String>,
    #[serde(default)]
    pub missed_run_policy: Option<String>,
    #[serde(default)]
    pub next_run_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMissionsFilter {
    #[serde(default)]
    pub owner_kind: Option<String>,
    #[serde(default)]
    pub owner_ref: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

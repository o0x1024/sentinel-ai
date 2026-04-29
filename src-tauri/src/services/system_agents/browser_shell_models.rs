use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserShellSession {
    pub id: String,
    pub tab_id: Option<i64>,
    pub frame_id: Option<i64>,
    pub page_url: String,
    pub page_title: Option<String>,
    pub ws_url: String,
    pub protocol: Option<String>,
    pub terminal_kind: String,
    pub writable: bool,
    pub connected: bool,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserShellSessionUpsertInput {
    pub id: String,
    pub tab_id: Option<i64>,
    pub frame_id: Option<i64>,
    pub page_url: String,
    pub page_title: Option<String>,
    pub ws_url: String,
    pub protocol: Option<String>,
    pub terminal_kind: Option<String>,
    pub writable: Option<bool>,
    pub connected: Option<bool>,
    pub occurred_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserShellSessionRemoveInput {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserShellFrame {
    pub id: String,
    pub session_id: String,
    pub direction: String,
    pub frame_type: String,
    pub text_preview: Option<String>,
    pub payload_base64: Option<String>,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserShellFrameInput {
    pub session_id: String,
    pub direction: String,
    pub frame_type: String,
    pub text_preview: Option<String>,
    pub payload_base64: Option<String>,
    pub occurred_at: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrowserShellWriteStatus {
    PendingApproval,
    Queued,
    Dispatching,
    Delivered,
    Failed,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserShellWriteRequest {
    pub request_id: String,
    pub session_id: String,
    pub input_text: String,
    pub input_base64: String,
    pub requires_approval: bool,
    pub status: BrowserShellWriteStatus,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserShellCommandAckInput {
    pub request_id: String,
    pub session_id: String,
    pub success: bool,
    pub error: Option<String>,
}

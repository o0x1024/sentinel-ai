//! Executor-specific types.

/// Tool call record (for persistence).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCallRecord {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub result: Option<String>,
    pub success: bool,
    /// Sequence number within one agent execution (0-based).
    pub sequence: u32,
    /// Tool call started timestamp (UTC millis).
    pub started_at_ms: i64,
    /// Tool call completed timestamp (UTC millis).
    pub completed_at_ms: i64,
    /// Tool call duration in millis.
    pub duration_ms: i64,
}


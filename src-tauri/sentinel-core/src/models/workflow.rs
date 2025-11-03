#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowStepDetail {
    pub step_id: String,
    pub step_name: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: u64,
    pub result_data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub retry_count: u32,
    pub dependencies: Vec<String>,
    pub tool_result: Option<serde_json::Value>,
}


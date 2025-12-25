#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct WorkflowStepDetail {
    pub step_id: String,
    pub step_name: String,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: i64,
    pub result_data: Option<String>,
    pub error: Option<String>,
    pub retry_count: i32,
    pub dependencies: Option<String>,
    pub tool_result: Option<String>,
}

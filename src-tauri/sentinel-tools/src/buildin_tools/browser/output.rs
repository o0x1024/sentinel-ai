use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct BrowserToolOutput {
    pub session_id: String,
    pub action: String,
    pub data: Value,
}

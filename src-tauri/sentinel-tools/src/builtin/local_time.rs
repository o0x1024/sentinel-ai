//! Local time tool (migrated)

use crate::unified_types::*;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Local, TimeZone, Utc};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug)]
pub struct LocalTimeTool {
    metadata: ToolMetadata,
    parameters: ToolParameters,
}

impl LocalTimeTool {
    pub fn new() -> Self {
        let metadata = ToolMetadata {
            author: "Built-in".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["time".to_string(), "date".to_string(), "utility".to_string()],
            install_command: None,
            requirements: vec![],
        };

        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition { name: "format".to_string(), param_type: ParameterType::String, description: "Optional strftime format, e.g. %Y-%m-%d %H:%M:%S".to_string(), required: false, default_value: None },
                ParameterDefinition { name: "timestamp_ms".to_string(), param_type: ParameterType::Number, description: "Optional unix timestamp in milliseconds to format; default now".to_string(), required: false, default_value: None },
                ParameterDefinition { name: "timezone".to_string(), param_type: ParameterType::String, description: "Optional timezone: 'local' (default) or 'utc'".to_string(), required: false, default_value: Some(json!("local")) },
            ],
            schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "format": {"type": "string"},
                    "timestamp_ms": {"type": "number"},
                    "timezone": {"type": "string", "enum": ["local", "utc"]}
                },
            }),
            required: vec![],
            optional: vec!["format".to_string(), "timestamp_ms".to_string(), "timezone".to_string()],
        };

        Self { metadata, parameters }
    }
}

#[async_trait]
impl UnifiedTool for LocalTimeTool {
    fn name(&self) -> &str { "local_time" }
    fn description(&self) -> &str { "Get local and UTC time with optional format and timestamp" }
    fn category(&self) -> ToolCategory { ToolCategory::Utility }
    fn parameters(&self) -> &ToolParameters { &self.parameters }
    fn metadata(&self) -> &ToolMetadata { &self.metadata }

    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        let execution_id = params.execution_id.unwrap_or_else(Uuid::new_v4);
        let start = std::time::Instant::now();

        let fmt = params.inputs.get("format").and_then(|v| v.as_str()).unwrap_or("%Y-%m-%d %H:%M:%S");
        let tz = params.inputs.get("timezone").and_then(|v| v.as_str()).unwrap_or("local");
        let ts_ms = params.inputs.get("timestamp_ms").and_then(|v| v.as_i64());

        let output_json = match (tz, ts_ms) {
            ("utc", Some(ms)) => {
                let dt: DateTime<Utc> = Utc.timestamp_millis_opt(ms).single().unwrap_or_else(|| Utc::now());
                json!({ "timezone": "utc", "iso": dt.to_rfc3339(), "formatted": dt.format(fmt).to_string(), "epoch_ms": dt.timestamp_millis() })
            }
            ("utc", None) => {
                let dt = Utc::now();
                json!({ "timezone": "utc", "iso": dt.to_rfc3339(), "formatted": dt.format(fmt).to_string(), "epoch_ms": dt.timestamp_millis() })
            }
            ("local", Some(ms)) | (_, Some(ms)) => {
                let dt_local: DateTime<Local> = Local.timestamp_millis_opt(ms).single().unwrap_or_else(|| Local::now());
                json!({ "timezone": "local", "iso": dt_local.to_rfc3339(), "formatted": dt_local.format(fmt).to_string(), "epoch_ms": dt_local.timestamp_millis() })
            }
            _ => {
                let dt_local = Local::now();
                json!({ "timezone": "local", "iso": dt_local.to_rfc3339(), "formatted": dt_local.format(fmt).to_string(), "epoch_ms": dt_local.timestamp_millis() })
            }
        };

        Ok(ToolExecutionResult { execution_id, tool_name: self.name().to_string(), tool_id: self.name().to_string(), success: true, output: output_json, error: None, execution_time_ms: start.elapsed().as_millis() as u64, metadata: HashMap::new(), started_at: chrono::Utc::now(), completed_at: Some(chrono::Utc::now()), status: ExecutionStatus::Completed })
    }
}



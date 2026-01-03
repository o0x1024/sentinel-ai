//! Local time tool using rig-core Tool trait

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use chrono::{Local, Utc};

/// Local time arguments
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct LocalTimeArgs {
    /// Timezone: "local" or "utc"
    #[serde(default = "default_timezone")]
    pub timezone: String,
    /// Date format string (e.g., "%Y-%m-%d %H:%M:%S")
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_timezone() -> String { "local".to_string() }
fn default_format() -> String { "%Y-%m-%d %H:%M:%S".to_string() }

/// Local time result
#[derive(Debug, Clone, Serialize)]
pub struct LocalTimeOutput {
    pub formatted: String,
    pub timestamp: i64,
    pub timezone: String,
    pub iso8601: String,
}

/// Local time errors
#[derive(Debug, thiserror::Error)]
pub enum LocalTimeError {
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

/// Local time tool
#[derive(Debug, Clone, Default)]
pub struct LocalTimeTool;

impl LocalTimeTool {
    pub const NAME: &'static str = "local_time";
    pub const DESCRIPTION: &'static str = "Get current local or UTC time in various formats.";
}

impl Tool for LocalTimeTool {
    const NAME: &'static str = Self::NAME;
    type Args = LocalTimeArgs;
    type Output = LocalTimeOutput;
    type Error = LocalTimeError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(LocalTimeArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let (formatted, timestamp, timezone, iso8601) = match args.timezone.to_lowercase().as_str() {
            "utc" => {
                let now = Utc::now();
                (
                    now.format(&args.format).to_string(),
                    now.timestamp(),
                    "UTC".to_string(),
                    now.to_rfc3339(),
                )
            }
            _ => {
                let now = Local::now();
                (
                    now.format(&args.format).to_string(),
                    now.timestamp(),
                    "Local".to_string(),
                    now.to_rfc3339(),
                )
            }
        };

        Ok(LocalTimeOutput {
            formatted,
            timestamp,
            timezone,
            iso8601,
        })
    }
}


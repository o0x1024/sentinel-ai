//! Agent Builder - Placeholder
//!
//! Note: This module needs to be updated when rig-core API is stabilized.

use anyhow::Result;

/// Security agent configuration
#[derive(Debug, Clone, Default)]
pub struct SecurityAgentConfig {
    pub api_key: String,
    pub model: String,
    pub preamble: Option<String>,
}

/// Default security preamble
pub const DEFAULT_SECURITY_PREAMBLE: &str = r#"You are an expert security analyst."#;

/// Simple agent wrapper
pub struct SecurityAgent {
    pub config: SecurityAgentConfig,
}

impl SecurityAgent {
    pub fn new(config: SecurityAgentConfig) -> Self {
        Self { config }
    }

    pub async fn prompt(&self, _input: &str) -> Result<String> {
        // Placeholder - implement with rig-core when API is stable
        Ok("Security agent not yet implemented with rig-core".to_string())
    }
}

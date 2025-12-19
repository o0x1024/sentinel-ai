use crate::engines::LlmConfig;
use serde::{Deserialize, Serialize};

/// Configuration for Vision Explorer V2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionExplorerV2Config {
    /// Target URL to start exploration
    pub target_url: String,

    /// Maximum depth to explore
    pub max_depth: u32,

    /// Maximum total steps (budget)
    pub max_steps: u32,

    /// UserAgent string
    pub user_agent: Option<String>,

    /// Headless mode
    pub headless: bool,

    /// AI Configuration
    pub ai_config: AIConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    /// Fast LLM for Scout Agents (e.g., Haiku, 4o-mini)
    pub fast_model_id: String,

    /// Smart VLM for Specialist Agents (e.g., Sonnet 3.5, GPT-4o)
    pub vision_model_id: String,

    /// Base provider for the fast model
    pub fast_provider: String,

    /// Base provider for the vision model
    pub vision_provider: String,

    /// API Key for fast model (optional, can use env var)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_api_key: Option<String>,

    /// API Key for vision model (optional, can use env var)  
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_api_key: Option<String>,

    /// Base URL for fast model (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_base_url: Option<String>,

    /// Base URL for vision model (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_base_url: Option<String>,
}

impl AIConfig {
    /// Create LlmConfig for fast model (used by PlannerAgent, Navigator, etc.)
    pub fn fast_llm_config(&self) -> LlmConfig {
        let mut config = LlmConfig::new(&self.fast_provider, &self.fast_model_id);
        if let Some(ref key) = self.fast_api_key {
            config = config.with_api_key(key.clone());
        }
        if let Some(ref url) = self.fast_base_url {
            config = config.with_base_url(url.clone());
        }
        config
    }

    /// Create LlmConfig for vision model (used by VisualAnalyst)
    pub fn vision_llm_config(&self) -> LlmConfig {
        let mut config = LlmConfig::new(&self.vision_provider, &self.vision_model_id);
        if let Some(ref key) = self.vision_api_key {
            config = config.with_api_key(key.clone());
        }
        if let Some(ref url) = self.vision_base_url {
            config = config.with_base_url(url.clone());
        }
        config
    }
}

/// Default implementation
impl Default for VisionExplorerV2Config {
    fn default() -> Self {
        Self {
            target_url: "about:blank".to_string(),
            max_depth: 5,
            max_steps: 100,
            user_agent: None,
            headless: false,
            ai_config: AIConfig {
                fast_model_id: "claude-3-haiku".to_string(),
                vision_model_id: "claude-3-sonnet".to_string(),
                fast_provider: "anthropic".to_string(),
                vision_provider: "anthropic".to_string(),
                fast_api_key: None,
                vision_api_key: None,
                fast_base_url: None,
                vision_base_url: None,
            },
        }
    }
}

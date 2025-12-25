//! V2 Vision Explorer Tool - Rig Tool implementation for V2 Engine
//!
//! This tool allows the AI Agent to invoke the V2 Engine for website exploration.

use crate::engines::vision_explorer_v2::emitter::V2MessageEmitter;
use crate::engines::vision_explorer_v2::{V2Engine, VisionExplorerV2Config};
use crate::engines::LlmConfig;
use crate::services::mcp::McpService;
use rig::completion::ToolDefinition;
use rig::tool::{Tool, ToolError};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;

#[derive(Deserialize)]
pub struct VisionExplorerV2Args {
    /// The URL to explore
    url: String,
    /// Maximum exploration depth
    max_depth: Option<u32>,
    /// Maximum steps
    max_steps: Option<u32>,
    /// Custom HTTP headers
    #[allow(dead_code)]
    headers: Option<HashMap<String, String>>,
    /// Allow destructive actions (delete, logout, etc.)
    allow_destructive: Option<bool>,
}

/// V2 Vision Explorer Tool for Agent integration
#[derive(Clone)]
pub struct VisionExplorerV2Tool {
    #[allow(dead_code)]
    mcp_service: Arc<McpService>,
    llm_config: LlmConfig,
    app_handle: Option<AppHandle>,
    execution_id: Option<String>,
}

impl VisionExplorerV2Tool {
    pub fn new(mcp_service: Arc<McpService>, llm_config: LlmConfig) -> Self {
        Self {
            mcp_service,
            llm_config,
            app_handle: None,
            execution_id: None,
        }
    }

    pub fn with_app_handle(mut self, app_handle: AppHandle) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    pub fn with_execution_id(mut self, execution_id: String) -> Self {
        self.execution_id = Some(execution_id);
        self
    }
}

impl Tool for VisionExplorerV2Tool {
    const NAME: &'static str = "vision_explorer";

    type Error = ToolError;
    type Args = VisionExplorerV2Args;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "vision_explorer".to_string(),
            description: "Explore a website using Vision Explorer V2 with event-driven architecture. Discovers APIs, pages, and interactive elements through intelligent navigation.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to explore"
                    },
                    "max_depth": {
                        "type": "integer",
                        "description": "Maximum exploration depth (default: 5)"
                    },
                    "max_steps": {
                        "type": "integer", 
                        "description": "Maximum exploration steps (default: 100)"
                    },
                    "headers": {
                        "type": "object",
                        "description": "Custom HTTP headers (e.g. Authorization)",
                        "additionalProperties": { "type": "string" }
                    },
                    "allow_destructive": {
                        "type": "boolean",
                        "description": "Allow destructive actions like delete/logout (default: false)"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Build V2 config
        let ai_config = crate::engines::vision_explorer_v2::types::AIConfig {
            fast_model_id: self.llm_config.model.clone(),
            vision_model_id: self.llm_config.model.clone(),
            fast_provider: self.llm_config.provider.clone(),
            vision_provider: self.llm_config.provider.clone(),
            fast_api_key: self.llm_config.api_key.clone(),
            vision_api_key: self.llm_config.api_key.clone(),
            fast_base_url: self.llm_config.base_url.clone(),
            vision_base_url: self.llm_config.base_url.clone(),
        };

        let config = VisionExplorerV2Config {
            target_url: args.url.clone(),
            max_depth: args.max_depth.unwrap_or(5),
            max_steps: args.max_steps.unwrap_or(100),
            ai_config,
            ..Default::default()
        };

        // Create and start engine
        let mut engine = V2Engine::new(config);
        let session_id = engine.session_id().to_string();
        let execution_id = self
            .execution_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Setup emitter if app_handle is available
        if let Some(ref app_handle) = self.app_handle {
            let emitter = V2MessageEmitter::new(
                Arc::new(app_handle.clone()),
                execution_id.clone(),
                session_id.clone(),
            );
            engine = engine.with_emitter(emitter);
        }

        // Configure safety based on args
        if args.allow_destructive.unwrap_or(false) {
            // Create permissive policy
            let permissive_policy = crate::engines::vision_explorer_v2::SafetyPolicy {
                enabled: true,
                blocked_keywords: vec![], // Allow all keywords
                blocked_classes: vec![],
                blocked_ids: vec![],
                blocked_url_patterns: vec![],
                allowed_overrides: vec![],
                max_form_submissions: 100,
                allow_modal_actions: true,
            };
            engine = engine.with_safety_policy(permissive_policy);
        }

        // Start exploration
        let start_time = std::time::Instant::now();

        match engine.start().await {
            Ok(_) => {
                let stats = engine.get_stats().await;
                let duration = start_time.elapsed().as_secs();

                Ok(format!(
                    "Vision Explorer V2 completed exploration of {}\n\
                     Session ID: {}\n\
                     Pages visited: {}\n\
                     APIs discovered: {}\n\
                     Actions performed: {}\n\
                     Duration: {}s",
                    args.url,
                    session_id,
                    stats.pages_visited,
                    stats.apis_discovered,
                    stats.actions_performed,
                    duration
                ))
            }
            Err(e) => Err(ToolError::ToolCallError(
                format!("Vision Explorer V2 failed: {}", e).into(),
            )),
        }
    }
}

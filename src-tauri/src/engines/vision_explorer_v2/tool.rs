//! V2 Vision Explorer Tool - Rig Tool implementation for ReAct Engine

use super::react_engine::ReActEngine;
use super::types::VisionExplorerV2Config;
use crate::engines::LlmConfig;
use crate::services::mcp::McpService;
use rig::completion::ToolDefinition;
use rig::tool::{Tool, ToolError};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

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
}

/// V2 Vision Explorer Tool for Agent integration
#[derive(Clone)]
pub struct VisionExplorerV2Tool {
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
            description: "Explore a website using Vision Explorer V2 with ReAct architecture. Systematically discovers pages, APIs, and interactive elements through intelligent reasoning and action.".to_string(),
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
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Check if Playwright MCP is connected
        let connections = self.mcp_service.get_connection_info().await.map_err(|e| {
            ToolError::ToolCallError(format!("Failed to check MCP connections: {}", e).into())
        })?;

        let playwright_connected = connections.iter().any(|c| {
            c.name.to_lowercase().contains("playwright") && c.status.to_lowercase() == "connected"
        });

        if !playwright_connected {
            return Err(ToolError::ToolCallError(
                "Playwright MCP server not connected. Please connect the server to use Vision Explorer.".into(),
            ));
        }

        // Build V2 config
        let mut ai_config = crate::engines::vision_explorer_v2::types::AIConfig {
            fast_model_id: self.llm_config.model.clone(),
            vision_model_id: self.llm_config.model.clone(),
            fast_provider: self.llm_config.provider.clone(),
            vision_provider: self.llm_config.provider.clone(),
            fast_api_key: self.llm_config.api_key.clone(),
            vision_api_key: self.llm_config.api_key.clone(),
            fast_base_url: self.llm_config.base_url.clone(),
            vision_base_url: self.llm_config.base_url.clone(),
        };

        // Try to override with global defaults if app_handle is available
        if let Some(ref app_handle) = self.app_handle {
            if let Some(ai_manager) = app_handle.try_state::<Arc<crate::services::ai::AiServiceManager>>() {
                // Get default LLM (Fast model)
                if let Ok(Some(model_info)) = ai_manager.get_default_model("llm").await {
                    if let Ok(Some(provider_cfg)) = ai_manager.get_provider_config(&model_info.provider).await {
                        log::info!("VisionExplorerV2: Using default LLM model {} ({})", model_info.name, provider_cfg.provider);
                        ai_config.fast_model_id = model_info.name;
                        ai_config.fast_provider = provider_cfg.provider;
                        ai_config.fast_api_key = provider_cfg.api_key;
                        ai_config.fast_base_url = provider_cfg.api_base;
                    }
                }
                
                // Get default VLM (Vision model)
                if let Ok(Some(model_info)) = ai_manager.get_default_model("vlm").await {
                    if let Ok(Some(provider_cfg)) = ai_manager.get_provider_config(&model_info.provider).await {
                        log::info!("VisionExplorerV2: Using default VLM model {} ({})", model_info.name, provider_cfg.provider);
                        ai_config.vision_model_id = model_info.name;
                        ai_config.vision_provider = provider_cfg.provider;
                        ai_config.vision_api_key = provider_cfg.api_key;
                        ai_config.vision_base_url = provider_cfg.api_base;
                    }
                }
            }
        }

        let config = VisionExplorerV2Config {
            target_url: args.url.clone(),
            max_depth: args.max_depth.unwrap_or(5),
            max_steps: args.max_steps.unwrap_or(100),
            user_agent: None,
            headless: false,
            ai_config,
        };

        // Create engine with message callback
        let execution_id = self
            .execution_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let app_handle_clone = self.app_handle.clone();
        let mut engine = ReActEngine::new(config, self.mcp_service.clone())
            .with_message_callback(move |msg| {
                if let Some(ref handle) = app_handle_clone {
                    let event_name = format!("vision-explorer-v2:{}", execution_id);
                    let _ = handle.emit(&event_name, msg);
                }
            });

        let session_id = engine.session_id().to_string();

        // Start exploration
        let start_time = std::time::Instant::now();

        match engine.run().await {
            Ok(result) => {
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
                    result.pages_visited,
                    result.apis_discovered,
                    result.actions_performed,
                    duration
                ))
            }
            Err(e) => Err(ToolError::ToolCallError(
                format!("Vision Explorer V2 failed: {}", e).into(),
            )),
        }
    }
}

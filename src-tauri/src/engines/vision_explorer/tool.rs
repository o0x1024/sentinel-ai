use rig::tool::{Tool, ToolError};
use rig::completion::ToolDefinition;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::collections::HashMap;
use crate::services::mcp::McpService;
use crate::engines::LlmConfig;
use super::explorer::VisionExplorer;
use super::types::VisionExplorerConfig;
use tauri::{AppHandle, Manager};

#[derive(Deserialize)]
pub struct VisionExplorerArgs {
    url: String,
    max_iterations: Option<u32>,
    /// Enable multimodal mode (use screenshots instead of text-based element annotation). Default: false
    enable_multimodal: Option<bool>,
    /// Run browser in headless mode. Default: false
    headless: Option<bool>,
    /// Custom HTTP headers to send with requests
    headers: Option<HashMap<String, String>>,
    /// Custom Local Storage data
    local_storage: Option<HashMap<String, String>>,
}


#[derive(Clone)]
pub struct VisionExplorerTool {
    mcp_service: Arc<McpService>,
    llm_config: LlmConfig,
    app_handle: Option<AppHandle>,
    execution_id: Option<String>,
}

impl VisionExplorerTool {
    pub fn new(
        mcp_service: Arc<McpService>,
        llm_config: LlmConfig,
    ) -> Self {
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

impl Tool for VisionExplorerTool {
    const NAME: &'static str = "vision_explorer";

    type Error = ToolError;
    type Args = VisionExplorerArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "vision_explorer".to_string(),
            description: "Explore a website using vision capabilities to discover APIs, pages, and interactive elements. This is a long-running process that will navigate, click, and analyze the site.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to explore"
                    },
                    // "max_iterations": {
                    //     "type": "integer",
                    //     "description": "Maximum number of exploration steps (default: 20)"
                    // },
                    // "enable_multimodal": {
                    //     "type": "boolean",
                    //     "description": "Enable multimodal mode with screenshots (default: false, uses text-based element annotation)"
                    // },
                    // "headless": {
                    //     "type": "boolean",
                    //     "description": "Run browser in headless mode (default: false)"
                    // },
                    "headers": {
                        "type": "object",
                        "description": "Custom HTTP headers to send with requests (e.g. Authorization)",
                        "additionalProperties": {
                            "type": "string"
                        }
                    },
                    "local_storage": {
                        "type": "object",
                        "description": "Custom Local Storage data to set before navigation (e.g. tokens)",
                        "additionalProperties": {
                            "type": "string"
                        }
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Use database setting as the single source of truth for multimodal mode.
        // If the config is missing/unreadable, default to false.
        let enable_multimodal = if let Some(app) = &self.app_handle {
            if let Some(db) = app.try_state::<Arc<crate::services::database::DatabaseService>>() {
                 db.get_config("ai", "enable_multimodal").await
                   .ok()
                   .flatten()
                   .and_then(|s| s.parse::<bool>().ok())
                   .unwrap_or(false) // Default to false if not set
            } else {
                false
            }
        } else {
            false
        };

        // Keep this log for visibility if tool args request multimodal while global setting is off.
        if !enable_multimodal && args.enable_multimodal == Some(true) {
            tracing::info!("VisionExplorer: Multimodal mode requested but disabled in global settings");
        }

        let mut config = VisionExplorerConfig {
            target_url: args.url.clone(),
            max_iterations: args.max_iterations.unwrap_or(100),
            enable_multimodal, 
            headless: args.headless.unwrap_or(false),
            headers: args.headers,
            local_storage: args.local_storage,
            ..Default::default()
        };


        // Set execution_id and a generated message_id to enable message emitting
        if let Some(exec_id) = &self.execution_id {
            config.execution_id = Some(exec_id.clone());
            config.conversation_id = Some(exec_id.clone()); // Usually execution_id is conversation_id in this context
            config.message_id = Some(uuid::Uuid::new_v4().to_string()); // Generate a new message ID for this run
        }
        
        let mut explorer = VisionExplorer::new(
            config,
            self.mcp_service.clone(),
            self.llm_config.clone(),
        );

        if let Some(app) = &self.app_handle {
             explorer = explorer.with_app_handle(app.clone());
             
             // Inject PassiveScanState if available
             if let Some(state) = app.try_state::<crate::commands::passive_scan_commands::PassiveScanState>() {
                 explorer = explorer.with_passive_scan_state(Arc::new(state.inner().clone()));
             }
             
             if let Some(state) = app.try_state::<crate::commands::passive_scan_commands::PassiveScanState>() {
                 if let Ok(db) = state.get_db_service().await {
                     explorer = explorer.with_passive_db(db);
                 }
             }
             
             // Inject PromptRepository if available
             if let Some(db) = app.try_state::<Arc<crate::services::database::DatabaseService>>() {
                 if let Ok(pool) = db.get_pool() {
                     let prompt_repo = Arc::new(crate::services::prompt_db::PromptRepository::new(pool.clone()));
                     explorer = explorer.with_prompt_repo(prompt_repo);
                 }
             }
        }

        // Register and inject cancellation token so UI "Stop" can cancel this execution.
        if let Some(exec_id) = &self.execution_id {
            let token = crate::managers::cancellation_manager::register_cancellation_token(exec_id.clone()).await;
            tracing::info!("VisionExplorerTool: Injecting cancellation token for execution {}", exec_id);
            explorer = explorer.with_cancellation_token(token);
        }

        match explorer.start().await {
            Ok(summary) => {
                Ok(format!(
                    "Exploration completed for {}.\nPages visited: {}\nAPIs discovered: {}\nDuration: {}s",
                    args.url,
                    summary.pages_visited,
                    summary.apis_discovered,
                    summary.duration_seconds
                ))
            }
            Err(e) => {
                Err(ToolError::ToolCallError(format!("Vision exploration failed: {}", e).into()))
            }
        }
    }
}


use rig::tool::{Tool, ToolError};
use rig::completion::ToolDefinition;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use crate::services::mcp::McpService;
use crate::engines::LlmConfig;
use super::explorer::VisionExplorer;
use super::types::VisionExplorerConfig;
use tauri::{AppHandle, Manager};

#[derive(Deserialize)]
pub struct VisionExplorerArgs {
    url: String,
    max_iterations: Option<u32>,
}

#[derive(Clone)]
pub struct VisionExplorerTool {
    mcp_service: Arc<McpService>,
    llm_config: LlmConfig,
    app_handle: Option<AppHandle>,
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
        }
    }

    pub fn with_app_handle(mut self, app_handle: AppHandle) -> Self {
        self.app_handle = Some(app_handle);
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
                    "max_iterations": {
                        "type": "integer",
                        "description": "Maximum number of exploration steps (default: 20)"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let config = VisionExplorerConfig {
            target_url: args.url.clone(),
            max_iterations: args.max_iterations.unwrap_or(20),
            ..Default::default()
        };
        
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
             
             // Inject PassiveDatabaseService
             // Assuming Access to DatabaseService?
             // crate::commands::passive_scan_commands::PassiveScanState handles DB lazy loading internally?
             // Warning: VisionExplorer.with_passive_db takes Arc<PassiveDatabaseService>
             // We can get it from state if we really want, but current code in explorer.rs might rely on
             // PassiveScanState to init proxy, and maybe use db from there?
             // Actually explorer.rs: line 145 has with_passive_db.
             
             // Let's try to get it from AppHandle state managing DatabaseService?
             // But DatabaseService in src-tauri is crate::services::database::DatabaseService
             // PassiveDatabaseService is sentinel_passive::PassiveDatabaseService
             // They are different types maybe?
             // Yes. But PassiveScanState has get_db_service().
             
             if let Some(state) = app.try_state::<crate::commands::passive_scan_commands::PassiveScanState>() {
                 if let Ok(db) = state.get_db_service().await {
                     explorer = explorer.with_passive_db(db);
                 }
             }
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

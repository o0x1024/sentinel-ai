use super::multi_agent::{ExplorationMode, MultiAgentConfig, MultiAgentExplorer};
use super::types::VisionExplorerConfig;
use crate::engines::LlmConfig;
use crate::services::mcp::McpService;
use rig::completion::ToolDefinition;
use rig::tool::{Tool, ToolError};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
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
                db.get_config("ai", "enable_multimodal")
                    .await
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
            tracing::info!(
                "VisionExplorer: Multimodal mode requested but disabled in global settings"
            );
        }

        let mut config = VisionExplorerConfig {
            target_url: args.url.clone(),
            max_iterations: args.max_iterations.unwrap_or(100),
            enable_multimodal,
            headless: args.headless.unwrap_or(false),
            headers: args.headers.clone(),
            local_storage: args.local_storage.clone(),
            enable_multi_agent: true,
            max_concurrent_workers: 3,
            ..Default::default()
        };

        // Set execution IDs - required for TakeoverManager registration
        if let Some(exec_id) = &self.execution_id {
            config.execution_id = Some(exec_id.clone());
            config.conversation_id = Some(exec_id.clone());
            config.message_id = Some(uuid::Uuid::new_v4().to_string());
        }

        // Multi-Agent Configuration - use default (Adaptive mode)
        let mut multi_config = MultiAgentConfig::default();
        multi_config.max_concurrent_workers = config.max_concurrent_workers as usize;
        multi_config.default_max_depth = config.worker_max_depth;

        tracing::info!(
            "VisionExplorerTool: Using multi-agent mode for {}",
            args.url
        );

        let mut explorer = MultiAgentExplorer::new(
            config.clone(),
            multi_config,
            self.mcp_service.clone(),
            self.llm_config.clone(),
        );

        // Inject dependencies if available
        if let Some(app) = &self.app_handle {
            explorer = explorer.with_app_handle(app.clone());

            if let Some(state) =
                app.try_state::<crate::commands::passive_scan_commands::PassiveScanState>()
            {
                explorer = explorer.with_passive_scan_state(Arc::new(state.inner().clone()));
            }
        }

        // Inject cancellation token
        if let Some(exec_id) = &self.execution_id {
            let token =
                crate::managers::cancellation_manager::register_cancellation_token(exec_id.clone())
                    .await;
            tracing::info!(
                "VisionExplorerTool: Injecting cancellation token for execution {}",
                exec_id
            );
            explorer = explorer.with_cancellation_token(token);
        }

        // Inject message emitter
        if let (Some(app), Some(exec_id)) = (&self.app_handle, &self.execution_id) {
            let emitter = Arc::new(super::message_emitter::VisionExplorerMessageEmitter::new(
                Arc::new(app.clone()),
                exec_id.clone(),
                uuid::Uuid::new_v4().to_string(),
                Some(exec_id.clone()),
                true,
            ));
            explorer = explorer.with_message_emitter(emitter);
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

// ============================================================================
// Multi-Agent Vision Explorer Tool
// ============================================================================

#[derive(Deserialize)]
pub struct MultiAgentVisionExplorerArgs {
    url: String,
    max_iterations: Option<u32>,
    /// Exploration mode: "sequential", "parallel", "adaptive"
    mode: Option<String>,
    /// Maximum concurrent workers for parallel mode
    max_concurrent_workers: Option<u32>,
    /// Enable multimodal mode
    enable_multimodal: Option<bool>,
    /// Run browser in headless mode
    headless: Option<bool>,
    /// Custom HTTP headers
    headers: Option<HashMap<String, String>>,
    /// Custom Local Storage data
    local_storage: Option<HashMap<String, String>>,
}

#[derive(Clone)]
pub struct MultiAgentVisionExplorerTool {
    mcp_service: Arc<McpService>,
    llm_config: LlmConfig,
    app_handle: Option<AppHandle>,
    execution_id: Option<String>,
}

impl MultiAgentVisionExplorerTool {
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

    fn parse_mode(mode_str: &str) -> ExplorationMode {
        match mode_str.to_lowercase().as_str() {
            "parallel" => ExplorationMode::Parallel,
            "adaptive" => ExplorationMode::Adaptive,
            _ => ExplorationMode::Sequential,
        }
    }
}

impl Tool for MultiAgentVisionExplorerTool {
    const NAME: &'static str = "multi_agent_vision_explorer";

    type Error = ToolError;
    type Args = MultiAgentVisionExplorerArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "multi_agent_vision_explorer".to_string(),
            description: "Explore a website using multi-agent architecture. A Manager agent analyzes the homepage navigation and divides the site into scopes, then Worker agents explore each scope in parallel or sequentially.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to explore"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["sequential", "parallel", "adaptive"],
                        "description": "Exploration mode: sequential (one worker at a time), parallel (multiple workers), adaptive (auto-select based on scope count)"
                    },
                    "max_concurrent_workers": {
                        "type": "integer",
                        "description": "Maximum concurrent workers for parallel mode (default: 3)"
                    },
                    "headers": {
                        "type": "object",
                        "description": "Custom HTTP headers to send with requests",
                        "additionalProperties": { "type": "string" }
                    },
                    "local_storage": {
                        "type": "object",
                        "description": "Custom Local Storage data to set before navigation",
                        "additionalProperties": { "type": "string" }
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Get multimodal setting from database
        let enable_multimodal = if let Some(app) = &self.app_handle {
            if let Some(db) = app.try_state::<Arc<crate::services::database::DatabaseService>>() {
                db.get_config("ai", "enable_multimodal")
                    .await
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse::<bool>().ok())
                    .unwrap_or(false)
            } else {
                false
            }
        } else {
            false
        };

        let mode = Self::parse_mode(args.mode.as_deref().unwrap_or("sequential"));

        let mut config = VisionExplorerConfig {
            target_url: args.url.clone(),
            max_iterations: args.max_iterations.unwrap_or(100),
            enable_multimodal: args.enable_multimodal.unwrap_or(enable_multimodal),
            headless: args.headless.unwrap_or(false),
            headers: args.headers,
            local_storage: args.local_storage,
            enable_multi_agent: true,
            multi_agent_mode: format!("{:?}", mode),
            max_concurrent_workers: args.max_concurrent_workers.unwrap_or(3),
            ..Default::default()
        };

        // Set execution IDs
        if let Some(exec_id) = &self.execution_id {
            config.execution_id = Some(exec_id.clone());
            config.conversation_id = Some(exec_id.clone());
            config.message_id = Some(uuid::Uuid::new_v4().to_string());
        }

        let multi_config = MultiAgentConfig {
            mode,
            max_concurrent_workers: args.max_concurrent_workers.unwrap_or(3) as usize,
            default_max_depth: 5,
            explore_authenticated: true,
            global_ignore_patterns: vec![
                "/logout".to_string(),
                "/signout".to_string(),
                "#".to_string(),
                "javascript:".to_string(),
            ],
        };

        let mut explorer = MultiAgentExplorer::new(
            config,
            multi_config,
            self.mcp_service.clone(),
            self.llm_config.clone(),
        );

        // Inject app handle and passive scan state for proxy management
        if let Some(app) = &self.app_handle {
            explorer = explorer.with_app_handle(app.clone());

            if let Some(state) =
                app.try_state::<crate::commands::passive_scan_commands::PassiveScanState>()
            {
                explorer = explorer.with_passive_scan_state(Arc::new(state.inner().clone()));
            }
        }

        // Inject cancellation token
        if let Some(exec_id) = &self.execution_id {
            let token =
                crate::managers::cancellation_manager::register_cancellation_token(exec_id.clone())
                    .await;
            tracing::info!(
                "MultiAgentVisionExplorerTool: Injecting cancellation token for {}",
                exec_id
            );
            explorer = explorer.with_cancellation_token(token);
        }

        // Inject message emitter if app handle available
        if let Some(app) = &self.app_handle {
            if let Some(exec_id) = &self.execution_id {
                let emitter = Arc::new(super::message_emitter::VisionExplorerMessageEmitter::new(
                    Arc::new(app.clone()),
                    exec_id.clone(),
                    uuid::Uuid::new_v4().to_string(),
                    Some(exec_id.clone()),
                    true,
                ));
                explorer = explorer.with_message_emitter(emitter);
            }
        }

        match explorer.start().await {
            Ok(summary) => {
                Ok(format!(
                    "Multi-Agent exploration completed for {}.\nPages visited: {}\nAPIs discovered: {}\nDuration: {}s",
                    args.url,
                    summary.pages_visited,
                    summary.apis_discovered,
                    summary.duration_seconds
                ))
            }
            Err(e) => {
                Err(ToolError::ToolCallError(format!("Multi-agent exploration failed: {}", e).into()))
            }
        }
    }
}

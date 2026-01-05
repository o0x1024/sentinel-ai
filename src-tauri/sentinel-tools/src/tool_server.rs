//! Tool Server Module
//!
//! Manages all tools (builtin, MCP, plugin, workflow) in a unified way.
//! Provides tool registration, execution, and lifecycle management.

use std::sync::Arc;
use once_cell::sync::Lazy;
use rig::tool::ToolSet;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::buildin_tools::{
    HttpRequestTool, LocalTimeTool, PortScanTool, ShellTool, SubdomainBruteTool,
};
use crate::dynamic_tool::{
    DynamicTool, DynamicToolBuilder, DynamicToolDef, ToolExecutor, ToolRegistry, ToolSource,
};

/// Global tool server instance
static TOOL_SERVER: Lazy<Arc<ToolServer>> = Lazy::new(|| Arc::new(ToolServer::new()));

/// Global Tavily API key storage
static TAVILY_API_KEY: Lazy<Arc<RwLock<Option<String>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));

/// Get the global tool server instance
pub fn get_tool_server() -> Arc<ToolServer> {
    TOOL_SERVER.clone()
}

/// Set the Tavily API key for web search
pub async fn set_tavily_api_key(api_key: Option<String>) {
    let mut key = TAVILY_API_KEY.write().await;
    *key = api_key;
}

/// Get the Tavily API key
pub async fn get_tavily_api_key() -> Option<String> {
    let key = TAVILY_API_KEY.read().await;
    key.clone()
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub tool_name: String,
    pub output: Option<Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Tool info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Option<Value>,
    pub source: String,
    pub category: String,
    pub enabled: bool,
}

/// Tool server - manages all tools
pub struct ToolServer {
    registry: ToolRegistry,
    builtin_initialized: RwLock<bool>,
}

impl ToolServer {
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
            builtin_initialized: RwLock::new(false),
        }
    }

    /// Initialize builtin tools
    pub async fn init_builtin_tools(&self) {
        let mut initialized = self.builtin_initialized.write().await;
        if *initialized {
            return;
        }

        tracing::info!("Initializing builtin tools...");

        // Register port_scan tool
        let port_scan_def = DynamicToolBuilder::new("port_scan")
            .description("High-performance TCP port scanner with service identification")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "Target IP address to scan"
                    },
                    "ports": {
                        "type": "string",
                        "description": "Port range or list (e.g., '1-1000', '80,443,8080', or 'common')",
                        "default": "common"
                    },
                    "threads": {
                        "type": "integer",
                        "description": "Number of concurrent threads",
                        "default": 100
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Connection timeout in seconds",
                        "default": 3
                    }
                },
                "required": ["target"]
            }))
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use crate::buildin_tools::port_scan::PortScanArgs;
                use rig::tool::Tool;
                
                let tool_args: PortScanArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                
                let tool = PortScanTool;
                let result = tool.call(tool_args).await
                    .map_err(|e| format!("Port scan failed: {}", e))?;
                
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build port_scan tool");

        self.registry.register(port_scan_def).await;

        // Register http_request tool
        let http_request_def = DynamicToolBuilder::new("http_request")
            .description("Make HTTP requests to any URL with custom headers and body")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Target URL"
                    },
                    "method": {
                        "type": "string",
                        "description": "HTTP method",
                        "default": "GET",
                        "enum": ["GET", "POST", "PUT", "DELETE", "HEAD", "PATCH"]
                    },
                    "headers": {
                        "type": "object",
                        "description": "Request headers as key-value pairs"
                    },
                    "body": {
                        "type": "string",
                        "description": "Request body"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Request timeout in seconds",
                        "default": 30
                    }
                },
                "required": ["url"]
            }))
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use crate::buildin_tools::http_request::HttpRequestArgs;
                use rig::tool::Tool;
                
                let tool_args: HttpRequestArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                
                let tool = HttpRequestTool::default();
                let result = tool.call(tool_args).await
                    .map_err(|e| format!("HTTP request failed: {}", e))?;
                
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build http_request tool");

        self.registry.register(http_request_def).await;

        // Register local_time tool
        let local_time_def = DynamicToolBuilder::new("local_time")
            .description("Get current local or UTC time in various formats")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "timezone": {
                        "type": "string",
                        "description": "Timezone: 'local' or 'utc'",
                        "default": "local"
                    },
                    "format": {
                        "type": "string",
                        "description": "Date format string (e.g., '%Y-%m-%d %H:%M:%S')",
                        "default": "%Y-%m-%d %H:%M:%S"
                    }
                }
            }))
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use crate::buildin_tools::local_time::LocalTimeArgs;
                use rig::tool::Tool;
                
                let tool_args: LocalTimeArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                
                let tool = LocalTimeTool;
                let result = tool.call(tool_args).await
                    .map_err(|e| format!("Local time failed: {}", e))?;
                
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build local_time tool");

        self.registry.register(local_time_def).await;

        // Register shell tool
        let shell_def = DynamicToolBuilder::new("shell")
            .description("Execute shell commands (use with caution)")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Command timeout in seconds",
                        "default": 60
                    }
                },
                "required": ["command"]
            }))
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use crate::buildin_tools::shell::ShellArgs;
                use rig::tool::Tool;
                
                let tool_args: ShellArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                
                let tool = ShellTool::new();
                let result = tool.call(tool_args).await
                    .map_err(|e| format!("Shell execution failed: {}", e))?;
                
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build shell tool");

        self.registry.register(shell_def).await;

        // Register subdomain_brute tool
        let subdomain_brute_def = DynamicToolBuilder::new("subdomain_brute")
            .description("High-performance subdomain brute-force scanner with DNS resolution and HTTP verification")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "domains": {
                        "type": "string",
                        "description": "Target domain(s) to scan, comma-separated for multiple domains"
                    },
                    "resolvers": {
                        "type": "string",
                        "description": "DNS resolvers (comma-separated)",
                        "default": "8.8.8.8,1.1.1.1,223.5.5.5"
                    },
                    "dictionary_file": {
                        "type": "string",
                        "description": "Dictionary file path (optional)"
                    },
                    "dictionary": {
                        "type": "string",
                        "description": "Dictionary words (comma-separated)"
                    },
                    "skip_wildcard": {
                        "type": "boolean",
                        "description": "Skip wildcard domains",
                        "default": true
                    },
                    "bandwidth_limit": {
                        "type": "string",
                        "description": "Bandwidth limit (e.g., '5M')",
                        "default": "5M"
                    },
                    "verify_mode": {
                        "type": "boolean",
                        "description": "Enable HTTP/HTTPS verification",
                        "default": true
                    },
                    "resolve_records": {
                        "type": "boolean",
                        "description": "Enable DNS record resolution",
                        "default": true
                    }
                },
                "required": ["domains"]
            }))
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use crate::buildin_tools::subdomain_brute::SubdomainBruteArgs;
                use rig::tool::Tool;
                
                let tool_args: SubdomainBruteArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                
                let tool = SubdomainBruteTool;
                let result = tool.call(tool_args).await
                    .map_err(|e| format!("Subdomain brute failed: {}", e))?;
                
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build subdomain_brute tool");

        self.registry.register(subdomain_brute_def).await;

        // Register task_planner tool
        let task_planner_def = DynamicToolBuilder::new("task_planner")
            .description("Manage and track the agent's execution plan. Actions: add_tasks (append), update_status (change status), get_plan (view), reset (clear all), replan (replace all tasks), update_task (modify description), delete_task (remove), insert_task (add at position). Mandatory for complex multi-step security tasks.")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "execution_id": {
                        "type": "string",
                        "description": "The current execution ID (mandatory)"
                    },
                    "action": {
                        "type": "string",
                        "description": "Action to perform",
                        "enum": ["add_tasks", "update_status", "get_plan", "reset", "replan", "update_task", "delete_task", "insert_task"]
                    },
                    "tasks": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of task descriptions (required for 'add_tasks' and 'replan')"
                    },
                    "task_index": {
                        "type": "integer",
                        "description": "Index of task (required for 'update_status', 'update_task', 'delete_task', 'insert_task')"
                    },
                    "status": {
                        "type": "string",
                        "description": "New status (required for 'update_status')",
                        "enum": ["pending", "in_progress", "completed", "failed"]
                    },
                    "result": {
                        "type": "string",
                        "description": "Optional observation or result to record"
                    },
                    "new_description": {
                        "type": "string",
                        "description": "New task description (required for 'update_task' and 'insert_task')"
                    }
                },
                "required": ["execution_id", "action"]
            }))
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use crate::buildin_tools::task_planner::{TaskPlannerArgs, TaskPlannerTool};
                use rig::tool::Tool;
                
                let tool_args: TaskPlannerArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                
                let tool = TaskPlannerTool;
                let result = tool.call(tool_args).await
                    .map_err(|e| format!("Task planning failed: {}", e))?;
                
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build task_planner tool");

        self.registry.register(task_planner_def).await;

        // Register memory_manager tool
        let memory_manager_def = DynamicToolBuilder::new("memory_manager")
            .description("Manage long-term memory for the agent. Use 'store' to save important solutions, workflows, or findings for future reference into the vector database. Use 'retrieve' to perform semantic search on past experiences when facing new problems.")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "The action to perform: 'store' or 'retrieve'",
                        "enum": ["store", "retrieve"]
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to store (if action='store') or query to retrieve (if action='retrieve')"
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Tags to categorize the memory (only for 'store')"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max number of results to return (only for 'retrieve'), default 5",
                        "default": 5
                    }
                },
                "required": ["action", "content"]
            }))
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use crate::buildin_tools::memory::{MemoryManagerTool, MemoryManagerArgs};
                use rig::tool::Tool;
                
                let tool_args: MemoryManagerArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                
                let tool = MemoryManagerTool;
                let result = tool.call(tool_args).await
                    .map_err(|e| format!("Memory operation failed: {}", e))?;
                
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build memory_manager tool");

        self.registry.register(memory_manager_def).await;

        // Register web_search tool
        let web_search_def = DynamicToolBuilder::new("web_search")
            .description("Search the web for real-time information using Tavily API. Returns relevant search results with titles, URLs, and content snippets. Useful for finding current information, documentation, CVEs, security advisories, and CTF writeups.")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results",
                        "default": 5
                    },
                    "search_depth": {
                        "type": "string",
                        "description": "Search depth: 'basic' or 'advanced'",
                        "default": "basic",
                        "enum": ["basic", "advanced"]
                    }
                },
                "required": ["query"]
            }))
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use crate::buildin_tools::web_search::WebSearchArgs;
                use rig::tool::Tool;
                
                let tool_args: WebSearchArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                
                // Get API key from global storage
                let api_key = get_tavily_api_key().await;
                
                let tool = if let Some(key) = api_key {
                    crate::buildin_tools::WebSearchTool::with_api_key(key)
                } else {
                    crate::buildin_tools::WebSearchTool::default()
                };
                
                let result = tool.call(tool_args).await
                    .map_err(|e| format!("Web search failed: {}", e))?;
                
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build web_search tool");

        self.registry.register(web_search_def).await;

        // Register ocr tool
        let ocr_def = DynamicToolBuilder::new("ocr")
            .description("Extract text from an image file using OCR (Optical Character Recognition). Support local file paths.")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "image_path": {
                        "type": "string",
                        "description": "Path to the image file (absolute path or relative to CWD)"
                    }
                },
                "required": ["image_path"]
            }))
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use crate::buildin_tools::ocr::{OcrArgs, OcrTool};
                use rig::tool::Tool;
                
                let tool_args: OcrArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                
                let tool = OcrTool::default();
                let result = tool.call(tool_args).await
                    .map_err(|e| format!("OCR failed: {}", e))?;
                
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build ocr tool");

        self.registry.register(ocr_def).await;

        *initialized = true;
        tracing::info!("Builtin tools initialized");
    }

    /// Register a dynamic tool
    pub async fn register_tool(&self, def: DynamicToolDef) {
        self.registry.register(def).await;
    }

    /// Unregister a tool
    pub async fn unregister_tool(&self, name: &str) -> bool {
        self.registry.unregister(name).await
    }

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, args: Value) -> ToolResult {
        let start = std::time::Instant::now();

        match self.registry.execute(name, args).await {
            Ok(output) => ToolResult {
                success: true,
                tool_name: name.to_string(),
                output: Some(output),
                error: None,
                execution_time_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => ToolResult {
                success: false,
                tool_name: name.to_string(),
                output: None,
                error: Some(e.to_string()),
                execution_time_ms: start.elapsed().as_millis() as u64,
            },
        }
    }

    /// List all tools
    pub async fn list_tools(&self) -> Vec<ToolInfo> {
        self.registry
            .list()
            .await
            .into_iter()
            .map(|def| ToolInfo {
                name: def.name.clone(),
                description: def.description.clone(),
                input_schema: def.input_schema.clone(),
                output_schema: def.output_schema.clone(),
                source: match &def.source {
                    ToolSource::Builtin => "builtin".to_string(),
                    ToolSource::Mcp { server_name } => format!("mcp::{}", server_name),
                    ToolSource::Plugin { plugin_id } => format!("plugin::{}", plugin_id),
                    ToolSource::Workflow { workflow_id } => format!("workflow::{}", workflow_id),
                },
                category: match &def.source {
                    ToolSource::Builtin => "builtin".to_string(),
                    ToolSource::Mcp { .. } => "mcp".to_string(),
                    ToolSource::Plugin { .. } => "plugin".to_string(),
                    ToolSource::Workflow { .. } => "workflow".to_string(),
                },
                enabled: true,
            })
            .collect()
    }

    /// Get tool by name
    pub async fn get_tool(&self, name: &str) -> Option<ToolInfo> {
        self.registry.get(name).await.map(|def| ToolInfo {
            name: def.name.clone(),
            description: def.description.clone(),
            input_schema: def.input_schema.clone(),
            output_schema: def.output_schema.clone(),
            source: match &def.source {
                ToolSource::Builtin => "builtin".to_string(),
                ToolSource::Mcp { server_name } => format!("mcp::{}", server_name),
                ToolSource::Plugin { plugin_id } => format!("plugin::{}", plugin_id),
                ToolSource::Workflow { workflow_id } => format!("workflow::{}", workflow_id),
            },
            category: match &def.source {
                ToolSource::Builtin => "builtin".to_string(),
                ToolSource::Mcp { .. } => "mcp".to_string(),
                ToolSource::Plugin { .. } => "plugin".to_string(),
                ToolSource::Workflow { .. } => "workflow".to_string(),
            },
            enabled: true,
        })
    }

    /// Get tool definitions for LLM
    pub async fn get_tool_definitions(&self, tool_names: &[String]) -> Vec<rig::completion::ToolDefinition> {
        self.registry.get_definitions(tool_names).await
    }

    /// Create a rig ToolSet from selected tools
    pub async fn create_toolset(&self, tool_names: &[String]) -> ToolSet {
        self.registry.create_toolset(tool_names).await
    }

    /// Get DynamicTool instances for selected tools
    pub async fn get_dynamic_tools(&self, tool_names: &[String]) -> Vec<DynamicTool> {
        self.registry.get_dynamic_tools(tool_names).await
    }

    /// Register MCP tool
    pub async fn register_mcp_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        description: &str,
        input_schema: Value,
        executor: ToolExecutor,
    ) {
        let full_name = format!("mcp__{}__{}", server_name, tool_name);
        
        let def = DynamicToolDef {
            name: full_name,
            description: description.to_string(),
            input_schema,
            output_schema: None,
            source: ToolSource::Mcp {
                server_name: server_name.to_string(),
            },
            executor,
        };

        self.registry.register(def).await;
    }

    /// Register plugin tool
    pub async fn register_plugin_tool(
        &self,
        plugin_id: &str,
        _tool_name: &str,
        description: &str,
        input_schema: Value,
        output_schema: Option<Value>,
        executor: ToolExecutor,
    ) {
        let sanitized_id = plugin_id.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
    let full_name = format!("plugin__{}", sanitized_id);
        
        let def = DynamicToolDef {
            name: full_name,
            description: description.to_string(),
            input_schema,
            output_schema,
            source: ToolSource::Plugin {
                plugin_id: plugin_id.to_string(),
            },
            executor,
        };

        self.registry.register(def).await;
    }

    /// Register workflow tool
    pub async fn register_workflow_tool(
        &self,
        workflow_id: &str,
        _workflow_name: &str,
        description: &str,
        input_schema: Value,
        output_schema: Option<Value>,
        executor: ToolExecutor,
    ) {
        let full_name = format!("workflow__{}", workflow_id);
        
        let def = DynamicToolDef {
            name: full_name,
            description: description.to_string(),
            input_schema,
            output_schema,
            source: ToolSource::Workflow {
                workflow_id: workflow_id.to_string(),
            },
            executor,
        };

        self.registry.register(def).await;
    }

    /// Clear all MCP tools
    pub async fn clear_mcp_tools(&self) {
        let tools = self.registry.list().await;
        for tool in tools {
            if matches!(tool.source, ToolSource::Mcp { .. }) {
                self.registry.unregister(&tool.name).await;
            }
        }
    }

    /// Clear all plugin tools
    pub async fn clear_plugin_tools(&self) {
        let tools = self.registry.list().await;
        for tool in tools {
            if matches!(tool.source, ToolSource::Plugin { .. }) {
                self.registry.unregister(&tool.name).await;
            }
        }
    }

    /// Clear all workflow tools
    pub async fn clear_workflow_tools(&self) {
        let tools = self.registry.list().await;
        for tool in tools {
            if matches!(tool.source, ToolSource::Workflow { .. }) {
                self.registry.unregister(&tool.name).await;
            }
        }
    }

    /// Get tool count
    pub async fn tool_count(&self) -> usize {
        self.registry.count().await
    }

    /// List tools by source type
    pub async fn list_tools_by_source(&self, source_type: &str) -> Vec<ToolInfo> {
        self.registry
            .list()
            .await
            .into_iter()
            .filter(|def| {
                match source_type {
                    "builtin" => matches!(def.source, ToolSource::Builtin),
                    "mcp" => matches!(def.source, ToolSource::Mcp { .. }),
                    "plugin" => matches!(def.source, ToolSource::Plugin { .. }),
                    "workflow" => matches!(def.source, ToolSource::Workflow { .. }),
                    _ => true,
                }
            })
            .map(|def| ToolInfo {
                name: def.name.clone(),
                description: def.description.clone(),
                input_schema: def.input_schema.clone(),
                output_schema: def.output_schema.clone(),
                source: match &def.source {
                    ToolSource::Builtin => "builtin".to_string(),
                    ToolSource::Mcp { server_name } => format!("mcp::{}", server_name),
                    ToolSource::Plugin { plugin_id } => format!("plugin::{}", plugin_id),
                    ToolSource::Workflow { workflow_id } => format!("workflow::{}", workflow_id),
                },
                category: match &def.source {
                    ToolSource::Builtin => "builtin".to_string(),
                    ToolSource::Mcp { .. } => "mcp".to_string(),
                    ToolSource::Plugin { .. } => "plugin".to_string(),
                    ToolSource::Workflow { .. } => "workflow".to_string(),
                },
                enabled: true,
            })
            .collect()
    }
}

impl Default for ToolServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_server_init() {
        let server = ToolServer::new();
        server.init_builtin_tools().await;
        
        assert!(server.tool_count().await >= 7);
        
        // Check builtin tools exist
        assert!(server.get_tool("port_scan").await.is_some());
        assert!(server.get_tool("http_request").await.is_some());
        assert!(server.get_tool("local_time").await.is_some());
        assert!(server.get_tool("shell").await.is_some());
        assert!(server.get_tool("subdomain_brute").await.is_some());
        assert!(server.get_tool("task_planner").await.is_some());
        assert!(server.get_tool("web_search").await.is_some());
    }

    #[tokio::test]
    async fn test_local_time_execution() {
        let server = ToolServer::new();
        server.init_builtin_tools().await;
        
        let result = server.execute("local_time", serde_json::json!({
            "timezone": "utc"
        })).await;
        
        assert!(result.success);
        assert!(result.output.is_some());
    }
}

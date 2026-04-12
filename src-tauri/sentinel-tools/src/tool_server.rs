//! Tool Server Module
//!
//! Manages all tools (builtin, MCP, plugin, workflow) in a unified way.
//! Provides tool registration, execution, and lifecycle management.

use once_cell::sync::Lazy;
use rig::tool::ToolSet;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::buildin_tools::{
    AskUserQuestionTool, HttpRequestTool, MemoryManagerTool, OcrTool, SearchExploitTool,
    ShellTool, SkillsTool, SubdomainBruteTool, TenthManTool, TodosTool, WebSearchTool,
};

use crate::terminal::server::TerminalServer;

use crate::dynamic_tool::{
    DynamicTool, DynamicToolBuilder, DynamicToolDef, ToolExecutionPolicy, ToolExecutor,
    ToolRegistry, ToolSource,
};

/// Global tool server instance
static TOOL_SERVER: Lazy<Arc<ToolServer>> = Lazy::new(|| Arc::new(ToolServer::new()));

/// Global Tavily API key storage
static TAVILY_API_KEY: Lazy<Arc<RwLock<Option<String>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Strip ANSI escape sequences and clean up redundant whitespace from text
fn strip_ansi_codes(text: &str) -> String {
    // Strip ANSI codes
    let re = regex::Regex::new(
        r"\x1b\[[0-9;]*[a-zA-Z]|\x1b\][0-9;]*[^\x07]*\x07|\x1b[=>]|\x1b\][0-9];[^\x07]*\x07",
    )
    .unwrap();
    let without_ansi = re.replace_all(text, "").to_string();

    // Normalize line endings: \r\n -> \n, standalone \r -> \n
    let normalized = without_ansi.replace("\r\n", "\n").replace('\r', "\n");

    // Remove consecutive blank lines (keep at most one blank line)
    let re_blank = regex::Regex::new(r"\n{3,}").unwrap();
    let cleaned = re_blank.replace_all(&normalized, "\n\n").to_string();

    cleaned.trim().to_string()
}

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
    pub execution_policy: ToolExecutionPolicy,
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

        // Register http_request tool
        let http_request_def = DynamicToolBuilder::new(HttpRequestTool::NAME.to_string())
            .description(HttpRequestTool::DESCRIPTION.to_string())
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
            .category("network")
            .execution_policy(ToolExecutionPolicy {
                read_only: true,
                mutating: false,
                concurrency_safe: true,
                requires_permission: false,
                supports_background: false,
            })
            .executor(|args| async move {
                use crate::buildin_tools::http_request::HttpRequestArgs;
                use rig::tool::Tool;

                let tool_args: HttpRequestArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;

                let tool = HttpRequestTool::default();
                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|e| format!("HTTP request failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build http_request tool");

        self.registry.register(http_request_def).await;

        let ask_user_question_def =
            DynamicToolBuilder::new(AskUserQuestionTool::NAME.to_string())
                .description(AskUserQuestionTool::DESCRIPTION.to_string())
                .input_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "questions": {
                            "type": "array",
                            "description": "Questions to ask the user (1-4 questions).",
                            "minItems": 1,
                            "maxItems": 4,
                            "items": {
                                "type": "object",
                                "properties": {
                                    "header": {
                                        "type": "string",
                                        "description": "Very short question header."
                                    },
                                    "question": {
                                        "type": "string",
                                        "description": "Question text shown to the user."
                                    },
                                    "options": {
                                        "type": "array",
                                        "description": "Multiple-choice options for this question.",
                                        "minItems": 2,
                                        "maxItems": 4,
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "label": {
                                                    "type": "string",
                                                    "description": "Short option label."
                                                },
                                                "description": {
                                                    "type": "string",
                                                    "description": "Short explanation of the option."
                                                },
                                                "preview": {
                                                    "type": "string",
                                                    "description": "Optional preview content for the option."
                                                }
                                            },
                                            "required": ["label", "description"]
                                        }
                                    }
                                },
                                "required": ["header", "question", "options"]
                            }
                        }
                    },
                    "required": ["questions"]
                }))
                .source(ToolSource::Builtin)
                .category("system")
                .execution_policy(ToolExecutionPolicy {
                    read_only: true,
                    mutating: false,
                    concurrency_safe: true,
                    requires_permission: false,
                    supports_background: false,
                })
                .executor(|args| async move {
                    use crate::buildin_tools::ask_user_question::{
                        AskUserQuestionArgs, AskUserQuestionTool,
                    };
                    use rig::tool::Tool;

                    let tool_args: AskUserQuestionArgs = serde_json::from_value(args)
                        .map_err(|e| format!("Invalid arguments: {}", e))?;

                    let tool = AskUserQuestionTool::new();
                    let result = tool
                        .call(tool_args)
                        .await
                        .map_err(|e| format!("AskUserQuestion failed: {}", e))?;

                    serde_json::to_value(result)
                        .map_err(|e| format!("Failed to serialize result: {}", e))
                })
                .build()
                .expect("Failed to build ask_user_question tool");

        self.registry.register(ask_user_question_def).await;

        // Register shell tool
        let shell_desc = {
            use rig::tool::Tool;
            let tool = ShellTool::new();
            tool.definition("".to_string()).await.description
        };
        let shell_def = DynamicToolBuilder::new(ShellTool::NAME.to_string())
            .description(shell_desc)
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
                    },
                    "run_in_background": {
                        "type": "boolean",
                        "description": "Run the command in a dedicated interactive shell session and return immediately.",
                        "default": false
                    }
                },
                "required": ["command"]
            }))
            .source(ToolSource::Builtin)
            .execution_policy(ToolExecutionPolicy {
                read_only: false,
                mutating: true,
                concurrency_safe: false,
                requires_permission: true,
                supports_background: true,
            })
            .executor(|args| async move {
                use crate::buildin_tools::shell::ShellArgs;
                use rig::tool::Tool;

                let tool_args: ShellArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;

                let tool = ShellTool::new();
                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|e| format!("Shell execution failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build shell tool");

        self.registry.register(shell_def).await;

        // Register todos tool
        let todos_def = DynamicToolBuilder::new(TodosTool::NAME.to_string())
            .description(TodosTool::DESCRIPTION.to_string())
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
                        "enum": ["add_items", "update_status", "get_list", "reset", "replan", "update_item", "delete_item", "insert_item", "cleanup"]
                    },
                    "items": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of item descriptions (required for 'add_items' and 'replan')"
                    },
                    "item_index": {
                        "type": "integer",
                        "description": "Index of item (required for 'update_status', 'update_item', 'delete_item', 'insert_item')"
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
                        "description": "New item description (required for 'update_item' and 'insert_item')"
                    }
                },
                "required": ["execution_id", "action"]
            }))
            .source(ToolSource::Builtin)
            .execution_policy(ToolExecutionPolicy {
                read_only: false,
                mutating: true,
                concurrency_safe: false,
                requires_permission: false,
                supports_background: false,
            })
            .executor(|args| async move {
                use crate::buildin_tools::todos::{TodosArgs, TodosTool};
                use rig::tool::Tool;
                
                let tool_args: TodosArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                
                let tool = TodosTool;
                let result = tool.call(tool_args).await
                    .map_err(|e| format!("Todos operation failed: {}", e))?;
                
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build todos tool");

        self.registry.register(todos_def).await;

        // Register skills tool
        let skills_def = DynamicToolBuilder::new(SkillsTool::NAME.to_string())
            .description(SkillsTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(
                    crate::buildin_tools::skills::SkillsToolArgs
                ))
                .unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .category("system")
            .executor(|args| async move {
                use crate::buildin_tools::skills::{SkillsTool, SkillsToolArgs};
                use rig::tool::Tool;

                let tool_args: SkillsToolArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;

                let tool = SkillsTool;
                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|e| format!("Skills operation failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build skills tool");

        self.registry.register(skills_def).await;

        // Register memory tool
        let memory_def = DynamicToolBuilder::new(MemoryManagerTool::NAME.to_string())
            .description(MemoryManagerTool::DESCRIPTION.to_string())
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
            .expect("Failed to build memory tool");

        self.registry.register(memory_def).await;

        // Register web_search tool
        let web_search_def = DynamicToolBuilder::new(WebSearchTool::NAME.to_string())
            .description(WebSearchTool::DESCRIPTION.to_string())
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

                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|e| format!("Web search failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build web_search tool");

        self.registry.register(web_search_def).await;

        // Register subdomain_brute tool
        let subdomain_brute_def = DynamicToolBuilder::new(SubdomainBruteTool::NAME.to_string())
            .description(SubdomainBruteTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(
                    crate::buildin_tools::subdomain_brute::SubdomainBruteArgs
                ))
                .unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .category("monitor")
            .executor(|args| async move {
                use crate::buildin_tools::subdomain_brute::{
                    SubdomainBruteArgs, SubdomainBruteTool,
                };
                use rig::tool::Tool;

                let tool_args: SubdomainBruteArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;

                let tool = SubdomainBruteTool;
                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|e| format!("Subdomain brute failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build subdomain_brute tool");

        self.registry.register(subdomain_brute_def).await;

        // Register search_exploit tool
        let search_exploit_def = DynamicToolBuilder::new(SearchExploitTool::NAME.to_string())
            .description(SearchExploitTool::DESCRIPTION.to_string())
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Operation mode: search or get",
                        "enum": ["search", "get"],
                        "default": "search"
                    },
                    "query": {
                        "type": "string",
                        "description": "Free-form exploit search query"
                    },
                    "cves": {
                        "type": "array",
                        "description": "CVE list (e.g. [\"CVE-2023-1234\"])",
                        "items": { "type": "string" }
                    },
                    "product": {
                        "type": "string",
                        "description": "Target product name"
                    },
                    "version": {
                        "type": "string",
                        "description": "Target product version"
                    },
                    "service": {
                        "type": "string",
                        "description": "Service/protocol hint, e.g. ssh/http"
                    },
                    "port": {
                        "type": "integer",
                        "description": "Service port hint"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum candidates to return for search",
                        "default": 5
                    },
                    "edb_id": {
                        "type": "integer",
                        "description": "ExploitDB ID, required when action=get"
                    }
                }
            }))
            .source(ToolSource::Builtin)
            .category("exploitation")
            .executor(|args| async move {
                use crate::buildin_tools::search_exploit::{SearchExploitArgs, SearchExploitTool};
                use rig::tool::Tool;

                let tool_args: SearchExploitArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;

                let tool = SearchExploitTool;
                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|e| format!("Search exploit failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build search_exploit tool");

        self.registry.register(search_exploit_def).await;

        // Register ocr tool
        let ocr_def = DynamicToolBuilder::new(OcrTool::NAME.to_string())
            .description(OcrTool::DESCRIPTION.to_string())
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

                let tool = OcrTool;
                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|e| format!("OCR failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build ocr tool");

        self.registry.register(ocr_def).await;

        // Register subagent tools (spawn, wait, run)
        self.register_subagent_tools().await;

        // Register tenth_man_review tool
        let tenth_man_def = DynamicToolBuilder::new(TenthManTool::NAME.to_string())
            .description(TenthManTool::DESCRIPTION.to_string())
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "execution_id": {
                        "type": "string",
                        "description": "The current execution ID"
                    },
                    "review_mode": {
                        "type": "object",
                        "description": "Review mode (defaults to full_history)",
                        "oneOf": [
                            {
                                "properties": {
                                    "mode": { "const": "full_history" }
                                },
                                "required": ["mode"]
                            },
                            {
                                "properties": {
                                    "mode": { "const": "recent_messages" },
                                    "count": {
                                        "type": "integer",
                                        "description": "Number of recent messages to review"
                                    }
                                },
                                "required": ["mode", "count"]
                            }
                        ],
                        "default": { "mode": "full_history" }
                    },
                    "review_type": {
                        "type": "string",
                        "description": "Type of review: 'quick' (lightweight) or 'full' (comprehensive)",
                        "default": "quick",
                        "enum": ["quick", "full"]
                    },
                    "focus_area": {
                        "type": "string",
                        "description": "Optional focus area for the review"
                    }
                },
                "required": ["execution_id"]
            }))
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use crate::buildin_tools::tenth_man_tool::{TenthManToolArgs, TenthManTool};
                use rig::tool::Tool;
                
                let tool_args: TenthManToolArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                
                let tool = TenthManTool::new();
                let result = tool.call(tool_args).await
                    .map_err(|e| format!("Tenth Man review failed: {}", e))?;
                
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build tenth_man_review tool");

        self.registry.register(tenth_man_def).await;

        // Register interactive_shell tool
        let interactive_shell_desc = {
            let config = crate::buildin_tools::shell::get_shell_config().await;
            let mut desc = TerminalServer::DESCRIPTION.to_string();
            if config.default_execution_mode
                == crate::buildin_tools::shell::ShellExecutionMode::Docker
            {
                desc.push_str(" [ENVIRONMENT: This interactive shell runs in a Kali Linux docker sandbox with pre-installed cybersecurity tools like nmap, sqlmap, msfconsole, masscan, dirb, etc. Do not hesitate to use these tools directly.]");
            } else {
                desc.push_str(" [ENVIRONMENT: This interactive shell runs on the Host OS.]");
            }
            desc
        };
        let interactive_shell_def = DynamicToolBuilder::new(TerminalServer::NAME.to_string())
            .description(interactive_shell_desc)
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "execution_mode": {
                        "type": "string",
                        "enum": ["docker", "host"],
                        "description": "Execution mode: 'docker' (run in container, recommended) or 'host' (run on host machine)",
                        "default": "docker"
                    },
                    "docker_image": {
                        "type": "string",
                        "description": "Docker image to use when execution_mode is 'docker' (default: sentinel-sandbox:latest)",
                        "default": "sentinel-sandbox:latest"
                    },
                    "command": {
                        "type": "string",
                        "description": "Command to execute in the terminal. Long-running commands like 'ping' will be auto-normalized (e.g., 'ping host' -> 'ping -c 4 host')"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Optional session ID to reuse an existing terminal session"
                    },
                    "wait_strategy": {
                        "type": "string",
                        "enum": ["auto", "prompt", "timeout", "lines"],
                        "description": "How to wait for output: 'auto' (detect completion via prompt + idle), 'prompt' (wait for shell prompt), 'timeout' (fixed timeout), 'lines' (wait for N lines)",
                        "default": "auto"
                    },
                    "wait_timeout": {
                        "type": "integer",
                        "description": "Maximum wait time in seconds (default: 30, max: 120)",
                        "default": 30
                    },
                    "expected_lines": {
                        "type": "integer",
                        "description": "For 'lines' strategy: number of output lines to wait for"
                    },
                    "skip_normalize": {
                        "type": "boolean",
                        "description": "Skip auto-normalization of long-running commands (default: false)",
                        "default": false
                    }
                }
            }))
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use crate::buildin_tools::shell::check_shell_permission;
                use crate::terminal::{TERMINAL_MANAGER, TerminalSessionConfig, WaitStrategy, normalize_command, detect_shell_prompt, ExecutionMode};
                use tokio::sync::mpsc;
                use tokio::time::{timeout, Duration};
                use tracing::info;
                
                // Parse arguments
                let execution_mode = args.get("execution_mode")
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "host" => ExecutionMode::Host,
                        _ => ExecutionMode::Docker,
                    })
                    .unwrap_or(ExecutionMode::Docker);
                
                let docker_image = args.get("docker_image")
                    .and_then(|v| v.as_str())
                    .unwrap_or("sentinel-sandbox:latest")
                    .to_string();
                
                // Support both 'command' and 'initial_command' for backward compatibility
                let command = args.get("command")
                    .or_else(|| args.get("initial_command"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                
                let requested_session_id = args.get("session_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                // Parse wait strategy options
                let wait_strategy = args.get("wait_strategy")
                    .and_then(|v| v.as_str())
                    .map(WaitStrategy::from_str)
                    .unwrap_or_default();
                
                let wait_timeout_secs = args.get("wait_timeout")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30)
                    .min(120); // Cap at 120 seconds
                
                let expected_lines = args.get("expected_lines")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);
                
                let skip_normalize = args.get("skip_normalize")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                
                // 1. Try to find an existing session (prefer requested_session_id, then first active)
                let sessions = TERMINAL_MANAGER.list_sessions().await;
                let active_session = if let Some(ref sid) = requested_session_id {
                    // Use the requested session if it exists
                    TERMINAL_MANAGER.get_session(sid).await
                } else if !sessions.is_empty() {
                    let session_opt = TERMINAL_MANAGER.get_session(&sessions[0].id).await;
                    // Check if session is healthy
                    if let Some(ref session_lock) = session_opt {
                        let session = session_lock.read().await;
                        if session.is_healthy() {
                            info!("Found healthy session: {}", session.id);
                            drop(session); // Release read lock
                            session_opt
                        } else {
                            let unhealthy_id = session.id.clone();
                            drop(session); // Release read lock before stopping
                            info!("Session {} is not healthy (stdin closed), stopping it", unhealthy_id);
                            let _ = TERMINAL_MANAGER.stop_session(&unhealthy_id).await;
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                let (session_id, mut output_rx, session_execution_mode): (String, mpsc::UnboundedReceiver<Vec<u8>>, ExecutionMode) = if let Some(session_lock) = active_session {
                    let id = {
                        let session = session_lock.read().await;
                        session.id.clone()
                    };
                    info!("Using existing terminal session: {}", id);
                    
                    // Create a new subscriber to capture ONLY new output (skip history)
                    let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();
                    {
                        let session = session_lock.read().await;
                        let exec_mode = session.config.execution_mode;
                        session.add_subscriber_no_history(tx).await;
                        (id, rx, exec_mode)
                    }
                } else {
                    // 2. Create a new persistent session if none exists
                    let config = TerminalSessionConfig {
                        execution_mode,
                        docker_image: docker_image.clone(),
                        working_dir: Some("/workspace".to_string()),
                        env_vars: std::collections::HashMap::new(),
                        shell: "bash".to_string(),
                        initial_command: None,
                        reuse_container: true,
                        container_name: Some("sentinel-sandbox-main".to_string()),
                    };
                    
                    let (id, rx) = TERMINAL_MANAGER.create_session(config).await?;
                    info!("Created new persistent terminal session: {}", id);
                    (id, rx, execution_mode)
                };

                // If no command, just return session info
                let Some(original_cmd) = command else {
                    return Ok(serde_json::json!({
                        "session_id": session_id,
                        "completed": false,
                        "message": "Connected to terminal session",
                        "instructions": "Use the Terminal panel to interact"
                    }));
                };

                // 3. Normalize command if needed (auto-add limits to long-running commands)
                let (cmd, was_normalized) = if skip_normalize {
                    (original_cmd.clone(), false)
                } else {
                    normalize_command(&original_cmd)
                };
                
                if was_normalized {
                    info!("Command normalized: '{}' -> '{}'", original_cmd, cmd);
                }

                // 4. Permission check for host execution only
                if session_execution_mode == ExecutionMode::Host {
                    let execution_id = args
                        .get("execution_id")
                        .and_then(|v| v.as_str());
                    check_shell_permission(&cmd, execution_id)
                        .await
                        .map_err(|e| format!("Permission denied: {}", e))?;
                }

                // 5. Execute the command in the session
                let cmd_with_newline = format!("{}\n", cmd);
                if let Err(e) = TERMINAL_MANAGER.write_to_session(&session_id, cmd_with_newline.into_bytes()).await {
                    return Err(format!("Failed to execute command: {}", e));
                }
                
                // 6. Collect output with smart waiting strategy
                let mut output = Vec::new();
                let collect_timeout = Duration::from_secs(wait_timeout_secs);
                let start = tokio::time::Instant::now();
                let mut line_count = 0;
                let mut idle_count = 0;
                let mut completed = false;
                
                while start.elapsed() < collect_timeout {
                    match timeout(Duration::from_millis(300), output_rx.recv()).await {
                        Ok(Some(data)) => {
                            idle_count = 0;
                            let text = String::from_utf8_lossy(&data);
                            line_count += text.matches('\n').count();
                            output.extend_from_slice(&data);
                            
                            let current_output = String::from_utf8_lossy(&output);
                            
                            match wait_strategy {
                                WaitStrategy::Prompt => {
                                    if detect_shell_prompt(&current_output) {
                                        completed = true;
                                        break;
                                    }
                                }
                                WaitStrategy::Lines => {
                                    if let Some(expected) = expected_lines {
                                        if line_count >= expected {
                                            completed = true;
                                            break;
                                        }
                                    }
                                }
                                WaitStrategy::Auto => {
                                    // Check for shell prompt
                                    if detect_shell_prompt(&current_output) {
                                        completed = true;
                                        break;
                                    }
                                }
                                WaitStrategy::Timeout => {
                                    // Just wait for timeout
                                }
                            }
                        }
                        Ok(None) => {
                            completed = true;
                            break;
                        }
                        Err(_) => {
                            // 300ms timeout - no new data
                            idle_count += 1;
                            
                            if matches!(wait_strategy, WaitStrategy::Auto) && !output.is_empty() {
                                // Auto mode: if idle for 1.5s (5 * 300ms), consider done
                                if idle_count >= 5 {
                                    // Double-check with prompt detection
                                    let current_output = String::from_utf8_lossy(&output);
                                    completed = detect_shell_prompt(&current_output);
                                    break;
                                }
                            } else if matches!(wait_strategy, WaitStrategy::Timeout) {
                                // Timeout mode: continue waiting
                            } else if !output.is_empty() && idle_count >= 3 {
                                // Other modes: break after 900ms idle if we have output
                                break;
                            }
                        }
                    }
                }
                
                let timed_out = start.elapsed() >= collect_timeout;
                let output_str = String::from_utf8_lossy(&output).to_string();
                
                // Strip ANSI escape sequences for LLM (keep raw output for terminal display)
                let clean_output = strip_ansi_codes(&output_str);
                
                // Build result with status info
                let mut result = serde_json::json!({
                    "session_id": session_id,
                    "command": cmd,
                    "output": clean_output,
                    "completed": completed,
                    "truncated": timed_out && !completed,
                });
                
                // Add helpful hints
                if was_normalized {
                    result["original_command"] = serde_json::json!(original_cmd);
                    result["note"] = serde_json::json!(format!(
                        "Command was auto-normalized to limit output. Original: '{}'. Use skip_normalize=true to disable.",
                        original_cmd
                    ));
                }
                
                if timed_out && !completed {
                    result["hint"] = serde_json::json!(
                        "Output was truncated due to timeout. The command may still be running. Consider: 1) Using 'wait_strategy: prompt' for commands that return to shell, 2) Adding flags to limit output (e.g., 'ping -c 4'), 3) Increasing 'wait_timeout'."
                    );
                }
                
                Ok(result)
            })
            .build()
            .expect("Failed to build interactive_shell tool");

        self.registry.register(interactive_shell_def).await;

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
                category: def.category.clone(),
                execution_policy: def.execution_policy.clone(),
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
            category: def.category.clone(),
            execution_policy: def.execution_policy.clone(),
            enabled: true,
        })
    }

    /// Get tool definitions for LLM
    pub async fn get_tool_definitions(
        &self,
        tool_names: &[String],
    ) -> Vec<rig::completion::ToolDefinition> {
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
            category: "mcp".to_string(),
            execution_policy: ToolExecutionPolicy::default(),
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
        category: Option<String>,
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
            category: category.unwrap_or_else(|| "other".to_string()),
            execution_policy: ToolExecutionPolicy::default(),
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
            category: "workflow".to_string(),
            execution_policy: ToolExecutionPolicy::default(),
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
            .filter(|def| match source_type {
                "builtin" => matches!(def.source, ToolSource::Builtin),
                "mcp" => matches!(def.source, ToolSource::Mcp { .. }),
                "plugin" => matches!(def.source, ToolSource::Plugin { .. }),
                "workflow" => matches!(def.source, ToolSource::Workflow { .. }),
                _ => true,
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
                execution_policy: def.execution_policy.clone(),
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

impl ToolServer {
    /// Register condensed subagent tools (execute, await, channel)
    async fn register_subagent_tools(&self) {
        use crate::buildin_tools::agent_control_tool::{
            CloseAgentArgs, CloseAgentTool, ListAgentsArgs, ListAgentsTool, SpawnAgentArgs,
            SpawnAgentTool, WaitAgentsArgs, WaitAgentsTool,
        };

        let spawn_def = DynamicToolBuilder::new(SpawnAgentTool::NAME.to_string())
            .description(SpawnAgentTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(SpawnAgentArgs)).unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use rig::tool::Tool;
                let tool_args: SpawnAgentArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                let tool = SpawnAgentTool;
                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|e| format!("spawn_agent failed: {}", e))?;
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build spawn_agent tool");
        self.registry.register(spawn_def).await;

        let wait_def = DynamicToolBuilder::new(WaitAgentsTool::NAME.to_string())
            .description(WaitAgentsTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(WaitAgentsArgs)).unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use rig::tool::Tool;
                let tool_args: WaitAgentsArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                let tool = WaitAgentsTool;
                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|e| format!("wait_agents failed: {}", e))?;
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build wait_agents tool");
        self.registry.register(wait_def).await;

        let list_def = DynamicToolBuilder::new(ListAgentsTool::NAME.to_string())
            .description(ListAgentsTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(ListAgentsArgs)).unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use rig::tool::Tool;
                let tool_args: ListAgentsArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                let tool = ListAgentsTool;
                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|e| format!("list_agents failed: {}", e))?;
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build list_agents tool");
        self.registry.register(list_def).await;

        let close_def = DynamicToolBuilder::new(CloseAgentTool::NAME.to_string())
            .description(CloseAgentTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(CloseAgentArgs)).unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .executor(|args| async move {
                use rig::tool::Tool;
                let tool_args: CloseAgentArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                let tool = CloseAgentTool;
                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|e| format!("close_agent failed: {}", e))?;
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build close_agent tool");
        self.registry.register(close_def).await;

        tracing::info!("Registered agent control tools: spawn, wait, list, close");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_server_init() {
        let server = ToolServer::new();
        server.init_builtin_tools().await;

        assert!(server.tool_count().await >= 4);

        // Check builtin tools exist
        assert!(server.get_tool("http_request").await.is_some());
        assert!(server.get_tool("shell").await.is_some());
        assert!(server.get_tool("todos").await.is_some());
        assert!(server.get_tool("web_search").await.is_some());
        assert!(server.get_tool("subdomain_brute").await.is_some());
    }
}

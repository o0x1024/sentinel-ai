use once_cell::sync::Lazy;
use rig::tool::ToolSet;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::buildin_tools::tool_search::set_tool_search_executor;
#[cfg(feature = "ocr")]
use crate::buildin_tools::OcrTool;
use crate::buildin_tools::{
    AskUserQuestionTool, BrowserShellTool, FileEditTool, FileReadTool, FileWriteTool, GlobTool,
    GrepTool, HttpRequestTool, LspTool, MemoryManagerTool, MissionSchedulerTool,
    PluginAuthoringTool, RouteDiscoveryTool, SearchExploitTool, ShellTool, SkillsTool,
    TenthManTool, ToolSearchArgs, ToolSearchOutput, ToolSearchTool, WebSearchTool,
};
#[cfg(feature = "plugins")]
use crate::buildin_tools::{PortScanTool, SubdomainBruteTool};
#[cfg(feature = "db")]
use crate::buildin_tools::{SopsTool, TasksTool};
use crate::dynamic_tool::{
    DynamicTool, DynamicToolBuilder, DynamicToolDef, ToolCategory, ToolExecutionPolicy,
    ToolExecutor, ToolExposure, ToolRegistry, ToolSource,
};
use crate::tool_search_runtime::run_tool_search;

static TOOL_SERVER: Lazy<Arc<ToolServer>> = Lazy::new(|| Arc::new(ToolServer::new()));
static TAVILY_API_KEY: Lazy<Arc<RwLock<Option<String>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

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
    pub source: ToolSource,
    pub category: ToolCategory,
    pub tags: Vec<String>,
    pub search_hint: Option<String>,
    pub exposure: ToolExposure,
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
                        "description": "Target URL. Optional when referenced_traffic_id or referenced_traffic_index is available in the agent runtime."
                    },
                    "referenced_traffic_id": {
                        "type": "integer",
                        "description": "Referenced traffic history id to replay with captured URL, method, body, and all request headers including Cookie."
                    },
                    "referenced_traffic_index": {
                        "type": "integer",
                        "description": "1-based referenced traffic index from the current user message to replay with captured request details."
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
                "required": []
            }))
            .source(ToolSource::Builtin)
            .category(ToolCategory::WebNetwork)
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
                .category(ToolCategory::Collaboration)
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

        let browser_shell_def = DynamicToolBuilder::new(BrowserShellTool::NAME.to_string())
            .description(BrowserShellTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(
                    crate::buildin_tools::browser_shell::BrowserShellArgs
                ))
                .unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .category(ToolCategory::WebNetwork)
            .tags(vec![
                "browser".to_string(),
                "shell".to_string(),
                "websocket".to_string(),
                "terminal".to_string(),
                "extension".to_string(),
            ])
            .search_hint("inspect or control third-party browser websocket shell sessions captured by the extension")
            .exposure(ToolExposure::Deferred)
            .execution_policy(ToolExecutionPolicy {
                read_only: false,
                mutating: true,
                concurrency_safe: false,
                requires_permission: false,
                supports_background: false,
            })
            .executor(|args| async move {
                use crate::buildin_tools::browser_shell::{BrowserShellArgs, BrowserShellTool};
                use rig::tool::Tool;

                let tool_args: BrowserShellArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;

                let tool = BrowserShellTool::default();
                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|e| format!("Browser shell tool failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build browser_shell tool");

        self.registry.register(browser_shell_def).await;

        let glob_def = DynamicToolBuilder::new(GlobTool::NAME.to_string())
            .description(GlobTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(crate::buildin_tools::glob::GlobArgs))
                    .unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .category(ToolCategory::FileCode)
            .tags(vec![
                "file".to_string(),
                "glob".to_string(),
                "pattern".to_string(),
                "path".to_string(),
                "wildcard".to_string(),
            ])
            .search_hint("find files by wildcard path pattern")
            .exposure(ToolExposure::Deferred)
            .execution_policy(ToolExecutionPolicy {
                read_only: true,
                mutating: false,
                concurrency_safe: true,
                requires_permission: false,
                supports_background: false,
            })
            .executor(|args| async move {
                use crate::buildin_tools::file_runtime::{
                    build_default_file_runtime_context, with_file_runtime_context,
                };
                use crate::buildin_tools::glob::{GlobArgs, GlobTool};
                use rig::tool::Tool;

                let tool_args: GlobArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                let runtime_context = build_default_file_runtime_context(None).await;
                let result = with_file_runtime_context(runtime_context, GlobTool.call(tool_args))
                    .await
                    .map_err(|e| format!("Glob failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build glob tool");

        self.registry.register(glob_def).await;

        let grep_def = DynamicToolBuilder::new(GrepTool::NAME.to_string())
            .description(GrepTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(crate::buildin_tools::grep::GrepArgs))
                    .unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .category(ToolCategory::FileCode)
            .tags(vec![
                "search".to_string(),
                "grep".to_string(),
                "regex".to_string(),
                "find".to_string(),
                "content".to_string(),
            ])
            .search_hint("search file contents with a regex pattern")
            .exposure(ToolExposure::Deferred)
            .execution_policy(ToolExecutionPolicy {
                read_only: true,
                mutating: false,
                concurrency_safe: true,
                requires_permission: false,
                supports_background: false,
            })
            .executor(|args| async move {
                use crate::buildin_tools::file_runtime::{
                    build_default_file_runtime_context, with_file_runtime_context,
                };
                use crate::buildin_tools::grep::{GrepArgs, GrepTool};
                use rig::tool::Tool;

                let tool_args: GrepArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                let runtime_context = build_default_file_runtime_context(None).await;
                let result = with_file_runtime_context(runtime_context, GrepTool.call(tool_args))
                    .await
                    .map_err(|e| format!("Grep failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build grep tool");

        self.registry.register(grep_def).await;

        let file_read_def = DynamicToolBuilder::new(FileReadTool::NAME.to_string())
            .description(FileReadTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(
                    crate::buildin_tools::file_read::FileReadArgs
                ))
                .unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .category(ToolCategory::FileCode)
            .tags(vec![
                "file".to_string(),
                "read".to_string(),
                "code".to_string(),
                "lines".to_string(),
                "snippet".to_string(),
            ])
            .search_hint("read a text file with line-range controls")
            .exposure(ToolExposure::Deferred)
            .execution_policy(ToolExecutionPolicy {
                read_only: true,
                mutating: false,
                concurrency_safe: true,
                requires_permission: false,
                supports_background: false,
            })
            .executor(|args| async move {
                use crate::buildin_tools::file_read::{FileReadArgs, FileReadTool};
                use crate::buildin_tools::file_runtime::{
                    build_default_file_runtime_context, with_file_runtime_context,
                };
                use rig::tool::Tool;

                let tool_args: FileReadArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                let runtime_context = build_default_file_runtime_context(None).await;
                let result =
                    with_file_runtime_context(runtime_context, FileReadTool.call(tool_args))
                        .await
                        .map_err(|e| format!("File read failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build file_read tool");

        self.registry.register(file_read_def).await;

        let file_edit_def = DynamicToolBuilder::new(FileEditTool::NAME.to_string())
            .description(FileEditTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(
                    crate::buildin_tools::file_edit::FileEditArgs
                ))
                .unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .category(ToolCategory::FileCode)
            .tags(vec![
                "file".to_string(),
                "edit".to_string(),
                "replace".to_string(),
                "patch".to_string(),
                "modify".to_string(),
            ])
            .search_hint("edit an existing text file by exact string replacement")
            .exposure(ToolExposure::Deferred)
            .execution_policy(ToolExecutionPolicy {
                read_only: false,
                mutating: true,
                concurrency_safe: false,
                requires_permission: false,
                supports_background: false,
            })
            .executor(|args| async move {
                use crate::buildin_tools::file_edit::{FileEditArgs, FileEditTool};
                use crate::buildin_tools::file_runtime::{
                    build_default_file_runtime_context, with_file_runtime_context,
                };
                use rig::tool::Tool;

                let tool_args: FileEditArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                let runtime_context = build_default_file_runtime_context(None).await;
                let result =
                    with_file_runtime_context(runtime_context, FileEditTool.call(tool_args))
                        .await
                        .map_err(|e| format!("File edit failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build file_edit tool");

        self.registry.register(file_edit_def).await;

        let file_write_def = DynamicToolBuilder::new(FileWriteTool::NAME.to_string())
            .description(FileWriteTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(
                    crate::buildin_tools::file_write::FileWriteArgs
                ))
                .unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .category(ToolCategory::FileCode)
            .tags(vec![
                "file".to_string(),
                "write".to_string(),
                "create".to_string(),
                "overwrite".to_string(),
            ])
            .search_hint("create a file or overwrite one when explicitly allowed")
            .exposure(ToolExposure::Deferred)
            .execution_policy(ToolExecutionPolicy {
                read_only: false,
                mutating: true,
                concurrency_safe: false,
                requires_permission: false,
                supports_background: false,
            })
            .executor(|args| async move {
                use crate::buildin_tools::file_runtime::{
                    build_default_file_runtime_context, with_file_runtime_context,
                };
                use crate::buildin_tools::file_write::{FileWriteArgs, FileWriteTool};
                use rig::tool::Tool;

                let tool_args: FileWriteArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                let runtime_context = build_default_file_runtime_context(None).await;
                let result =
                    with_file_runtime_context(runtime_context, FileWriteTool.call(tool_args))
                        .await
                        .map_err(|e| format!("File write failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build file_write tool");

        self.registry.register(file_write_def).await;

        let lsp_def = DynamicToolBuilder::new(LspTool::NAME.to_string())
            .description(LspTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(crate::buildin_tools::lsp::LspArgs))
                    .unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .category(ToolCategory::FileCode)
            .tags(vec![
                "code".to_string(),
                "symbol".to_string(),
                "definition".to_string(),
                "reference".to_string(),
                "navigation".to_string(),
            ])
            .search_hint("navigate source code by symbols, definitions, and references")
            .exposure(ToolExposure::Deferred)
            .execution_policy(ToolExecutionPolicy {
                read_only: true,
                mutating: false,
                concurrency_safe: true,
                requires_permission: false,
                supports_background: false,
            })
            .executor(|args| async move {
                use crate::buildin_tools::file_runtime::{
                    build_default_file_runtime_context, with_file_runtime_context,
                };
                use crate::buildin_tools::lsp::{LspArgs, LspTool};
                use rig::tool::Tool;

                let tool_args: LspArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                let runtime_context = build_default_file_runtime_context(None).await;
                let result = with_file_runtime_context(runtime_context, LspTool.call(tool_args))
                    .await
                    .map_err(|e| format!("LSP navigation failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build lsp tool");

        self.registry.register(lsp_def).await;

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
                        "description": "Shell command to execute with a bounded wait. If it is still running or waiting for input, shell returns a session_id that can be continued."
                    },
                    "cmd": {
                        "type": "string",
                        "description": "Alias for command when starting a prompt-capable shell session."
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory"
                    },
                    "workdir": {
                        "type": "string",
                        "description": "Alias for cwd when starting a prompt-capable shell session."
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Command timeout in seconds",
                        "default": 60
                    },
                    "yield_time_ms": {
                        "type": "integer",
                        "description": "Bounded wait time in milliseconds for prompt-capable shell sessions. When set, shell returns session_id/process_id instead of blocking indefinitely."
                    },
                    "max_output_tokens": {
                        "type": "integer",
                        "description": "Approximate maximum output tokens to return for prompt-capable shell sessions."
                    },
                    "tty": {
                        "type": "boolean",
                        "description": "Use a TTY-backed session for prompt-capable shell execution.",
                        "default": true
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Session id returned by a prior shell call with yield_time_ms."
                    },
                    "process_id": {
                        "type": "string",
                        "description": "Alias for session_id."
                    },
                    "chars": {
                        "type": "string",
                        "description": "Raw characters to write to a running shell session.",
                        "default": ""
                    },
                    "key": {
                        "type": "string",
                        "description": "Terminal key for action=key. Supported keys: ArrowUp, ArrowDown, ArrowLeft, ArrowRight, Enter, Escape, CtrlC, Backspace, Tab, Space."
                    },
                    "repeat": {
                        "type": "integer",
                        "description": "Repeat count for action=key.",
                        "minimum": 1,
                        "maximum": 100,
                        "default": 1
                    },
                    "action": {
                        "type": "string",
                        "description": "Shell action. start requires command/cmd; poll reads without writing; write sends raw chars/input/input_text; key sends an explicit terminal key; submit presses TTY Enter; cancel stops the session.",
                        "enum": ["start", "poll", "write", "key", "submit", "cancel"]
                    },
                    "run_in_background": {
                        "type": "boolean",
                        "description": "Run the command in a dedicated interactive shell session and return immediately.",
                        "default": false
                    }
                },
                "anyOf": [
                    { "required": ["command"] },
                    { "required": ["cmd"] },
                    { "required": ["session_id"] },
                    { "required": ["process_id"] }
                ]
            }))
            .source(ToolSource::Builtin)
            .category(ToolCategory::Terminal)
            .execution_policy(ToolExecutionPolicy {
                read_only: false,
                mutating: true,
                concurrency_safe: false,
                requires_permission: true,
                supports_background: true,
            })
            .executor(|args| async move {
                use crate::buildin_tools::shell::{ShellArgs, ShellError};
                use crate::terminal::unified_exec_tool::{
                    execute_shell_session_command, execute_shell_session_input,
                };
                use rig::tool::Tool;
                use std::time::Instant;

                fn shell_execution_mode_label(
                    mode: Option<&crate::buildin_tools::shell::ShellExecutionMode>,
                ) -> String {
                    match mode {
                        Some(crate::buildin_tools::shell::ShellExecutionMode::Host) => {
                            "host".to_string()
                        }
                        Some(crate::buildin_tools::shell::ShellExecutionMode::Docker) => {
                            "docker".to_string()
                        }
                        None => String::new(),
                    }
                }

                fn build_shell_failure_result(
                    command: String,
                    execution_mode: String,
                    error: String,
                    execution_time_ms: u64,
                ) -> Value {
                    serde_json::json!({
                        "command": command,
                        "stdout": "",
                        "stderr": error.clone(),
                        "exit_code": serde_json::Value::Null,
                        "completed": false,
                        "success": false,
                        "execution_time_ms": execution_time_ms,
                        "execution_mode": execution_mode,
                        "error": error,
                        "interaction_required": false,
                        "interaction_kind": serde_json::Value::Null,
                        "recommended_tool": serde_json::Value::Null,
                        "suggested_action": serde_json::Value::Null,
                        "backgrounded": false,
                        "background_task_id": serde_json::Value::Null,
                        "background_session_id": serde_json::Value::Null,
                        "background_status": serde_json::Value::Null,
                        "note": serde_json::Value::Null,
                        "stored_artifacts": [],
                    })
                }

                fn build_shell_timeout_failure_result(
                    command: String,
                    execution_mode: String,
                    stdout: String,
                    stderr: String,
                    timeout_secs: u64,
                    execution_time_ms: u64,
                ) -> Value {
                    serde_json::json!({
                        "command": command,
                        "stdout": stdout,
                        "stderr": stderr,
                        "exit_code": serde_json::Value::Null,
                        "completed": false,
                        "success": false,
                        "execution_time_ms": execution_time_ms,
                        "execution_mode": execution_mode,
                        "error": format!("Command timeout after {} seconds", timeout_secs),
                        "interaction_required": false,
                        "interaction_kind": serde_json::Value::Null,
                        "recommended_tool": serde_json::Value::Null,
                        "suggested_action": serde_json::Value::Null,
                        "backgrounded": false,
                        "background_task_id": serde_json::Value::Null,
                        "background_session_id": serde_json::Value::Null,
                        "background_status": serde_json::Value::Null,
                        "note": serde_json::Value::Null,
                        "stored_artifacts": [],
                    })
                }

                fn build_shell_interaction_failure_result(
                    command: String,
                    execution_mode: String,
                    message: String,
                    stdout: String,
                    stderr: String,
                    interaction_kind: String,
                    recommended_tool: String,
                    execution_time_ms: u64,
                ) -> Value {
                    serde_json::json!({
                        "command": command,
                        "stdout": stdout,
                        "stderr": stderr,
                        "exit_code": serde_json::Value::Null,
                        "completed": false,
                        "success": false,
                        "execution_time_ms": execution_time_ms,
                        "execution_mode": execution_mode,
                        "error": message,
                        "interaction_required": true,
                        "interaction_kind": interaction_kind,
                        "recommended_tool": recommended_tool,
                        "suggested_action": "Call shell with yield_time_ms to start a prompt-capable session. Poll with action=poll, write raw stdin with action=write and chars, send terminal keys with action=key and key, or confirm prompts with action=submit.",
                        "backgrounded": false,
                        "background_task_id": serde_json::Value::Null,
                        "background_session_id": serde_json::Value::Null,
                        "background_status": serde_json::Value::Null,
                        "note": serde_json::Value::Null,
                        "stored_artifacts": [],
                    })
                }

                fn has_non_empty_string(args: &Value, key: &str) -> bool {
                    args.get(key)
                        .and_then(|value| value.as_str())
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .is_some()
                }

                fn should_continue_shell_session(args: &Value) -> bool {
                    has_non_empty_string(args, "session_id")
                        || has_non_empty_string(args, "process_id")
                }

                fn has_true_bool(args: &Value, key: &str) -> bool {
                    args.get(key).and_then(|value| value.as_bool()) == Some(true)
                }

                fn should_start_prompt_capable_shell(args: &Value) -> bool {
                    const START_ACTION: &str = "start";

                    if !(has_non_empty_string(args, "command") || has_non_empty_string(args, "cmd"))
                    {
                        return false;
                    }

                    if has_non_empty_string(args, "cmd") {
                        return true;
                    }

                    if args.get("yield_time_ms").is_some() {
                        return true;
                    }

                    if has_true_bool(args, "tty") || has_true_bool(args, "run_in_background") {
                        return true;
                    }

                    args.get("action")
                        .and_then(|value| value.as_str())
                        .map(|value| value.trim().eq_ignore_ascii_case(START_ACTION))
                        .unwrap_or(false)
                }

                fn normalize_prompt_shell_args(args: &mut Value) {
                    let Some(obj) = args.as_object_mut() else {
                        return;
                    };
                    if !obj.contains_key("cmd") {
                        if let Some(command) = obj.get("command").cloned() {
                            obj.insert("cmd".to_string(), command);
                        }
                    }
                    if !obj.contains_key("workdir") {
                        if let Some(cwd) = obj.get("cwd").cloned() {
                            obj.insert("workdir".to_string(), cwd);
                        }
                    }
                }

                fn strip_prompt_shell_only_args(args: &mut Value) {
                    let Some(obj) = args.as_object_mut() else {
                        return;
                    };
                    obj.remove("cmd");
                    obj.remove("workdir");
                    obj.remove("yield_time_ms");
                    obj.remove("max_output_tokens");
                    obj.remove("tty");
                    obj.remove("session_id");
                    obj.remove("process_id");
                    obj.remove("chars");
                }

                let started_at = Instant::now();
                let mut args = args;
                if should_continue_shell_session(&args) {
                    return execute_shell_session_input(args, None).await;
                }

                if should_start_prompt_capable_shell(&args) {
                    normalize_prompt_shell_args(&mut args);
                    return execute_shell_session_command(args, None, None).await;
                }

                let raw_command = args
                    .get("command")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string();
                strip_prompt_shell_only_args(&mut args);

                let tool_args: ShellArgs = match serde_json::from_value(args) {
                    Ok(args) => args,
                    Err(e) => {
                        return Ok(build_shell_failure_result(
                            raw_command,
                            String::new(),
                            format!("Invalid arguments: {}", e),
                            started_at.elapsed().as_millis() as u64,
                        ));
                    }
                };
                let execution_mode = shell_execution_mode_label(tool_args.execution_mode.as_ref());

                let tool = ShellTool::new();
                let result = match tool.call(tool_args.clone()).await {
                    Ok(result) => result,
                    Err(error) => match error {
                        ShellError::InteractionRequired {
                            message,
                            stdout,
                            stderr,
                            interaction_kind,
                            recommended_tool,
                        } => {
                            return Ok(build_shell_interaction_failure_result(
                                tool_args.command,
                                execution_mode,
                                message,
                                stdout,
                                stderr,
                                interaction_kind,
                                recommended_tool,
                                started_at.elapsed().as_millis() as u64,
                            ));
                        }
                        ShellError::TimeoutWithOutput {
                            timeout_secs,
                            stdout,
                            stderr,
                        } => {
                            return Ok(build_shell_timeout_failure_result(
                                tool_args.command,
                                execution_mode,
                                stdout,
                                stderr,
                                timeout_secs,
                                started_at.elapsed().as_millis() as u64,
                            ));
                        }
                        other => {
                            return Ok(build_shell_failure_result(
                                raw_command,
                                execution_mode,
                                format!("Shell execution failed: {}", other),
                                started_at.elapsed().as_millis() as u64,
                            ));
                        }
                    },
                };

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build shell tool");

        self.registry.register(shell_def).await;

        #[cfg(feature = "db")]
        // Register tasks tool
        let tasks_def = DynamicToolBuilder::new(TasksTool::NAME.to_string())
            .description(TasksTool::DESCRIPTION.to_string())
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "execution_id": {
                        "type": "string",
                        "description": "The current execution ID (mandatory)"
                    },
                    "explanation": {
                        "type": "string",
                        "description": "Optional short explanation for this plan update"
                    },
                    "plan": {
                        "type": "array",
                        "description": "Complete current plan. Omitted old items are removed from the displayed plan.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "description": {
                                    "type": "string",
                                    "description": "Step description"
                                },
                                "status": {
                                    "type": "string",
                                    "description": "Step status",
                                    "enum": ["pending", "in_progress", "completed"]
                                },
                                "result": {
                                    "type": "string",
                                    "description": "Optional observation or result for this step"
                                }
                            },
                            "required": ["description", "status"]
                        }
                    }
                },
                "required": ["execution_id", "plan"]
            }))
            .source(ToolSource::Builtin)
            .category(ToolCategory::Collaboration)
            .execution_policy(ToolExecutionPolicy {
                read_only: false,
                mutating: true,
                concurrency_safe: false,
                requires_permission: false,
                supports_background: false,
            })
            .executor(|args| async move {
                use crate::buildin_tools::tasks::{TasksArgs, TasksTool};
                use rig::tool::Tool;

                let tool_args: TasksArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;

                let tool = TasksTool;
                let result = tool.call(tool_args).await
                    .map_err(|e| format!("Tasks operation failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build tasks tool");

        #[cfg(feature = "db")]
        self.registry.register(tasks_def).await;

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
            .category(ToolCategory::KnowledgeExtension)
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

        let tool_search_executor = Arc::new(|args: ToolSearchArgs| {
            Box::pin(async move {
                run_tool_search(args).await.map_err(|message| {
                    crate::buildin_tools::tool_search::ToolSearchError { message }
                })
            })
                as std::pin::Pin<
                    Box<
                        dyn std::future::Future<
                                Output = Result<
                                    ToolSearchOutput,
                                    crate::buildin_tools::tool_search::ToolSearchError,
                                >,
                            > + Send,
                    >,
                >
        });
        set_tool_search_executor(tool_search_executor);

        let tool_search_def = DynamicToolBuilder::new(ToolSearchTool::NAME.to_string())
            .description(ToolSearchTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(ToolSearchArgs)).unwrap_or_default(),
            )
            .output_schema(Some(
                serde_json::to_value(schemars::schema_for!(ToolSearchOutput)).unwrap_or_default(),
            ))
            .source(ToolSource::Builtin)
            .category(ToolCategory::KnowledgeExtension)
            .tags(vec![
                "tool".to_string(),
                "search".to_string(),
                "catalog".to_string(),
                "activate".to_string(),
                "discover".to_string(),
            ])
            .search_hint("search tool capabilities and activate deferred tools")
            .exposure(ToolExposure::Standard)
            .execution_policy(ToolExecutionPolicy {
                read_only: true,
                mutating: false,
                concurrency_safe: true,
                requires_permission: false,
                supports_background: false,
            })
            .executor(|args| async move {
                let tool_args: ToolSearchArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;
                let result = run_tool_search(tool_args).await?;
                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize tool_search result: {}", e))
            })
            .build()
            .expect("Failed to build tool_search tool");

        self.registry.register(tool_search_def).await;

        #[cfg(feature = "db")]
        let sops_def = DynamicToolBuilder::new(SopsTool::NAME.to_string())
            .description(SopsTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(
                    crate::buildin_tools::sops::SopsToolArgs
                ))
                .unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .category(ToolCategory::KnowledgeExtension)
            .executor(|args| async move {
                use crate::buildin_tools::sops::{SopsTool, SopsToolArgs};
                use rig::tool::Tool;

                let tool_args: SopsToolArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;

                let tool = SopsTool;
                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|e| format!("Sops operation failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build sops tool");

        #[cfg(feature = "db")]
        self.registry.register(sops_def).await;

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
                    "title": {
                        "type": "string",
                        "description": "Optional title for the memory (only for 'store'). If not provided, a title will be generated from content."
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
            .category(ToolCategory::KnowledgeExtension)
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

        let mission_scheduler_def =
            DynamicToolBuilder::new(MissionSchedulerTool::NAME.to_string())
                .description(MissionSchedulerTool::DESCRIPTION.to_string())
                .input_schema(
                    serde_json::to_value(schemars::schema_for!(
                        crate::buildin_tools::mission_scheduler::MissionSchedulerArgs
                    ))
                    .unwrap_or_default(),
                )
                .source(ToolSource::Builtin)
                .category(ToolCategory::Monitoring)
                .tags(vec![
                    "mission".to_string(),
                    "schedule".to_string(),
                    "cron".to_string(),
                    "recurring".to_string(),
                    "bot".to_string(),
                ])
                .search_hint(
                    "Create durable recurring missions, cron jobs, monitoring tasks, and scheduled bot deliveries."
                        .to_string(),
                )
                .execution_policy(ToolExecutionPolicy {
                    read_only: false,
                    mutating: true,
                    concurrency_safe: false,
                    requires_permission: false,
                    supports_background: false,
                })
                .executor(|args| async move {
                    use crate::buildin_tools::mission_scheduler::{
                        MissionSchedulerArgs, MissionSchedulerTool,
                    };
                    use rig::tool::Tool;

                    let tool_args: MissionSchedulerArgs = serde_json::from_value(args)
                        .map_err(|e| format!("Invalid arguments: {}", e))?;

                    let tool = MissionSchedulerTool;
                    let result = tool
                        .call(tool_args)
                        .await
                        .map_err(|e| format!("Mission scheduler operation failed: {}", e))?;

                    serde_json::to_value(result)
                        .map_err(|e| format!("Failed to serialize result: {}", e))
                })
                .build()
                .expect("Failed to build mission_scheduler tool");

        self.registry.register(mission_scheduler_def).await;

        let plugin_authoring_def = DynamicToolBuilder::new(PluginAuthoringTool::NAME.to_string())
            .description(PluginAuthoringTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(
                    crate::buildin_tools::plugin_authoring::PluginAuthoringArgs
                ))
                .unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .category(ToolCategory::KnowledgeExtension)
            .tags(vec![
                "plugin".to_string(),
                "authoring".to_string(),
                "generate".to_string(),
                "draft".to_string(),
                "enable".to_string(),
            ])
            .search_hint("generate, validate, test, save draft, or enable a Sentinel plugin")
            .exposure(ToolExposure::Standard)
            .execution_policy(ToolExecutionPolicy {
                read_only: false,
                mutating: true,
                concurrency_safe: false,
                requires_permission: false,
                supports_background: false,
            })
            .executor(|args| async move {
                use crate::buildin_tools::plugin_authoring::{
                    PluginAuthoringArgs, PluginAuthoringTool,
                };
                use rig::tool::Tool;

                let tool_args: PluginAuthoringArgs = serde_json::from_value(args)
                    .map_err(|error| format!("Invalid arguments: {error}"))?;

                let tool = PluginAuthoringTool;
                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|error| format!("Plugin authoring failed: {error}"))?;

                serde_json::to_value(result)
                    .map_err(|error| format!("Failed to serialize result: {error}"))
            })
            .build()
            .expect("Failed to build plugin_authoring tool");

        self.registry.register(plugin_authoring_def).await;

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
            .category(ToolCategory::WebNetwork)
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

        let route_discovery_def = DynamicToolBuilder::new(RouteDiscoveryTool::NAME.to_string())
            .description(RouteDiscoveryTool::DESCRIPTION.to_string())
            .input_schema(
                serde_json::to_value(schemars::schema_for!(
                    crate::buildin_tools::route_discovery::RouteDiscoveryArgs
                ))
                .unwrap_or_default(),
            )
            .source(ToolSource::Builtin)
            .category(ToolCategory::SecurityRecon)
            .tags(vec![
                "route".to_string(),
                "discovery".to_string(),
                "content".to_string(),
                "endpoint".to_string(),
                "recon".to_string(),
            ])
            .search_hint("probe likely web routes and hidden endpoints with wildcard filtering")
            .exposure(ToolExposure::Deferred)
            .execution_policy(ToolExecutionPolicy {
                read_only: true,
                mutating: false,
                concurrency_safe: true,
                requires_permission: false,
                supports_background: false,
            })
            .executor(|args| async move {
                use crate::buildin_tools::route_discovery::{
                    RouteDiscoveryArgs, RouteDiscoveryTool,
                };
                use rig::tool::Tool;

                let tool_args: RouteDiscoveryArgs = serde_json::from_value(args)
                    .map_err(|e| format!("Invalid arguments: {}", e))?;

                let tool = RouteDiscoveryTool::default();
                let result = tool
                    .call(tool_args)
                    .await
                    .map_err(|e| format!("Route discovery failed: {}", e))?;

                serde_json::to_value(result)
                    .map_err(|e| format!("Failed to serialize result: {}", e))
            })
            .build()
            .expect("Failed to build route_discovery tool");

        self.registry.register(route_discovery_def).await;

        #[cfg(feature = "plugins")]
        {
            let port_scan_def = DynamicToolBuilder::new(PortScanTool::NAME.to_string())
                .description(PortScanTool::DESCRIPTION.to_string())
                .input_schema(
                    serde_json::to_value(schemars::schema_for!(
                        crate::buildin_tools::port_scan::PortScanArgs
                    ))
                    .unwrap_or_default(),
                )
                .source(ToolSource::Builtin)
                .category(ToolCategory::SecurityRecon)
                .tags(vec![
                    "port".to_string(),
                    "scan".to_string(),
                    "masscan".to_string(),
                    "network".to_string(),
                    "scanning".to_string(),
                    "recon".to_string(),
                ])
                .search_hint("scan TCP ports on focused hosts, CIDR ranges, or IP ranges")
                .exposure(ToolExposure::Deferred)
                .executor(|args| async move {
                    use crate::buildin_tools::port_scan::{PortScanArgs, PortScanTool};
                    use rig::tool::Tool;

                    let tool_args: PortScanArgs = serde_json::from_value(args)
                        .map_err(|e| format!("Invalid arguments: {}", e))?;

                    let tool = PortScanTool;
                    let result = tool
                        .call(tool_args)
                        .await
                        .map_err(|e| format!("Port scan failed: {}", e))?;

                    serde_json::to_value(result)
                        .map_err(|e| format!("Failed to serialize result: {}", e))
                })
                .build()
                .expect("Failed to build port_scan tool");

            let subdomain_brute_def = DynamicToolBuilder::new(SubdomainBruteTool::NAME.to_string())
                .description(SubdomainBruteTool::DESCRIPTION.to_string())
                .input_schema(
                    serde_json::to_value(schemars::schema_for!(
                        crate::buildin_tools::subdomain_brute::SubdomainBruteArgs
                    ))
                    .unwrap_or_default(),
                )
                .source(ToolSource::Builtin)
                .category(ToolCategory::SecurityRecon)
                .tags(vec![
                    "subdomain".to_string(),
                    "dns".to_string(),
                    "recon".to_string(),
                    "scanning".to_string(),
                ])
                .search_hint("enumerate likely subdomains for an asset or program")
                .exposure(ToolExposure::Deferred)
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

            self.registry.register(port_scan_def).await;
            self.registry.register(subdomain_brute_def).await;
        }

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
            .category(ToolCategory::VulnerabilityResearch)
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

        #[cfg(feature = "ocr")]
        {
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
                .category(ToolCategory::KnowledgeExtension)
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
        }

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
                        "description": "The current execution ID for the active agent run"
                    },
                    "review_mode": {
                        "type": "object",
                        "description": "Review scope. Usually use { mode: 'full_history' } for stuck-state or full-plan review. Use recent_messages only for a narrow local check.",
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
                        "description": "Review depth. 'quick' finds the highest-risk issue fast. 'full' re-evaluates assumptions, logic, alternatives, and mitigations. Defaults to 'full'.",
                        "default": "full",
                        "enum": ["quick", "full"]
                    },
                    "focus_area": {
                        "type": "string",
                        "description": "Optional short phrase describing what to stress test, for example 'root cause hypothesis', 'security bypass risk', or 'rollback plan'"
                    }
                },
                "required": ["execution_id"]
            }))
            .source(ToolSource::Builtin)
            .category(ToolCategory::Collaboration)
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
                source: def.source.clone(),
                category: def.category.clone(),
                tags: def.tags.clone(),
                search_hint: def.search_hint.clone(),
                exposure: def.exposure.clone(),
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
            source: def.source.clone(),
            category: def.category.clone(),
            tags: def.tags.clone(),
            search_hint: def.search_hint.clone(),
            exposure: def.exposure.clone(),
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
            category: ToolCategory::Mcp,
            tags: Vec::new(),
            search_hint: None,
            exposure: ToolExposure::Deferred,
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
            category: ToolCategory::Plugin,
            tags: Vec::new(),
            search_hint: None,
            exposure: ToolExposure::Deferred,
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
            category: ToolCategory::Workflow,
            tags: Vec::new(),
            search_hint: None,
            exposure: ToolExposure::Deferred,
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
                source: def.source.clone(),
                category: def.category.clone(),
                tags: def.tags.clone(),
                search_hint: def.search_hint.clone(),
                exposure: def.exposure.clone(),
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
            .category(ToolCategory::AgentOrchestration)
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
            .category(ToolCategory::AgentOrchestration)
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
            .category(ToolCategory::AgentOrchestration)
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
            .category(ToolCategory::AgentOrchestration)
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
#[path = "tool_server_tests.rs"]
mod tool_server_tests;

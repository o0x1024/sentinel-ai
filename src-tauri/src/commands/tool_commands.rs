//! Tool commands - Tauri commands for builtin tools and workflow tools

use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::Lazy;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use sentinel_tools::buildin_tools::shell::ShellConfig;
use sentinel_tools::buildin_tools::{
    HttpRequestTool, LocalTimeTool, OcrTool, PortScanTool, ShellTool,
};
use sentinel_tools::get_tool_server;

/// Builtin tool info for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub enabled: bool,
    pub input_schema: Option<serde_json::Value>,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Global state for tool enabled/disabled status
static TOOL_STATES: Lazy<RwLock<HashMap<String, bool>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    // All tools enabled by default
    map.insert(PortScanTool::NAME.to_string(), true);
    map.insert(HttpRequestTool::NAME.to_string(), true);
    map.insert(LocalTimeTool::NAME.to_string(), true);
    map.insert(ShellTool::NAME.to_string(), true);
    map.insert(sentinel_tools::buildin_tools::SubdomainBruteTool::NAME.to_string(), true);
    map.insert(sentinel_tools::buildin_tools::WebSearchTool::NAME.to_string(), true);
    map.insert(sentinel_tools::buildin_tools::MemoryManagerTool::NAME.to_string(), true);
    map.insert(OcrTool::NAME.to_string(), true);
    map.insert("interactive_shell".to_string(), true);
    map.insert("tenth_man_review".to_string(), true);
    // Browser automation tools
    map.insert("browser_open".to_string(), true);
    map.insert("browser_snapshot".to_string(), true);
    map.insert("browser_click".to_string(), true);
    map.insert("browser_fill".to_string(), true);
    map.insert("browser_type".to_string(), true);
    map.insert("browser_select".to_string(), true);
    map.insert("browser_scroll".to_string(), true);
    map.insert("browser_wait".to_string(), true);
    map.insert("browser_get_text".to_string(), true);
    map.insert("browser_screenshot".to_string(), true);
    map.insert("browser_back".to_string(), true);
    map.insert("browser_press".to_string(), true);
    map.insert("browser_hover".to_string(), true);
    map.insert("browser_evaluate".to_string(), true);
    map.insert("browser_get_url".to_string(), true);
    map.insert("browser_close".to_string(), true);
    RwLock::new(map)
});

/// Get all builtin tools with their status
#[tauri::command]
pub async fn get_builtin_tools_with_status() -> Result<Vec<BuiltinToolInfo>, String> {
    let states = TOOL_STATES.read().await;

    let mut tools = vec![
        BuiltinToolInfo {
            id: PortScanTool::NAME.to_string(),
            name: PortScanTool::NAME.to_string(),
            description: PortScanTool::DESCRIPTION.to_string(),
            category: "network".to_string(),
            version: "1.0.0".to_string(),
            enabled: *states.get(PortScanTool::NAME).unwrap_or(&true),
            input_schema: Some(serde_json::json!({
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
                        "description": "Number of concurrent threads (1-1000)",
                        "default": 100,
                        "minimum": 1,
                        "maximum": 1000
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Connection timeout in seconds",
                        "default": 3
                    }
                },
                "required": ["target"]
            })),
        },
        BuiltinToolInfo {
            id: HttpRequestTool::NAME.to_string(),
            name: HttpRequestTool::NAME.to_string(),
            description: HttpRequestTool::DESCRIPTION.to_string(),
            category: "network".to_string(),
            version: "1.0.0".to_string(),
            enabled: *states.get(HttpRequestTool::NAME).unwrap_or(&true),
            input_schema: Some(serde_json::json!({
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
                        "description": "Request body (for POST, PUT, etc.)"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Request timeout in seconds",
                        "default": 30
                    },
                    "follow_redirects": {
                        "type": "boolean",
                        "description": "Follow redirects",
                        "default": true
                    }
                },
                "required": ["url"]
            })),
        },
        BuiltinToolInfo {
            id: LocalTimeTool::NAME.to_string(),
            name: LocalTimeTool::NAME.to_string(),
            description: LocalTimeTool::DESCRIPTION.to_string(),
            category: "utility".to_string(),
            version: "1.0.0".to_string(),
            enabled: *states.get(LocalTimeTool::NAME).unwrap_or(&true),
            input_schema: Some(serde_json::json!({
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
            })),
        },
        BuiltinToolInfo {
            id: ShellTool::NAME.to_string(),
            name: ShellTool::NAME.to_string(),
            description: ShellTool::DESCRIPTION.to_string(),
            category: "system".to_string(),
            version: "1.0.0".to_string(),
            enabled: *states.get(ShellTool::NAME).unwrap_or(&true),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory (optional)"
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Command timeout in seconds",
                        "default": 60
                    }
                },
                "required": ["command"]
            })),
        },
        BuiltinToolInfo {
            id: sentinel_tools::buildin_tools::SubdomainBruteTool::NAME.to_string(),
            name: sentinel_tools::buildin_tools::SubdomainBruteTool::NAME.to_string(),
            description: sentinel_tools::buildin_tools::SubdomainBruteTool::DESCRIPTION.to_string(),
            category: "network".to_string(),
            version: "1.0.0".to_string(),
            enabled: *states.get(sentinel_tools::buildin_tools::SubdomainBruteTool::NAME).unwrap_or(&true),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "domains": {
                        "type": "string",
                        "description": "Target domain(s) to scan, comma-separated for multiple domains"
                    },
                    "resolvers": {
                        "type": "string",
                        "description": "DNS resolvers (comma-separated, e.g., '8.8.8.8,1.1.1.1')",
                        "default": "8.8.8.8,1.1.1.1,223.5.5.5"
                    },
                    "dictionary_file": {
                        "type": "string",
                        "description": "Dictionary file path (optional, uses built-in if not provided)"
                    },
                    "dictionary": {
                        "type": "string",
                        "description": "Dictionary words (comma-separated, e.g., 'www,mail,api,admin')"
                    },
                    "skip_wildcard": {
                        "type": "boolean",
                        "description": "Skip wildcard domains",
                        "default": true
                    },
                    "bandwidth_limit": {
                        "type": "string",
                        "description": "Bandwidth limit (e.g., '5M', '10M')",
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
            })),
        },
        BuiltinToolInfo {
            id: sentinel_tools::buildin_tools::WebSearchTool::NAME.to_string(),
            name: sentinel_tools::buildin_tools::WebSearchTool::NAME.to_string(),
            description: sentinel_tools::buildin_tools::WebSearchTool::DESCRIPTION.to_string(),
            category: "network".to_string(),
            version: "1.0.0".to_string(),
            enabled: *states.get(sentinel_tools::buildin_tools::WebSearchTool::NAME).unwrap_or(&true),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results (default: 5)",
                        "default": 5
                    },
                    "search_depth": {
                        "type": "string",
                        "description": "Search depth: 'basic' or 'advanced' (default: 'basic')",
                        "default": "basic",
                        "enum": ["basic", "advanced"]
                    }
                },
                "required": ["query"]
            })),
        },
        BuiltinToolInfo {
            id: sentinel_tools::buildin_tools::MemoryManagerTool::NAME.to_string(),
            name: sentinel_tools::buildin_tools::MemoryManagerTool::NAME.to_string(),
            description: sentinel_tools::buildin_tools::MemoryManagerTool::DESCRIPTION.to_string(),
            category: "ai".to_string(),
            version: "1.0.0".to_string(),
            enabled: *states.get(sentinel_tools::buildin_tools::MemoryManagerTool::NAME).unwrap_or(&true),
            input_schema: Some(serde_json::json!({
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
            })),
        },
        BuiltinToolInfo {
            id: OcrTool::NAME.to_string(),
            name: OcrTool::NAME.to_string(),
            description: OcrTool::DESCRIPTION.to_string(),
            category: "ai".to_string(),
            version: "1.0.0".to_string(),
            enabled: *states.get(OcrTool::NAME).unwrap_or(&true),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "image_path": {
                        "type": "string",
                        "description": "Path to the image file to extract text from"
                    }
                },
                "required": ["image_path"]
            })),
        },
    ];

    // Add vision_explorer
    tools.push(BuiltinToolInfo {
        id: "vision_explorer".to_string(),
        name: "vision_explorer".to_string(),
        description: "Explore a website using vision capabilities to discover APIs, pages, and interactive elements.".to_string(),
        category: "ai".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("vision_explorer").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to explore"
                },
                "max_iterations": {
                    "type": "integer",
                    "description": "Maximum number of exploration steps (default: 20)",
                    "default": 20
                }
            },
            "required": ["url"]
        })),
    });

    // Add interactive_shell
    tools.push(BuiltinToolInfo {
        id: "interactive_shell".to_string(),
        name: "interactive_shell".to_string(),
        description: "Create persistent terminal session for tools requiring continuous interaction: REQUIRED for ssh, msfconsole, sqlmap, mysql/psql clients, Python/Node REPL, or any tool that maintains state between commands. Returns session ID for multi-turn interaction. Use this when a tool needs to stay running between commands.".to_string(),
        category: "system".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("interactive_shell").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "use_docker": {
                    "type": "boolean",
                    "description": "Whether to run in Docker container (recommended for security)",
                    "default": true
                },
                "docker_image": {
                    "type": "string",
                    "description": "Docker image to use (default: sentinel-sandbox:latest)",
                    "default": "sentinel-sandbox:latest"
                },
                "initial_command": {
                    "type": "string",
                    "description": "Optional initial command to run (e.g., 'msfconsole', 'sqlmap')"
                }
            }
        })),
    });

    // Add tenth_man_review
    tools.push(BuiltinToolInfo {
        id: "tenth_man_review".to_string(),
        name: "tenth_man_review".to_string(),
        description: "Get adversarial feedback on your work. Uncovers hidden problems, alternative perspectives, and potential failures. 'quick' mode: Fast risk identification. 'full' mode: Comprehensive analysis with recommendations. Perfect for validating plans, reviewing code, and avoiding costly mistakes.".to_string(),
        category: "ai".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("tenth_man_review").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "execution_id": {
                    "type": "string",
                    "description": "The current execution ID (required)"
                },
                "content_to_review": {
                    "type": "string",
                    "description": "Content to review (e.g., current plan, proposed solution, or conclusion)",
                    "x-ui-widget": "textarea"
                },
                "context_description": {
                    "type": "string",
                    "description": "Context description (what this review is about)"
                },
                "review_type": {
                    "type": "string",
                    "description": "Type of review: 'quick' (lightweight) or 'full' (comprehensive)",
                    "default": "quick",
                    "enum": ["quick", "full"]
                }
            },
            "required": ["execution_id", "content_to_review"]
        })),
    });

    // Browser automation tools
    tools.push(BuiltinToolInfo {
        id: "browser_open".to_string(),
        name: "browser_open".to_string(),
        description: "Open a URL in browser and get page snapshot. Use this to start web tasks like booking tickets, searching information, or filling forms.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_open").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to open"
                },
                "wait_until": {
                    "type": "string",
                    "description": "Wait condition: 'load', 'domcontentloaded', or 'networkidle'",
                    "default": "load",
                    "enum": ["load", "domcontentloaded", "networkidle"]
                }
            },
            "required": ["url"]
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_snapshot".to_string(),
        name: "browser_snapshot".to_string(),
        description: "Get current page structure as accessibility tree with refs (@e1, @e2).".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_snapshot").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "interactive_only": {
                    "type": "boolean",
                    "description": "Only show interactive elements",
                    "default": true
                },
                "compact": {
                    "type": "boolean",
                    "description": "Remove empty structural elements",
                    "default": true
                }
            }
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_click".to_string(),
        name: "browser_click".to_string(),
        description: "Click an element by ref (@e1) or CSS selector.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_click").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "description": "Element ref (e.g., '@e1') or CSS selector"
                }
            },
            "required": ["target"]
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_fill".to_string(),
        name: "browser_fill".to_string(),
        description: "Fill text into an input field.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_fill").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "description": "Element ref or CSS selector"
                },
                "value": {
                    "type": "string",
                    "description": "Text to fill"
                }
            },
            "required": ["target", "value"]
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_type".to_string(),
        name: "browser_type".to_string(),
        description: "Type text character by character.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_type").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "description": "Element ref or CSS selector"
                },
                "text": {
                    "type": "string",
                    "description": "Text to type"
                },
                "delay_ms": {
                    "type": "integer",
                    "description": "Delay between keystrokes in ms"
                }
            },
            "required": ["target", "text"]
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_select".to_string(),
        name: "browser_select".to_string(),
        description: "Select an option from a dropdown.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_select").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "description": "Element ref or CSS selector"
                },
                "value": {
                    "type": "string",
                    "description": "Option value to select"
                }
            },
            "required": ["target", "value"]
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_scroll".to_string(),
        name: "browser_scroll".to_string(),
        description: "Scroll the page.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_scroll").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "direction": {
                    "type": "string",
                    "description": "Scroll direction",
                    "default": "down",
                    "enum": ["up", "down", "left", "right"]
                },
                "amount": {
                    "type": "integer",
                    "description": "Scroll amount in pixels",
                    "default": 300
                }
            }
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_wait".to_string(),
        name: "browser_wait".to_string(),
        description: "Wait for an element or timeout.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_wait").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "selector": {
                    "type": "string",
                    "description": "CSS selector to wait for"
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "Maximum wait time in ms",
                    "default": 30000
                }
            }
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_get_text".to_string(),
        name: "browser_get_text".to_string(),
        description: "Get text content of an element.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_get_text").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "description": "Element ref or CSS selector"
                }
            },
            "required": ["target"]
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_screenshot".to_string(),
        name: "browser_screenshot".to_string(),
        description: "Take a screenshot of the page.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_screenshot").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "full_page": {
                    "type": "boolean",
                    "description": "Capture full page",
                    "default": false
                }
            }
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_back".to_string(),
        name: "browser_back".to_string(),
        description: "Navigate back to previous page.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_back").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {}
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_press".to_string(),
        name: "browser_press".to_string(),
        description: "Press a keyboard key.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_press").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "Key to press (e.g., 'Enter', 'Tab')"
                },
                "target": {
                    "type": "string",
                    "description": "Optional element to focus first"
                }
            },
            "required": ["key"]
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_hover".to_string(),
        name: "browser_hover".to_string(),
        description: "Hover over an element.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_hover").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "description": "Element ref or CSS selector"
                }
            },
            "required": ["target"]
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_evaluate".to_string(),
        name: "browser_evaluate".to_string(),
        description: "Execute JavaScript in the browser.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_evaluate").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {
                "script": {
                    "type": "string",
                    "description": "JavaScript code to execute"
                }
            },
            "required": ["script"]
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_get_url".to_string(),
        name: "browser_get_url".to_string(),
        description: "Get current page URL and title.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_get_url").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {}
        })),
    });

    tools.push(BuiltinToolInfo {
        id: "browser_close".to_string(),
        name: "browser_close".to_string(),
        description: "Close the browser.".to_string(),
        category: "browser".to_string(),
        version: "1.0.0".to_string(),
        enabled: *states.get("browser_close").unwrap_or(&true),
        input_schema: Some(serde_json::json!({
            "type": "object",
            "properties": {}
        })),
    });

    Ok(tools)
}

/// Toggle a builtin tool on/off
#[tauri::command]
pub async fn toggle_builtin_tool(tool_name: String, enabled: bool) -> Result<(), String> {
    let mut states = TOOL_STATES.write().await;
    states.insert(tool_name.clone(), enabled);
    tracing::info!("Tool '{}' toggled to {}", tool_name, enabled);
    Ok(())
}

/// Unified tool execution for builtin tools and workflow tools
#[tauri::command]
pub async fn unified_execute_tool(
    tool_name: String,
    inputs: serde_json::Value,
    _context: Option<serde_json::Value>,
    _timeout: Option<u64>,
) -> Result<ToolExecutionResult, String> {
    let start = std::time::Instant::now();

    // License check for tool execution
    #[cfg(not(debug_assertions))]
    if !sentinel_license::is_licensed() {
        return Ok(ToolExecutionResult {
            success: false,
            output: None,
            error: Some("License required for tool execution".to_string()),
            execution_time_ms: start.elapsed().as_millis() as u64,
        });
    }

    // Check tool enabled status for builtin tools
    if !tool_name.contains("::") {
        let states = TOOL_STATES.read().await;
        if !states.get(&tool_name).unwrap_or(&true) {
            return Ok(ToolExecutionResult {
                success: false,
                output: None,
                error: Some(format!("Tool '{}' is disabled", tool_name)),
                execution_time_ms: start.elapsed().as_millis() as u64,
            });
        }
    }

    // Use ToolServer for execution
    let tool_server = get_tool_server();
    tool_server.init_builtin_tools().await;

    let result = tool_server.execute(&tool_name, inputs).await;

    Ok(ToolExecutionResult {
        success: result.success,
        output: result.output,
        error: result.error,
        execution_time_ms: result.execution_time_ms,
    })
}

#[allow(dead_code)]
async fn execute_port_scan(inputs: serde_json::Value) -> Result<serde_json::Value, String> {
    use sentinel_tools::buildin_tools::port_scan::PortScanArgs;

    let args: PortScanArgs = serde_json::from_value(inputs)
        .map_err(|e| format!("Invalid port_scan arguments: {}", e))?;

    let tool = PortScanTool;
    let result = tool
        .call(args)
        .await
        .map_err(|e| format!("Port scan failed: {}", e))?;

    serde_json::to_value(result).map_err(|e| format!("Failed to serialize result: {}", e))
}

#[allow(dead_code)]
async fn execute_http_request(inputs: serde_json::Value) -> Result<serde_json::Value, String> {
    use sentinel_tools::buildin_tools::http_request::HttpRequestArgs;

    let args: HttpRequestArgs = serde_json::from_value(inputs)
        .map_err(|e| format!("Invalid http_request arguments: {}", e))?;

    let tool = HttpRequestTool::default();
    let result = tool
        .call(args)
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    serde_json::to_value(result).map_err(|e| format!("Failed to serialize result: {}", e))
}

#[allow(dead_code)]
async fn execute_local_time(inputs: serde_json::Value) -> Result<serde_json::Value, String> {
    use sentinel_tools::buildin_tools::local_time::LocalTimeArgs;

    let args: LocalTimeArgs = serde_json::from_value(inputs)
        .map_err(|e| format!("Invalid local_time arguments: {}", e))?;

    let tool = LocalTimeTool;
    let result = tool
        .call(args)
        .await
        .map_err(|e| format!("Local time failed: {}", e))?;

    serde_json::to_value(result).map_err(|e| format!("Failed to serialize result: {}", e))
}

#[allow(dead_code)]
async fn execute_shell(inputs: serde_json::Value) -> Result<serde_json::Value, String> {
    use sentinel_tools::buildin_tools::shell::ShellArgs;

    let args: ShellArgs =
        serde_json::from_value(inputs).map_err(|e| format!("Invalid shell arguments: {}", e))?;

    let tool = ShellTool::new();
    let result = tool
        .call(args)
        .await
        .map_err(|e| format!("Shell execution failed: {}", e))?;

    serde_json::to_value(result).map_err(|e| format!("Failed to serialize result: {}", e))
}

#[allow(dead_code)]
async fn execute_workflow_tool(
    tool_name: &str,
    _inputs: serde_json::Value,
    _timeout: Option<u64>,
) -> Result<ToolExecutionResult, String> {
    let start = std::time::Instant::now();

    // Extract workflow ID from tool_name (format: "workflow::{id}")
    let workflow_id = tool_name
        .strip_prefix("workflow::")
        .ok_or_else(|| "Invalid workflow tool name".to_string())?;

    // TODO: Load workflow definition from database and execute
    // For now, return a placeholder result
    tracing::warn!(
        "Workflow tool execution not yet fully implemented: {}",
        workflow_id
    );

    Ok(ToolExecutionResult {
        success: false,
        output: None,
        error: Some(format!("Workflow tool '{}' execution not yet implemented. Please use WorkflowStudio to run workflows.", workflow_id)),
        execution_time_ms: start.elapsed().as_millis() as u64,
    })
}

/// List all unified tools (builtin + workflow + plugin)
#[tauri::command]
pub async fn list_unified_tools() -> Result<Vec<serde_json::Value>, String> {
    let builtin_tools = get_builtin_tools_with_status().await?;

    let tools: Vec<serde_json::Value> = builtin_tools
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "category": t.category,
                "source": "builtin",
                "available": t.enabled,
            })
        })
        .collect();

    // TODO: Add workflow tools and plugin tools

    Ok(tools)
}

/// Node catalog item for workflow studio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCatalogItem {
    pub node_type: String,
    pub label: String,
    pub category: String,
    pub params_schema: serde_json::Value,
    pub input_ports: Vec<PortDef>,
    pub output_ports: Vec<PortDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortDef {
    pub id: String,
    pub name: String,
    pub port_type: String,
    pub required: bool,
}

/// 从插件代码获取 input_schema（仅通过运行时调用 get_input_schema）
/// 
/// 调用插件导出的 get_input_schema() 函数获取 schema。
/// 如果插件未导出该函数，返回默认空 schema。
async fn get_plugin_input_schema_async(
    plugin_id: &str,
    plugin_name: &str,
    code: &str,
) -> serde_json::Value {
    // 构建临时元数据
    let metadata = sentinel_plugins::PluginMetadata {
        id: plugin_id.to_string(),
        name: plugin_name.to_string(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "agent".to_string(),
        category: "tool".to_string(),
        default_severity: sentinel_plugins::Severity::Medium,
        tags: vec![],
        description: Some(format!("Agent tool plugin: {}", plugin_name)),
    };

    // 运行时获取 schema
    match sentinel_plugins::get_input_schema_from_code(code, metadata).await {
        Ok(schema) => {
            tracing::info!(
                "Got input schema from plugin runtime: {} ({})",
                plugin_name,
                plugin_id
            );
            schema
        }
        Err(e) => {
            tracing::warn!(
                "Failed to get schema from runtime for {}: {}, plugin must export get_input_schema()",
                plugin_id,
                e
            );
            // 返回默认空 schema
            serde_json::json!({
                "type": "object",
                "properties": {}
            })
        }
    }
}

/// List all available node types for workflow studio
#[tauri::command]
pub async fn list_node_catalog(
    traffic_state: tauri::State<'_, crate::commands::traffic_analysis_commands::TrafficAnalysisState>,
) -> Result<Vec<NodeCatalogItem>, String> {
    build_node_catalog(traffic_state.inner()).await
}

/// Build node catalog for use by other commands (includes MCP and enabled plugins).
pub async fn build_node_catalog(
    traffic_state: &crate::commands::traffic_analysis_commands::TrafficAnalysisState,
) -> Result<Vec<NodeCatalogItem>, String> {
    let mut catalog = Vec::new();

    // Trigger nodes
    catalog.push(NodeCatalogItem {
        node_type: "start".to_string(),
        label: "开始".to_string(),
        category: "trigger".to_string(),
        params_schema: serde_json::json!({"type": "object", "properties": {}}),
        input_ports: vec![],
        output_ports: vec![PortDef {
            id: "out".to_string(),
            name: "输出".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
    });

    catalog.push(NodeCatalogItem {
        node_type: "trigger_schedule".to_string(),
        label: "定时触发".to_string(),
        category: "trigger".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "trigger_type": {"type": "string", "enum": ["interval", "daily", "weekly"], "default": "interval"},
                "interval_seconds": {"type": "integer", "default": 60, "description": "间隔秒数（interval模式）"},
                "hour": {"type": "integer", "default": 9, "minimum": 0, "maximum": 23},
                "minute": {"type": "integer", "default": 0, "minimum": 0, "maximum": 59},
                "second": {"type": "integer", "default": 0, "minimum": 0, "maximum": 59},
                "weekdays": {"type": "string", "default": "1,2,3,4,5", "description": "周几触发，逗号分隔"}
            }
        }),
        input_ports: vec![],
        output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
    });

    // Control flow nodes
    catalog.push(NodeCatalogItem {
        node_type: "branch".to_string(),
        label: "条件分支".to_string(),
        category: "control".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "expr": {"type": "string", "default": "true", "description": "条件表达式"}
            }
        }),
        input_ports: vec![PortDef {
            id: "in".to_string(),
            name: "输入".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
        output_ports: vec![
            PortDef {
                id: "true".to_string(),
                name: "真".to_string(),
                port_type: "Json".to_string(),
                required: false,
            },
            PortDef {
                id: "false".to_string(),
                name: "假".to_string(),
                port_type: "Json".to_string(),
                required: false,
            },
        ],
    });

    catalog.push(NodeCatalogItem {
        node_type: "merge".to_string(),
        label: "合并".to_string(),
        category: "control".to_string(),
        params_schema: serde_json::json!({"type": "object", "properties": {}}),
        input_ports: vec![
            PortDef {
                id: "in1".to_string(),
                name: "输入1".to_string(),
                port_type: "Json".to_string(),
                required: false,
            },
            PortDef {
                id: "in2".to_string(),
                name: "输入2".to_string(),
                port_type: "Json".to_string(),
                required: false,
            },
        ],
        output_ports: vec![PortDef {
            id: "out".to_string(),
            name: "输出".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
    });

    catalog.push(NodeCatalogItem {
        node_type: "retry".to_string(),
        label: "重试".to_string(),
        category: "control".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "times": {"type": "integer", "default": 3},
                "delay_ms": {"type": "integer", "default": 500},
                "tool_name": {"type": "string"},
                "tool_params": {"type": "object"}
            }
        }),
        input_ports: vec![PortDef {
            id: "in".to_string(),
            name: "输入".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
        output_ports: vec![PortDef {
            id: "out".to_string(),
            name: "输出".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
    });

    // AI nodes
    catalog.push(NodeCatalogItem {
        node_type: "ai_chat".to_string(),
        label: "AI对话".to_string(),
        category: "ai".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "provider": {"type": "string", "x-ui-widget": "ai-provider-select"},
                "model": {"type": "string", "x-ui-widget": "ai-model-select"},
                "prompt": {"type": "string", "x-ui-widget": "textarea", "description": "用户消息，支持 {{input}} 变量"},
                "system_prompt": {"type": "string", "x-ui-widget": "textarea", "description": "系统提示词"}
            }
        }),
        input_ports: vec![PortDef { id: "in".to_string(), name: "输入".to_string(), port_type: "Json".to_string(), required: false }],
        output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
    });

    catalog.push(NodeCatalogItem {
        node_type: "ai_agent".to_string(),
        label: "AI Agent".to_string(),
        category: "ai".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "provider": {"type": "string", "x-ui-widget": "ai-provider-select"},
                "model": {"type": "string", "x-ui-widget": "ai-model-select"},
                "prompt": {"type": "string", "x-ui-widget": "textarea"},
                "system_prompt": {"type": "string", "x-ui-widget": "textarea"},
                "tools": {"type": "array", "x-ui-widget": "tools-multiselect", "items": {"type": "string"}}
            }
        }),
        input_ports: vec![PortDef { id: "in".to_string(), name: "输入".to_string(), port_type: "Json".to_string(), required: false }],
        output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
    });

    // Builtin tools as nodes
    let builtin_tools = get_builtin_tools_with_status().await?;
    for tool in builtin_tools {
        catalog.push(NodeCatalogItem {
            node_type: format!("tool::{}", tool.name),
            label: tool.name.clone(),
            category: "tool".to_string(),
            params_schema: tool
                .input_schema
                .unwrap_or(serde_json::json!({"type": "object", "properties": {}})),
            input_ports: vec![PortDef {
                id: "in".to_string(),
                name: "输入".to_string(),
                port_type: "Json".to_string(),
                required: false,
            }],
            output_ports: vec![PortDef {
                id: "out".to_string(),
                name: "输出".to_string(),
                port_type: "Json".to_string(),
                required: false,
            }],
        });
    }

    // RAG nodes
    catalog.push(NodeCatalogItem {
        node_type: "rag::ingest".to_string(),
        label: "RAG导入".to_string(),
        category: "data".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {"type": "string", "description": "文件路径"},
                "collection_id": {"type": "string", "description": "集合ID"},
                "metadata": {"type": "object", "description": "元数据"}
            },
            "required": ["file_path"]
        }),
        input_ports: vec![PortDef {
            id: "in".to_string(),
            name: "输入".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
        output_ports: vec![PortDef {
            id: "out".to_string(),
            name: "输出".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
    });

    catalog.push(NodeCatalogItem {
        node_type: "rag::query".to_string(),
        label: "RAG查询".to_string(),
        category: "data".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "查询内容"},
                "collection_id": {"type": "string"},
                "top_k": {"type": "integer", "default": 5},
                "use_mmr": {"type": "boolean", "default": false},
                "mmr_lambda": {"type": "number", "default": 0.5}
            },
            "required": ["query"]
        }),
        input_ports: vec![PortDef {
            id: "in".to_string(),
            name: "输入".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
        output_ports: vec![PortDef {
            id: "out".to_string(),
            name: "输出".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
    });

    // Notification node
    catalog.push(NodeCatalogItem {
        node_type: "notify".to_string(),
        label: "通知".to_string(),
        category: "output".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "notification_rule_id": {"type": "string", "description": "通知规则ID"},
                "title": {"type": "string", "default": "Workflow Notification"},
                "content": {"type": "string", "x-ui-widget": "textarea"},
                "use_input_as_content": {"type": "boolean", "default": false}
            }
        }),
        input_ports: vec![PortDef {
            id: "in".to_string(),
            name: "输入".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
        output_ports: vec![PortDef {
            id: "out".to_string(),
            name: "输出".to_string(),
            port_type: "Json".to_string(),
            required: false,
        }],
    });

    // Prompt build node
    catalog.push(NodeCatalogItem {
        node_type: "prompt::build".to_string(),
        label: "构建Prompt".to_string(),
        category: "data".to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "build_type": {"type": "string", "enum": ["Planner", "Executor", "Replanner", "ReportGenerator"], "default": "Planner"},
                "user_query": {"type": "string", "x-ui-widget": "textarea"}
            }
        }),
        input_ports: vec![PortDef { id: "in".to_string(), name: "输入".to_string(), port_type: "Json".to_string(), required: false }],
        output_ports: vec![PortDef { id: "out".to_string(), name: "输出".to_string(), port_type: "Json".to_string(), required: false }],
    });

    // MCP 工具节点 - 从已连接的 MCP 服务器获取
    let mcp_tools = crate::commands::mcp_commands::mcp_get_all_tools()
        .await
        .unwrap_or_default();
    for tool in mcp_tools {
        let server_name = tool
            .get("server_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let tool_name = tool
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let description = tool.get("description").and_then(|v| v.as_str());
        let input_schema = tool
            .get("input_schema")
            .cloned()
            .unwrap_or(serde_json::json!({"type": "object", "properties": {}}));

        catalog.push(NodeCatalogItem {
            node_type: format!("mcp::{}::{}", server_name, tool_name),
            label: format!("[{}] {}", server_name, tool_name),
            category: "mcp".to_string(),
            params_schema: input_schema,
            input_ports: vec![PortDef {
                id: "in".to_string(),
                name: "输入".to_string(),
                port_type: "Json".to_string(),
                required: false,
            }],
            output_ports: vec![PortDef {
                id: "out".to_string(),
                name: "输出".to_string(),
                port_type: "Json".to_string(),
                required: false,
            }],
        });

        tracing::debug!(
            "Added MCP tool node: mcp::{}::{} - {:?}",
            server_name,
            tool_name,
            description
        );
    }

    // Agent 插件工具节点 - 从数据库获取已启用的 Agent 工具插件
    if let Ok(plugins) = traffic_state.list_plugins_internal().await {
        // 获取数据库服务用于查询插件代码
        let db_service = Some(traffic_state.get_db_service());

        for plugin in plugins {
            // 只添加已启用的 Agent 类型插件
            if plugin.status == sentinel_traffic::PluginStatus::Enabled
                && plugin.metadata.main_category == "agent"
            {
                // 获取插件代码并通过运行时获取 schema（优先），静态解析作为 fallback
                let params_schema = if let Some(ref db) = db_service {
                    if let Ok(Some(code)) = db.get_traffic_plugin_code(&plugin.metadata.id).await {
                        // 使用运行时方法获取 schema
                        get_plugin_input_schema_async(
                            &plugin.metadata.id,
                            &plugin.metadata.name,
                            &code,
                        )
                        .await
                    } else {
                        serde_json::json!({
                            "type": "object",
                            "properties": {
                                "input": {"type": "string", "x-ui-widget": "textarea", "description": "工具输入参数"}
                            }
                        })
                    }
                } else {
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "input": {"type": "string", "x-ui-widget": "textarea", "description": "工具输入参数"}
                        }
                    })
                };

                catalog.push(NodeCatalogItem {
                    node_type: format!("plugin::{}", plugin.metadata.id),
                    label: plugin.metadata.name.clone(),
                    category: "plugin".to_string(),
                    params_schema,
                    input_ports: vec![PortDef {
                        id: "in".to_string(),
                        name: "输入".to_string(),
                        port_type: "Json".to_string(),
                        required: false,
                    }],
                    output_ports: vec![PortDef {
                        id: "out".to_string(),
                        name: "输出".to_string(),
                        port_type: "Json".to_string(),
                        required: false,
                    }],
                });

                tracing::debug!(
                    "Added Agent plugin node: plugin::{} - {}",
                    plugin.metadata.id,
                    plugin.metadata.name
                );
            }
        }
    }

    Ok(catalog)
}

// ============================================================================
// Tool Metadata Management Commands
// ============================================================================

use crate::agents::tool_router::{
    clear_tool_usage_records, get_tool_usage_statistics, ToolCategory, ToolMetadata, ToolRouter,
    ToolStatistics, ToolUsageStatistics,
};

/// Get all tool metadata
#[tauri::command]
pub async fn get_all_tool_metadata(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<ToolMetadata>, String> {
    let router = ToolRouter::new_with_all_tools(Some(db_service.inner())).await;
    Ok(router.list_all_tools())
}

/// Get tool metadata by category
#[tauri::command]
pub async fn get_tools_by_category(
    category: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<ToolMetadata>, String> {
    let router = ToolRouter::new_with_all_tools(Some(db_service.inner())).await;

    let category_enum = match category.to_lowercase().as_str() {
        "network" => ToolCategory::Network,
        "security" => ToolCategory::Security,
        "data" => ToolCategory::Data,
        "ai" => ToolCategory::AI,
        "system" => ToolCategory::System,
        "mcp" => ToolCategory::MCP,
        "plugin" => ToolCategory::Plugin,
        "workflow" => ToolCategory::Workflow,
        "browser" => ToolCategory::Browser,
        _ => return Err(format!("Unknown category: {}", category)),
    };

    Ok(router.list_tools_by_category(category_enum))
}

/// Search tools by query
#[tauri::command]
pub async fn search_tools(
    query: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<ToolMetadata>, String> {
    let router = ToolRouter::new_with_all_tools(Some(db_service.inner())).await;
    Ok(router.search_tools(&query))
}

/// Get tool statistics
#[tauri::command]
pub async fn get_tool_statistics(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<ToolStatistics, String> {
    let router = ToolRouter::new_with_all_tools(Some(db_service.inner())).await;
    Ok(router.get_statistics())
}

/// Get tool metadata by ID
#[tauri::command]
pub async fn get_tool_metadata(
    tool_id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Option<ToolMetadata>, String> {
    let router = ToolRouter::new_with_all_tools(Some(db_service.inner())).await;
    Ok(router.get_tool_metadata(&tool_id))
}

/// Get tool usage statistics
#[tauri::command]
pub async fn get_tool_usage_stats() -> Result<ToolUsageStatistics, String> {
    Ok(get_tool_usage_statistics().await)
}

/// Clear tool usage records
#[tauri::command]
pub async fn clear_tool_usage_stats() -> Result<(), String> {
    clear_tool_usage_records().await;
    Ok(())
}

pub mod tool_server;
pub use tool_server::{
    execute_tool_server_tool, get_tool_server_stats, get_tool_server_tool, init_tool_server,
    list_tool_server_tools, list_tools_by_source, refresh_all_dynamic_tools,
    register_mcp_tools_from_server, register_workflow_tools,
};

// ============================================================================
// Vision Explorer Credential Commands (V2 Compatible)
// ============================================================================

// Note: vision_bridge is disabled after ReAct refactoring
// The new ReAct architecture doesn't require these bridge commands
// mod vision_bridge;

mod shell_permissions;
pub use shell_permissions::PendingPermissionRequest;

mod agent_config;
pub use agent_config::AgentConfig;

mod ability_groups;

#[tauri::command]
pub async fn init_shell_permission_handler(app: tauri::AppHandle) -> Result<(), String> {
    shell_permissions::init_shell_permission_handler(app).await
}

#[tauri::command]
pub async fn get_shell_tool_config() -> Result<ShellConfig, String> {
    shell_permissions::get_shell_tool_config().await
}

#[tauri::command]
pub async fn set_shell_tool_config(config: ShellConfig) -> Result<(), String> {
    shell_permissions::set_shell_tool_config(config).await
}

#[tauri::command]
pub async fn respond_shell_permission(id: String, allowed: bool) -> Result<(), String> {
    shell_permissions::respond_shell_permission(id, allowed).await
}

#[tauri::command]
pub async fn get_pending_shell_permissions() -> Result<Vec<PendingPermissionRequest>, String> {
    shell_permissions::get_pending_shell_permissions().await
}

#[tauri::command]
pub async fn get_agent_config(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<AgentConfig, String> {
    agent_config::get_agent_config(db_service).await
}

#[tauri::command]
pub async fn save_agent_config(
    config: AgentConfig,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<(), String> {
    agent_config::save_agent_config(config, db_service).await
}

#[tauri::command]
pub async fn list_ability_groups(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<sentinel_db::AbilityGroupSummary>, String> {
    ability_groups::list_ability_groups(db_service).await
}

#[tauri::command]
pub async fn list_ability_groups_full(
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Vec<sentinel_db::AbilityGroup>, String> {
    ability_groups::list_ability_groups_full(db_service).await
}

#[tauri::command]
pub async fn get_ability_group_detail(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Option<sentinel_db::AbilityGroupDetail>, String> {
    ability_groups::get_ability_group_detail(id, db_service).await
}

#[tauri::command]
pub async fn get_ability_group(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<Option<sentinel_db::AbilityGroup>, String> {
    ability_groups::get_ability_group(id, db_service).await
}

#[tauri::command]
pub async fn create_ability_group(
    payload: sentinel_db::CreateAbilityGroup,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<sentinel_db::AbilityGroup, String> {
    ability_groups::create_ability_group(payload, db_service).await
}

#[tauri::command]
pub async fn update_ability_group(
    id: String,
    payload: sentinel_db::UpdateAbilityGroup,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    ability_groups::update_ability_group(id, payload, db_service).await
}

#[tauri::command]
pub async fn delete_ability_group(
    id: String,
    db_service: tauri::State<'_, Arc<sentinel_db::DatabaseService>>,
) -> Result<bool, String> {
    ability_groups::delete_ability_group(id, db_service).await
}

use serde::{Deserialize, Serialize};
use serde_json::json;

/// Orchestrator tool definitions
/// 
/// These tools define the interface for Orchestrator's sub-agent coordination.
/// Note: Currently these are schema definitions; actual dispatch is handled
/// in Rust via SubAgentExecutor, not through LLM tool calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorTool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub fn get_orchestrator_tools() -> Vec<OrchestratorTool> {
    vec![
        OrchestratorTool {
            name: "call_plan_agent".to_string(),
            description: "Invoke the ReWOO sub-agent to create a multi-branch security test plan. Use this when you need to decompose complex objectives into parallel/sequential tasks or create a comprehensive roadmap.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "objective": {
                        "type": "string",
                        "description": "Clear description of what needs to be planned"
                    },
                    "constraints": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Optional list of limitations or requirements"
                    }
                },
                "required": ["objective"]
            }),
        },
        OrchestratorTool {
            name: "call_execution_agent".to_string(),
            description: "Invoke the Plan-and-Execute sub-agent to execute a linear task chain. Use this for step-by-step operations that maintain state (e.g., login → enumerate → test).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "objective": {
                        "type": "string",
                        "description": "Clear description of the execution goal"
                    },
                    "constraints": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Optional list of limitations or requirements"
                    }
                },
                "required": ["objective"]
            }),
        },
        OrchestratorTool {
            name: "call_compiler_agent".to_string(),
            description: "Invoke the LLM-Compiler sub-agent to generate code, scripts, or payloads. Use this when you need to create test automation, craft exploits, or generate fuzz templates.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "objective": {
                        "type": "string",
                        "description": "Clear description of what code/script is needed"
                    },
                    "language": {
                        "type": "string",
                        "description": "Target programming language (e.g., python, javascript, bash)"
                    },
                    "context": {
                        "type": "string",
                        "description": "Additional context like API schemas, error messages, or requirements"
                    }
                },
                "required": ["objective"]
            }),
        },
        OrchestratorTool {
            name: "update_session_state".to_string(),
            description: "Update the current test session state, including stage and summary.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "stage": {
                        "type": "string",
                        "enum": [
                            "recon", "login", "api_mapping", "vuln_scan", "exploit",
                            "log_collection", "timeline_reconstruction", "ioc_extraction", "behavior_analysis",
                            "challenge_analysis", "vuln_identification", "payload_crafting", "flag_extraction", "writeup",
                            "binary_loading", "static_analysis", "dynamic_analysis", "deobfuscation", "behavior_summary",
                            "report", "completed"
                        ],
                        "description": "New stage to transition to"
                    },
                    "summary": {
                        "type": "string",
                        "description": "Brief summary of current progress"
                    }
                },
                "required": ["stage", "summary"]
            }),
        },
        OrchestratorTool {
            name: "record_finding".to_string(),
            description: "Record a security finding or vulnerability discovered during testing.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "URL/endpoint/file where finding was discovered"
                    },
                    "method": {
                        "type": "string",
                        "description": "HTTP method if applicable (GET, POST, etc.)"
                    },
                    "risk_level": {
                        "type": "string",
                        "enum": ["info", "low", "medium", "high", "critical"],
                        "description": "Risk level of the finding"
                    },
                    "title": {
                        "type": "string",
                        "description": "Brief title of the finding"
                    },
                    "description": {
                        "type": "string",
                        "description": "Detailed description of the vulnerability"
                    },
                    "evidence": {
                        "type": "string",
                        "description": "Supporting evidence (request/response, logs, screenshots)"
                    },
                    "reproduction_steps": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Steps to reproduce the finding"
                    }
                },
                "required": ["location", "risk_level", "title", "description", "evidence"]
            }),
        },
        OrchestratorTool {
            name: "update_auth_context".to_string(),
            description: "Update the authentication context for the session (cookies, tokens, headers).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "cookies": {
                        "type": "object",
                        "description": "Dictionary of cookie name-value pairs"
                    },
                    "headers": {
                        "type": "object",
                        "description": "Dictionary of header name-value pairs"
                    },
                    "tokens": {
                        "type": "object",
                        "description": "Dictionary of token type-value pairs (e.g., bearer, api_key)"
                    }
                },
                "required": []
            }),
        },
    ]
}

/// Tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

impl ToolResult {
    pub fn success(output: String) -> Self {
        Self {
            success: true,
            output,
            error: None,
        }
    }
    
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error),
        }
    }
}


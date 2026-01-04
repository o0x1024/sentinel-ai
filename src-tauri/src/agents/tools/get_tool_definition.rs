//! Get Tool Definition Tool - 渐进式工具披露的核心工具
//!
//! 允许 LLM 按需查询工具的详细定义，避免一次性加载所有工具 schema

use crate::agents::tool_router::ToolRouter;
use rig::completion::ToolDefinition;
use rig::tool::{Tool, ToolError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct GetToolDefinitionArgs {
    /// Tool name or ID to get definition for
    pub tool_name: String,
}

/// Tool Definition Tool - 获取工具的详细定义
#[derive(Clone)]
pub struct GetToolDefinitionTool {
    tool_router: Arc<ToolRouter>,
}

impl GetToolDefinitionTool {
    pub fn new(tool_router: Arc<ToolRouter>) -> Self {
        Self { tool_router }
    }
}

impl Tool for GetToolDefinitionTool {
    const NAME: &'static str = "get_tool_definition";

    type Error = ToolError;
    type Args = GetToolDefinitionArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get detailed definition, parameters, and usage instructions for a specific tool. Use this when you need to know how to use a tool before calling it.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tool_name": {
                        "type": "string",
                        "description": "The name or ID of the tool to get definition for"
                    }
                },
                "required": ["tool_name"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!("GetToolDefinitionTool: Fetching definition for '{}'", args.tool_name);

        // Get tool metadata
        let all_tools = self.tool_router.get_all_available_tools();
        
        // Normalize tool name for matching (replace :: with __)
        let normalized_query = args.tool_name.replace("::", "__");
        
        let tool_meta = all_tools
            .iter()
            .find(|t| {
                t.name.eq_ignore_ascii_case(&args.tool_name)
                    || t.id == args.tool_name
                    || t.id == normalized_query
                    || t.name.eq_ignore_ascii_case(&normalized_query)
            })
            .ok_or_else(|| {
                ToolError::ToolCallError(
                    format!(
                        "Tool '{}' not found. Available tools can be seen in the system prompt.",
                        args.tool_name
                    )
                    .into(),
                )
            })?;

        // Get full tool definition with parameters
        let tool_def_result = self.tool_router.get_tool_full_definition(&tool_meta.id).await;
        
        let definition = match tool_def_result {
            Ok(full_def) => {
                // Return full definition with parameters
                json!({
                    "name": tool_meta.name,
                    "id": tool_meta.id,
                    "description": tool_meta.description,
                    "category": format!("{:?}", tool_meta.category),
                    "parameters": full_def.parameters,
                    "usage_notes": full_def.usage_notes,
                    "examples": full_def.examples,
                })
            }
            Err(e) => {
                tracing::warn!("Failed to get full definition for '{}': {}", tool_meta.id, e);
                // Fallback to basic metadata
                json!({
                    "name": tool_meta.name,
                    "id": tool_meta.id,
                    "description": tool_meta.description,
                    "category": format!("{:?}", tool_meta.category),
                    "tags": tool_meta.tags,
                    "note": "Full parameter schema not available. Try using the tool directly."
                })
            }
        };

        let formatted = serde_json::to_string_pretty(&definition)
            .unwrap_or_else(|_| "Failed to format tool definition".to_string());

        tracing::info!(
            "GetToolDefinitionTool: Returned definition for '{}' ({} chars)",
            tool_meta.name,
            formatted.len()
        );

        Ok(formatted)
    }
}

/// Full tool definition with parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFullDefinition {
    pub parameters: serde_json::Value,
    pub usage_notes: Option<String>,
    pub examples: Vec<String>,
}

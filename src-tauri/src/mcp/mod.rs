pub mod client;
pub mod protocol;
pub mod server;
pub mod tool_executor;
pub mod tool_manager;
pub mod tools;
pub mod types;

use std::sync::Arc;

// Re-export core types from the single source of truth, types.rs
pub use self::types::*;

// Re-export key components for easy access from other modules
pub use client::{McpClient, McpClientManager, McpConnection};
pub use server::{McpServerManager, SentinelMcpServer};
pub use tool_manager::ToolManager;

// Re-export types from the underlying rmcp crate, aliasing to avoid name conflicts
pub use rmcp::model::{
    CallToolResult as RmcpCallToolResult, Content, TextContent, Tool as RmcpTool,
};

// Conversion functions to bridge our internal types and rmcp types

/// Converts our internal ToolDefinition to the rmcp Tool type.
pub fn convert_to_rmcp_tool(tool: &ToolDefinition) -> RmcpTool {
    RmcpTool {
        name: tool.name.clone().into(),
        description: Some(tool.description.clone().into()),
        input_schema: Arc::new(tool.input_schema.as_object().cloned().unwrap_or_default()),
        annotations: None,
    }
}

/// Converts rmcp's Content type to our internal, simplified ToolContent.
pub fn convert_from_rmcp_content(content: Content) -> ToolContent {
    let text = match content.raw {
        rmcp::model::RawContent::Text(text_content) => text_content.text,
        _ => serde_json::to_string(&content).unwrap_or_else(|_| "{}".to_string()),
    };
    ToolContent { text }
}

/// Converts our internal ToolContent to rmcp's Content type.
pub fn convert_to_rmcp_content(content: ToolContent) -> Content {
    Content::text(content.text)
} 
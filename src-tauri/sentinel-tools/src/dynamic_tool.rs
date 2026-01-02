//! Dynamic Tool Module
//!
//! Implements dynamic tool registration and Rig Tool trait adaptation.
//! Supports builtin tools, MCP tools, plugin tools, and workflow tools.

use rig::completion::ToolDefinition;
use rig::tool::{Tool, ToolSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tool execution function type
pub type ToolExecutor = Arc<
    dyn Fn(
            Value,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send>>
        + Send
        + Sync,
>;

/// Tool source type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolSource {
    /// Built-in tools (port_scan, http_request, etc.)
    Builtin,
    /// MCP server tools
    Mcp { server_name: String },
    /// Plugin tools
    Plugin { plugin_id: String },
    /// Workflow tools
    Workflow { workflow_id: String },
}

/// Dynamic tool definition
#[derive(Clone)]
pub struct DynamicToolDef {
    /// Tool name (unique identifier)
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON Schema for input parameters
    pub input_schema: Value,
    /// JSON Schema for output
    pub output_schema: Option<Value>,
    /// Tool source
    pub source: ToolSource,
    /// Tool executor function
    pub executor: ToolExecutor,
}

impl std::fmt::Debug for DynamicToolDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicToolDef")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("source", &self.source)
            .finish()
    }
}

/// Dynamic tool error
#[derive(Debug, thiserror::Error)]
pub enum DynamicToolError {
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("Invalid output: {0}")]
    InvalidOutput(String),
    #[error("Tool not found: {0}")]
    NotFound(String),
}

/// Dynamic tool instance - implements Rig's Tool trait
#[derive(Clone)]
pub struct DynamicTool {
    def: DynamicToolDef,
}

impl DynamicTool {
    pub fn new(def: DynamicToolDef) -> Self {
        Self { def }
    }

    pub fn name(&self) -> &str {
        &self.def.name
    }

    pub fn description(&self) -> &str {
        &self.def.description
    }

    pub fn source(&self) -> &ToolSource {
        &self.def.source
    }
}

/// Implementation of Rig's Tool trait for DynamicTool
impl Tool for DynamicTool {
    const NAME: &'static str = "dynamic_tool";
    type Args = Value;
    type Output = Value;
    type Error = DynamicToolError;

    /// Override name() to return the actual tool name instead of const NAME
    fn name(&self) -> String {
        self.def.name.clone()
    }

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.def.name.clone(),
            description: self.def.description.clone(),
            parameters: self.def.input_schema.clone(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = self.def.executor.clone();
        if should_validate_schema(&self.def.input_schema) {
            validate_schema(&self.def.input_schema, &args)
                .map_err(DynamicToolError::InvalidArguments)?;
        }

        let result = executor(args)
            .await
            .map_err(DynamicToolError::ExecutionFailed)?;

        if let Some(schema) = &self.def.output_schema {
            if should_validate_schema(schema) {
                validate_schema(schema, &result)
                    .map_err(DynamicToolError::InvalidOutput)?;
            }
        }

        Ok(result)
    }
}

/// Tool registry - manages all dynamic tools
#[derive(Default)]
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, DynamicToolDef>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    /// Register a tool
    pub async fn register(&self, def: DynamicToolDef) {
        let mut tools = self.tools.write().await;
        tracing::info!("Registering tool: {} (source: {:?})", def.name, def.source);
        tools.insert(def.name.clone(), def);
    }

    /// Unregister a tool
    pub async fn unregister(&self, name: &str) -> bool {
        let mut tools = self.tools.write().await;
        tools.remove(name).is_some()
    }

    /// Get a tool by name
    pub async fn get(&self, name: &str) -> Option<DynamicToolDef> {
        let tools = self.tools.read().await;
        tools.get(name).cloned()
    }

    /// List all tools
    pub async fn list(&self) -> Vec<DynamicToolDef> {
        let tools = self.tools.read().await;
        tools.values().cloned().collect()
    }

    /// List tools by source
    pub async fn list_by_source(&self, source: &ToolSource) -> Vec<DynamicToolDef> {
        let tools = self.tools.read().await;
        tools
            .values()
            .filter(|t| &t.source == source)
            .cloned()
            .collect()
    }

    /// Clear all tools of a specific source
    pub async fn clear_by_source(&self, source: &ToolSource) {
        let mut tools = self.tools.write().await;
        tools.retain(|_, t| &t.source != source);
    }

    /// Get tool count
    pub async fn count(&self) -> usize {
        let tools = self.tools.read().await;
        tools.len()
    }

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, args: Value) -> Result<Value, DynamicToolError> {
        let def = self
            .get(name)
            .await
            .ok_or_else(|| DynamicToolError::NotFound(name.to_string()))?;

        (def.executor)(args)
            .await
            .map_err(DynamicToolError::ExecutionFailed)
    }

    /// Create a ToolSet from registered tools
    pub async fn create_toolset(&self, tool_names: &[String]) -> ToolSet {
        let mut toolset = ToolSet::default();
        let tools = self.tools.read().await;

        for name in tool_names {
            if let Some(def) = tools.get(name) {
                let tool = DynamicTool::new(def.clone());
                // Note: We use add_tool which requires the tool to implement Tool trait
                // Since our DynamicTool has const NAME = "dynamic_tool", we need a workaround
                toolset.add_dyn_tool(tool);
            }
        }

        toolset
    }

    /// Get DynamicTool instances for selected tools
    pub async fn get_dynamic_tools(&self, tool_names: &[String]) -> Vec<DynamicTool> {
        let tools = self.tools.read().await;
        let mut result = Vec::new();

        for name in tool_names {
            if let Some(def) = tools.get(name) {
                result.push(DynamicTool::new(def.clone()));
            }
        }

        result
    }

    /// Get tool definitions for LLM
    pub async fn get_definitions(&self, tool_names: &[String]) -> Vec<ToolDefinition> {
        let tools = self.tools.read().await;
        let mut definitions = Vec::new();

        for name in tool_names {
            if let Some(def) = tools.get(name) {
                definitions.push(ToolDefinition {
                    name: def.name.clone(),
                    description: def.description.clone(),
                    parameters: def.input_schema.clone(),
                });
            }
        }

        definitions
    }
}

/// Helper trait extension for ToolSet to add dynamic tools
trait ToolSetExt {
    fn add_dyn_tool(&mut self, tool: DynamicTool);
}

impl ToolSetExt for ToolSet {
    fn add_dyn_tool(&mut self, tool: DynamicTool) {
        // Add the DynamicTool directly since it implements Tool
        self.add_tool(tool);
    }
}

/// Create executor from async function
pub fn create_executor<F, Fut>(f: F) -> ToolExecutor
where
    F: Fn(Value) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<Value, String>> + Send + 'static,
{
    Arc::new(move |args| Box::pin(f(args)))
}

/// Builder for creating DynamicToolDef
pub struct DynamicToolBuilder {
    name: String,
    description: String,
    input_schema: Value,
    output_schema: Option<Value>,
    source: ToolSource,
    executor: Option<ToolExecutor>,
}

impl DynamicToolBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
            output_schema: None,
            source: ToolSource::Builtin,
            executor: None,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn input_schema(mut self, schema: Value) -> Self {
        self.input_schema = schema;
        self
    }

    pub fn output_schema(mut self, schema: Option<Value>) -> Self {
        self.output_schema = schema;
        self
    }

    pub fn source(mut self, source: ToolSource) -> Self {
        self.source = source;
        self
    }

    pub fn executor<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Value, String>> + Send + 'static,
    {
        self.executor = Some(create_executor(f));
        self
    }

    pub fn executor_raw(mut self, executor: ToolExecutor) -> Self {
        self.executor = Some(executor);
        self
    }

    pub fn build(self) -> Result<DynamicToolDef, String> {
        let executor = self.executor.ok_or("Executor is required")?;

        Ok(DynamicToolDef {
            name: self.name,
            description: self.description,
            input_schema: self.input_schema,
            output_schema: self.output_schema,
            source: self.source,
            executor,
        })
    }
}

fn should_validate_schema(schema: &Value) -> bool {
    match schema {
        Value::Null => false,
        Value::Object(map) => !map.is_empty(),
        Value::Array(arr) => !arr.is_empty(),
        _ => true,
    }
}

fn validate_schema(schema: &Value, instance: &Value) -> Result<(), String> {
    let compiled = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(schema)
        .map_err(|e| format!("schema compile failed: {}", e))?;

    if let Err(errors) = compiled.validate(instance) {
        let details = errors
            .into_iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(details);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_registry() {
        let registry = ToolRegistry::new();

        let tool_def = DynamicToolBuilder::new("test_tool")
            .description("A test tool")
            .input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            }))
            .executor(|args| async move {
                Ok(serde_json::json!({
                    "result": format!("Got: {:?}", args)
                }))
            })
            .build()
            .unwrap();

        registry.register(tool_def).await;

        assert_eq!(registry.count().await, 1);
        assert!(registry.get("test_tool").await.is_some());

        let result = registry
            .execute("test_tool", serde_json::json!({"input": "hello"}))
            .await
            .unwrap();

        assert!(result.get("result").is_some());
    }
}

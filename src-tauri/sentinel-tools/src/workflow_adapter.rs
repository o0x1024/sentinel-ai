//! Workflow Tool Adapter
//!
//! Adapts workflow definitions to the unified tool system.
//! Supports loading workflows as tools and executing them.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::dynamic_tool::{create_executor, DynamicToolDef, ToolExecutor, ToolSource};
use crate::tool_server::ToolServer;

/// Workflow tool metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowToolMeta {
    pub workflow_id: String,
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub is_tool: bool,
}

/// Workflow execution context
#[derive(Debug, Clone)]
pub struct WorkflowContext {
    pub workflow_id: String,
    pub name: String,
    pub definition: Value,
}

/// Global workflow registry
static WORKFLOW_CONTEXTS: once_cell::sync::Lazy<RwLock<HashMap<String, WorkflowContext>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Workflow executor callback type
pub type WorkflowExecutorFn = Arc<
    dyn Fn(
            String,
            Value,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send>>
        + Send
        + Sync,
>;

/// Global workflow executor (set by the application)
static WORKFLOW_EXECUTOR: once_cell::sync::Lazy<RwLock<Option<WorkflowExecutorFn>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(None));

/// Set the workflow executor callback
pub async fn set_workflow_executor(executor: WorkflowExecutorFn) {
    let mut exec = WORKFLOW_EXECUTOR.write().await;
    *exec = Some(executor);
}

/// Register workflow context
pub async fn register_workflow_context(ctx: WorkflowContext) {
    let mut contexts = WORKFLOW_CONTEXTS.write().await;
    contexts.insert(ctx.workflow_id.clone(), ctx);
}

/// Unregister workflow context
pub async fn unregister_workflow_context(workflow_id: &str) {
    let mut contexts = WORKFLOW_CONTEXTS.write().await;
    contexts.remove(workflow_id);
}

/// Get workflow context
pub async fn get_workflow_context(workflow_id: &str) -> Option<WorkflowContext> {
    let contexts = WORKFLOW_CONTEXTS.read().await;
    contexts.get(workflow_id).cloned()
}

/// List all workflow contexts
pub async fn list_workflow_contexts() -> Vec<WorkflowContext> {
    let contexts = WORKFLOW_CONTEXTS.read().await;
    contexts.values().cloned().collect()
}

/// Create workflow tool executor
pub fn create_workflow_executor(workflow_id: String) -> ToolExecutor {
    create_executor(move |args: Value| {
        let wid = workflow_id.clone();

        async move {
            let ctx = get_workflow_context(&wid)
                .await
                .ok_or_else(|| format!("Workflow '{}' not registered", wid))?;

            execute_workflow_internal(&ctx, args).await
        }
    })
}

/// Internal workflow execution
async fn execute_workflow_internal(ctx: &WorkflowContext, args: Value) -> Result<Value, String> {
    tracing::info!("Executing workflow: {} (id: {})", ctx.name, ctx.workflow_id);

    // Check if we have a workflow executor
    let executor_opt = WORKFLOW_EXECUTOR.read().await;

    if let Some(executor) = executor_opt.as_ref() {
        // Use the registered executor
        executor(ctx.workflow_id.clone(), args).await
    } else {
        // Fallback: return placeholder result
        Ok(serde_json::json!({
            "workflow_id": ctx.workflow_id,
            "workflow_name": ctx.name,
            "input": args,
            "status": "pending",
            "message": "Workflow execution initiated. Use WorkflowStudio to monitor progress."
        }))
    }
}

/// Load and register workflow tools to server
pub async fn load_workflow_tools_to_server(
    tool_server: &ToolServer,
    workflows: Vec<WorkflowToolMeta>,
) {
    tracing::info!("Loading {} workflow tools", workflows.len());

    for workflow_meta in workflows {
        // Only load workflows marked as tools
        if !workflow_meta.is_tool {
            continue;
        }

        let full_name = format!("workflow::{}", workflow_meta.workflow_id);

        let executor = create_workflow_executor(workflow_meta.workflow_id.clone());

        tool_server
            .register_workflow_tool(
                &workflow_meta.workflow_id,
                &workflow_meta.name,
                &workflow_meta.description,
                workflow_meta.input_schema,
                executor,
            )
            .await;

        tracing::debug!("Registered workflow tool: {}", full_name);
    }
}

/// Refresh workflow tools
pub async fn refresh_workflow_tools(tool_server: &ToolServer) {
    tool_server.clear_workflow_tools().await;
    // Workflow tools will be reloaded by the caller with fresh data from database
}

/// Workflow tool adapter
pub struct WorkflowToolAdapter;

impl WorkflowToolAdapter {
    /// Create a DynamicToolDef from workflow metadata
    pub fn create_tool_def(meta: &WorkflowToolMeta) -> DynamicToolDef {
        let workflow_id = meta.workflow_id.clone();
        let full_name = format!("workflow::{}", workflow_id);

        DynamicToolDef {
            name: full_name,
            description: meta.description.clone(),
            input_schema: meta.input_schema.clone(),
            source: ToolSource::Workflow {
                workflow_id: workflow_id.clone(),
            },
            executor: create_workflow_executor(workflow_id),
        }
    }

    /// Extract input schema from workflow definition
    pub async fn extract_input_schema(definition: &Value, tool_server: Option<&ToolServer>) -> Value {
        let keys: Vec<String> = definition.as_object()
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default();
        tracing::info!(
            "Extracting input schema from workflow definition (keys: {:?})",
            keys
        );

        // helper to build schema from a list of param objects or a params map
        let build_schema_from_params = |params: &Value| -> Option<Value> {
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();

            if let Some(arr) = params.as_array() {
                for item in arr {
                    // Handle simple string inputs (just names)
                    if let Some(name) = item.as_str() {
                        properties.insert(
                            name.to_string(),
                            serde_json::json!({
                                "type": "string",
                                "description": format!("Input parameter: {}", name)
                            }),
                        );
                    } 
                    // Handle object inputs {"name": "foo", "type": "string", ...}
                    else if let Some(obj) = item.as_object() {
                        if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                            let type_str = obj.get("type").and_then(|v| v.as_str()).unwrap_or("string");
                            let desc = obj
                                .get("label")
                                .or(obj.get("description"))
                                .and_then(|v| v.as_str())
                                .unwrap_or(name);
                            
                            properties.insert(
                                name.to_string(),
                                serde_json::json!({
                                    "type": type_str,
                                    "description": desc
                                }),
                            );
                            
                            // Check if required
                            if obj.get("required").and_then(|v| v.as_bool()).unwrap_or(false) {
                                required.push(name.to_string());
                            }
                        }
                    }
                }
            } else if let Some(obj) = params.as_object() {
                // Handle map inputs {"foo": {"type": "string"}}
                for (name, spec) in obj {
                    if name == "schema" { continue; } // skip schema key if it's mixed in
                    
                    let mut prop = spec.clone();
                    if !prop.is_object() {
                        // If spec is just a string (type), wrap it
                        if let Some(type_str) = spec.as_str() {
                            prop = serde_json::json!({"type": type_str});
                        } else {
                            prop = serde_json::json!({"type": "string"});
                        }
                    }
                    properties.insert(name.clone(), prop);
                }
            }

            if !properties.is_empty() {
                let mut schema = serde_json::json!({
                    "type": "object",
                    "properties": properties
                });
                if !required.is_empty() {
                    schema.as_object_mut()?.insert("required".to_string(), serde_json::json!(required));
                }
                return Some(schema);
            }
            None
        };

        // 1. Check "inputs" key (most common)
        if let Some(inputs) = definition.get("inputs") {
            // Case 1a: inputs has a "schema" field (direct JSON schema)
            if let Some(schema) = inputs.get("schema") {
                return schema.clone();
            }
            // Case 1b: inputs is the parameters definition itself
            if let Some(schema) = build_schema_from_params(inputs) {
                return schema;
            }
        }

        // 2. Check "parameters" key (alias)
        if let Some(params) = definition.get("parameters") {
             if let Some(schema) = build_schema_from_params(params) {
                return schema;
            }
        }

        // 3. Try "input_schema" at root
        if let Some(schema) = definition.get("input_schema") {
            return schema.clone();
        }

        // 4. Try to find from nodes (Start/trigger block or first node)
        if let Some(nodes) = definition.get("nodes").and_then(|n| n.as_array()) {
            tracing::debug!("Workflow has {} nodes, checking for start/trigger node", nodes.len());
            
            for node in nodes {
                // Check both "type" and "node_type" (different formats)
                let node_type = node.get("node_type")
                    .or(node.get("type"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                
                tracing::debug!("Checking node type: '{}' for workflow input schema", node_type);
                
                if node_type == "start" || node_type == "trigger" || node_type == "webhook" 
                    || node_type == "trigger_schedule" || node_type == "input" {
                    
                    // Method 1: Check for input_ports array (standard node format)
                    if let Some(input_ports) = node.get("input_ports").and_then(|p| p.as_array()) {
                        if !input_ports.is_empty() {
                            let mut properties = serde_json::Map::new();
                            let mut required = Vec::new();
                            
                            for port in input_ports {
                                if let Some(port_obj) = port.as_object() {
                                    let port_id = port_obj.get("id").and_then(|v| v.as_str()).unwrap_or("input");
                                    
                                    // Skip generic flow ports like "in", "flow", "trigger"
                                    if port_id == "in" || port_id == "flow" || port_id == "trigger" || port_id == "input" {
                                        continue;
                                    }

                                    let port_name = port_obj.get("name").and_then(|v| v.as_str()).unwrap_or(port_id);
                                    let port_type = port_obj.get("port_type").and_then(|v| v.as_str()).unwrap_or("string");
                                    let is_required = port_obj.get("required").and_then(|v| v.as_bool()).unwrap_or(false);
                                    
                                    // Convert port_type to JSON schema type
                                    let json_type = match port_type.to_lowercase().as_str() {
                                        "integer" | "int" | "number" => "number",
                                        "boolean" | "bool" => "boolean",
                                        "array" => "array",
                                        "object" | "json" => "object",
                                        _ => "string",
                                    };
                                    
                                    properties.insert(port_id.to_string(), serde_json::json!({
                                        "type": json_type,
                                        "description": port_name
                                    }));
                                    
                                    if is_required {
                                        required.push(port_id.to_string());
                                    }
                                }
                            }
                            
                            if !properties.is_empty() {
                                let mut schema = serde_json::json!({
                                    "type": "object",
                                    "properties": properties
                                });
                                if !required.is_empty() {
                                    schema.as_object_mut().unwrap().insert("required".to_string(), serde_json::json!(required));
                                }
                                tracing::info!("Extracted workflow input schema from input_ports: {:?}", schema);
                                return schema;
                            }
                        }
                    }
                    
                    // Method 2: Check params for schema definition
                    if let Some(params) = node.get("params").and_then(|p| p.as_object()) {
                        // Check if params has input_schema or schema field
                        if let Some(schema) = params.get("input_schema").or(params.get("schema")) {
                            tracing::info!("Found input_schema in node params");
                            return schema.clone();
                        }
                        
                        // Convert params to schema (each param key becomes a property)
                        if !params.is_empty() {
                            let mut properties = serde_json::Map::new();
                            for (key, value) in params {
                                // Skip internal fields
                                if key.starts_with("_") || key == "type" || key == "node_type" {
                                    continue;
                                }
                                
                                let param_type = if value.is_string() { "string" }
                                    else if value.is_number() { "number" }
                                    else if value.is_boolean() { "boolean" }
                                    else if value.is_array() { "array" }
                                    else { "object" };
                                
                                properties.insert(key.clone(), serde_json::json!({
                                    "type": param_type,
                                    "description": format!("Parameter: {}", key)
                                }));
                            }
                            
                            if !properties.is_empty() {
                                let schema = serde_json::json!({
                                    "type": "object",
                                    "properties": properties
                                });
                                tracing::info!("Extracted workflow input schema from params: {:?}", schema);
                                return schema;
                            }
                        }
                    }
                    
                    // Method 3: Check data/inputs fields
                    if let Some(data) = node.get("data").or(node.get("inputs")) {
                        if let Some(schema) = data.get("input_schema") {
                            tracing::info!("Found input_schema in node data");
                            return schema.clone();
                        }
                        if let Some(inputs) = data.get("inputs") {
                            if let Some(schema) = build_schema_from_params(inputs) {
                                tracing::info!("Built schema from node data.inputs");
                                return schema;
                            }
                        }
                        if let Some(schema) = build_schema_from_params(data) {
                            tracing::info!("Built schema from node data");
                            return schema;
                        }
                    }
                }
            }
            
            // If no start node found, try to use the first node's input_ports
            if let Some(first_node) = nodes.first() {
                if let Some(input_ports) = first_node.get("input_ports").and_then(|p| p.as_array()) {
                    if !input_ports.is_empty() {
                        let mut properties = serde_json::Map::new();
                        let mut required = Vec::new();
                        
                        for port in input_ports {
                            if let Some(port_obj) = port.as_object() {
                                let port_id = port_obj.get("id").and_then(|v| v.as_str()).unwrap_or("input");
                                
                                // Skip generic flow ports like "in", "flow", "trigger"
                                if port_id == "in" || port_id == "flow" || port_id == "trigger" || port_id == "input" {
                                    continue;
                                }

                                let port_name = port_obj.get("name").and_then(|v| v.as_str()).unwrap_or(port_id);
                                let port_type = port_obj.get("port_type").and_then(|v| v.as_str()).unwrap_or("string");
                                let is_required = port_obj.get("required").and_then(|v| v.as_bool()).unwrap_or(false);
                                
                                let json_type = match port_type.to_lowercase().as_str() {
                                    "integer" | "int" | "number" => "number",
                                    "boolean" | "bool" => "boolean",
                                    "array" => "array",
                                    "object" | "json" => "object",
                                    _ => "string",
                                };
                                
                                properties.insert(port_id.to_string(), serde_json::json!({
                                    "type": json_type,
                                    "description": port_name
                                }));
                                
                                if is_required {
                                    required.push(port_id.to_string());
                                }
                            }
                        }
                        
                        if !properties.is_empty() {
                            let mut schema = serde_json::json!({
                                "type": "object",
                                "properties": properties
                            });
                            if !required.is_empty() {
                                schema.as_object_mut().unwrap().insert("required".to_string(), serde_json::json!(required));
                            }
                            tracing::info!("Extracted workflow input schema from first node input_ports: {:?}", schema);
                            return schema;
                        }
                    }
                }
                
                // Fallback Method 2: Check params for schema definition (for first node)
                if let Some(params) = first_node.get("params").and_then(|p| p.as_object()) {
                    // Check if params has input_schema or schema field
                    if let Some(schema) = params.get("input_schema").or(params.get("schema")) {
                        tracing::info!("Found input_schema in first node params");
                        return schema.clone();
                    }
                    
                    // Convert params to schema (each param key becomes a property)
                    if !params.is_empty() {
                        let mut properties = serde_json::Map::new();
                        for (key, value) in params {
                            // Skip internal fields
                            if key.starts_with("_") || key == "type" || key == "node_type" {
                                continue;
                            }
                            
                            let param_type = if value.is_string() { "string" }
                                else if value.is_number() { "number" }
                                else if value.is_boolean() { "boolean" }
                                else if value.is_array() { "array" }
                                else { "object" };
                            
                            properties.insert(key.clone(), serde_json::json!({
                                "type": param_type,
                                "description": format!("Parameter: {}", key)
                            }));
                        }
                        
                        if !properties.is_empty() {
                            let schema = serde_json::json!({
                                "type": "object",
                                "properties": properties
                            });
                            tracing::info!("Extracted workflow input schema from first node params: {:?}", schema);
                            return schema;
                        }
                    }
                }

                // Fallback Method 3: Check data/inputs fields (for first node)
                if let Some(data) = first_node.get("data").or(first_node.get("inputs")) {
                    if let Some(schema) = data.get("input_schema") {
                        tracing::info!("Found input_schema in first node data");
                        return schema.clone();
                    }
                    if let Some(inputs) = data.get("inputs") {
                        if let Some(schema) = build_schema_from_params(inputs) {
                            tracing::info!("Built schema from first node data.inputs");
                            return schema;
                        }
                    }
                    if let Some(schema) = build_schema_from_params(data) {
                        tracing::info!("Built schema from first node data");
                        return schema;
                    }
                }

                // Fallback Method 4: Lookup tool definition in ToolServer
                // If the first node is a known tool, use its schema
                if let Some(server) = tool_server {
                    // Start node types like "start" are already handled, so if we are here,
                    // it might be a tool node. The 'node_type' might correspond to a tool ID.
                    // Or it might be stored in data.tool_id or similar.
                    
                    let mut candidate_tool_ids = Vec::new();

                    // 1. Direct node type (e.g. "http_request", "plugin__xyz")
                    if let Some(nt) = first_node.get("node_type").and_then(|t| t.as_str()) {
                         candidate_tool_ids.push(nt.to_string());
                    }
                     if let Some(nt) = first_node.get("type").and_then(|t| t.as_str()) {
                         candidate_tool_ids.push(nt.to_string());
                    }

                    // 2. data.tool_id
                    if let Some(data) = first_node.get("data").and_then(|d| d.as_object()) {
                        if let Some(tid) = data.get("tool_id").and_then(|t| t.as_str()) {
                            candidate_tool_ids.push(tid.to_string());
                        }
                    }

                    for tool_name in candidate_tool_ids {
                        // Skip internal node types
                        if tool_name == "start" || tool_name == "trigger" || tool_name == "note" {
                            continue;
                        }

                        if let Some(tool_info) = server.get_tool(&tool_name).await {
                             tracing::info!("Found schema for first node tool: {}", tool_name);
                             return tool_info.input_schema.clone();
                        }
                    }
                }
            }
        }

        // Default fallback
        tracing::warn!(
            "Could not extract input schema from workflow definition, using default. Available keys: {:?}",
            keys
        );
        serde_json::json!({
            "type": "object",
            "properties": {
                "inputs": {
                    "type": "object",
                    "description": "Workflow input parameters"
                }
            }
        })
    }

    /// Extract workflow tags for search/matching
    pub fn extract_tags(name: &str, description: &str) -> Vec<String> {
        let mut tags = Vec::new();

        // From name
        let name_lower = name.to_lowercase();
        let name_words: Vec<&str> = name_lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
            .collect();
        tags.extend(name_words.into_iter().map(String::from));

        // From description keywords
        let desc_lower = description.to_lowercase();
        let keywords = [
            "scan",
            "test",
            "analyze",
            "report",
            "monitor",
            "alert",
            "security",
            "vulnerability",
            "penetration",
            "reconnaissance",
            "network",
            "web",
            "api",
            "database",
            "file",
            "system",
        ];

        for keyword in keywords {
            if desc_lower.contains(keyword) {
                tags.push(keyword.to_string());
            }
        }

        // Deduplicate
        tags.sort();
        tags.dedup();

        tags
    }
}

/// Helper function to load workflows from database and register them
pub async fn load_workflows_from_db<F, Fut>(tool_server: &ToolServer, loader: F)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Vec<Value>, String>>,
{
    match loader().await {
        Ok(workflows) => {
            let mut count = 0;
            for workflow in workflows {
                // Check if workflow is marked as tool
                let is_tool = workflow
                    .get("is_tool")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if !is_tool {
                    continue;
                }

                let id = workflow.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let name = workflow
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let description = workflow
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Workflow tool");

                if id.is_empty() {
                    continue;
                }

                let input_schema = WorkflowToolAdapter::extract_input_schema(&workflow, Some(tool_server)).await;

                // Register workflow context
                let ctx = WorkflowContext {
                    workflow_id: id.to_string(),
                    name: name.to_string(),
                    definition: workflow.clone(),
                };
                register_workflow_context(ctx).await;

                // Register as tool
                let executor = create_workflow_executor(id.to_string());
                tool_server
                    .register_workflow_tool(id, name, description, input_schema, executor)
                    .await;

                count += 1;
            }

            tracing::info!("Loaded {} workflow tools from database", count);
        }
        Err(e) => {
            tracing::warn!("Failed to load workflows from database: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_context_registry() {
        let ctx = WorkflowContext {
            workflow_id: "test-workflow".to_string(),
            name: "Test Workflow".to_string(),
            definition: serde_json::json!({"nodes": []}),
        };

        register_workflow_context(ctx.clone()).await;

        let retrieved = get_workflow_context("test-workflow").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Workflow");

        unregister_workflow_context("test-workflow").await;
        assert!(get_workflow_context("test-workflow").await.is_none());
    }

    #[test]
    fn test_extract_tags() {
        let tags = WorkflowToolAdapter::extract_tags(
            "port_scanner_workflow",
            "Scan network ports for vulnerabilities",
        );

        assert!(tags.contains(&"port".to_string()));
        assert!(tags.contains(&"scan".to_string()));
        assert!(tags.contains(&"network".to_string()));
        assert!(tags.contains(&"vulnerability".to_string()));
    }

    #[tokio::test]
    async fn test_extract_input_schema() {
        let definition = serde_json::json!({
            "inputs": {
                "schema": {
                    "type": "object",
                    "properties": {
                        "target": {"type": "string"}
                    }
                }
            }
        });

        let schema = WorkflowToolAdapter::extract_input_schema(&definition, None).await;
        assert!(schema.get("properties").is_some());
    }
}

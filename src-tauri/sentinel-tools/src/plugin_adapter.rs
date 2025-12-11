//! Plugin Tool Adapter
//!
//! Adapts plugin tools to the unified tool system.
//! Supports loading tools from JavaScript/TypeScript plugins.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::dynamic_tool::{create_executor, DynamicToolDef, ToolExecutor, ToolSource};
use crate::tool_server::ToolServer;

/// Plugin tool metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginToolMeta {
    pub plugin_id: String,
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub code: Option<String>,
}

/// Plugin execution context
#[derive(Debug, Clone)]
pub struct PluginContext {
    pub plugin_id: String,
    pub name: String,
    pub code: String,
}

/// Global plugin registry
static PLUGIN_CONTEXTS: once_cell::sync::Lazy<RwLock<HashMap<String, PluginContext>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Register plugin context
pub async fn register_plugin_context(ctx: PluginContext) {
    let mut contexts = PLUGIN_CONTEXTS.write().await;
    contexts.insert(ctx.plugin_id.clone(), ctx);
}

/// Unregister plugin context
pub async fn unregister_plugin_context(plugin_id: &str) {
    let mut contexts = PLUGIN_CONTEXTS.write().await;
    contexts.remove(plugin_id);
}

/// Get plugin context
pub async fn get_plugin_context(plugin_id: &str) -> Option<PluginContext> {
    let contexts = PLUGIN_CONTEXTS.read().await;
    contexts.get(plugin_id).cloned()
}

/// Create plugin tool executor
fn create_plugin_executor(plugin_id: String) -> ToolExecutor {
    create_executor(move |args: Value| {
        let pid = plugin_id.clone();
        
        async move {
            let ctx = get_plugin_context(&pid)
                .await
                .ok_or_else(|| format!("Plugin '{}' not registered", pid))?;
            
            // Clone the context data for use in spawn_blocking
            let plugin_id = ctx.plugin_id.clone();
            let plugin_name = ctx.name.clone();
            let plugin_code = ctx.code.clone();
            let args_clone = args.clone();
            
            // Run plugin execution in a blocking thread since JsRuntime is not Send+Sync
            // We use spawn_blocking + LocalSet to handle non-Send futures
            let result = tokio::task::spawn_blocking(move || {
                // Create a new tokio runtime inside the blocking thread
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("Failed to create runtime: {}", e))?;
                
                // Use LocalSet to run non-Send futures
                let local = tokio::task::LocalSet::new();
                local.block_on(&rt, execute_plugin_async(
                    plugin_id,
                    plugin_name,
                    plugin_code,
                    args_clone,
                ))
            })
            .await
            .map_err(|e| format!("Plugin execution task failed: {}", e))??;
            
            Ok(result)
        }
    })
}

/// Execute plugin in a dedicated runtime (called from spawn_blocking)
async fn execute_plugin_async(
    plugin_id: String,
    plugin_name: String,
    plugin_code: String,
    args: Value,
) -> Result<Value, String> {
    tracing::info!(
        "Executing plugin: {} (id: {})",
        plugin_name, plugin_id
    );
    
    // Create a new PluginEngine instance
    let mut engine = sentinel_plugins::PluginEngine::new()
        .map_err(|e| format!("Failed to create plugin engine: {}", e))?;
    
    // Create plugin metadata for loading
    let metadata = sentinel_plugins::PluginMetadata {
        id: plugin_id.clone(),
        name: plugin_name.clone(),
        version: "1.0.0".to_string(),
        author: None,
        main_category: "agent".to_string(),
        category: "tool".to_string(),
        default_severity: sentinel_plugins::Severity::Medium,
        tags: vec![],
        description: Some(format!("Agent tool plugin: {}", plugin_name)),
    };
    
    // Load the plugin code with metadata
    engine.load_plugin_with_metadata(&plugin_code, metadata)
        .await
        .map_err(|e| format!("Failed to load plugin code: {}", e))?;
    
    tracing::debug!(
        "Plugin loaded successfully, executing agent function with args: {:?}",
        args
    );
    
    // Execute the plugin's agent function (analyze/run/execute)
    let (findings, last_result) = engine.execute_agent(&args)
        .await
        .map_err(|e| format!("Plugin execution failed: {}", e))?;
    
    tracing::info!(
        "Plugin {} executed successfully, findings: {}, has_result: {}",
        plugin_id,
        findings.len(),
        last_result.is_some()
    );
    
    // Build the result
    // If the plugin returned a result via op_plugin_return, use that
    // Otherwise, return the findings and execution status
    if let Some(result) = last_result {
        // Plugin explicitly returned a result
        Ok(result)
    } else if !findings.is_empty() {
        // Plugin emitted findings via Sentinel.emitFinding()
        let findings_json: Vec<Value> = findings.into_iter().map(|f| {
            serde_json::json!({
                "id": f.id,
                "vuln_type": f.vuln_type,
                "severity": format!("{:?}", f.severity).to_lowercase(),
                "confidence": format!("{:?}", f.confidence).to_lowercase(),
                "title": f.title,
                "description": f.description,
                "evidence": f.evidence,
                "location": f.location,
                "url": f.url,
                "method": f.method,
                "cwe": f.cwe,
                "owasp": f.owasp,
                "remediation": f.remediation,
            })
        }).collect();
        
        Ok(serde_json::json!({
            "plugin_id": plugin_id,
            "plugin_name": plugin_name,
            "status": "success",
            "findings": findings_json,
            "findings_count": findings_json.len()
        }))
    } else {
        // Plugin executed but didn't return anything or emit findings
        Ok(serde_json::json!({
            "plugin_id": plugin_id,
            "plugin_name": plugin_name,
            "status": "success",
            "message": "Plugin executed successfully with no explicit output"
        }))
    }
}

/// Load and register plugin tools to server
pub async fn load_plugin_tools_to_server(
    tool_server: &ToolServer,
    plugins: Vec<PluginToolMeta>,
) {
    tracing::info!("Loading {} plugin tools", plugins.len());
    
    for plugin_meta in plugins {
        let full_name = format!("plugin::{}", plugin_meta.plugin_id);
        
        // Register plugin context if code is available
        if let Some(code) = &plugin_meta.code {
            let ctx = PluginContext {
                plugin_id: plugin_meta.plugin_id.clone(),
                name: plugin_meta.name.clone(),
                code: code.clone(),
            };
            register_plugin_context(ctx).await;
        }
        
        let executor = create_plugin_executor(plugin_meta.plugin_id.clone());
        
        tool_server.register_plugin_tool(
            &plugin_meta.plugin_id,
            &plugin_meta.name,
            &plugin_meta.description,
            plugin_meta.input_schema,
            executor,
        ).await;
        
        tracing::debug!("Registered plugin tool: {}", full_name);
    }
}

/// Refresh plugin tools
pub async fn refresh_plugin_tools(tool_server: &ToolServer) {
    tool_server.clear_plugin_tools().await;
    // Plugin tools will be reloaded by the caller with fresh data from database
}

/// Plugin tool adapter
pub struct PluginToolAdapter;

impl PluginToolAdapter {
    /// Create a DynamicToolDef from plugin metadata
    pub fn create_tool_def(meta: &PluginToolMeta) -> DynamicToolDef {
        let plugin_id = meta.plugin_id.clone();
        let full_name = format!("plugin::{}", plugin_id);
        
        DynamicToolDef {
            name: full_name,
            description: meta.description.clone(),
            input_schema: meta.input_schema.clone(),
            source: ToolSource::Plugin { plugin_id: plugin_id.clone() },
            executor: create_plugin_executor(plugin_id),
        }
    }
    
    /// Parse TypeScript interface ToolInput to JSON Schema
    pub fn parse_tool_input_schema(code: &str) -> Value {
        use regex::Regex;
        
        tracing::info!("Parsing ToolInput schema from plugin code ({} chars)", code.len());
        
        // Find start of interface
        let start_pattern = Regex::new(r"interface\s+ToolInput\s*\{").unwrap();
        
        let start_index = if let Some(m) = start_pattern.find(code) {
            tracing::info!("Found interface ToolInput at position {}", m.start());
             m.end()
        } else {
            // Also try `type ToolInput = {`
            let type_pattern = Regex::new(r"type\s+ToolInput\s*=\s*\{").unwrap();
            if let Some(m) = type_pattern.find(code) {
                tracing::info!("Found type ToolInput at position {}", m.start());
                m.end()
            } else {
                // Default fallback schema
                tracing::warn!("Could not find ToolInput interface/type in plugin code, using default schema");
                return serde_json::json!({
                    "type": "object",
                    "properties": {
                        "input": {"type": "string", "description": "Tool input parameter"}
                    }
                });
            }
        };

        // Extract body by counting braces
        let mut brace_count = 1;
        let mut body_end = start_index;
        let bytes = code.as_bytes();
        
        for (i, &b) in bytes[start_index..].iter().enumerate() {
            if b == b'{' {
                brace_count += 1;
            } else if b == b'}' {
                brace_count -= 1;
                if brace_count == 0 {
                    body_end = start_index + i;
                    break;
                }
            }
        }
        
        let body = &code[start_index..body_end];
        
        // Parse fields from body
        // We'll process the body to extract fields and comments
        // Strategy: 
        // 1. Remove block comments /* */ and store them associated with lines? 
        //    Actually simpler: regex that captures doc comment before field.
        
        // Match: 
        // (optional doc comment) 
        // (field name) (optional ?) : (type) (semicolon or newline)
        let field_re = Regex::new(
            r"(?ms)(?:/\*\*([\s\S]*?)\*/\s*)?([a-zA-Z0-9_]+)(\?)?\s*:\s*([^;}\n]+)(?:;|\n|$)"
        ).unwrap();
        
        let mut properties = serde_json::Map::new();
        let mut required_fields = Vec::new();
        
        for cap in field_re.captures_iter(body) {
            let comment = cap.get(1).map(|m| {
                m.as_str()
                    .lines()
                    .map(|l| l.trim().trim_start_matches('*').trim())
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ")
            }).unwrap_or_default();
            
            let field_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let optional = cap.get(3).is_some();
            let type_str = cap.get(4).map(|m| m.as_str().trim()).unwrap_or("string");
            
            if field_name.is_empty() {
                continue;
            }
            
            // Skip if this field looks like a method or something else
            if type_str.contains("(") {
                continue;
            }

            let mut prop = Self::ts_type_to_json_schema(type_str);
            if !comment.is_empty() {
                prop.as_object_mut().map(|p| {
                    p.insert("description".to_string(), serde_json::json!(comment))
                });
            }
            
            properties.insert(field_name.to_string(), prop);
            
            if !optional {
                required_fields.push(field_name.to_string());
            }
        }
        
        if !properties.is_empty() {
            let field_names: Vec<&String> = properties.keys().collect();
            tracing::info!(
                "Successfully parsed {} fields from ToolInput: {:?}",
                properties.len(),
                field_names
            );
            serde_json::json!({
                "type": "object",
                "properties": properties,
                "required": required_fields
            })
        } else {
             // Fallback
             tracing::warn!("No fields parsed from ToolInput interface body, using default schema");
             serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string", "description": "Tool input parameter"}
                }
            })
        }
    }
    
    /// Convert TypeScript type to JSON Schema
    fn ts_type_to_json_schema(type_str: &str) -> Value {
        let type_str = type_str.trim();
        
        if type_str.contains("string[]") || type_str.contains("Array<string>") {
            serde_json::json!({
                "type": "array",
                "items": {"type": "string"}
            })
        } else if type_str == "string" {
            serde_json::json!({"type": "string"})
        } else if type_str == "number" {
            serde_json::json!({"type": "number"})
        } else if type_str == "boolean" {
            serde_json::json!({"type": "boolean"})
        } else if type_str.contains("object") || type_str.starts_with('{') {
            serde_json::json!({"type": "object"})
        } else {
            // Default to string for unknown types
             serde_json::json!({"type": "string"})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_context_registry() {
        let ctx = PluginContext {
            plugin_id: "test-plugin".to_string(),
            name: "Test Plugin".to_string(),
            code: "console.log('hello')".to_string(),
        };

        register_plugin_context(ctx.clone()).await;
        
        let retrieved = get_plugin_context("test-plugin").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Plugin");

        unregister_plugin_context("test-plugin").await;
        assert!(get_plugin_context("test-plugin").await.is_none());
    }

    #[test]
    fn test_parse_tool_input_schema() {
        let code = r#"
            interface ToolInput {
                /** Target URL to scan */
                url: string;
                /** Number of threads */
                threads?: number;
            }
        "#;
        
        let schema = PluginToolAdapter::parse_tool_input_schema(code);
        
        assert!(schema.get("properties").is_some());
        let props = schema.get("properties").unwrap();
        assert!(props.get("url").is_some());
        assert!(props.get("threads").is_some());
    }
}

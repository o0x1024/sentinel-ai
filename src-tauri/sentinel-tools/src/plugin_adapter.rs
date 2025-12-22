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
    
    // Create plugin metadata
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
    
    // Create a PluginExecutor with restart capability (1000 executions before restart warning)
    let executor = sentinel_plugins::PluginExecutor::new(metadata, plugin_code, 1000)
        .map_err(|e| format!("Failed to create plugin executor: {}", e))?;
    
    tracing::debug!(
        "Plugin loaded successfully, executing agent function with args: {:?}",
        args
    );
    
    // Execute the plugin's agent function (analyze/run/execute)
    let (findings, last_result) = executor.execute_agent(&args)
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
    
    /// Parse plugin code to extract input_schema.
    /// Priority:
    /// 1. Header block: `/* @sentinel_schema { ... } */`
    /// 2. Fallback: parse TS interface ToolInput
    pub fn parse_tool_input_schema(code: &str) -> Value {
        tracing::info!("Parsing input schema from plugin code ({} chars)", code.len());
        
        // 1. Try header schema block first
        if let Some(schema) = Self::parse_header_schema_block(code) {
            tracing::info!("Found @sentinel_schema header block, using it");
            return schema;
        }
        
        // 2. Fallback: parse TS interface ToolInput
        Self::parse_ts_interface_schema(code)
    }
    
    /// Parse header schema block: `/* @sentinel_schema { ... } */`
    fn parse_header_schema_block(code: &str) -> Option<Value> {
        use regex::Regex;
        
        // Match: /* @sentinel_schema { ... } */
        // The JSON content is between @sentinel_schema and the closing */
        let schema_re = Regex::new(
            r"(?s)/\*\s*@sentinel_schema\s*(\{[\s\S]*?\})\s*\*/"
        ).ok()?;
        
        let captures = schema_re.captures(code)?;
        let json_str = captures.get(1)?.as_str();
        
        // Parse JSON
        match serde_json::from_str::<Value>(json_str) {
            Ok(mut schema) => {
                // Ensure it has proper structure
                if schema.get("type").is_none() {
                    schema.as_object_mut().map(|obj| {
                        obj.insert("type".to_string(), serde_json::json!("object"));
                    });
                }
                if schema.get("properties").is_none() {
                    // If input_schema is nested, extract it
                    if let Some(input_schema) = schema.get("input_schema").cloned() {
                        return Some(input_schema);
                    }
                }
                Some(schema)
            }
            Err(e) => {
                tracing::warn!("Failed to parse @sentinel_schema JSON: {}", e);
                None
            }
        }
    }
    
    /// Fallback: Parse TypeScript interface ToolInput to JSON Schema
    fn parse_ts_interface_schema(code: &str) -> Value {
        use regex::Regex;
        
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
                tracing::warn!("Could not find ToolInput interface/type in plugin code, using default schema");
                return Self::default_schema();
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
        
        // Match fields with optional doc comments (/** */ or // style)
        let field_re = Regex::new(
            r"(?ms)(?:/\*\*([\s\S]*?)\*/\s*|(?://\s*(.+)\n)+)?([a-zA-Z0-9_]+)(\?)?\s*:\s*([^;}\n]+)(?:;|\n|$)"
        ).unwrap();
        
        let mut properties = serde_json::Map::new();
        let mut required_fields = Vec::new();
        
        for cap in field_re.captures_iter(body) {
            // Try block comment first, then line comments
            let comment = cap.get(1).map(|m| {
                m.as_str()
                    .lines()
                    .map(|l| l.trim().trim_start_matches('*').trim())
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ")
            }).or_else(|| cap.get(2).map(|m| m.as_str().trim().to_string()))
            .unwrap_or_default();
            
            let field_name = cap.get(3).map(|m| m.as_str()).unwrap_or("");
            let optional = cap.get(4).is_some();
            let type_str = cap.get(5).map(|m| m.as_str().trim()).unwrap_or("string");
            
            if field_name.is_empty() {
                continue;
            }
            
            // Skip if this field looks like a method
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
            tracing::warn!("No fields parsed from ToolInput interface body, using default schema");
            Self::default_schema()
        }
    }
    
    /// Default fallback schema
    fn default_schema() -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "input": {"type": "string", "description": "Tool input parameter"}
            }
        })
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
    fn test_parse_tool_input_schema_with_ts_interface() {
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
    
    #[test]
    fn test_parse_header_schema_block() {
        let code = r#"
/* @sentinel_schema
{
    "type": "object",
    "required": ["targets"],
    "properties": {
        "targets": { "type": "array", "items": { "type": "string" }, "description": "Target hosts or URLs" },
        "concurrency": { "type": "integer", "default": 10, "description": "Parallelism" }
    }
}
*/

interface ToolInput {
    targets: string[];
    concurrency?: number;
}

export async function analyze(input: ToolInput) {
    return { success: true };
}
globalThis.analyze = analyze;
        "#;
        
        let schema = PluginToolAdapter::parse_tool_input_schema(code);
        
        // Should use header block, not TS interface
        assert_eq!(schema.get("type").unwrap(), "object");
        let props = schema.get("properties").unwrap();
        
        // Check targets property
        let targets = props.get("targets").unwrap();
        assert_eq!(targets.get("type").unwrap(), "array");
        assert_eq!(targets.get("description").unwrap(), "Target hosts or URLs");
        
        // Check concurrency property
        let concurrency = props.get("concurrency").unwrap();
        assert_eq!(concurrency.get("type").unwrap(), "integer");
        assert_eq!(concurrency.get("default").unwrap(), 10);
        assert_eq!(concurrency.get("description").unwrap(), "Parallelism");
        
        // Check required
        let required = schema.get("required").unwrap().as_array().unwrap();
        assert!(required.iter().any(|v| v == "targets"));
    }
    
    #[test]
    fn test_fallback_to_ts_interface_when_no_header() {
        let code = r#"
interface ToolInput {
    /** Target URL */
    url: string;
}

export async function analyze(input: ToolInput) {
    return { success: true };
}
        "#;
        
        let schema = PluginToolAdapter::parse_tool_input_schema(code);
        
        let props = schema.get("properties").unwrap();
        let url = props.get("url").unwrap();
        assert_eq!(url.get("type").unwrap(), "string");
        assert_eq!(url.get("description").unwrap(), "Target URL");
    }
}

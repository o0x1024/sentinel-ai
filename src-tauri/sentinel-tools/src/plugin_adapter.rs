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
    
    /// 从插件代码获取 input_schema（仅通过运行时调用 get_input_schema）
    ///
    /// 加载插件后调用其导出的 `get_input_schema()` 函数获取 schema。
    /// 插件必须导出 get_input_schema() 函数，否则返回空 schema。
    ///
    /// # 参数
    /// - `code`: 插件代码
    /// - `metadata`: 插件元数据（用于初始化 PluginEngine）
    ///
    /// # 返回
    /// - 成功：返回插件定义的 JSON Schema
    /// - 失败：返回默认空 schema
    pub async fn get_input_schema_runtime(code: &str, metadata: sentinel_plugins::PluginMetadata) -> Value {
        tracing::info!("Getting input schema via runtime call for plugin: {}", metadata.id);
        
        match sentinel_plugins::get_input_schema_from_code(code, metadata).await {
            Ok(schema) => {
                tracing::info!("Successfully got input schema from plugin runtime");
                schema
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to get input schema from runtime: {}, plugin must export get_input_schema()",
                    e
                );
                // 返回默认空 schema
                Self::default_schema()
            }
        }
    }
    
    /// Default empty schema
    fn default_schema() -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
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
    fn test_default_schema() {
        let schema = PluginToolAdapter::default_schema();
        assert_eq!(schema.get("type").unwrap(), "object");
        assert!(schema.get("properties").is_some());
    }
}

//! Agent插件工具测试命令
//!
//! 提供Tauri命令来测试agent插件工具的集成

use crate::tools::{get_global_tool_system, ToolSystem};
use sentinel_tools::unified_types::{ToolExecutionParams, ToolSearchQuery};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginToolTestRequest {
    pub plugin_id: String,
    pub target: Option<String>,
    pub context: Option<Value>,
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginToolTestResponse {
    pub success: bool,
    pub tool_name: String,
    pub output: Value,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// 列出所有可用的插件工具
#[tauri::command]
pub async fn list_agent_plugin_tools() -> Result<Vec<String>, String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("Tool system not initialized: {}", e))?;
    
    let tools = tool_system.list_tools().await;
    
    // 过滤出插件工具
    let plugin_tools: Vec<String> = tools
        .into_iter()
        .filter(|t| t.name.starts_with("plugin::"))
        .map(|t| t.name)
        .collect();
    
    Ok(plugin_tools)
}

/// 搜索插件工具
#[tauri::command]
pub async fn search_agent_plugin_tools(query: String) -> Result<Vec<Value>, String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("Tool system not initialized: {}", e))?;
    
    let search_query = ToolSearchQuery {
        query,
        category: None,
        tags: vec!["agent".to_string(), "plugin".to_string()],
        available_only: true,
        installed_only: false,
    };
    
    let result = tool_system.search_tools(search_query).await;
    
    let tools: Vec<Value> = result.tools
        .into_iter()
        .filter(|t| t.name.starts_with("plugin::"))
        .map(|t| json!({
            "name": t.name,
            "description": t.description,
            "category": t.category.to_string(),
            "available": t.available,
        }))
        .collect();
    
    Ok(tools)
}

/// 测试执行插件工具
#[tauri::command]
pub async fn test_execute_plugin_tool(
    request: PluginToolTestRequest,
) -> Result<PluginToolTestResponse, String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("Tool system not initialized: {}", e))?;
    
    // 构建工具名称
    let tool_name = if request.plugin_id.starts_with("plugin::") {
        request.plugin_id.clone()
    } else {
        format!("plugin::{}", request.plugin_id)
    };
    
    // 构建执行参数
    let mut inputs = HashMap::new();
    
    if let Some(target) = request.target {
        inputs.insert("target".to_string(), json!(target));
    }
    
    if let Some(context) = request.context {
        inputs.insert("context".to_string(), context);
    }
    
    if let Some(data) = request.data {
        inputs.insert("data".to_string(), data);
    }
    
    let params = ToolExecutionParams {
        inputs,
        context: HashMap::new(),
        timeout: Some(std::time::Duration::from_secs(30)),
        execution_id: Some(uuid::Uuid::new_v4()),
    };
    
    // 执行工具
    let result = tool_system.execute_tool(&tool_name, params).await
        .map_err(|e| format!("Failed to execute tool: {}", e))?;
    
    Ok(PluginToolTestResponse {
        success: result.success,
        tool_name: result.tool_name,
        output: result.output,
        error: result.error,
        execution_time_ms: result.execution_time_ms,
    })
}

/// 获取插件工具的详细信息
#[tauri::command]
pub async fn get_plugin_tool_info(plugin_id: String) -> Result<Value, String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("Tool system not initialized: {}", e))?;
    
    let tool_name = if plugin_id.starts_with("plugin::") {
        plugin_id
    } else {
        format!("plugin::{}", plugin_id)
    };
    
    let tools = tool_system.list_tools().await;
    
    let tool = tools
        .into_iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| format!("Plugin tool not found: {}", tool_name))?;
    
    Ok(json!({
        "name": tool.name,
        "description": tool.description,
        "category": tool.category.to_string(),
        "available": tool.available,
        "parameters": tool.parameters.parameters.iter().map(|p| json!({
            "name": p.name,
            "description": p.description,
            "type": format!("{:?}", p.param_type),
            "required": p.required,
            "default": p.default_value,
        })).collect::<Vec<_>>(),
        "metadata": {
            "author": tool.metadata.author,
            "version": tool.metadata.version,
            "tags": tool.metadata.tags,
        }
    }))
}

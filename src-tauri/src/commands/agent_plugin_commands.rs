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

#[tauri::command(rename_all = "snake_case")]
pub async fn test_agent_plugin_advanced(
    plugin_id: String,
    inputs: Option<Value>,
    runs: Option<u32>,
    concurrency: Option<u32>,
) -> Result<crate::commands::passive_scan_commands::CommandResponse<crate::commands::passive_scan_commands::AdvancedTestResult>, String> {
    let runs = runs.unwrap_or(1).max(1);
    let concurrency = concurrency.unwrap_or(1).max(1);

    let tool_system = get_global_tool_system()
        .map_err(|e| format!("Tool system not initialized: {}", e))?;

    if let Err(e) = tool_system.refresh_all().await {
        tracing::warn!("Failed to refresh tool system: {}", e);
    }

    let tool_name = if plugin_id.starts_with("plugin::") {
        plugin_id.clone()
    } else {
        format!("plugin::{}", plugin_id)
    };

    let base_inputs_map: HashMap<String, Value> = match inputs {
        Some(Value::Object(map)) => map.into_iter().collect(),
        _ => {
            let mut m = HashMap::new();
            m.insert("target".to_string(), json!("https://example.com/test"));
            m.insert("context".to_string(), json!({"test_mode": true}));
            m.insert("data".to_string(), json!({}));
            m
        }
    };

    use futures::{stream, StreamExt};
    let mut indices: Vec<u32> = (0..runs).collect();
    let start_all = std::time::Instant::now();
    let mut run_stats: Vec<crate::commands::passive_scan_commands::AdvancedRunStat> = Vec::with_capacity(runs as usize);
    let mut all_findings: Vec<crate::commands::passive_scan_commands::TestFinding> = Vec::new();
    let mut all_outputs: Vec<serde_json::Value> = Vec::new();

    let results = stream::iter(indices.into_iter())
        .map(|i| {
            let tool_system = tool_system.clone();
            let tool_name = tool_name.clone();
            let inputs_map = base_inputs_map.clone();
            async move {
                let run_start = std::time::Instant::now();
                let params = ToolExecutionParams {
                    inputs: inputs_map,
                    context: HashMap::new(),
                    timeout: Some(std::time::Duration::from_secs(30)),
                    execution_id: Some(uuid::Uuid::new_v4()),
                };
                match tool_system.execute_tool(&tool_name, params).await {
                    Ok(result) => {
                        let dur = run_start.elapsed().as_millis();
                        let findings_vec: Vec<crate::commands::passive_scan_commands::TestFinding> = result
                            .output
                            .get("findings")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .map(|it| crate::commands::passive_scan_commands::TestFinding {
                                        title: it.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                        description: it.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                        severity: it.get("severity").and_then(|v| v.as_str()).unwrap_or("info").to_string(),
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        let output_val = result.output.get("result").cloned().unwrap_or_else(|| result.output.clone());
                        (i, Ok((dur, findings_vec, output_val)))
                    }
                    Err(e) => {
                        let dur = run_start.elapsed().as_millis();
                        (i, Err((dur, e.to_string())))
                    }
                }
            }
        })
        .buffer_unordered(concurrency as usize)
        .collect::<Vec<(u32, Result<(u128, Vec<crate::commands::passive_scan_commands::TestFinding>, serde_json::Value), (u128, String)>)>>()
        .await;

    for (idx, res) in results.into_iter() {
        match res {
            Ok((dur, findings, output_val)) => {
                run_stats.push(crate::commands::passive_scan_commands::AdvancedRunStat { run_index: idx, duration_ms: dur, findings: findings.len(), error: None });
                all_findings.extend(findings);
                all_outputs.push(output_val);
            }
            Err((dur, err)) => {
                run_stats.push(crate::commands::passive_scan_commands::AdvancedRunStat { run_index: idx, duration_ms: dur, findings: 0, error: Some(err) });
            }
        }
    }

    let total_duration_ms = start_all.elapsed().as_millis();
    let avg_duration_ms = if run_stats.is_empty() {
        0.0
    } else {
        (run_stats.iter().map(|r| r.duration_ms).sum::<u128>() as f64) / (run_stats.len() as f64)
    };

    use std::collections::HashSet;
    let mut uniq = HashSet::new();
    let mut unique_list: Vec<crate::commands::passive_scan_commands::TestFinding> = Vec::new();
    for f in &all_findings {
        let key = format!("{}|{}|{}", f.title, f.severity, f.description);
        if uniq.insert(key) {
            unique_list.push(f.clone());
        }
    }

    Ok(crate::commands::passive_scan_commands::CommandResponse::ok(
        crate::commands::passive_scan_commands::AdvancedTestResult {
            plugin_id,
            success: run_stats.iter().all(|r| r.error.is_none()),
            total_runs: runs,
            concurrency,
            total_duration_ms,
            avg_duration_ms,
            total_findings: all_findings.len(),
            unique_findings: unique_list.len(),
            findings: unique_list,
            runs: run_stats,
            message: Some("高级测试完成".to_string()),
            error: None,
            outputs: Some(all_outputs),
        },
    ))
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
        "schema": tool.parameters.schema,
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

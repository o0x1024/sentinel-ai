use crate::tools::{get_global_tool_system, ToolExecutionParams, ToolSearchQuery, BatchExecutionRequest, BatchExecutionMode, ToolExecutionRequest, ToolInfo, ToolCategory};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;



#[derive(Debug, Serialize, Deserialize)]
pub struct ToolExecutionResponse {
    pub success: bool,
    pub output: Value,
    pub execution_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolSearchResponse {
    pub tools: Vec<ToolInfo>,
    pub total_count: usize,
    pub query: String,
}

/// 获取所有可用工具列表
#[tauri::command]
pub async fn unified_list_tools() -> Result<Vec<ToolInfo>, String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("获取工具系统失败: {}", e))?;
    
    let tools = tool_system.list_tools().await;
    
    let tool_infos = tools.into_iter().map(|tool| ToolInfo {
        id: tool.id,
        name: tool.name,
        description: tool.description,
        version: tool.version,
        category: tool.category,
        parameters: tool.parameters,
        metadata: tool.metadata,
        available: tool.available,
        installed: tool.installed,
        source: tool.source,
    }).collect();
    
    Ok(tool_infos)
}

/// 搜索工具
#[tauri::command]
pub async fn unified_search_tools(
    query: String,
    category: Option<String>,
    tags: Option<Vec<String>>,
    available_only: Option<bool>,
) -> Result<ToolSearchResponse, String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("获取工具系统失败: {}", e))?;
    
    let search_query = ToolSearchQuery {
        query: query.clone(),
        category: category.map(|c| {
            match c.as_str() {
                "network_scanning" => ToolCategory::NetworkScanning,
                "vulnerability_scanning" => ToolCategory::VulnerabilityScanning,
                "service_detection" => ToolCategory::ServiceDetection,
                "code_analysis" => ToolCategory::CodeAnalysis,
                "data_processing" => ToolCategory::DataProcessing,
                "system_utility" => ToolCategory::SystemUtility,
                other => ToolCategory::Custom(other.to_string()),
            }
        }),
        tags: tags.unwrap_or_default(),
        available_only: available_only.unwrap_or(false),
        installed_only: false,
    };
    
    let search_result = tool_system.search_tools(search_query).await;
    
    let tool_infos = search_result.tools.into_iter().map(|tool| ToolInfo {
        id: tool.id,
        name: tool.name,
        description: tool.description,
        version: tool.version,
        category: tool.category,
        parameters: tool.parameters,
        metadata: tool.metadata,
        available: tool.available,
        installed: tool.installed,
        source: tool.source,
    }).collect();
    
    Ok(ToolSearchResponse {
        tools: tool_infos,
        total_count: search_result.total_count,
        query,
    })
}

/// 执行单个工具
#[tauri::command]
pub async fn unified_execute_tool(
    tool_name: String,
    inputs: HashMap<String, Value>,
    context: Option<HashMap<String, Value>>,
    timeout: Option<u64>,
) -> Result<ToolExecutionResponse, String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("获取工具系统失败: {}", e))?;
    
    let execution_id = Uuid::new_v4().to_string();
    
    let params = ToolExecutionParams {
        inputs,
        context: context.unwrap_or_default(),
        timeout: timeout.map(Duration::from_secs),
        execution_id: Some(Uuid::new_v4()),
    };
    
    match tool_system.execute_tool(&tool_name, params).await {
        Ok(result) => Ok(ToolExecutionResponse {
            success: true,
            output: result.output,
            execution_id: Some(execution_id),
            error: None,
        }),
        Err(e) => Ok(ToolExecutionResponse {
            success: false,
            output: Value::Null,
            execution_id: Some(execution_id),
            error: Some(e.to_string()),
        }),
    }
}

/// 批量执行工具
#[tauri::command]
pub async fn unified_execute_batch_tools(
    requests: Vec<HashMap<String, Value>>,
    mode: String,
    stop_on_error: Option<bool>,
) -> Result<Value, String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("获取工具系统失败: {}", e))?;
    
    let execution_mode = match mode.as_str() {
        "parallel" => BatchExecutionMode::Parallel,
        "sequential" => BatchExecutionMode::Sequential,
        "pipeline" => BatchExecutionMode::Pipeline,
        _ => return Err("无效的执行模式，支持: parallel, sequential, pipeline".to_string()),
    };
    
    let mut tool_requests = Vec::new();
    
    for req in requests {
        let tool_name = req.get("tool_name")
            .and_then(|v| v.as_str())
            .ok_or("缺少tool_name字段")?;
        
        let inputs = req.get("inputs")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        
        let context = req.get("context")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        
        let timeout = req.get("timeout")
            .and_then(|v| v.as_u64());
        
        tool_requests.push(ToolExecutionRequest {
            tool_name: tool_name.to_string(),
            params: ToolExecutionParams {
                inputs,
                context,
                timeout: timeout.map(Duration::from_secs),
                execution_id: Some(Uuid::new_v4()),
            },
            priority: None,
        });
    }
    
    let batch_request = BatchExecutionRequest {
        requests: tool_requests,
        mode: execution_mode,
        stop_on_error: stop_on_error.unwrap_or(false),
    };
    
    match tool_system.execute_batch(batch_request).await {
        Ok(result) => {
            let response = serde_json::json!({
                "success": true,
                "results": result.results.into_iter().map(|r| {
                    serde_json::json!({
                        "tool_name": r.tool_name,
                        "success": r.success,
                        "output": r.output,
                        "error": r.error,
                        "execution_time": r.execution_time_ms,
                        "execution_id": r.execution_id
                    })
                }).collect::<Vec<_>>(),
                "total_time": result.total_execution_time_ms,
                "successful_count": result.success_count,
                "failed_count": result.failure_count
            });
            Ok(response)
        },
        Err(e) => Err(format!("批量执行失败: {}", e)),
    }
}

/// 获取工具信息
#[tauri::command]
pub async fn unified_get_tool_info(tool_name: String) -> Result<Option<ToolInfo>, String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("获取工具系统失败: {}", e))?;
    
    let tools = tool_system.list_tools().await;
    
    let tool_info = tools.into_iter()
        .find(|tool| tool.name == tool_name)
        .map(|tool| ToolInfo {
            id: tool.id,
            name: tool.name,
            description: tool.description,
            version: tool.version,
            category: tool.category,
            parameters: tool.parameters,
            metadata: tool.metadata,
            available: tool.available,
            installed: tool.installed,
            source: tool.source,
        });
    
    Ok(tool_info)
}

/// 获取执行历史
#[tauri::command]
pub async fn unified_get_execution_history(
    limit: Option<usize>,
    tool_name: Option<String>,
) -> Result<Value, String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("获取工具系统失败: {}", e))?;
    
    let history = tool_system.get_execution_history(limit).await;
    
    let mut filtered_history = history;
    
    // 按工具名称过滤
    if let Some(name) = tool_name {
        filtered_history.retain(|record| record.tool_name == name);
    }
    
    // 限制返回数量
    if let Some(limit) = limit {
        filtered_history.truncate(limit);
    }
    
    Ok(serde_json::to_value(filtered_history)
        .map_err(|e| format!("序列化执行历史失败: {}", e))?)
}

/// 获取工具统计信息
#[tauri::command]
pub async fn unified_get_tool_statistics() -> Result<Value, String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("获取工具系统失败: {}", e))?;
    
    let stats = tool_system.get_tool_statistics().await;
    
    Ok(serde_json::to_value(stats)
        .map_err(|e| format!("序列化工具统计失败: {}", e))?)
}

/// 刷新所有工具提供者
#[tauri::command]
pub async fn unified_refresh_all_tools() -> Result<(), String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("获取工具系统失败: {}", e))?;
    
    tool_system.refresh_all().await
        .map_err(|e| format!("刷新工具失败: {}", e))
}

/// 清除执行历史
#[tauri::command]
pub async fn unified_clear_execution_history() -> Result<(), String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("获取工具系统失败: {}", e))?;
    
    tool_system.clear_history().await;
    Ok(())
}

/// 获取工具分类列表
#[tauri::command]
pub async fn unified_get_tool_categories() -> Result<Vec<String>, String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("获取工具系统失败: {}", e))?;
    
    let tools = tool_system.list_tools().await;
    
    let mut categories: Vec<String> = tools.into_iter()
        .map(|tool| tool.category.to_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    
    categories.sort();
    Ok(categories)
}

/// 检查工具是否可用
#[tauri::command]
pub async fn unified_is_tool_available(tool_name: String) -> Result<bool, String> {
    let tool_system = get_global_tool_system()
        .map_err(|e| format!("获取工具系统失败: {}", e))?;
    
    let tools = tool_system.list_tools().await;
    
    let is_available = tools.into_iter()
        .find(|tool| tool.name == tool_name)
        .map(|tool| tool.available)
        .unwrap_or(false);
    
    Ok(is_available)
}
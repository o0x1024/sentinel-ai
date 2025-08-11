use crate::tools::{ToolSystem, ToolInfo, ToolExecutionParams};
use std::collections::HashMap as StdHashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct StartScanRequest {
    pub tool_name: String,
    pub target: String,
    pub timeout: Option<u64>,
    pub threads: Option<usize>,
    pub options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
}

#[tauri::command]
pub async fn list_scan_tools(tool_system: State<'_, ToolSystem>) -> Result<ScanResponse, String> {
    let tools = tool_system.list_tools().await;
    let tool_list: Vec<SimpleToolInfo> = tools
        .iter()
        .map(|tool| {
            SimpleToolInfo {
                id: tool.name.clone(),
                name: tool.name.clone(),
                description: tool.description.clone(),
                category: format!("{:?}", tool.category),
            }
        })
        .collect();

    Ok(ScanResponse {
        success: true,
        message: "工具列表获取成功".to_string(),
        data: Some(serde_json::to_value(tool_list).unwrap()),
    })
}

#[tauri::command]
pub async fn start_scan(
    request: StartScanRequest,
    tool_system: State<'_, ToolSystem>,
) -> Result<ScanResponse, String> {
    let mut inputs = StdHashMap::new();
    inputs.insert("target".to_string(), serde_json::json!(request.target));
    inputs.insert("timeout".to_string(), serde_json::json!(request.timeout.unwrap_or(5)));
    inputs.insert("threads".to_string(), serde_json::json!(request.threads.unwrap_or(50)));
    
    // 添加其他选项
    if let Some(options) = request.options {
        for (key, value) in options {
            inputs.insert(key, value);
        }
    }
    
    let params = ToolExecutionParams {
        inputs,
        context: StdHashMap::new(),
        timeout: None,
        execution_id: None,
    };

    match tool_system.execute_tool(&request.tool_name, params).await {
        Ok(result) => Ok(ScanResponse {
            success: true,
            message: "扫描任务启动成功".to_string(),
            data: Some(result.output),
        }),
        Err(e) => Ok(ScanResponse {
            success: false,
            message: format!("启动扫描失败: {}", e),
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn get_scan_result(
    scan_id: String,
    tool_system: State<'_, ToolSystem>,
) -> Result<ScanResponse, String> {
    let _uuid = Uuid::parse_str(&scan_id).map_err(|e| format!("无效的扫描ID: {}", e))?;

    // 注意：新的ToolSystem使用不同的历史记录机制
    let history = tool_system.get_execution_history(Some(100)).await;
    if let Some(record) = history.iter().find(|r| r.execution_id.to_string() == scan_id) {
        Ok(ScanResponse {
            success: true,
            message: "扫描结果获取成功".to_string(),
            data: Some(serde_json::to_value(&record.result).unwrap()),
        })
    } else {
        Err("扫描结果不存在".to_string())
    }
}

#[tauri::command]
pub async fn cancel_scan(
    scan_id: String,
    tool_system: State<'_, ToolSystem>,
) -> Result<ScanResponse, String> {
    let _uuid = Uuid::parse_str(&scan_id).map_err(|e| format!("无效的扫描ID: {}", e))?;

    // 注意：新的ToolSystem不支持取消操作，返回错误
    Ok(ScanResponse {
        success: false,
        message: "当前版本不支持取消扫描操作".to_string(),
        data: None,
    })
}

#[tauri::command]
pub async fn list_running_scans(
    tool_system: State<'_, ToolSystem>,
) -> Result<ScanResponse, String> {
    let history = tool_system.get_execution_history(Some(100)).await;
    let running_scans: Vec<_> = history.iter().filter(|r| {
        if let Some(ref result) = r.result {
            result.success
        } else {
            false
        }
    }).collect();
    Ok(ScanResponse {
        success: true,
        message: "获取运行中的扫描任务成功".to_string(),
        data: Some(serde_json::to_value(running_scans).map_err(|e| e.to_string())?),
    })
}

#[tauri::command]
pub async fn get_available_scan_tools(tool_system: State<'_, ToolSystem>) -> Result<Vec<SimpleToolInfo>, String> {
    let tools = tool_system.list_tools().await;
    let tool_list: Vec<SimpleToolInfo> = tools
        .iter()
        .map(|tool| {
            SimpleToolInfo {
                id: tool.name.clone(),
                name: tool.name.clone(),
                description: tool.description.clone(),
                category: format!("{:?}", tool.category),
            }
        })
        .collect();

    Ok(tool_list)
}

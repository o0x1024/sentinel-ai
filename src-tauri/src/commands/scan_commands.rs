use crate::mcp::ToolManager;
use crate::tools::ScanConfig;
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
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
}

#[tauri::command]
pub async fn list_scan_tools(tool_manager: State<'_, ToolManager>) -> Result<ScanResponse, String> {
    let tools = tool_manager.get_tools().await;
    let tool_list: Vec<ToolInfo> = tools
        .iter()
        .map(|tool| {
            let def = tool.definition();
            ToolInfo {
                id: def.name.clone(),
                name: def.name.clone(),
                description: def.description.clone(),
                category: format!("{:?}", def.category),
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
    tool_manager: State<'_, ToolManager>,
) -> Result<ScanResponse, String> {
    let config = ScanConfig {
        target: request.target,
        timeout: request.timeout.unwrap_or(5),
        threads: request.threads.unwrap_or(50),
        options: request.options.unwrap_or_default(),
    };

    // 创建工具执行请求
    let execution_request = crate::mcp::types::ToolExecutionRequest {
        tool_id: request.tool_name.clone(),
        parameters: std::collections::HashMap::new(),
        timeout: Some(300), // 5分钟超时
        priority: crate::mcp::types::ExecutionPriority::Normal,
    };

    match tool_manager.execute_tool(execution_request).await {
        Ok(scan_id) => Ok(ScanResponse {
            success: true,
            message: "扫描任务启动成功".to_string(),
            data: Some(serde_json::to_value(scan_id).map_err(|e| e.to_string())?),
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
    tool_manager: State<'_, ToolManager>,
) -> Result<ScanResponse, String> {
    let uuid = Uuid::parse_str(&scan_id).map_err(|e| format!("无效的扫描ID: {}", e))?;

    match tool_manager.get_execution_result(uuid).await {
        Some(result) => Ok(ScanResponse {
            success: true,
            message: "扫描结果获取成功".to_string(),
            data: Some(serde_json::to_value(result).unwrap()),
        }),
        None => Err("扫描结果不存在".to_string()),
    }
}

#[tauri::command]
pub async fn cancel_scan(
    scan_id: String,
    tool_manager: State<'_, ToolManager>,
) -> Result<ScanResponse, String> {
    let uuid = Uuid::parse_str(&scan_id).map_err(|e| format!("无效的扫描ID: {}", e))?;

    match tool_manager.cancel_execution(uuid).await {
        Ok(_) => Ok(ScanResponse {
            success: true,
            message: "扫描任务取消成功".to_string(),
            data: None,
        }),
        Err(e) => Ok(ScanResponse {
            success: false,
            message: format!("取消扫描失败: {}", e),
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn list_running_scans(
    tool_manager: State<'_, ToolManager>,
) -> Result<ScanResponse, String> {
    let running_scans = tool_manager.get_execution_history().await;
    let running_scans: Vec<_> = running_scans
        .into_iter()
        .filter(|result| matches!(result.status, crate::mcp::types::ExecutionStatus::Running))
        .collect();
    Ok(ScanResponse {
        success: true,
        message: "获取运行中的扫描任务成功".to_string(),
        data: Some(serde_json::to_value(running_scans).map_err(|e| e.to_string())?),
    })
}

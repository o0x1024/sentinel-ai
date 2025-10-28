//! MCP工具提供者
//! 
//! 从MCP服务器获取工具并包装为统一工具接口

use super::*;
use crate::services::mcp::McpService;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use chrono::Utc;
use uuid::Uuid;

// ============================================================================
// MCP工具提供者
// ============================================================================

#[derive(Debug)]
pub struct McpToolProvider {
    mcp_service: Arc<McpService>,
    tools: Arc<tokio::sync::RwLock<Vec<Arc<dyn UnifiedTool>>>>,
}

impl McpToolProvider {
    pub fn new(mcp_service: Arc<McpService>) -> Self {
        Self {
            mcp_service,
            tools: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    /// 刷新MCP工具列表
    async fn refresh_tools(&self) -> Result<()> {
        debug!("Refreshing MCP tools from service");
        
        // 从MCP服务获取工具列表
        let tool_infos = self.mcp_service.get_available_tools().await?;
        
        // 转换为统一工具接口
        let mut unified_tools = Vec::new();
        for tool_info in tool_infos {
            let unified_tool = Arc::new(McpToolWrapper::new(
                tool_info,
                self.mcp_service.clone(),
            ));
            unified_tools.push(unified_tool as Arc<dyn UnifiedTool>);
        }
        
        // 更新工具列表
        let mut tools_guard = self.tools.write().await;
        *tools_guard = unified_tools;
        
        info!("Refreshed {} MCP tools", tools_guard.len());
        Ok(())
    }
}

#[async_trait]
impl ToolProvider for McpToolProvider {
    fn name(&self) -> &str {
        "mcp"
    }

    fn description(&self) -> &str {
        "MCP (Model Context Protocol) tools from connected servers"
    }

    async fn get_tools(&self) -> Result<Vec<Arc<dyn UnifiedTool>>> {
        // 先刷新工具列表
        if let Err(e) = self.refresh_tools().await {
            warn!("Failed to refresh MCP tools: {}", e);
        }
        
        let tools_guard = self.tools.read().await;
        Ok(tools_guard.clone())
    }

    async fn get_tool(&self, name: &str) -> Result<Option<Arc<dyn UnifiedTool>>> {
        let tools_guard = self.tools.read().await;
        for tool in tools_guard.iter() {
            if tool.name() == name {
                return Ok(Some(tool.clone()));
            }
        }
        Ok(None)
    }

    async fn refresh(&self) -> Result<()> {
        self.refresh_tools().await
    }

    async fn is_available(&self) -> bool {
        // 检查MCP客户端是否有连接
        match self.mcp_service.get_connection_info().await {
            Ok(connections) => connections.iter().any(|c| c.status == "connected"),
            Err(_) => false,
        }
    }
}

// ============================================================================
// MCP工具包装器 - 将MCP工具适配为统一工具接口
// ============================================================================

#[derive(Debug)]
pub struct McpToolWrapper {
    tool_info: ToolInfo,
    mcp_service: Arc<McpService>,
    connection_name: Option<String>, // 记录工具来源的连接名
}

impl McpToolWrapper {
    pub fn new(tool_info: ToolInfo, mcp_service: Arc<McpService>) -> Self {
        // 从工具metadata中提取连接名
        let connection_name = tool_info.metadata.tags.iter()
            .find(|tag| tag.starts_with("connection:"))
            .map(|tag| tag.strip_prefix("connection:").unwrap().to_string());
            
        Self {
            tool_info,
            mcp_service,
            connection_name,
        }
    }

    pub fn new_with_connection(tool_info: ToolInfo, mcp_service: Arc<McpService>, connection_name: String) -> Self {
        Self {
            tool_info,
            mcp_service,
            connection_name: Some(connection_name),
        }
    }
}

#[async_trait]
impl UnifiedTool for McpToolWrapper {
    fn name(&self) -> &str {
        &self.tool_info.name
    }

    fn description(&self) -> &str {
        &self.tool_info.description
    }

    fn category(&self) -> ToolCategory {
        self.tool_info.category.clone()
    }

    fn parameters(&self) -> &ToolParameters {
        &self.tool_info.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.tool_info.metadata
    }

    async fn is_available(&self) -> bool {
        // 检查工具是否可用且MCP客户端有连接
        if !self.tool_info.available {
            return false;
        }
        
        match self.mcp_service.get_connection_info().await {
            Ok(connections) => connections.iter().any(|c| c.status == "connected"),
            Err(_) => false,
        }
    }

    async fn is_installed(&self) -> bool {
        self.tool_info.installed
    }

    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        // 验证参数
        self.validate_params(&params)?;
        
        info!("Executing MCP tool '{}' with params: {:?}", self.name(), params.inputs);
        
        // 将参数转换为serde_json::Value
        let mut json_params = serde_json::Map::new();
        for (key, value) in params.inputs {
            json_params.insert(key, value);
        }
        let params_value = Value::Object(json_params);
        
        // 生成执行ID和时间戳
        let execution_id = Uuid::new_v4();
        let started_at = Utc::now();

        // 智能解析工具来源和执行
        let tool_name = self.name();
        let execution_result = if let Some(connection_name) = &self.connection_name {
            // 如果明确知道连接名，直接使用
            info!("Executing MCP client tool '{}' from known connection '{}'", tool_name, connection_name);
            self.mcp_service.execute_client_tool(connection_name, tool_name, params_value).await
        } else if let Some((connection_name, actual_tool_name)) = Self::split_connection_and_tool(tool_name) {
            // 尝试传统的名称分割解析
            info!("Executing MCP client tool '{}' from parsed connection '{}'", actual_tool_name, connection_name);
            self.mcp_service.execute_client_tool(connection_name, actual_tool_name, params_value).await
        } else {
            // 智能查找：在所有连接中寻找提供此工具的连接
            match self.find_tool_connection(tool_name).await {
                Some(connection_name) => {
                    info!("Executing MCP client tool '{}' from discovered connection '{}'", tool_name, connection_name);
                    self.mcp_service.execute_client_tool(&connection_name, tool_name, params_value).await
                }
                None => {
                    // 通过MCP服务执行工具（内置工具）
                    info!("Executing tool '{}' as built-in tool", tool_name);
                    self.mcp_service.execute_tool(tool_name, params_value).await
                }
            }
        };
        // info!("execution_result: {:?}", execution_result);
        // 处理执行结果
        match execution_result {
            Ok(result) => {
                let completed_at = Utc::now();
                let execution_time_ms = (completed_at - started_at).num_milliseconds() as u64;

                info!("MCP tool '{}' executed successfully", self.name());
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_name: self.name().to_string(),
                    tool_id: self.tool_info.id.clone(),
                    success: true,
                    output: result,
                    error: None,
                    execution_time_ms,
                    metadata: HashMap::new(),
                    started_at,
                    completed_at: Some(completed_at),
                    status: ExecutionStatus::Completed,
                })
            }
            Err(e) => {
                let completed_at = Utc::now();
                let execution_time_ms = (completed_at - started_at).num_milliseconds() as u64;

                error!("MCP tool '{}' execution failed: {}", self.name(), e);
                Ok(ToolExecutionResult {
                    execution_id,
                    tool_name: self.name().to_string(),
                    tool_id: self.tool_info.id.clone(),
                    success: false,
                    output: Value::Null,
                    error: Some(e.to_string()),
                    execution_time_ms,
                    metadata: HashMap::new(),
                    started_at,
                    completed_at: Some(completed_at),
                    status: ExecutionStatus::Failed,
                })
            }
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 创建MCP工具提供者（如果MCP客户端有连接）
pub async fn create_mcp_tool_provider(mcp_service: Arc<McpService>) -> Result<Option<Box<dyn ToolProvider>>> {
    // 检查MCP客户端是否有连接的服务器
    let connections = mcp_service.get_connection_info().await.unwrap_or_default();
    let connected_count = connections.iter().filter(|c| c.status == "connected").count();
    
    if connected_count == 0 {
        warn!("No MCP client connections available, skipping MCP tool provider creation");
        return Ok(None);
    }
    
    info!("Found {} connected MCP servers, creating MCP tool provider", connected_count);
    let provider = Box::new(McpToolProvider::new(mcp_service));
    
    info!("Created MCP tool provider successfully");
    Ok(Some(provider))
}

impl McpToolWrapper {
    fn split_connection_and_tool(name: &str) -> Option<(&str, &str)> {
        let mut iter = name.splitn(2, '_');
        let conn = iter.next()?;
        let tool = iter.next()?;
        if conn.is_empty() || tool.is_empty() { return None; }
        Some((conn, tool))
    }

    /// 智能查找提供指定工具的MCP连接
    async fn find_tool_connection(&self, tool_name: &str) -> Option<String> {
        // 获取所有MCP连接信息
        if let Ok(connections) = self.mcp_service.get_connection_info().await {
            for connection in connections {
                if connection.status == "connected" {
                    // 检查此连接是否提供目标工具
                    if let Some(session) = self.mcp_service.get_client_manager().get_session(&connection.name).await {
                        if let Ok(tools_result) = session.list_tools_paginated(None).await {
                            // 检查工具列表中是否包含目标工具
                            for tool in tools_result.tools {
                                if tool.name == tool_name {
                                    info!("Found tool '{}' in connection '{}'", tool_name, connection.name);
                                    return Some(connection.name);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        warn!("Tool '{}' not found in any connected MCP server", tool_name);
        None
    }
}

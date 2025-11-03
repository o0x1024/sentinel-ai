//! ToolManager trait implementation wrapping existing tool system

use sentinel_engines::{ToolManager, ToolExecutionResult as EngineToolResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use std::collections::HashMap;
use crate::tools::{ToolSystem, ToolExecutionParams};

/// Tool manager implementation wrapping the global tool system
pub struct ToolManagerImpl {
    tool_system: Arc<ToolSystem>,
}

impl ToolManagerImpl {
    pub fn new(tool_system: Arc<ToolSystem>) -> Self {
        Self { tool_system }
    }
}

#[async_trait]
impl ToolManager for ToolManagerImpl {
    async fn execute_tool(&self, tool_name: &str, params: Value) -> Result<EngineToolResult> {
        // Convert Value to ToolExecutionParams expected by tool system
        let inputs = if let Value::Object(map) = params {
            map.into_iter().collect()
        } else {
            HashMap::new()
        };
        
        let tool_params = ToolExecutionParams {
            inputs,
            context: HashMap::new(),
            timeout: None,
            execution_id: None,
        };
        
        let result = self.tool_system.execute_tool(tool_name, tool_params).await?;
        
        // Convert ToolExecutionResult to EngineToolResult
        Ok(EngineToolResult {
            success: result.success,
            output: result.output,
            error: result.error,
        })
    }
    
    fn list_tools(&self) -> Vec<String> {
        // ToolSystem 的 list_tools 是异步的，这里需要阻塞等待
        // 暂时返回空列表，等待 ToolManager trait 改为异步
        vec![]
    }
    
    fn has_tool(&self, tool_name: &str) -> bool {
        // ToolSystem 没有同步的 has_tool 方法
        // 暂时返回 true，等待 ToolManager trait 改为异步
        let _ = tool_name;
        true
    }
}


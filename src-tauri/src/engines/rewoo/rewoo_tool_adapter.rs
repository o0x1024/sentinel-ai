//! ReWOO 工具适配器
//! 
//! 将统一工具管理系统适配到 ReWOO 架构中，提供动态工具调用功能
use super::*;
use crate::tools::{
    ToolSystem, ToolExecutionParams, ToolExecutionResult as ToolExecResult,
    ToolManagerConfig, ToolInfo, ToolSearchQuery, ToolParameters
};
use crate::services::database::DatabaseService;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use tokio::time::timeout;
use tracing::{info, warn, error};
use uuid::Uuid;

/// ReWOO 工具调用结构
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: HashMap<String, Value>,
}

/// ReWOO 工具管理器适配器
pub struct ReWOOToolManager {
    /// 统一工具系统
    tool_system: ToolSystem,
    /// 工具缓存
    tool_cache: Arc<tokio::sync::RwLock<HashMap<String, ToolInfo>>>,
}

impl ReWOOToolManager {
    /// 创建新的工具管理器
    pub async fn new(db_service: Arc<DatabaseService>) -> Result<Self> {
        let config = ToolManagerConfig {
            max_concurrent_executions: 10,
            default_timeout: std::time::Duration::from_secs(300),
            log_executions: true,
        };
        
        let tool_system = ToolSystem::new(config);
        tool_system.initialize(db_service).await?;
        
        let tool_cache = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
        
        let manager = Self {
            tool_system,
            tool_cache,
        };
        
        // 初始化工具缓存
        manager.refresh_tool_cache().await?;
        
        Ok(manager)
    }
    
    /// 刷新工具缓存
    pub async fn refresh_tool_cache(&self) -> Result<()> {
        let tools = self.tool_system.list_tools().await;
        let mut cache = self.tool_cache.write().await;
        cache.clear();
        
        for tool in tools {
            cache.insert(tool.name.clone(), tool);
        }
        
        info!("Refreshed tool cache with {} tools", cache.len());
        Ok(())
    }
    
    /// 执行工具调用
    pub async fn execute_tool(&self, tool_call: &ToolCall) -> Result<ToolResult> {
        let start_time = SystemTime::now();
        
        // 检查工具是否存在
        if !self.is_tool_available(&tool_call.name) {
            return Ok(ToolResult {
                success: false,
                content: String::new(),
                error: Some(format!("Tool '{}' is not available", tool_call.name)),
                execution_time_ms: 0,
            });
        }
        
        // 构建执行参数
        let params = ToolExecutionParams {
            inputs: tool_call.args.clone(),
            context: HashMap::new(),
            timeout: Some(std::time::Duration::from_secs(300)),
            execution_id: Some(Uuid::parse_str(&tool_call.id).unwrap_or_else(|_| Uuid::new_v4())),
        };
        
        // 执行工具
        match self.tool_system.execute_tool(&tool_call.name, params).await {
            Ok(result) => {
                let execution_time = start_time.elapsed().unwrap_or(Duration::from_secs(0));
                
                Ok(ToolResult {
                    success: result.success,
                    content: self.format_tool_output(&result),
                    error: result.error,
                    execution_time_ms: execution_time.as_millis() as u64,
                })
            }
            Err(e) => {
                let execution_time = start_time.elapsed().unwrap_or(Duration::from_secs(0));
                
                Ok(ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some(e.to_string()),
                    execution_time_ms: execution_time.as_millis() as u64,
                })
            }
        }
    }
    
    /// 格式化工具输出
    fn format_tool_output(&self, result: &ToolExecResult) -> String {
        match &result.output {
            Value::String(s) => s.clone(),
            Value::Object(_) | Value::Array(_) => {
                serde_json::to_string_pretty(&result.output).unwrap_or_else(|_| "Invalid JSON output".to_string())
            }
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
        }
    }
    
    /// 检查工具是否可用
    pub fn is_tool_available(&self, tool_name: &str) -> bool {
        // 使用异步块来检查缓存
        let cache = self.tool_cache.try_read();
        match cache {
            Ok(cache) => {
                cache.get(tool_name)
                    .map(|tool| tool.available)
                    .unwrap_or(false)
            }
            Err(_) => false,
        }
    }
    
    /// 获取可用工具列表
    pub fn get_available_tools(&self) -> Vec<String> {
        let cache = self.tool_cache.try_read();
        match cache {
            Ok(cache) => {
                cache.values()
                    .filter(|tool| tool.available)
                    .map(|tool| tool.name.clone())
                    .collect()
            }
            Err(_) => Vec::new(),
        }
    }
    
    /// 获取工具信息
    pub async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        let cache = self.tool_cache.read().await;
        cache.get(tool_name).cloned()
    }
    
    /// 搜索工具
    pub async fn search_tools(&self, query: &str) -> Result<Vec<ToolInfo>> {
        let search_query = ToolSearchQuery {
            query: query.to_string(),
            category: None,
            tags: vec![],
            available_only: true,
            installed_only: false,
        };
        
        let result = self.tool_system.search_tools(search_query).await;
        Ok(result.tools)
    }
    
    /// 获取工具使用建议
    pub async fn get_tool_suggestions(&self, task_description: &str) -> Result<Vec<String>> {
        // 基于任务描述搜索相关工具
        let tools = self.search_tools(task_description).await?;
        
        // 返回前5个最相关的工具名称
        Ok(tools.into_iter()
            .take(5)
            .map(|tool| tool.name)
            .collect())
    }
    
    /// 验证工具调用参数
    pub async fn validate_tool_call(&self, tool_call: &ToolCall) -> Result<(), ReWOOError> {
        // 检查工具是否存在
        let tool_info = self.get_tool_info(&tool_call.name).await
            .ok_or_else(|| ReWOOError::ToolExecutionError(
                format!("Tool '{}' not found", tool_call.name)
            ))?;
        
        // 检查工具是否可用
        if !tool_info.available {
            return Err(ReWOOError::ToolExecutionError(
                format!("Tool '{}' is not available", tool_call.name)
            ));
        }
        
        // 验证必需参数
        for param in &tool_info.parameters.parameters {
            if param.required && !tool_call.args.contains_key(&param.name) {
                return Err(ReWOOError::ToolExecutionError(
                    format!("Missing required parameter '{}' for tool '{}'", param.name, tool_call.name)
                ));
            }
        }
        
        Ok(())
    }
    
    /// 获取工具执行统计
    pub async fn get_tool_statistics(&self) -> Result<HashMap<String, u64>> {
        // 这里可以从工具系统获取统计信息
        // 目前返回基本的工具数量统计
        let cache = self.tool_cache.read().await;
        let mut stats = HashMap::new();
        
        stats.insert("total_tools".to_string(), cache.len() as u64);
        stats.insert("available_tools".to_string(), 
            cache.values().filter(|tool| tool.available).count() as u64);
        stats.insert("installed_tools".to_string(), 
            cache.values().filter(|tool| tool.installed).count() as u64);
        
        Ok(stats)
    }
}

/// 工具管理器 trait，用于 ReWOO Worker
#[async_trait::async_trait]
pub trait ToolManager: Send + Sync {
    /// 执行工具调用
    async fn execute_tool(&self, tool_call: &ToolCall) -> Result<ToolResult>;
    
    /// 检查工具是否可用
    fn is_tool_available(&self, tool_name: &str) -> bool;
    
    /// 获取可用工具列表
    fn get_available_tools(&self) -> Vec<String>;
    
    /// 获取指定工具的参数定义（用于提示/Schema生成）
    fn get_tool_parameters(&self, tool_name: &str) -> Option<ToolParameters>;
    
    /// 验证工具调用
    async fn validate_tool_call(&self, tool_call: &ToolCall) -> Result<(), ReWOOError>;
}

#[async_trait::async_trait]
impl ToolManager for ReWOOToolManager {
    async fn execute_tool(&self, tool_call: &ToolCall) -> Result<ToolResult> {
        self.execute_tool(tool_call).await
    }
    
    fn is_tool_available(&self, tool_name: &str) -> bool {
        self.is_tool_available(tool_name)
    }
    
    fn get_available_tools(&self) -> Vec<String> {
        self.get_available_tools()
    }
    
    fn get_tool_parameters(&self, tool_name: &str) -> Option<ToolParameters> {
        // 直接从缓存中读取，以避免异步接口变更
        let cache = self.tool_cache.try_read().ok()?;
        cache.get(tool_name).map(|ti| ti.parameters.clone())
    }
    
    async fn validate_tool_call(&self, tool_call: &ToolCall) -> Result<(), ReWOOError> {
        self.validate_tool_call(tool_call).await
    }
}

/// 创建默认的 ReWOO 工具管理器
pub async fn create_rewoo_tool_manager(db_service: Arc<DatabaseService>) -> Result<Arc<dyn ToolManager>> {
    let manager = ReWOOToolManager::new(db_service).await?;
    Ok(Arc::new(manager))
}

/// 工具调用构建器
pub struct ToolCallBuilder {
    name: String,
    args: HashMap<String, Value>,
}

impl ToolCallBuilder {
    /// 创建新的工具调用构建器
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            args: HashMap::new(),
        }
    }
    
    /// 添加参数
    pub fn arg<T: Into<Value>>(mut self, key: &str, value: T) -> Self {
        self.args.insert(key.to_string(), value.into());
        self
    }
    
    /// 构建工具调用
    pub fn build(self) -> ToolCall {
        ToolCall {
            id: Uuid::new_v4().to_string(),
            name: self.name,
            args: self.args,
        }
    }
}

/// 便捷的工具调用宏
#[macro_export]
macro_rules! tool_call {
    ($name:expr) => {
        ToolCallBuilder::new($name).build()
    };
    ($name:expr, $($key:expr => $value:expr),*) => {
        {
            let mut builder = ToolCallBuilder::new($name);
            $(
                builder = builder.arg($key, $value);
            )*
            builder.build()
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_tool_call_builder() {
        let call = ToolCallBuilder::new("port_scan")
            .arg("target", "192.168.1.1")
            .arg("ports", "80,443")
            .build();
        
        assert_eq!(call.name, "port_scan");
        assert_eq!(call.args.get("target").unwrap(), &json!("192.168.1.1"));
        assert_eq!(call.args.get("ports").unwrap(), &json!("80,443"));
    }
    
    #[test]
    fn test_tool_call_macro() {
        let call = tool_call!("subdomain_scan", "domain" => "example.com", "wordlist" => true);
        
        assert_eq!(call.name, "subdomain_scan");
        assert_eq!(call.args.get("domain").unwrap(), &json!("example.com"));
        assert_eq!(call.args.get("wordlist").unwrap(), &json!(true));
    }
}
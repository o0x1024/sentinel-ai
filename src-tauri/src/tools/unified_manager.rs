//! 统一工具管理器实现

use super::unified_types::*;
use super::BuiltinToolProvider;
use sentinel_tools::UnifiedToolManager;
use crate::services::database::DatabaseService;
use crate::tools::mapping::map_pipeline_input_to_tool_inputs;

use anyhow::{anyhow, Result};
use chrono::Utc;
use futures;
use serde_json::{de, Value};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// ============================================================================
// 统一工具管理器
// ============================================================================

// UnifiedToolManager is now provided by sentinel-tools

// ============================================================================
// 统一工具系统（主入口）
// ============================================================================

/// 统一工具系统
/// 
/// 这是整个工具系统的主入口，提供了所有工具管理功能
pub struct ToolSystem {
    manager: Arc<RwLock<UnifiedToolManager>>,
    // MCP管理器已移除，使用统一管理器
}

impl ToolSystem {
    /// 创建新的工具系统实例
    pub fn new(config: ToolManagerConfig) -> Self {
        let manager = Arc::new(RwLock::new(UnifiedToolManager::new(config)));
        
        Self {
            manager,
        }
    }

    /// 初始化工具系统
    pub async fn initialize(&self, db_service: Arc<DatabaseService>) -> Result<()> {
        debug!("Initializing tool system");
        
        // 注册内置工具提供者
        {
            let mut manager = self.manager.write().await;
            let builtin_provider = Box::new(BuiltinToolProvider::new(db_service.clone()));
            manager.register_provider(builtin_provider).await?;
        }
        
        // 初始化时将内置工具写入数据库（幂等）
        self.persist_builtin_tools_to_db(db_service.clone()).await?;
        
        debug!("Tool system initialized successfully");
        Ok(())
    }

    /// 添加MCP工具提供者
    pub async fn add_mcp_provider_to_system(&self, mcp_service: Arc<crate::services::mcp::McpService>) -> Result<()> {
        debug!("Adding MCP tool provider to system");
        
        // 尝试创建MCP工具提供者
        if let Some(mcp_provider) = crate::tools::create_mcp_tool_provider(mcp_service).await? {
            let mut manager = self.manager.write().await;
            manager.register_provider(mcp_provider).await?;
            debug!("MCP tool provider registered successfully");
        } else {
            warn!("MCP tool provider not available, skipping registration");
        }
        
        Ok(())
    }

    /// 添加MCP提供者（已简化）
    pub async fn add_mcp_provider(&self, _name: String, _config: Option<McpConfig>) -> Result<()> {
        // MCP功能已简化，暂时返回成功
        info!("MCP provider functionality simplified");
        Ok(())
    }

    /// 执行工具
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        params: ToolExecutionParams,
    ) -> Result<ToolExecutionResult> {
        let manager = self.manager.read().await;
        manager.call_tool(tool_name, params).await
    }

    /// 获取所有可用工具
    pub async fn list_tools(&self) -> Vec<ToolInfo> {
        let manager = self.manager.read().await;
        manager.list_tools().await
    }

    /// 搜索工具
    pub async fn search_tools(&self, query: ToolSearchQuery) -> ToolSearchResult {
        let manager = self.manager.read().await;
        manager.search_tools(query).await
    }

    /// 批量执行工具
    pub async fn execute_batch(&self, request: BatchExecutionRequest) -> Result<BatchExecutionResult> {
        let manager = self.manager.read().await;
        manager.execute_batch(request).await
    }

    /// 获取执行历史
    pub async fn get_execution_history(&self, limit: Option<usize>) -> Vec<ToolExecutionRecord> {
        let manager = self.manager.read().await;
        manager.get_execution_history(limit).await
    }

    /// 获取工具统计信息
    pub async fn get_tool_statistics(&self) -> std::collections::HashMap<String, ToolStatistics> {
        let manager = self.manager.read().await;
        manager.get_tool_statistics().await
    }

    /// 刷新所有提供者
    pub async fn refresh_all(&self) -> Result<()> {
        // 刷新主管理器
        {
            let mut manager = self.manager.write().await;
            manager.refresh_all_providers().await?;
        }
        
        // MCP管理器已简化
        
        Ok(())
    }

    /// 将当前已注册的内置工具持久化到 `mcp_tools`（按 name UPSERT）
    async fn persist_builtin_tools_to_db(&self, db_service: Arc<DatabaseService>) -> Result<()> {
        let manager = self.manager.read().await;
        let tools = manager.list_tools().await;

        let pool = db_service.get_pool()?;

        for info in tools {
            // 统一管理器目前注册的是内置工具，标记为 builtin
            let tool_name = info.name;
            let enabled = true;
            let updated_at = Utc::now().timestamp();
            
            sqlx::query(
                r#"
                INSERT INTO builtin_tool_settings (
                   tool_name, enabled, updated_at
                ) VALUES (?, ?, ?)
                ON CONFLICT(tool_name) DO NOTHING 
                "#
            )
            .bind(tool_name)
            .bind(enabled)
            .bind(updated_at)
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    /// 清理执行历史
    pub async fn clear_history(&self) {
        let manager = self.manager.read().await;
        manager.clear_execution_history().await;
    }

    /// 获取工具管理器的引用（用于动态适配器）
    pub fn get_manager(&self) -> Arc<RwLock<UnifiedToolManager>> {
        self.manager.clone()
    }
}

// ============================================================================
// 全局工具系统单例
// ============================================================================

/// 全局工具系统单例
static GLOBAL_TOOL_SYSTEM: OnceLock<Arc<ToolSystem>> = OnceLock::new();

/// 初始化全局工具系统
pub async fn initialize_global_tool_system(db_service: Arc<DatabaseService>) -> Result<()> {
    let system = ToolSystem::new(ToolManagerConfig::default());
    system.initialize(db_service).await?;
    let system = Arc::new(system);
    
    GLOBAL_TOOL_SYSTEM.set(system)
        .map_err(|_| anyhow!("Global tool system already initialized"))?;
    
    debug!("Global tool system initialized successfully");
    Ok(())
}

/// 获取全局工具系统实例
pub fn get_global_tool_system() -> Result<Arc<ToolSystem>> {
    GLOBAL_TOOL_SYSTEM.get()
        .cloned()
        .ok_or_else(|| anyhow!("Global tool system not initialized. Call initialize_global_tool_system first."))
}

/// 检查全局工具系统是否已初始化
pub fn is_global_tool_system_initialized() -> bool {
    GLOBAL_TOOL_SYSTEM.get().is_some()
}

/// 便捷函数：创建默认工具管理器
pub async fn create_default_tool_system(db_service: Arc<DatabaseService>) -> Result<UnifiedToolManager> {
    let mut manager = UnifiedToolManager::new(ToolManagerConfig::default());
    
    // 注册内置工具提供者
    let builtin_provider = Box::new(BuiltinToolProvider::new(db_service.clone()));
    manager.register_provider(builtin_provider).await?;
    
    Ok(manager)
}

/// 创建带有MCP优化的工具管理器
pub async fn create_mcp_optimized_tool_manager(
    db_service: Arc<DatabaseService>,
    mcp_service: Option<Arc<crate::services::mcp::McpService>>,
) -> Result<UnifiedToolManager> {
    let mut manager = UnifiedToolManager::new(ToolManagerConfig::default());
    
    // 注册内置工具提供者
    let builtin_provider = Box::new(BuiltinToolProvider::new(db_service.clone()));
    manager.register_provider(builtin_provider).await?;
    
    // 注册MCP工具提供者（如果可用）
    if let Some(mcp_service) = mcp_service {
        if let Some(mcp_provider) = crate::tools::create_mcp_tool_provider(mcp_service).await? {
            manager.register_provider(mcp_provider).await?;
            info!("MCP tool provider registered in optimized manager");
        } else {
            warn!("MCP tool provider not available for optimized manager");
        }
    } else {
        info!("No MCP service provided, skipping MCP tools");
    }
    
    info!("Created MCP-optimized tool manager with performance enhancements");
    Ok(manager)
}

// ============================================================================
// 动态工具调用适配器
// ============================================================================

/// 动态工具调用适配器
/// 提供便捷的工具调用方法，用于替代硬编码的工具调用
pub struct DynamicToolAdapter {
    tool_system: Arc<ToolSystem>,
}

impl DynamicToolAdapter {
    pub fn new(tool_system: Arc<ToolSystem>) -> Self {
        Self { tool_system }
    }
    
    /// 执行扫描任务（替代硬编码的扫描函数）
    pub async fn execute_scan_task(
        &self,
        tool_name: &str,
        target: &str,
        params: Option<HashMap<String, Value>>,
    ) -> Result<ToolExecutionResult> {
        let mut inputs = params.unwrap_or_default();
        inputs.insert("target".to_string(), serde_json::json!(target));
        
        let execution_params = ToolExecutionParams {
            inputs,
            context: HashMap::new(),
            timeout: Some(Duration::from_secs(300)),
            execution_id: Some(Uuid::new_v4()),
        };
        
        info!("Executing dynamic tool: {} for target: {}", tool_name, target);
        
        match self.tool_system.execute_tool(tool_name, execution_params).await {
            Ok(result) => {
                info!("Tool {} executed successfully", tool_name);
                Ok(result)
            }
            Err(e) => {
                error!("Tool {} execution failed: {}", tool_name, e);
                Err(e)
            }
        }
    }
    
    /// 执行通用任务
    pub async fn execute_generic_task(
        &self,
        tool_name: &str,
        params: HashMap<String, Value>,
    ) -> Result<ToolExecutionResult> {
        let execution_params = ToolExecutionParams {
            inputs: params,
            context: HashMap::new(),
            timeout: Some(Duration::from_secs(300)),
            execution_id: Some(Uuid::new_v4()),
        };
        
        self.tool_system.execute_tool(tool_name, execution_params).await
    }
    
    /// 获取可用工具列表
    pub async fn get_available_tools(&self) -> Result<Vec<ToolInfo>> {
        Ok(self.tool_system.list_tools().await)
    }
    
    /// 搜索工具
    pub async fn search_tools(&self, query: &str) -> Result<Vec<ToolInfo>> {
        let search_query = ToolSearchQuery {
            query: query.to_string(),
            category: None,
            tags: vec![],
            available_only: false,
            installed_only: false,
        };
        let result = self.tool_system.search_tools(search_query).await;
        Ok(result.tools)
    }
    

    
    /// 获取工具执行历史
    pub async fn get_execution_history(&self) -> Result<Vec<ToolExecutionRecord>> {
        Ok(self.tool_system.get_execution_history(None).await)
    }
    
    /// 获取工具统计信息
    pub async fn get_tool_statistics(&self) -> Result<HashMap<String, ToolStatistics>> {
        Ok(self.tool_system.get_tool_statistics().await)
    }
    
    /// 批量执行工具
    pub async fn execute_batch(
        &self,
        request: BatchExecutionRequest,
    ) -> Result<BatchExecutionResult> {
        self.tool_system.execute_batch(request).await
    }

    // ============================================================================
    // 引擎便捷方法 (替代EngineToolAdapter)
    // ============================================================================

    /// 执行工具 (引擎调用方式 - 直接传入工具名称和参数)
    pub async fn execute_tool(&self, tool_name: &str, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        self.tool_system.execute_tool(tool_name, params).await
    }

    /// 获取可用工具列表
    pub async fn list_available_tools(&self) -> Vec<String> {
        let tools = self.tool_system.list_tools().await;
        tools.into_iter()
            .filter(|tool| tool.available)
            .map(|tool| tool.name)
            .collect()
    }

    /// 获取工具信息
    pub async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        let tools = self.tool_system.list_tools().await;
        tools.into_iter().find(|tool| tool.name == tool_name)
    }

    /// 检查工具是否可用
    pub async fn is_tool_available(&self, tool_name: &str) -> bool {
        let tools = self.tool_system.list_tools().await;
        tools.iter().any(|tool| tool.name == tool_name && tool.available)
    }

    /// 验证工具调用参数
    pub async fn validate_tool_call(&self, tool_name: &str, params: &ToolExecutionParams) -> Result<()> {
        // 检查工具是否存在
        if !self.is_tool_available(tool_name).await {
            return Err(anyhow!("Tool '{}' is not available", tool_name));
        }
        
        // 获取工具信息并验证参数
        if let Some(tool_info) = self.get_tool_info(tool_name).await {
            for param in &tool_info.parameters.parameters {
                if param.required && !params.inputs.contains_key(&param.name) {
                    return Err(anyhow!(
                        "Missing required parameter '{}' for tool '{}'", 
                        param.name, tool_name
                    ));
                }
            }
        }
        
        Ok(())
    }

    /// 批量执行工具
    pub async fn execute_tools_batch(&self, calls: Vec<(String, ToolExecutionParams)>) -> Vec<Result<ToolExecutionResult>> {
        if calls.is_empty() {
            return Vec::new();
        }
        
        info!("Executing batch of {} tools", calls.len());
        
        let mut results = Vec::new();
        for (tool_name, params) in calls {
            let result = self.execute_tool(&tool_name, params).await;
            results.push(result);
        }
        
        info!("Batch execution completed with {} results", results.len());
        results
    }
}





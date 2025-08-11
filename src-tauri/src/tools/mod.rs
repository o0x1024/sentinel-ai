//! 统一工具管理系统
//! 
//! 这是一个完整的工具管理系统，提供：
//! - 统一的工具接口和管理
//! - 内置工具和MCP工具的统一调度
//! - 动态工具发现和执行
//! - 批量执行和并发控制
//! - 执行历史和统计信息
//! - 便捷的工具调用适配器

pub mod builtin;
pub mod mcp;
pub mod mapping;

// 重新导出子模块
pub use builtin::BuiltinToolProvider;
pub use mcp::{McpToolProvider, McpConfig, McpToolManager};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{error, info, warn};
use uuid::Uuid;
use crate::services::database::DatabaseService;
use crate::tools::mapping::map_pipeline_input_to_tool_inputs;

// ============================================================================
// 核心类型定义
// ============================================================================

/// 工具分类
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCategory {
    NetworkScanning,
    VulnerabilityScanning,
    ServiceDetection,
    CodeAnalysis,
    DataProcessing,
    SystemUtility,
    Custom(String),
}

impl std::fmt::Display for ToolCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolCategory::NetworkScanning => write!(f, "network_scanning"),
            ToolCategory::VulnerabilityScanning => write!(f, "vulnerability_scanning"),
            ToolCategory::ServiceDetection => write!(f, "service_detection"),
            ToolCategory::CodeAnalysis => write!(f, "code_analysis"),
            ToolCategory::DataProcessing => write!(f, "data_processing"),
            ToolCategory::SystemUtility => write!(f, "system_utility"),
            ToolCategory::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// 工具参数类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
}

/// 工具参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDefinition {
    pub name: String,
    pub param_type: ParameterType,
    pub description: String,
    pub required: bool,
    pub default_value: Option<Value>,
}

/// 工具参数集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    pub parameters: Vec<ParameterDefinition>,
}

/// 工具元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub author: String,
    pub version: String,
    pub license: String,
    pub tags: Vec<String>,
    pub requirements: Vec<String>,
}

/// 工具执行参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionParams {
    pub inputs: HashMap<String, Value>,
    pub context: HashMap<String, Value>,
    pub timeout: Option<Duration>,
    pub execution_id: Option<Uuid>,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub execution_id: Uuid,
    pub tool_name: String,
    pub success: bool,
    pub output: Value,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub metadata: HashMap<String, Value>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// 工具信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub parameters: ToolParameters,
    pub metadata: ToolMetadata,
    pub available: bool,
    pub installed: bool,
}

/// 工具执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRequest {
    pub tool_name: String,
    pub params: ToolExecutionParams,
    pub priority: Option<u8>,
}

/// 工具执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionRecord {
    pub execution_id: Uuid,
    pub tool_name: String,
    pub params: ToolExecutionParams,
    pub result: Option<ToolExecutionResult>,
    pub created_at: DateTime<Utc>,
}

/// 工具搜索查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSearchQuery {
    pub query: String,
    pub category: Option<ToolCategory>,
    pub tags: Vec<String>,
    pub available_only: bool,
    pub installed_only: bool,
}

/// 工具搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSearchResult {
    pub tools: Vec<ToolInfo>,
    pub total_count: usize,
    pub query: ToolSearchQuery,
}

/// 批量执行模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchExecutionMode {
    Parallel,
    Sequential,
    Pipeline,
}

/// 批量执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchExecutionRequest {
    pub requests: Vec<ToolExecutionRequest>,
    pub mode: BatchExecutionMode,
    pub stop_on_error: bool,
}

/// 批量执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchExecutionResult {
    pub batch_id: Uuid,
    pub results: Vec<ToolExecutionResult>,
    pub success_count: usize,
    pub failure_count: usize,
    pub total_execution_time_ms: u64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// 工具统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatistics {
    pub tool_name: String,
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub average_execution_time_ms: f64,
    pub total_execution_time_ms: u64,
}

// ============================================================================
// 核心接口定义
// ============================================================================

/// 统一工具接口
#[async_trait]
pub trait UnifiedTool: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn category(&self) -> ToolCategory;
    fn parameters(&self) -> &ToolParameters;
    fn metadata(&self) -> &ToolMetadata;
    
    async fn is_available(&self) -> bool {
        true
    }
    
    async fn is_installed(&self) -> bool {
        true
    }
    
    fn validate_params(&self, params: &ToolExecutionParams) -> Result<()> {
        for param_def in &self.parameters().parameters {
            if param_def.required && !params.inputs.contains_key(&param_def.name) {
                return Err(anyhow!("Missing required parameter: {}", param_def.name));
            }
        }
        Ok(())
    }
    
    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult>;
}

/// 工具提供者接口
#[async_trait]
pub trait ToolProvider: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn get_tools(&self) -> Result<Vec<Arc<dyn UnifiedTool>>>;
    async fn get_tool(&self, name: &str) -> Result<Option<Arc<dyn UnifiedTool>>>;
    async fn refresh(&self) -> Result<()>;
    async fn is_available(&self) -> bool {
        true
    }
}

// ============================================================================
// 工具管理器配置
// ============================================================================

/// 工具管理器配置
#[derive(Debug, Clone)]
pub struct ToolManagerConfig {
    pub max_concurrent_executions: usize,
    pub default_timeout: Duration,
    pub log_executions: bool,
}

impl Default for ToolManagerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_executions: 10,
            default_timeout: Duration::from_secs(300),
            log_executions: true,
        }
    }
}

// ============================================================================
// 统一工具管理器
// ============================================================================

/// 统一工具管理器
#[derive(Debug)]
pub struct UnifiedToolManager {
    providers: HashMap<String, Box<dyn ToolProvider>>,
    tool_registry: Arc<RwLock<HashMap<String, Arc<dyn UnifiedTool>>>>,
    execution_history: Arc<RwLock<Vec<ToolExecutionRecord>>>,
    config: ToolManagerConfig,
    execution_counter: Arc<RwLock<usize>>,
}

impl UnifiedToolManager {
    pub fn new(config: ToolManagerConfig) -> Self {
        Self {
            providers: HashMap::new(),
            tool_registry: Arc::new(RwLock::new(HashMap::new())),
            execution_history: Arc::new(RwLock::new(Vec::new())),
            config,
            execution_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// 注册工具提供者
    pub async fn register_provider(&mut self, provider: Box<dyn ToolProvider>) -> Result<()> {
        let provider_name = provider.name().to_string();
        info!("Registering tool provider: {}", provider_name);

        if !provider.is_available().await {
            warn!("Tool provider {} is not available", provider_name);
            return Err(anyhow!("Tool provider {} is not available", provider_name));
        }

        let tools = provider.get_tools().await?;
        let mut registry = self.tool_registry.write().await;
        
        for tool in tools {
            let tool_name = tool.name().to_string();
            info!("Registering tool: {} from provider: {}", tool_name, provider_name);
            registry.insert(tool_name, tool);
        }

        self.providers.insert(provider_name, provider);
        Ok(())
    }

    /// 刷新所有提供者的工具
    pub async fn refresh_all_providers(&mut self) -> Result<()> {
        info!("Refreshing all tool providers");
        
        for (provider_name, provider) in &self.providers {
            info!("Refreshing provider: {}", provider_name);
            if let Err(e) = provider.refresh().await {
                error!("Failed to refresh provider {}: {}", provider_name, e);
                continue;
            }

            match provider.get_tools().await {
                Ok(tools) => {
                    let mut registry = self.tool_registry.write().await;
                    for tool in tools {
                        let tool_name = tool.name().to_string();
                        registry.insert(tool_name, tool);
                    }
                }
                Err(e) => {
                    error!("Failed to get tools from provider {}: {}", provider_name, e);
                }
            }
        }

        Ok(())
    }

    /// 动态调用工具
    pub async fn call_tool(
        &self,
        tool_name: &str,
        mut params: ToolExecutionParams,
    ) -> Result<ToolExecutionResult> {
        let start_time = Instant::now();
        let execution_id = Uuid::new_v4();
        params.execution_id = Some(execution_id);

        info!("Executing tool: {} with execution_id: {}", tool_name, execution_id);

        // 检查并发限制
        {
            let mut counter = self.execution_counter.write().await;
            *counter += 1;
            if *counter > self.config.max_concurrent_executions {
                *counter -= 1;
                return Err(anyhow!(
                    "Maximum concurrent executions ({}) exceeded",
                    self.config.max_concurrent_executions
                ));
            }
        }

        // 获取工具
        let tool = {
            let registry = self.tool_registry.read().await;
            registry.get(tool_name).cloned()
        };

        let tool = match tool {
            Some(tool) => tool,
            None => {
                self.decrement_counter().await;
                return Err(anyhow!("Tool '{}' not found", tool_name));
            }
        };

        // 验证参数
        if let Err(e) = tool.validate_params(&params) {
            self.decrement_counter().await;
            return Err(anyhow!("Parameter validation failed for tool '{}': {}", tool_name, e));
        }

        // 检查工具是否可用
        if !tool.is_available().await {
            self.decrement_counter().await;
            return Err(anyhow!("Tool '{}' is not available", tool_name));
        }
        tracing::info!("tool: {:?},params: {:?}", tool.name(), params);
        // 执行工具（带超时）
        let execution_timeout = params.timeout.unwrap_or(self.config.default_timeout);
        // 在执行前，如果存在 pipeline_input，尝试根据工具参数进行映射补全
        let mut patched_params = params.clone();
        if let Some(pipeline_input) = patched_params.context.remove("pipeline_input") {
            let new_inputs = map_pipeline_input_to_tool_inputs(tool.as_ref(), patched_params.inputs.clone(), pipeline_input);
            patched_params.inputs = new_inputs;
        }

        let result = match timeout(execution_timeout, tool.execute(patched_params)).await {
            Ok(Ok(mut result)) => {
                result.execution_id = execution_id;
                result.execution_time_ms = start_time.elapsed().as_millis() as u64;
                result.started_at = Utc::now();
                result.completed_at = Some(Utc::now());
                Ok(result)
            }
            Ok(Err(e)) => {
                error!("Tool execution failed: {}", e);
                Err(e)
            }
            Err(_) => {
                error!("Tool execution timed out after {:?}", execution_timeout);
                Err(anyhow!("Tool execution timed out after {:?}", execution_timeout))
            }
        };

        // 减少执行计数器
        self.decrement_counter().await;

        // 记录执行历史
        if self.config.log_executions {
            let record = ToolExecutionRecord {
                execution_id,
                tool_name: tool_name.to_string(),
                params,
                result: result.as_ref().ok().cloned(),
                created_at: Utc::now(),
            };
            
            let mut history = self.execution_history.write().await;
            history.push(record);
            
            // 限制历史记录数量
            if history.len() > 1000 {
                history.drain(0..100);
            }
        }

        result
    }

    /// 获取可用工具列表
    pub async fn list_tools(&self) -> Vec<ToolInfo> {
        let registry = self.tool_registry.read().await;
        let mut tools = Vec::new();

        for tool in registry.values() {
            let tool_info = ToolInfo {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                category: tool.category(),
                parameters: tool.parameters().clone(),
                metadata: tool.metadata().clone(),
                available: tool.is_available().await,
                installed: tool.is_installed().await,
            };
            tools.push(tool_info);
        }

        tools
    }

    /// 搜索工具
    pub async fn search_tools(&self, query: ToolSearchQuery) -> ToolSearchResult {
        let all_tools = self.list_tools().await;
        let query_lower = query.query.to_lowercase();
        
        let filtered_tools: Vec<ToolInfo> = all_tools
            .into_iter()
            .filter(|tool| {
                // 文本匹配
                let text_match = tool.name.to_lowercase().contains(&query_lower)
                    || tool.description.to_lowercase().contains(&query_lower)
                    || tool.metadata.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower));
                
                // 分类匹配
                let category_match = query.category.as_ref()
                    .map(|cat| std::mem::discriminant(cat) == std::mem::discriminant(&tool.category))
                    .unwrap_or(true);
                
                // 标签匹配
                let tag_match = query.tags.is_empty() || query.tags.iter().any(|tag| {
                    tool.metadata.tags.iter().any(|tool_tag| tool_tag.to_lowercase().contains(&tag.to_lowercase()))
                });
                
                // 可用性过滤
                let available_filter = !query.available_only || tool.available;
                let installed_filter = !query.installed_only || tool.installed;
                
                text_match && category_match && tag_match && available_filter && installed_filter
            })
            .collect();

        ToolSearchResult {
            total_count: filtered_tools.len(),
            tools: filtered_tools,
            query,
        }
    }

    /// 批量执行工具
    pub async fn execute_batch(&self, request: BatchExecutionRequest) -> Result<BatchExecutionResult> {
        let batch_id = Uuid::new_v4();
        let start_time = Instant::now();
        let started_at = Utc::now();
        
        info!("Starting batch execution: {} with {} tools", batch_id, request.requests.len());

        let mut results = Vec::new();
        let mut success_count = 0;
        let mut failure_count = 0;

        match request.mode {
            BatchExecutionMode::Parallel => {
                let mut futures = Vec::new();
                for req in &request.requests {
                    let future = self.call_tool(&req.tool_name, req.params.clone());
                    futures.push(future);
                }
                
                let batch_results = futures::future::join_all(futures).await;
                
                for result in batch_results {
                    match result {
                        Ok(exec_result) => {
                            if exec_result.success {
                                success_count += 1;
                            } else {
                                failure_count += 1;
                            }
                            results.push(exec_result);
                        }
                        Err(e) => {
                            error!("Batch execution error: {}", e);
                            failure_count += 1;
                            if request.stop_on_error {
                                break;
                            }
                        }
                    }
                }
            }
            BatchExecutionMode::Sequential => {
                for req in request.requests {
                    match self.call_tool(&req.tool_name, req.params).await {
                        Ok(exec_result) => {
                            if exec_result.success {
                                success_count += 1;
                            } else {
                                failure_count += 1;
                            }
                            results.push(exec_result);
                        }
                        Err(e) => {
                            error!("Sequential execution error: {}", e);
                            failure_count += 1;
                            if request.stop_on_error {
                                break;
                            }
                        }
                    }
                }
            }
            BatchExecutionMode::Pipeline => {
                let mut previous_output: Option<Value> = None;
                
                for mut req in request.requests {
                    if let Some(ref output) = previous_output {
                        req.params.inputs.insert("pipeline_input".to_string(), output.clone());
                    }
                    
                    match self.call_tool(&req.tool_name, req.params).await {
                        Ok(exec_result) => {
                            if exec_result.success {
                                success_count += 1;
                                previous_output = Some(exec_result.output.clone());
                            } else {
                                failure_count += 1;
                                if request.stop_on_error {
                                    break;
                                }
                            }
                            results.push(exec_result);
                        }
                        Err(e) => {
                            error!("Pipeline execution error: {}", e);
                            failure_count += 1;
                            if request.stop_on_error {
                                break;
                            }
                        }
                    }
                }
            }
        }

        let total_execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(BatchExecutionResult {
            batch_id,
            results,
            success_count,
            failure_count,
            total_execution_time_ms,
            started_at,
            completed_at: Some(Utc::now()),
        })
    }

    /// 获取执行历史
    pub async fn get_execution_history(&self, limit: Option<usize>) -> Vec<ToolExecutionRecord> {
        let history = self.execution_history.read().await;
        let limit = limit.unwrap_or(100);
        
        if history.len() <= limit {
            history.clone()
        } else {
            history[history.len() - limit..].to_vec()
        }
    }

    /// 获取工具统计信息
    pub async fn get_tool_statistics(&self) -> HashMap<String, ToolStatistics> {
        let history = self.execution_history.read().await;
        let mut stats: HashMap<String, ToolStatistics> = HashMap::new();

        for record in history.iter() {
            let tool_stats = stats.entry(record.tool_name.clone()).or_insert_with(|| ToolStatistics {
                tool_name: record.tool_name.clone(),
                total_executions: 0,
                successful_executions: 0,
                failed_executions: 0,
                average_execution_time_ms: 0.0,
                total_execution_time_ms: 0,
            });

            tool_stats.total_executions += 1;
            
            if let Some(ref result) = record.result {
                if result.success {
                    tool_stats.successful_executions += 1;
                } else {
                    tool_stats.failed_executions += 1;
                }
                tool_stats.total_execution_time_ms += result.execution_time_ms;
            } else {
                tool_stats.failed_executions += 1;
            }
        }

        // 计算平均执行时间
        for stat in stats.values_mut() {
            if stat.total_executions > 0 {
                stat.average_execution_time_ms = stat.total_execution_time_ms as f64 / stat.total_executions as f64;
            }
        }

        stats
    }

    /// 清理执行历史
    pub async fn clear_execution_history(&self) {
        let mut history = self.execution_history.write().await;
        history.clear();
        info!("Execution history cleared");
    }

    /// 减少执行计数器
    async fn decrement_counter(&self) {
        let mut counter = self.execution_counter.write().await;
        if *counter > 0 {
            *counter -= 1;
        }
    }
}

// ============================================================================
// 统一工具系统（主入口）
// ============================================================================

/// 统一工具系统
/// 
/// 这是整个工具系统的主入口，提供了所有工具管理功能
pub struct ToolSystem {
    manager: Arc<RwLock<UnifiedToolManager>>,
    mcp_manager: Arc<RwLock<McpToolManager>>,
}

impl ToolSystem {
    /// 创建新的工具系统实例
    pub fn new(config: ToolManagerConfig) -> Self {
        let manager = Arc::new(RwLock::new(UnifiedToolManager::new(config)));
        let mcp_manager = Arc::new(RwLock::new(McpToolManager::new(McpConfig::default())));
        
        Self {
            manager,
            mcp_manager,
        }
    }

    /// 初始化工具系统
    pub async fn initialize(&self, db_service: Arc<DatabaseService>) -> Result<()> {
        info!("Initializing tool system");
        
        // 注册内置工具提供者
        {
            let mut manager = self.manager.write().await;
            let builtin_provider = Box::new(BuiltinToolProvider::new(db_service.clone()));
            manager.register_provider(builtin_provider).await?;
        }
        // 初始化时将内置工具写入数据库（幂等）
        self.persist_builtin_tools_to_db(db_service.clone()).await?;
        
        info!("Tool system initialized successfully");
        Ok(())
    }

    /// 添加MCP提供者
    pub async fn add_mcp_provider(&self, name: String, config: Option<McpConfig>) -> Result<()> {
        let mut mcp_manager = self.mcp_manager.write().await;
        mcp_manager.add_provider(name, config).await?;
        
        // 将MCP工具注册到主管理器
        let _mcp_tools = mcp_manager.get_all_tools().await?;
        let mut manager = self.manager.write().await;
        
        // 创建一个临时的MCP提供者来注册工具
        let mcp_provider = Box::new(McpToolProvider::new());
        manager.register_provider(mcp_provider).await?;
        
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
        
        // 刷新MCP管理器
        {
            let mcp_manager = self.mcp_manager.read().await;
            mcp_manager.refresh_all().await?;
        }
        
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
        inputs.insert("target".to_string(), json!(target));
        
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
    
    /// 检查工具是否可用
    pub async fn is_tool_available(&self, tool_name: &str) -> bool {
        match self.get_available_tools().await {
            Ok(tools) => tools.iter().any(|t| t.name == tool_name),
            Err(_) => false,
        }
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
    
    /// 根据动作名称动态选择和执行工具
    pub async fn execute_by_action(
        &self,
        action: &str,
        target: &str,
        params: Option<HashMap<String, Value>>,
    ) -> Result<ToolExecutionResult> {
        let tool_name = match action {
            "nmap_scan" | "port_scan" => "nmap_scan",
            "service_scan" | "service_detection" => "service_detection",
            "nuclei_scan" | "vulnerability_scan" => "nuclei_scan",
            "subdomain_enum" | "subdomain_enumeration" => "subdomain_enum",
            "web_scan" | "web_scanning" => "web_scan",
            "ssl_scan" | "ssl_check" => "ssl_scan",
            "code_analysis" | "static_analysis" => "code_analysis",
            "security_scan" => "security_scan",
            _ => {
                warn!("Unknown action: {}, falling back to generic execution", action);
                action
            }
        };
        
        self.execute_scan_task(tool_name, target, params).await
    }
}

/// 工具执行上下文
#[derive(Debug, Clone)]
pub struct ToolExecutionContext {
    pub session_id: Uuid,
    pub user_id: Option<String>,
    pub workspace: Option<String>,
    pub environment: HashMap<String, String>,
}

impl Default for ToolExecutionContext {
    fn default() -> Self {
        Self {
            session_id: Uuid::new_v4(),
            user_id: None,
            workspace: None,
            environment: HashMap::new(),
        }
    }
}

/// 扩展的动态工具适配器，支持上下文和会话管理
pub struct ContextualToolAdapter {
    adapter: DynamicToolAdapter,
    context: ToolExecutionContext,
}

impl ContextualToolAdapter {
    pub fn new(tool_system: Arc<ToolSystem>, context: ToolExecutionContext) -> Self {
        Self {
            adapter: DynamicToolAdapter::new(tool_system),
            context,
        }
    }
    
    pub fn with_default_context(tool_system: Arc<ToolSystem>) -> Self {
        Self::new(tool_system, ToolExecutionContext::default())
    }
    
    /// 在当前上下文中执行工具
    pub async fn execute_with_context(
        &self,
        tool_name: &str,
        target: &str,
        params: Option<HashMap<String, Value>>,
    ) -> Result<ToolExecutionResult> {
        let mut enhanced_params = params.unwrap_or_default();
        
        // 添加上下文信息
        enhanced_params.insert("session_id".to_string(), json!(self.context.session_id));
        if let Some(user_id) = &self.context.user_id {
            enhanced_params.insert("user_id".to_string(), json!(user_id));
        }
        if let Some(workspace) = &self.context.workspace {
            enhanced_params.insert("workspace".to_string(), json!(workspace));
        }
        
        // 添加环境变量
        for (key, value) in &self.context.environment {
            enhanced_params.insert(format!("env_{}", key), json!(value));
        }
        
        self.adapter.execute_scan_task(tool_name, target, Some(enhanced_params)).await
    }
    
    /// 获取适配器引用
    pub fn adapter(&self) -> &DynamicToolAdapter {
        &self.adapter
    }
    
    /// 获取上下文引用
    pub fn context(&self) -> &ToolExecutionContext {
        &self.context
    }
    
    /// 更新上下文
    pub fn update_context(&mut self, context: ToolExecutionContext) {
        self.context = context;
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
    
    info!("Global tool system initialized successfully");
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

// ============================================================================
// 便捷函数
// ============================================================================

/// 创建默认的工具系统实例
pub async fn create_default_tool_system(db_service: Arc<DatabaseService>) -> Result<ToolSystem> {
    let system = ToolSystem::new(ToolManagerConfig::default());
    system.initialize(db_service).await?;
    Ok(system)
}

/// 创建带有自定义配置的工具系统实例
pub async fn create_tool_system_with_config(config: ToolManagerConfig, db_service: Arc<DatabaseService>) -> Result<ToolSystem> {
    let system = ToolSystem::new(config);
    system.initialize(db_service).await?;
    Ok(system)
}

/// 创建动态工具适配器（使用全局工具系统）
pub fn create_dynamic_adapter() -> Result<DynamicToolAdapter> {
    let tool_system = get_global_tool_system()?;
    Ok(DynamicToolAdapter::new(tool_system))
}

/// 创建动态工具适配器（使用指定的工具系统）
pub fn create_dynamic_adapter_with_system(tool_system: Arc<ToolSystem>) -> DynamicToolAdapter {
    DynamicToolAdapter::new(tool_system)
}

/// 创建上下文工具适配器（使用全局工具系统）
pub fn create_contextual_adapter(context: ToolExecutionContext) -> Result<ContextualToolAdapter> {
    let tool_system = get_global_tool_system()?;
    Ok(ContextualToolAdapter::new(tool_system, context))
}

/// 创建上下文工具适配器（使用指定的工具系统）
pub fn create_contextual_adapter_with_system(tool_system: Arc<ToolSystem>, context: ToolExecutionContext) -> ContextualToolAdapter {
    ContextualToolAdapter::new(tool_system, context)
}

// ============================================================================
// 测试辅助函数
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_tool_system_creation() {
        let mut db_service = DatabaseService::new();
        db_service.initialize().await.unwrap();
        let db_service = Arc::new(db_service);
        let system = create_default_tool_system(db_service).await.unwrap();
        let tools = system.list_tools().await;
        println!("{:?}", tools);
        assert!(!tools.is_empty());
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let mut db_service = DatabaseService::new();
        db_service.initialize().await.unwrap();
        let db_service = Arc::new(db_service);
        let system = create_default_tool_system(db_service).await.unwrap();
        
        let mut inputs = HashMap::new();
        inputs.insert("target".to_string(), json!("127.0.0.1"));
        inputs.insert("start_port".to_string(), json!(1));
        inputs.insert("end_port".to_string(), json!(1000));
        
        let params = ToolExecutionParams {
            inputs,
            context: HashMap::new(),
            timeout: None,
            execution_id: None,
        };
        
        let result = system.execute_tool("port_scan", params).await;
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tool_execution_subdomain() {
        let mut db_service = DatabaseService::new();
        db_service.initialize().await.unwrap();
        let db_service = Arc::new(db_service);
        let system = create_default_tool_system(db_service).await.unwrap();
        
        let mut inputs = HashMap::new();
        inputs.insert("domain".to_string(), json!("mgtv.com"));
        
        let params = ToolExecutionParams {
            inputs,
            context: HashMap::new(),
            timeout: None,
            execution_id: None,
        };
        
        let result = system.execute_tool("rsubdomain", params).await;
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dynamic_adapter() {
        let mut db_service = DatabaseService::new();
        db_service.initialize().await.unwrap();
        let db_service = Arc::new(db_service);
        let system = Arc::new(create_default_tool_system(db_service).await.unwrap());
        let adapter = create_dynamic_adapter_with_system(system);
        
        let tools = adapter.get_available_tools().await.unwrap();
        assert!(!tools.is_empty());
        
        let mut params = HashMap::new();
        params.insert("port_range".to_string(), json!("80-85"));
        let result = adapter.execute_scan_task("port_scan", "127.0.0.1", Some(params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tool_search() {
        let mut db_service = DatabaseService::new();
        db_service.initialize().await.unwrap();
        let db_service = Arc::new(db_service);
        let system = create_default_tool_system(db_service).await.unwrap();
        
        let query = ToolSearchQuery {
            query: "scan".to_string(),
            category: None,
            tags: vec![],
            available_only: false,
            installed_only: false,
        };
        
        let results = system.search_tools(query).await;
        assert!(!results.tools.is_empty());
    }

    #[tokio::test]
    async fn test_batch_execution() {
        let mut db_service = DatabaseService::new();
        db_service.initialize().await.unwrap();
        let db_service = Arc::new(db_service);
        let system = create_default_tool_system(db_service).await.unwrap();
        
        let mut inputs1 = HashMap::new();
        inputs1.insert("target".to_string(), json!("127.0.0.1"));
        inputs1.insert("start_port".to_string(), json!(80));
        inputs1.insert("end_port".to_string(), json!(82));
        
        let mut inputs2 = HashMap::new();
        inputs2.insert("target".to_string(), json!("127.0.0.1"));
        inputs2.insert("start_port".to_string(), json!(443));
        inputs2.insert("end_port".to_string(), json!(445));
        
        let requests = vec![
            ToolExecutionRequest {
                tool_name: "port_scan".to_string(),
                params: ToolExecutionParams {
                    inputs: inputs1,
                    context: HashMap::new(),
                    timeout: None,
                    execution_id: None,
                },
                priority: None,
            },
            ToolExecutionRequest {
                tool_name: "port_scan".to_string(),
                params: ToolExecutionParams {
                    inputs: inputs2,
                    context: HashMap::new(),
                    timeout: None,
                    execution_id: None,
                },
                priority: None,
            },
        ];
        
        let batch_request = BatchExecutionRequest {
            requests,
            mode: BatchExecutionMode::Parallel,
            stop_on_error: false,
        };
        
        let result = system.execute_batch(batch_request).await;
        assert!(result.is_ok());
        
        let batch_result = result.unwrap();
        assert_eq!(batch_result.results.len(), 2);
    }
}

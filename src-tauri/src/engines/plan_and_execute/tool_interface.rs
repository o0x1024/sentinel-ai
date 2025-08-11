//! Tool Interface 组件 - 工具接口
//! 
//! 负责管理和调用各种工具，提供统一的工具调用接口
//! 现在使用统一工具管理系统

use crate::engines::plan_and_execute::types::*;
use crate::tools::{
    UnifiedToolManager, ToolManagerConfig, BuiltinToolProvider,
    ToolExecutionParams, ToolExecutionResult, ToolInfo,
    ToolCategory as UnifiedToolCategory, ParameterType, ParameterDefinition,
    ToolParameters, ToolMetadata
};
use crate::services::database::DatabaseService;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::{debug, error, info, warn};
use chrono::{DateTime, Utc};

/// 工具接口配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInterfaceConfig {
    /// 默认超时时间（秒）
    pub default_timeout: u64,
    /// 最大并发工具调用数
    pub max_concurrent_calls: u32,
    /// 是否启用工具缓存
    pub enable_caching: bool,
    /// 缓存过期时间（秒）
    pub cache_expiry_seconds: u64,
    /// 重试配置
    pub default_retry_config: RetryConfig,
    /// 工具发现配置
    pub discovery_config: ToolDiscoveryConfig,
}

/// 工具发现配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDiscoveryConfig {
    /// 是否启用自动发现
    pub auto_discovery: bool,
    /// 工具注册表路径
    pub registry_paths: Vec<String>,
    /// 扫描间隔（秒）
    pub scan_interval_seconds: u64,
    /// 支持的工具类型
    pub supported_types: Vec<ToolType>,
}

/// 工具类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolType {
    /// 网络扫描工具
    NetworkScanner,
    /// 漏洞扫描工具
    VulnerabilityScanner,
    /// 端口扫描工具
    PortScanner,
    /// Web爬虫工具
    WebCrawler,
    /// 数据分析工具
    DataAnalyzer,
    /// 报告生成工具
    ReportGenerator,
    /// 文件处理工具
    FileProcessor,
    /// API调用工具
    ApiCaller,
    /// 数据库工具
    DatabaseTool,
    /// 系统工具
    SystemTool,
    /// 自定义工具
    Custom(String),
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 工具ID
    pub id: String,
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 工具类型
    pub tool_type: ToolType,
    /// 工具版本
    pub version: String,
    /// 输入参数定义
    pub input_schema: serde_json::Value,
    /// 输出格式定义
    pub output_schema: serde_json::Value,
    /// 工具配置
    pub config: ToolConfig,
    /// 工具元数据
    pub metadata: HashMap<String, serde_json::Value>,
    /// 是否可用
    pub available: bool,
    /// 最后检查时间
    pub last_checked: SystemTime,
}

/// 工具调用请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 调用ID
    pub id: String,
    /// 工具名称
    pub tool_name: String,
    /// 调用参数
    pub parameters: HashMap<String, serde_json::Value>,
    /// 超时时间（秒）
    pub timeout: Option<u64>,
    /// 重试配置
    pub retry_config: Option<RetryConfig>,
    /// 调用上下文
    pub context: Option<HashMap<String, serde_json::Value>>,
}

/// 工具调用结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 调用ID
    pub call_id: String,
    /// 工具名称
    pub tool_name: String,
    /// 调用状态
    pub status: ToolCallStatus,
    /// 结果数据
    pub result: serde_json::Value,
    /// 错误信息
    pub error: Option<String>,
    /// 执行时间（毫秒）
    pub duration_ms: u64,
    /// 开始时间
    pub started_at: SystemTime,
    /// 结束时间
    pub completed_at: SystemTime,
    /// 使用的资源
    pub resources_used: ResourceUsage,
    /// 工具输出日志
    pub logs: Vec<ToolLog>,
}

/// 工具调用状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolCallStatus {
    /// 成功
    Success,
    /// 失败
    Failed,
    /// 超时
    Timeout,
    /// 取消
    Cancelled,
    /// 部分成功
    PartialSuccess,
}

/// 资源使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU使用率
    pub cpu_percent: f64,
    /// 内存使用量（字节）
    pub memory_bytes: u64,
    /// 网络流量（字节）
    pub network_bytes: u64,
    /// 磁盘IO（字节）
    pub disk_io_bytes: u64,
}

/// 工具日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolLog {
    /// 日志级别
    pub level: LogLevel,
    /// 日志消息
    pub message: String,
    /// 时间戳
    pub timestamp: SystemTime,
    /// 来源
    pub source: String,
}

/// 日志级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// 工具缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    result: ToolResult,
    expires_at: SystemTime,
    access_count: u64,
    last_accessed: SystemTime,
}

/// 工具统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatistics {
    /// 总调用次数
    pub total_calls: u64,
    /// 成功调用次数
    pub successful_calls: u64,
    /// 失败调用次数
    pub failed_calls: u64,
    /// 平均执行时间（毫秒）
    pub avg_duration_ms: u64,
    /// 缓存命中率
    pub cache_hit_rate: f64,
    /// 各工具调用统计
    pub tool_stats: HashMap<String, ToolCallStats>,
    /// 错误统计
    pub error_stats: HashMap<String, u64>,
}

/// 单个工具调用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallStats {
    /// 调用次数
    pub call_count: u64,
    /// 成功次数
    pub success_count: u64,
    /// 失败次数
    pub failure_count: u64,
    /// 平均执行时间
    pub avg_duration_ms: u64,
    /// 最后调用时间
    pub last_called: Option<SystemTime>,
}

/// 工具接口管理器
#[derive(Debug)]
pub struct ToolInterface {
    config: ToolInterfaceConfig,
    tool_manager: Arc<RwLock<UnifiedToolManager>>,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    statistics: Arc<RwLock<ToolStatistics>>,
    active_calls: Arc<RwLock<HashMap<String, SystemTime>>>,
}

impl ToolInterface {
    /// 创建新的工具接口实例
    pub async fn new(db_service: Arc<DatabaseService>) -> Result<Self, PlanAndExecuteError> {
        let config = ToolInterfaceConfig::default();
        Self::with_config(config, db_service).await
    }

    /// 使用自定义配置创建工具接口
    pub async fn with_config(config: ToolInterfaceConfig, db_service: Arc<DatabaseService>) -> Result<Self, PlanAndExecuteError> {
        // 创建工具管理器配置
        let tool_config = ToolManagerConfig {
            max_concurrent_executions: config.max_concurrent_calls as usize,
            default_timeout: Duration::from_secs(config.default_timeout),
            log_executions: true,
        };
        
        // 创建统一工具管理器
        let mut tool_manager = UnifiedToolManager::new(tool_config);
        
        // 注册内置工具提供者
        let builtin_provider = Box::new(BuiltinToolProvider::new(db_service));
        tool_manager.register_provider(builtin_provider).await
            .map_err(|e| PlanAndExecuteError::ToolFailed(format!("注册内置工具失败: {}", e)))?;
        
        Ok(Self {
            config,
            tool_manager: Arc::new(RwLock::new(tool_manager)),
            cache: Arc::new(RwLock::new(HashMap::new())),
            statistics: Arc::new(RwLock::new(ToolStatistics::default())),
            active_calls: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 刷新工具提供者
    pub async fn refresh_providers(&self) -> Result<(), PlanAndExecuteError> {
        let mut tool_manager = self.tool_manager.write().await;
        tool_manager.refresh_all_providers().await
            .map_err(|e| PlanAndExecuteError::ToolFailed(format!("刷新工具提供者失败: {}", e)))
    }

    /// 获取可用工具列表
    pub async fn list_tools(&self) -> Vec<ToolDefinition> {
        let tool_manager = self.tool_manager.read().await;
        let tool_infos = tool_manager.list_tools().await;
        
        tool_infos.into_iter().map(|info| self.convert_tool_info_to_definition(info)).collect()
    }

    /// 获取工具定义
    pub async fn get_tool(&self, tool_name: &str) -> Option<ToolDefinition> {
        let tool_manager = self.tool_manager.read().await;
        let tool_infos = tool_manager.list_tools().await;
        
        tool_infos.into_iter()
            .find(|info| info.name == tool_name)
            .map(|info| self.convert_tool_info_to_definition(info))
    }
    
    /// 将 ToolInfo 转换为 ToolDefinition
    fn convert_tool_info_to_definition(&self, info: ToolInfo) -> ToolDefinition {
        ToolDefinition {
            id: info.name.clone(),
            name: info.name,
            description: info.description,
            tool_type: self.convert_category_to_tool_type(&info.category),
            version: info.metadata.version,
            input_schema: self.convert_parameters_to_schema(&info.parameters),
            output_schema: serde_json::json!({}), // 简化处理
            config: ToolConfig::default(),
            metadata: HashMap::new(), // 简化处理
            available: info.available,
            last_checked: SystemTime::now(),
        }
    }
    
    /// 转换工具分类
    fn convert_category_to_tool_type(&self, category: &UnifiedToolCategory) -> ToolType {
        match category {
            UnifiedToolCategory::NetworkScanning => ToolType::NetworkScanner,
            UnifiedToolCategory::VulnerabilityScanning => ToolType::VulnerabilityScanner,
            UnifiedToolCategory::ServiceDetection => ToolType::PortScanner,
            UnifiedToolCategory::CodeAnalysis => ToolType::DataAnalyzer,
            UnifiedToolCategory::DataProcessing => ToolType::DataAnalyzer,
            UnifiedToolCategory::SystemUtility => ToolType::SystemTool,
            UnifiedToolCategory::Custom(name) => ToolType::Custom(name.clone()),
        }
    }
    
    /// 转换参数定义为JSON Schema
    fn convert_parameters_to_schema(&self, params: &ToolParameters) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();
        
        for param in &params.parameters {
            if param.required {
                required.push(param.name.clone());
            }
            
            let param_type = match param.param_type {
                ParameterType::String => "string",
                ParameterType::Number => "number",
                ParameterType::Boolean => "boolean",
                ParameterType::Array => "array",
                ParameterType::Object => "object",
            };
            
            properties.insert(param.name.clone(), serde_json::json!({
                "type": param_type,
                "description": param.description
            }));
        }
        
        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required
        })
    }

    /// 调用工具
    pub async fn call_tool(&self, mut call: ToolCall) -> Result<ToolResult, PlanAndExecuteError> {
        debug!("调用工具: {} ({})", call.tool_name, call.id);
        
        // 检查并发限制
        self.check_concurrency_limit().await?;
        
        // 检查缓存
        if self.config.enable_caching {
            if let Some(cached_result) = self.get_cached_result(&call).await {
                debug!("使用缓存结果: {}", call.id);
                self.update_cache_statistics().await;
                return Ok(cached_result);
            }
        }
        
        // 设置默认值
        if call.timeout.is_none() {
            call.timeout = Some(self.config.default_timeout);
        }
        
        // 记录活跃调用
        {
            let mut active_calls = self.active_calls.write().await;
            active_calls.insert(call.id.clone(), SystemTime::now());
        }
        
        // 准备工具执行参数
        let execution_params = ToolExecutionParams {
            inputs: call.parameters.clone(),
            context: call.context.clone().unwrap_or_default(),
            timeout: call.timeout.map(Duration::from_secs),
            execution_id: Some(Uuid::parse_str(&call.id).unwrap_or_else(|_| Uuid::new_v4())),
        };
        
        // 执行工具调用
        let result = {
            let tool_manager = self.tool_manager.read().await;
            tool_manager.call_tool(&call.tool_name, execution_params).await
        };
        
        // 移除活跃调用记录
        {
            let mut active_calls = self.active_calls.write().await;
            active_calls.remove(&call.id);
        }
        
        // 处理结果
        match result {
            Ok(execution_result) => {
                let tool_result = self.convert_execution_result_to_tool_result(execution_result, &call);
                
                // 缓存结果
                if self.config.enable_caching && tool_result.status == ToolCallStatus::Success {
                    self.cache_result(&call, &tool_result).await;
                }
                
                // 更新统计信息
                self.update_statistics(&call, &tool_result).await;
                
                debug!("工具调用成功: {} ({}ms)", call.id, tool_result.duration_ms);
                Ok(tool_result)
            },
            Err(error) => {
                // 创建错误结果
                let error_result = ToolResult {
                    call_id: call.id.clone(),
                    tool_name: call.tool_name.clone(),
                    status: ToolCallStatus::Failed,
                    result: serde_json::json!({}),
                    error: Some(error.to_string()),
                    duration_ms: 0,
                    started_at: SystemTime::now(),
                    completed_at: SystemTime::now(),
                    resources_used: ResourceUsage::default(),
                    logs: Vec::new(),
                };
                
                // 更新统计信息
                self.update_statistics(&call, &error_result).await;
                
                error!("工具调用失败: {} - {}", call.id, error);
                Err(PlanAndExecuteError::ToolFailed(error.to_string()))
            }
        }
    }
    
    /// 将 ToolExecutionResult 转换为 ToolResult
    fn convert_execution_result_to_tool_result(&self, exec_result: ToolExecutionResult, call: &ToolCall) -> ToolResult {
        let status = if exec_result.success {
            ToolCallStatus::Success
        } else {
            ToolCallStatus::Failed
        };
        
        let started_at = exec_result.started_at.timestamp() as u64;
        let completed_at = exec_result.completed_at
            .map(|dt| dt.timestamp() as u64)
            .unwrap_or(started_at);
        
        ToolResult {
            call_id: call.id.clone(),
            tool_name: call.tool_name.clone(),
            status,
            result: exec_result.output,
            error: exec_result.error,
            duration_ms: exec_result.execution_time_ms,
            started_at: SystemTime::UNIX_EPOCH + Duration::from_secs(started_at),
            completed_at: SystemTime::UNIX_EPOCH + Duration::from_secs(completed_at),
            resources_used: ResourceUsage::default(), // 简化处理
            logs: Vec::new(), // 简化处理
        }
    }

    /// 批量调用工具
    pub async fn batch_call_tools(
        &self,
        calls: Vec<ToolCall>,
    ) -> Vec<Result<ToolResult, PlanAndExecuteError>> {
        let mut handles = Vec::new();
        
        for call in calls {
            let interface = self.clone();
            let handle = tokio::spawn(async move {
                interface.call_tool(call).await
            });
            handles.push(handle);
        }
        
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(join_error) => {
                    results.push(Err(PlanAndExecuteError::ToolFailed(
                        format!("批量调用任务失败: {}", join_error)
                    )));
                }
            }
        }
        
        results
    }

    /// 取消工具调用
    pub async fn cancel_call(&self, call_id: &str) -> Result<bool, PlanAndExecuteError> {
        let mut active_calls = self.active_calls.write().await;
        
        if active_calls.remove(call_id).is_some() {
            log::info!("取消工具调用: {}", call_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 获取活跃调用列表
    pub async fn get_active_calls(&self) -> Vec<String> {
        let active_calls = self.active_calls.read().await;
        active_calls.keys().cloned().collect()
    }

    /// 检查工具健康状态
    pub async fn check_tool_health(&self, tool_id: &str) -> Result<ToolHealthStatus, PlanAndExecuteError> {
        let tool_manager = self.tool_manager.read().await;
        let tools = tool_manager.list_tools().await;
        
        if let Some(tool) = tools.iter().find(|t| t.name == tool_id) {
            // 简化的健康检查
            let health_status = ToolHealthStatus {
                tool_id: tool_id.to_string(),
                is_healthy: tool.available,
                last_check: SystemTime::now(),
                response_time_ms: 50, // 模拟响应时间
                error_rate: 0.05,     // 模拟错误率
                availability: 0.99,   // 模拟可用性
            };
            
            Ok(health_status)
        } else {
            Err(PlanAndExecuteError::ToolNotFound(format!("工具 '{}' 未找到", tool_id)))
        }
    }

    /// 获取工具统计信息
    pub async fn get_statistics(&self) -> ToolStatistics {
        self.statistics.read().await.clone()
    }

    /// 清理缓存
    pub async fn clear_cache(&self) -> Result<u64, PlanAndExecuteError> {
        let mut cache = self.cache.write().await;
        let cleared_count = cache.len() as u64;
        cache.clear();
        
        log::info!("清理了 {} 个缓存条目", cleared_count);
        Ok(cleared_count)
    }

    /// 清理过期缓存
    pub async fn cleanup_expired_cache(&self) -> Result<u64, PlanAndExecuteError> {
        let mut cache = self.cache.write().await;
        let now = SystemTime::now();
        let initial_count = cache.len();
        
        cache.retain(|_, entry| now < entry.expires_at);
        
        let cleaned_count = initial_count - cache.len();
        
        if cleaned_count > 0 {
            log::info!("清理了 {} 个过期缓存条目", cleaned_count);
        }
        
        Ok(cleaned_count as u64)
    }

    /// 启动工具发现
    pub async fn start_tool_discovery(&self) -> Result<(), PlanAndExecuteError> {
        if !self.config.discovery_config.auto_discovery {
            return Ok(());
        }
        
        let interface = self.clone();
        let interval = Duration::from_secs(self.config.discovery_config.scan_interval_seconds);
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                
                if let Err(e) = interface.discover_tools().await {
                    log::error!("工具发现失败: {}", e);
                }
            }
        });
        
        log::info!("启动工具发现，扫描间隔: {} 秒", 
                  self.config.discovery_config.scan_interval_seconds);
        Ok(())
    }

    // 私有方法实现
    
    async fn check_concurrency_limit(&self) -> Result<(), PlanAndExecuteError> {
        let active_calls = self.active_calls.read().await;
        
        if active_calls.len() >= self.config.max_concurrent_calls as usize {
            return Err(PlanAndExecuteError::ResourceLimitExceeded(
                format!("并发调用数已达上限: {}", self.config.max_concurrent_calls)
            ));
        }
        
        Ok(())
    }

    async fn get_cached_result(&self, call: &ToolCall) -> Option<ToolResult> {
        let cache_key = self.generate_cache_key(call);
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.get_mut(&cache_key) {
            let now = SystemTime::now();
            
            if now < entry.expires_at {
                entry.access_count += 1;
                entry.last_accessed = now;
                return Some(entry.result.clone());
            } else {
                cache.remove(&cache_key);
            }
        }
        
        None
    }

    async fn cache_result(&self, call: &ToolCall, result: &ToolResult) {
        let cache_key = self.generate_cache_key(call);
        let expires_at = SystemTime::now() + Duration::from_secs(self.config.cache_expiry_seconds);
        
        let entry = CacheEntry {
            result: result.clone(),
            expires_at,
            access_count: 0,
            last_accessed: SystemTime::now(),
        };
        
        let mut cache = self.cache.write().await;
        cache.insert(cache_key, entry);
    }

    fn generate_cache_key(&self, call: &ToolCall) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        call.tool_name.hash(&mut hasher);
        
        // 对参数进行排序后哈希，确保相同参数的不同顺序产生相同的key
        let mut sorted_params: Vec<_> = call.parameters.iter().collect();
        sorted_params.sort_by_key(|(k, _)| *k);
        
        for (key, value) in sorted_params {
            key.hash(&mut hasher);
            value.to_string().hash(&mut hasher);
        }
        
        format!("{}_{:x}", call.tool_name, hasher.finish())
    }







    async fn update_statistics(&self, call: &ToolCall, result: &ToolResult) {
        let mut stats = self.statistics.write().await;
        
        stats.total_calls += 1;
        
        if result.status == ToolCallStatus::Success {
            stats.successful_calls += 1;
        } else {
            stats.failed_calls += 1;
            
            if let Some(ref error) = result.error {
                *stats.error_stats.entry(error.clone()).or_insert(0) += 1;
            }
        }
        
        // 更新平均执行时间
        stats.avg_duration_ms = 
            (stats.avg_duration_ms * (stats.total_calls - 1) + result.duration_ms) / stats.total_calls;
        
        // 更新工具特定统计
        let tool_stats = stats.tool_stats.entry(call.tool_name.clone()).or_insert(ToolCallStats {
            call_count: 0,
            success_count: 0,
            failure_count: 0,
            avg_duration_ms: 0,
            last_called: None,
        });
        
        tool_stats.call_count += 1;
        tool_stats.last_called = Some(SystemTime::now());
        
        if result.status == ToolCallStatus::Success {
            tool_stats.success_count += 1;
        } else {
            tool_stats.failure_count += 1;
        }
        
        tool_stats.avg_duration_ms = 
            (tool_stats.avg_duration_ms * (tool_stats.call_count - 1) + result.duration_ms) / tool_stats.call_count;
    }

    async fn update_cache_statistics(&self) {
        let mut stats = self.statistics.write().await;
        // 简化的缓存命中率计算
        stats.cache_hit_rate = (stats.cache_hit_rate * 0.9) + (1.0 * 0.1);
    }

    async fn discover_tools(&self) -> Result<(), PlanAndExecuteError> {
        // 简化的工具发现实现
        log::debug!("执行工具发现扫描");
        
        // 在实际实现中，这里会扫描指定路径，查找工具定义文件
        // 并自动注册发现的工具
        
        Ok(())
    }
}

// 实现Clone trait
impl Clone for ToolInterface {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            tool_manager: Arc::clone(&self.tool_manager),
            cache: Arc::clone(&self.cache),
            statistics: Arc::clone(&self.statistics),
            active_calls: Arc::clone(&self.active_calls),
        }
    }
}

// 辅助结构体

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolHealthStatus {
    pub tool_id: String,
    pub is_healthy: bool,
    pub last_check: SystemTime,
    pub response_time_ms: u64,
    pub error_rate: f64,
    pub availability: f64,
}

// 默认实现

impl Default for ToolInterfaceConfig {
    fn default() -> Self {
        Self {
            default_timeout: 300, // 5分钟
            max_concurrent_calls: 10,
            enable_caching: true,
            cache_expiry_seconds: 3600, // 1小时
            default_retry_config: RetryConfig::default(),
            discovery_config: ToolDiscoveryConfig::default(),
        }
    }
}

impl Default for ToolDiscoveryConfig {
    fn default() -> Self {
        Self {
            auto_discovery: false,
            registry_paths: vec!["/usr/local/bin".to_string(), "./tools".to_string()],
            scan_interval_seconds: 300, // 5分钟
            supported_types: vec![
                ToolType::NetworkScanner,
                ToolType::VulnerabilityScanner,
                ToolType::PortScanner,
                ToolType::WebCrawler,
            ],
        }
    }
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_bytes: 0,
            network_bytes: 0,
            disk_io_bytes: 0,
        }
    }
}

impl Default for ToolStatistics {
    fn default() -> Self {
        Self {
            total_calls: 0,
            successful_calls: 0,
            failed_calls: 0,
            avg_duration_ms: 0,
            cache_hit_rate: 0.0,
            tool_stats: HashMap::new(),
            error_stats: HashMap::new(),
        }
    }
}
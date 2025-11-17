//! 框架适配器实现
//! 
//! 为三个框架(Plan&Execute, ReWOO, LLM Compiler)提供统一的工具调用接口

use super::unified_types::*;
use super::UnifiedToolManager;
use anyhow::{anyhow, Result};
use async_trait::async_trait;

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, error, info};
use uuid::Uuid;

// ============================================================================
// 基础适配器实现
// ============================================================================

/// 基础框架适配器 - 提供通用功能实现
#[derive(Debug)]
pub struct BaseFrameworkAdapter {
    tool_manager: Arc<RwLock<UnifiedToolManager>>,
    config: AdapterConfig,
    cache: Arc<RwLock<HashMap<String, (UnifiedToolResult, Instant)>>>,
}

impl BaseFrameworkAdapter {
    pub fn new(tool_manager: Arc<RwLock<UnifiedToolManager>>, config: AdapterConfig) -> Self {
        Self {
            tool_manager,
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 执行单次工具调用（不做重试），供内部复用
    async fn execute_once(
        &self,
        call: &UnifiedToolCall,
        timeout_duration: Duration,
    ) -> Result<UnifiedToolResult> {
        let execution_params = ToolExecutionParams {
            inputs: call.parameters.clone(),
            context: call.context.clone(),
            timeout: Some(timeout_duration),
            execution_id: Some(
                Uuid::parse_str(&call.id).unwrap_or_else(|_| Uuid::new_v4()),
            ),
        };

        let manager = self.tool_manager.read().await;

        match timeout(timeout_duration, manager.call_tool(&call.tool_name, execution_params)).await {
            Ok(Ok(tool_result)) => Ok(UnifiedToolResult {
                id: call.id.clone(),
                tool_name: call.tool_name.clone(),
                success: tool_result.success,
                output: tool_result.output,
                error: tool_result.error,
                execution_time_ms: tool_result.execution_time_ms,
                metadata: tool_result.metadata,
            }),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                let timeout_error =
                    anyhow!("Tool execution timeout after {:?}", timeout_duration);
                error!(
                    "Tool execution timeout (no retry path): {}",
                    call.tool_name
                );
                Err(timeout_error)
            }
        }
    }

    /// 执行工具调用的核心逻辑
    async fn execute_tool_internal(&self, call: UnifiedToolCall) -> Result<UnifiedToolResult> {
        let _start_time = Instant::now();
        
        // 检查缓存
        if self.config.cache_enabled {
            let cache_key = self.build_cache_key(&call);
            if let Some(cached_result) = self.get_cached_result(&cache_key).await {
                debug!("Using cached result for tool: {}", call.tool_name);
                return Ok(cached_result);
            }
        }
        
        // 执行工具调用
        let timeout_duration = call.timeout.unwrap_or(self.config.default_timeout);

        // 对于 generate_advanced_plugin 这种重型长耗时工具：
        // - 使用更长的超时时间（例如 15 分钟）
        // - 不做自动重试，避免重复生成插件
        let result = if call.tool_name == "generate_advanced_plugin" {
            let long_timeout = Duration::from_secs(1800);
            info!(
                "Executing heavy tool '{}' with long timeout {:?} and no retries",
                call.tool_name, long_timeout
            );
            self.execute_once(&call, long_timeout).await?
        } else {
            self.execute_with_retry(call.clone(), timeout_duration).await?
        };
        
        // 缓存结果
        if self.config.cache_enabled && result.success {
            let cache_key = self.build_cache_key(&call);
            self.cache_result(cache_key, result.clone()).await;
        }
        
        Ok(result)
    }

    /// 带重试的执行逻辑
    async fn execute_with_retry(&self, call: UnifiedToolCall, timeout_duration: Duration) -> Result<UnifiedToolResult> {
        let mut last_error = None;
        
        for attempt in 0..=self.config.retry_policy.max_retries {
            if attempt > 0 {
                let delay = self.calculate_retry_delay(attempt);
                tokio::time::sleep(Duration::from_millis(delay)).await;
                info!("Retrying tool execution: {} (attempt {})", call.tool_name, attempt + 1);
            }
            
            let execution_params = ToolExecutionParams {
                inputs: call.parameters.clone(),
                context: call.context.clone(),
                timeout: Some(timeout_duration),
                execution_id: Some(Uuid::parse_str(&call.id).unwrap_or_else(|_| Uuid::new_v4())),
            };
            
            let manager = self.tool_manager.read().await;
            
            match timeout(timeout_duration, manager.call_tool(&call.tool_name, execution_params)).await {
                Ok(Ok(tool_result)) => {
                    return Ok(UnifiedToolResult {
                        id: call.id.clone(),
                        tool_name: call.tool_name.clone(),
                        success: tool_result.success,
                        output: tool_result.output,
                        error: tool_result.error,
                        execution_time_ms: tool_result.execution_time_ms,
                        metadata: tool_result.metadata,
                    });
                }
                Ok(Err(e)) => {
                    error!("Tool execution failed: {} (attempt {}): {}", call.tool_name, attempt + 1, e);
                    last_error = Some(e);
                }
                Err(_) => {
                    let timeout_error = anyhow!("Tool execution timeout after {:?}", timeout_duration);
                    error!("Tool execution timeout: {} (attempt {})", call.tool_name, attempt + 1);
                    last_error = Some(timeout_error);
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow!("Tool execution failed after all retries")))
    }

    /// 计算重试延迟
    fn calculate_retry_delay(&self, attempt: u32) -> u64 {
        let base_delay = self.config.retry_policy.base_delay_ms;
        let multiplier = self.config.retry_policy.backoff_multiplier;
        let max_delay = self.config.retry_policy.max_delay_ms;
        
        let delay = (base_delay as f64 * multiplier.powi(attempt as i32 - 1)) as u64;
        delay.min(max_delay)
    }

    /// 构建缓存键
    fn build_cache_key(&self, call: &UnifiedToolCall) -> String {
        format!("{}:{}:{}", call.tool_name, 
                serde_json::to_string(&call.parameters).unwrap_or_default(), 
                call.context.get("cache_version").unwrap_or(&Value::Null))
    }

    /// 获取缓存结果
    async fn get_cached_result(&self, cache_key: &str) -> Option<UnifiedToolResult> {
        let cache = self.cache.read().await;
        if let Some((result, timestamp)) = cache.get(cache_key) {
            // 检查缓存是否过期 (5分钟)
            if timestamp.elapsed() < Duration::from_secs(300) {
                return Some(result.clone());
            }
        }
        None
    }

    /// 缓存结果
    async fn cache_result(&self, cache_key: String, result: UnifiedToolResult) {
        let mut cache = self.cache.write().await;
        cache.insert(cache_key, (result, Instant::now()));
        
        // 限制缓存大小
        if cache.len() > 1000 {
            // 清理过期条目
            let cutoff = Instant::now() - Duration::from_secs(300);
            cache.retain(|_, (_, timestamp)| *timestamp > cutoff);
        }
    }
}

#[async_trait]
impl FrameworkToolAdapter for BaseFrameworkAdapter {
    fn adapter_name(&self) -> &str {
        "BaseFrameworkAdapter"
    }

    async fn execute_tool(&self, call: UnifiedToolCall) -> Result<UnifiedToolResult> {
        self.execute_tool_internal(call).await
    }

    async fn execute_tools_batch(&self, calls: Vec<UnifiedToolCall>) -> Vec<Result<UnifiedToolResult>> {
        if calls.is_empty() {
            return Vec::new();
        }
        
        info!("Executing batch of {} tools", calls.len());
        
        // 控制并发数量
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent_calls));
        let tasks: Vec<_> = calls.into_iter().map(|call| {
            let semaphore = semaphore.clone();
            let adapter = self;
            
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                adapter.execute_tool_internal(call).await
            }
        }).collect();
        
        let results = futures::future::join_all(tasks).await;
        info!("Batch execution completed with {} results", results.len());
        results
    }

    async fn list_available_tools(&self) -> Vec<String> {
        let manager = self.tool_manager.read().await;
        let tools = manager.list_tools().await;
        tools.into_iter()
            .filter(|tool| tool.available)
            .map(|tool| tool.name)
            .collect()
    }

    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        let manager = self.tool_manager.read().await;
        let tools = manager.list_tools().await;
        tools.into_iter().find(|tool| tool.name == tool_name)
    }

    async fn is_tool_available(&self, tool_name: &str) -> bool {
        let available_tools = self.list_available_tools().await;
        available_tools.contains(&tool_name.to_string())
    }

    async fn validate_tool_call(&self, tool_name: &str, call: &UnifiedToolCall) -> Result<()> {
        if !self.is_tool_available(tool_name).await {
            return Err(anyhow!("Tool '{}' is not available", tool_name));
        }
        
        if let Some(tool_info) = self.get_tool_info(tool_name).await {
            for param in &tool_info.parameters.parameters {
                if param.required && !call.parameters.contains_key(&param.name) {
                    return Err(anyhow!(
                        "Missing required parameter '{}' for tool '{}'", 
                        param.name, tool_name
                    ));
                }
            }
        }
        
        Ok(())
    }
}

// ============================================================================
// Plan & Execute 框架适配器
// ============================================================================

/// Plan & Execute 框架专用适配器
#[derive(Debug)]
pub struct PlanAndExecuteAdapter {
    base: BaseFrameworkAdapter,
}

impl PlanAndExecuteAdapter {
    pub fn new(tool_manager: Arc<RwLock<UnifiedToolManager>>) -> Self {
        let config = AdapterConfig {
            framework_type: FrameworkType::PlanAndExecute,
            cache_enabled: true,
            max_concurrent_calls: 5, // 偏保守的并发数
            default_timeout: Duration::from_secs(300),
            retry_policy: RetryPolicy {
                max_retries: 2,
                base_delay_ms: 2000,
                max_delay_ms: 10000,
                backoff_multiplier: 2.0,
            },
        };
        
        Self {
            base: BaseFrameworkAdapter::new(tool_manager, config),
        }
    }
}

#[async_trait]
impl FrameworkToolAdapter for PlanAndExecuteAdapter {
    fn adapter_name(&self) -> &str {
        "PlanAndExecuteAdapter"
    }

    async fn execute_tool(&self, call: UnifiedToolCall) -> Result<UnifiedToolResult> {
        debug!("Plan & Execute: Executing tool {}", call.tool_name);
        self.base.execute_tool(call).await
    }

    async fn execute_tools_batch(&self, calls: Vec<UnifiedToolCall>) -> Vec<Result<UnifiedToolResult>> {
        // Plan & Execute 通常顺序执行步骤
        let mut results = Vec::new();
        for call in calls {
            let result = self.execute_tool(call).await;
            results.push(result);
        }
        results
    }

    async fn list_available_tools(&self) -> Vec<String> {
        self.base.list_available_tools().await
    }

    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        self.base.get_tool_info(tool_name).await
    }

    async fn is_tool_available(&self, tool_name: &str) -> bool {
        self.base.is_tool_available(tool_name).await
    }

    async fn validate_tool_call(&self, tool_name: &str, call: &UnifiedToolCall) -> Result<()> {
        self.base.validate_tool_call(tool_name, call).await
    }
}

// ============================================================================
// ReWOO 框架适配器
// ============================================================================

/// ReWOO 框架专用适配器
#[derive(Debug)]
pub struct ReWOOAdapter {
    base: BaseFrameworkAdapter,
}

impl ReWOOAdapter {
    pub fn new(tool_manager: Arc<RwLock<UnifiedToolManager>>) -> Self {
        let config = AdapterConfig {
            framework_type: FrameworkType::ReWOO,
            cache_enabled: true,
            max_concurrent_calls: 3, // ReWOO通常顺序执行
            default_timeout: Duration::from_secs(180),
            retry_policy: RetryPolicy {
                max_retries: 1,
                base_delay_ms: 1000,
                max_delay_ms: 5000,
                backoff_multiplier: 1.5,
            },
        };
        
        Self {
            base: BaseFrameworkAdapter::new(tool_manager, config),
        }
    }
    
    /// ReWOO特有的参数替换逻辑
    pub fn substitute_variables(&self, args: &str, variables: &HashMap<String, Value>) -> String {
        let mut result = args.to_string();
        for (key, value) in variables {
            let placeholder = format!("#{}", key);
            let replacement = match value {
                Value::String(s) => s.clone(),
                _ => value.to_string(),
            };
            result = result.replace(&placeholder, &replacement);
        }
        result
    }
}

#[async_trait]
impl FrameworkToolAdapter for ReWOOAdapter {
    fn adapter_name(&self) -> &str {
        "ReWOOAdapter"
    }

    async fn execute_tool(&self, mut call: UnifiedToolCall) -> Result<UnifiedToolResult> {
        debug!("ReWOO: Executing tool {}", call.tool_name);
        
        // ReWOO特有的变量替换
        if let Some(variables) = call.context.get("variables") {
            if let Value::Object(var_map) = variables {
                let substituted_params = call.parameters.iter()
                    .map(|(k, v)| {
                        let substituted_value = if let Value::String(s) = v {
                            Value::String(self.substitute_variables(s, &var_map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()))
                        } else {
                            v.clone()
                        };
                        (k.clone(), substituted_value)
                    })
                    .collect();
                call.parameters = substituted_params;
            }
        }
        
        self.base.execute_tool(call).await
    }

    async fn execute_tools_batch(&self, calls: Vec<UnifiedToolCall>) -> Vec<Result<UnifiedToolResult>> {
        // ReWOO 通常顺序执行，支持变量传递
        let mut results = Vec::new();
        let mut variables: HashMap<String, Value> = HashMap::new();
        
        for mut call in calls {
            // 传递之前步骤的变量
            call.context.insert("variables".to_string(), serde_json::to_value(&variables).unwrap());
            
            let result = self.execute_tool(call.clone()).await;
            
            // 更新变量
            if let Ok(ref tool_result) = result {
                variables.insert(format!("#{}", call.tool_name), tool_result.output.clone());
            }
            
            results.push(result);
        }
        results
    }

    async fn list_available_tools(&self) -> Vec<String> {
        self.base.list_available_tools().await
    }

    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        self.base.get_tool_info(tool_name).await
    }

    async fn is_tool_available(&self, tool_name: &str) -> bool {
        self.base.is_tool_available(tool_name).await
    }

    async fn validate_tool_call(&self, tool_name: &str, call: &UnifiedToolCall) -> Result<()> {
        self.base.validate_tool_call(tool_name, call).await
    }
}

// ============================================================================
// LLM Compiler 框架适配器
// ============================================================================

/// LLM Compiler 框架专用适配器
#[derive(Debug)]
pub struct LLMCompilerAdapter {
    base: BaseFrameworkAdapter,
}

impl LLMCompilerAdapter {
    pub fn new(tool_manager: Arc<RwLock<UnifiedToolManager>>) -> Self {
        let config = AdapterConfig {
            framework_type: FrameworkType::LLMCompiler,
            cache_enabled: true,
            max_concurrent_calls: 10, // 支持高并发
            default_timeout: Duration::from_secs(120),
            retry_policy: RetryPolicy {
                max_retries: 3,
                base_delay_ms: 500,
                max_delay_ms: 8000,
                backoff_multiplier: 2.0,
            },
        };
        
        Self {
            base: BaseFrameworkAdapter::new(tool_manager, config),
        }
    }
}

#[async_trait]
impl FrameworkToolAdapter for LLMCompilerAdapter {
    fn adapter_name(&self) -> &str {
        "LLMCompilerAdapter"
    }

    async fn execute_tool(&self, call: UnifiedToolCall) -> Result<UnifiedToolResult> {
        debug!("LLM Compiler: Executing tool {}", call.tool_name);
        self.base.execute_tool(call).await
    }

    async fn execute_tools_batch(&self, calls: Vec<UnifiedToolCall>) -> Vec<Result<UnifiedToolResult>> {
        // LLM Compiler 支持高并发批量执行
        self.base.execute_tools_batch(calls).await
    }

    async fn list_available_tools(&self) -> Vec<String> {
        self.base.list_available_tools().await
    }

    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        self.base.get_tool_info(tool_name).await
    }

    async fn is_tool_available(&self, tool_name: &str) -> bool {
        self.base.is_tool_available(tool_name).await
    }

    async fn validate_tool_call(&self, tool_name: &str, call: &UnifiedToolCall) -> Result<()> {
        self.base.validate_tool_call(tool_name, call).await
    }
}

/// EngineToolAdapter兼容包装器 (专为LLM Compiler)
#[async_trait]
impl EngineToolAdapter for LLMCompilerAdapter {
    async fn execute_tool(&self, call: UnifiedToolCall) -> Result<UnifiedToolResult> {
        FrameworkToolAdapter::execute_tool(self, call).await
    }

    async fn list_available_tools(&self) -> Vec<String> {
        FrameworkToolAdapter::list_available_tools(self).await
    }

    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        FrameworkToolAdapter::get_tool_info(self, tool_name).await
    }
}

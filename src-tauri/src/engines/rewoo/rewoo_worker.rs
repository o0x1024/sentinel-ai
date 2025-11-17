//! ReWOO Worker 实现
//! 
//! 基于 LangGraph ReWOO 标准实现的 Worker 模块
//! 负责执行具体的工具调用
use super::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use tokio::time::timeout;
use uuid::Uuid;
use tracing::{info, warn, error};
use serde_json::Value;

/// ReWOO Worker - 负责执行工具调用
#[derive(Debug)]
pub struct ReWOOWorker {
    /// 框架适配器
    framework_adapter: Arc<dyn crate::tools::FrameworkToolAdapter>,
    /// 配置
    config: WorkerConfig,
    /// 运行时参数（包含工具权限等）
    runtime_params: Option<HashMap<String, serde_json::Value>>,
}

impl ReWOOWorker {
    /// 创建新的 Worker
    pub fn new(
        framework_adapter: Arc<dyn crate::tools::FrameworkToolAdapter>,
        config: WorkerConfig,
    ) -> Self {
        Self {
            framework_adapter,
            config,
            runtime_params: None,
        }
    }
    
    /// 设置运行时参数
    pub fn set_runtime_params(&mut self, params: HashMap<String, serde_json::Value>) {
        self.runtime_params = Some(params);
    }
    
    /// 使用全局框架适配器创建Worker
    pub async fn new_with_global_adapter(config: WorkerConfig) -> Result<Self, ReWOOError> {
        let framework_adapter = crate::tools::get_framework_adapter(crate::tools::FrameworkType::ReWOO).await
            .map_err(|e| ReWOOError::ToolSystemError(format!("获取ReWOO框架适配器失败: {}", e)))?;
        
        Ok(Self {
            framework_adapter,
            config,
            runtime_params: None,
        })
    }
    
    /// 执行工具调用
    pub async fn execute_tool(
        &self,
        step: &PlanStep,
        substituted_args: &str,
    ) -> Result<ToolResult, ReWOOError> {
        let start_time = SystemTime::now();
        
        info!("step: {:?}", step);

        info!("Executing tool '{}' with args: {}", step.tool, substituted_args);
        
        // 工具权限检查（从runtime_params读取）
        if let Some(params) = &self.runtime_params {
            let allow_list = params
                .get("tools_allow")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            let deny_list = params
                .get("tools_deny")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            
            // 如果没有白名单（空数组），则不允许任何工具
            if allow_list.is_empty() {
                return Err(ReWOOError::ToolExecutionError(format!(
                    "工具 '{}' 不在允许列表中（未配置工具权限）", step.tool
                )));
            }
            // 如果有白名单且工具不在白名单中，拒绝
            if !allow_list.iter().any(|&n| n == step.tool) {
                return Err(ReWOOError::ToolExecutionError(format!(
                    "工具 '{}' 不在允许列表中", step.tool
                )));
            }
            // 如果工具在黑名单中，拒绝
            if deny_list.iter().any(|&n| n == step.tool) {
                return Err(ReWOOError::ToolExecutionError(format!(
                    "工具 '{}' 被禁止使用", step.tool
                )));
            }
        }
        
        // 构建工具调用
        let tool_call = self.build_unified_tool_call(step, substituted_args).await?;
        
        tracing::info!("tool_call: {:?}", tool_call);
        // 验证工具调用
        if let Err(e) = self.framework_adapter.validate_tool_call(&step.tool, &tool_call).await {
            warn!("Tool call validation failed: {}", e);
            return Err(ReWOOError::ToolExecutionError(e.to_string()));
        }
        
        // 执行工具调用（带超时）
        let execution_result = match timeout(
            Duration::from_secs(self.config.timeout_seconds),
            self.framework_adapter.execute_tool(tool_call)
        ).await {
            Ok(result) => result,
            Err(_) => {
                error!("Tool execution timeout after {} seconds", self.config.timeout_seconds);
                return Err(ReWOOError::ToolExecutionError(
                    format!("Tool execution timeout after {} seconds", self.config.timeout_seconds)
                ));
            }
        };
        
        // 处理执行结果
        let tool_result = match execution_result {
            Ok(result) => {
                if result.success {
                    info!("Tool '{}' executed successfully", step.tool);
                } else {
                    warn!("Tool '{}' execution failed: {:?}", step.tool, result.error);
                }
                // 转换UnifiedToolResult为ToolResult，保留结构化输出
                ToolResult {
                    success: result.success,
                    content: result.output.to_string(), // 保持字符串兼容性
                    json_content: Some(result.output), // 新增 JSON 字段
                    error: result.error,
                    execution_time_ms: result.execution_time_ms,
                }
            },
            Err(e) => {
                error!("Tool '{}' execution error: {}", step.tool, e);
                ToolResult {
                    success: false,
                    content: String::new(),
                    json_content: None,
                    error: Some(e.to_string()),
                    execution_time_ms: start_time.elapsed().unwrap_or(Duration::from_secs(0)).as_millis() as u64,
                }
            }
        };
        
        Ok(tool_result)
    }
    
    /// 构建工具调用 (保留用于兼容性)
    async fn build_tool_execution_params(
        &self,
        step: &PlanStep,
        substituted_args: &str,
    ) -> Result<crate::tools::ToolExecutionParams, ReWOOError> {
        // 解析工具参数（结合工具定义做智能映射）
        let args = self.parse_tool_args_with_tool(&step.tool, substituted_args).await?;
        
        Ok(crate::tools::ToolExecutionParams {
            inputs: args,
            context: std::collections::HashMap::new(),
            timeout: Some(std::time::Duration::from_secs(self.config.timeout_seconds)),
            execution_id: None,
        })
    }
    
    /// 构建统一工具调用 (新的框架适配器接口)
    async fn build_unified_tool_call(
        &self,
        step: &PlanStep,
        substituted_args: &str,
    ) -> Result<crate::tools::UnifiedToolCall, ReWOOError> {
        // 解析工具参数
        let args = self.parse_tool_args_with_tool(&step.tool, substituted_args).await?;
        
        Ok(crate::tools::UnifiedToolCall {
            id: Uuid::new_v4().to_string(),
            tool_name: step.tool.clone(),
            parameters: args,
            timeout: Some(std::time::Duration::from_secs(self.config.timeout_seconds)),
            context: std::collections::HashMap::new(),
            retry_count: 0,
        })
    }
    
    /// 解析工具参数 - 升级为 schema 驱动版本
    async fn parse_tool_args_with_tool(&self, tool_name: &str, args_str: &str) -> Result<HashMap<String, Value>, ReWOOError> {
        // 第一步：基础解析
        let basic_args = self.parse_tool_args_basic(args_str).await?;
        
        // 第二步：schema 验证和规范化
        let validation_result = self.validate_and_normalize_params(tool_name, basic_args).await?;
        
        if !validation_result.is_valid {
            return Err(ReWOOError::ToolExecutionError(
                format!("Parameter validation failed for tool '{}': {}", 
                    tool_name, 
                    validation_result.errors.join("; ")
                )
            ));
        }
        
        // 记录警告和应用的默认值
        if !validation_result.warnings.is_empty() {
            log::warn!("Parameter warnings for tool '{}': {}", tool_name, validation_result.warnings.join("; "));
        }
        if !validation_result.applied_defaults.is_empty() {
            log::info!("Applied default values for tool '{}': {}", tool_name, validation_result.applied_defaults.join(", "));
        }
        
        Ok(validation_result.normalized_params)
    }
    
    /// 基础参数解析（原有逻辑）
    async fn parse_tool_args_basic(&self, args_str: &str) -> Result<HashMap<String, Value>, ReWOOError> {
        let mut args = HashMap::new();
        
        // 规范化辅助：去除首尾引号
        fn normalize_str_token(s: &str) -> String {
            let trimmed = s.trim();
            if (trimmed.starts_with('"') && trimmed.ends_with('"')) || (trimmed.starts_with('\'') && trimmed.ends_with('\'')) {
                trimmed[1..trimmed.len().saturating_sub(1)].to_string()
            } else {
                trimmed.to_string()
            }
        }

        // 尝试解析为 JSON
        if let Ok(json_value) = serde_json::from_str::<Value>(args_str) {
            if let Some(obj) = json_value.as_object() {
                for (key, value) in obj {
                    let v = match value {
                        Value::String(s) => Value::String(normalize_str_token(s)),
                        other => other.clone(),
                    };
                    args.insert(key.clone(), v);
                }
                return Ok(args);
            }
        }
        
        // 尝试解析为 key=value 或 key: value 列表，逗号分隔
        let mut parsed_any_kv = false;
        for part in args_str.split(',') {
            let token = part.trim();
            if token.is_empty() { continue; }
            if let Some((k, v)) = token.split_once('=')
                .or_else(|| token.split_once(':'))
            {
                let key = k.trim();
                let val_str = v.trim();
                let val_str_norm = normalize_str_token(val_str);
                // 尝试将布尔/数字解析
                let val = if let Ok(b) = val_str_norm.parse::<bool>() {
                    Value::Bool(b)
                } else if let Ok(n) = val_str_norm.parse::<f64>() {
                    serde_json::Number::from_f64(n)
                        .map(Value::Number)
                        .unwrap_or(Value::String(val_str_norm))
                } else {
                    Value::String(val_str_norm)
                };
                args.insert(key.to_string(), val);
                parsed_any_kv = true;
            }
        }
        if parsed_any_kv {
            return Ok(args);
        }

        // 最后回退到简单的字符串参数
        args.insert("input".to_string(), Value::String(normalize_str_token(args_str)));
        
        Ok(args)
    }
    
    /// Schema 驱动的参数验证和规范化
    async fn validate_and_normalize_params(
        &self, 
        tool_name: &str, 
        raw_params: HashMap<String, Value>
    ) -> Result<ParameterValidationResult, ReWOOError> {
        let mut result = ParameterValidationResult::default();
        // 先拷贝一份可变参数，便于做预处理归一化
        let mut preprocessed_params = raw_params;
        
        // 针对已知工具做必要的参数归一化，避免上游包装结构导致的类型/字段不匹配
        // 场景：analyze_website 的输出通常为 { "analysis": {...}, "summary": "..." }
        // 而 generate_advanced_plugin 期望的 "analysis" 是内部的对象本身
        if tool_name == "generate_advanced_plugin" {
            // 解包嵌套的 analysis: { analysis: {...}, summary: ... }
            if let Some(val) = preprocessed_params.get("analysis").cloned() {
                if let Value::Object(obj) = val {
                    if let Some(inner) = obj.get("analysis") {
                        if inner.is_object() {
                            preprocessed_params.insert("analysis".to_string(), inner.clone());
                        }
                    }
                }
            }
            // 兼容常见别名（如果缺少 vuln_types，尝试从其它字段映射）
            if !preprocessed_params.contains_key("vuln_types") {
                if let Some(v) = preprocessed_params
                    .get("vulnTypes")
                    .or_else(|| preprocessed_params.get("vulnerabilities"))
                    .or_else(|| preprocessed_params.get("types"))
                    .cloned()
                {
                    preprocessed_params.insert("vuln_types".to_string(), v);
                }
            }
        }
        
        // 获取工具的参数定义
        let tool_info = match self.framework_adapter.get_tool_info(tool_name).await {
            Some(info) => info,
            None => {
                // 如果没有工具信息，直接返回原始参数
                result.normalized_params = preprocessed_params;
                result.warnings.push(format!("No schema found for tool '{}'", tool_name));
                return Ok(result);
            }
        };
        
        let param_definitions = &tool_info.parameters.parameters;
        
        // 验证必填参数
        for param_def in param_definitions {
            if param_def.required && !preprocessed_params.contains_key(&param_def.name) {
                // 尝试应用默认值
                if let Some(default_value) = &param_def.default_value {
                    result.normalized_params.insert(param_def.name.clone(), default_value.clone());
                    result.applied_defaults.push(param_def.name.clone());
                } else {
                    result.errors.push(format!("Missing required parameter: {}", param_def.name));
                    result.is_valid = false;
                }
            }
        }
        
        // 验证和转换每个提供的参数
        for (param_name, param_value) in preprocessed_params {
            if let Some(param_def) = param_definitions.iter().find(|p| p.name == param_name) {
                // 类型验证和转换
                match self.validate_and_convert_param_type(param_def, &param_value) {
                    Ok(converted_value) => {
                        result.normalized_params.insert(param_name.clone(), converted_value);
                    }
                    Err(error) => {
                        result.errors.push(format!("Parameter '{}': {}", param_name, error));
                        result.is_valid = false;
                    }
                }
            } else {
                // 未知参数，发出警告但保留
                result.warnings.push(format!("Unknown parameter: {}", param_name));
                result.normalized_params.insert(param_name, param_value);
            }
        }
        
        // 为缺失的可选参数应用默认值
        for param_def in param_definitions {
            if !param_def.required && 
               !result.normalized_params.contains_key(&param_def.name) {
                if let Some(default_value) = &param_def.default_value {
                    result.normalized_params.insert(param_def.name.clone(), default_value.clone());
                    result.applied_defaults.push(param_def.name.clone());
                }
            }
        }
        
        Ok(result)
    }
    
    /// 验证和转换单个参数类型
    fn validate_and_convert_param_type(
        &self,
        param_def: &crate::tools::ParameterDefinition,
        value: &Value,
    ) -> Result<Value, String> {
        use crate::tools::ParameterType;
        
        match param_def.param_type {
            ParameterType::String => {
                match value {
                    Value::String(_) => Ok(value.clone()),
                    Value::Number(n) => Ok(Value::String(n.to_string())),
                    Value::Bool(b) => Ok(Value::String(b.to_string())),
                    _ => Ok(Value::String(value.to_string())),
                }
            }
            ParameterType::Number => {
                match value {
                    Value::Number(_) => Ok(value.clone()),
                    Value::String(s) => {
                        if let Ok(n) = s.parse::<f64>() {
                            Ok(serde_json::Number::from_f64(n)
                                .map(Value::Number)
                                .unwrap_or(value.clone()))
                        } else {
                            Err(format!("Cannot convert '{}' to number", s))
                        }
                    }
                    _ => Err(format!("Cannot convert {:?} to number", value)),
                }
            }
            ParameterType::Boolean => {
                match value {
                    Value::Bool(_) => Ok(value.clone()),
                    Value::String(s) => {
                        match s.to_lowercase().as_str() {
                            "true" | "1" | "yes" | "on" => Ok(Value::Bool(true)),
                            "false" | "0" | "no" | "off" => Ok(Value::Bool(false)),
                            _ => Err(format!("Cannot convert '{}' to boolean", s)),
                        }
                    }
                    Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            Ok(Value::Bool(i != 0))
                        } else {
                            Err(format!("Cannot convert number to boolean"))
                        }
                    }
                    _ => Err(format!("Cannot convert {:?} to boolean", value)),
                }
            }
            ParameterType::Array => {
                match value {
                    Value::Array(_) => Ok(value.clone()),
                    Value::String(s) => {
                        // 尝试解析为JSON数组或逗号分隔的字符串
                        if let Ok(arr) = serde_json::from_str::<Value>(s) {
                            if arr.is_array() {
                                Ok(arr)
                            } else {
                                Ok(Value::Array(vec![Value::String(s.clone())]))
                            }
                        } else {
                            // 逗号分隔
                            let items: Vec<Value> = s.split(',')
                                .map(|item| Value::String(item.trim().to_string()))
                                .collect();
                            Ok(Value::Array(items))
                        }
                    }
                    _ => Ok(Value::Array(vec![value.clone()])),
                }
            }
            ParameterType::Object => {
                match value {
                    Value::Object(_) => Ok(value.clone()),
                    Value::String(s) => {
                        if let Ok(obj) = serde_json::from_str::<Value>(s) {
                            if obj.is_object() {
                                Ok(obj)
                            } else {
                                Err(format!("String '{}' is not a valid JSON object", s))
                            }
                        } else {
                            Err(format!("Cannot parse '{}' as JSON object", s))
                        }
                    }
                    _ => Err(format!("Cannot convert {:?} to object", value)),
                }
            }
        }
    }
    
    /// 验证工具是否可用
    pub async fn is_tool_available(&self, tool_name: &str) -> bool {
        self.framework_adapter.is_tool_available(tool_name).await
    }
    
    /// 获取可用工具列表
    pub async fn get_available_tools(&self) -> Vec<String> {
        self.framework_adapter.list_available_tools().await
    }
    
    /// 验证步骤是否可执行
    pub async fn validate_step(&self, step: &PlanStep) -> Result<(), ReWOOError> {
        // 检查工具是否可用
        if !self.is_tool_available(&step.tool).await {
            return Err(ReWOOError::ToolExecutionError(
                format!("Tool '{}' is not available", step.tool)
            ));
        }
        
        // 检查参数是否为空
        if step.args.trim().is_empty() {
            return Err(ReWOOError::ToolExecutionError(
                format!("Empty arguments for tool '{}'", step.tool)
            ));
        }
        
        Ok(())
    }
    
    /// 执行多个步骤（并行执行）
    pub async fn execute_steps_parallel(
        &self,
        steps: Vec<(PlanStep, String)>, // (step, substituted_args)
    ) -> Vec<Result<ToolResult, ReWOOError>> {
        let mut handles = Vec::new();
        
        for (step, substituted_args) in steps {
            let worker = self.clone();
            let handle = tokio::spawn(async move {
                worker.execute_tool(&step, &substituted_args).await
            });
            handles.push(handle);
        }
        
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(ReWOOError::ToolExecutionError(
                    format!("Task join error: {}", e)
                ))),
            }
        }
        
        results
    }
    
    /// 智能重试执行工具 - 支持错误分级策略
    pub async fn execute_tool_with_retry(
        &self,
        step: &PlanStep,
        substituted_args: &str,
        max_retries: u32,
    ) -> Result<ToolResult, ReWOOError> {
        let mut last_error: Option<String>;
        let mut retry_count = 0;
        
        loop {
            match self.execute_tool(step, substituted_args).await {
                Ok(result) => {
                    if result.success {
                        return Ok(result);
                    } else {
                        last_error = result.error.clone();
                        
                        // 分析错误并决定重试策略
                        if let Some(error_msg) = &result.error {
                            let error_analysis = self.analyze_error(error_msg);
                            
                            if !error_analysis.should_retry || retry_count >= max_retries {
                                break;
                            }
                            
                            // 应用智能重试策略
                            let delay = self.calculate_retry_delay(&error_analysis.retry_strategy, retry_count);
                            log::info!("Retrying tool '{}' after {}ms due to {} error (attempt {}/{})", 
                                step.tool, delay, error_analysis.analysis, retry_count + 1, max_retries);
                            
                            tokio::time::sleep(Duration::from_millis(delay)).await;
                            retry_count += 1;
                        } else {
                            break;
                        }
                    }
                },
                Err(e) => {
                    last_error = Some(e.to_string());
                    
                    // 分析错误并决定重试策略
                    let error_analysis = self.analyze_error(&e.to_string());
                    
                    if !error_analysis.should_retry || retry_count >= max_retries {
                        break;
                    }
                    
                    // 应用智能重试策略
                    let delay = self.calculate_retry_delay(&error_analysis.retry_strategy, retry_count);
                    log::info!("Retrying tool '{}' after {}ms due to {} error (attempt {}/{})", 
                        step.tool, delay, error_analysis.analysis, retry_count + 1, max_retries);
                    
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    retry_count += 1;
                }
            }
        }
        
        Err(ReWOOError::ToolExecutionError(
            format!("Tool execution failed after {} retries. Last error: {}", 
                retry_count, 
                last_error.unwrap_or_else(|| "Unknown error".to_string())
            )
        ))
    }
    
    /// 分析错误类型并返回重试策略
    fn analyze_error(&self, error_message: &str) -> ErrorAnalysis {
        let error_lower = error_message.to_lowercase();
        
        // 网络相关错误
        if error_lower.contains("connection") || error_lower.contains("network") || 
           error_lower.contains("dns") || error_lower.contains("resolve") {
            return ErrorAnalysis {
                category: ErrorCategory::Network,
                should_retry: true,
                retry_strategy: RetryStrategy {
                    error_category: ErrorCategory::Network,
                    max_retries: 5,
                    base_delay_ms: 2000,
                    exponential_backoff: true,
                    backoff_multiplier: 1.5,
                    max_delay_ms: 15000,
                    jitter: true,
                },
                analysis: "Network connectivity issue".to_string(),
                suggestions: vec![
                    "Check network connectivity".to_string(),
                    "Verify DNS resolution".to_string(),
                ],
            };
        }
        
        // 超时错误
        if error_lower.contains("timeout") || error_lower.contains("timed out") {
            return ErrorAnalysis {
                category: ErrorCategory::Timeout,
                should_retry: true,
                retry_strategy: RetryStrategy {
                    error_category: ErrorCategory::Timeout,
                    max_retries: 3,
                    base_delay_ms: 5000,
                    exponential_backoff: true,
                    backoff_multiplier: 2.0,
                    max_delay_ms: 30000,
                    jitter: false,
                },
                analysis: "Request timeout".to_string(),
                suggestions: vec![
                    "Increase timeout duration".to_string(),
                    "Check target service responsiveness".to_string(),
                ],
            };
        }
        
        // 参数错误
        if error_lower.contains("parameter") || error_lower.contains("argument") || 
           error_lower.contains("invalid") || error_lower.contains("malformed") {
            return ErrorAnalysis {
                category: ErrorCategory::Parameter,
                should_retry: false,
                retry_strategy: RetryStrategy::default(),
                analysis: "Parameter validation failed".to_string(),
                suggestions: vec![
                    "Check parameter format and values".to_string(),
                    "Refer to tool documentation".to_string(),
                ],
            };
        }
        
        // 工具不可用
        if error_lower.contains("not found") || error_lower.contains("unavailable") || 
           error_lower.contains("not available") || error_lower.contains("command not found") {
            return ErrorAnalysis {
                category: ErrorCategory::ToolUnavailable,
                should_retry: false,
                retry_strategy: RetryStrategy::default(),
                analysis: "Tool not available".to_string(),
                suggestions: vec![
                    "Check if tool is installed".to_string(),
                    "Try alternative tool".to_string(),
                ],
            };
        }
        
        // 资源不足
        if error_lower.contains("rate limit") || error_lower.contains("quota") || 
           error_lower.contains("too many") || error_lower.contains("resource") {
            return ErrorAnalysis {
                category: ErrorCategory::ResourceExhaustion,
                should_retry: true,
                retry_strategy: RetryStrategy {
                    error_category: ErrorCategory::ResourceExhaustion,
                    max_retries: 3,
                    base_delay_ms: 10000,
                    exponential_backoff: true,
                    backoff_multiplier: 3.0,
                    max_delay_ms: 300000, // 5 minutes
                    jitter: true,
                },
                analysis: "Resource exhaustion or rate limiting".to_string(),
                suggestions: vec![
                    "Reduce request frequency".to_string(),
                    "Wait for quota reset".to_string(),
                ],
            };
        }
        
        // 权限错误
        if error_lower.contains("permission") || error_lower.contains("forbidden") || 
           error_lower.contains("unauthorized") || error_lower.contains("access denied") {
            return ErrorAnalysis {
                category: ErrorCategory::Permission,
                should_retry: false,
                retry_strategy: RetryStrategy::default(),
                analysis: "Permission or authorization issue".to_string(),
                suggestions: vec![
                    "Check API credentials".to_string(),
                    "Verify access permissions".to_string(),
                ],
            };
        }
        
        // 服务器错误
        if error_lower.contains("500") || error_lower.contains("502") || 
           error_lower.contains("503") || error_lower.contains("server error") {
            return ErrorAnalysis {
                category: ErrorCategory::ServerError,
                should_retry: true,
                retry_strategy: RetryStrategy {
                    error_category: ErrorCategory::ServerError,
                    max_retries: 4,
                    base_delay_ms: 3000,
                    exponential_backoff: true,
                    backoff_multiplier: 2.0,
                    max_delay_ms: 60000,
                    jitter: true,
                },
                analysis: "Server-side error".to_string(),
                suggestions: vec![
                    "Wait for service recovery".to_string(),
                    "Check service status".to_string(),
                ],
            };
        }
        
        // 默认：未知错误，谨慎重试
        ErrorAnalysis {
            category: ErrorCategory::Unknown,
            should_retry: true,
            retry_strategy: RetryStrategy {
                error_category: ErrorCategory::Unknown,
                max_retries: 2,
                base_delay_ms: 1000,
                exponential_backoff: true,
                backoff_multiplier: 2.0,
                max_delay_ms: 10000,
                jitter: true,
            },
            analysis: "Unknown error type".to_string(),
            suggestions: vec![
                "Check error details".to_string(),
                "Review tool configuration".to_string(),
            ],
        }
    }
    
    /// 计算重试延迟
    fn calculate_retry_delay(&self, strategy: &RetryStrategy, attempt: u32) -> u64 {
        let mut delay = strategy.base_delay_ms;
        
        if strategy.exponential_backoff {
            delay = (delay as f32 * strategy.backoff_multiplier.powi(attempt as i32)) as u64;
        } else {
            delay = delay * (attempt + 1) as u64;
        }
        
        // 应用最大延迟限制
        delay = delay.min(strategy.max_delay_ms);
        
        // 添加随机抖动以避免雷群效应
        if strategy.jitter {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let jitter_factor = rng.gen_range(0.8..1.2);
            delay = (delay as f32 * jitter_factor) as u64;
        }
        
        delay
    }
    
    /// 获取工具执行统计
    pub fn get_execution_stats(&self) -> HashMap<String, f64> {
        // 这里可以实现工具执行统计
        // 目前返回空的统计信息
        HashMap::new()
    }
}

// 实现 Clone trait 以支持并行执行
impl Clone for ReWOOWorker {
    fn clone(&self) -> Self {
        Self {
            framework_adapter: Arc::clone(&self.framework_adapter),
            config: self.config.clone(),
            runtime_params: self.runtime_params.clone(),
        }
    }
}

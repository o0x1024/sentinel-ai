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
pub struct ReWOOWorker {
    /// 框架适配器
    framework_adapter: Arc<dyn crate::tools::FrameworkToolAdapter>,
    /// 配置
    config: WorkerConfig,
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
        }
    }
    
    /// 使用全局框架适配器创建Worker
    pub async fn new_with_global_adapter(config: WorkerConfig) -> Result<Self, ReWOOError> {
        let framework_adapter = crate::tools::get_framework_adapter(crate::tools::FrameworkType::ReWOO).await
            .map_err(|e| ReWOOError::ToolSystemError(format!("获取ReWOO框架适配器失败: {}", e)))?;
        
        Ok(Self {
            framework_adapter,
            config,
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
                // 转换UnifiedToolResult为ToolResult
                ToolResult {
                    success: result.success,
                    content: result.output.to_string(),
                    error: result.error,
                    execution_time_ms: result.execution_time_ms,
                }
            },
            Err(e) => {
                error!("Tool '{}' execution error: {}", step.tool, e);
                ToolResult {
                    success: false,
                    content: String::new(),
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
    
    /// 解析工具参数
    async fn parse_tool_args_with_tool(&self, tool_name: &str, args_str: &str) -> Result<HashMap<String, Value>, ReWOOError> {
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
        // 例如："domain=mgtv.com, use_database_wordlist=true" 或 "domain: mgtv.com"
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
                    // 统一用Number，整数/浮点都支持
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

        // 裸字符串：根据工具定义推断映射到唯一必填参数
        if let Some(tool_info) = self.framework_adapter.get_tool_info(tool_name).await {
            let params = &tool_info.parameters;
            let required: Vec<_> = params.parameters.iter().filter(|p| p.required).collect();
            if required.len() == 1 {
                let key = &required[0].name;
                args.insert(key.clone(), Value::String(normalize_str_token(args_str)));
                return Ok(args);
            }
            // 如果没有必填但只有一个参数，也做映射
            if required.is_empty() && params.parameters.len() == 1 {
                let key = &params.parameters[0].name;
                args.insert(key.clone(), Value::String(normalize_str_token(args_str)));
                return Ok(args);
            }
        }

        // 最后回退到简单的字符串参数
        args.insert("input".to_string(), Value::String(normalize_str_token(args_str)));
        
        Ok(args)
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
    
    /// 重试执行工具
    pub async fn execute_tool_with_retry(
        &self,
        step: &PlanStep,
        substituted_args: &str,
        max_retries: u32,
    ) -> Result<ToolResult, ReWOOError> {
        let mut last_error = None;
        
        for attempt in 0..=max_retries {
            match self.execute_tool(step, substituted_args).await {
                Ok(result) => {
                    if result.success {
                        return Ok(result);
                    } else {
                        last_error = result.error.clone();
                        if attempt < max_retries {
                            // 等待一段时间后重试
                            tokio::time::sleep(Duration::from_millis(1000 * (attempt + 1) as u64)).await;
                        }
                    }
                },
                Err(e) => {
                    last_error = Some(e.to_string());
                    if attempt < max_retries {
                        // 等待一段时间后重试
                        tokio::time::sleep(Duration::from_millis(1000 * (attempt + 1) as u64)).await;
                    }
                }
            }
        }
        
        Err(ReWOOError::ToolExecutionError(
            format!("Tool execution failed after {} retries. Last error: {}", 
                max_retries, 
                last_error.unwrap_or_else(|| "Unknown error".to_string())
            )
        ))
    }
    
    /// 获取工具执行统计
    pub fn get_execution_stats(&self) -> HashMap<String, u64> {
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
        }
    }
}

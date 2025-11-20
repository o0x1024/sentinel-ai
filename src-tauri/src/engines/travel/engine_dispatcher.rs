//! 引擎调度器
//!
//! 根据任务复杂度选择合适的执行引擎

use super::types::*;
use super::react_executor::TravelReactExecutor;
use crate::services::ai::AiService;
use crate::services::prompt_db::PromptRepository;
use crate::tools::FrameworkToolAdapter;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// 引擎调度器
pub struct EngineDispatcher {
    // 服务依赖
    pub(crate) ai_service: Option<Arc<AiService>>,
    pub(crate) prompt_repo: Option<Arc<PromptRepository>>,
    pub(crate) framework_adapter: Option<Arc<dyn FrameworkToolAdapter>>,
    pub(crate) app_handle: Option<tauri::AppHandle>,
    // 配置
    pub(crate) max_react_iterations: u32,
    pub(crate) conversation_id: Option<String>,
    pub(crate) message_id: Option<String>,
    pub(crate) cancellation_token: Option<CancellationToken>,
}

impl EngineDispatcher {
    pub fn new() -> Self {
        Self {
            ai_service: None,
            prompt_repo: None,
            framework_adapter: None,
            app_handle: None,
            max_react_iterations: 100,
            conversation_id: None,
            message_id: None,
            cancellation_token: None,
        }
    }

    /// 设置AI服务
    pub fn with_ai_service(mut self, service: Arc<AiService>) -> Self {
        self.ai_service = Some(service);
        self
    }

    /// 设置Prompt仓库
    pub fn with_prompt_repo(mut self, repo: Arc<PromptRepository>) -> Self {
        log::info!("EngineDispatcher: Received prompt_repo");
        self.prompt_repo = Some(repo);
        self
    }

    /// 设置框架适配器
    pub fn with_framework_adapter(mut self, adapter: Arc<dyn FrameworkToolAdapter>) -> Self {
        self.framework_adapter = Some(adapter);
        self
    }

    /// 设置App Handle
    pub fn with_app_handle(mut self, handle: tauri::AppHandle) -> Self {
        self.app_handle = Some(handle);
        self
    }

    /// 设置最大ReAct迭代次数
    pub fn with_max_react_iterations(mut self, max: u32) -> Self {
        self.max_react_iterations = max;
        self
    }

    /// 设置会话ID
    pub fn with_conversation_id(mut self, id: String) -> Self {
        self.conversation_id = Some(id);
        self
    }

    /// 设置消息ID
    pub fn with_message_id(mut self, id: String) -> Self {
        self.message_id = Some(id);
        self
    }

    /// 设置取消令牌
    pub fn with_cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancellation_token = Some(token);
        self
    }

    /// 调度任务执行
    pub async fn dispatch(
        &self,
        complexity: TaskComplexity,
        action_plan: &ActionPlan,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        log::info!(
            "Dispatching task with complexity: {:?}, plan: {}",
            complexity,
            action_plan.name
        );

        match complexity {
            TaskComplexity::Simple => {
                self.dispatch_simple_task(action_plan, context).await
            }
            TaskComplexity::Medium => {
                self.dispatch_medium_task(action_plan, context).await
            }
            TaskComplexity::Complex => {
                self.dispatch_complex_task(action_plan, context).await
            }
        }
    }

    /// 调度简单任务(直接工具调用)
    async fn dispatch_simple_task(
        &self,
        action_plan: &ActionPlan,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        log::info!("Dispatching simple task: direct tool execution");

        let mut results = Vec::new();

        for step in &action_plan.steps {
            match &step.step_type {
                ActionStepType::DirectToolCall => {
                    if let Some(tool_name) = &step.tool_name {
                        let result = self.execute_tool(tool_name, &step.tool_args, context).await?;
                        results.push(serde_json::json!({
                            "step_id": step.id,
                            "step_name": step.name,
                            "tool": tool_name,
                            "result": result,
                        }));
                    }
                }
                _ => {
                    log::warn!("Unexpected step type in simple task: {:?}", step.step_type);
                }
            }
        }

        Ok(serde_json::json!({
            "execution_type": "simple",
            "results": results,
        }))
    }

    /// 调度中等任务(多工具顺序调用)
    async fn dispatch_medium_task(
        &self,
        action_plan: &ActionPlan,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        log::info!("Dispatching medium task: sequential tool execution");

        let mut results = Vec::new();
        let mut shared_context = context.clone();

        for step in &action_plan.steps {
            match &step.step_type {
                ActionStepType::DirectToolCall => {
                    if let Some(tool_name) = &step.tool_name {
                        match self.execute_tool(tool_name, &step.tool_args, &shared_context).await {
                            Ok(result) => {
                                // 将结果添加到共享上下文,供后续步骤使用
                                shared_context.insert(
                                    format!("step_{}_result", step.id),
                                    result.clone(),
                                );

                                results.push(serde_json::json!({
                                    "step_id": step.id,
                                    "step_name": step.name,
                                    "tool": tool_name,
                                    "result": result,
                                    "status": "success",
                                }));
                            }
                            Err(e) => {
                                log::error!("Tool {} execution failed: {}", tool_name, e);
                                results.push(serde_json::json!({
                                    "step_id": step.id,
                                    "step_name": step.name,
                                    "tool": tool_name,
                                    "error": e.to_string(),
                                    "status": "failed",
                                }));
                                // 继续执行后续步骤，不中断整个流程
                            }
                        }
                    } else {
                        log::warn!("Step {} has no tool name specified", step.id);
                    }
                }
                ActionStepType::ReactEngine => {
                    // ReactEngine 步骤应该由 dispatch_complex_task 处理
                    // 但如果在这里遇到，尝试降级处理
                    log::warn!("ReactEngine step in medium task, attempting to extract and execute tools");
                    
                    // 尝试从步骤描述中提取可执行的操作
                    if let Some(result) = self.try_execute_react_step_fallback(step, &shared_context).await {
                        results.push(result);
                    } else {
                        results.push(serde_json::json!({
                            "step_id": step.id,
                            "step_name": step.name,
                            "status": "skipped",
                            "reason": "ReactEngine step requires AI service",
                        }));
                    }
                }
                _ => {
                    log::warn!("Unexpected step type in medium task: {:?}", step.step_type);
                }
            }
        }

        Ok(serde_json::json!({
            "execution_type": "medium",
            "results": results,
            "total_steps": action_plan.steps.len(),
            "successful_steps": results.iter().filter(|r| r.get("status").and_then(|s| s.as_str()) == Some("success")).count(),
        }))
    }
    
    /// 尝试执行 ReactEngine 步骤的降级方案
    async fn try_execute_react_step_fallback(
        &self,
        step: &ActionStep,
        context: &HashMap<String, serde_json::Value>,
    ) -> Option<serde_json::Value> {
        // 从步骤参数中提取目标
        let target = step.tool_args.get("target")
            .and_then(|v| v.as_str())
            .or_else(|| context.get("target").and_then(|v| v.as_str()))?;
        
        log::info!("Attempting fallback execution for ReactEngine step: {}", step.name);
        
        // 执行基本的安全检查工具
        let mut results = Vec::new();
        
        // 1. 网站分析
        // 从 URL 中提取域名
        let domain = target
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .split('/')
            .next()
            .unwrap_or(target)
            .split(':')
            .next()
            .unwrap_or(target);
        
        if let Ok(result) = self.execute_tool(
            "analyze_website",
            &{
                let mut args = HashMap::new();
                args.insert("domain".to_string(), serde_json::json!(domain));
                args
            },
            context
        ).await {
            results.push(("analyze_website", result));
        }
        
        // 2. HTTP 请求
        if let Ok(result) = self.execute_tool(
            "http_request",
            &{
                let mut args = HashMap::new();
                args.insert("url".to_string(), serde_json::json!(target));
                args.insert("method".to_string(), serde_json::json!("GET"));
                args
            },
            context
        ).await {
            results.push(("http_request", result));
        }
        
        if results.is_empty() {
            None
        } else {
            Some(serde_json::json!({
                "step_id": step.id,
                "step_name": step.name,
                "status": "completed_with_fallback",
                "results": results.into_iter().map(|(tool, result)| {
                    serde_json::json!({
                        "tool": tool,
                        "result": result,
                    })
                }).collect::<Vec<_>>(),
            }))
        }
    }

    /// 调度复杂任务(使用内嵌ReAct执行器)
    async fn dispatch_complex_task(
        &self,
        action_plan: &ActionPlan,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        log::info!("Dispatching complex task: using embedded ReAct executor");

        // 检查必要的服务
        let ai_service = match &self.ai_service {
            Some(service) => service,
            None => {
                log::warn!("AI service not available, falling back to sequential execution");
                return self.dispatch_medium_task(action_plan, context).await;
            }
        };

        // 构建ReAct任务描述
        let task_description = self.build_react_task_description(action_plan, context);
        
        // 构建上下文字符串
        let context_str = serde_json::to_string_pretty(context)
            .unwrap_or_else(|_| "{}".to_string());

        log::info!("ReAct task description: {}", task_description);

        // 获取 framework_adapter（如果没有，使用全局适配器）
        let framework_adapter = if let Some(adapter) = &self.framework_adapter {
            Some(adapter.clone())
        } else {
            // 尝试获取全局 engine adapter 并转换为 framework adapter
            log::info!("No framework adapter set, attempting to use global engine adapter");
            match crate::tools::get_global_engine_adapter() {
                Ok(engine_adapter) => {
                    // EngineToolAdapter 也实现了工具执行，可以包装使用
                    // 但由于类型不同，我们需要创建一个包装器
                    // 暂时返回 None，让 ReAct 执行器内部处理
                    log::warn!("Global engine adapter available but type mismatch, ReAct will use fallback");
                    None
                }
                Err(e) => {
                    log::error!("Failed to get global adapter: {}", e);
                    None
                }
            }
        };

        // 提取工具权限
        let (allowed_tools, denied_tools, _) = self.extract_tool_permissions(context);
        
        // 创建ReAct执行器
        let mut react_executor = TravelReactExecutor::new(
            ai_service.clone(),
            self.prompt_repo.clone(),
            framework_adapter,
            self.max_react_iterations,
            self.conversation_id.clone(),
            self.message_id.clone(),
            self.app_handle.clone(),
            self.cancellation_token.clone(),
        );
        
        // 设置工具权限
        if !allowed_tools.is_empty() {
            react_executor = react_executor.with_allowed_tools(allowed_tools);
        }
        if !denied_tools.is_empty() {
            react_executor = react_executor.with_denied_tools(denied_tools);
        }

        // 执行ReAct循环
        match react_executor.execute(&task_description, &context_str).await {
            Ok(final_answer) => {
                log::info!("ReAct execution completed successfully");
                Ok(serde_json::json!({
                    "execution_type": "complex",
                    "engine": "ReAct",
                    "status": "completed",
                    "final_answer": final_answer,
                }))
            }
            Err(e) => {
                log::error!("ReAct execution failed: {}, falling back to sequential", e);
                self.dispatch_medium_task(action_plan, context).await
            }
        }
    }

    /// 构建ReAct任务描述
    fn build_react_task_description(
        &self,
        action_plan: &ActionPlan,
        context: &HashMap<String, serde_json::Value>,
    ) -> String {
        let mut description = format!("Task: {}\n\n", action_plan.name);
        description.push_str(&format!("Description: {}\n\n", action_plan.description));
        description.push_str("Steps to execute:\n");

        for (idx, step) in action_plan.steps.iter().enumerate() {
            description.push_str(&format!(
                "{}. {} - {}\n",
                idx + 1,
                step.name,
                step.description
            ));
        }

        description.push_str("\nContext:\n");
        for (key, value) in context {
            description.push_str(&format!("- {}: {}\n", key, value));
        }

        description
    }

    /// 执行单个工具(带权限检查和超时控制)
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        args: &HashMap<String, serde_json::Value>,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        log::info!("Executing tool: {} with args: {:?}", tool_name, args);

        // 1. 工具权限检查
        let (allow_list, deny_list, timeout_sec) = self.extract_tool_permissions(context);

        // 如果没有配置白名单,拒绝所有工具
        if allow_list.is_empty() {
            return Err(anyhow::anyhow!(
                "Tool '{}' not allowed (no tool permissions configured)",
                tool_name
            ));
        }

        // 检查白名单
        if !allow_list.iter().any(|n| n == tool_name) {
            return Err(anyhow::anyhow!(
                "Tool '{}' not in allow list",
                tool_name
            ));
        }

        // 检查黑名单
        if deny_list.iter().any(|n| n == tool_name) {
            return Err(anyhow::anyhow!(
                "Tool '{}' is denied",
                tool_name
            ));
        }

        // 2. 参数替换(替换变量引用)
        let substituted_args = self.substitute_variables(args, context);

        // 3. 构造统一工具调用
        let unified_call = crate::tools::UnifiedToolCall {
            id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool_name.to_string(),
            parameters: substituted_args.clone(),
            timeout: Some(std::time::Duration::from_secs(timeout_sec)),
            context: HashMap::new(),
            retry_count: 0,
        };

        // 4. 获取适配器并执行工具
        let result = if let Some(adapter) = &self.framework_adapter {
            // 使用设置的 framework adapter
            let timeout_duration = std::time::Duration::from_secs(timeout_sec);
            tokio::time::timeout(timeout_duration, adapter.execute_tool(unified_call)).await
        } else {
            // 使用全局 engine adapter
            log::info!("Using global engine adapter for tool execution");
            match crate::tools::get_global_engine_adapter() {
                Ok(engine_adapter) => {
                    let timeout_duration = std::time::Duration::from_secs(timeout_sec);
                    tokio::time::timeout(
                        timeout_duration,
                        engine_adapter.execute_tool(unified_call)
                    ).await
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to get global adapter: {}", e));
                }
            }
        };

        // 5. 处理执行结果
        match result {
            Ok(Ok(tool_result)) => {
                log::info!("Tool {} executed successfully", tool_name);
                Ok(tool_result.output)
            }
            Ok(Err(e)) => {
                log::error!("Tool {} execution failed: {}", tool_name, e);
                Err(anyhow::anyhow!("Tool execution failed: {}", e))
            }
            Err(_) => {
                log::error!("Tool {} execution timeout", tool_name);
                Err(anyhow::anyhow!("Tool execution timeout"))
            }
        }
    }

    /// 提取工具权限配置
    fn extract_tool_permissions(
        &self,
        context: &HashMap<String, serde_json::Value>,
    ) -> (Vec<String>, Vec<String>, u64) {
        let allow_list = context
            .get("tools_allow")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(Vec::new);

        let deny_list = context
            .get("tools_deny")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(Vec::new);

        let timeout_sec = context
            .get("execution_timeout_sec")
            .and_then(|v| v.as_u64())
            .unwrap_or(30); // 默认30秒

        (allow_list, deny_list, timeout_sec)
    }

    /// 替换参数中的变量引用
    fn substitute_variables(
        &self,
        args: &HashMap<String, serde_json::Value>,
        context: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut result = HashMap::new();

        for (key, value) in args {
            let substituted = match value {
                serde_json::Value::String(s) => {
                    // 替换 {{variable}} 格式的引用
                    let mut new_str = s.clone();
                    for (ctx_key, ctx_value) in context {
                        let pattern = format!("{{{{{}}}}}", ctx_key);
                        if new_str.contains(&pattern) {
                            let replacement = match ctx_value {
                                serde_json::Value::String(s) => s.clone(),
                                _ => ctx_value.to_string(),
                            };
                            new_str = new_str.replace(&pattern, &replacement);
                        }
                    }
                    serde_json::Value::String(new_str)
                }
                _ => value.clone(),
            };
            result.insert(key.clone(), substituted);
        }

        result
    }
}

impl Default for EngineDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dispatch_simple_task() {
        let dispatcher = EngineDispatcher::new();

        let action_plan = ActionPlan {
            id: "test-plan".to_string(),
            name: "Simple Scan".to_string(),
            description: "Scan a single port".to_string(),
            steps: vec![ActionStep {
                id: "step-1".to_string(),
                name: "Port Scan".to_string(),
                description: "Scan port 80".to_string(),
                step_type: ActionStepType::DirectToolCall,
                tool_name: Some("nmap".to_string()),
                tool_args: {
                    let mut args = HashMap::new();
                    args.insert("target".to_string(), serde_json::json!("localhost"));
                    args.insert("port".to_string(), serde_json::json!(80));
                    args
                },
                estimated_duration: 10,
            }],
            estimated_duration: 10,
            risk_assessment: RiskAssessment {
                risk_level: RiskLevel::Low,
                risk_factors: vec![],
                mitigations: vec![],
                requires_manual_approval: false,
            },
        };

        let context = HashMap::new();
        let result = dispatcher
            .dispatch(TaskComplexity::Simple, &action_plan, &context)
            .await
            .unwrap();

        assert_eq!(result["execution_type"], "simple");
    }
}



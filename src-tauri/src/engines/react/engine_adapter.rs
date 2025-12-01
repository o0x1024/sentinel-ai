//! ReAct 引擎适配器
//!
//! 实现 BaseExecutionEngine trait，对接 AI 服务、工具调用、RAG 等
//! 使用专用 LLM 客户端实现对消息流的精确控制

use super::executor::{ReactExecutor, ReactExecutorConfig};
use super::message_emitter::{ReactMessageEmitter, ReactLlmClient};
use crate::engines::llm_client::LlmConfig;
use super::types::*;
use crate::agents::traits::{
    AgentExecutionResult, AgentSession, AgentTask, PerformanceCharacteristics,
};
use crate::services::ai::AiService;
use crate::services::database::DatabaseService;
use crate::services::mcp::McpService;
use crate::services::prompt_db::PromptRepository;                   
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tauri::AppHandle;

/// ReAct 引擎
pub struct ReactEngine {
    config: ReactConfig,
    ai_service: Option<Arc<AiService>>,
    mcp_service: Option<Arc<McpService>>,
    db_service: Option<Arc<DatabaseService>>,
    app_handle: Option<AppHandle>,
}

impl ReactEngine {
    /// 创建新的 ReAct 引擎
    pub fn new(config: ReactConfig) -> Self {
        Self {
            config,
            ai_service: None,
            mcp_service: None,
            db_service: None,
            app_handle: None,
        }
    }

    /// 使用默认配置创建
    pub fn with_defaults() -> Self {
        Self::new(ReactConfig::default())
    }

    /// 设置依赖服务
    pub fn with_services(
        mut self,
        ai_service: Arc<AiService>,
        mcp_service: Option<Arc<McpService>>,
        db_service: Option<Arc<DatabaseService>>,
        app_handle: Option<AppHandle>,
    ) -> Self {
        self.ai_service = Some(ai_service);
        self.mcp_service = mcp_service;
        self.db_service = db_service;
        self.app_handle = app_handle;
        self
    }

    /// 执行 ReAct 流程
    pub async fn execute(
        &self,
        task: &AgentTask,
        _session: &mut dyn AgentSession,
    ) -> Result<AgentExecutionResult> {
        // 创建 PromptRepository（如果有数据库服务）
        let prompt_repo = if let Some(db_service) = &self.db_service {
            let pool = db_service
                .get_pool()
                .map_err(|e| anyhow!("DB pool error: {}", e))?;
            Some(Arc::new(PromptRepository::new(pool.clone())))
        } else {
            None
        };

        // 获取框架工具适配器
        let framework_adapter = match crate::tools::get_global_adapter_manager() {
            Ok(_adapter_manager) => {
                let adapter = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        crate::tools::get_framework_adapter(crate::tools::FrameworkType::React)
                            .await
                    })
                });
                match adapter {
                    Ok(adapter) => Some(adapter),
                    Err(e) => {
                        log::warn!("Failed to get framework adapter: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to get framework adapter manager: {}", e);
                None
            }
        };

        // 从任务参数中获取前端传入的会话/消息ID/执行ID（与 Plan-and-Execute 引擎保持一致）
        let conversation_id = task
            .parameters
            .get("conversation_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let message_id = task
            .parameters
            .get("message_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let execution_id = task
            .parameters
            .get("execution_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        println!(
            "ReactEngine execute: conversation_id={:?}, message_id={:?}, execution_id={:?}",
            conversation_id, message_id, execution_id
        );

        // ✅ 获取取消令牌（优先从全局管理器获取，如果有execution_id）
        let cancellation_token = if let Some(exec_id) = &execution_id {
            match crate::managers::cancellation_manager::get_token(exec_id).await {
                Some(token) => {
                    log::info!("✅ Retrieved cancellation token for execution: {}", exec_id);
                    Some(token)
                }
                None => {
                    log::warn!("⚠️ No cancellation token found for execution: {}", exec_id);
                    None
                }
            }
        } else {
            None
        };

        // === Memory 集成：获取全局 Memory 并创建集成器 ===
        let memory_integration = {
            use crate::engines::memory::get_global_memory;
            use super::memory_integration::ReactMemoryIntegration;
            
            let memory = get_global_memory();
            Some(Arc::new(ReactMemoryIntegration::new(memory)))
        };

        // 创建共享的消息发送器（executor 和 llm_client 共用）
        let emitter = if self.app_handle.is_some() {
            let msg_id = message_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let exec_id = execution_id.clone().unwrap_or_else(|| msg_id.clone());
            Some(Arc::new(ReactMessageEmitter::new(
                Arc::new(self.app_handle.as_ref().unwrap().clone()),
                exec_id,
                msg_id,
                conversation_id.clone(),
            )))
        } else {
            None
        };

        let executor_config = ReactExecutorConfig {
            react_config: self.config.clone(),
            enable_streaming: true,
            conversation_id: conversation_id.clone(),
            message_id: message_id.clone(),
            execution_id: execution_id.clone(),
            app_handle: self.app_handle.clone(),
            prompt_repo,
            framework_adapter,
            task_parameters: Some(
                serde_json::to_value(&task.parameters)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
            ),
            cancellation_token, // ✅ 传递取消令牌
            memory_integration, // ✅ Memory 集成
            summarization_threshold: 8, // 超过 8 步时进行摘要
            emitter: emitter.clone(), // ✅ 共享消息发送器
        };

        let executor = ReactExecutor::new(task.description.clone(), executor_config.clone());

        // 创建 LLM 配置（从 ai_service 获取）
        let llm_config = if let Some(ref service) = self.ai_service {
            let cfg = service.get_config();
            LlmConfig {
                provider: cfg.provider.clone(),
                model: cfg.model.clone(),
                api_key: cfg.api_key.clone(),
                base_url: cfg.api_base.clone(),
                timeout_secs: 120,
            }
        } else {
            LlmConfig::default()
        };

        // 定义 LLM 调用闭包（使用专用 LLM 客户端）
        let llm_call = {
            let emitter_for_llm = emitter.clone();
            let llm_config = llm_config.clone();
            
            move |system_prompt: Option<String>,
                  user_prompt: String,
                  _skip_save_user_message: bool,
                  _original_user_input: String| {
                let emitter = emitter_for_llm.clone();
                let config = llm_config.clone();
                
                Box::pin(async move {
                    if let Some(emitter) = emitter {
                        let client = ReactLlmClient::new(config, emitter);
                        client.stream_completion(system_prompt.as_deref(), &user_prompt, 0).await
                    } else {
                        Err(anyhow!("Emitter not configured"))
                    }
                }) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>
            }
        };

        // 定义工具执行闭包 - 使用框架适配器统一路由工具调用
        let tool_executor = move |tool_call: ReactToolCall| {
            let framework_adapter = executor_config.framework_adapter.clone();
            Box::pin(async move {
                if let Some(adapter) = framework_adapter {
                    // 构造 UnifiedToolCall
                    use sentinel_tools::unified_types::UnifiedToolCall;
                    use std::collections::HashMap;

                    let parameters: HashMap<String, serde_json::Value> =
                        if let Some(obj) = tool_call.args.as_object() {
                            obj.clone().into_iter().collect()
                        } else {
                            HashMap::new()
                        };

                    let unified_call = UnifiedToolCall {
                        id: uuid::Uuid::new_v4().to_string(),
                        tool_name: tool_call.tool.clone(),
                        parameters,
                        timeout: None,
                        context: HashMap::new(),
                        retry_count: 0,
                    };

                    // 通过框架适配器执行工具（统一路由内置工具和MCP工具）
                    match adapter.execute_tool(unified_call).await {
                        Ok(result) => {
                            if result.success {
                                Ok(result.output)
                            } else {
                                Err(anyhow!(
                                    "Tool execution failed: {}",
                                    result.error.unwrap_or_else(|| "Unknown error".to_string())
                                ))
                            }
                        }
                        Err(e) => Err(anyhow!("Tool execution failed: {}", e)),
                    }
                } else {
                    // 降级：如果没有框架适配器，返回错误
                    tracing::error!("Framework adapter not available for tool execution");
                    Err(anyhow!("Tool does not exist: {}", tool_call.tool))
                }
            })
                as std::pin::Pin<
                    Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send>,
                >
        };

        // 执行循环
        let trace = executor.run(llm_call, tool_executor).await?;

        // ✅ 清理取消令牌
        if let Some(exec_id) = &execution_id {
            crate::managers::cancellation_manager::cleanup_token(exec_id).await;
        }

        // 保存完整的AI响应消息到数据库
        if let (Some(db), Some(conv_id), Some(msg_id)) =
            (&self.db_service, &conversation_id, &message_id)
        {
            use crate::models::database::AiMessage;
            use chrono::Utc;

            // 获取完整的消息内容（从 emitter 收集的所有内容）
            let full_content = emitter
                .as_ref()
                .map(|e| e.get_full_content())
                .unwrap_or_default();

            // 如果收集的内容为空，回退到提取 final answer
            let content_to_save = if full_content.is_empty() {
                trace
                    .steps
                    .iter()
                    .rev()
                    .find(|step| matches!(step.step_type, ReactStepType::Final { .. }))
                    .and_then(|step| {
                        if let ReactStepType::Final { answer, .. } = &step.step_type {
                            Some(answer.clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| "No final answer".to_string())
            } else {
                full_content
            };

            // 构建结构化数据
            let structured_data = serde_json::json!({
                "reactSteps": trace.steps.iter().map(|step| {
                    match &step.step_type {
                        ReactStepType::Thought { content, .. } => serde_json::json!({
                            "thought": content,
                        }),
                        ReactStepType::Action { tool_call } => serde_json::json!({
                            "action": {
                                "tool": tool_call.tool,
                                "args": tool_call.args,
                                "status": "completed",
                            }
                        }),
                        ReactStepType::Observation { tool_name, result, success, .. } => serde_json::json!({
                            "observation": result,
                            "action": {
                                "tool": tool_name,
                                "status": if *success { "success" } else { "failed" },
                            }
                        }),
                        ReactStepType::Final { answer, citations } => serde_json::json!({
                            "finalAnswer": answer,
                            "citations": citations,
                        }),
                        ReactStepType::Error { error_type, message, retryable } => serde_json::json!({
                            "error": {
                                "type": error_type,
                                "message": message,
                                "retryable": retryable,
                            }
                        }),
                    }
                }).collect::<Vec<_>>(),
            });

            let ai_msg = AiMessage {
                id: msg_id.clone(),
                conversation_id: conv_id.clone(),
                role: "assistant".to_string(),
                content: content_to_save,
                metadata: None,
                token_count: Some(trace.metrics.total_tokens as i32),
                cost: None,
                tool_calls: None,
                attachments: None,
                timestamp: Utc::now(),
                architecture_type: Some("ReAct".to_string()),
                architecture_meta: Some(
                    serde_json::to_string(&serde_json::json!({
                        "type": "ReAct",
                        "steps": trace.steps.iter().map(|step| {
                            match &step.step_type {
                                ReactStepType::Thought { content, .. } => serde_json::json!({
                                    "thought": content,
                                }),
                                ReactStepType::Action { tool_call } => serde_json::json!({
                                    "action": {
                                        "tool": tool_call.tool,
                                        "args": tool_call.args,
                                        "status": "pending",
                                    }
                                }),
                                ReactStepType::Observation { tool_name, result, success, .. } => serde_json::json!({
                                    "observation": result,
                                    "action": {
                                        "tool": tool_name,
                                        "args": {},
                                        "status": if *success { "success" } else { "failed" },
                                    }
                                }),
                                ReactStepType::Final { answer, citations } => serde_json::json!({
                                    "finalAnswer": answer,
                                    "citations": citations,
                                }),
                                ReactStepType::Error { error_type, message, retryable } => serde_json::json!({
                                    "error": {
                                        "type": error_type,
                                        "message": message,
                                        "retryable": retryable,
                                    }
                                }),
                            }
                        }).collect::<Vec<_>>(),
                        "statistics": {
                            "total_iterations": trace.metrics.total_iterations,
                            "tool_calls_count": trace.metrics.tool_calls_count,
                            "successful_tool_calls": trace.metrics.successful_tool_calls,
                            "failed_tool_calls": trace.metrics.failed_tool_calls,
                            "total_duration_ms": trace.metrics.total_duration_ms,
                            "status": format!("{:?}", trace.status),
                        }
                    }))
                    .unwrap_or_default(),
                ),
                structured_data: Some(serde_json::to_string(&structured_data).unwrap_or_default()),
            };

            if let Err(e) = db.upsert_ai_message_append(&ai_msg).await {
                log::warn!("Failed to save ReAct message to database: {}", e);
            } else {
                log::info!("ReAct message saved to database: {}", msg_id);
            }
        }

        // 转换为 AgentExecutionResult
        self.trace_to_result(trace)
    }

    /// 将 ReactTrace 转换为 AgentExecutionResult
    fn trace_to_result(&self, trace: ReactTrace) -> Result<AgentExecutionResult> {
        // 提取最终答案
        let final_step = trace
            .steps
            .iter()
            .rev()
            .find(|step| matches!(step.step_type, ReactStepType::Final { .. }));

        let output = match final_step {
            Some(step) => {
                if let ReactStepType::Final { answer, .. } = &step.step_type {
                    answer.clone()
                } else {
                    "No final answer found".to_string()
                }
            }
            None => {
                if trace.status == ReactStatus::MaxIterationsReached {
                    "Maximum iterations reached without final answer".to_string()
                } else {
                    "Execution failed or was cancelled".to_string()
                }
            }
        };

        Ok(AgentExecutionResult {
            id: trace.trace_id.clone(),
            success: trace.status == ReactStatus::Completed,
            data: Some(serde_json::json!({
                "output": output,
                "trace_id": trace.trace_id,
                "iterations": trace.metrics.total_iterations,
                "tool_calls": trace.metrics.tool_calls_count,
                "duration_ms": trace.metrics.total_duration_ms,
                "status": format!("{:?}", trace.status),
            })),
            error: if trace.status == ReactStatus::Failed {
                Some("Execution failed".to_string())
            } else {
                None
            },
            execution_time_ms: trace.metrics.total_duration_ms,
            resources_used: std::collections::HashMap::new(),
            artifacts: Vec::new(),
        })
    }
}

// 实现 BaseExecutionEngine trait
#[async_trait]
impl crate::engines::traits::BaseExecutionEngine for ReactEngine {
    fn get_name(&self) -> &str {
        "ReAct"
    }

    fn get_description(&self) -> &str {
        "Reasoning + Acting framework for iterative problem solving with tool use"
    }

    fn get_version(&self) -> &str {
        "1.0.0"
    }

    fn get_supported_scenarios(&self) -> Vec<String> {
        vec![
            "multi_step_reasoning".to_string(),
            "tool_based_qa".to_string(),
            "research_tasks".to_string(),
            "debugging".to_string(),
            "information_gathering".to_string(),
        ]
    }

    fn get_performance_characteristics(&self) -> PerformanceCharacteristics {
        PerformanceCharacteristics {
            token_efficiency: 60,       // 中等，因为需要多轮交互
            execution_speed: 50,        // 中等，迭代执行较慢
            resource_usage: 50,         // 中等资源消耗
            concurrency_capability: 70, // 良好的并发能力
            complexity_handling: 80,    // 高复杂度处理能力
        }
    }
}

// 定义 ReAct 专属 trait（扩展）
#[async_trait]
pub trait ReactEngineExt: crate::engines::traits::BaseExecutionEngine {
    /// 执行 ReAct 流程
    async fn run_react(
        &self,
        task: &AgentTask,
        session: &mut dyn AgentSession,
    ) -> Result<AgentExecutionResult>;

    /// 流式执行（可选）
    async fn stream_react(&self, task: &AgentTask, session: &mut dyn AgentSession) -> Result<()>;
}

#[async_trait]
impl ReactEngineExt for ReactEngine {
    async fn run_react(
        &self,
        task: &AgentTask,
        session: &mut dyn AgentSession,
    ) -> Result<AgentExecutionResult> {
        self.execute(task, session).await
    }

    async fn stream_react(&self, _task: &AgentTask, _session: &mut dyn AgentSession) -> Result<()> {
        // TODO: 实现流式版本
        Err(anyhow!("Streaming not yet implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = ReactEngine::with_defaults();
        assert!(engine.ai_service.is_none());
        assert!(engine.mcp_service.is_none());
    }

    #[test]
    fn test_config_defaults() {
        let config = ReactConfig::default();
        assert_eq!(config.max_iterations, 100);
        assert!(config.enable_rag);
    }
}

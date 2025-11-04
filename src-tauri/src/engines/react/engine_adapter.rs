//! ReAct 引擎适配器
//!
//! 实现 BaseExecutionEngine trait，对接 AI 服务、工具调用、RAG 等

use super::executor::{ReactExecutor, ReactExecutorConfig};
use super::types::*;
use crate::agents::traits::{AgentTask, AgentSession, AgentExecutionResult, PerformanceCharacteristics};
use crate::services::ai::AiService;
use crate::services::database::DatabaseService;
use crate::services::mcp::McpService;
use crate::services::prompt_db::PromptRepository;
use crate::utils::ordered_message::ChunkType;
use anyhow::{Result, anyhow};
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
            let pool = db_service.get_pool()
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
                        crate::tools::get_framework_adapter(crate::tools::FrameworkType::React).await
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

        println!("ReactEngine execute: conversation_id={:?}, message_id={:?}, execution_id={:?}", 
            conversation_id, message_id, execution_id);
        let executor_config = ReactExecutorConfig {
            react_config: self.config.clone(),
            enable_streaming: true,
            conversation_id: conversation_id.clone(),
            message_id: message_id.clone(),
            execution_id: execution_id.clone(),
            app_handle: self.app_handle.clone(),
            prompt_repo,
            framework_adapter,
            task_parameters: Some(serde_json::to_value(&task.parameters).unwrap_or(serde_json::Value::Object(serde_json::Map::new()))),
        };

        let executor = ReactExecutor::new(
            task.description.clone(),
            executor_config.clone(),
        );

        // 克隆服务引用和 IDs 供闭包使用
        let ai_service = self.ai_service.clone();
        let mcp_service = self.mcp_service.clone();
        let conv_id_for_llm = conversation_id.clone();
        let msg_id_for_llm = message_id.clone();
        
        // 定义 LLM 调用闭包
        let llm_call = move |system_prompt: Option<String>, user_prompt: String, skip_save_user_message: bool, original_user_input: String| {
            let ai_service = ai_service.clone();
            let conv_id = conv_id_for_llm.clone();
            let msg_id = msg_id_for_llm.clone();
            Box::pin(async move {
                if let Some(service) = ai_service {
                    // 根据skip_save_user_message决定要保存的内容
                    let user_to_save = if skip_save_user_message {
                        None // 不保存
                    } else if !original_user_input.is_empty() {
                        Some(original_user_input.as_str()) // 保存原始用户输入
                    } else {
                        None // 没有原始输入则不保存
                    };
                    
                    // 调用 AI 服务进行 LLM 请求
                    service
                        .send_message_stream_with_save_control(
                            Some(&user_prompt), // 完整的user_prompt用于LLM推理
                            user_to_save, // 保存到数据库的用户消息（原始输入或None）
                            system_prompt.as_deref(), // system_prompt
                            conv_id, // conversation_id
                            msg_id, // message_id
                            true, // stream (启用流式输出)
                            false, // is_final (由每个chunk自行决定)
                            Some(ChunkType::Content), // chunk_type
                        )
                        .await
                        .map_err(|e| anyhow!("LLM call failed: {}", e))
                } else {
                    Err(anyhow!("AI service not configured"))
                }
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>
        };

        // 定义工具执行闭包 - 使用框架适配器统一路由工具调用
        let tool_executor = move |tool_call: ReactToolCall| {
            let framework_adapter = executor_config.framework_adapter.clone();
            Box::pin(async move {
                if let Some(adapter) = framework_adapter {
                    // 构造 UnifiedToolCall
                    use sentinel_tools::unified_types::UnifiedToolCall;
                    use std::collections::HashMap;
                    
                    let parameters: HashMap<String, serde_json::Value> = if let Some(obj) = tool_call.args.as_object() {
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
                                Err(anyhow!("Tool execution failed: {}", result.error.unwrap_or_else(|| "Unknown error".to_string())))
                            }
                        }
                        Err(e) => Err(anyhow!("Tool execution failed: {}", e)),
                    }
                } else {
                    // 降级：如果没有框架适配器，返回错误
                    tracing::error!("Framework adapter not available for tool execution");
                    Err(anyhow!("Tool does not exist: {}", tool_call.tool))
                }
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send>>
        };

        // 执行循环
        let trace = executor.run(llm_call, tool_executor).await?;

        // 转换为 AgentExecutionResult
        self.trace_to_result(trace)
    }

    /// 将 ReactTrace 转换为 AgentExecutionResult
    fn trace_to_result(&self, trace: ReactTrace) -> Result<AgentExecutionResult> {
        // 提取最终答案
        let final_step = trace.steps.iter().rev().find(|step| {
            matches!(step.step_type, ReactStepType::Final { .. })
        });

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
            token_efficiency: 60, // 中等，因为需要多轮交互
            execution_speed: 50, // 中等，迭代执行较慢
            resource_usage: 50, // 中等资源消耗
            concurrency_capability: 70, // 良好的并发能力
            complexity_handling: 80, // 高复杂度处理能力
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
    async fn stream_react(
        &self,
        task: &AgentTask,
        session: &mut dyn AgentSession,
    ) -> Result<()>;
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

    async fn stream_react(
        &self,
        _task: &AgentTask,
        _session: &mut dyn AgentSession,
    ) -> Result<()> {
        // TODO: 实现流式版本
        Err(anyhow!("Streaming not yet implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_metadata() {
        // let engine = ReactEngine::with_defaults();
        // assert_eq!(engine.get_name(), "ReAct");
        // assert!(engine.get_supported_scenarios().contains(&"multi_step_reasoning".to_string()));
    }
}

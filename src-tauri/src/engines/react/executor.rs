//! ReAct 执行器
//!
//! 实现核心循环：Thought → Action → Observation → 收敛判定
//!
//! 集成功能：
//! - Memory 系统（经验学习、工具缓存）
//! - Context Summarization（长对话摘要）
//! - RAG 知识检索
//! - 结构化消息发送（前端友好）

use super::memory_integration::{ContextSummarizer, ReactMemoryIntegration};
use super::message_emitter::{ReactMessageEmitter, ReactExecutionStats};
use super::parser::ActionParser;
use super::types::*;
use crate::services::prompt_db::PromptRepository;
use anyhow::{anyhow, Context, Result};
use sentinel_core::models::prompt::{ArchitectureType, StageType};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

/// ReAct 执行器配置
#[derive(Clone)]
pub struct ReactExecutorConfig {
    pub react_config: ReactConfig,
    /// 是否启用流式输出
    pub enable_streaming: bool,
    /// Conversation ID（用于流式消息）
    pub conversation_id: Option<String>,
    /// Message ID（前端创建的助手消息ID，用于流式消息）
    pub message_id: Option<String>,
    /// Execution ID（用于跟踪整个执行过程的唯一标识）
    pub execution_id: Option<String>,
    /// App Handle（用于发送事件）
    pub app_handle: Option<tauri::AppHandle>,
    /// Prompt Repository（用于加载提示词模板）
    pub prompt_repo: Option<Arc<PromptRepository>>,
    /// 框架工具适配器（用于获取工具列表）
    pub framework_adapter: Option<Arc<dyn crate::tools::FrameworkToolAdapter>>,
    /// 任务参数（包含角色提示词、工具过滤等）
    pub task_parameters: Option<serde_json::Value>,
    /// 取消令牌（用于支持任务取消）
    pub cancellation_token: Option<CancellationToken>,
    /// Memory 集成（用于经验学习、工具缓存）
    pub memory_integration: Option<Arc<ReactMemoryIntegration>>,
    /// Context Summarization 阈值（超过此步数时进行摘要，0 表示禁用）
    pub summarization_threshold: usize,
    /// 消息发送器（外部创建，确保 llm_call 和 executor 使用同一个）
    pub emitter: Option<Arc<ReactMessageEmitter>>,
}

impl std::fmt::Debug for ReactExecutorConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReactExecutorConfig")
            .field("react_config", &self.react_config)
            .field("enable_streaming", &self.enable_streaming)
            .field("conversation_id", &self.conversation_id)
            .field("message_id", &self.message_id)
            .field("execution_id", &self.execution_id)
            .field("has_app_handle", &self.app_handle.is_some())
            .field("has_prompt_repo", &self.prompt_repo.is_some())
            .field("has_framework_adapter", &self.framework_adapter.is_some())
            .field("task_parameters", &self.task_parameters)
            .field("has_memory_integration", &self.memory_integration.is_some())
            .field("summarization_threshold", &self.summarization_threshold)
            .field("has_emitter", &self.emitter.is_some())
            .finish()
    }
}

/// ReAct 执行器
pub struct ReactExecutor {
    config: ReactExecutorConfig,
    trace: Arc<RwLock<ReactTrace>>,
    cancellation_token: CancellationToken,
}

impl ReactExecutor {
    /// 创建新的执行器
    pub fn new(task: String, config: ReactExecutorConfig) -> Self {
        let trace = ReactTrace::new(task);
        let cancellation_token = config.cancellation_token.clone()
            .unwrap_or_else(|| CancellationToken::new());
        
        Self {
            config,
            trace: Arc::new(RwLock::new(trace)),
            cancellation_token,
        }
    }

    /// 执行主循环
    pub async fn run<F, Ft>(&self, llm_call: F, tool_executor: Ft) -> Result<ReactTrace>
    where
        F: Fn(
                Option<String>,
                String,
                bool,
                String,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>
            + Send
            + Sync,
        Ft: Fn(
                ReactToolCall,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send>,
            > + Send
            + Sync,
    {
        let start_time = SystemTime::now();
        let mut iteration = 0;
        let mut context_history = Vec::new();

        // 初始任务描述
        let task = {
            let trace = self.trace.read().await;
            trace.task.clone()
        };

        // 使用外部传入的 emitter（确保与 llm_call 共享同一个实例）
        let emitter = self.config.emitter.clone();

        // 发送执行开始信号
        if let Some(ref emitter) = emitter {
            emitter.emit_start(Some(serde_json::json!({
                "max_iterations": self.config.react_config.max_iterations,
                "enable_rag": self.config.react_config.enable_rag,
                "memory_enabled": self.config.memory_integration.is_some(),
            })));
        }

        // === Memory 集成：检索相似推理链作为 Few-shot 示例 ===
        let mut few_shot_examples = String::new();
        if let Some(ref memory) = self.config.memory_integration {
            match memory.retrieve_reasoning_chains(&task).await {
                Ok(examples) if !examples.is_empty() => {
                    log::info!("ReAct: Retrieved {} similar reasoning chain examples from memory", examples.len());
                    few_shot_examples = self.format_few_shot_examples(&examples);
                }
                Ok(_) => {
                    log::debug!("ReAct: No similar reasoning chains found in memory");
                }
                Err(e) => {
                    log::warn!("ReAct: Failed to retrieve reasoning chains from memory: {}", e);
                }
            }
        }

        // Context Summarizer（如果启用）
        let summarizer = if self.config.summarization_threshold > 0 {
            Some(ContextSummarizer::new(self.config.summarization_threshold))
        } else {
            None
        };

        // 可选：首次思考前注入 RAG 证据
        let mut rag_context = String::new();
        if self.config.react_config.enable_rag {
            if let Some(rag_cfg) = &self.config.react_config.rag_config {
                if matches!(rag_cfg.injection_point, RagInjectionPoint::Initial) {
                    rag_context = self.fetch_rag_context(&task).await?;
                }
            }
        }

        // 跟踪连续失败的计划执行次数
        let mut consecutive_plan_failures = 0;
        const MAX_CONSECUTIVE_PLAN_FAILURES: u32 = 3;
        
        // 跟踪上次失败的计划描述（用于检测重复失败）
        let mut last_failed_plan: Option<String> = None;
        
        loop {
            iteration += 1;

            // ✅ 检查取消状态（优先级最高）
            if self.cancellation_token.is_cancelled() {
                tracing::info!("❌ ReAct: Execution cancelled by user (iteration {})", iteration);
                let mut trace = self.trace.write().await;
                trace.complete(ReactStatus::Cancelled);
                trace.metrics.total_iterations = iteration - 1;
                trace.metrics.total_duration_ms = start_time
                    .elapsed()
                    .unwrap_or(Duration::from_secs(0))
                    .as_millis() as u64;
                return Ok(trace.clone());
            }

            // 检查终止条件
            if iteration > self.config.react_config.max_iterations {
                let mut trace = self.trace.write().await;
                trace.complete(ReactStatus::MaxIterationsReached);
                trace.metrics.total_iterations = iteration - 1;
                return Ok(trace.clone());
            }

            // === 步骤 1: Thought（思考） ===
            let thought_start = SystemTime::now();
            let (system_prompt, user_prompt) = self
                .build_thought_prompt(&task, &context_history, &rag_context, &few_shot_examples)
                .await;

            // 调用LLM时，传入原始任务作为要保存的用户消息（仅第一次迭代）
            let original_user_input = if iteration == 1 {
                task.clone()
            } else {
                String::new()
            };
            let skip_save = iteration > 1; // 第一次迭代后不再保存用户消息

            let llm_output = llm_call(system_prompt, user_prompt, skip_save, original_user_input)
                .await
                .context("LLM call failed during Thought phase")?;

            // ✅ LLM调用后再次检查取消状态
            if self.cancellation_token.is_cancelled() {
                tracing::info!("❌ ReAct: Execution cancelled after LLM call (iteration {})", iteration);
                let mut trace = self.trace.write().await;
                trace.complete(ReactStatus::Cancelled);
                trace.metrics.total_iterations = iteration;
                trace.metrics.total_duration_ms = start_time
                    .elapsed()
                    .unwrap_or(Duration::from_secs(0))
                    .as_millis() as u64;
                return Ok(trace.clone());
            }

            let thought_duration = thought_start
                .elapsed()
                .unwrap_or(Duration::from_secs(0))
                .as_millis() as u64;

            // 记录 Thought 步骤
            {
                let mut trace = self.trace.write().await;
                trace.add_step(ReactStep {
                    id: format!("thought_{}", iteration),
                    step_type: ReactStepType::Thought {
                        content: llm_output.clone(),
                        has_rag_context: !rag_context.is_empty(),
                    },
                    timestamp: thought_start,
                    duration_ms: Some(thought_duration),
                    token_usage: None, // TODO: 从 LLM 响应提取
                    error: None,
                });
            }

            // LLM 输出已通过 llm_client 流式发送，这里不再重复发送

            // === 步骤 2: 解析 Action ===
            let instruction = match ActionParser::parse(&llm_output) {
                Ok(inst) => inst,
                Err(e) => {
                    // 解析失败，尝试重试
                    tracing::warn!("Failed to parse action: {}", e);

                    if iteration <= self.config.react_config.retry_config.max_retries {
                        context_history.push(format!(
                            "Thought: {}\nError: Failed to parse action. Please use valid JSON format or 'Action: <tool>' format.",
                            llm_output
                        ));
                        continue;
                    } else {
                        let mut trace = self.trace.write().await;
                        trace.complete(ReactStatus::Failed);
                        return Err(anyhow!("Failed to parse action after retries: {}", e));
                    }
                }
            };

            // === 步骤 3: 处理指令 ===
            match instruction {
                ActionInstruction::FinalAnswer { final_answer } => {
                    // 达成最终答案
                    tracing::info!(
                        "✅ ReAct: Reached Final Answer (length: {} chars)",
                        final_answer.answer.len()
                    );

                    // 更新 trace 状态
                    let trace_clone = {
                        let mut trace_guard = self.trace.write().await;
                        trace_guard.add_step(ReactStep {
                            id: format!("final_{}", iteration),
                            step_type: ReactStepType::Final {
                                answer: final_answer.answer.clone(),
                                citations: final_answer.citations.clone(),
                            },
                            timestamp: SystemTime::now(),
                            duration_ms: None,
                            token_usage: None,
                            error: None,
                        });
                        trace_guard.complete(ReactStatus::Completed);
                        trace_guard.metrics.total_iterations = iteration;
                        trace_guard.metrics.total_duration_ms = start_time
                            .elapsed()
                            .unwrap_or(Duration::from_secs(0))
                            .as_millis() as u64;
                        trace_guard.clone()
                    };

                    // === Memory 集成：存储执行轨迹 ===
                    if let Some(ref memory) = self.config.memory_integration {
                        if let Err(e) = memory.store_trace(&trace_clone).await {
                            log::warn!("ReAct: Failed to store trace to memory: {}", e);
                        } else {
                            log::info!("ReAct: Trace stored to memory successfully");
                        }
                    }

                    // 发送执行完成信号
                    if let Some(ref emitter) = emitter {
                        emitter.emit_complete(ReactExecutionStats {
                            total_iterations: iteration,
                            tool_calls_count: trace_clone.metrics.tool_calls_count,
                            successful_tool_calls: trace_clone.metrics.successful_tool_calls,
                            failed_tool_calls: trace_clone.metrics.failed_tool_calls,
                            total_duration_ms: trace_clone.metrics.total_duration_ms,
                            status: "Completed".to_string(),
                        });
                    }

                    return Ok(trace_clone);
                }
                ActionInstruction::ToolCall { action, .. } => {
                    // === 步骤 4: Action（工具调用） ===
                    let action_start = SystemTime::now();

                    // 发送工具调用信息到前端
                    if let Some(ref emitter) = emitter {
                        emitter.emit_tool_call(iteration, &action.tool, &action.args);
                    }

                    // 记录 Action 步骤
                    {
                        let mut trace = self.trace.write().await;
                        trace.add_step(ReactStep {
                            id: format!("action_{}", iteration),
                            step_type: ReactStepType::Action {
                                tool_call: action.clone(),
                            },
                            timestamp: action_start,
                            duration_ms: None,
                            token_usage: None,
                            error: None,
                        });
                        trace.metrics.tool_calls_count += 1;
                    }

                    // === Memory 集成：检查工具调用缓存 ===
                    let cached_result = if let Some(ref memory) = self.config.memory_integration {
                        // 先检查会话级缓存
                        match memory.check_tool_cache(&action.tool, &action.args).await {
                            Ok(Some(result)) => {
                                log::info!("ReAct: Tool cache hit for {} (session cache)", action.tool);
                                Some(result)
                            }
                            _ => {
                                // 再检查持久化缓存
                                match memory.check_persistent_cache(&action.tool, &action.args).await {
                                    Ok(Some(result)) => {
                                        log::info!("ReAct: Tool cache hit for {} (persistent cache)", action.tool);
                                        Some(result)
                                    }
                                    _ => None
                                }
                            }
                        }
                    } else {
                        None
                    };

                    // 执行工具（或使用缓存结果）
                    let observation_result = if let Some(cached) = cached_result {
                        Ok(cached)
                    } else {
                        tool_executor(action.clone()).await
                    };

                    // ✅ 工具执行后检查取消状态
                    if self.cancellation_token.is_cancelled() {
                        tracing::info!("❌ ReAct: Execution cancelled after tool execution (iteration {})", iteration);
                        let mut trace = self.trace.write().await;
                        trace.complete(ReactStatus::Cancelled);
                        trace.metrics.total_iterations = iteration;
                        trace.metrics.total_duration_ms = start_time
                            .elapsed()
                            .unwrap_or(Duration::from_secs(0))
                            .as_millis() as u64;
                        return Ok(trace.clone());
                    }

                    let action_duration = action_start
                        .elapsed()
                        .unwrap_or(Duration::from_secs(0))
                        .as_millis() as u64;

                    // === 步骤 5: Observation（工具返回） ===
                    match observation_result {
                        Ok(result) => {
                            // 发送工具执行结果到前端
                            if let Some(ref emitter) = emitter {
                                emitter.emit_tool_result(iteration, &action.tool, &action.args, &result, true, action_duration);
                            }

                            {
                                let mut trace = self.trace.write().await;
                                trace.add_step(ReactStep {
                                    id: format!("observation_{}", iteration),
                                    step_type: ReactStepType::Observation {
                                        tool_name: action.tool.clone(),
                                        result: result.clone(),
                                        success: true,
                                    },
                                    timestamp: SystemTime::now(),
                                    duration_ms: Some(action_duration),
                                    token_usage: None,
                                    error: None,
                                });
                                trace.metrics.successful_tool_calls += 1;
                            }

                            // === Memory 集成：缓存工具调用结果 ===
                            if let Some(ref memory) = self.config.memory_integration {
                                if let Err(e) = memory
                                    .cache_tool_result(&action.tool, &action.args, &result, action_duration)
                                    .await
                                {
                                    log::warn!("ReAct: Failed to cache tool result: {}", e);
                                }
                            }

                            // 添加到上下文历史
                            context_history.push(format!(
                                "Thought: {}\nAction: {}\nObservation: {}",
                                llm_output,
                                serde_json::to_string(&action).unwrap_or_default(),
                                serde_json::to_string(&result).unwrap_or_default()
                            ));
                        }
                        Err(e) => {
                            // 发送工具执行失败到前端
                            if let Some(ref emitter) = emitter {
                                emitter.emit_tool_result(iteration, &action.tool, &action.args, &serde_json::json!({"error": e.to_string()}), false, action_duration);
                            }

                            // 工具执行失败
                            {
                                let mut trace = self.trace.write().await;
                                trace.add_step(ReactStep {
                                    id: format!("observation_{}", iteration),
                                    step_type: ReactStepType::Observation {
                                        tool_name: action.tool.clone(),
                                        result: serde_json::json!({"error": e.to_string()}),
                                        success: false,
                                    },
                                    timestamp: SystemTime::now(),
                                    duration_ms: Some(action_duration),
                                    token_usage: None,
                                    error: Some(e.to_string()),
                                });
                                trace.metrics.failed_tool_calls += 1;
                            }

                            context_history.push(format!(
                                "Thought: {}\nAction: {}\nObservation: Tool execution failed: {}",
                                llm_output,
                                serde_json::to_string(&action).unwrap_or_default(),
                                e
                            ));
                        }
                    }
                }
                
                // ========== 新增执行模式 ==========
                
                ActionInstruction::ParallelTools { tools, aggregation } => {
                    // 并行执行多个工具
                    tracing::info!("ReAct: Executing {} parallel tools", tools.len());
                    let parallel_start = SystemTime::now();
                    
                    let result = self.execute_parallel_tools(&tools, &aggregation, &tool_executor).await;
                    
                    let parallel_duration = parallel_start
                        .elapsed()
                        .unwrap_or(Duration::from_secs(0))
                        .as_millis() as u64;
                    
                    match result {
                        Ok(parallel_result) => {
                            // 记录步骤
                            {
                                let mut trace = self.trace.write().await;
                                trace.add_step(ReactStep {
                                    id: format!("parallel_{}", iteration),
                                    step_type: ReactStepType::ParallelExecution {
                                        tool_calls: tools.clone(),
                                        results: parallel_result.clone(),
                                    },
                                    timestamp: parallel_start,
                                    duration_ms: Some(parallel_duration),
                                    token_usage: None,
                                    error: None,
                                });
                                trace.metrics.tool_calls_count += tools.len() as u32;
                                trace.metrics.successful_tool_calls += parallel_result.success_count as u32;
                                trace.metrics.failed_tool_calls += parallel_result.failure_count as u32;
                            }
                            
                            // 构建观察结果
                            context_history.push(format!(
                                "Thought: {}\nParallel Execution: {} tools ({} success, {} failed)\nResults: {}",
                                llm_output,
                                tools.len(),
                                parallel_result.success_count,
                                parallel_result.failure_count,
                                serde_json::to_string_pretty(&parallel_result.results).unwrap_or_default()
                            ));
                        }
                        Err(e) => {
                            tracing::error!("ReAct: Parallel execution failed: {}", e);
                            context_history.push(format!(
                                "Thought: {}\nParallel Execution Failed: {}",
                                llm_output,
                                e
                            ));
                        }
                    }
                }
                
                ActionInstruction::ReasoningChain { steps, solve_prompt } => {
                    // 执行推理链
                    tracing::info!("ReAct: Executing reasoning chain with {} steps", steps.len());
                    let chain_start = SystemTime::now();
                    
                    let result = self.execute_reasoning_chain(&steps, solve_prompt.as_deref(), &tool_executor, &llm_call).await;
                    
                    let chain_duration = chain_start
                        .elapsed()
                        .unwrap_or(Duration::from_secs(0))
                        .as_millis() as u64;
                    
                    match result {
                        Ok(chain_result) => {
                            // 记录步骤
                            {
                                let mut trace = self.trace.write().await;
                                trace.add_step(ReactStep {
                                    id: format!("reasoning_{}", iteration),
                                    step_type: ReactStepType::ReasoningChainExecution {
                                        steps: steps.clone(),
                                        result: chain_result.clone(),
                                    },
                                    timestamp: chain_start,
                                    duration_ms: Some(chain_duration),
                                    token_usage: None,
                                    error: None,
                                });
                                trace.metrics.tool_calls_count += steps.len() as u32;
                            }
                            
                            context_history.push(format!(
                                "Thought: {}\nReasoning Chain: {} steps\nVariables: {:?}\nFinal: {}",
                                llm_output,
                                steps.len(),
                                chain_result.variables.keys().collect::<Vec<_>>(),
                                chain_result.final_result.as_deref().unwrap_or("N/A")
                            ));
                        }
                        Err(e) => {
                            tracing::error!("ReAct: Reasoning chain failed: {}", e);
                            context_history.push(format!(
                                "Thought: {}\nReasoning Chain Failed: {}",
                                llm_output,
                                e
                            ));
                        }
                    }
                }
                
                ActionInstruction::PhaseExecution { phase, next_phase_hint } => {
                    // 执行阶段
                    tracing::info!("ReAct: Executing phase '{}'", phase.name);
                    let phase_start = SystemTime::now();
                    
                    let result = self.execute_phase(&phase, &tool_executor).await;
                    
                    let phase_duration = phase_start
                        .elapsed()
                        .unwrap_or(Duration::from_secs(0))
                        .as_millis() as u64;
                    
                    match result {
                        Ok(phase_result) => {
                            // 记录步骤
                            {
                                let mut trace = self.trace.write().await;
                                trace.add_step(ReactStep {
                                    id: format!("phase_{}_{}", phase.name, iteration),
                                    step_type: ReactStepType::PhaseExecution {
                                        phase: phase.clone(),
                                        result: phase_result.clone(),
                                    },
                                    timestamp: phase_start,
                                    duration_ms: Some(phase_duration),
                                    token_usage: None,
                                    error: None,
                                });
                                trace.metrics.tool_calls_count += phase.tool_calls.len() as u32;
                            }
                            
                            // 将阶段结果和下一步提示添加到上下文
                            let mut observation = format!(
                                "Phase '{}' completed with status: {:?}\nFindings: {} items\nHandoff data: {}",
                                phase.name,
                                phase_result.status,
                                phase_result.findings.len(),
                                serde_json::to_string_pretty(&phase_result.handoff_data).unwrap_or_default()
                            );
                            if let Some(hint) = next_phase_hint {
                                observation.push_str(&format!("\nNext phase hint: {}", hint));
                            }
                            
                            context_history.push(format!(
                                "Thought: {}\n{}",
                                llm_output,
                                observation
                            ));
                        }
                        Err(e) => {
                            tracing::error!("ReAct: Phase execution failed: {}", e);
                            context_history.push(format!(
                                "Thought: {}\nPhase '{}' Failed: {}",
                                llm_output,
                                phase.name,
                                e
                            ));
                        }
                    }
                }
                
                ActionInstruction::SubPlan { plan_description, steps, allow_replanning } => {
                    // 检查是否重复失败同一个计划
                    if let Some(ref last_plan) = last_failed_plan {
                        if last_plan == &plan_description {
                            consecutive_plan_failures += 1;
                            tracing::warn!(
                                "ReAct: Same plan '{}' failed again, consecutive failures: {}/{}",
                                plan_description, consecutive_plan_failures, MAX_CONSECUTIVE_PLAN_FAILURES
                            );
                        } else {
                            // 新计划，重置计数
                            consecutive_plan_failures = 1;
                        }
                    }
                    
                    // 检查是否超过最大连续失败次数
                    if consecutive_plan_failures >= MAX_CONSECUTIVE_PLAN_FAILURES {
                        tracing::error!(
                            "ReAct: Exceeded max consecutive plan failures ({}), stopping execution",
                            MAX_CONSECUTIVE_PLAN_FAILURES
                        );
                        
                        // 发送错误消息到前端
                        if let Some(ref emitter) = emitter {
                            emitter.emit_error(&format!(
                                "任务执行失败：计划 '{}' 连续失败 {} 次。可能的原因：\n\
                                - 所需工具不可用或连接已断开\n\
                                - 工具参数配置错误\n\
                                - 网络连接问题\n\n\
                                请检查工具配置后重试。",
                                plan_description, MAX_CONSECUTIVE_PLAN_FAILURES
                            ));
                        }
                        
                        // 更新 trace 状态
                        let mut trace = self.trace.write().await;
                        trace.complete(ReactStatus::Failed);
                        return Err(anyhow!(
                            "Exceeded max consecutive plan failures ({}) for plan: {}",
                            MAX_CONSECUTIVE_PLAN_FAILURES,
                            plan_description
                        ));
                    }
                    
                    // 执行子计划
                    tracing::info!("ReAct: Executing sub-plan '{}' with {} steps", plan_description, steps.len());
                    let plan_start = SystemTime::now();
                    
                    let result = self.execute_sub_plan(&plan_description, &steps, allow_replanning, &tool_executor, emitter.as_ref()).await;
                    
                    let plan_duration = plan_start
                        .elapsed()
                        .unwrap_or(Duration::from_secs(0))
                        .as_millis() as u64;
                    
                    match result {
                        Ok(completed_count) => {
                            // 记录步骤
                            {
                                let mut trace = self.trace.write().await;
                                trace.add_step(ReactStep {
                                    id: format!("subplan_{}", iteration),
                                    step_type: ReactStepType::SubPlanExecution {
                                        plan: plan_description.clone(),
                                        steps_completed: completed_count,
                                        steps_total: steps.len(),
                                        current_step: None,
                                    },
                                    timestamp: plan_start,
                                    duration_ms: Some(plan_duration),
                                    token_usage: None,
                                    error: None,
                                });
                            }
                            
                            // 检查是否全部完成
                            if completed_count == steps.len() {
                                // 全部完成，重置失败计数
                                consecutive_plan_failures = 0;
                                last_failed_plan = None;
                                
                                context_history.push(format!(
                                    "Thought: {}\nSub-plan '{}' completed successfully: {}/{} steps done",
                                    llm_output,
                                    plan_description,
                                    completed_count,
                                    steps.len()
                                ));
                            } else {
                                // 部分完成（有步骤失败）
                                last_failed_plan = Some(plan_description.clone());
                                
                                // 获取失败步骤的信息
                                let failed_step_idx = completed_count;
                                let failed_step_info = if failed_step_idx < steps.len() {
                                    let step = &steps[failed_step_idx];
                                    format!(
                                        "步骤 {} ('{}') 使用工具 '{}' 失败",
                                        step.id,
                                        step.description,
                                        step.tool.as_ref().map(|t| t.tool.as_str()).unwrap_or("unknown")
                                    )
                                } else {
                                    "未知步骤失败".to_string()
                                };
                                
                                context_history.push(format!(
                                    "Thought: {}\n\n⚠️ Sub-plan '{}' PARTIALLY FAILED:\n\
                                    - Completed: {}/{} steps\n\
                                    - Failed at: {}\n\n\
                                    ⚠️ IMPORTANT: The tool used in the failed step is not working. \
                                    You MUST create a NEW DIFFERENT plan using ALTERNATIVE tools. \
                                    Do NOT retry the same tool. Consider using playwright_navigate + playwright_get_visible_text instead.",
                                    llm_output,
                                    plan_description,
                                    completed_count,
                                    steps.len(),
                                    failed_step_info
                                ));
                            }
                        }
                        Err(e) => {
                            last_failed_plan = Some(plan_description.clone());
                            
                            tracing::error!("ReAct: Sub-plan execution failed: {}", e);
                            context_history.push(format!(
                                "Thought: {}\n\n❌ Sub-plan '{}' FAILED: {}\n\n\
                                ⚠️ IMPORTANT: You MUST create a COMPLETELY DIFFERENT plan using ALTERNATIVE tools. \
                                The previously used tool is not available. \
                                Use playwright tools (playwright_navigate, playwright_get_visible_text, etc.) as an alternative.",
                                llm_output,
                                plan_description,
                                e
                            ));
                        }
                    }
                }
            }

            // === Context Summarization：检查是否需要摘要 ===
            if let Some(ref summarizer) = summarizer {
                if summarizer.needs_summarization(context_history.len()) {
                    log::info!(
                        "ReAct: Context history exceeds threshold ({} > {}), performing summarization",
                        context_history.len(),
                        self.config.summarization_threshold
                    );

                    // 构建摘要提示词
                    let summary_prompt = summarizer.build_summarization_prompt(&context_history);

                    // 调用 LLM 生成摘要
                    match llm_call(
                        Some("You are a helpful assistant that summarizes reasoning steps.".to_string()),
                        summary_prompt,
                        true, // skip_save
                        String::new(),
                    )
                    .await
                    {
                        Ok(summary) => {
                            log::info!("ReAct: Context summarization completed");
                            // 应用摘要，保留最近的历史
                            let mut summarizer_mut = summarizer.clone();
                            summarizer_mut.apply_summary(&mut context_history, summary);
                        }
                        Err(e) => {
                            log::warn!("ReAct: Failed to summarize context: {}", e);
                            // 失败时简单截断历史
                            let keep = self.config.summarization_threshold / 2;
                            if context_history.len() > keep {
                                context_history.drain(..context_history.len() - keep);
                            }
                        }
                    }
                }
            }

            // 清除旧的 RAG 上下文（如果每次都注入，这里应重新获取）
            if self.config.react_config.enable_rag {
                if let Some(rag_cfg) = &self.config.react_config.rag_config {
                    if matches!(rag_cfg.injection_point, RagInjectionPoint::EveryThought) {
                        rag_context = self.fetch_rag_context(&task).await?;
                    }
                }
            }
        }
    }

    /// 构建 Thought 阶段的提示词
    /// 返回: (system_prompt, user_prompt)
    async fn build_thought_prompt(
        &self,
        task: &str,
        history: &[String],
        rag_context: &str,
        few_shot_examples: &str,
    ) -> (Option<String>, String) {
        let mut system_prompt;
        let mut user_prompt = String::new();

        // 尝试从数据库加载提示词模板
        if let Some(repo) = &self.config.prompt_repo {
            if let Ok(Some(template)) = repo
                .get_template_by_arch_stage(ArchitectureType::ReAct, StageType::Planning)
                .await
            {
                // 使用数据库中的模板作为 system prompt
                system_prompt = template.content.clone();

                // 构建工具列表并替换 {tools} 占位符
                let tools_block = self.build_tools_information().await;
                system_prompt = system_prompt.replace("{tools}", &tools_block);

                // 清理多余的空行
                while system_prompt.contains("\n\n\n") {
                    system_prompt = system_prompt.replace("\n\n\n", "\n\n");
                }
                system_prompt = system_prompt.trim().to_string();

                // 集成角色提示词到 system prompt（如果存在）
                if let Some(params) = &self.config.task_parameters {
                    if let Some(role_prompt) = params.get("role_prompt").and_then(|v| v.as_str()) {
                        if !role_prompt.trim().is_empty() {
                            system_prompt = format!("{}\n\n{}", role_prompt, system_prompt);
                            log::info!("ReAct executor: integrated role prompt into system prompt");
                        }
                    }
                }

                // 构建 user prompt
                user_prompt.push_str(&format!("用户问题: {}", task));

                // 注入 Few-shot 示例（来自 Memory）
                if !few_shot_examples.is_empty() {
                    user_prompt.push_str(few_shot_examples);
                }

                // 注入 RAG 证据到 user prompt
                if !rag_context.is_empty() {
                    user_prompt.push_str("\n=== Evidence from Knowledge Base ===\n");
                    user_prompt.push_str(rag_context);
                    user_prompt.push_str("\n\n");
                }

                // 添加历史上下文到 user prompt
                if !history.is_empty() {
                    user_prompt.push_str("\n=== 前置步骤 ===\n");
                    for (idx, h) in history.iter().enumerate() {
                        user_prompt.push_str(&format!("Step {}:\n{}\n\n", idx + 1, h));
                    }
                    // 在有历史时，添加明确的提示引导下一步思考
                    user_prompt.push_str(
                        "=== Your Turn ===\n基于之前的步骤，你的下一步思考和行动是什么？\n",
                    );
                } else {
                    // 首次思考时的提示
                    user_prompt.push_str("\n=== Your Turn ===\n你有什么想法和行动？\n");
                }

                return (Some(system_prompt), user_prompt);
            }
        }

        // 没有找到数据库模板时的默认行为
        // 构建默认的 system prompt（包含工具列表和说明）
        let tools_block = self.build_tools_information().await;
        system_prompt = format!(
            "You are a helpful AI assistant using the ReAct (Reasoning + Acting) framework.\n\
            You can use the following tools:\n{}\n\n\
            Response Format:\n\
            You should respond with your thoughts and actions in the following format:\n\n\
            Thought: [Your reasoning about what to do next]\n\
            Action: [tool_name]\n\
            Action Input: {{\"key\": \"value\"}}\n\n\
            When you have enough information to answer, respond with:\n\
            Thought: [Your final reasoning]\n\
            Final Answer: [Your complete answer to the task]\n\n\
            Important Notes:\n\
            - Think step-by-step before taking action\n\
            - Use tools when you need external information or capabilities\n\
            - Cite sources when available\n\
            - Provide clear final answers",
            tools_block
        );

        // User prompt 只包含任务
        user_prompt.push_str(&format!("Task: {}\n\n", task));

        // 注入 Few-shot 示例
        if !few_shot_examples.is_empty() {
            user_prompt.push_str(few_shot_examples);
        }

        return (Some(system_prompt), user_prompt);
    }

    /// 构建工具信息块（参考 Plan-and-Execute 的实现）
    async fn build_tools_information(&self) -> String {
        use crate::tools::ToolInfo;
        use std::collections::{HashMap, HashSet};

        // 读取任务参数中的工具白名单/黑名单
        let (allow, allow_present, deny): (HashSet<String>, bool, HashSet<String>) =
            if let Some(params) = &self.config.task_parameters {
                log::info!("ReAct executor: task_parameters = {:?}", params);
                let allow_present = params.get("tools_allow").is_some();
                let allow = params
                    .get("tools_allow")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_else(HashSet::new);
                let deny = params
                    .get("tools_deny")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_else(HashSet::new);
                (allow, allow_present, deny)
            } else {
                log::warn!("ReAct executor: task_parameters is None!");
                (HashSet::new(), false, HashSet::new())
            };

        // 语义约定：当前端显式传入 tools_allow 但为空数组 ⇒ 严格模式：禁用所有工具
        if allow_present && allow.is_empty() {
            log::info!("ReAct executor: 检测到显式空白名单 => 禁用所有工具");
            return "No tools available".to_string();
        }

        log::debug!(
            "ReAct executor: 工具过滤配置 - 白名单: {:?}, 黑名单: {:?}",
            if allow_present && allow.is_empty() {
                "空(禁用所有)".to_string()
            } else if allow.is_empty() {
                "未配置(允许所有)".to_string()
            } else {
                format!("{:?}", allow)
            },
            if deny.is_empty() {
                "未配置".to_string()
            } else {
                format!("{:?}", deny)
            }
        );

        let mut all_tools: Vec<ToolInfo> = Vec::new();

        // 从框架适配器获取工具
        if let Some(framework_adapter) = &self.config.framework_adapter {
            let available_tools = framework_adapter.list_available_tools().await;
            // log::info!(
            //     "ReAct executor: 框架适配器提供了 {} 个工具 => {:?}",
            //     available_tools.len(),
            //     available_tools
            // );

            for tool_name in available_tools {
                // 兼容：某些插件工具在 ToolInfo 中可能存储去掉前缀的 id（如 "test_1"），
                // 而白名单里是 "plugin::test_1"。这里做前缀匹配补偿。
                let mut whitelist_hit = allow.contains(&tool_name);
                let plugin_prefixed_candidate = format!("plugin::{}", tool_name);
                let prefixed_whitelist_hit = allow.contains(&plugin_prefixed_candidate);
                let _is_plugin = prefixed_whitelist_hit || tool_name.starts_with("plugin::");

                // 过滤白名单/黑名单（与 Plan-and-Execute 保持一致）
                // 如果有白名单且工具不在白名单中，跳过
                if !allow.is_empty() {
                    // 如果直接命中或前缀命中，则视为命中
                    whitelist_hit = whitelist_hit || prefixed_whitelist_hit;
                    if !whitelist_hit {
                        continue;
                    }
                }
                // 如果工具在黑名单中，跳过
                if deny.contains(&tool_name) {
                    log::debug!(
                        "ReAct executor: 工具 '{}' 在黑名单中，跳过 (deny={:?})",
                        tool_name, deny
                    );
                    continue;
                }
                match framework_adapter.get_tool_info(&tool_name).await {
                    Some(tool_info) => {
                        // 如果白名单里仅存在带前缀形式，且当前工具名无前缀，但该工具属于被动扫描（tags 含 passive），
                        // 则不应用前缀补偿，避免 passive 的 "test_params" 被误当成 "plugin::test_params" 覆盖 agent 工具。
                        if prefixed_whitelist_hit
                            && !tool_info.name.starts_with("plugin::")
                            && tool_info.metadata.tags.iter().any(|t| t == "passive")
                        {
                            log::debug!(
                                "ReAct executor: 跳过对被动工具 '{}' 的前缀补偿 (候选='{}')",
                                tool_info.name,
                                plugin_prefixed_candidate
                            );
                            // 放弃该工具，继续后续
                            continue;
                        }
                        // 如果白名单里仅存在带前缀形式，且当前工具名无前缀，则在 system prompt 展示时补前缀
                        let effective_name = if !tool_info.name.starts_with("plugin::") && prefixed_whitelist_hit {
                            plugin_prefixed_candidate.clone()
                        } else {
                            tool_info.name.clone()
                        };
                        log::debug!(
                            "ReAct executor: 接收工具 '{}' => effective='{}' (available={}, source={:?}, plugin_fix={})",
                            tool_info.name,
                            effective_name,
                            tool_info.available,
                            tool_info.source,
                            if effective_name != tool_info.name { "applied" } else { "none" }
                        );
                        // 在 ToolInfo 进入后续去重前调整其 name（仅影响 system prompt 展示，不改原对象其他字段）
                        let mut adjusted = tool_info;
                        if effective_name != adjusted.name {
                            // 复制并覆盖 name 字段
                            adjusted.name = effective_name;
                        }
                        all_tools.push(adjusted);
                    }
                    None => {
                        log::warn!(
                            "ReAct executor: list_available_tools() 包含 '{}' 但 get_tool_info 返回 None",
                            tool_name
                        );
                    }
                }
            }
        }

        log::info!("ReAct executor: 所有工具（包括MCP工具）已通过框架适配器统一获取");

        // 去重工具（按名称）
        let mut unique_tools: HashMap<String, ToolInfo> = HashMap::new();
        for tool in all_tools {
            let existed = unique_tools.contains_key(&tool.name);
            if existed {
                log::debug!("ReAct executor: 去重丢弃重复工具 '{}'", tool.name);
            }
            unique_tools.entry(tool.name.clone()).or_insert(tool);
        }

        let tool_infos: Vec<&ToolInfo> = unique_tools.values().collect();

        if tool_infos.is_empty() {
            log::warn!("ReAct executor: 没有找到任何可用工具 (unique_tools.size={})", unique_tools.len());
            return "No tools available".to_string();
        }

        log::info!(
            "ReAct executor: 构建工具信息，共 {} 个工具",
            tool_infos.len()
        );
        let mut tool_lines: Vec<String> = Vec::new();
        for info in &tool_infos {
            // 构建工具参数签名
            let mut parts: Vec<String> = Vec::new();
            for param in &info.parameters.parameters {
                let param_type = match param.param_type {
                    crate::tools::ParameterType::String => "string",
                    crate::tools::ParameterType::Number => "number",
                    crate::tools::ParameterType::Boolean => "boolean",
                    crate::tools::ParameterType::Array => "array",
                    crate::tools::ParameterType::Object => "object",
                };
                let param_str = if param.required {
                    format!("{}: {}", param.name, param_type)
                } else {
                    format!("{}?: {}", param.name, param_type)
                };
                parts.push(param_str);
            }

            let signature = if parts.is_empty() {
                String::new()
            } else {
                parts.join(", ")
            };

            tool_lines.push(format!("- {}({}) - {}", info.name, signature, info.description));
        }
        tool_lines.join("\n")
    }

    /// 格式化 Few-shot 推理链示例
    fn format_few_shot_examples(
        &self,
        examples: &[super::memory_integration::ReasoningChainExample],
    ) -> String {
        if examples.is_empty() {
            return String::new();
        }

        let mut result = String::from("\n=== Similar Task Examples from History ===\n");
        
        for (idx, example) in examples.iter().enumerate() {
            // 安全截断，确保不在 UTF-8 字符中间切断
            let task_preview = {
                let max = 200.min(example.task.len());
                let mut end = max;
                while end > 0 && !example.task.is_char_boundary(end) {
                    end -= 1;
                }
                &example.task[..end]
            };
            result.push_str(&format!(
                "\nExample {}: Task: {}\n",
                idx + 1,
                task_preview
            ));
            result.push_str(&format!("Steps: {}\n", example.steps_summary));
            if let Some(ref answer) = example.final_answer {
                let preview = if answer.len() > 300 {
                    let mut end = 300;
                    while end > 0 && !answer.is_char_boundary(end) {
                        end -= 1;
                    }
                    format!("{}...", &answer[..end])
                } else {
                    answer.clone()
                };
                result.push_str(&format!("Result: {}\n", preview));
            }
            result.push_str(&format!(
                "Success Rate: {:.0}%, Similarity: {:.0}%\n",
                example.success_rate * 100.0,
                example.similarity_score * 100.0
            ));
        }
        
        result.push_str("\n=== End of Examples ===\n");
        result
    }

    /// 获取 RAG 上下文
    async fn fetch_rag_context(&self, query: &str) -> Result<String> {
        use crate::commands::rag_commands::get_global_rag_service;
        use sentinel_rag::models::AssistantRagRequest;

        // 获取 RAG 配置
        let rag_cfg = match &self.config.react_config.rag_config {
            Some(cfg) => cfg,
            None => return Ok(String::new()),
        };

        // 获取全局 RAG 服务
        let rag_service = match get_global_rag_service().await {
            Ok(service) => service,
            Err(e) => {
                log::warn!("ReAct: Failed to get RAG service: {}", e);
                return Ok(String::new());
            }
        };

        // 构建 RAG 请求
        let rag_req = AssistantRagRequest {
            query: query.to_string(),
            collection_id: None,
            conversation_history: None,
            top_k: Some(rag_cfg.top_k),
            use_mmr: Some(rag_cfg.use_mmr),
            mmr_lambda: Some(rag_cfg.mmr_lambda),
            similarity_threshold: Some(rag_cfg.similarity_threshold),
            reranking_enabled: Some(false),
            model_provider: None,
            model_name: None,
            max_tokens: None,
            temperature: None,
            system_prompt: None,
        };

        // 执行 RAG 查询
        match rag_service.query_for_assistant(&rag_req).await {
            Ok((context, citations)) => {
                if context.trim().is_empty() {
                    log::debug!("ReAct: RAG query returned empty context");
                    Ok(String::new())
                } else {
                    log::info!(
                        "ReAct: RAG query returned {} chars context with {} citations",
                        context.len(),
                        citations.len()
                    );
                    Ok(context)
                }
            }
            Err(e) => {
                log::warn!("ReAct: RAG query failed: {}", e);
                Ok(String::new())
            }
        }
    }

    /// 获取当前轨迹快照
    pub async fn get_trace(&self) -> ReactTrace {
        self.trace.read().await.clone()
    }

    // ========== 新增执行模式方法 ==========

    /// 并行执行多个工具
    async fn execute_parallel_tools<Ft>(
        &self,
        tools: &[ParallelToolCall],
        aggregation: &AggregationStrategy,
        tool_executor: &Ft,
    ) -> Result<ParallelExecutionResult>
    where
        Ft: Fn(ReactToolCall)
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send>>
            + Send
            + Sync,
    {
        use std::time::Instant;
        
        let start = Instant::now();
        let mut results = HashMap::new();
        let mut success_count = 0;
        let mut failure_count = 0;
        
        // 构建依赖图并按依赖顺序执行
        let mut completed: HashMap<String, serde_json::Value> = HashMap::new();
        let mut pending: Vec<&ParallelToolCall> = tools.iter().collect();
        
        while !pending.is_empty() {
            // 找出可以执行的任务（依赖已满足）
            let (ready, not_ready): (Vec<_>, Vec<_>) = pending.into_iter()
                .partition(|t| t.depends_on.iter().all(|dep| completed.contains_key(dep)));
            
            if ready.is_empty() && !not_ready.is_empty() {
                // 存在循环依赖或无法满足的依赖
                return Err(anyhow!("Circular or unsatisfiable dependencies detected"));
            }
            
            // 并行执行所有就绪的任务
            let mut futures = Vec::new();
            
            for call in &ready {
                let args = self.substitute_variables(&call.args, &completed);
                let tool_call = ReactToolCall {
                    tool: call.tool.clone(),
                    args,
                    call_id: call.id.clone(),
                    is_parallel: true,
                };
                
                let call_id = call.id.clone();
                let tool_name = call.tool.clone();
                let future = tool_executor(tool_call);
                
                futures.push(async move {
                    let call_start = Instant::now();
                    let result = future.await;
                    let duration = call_start.elapsed().as_millis() as u64;
                    (call_id, tool_name, result, duration)
                });
            }
            
            // 等待所有并行任务完成
            let task_results = futures::future::join_all(futures).await;
            
            for (id, tool_name, result, duration) in task_results {
                match result {
                    Ok(value) => {
                        completed.insert(id.clone(), value.clone());
                        results.insert(id, ToolExecutionResult {
                            tool_name,
                            success: true,
                            result: value,
                            error: None,
                            duration_ms: duration,
                        });
                        success_count += 1;
                    }
                    Err(e) => {
                        results.insert(id, ToolExecutionResult {
                            tool_name,
                            success: false,
                            result: serde_json::Value::Null,
                            error: Some(e.to_string()),
                            duration_ms: duration,
                        });
                        failure_count += 1;
                        
                        // 根据聚合策略决定是否继续
                        if matches!(aggregation, AggregationStrategy::WaitAll) {
                            return Err(e);
                        }
                    }
                }
            }
            
            pending = not_ready;
        }
        
        Ok(ParallelExecutionResult {
            results,
            total_duration_ms: start.elapsed().as_millis() as u64,
            success_count,
            failure_count,
        })
    }

    /// 执行推理链（ReWOO 风格）
    async fn execute_reasoning_chain<F, Ft>(
        &self,
        steps: &[ReasoningStep],
        solve_prompt: Option<&str>,
        tool_executor: &Ft,
        llm_call: &F,
    ) -> Result<ReasoningChainResult>
    where
        F: Fn(Option<String>, String, bool, String)
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>
            + Send
            + Sync,
        Ft: Fn(ReactToolCall)
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send>>
            + Send
            + Sync,
    {
        let mut variables: HashMap<String, serde_json::Value> = HashMap::new();
        let mut trace = Vec::new();
        
        // 按顺序执行每个步骤
        for step in steps {
            let start = std::time::Instant::now();
            
            // 替换参数中的变量引用
            let substituted_args = self.substitute_variables_in_string(&step.args, &variables);
            
            // 解析参数为 JSON
            let args: serde_json::Value = serde_json::from_str(&substituted_args)
                .unwrap_or_else(|_| serde_json::json!({ "input": substituted_args }));
            
            // 执行工具
            let tool_call = ReactToolCall {
                tool: step.tool.clone(),
                args,
                call_id: step.variable.clone(),
                is_parallel: false,
            };
            
            tracing::info!("ReAct: Executing reasoning step {} with tool {}", step.variable, step.tool);
            let result = tool_executor(tool_call).await;
            let duration = start.elapsed().as_millis() as u64;
            
            match result {
                Ok(value) => {
                    variables.insert(step.variable.clone(), value.clone());
                    trace.push(ReasoningStepResult {
                        variable: step.variable.clone(),
                        tool: step.tool.clone(),
                        substituted_args,
                        result: value,
                        success: true,
                        duration_ms: duration,
                    });
                }
                Err(e) => {
                    tracing::warn!("ReAct: Reasoning step {} failed: {}", step.variable, e);
                    trace.push(ReasoningStepResult {
                        variable: step.variable.clone(),
                        tool: step.tool.clone(),
                        substituted_args,
                        result: serde_json::json!({"error": e.to_string()}),
                        success: false,
                        duration_ms: duration,
                    });
                    // 继续执行，让后续步骤可以处理错误
                    variables.insert(step.variable.clone(), serde_json::json!({"error": e.to_string()}));
                }
            }
        }
        
        // 如果有 solve_prompt，调用 LLM 生成最终结果
        let final_result = if let Some(prompt) = solve_prompt {
            let context = format!(
                "Based on the following reasoning chain results:\n{}\n\n{}",
                serde_json::to_string_pretty(&variables).unwrap_or_default(),
                prompt
            );
            
            match llm_call(None, context, true, String::new()).await {
                Ok(answer) => Some(answer),
                Err(e) => {
                    tracing::warn!("ReAct: Solve step failed: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        Ok(ReasoningChainResult {
            variables,
            final_result,
            trace,
        })
    }

    /// 执行阶段
    async fn execute_phase<Ft>(
        &self,
        phase: &PhaseDefinition,
        tool_executor: &Ft,
    ) -> Result<PhaseExecutionResult>
    where
        Ft: Fn(ReactToolCall)
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send>>
            + Send
            + Sync,
    {
        let start = std::time::Instant::now();
        
        // 并行执行阶段内的所有工具
        let parallel_result = self.execute_parallel_tools(
            &phase.tool_calls,
            &AggregationStrategy::Merge,
            tool_executor,
        ).await?;
        
        // 从结果中提取发现
        let findings = self.extract_findings_from_results(&parallel_result, &phase.phase_type);
        
        // 构建 handoff 数据
        let handoff_data = self.build_handoff_data(&parallel_result, &phase.phase_type);
        
        let status = if parallel_result.failure_count == 0 {
            PhaseStatus::Completed
        } else if parallel_result.success_count > 0 {
            PhaseStatus::PartiallyCompleted
        } else {
            PhaseStatus::Failed
        };
        
        Ok(PhaseExecutionResult {
            phase_name: phase.name.clone(),
            phase_type: phase.phase_type.clone(),
            status,
            findings,
            duration_ms: start.elapsed().as_millis() as u64,
            handoff_data,
        })
    }

    /// 执行子计划
    async fn execute_sub_plan<Ft>(
        &self,
        plan_description: &str,
        steps: &[SubPlanStep],
        allow_replanning: bool,
        tool_executor: &Ft,
        emitter: Option<&Arc<ReactMessageEmitter>>,
    ) -> Result<usize>
    where
        Ft: Fn(ReactToolCall)
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send>>
            + Send
            + Sync,
    {
        let mut completed_steps = 0;
        let mut step_results: HashMap<String, serde_json::Value> = HashMap::new();
        let total_steps = steps.len();
        
        for step in steps {
            // 检查依赖
            let deps_satisfied = step.depends_on.iter()
                .all(|dep| step_results.contains_key(dep));
            
            if !deps_satisfied {
                if step.skippable {
                    tracing::info!("ReAct: Skipping step '{}' due to unsatisfied dependencies", step.id);
                    continue;
                } else {
                    return Err(anyhow!("Dependencies not satisfied for step: {}", step.id));
                }
            }
            
            // 执行步骤
            if let Some(tool_call) = &step.tool {
                tracing::info!("ReAct: Executing sub-plan step '{}': {}", step.id, step.description);
                
                // 发送步骤开始执行的进度更新
                if let Some(em) = emitter {
                    em.emit_step_progress(
                        &step.id,
                        &step.description,
                        "running",
                        completed_steps,
                        total_steps,
                    );
                }
                
                // 替换工具参数中的变量
                let args = self.substitute_variables(&tool_call.args, &step_results);
                let actual_call = ReactToolCall {
                    tool: tool_call.tool.clone(),
                    args,
                    call_id: step.id.clone(),
                    is_parallel: false,
                };
                
                match tool_executor(actual_call).await {
                    Ok(result) => {
                        step_results.insert(step.id.clone(), result);
                        completed_steps += 1;
                        tracing::info!(
                            "ReAct: Step '{}' ({}) completed successfully",
                            step.id, step.description
                        );
                        
                        // 发送步骤进度更新
                        if let Some(em) = emitter {
                            em.emit_step_progress(
                                &step.id,
                                &step.description,
                                "completed",
                                completed_steps,
                                total_steps,
                            );
                        }
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        tracing::error!(
                            "ReAct: Step '{}' ({}) failed with tool '{}': {}",
                            step.id, step.description, tool_call.tool, error_msg
                        );
                        
                        // 发送步骤失败的进度更新
                        if let Some(em) = emitter {
                            em.emit_step_progress(
                                &step.id,
                                &format!("{} - 失败: {}", step.description, error_msg),
                                "failed",
                                completed_steps,
                                total_steps,
                            );
                        }
                        
                        if step.skippable {
                            tracing::warn!("ReAct: Skippable step '{}' failed, continuing: {}", step.id, error_msg);
                            continue;
                        } else if allow_replanning {
                            tracing::info!(
                                "ReAct: Step '{}' failed (tool: '{}'), returning to main loop for replanning. Error: {}",
                                step.id, tool_call.tool, error_msg
                            );
                            // 返回当前完成的步骤数，让主循环决定是否重规划
                            return Ok(completed_steps);
                        } else {
                            return Err(anyhow!(
                                "Step '{}' failed: tool '{}' returned error: {}",
                                step.id, tool_call.tool, error_msg
                            ));
                        }
                    }
                }
            } else {
                // 无工具的步骤直接标记完成
                step_results.insert(step.id.clone(), serde_json::Value::Null);
                completed_steps += 1;
                
                // 发送步骤进度更新（无工具步骤）
                if let Some(em) = emitter {
                    em.emit_step_progress(
                        &step.id,
                        &step.description,
                        "completed",
                        completed_steps,
                        total_steps,
                    );
                }
            }
        }
        
        // 发送计划完成的总体进度
        if completed_steps == total_steps {
            if let Some(em) = emitter {
                em.emit_content(
                    &format!("\n✅ **计划完成**: '{}' - 所有 {} 个步骤已成功执行\n", plan_description, total_steps),
                    false,
                );
            }
        }
        
        Ok(completed_steps)
    }

    /// 变量替换（用于并行执行）
    fn substitute_variables(
        &self,
        args: &serde_json::Value,
        completed: &HashMap<String, serde_json::Value>,
    ) -> serde_json::Value {
        let args_str = serde_json::to_string(args).unwrap_or_default();
        let substituted = self.substitute_variables_in_string(&args_str, completed);
        serde_json::from_str(&substituted).unwrap_or_else(|_| args.clone())
    }

    /// 字符串中的变量替换
    fn substitute_variables_in_string(
        &self,
        input: &str,
        variables: &HashMap<String, serde_json::Value>,
    ) -> String {
        let mut result = input.to_string();
        
        // 替换 #E1, #E2 等变量引用
        for (var_name, value) in variables {
            let placeholder = var_name.clone();
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                _ => serde_json::to_string(&value).unwrap_or_default(),
            };
            result = result.replace(&placeholder, &value_str);
        }
        
        // 替换 $1, $2 等变量引用
        for (var_name, value) in variables {
            if var_name.starts_with("#E") {
                if let Some(num) = var_name.strip_prefix("#E") {
                    let dollar_placeholder = format!("${}", num);
                    let value_str = match value {
                        serde_json::Value::String(s) => s.clone(),
                        _ => serde_json::to_string(&value).unwrap_or_default(),
                    };
                    result = result.replace(&dollar_placeholder, &value_str);
                }
            }
        }
        
        // 替换 $step_id 形式的变量
        for (var_name, value) in variables {
            let dollar_placeholder = format!("${}", var_name);
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                _ => serde_json::to_string(&value).unwrap_or_default(),
            };
            result = result.replace(&dollar_placeholder, &value_str);
        }
        
        result
    }

    /// 从结果中提取发现
    fn extract_findings_from_results(
        &self,
        results: &ParallelExecutionResult,
        phase_type: &PhaseType,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();
        
        for (_id, result) in &results.results {
            if !result.success {
                continue;
            }
            
            // 根据阶段类型解析结果
            match phase_type {
                PhaseType::Reconnaissance => {
                    // 提取域名、IP 等
                    if let Some(domains) = result.result.get("domains") {
                        if let Some(arr) = domains.as_array() {
                            for domain in arr {
                                if let Some(d) = domain.as_str() {
                                    findings.push(Finding {
                                        finding_type: "subdomain".to_string(),
                                        target: d.to_string(),
                                        details: serde_json::json!({}),
                                        severity: "info".to_string(),
                                        source_tool: result.tool_name.clone(),
                                    });
                                }
                            }
                        }
                    }
                    if let Some(ips) = result.result.get("ips") {
                        if let Some(arr) = ips.as_array() {
                            for ip in arr {
                                if let Some(i) = ip.as_str() {
                                    findings.push(Finding {
                                        finding_type: "ip_address".to_string(),
                                        target: i.to_string(),
                                        details: serde_json::json!({}),
                                        severity: "info".to_string(),
                                        source_tool: result.tool_name.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
                PhaseType::Scanning => {
                    // 提取开放端口、漏洞等
                    if let Some(ports) = result.result.get("open_ports") {
                        if let Some(arr) = ports.as_array() {
                            for port in arr {
                                findings.push(Finding {
                                    finding_type: "open_port".to_string(),
                                    target: port.to_string(),
                                    details: port.clone(),
                                    severity: "info".to_string(),
                                    source_tool: result.tool_name.clone(),
                                });
                            }
                        }
                    }
                    if let Some(vulns) = result.result.get("vulnerabilities") {
                        if let Some(arr) = vulns.as_array() {
                            for vuln in arr {
                                findings.push(Finding {
                                    finding_type: "vulnerability".to_string(),
                                    target: vuln.get("target")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or("unknown")
                                        .to_string(),
                                    details: vuln.clone(),
                                    severity: vuln.get("severity")
                                        .and_then(|s| s.as_str())
                                        .unwrap_or("medium")
                                        .to_string(),
                                    source_tool: result.tool_name.clone(),
                                });
                            }
                        }
                    }
                }
                PhaseType::Validation | PhaseType::Exploitation => {
                    // 直接将成功的结果作为发现
                    findings.push(Finding {
                        finding_type: format!("{:?}_result", phase_type),
                        target: result.tool_name.clone(),
                        details: result.result.clone(),
                        severity: "info".to_string(),
                        source_tool: result.tool_name.clone(),
                    });
                }
                _ => {
                    // 通用处理
                    findings.push(Finding {
                        finding_type: "generic".to_string(),
                        target: result.tool_name.clone(),
                        details: result.result.clone(),
                        severity: "info".to_string(),
                        source_tool: result.tool_name.clone(),
                    });
                }
            }
        }
        
        findings
    }

    /// 构建 handoff 数据
    fn build_handoff_data(
        &self,
        results: &ParallelExecutionResult,
        _phase_type: &PhaseType,
    ) -> serde_json::Value {
        // 将所有成功结果聚合
        let mut handoff = serde_json::Map::new();
        
        for (id, result) in &results.results {
            if result.success {
                handoff.insert(id.clone(), result.result.clone());
            }
        }
        
        serde_json::Value::Object(handoff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_executor_creation() {
        let config = ReactExecutorConfig {
            react_config: ReactConfig::default(),
            enable_streaming: false,
            conversation_id: None,
            message_id: None,
            execution_id: None,
            app_handle: None,
            prompt_repo: None,
            framework_adapter: None,
            task_parameters: None,
            cancellation_token: None,
            memory_integration: None,
            summarization_threshold: 0,
            emitter: None,
        };
        let executor = ReactExecutor::new("Test task".to_string(), config);
        let trace = executor.get_trace().await;
        assert_eq!(trace.task, "Test task");
        assert_eq!(trace.status, ReactStatus::Running);
    }

    #[tokio::test]
    async fn test_executor_with_memory() {
        use crate::engines::memory::IntelligentMemory;

        let memory = Arc::new(RwLock::new(IntelligentMemory::new()));
        let memory_integration = Arc::new(ReactMemoryIntegration::new(memory));

        let config = ReactExecutorConfig {
            react_config: ReactConfig::default(),
            enable_streaming: false,
            conversation_id: None,
            message_id: None,
            execution_id: None,
            app_handle: None,
            prompt_repo: None,
            framework_adapter: None,
            task_parameters: None,
            cancellation_token: None,
            memory_integration: Some(memory_integration),
            summarization_threshold: 8,
            emitter: None,
        };
        let executor = ReactExecutor::new("Test task with memory".to_string(), config);
        let trace = executor.get_trace().await;
        assert_eq!(trace.task, "Test task with memory");
    }
}

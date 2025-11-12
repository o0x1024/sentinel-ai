//! ReAct 执行器
//!
//! 实现核心循环：Thought → Action → Observation → 收敛判定

use super::parser::ActionParser;
use super::types::*;
use crate::services::prompt_db::PromptRepository;
use crate::utils::ordered_message::{emit_message_chunk, ChunkType};
use anyhow::{anyhow, Context, Result};
use sentinel_core::models::prompt::{ArchitectureType, StageType};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// ReAct 执行器配置
#[derive(Debug, Clone)]
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
}

/// ReAct 执行器
pub struct ReactExecutor {
    config: ReactExecutorConfig,
    trace: Arc<RwLock<ReactTrace>>,
}

impl ReactExecutor {
    /// 创建新的执行器
    pub fn new(task: String, config: ReactExecutorConfig) -> Self {
        let trace = ReactTrace::new(task);
        Self {
            config,
            trace: Arc::new(RwLock::new(trace)),
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

        // 可选：首次思考前注入 RAG 证据
        let mut rag_context = String::new();
        if self.config.react_config.enable_rag {
            if let Some(rag_cfg) = &self.config.react_config.rag_config {
                if matches!(rag_cfg.injection_point, RagInjectionPoint::Initial) {
                    // TODO: 实际调用 RAG 服务获取证据
                    rag_context = self.fetch_rag_context(&task).await?;
                }
            }
        }

        loop {
            iteration += 1;

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
                .build_thought_prompt(&task, &context_history, &rag_context)
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

            // 发送流式消息（Thought）
            if self.config.enable_streaming {
                if let Some(app) = &self.config.app_handle {
                    let trace = self.trace.read().await;
                    let message_id = self.config.message_id.clone().unwrap_or_else(|| trace.trace_id.clone());
                    let execution_id = self.config.execution_id.clone().unwrap_or_else(|| trace.trace_id.clone());
                    emit_message_chunk(
                        app,
                        &execution_id,  // 使用统一的 execution_id
                        &message_id,
                        self.config.conversation_id.as_deref(),
                        ChunkType::Thinking,
                        &llm_output,
                        false,
                        Some("react"),
                        None,
                    );
                }
            }

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

                    // 获取 message_id、execution_id、trace_id 和 conversation_id 用于发送消息
                    // 优先使用前端传入的 message_id 和 execution_id，否则回退到 trace_id
                    let (message_id, execution_id, trace_id, conversation_id) = {
                        let trace = self.trace.read().await;
                        let msg_id = self.config.message_id.clone().unwrap_or_else(|| trace.trace_id.clone());
                        let exec_id = self.config.execution_id.clone().unwrap_or_else(|| trace.trace_id.clone());
                        (msg_id, exec_id, trace.trace_id.clone(), self.config.conversation_id.clone())
                    };

                    // 发送最终完成标记（不发送内容，因为内容已经通过 LLM 流式输出发送过了）
                    if self.config.enable_streaming {
                        if let Some(app) = &self.config.app_handle {
                            tracing::info!(
                                "📤 ReAct: Emitting Final completion marker with is_final=true, execution_id={}, message_id={}", 
                                execution_id, message_id
                            );
                            emit_message_chunk(
                                app,
                                &execution_id,  // 使用统一的 execution_id
                                &message_id,
                                conversation_id.as_deref(),
                                ChunkType::Meta,  // 改为 Meta 类型，表示这是元数据标记
                                "",  // 空内容，不重复发送
                                true, // 这是最终消息标记
                                Some("react"),
                                None,
                            );
                        }
                    }

                    // 更新 trace 状态
                    let mut trace = self.trace.write().await;
                    trace.add_step(ReactStep {
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
                    trace.complete(ReactStatus::Completed);
                    trace.metrics.total_iterations = iteration;
                    trace.metrics.total_duration_ms = start_time
                        .elapsed()
                        .unwrap_or(Duration::from_secs(0))
                        .as_millis() as u64;

                    return Ok(trace.clone());
                }
                ActionInstruction::ToolCall { action, .. } => {
                    // === 步骤 4: Action（工具调用） ===
                    let action_start = SystemTime::now();

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

                    // 发送工具调用流式消息
                    if self.config.enable_streaming {
                        if let Some(app) = &self.config.app_handle {
                            let trace = self.trace.read().await;
                            let message_id = self.config.message_id.clone().unwrap_or_else(|| trace.trace_id.clone());
                            let execution_id = self.config.execution_id.clone().unwrap_or_else(|| trace.trace_id.clone());
                            let tool_info = serde_json::json!({
                                "tool": action.tool,
                                "args": action.args,
                                "status": "executing"
                            });
                            emit_message_chunk(
                                app,
                                &execution_id,  // 使用统一的 execution_id
                                &message_id,
                                self.config.conversation_id.as_deref(),
                                ChunkType::Meta,
                                &serde_json::to_string(&tool_info).unwrap_or_default(),
                                false,
                                Some("react"),
                                Some(&action.tool),
                            );
                        }
                    }

                    // 执行工具
                    let observation_result = tool_executor(action.clone()).await;

                    let action_duration = action_start
                        .elapsed()
                        .unwrap_or(Duration::from_secs(0))
                        .as_millis() as u64;

                    // === 步骤 5: Observation（工具返回） ===
                    match observation_result {
                        Ok(result) => {
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

                            // 🔧 修复：立即一次性发送完整的 Observation 结果
                            // 不要等待下一次迭代的 LLM 流式输出
                            if self.config.enable_streaming {
                                if let Some(app) = &self.config.app_handle {
                                    let trace: tokio::sync::RwLockReadGuard<'_, ReactTrace> =
                                        self.trace.read().await;
                                    let message_id = self.config.message_id.clone().unwrap_or_else(|| trace.trace_id.clone());
                                    let execution_id = self.config.execution_id.clone().unwrap_or_else(|| trace.trace_id.clone());
                                    
                                    // 一次性发送完整的工具结果，不进行流式分块
                                    let observation_content = serde_json::to_string(&result).unwrap_or_default();
                                    emit_message_chunk(
                                        app,
                                        &execution_id,  // 使用统一的 execution_id
                                        &message_id,
                                        self.config.conversation_id.as_deref(),
                                        ChunkType::ToolResult,
                                        &observation_content,
                                        false,
                                        Some("react"),
                                        Some(&action.tool),
                                    );
                                    
                                    tracing::info!(
                                        "📤 Observation sent as ToolResult chunk: tool={}, length={}",
                                        action.tool,
                                        observation_content.len()
                                    );
                                }
                            }

                            // 添加到上下文历史（但不会在 LLM 流式输出中重复显示）
                            context_history.push(format!(
                                "Thought: {}\nAction: {}\nObservation: {}",
                                llm_output,
                                serde_json::to_string(&action).unwrap_or_default(),
                                serde_json::to_string(&result).unwrap_or_default()
                            ));
                        }
                        Err(e) => {
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

                            // 🔧 修复：失败时也一次性发送完整的错误信息
                            if self.config.enable_streaming {
                                if let Some(app) = &self.config.app_handle {
                                    let trace: tokio::sync::RwLockReadGuard<'_, ReactTrace> =
                                        self.trace.read().await;
                                    let message_id = self.config.message_id.clone().unwrap_or_else(|| trace.trace_id.clone());
                                    let execution_id = self.config.execution_id.clone().unwrap_or_else(|| trace.trace_id.clone());
                                    
                                    let error_content = serde_json::json!({
                                        "error": e.to_string(),
                                        "success": false
                                    }).to_string();
                                    
                                    emit_message_chunk(
                                        app,
                                        &execution_id,
                                        &message_id,
                                        self.config.conversation_id.as_deref(),
                                        ChunkType::ToolResult,
                                        &error_content,
                                        false,
                                        Some("react"),
                                        Some(&action.tool),
                                    );
                                    
                                    tracing::warn!(
                                        "📤 Observation error sent as ToolResult chunk: tool={}, error={}",
                                        action.tool,
                                        e
                                    );
                                }
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
    ) -> (Option<String>, String) {
        let mut system_prompt = String::new();
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

                // 注入 RAG 证据到 user prompt
                if !rag_context.is_empty() {
                    user_prompt.push_str("=== Evidence from Knowledge Base ===\n");
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

        log::info!(
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
            log::info!(
                "ReAct executor: 框架适配器提供了 {} 个工具 => {:?}",
                available_tools.len(),
                available_tools
            );

            for tool_name in available_tools {
                // 兼容：某些插件工具在 ToolInfo 中可能存储去掉前缀的 id（如 "test_1"），
                // 而白名单里是 "plugin::test_1"。这里做前缀匹配补偿。
                let mut whitelist_hit = allow.contains(&tool_name);
                let plugin_prefixed_candidate = format!("plugin::{}", tool_name);
                let prefixed_whitelist_hit = allow.contains(&plugin_prefixed_candidate);
                let is_plugin = prefixed_whitelist_hit || tool_name.starts_with("plugin::");

                // 过滤白名单/黑名单（与 Plan-and-Execute 保持一致）
                // 如果有白名单且工具不在白名单中，跳过
                if !allow.is_empty() {
                    // 如果直接命中或前缀命中，则视为命中
                    whitelist_hit = whitelist_hit || prefixed_whitelist_hit;
                    if !whitelist_hit {
                        log::debug!(
                            "ReAct executor: 工具 '{}' 未命中白名单 (raw='{}', prefixed='{}'), allow={:?}",
                            tool_name,
                            allow.contains(&tool_name),
                            prefixed_whitelist_hit,
                            allow
                        );
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

            log::info!(
                "ReAct executor: 工具列入 system prompt => name='{}', signature='{}', available={}, source={:?}",
                info.name,
                signature,
                info.available,
                info.source
            );
            tool_lines.push(format!("- {}({}) - {}", info.name, signature, info.description));
        }
        tool_lines.join("\n")
    }

    /// 获取 RAG 上下文（占位符，实际应调用 RAG 服务）
    async fn fetch_rag_context(&self, _query: &str) -> Result<String> {
        // TODO: 实际调用 RAG 服务
        // 示例代码：
        // use crate::commands::rag_commands::get_global_rag_service;
        // use sentinel_rag::models::AssistantRagRequest;
        //
        // let rag_service = get_global_rag_service().await
        //     .map_err(|e| anyhow!("Failed to get RAG service: {}", e))?;
        //
        // let rag_req = AssistantRagRequest {
        //     query: query.to_string(),
        //     collection_id: None,
        //     conversation_history: None,
        //     top_k: Some(5),
        //     use_mmr: Some(true),
        //     mmr_lambda: Some(0.7),
        //     similarity_threshold: Some(0.65),
        //     reranking_enabled: Some(false),
        //     model_provider: None,
        //     model_name: None,
        //     max_tokens: None,
        //     temperature: None,
        //     system_prompt: None,
        // };
        //
        // match rag_service.query_for_assistant(&rag_req).await {
        //     Ok((context, _citations)) if !context.trim().is_empty() => Ok(context),
        //     _ => Ok(String::new()),
        // }

        // 占位符返回
        Ok(String::new())
    }

    /// 获取当前轨迹快照
    pub async fn get_trace(&self) -> ReactTrace {
        self.trace.read().await.clone()
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
        };
        let executor = ReactExecutor::new("Test task".to_string(), config);
        let trace = executor.get_trace().await;
        assert_eq!(trace.task, "Test task");
        assert_eq!(trace.status, ReactStatus::Running);
    }
}

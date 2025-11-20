//! Travel专用ReAct执行器
//!
//! 简化版ReAct执行逻辑,直接集成到Travel的Act阶段

use crate::services::ai::AiService;
use crate::services::prompt_db::PromptRepository;
use crate::tools::FrameworkToolAdapter;
use crate::utils::ordered_message::ChunkType;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sentinel_core::models::prompt::{ArchitectureType, StageType};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio_util::sync::CancellationToken;

/// ReAct步骤类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReactStepType {
    Thought,
    Action,
    Observation,
    FinalAnswer,
}

/// ReAct步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactStep {
    pub step_type: ReactStepType,
    pub content: String,
    pub tool_call: Option<ReactToolCall>,
    pub tool_result: Option<serde_json::Value>,
    pub duration_ms: u64,
}

/// ReAct工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactToolCall {
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

/// Travel专用ReAct执行器
pub struct TravelReactExecutor {
    ai_service: Arc<AiService>,
    prompt_repo: Option<Arc<PromptRepository>>,
    framework_adapter: Option<Arc<dyn FrameworkToolAdapter>>,
    max_iterations: u32,
    conversation_id: Option<String>,
    message_id: Option<String>,
    app_handle: Option<tauri::AppHandle>,
    cancellation_token: CancellationToken,
    /// 允许的工具白名单
    allowed_tools: Option<Vec<String>>,
    /// 禁止的工具黑名单
    denied_tools: Option<Vec<String>>,
}

impl TravelReactExecutor {
    /// 创建新的执行器
    pub fn new(
        ai_service: Arc<AiService>,
        prompt_repo: Option<Arc<PromptRepository>>,
        framework_adapter: Option<Arc<dyn FrameworkToolAdapter>>,
        max_iterations: u32,
        conversation_id: Option<String>,
        message_id: Option<String>,
        app_handle: Option<tauri::AppHandle>,
        cancellation_token: Option<CancellationToken>,
    ) -> Self {
        Self {
            ai_service,
            prompt_repo,
            framework_adapter,
            max_iterations,
            conversation_id,
            message_id,
            app_handle,
            cancellation_token: cancellation_token.unwrap_or_else(|| CancellationToken::new()),
            allowed_tools: None,
            denied_tools: None,
        }
    }
    
    /// 设置允许的工具白名单
    pub fn with_allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = Some(tools);
        self
    }
    
    /// 设置禁止的工具黑名单
    pub fn with_denied_tools(mut self, tools: Vec<String>) -> Self {
        self.denied_tools = Some(tools);
        self
    }

    /// 执行ReAct循环
    pub async fn execute(&self, task: &str, context: &str) -> Result<String> {
        let start_time = SystemTime::now();
        let mut iteration = 0;
        let mut context_history = Vec::new();
        let mut steps = Vec::new();

        // 添加初始上下文
        if !context.is_empty() {
            context_history.push(format!("Context: {}", context));
        }

        loop {
            iteration += 1;

            // 检查取消状态
            if self.cancellation_token.is_cancelled() {
                log::info!("Travel ReAct: Execution cancelled (iteration {})", iteration);
                return Err(anyhow!("Execution cancelled"));
            }

            // 检查最大迭代次数
            if iteration > self.max_iterations {
                log::warn!("Travel ReAct: Max iterations reached");
                return Err(anyhow!("Max iterations reached"));
            }

            // === 步骤1: Thought (思考) ===
            let thought_start = SystemTime::now();
            let (system_prompt, user_prompt) = self
                .build_thought_prompt(task, &context_history)
                .await?;

            // 调用LLM
            let llm_output = self
                .call_llm(&system_prompt, &user_prompt, iteration == 1, task)
                .await
                .context("LLM call failed during Thought phase")?;

            // 再次检查取消状态
            if self.cancellation_token.is_cancelled() {
                return Err(anyhow!("Execution cancelled after LLM call"));
            }

            let thought_duration = thought_start
                .elapsed()
                .unwrap_or(Duration::from_secs(0))
                .as_millis() as u64;

            // 解析LLM输出
            let parsed = self.parse_llm_output(&llm_output)?;

            // 记录Thought步骤
            steps.push(ReactStep {
                step_type: ReactStepType::Thought,
                content: parsed.thought.clone(),
                tool_call: None,
                tool_result: None,
                duration_ms: thought_duration,
            });

            // === 步骤2: 判断是Final Answer还是Action ===
            if let Some(final_answer) = parsed.final_answer {
                // 找到最终答案,结束循环
                steps.push(ReactStep {
                    step_type: ReactStepType::FinalAnswer,
                    content: final_answer.clone(),
                    tool_call: None,
                    tool_result: None,
                    duration_ms: 0,
                });

                log::info!(
                    "Travel ReAct: Completed in {} iterations, duration: {}ms",
                    iteration,
                    start_time.elapsed().unwrap_or(Duration::from_secs(0)).as_millis()
                );

                return Ok(final_answer);
            }

            // === 步骤3: Action (执行工具) ===
            if let Some(action) = parsed.action {
                let action_start = SystemTime::now();

                // 记录Action步骤
                steps.push(ReactStep {
                    step_type: ReactStepType::Action,
                    content: format!("{}({})", action.tool_name, action.arguments),
                    tool_call: Some(action.clone()),
                    tool_result: None,
                    duration_ms: 0,
                });

                // 执行工具
                let tool_result = self.execute_tool(&action).await?;

                let action_duration = action_start
                    .elapsed()
                    .unwrap_or(Duration::from_secs(0))
                    .as_millis() as u64;

                // 记录Observation步骤
                let observation = format!("{}", tool_result);
                steps.push(ReactStep {
                    step_type: ReactStepType::Observation,
                    content: observation.clone(),
                    tool_call: None,
                    tool_result: Some(tool_result),
                    duration_ms: action_duration,
                });

                // 添加到上下文历史
                context_history.push(format!("Thought: {}", parsed.thought));
                context_history.push(format!("Action: {}", action.tool_name));
                context_history.push(format!("Action Input: {}", action.arguments));
                context_history.push(format!("Observation: {}", observation));
            } else {
                // 没有Action也没有Final Answer,这是错误的输出格式
                return Err(anyhow!(
                    "Invalid LLM output: no Action or Final Answer found"
                ));
            }
        }
    }

    /// 构建思考提示词
    async fn build_thought_prompt(
        &self,
        task: &str,
        context_history: &[String],
    ) -> Result<(String, String)> {
        use crate::models::prompt::{ArchitectureType, StageType};
        
        // 获取系统提示词模板（优先使用 Travel 架构的 Act 阶段 prompt）
        let system_prompt = if let Some(repo) = &self.prompt_repo {
            // 尝试获取 Travel Act 阶段的 prompt（用于 ReAct 执行）
            if let Ok(Some(template)) = repo
                .get_template_by_arch_stage(ArchitectureType::Travel, StageType::Act)
                .await
            {
                log::info!("Travel ReAct: Using Travel Act prompt from database");
                template.content
            } else {
                log::warn!("Travel ReAct: Travel Act prompt not found, using default");
                self.default_system_prompt()
            }
        } else {
            log::warn!("Travel ReAct: No prompt repository available, using default");
            self.default_system_prompt()
        };

        // 获取可用工具列表（参考 ReAct 的实现）
        let tools_description = self.build_tools_information().await;

        // 替换工具占位符
        let system_prompt = system_prompt.replace("{tools}", &tools_description);

        // 构建用户提示词
        let mut user_prompt = String::new();
        user_prompt.push_str(&format!("任务: {}\n\n", task));

        if !context_history.is_empty() {
            user_prompt.push_str("历史记录:\n");
            for entry in context_history {
                user_prompt.push_str(&format!("{}\n", entry));
            }
            user_prompt.push_str("\n");
        }

        user_prompt.push_str("现在，你的下一步思考和行动是什么？");

        Ok((system_prompt, user_prompt))
    }

    /// 构建工具信息（参考 ReAct 的实现）
    async fn build_tools_information(&self) -> String {
        use crate::tools::ToolInfo;
        use std::collections::HashMap;

        let mut all_tools: Vec<ToolInfo> = Vec::new();

        // 从框架适配器获取工具
        if let Some(framework_adapter) = &self.framework_adapter {
            let available_tools = framework_adapter.list_available_tools().await;
            log::info!(
                "Travel ReAct: Framework adapter provided {} tools",
                available_tools.len()
            );

            for tool_name in available_tools {
                match framework_adapter.get_tool_info(&tool_name).await {
                    Some(tool_info) => {
                        log::debug!("Travel ReAct: Got tool info for '{}'", tool_info.name);
                        all_tools.push(tool_info);
                    }
                    None => {
                        log::warn!(
                            "Travel ReAct: list_available_tools() contains '{}' but get_tool_info returned None",
                            tool_name
                        );
                    }
                }
            }
        } else {
            // 降级：尝试使用全局 engine adapter
            log::info!("Travel ReAct: No framework adapter, trying global engine adapter");
            match crate::tools::get_global_engine_adapter() {
                Ok(engine_adapter) => {
                    let available_tools = engine_adapter.list_available_tools().await;
                    log::info!(
                        "Travel ReAct: Global engine adapter provided {} tools",
                        available_tools.len()
                    );

                    for tool_name in available_tools {
                        match engine_adapter.get_tool_info(&tool_name).await {
                            Some(tool_info) => {
                                all_tools.push(tool_info);
                            }
                            None => {
                                log::warn!(
                                    "Travel ReAct: Global adapter list contains '{}' but get_tool_info returned None",
                                    tool_name
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Travel ReAct: Failed to get global engine adapter: {}", e);
                    return "No tools available".to_string();
                }
            }
        }

        // 去重工具（按名称）
        let mut unique_tools: HashMap<String, ToolInfo> = HashMap::new();
        for tool in all_tools {
            unique_tools.entry(tool.name.clone()).or_insert(tool);
        }

        // 应用工具白名单/黑名单过滤
        let mut filtered_tools: Vec<ToolInfo> = unique_tools.into_values().collect();
        
        // 如果有白名单，只保留白名单中的工具
        if let Some(allowed) = &self.allowed_tools {
            if !allowed.is_empty() {
                filtered_tools.retain(|tool| allowed.contains(&tool.name));
                log::info!(
                    "Travel ReAct: Applied allow list, {} tools remain",
                    filtered_tools.len()
                );
            }
        }
        
        // 如果有黑名单，移除黑名单中的工具
        if let Some(denied) = &self.denied_tools {
            if !denied.is_empty() {
                filtered_tools.retain(|tool| !denied.contains(&tool.name));
                log::info!(
                    "Travel ReAct: Applied deny list, {} tools remain",
                    filtered_tools.len()
                );
            }
        }

        let tool_infos: Vec<&ToolInfo> = filtered_tools.iter().collect();

        if tool_infos.is_empty() {
            log::warn!("Travel ReAct: No tools available after filtering");
            return "No tools available".to_string();
        }

        log::info!(
            "Travel ReAct: Building tool information for {} tools (after filtering)",
            tool_infos.len()
        );

        // 构建工具描述（包含参数签名）
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

    /// 默认系统提示词（中文版 ReAct）
    fn default_system_prompt(&self) -> String {
        r#"你是 Travel 安全测试智能体的执行者，使用 ReAct（推理 + 行动）框架进行安全测试。

## 可用工具

{tools}

## 执行格式

### 需要使用工具时：

```
Thought: [你对下一步的推理和分析]

Action: [工具名称]

Action Input: {"参数名": "参数值"}
```

### 有足够信息回答时：

```
Thought: [你的最终推理]

Final Answer: [你对任务的完整答案]
```

## 关键规则

1. **单步执行**: 一次只执行一个 Action
2. **等待观察**: 执行 Action 后等待 Observation，不要自己输出 "Observation:"
3. **不要提前规划**: 不要一次性输出多个步骤
4. **基于实际结果**: 下一步行动必须基于真实的 Observation

## 安全测试最佳实践

1. **系统化侦察**: 先收集信息，再进行测试
2. **渐进式测试**: 从被动扫描到主动测试
3. **记录发现**: 详细记录所有发现的漏洞和安全问题
4. **合法性**: 确保在授权范围内进行测试

现在开始执行任务！
"#
        .to_string()
    }

    /// 调用LLM
    async fn call_llm(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        is_first_iteration: bool,
        original_task: &str,
    ) -> Result<String> {
        // 决定是否保存用户消息
        let user_to_save = if is_first_iteration {
            Some(original_task)
        } else {
            None
        };

        self.ai_service
            .send_message_stream_with_save_control(
                Some(user_prompt),
                user_to_save,
                Some(system_prompt),
                self.conversation_id.clone(),
                self.message_id.clone(),
                true, // stream
                false,
                Some(ChunkType::Content),
            )
            .await
            .map_err(|e| anyhow!("LLM call failed: {}", e))
    }

    /// 解析LLM输出
    fn parse_llm_output(&self, output: &str) -> Result<ParsedOutput> {
        let mut thought = String::new();
        let mut action_name = None;
        let mut action_input = None;
        let mut final_answer = None;

        let lines: Vec<&str> = output.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            if line.starts_with("Thought:") {
                thought = line.strip_prefix("Thought:").unwrap_or("").trim().to_string();
                // 继续读取多行thought
                i += 1;
                while i < lines.len()
                    && !lines[i].trim().starts_with("Action:")
                    && !lines[i].trim().starts_with("Final Answer:")
                {
                    if !lines[i].trim().is_empty() {
                        thought.push(' ');
                        thought.push_str(lines[i].trim());
                    }
                    i += 1;
                }
                continue;
            } else if line.starts_with("Action:") {
                action_name = Some(
                    line.strip_prefix("Action:")
                        .unwrap_or("")
                        .trim()
                        .to_string(),
                );
            } else if line.starts_with("Action Input:") {
                let input_str = line.strip_prefix("Action Input:").unwrap_or("").trim();
                // 尝试解析JSON
                action_input = serde_json::from_str(input_str).ok();
            } else if line.starts_with("Final Answer:") {
                final_answer = Some(
                    line.strip_prefix("Final Answer:")
                        .unwrap_or("")
                        .trim()
                        .to_string(),
                );
                // 继续读取多行final answer
                i += 1;
                while i < lines.len() {
                    if !lines[i].trim().is_empty() {
                        if let Some(ref mut answer) = final_answer {
                            answer.push(' ');
                            answer.push_str(lines[i].trim());
                        }
                    }
                    i += 1;
                }
                break;
            }

            i += 1;
        }

        // 构建action
        let action = if let (Some(name), Some(input)) = (action_name, action_input) {
            Some(ReactToolCall {
                tool_name: name,
                arguments: input,
            })
        } else {
            None
        };

        Ok(ParsedOutput {
            thought,
            action,
            final_answer,
        })
    }

    /// 执行工具
    async fn execute_tool(&self, tool_call: &ReactToolCall) -> Result<serde_json::Value> {
        // 构造统一工具调用
        let parameters = if let serde_json::Value::Object(map) = &tool_call.arguments {
            map.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            std::collections::HashMap::new()
        };

        let unified_call = crate::tools::UnifiedToolCall {
            id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool_call.tool_name.clone(),
            parameters,
            timeout: Some(std::time::Duration::from_secs(30)),
            context: std::collections::HashMap::new(),
            retry_count: 0,
        };

        // 优先使用设置的 framework_adapter
        if let Some(adapter) = &self.framework_adapter {
            let result = adapter.execute_tool(unified_call).await?;
            return Ok(result.output);
        }
        
        // 降级：使用全局 engine adapter
        log::info!("No framework adapter, using global engine adapter for tool: {}", tool_call.tool_name);
        match crate::tools::get_global_engine_adapter() {
            Ok(engine_adapter) => {
                let result = engine_adapter.execute_tool(unified_call).await?;
                Ok(result.output)
            }
            Err(e) => {
                Err(anyhow!("No framework adapter available and failed to get global adapter: {}", e))
            }
        }
    }
}

/// 解析后的LLM输出
struct ParsedOutput {
    thought: String,
    action: Option<ReactToolCall>,
    final_answer: Option<String>,
}


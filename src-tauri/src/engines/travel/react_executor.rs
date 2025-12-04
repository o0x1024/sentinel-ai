//! Travel专用ReAct执行器
//!
//! 简化版ReAct执行逻辑,直接集成到Travel的Act阶段
//! 使用自有 LLM 客户端实现流式消息展示
//! 
//! ## Token优化
//! - 使用 ContextManager 压缩观察结果
//! - 滑动窗口历史管理
//! - HTML/JSON 智能裁剪
//!
//! ## 循环检测
//! - 跟踪已访问URL和已测试参数
//! - 检测重复工具调用
//! - 强制推进测试进度

use crate::services::ai::AiService;
use crate::services::prompt_db::PromptRepository;
use crate::tools::FrameworkToolAdapter;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio_util::sync::CancellationToken;

use super::message_emitter::{TravelMessageEmitter, TravelExecutionStats};
use super::message_emitter::TravelLlmClient;
use super::context_manager::ContextManager;
use super::types::ContextManagerConfig;

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

/// 测试进度跟踪 - 防止循环
#[derive(Debug, Clone, Default)]
pub struct TestProgress {
    /// 已访问的 URL
    pub visited_urls: HashSet<String>,
    /// 已测试的参数 (URL + param_name)
    pub tested_params: HashSet<String>,
    /// 工具调用历史 (tool_name + args_hash)
    pub tool_call_history: Vec<String>,
    /// 连续重复调用计数
    pub repeat_count: u32,
    /// 最后的工具调用签名
    pub last_tool_signature: Option<String>,
    /// 已完成的测试阶段
    pub completed_phases: Vec<String>,
}

impl TestProgress {
    /// 记录 URL 访问
    pub fn record_url_visit(&mut self, url: &str) {
        // 标准化 URL (移除参数值)
        let normalized = Self::normalize_url(url);
        self.visited_urls.insert(normalized);
    }
    
    /// 记录参数测试
    pub fn record_param_test(&mut self, url: &str, param: &str) {
        let key = format!("{}:{}", Self::normalize_url(url), param);
        self.tested_params.insert(key);
    }
    
    /// 检查工具调用是否重复
    pub fn check_and_record_tool_call(&mut self, tool_name: &str, args: &serde_json::Value) -> bool {
        let signature = format!("{}:{}", tool_name, Self::hash_args(args));
        
        // 检查是否与上次相同
        let is_repeat = self.last_tool_signature.as_ref() == Some(&signature);
        
        if is_repeat {
            self.repeat_count += 1;
        } else {
            self.repeat_count = 0;
        }
        
        self.tool_call_history.push(signature.clone());
        self.last_tool_signature = Some(signature);
        
        is_repeat
    }
    
    /// 是否需要强制推进 (连续重复 >= 2 次)
    pub fn should_force_progress(&self) -> bool {
        self.repeat_count >= 2
    }
    
    /// 生成进度摘要
    pub fn generate_summary(&self) -> String {
        let mut summary = String::new();
        
        if !self.visited_urls.is_empty() {
            summary.push_str(&format!("[已访问URL] {} 个\n", self.visited_urls.len()));
        }
        
        if !self.tested_params.is_empty() {
            summary.push_str(&format!("[已测试参数] {} 个\n", self.tested_params.len()));
        }
        
        if !self.completed_phases.is_empty() {
            summary.push_str(&format!("[已完成阶段] {}\n", self.completed_phases.join(", ")));
        }
        
        summary.push_str(&format!("[总工具调用] {} 次\n", self.tool_call_history.len()));
        
        if self.repeat_count > 0 {
            summary.push_str(&format!("[警告] 检测到连续 {} 次重复操作\n", self.repeat_count));
        }
        
        summary
    }
    
    /// 标准化 URL (只保留路径，移除参数值)
    fn normalize_url(url: &str) -> String {
        if let Ok(parsed) = url::Url::parse(url) {
            format!("{}://{}{}", parsed.scheme(), parsed.host_str().unwrap_or(""), parsed.path())
        } else {
            url.to_string()
        }
    }
    
    /// 计算参数哈希
    fn hash_args(args: &serde_json::Value) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        args.to_string().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
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
    /// 执行ID
    execution_id: Option<String>,
    /// Travel 专用消息发送器
    travel_emitter: Option<Arc<TravelMessageEmitter>>,
    /// Travel 专用 LLM 客户端
    llm_client: Option<Arc<TravelLlmClient>>,
    /// 上下文管理器 - Token优化
    context_manager: ContextManager,
    /// 历史窗口大小 (保留最近N轮)
    history_window_size: usize,
    /// 测试进度跟踪 - 防止循环
    test_progress: std::sync::Mutex<TestProgress>,
    /// OODA 循环号（在整个 ReAct 执行期间保持不变）
    ooda_cycle: u32,
}

/// 安全截断 UTF-8 字符串，确保不会在字符中间切断
fn safe_truncate(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }
    // 找到最接近 max_len 的字符边界
    let mut end = max_len;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
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
        // 创建 Travel 专用消息发送器和 LLM 客户端
        let (travel_emitter, llm_client) = if let Some(app) = &app_handle {
            let exec_id = message_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let msg_id = message_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            
            let emitter = Arc::new(TravelMessageEmitter::new(
                Arc::new(app.clone()),
                exec_id,
                msg_id,
                conversation_id.clone(),
            ));
            
            let llm_config = crate::engines::create_llm_config(&ai_service);
            let client = Arc::new(TravelLlmClient::new(llm_config, emitter.clone()));
            
            (Some(emitter), Some(client))
        } else {
            (None, None)
        };
        
        // 创建上下文管理器 - Token优化配置
        let context_config = ContextManagerConfig {
            enable_compression: true,
            max_context_tokens: 4000,
            max_history_entries: 10,
            max_tool_result_length: 1000,  // 工具结果最大1000字符
            preserve_fields: vec![
                "success".to_string(),
                "error".to_string(),
                "status".to_string(),
                "url".to_string(),
                "target".to_string(),
            ],
        };

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
            execution_id: None,
            travel_emitter,
            llm_client,
            context_manager: ContextManager::new(context_config),
            history_window_size: 5,  // 保留最近5轮历史
            test_progress: std::sync::Mutex::new(TestProgress::default()),
            ooda_cycle: 1,  // 默认 OODA 循环号
        }
    }

    /// 设置执行ID
    pub fn with_execution_id(mut self, execution_id: String) -> Self {
        self.execution_id = Some(execution_id);
        self
    }

    /// 设置 OODA 循环号（在整个 ReAct 执行期间保持不变）
    pub fn with_ooda_cycle(mut self, cycle: u32) -> Self {
        self.ooda_cycle = cycle;
        self
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
        let mut tool_calls_count = 0u32;
        let mut successful_tool_calls = 0u32;
        let mut failed_tool_calls = 0u32;

        // 发送开始消息，使用固定的 OODA 循环号
        if let Some(te) = &self.travel_emitter {
            te.emit_start(self.ooda_cycle);
        }

        // 添加初始上下文
        if !context.is_empty() {
            context_history.push(format!("Context: {}", context));
        }

        loop {
            iteration += 1;

            // 检查取消状态
            if self.cancellation_token.is_cancelled() {
                log::info!("Travel ReAct: Execution cancelled (iteration {})", iteration);
                if let Some(te) = &self.travel_emitter {
                    te.emit_error("Execution cancelled");
                }
                return Err(anyhow!("Execution cancelled"));
            }

            // 检查最大迭代次数
            if iteration > self.max_iterations {
                log::warn!("Travel ReAct: Max iterations reached");
                if let Some(te) = &self.travel_emitter {
                    te.emit_error(&format!("Max iterations ({}) reached", self.max_iterations));
                }
                return Err(anyhow!("Max iterations reached"));
            }

            let thought_start = SystemTime::now();
            let (system_prompt, user_prompt) =
                self.build_thought_prompt(task, &context_history).await?;

            // 调用LLM - 使用 Travel 专用 LLM 客户端，带重试机制
            let max_retries = 3;
            let mut retry_count = 0;
            let mut llm_output;
            let mut parsed;
            
            loop {
                llm_output = self
                    .call_llm(&system_prompt, &user_prompt, iteration == 1, task, iteration)
                    .await
                    .context("LLM call failed during Thought phase")?;

                // 再次检查取消状态
                if self.cancellation_token.is_cancelled() {
                    if let Some(te) = &self.travel_emitter {
                        te.emit_error("Execution cancelled after LLM call");
                    }
                    return Err(anyhow!("Execution cancelled after LLM call"));
                }

                // 检查输出是否完整
                let (is_complete, issue) = self.check_output_completeness(&llm_output);
                
                if !is_complete && retry_count < max_retries {
                    retry_count += 1;
                    log::warn!(
                        "Travel ReAct: Incomplete LLM output detected ({}), retrying ({}/{})",
                        issue, retry_count, max_retries
                    );
                    if let Some(te) = &self.travel_emitter {
                        te.emit_error(&format!(
                            "Incomplete response detected: {}, retrying ({}/{})",
                            issue, retry_count, max_retries
                        ));
                    }
                    // 短暂延迟后重试
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }
                
                // 尝试解析输出
                match self.parse_llm_output(&llm_output) {
                    Ok(p) => {
                        // 检查解析结果是否有效（有action或final_answer）
                        if p.action.is_none() && p.final_answer.is_none() && retry_count < max_retries {
                            retry_count += 1;
                            log::warn!(
                                "Travel ReAct: Invalid output (no action/final_answer), retrying ({}/{})",
                                retry_count, max_retries
                            );
                            if let Some(te) = &self.travel_emitter {
                                te.emit_error(&format!(
                                    "Invalid response (no action/final_answer), retrying ({}/{})",
                                    retry_count, max_retries
                                ));
                            }
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            continue;
                        }
                        parsed = p;
                        break;
                    }
                    Err(e) if retry_count < max_retries => {
                        retry_count += 1;
                        log::warn!(
                            "Travel ReAct: Parse error ({}), retrying ({}/{})",
                            e, retry_count, max_retries
                        );
                        if let Some(te) = &self.travel_emitter {
                            te.emit_error(&format!(
                                "Parse error: {}, retrying ({}/{})",
                                e, retry_count, max_retries
                            ));
                        }
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                    Err(e) => {
                        return Err(e.context("Failed to parse LLM output after retries"));
                    }
                }
            }

            let thought_duration = thought_start
                .elapsed()
                .unwrap_or(Duration::from_secs(0))
                .as_millis() as u64;

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

                let total_duration = start_time
                    .elapsed()
                    .unwrap_or(Duration::from_secs(0))
                    .as_millis() as u64;

                log::info!(
                    "Travel ReAct: Completed in {} iterations, duration: {}ms",
                    iteration, total_duration
                );

                // 发送完成统计
                if let Some(te) = &self.travel_emitter {
                    te.emit_complete(TravelExecutionStats {
                        total_iterations: iteration,
                        tool_calls_count,
                        successful_tool_calls,
                        failed_tool_calls,
                        total_duration_ms: total_duration,
                        status: "completed".to_string(),
                    });
                }

                return Ok(final_answer);
            }

            // === 步骤3: Action (执行工具) ===
            if let Some(action) = parsed.action {
                let action_start = SystemTime::now();
                
                // === 循环检测 ===
                let (is_repeat, should_force) = {
                    let mut progress = self.test_progress.lock().unwrap();
                    let is_repeat = progress.check_and_record_tool_call(&action.tool_name, &action.arguments);
                    let should_force = progress.should_force_progress();
                    
                    // 跟踪 URL 访问
                    if action.tool_name.contains("navigate") || action.tool_name == "http_request" {
                        if let Some(url) = action.arguments.get("url").and_then(|v| v.as_str()) {
                            progress.record_url_visit(url);
                        }
                    }
                    
                    (is_repeat, should_force)
                };
                
                // 如果检测到过多重复，发出警告并强制推进
                if should_force {
                    log::warn!("Travel ReAct: Detected repetitive loop (>2 times), forcing progress");
                    if let Some(te) = &self.travel_emitter {
                        te.emit_error("Detected repetitive operations, forcing progress to next phase");
                    }
                    
                    // 添加强制推进提示到上下文
                    context_history.push(format!(
                        "[系统警告] 检测到重复操作循环！请跳过当前操作，进入下一测试阶段。已完成 {} 次工具调用。",
                        tool_calls_count
                    ));
                }
                
                if is_repeat {
                    log::info!("Travel ReAct: Repeated tool call detected: {}", action.tool_name);
                }

                // 使用 travel_emitter 发送工具调用信息（如果没有通过 LLM 客户端发送）
                if self.llm_client.is_none() {
                    if let Some(te) = &self.travel_emitter {
                        te.emit_tool_call(iteration, &action.tool_name, &action.arguments);
                    }
                }

                // 记录Action步骤
                steps.push(ReactStep {
                    step_type: ReactStepType::Action,
                    content: format!("{}({})", action.tool_name, action.arguments),
                    tool_call: Some(action.clone()),
                    tool_result: None,
                    duration_ms: 0,
                });

                tool_calls_count += 1;
                log::info!("Travel ReAct: Starting tool execution #{}: {}", tool_calls_count, action.tool_name);

                // 执行工具
                let tool_result = match self.execute_tool(&action).await {
                    Ok(result) => {
                        successful_tool_calls += 1;
                        log::info!("Travel ReAct: Tool {} executed successfully (#{} total)", action.tool_name, tool_calls_count);
                        result
                    }
                    Err(e) => {
                        failed_tool_calls += 1;
                        log::error!("Travel ReAct: Tool {} execution failed: {}", action.tool_name, e);
                        if let Some(te) = &self.travel_emitter {
                            te.emit_error(&format!("Tool {} failed: {}", action.tool_name, e));
                        }
                        serde_json::json!({"error": e.to_string()})
                    }
                };

                let action_duration = action_start
                    .elapsed()
                    .unwrap_or(Duration::from_secs(0))
                    .as_millis() as u64;

                // 使用 travel_emitter 发送工具结果
                // 判断成功：优先使用 success 字段，其次检查 error 是否为非空值
                let is_success = tool_result.get("success")
                    .and_then(|v| v.as_bool())
                    .unwrap_or_else(|| {
                        // 没有 success 字段时，检查 error 是否为空/null
                        match tool_result.get("error") {
                            None => true,
                            Some(v) => v.is_null() || (v.is_string() && v.as_str().unwrap_or("").is_empty()),
                        }
                    });
                
                if let Some(te) = &self.travel_emitter {
                    te.emit_tool_result(
                        iteration,
                        &action.tool_name,
                        &tool_result,
                        is_success,
                        action_duration,
                    );
                }

                // 压缩工具结果 - Token优化
                let compressed_result = self.context_manager.compress_tool_result(&tool_result);
                let observation = self.compress_observation(&action.tool_name, &compressed_result);

                // 记录Observation步骤
                steps.push(ReactStep {
                    step_type: ReactStepType::Observation,
                    content: observation.clone(),
                    tool_call: None,
                    tool_result: Some(compressed_result),
                    duration_ms: action_duration,
                });

                // 添加到上下文历史 (JSON 格式)
                let history_entry = serde_json::json!({
                    "thought": self.truncate_thought(&parsed.thought),
                    "action": {
                        "name": action.tool_name,
                        "input": self.compress_action_input_json(&action.arguments)
                    },
                    "observation": observation
                });
                context_history.push(history_entry.to_string());
                
                // 滑动窗口 - 保留最近N轮历史
                self.apply_history_window(&mut context_history);
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
        
        // === 添加测试进度摘要 ===
        let progress_summary = {
            let progress = self.test_progress.lock().unwrap();
            progress.generate_summary()
        };
        
        if !progress_summary.is_empty() {
            user_prompt.push_str("## 测试进度\n");
            user_prompt.push_str(&progress_summary);
            user_prompt.push_str("\n");
        }

        if !context_history.is_empty() {
            user_prompt.push_str("## 历史记录\n");
            for entry in context_history {
                user_prompt.push_str(&format!("{}\n", entry));
            }
            user_prompt.push_str("\n");
        }

        user_prompt.push_str("现在，你的下一步思考和行动是什么？（注意避免重复已完成的操作）");

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

            tool_lines.push(format!(
                "- {}({}) - {}",
                info.name, signature, info.description
            ));
        }

        tool_lines.join("\n")
    }

    /// 默认系统提示词（JSON 格式 ReAct）
    fn default_system_prompt(&self) -> String {
        r#"你是 Travel 安全测试智能体，使用 ReAct（推理 + 行动）框架执行任务。

## 可用工具

{tools}

## 输出格式 - 必须严格遵守！

你的每次回复必须是一个**有效的 JSON 对象**，格式如下：

### 需要调用工具时：
```json
{
  "thought": "你的推理分析过程",
  "action": {
    "name": "工具名称",
    "input": {"参数名": "参数值"}
  }
}
```

### 任务完成时：
```json
{
  "thought": "你的最终推理",
  "final_answer": "对任务的完整回答和总结"
}
```

## 关键规则

1. **必须输出有效 JSON**: 每次回复只能是一个完整的 JSON 对象
2. **单步执行**: 每次只能有一个 action 或 final_answer
3. **等待观察**: 系统会返回 Observation，基于它决定下一步
4. **不要自造结果**: 不要假设工具执行结果

## 示例

### 调用工具：
```json
{
  "thought": "需要导航到目标网站查看页面结构",
  "action": {
    "name": "playwright_navigate",
    "input": {"url": "http://example.com"}
  }
}
```

### 完成任务：
```json
{
  "thought": "已完成所有测试，发现2个SQL注入漏洞",
  "final_answer": "安全测试完成。发现以下漏洞：\n1. /login.php 存在SQL注入\n2. /search.php 存在XSS漏洞"
}
```

现在开始执行任务，记住：只输出 JSON！
"#
        .to_string()
    }

    /// 调用LLM - 使用 Travel 专用 LLM 客户端实现流式输出
    async fn call_llm(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        _is_first_iteration: bool,
        _original_task: &str,
        iteration: u32,
    ) -> Result<String> {
        // 使用 Travel 专用 LLM 客户端（不降级）
        match &self.llm_client {
            Some(llm_client) => {
                log::info!("Travel ReAct: Using TravelLlmClient for iteration {}", iteration);
                llm_client
                    .stream_completion(Some(system_prompt), user_prompt, iteration)
                    .await
            }
            None => {
                log::error!("Travel ReAct: TravelLlmClient not available, cannot proceed");
                Err(anyhow!("Travel LLM client not initialized. Check AI service configuration."))
            }
        }
    }

    /// 检查LLM输出是否完整
    /// 返回 (is_complete, issue_description)
    fn check_output_completeness(&self, output: &str) -> (bool, String) {
        let trimmed = output.trim();
        
        // 空输出
        if trimmed.is_empty() {
            return (false, "empty response".to_string());
        }
        
        // 检查是否是被截断的JSON
        let json_start = trimmed.find('{');
        let json_end = trimmed.rfind('}');
        
        if let Some(start) = json_start {
            // 有JSON开始标记
            match json_end {
                None => {
                    // 没有闭合的}，明显被截断
                    return (false, "JSON not closed (missing '}')".to_string());
                }
                Some(end) if end < start => {
                    // }在{之前，不合法
                    return (false, "malformed JSON structure".to_string());
                }
                Some(end) => {
                    // 检查括号是否匹配
                    let json_str = &trimmed[start..=end];
                    let open_braces = json_str.matches('{').count();
                    let close_braces = json_str.matches('}').count();
                    if open_braces != close_braces {
                        return (false, format!(
                            "unbalanced braces (open: {}, close: {})",
                            open_braces, close_braces
                        ));
                    }
                    
                    // 检查方括号是否匹配
                    let open_brackets = json_str.matches('[').count();
                    let close_brackets = json_str.matches(']').count();
                    if open_brackets != close_brackets {
                        return (false, format!(
                            "unbalanced brackets (open: {}, close: {})",
                            open_brackets, close_brackets
                        ));
                    }
                    
                    // 检查引号是否匹配（简单检查，不处理转义）
                    let quotes = json_str.matches('"').count();
                    if quotes % 2 != 0 {
                        return (false, "unclosed string (odd number of quotes)".to_string());
                    }
                }
            }
        }
        
        // 检查输出长度是否过短（可能被截断）
        if trimmed.len() < 20 {
            // 非常短的输出可能有问题，除非是简单的final_answer
            if !trimmed.contains("final_answer") && !trimmed.contains("Final Answer") {
                return (false, "response too short".to_string());
            }
        }
        
        // 检查是否以不完整的JSON结尾（常见截断模式）
        let incomplete_endings = [
            "\"thought\":",
            "\"action\":",
            "\"name\":",
            "\"input\":",
            "\"final_answer\":",
            ": \"",
            ",",
        ];
        
        for ending in incomplete_endings {
            if trimmed.ends_with(ending) {
                return (false, format!("truncated at '{}'", ending));
            }
        }
        
        (true, String::new())
    }

    /// 解析LLM输出 - JSON 格式优先，兼容旧文本格式
    fn parse_llm_output(&self, output: &str) -> Result<ParsedOutput> {
        // 清理输出：移除 markdown 代码块标记
        let cleaned = Self::extract_json_from_output(output);
        
        // 尝试 JSON 解析
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&cleaned) {
            return self.parse_json_output(&json);
        }
        
        // JSON 解析失败，尝试兼容旧的文本格式
        log::warn!("JSON parsing failed, falling back to text format");
        self.parse_text_output(output)
    }
    
    /// 从输出中提取 JSON（移除 markdown 代码块等）
    fn extract_json_from_output(output: &str) -> String {
        let trimmed = output.trim();
        
        // 移除 ```json ... ``` 包裹
        let mut s = trimmed.to_string();
        if s.starts_with("```json") {
            s = s.trim_start_matches("```json").to_string();
        } else if s.starts_with("```") {
            s = s.trim_start_matches("```").to_string();
        }
        if s.ends_with("```") {
            s = s.trim_end_matches("```").to_string();
        }
        
        // 尝试找到 JSON 对象边界
        let s = s.trim();
        if let Some(start) = s.find('{') {
            if let Some(end) = s.rfind('}') {
                if end >= start {
                    return s[start..=end].to_string();
                }
            }
        }
        
        s.to_string()
    }
    
    /// 解析 JSON 格式输出
    fn parse_json_output(&self, json: &serde_json::Value) -> Result<ParsedOutput> {
        let thought = json.get("thought")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        // 检查是否有 final_answer
        if let Some(answer) = json.get("final_answer").and_then(|v| v.as_str()) {
            return Ok(ParsedOutput {
                thought,
                action: None,
                final_answer: Some(answer.to_string()),
            });
        }
        
        // 检查是否有 action
        if let Some(action_obj) = json.get("action") {
            let tool_name = action_obj.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let arguments = action_obj.get("input")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            
            if !tool_name.is_empty() {
                return Ok(ParsedOutput {
                    thought,
                    action: Some(ReactToolCall { tool_name, arguments }),
                    final_answer: None,
                });
            }
        }
        
        // 没有有效的 action 或 final_answer
        if !thought.is_empty() {
            log::warn!("JSON output has thought but no action or final_answer");
        }
        
        Ok(ParsedOutput {
            thought,
            action: None,
            final_answer: None,
        })
    }
    
    /// 解析旧的文本格式输出（兼容）
    fn parse_text_output(&self, output: &str) -> Result<ParsedOutput> {
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
                action_name = Some(line.strip_prefix("Action:").unwrap_or("").trim().to_string());
            } else if line.starts_with("Action Input:") {
                let first_part = line.strip_prefix("Action Input:").unwrap_or("").trim();
                let sanitize = |s: &str| {
                    let mut t = s.trim().to_string();
                    if t.starts_with("```json") { t = t.trim_start_matches("```json").trim().to_string(); }
                    if t.starts_with("```") { t = t.trim_start_matches("```").trim().to_string(); }
                    if t.ends_with("```") { t = t.trim_end_matches("```").trim().to_string(); }
                    t
                };

                if !first_part.is_empty() {
                    let candidate = sanitize(first_part);
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&candidate) {
                        action_input = Some(v);
                        i += 1;
                        continue;
                    }
                }

                i += 1;
                let mut json_buf = String::new();
                while i < lines.len() {
                    let current_line = lines[i].trim();
                    if current_line.starts_with("Thought:")
                        || current_line.starts_with("Action:")
                        || current_line.starts_with("Final Answer:")
                        || current_line.starts_with("Observation:") {
                        break;
                    }
                    if !current_line.is_empty() && !current_line.starts_with("```") {
                        json_buf.push_str(current_line);
                    }
                    i += 1;
                }

                let candidate = sanitize(&json_buf);
                if !candidate.is_empty() {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&candidate) {
                        action_input = Some(v);
                    }
                } else {
                    action_input = Some(serde_json::json!({}));
                }
                continue;
            } else if line.starts_with("Final Answer:") {
                final_answer = Some(line.strip_prefix("Final Answer:").unwrap_or("").trim().to_string());
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

        let action = if let (Some(name), Some(input)) = (action_name, action_input) {
            Some(ReactToolCall { tool_name: name, arguments: input })
        } else {
            None
        };

        Ok(ParsedOutput { thought, action, final_answer })
    }

    /// 执行工具
    async fn execute_tool(&self, tool_call: &ReactToolCall) -> Result<serde_json::Value> {
        log::info!("Travel ReAct: Executing tool: {}", tool_call.tool_name);
        
        // 构造统一工具调用
        let parameters = if let serde_json::Value::Object(map) = &tool_call.arguments {
            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else {
            std::collections::HashMap::new()
        };

        let unified_call = crate::tools::UnifiedToolCall {
            id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool_call.tool_name.clone(),
            parameters,
            timeout: Some(std::time::Duration::from_secs(60)),  // 增加到60秒
            context: std::collections::HashMap::new(),
            retry_count: 0,
        };

        // 优先使用设置的 framework_adapter
        if let Some(adapter) = &self.framework_adapter {
            log::info!("Travel ReAct: Using framework adapter for tool: {}", tool_call.tool_name);
            let result = adapter.execute_tool(unified_call).await?;
            log::info!("Travel ReAct: Tool {} completed via framework adapter", tool_call.tool_name);
            return Ok(result.output);
        }

        // 降级：使用全局 engine adapter
        log::info!(
            "Travel ReAct: No framework adapter, using global engine adapter for tool: {}",
            tool_call.tool_name
        );
        match crate::tools::get_global_engine_adapter() {
            Ok(engine_adapter) => {
                log::info!("Travel ReAct: Got global engine adapter, executing tool: {}", tool_call.tool_name);
                let result = engine_adapter.execute_tool(unified_call).await?;
                log::info!("Travel ReAct: Tool {} completed successfully via global adapter", tool_call.tool_name);
                Ok(result.output)
            }
            Err(e) => {
                log::error!("Travel ReAct: Failed to get global adapter for tool {}: {}", tool_call.tool_name, e);
                Err(anyhow!(
                    "No framework adapter available and failed to get global adapter: {}",
                    e
                ))
            }
        }
    }
    
    // ========== Token 优化辅助方法 ==========
    
    /// 压缩观察结果 - 根据工具类型智能裁剪
    fn compress_observation(&self, tool_name: &str, result: &serde_json::Value) -> String {
        let max_len = 800; // 单个观察结果最大长度
        
        // 特殊处理 HTML 相关工具
        if tool_name.contains("html") || tool_name.contains("visible") {
            return self.compress_html_result(result, max_len);
        }
        
        // 特殊处理网站分析结果
        if tool_name.contains("analyze") {
            return self.compress_analysis_result(result, max_len);
        }
        
        // 通用压缩
        let result_str = result.to_string();
        if result_str.len() > max_len {
            format!("{}...(truncated {} chars)", safe_truncate(&result_str, max_len), result_str.len() - max_len)
        } else {
            result_str
        }
    }
    
    /// 压缩 HTML 相关结果
    fn compress_html_result(&self, result: &serde_json::Value, max_len: usize) -> String {
        if let Some(obj) = result.as_object() {
            let mut summary = String::new();
            
            // 保留 success 状态
            if let Some(success) = obj.get("success") {
                summary.push_str(&format!("success:{}", success));
            }
            
            // 提取 HTML 内容并压缩
            if let Some(output) = obj.get("output").and_then(|v| v.as_str()) {
                // 提取关键元素: 表单、输入框、链接
                let forms = self.extract_html_elements(output, "form");
                let inputs = self.extract_html_elements(output, "input");
                let links_count = output.matches("<a ").count();
                
                summary.push_str(&format!(", forms:{}, inputs:{}, links:{}", 
                    forms.len(), inputs.len(), links_count));
                
                // 如果有表单/输入框，保留关键信息
                if !inputs.is_empty() {
                    let input_names: Vec<&str> = inputs.iter()
                        .filter_map(|s| self.extract_attr(s, "name"))
                        .take(5)
                        .collect();
                    if !input_names.is_empty() {
                        summary.push_str(&format!(", input_names:[{}]", input_names.join(",")));
                    }
                }
            }
            
            return format!("{{{}}}", summary);
        }
        
        // 降级：直接截断
        let s = result.to_string();
        if s.len() > max_len {
            format!("{}...", safe_truncate(&s, max_len))
        } else {
            s
        }
    }
    
    /// 压缩网站分析结果
    fn compress_analysis_result(&self, result: &serde_json::Value, max_len: usize) -> String {
        if let Some(obj) = result.as_object() {
            let mut summary = serde_json::Map::new();
            
            // 保留关键字段
            for key in &["domain", "total_requests", "api_endpoints_count", "tech_stack"] {
                if let Some(v) = obj.get(*key) {
                    summary.insert((*key).to_string(), v.clone());
                }
            }
            
            // 压缩 endpoints 列表
            if let Some(endpoints) = obj.get("endpoints").and_then(|v| v.as_array()) {
                let endpoint_summaries: Vec<serde_json::Value> = endpoints.iter()
                    .take(5)  // 只保留前5个端点
                    .filter_map(|e| {
                        e.as_object().map(|ep| {
                            serde_json::json!({
                                "path": ep.get("path"),
                                "method": ep.get("method"),
                                "params": ep.get("query_params").and_then(|p| p.as_array()).map(|a| a.len())
                            })
                        })
                    })
                    .collect();
                summary.insert("endpoints".to_string(), serde_json::Value::Array(endpoint_summaries));
                
                if endpoints.len() > 5 {
                    summary.insert("more_endpoints".to_string(), 
                        serde_json::json!(endpoints.len() - 5));
                }
            }
            
            return serde_json::Value::Object(summary).to_string();
        }
        
        let s = result.to_string();
        if s.len() > max_len {
            format!("{}...", safe_truncate(&s, max_len))
        } else {
            s
        }
    }
    
    /// 提取 HTML 元素
    fn extract_html_elements<'a>(&self, html: &'a str, tag: &str) -> Vec<&'a str> {
        let start_tag = format!("<{}", tag);
        let mut elements = Vec::new();
        let mut pos = 0;
        
        while let Some(start) = html[pos..].find(&start_tag) {
            let abs_start = pos + start;
            if let Some(end) = html[abs_start..].find('>') {
                elements.push(&html[abs_start..abs_start + end + 1]);
                pos = abs_start + end + 1;
            } else {
                break;
            }
        }
        
        elements
    }
    
    /// 提取 HTML 属性值
    fn extract_attr<'a>(&self, element: &'a str, attr: &str) -> Option<&'a str> {
        let pattern = format!("{}=\"", attr);
        if let Some(start) = element.find(&pattern) {
            let value_start = start + pattern.len();
            if let Some(end) = element[value_start..].find('"') {
                return Some(&element[value_start..value_start + end]);
            }
        }
        None
    }
    
    /// 截断 Thought 内容
    fn truncate_thought(&self, thought: &str) -> String {
        let max_len = 300;
        if thought.len() > max_len {
            format!("{}...", safe_truncate(thought, max_len))
        } else {
            thought.to_string()
        }
    }
    
    /// 压缩 Action Input (返回字符串)
    fn compress_action_input(&self, args: &serde_json::Value) -> String {
        let max_len = 200;
        let s = args.to_string();
        if s.len() > max_len {
            format!("{}...", safe_truncate(&s, max_len))
        } else {
            s
        }
    }
    
    /// 压缩 Action Input (返回 JSON Value)
    fn compress_action_input_json(&self, args: &serde_json::Value) -> serde_json::Value {
        // 如果是对象，只保留关键字段
        if let Some(obj) = args.as_object() {
            let mut compressed = serde_json::Map::new();
            for (key, value) in obj {
                // 压缩过长的字符串值
                if let Some(s) = value.as_str() {
                    if s.len() > 100 {
                        compressed.insert(key.clone(), serde_json::json!(format!("{}...", safe_truncate(s, 100))));
                    } else {
                        compressed.insert(key.clone(), value.clone());
                    }
                } else {
                    compressed.insert(key.clone(), value.clone());
                }
            }
            serde_json::Value::Object(compressed)
        } else {
            args.clone()
        }
    }
    
    /// 滑动窗口 - 保留最近N轮历史 (JSON 格式，每轮1条)
    fn apply_history_window(&self, history: &mut Vec<String>) {
        // 查找 Context 条目（初始上下文，始终保留）
        let context_entries: Vec<String> = history.iter()
            .filter(|s| s.starts_with("Context:") || s.contains("\"context\""))
            .cloned()
            .collect();
        
        // 移除 Context 条目，只对历史进行窗口化
        history.retain(|s| !s.starts_with("Context:") && !s.contains("\"context\""));
        
        // 每轮现在是1条 JSON 条目
        if history.len() > self.history_window_size {
            let entries_to_remove = history.len() - self.history_window_size;
            history.drain(0..entries_to_remove);
            
            log::info!("History window applied: removed {} entries, {} remain", 
                entries_to_remove, history.len());
        }
        
        // 重新添加 Context（如果有的话，只保留压缩版本）
        if !context_entries.is_empty() {
            let compressed_context = self.compress_initial_context(&context_entries[0]);
            history.insert(0, compressed_context);
        }
    }
    
    /// 压缩初始上下文
    fn compress_initial_context(&self, context: &str) -> String {
        // 尝试解析为 JSON 并提取关键字段
        if let Some(json_start) = context.find('{') {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&context[json_start..]) {
                if let Some(obj) = value.as_object() {
                    let mut summary = serde_json::Map::new();
                    
                    // 只保留关键字段
                    for key in &["target", "query", "task_type", "target_type"] {
                        if let Some(v) = obj.get(*key) {
                            summary.insert((*key).to_string(), v.clone());
                        }
                    }
                    
                    // 保留工具列表（只保留数量）
                    if let Some(tools) = obj.get("tools_allow").and_then(|v| v.as_array()) {
                        summary.insert("tools_count".to_string(), 
                            serde_json::json!(tools.len()));
                    }
                    
                    return format!("Context: {}", serde_json::Value::Object(summary));
                }
            }
        }
        
        // 降级：截断
        let max_len = 500;
        if context.len() > max_len {
            format!("{}...", safe_truncate(context, max_len))
        } else {
            context.to_string()
        }
    }
}

/// 解析后的LLM输出
struct ParsedOutput {
    thought: String,
    action: Option<ReactToolCall>,
    final_answer: Option<String>,
}

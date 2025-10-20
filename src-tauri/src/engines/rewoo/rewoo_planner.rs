//! ReWOO Planner 实现
//! 
//! 基于 LangGraph ReWOO 标准实现的 Planner 模块
//! 负责生成标准格式的执行计划：Plan: <reasoning> #E1 = Tool[args]
use super::*;

use crate::ai_adapter::{AiProvider, Tool};
use crate::utils::ordered_message::ChunkType;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use crate::services::prompt_db::PromptRepository;

/// ReWOO Planner - 负责生成执行计划
pub struct ReWOOPlanner {
    /// AI 提供商
    ai_provider: Arc<dyn AiProvider>,
    /// 框架适配器
    framework_adapter: Arc<dyn crate::tools::FrameworkToolAdapter>,
    /// 配置
    config: PlannerConfig,
    /// 计划解析正则表达式
    plan_regex: Regex,
    /// 动态Prompt仓库
    prompt_repo: Option<PromptRepository>,
    /// 运行期参数（来自 AgentTask.parameters），用于 prompt_ids 覆盖
    runtime_params: Option<std::collections::HashMap<String, serde_json::Value>>,
    /// AI服务管理器（用于统一的流式消息发送）
    ai_service_manager: Option<Arc<crate::services::ai::AiServiceManager>>,
}

impl ReWOOPlanner {
    pub fn set_runtime_params(&mut self, params: std::collections::HashMap<String, serde_json::Value>) {
        self.runtime_params = Some(params);
    }
    /// 创建新的 Planner
    pub fn new(
        ai_provider: Arc<dyn AiProvider>,
        framework_adapter: Arc<dyn crate::tools::FrameworkToolAdapter>,
        config: PlannerConfig,
        prompt_repo: Option<PromptRepository>,
    ) -> Result<Self, ReWOOError> {
        // 用于解析计划步骤的正则表达式
        // 匹配格式：#E1 = Tool[args]
        let plan_regex = Regex::new(r"#E(\d+)\s*=\s*(\w+)\[([^\]]+)\]")
            .map_err(|e| ReWOOError::ConfigurationError(format!("Invalid regex: {}", e)))?;
        
        Ok(Self {
            ai_provider,
            framework_adapter,
            config,
            plan_regex,
            prompt_repo,
            runtime_params: None,
            ai_service_manager: None,
        })
    }

    /// 创建带AI服务管理器的 Planner
    pub fn new_with_ai_service_manager(
        ai_provider: Arc<dyn AiProvider>,
        framework_adapter: Arc<dyn crate::tools::FrameworkToolAdapter>,
        config: PlannerConfig,
        prompt_repo: Option<PromptRepository>,
        ai_service_manager: Arc<crate::services::ai::AiServiceManager>,
    ) -> Result<Self, ReWOOError> {
        // 用于解析计划步骤的正则表达式
        // 匹配格式：#E1 = Tool[args]
        let plan_regex = Regex::new(r"#E(\d+)\s*=\s*(\w+)\[([^\]]+)\]")
            .map_err(|e| ReWOOError::ConfigurationError(format!("Invalid regex: {}", e)))?;
        
        Ok(Self {
            ai_provider,
            framework_adapter,
            config,
            plan_regex,
            prompt_repo,
            runtime_params: None,
            ai_service_manager: Some(ai_service_manager),
        })
    }
    
    /// 将FrameworkAdapter提供的可用工具名称转换为LLM函数调用的Tool定义
    async fn build_tools_from_manager(&self) -> Vec<Tool> {
        let names = self.framework_adapter.list_available_tools().await;
        let mut tools = Vec::new();
        for name in names {
            // 从适配器获取参数定义，转换为 JSON Schema
            let schema = if let Some(tool_info) = self.framework_adapter.get_tool_info(&name).await {
                let params = &tool_info.parameters;
                // 将 ParameterDefinition 列表转为 OpenAI function schema
                let mut properties = serde_json::Map::new();
                let mut required: Vec<String> = Vec::new();
                for p in &params.parameters {
                    let ty = match p.param_type {
                        crate::tools::ParameterType::String => "string",
                        crate::tools::ParameterType::Number => "number",
                        crate::tools::ParameterType::Boolean => "boolean",
                        crate::tools::ParameterType::Array => "array",
                        crate::tools::ParameterType::Object => "object",
                    };
                    let mut prop = serde_json::json!({
                        "type": ty,
                        "description": p.description,
                    });
                    if let Some(defv) = &p.default_value {
                        if let Some(obj) = prop.as_object_mut() {
                            obj.insert("default".to_string(), defv.clone());
                        }
                    }
                    properties.insert(p.name.clone(), prop);
                    if p.required {
                        required.push(p.name.clone());
                    }
                }
                serde_json::json!({
                    "type": "object",
                    "properties": properties,
                    "required": required,
                    "additionalProperties": false
                })
            } else {
                serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": true
                })
            };

            tools.push(Tool {
                r#type: "function".to_string(),
                name: name.clone(),
                description: format!("Tool {}", name),
                parameters: schema,
            });
        }
        tools
    }
    /// 生成计划
    pub async fn plan(&self, state: &mut ReWOOState) -> Result<(), ReWOOError> {
        let _start_time: SystemTime = SystemTime::now();
        
        // 构建计划生成提示（区分 system 与 user）
        let (system_prompt, user_prompt) = self.build_planning_prompts(&state.task).await?;
        
        // 调用 AI 生成计划（使用统一的流式消息方法）
        let plan_string = self.generate_plan_string(&system_prompt, &user_prompt).await?;
        
        // 解析计划步骤
        let steps = self.parse_plan_steps(&plan_string)?;
        
        // 更新状态
        state.plan_string = plan_string;
        state.steps = steps;
        
        Ok(())
    }
    
    /// 构建规划所需的 system 与 user 提示词（参考 plan-and-execute 的实现）
    async fn build_planning_prompts(&self, task: &str) -> Result<(String, String), ReWOOError> {
        use crate::utils::prompt_resolver::{PromptResolver, CanonicalStage, AgentPromptConfig};
        use crate::models::prompt::ArchitectureType;

        // 获取可用工具信息（放入 system 提示）
        let tools_block = self.build_tools_information().await;

        // 解析并获取 system 模板（仅渲染通用变量，避免注入用户任务内容）
        let system_template = if let Some(repo) = &self.prompt_repo {
            let resolver = PromptResolver::new(repo.clone());
            let empty: HashMap<String, serde_json::Value> = HashMap::new();
            let params_ref = self.runtime_params.as_ref().unwrap_or(&empty);
            let agent_config = AgentPromptConfig::parse_agent_config(params_ref);
            resolver
                .resolve_prompt(
                    &agent_config,
                    ArchitectureType::ReWOO,
                    CanonicalStage::Planner,
                    Some(&"".to_string()),
                )
                .await
                .unwrap_or_else(|_| "".to_string())
        } else {
           "".to_string()
        };

        // 仅渲染通用上下文（如工具清单）到 system 提示
        let mut system_ctx = HashMap::new();
        system_ctx.insert("tools".to_string(), serde_json::Value::String(tools_block.clone()));
        let mut system_prompt = if let Some(repo) = &self.prompt_repo {
            let resolver = PromptResolver::new(repo.clone());
            resolver
                .render_variables(&system_template, &system_ctx)
                .unwrap_or(system_template)
        } else {
            // 使用默认的 system prompt 模板
           "".to_string()
        };

        // 将具体任务信息放入 user 提示
        let user_prompt = format!("{}", task);

        // RAG augmentation for ReWOO planning (global toggle)
        if let Ok(rag_service) = crate::commands::rag_commands::get_global_rag_service().await {
            if rag_service.get_config().augmentation_enabled {
                use tokio::time::{timeout, Duration};
                let (primary, fallback) = crate::rag::query_utils::build_rag_query_pair(task);
                let rag_request = crate::rag::models::AssistantRagRequest {
                    query: primary.clone(),
                    collection_id: None,
                    conversation_history: None,
                    top_k: Some(5),
                    use_mmr: Some(true),
                    mmr_lambda: Some(0.7),
                    similarity_threshold: Some(0.65),
                    reranking_enabled: Some(false),
                    model_provider: None,
                    model_name: None,
                    max_tokens: None,
                    temperature: None,
                };
                if let Ok(Ok((knowledge_context, _))) = timeout(
                    Duration::from_millis(1200),
                    rag_service.query_for_assistant(&rag_request),
                )
                .await
                {
                    if !knowledge_context.trim().is_empty() {
                        log::info!("Augmenting ReWOO planner system prompt with RAG context");
                        system_prompt.push_str("\n\n[KNOWLEDGE CONTEXT]\n");
                        system_prompt.push_str(&knowledge_context);
                    } else {
                        let fallback_req = crate::rag::models::AssistantRagRequest {
                            query: fallback,
                            collection_id: None,
                            conversation_history: None,
                            top_k: Some(7),
                            use_mmr: Some(true),
                            mmr_lambda: Some(0.7),
                            similarity_threshold: Some(0.55),
                            reranking_enabled: Some(false),
                            model_provider: None,
                            model_name: None,
                            max_tokens: None,
                            temperature: None,
                        };
                        if let Ok(Ok((kb2, _))) = timeout(Duration::from_millis(1200), rag_service.query_for_assistant(&fallback_req)).await {
                            if !kb2.trim().is_empty() {
                                log::info!("Augmenting ReWOO planner system prompt with fallback RAG context");
                                system_prompt.push_str("\n\n[KNOWLEDGE CONTEXT]\n");
                                system_prompt.push_str(&kb2);
                            }
                        }
                    }
                }
            }
        }

        Ok((system_prompt, user_prompt))
    }

    /// 构建工具信息块
    async fn build_tools_information(&self) -> String {
        use std::collections::HashSet;
        // 读取Agent传入的工具白名单/黑名单
        let (allow, deny): (HashSet<String>, HashSet<String>) = {
            let empty_params = HashMap::new();
            let params = self.runtime_params.as_ref().unwrap_or(&empty_params);
            let allow = params
                .get("tools_allow")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_else(HashSet::new);
            let deny = params
                .get("tools_deny")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_else(HashSet::new);
            (allow, deny)
        };
        
        // 动态获取可用工具名称
        let tool_names = self.framework_adapter.list_available_tools().await;
        // 生成带参数签名的工具清单，形如：
        // - rsubdomain(domain: string, use_database_wordlist?: boolean)
        let mut tool_lines: Vec<String> = Vec::new();
        for name in &tool_names {
            // 过滤白名单/黑名单
            // 如果有白名单且工具不在白名单中，跳过
            if !allow.is_empty() && !allow.contains(name) {
                continue;
            }
            // 如果没有白名单（空数组），则不允许任何工具
            if allow.is_empty() {
                continue;
            }
            // 如果工具在黑名单中，跳过
            if deny.contains(name) {
                continue;
            }
            if let Some(tool_info) = self.framework_adapter.get_tool_info(name).await {
                let params = &tool_info.parameters;
                let mut parts: Vec<String> = Vec::new();
                for p in &params.parameters {
                    let ty = match p.param_type {
                        crate::tools::ParameterType::String => "string",
                        crate::tools::ParameterType::Number => "number",
                        crate::tools::ParameterType::Boolean => "boolean",
                        crate::tools::ParameterType::Array => "array",
                        crate::tools::ParameterType::Object => "object",
                    };
                    let piece = if p.required {
                        format!("{}: {}", p.name, ty)
                    } else {
                        format!("{}?: {}", p.name, ty)
                    };
                    parts.push(piece);
                }
                let sig = if parts.is_empty() { String::new() } else { parts.join(", ") };
                tool_lines.push(format!("- {}({})", name, sig));
            } else {
                tool_lines.push(format!("- {}()", name));
            }
        }
        if tool_lines.is_empty() {
            "无工具可用".to_string()
        } else {
            tool_lines.join("\n")
        }
    }

    /// 调用 AI 进行规划（使用 ai_service 的 send_message_stream 方法）
    async fn generate_plan_string(&self, system_prompt: &str, user_prompt: &str) -> Result<String, ReWOOError> {
        // 尝试获取模型配置配置的规划模型
        let ai_config = self.get_planning_ai_config().await?;
        
        let (provider_name, model_name) = if let Some(config) = ai_config {
            log::info!("使用模型配置配置的规划模型: {} ({})", config.model, config.provider);
            (config.provider, config.model)
        } else {
            log::info!("使用默认规划模型: {} ({})", self.config.model_name, "default");
            ("default".to_string(), self.config.model_name.clone())
        };
        
        // 获取AI服务
        let ai_service = if let Some(ref ai_service_manager) = self.ai_service_manager {
            match ai_service_manager.find_service_by_provider_and_model(&provider_name, &model_name).await {
                Ok(Some(service)) => service,
                Ok(None) => {
                    return Err(ReWOOError::AiProviderError(format!(
                        "无法找到提供商 '{}' 的模型 '{}'", provider_name, model_name
                    )));
                },
                Err(e) => {
                    return Err(ReWOOError::AiProviderError(format!(
                        "查找AI服务失败: {}", e
                    )));
                }
            }
        } else {
            return Err(ReWOOError::AiProviderError(
                "AI服务管理器未初始化".to_string()
            ));
        };
        
        // 使用流式消息API发送请求（分别传递 system 与 user 提示）
        // 注意：规划步骤是无状态的，不需要保存到会话历史
        let execution_id = uuid::Uuid::new_v4().to_string();
        let result = ai_service.send_message_stream(
            Some(user_prompt), // 用户提示
            Some(system_prompt), // 系统提示
            None, // 不指定会话ID，保持无状态
            Some(execution_id.clone()), // 消息ID用于前端显示
            false,
            false,
            Some(ChunkType::Content),
        ).await.map_err(|e| ReWOOError::AiProviderError(format!("AI service call failed: {}", e)))?;
        
        log::info!("AI响应内容: {}", result);
        Ok(result)
    }

    /// 获取规划阶段的AI配置
    async fn get_planning_ai_config(&self) -> Result<Option<crate::services::ai::AiConfig>, ReWOOError> {
        // 如果有统一提示词参数，尝试从中解析规划模型配置
        if let Some(params) = &self.runtime_params {
            // 从统一提示词系统获取配置（如果有的话）
            if let Some(prompt_ids) = params.get("prompt_ids") {
                if let Some(planner_id) = prompt_ids.get("planner") {
                    log::debug!("Found planner prompt template ID: {:?}", planner_id);
                    // 这里可以进一步解析提示词模板中的模型配置
                }
            }
        }
        
        // 当前返回None，使用默认配置
        Ok(None)
    }

    /// 解析计划步骤 - 增加二道门防护
    fn parse_plan_steps(&self, plan_string: &str) -> Result<Vec<String>, ReWOOError> {
        // 第一道门：预清洗输出
        let cleaned_plan = self.clean_plan_output(plan_string);
        
        let mut steps = Vec::new();
        let mut step_numbers = std::collections::HashSet::new();
        
        // 按行分割并查找步骤
        for line in cleaned_plan.lines() {
            let line = line.trim();
            if line.starts_with("#E") {
                // 验证步骤格式
                if self.plan_regex.is_match(line) {
                    // 第二道门：验证步骤序号连续性
                    if let Some(captures) = self.plan_regex.captures(line) {
                        let step_num: u32 = captures[1].parse().unwrap_or(0);
                        
                        // 检查重复序号
                        if step_numbers.contains(&step_num) {
                            log::warn!("Duplicate step number {} detected, skipping", step_num);
                            continue;
                        }
                        step_numbers.insert(step_num);
                        
                        // 检查序号合理性（应该从1开始且连续）
                        if step_num == 0 || step_num > self.config.max_steps {
                            log::warn!("Invalid step number {} detected, skipping", step_num);
                            continue;
                        }
                    }
                    
                    steps.push(line.to_string());
                } else {
                    log::warn!("Invalid step format detected: {}", line);
                }
            }
        }
        
        // 第三道门：验证步骤序号连续性
        steps = self.ensure_step_continuity(steps)?;
        
        if steps.is_empty() {
            return Err(ReWOOError::PlanParsingError(
                "No valid steps found in plan after cleanup".to_string()
            ));
        }
        
        if steps.len() > self.config.max_steps as usize {
            log::warn!("Too many steps detected ({}), truncating to {}", steps.len(), self.config.max_steps);
            steps.truncate(self.config.max_steps as usize);
        }
        
        Ok(steps)
    }
    
    /// 清洗计划输出
    fn clean_plan_output(&self, plan_string: &str) -> String {
        let mut cleaned_lines = Vec::new();
        let mut in_code_block = false;
        let max_lines = 100; // 最大行数限制
        
        for (i, line) in plan_string.lines().enumerate() {
            if i >= max_lines {
                log::warn!("Plan output too long, truncating at {} lines", max_lines);
                break;
            }
            
            let line = line.trim();
            
            // 跳过空行
            if line.is_empty() {
                continue;
            }
            
            // 处理代码块标记
            if line.starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }
            
            // 跳过代码块内容（除非是步骤）
            if in_code_block && !line.starts_with("#E") {
                continue;
            }
            
            // 过滤无效行
            if self.is_valid_plan_line(line) {
                cleaned_lines.push(line.to_string());
            }
        }
        
        cleaned_lines.join("\n")
    }
    
    /// 检查是否为有效的计划行
    fn is_valid_plan_line(&self, line: &str) -> bool {
        // 允许的行类型：
        // 1. Plan: 开头的推理行
        // 2. #E 开头的步骤行
        // 3. 一些说明性文字（但要限制）
        
        if line.starts_with("Plan:") || line.starts_with("#E") {
            return true;
        }
        
        // 过滤明显的垃圾内容
        let invalid_patterns = [
            "```", "json", "python", "javascript", "curl", "http", "www.",
            "example.com", "localhost", "127.0.0.1", "AI", "model", "prompt"
        ];
        
        for pattern in &invalid_patterns {
            if line.to_lowercase().contains(pattern) {
                return false;
            }
        }
        
        // 允许简短的说明行（不超过100字符）
        line.len() <= 100
    }
    
    /// 确保步骤序号连续性
    fn ensure_step_continuity(&self, steps: Vec<String>) -> Result<Vec<String>, ReWOOError> {
        if steps.is_empty() {
            return Ok(steps);
        }
        
        // 提取步骤序号并排序
        let mut step_pairs: Vec<(u32, String)> = Vec::new();
        
        for step in steps {
            if let Some(captures) = self.plan_regex.captures(&step) {
                let step_num: u32 = captures[1].parse().unwrap_or(0);
                step_pairs.push((step_num, step));
            }
        }
        
        // 按序号排序
        step_pairs.sort_by_key(|(num, _)| *num);
        
        // 重新编号以确保连续性
        let mut renumbered_steps = Vec::new();
        for (i, (_, step)) in step_pairs.into_iter().enumerate() {
            let new_step_num = i + 1;
            let renumbered_step = self.renumber_step(&step, new_step_num);
            renumbered_steps.push(renumbered_step);
        }
        
        Ok(renumbered_steps)
    }
    
    /// 重新编号步骤
    fn renumber_step(&self, step: &str, new_num: usize) -> String {
        if let Some(captures) = self.plan_regex.captures(step) {
            let tool = &captures[2];
            let args = &captures[3];
            format!("#E{} = {}[{}]", new_num, tool, args)
        } else {
            step.to_string()
        }
    }
    
    /// 解析单个步骤
    pub fn parse_step(&self, step: &str) -> Result<PlanStep, ReWOOError> {
        if let Some(captures) = self.plan_regex.captures(step) {
            let variable = format!("#E{}", &captures[1]);
            let tool = captures[2].to_string();
            let args = captures[3].to_string();
            
            // 从计划字符串中提取推理部分（Plan: 后面的内容）
            let reasoning = self.extract_reasoning_for_step(step);
            
            Ok(PlanStep {
                variable,
                tool,
                args,
                reasoning,
            })
        } else {
            Err(ReWOOError::PlanParsingError(
                format!("Cannot parse step: {}", step)
            ))
        }
    }
    
    /// 解析单个步骤（带计划字符串） - 新增重载
    pub fn parse_step_with_plan(&self, step: &str, plan_string: &str) -> Result<PlanStep, ReWOOError> {
        if let Some(captures) = self.plan_regex.captures(step) {
            let variable = format!("#E{}", &captures[1]);
            let tool = captures[2].to_string();
            let args = captures[3].to_string();
            
            // 从完整计划字符串中提取推理部分
            let reasoning = self.extract_reasoning_from_plan(plan_string, &variable);
            
            Ok(PlanStep {
                variable,
                tool,
                args,
                reasoning,
            })
        } else {
            Err(ReWOOError::PlanParsingError(
                format!("Cannot parse step: {}", step)
            ))
        }
    }
    
    /// 提取步骤的推理部分 - 从 plan_string 中提取对应的 Plan: 内容
    fn extract_reasoning_for_step(&self, step: &str) -> String {
        // 提取步骤变量（如 #E1）
        if let Some(captures) = self.plan_regex.captures(step) {
            let variable = format!("#E{}", &captures[1]);
            
            // 在当前 state 中查找对应的 plan_string（需要传入 plan_string）
            // 由于方法签名限制，这里先返回基础实现
            // 实际应该在 parse_step 时同时传入 plan_string
            format!("Execute step {}", variable)
        } else {
            "Executing planned step".to_string()
        }
    }
    
    /// 从完整计划字符串中提取推理 - 新增方法
    pub fn extract_reasoning_from_plan(&self, plan_string: &str, step_variable: &str) -> String {
        let lines: Vec<&str> = plan_string.lines().collect();
        let mut reasoning = String::new();
        let mut found_step = false;
        
        for (i, line) in lines.iter().enumerate() {
            let line = line.trim();
            
            // 查找包含目标步骤变量的行
            if line.contains(step_variable) && self.plan_regex.is_match(line) {
                found_step = true;
                
                // 向前查找最近的 Plan: 行
                for j in (0..i).rev() {
                    let prev_line = lines[j].trim();
                    if prev_line.starts_with("Plan:") {
                        reasoning = prev_line.strip_prefix("Plan:").unwrap_or(prev_line).trim().to_string();
                        break;
                    }
                }
                break;
            }
        }
        
        if reasoning.is_empty() {
            if found_step {
                format!("Execute {}", step_variable)
            } else {
                "Executing planned step".to_string()
            }
        } else {
            reasoning
        }
    }
    
    /// 获取当前未执行的步骤
    pub fn get_current_step(&self, state: &ReWOOState) -> Option<String> {
        for step in &state.steps {
            if let Ok(parsed_step) = self.parse_step(step) {
                if !state.results.contains_key(&parsed_step.variable) {
                    return Some(step.clone());
                }
            }
        }
        None
    }
    
    /// 检查是否所有步骤都已完成
    pub fn all_steps_completed(&self, state: &ReWOOState) -> bool {
        for step in &state.steps {
            if let Ok(parsed_step) = self.parse_step(step) {
                if !state.results.contains_key(&parsed_step.variable) {
                    return false;
                }
            }
        }
        true
    }
    
    /// 替换步骤中的变量引用 - 支持字段路径 (#E1.field)
    pub fn substitute_variables(&self, args: &str, results: &HashMap<String, serde_json::Value>) -> String {
        let mut substituted = args.to_string();
        
        // 查找并替换字段路径引用 (#E1.field.subfield)
        let field_regex = Regex::new(r"#E(\d+)\.([a-zA-Z_][a-zA-Z0-9_.]*)")
            .unwrap_or_else(|_| Regex::new(r"#E(\d+)").unwrap());
        
        for captures in field_regex.captures_iter(args) {
            let full_match = captures.get(0).unwrap().as_str();
            let var_name = format!("#E{}", &captures[1]);
            
            if let Some(result_value) = results.get(&var_name) {
                let replacement = if captures.len() > 2 {
                    // 字段路径访问 (#E1.field)
                    let field_path = &captures[2];
                    self.extract_field_from_json(result_value, field_path)
                } else {
                    // 直接变量引用 (#E1)
                    self.json_to_string(result_value)
                };
                substituted = substituted.replace(full_match, &replacement);
            }
        }
        
        // 处理简单变量引用 (#E1)
        let var_regex = Regex::new(r"#E(\d+)").unwrap();
        for captures in var_regex.captures_iter(args) {
            let full_match = captures.get(0).unwrap().as_str();
            if let Some(value) = results.get(full_match) {
                let replacement = self.json_to_string(value);
                substituted = substituted.replace(full_match, &replacement);
            }
        }
        
        substituted
    }
    
    /// 从 JSON 值中提取字段路径
    fn extract_field_from_json(&self, value: &serde_json::Value, field_path: &str) -> String {
        let mut current = value;
        
        for field in field_path.split('.') {
            match current {
                serde_json::Value::Object(obj) => {
                    if let Some(next_value) = obj.get(field) {
                        current = next_value;
                    } else {
                        return format!("{{field_not_found: {}}}", field);
                    }
                }
                serde_json::Value::Array(arr) => {
                    if let Ok(index) = field.parse::<usize>() {
                        if let Some(next_value) = arr.get(index) {
                            current = next_value;
                        } else {
                            return format!("{{index_out_of_bounds: {}}}", field);
                        }
                    } else {
                        return format!("{{invalid_array_index: {}}}", field);
                    }
                }
                _ => return format!("{{cannot_access_field: {}}}", field),
            }
        }
        
        self.json_to_string(current)
    }
    
    /// 将 JSON 值转换为字符串
    fn json_to_string(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => "null".to_string(),
            _ => value.to_string(),
        }
    }
    
    /// 分析步骤依赖关系
    pub fn analyze_dependencies(&self, state: &ReWOOState) -> Vec<StepDependency> {
        let mut dependencies = Vec::new();
        
        for step in &state.steps {
            if let Ok(parsed_step) = self.parse_step_with_plan(step, &state.plan_string) {
                let step_deps = self.extract_variable_references(&parsed_step.args);
                dependencies.push(StepDependency {
                    variable: parsed_step.variable.clone(),
                    dependencies: step_deps,
                    ready: false,
                });
            }
        }
        
        dependencies
    }
    
    /// 提取参数中的变量引用
    fn extract_variable_references(&self, args: &str) -> Vec<String> {
        let var_regex = Regex::new(r"#E(\d+)").unwrap();
        let mut references = Vec::new();
        
        for captures in var_regex.captures_iter(args) {
            let variable = format!("#E{}", &captures[1]);
            if !references.contains(&variable) {
                references.push(variable);
            }
        }
        
        references
    }
    
    /// 创建执行批次（拓扑排序）
    pub fn create_execution_batches(&self, state: &ReWOOState) -> Vec<ExecutionBatch> {
        let mut dependencies = self.analyze_dependencies(state);
        let mut batches = Vec::new();
        let mut batch_id = 0;
        let mut completed_variables = std::collections::HashSet::new();
        
        while !dependencies.is_empty() {
            let mut current_batch_steps = Vec::new();
            
            // 找到当前可执行的步骤（无依赖或依赖已满足）
            for (i, dep) in dependencies.iter().enumerate() {
                let ready = dep.dependencies.iter().all(|var| completed_variables.contains(var));
                if ready {
                    current_batch_steps.push(i);
                }
            }
            
            if current_batch_steps.is_empty() {
                // 检查是否有循环依赖
                log::warn!("Possible circular dependency detected, forcing execution of remaining steps");
                current_batch_steps.push(0); // 强制执行第一个剩余步骤
            }
            
            // 创建当前批次
            let mut batch_steps = Vec::new();
            for &index in current_batch_steps.iter().rev() { // 倒序移除
                let dep = dependencies.remove(index);
                batch_steps.push(dep.variable.clone());
                completed_variables.insert(dep.variable);
            }
            
            if !batch_steps.is_empty() {
                batches.push(ExecutionBatch {
                    batch_id,
                    steps: batch_steps,
                    completed: false,
                });
                batch_id += 1;
            }
        }
        
        batches
    }
    
    /// 生成步骤纠错建议
    pub async fn generate_correction_suggestions(
        &self,
        failed_step: &PlanStep,
        error_message: &str,
    ) -> Result<Vec<CorrectionSuggestion>, ReWOOError> {
        let mut suggestions = Vec::new();
        
        // 1. 检查工具是否可用
        let available_tools = self.framework_adapter.list_available_tools().await;
        if !available_tools.contains(&failed_step.tool) {
            // 建议替换工具
            if let Some(replacement) = self.find_replacement_tool(&failed_step.tool, &available_tools).await {
                suggestions.push(CorrectionSuggestion {
                    step_variable: failed_step.variable.clone(),
                    correction_type: CorrectionType::ToolReplacement {
                        original_tool: failed_step.tool.clone(),
                        replacement_tool: replacement.clone(),
                    },
                    confidence: 85.0,
                    reason: format!("Tool '{}' not available, suggested replacement: '{}'", failed_step.tool, replacement),
                    corrected_step: Some(format!("{} = {}[{}]", 
                        failed_step.variable, replacement, failed_step.args)),
                });
            }
        }
        
        // 2. 检查参数格式
        if error_message.contains("parameter") || error_message.contains("argument") {
            if let Some(corrected_args) = self.suggest_parameter_correction(&failed_step.tool, &failed_step.args).await {
                suggestions.push(CorrectionSuggestion {
                    step_variable: failed_step.variable.clone(),
                    correction_type: CorrectionType::ParameterFix {
                        original_args: failed_step.args.clone(),
                        corrected_args: corrected_args.clone(),
                    },
                    confidence: 70.0,
                    reason: "Parameter format correction suggested".to_string(),
                    corrected_step: Some(format!("{} = {}[{}]", 
                        failed_step.variable, failed_step.tool, corrected_args)),
                });
            }
        }
        
        // 3. 如果错误严重，建议移除步骤
        if error_message.contains("critical") || error_message.contains("fatal") {
            suggestions.push(CorrectionSuggestion {
                step_variable: failed_step.variable.clone(),
                correction_type: CorrectionType::StepRemoval {
                    reason: "Critical error in step execution".to_string(),
                },
                confidence: 60.0,
                reason: format!("Critical error detected: {}", error_message),
                corrected_step: None,
            });
        }
        
        Ok(suggestions)
    }
    
    /// 查找替代工具
    async fn find_replacement_tool(&self, original_tool: &str, available_tools: &[String]) -> Option<String> {
        // 简单的工具映射策略
        let tool_mappings = [
            ("nmap", "portscan"),
            ("portscan", "nmap"),
            ("subdomain", "rsubdomain"),
            ("rsubdomain", "subdomain"),
            ("nuclei", "vulnerability_scan"),
            ("vulnerability_scan", "nuclei"),
        ];
        
        for (from, to) in &tool_mappings {
            if original_tool.to_lowercase() == *from && available_tools.contains(&to.to_string()) {
                return Some(to.to_string());
            }
        }
        
        // 模糊匹配：找到名称最相似的工具
        let mut best_match = None;
        let mut best_score = 0;
        
        for tool in available_tools {
            let score = self.calculate_similarity(original_tool, tool);
            if score > best_score && score > 50 { // 至少50%相似度
                best_score = score;
                best_match = Some(tool.clone());
            }
        }
        
        best_match
    }
    
    /// 计算字符串相似度 (简化版Levenshtein距离)
    fn calculate_similarity(&self, s1: &str, s2: &str) -> i32 {
        let s1 = s1.to_lowercase();
        let s2 = s2.to_lowercase();
        
        if s1 == s2 { return 100; }
        if s1.contains(&s2) || s2.contains(&s1) { return 80; }
        
        let common_chars = s1.chars().filter(|c| s2.contains(*c)).count();
        let max_len = s1.len().max(s2.len());
        
        if max_len == 0 { return 0; }
        (common_chars * 100 / max_len) as i32
    }
    
    /// 建议参数纠正
    async fn suggest_parameter_correction(&self, tool_name: &str, args: &str) -> Option<String> {
        // 获取工具信息
        if let Some(tool_info) = self.framework_adapter.get_tool_info(tool_name).await {
            let params = &tool_info.parameters;
            
            // 如果参数为空但有必填参数
            if args.trim().is_empty() && !params.parameters.is_empty() {
                let required_params: Vec<_> = params.parameters.iter()
                    .filter(|p| p.required)
                    .collect();
                    
                if required_params.len() == 1 {
                    return Some(format!("{}: <value>", required_params[0].name));
                }
            }
            
            // 如果参数格式不正确，尝试转换为正确格式
            if !args.contains(':') && !args.contains('=') && !args.contains('{') {
                // 裸字符串，映射到第一个必填参数
                let required_params: Vec<_> = params.parameters.iter()
                    .filter(|p| p.required)
                    .collect();
                    
                if required_params.len() == 1 {
                    return Some(format!("{}: {}", required_params[0].name, args.trim()));
                }
            }
        }
        
        None
    }
    
    /// 应用纠错建议到状态
    pub fn apply_correction(&self, state: &mut ReWOOState, suggestion: &CorrectionSuggestion) -> Result<(), ReWOOError> {
        match &suggestion.correction_type {
            CorrectionType::ToolReplacement { .. } | 
            CorrectionType::ParameterFix { .. } => {
                if let Some(corrected_step) = &suggestion.corrected_step {
                    // 找到并替换原步骤
                    for (i, step) in state.steps.iter_mut().enumerate() {
                        if step.contains(&suggestion.step_variable) {
                            state.steps[i] = corrected_step.clone();
                            log::info!("Applied correction to step {}: {}", suggestion.step_variable, corrected_step);
                            break;
                        }
                    }
                }
            }
            CorrectionType::StepRemoval { reason } => {
                // 移除步骤
                state.steps.retain(|step| !step.contains(&suggestion.step_variable));
                log::info!("Removed step {} due to: {}", suggestion.step_variable, reason);
            }
            CorrectionType::ParameterAddition { missing_params } => {
                // 添加缺失参数（这里简化实现）
                log::info!("Parameter addition for step {}: {:?}", suggestion.step_variable, missing_params);
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use std::collections::HashMap;
    
    #[test]
    fn test_parse_step() {
        // let config = PlannerConfig {
        //     model_name: "test".to_string(),
        //     temperature: 0.0,
        //     max_tokens: 1000,
        //     max_steps: 10,
        // };
        
        // 创建一个模拟的 AI 提供商
        // 注意：这里需要一个测试用的 AI 提供商实现
        // let planner = ReWOOPlanner::new(ai_provider, config).unwrap();
        
        // let step = "#E1 = Search[vulnerability in Apache]";
        // let parsed = planner.parse_step(step).unwrap();
        
        // assert_eq!(parsed.variable, "#E1");
        // assert_eq!(parsed.tool, "Search");
        // assert_eq!(parsed.args, "vulnerability in Apache");
    }
    
    #[test]
    fn test_substitute_variables() {
        // let config = PlannerConfig {
        //     model_name: "test".to_string(),
        //     temperature: 0.0,
        //     max_tokens: 1000,
        //     max_steps: 10,
        // };
        
        // 创建一个模拟的 AI 提供商
        // let planner = ReWOOPlanner::new(ai_provider, config).unwrap();
        
        // let mut results = HashMap::new();
        // results.insert("#E1".to_string(), "192.168.1.1".to_string());
        
        // let substituted = planner.substitute_variables("scan #E1 for ports", &results);
        // assert_eq!(substituted, "scan 192.168.1.1 for ports");
    }
}

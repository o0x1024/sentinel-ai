//! 自主 Observe 模块
//!
//! 实现 LLM 驱动的自主信息收集策略：
//! - 动态决定收集什么信息
//! - 自适应选择工具
//! - 评估信息充分性
//! - 智能退出条件

use super::types::*;
use crate::engines::LlmClient;
use crate::services::ai::AiService;
use crate::services::prompt_db::PromptRepository;
use crate::tools::{FrameworkToolAdapter, UnifiedToolCall, ToolInfo};
use crate::utils::ordered_message::{emit_message_chunk_arc, ArchitectureType, ChunkType};
use crate::models::prompt::{ArchitectureType as PromptArchType, StageType};
use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// 自主观察器
pub struct AutonomousObserver {
    /// AI 服务
    ai_service: Arc<AiService>,
    /// Prompt 仓库
    prompt_repo: Option<Arc<PromptRepository>>,
    /// 工具适配器
    tool_adapter: Option<Arc<dyn FrameworkToolAdapter>>,
    /// 配置
    config: AutonomousObserveConfig,
    /// 取消令牌
    cancellation_token: Option<CancellationToken>,
    /// 消息发送
    app_handle: Option<Arc<tauri::AppHandle>>,
    execution_id: Option<String>,
    message_id: Option<String>,
    conversation_id: Option<String>,
}

/// 自主观察结果
#[derive(Debug, Clone)]
pub struct AutonomousObserveResult {
    /// 收集到的信息
    pub gathered_info: HashMap<String, serde_json::Value>,
    /// 收集的信息类型
    pub collected_types: Vec<InfoType>,
    /// 缺失的信息类型
    pub missing_types: Vec<InfoType>,
    /// 工具调用记录
    pub tool_calls: Vec<ObserveToolCall>,
    /// 充分性评估
    pub assessment: ObserveAssessment,
    /// 观察策略
    pub strategy_used: Option<ObserveStrategy>,
    /// 总耗时
    pub duration_ms: u64,
}

/// 观察工具调用记录
#[derive(Debug, Clone)]
pub struct ObserveToolCall {
    pub tool_name: String,
    pub arguments: HashMap<String, serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub success: bool,
    pub duration_ms: u64,
    pub info_types_gathered: Vec<InfoType>,
}

impl AutonomousObserver {
    pub fn new(ai_service: Arc<AiService>, config: AutonomousObserveConfig) -> Self {
        Self {
            ai_service,
            prompt_repo: None,
            tool_adapter: None,
            config,
            cancellation_token: None,
            app_handle: None,
            execution_id: None,
            message_id: None,
            conversation_id: None,
        }
    }

    pub fn with_prompt_repo(mut self, repo: Arc<PromptRepository>) -> Self {
        self.prompt_repo = Some(repo);
        self
    }

    pub fn with_tool_adapter(mut self, adapter: Arc<dyn FrameworkToolAdapter>) -> Self {
        self.tool_adapter = Some(adapter);
        self
    }

    pub fn with_cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancellation_token = Some(token);
        self
    }

    pub fn with_message_context(
        mut self,
        app_handle: Arc<tauri::AppHandle>,
        execution_id: String,
        message_id: String,
        conversation_id: Option<String>,
    ) -> Self {
        self.app_handle = Some(app_handle);
        self.execution_id = Some(execution_id);
        self.message_id = Some(message_id);
        self.conversation_id = conversation_id;
        self
    }

    /// 发送消息
    fn emit_message(&self, chunk_type: ChunkType, content: &str, data: Option<serde_json::Value>) {
        if let (Some(app_handle), Some(execution_id), Some(message_id)) =
            (&self.app_handle, &self.execution_id, &self.message_id)
        {
            emit_message_chunk_arc(
                app_handle,
                execution_id,
                message_id,
                self.conversation_id.as_deref(),
                chunk_type,
                content,
                false,
                Some("AutonomousObserver"),
                None,
                Some(ArchitectureType::Travel),
                data,
            );
        }
    }

    /// 执行自主观察
    pub async fn observe(
        &self,
        task_description: &str,
        target: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<AutonomousObserveResult> {
        let start_time = Instant::now();
        log::info!("AutonomousObserver: Starting observation for target: {}", target);

        self.emit_message(
            ChunkType::Thinking,
            "[OBSERVE] 开始自主观察...",
            Some(serde_json::json!({ "target": target })),
        );

        let llm_client = crate::engines::create_client(self.ai_service.as_ref());

        // 1. 分析任务，决定需要收集什么信息
        let strategy = self.plan_observation_strategy(
            &llm_client,
            task_description,
            target,
            context,
        ).await?;

        self.emit_message(
            ChunkType::PlanInfo,
            &format!("[STRATEGY] 使用策略: {}", strategy.name),
            Some(serde_json::json!({
                "strategy": strategy.name,
                "steps": strategy.collection_order.len(),
                "required_info": strategy.required_info.iter()
                    .map(|t| format!("{:?}", t))
                    .collect::<Vec<_>>()
            })),
        );

        // 2. 执行观察步骤
        let mut gathered_info: HashMap<String, serde_json::Value> = HashMap::new();
        let mut collected_types: HashSet<InfoType> = HashSet::new();
        let mut tool_calls: Vec<ObserveToolCall> = Vec::new();
        let mut tool_call_count = 0u32;

        for step in &strategy.collection_order {
            // 检查取消
            if let Some(token) = &self.cancellation_token {
                if token.is_cancelled() {
                    log::info!("AutonomousObserver: Cancelled");
                    break;
                }
            }

            // 检查是否超过最大工具调用次数
            if tool_call_count >= self.config.max_tool_calls {
                log::info!("AutonomousObserver: Max tool calls reached");
                break;
            }

            // 检查依赖是否满足
            let deps_satisfied = step.depends_on.iter().all(|dep| {
                gathered_info.contains_key(dep)
            });

            if !deps_satisfied && !step.optional {
                log::warn!("Step {} dependencies not satisfied", step.id);
                continue;
            }

            // 执行步骤
            self.emit_message(
                ChunkType::Content,
                &format!("[STEP] Executing: {}", step.objective),
                None,
            );

            let step_result = self.execute_observe_step(step, &gathered_info, context).await;

            match step_result {
                Ok((result, info_types)) => {
                    // 记录结果
                    let key = format!("{}_result", step.tool);
                    gathered_info.insert(key, result.clone());
                    
                    for info_type in &info_types {
                        collected_types.insert(info_type.clone());
                    }

                    tool_calls.push(ObserveToolCall {
                        tool_name: step.tool.clone(),
                        arguments: step.args_template.clone(),
                        result: Some(result),
                        success: true,
                        duration_ms: 0, // TODO: 实际计时
                        info_types_gathered: info_types,
                    });

                    tool_call_count += 1;
                }
                Err(e) => {
                    log::warn!("Step {} failed: {}", step.id, e);
                    
                    if !step.optional {
                        self.emit_message(
                            ChunkType::Error,
                            &format!("[FAIL] Step {} failed: {}", step.id, e),
                            None,
                        );
                    }

                    tool_calls.push(ObserveToolCall {
                        tool_name: step.tool.clone(),
                        arguments: step.args_template.clone(),
                        result: None,
                        success: false,
                        duration_ms: 0,
                        info_types_gathered: Vec::new(),
                    });

                    tool_call_count += 1;
                }
            }

            // 评估是否已收集足够信息
            let current_sufficiency = self.calculate_sufficiency(&collected_types, &strategy.required_info);
            if current_sufficiency >= self.config.sufficiency_threshold {
                log::info!("AutonomousObserver: Sufficiency threshold reached ({:.2})", current_sufficiency);
                break;
            }
        }

        // 3. 如果信息不足且启用自适应，尝试额外收集
        if self.config.adaptive_strategy {
            let current_sufficiency = self.calculate_sufficiency(&collected_types, &strategy.required_info);
            if current_sufficiency < self.config.sufficiency_threshold && tool_call_count < self.config.max_tool_calls {
                self.emit_message(
                    ChunkType::Thinking,
                    "[ADAPTIVE] 信息不足，尝试额外收集...",
                    None,
                );

                let additional_result = self.adaptive_collection(
                    &llm_client,
                    task_description,
                    target,
                    &gathered_info,
                    &collected_types,
                    &strategy.required_info,
                    context,
                    self.config.max_tool_calls - tool_call_count,
                ).await?;

                // 合并额外收集的信息
                for (key, value) in additional_result.0 {
                    gathered_info.insert(key, value);
                }
                for info_type in additional_result.1 {
                    collected_types.insert(info_type);
                }
                tool_calls.extend(additional_result.2);
            }
        }

        // 4. 最终评估
        let collected_types_vec: Vec<InfoType> = collected_types.into_iter().collect();
        let missing_types: Vec<InfoType> = strategy.required_info.iter()
            .filter(|t| !collected_types_vec.contains(t))
            .cloned()
            .collect();

        let assessment = ObserveAssessment {
            sufficiency_score: self.calculate_sufficiency_from_vec(&collected_types_vec, &strategy.required_info),
            collected_info: collected_types_vec.clone(),
            missing_info: missing_types.clone(),
            suggested_next_steps: if missing_types.is_empty() {
                Vec::new()
            } else {
                missing_types.iter()
                    .map(|t| format!("Collect {:?} information", t))
                    .collect()
            },
            ready_for_orient: missing_types.is_empty() || 
                self.calculate_sufficiency_from_vec(&collected_types_vec, &strategy.required_info) >= self.config.sufficiency_threshold,
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        self.emit_message(
            ChunkType::Content,
            &format!(
                "[COMPLETE] 观察完成: {:.0}% 充分, {} 类型收集",
                assessment.sufficiency_score * 100.0,
                collected_types_vec.len()
            ),
            Some(serde_json::json!({
                "sufficiency": assessment.sufficiency_score,
                "collected": collected_types_vec.iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>(),
                "missing": missing_types.iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>(),
                "tool_calls": tool_calls.len()
            })),
        );

        Ok(AutonomousObserveResult {
            gathered_info,
            collected_types: collected_types_vec,
            missing_types,
            tool_calls,
            assessment,
            strategy_used: Some(strategy),
            duration_ms,
        })
    }

    /// 规划观察策略
    async fn plan_observation_strategy(
        &self,
        llm_client: &LlmClient,
        task_description: &str,
        target: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<ObserveStrategy> {
        log::info!("AutonomousObserver: Planning observation strategy");

        // 获取可用工具
        let available_tools = self.get_available_tools(context).await;

        // 构建策略规划 prompt
        let system_prompt = self.get_strategy_planning_prompt().await;

        let user_prompt = format!(
            r#"任务: {}
目标: {}
任务类型: {}

可用工具:
{}

请规划观察策略，输出 JSON 格式:
```json
{{
  "name": "策略名称",
  "required_info": ["TargetStructure", "ApiEndpoints", ...],
  "steps": [
    {{
      "id": "1",
      "objective": "步骤目标",
      "tool": "工具名",
      "args": {{"arg1": "value1"}},
      "depends_on": [],
      "optional": false
    }}
  ],
  "success_criteria": "成功条件描述"
}}
```"#,
            task_description,
            target,
            context.get("task_type").and_then(|v| v.as_str()).unwrap_or("unknown"),
            available_tools,
        );

        let response = llm_client
            .completion(Some(&system_prompt), &user_prompt)
            .await?;

        self.parse_strategy_response(&response, target)
    }

    /// 获取策略规划 prompt
    async fn get_strategy_planning_prompt(&self) -> String {
        // 尝试从数据库获取
        if let Some(repo) = &self.prompt_repo {
            if let Ok(Some(template)) = repo
                .get_template_by_arch_stage(PromptArchType::Travel, StageType::Observe)
                .await
            {
                return template.content;
            }
        }

        // 默认 prompt
        r#"你是一个信息收集策略专家。根据任务和目标，规划最优的信息收集策略。

## 信息类型
- TargetStructure: 目标结构（页面、路由等）
- ApiEndpoints: API 端点
- FormsAndInputs: 表单和输入点
- TechStack: 技术栈
- Authentication: 认证机制
- ErrorMessages: 错误信息
- Configuration: 配置信息

## 规则
1. 根据任务类型选择需要收集的信息
2. 优先收集最关键的信息
3. 考虑步骤之间的依赖关系
4. 标记可选步骤

只输出 JSON，不要其他文字。"#.to_string()
    }

    /// 获取可用工具
    async fn get_available_tools(&self, context: &HashMap<String, serde_json::Value>) -> String {
        let allowed_tools: Vec<String> = context
            .get("tools_allow")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        if allowed_tools.is_empty() {
            return "No tools available".to_string();
        }

        let mut descriptions = Vec::new();

        if let Some(adapter) = &self.tool_adapter {
            for tool_name in &allowed_tools {
                if let Some(info) = adapter.get_tool_info(tool_name).await {
                    descriptions.push(format!("- {}: {}", info.name, info.description));
                }
            }
        } else {
            for name in &allowed_tools {
                descriptions.push(format!("- {}", name));
            }
        }

        descriptions.join("\n")
    }

    /// 解析策略响应
    fn parse_strategy_response(&self, response: &str, target: &str) -> Result<ObserveStrategy> {
        // 提取 JSON
        let json_str = if response.contains("```json") {
            response
                .split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(response)
                .trim()
        } else if response.contains("```") {
            response
                .split("```")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(response)
                .trim()
        } else {
            response.trim()
        };

        let json: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| anyhow!("Failed to parse strategy JSON: {}", e))?;

        // 解析 required_info
        let required_info: Vec<InfoType> = json.get("required_info")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| self.parse_info_type(s))
                    .collect()
            })
            .unwrap_or_else(|| vec![InfoType::TargetStructure]);

        // 解析步骤
        let steps: Vec<ObserveStep> = json.get("steps")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|step| self.parse_observe_step(step, target))
                    .collect()
            })
            .unwrap_or_default();

        Ok(ObserveStrategy {
            name: json.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string(),
            required_info,
            suggested_tools: Vec::new(),
            collection_order: steps,
            success_criteria: json.get("success_criteria")
                .and_then(|v| v.as_str())
                .unwrap_or("Collect all required information")
                .to_string(),
        })
    }

    /// 解析信息类型
    fn parse_info_type(&self, s: &str) -> Option<InfoType> {
        match s.to_lowercase().as_str() {
            "targetstructure" | "target_structure" => Some(InfoType::TargetStructure),
            "apiendpoints" | "api_endpoints" => Some(InfoType::ApiEndpoints),
            "formsandinputs" | "forms_and_inputs" | "forms" => Some(InfoType::FormsAndInputs),
            "techstack" | "tech_stack" => Some(InfoType::TechStack),
            "authentication" | "auth" => Some(InfoType::Authentication),
            "errormessages" | "error_messages" | "errors" => Some(InfoType::ErrorMessages),
            "configuration" | "config" => Some(InfoType::Configuration),
            _ => Some(InfoType::Custom(s.to_string())),
        }
    }

    /// 解析观察步骤
    fn parse_observe_step(&self, step: &serde_json::Value, target: &str) -> Option<ObserveStep> {
        let id = step.get("id")?.as_str()?.to_string();
        let tool = step.get("tool")?.as_str()?.to_string();
        let objective = step.get("objective")
            .and_then(|v| v.as_str())
            .unwrap_or("Collect information")
            .to_string();

        let mut args_template: HashMap<String, serde_json::Value> = step.get("args")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        // 如果没有指定 target/url/domain，自动添加
        if !args_template.contains_key("target") 
            && !args_template.contains_key("url") 
            && !args_template.contains_key("domain") 
        {
            // 根据工具类型添加适当的参数
            if tool.contains("http") || tool.contains("request") {
                args_template.insert("url".to_string(), serde_json::json!(target));
            } else if tool.contains("analyze") {
                let domain = target
                    .trim_start_matches("http://")
                    .trim_start_matches("https://")
                    .split('/')
                    .next()
                    .unwrap_or(target)
                    .split(':')
                    .next()
                    .unwrap_or(target);
                args_template.insert("domain".to_string(), serde_json::json!(domain));
            } else {
                args_template.insert("target".to_string(), serde_json::json!(target));
            }
        }

        let depends_on: Vec<String> = step.get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let optional = step.get("optional")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Some(ObserveStep {
            id,
            objective,
            tool,
            args_template,
            depends_on,
            optional,
        })
    }

    /// 执行观察步骤
    async fn execute_observe_step(
        &self,
        step: &ObserveStep,
        gathered_info: &HashMap<String, serde_json::Value>,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<(serde_json::Value, Vec<InfoType>)> {
        log::info!("AutonomousObserver: Executing step {} - {}", step.id, step.tool);

        // 解析参数中的变量引用
        let resolved_args = self.resolve_step_arguments(&step.args_template, gathered_info);

        // 执行工具
        let result = self.execute_tool(&step.tool, resolved_args, context).await?;

        // 分析结果，确定收集到了哪些信息类型
        let info_types = self.analyze_result_info_types(&step.tool, &result);

        Ok((result, info_types))
    }

    /// 解析步骤参数
    fn resolve_step_arguments(
        &self,
        template: &HashMap<String, serde_json::Value>,
        gathered_info: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut resolved = HashMap::new();

        for (key, value) in template {
            let resolved_value = if let serde_json::Value::String(s) = value {
                if s.starts_with('$') {
                    // 解析变量引用
                    let var_name = s.trim_start_matches('$');
                    if let Some(parts) = var_name.split_once('.') {
                        if let Some(base) = gathered_info.get(parts.0) {
                            base.get(parts.1).cloned().unwrap_or(value.clone())
                        } else {
                            value.clone()
                        }
                    } else {
                        gathered_info.get(var_name).cloned().unwrap_or(value.clone())
                    }
                } else {
                    value.clone()
                }
            } else {
                value.clone()
            };
            resolved.insert(key.clone(), resolved_value);
        }

        resolved
    }

    /// 执行工具
    async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: HashMap<String, serde_json::Value>,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let unified_call = UnifiedToolCall {
            id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool_name.to_string(),
            parameters: arguments,
            timeout: Some(Duration::from_secs(60)),
            context: HashMap::new(),
            retry_count: 0,
        };

        if let Some(adapter) = &self.tool_adapter {
            let result = adapter.execute_tool(unified_call).await?;
            Ok(result.output)
        } else {
            let engine_adapter = crate::tools::get_global_engine_adapter()
                .map_err(|e| anyhow!("No tool adapter: {}", e))?;
            let result = engine_adapter.execute_tool(unified_call).await?;
            Ok(result.output)
        }
    }

    /// 分析结果的信息类型
    fn analyze_result_info_types(&self, tool_name: &str, result: &serde_json::Value) -> Vec<InfoType> {
        let mut types = Vec::new();

        // 根据工具名和结果内容判断
        let tool_lower = tool_name.to_lowercase();

        if tool_lower.contains("analyze") || tool_lower.contains("crawl") {
            types.push(InfoType::TargetStructure);
            
            // 检查是否包含 API 信息
            if result.get("api_endpoints").is_some() || result.get("endpoints").is_some() {
                types.push(InfoType::ApiEndpoints);
            }
            
            // 检查是否包含技术栈信息
            if result.get("tech_stack").is_some() || result.get("technologies").is_some() {
                types.push(InfoType::TechStack);
            }
        }

        if tool_lower.contains("http") || tool_lower.contains("request") {
            types.push(InfoType::TargetStructure);
            
            // 检查响应中是否有表单
            if let Some(body) = result.get("body").and_then(|v| v.as_str()) {
                if body.contains("<form") || body.contains("<input") {
                    types.push(InfoType::FormsAndInputs);
                }
            }
        }

        if tool_lower.contains("form") || tool_lower.contains("input") {
            types.push(InfoType::FormsAndInputs);
        }

        if tool_lower.contains("tech") || tool_lower.contains("fingerprint") {
            types.push(InfoType::TechStack);
        }

        if tool_lower.contains("auth") || tool_lower.contains("login") {
            types.push(InfoType::Authentication);
        }

        // 如果没有识别到具体类型，至少标记为收集了一些信息
        if types.is_empty() {
            types.push(InfoType::TargetStructure);
        }

        types
    }

    /// 计算充分性得分
    fn calculate_sufficiency(&self, collected: &HashSet<InfoType>, required: &[InfoType]) -> f32 {
        if required.is_empty() {
            return 1.0;
        }

        let matched = required.iter().filter(|r| collected.contains(r)).count();
        matched as f32 / required.len() as f32
    }

    fn calculate_sufficiency_from_vec(&self, collected: &[InfoType], required: &[InfoType]) -> f32 {
        if required.is_empty() {
            return 1.0;
        }

        let matched = required.iter().filter(|r| collected.contains(r)).count();
        matched as f32 / required.len() as f32
    }

    /// 自适应额外收集
    async fn adaptive_collection(
        &self,
        llm_client: &LlmClient,
        task_description: &str,
        target: &str,
        gathered_info: &HashMap<String, serde_json::Value>,
        collected_types: &HashSet<InfoType>,
        required_types: &[InfoType],
        context: &HashMap<String, serde_json::Value>,
        remaining_calls: u32,
    ) -> Result<(HashMap<String, serde_json::Value>, Vec<InfoType>, Vec<ObserveToolCall>)> {
        log::info!("AutonomousObserver: Adaptive collection with {} remaining calls", remaining_calls);

        // 确定缺失的信息
        let missing: Vec<&InfoType> = required_types.iter()
            .filter(|t| !collected_types.contains(t))
            .collect();

        if missing.is_empty() {
            return Ok((HashMap::new(), Vec::new(), Vec::new()));
        }

        // 让 LLM 决定如何补充收集
        let available_tools = self.get_available_tools(context).await;

        let system_prompt = r#"你是一个信息收集专家。根据已有信息和缺失信息，决定下一步收集什么。

输出 JSON 格式:
```json
{
  "tool": "工具名",
  "args": {"参数": "值"},
  "reason": "选择理由"
}
```

只输出一个工具调用。如果无法收集更多信息，输出 {"done": true}。"#;

        let user_prompt = format!(
            r#"任务: {}
目标: {}

已收集的信息类型: {:?}
缺失的信息类型: {:?}

已收集的数据摘要:
{}

可用工具:
{}

剩余调用次数: {}

请决定下一步:"#,
            task_description,
            target,
            collected_types.iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>(),
            missing.iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>(),
            gathered_info.keys().take(10).cloned().collect::<Vec<_>>().join(", "),
            available_tools,
            remaining_calls,
        );

        let response = llm_client
            .completion(Some(system_prompt), &user_prompt)
            .await?;

        // 解析响应
        let json_str = if response.contains("```json") {
            response
                .split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(&response)
                .trim()
        } else {
            response.trim()
        };

        let json: serde_json::Value = serde_json::from_str(json_str)
            .unwrap_or(serde_json::json!({"done": true}));

        // 检查是否完成
        if json.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
            return Ok((HashMap::new(), Vec::new(), Vec::new()));
        }

        // 执行建议的工具
        let tool_name = json.get("tool")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No tool specified"))?;

        let args: HashMap<String, serde_json::Value> = json.get("args")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        let result = self.execute_tool(tool_name, args.clone(), context).await?;
        let info_types = self.analyze_result_info_types(tool_name, &result);

        let mut new_info = HashMap::new();
        new_info.insert(format!("{}_result", tool_name), result.clone());

        let tool_call = ObserveToolCall {
            tool_name: tool_name.to_string(),
            arguments: args,
            result: Some(result),
            success: true,
            duration_ms: 0,
            info_types_gathered: info_types.clone(),
        };

        Ok((new_info, info_types, vec![tool_call]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_info_type() {
        // 测试需要 mock，暂时跳过
    }
}


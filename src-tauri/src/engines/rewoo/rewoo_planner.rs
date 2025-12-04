//! ReWOO Planner 实现
//! 
//! 基于 LangGraph ReWOO 标准实现的 Planner 模块
//! 负责生成标准格式的执行计划

use super::*;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use crate::engines::LlmClient;
use crate::services::prompt_db::PromptRepository;
use crate::services::ai::AiServiceManager;
use anyhow::{Result, anyhow};
use tracing::{info, warn, error};

const DEFAULT_PLANNER_PROMPT: &str = r##"You are a ReWOO (Reasoning without Observation) planning assistant. Your task is to create a detailed execution plan for the given query using available tools.

**Task:** {query}

**Available Tools:** {tools}

**Context:** {context}

OUTPUT FORMAT (MANDATORY):
- You MUST respond with a single JSON object, no markdown fences, no explanations.
- The JSON MUST follow this exact structure:
{
  "plan_summary": "string (brief reasoning/strategy)",
  "steps": [
    {
      "id": "E1",                    // step id without '#'
      "tool": "tool_name",           // MUST be in available tools
      "args": { ... },               // JSON arguments
      "depends_on": ["E<k>"],        // optional dependencies
      "description": "string"        // optional short description
    }
  ]
}

Rules:
1) Use sequential ids: E1, E2, E3...
2) When referencing a previous step result in args, use the string "#E<k>" (e.g., "#E1")
3) Ensure args are valid JSON for the tool (no comments, no trailing commas)
4) Include resource cleanup steps when required (e.g., playwright_close, stop_passive_scan)

Now generate ONLY the JSON object for the execution plan:"##;

/// ReWOO Planner - 负责生成执行计划
#[derive(Debug)]
pub struct ReWOOPlanner {
    /// AI服务管理器
    ai_service_manager: Arc<AiServiceManager>,
    /// Prompt仓库
    prompt_repo: Arc<PromptRepository>,
    /// 配置
    config: PlannerConfig,
    /// 框架适配器（用于获取工具详细信息）
    framework_adapter: Arc<dyn crate::tools::FrameworkToolAdapter>,
    /// 运行时参数（包含工具权限等）
    runtime_params: Option<HashMap<String, serde_json::Value>>,
}

impl ReWOOPlanner {
    /// 创建新的 ReWOO Planner
    pub fn new(
        ai_service_manager: Arc<AiServiceManager>,
        prompt_repo: Arc<PromptRepository>,
        config: PlannerConfig,
        framework_adapter: Arc<dyn crate::tools::FrameworkToolAdapter>,
    ) -> Result<Self> {
        Ok(Self {
            ai_service_manager,
            prompt_repo,
            config,
            framework_adapter,
            runtime_params: None,
        })
    }
    
    /// 设置运行时参数
    pub fn set_runtime_params(&mut self, params: HashMap<String, serde_json::Value>) {
        self.runtime_params = Some(params);
    }

    /// 生成执行计划
    pub async fn plan(
        &self,
        query: &str,
        available_tools: &[String],
        context: Option<&str>,
        execution_id: &str,
    ) -> Result<ReWOOPlan> {
        info!("ReWOO Planner: Generating plan for query: {}", query);
        
        // 构建prompt（返回system prompt和user prompt）
        let (system_prompt, user_prompt) = self.build_planning_prompt(query, available_tools, context).await?;
        
        // 调用LLM生成计划
        let plan_string = self.call_llm(&system_prompt, &user_prompt, execution_id).await?;
        
        // 解析计划
        let steps = self.parse_plan(&plan_string)?;
        
        Ok(ReWOOPlan {
            steps,
            reasoning: plan_string.clone(),
            execution_id: execution_id.to_string(),
            created_at: SystemTime::now(),
        })
    }
    
    /// 构建规划prompt（返回system prompt和user prompt）
    async fn build_planning_prompt(
        &self,
        query: &str,
        available_tools: &[String],
        context: Option<&str>,
    ) -> Result<(String, String)> {
        use crate::models::prompt::{ArchitectureType, StageType};
        
        // 优先使用 runtime_params 中的自定义 system prompt（来自 Orchestrator）
        let system_template = if let Some(params) = &self.runtime_params {
            if let Some(custom_prompt) = params.get("custom_system_prompt").and_then(|v| v.as_str()) {
                // info!("ReWOO Planner: Using custom system prompt from runtime_params");
                custom_prompt.to_string()
            } else {
                // 从数据库获取ReWOO planner模板作为system prompt
                if let Ok(Some(template)) = self.prompt_repo
                    .get_template_by_arch_stage(ArchitectureType::ReWOO, StageType::Planner)
                    .await
                {
                    // info!("ReWOO Planner: Using prompt from database");
                    template.content
                } else {
                    // Fallback到默认模板
                    warn!("ReWOO planner template not found in database, using default template");
                    include_str!("../prompt_md/rewoo_prompts.md")
                        .split("## rewoo_planner")
                        .nth(1)
                        .and_then(|s| s.split("---").next())
                        .unwrap_or(DEFAULT_PLANNER_PROMPT)
                        .to_string()
                }
            }
        } else {
            // 从数据库获取ReWOO planner模板作为system prompt
            if let Ok(Some(template)) = self.prompt_repo
                .get_template_by_arch_stage(ArchitectureType::ReWOO, StageType::Planner)
                .await
            {
                info!("ReWOO Planner: Using prompt from database");
                template.content
            } else {
                // Fallback到默认模板
                warn!("ReWOO planner template not found in database, using default template");
                include_str!("../prompt_md/rewoo_prompts.md")
                    .split("## rewoo_planner")
                    .nth(1)
                    .and_then(|s| s.split("---").next())
                    .unwrap_or(DEFAULT_PLANNER_PROMPT)
                    .to_string()
            }
        };
        
        // 构建详细的工具列表描述（参考ReAct的实现）
        let tools_desc = self.build_tools_description(available_tools).await;
        
        // 填充system prompt中的占位符
        let mut system_prompt = system_template.replace("{tools}", &tools_desc);
        
        if let Some(ctx) = context {
            system_prompt = system_prompt.replace("{context}", ctx);
        } else {
            system_prompt = system_prompt.replace("{context}", "No additional context provided.");
        }
        
        // 不要在 system prompt 中替换 {task}，保持模板原样
        // user prompt 才是用户的真实输入
        let user_part = query.to_string();
        
        Ok((system_prompt, user_part))
    }
    
    /// 调用LLM生成计划
    async fn call_llm(&self, system_prompt: &str, user_prompt: &str, execution_id: &str) -> Result<String> {
        // 从调度器配置获取规划器模型（Planning阶段）
        let ai_service = match self.ai_service_manager
            .get_service_for_stage(crate::services::ai::SchedulerStage::Planning)
            .await 
        {
            Ok(Some(service)) => {
                info!("ReWOO Planner: Using scheduler config for Planning stage");
                service
            }
            Ok(None) | Err(_) => {
                // 回退到默认服务
                warn!("ReWOO Planner: Scheduler config not available, using fallback model: {}", self.config.model_name);
                let provider = &self.config.model_name;
                self.ai_service_manager
                    .get_service(provider)
                    .ok_or_else(|| anyhow!("AI service '{}' not found", provider))?
            }
        };
        
        // 使用公共 llm_client 模块
        let llm_client = crate::engines::create_client(&ai_service);
        let config = llm_client.config();
        info!("ReWOO Planner: Using provider={}, model={}, execution_id={}", config.provider, config.model, execution_id);
        
        // 调用 LlmClient
        let content = llm_client
            .completion(Some(system_prompt), user_prompt)
            .await
            .map_err(|e| {
                error!("ReWOO Planner: LLM call failed: {}", e);
                anyhow!("LLM call failed: {}", e)
            })?;
        
        if content.is_empty() {
            return Err(anyhow!("LLM returned empty response"));
        }
        
        info!("ReWOO Planner: Generated plan with {} chars", content.len());
        Ok(content)
    }
    
    /// 解析计划字符串为步骤列表（支持JSON计划，向后兼容旧格式）
    fn parse_plan(&self, plan_string: &str) -> Result<Vec<ReWOOStep>> {
        // 优先尝试解析为JSON计划
        if let Ok(v) = serde_json::from_str::<Value>(plan_string) {
            if let Some(json_steps) = v.get("steps").and_then(|s| s.as_array()) {
                let mut steps: Vec<ReWOOStep> = Vec::new();
                for (idx, s) in json_steps.iter().enumerate() {
                    let id = s.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let tool = s.get("tool").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let args = s.get("args").cloned().unwrap_or(Value::Object(Default::default()));
                    let desc = s.get("description").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    // 兼容 depends_on: ["E1","E2"] 与空
                    let deps: Vec<String> = s.get("depends_on")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| {
                                    let clean = s.trim_start_matches('#');
                                    format!("#{}", clean)
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    
                    // 验证关键字段
                    if tool.is_empty() {
                        return Err(anyhow!(format!("Invalid JSON plan: step {} missing tool", idx+1)));
                    }
                    let step_id = if id.is_empty() {
                        format!("#E{}", idx + 1)
                    } else {
                        format!("#{}", id.trim_start_matches('#'))
                    };
                    
                    // 解析args为 HashMap
                    let mut tool_args = HashMap::new();
                    if let Some(obj) = args.as_object() {
                        for (k, v) in obj {
                            tool_args.insert(k.clone(), v.clone());
                        }
                    }
                    
                    steps.push(ReWOOStep {
                        step_id,
                        tool_name: tool,
                        tool_args,
                        dependencies: deps,
                        description: desc,
                    });
                }
                
                if steps.is_empty() {
                    return Err(anyhow!("JSON plan contains no steps"));
                }
                return Ok(steps);
            }
        }
        
        // 向后兼容：旧的文本格式
        let mut steps = Vec::new();
        let re = Regex::new(r"#E(\d+)\s*=\s*(\w+)\[([^\]]*)\]")?;
        for cap in re.captures_iter(plan_string) {
            let step_id = format!("#E{}", &cap[1]);
            let tool_name = cap[2].to_string();
            let args_str = cap[3].to_string();
            let tool_args = self.parse_tool_args(&args_str)?;
            let dep_re = Regex::new(r"#E(\d+)")?;
            let dependencies: Vec<String> = dep_re
                .captures_iter(&args_str)
                .map(|c| format!("#E{}", &c[1]))
                .collect();
            let description = self.extract_step_description(plan_string, &step_id);
            steps.push(ReWOOStep {
                step_id,
                tool_name,
                tool_args,
                dependencies,
                description,
            });
        }
        if steps.is_empty() {
            return Err(anyhow!("Failed to parse any steps from plan (neither JSON nor legacy format)"));
        }
        Ok(steps)
    }
    
    /// 解析工具参数
    fn parse_tool_args(&self, args_str: &str) -> Result<HashMap<String, serde_json::Value>> {
        let mut args = HashMap::new();
        
        // 尝试解析为JSON
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(args_str) {
            if let Some(obj) = json_value.as_object() {
                for (key, value) in obj {
                    args.insert(key.clone(), value.clone());
                }
                return Ok(args);
            }
        }
        
        // 回退到简单的key=value解析
        for part in args_str.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once('=') {
                args.insert(
                    key.trim().to_string(),
                    serde_json::Value::String(value.trim().to_string()),
                );
            }
        }
        
        Ok(args)
    }
    
    /// 提取步骤描述
    fn extract_step_description(&self, plan_string: &str, step_id: &str) -> String {
        // 查找Plan: 和步骤ID之间的内容
        if let Some(plan_start) = plan_string.find("Plan:") {
            if let Some(step_pos) = plan_string.find(step_id) {
                let desc = &plan_string[plan_start + 5..step_pos];
                return desc.trim().to_string();
            }
        }
        String::new()
    }
    
    /// 构建详细的工具描述信息（参考ReAct的实现）
    async fn build_tools_description(&self, tool_names: &[String]) -> String {
        use crate::tools::ToolInfo;
        use std::collections::{HashMap, HashSet};
        
        // 从runtime_params读取工具白名单/黑名单
        let (allow, allow_present, deny): (HashSet<String>, bool, HashSet<String>) =
            if let Some(params) = &self.runtime_params {
                log::debug!("ReWOO Planner: task_parameters = {:?}", params);
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
                log::warn!("ReWOO Planner: runtime_params is None!");
                (HashSet::new(), false, HashSet::new())
            };
        
        // 如果显式传入空白名单，禁用所有工具
        if allow_present && allow.is_empty() {
            log::info!("ReWOO Planner: 检测到显式空白名单 => 禁用所有工具");
            return "No tools available".to_string();
        }
        
        log::debug!(
            "ReWOO Planner: 工具过滤配置 - 白名单: {:?}, 黑名单: {:?}",
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
        
        // 从框架适配器获取工具详细信息
        for tool_name in tool_names {
            // 应用白名单/黑名单过滤
            let mut whitelist_hit = allow.contains(tool_name);
            let plugin_prefixed_candidate = format!("plugin::{}", tool_name);
            let prefixed_whitelist_hit = allow.contains(&plugin_prefixed_candidate);
            
            // 如果有白名单且工具不在白名单中，跳过
            if !allow.is_empty() {
                whitelist_hit = whitelist_hit || prefixed_whitelist_hit;
                if !whitelist_hit {
                    log::debug!(
                        "ReWOO Planner: 工具 '{}' 未命中白名单, allow={:?}",
                        tool_name, allow
                    );
                    continue;
                }
            }
            
            // 如果工具在黑名单中，跳过
            if deny.contains(tool_name) {
                log::debug!(
                    "ReWOO Planner: 工具 '{}' 在黑名单中，跳过 (deny={:?})",
                    tool_name, deny
                );
                continue;
            }
            
            // 获取工具详细信息
            if let Some(tool_info) = self.framework_adapter.get_tool_info(tool_name).await {
                // 处理前缀补偿（与ReAct保持一致）
                if prefixed_whitelist_hit
                    && !tool_info.name.starts_with("plugin::")
                    && tool_info.metadata.tags.iter().any(|t| t == "passive")
                {
                    log::debug!(
                        "ReWOO Planner: 跳过对被动工具 '{}' 的前缀补偿",
                        tool_info.name
                    );
                    continue;
                }
                
                let effective_name = if !tool_info.name.starts_with("plugin::") && prefixed_whitelist_hit {
                    plugin_prefixed_candidate.clone()
                } else {
                    tool_info.name.clone()
                };
                
                log::debug!(
                    "ReWOO Planner: 接收工具 '{}' => effective='{}' (available={}, source={:?})",
                    tool_info.name, effective_name, tool_info.available, tool_info.source
                );
                
                let mut adjusted = tool_info;
                if effective_name != adjusted.name {
                    adjusted.name = effective_name;
                }
                all_tools.push(adjusted);
            } else {
                log::warn!(
                    "ReWOO Planner: 工具 '{}' 在列表中但无法获取详细信息",
                    tool_name
                );
            }
        }
        
        // 去重工具（按名称）
        let mut unique_tools: HashMap<String, ToolInfo> = HashMap::new();
        for tool in all_tools {
            let existed = unique_tools.contains_key(&tool.name);
            if existed {
                log::debug!("ReWOO Planner: 去重丢弃重复工具 '{}'", tool.name);
            }
            unique_tools.entry(tool.name.clone()).or_insert(tool);
        }
        
        let tool_infos: Vec<&ToolInfo> = unique_tools.values().collect();
        
        if tool_infos.is_empty() {
            log::warn!("ReWOO Planner: 没有找到任何可用工具");
            return "No tools available".to_string();
        }
        
        log::info!(
            "ReWOO Planner: 构建工具信息，共 {} 个工具",
            tool_infos.len()
        );
        
        // 构建工具描述字符串
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
}

/// ReWOO 执行计划
#[derive(Debug, Clone)]
pub struct ReWOOPlan {
    pub steps: Vec<ReWOOStep>,
    pub reasoning: String,
    pub execution_id: String,
    pub created_at: SystemTime,
}

/// ReWOO 执行步骤
#[derive(Debug, Clone)]
pub struct ReWOOStep {
    pub step_id: String,
    pub tool_name: String,
    pub tool_args: HashMap<String, serde_json::Value>,
    pub dependencies: Vec<String>,
    pub description: String,
}
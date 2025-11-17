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
use crate::services::prompt_db::PromptRepository;
use crate::services::ai::AiServiceManager;
use anyhow::{Result, anyhow};
use tracing::{info, warn};

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
}

impl ReWOOPlanner {
    /// 创建新的 ReWOO Planner
    pub fn new(
        ai_service_manager: Arc<AiServiceManager>,
        prompt_repo: Arc<PromptRepository>,
        config: PlannerConfig,
    ) -> Result<Self> {
        Ok(Self {
            ai_service_manager,
            prompt_repo,
            config,
        })
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
        
        // 从数据库获取ReWOO planner模板作为system prompt
        let system_template = if let Ok(Some(template)) = self.prompt_repo
            .get_template_by_arch_stage(ArchitectureType::ReWOO, StageType::Planner)
            .await
        {
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
        };
        
        // 构建工具列表描述
        let tools_desc = available_tools.join(", ");
        
        // 填充system prompt中的占位符
        let mut system_prompt = system_template.replace("{tools}", &tools_desc);
        
        if let Some(ctx) = context {
            system_prompt = system_prompt.replace("{context}", ctx);
        } else {
            system_prompt = system_prompt.replace("{context}", "No additional context provided.");
        }
        
        // 替换{task}占位符为用户输入
        let system_part = system_prompt.replace("{task}", query);
        
        // user prompt就是用户的输入
        let user_part = query.to_string();
        
        Ok((system_part, user_part))
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
        
        let config = ai_service.get_config();
        info!("ReWOO Planner: Using provider={}, model={}, execution_id={}", config.provider, config.model, execution_id);
        
        // 调用 AiService，不保存到数据库（conversation_id=None）
        let content = ai_service
            .send_message_stream(
                Some(user_prompt),
                Some(system_prompt),
                None, // 不关联会话
                Some(execution_id.to_string()),
                false, // 不流式发送到前端
                false, // 不是最终消息
                None,
            )
            .await?;
        
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
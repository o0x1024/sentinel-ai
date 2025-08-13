//! ReWOO Planner 实现
//! 
//! 基于 LangGraph ReWOO 标准实现的 Planner 模块
//! 负责生成标准格式的执行计划：Plan: <reasoning> #E1 = Tool[args]
use super::*;
use super::rewoo_tool_adapter::ToolManager;
use crate::ai_adapter::types::{ChatRequest, Message, MessageContent, MessageRole, Tool};
use crate::ai_adapter::{AiProvider, ChatOptions};
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use crate::models::prompt::{ArchitectureType, StageType};
use crate::services::prompt_db::PromptRepository;

/// ReWOO Planner - 负责生成执行计划
pub struct ReWOOPlanner {
    /// AI 提供商
    ai_provider: Arc<dyn AiProvider>,
    tool_manager: Arc<dyn ToolManager>,
    /// 配置
    config: PlannerConfig,
    /// 计划解析正则表达式
    plan_regex: Regex,
    /// 动态Prompt仓库
    prompt_repo: Option<PromptRepository>,
}

impl ReWOOPlanner {
    /// 创建新的 Planner
    pub fn new(
        ai_provider: Arc<dyn AiProvider>,
        tool_manager: Arc<dyn ToolManager>,
        config: PlannerConfig,
        prompt_repo: Option<PromptRepository>,
    ) -> Result<Self, ReWOOError> {
        // 用于解析计划步骤的正则表达式
        // 匹配格式：#E1 = Tool[args]
        let plan_regex = Regex::new(r"#E(\d+)\s*=\s*(\w+)\[([^\]]+)\]")
            .map_err(|e| ReWOOError::ConfigurationError(format!("Invalid regex: {}", e)))?;
        
        Ok(Self {
            ai_provider,
            tool_manager,
            config,
            plan_regex,
            prompt_repo,
        })
    }
    
    /// 将ToolManager提供的可用工具名称转换为LLM函数调用的Tool定义
    fn build_tools_from_manager(&self) -> Vec<Tool> {
        let names = self.tool_manager.get_available_tools();
        let mut tools = Vec::new();
        for name in names {
            // 从管理器获取参数定义，转换为 JSON Schema
            let schema = if let Some(params) = self.tool_manager.get_tool_parameters(&name) {
                // 将 ParameterDefinition 列表转为 OpenAI function schema
                let mut properties = serde_json::Map::new();
                let mut required: Vec<String> = Vec::new();
                for p in params.parameters {
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
                    if let Some(defv) = p.default_value {
                        if let Some(obj) = prop.as_object_mut() {
                            obj.insert("default".to_string(), defv);
                        }
                    }
                    properties.insert(p.name.clone(), prop);
                    if p.required {
                        required.push(p.name);
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
                name: name.clone(),
                description: format!("Tool {}", name),
                parameters: schema,
            });
        }
        tools
    }
    /// 生成计划
    pub async fn plan(&self, state: &mut ReWOOState) -> Result<(), ReWOOError> {
        let start_time = SystemTime::now();
        
        // 构建计划生成提示
        let prompt = self.build_planning_prompt(&state.task).await?;
        
        // 调用 AI 生成计划
        let plan_string = self.generate_plan_string(&prompt).await?;
        
        // 解析计划步骤
        let steps = self.parse_plan_steps(&plan_string)?;
        
        // 更新状态
        state.plan_string = plan_string;
        state.steps = steps;
        
        Ok(())
    }
    
    /// 构建计划生成提示
    async fn build_planning_prompt(&self, task: &str) -> Result<String, ReWOOError> {
        // 动态获取可用工具名称
        let tool_names = self.tool_manager.get_available_tools();
        // 生成带参数签名的工具清单，形如：
        // - rsubdomain(domain: string, use_database_wordlist?: boolean)
        let mut tool_lines: Vec<String> = Vec::new();
        for name in &tool_names {
            if let Some(params) = self.tool_manager.get_tool_parameters(name) {
                let mut parts: Vec<String> = Vec::new();
                for p in params.parameters {
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
        let tools_block = if tool_lines.is_empty() {
            "(no tools available)".to_string()
        } else {
            tool_lines.join("\n")
        };

        let base = format!(r#"
 You are a planner for the ReWOO (Reasoning without Observation) framework.
 Your job is to create a step-by-step plan to solve the given task.
 
 IMPORTANT FORMATTING RULES:
 1. Each step must follow this exact format: Plan: <reasoning> #E<number> = Tool[key: value]
 2. Use #E1, #E2, #E3, etc. for step variables
 3. You can reference previous steps using their variables (e.g., #E1, #E2)
 4. Available tools (with parameters):
{tools}
 
 Example format:
 Plan: I need to search for information about the target
 #E1 = Search[target information]
 Plan: Based on the search results, I'll scan for open ports
 #E2 = PortScanner[target from #E1]
 Plan: Now I'll check for vulnerabilities on the discovered ports
 #E3 = VulnerabilityScanner[ports from #E2]
 
  Task: {task}
 
Guidelines:
- do not reponse thinking process, just generate the plan

  "#, tools = tools_block, task = task);

        if let Some(repo) = &self.prompt_repo {
            match repo.get_active_prompt(ArchitectureType::ReWOO, StageType::Planner).await {
                Ok(Some(dynamic)) => {
                    let replaced = Self::apply_placeholders(&dynamic, vec![
                        ("{{TOOLS}}", &tools_block),
                        ("{tools}", &tools_block),
                        ("{{TASK}}", task),
                        ("{task}", task),
                    ]);
                    Ok(replaced)
                },
                _ => Ok(base),
            }
        } else {
            Ok(base)
        }
    }

    fn apply_placeholders(template: &str, pairs: Vec<(&str, &str)>) -> String {
        let mut out = template.to_string();
        for (k, v) in pairs {
            out = out.replace(k, v);
        }
        out
    }
    
    /// 生成计划字符串
    async fn generate_plan_string(&self, prompt: &str) -> Result<String, ReWOOError> {
        let request = ChatRequest {
            model: self.config.model_name.clone(),
            messages: vec![Message {
                role: MessageRole::User,
                content: prompt.to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: None,
            tool_choice: None,
            user: None,
            extra_params: None,
            options: Some(ChatOptions {
                temperature: Some(self.config.temperature),
                max_tokens: Some(self.config.max_tokens),
                ..Default::default()
            }),
        };
        
        let response = self.ai_provider.send_chat_request(&request).await
            .map_err(|e| ReWOOError::AiProviderError(format!("Failed to generate plan: {}", e)))?;
        
        if let Some(choice) = response.choices.first() {
            return Ok(choice.message.content.clone());
        }
        
        Err(ReWOOError::PlanningError("No valid response from AI provider".to_string()))
    }
    
    /// 解析计划步骤
    fn parse_plan_steps(&self, plan_string: &str) -> Result<Vec<String>, ReWOOError> {
        let mut steps = Vec::new();
        
        // 按行分割并查找步骤
        for line in plan_string.lines() {
            let line = line.trim();
            if line.starts_with("#E") {
                // 验证步骤格式
                if self.plan_regex.is_match(line) {
                    steps.push(line.to_string());
                } else {
                    return Err(ReWOOError::PlanParsingError(
                        format!("Invalid step format: {}", line)
                    ));
                }
            }
        }
        
        if steps.is_empty() {
            return Err(ReWOOError::PlanParsingError(
                "No valid steps found in plan".to_string()
            ));
        }
        
        if steps.len() > self.config.max_steps as usize {
            return Err(ReWOOError::PlanParsingError(
                format!("Too many steps: {} (max: {})", steps.len(), self.config.max_steps)
            ));
        }
        
        Ok(steps)
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
    
    /// 提取步骤的推理部分
    fn extract_reasoning_for_step(&self, _step: &str) -> String {
        // 简化实现：返回默认推理
        // 在实际实现中，应该从完整的计划字符串中提取对应的 Plan: 部分
        "Executing planned step".to_string()
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
    
    /// 替换步骤中的变量引用
    pub fn substitute_variables(&self, args: &str, results: &HashMap<String, String>) -> String {
        let mut substituted = args.to_string();
        
        // 查找并替换所有变量引用 (#E1, #E2, etc.)
        let var_regex = Regex::new(r"#E(\d+)").unwrap();
        
        for captures in var_regex.captures_iter(args) {
            let full_match = captures.get(0).unwrap().as_str();
            if let Some(value) = results.get(full_match) {
                substituted = substituted.replace(full_match, value);
            }
        }
        
        substituted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[test]
    fn test_parse_step() {
        let config = PlannerConfig {
            model_name: "test".to_string(),
            temperature: 0.0,
            max_tokens: 1000,
            max_steps: 10,
        };
        
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
        let config = PlannerConfig {
            model_name: "test".to_string(),
            temperature: 0.0,
            max_tokens: 1000,
            max_steps: 10,
        };
        
        // 创建一个模拟的 AI 提供商
        // let planner = ReWOOPlanner::new(ai_provider, config).unwrap();
        
        let mut results = HashMap::new();
        results.insert("#E1".to_string(), "192.168.1.1".to_string());
        
        // let substituted = planner.substitute_variables("scan #E1 for ports", &results);
        // assert_eq!(substituted, "scan 192.168.1.1 for ports");
    }
}

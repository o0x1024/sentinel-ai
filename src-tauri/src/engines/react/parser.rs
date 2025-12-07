//! Action 指令解析器
//!
//! 支持两种解析策略：
//! 1. 强 JSON：严格校验 JSON Schema
//! 2. 自然语言兜底：正则提取 Action/Action Input

use super::types::{
    ActionInstruction, ParsedAction, ReactToolCall, FinalAnswer,
    ParallelToolCall, AggregationStrategy, ReasoningStep, PhaseDefinition, PhaseType,
    SubPlanStep,
};
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json;
use uuid::Uuid;

pub struct ActionParser;

impl ActionParser {
    /// 解析 LLM 输出为 ActionInstruction（优先 JSON）- 扩展版本
    pub fn parse(output: &str) -> Result<ActionInstruction> {
        // 先尝试直接解析整个输出为 JSON
        if let Ok(instruction) = Self::parse_json(output) {
            return Ok(instruction);
        }

        // 尝试提取 ```json 代码块
        let json_content = Self::extract_json_block(output).unwrap_or_else(|| output.to_string());
        
        // 尝试解析为计划 JSON 格式（goal + steps）
        if let Ok(plan_instruction) = Self::parse_plan_json(&json_content) {
            return Ok(plan_instruction);
        }

        // 尝试解析扩展格式
        
        // 检查并行执行格式
        if output.contains("Parallel:") || output.to_lowercase().contains("parallel_tools") {
            if let Ok(parallel) = Self::parse_parallel_format(output) {
                return Ok(parallel);
            }
        }
        
        // 检查推理链格式
        if (output.contains("Plan:") || output.contains("plan:")) && output.contains("#E") {
            if let Ok(chain) = Self::parse_reasoning_chain_format(output) {
                return Ok(chain);
            }
        }
        
        // 检查阶段执行格式
        if output.contains("Phase:") || output.contains("phase:") {
            if let Ok(phase) = Self::parse_phase_format(output) {
                return Ok(phase);
            }
        }
        
        // 检查子计划格式
        if output.contains("SubPlan:") || output.contains("sub_plan:") {
            if let Ok(sub_plan) = Self::parse_sub_plan_format(output) {
                return Ok(sub_plan);
            }
        }

        // 兜底：自然语言解析
        Self::parse_natural_language(output)
    }

    /// 纯 JSON 解析
    fn parse_json(text: &str) -> Result<ActionInstruction> {
        serde_json::from_str(text)
            .context("Failed to parse as JSON ActionInstruction")
    }

    /// 提取 Markdown 代码块中的 JSON
    fn extract_json_block(text: &str) -> Option<String> {
        // 匹配 ```json ... ``` 或 ``` ... ```
        let re = regex::Regex::new(r"```(?:json)?\s*\n([\s\S]*?)\n```").ok()?;
        re.captures(text)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
    }

    /// 解析计划 JSON 格式（来自 react_planning.md 的输出）
    /// 格式示例:
    /// {
    ///   "goal": "任务目标",
    ///   "complexity": "medium",
    ///   "steps": [
    ///     {"id": 1, "description": "...", "tool": "tool_name", "params": {...}, "depends_on": []}
    ///   ],
    ///   "expected_outcome": "预期结果"
    /// }
    fn parse_plan_json(text: &str) -> Result<ActionInstruction> {
        #[derive(Deserialize)]
        struct PlanJson {
            goal: Option<String>,
            steps: Vec<PlanStep>,
            #[allow(dead_code)]
            expected_outcome: Option<String>,
        }
        
        #[derive(Deserialize)]
        struct PlanStep {
            id: Option<i32>,
            description: Option<String>,
            tool: String,
            params: Option<serde_json::Value>,
            depends_on: Option<Vec<i32>>,
        }
        
        let plan: PlanJson = serde_json::from_str(text)
            .context("Failed to parse as plan JSON")?;
        
        // 确保至少有一个步骤
        if plan.steps.is_empty() {
            return Err(anyhow!("Plan has no steps"));
        }
        
        // 转换为 SubPlan 格式
        let sub_plan_steps: Vec<SubPlanStep> = plan.steps.iter().map(|s| {
            SubPlanStep {
                id: s.id.map(|i| i.to_string()).unwrap_or_else(|| Uuid::new_v4().to_string()),
                description: s.description.clone().unwrap_or_else(|| format!("Execute {}", s.tool)),
                tool: Some(ReactToolCall::new(
                    s.tool.clone(),
                    s.params.clone().unwrap_or(serde_json::json!({})),
                )),
                depends_on: s.depends_on.clone().unwrap_or_default().iter().map(|i| i.to_string()).collect(),
                skippable: false,
            }
        }).collect();
        
        let plan_description = plan.goal.unwrap_or_else(|| "Execute planned steps".to_string());
        
        tracing::info!(
            "Parsed plan JSON: {} steps, goal: {}",
            sub_plan_steps.len(),
            plan_description
        );
        
        Ok(ActionInstruction::SubPlan {
            plan_description,
            steps: sub_plan_steps,
            allow_replanning: true,
        })
    }

    /// 自然语言解析（兜底策略）
    /// 支持格式：
    ///   Action: tool_name
    ///   Action Input: {"key": "value"}
    /// 或：
    ///   Final Answer: xxx
    /// 或：
    ///   普通对话文本（作为最终答案）
    fn parse_natural_language(text: &str) -> Result<ActionInstruction> {
        // 检查是否为最终答案
        if let Some(final_answer) = Self::extract_final_answer(text) {
            return Ok(ActionInstruction::FinalAnswer {
                final_answer: FinalAnswer {
                    answer: final_answer,
                    citations: Vec::new(),
                },
            });
        }

        // 提取 Action 和 Action Input
        match Self::extract_action_and_input(text) {
            Ok(parsed) => {
                Ok(ActionInstruction::ToolCall {
                    action: ReactToolCall::new(parsed.tool_name, parsed.args),
                    final_answer: false,
                })
            }
            Err(_) => {
                // 如果没有找到 Action，但文本看起来像是回答问题，当作最终答案
                if !text.trim().is_empty() && Self::looks_like_answer(text) {
                    Ok(ActionInstruction::FinalAnswer {
                        final_answer: FinalAnswer {
                            answer: text.trim().to_string(),
                            citations: Vec::new(),
                        },
                    })
                } else {
                    // 真的无法解析
                    Err(anyhow::anyhow!("No Action or Final Answer found in text"))
                }
            }
        }
    }

    /// 判断文本是否看起来像是一个答案（而不是需要工具调用的计划）
    fn looks_like_answer(text: &str) -> bool {
        let text_lower = text.to_lowercase();
        
        // 如果包含明确的思考标记但没有行动，可能是不完整的输出
        if text_lower.contains("thought:") && !text_lower.contains("action:") {
            // 检查是否有足够的内容作为答案
            return text.len() > 100;
        }
        
        // 如果文本较长且没有特定的 ReAct 格式标记，可能是直接回答
        if text.len() > 50 
            && !text_lower.contains("action:")
            && !text_lower.contains("action input:")
            && !text_lower.contains("thought:") {
            return true;
        }
        
        // 包含常见答案模式
        text_lower.contains("the answer is")
            || text_lower.contains("based on")
            || text_lower.contains("according to")
            || text_lower.contains("in summary")
            || text_lower.contains("in conclusion")
    }

    /// 提取 Final Answer
    fn extract_final_answer(text: &str) -> Option<String> {
        // 使用 (?s) 标志让 . 匹配换行符，并使用 .+? 非贪婪匹配
        // 匹配 "Final Answer:" 之后的所有内容
        let re = regex::Regex::new(r"(?is)Final\s*Answer\s*:\s*(.+)").ok()?;
        re.captures(text)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
    }

    /// 提取 Action 和 Action Input
    fn extract_action_and_input(text: &str) -> Result<ParsedAction> {
        // 提取 Action
        let action_re = regex::Regex::new(r"(?i)Action\s*:\s*([^\n]+)")
            .context("Failed to compile Action regex")?;
        let tool_name = action_re
            .captures(text)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
            .context("No Action found in text")?;

        // 提取 Action Input（支持多行JSON）
        // (?s) 使 . 匹配换行符，.+? 非贪婪匹配
        let input_re = regex::Regex::new(r"(?is)Action\s*Input\s*:\s*(.+?)(?:\n\n|$)")
            .context("Failed to compile Action Input regex")?;
        let input_str = input_re
            .captures(text)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| "{}".to_string());

        let args = serde_json::from_str(&input_str)
            .unwrap_or_else(|_| serde_json::json!({ "query": input_str }));

        Ok(ParsedAction { tool_name, args })
    }

    // ========== 新增执行模式解析方法 ==========

    /// 解析并行执行格式
    /// 格式示例:
    /// Parallel:
    /// - tool1({"arg": "value"})
    /// - tool2({"arg": "value"}) depends_on: t1
    fn parse_parallel_format(output: &str) -> Result<ActionInstruction> {
        let mut tools = Vec::new();
        let mut in_parallel = false;
        let mut tool_id = 0;

        for line in output.lines() {
            let trimmed = line.trim();

            if trimmed.to_lowercase().starts_with("parallel:") {
                in_parallel = true;
                continue;
            }

            if in_parallel && trimmed.starts_with("- ") {
                let content = trimmed.trim_start_matches("- ");
                
                // 解析依赖关系
                let (tool_part, deps) = if let Some((tp, dp)) = content.split_once(" depends_on:") {
                    let deps: Vec<String> = dp
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    (tp.trim(), deps)
                } else {
                    (content.trim(), vec![])
                };

                // 解析工具调用
                if let Ok((tool_name, args)) = Self::parse_tool_call_inline(tool_part) {
                    let id = format!("t{}", tool_id);
                    tool_id += 1;
                    tools.push(ParallelToolCall {
                        id,
                        tool: tool_name,
                        args,
                        depends_on: deps,
                    });
                }
            } else if in_parallel && !trimmed.is_empty() && !trimmed.starts_with("-") && !trimmed.starts_with("Thought") {
                // 并行块结束
                break;
            }
        }

        if tools.is_empty() {
            return Err(anyhow!("No parallel tools found"));
        }

        Ok(ActionInstruction::ParallelTools {
            tools,
            aggregation: AggregationStrategy::Merge,
        })
    }

    /// 解析推理链格式（ReWOO 风格）
    /// 格式示例:
    /// Plan:
    /// #E1 = tool1(arg1) // reasoning
    /// #E2 = tool2(#E1) // use result from E1
    /// Solve: Based on the results, answer the question.
    fn parse_reasoning_chain_format(output: &str) -> Result<ActionInstruction> {
        let mut steps = Vec::new();
        let mut solve_prompt = None;
        let mut in_plan = false;

        for line in output.lines() {
            let trimmed = line.trim();

            if trimmed.to_lowercase().starts_with("plan:") {
                in_plan = true;
                continue;
            }

            if trimmed.to_lowercase().starts_with("solve:") {
                solve_prompt = Some(trimmed.trim_start_matches("Solve:").trim_start_matches("solve:").trim().to_string());
                in_plan = false;
                continue;
            }

            if in_plan && trimmed.starts_with("#E") {
                // 解析推理步骤: #E1 = tool(args) // reasoning
                if let Some((var_part, rest)) = trimmed.split_once(" = ") {
                    let variable = var_part.trim().to_string();

                    // 分离工具调用和推理说明
                    let (tool_call, reasoning) = if let Some((tc, r)) = rest.split_once(" // ") {
                        (tc.trim(), r.trim().to_string())
                    } else if let Some((tc, r)) = rest.split_once(" //") {
                        (tc.trim(), r.trim().to_string())
                    } else {
                        (rest.trim(), String::new())
                    };

                    // 解析工具名和参数
                    if let Some((tool_name, args_str)) = tool_call.split_once('(') {
                        let args = args_str.trim_end_matches(')').to_string();
                        steps.push(ReasoningStep {
                            variable,
                            tool: tool_name.trim().to_string(),
                            args,
                            reasoning,
                        });
                    }
                }
            }
        }

        if steps.is_empty() {
            return Err(anyhow!("No reasoning steps found"));
        }

        Ok(ActionInstruction::ReasoningChain {
            steps,
            solve_prompt,
        })
    }

    /// 解析阶段执行格式
    /// 格式示例:
    /// Phase: Reconnaissance
    /// Description: Gather information about the target
    /// Tools:
    /// - rsubdomain({"target": "example.com"})
    /// - dns_scanner({"domain": "example.com"})
    /// Next: Scanning phase
    fn parse_phase_format(output: &str) -> Result<ActionInstruction> {
        let mut phase_name = String::new();
        let mut description = String::new();
        let mut tool_calls = Vec::new();
        let mut next_hint = None;
        let mut phase_type = PhaseType::Custom("default".to_string());
        let mut in_tools = false;
        let mut tool_id = 0;

        for line in output.lines() {
            let trimmed = line.trim();

            if trimmed.to_lowercase().starts_with("phase:") {
                phase_name = trimmed.split_once(':').map(|(_, v)| v.trim().to_string()).unwrap_or_default();
                phase_type = Self::infer_phase_type(&phase_name);
                in_tools = false;
            } else if trimmed.to_lowercase().starts_with("description:") {
                description = trimmed.split_once(':').map(|(_, v)| v.trim().to_string()).unwrap_or_default();
            } else if trimmed.to_lowercase().starts_with("tools:") {
                in_tools = true;
            } else if trimmed.to_lowercase().starts_with("next:") {
                next_hint = Some(trimmed.split_once(':').map(|(_, v)| v.trim().to_string()).unwrap_or_default());
                in_tools = false;
            } else if in_tools && trimmed.starts_with("- ") {
                let content = trimmed.trim_start_matches("- ");
                if let Ok((tool_name, args)) = Self::parse_tool_call_inline(content) {
                    tool_calls.push(ParallelToolCall {
                        id: format!("phase_t{}", tool_id),
                        tool: tool_name,
                        args,
                        depends_on: vec![],
                    });
                    tool_id += 1;
                }
            }
        }

        if phase_name.is_empty() {
            return Err(anyhow!("Phase name not found"));
        }

        Ok(ActionInstruction::PhaseExecution {
            phase: PhaseDefinition {
                name: phase_name,
                description,
                phase_type,
                tool_calls,
                timeout_seconds: 600,
            },
            next_phase_hint: next_hint,
        })
    }

    /// 解析子计划格式
    /// 格式示例:
    /// SubPlan: Execute security scan
    /// Steps:
    /// - step1: Enumerate subdomains | tool: rsubdomain({"target": "example.com"})
    /// - step2: Scan ports | depends_on: step1 | tool: port_scan({"target": "$step1"})
    fn parse_sub_plan_format(output: &str) -> Result<ActionInstruction> {
        let mut plan_description = String::new();
        let mut steps = Vec::new();
        let mut in_steps = false;

        for line in output.lines() {
            let trimmed = line.trim();

            if trimmed.to_lowercase().starts_with("subplan:") || trimmed.to_lowercase().starts_with("sub_plan:") {
                plan_description = trimmed.split_once(':').map(|(_, v)| v.trim().to_string()).unwrap_or_default();
            } else if trimmed.to_lowercase().starts_with("steps:") {
                in_steps = true;
            } else if in_steps && trimmed.starts_with("- ") {
                let content = trimmed.trim_start_matches("- ");
                
                // 解析步骤: step_id: description | depends_on: ... | tool: ...
                let parts: Vec<&str> = content.split('|').map(|s| s.trim()).collect();
                
                if let Some(first_part) = parts.first() {
                    let (step_id, step_desc) = if let Some((id, desc)) = first_part.split_once(':') {
                        (id.trim().to_string(), desc.trim().to_string())
                    } else {
                        (Uuid::new_v4().to_string(), first_part.to_string())
                    };
                    
                    let mut depends_on = Vec::new();
                    let mut tool_call = None;
                    let mut skippable = false;
                    
                    for part in parts.iter().skip(1) {
                        if part.to_lowercase().starts_with("depends_on:") {
                            depends_on = part.split_once(':')
                                .map(|(_, v)| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                                .unwrap_or_default();
                        } else if part.to_lowercase().starts_with("tool:") {
                            if let Some((_, tool_str)) = part.split_once(':') {
                                if let Ok((tool_name, args)) = Self::parse_tool_call_inline(tool_str.trim()) {
                                    tool_call = Some(ReactToolCall::new(tool_name, args));
                                }
                            }
                        } else if part.to_lowercase().contains("skippable") {
                            skippable = true;
                        }
                    }
                    
                    steps.push(SubPlanStep {
                        id: step_id,
                        description: step_desc,
                        depends_on,
                        tool: tool_call,
                        skippable,
                    });
                }
            }
        }

        if steps.is_empty() {
            return Err(anyhow!("No sub-plan steps found"));
        }

        Ok(ActionInstruction::SubPlan {
            plan_description,
            steps,
            allow_replanning: false,
        })
    }

    /// 解析内联工具调用格式
    /// 格式: tool_name({"key": "value"}) 或 tool_name(key=value)
    fn parse_tool_call_inline(content: &str) -> Result<(String, serde_json::Value)> {
        let content = content.trim();
        
        // 查找括号位置
        if let Some(paren_start) = content.find('(') {
            let tool_name = content[..paren_start].trim().to_string();
            let args_str = content[paren_start + 1..].trim_end_matches(')').trim();
            
            // 尝试解析为 JSON
            let args = if args_str.starts_with('{') {
                serde_json::from_str(args_str)
                    .unwrap_or_else(|_| serde_json::json!({ "input": args_str }))
            } else if args_str.is_empty() {
                serde_json::json!({})
            } else {
                // 尝试解析 key=value 格式
                let mut map = serde_json::Map::new();
                for part in args_str.split(',') {
                    if let Some((k, v)) = part.split_once('=') {
                        let key = k.trim().trim_matches('"');
                        let value = v.trim().trim_matches('"');
                        map.insert(key.to_string(), serde_json::Value::String(value.to_string()));
                    }
                }
                if map.is_empty() {
                    serde_json::json!({ "input": args_str })
                } else {
                    serde_json::Value::Object(map)
                }
            };
            
            Ok((tool_name, args))
        } else {
            // 无参数工具
            Ok((content.to_string(), serde_json::json!({})))
        }
    }

    /// 推断阶段类型
    fn infer_phase_type(name: &str) -> PhaseType {
        let lower = name.to_lowercase();
        if lower.contains("recon") || lower.contains("信息") || lower.contains("收集") || lower.contains("enumerat") {
            PhaseType::Reconnaissance
        } else if lower.contains("scan") || lower.contains("扫描") || lower.contains("探测") {
            PhaseType::Scanning
        } else if lower.contains("valid") || lower.contains("验证") || lower.contains("verif") {
            PhaseType::Validation
        } else if lower.contains("analy") || lower.contains("分析") {
            PhaseType::Analysis
        } else if lower.contains("exploit") || lower.contains("利用") || lower.contains("attack") {
            PhaseType::Exploitation
        } else if lower.contains("report") || lower.contains("报告") {
            PhaseType::Reporting
        } else {
            PhaseType::Custom(name.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_tool_call() {
        let json = r#"{"action": {"tool": "search_web", "args": {"query": "test"}, "call_id": "123", "is_parallel": false}, "final_answer": false}"#;
        let result = ActionParser::parse(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_json_block() {
        let text = r#"
```json
{"action": {"tool": "search_web", "args": {"query": "test"}, "call_id": "456", "is_parallel": false}, "final_answer": false}
```
"#;
        let result = ActionParser::parse(text);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_natural_language() {
        let text = r#"
Thought: I need to search for information.
Action: search_web
Action Input: {"query": "Rust programming"}
"#;
        let result = ActionParser::parse(text);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_final_answer() {
        let text = "Final Answer: The answer is 42.";
        let result = ActionParser::parse(text);
        assert!(result.is_ok());
        match result.unwrap() {
            ActionInstruction::FinalAnswer { final_answer } => {
                assert_eq!(final_answer.answer, "The answer is 42.");
            }
            _ => panic!("Expected FinalAnswer"),
        }
    }

    #[test]
    fn test_parse_multiline_action_input() {
        // 测试多行格式化的JSON（这是LLM常见的输出格式）
        let text = r#"
Thought: I need to navigate to the URL with proxy settings.
Action: playwright_navigate
Action Input: {
  "url": "http://testphp.vulnweb.com",
  "proxy": {
    "server": "http://127.0.0.1:8080"
  }
}
"#;
        let result = ActionParser::parse(text);
        assert!(result.is_ok(), "Failed to parse multiline Action Input");
        
        match result.unwrap() {
            ActionInstruction::ToolCall { action, .. } => {
                assert_eq!(action.tool, "playwright_navigate");
                assert!(action.args.get("url").is_some());
                assert_eq!(action.args["url"], "http://testphp.vulnweb.com");
                assert!(action.args.get("proxy").is_some());
            }
            _ => panic!("Expected ToolCall"),
        }
    }

    #[test]
    fn test_parse_single_line_action_input() {
        // 测试单行紧凑的JSON
        let text = r#"
Action: playwright_navigate
Action Input: {"url": "http://testphp.vulnweb.com"}
"#;
        let result = ActionParser::parse(text);
        assert!(result.is_ok(), "Failed to parse single-line Action Input");
        
        match result.unwrap() {
            ActionInstruction::ToolCall { action, .. } => {
                assert_eq!(action.tool, "playwright_navigate");
                assert_eq!(action.args["url"], "http://testphp.vulnweb.com");
            }
            _ => panic!("Expected ToolCall"),
        }
    }
}

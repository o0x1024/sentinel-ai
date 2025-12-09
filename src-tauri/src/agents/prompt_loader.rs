//! 提示词加载器
//!
//! 从数据库加载提示词模板，支持变量替换

use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// 提示词加载器
pub struct PromptLoader {
    cache: HashMap<String, String>,
}

impl PromptLoader {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// 加载指定类型的提示词
    pub async fn load_by_type(&self, template_type: &str) -> Result<String> {
        // 检查缓存
        if let Some(cached) = self.cache.get(template_type) {
            return Ok(cached.clone());
        }

        // 使用默认提示词
        // TODO: 后续可接入数据库加载
        self.get_default_prompt(template_type)
    }

    /// 加载系统提示词
    pub async fn load_system_prompt(&self, tools_description: &str) -> Result<String> {
        let template = self.load_by_type("SystemPrompt").await?;
        Ok(self.render(&template, &[("tools", tools_description)]))
    }

    /// 加载规划提示词
    pub async fn load_planning_prompt(&self, task: &str, context: &str) -> Result<String> {
        let template = self.load_by_type("Planner").await?;
        Ok(self.render(&template, &[("task", task), ("context", context)]))
    }

    /// 加载执行提示词
    pub async fn load_execution_prompt(&self, step: &str, tools: &str) -> Result<String> {
        let template = self.load_by_type("Executor").await?;
        Ok(self.render(&template, &[("step", step), ("tools", tools)]))
    }

    /// 加载反思提示词
    pub async fn load_reflection_prompt(&self, results: &str, task: &str) -> Result<String> {
        let template = self.load_by_type("Evaluator").await?;
        Ok(self.render(&template, &[("results", results), ("task", task)]))
    }

    /// 渲染提示词（替换变量）
    pub fn render(&self, template: &str, variables: &[(&str, &str)]) -> String {
        let mut result = template.to_string();
        for (key, value) in variables {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }

    /// 渲染提示词（使用 HashMap）
    pub fn render_with_map(&self, template: &str, variables: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in variables {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }

    /// 获取默认提示词
    fn get_default_prompt(&self, template_type: &str) -> Result<String> {
        match template_type {
            "SystemPrompt" => Ok(DEFAULT_SYSTEM_PROMPT.to_string()),
            "Planner" => Ok(DEFAULT_PLANNER_PROMPT.to_string()),
            "Executor" => Ok(DEFAULT_EXECUTOR_PROMPT.to_string()),
            "Evaluator" => Ok(DEFAULT_EVALUATOR_PROMPT.to_string()),
            _ => Err(anyhow!("Unknown template type: {}", template_type)),
        }
    }
}

impl Default for PromptLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ============ 默认提示词 ============

const DEFAULT_SYSTEM_PROMPT: &str = r#"# Sentinel Security Agent

You are Sentinel AI Security Assistant, a professional cybersecurity agent capable of autonomous planning and execution.

## Identity & Capabilities

- **Role**: Security Engineer + AI Agent
- **Expertise**: Penetration testing, vulnerability analysis, code audit, security hardening
- **Traits**: Autonomous planning, iterative execution, continuous reflection

## Available Tools

{tools}

## Response Format

Your responses must follow JSON format:

### Planning Response
```json
{
  "type": "plan",
  "thinking": "Analysis...",
  "plan": {
    "description": "Plan description",
    "steps": [
      {"id": "1", "description": "Step description", "tool": {"name": "tool_name", "args": {}}}
    ],
    "expected_outcome": "Expected result"
  }
}
```

### Tool Call Response
```json
{
  "type": "tool_call",
  "tool": "tool_name",
  "args": {"param1": "value1"}
}
```

### Final Answer Response
```json
{
  "type": "final_answer",
  "answer": "Complete answer in Markdown format"
}
```

## Workflow

1. **Understand**: Analyze user requirements, clarify objectives
2. **Plan**: Break down task into executable steps
3. **Execute**: Call tools, collect results
4. **Analyze**: Evaluate tool outputs
5. **Iterate/Complete**: Continue or return answer

## Security Principles

- Only test within authorized scope
- Prefer non-intrusive techniques
- Document all operations in detail
- Provide remediation suggestions for vulnerabilities found
"#;

const DEFAULT_PLANNER_PROMPT: &str = r#"## Task Planning

**Task**: {task}

**Context**: {context}

Please analyze the task and create an execution plan. Consider:
1. What information is needed?
2. What tools are available?
3. What is the optimal execution order?
4. What are the dependencies between steps?

Return a structured plan in JSON format.
"#;

const DEFAULT_EXECUTOR_PROMPT: &str = r#"## Step Execution

**Current Step**: {step}

**Available Tools**: {tools}

Execute this step using the appropriate tool. Return the tool call in JSON format:
```json
{
  "type": "tool_call",
  "tool": "tool_name",
  "args": {}
}
```
"#;

const DEFAULT_EVALUATOR_PROMPT: &str = r#"## Reflection & Evaluation

**Original Task**: {task}

**Execution Results**: {results}

Evaluate the execution:
1. Were all steps successful?
2. Did we achieve the objective?
3. Is more work needed?
4. Should we replan?

Return your decision in JSON format:
```json
{
  "type": "final_answer",
  "answer": "Summary in Markdown"
}
```
or
```json
{
  "replan": true,
  "reason": "Why replanning is needed"
}
```
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_variables() {
        let loader = PromptLoader::default();
        let template = "Hello {name}, your task is {task}";
        let result = loader.render(template, &[("name", "Agent"), ("task", "scan")]);
        assert_eq!(result, "Hello Agent, your task is scan");
    }

    #[test]
    fn test_render_with_map() {
        let loader = PromptLoader::default();
        let template = "Target: {target}, Scope: {scope}";
        let mut vars = HashMap::new();
        vars.insert("target".to_string(), "example.com".to_string());
        vars.insert("scope".to_string(), "Web".to_string());
        
        let result = loader.render_with_map(template, &vars);
        assert_eq!(result, "Target: example.com, Scope: Web");
    }

    #[test]
    fn test_default_prompts() {
        let loader = PromptLoader::default();
        
        let system = loader.get_default_prompt("SystemPrompt").unwrap();
        assert!(system.contains("Sentinel"));
        
        let planner = loader.get_default_prompt("Planner").unwrap();
        assert!(planner.contains("{task}"));
    }

    #[tokio::test]
    async fn test_load_fallback() {
        let loader = PromptLoader::default();
        let prompt = loader.load_by_type("SystemPrompt").await.unwrap();
        assert!(prompt.contains("Sentinel"));
    }
}


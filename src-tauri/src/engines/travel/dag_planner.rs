//! DAG 规划器 - Token优化核心组件
//!
//! 借鉴 LLMCompiler 的一次规划理念，用单次 LLM 调用生成完整执行计划

use super::types::*;
use crate::engines::llm_client::{LlmClient, create_client as create_llm_client};
use crate::models::prompt::{ArchitectureType, StageType};
use crate::services::ai::AiService;
use crate::services::prompt_db::PromptRepository;
use crate::tools::FrameworkToolAdapter;
use anyhow::{anyhow, Result};
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

/// DAG 规划器
pub struct DagPlanner {
    llm_client: LlmClient,
    tool_adapter: Option<Arc<dyn FrameworkToolAdapter>>,
    prompt_repo: Option<Arc<PromptRepository>>,
    config: LiteModeConfig,
}

impl DagPlanner {
    pub fn new(ai_service: Arc<AiService>, config: LiteModeConfig) -> Self {
        Self {
            llm_client: create_llm_client(&ai_service),
            tool_adapter: None,
            prompt_repo: None,
            config,
        }
    }

    pub fn with_tool_adapter(mut self, adapter: Arc<dyn FrameworkToolAdapter>) -> Self {
        self.tool_adapter = Some(adapter);
        self
    }

    pub fn with_prompt_repo(mut self, repo: Arc<PromptRepository>) -> Self {
        self.prompt_repo = Some(repo);
        self
    }

    /// 生成 DAG 执行计划 (单次 LLM 调用，使用内置 LlmClient)
    pub async fn generate_plan(
        &self,
        task_description: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<DagPlan> {
        log::info!("DagPlanner: Generating DAG plan for: {}", task_description);

        // 1. 获取可用工具描述
        let tools_description = self.build_tools_description(context).await;

        // 2. 构建 prompt
        let (system_prompt, user_prompt) = self
            .build_planning_prompt(task_description, context, &tools_description)
            .await?;

        // 3. 单次 LLM 调用生成计划（使用内置客户端）
        let response = self
            .llm_client
            .completion(Some(&system_prompt), &user_prompt)
            .await
            .map_err(|e| anyhow!("Travel DAG planning LLM call failed: {}", e))?;

        // 4. 解析 LLM 响应为 DAG 计划
        self.parse_dag_plan(&response, task_description)
    }

    /// 构建工具描述 (精简格式节省 Token)
    async fn build_tools_description(
        &self,
        context: &HashMap<String, serde_json::Value>,
    ) -> String {
        // 获取工具白名单
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
            log::warn!("DagPlanner: No allowed tools in context");
            return "No tools available".to_string();
        }

        let mut descriptions = Vec::new();

        // 优先使用 FrameworkToolAdapter
        if let Some(adapter) = &self.tool_adapter {
            for tool_name in &allowed_tools {
                if let Some(info) = adapter.get_tool_info(tool_name).await {
                    // 精简格式: tool_name(params) - description
                    let params: Vec<String> = info
                        .parameters
                        .parameters
                        .iter()
                        .map(|p| {
                            let type_str = match p.param_type {
                                crate::tools::ParameterType::String => "str",
                                crate::tools::ParameterType::Number => "num",
                                crate::tools::ParameterType::Boolean => "bool",
                                crate::tools::ParameterType::Array => "arr",
                                crate::tools::ParameterType::Object => "obj",
                            };
                            if p.required {
                                format!("{}: {}", p.name, type_str)
                            } else {
                                format!("{}?: {}", p.name, type_str)
                            }
                        })
                        .collect();

                    descriptions.push(format!(
                        "- {}({}) - {}",
                        info.name,
                        params.join(", "),
                        info.description
                    ));
                }
            }
        } else {
            // 降级使用全局 adapter
            if let Ok(engine_adapter) = crate::tools::get_global_engine_adapter() {
                for tool_name in &allowed_tools {
                    if let Some(info) = engine_adapter.get_tool_info(tool_name).await {
                        let params: Vec<String> = info
                            .parameters
                            .parameters
                            .iter()
                            .map(|p| {
                                let type_str = match p.param_type {
                                    crate::tools::ParameterType::String => "str",
                                    crate::tools::ParameterType::Number => "num",
                                    crate::tools::ParameterType::Boolean => "bool",
                                    crate::tools::ParameterType::Array => "arr",
                                    crate::tools::ParameterType::Object => "obj",
                                };
                                if p.required {
                                    format!("{}: {}", p.name, type_str)
                                } else {
                                    format!("{}?: {}", p.name, type_str)
                                }
                            })
                            .collect();

                        descriptions.push(format!(
                            "- {}({}) - {}",
                            info.name,
                            params.join(", "),
                            info.description
                        ));
                    }
                }
            }
        }

        if descriptions.is_empty() {
            allowed_tools.iter().map(|n| format!("- {}", n)).collect::<Vec<_>>().join("\n")
        } else {
            descriptions.join("\n")
        }
    }

    /// 构建规划 prompt (精简版)
    async fn build_planning_prompt(
        &self,
        task_description: &str,
        context: &HashMap<String, serde_json::Value>,
        tools_description: &str,
    ) -> Result<(String, String)> {
        // 尝试从数据库获取 prompt 模板
        let system_template = if let Some(repo) = &self.prompt_repo {
            if let Ok(Some(template)) = repo
                .get_template_by_arch_stage(ArchitectureType::Travel, StageType::Planner)
                .await
            {
                log::info!("DagPlanner: Using prompt from database");
                template.content
            } else {
                log::info!("DagPlanner: Using default prompt template");
                self.default_planning_prompt()
            }
        } else {
            self.default_planning_prompt()
        };

        // 提取目标信息
        let target = context
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        // 替换占位符
        let system_prompt = system_template
            .replace("{tools}", tools_description)
            .replace("{max_steps}", &self.config.max_steps.to_string());

        let user_prompt = format!(
            "任务: {}\n目标: {}\n\n请生成执行计划。",
            task_description, target
        );

        Ok((system_prompt, user_prompt))
    }

    /// 默认规划 prompt (精简版节省Token)
    fn default_planning_prompt(&self) -> String {
        r#"你是安全测试规划器。根据任务生成工具调用计划。

## 可用工具
{tools}

## 输出格式 (每行一个任务)
```
1. tool_name(arg1="val1", arg2="val2")
2. tool_name(arg1=$1.field) depends: 1
3. join()
```

## 规则
1. 用 $N 引用第N个任务的结果，如 $1.status
2. 用 depends: N,M 声明依赖关系
3. 用 join() 结束计划
4. 最多 {max_steps} 个任务
5. 只输出计划，不要解释

生成计划:"#
            .to_string()
    }

    /// 解析 LLM 响应为 DAG 计划
    fn parse_dag_plan(&self, response: &str, task_description: &str) -> Result<DagPlan> {
        let mut plan = DagPlan::new(task_description.to_string());

        // 提取代码块中的内容
        let content = if response.contains("```") {
            response
                .split("```")
                .nth(1)
                .map(|s| s.trim_start_matches("plaintext").trim_start_matches('\n'))
                .unwrap_or(response)
                .trim()
        } else {
            response.trim()
        };

        // 解析每行任务
        // 格式: N. tool_name(arg1="val1", arg2="val2") depends: M,K
        let task_regex = Regex::new(
            r#"(\d+)\.\s*(\w+)\s*\(([^)]*)\)(?:\s*depends:\s*([\d,\s]+))?"#
        )?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // 跳过 join() 结束标记
            if line.contains("join()") {
                continue;
            }

            if let Some(captures) = task_regex.captures(line) {
                let task_id = captures.get(1).map(|m| m.as_str()).unwrap_or("0");
                let tool_name = captures.get(2).map(|m| m.as_str()).unwrap_or("");
                let args_str = captures.get(3).map(|m| m.as_str()).unwrap_or("");
                let depends_str = captures.get(4).map(|m| m.as_str());

                // 解析参数
                let arguments = self.parse_arguments(args_str);

                // 解析依赖
                let depends_on: Vec<String> = depends_str
                    .map(|s| {
                        s.split(',')
                            .map(|d| d.trim().to_string())
                            .filter(|d| !d.is_empty())
                            .collect()
                    })
                    .unwrap_or_default();

                let task = DagTask::new(task_id.to_string(), tool_name.to_string(), arguments)
                    .with_depends(depends_on);

                plan.add_task(task);
            }
        }

        if plan.tasks.is_empty() {
            return Err(anyhow!("Failed to parse any tasks from LLM response"));
        }

        log::info!("DagPlanner: Parsed {} tasks from LLM response", plan.tasks.len());
        Ok(plan)
    }

    /// 解析参数字符串
    fn parse_arguments(&self, args_str: &str) -> HashMap<String, serde_json::Value> {
        let mut arguments = HashMap::new();

        // 格式: arg1="val1", arg2="val2", arg3=$1.field
        let arg_regex = Regex::new(r#"(\w+)\s*=\s*(?:"([^"]*)"|(\$[\d.]+\w*)|(\d+(?:\.\d+)?)|(\w+))"#)
            .unwrap();

        for captures in arg_regex.captures_iter(args_str) {
            let name = captures.get(1).map(|m| m.as_str()).unwrap_or("");

            let value = if let Some(quoted) = captures.get(2) {
                // 字符串值
                serde_json::Value::String(quoted.as_str().to_string())
            } else if let Some(var_ref) = captures.get(3) {
                // 变量引用 $1.field
                serde_json::Value::String(var_ref.as_str().to_string())
            } else if let Some(num) = captures.get(4) {
                // 数字
                if let Ok(n) = num.as_str().parse::<f64>() {
                    serde_json::json!(n)
                } else {
                    serde_json::Value::String(num.as_str().to_string())
                }
            } else if let Some(word) = captures.get(5) {
                // 布尔或其他
                match word.as_str() {
                    "true" => serde_json::Value::Bool(true),
                    "false" => serde_json::Value::Bool(false),
                    _ => serde_json::Value::String(word.as_str().to_string()),
                }
            } else {
                serde_json::Value::Null
            };

            if !name.is_empty() {
                arguments.insert(name.to_string(), value);
            }
        }

        arguments
    }

    /// 解析变量引用并替换值
    pub fn resolve_variable_references(
        arguments: &mut HashMap<String, serde_json::Value>,
        task_results: &HashMap<String, serde_json::Value>,
    ) {
        for (_, value) in arguments.iter_mut() {
            if let serde_json::Value::String(s) = value {
                if s.starts_with('$') {
                    // 解析 $1.field 格式
                    if let Some(resolved) = Self::resolve_reference(s, task_results) {
                        *value = resolved;
                    }
                }
            }
        }
    }

    /// 解析单个变量引用
    fn resolve_reference(
        reference: &str,
        task_results: &HashMap<String, serde_json::Value>,
    ) -> Option<serde_json::Value> {
        // 格式: $1 或 $1.field 或 $1.nested.field
        let parts: Vec<&str> = reference.trim_start_matches('$').split('.').collect();
        if parts.is_empty() {
            return None;
        }

        let task_id = parts[0];
        let result = task_results.get(task_id)?;

        if parts.len() == 1 {
            return Some(result.clone());
        }

        // 访问嵌套字段
        let mut current = result;
        for field in &parts[1..] {
            current = current.get(field)?;
        }

        Some(current.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arguments() {
        let planner = DagPlanner::new(
            Arc::new(AiService::default()),
            LiteModeConfig::default(),
        );

        let args = planner.parse_arguments(r#"target="192.168.1.1", ports="80,443""#);
        assert_eq!(args.get("target"), Some(&serde_json::json!("192.168.1.1")));
        assert_eq!(args.get("ports"), Some(&serde_json::json!("80,443")));
    }

    #[test]
    fn test_resolve_variable() {
        let mut results = HashMap::new();
        results.insert(
            "1".to_string(),
            serde_json::json!({"host": "example.com", "status": "success"}),
        );

        let resolved = DagPlanner::resolve_reference("$1.host", &results);
        assert_eq!(resolved, Some(serde_json::json!("example.com")));
    }
}


//! LLMCompiler Planner组件
//!
//! 负责生成DAG（有向无环图）任务计划的智能规划器
//! 核心特性：
//! - 流式生成任务（边生成边执行）
//! - 识别任务间的依赖关系
//! - 支持变量替换（$1, $2等）
//! - 可重新规划

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, warn, debug};

use crate::services::ai::AiService;

use super::types::*;
use crate::services::prompt_db::PromptRepository;
use crate::models::prompt::{ArchitectureType, StageType};

/// LLMCompiler Planner - 智能任务规划器
pub struct LlmCompilerPlanner {
    ai_service: Arc<AiService>,
    tool_adapter: Arc<dyn crate::tools::EngineToolAdapter>,
    #[allow(unused)]
    config: LlmCompilerConfig,
    prompt_repo: Option<PromptRepository>,
}

impl LlmCompilerPlanner {
    pub fn new(ai_service: Arc<AiService>, tool_adapter: Arc<dyn crate::tools::EngineToolAdapter>, config: LlmCompilerConfig, prompt_repo: Option<PromptRepository>) -> Self {
        Self { ai_service, tool_adapter, config, prompt_repo }
    }

    /// 生成DAG执行计划
    pub async fn generate_dag_plan(
        &self,
        user_input: &str,
        context: &HashMap<String, Value>,
    ) -> Result<DagExecutionPlan> {
        info!("开始生成DAG执行计划: {}", user_input);

        // 获取可用工具列表
        let available_tools = self.get_available_tools_description().await?;
        let prompt = self.build_planning_prompt_with_tools(user_input, context, &available_tools).await?;
        
        // 调用LLM生成计划
        let response = self.ai_service.send_message_stream_with_prompt(&prompt, None, None).await?;
        
        // 解析LLM响应为DAG计划
        self.parse_dag_plan(&response, user_input).await
    }
    
    /// 获取可用工具描述
    async fn get_available_tools_description(&self) -> Result<String> {
        let tools = self.tool_adapter.list_available_tools().await;
        
        if tools.is_empty() {
            return Ok("暂无可用工具".to_string());
        }
        
        let mut tool_descriptions = Vec::new();
        for tool_name in &tools {
            if let Some(tool_info) = self.tool_adapter.get_tool_info(tool_name).await {
                tool_descriptions.push(format!(
                    "- {}: {} (类别: {:?})", 
                    tool_info.name, 
                    tool_info.description,
                    tool_info.category
                ));
            } else {
                tool_descriptions.push(format!("- {}: 工具信息不可用", tool_name));
            }
        }
            
        Ok(tool_descriptions.join("\n"))
    }
    
    /// 构建包含工具信息的规划提示
    async fn build_planning_prompt_with_tools(
        &self, 
        user_input: &str, 
        context: &HashMap<String, Value>,
        available_tools: &str
    ) -> Result<String> {
        let base = format!(
            r#"你是一个专业的LLMCompiler规划器，专门设计并行执行的DAG任务计划。

**用户输入**: {}
**上下文**: {}

**规划要求**：
1. 生成可并行执行的任务DAG
2. 最小化任务间的依赖关系
3. 使用变量引用（$1, $2等）处理任务间数据传递
4. 优化执行效率和资源利用

**可用工具**:
{}

**工具使用指南**：
- 选择最适合任务目标的工具
- 考虑工具间的数据依赖关系
- 优先使用可并行执行的工具组合
- 确保工具参数格式正确
- 只使用上述列出的可用工具

**DAG设计原则**：
1. **并行优先**: 无依赖的任务应该并行执行
2. **依赖最小化**: 减少不必要的任务依赖
3. **数据流清晰**: 使用变量引用明确数据传递
4. **错误隔离**: 单个任务失败不应影响无关任务

请以JSON格式返回DAG计划，包含：
- nodes: 任务节点列表
- dependency_graph: 依赖关系
- variable_mappings: 变量映射

示例格式：
{{
  "nodes": [
    {{
      "id": "task_1",
      "name": "基础信息收集",
      "description": "收集目标的基本信息",
      "tool_name": "dns_scanner",
      "inputs": {{"target": "example.com"}},
      "dependencies": [],
      "variable_refs": [],
      "priority": 1,
      "estimated_duration": 30
    }},
    {{
      "id": "task_2",
      "name": "端口扫描",
      "description": "基于IP地址进行端口扫描",
      "tool_name": "port_scanner",
      "inputs": {{"target": "$1", "scan_type": "tcp"}},
      "dependencies": ["task_1"],
      "variable_refs": ["$1"],
      "priority": 2,
      "estimated_duration": 60
    }}
  ],
  "dependency_graph": {{"task_2": ["task_1"]}},
  "variable_mappings": {{"$1": "task_1.outputs.ip_address"}}
}}
"#,
            user_input,
            serde_json::to_string_pretty(context).unwrap_or_default(),
            available_tools
         );
        if let Some(repo) = &self.prompt_repo {
            if let Ok(Some(dynamic)) = repo.get_active_prompt(ArchitectureType::LLMCompiler, StageType::Planning).await {
                let replaced = Self::apply_placeholders(&dynamic, vec![
                    ("{{USER_INPUT}}", user_input),
                    ("{user_input}", user_input),
                    ("{{CONTEXT}}", &serde_json::to_string_pretty(context).unwrap_or_default()),
                    ("{context}", &serde_json::to_string_pretty(context).unwrap_or_default()),
                    ("{{TOOLS}}", available_tools),
                    ("{tools}", available_tools),
                ]);
                return Ok(replaced);
            }
        }
        Ok(base)
    }

    fn apply_placeholders(template: &str, pairs: Vec<(&str, &str)>) -> String {
        let mut out = template.to_string();
        for (k, v) in pairs {
            out = out.replace(k, v);
        }
        out
    }

    /// 重新规划
    pub async fn replan(
        &self,
        original_plan: &DagExecutionPlan,
        execution_results: &[TaskExecutionResult],
        feedback: &str,
    ) -> Result<DagExecutionPlan> {
        info!("开始重新规划，反馈: {}", feedback);

        let prompt = self.build_replanning_prompt(original_plan, execution_results, feedback);
        let response = self.ai_service.send_message_stream_with_prompt(&prompt, None, None).await?;
        
        self.parse_dag_plan(&response, &format!("重规划-{}", original_plan.name)).await
    }

    /// 构建重规划提示
    fn build_replanning_prompt(
        &self,
        original_plan: &DagExecutionPlan,
        results: &[TaskExecutionResult],
        feedback: &str,
    ) -> String {
        // 分析已完成的任务和结果
        let completed_tasks: Vec<_> = results.iter()
            .filter(|r| r.status == TaskStatus::Completed)
            .collect();
        
        let failed_tasks: Vec<_> = results.iter()
            .filter(|r| r.status == TaskStatus::Failed)
            .collect();

        format!(
            r#"你是一个专业的LLMCompiler重规划专家。请根据执行反馈重新制定计划。

**原始计划**: {}
**已完成任务**: {} 个
**失败任务**: {} 个
**执行结果**: {}
**反馈信息**: {}

**重规划指导原则**：
1. **保留有效结果**: 利用已完成任务的输出
2. **解决发现的问题**: 针对反馈中提到的问题设计新任务
3. **优化执行路径**: 基于新发现调整任务优先级
4. **深入分析**: 对发现的安全问题进行更详细的测试

**重规划策略**：
- 如果发现了安全漏洞，添加深度分析任务
- 如果发现了新的攻击面，添加相应的测试任务
- 如果某些工具失败，尝试替代方案
- 基于已有结果优化后续任务的输入参数

请基于以上信息生成新的DAG计划，使用相同的JSON格式。
新计划应该：
1. 包含针对发现问题的深入测试
2. 利用已有的执行结果
3. 保持高效的并行执行结构
"#,
            serde_json::to_string_pretty(original_plan).unwrap_or_default(),
            completed_tasks.len(),
            failed_tasks.len(),
            serde_json::to_string_pretty(results).unwrap_or_default(),
            feedback
        )
    }

    /// 解析DAG计划
    async fn parse_dag_plan(&self, response: &str, plan_name: &str) -> Result<DagExecutionPlan> {
        debug!("解析LLM响应: {}", response);
        
        // 尝试提取JSON部分（LLM可能返回带解释的文本）
        let json_str = self.extract_json_from_response(response)?;
        
        // 解析JSON响应
        let parsed: Value = serde_json::from_str(&json_str)
            .map_err(|e| anyhow::anyhow!("解析LLM响应失败: {}", e))?;

        let nodes = parsed["nodes"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("缺少nodes字段"))?
            .iter()
            .enumerate()
            .map(|(index, node)| {
                Ok(DagTaskNode {
                    id: node["id"].as_str()
                        .unwrap_or(&format!("task_{}", index + 1))
                        .to_string(),
                    name: node["name"].as_str().unwrap_or("未命名任务").to_string(),
                    description: node["description"].as_str().unwrap_or("").to_string(),
                    tool_name: node["tool_name"].as_str()
                        .ok_or_else(|| anyhow::anyhow!("任务缺少tool_name字段"))?
                        .to_string(),
                    inputs: node["inputs"].as_object().cloned().unwrap_or_default()
                        .into_iter().collect(),
                    dependencies: node["dependencies"].as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect(),
                    variable_refs: node["variable_refs"].as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect(),
                    status: TaskStatus::Pending,
                    priority: node["priority"].as_i64().unwrap_or(1) as i32,
                    estimated_duration: node["estimated_duration"].as_u64(),
                    created_at: Utc::now(),
                    max_retries: node["max_retries"].as_u64().unwrap_or(3) as u32,
                    retry_count: 0,
                    tags: node["tags"].as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let dependency_graph = parsed["dependency_graph"]
            .as_object()
            .unwrap_or(&serde_json::Map::new())
            .iter()
            .map(|(k, v)| {
                let deps = v.as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect();
                (k.clone(), deps)
            })
            .collect();

        let variable_mappings = parsed["variable_mappings"]
            .as_object()
            .unwrap_or(&serde_json::Map::new())
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
            .collect();

        // 验证计划的有效性
        self.validate_dag_plan(&nodes, &dependency_graph)?;

        Ok(DagExecutionPlan {
            id: Uuid::new_v4().to_string(),
            name: plan_name.to_string(),
            nodes,
            dependency_graph,
            variable_mappings,
            global_config: HashMap::new(),
            created_at: Utc::now(),
            version: 1,
            parent_plan_id: None,
        })
    }

    /// 从响应中提取JSON
    fn extract_json_from_response(&self, response: &str) -> Result<String> {
        // 尝试找到JSON块
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                if end > start {
                    return Ok(response[start..=end].to_string());
                }
            }
        }
        
        // 如果没有找到JSON块，假设整个响应就是JSON
        Ok(response.to_string())
    }

    /// 验证DAG计划的有效性
    fn validate_dag_plan(
        &self,
        nodes: &[DagTaskNode],
        dependency_graph: &HashMap<String, Vec<String>>,
    ) -> Result<()> {
        // 检查是否有循环依赖
        if self.has_circular_dependency(dependency_graph) {
            return Err(anyhow::anyhow!("检测到循环依赖"));
        }

        // 检查依赖的任务是否存在
        let node_ids: std::collections::HashSet<_> = nodes.iter().map(|n| &n.id).collect();
        for (task_id, deps) in dependency_graph {
            if !node_ids.contains(task_id) {
                warn!("依赖图中的任务 {} 不存在于节点列表中", task_id);
            }
            for dep in deps {
                if !node_ids.contains(dep) {
                    return Err(anyhow::anyhow!("任务 {} 依赖的任务 {} 不存在", task_id, dep));
                }
            }
        }

        info!("DAG计划验证通过: {} 个任务, {} 个依赖关系", nodes.len(), dependency_graph.len());
        Ok(())
    }

    /// 检查是否有循环依赖
    fn has_circular_dependency(&self, dependency_graph: &HashMap<String, Vec<String>>) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for node in dependency_graph.keys() {
            if self.has_cycle_util(node, dependency_graph, &mut visited, &mut rec_stack) {
                return true;
            }
        }
        false
    }

    /// 循环检测辅助函数
    fn has_cycle_util(
        &self,
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
    ) -> bool {
        if rec_stack.contains(node) {
            return true;
        }
        if visited.contains(node) {
            return false;
        }

        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(deps) = graph.get(node) {
            for dep in deps {
                if self.has_cycle_util(dep, graph, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }
}
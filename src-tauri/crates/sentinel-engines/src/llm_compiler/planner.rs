//! LLMCompiler Planner组件
//!
//! 负责生成DAG（有向无环图）任务计划的智能规划器
//! 核心特性：
//! - 流式生成任务（边生成边执行）
//! - 识别任务间的依赖关系
//! - 支持变量替换（$1, $2等）
//! - 可重新规划

use anyhow::Result;
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::services::ai::AiService;
use crate::utils::ordered_message::ChunkType;

use super::types::*;
use crate::models::prompt::ArchitectureType;
use crate::services::prompt_db::PromptRepository;
use crate::utils::prompt_resolver::{AgentPromptConfig, CanonicalStage, PromptResolver};

/// LLMCompiler Planner - 智能任务规划器
pub struct LlmCompilerPlanner {
    ai_service: Arc<AiService>,
    tool_adapter: Arc<dyn crate::tools::EngineToolAdapter>,
    #[allow(unused)]
    config: LlmCompilerConfig,
    prompt_repo: Option<PromptRepository>,
    runtime_params: Option<HashMap<String, Value>>,
}

impl LlmCompilerPlanner {
    pub fn new(
        ai_service: Arc<AiService>,
        tool_adapter: Arc<dyn crate::tools::EngineToolAdapter>,
        config: LlmCompilerConfig,
        prompt_repo: Option<PromptRepository>,
    ) -> Self {
        Self {
            ai_service,
            tool_adapter,
            config,
            prompt_repo,
            runtime_params: None,
        }
    }

    pub fn set_runtime_params(&mut self, params: HashMap<String, Value>) {
        self.runtime_params = Some(params)
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
        let prompt = self
            .build_planning_prompt_with_tools(user_input, context, &available_tools)
            .await?;

        // 调用LLM生成计划
        let response = self
            .ai_service
            .send_message_stream(
                None, 
                Some(&prompt), 
                None, 
                None, 
                true, 
                false,
                Some(ChunkType::PlanInfo)
            )
            .await?;

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
                    tool_info.name, tool_info.description, tool_info.category
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
        available_tools: &str,
    ) -> Result<String> {
        let mut base = format!(
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
        // RAG augmentation for LLMCompiler planning (global toggle)
        if let Ok(rag_service) = crate::commands::rag_commands::get_global_rag_service().await {
            if rag_service.get_config().augmentation_enabled {
                use tokio::time::{timeout, Duration};
                let (primary, fallback) = crate::rag::query_utils::build_rag_query_pair(&format!("{} {}", user_input, serde_json::to_string_pretty(context).unwrap_or_default()));
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
                    system_prompt: None,
                };
                if let Ok(Ok((knowledge_context, _))) = timeout(
                    Duration::from_millis(1200),
                    rag_service.query_for_assistant(&rag_request),
                )
                .await
                {
                    if !knowledge_context.trim().is_empty() {
                        base.push_str("\n\n[KNOWLEDGE CONTEXT]\n");
                        base.push_str(&knowledge_context);
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
                            system_prompt: None,
                        };
                        if let Ok(Ok((kb2, _))) = timeout(Duration::from_millis(1200), rag_service.query_for_assistant(&fallback_req)).await {
                            if !kb2.trim().is_empty() {
                                base.push_str("\n\n[KNOWLEDGE CONTEXT]\n");
                                base.push_str(&kb2);
                            }
                        }
                    }
                }
            }
        }

        if let Some(repo) = &self.prompt_repo {
            // 使用统一提示词解析器，尽量从 context 解析 agent-level 配置
            let resolver = PromptResolver::new(repo.clone());
            let agent_config = AgentPromptConfig::parse_agent_config(context);
            let template = resolver
                .resolve_prompt(
                    &agent_config,
                    ArchitectureType::LLMCompiler,
                    CanonicalStage::Planner,
                    Some(&base),
                )
                .await
                .unwrap_or(base.clone());

            let mut replaced = Self::apply_placeholders(
                &template,
                vec![
                    ("{{USER_INPUT}}", user_input),
                    ("{user_input}", user_input),
                    (
                        "{{CONTEXT}}",
                        &serde_json::to_string_pretty(context).unwrap_or_default(),
                    ),
                    (
                        "{context}",
                        &serde_json::to_string_pretty(context).unwrap_or_default(),
                    ),
                    ("{{TOOLS}}", available_tools),
                    ("{tools}", available_tools),
                ],
            );
            
            // 集成角色提示词（如果存在）
            if let Some(role_prompt) = context.get("role_prompt").and_then(|v| v.as_str()) {
                if !role_prompt.trim().is_empty() {
                    replaced = if replaced.trim().is_empty() {
                        role_prompt.to_string()
                    } else {
                        format!("{}\n\n{}", role_prompt, replaced)
                    };
                    log::info!("LLM-Compiler planner: integrated role prompt");
                }
            }
            
            return Ok(replaced);
        }
        
        // 如果没有 prompt_repo，也要检查角色提示词
        if let Some(role_prompt) = context.get("role_prompt").and_then(|v| v.as_str()) {
            if !role_prompt.trim().is_empty() {
                base = if base.trim().is_empty() {
                    role_prompt.to_string()
                } else {
                    format!("{}\n\n{}", role_prompt, base)
                };
                log::info!("LLM-Compiler planner: integrated role prompt (no repo)");
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
        let response = self
            .ai_service
            .send_message_stream(
                None, 
                Some(&prompt), 
                None, 
                None, 
                false,
                false,
                Some(ChunkType::PlanInfo),
            )
            .await?;

        self.parse_dag_plan(&response, &format!("重规划-{}", original_plan.name))
            .await
    }

    /// 构建重规划提示
    fn build_replanning_prompt(
        &self,
        original_plan: &DagExecutionPlan,
        results: &[TaskExecutionResult],
        feedback: &str,
    ) -> String {
        // 分析已完成的任务和结果
        let completed_tasks: Vec<_> = results
            .iter()
            .filter(|r| r.status == TaskStatus::Completed)
            .collect();

        let failed_tasks: Vec<_> = results
            .iter()
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
                    id: node["id"]
                        .as_str()
                        .unwrap_or(&format!("task_{}", index + 1))
                        .to_string(),
                    name: node["name"].as_str().unwrap_or("未命名任务").to_string(),
                    description: node["description"].as_str().unwrap_or("").to_string(),
                    tool_name: node["tool_name"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("任务缺少tool_name字段"))?
                        .to_string(),
                    inputs: node["inputs"]
                        .as_object()
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .collect(),
                    dependencies: node["dependencies"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect(),
                    variable_refs: node["variable_refs"]
                        .as_array()
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
                    tags: node["tags"]
                        .as_array()
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
                let deps = v
                    .as_array()
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

    /// 从响应中提取JSON（增强的鲁棒性）
    fn extract_json_from_response(&self, response: &str) -> Result<String> {
        let trimmed = response.trim();

        // 1. 尝试多种JSON提取策略
        let candidates = vec![
            self.extract_json_from_code_blocks(trimmed),
            self.extract_json_from_braces(trimmed),
            self.extract_json_from_lines(trimmed),
            trimmed.to_string(), // 最后尝试原始响应
        ];

        // 2. 依次验证每个候选JSON
        for candidate in candidates {
            if let Some(cleaned) = self.clean_and_validate_json(&candidate) {
                return Ok(cleaned);
            }
        }

        // 3. 如果都失败，尝试修复常见的JSON错误
        if let Some(repaired) = self.repair_json(trimmed) {
            return Ok(repaired);
        }

        Err(anyhow::anyhow!(
            "无法从响应中提取有效的JSON: {}",
            if trimmed.len() > 200 {
                format!("{}...", &trimmed[..200])
            } else {
                trimmed.to_string()
            }
        ))
    }

    /// 从代码块中提取JSON (```json ... ```)
    fn extract_json_from_code_blocks(&self, text: &str) -> String {
        // 查找 ```json 或 ``` 代码块
        let patterns = ["```json\n", "```\n", "```json", "```"];

        for pattern in &patterns {
            if let Some(start_pos) = text.find(pattern) {
                let content_start = start_pos + pattern.len();
                if let Some(end_pos) = text[content_start..].find("```") {
                    let json_content = &text[content_start..content_start + end_pos];
                    return json_content.trim().to_string();
                }
            }
        }

        text.to_string()
    }

    /// 通过大括号提取JSON
    fn extract_json_from_braces(&self, text: &str) -> String {
        let mut brace_count = 0;
        let mut start_pos = None;
        let mut end_pos = None;

        for (i, ch) in text.char_indices() {
            match ch {
                '{' => {
                    if brace_count == 0 {
                        start_pos = Some(i);
                    }
                    brace_count += 1;
                }
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 && start_pos.is_some() {
                        end_pos = Some(i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let (Some(start), Some(end)) = (start_pos, end_pos) {
            text[start..end].to_string()
        } else {
            text.to_string()
        }
    }

    /// 从行中提取JSON（去掉非JSON行）
    fn extract_json_from_lines(&self, text: &str) -> String {
        let lines: Vec<&str> = text
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                // 保留可能是JSON的行
                trimmed.starts_with('{')
                    || trimmed.starts_with('}')
                    || trimmed.starts_with('"')
                    || trimmed.contains(':')
                    || trimmed.contains('[')
                    || trimmed.contains(']')
                    || trimmed.is_empty()
            })
            .collect();

        lines.join("\n")
    }

    /// 清理和验证JSON
    fn clean_and_validate_json(&self, json_str: &str) -> Option<String> {
        let cleaned = json_str
            .trim()
            .replace("\u{0000}", "") // 移除null字符
            .replace("\\n", "\n") // 修复转义的换行符
            .replace("\\'", "'"); // 修复转义的单引号

        // 尝试解析JSON来验证
        match serde_json::from_str::<serde_json::Value>(&cleaned) {
            Ok(_) => Some(cleaned),
            Err(_) => None,
        }
    }

    /// 修复常见的JSON错误
    fn repair_json(&self, text: &str) -> Option<String> {
        let mut repaired = text.to_string();

        // 1. 修复缺失的引号
        repaired = self.fix_missing_quotes(&repaired);

        // 2. 修复尾随逗号
        repaired = self.fix_trailing_commas(&repaired);

        // 3. 修复单引号
        repaired = repaired.replace("'", "\"");

        // 4. 确保大括号平衡
        repaired = self.balance_braces(&repaired);

        // 验证修复后的JSON
        self.clean_and_validate_json(&repaired)
    }

    /// 修复缺失的引号
    fn fix_missing_quotes(&self, text: &str) -> String {
        let mut result = String::new();
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch.is_alphabetic() || ch == '_' {
                // 可能是属性名，检查后面是否跟着冒号
                let mut word = String::new();
                word.push(ch);

                // 收集完整的单词
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphanumeric() || next_ch == '_' {
                        word.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                // 检查是否跟着冒号（可能有空格）
                let mut has_colon = false;
                let mut spaces = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_whitespace() {
                        spaces.push(chars.next().unwrap());
                    } else if next_ch == ':' {
                        has_colon = true;
                        break;
                    } else {
                        break;
                    }
                }

                if has_colon {
                    // 添加引号
                    result.push('"');
                    result.push_str(&word);
                    result.push('"');
                } else {
                    result.push_str(&word);
                }
                result.push_str(&spaces);
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// 修复尾随逗号
    fn fix_trailing_commas(&self, text: &str) -> String {
        text.replace(",}", "}")
            .replace(",]", "]")
            .replace(",\n}", "}")
            .replace(",\n]", "]")
    }

    /// 平衡大括号
    fn balance_braces(&self, text: &str) -> String {
        let mut brace_count = 0;
        let mut result = text.to_string();

        for ch in text.chars() {
            match ch {
                '{' => brace_count += 1,
                '}' => brace_count -= 1,
                _ => {}
            }
        }

        // 如果缺少右大括号，添加它们
        while brace_count > 0 {
            result.push('}');
            brace_count -= 1;
        }

        result
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
                    return Err(anyhow::anyhow!(
                        "任务 {} 依赖的任务 {} 不存在",
                        task_id,
                        dep
                    ));
                }
            }
        }

        info!(
            "DAG计划验证通过: {} 个任务, {} 个依赖关系",
            nodes.len(),
            dependency_graph.len()
        );
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

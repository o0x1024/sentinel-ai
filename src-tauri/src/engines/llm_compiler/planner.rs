//! LLMCompiler Planner组件
//!
//! 负责生成DAG（有向无环图）任务计划的智能规划器
//! 核心特性：
//! - 流式生成任务（边生成边执行）
//! - 识别任务间的依赖关系
//! - 支持变量替换（$1, $2等）
//! - 可重新规划
//! - Memory-enhanced replanning

use sentinel_rag::models::AssistantRagRequest;
use sentinel_rag::query_utils::build_rag_query_pair;

use anyhow::Result;
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::memory_integration::LlmCompilerMemoryIntegration;
use super::types::*;
use crate::engines::LlmClient;
use crate::services::ai::AiService;
use crate::services::prompt_db::PromptRepository;
use crate::utils::prompt_resolver::{AgentPromptConfig, CanonicalStage, PromptResolver};

/// LLMCompiler Planner - 智能任务规划器
pub struct LlmCompilerPlanner {
    llm_client: Arc<LlmClient>,
    tool_adapter: Arc<dyn crate::tools::EngineToolAdapter>,
    #[allow(unused)]
    config: LlmCompilerConfig,
    prompt_repo: Option<PromptRepository>,
    runtime_params: Option<HashMap<String, Value>>,
    memory_integration: Option<Arc<LlmCompilerMemoryIntegration>>,
}

impl LlmCompilerPlanner {
    pub fn new(
        ai_service: Arc<AiService>,
        tool_adapter: Arc<dyn crate::tools::EngineToolAdapter>,
        config: LlmCompilerConfig,
        prompt_repo: Option<PromptRepository>,
    ) -> Self {
        let llm_client = Arc::new(crate::engines::create_client(ai_service.as_ref()));
        Self {
            llm_client,
            tool_adapter,
            config,
            prompt_repo,
            runtime_params: None,
            memory_integration: None,
        }
    }

    /// Create with memory integration
    pub fn new_with_memory(
        ai_service: Arc<AiService>,
        tool_adapter: Arc<dyn crate::tools::EngineToolAdapter>,
        config: LlmCompilerConfig,
        prompt_repo: Option<PromptRepository>,
        memory_integration: Option<Arc<LlmCompilerMemoryIntegration>>,
    ) -> Self {
        let llm_client = Arc::new(crate::engines::create_client(ai_service.as_ref()));
        Self {
            llm_client,
            tool_adapter,
            config,
            prompt_repo,
            runtime_params: None,
            memory_integration,
        }
    }

    /// Set memory integration
    pub fn set_memory_integration(&mut self, memory: Arc<LlmCompilerMemoryIntegration>) {
        self.memory_integration = Some(memory);
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

        // ✅ 获取可用工具列表（从context读取工具权限配置）
        let available_tools = self.get_available_tools_description(context).await?;

        // ✅ 拆分为system_prompt和user_prompt
        let (system_prompt, user_prompt) = self
            .build_planning_prompts(user_input, context, &available_tools)
            .await?;

        // 使用公共 llm_client 模块进行 LLM 调用
        let response = match self
            .llm_client
            .completion(Some(&system_prompt), &user_prompt)
            .await
        {
            Ok(resp) => {
                debug!("LLMCompiler Planner: LLM response length {} chars", resp.len());
                resp
            }
            Err(e) => {
                error!("LLMCompiler Planner: LLM call failed: {}", e);
                return Err(anyhow::anyhow!(
                    "Failed to generate DAG plan: LLM call failed - {}. Please check your AI service configuration.",
                    e
                ));
            }
        };

        // 解析LLM响应为DAG计划
        self.parse_dag_plan(&response, user_input).await
    }

    /// 获取可用工具描述（应用工具权限过滤）
    async fn get_available_tools_description(
        &self,
        context: &HashMap<String, Value>,
    ) -> Result<String> {
        use std::collections::HashSet;

        // ✅ 优先从context读取工具白名单/黑名单，其次从runtime_params读取
        let params = if !context.is_empty() {
            context
        } else if let Some(ref rp) = self.runtime_params {
            rp
        } else {
            context // 空HashMap
        };

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

        info!("LLMCompiler Planner: 工具过滤配置 - allow_present: {}, allow_count: {}, deny_count: {}", 
            allow_present, allow.len(), deny.len());
        if allow_present {
            info!("LLMCompiler Planner: 允许的工具: {:?}", allow);
        }

        // 如果显式传入空白名单，表示禁用所有工具
        if allow_present && allow.is_empty() {
            info!("LLMCompiler Planner: 检测到显式空白名单 => 禁用所有工具");
            return Ok("暂无可用工具".to_string());
        }

        let all_tools = self.tool_adapter.list_available_tools().await;

        // 应用工具过滤
        let filtered_tools: Vec<String> = all_tools
            .into_iter()
            .filter(|tool_name| {
                // 如果有白名单（非空），只允许白名单中的工具
                if allow_present && !allow.is_empty() {
                    if !allow.contains(tool_name) {
                        debug!(
                            "LLMCompiler Planner: 工具 '{}' 不在白名单中，已过滤",
                            tool_name
                        );
                        return false;
                    }
                }
                // 如果在黑名单中，拒绝
                if deny.contains(tool_name) {
                    debug!(
                        "LLMCompiler Planner: 工具 '{}' 在黑名单中，已过滤",
                        tool_name
                    );
                    return false;
                }
                true
            })
            .collect();

        if filtered_tools.is_empty() {
            return Ok("暂无可用工具".to_string());
        }

        info!(
            "LLMCompiler Planner: 过滤后可用工具数量: {}/{}",
            filtered_tools.len(),
            filtered_tools.len()
        );

        let mut tool_descriptions = Vec::new();
        for tool_name in &filtered_tools {
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

    /// 过滤用户上下文，移除系统内部参数
    fn filter_user_context(&self, context: &HashMap<String, Value>) -> HashMap<String, Value> {
        // 定义需要过滤掉的系统内部参数
        let system_params = vec![
            "conversation_id",
            "execution_id",
            "message_id",
            "prompt_ids",
            "prompts",
            "execution_retry_max",
            "execution_retry_backoff",
            "execution_timeout_sec",
            "prompt_strategy",
            "group_id",
            "agent_id",
            "task_mode",
            "pinned_versions",
            "llm",
            "tools_allow",
            "tools_deny",
            "task_description", // 这个已经是user_input了，不需要重复
        ];

        context
            .iter()
            .filter(|(key, _)| !system_params.contains(&key.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// 构建规划提示（拆分为system_prompt和user_prompt）
    async fn build_planning_prompts(
        &self,
        user_input: &str,
        context: &HashMap<String, Value>,
        available_tools: &str,
    ) -> Result<(String, String)> {
        use crate::models::prompt::ArchitectureType;
        use crate::utils::prompt_resolver::{AgentPromptConfig, CanonicalStage, PromptResolver};

        // 优先检查 runtime_params 或 context 中是否有来自 Orchestrator 的自定义 prompt
        let system_template = if let Some(params) = &self.runtime_params {
            if let Some(custom_prompt) = params.get("custom_system_prompt").and_then(|v| v.as_str())
            {
                info!("LLMCompiler Planner: Using custom system prompt from runtime_params (Orchestrator)");
                custom_prompt.to_string()
            } else if let Some(repo) = &self.prompt_repo {
                // ✅ 从数据库读取 system 模板（优先使用配置的模板）
                let resolver = PromptResolver::new(repo.clone());
                let agent_config = AgentPromptConfig::parse_agent_config(context);
                resolver
                    .resolve_prompt(
                        &agent_config,
                        ArchitectureType::LLMCompiler,
                        CanonicalStage::Planner,
                        Some(&"".to_string()),
                    )
                    .await
                    .unwrap_or_else(|_| {
                        info!("LLMCompiler Planner: Using prompt from database");
                        self.get_default_planner_template()
                    })
            } else {
                self.get_default_planner_template()
            }
        } else if let Some(repo) = &self.prompt_repo {
            // ✅ 从数据库读取 system 模板（优先使用配置的模板）
            let resolver = PromptResolver::new(repo.clone());
            let agent_config = AgentPromptConfig::parse_agent_config(context);
            resolver
                .resolve_prompt(
                    &agent_config,
                    ArchitectureType::LLMCompiler,
                    CanonicalStage::Planner,
                    Some(&"".to_string()),
                )
                .await
                .unwrap_or_else(|_| {
                    info!("LLMCompiler Planner: Using prompt from database");
                    self.get_default_planner_template()
                })
        } else {
            self.get_default_planner_template()
        };

        // 渲染工具变量到 system 提示
        let mut system_ctx = HashMap::new();
        system_ctx.insert(
            "tools".to_string(),
            serde_json::Value::String(available_tools.to_string()),
        );

        let mut system_prompt = if let Some(repo) = &self.prompt_repo {
            let resolver = PromptResolver::new(repo.clone());
            resolver
                .render_variables(&system_template, &system_ctx)
                .unwrap_or_else(|_| system_template.replace("{tools}", available_tools))
        } else {
            system_template.replace("{tools}", available_tools)
        };

        // ✅ user_prompt: 用户的实际任务（不包含系统内部参数）
        // 过滤掉内部系统参数，只保留用户业务相关的上下文
        let filtered_context = self.filter_user_context(context);

        let user_prompt = if filtered_context.is_empty() {
            format!("请为以下任务生成DAG执行计划：\n\n{}", user_input)
        } else {
            format!(
                "请为以下任务生成DAG执行计划：\n\n**任务**: {}\n\n**业务上下文**: {}",
                user_input,
                serde_json::to_string_pretty(&filtered_context).unwrap_or_default()
            )
        };

        // 集成角色提示词（如果存在）
        if let Some(role_prompt) = context.get("role_prompt").and_then(|v| v.as_str()) {
            if !role_prompt.trim().is_empty() {
                system_prompt = if system_prompt.trim().is_empty() {
                    role_prompt.to_string()
                } else {
                    format!("{}\n\n{}", role_prompt, system_prompt)
                };
                log::info!("LLM-Compiler planner: integrated role prompt");
            }
        }

        // RAG augmentation for LLMCompiler planning (global toggle)
        if let Ok(rag_service) = crate::commands::rag_commands::get_global_rag_service().await {
            if rag_service.get_config().augmentation_enabled {
                use tokio::time::{timeout, Duration};
                let (primary, fallback) = build_rag_query_pair(&format!(
                    "{} {}",
                    user_input,
                    serde_json::to_string_pretty(context).unwrap_or_default()
                ));
                let rag_request = AssistantRagRequest {
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
                        system_prompt.push_str("\n\n[KNOWLEDGE CONTEXT]\n");
                        system_prompt.push_str(&knowledge_context);
                        info!("LLM-Compiler planner: Added RAG context to system prompt");
                    }
                }
            }
        }

        // ✅ 返回(system_prompt, user_prompt)元组
        Ok((system_prompt, user_prompt))
    }

    /// 获取默认的规划器模板（作为后备）
    fn get_default_planner_template(&self) -> String {
        r#"你是一个专业的LLMCompiler规划器，专门设计并行执行的DAG任务计划。

**规划要求**：
1. 生成可并行执行的任务DAG
2. 最小化任务间的依赖关系
3. 使用变量引用（$1, $2等）处理任务间数据传递
4. 优化执行效率和资源利用

**可用工具**:
{tools}

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

**输出格式要求**：
请以JSON格式返回DAG计划，包含：
- nodes: 任务节点列表
- dependency_graph: 依赖关系
- variable_mappings: 变量映射
- 不要使用任何markdown标记，请直接返回可解析的JSON

示例格式：
{
  "nodes": [
    {
      "id": "task_1",
      "name": "基础信息收集",
      "description": "收集目标的基本信息",
      "tool_name": "dns_scanner",
      "inputs": {"target": "example.com"},
      "dependencies": [],
      "variable_refs": [],
      "priority": 1,
      "estimated_duration": 30
    },
    {
      "id": "task_2",
      "name": "端口扫描",
      "description": "基于IP地址进行端口扫描",
      "tool_name": "port_scanner",
      "inputs": {"target": "$1", "scan_type": "tcp"},
      "dependencies": ["task_1"],
      "variable_refs": ["$1"],
      "priority": 2,
      "estimated_duration": 60
    }
  ],
  "dependency_graph": {"task_2": ["task_1"]},
  "variable_mappings": {"$1": "task_1.outputs.ip_address"}
}"#
        .to_string()
    }

    fn apply_placeholders(template: &str, pairs: Vec<(&str, &str)>) -> String {
        let mut out = template.to_string();
        for (k, v) in pairs {
            out = out.replace(k, v);
        }
        out
    }

    /// 重新规划 (with Memory-enhanced failure pattern analysis)
    pub async fn replan(
        &self,
        original_plan: &DagExecutionPlan,
        execution_results: &[TaskExecutionResult],
        feedback: &str,
    ) -> Result<DagExecutionPlan> {
        info!("Starting replanning with feedback: {}", feedback);

        // Query failure patterns from memory
        let failure_patterns = self.get_failure_patterns_from_memory(execution_results).await;

        // Build system_prompt and user_prompt with failure patterns
        let (mut system_prompt, user_prompt) =
            self.build_replanning_prompts(original_plan, execution_results, feedback);

        // Augment system prompt with failure patterns from memory
        if !failure_patterns.is_empty() {
            system_prompt.push_str("\n\n[HISTORICAL FAILURE PATTERNS]\n");
            system_prompt.push_str("Based on past executions, these patterns caused failures:\n");
            for pattern in &failure_patterns {
                system_prompt.push_str(&format!("- {}\n", pattern));
            }
            system_prompt.push_str("\nAvoid repeating these patterns in the new plan.\n");
        }

        // 使用公共 llm_client 模块进行 LLM 调用
        let response = self
            .llm_client
            .completion(Some(&system_prompt), &user_prompt)
            .await?;

        self.parse_dag_plan(&response, &format!("Replan-{}", original_plan.name))
            .await
    }

    /// Get failure patterns from memory for similar tasks
    async fn get_failure_patterns_from_memory(
        &self,
        execution_results: &[TaskExecutionResult],
    ) -> Vec<String> {
        let mut patterns = Vec::new();

        let Some(ref memory) = self.memory_integration else {
            return patterns;
        };

        // Extract error patterns from current execution
        for result in execution_results {
            if result.status == TaskStatus::Failed {
                if let Some(ref error) = result.error {
                    // Query memory for similar failures
                    if let Ok(similar_failures) = memory
                        .retrieve_failure_patterns(&result.task_id, error)
                        .await
                    {
                        for failure in similar_failures.iter().take(3) {
                            if failure.similarity_score > 0.6 {
                                // Extract failure info from failure_info field
                                let failure_desc = failure
                                    .item
                                    .failure_info
                                    .as_ref()
                                    .and_then(|f| f.get("error").and_then(|e| e.as_str()))
                                    .unwrap_or("No details");

                                let pattern = format!(
                                    "Task type '{}' failed with similar error (similarity: {:.2}): {}",
                                    failure.item.task_type, failure.similarity_score, failure_desc
                                );
                                if !patterns.contains(&pattern) {
                                    patterns.push(pattern);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Get tool effectiveness recommendations
        for result in execution_results {
            if result.status == TaskStatus::Failed {
                // Try to get tool effectiveness from outputs
                if let Some(tool_name) = result.outputs.get("tool_name").and_then(|v| v.as_str())
                {
                    if let Ok(effectiveness) =
                        memory.get_tool_effectiveness(tool_name, None, None).await
                    {
                        if effectiveness < 0.5 {
                            patterns.push(format!(
                                "Tool '{}' has low effectiveness ({:.1}%), consider alternatives",
                                tool_name,
                                effectiveness * 100.0
                            ));
                        }
                    }
                }
            }
        }

        patterns
    }

    /// 构建重规划提示（拆分为system_prompt和user_prompt）
    fn build_replanning_prompts(
        &self,
        original_plan: &DagExecutionPlan,
        results: &[TaskExecutionResult],
        feedback: &str,
    ) -> (String, String) {
        // 分析已完成的任务和结果
        let completed_tasks: Vec<_> = results
            .iter()
            .filter(|r| r.status == TaskStatus::Completed)
            .collect();

        let failed_tasks: Vec<_> = results
            .iter()
            .filter(|r| r.status == TaskStatus::Failed)
            .collect();

        // ✅ system_prompt: 重规划指导原则（固定内容）
        let system_prompt = r#"你是一个专业的LLMCompiler重规划专家。请根据执行反馈重新制定计划。

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

**输出要求**：
请生成新的DAG计划，使用JSON格式（不要使用markdown标记）。
新计划应该：
1. 包含针对发现问题的深入测试
2. 利用已有的执行结果
3. 保持高效的并行执行结构
"#
        .to_string();

        // ✅ user_prompt: 具体的重规划请求和上下文
        let user_prompt = format!(
            r#"请基于以下信息重新生成DAG执行计划：

**原始计划**: {}

**执行统计**:
- 已完成任务: {} 个
- 失败任务: {} 个

**执行结果详情**: 
{}

**反馈信息**: 
{}

请生成优化后的新计划。
"#,
            serde_json::to_string_pretty(original_plan).unwrap_or_default(),
            completed_tasks.len(),
            failed_tasks.len(),
            serde_json::to_string_pretty(results).unwrap_or_default(),
            feedback
        );

        (system_prompt, user_prompt)
    }

    /// 解析DAG计划
    async fn parse_dag_plan(&self, response: &str, plan_name: &str) -> Result<DagExecutionPlan> {
        debug!("解析LLM响应: {}", response);

        // 尝试提取JSON部分（LLM可能返回带解释的文本）
        let json_str = self.extract_json_from_response(response)?;

        // 解析JSON响应
        let parsed: Value = serde_json::from_str(&json_str)
            .map_err(|e| anyhow::anyhow!("解析LLM响应失败: {}", e))?;

        // ✅ 同时支持 "nodes" 和 "tasks" 字段（拟人化prompt使用tasks）
        let nodes = parsed["nodes"]
            .as_array()
            .or_else(|| parsed["tasks"].as_array())
            .ok_or_else(|| anyhow::anyhow!("缺少nodes或tasks字段"))?
            .iter()
            .enumerate()
            .map(|(index, node)| {
                Ok(DagTaskNode {
                    id: node["task_id"]
                        .as_str()
                        .or_else(|| node["id"].as_str())
                        .unwrap_or(&format!("task_{}", index + 1))
                        .to_string(),
                    name: node["name"].as_str().unwrap_or("未命名任务").to_string(),
                    description: node["description"].as_str().unwrap_or("").to_string(),
                    // ✅ 同时支持 "tool_name" 和 "tool" 字段（拟人化prompt使用tool）
                    tool_name: node["tool_name"]
                        .as_str()
                        .or_else(|| node["tool"].as_str())
                        .ok_or_else(|| anyhow::anyhow!("任务缺少tool_name或tool字段"))?
                        .to_string(),
                    // ✅ 同时支持 "arguments" 和 "inputs" 字段（优先 arguments）
                    inputs: node["arguments"]
                        .as_object()
                        .or_else(|| node["inputs"].as_object())
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

    /// 从响应中提取JSON（增强的鲁棒性，支持拟人化格式）
    fn extract_json_from_response(&self, response: &str) -> Result<String> {
        let trimmed = response.trim();

        // ✅ 首先尝试从拟人化格式中提取 [PLAN] 部分
        if let Some(plan_json) = self.extract_plan_from_humanized_response(trimmed) {
            debug!("从拟人化响应中提取到PLAN部分");
            return Ok(plan_json);
        }

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
                // 安全截断，确保不在 UTF-8 字符中间切断
                let mut end = 200;
                while end > 0 && !trimmed.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &trimmed[..end])
            } else {
                trimmed.to_string()
            }
        ))
    }

    /// 从拟人化响应中提取[PLAN]部分的JSON
    fn extract_plan_from_humanized_response(&self, response: &str) -> Option<String> {
        // 查找 [PLAN] 标记
        if let Some(plan_start) = response.find("[PLAN]") {
            let content_after_plan = &response[plan_start + 6..]; // 跳过 "[PLAN]"

            // 提取 PLAN 后的JSON内容（可能在代码块中或直接是JSON）
            let json_candidates = vec![
                self.extract_json_from_code_blocks(content_after_plan),
                self.extract_json_from_braces(content_after_plan),
            ];

            for candidate in json_candidates {
                if let Some(cleaned) = self.clean_and_validate_json(&candidate) {
                    return Some(cleaned);
                }
            }
        }

        None
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

//! ReWOO Solver 实现
//! 
//! 基于 LangGraph ReWOO 标准实现的 Solver 模块
//! 负责综合所有工具执行结果并生成最终答案
use super::*;
use crate::ai_adapter::types::{ChatRequest, Message, MessageRole};
use crate::ai_adapter::{AiProvider, ChatOptions};
use std::sync::Arc;
use crate::services::prompt_db::PromptRepository;
use crate::models::prompt::ArchitectureType;
use crate::utils::prompt_resolver::{PromptResolver, CanonicalStage, AgentPromptConfig};

/// ReWOO Solver - 负责综合结果并生成最终答案
pub struct ReWOOSolver {
    /// AI 提供商
    ai_provider: Arc<dyn AiProvider>,
    /// 配置
    config: SolverConfig,
    prompt_repo: Option<PromptRepository>,
    runtime_params: Option<std::collections::HashMap<String, serde_json::Value>>, // 用于 prompt_ids 覆盖
}

impl ReWOOSolver {
    /// 创建新的 Solver
    pub fn new(
        ai_provider: Arc<dyn AiProvider>,
        config: SolverConfig,
        prompt_repo: Option<PromptRepository>,
    ) -> Self {
        Self {
            ai_provider,
            config,
            prompt_repo,
            runtime_params: None,
        }
    }
    
    pub fn set_runtime_params(&mut self, params: std::collections::HashMap<String, serde_json::Value>) {
        self.runtime_params = Some(params);
    }
    
    /// 生成最终答案
    pub async fn solve(&self, state: &ReWOOState) -> Result<String, ReWOOError> {
        // 验证所有步骤是否已完成
        if !self.all_steps_completed(state) {
            return Err(ReWOOError::SolvingError(
                "Cannot solve: not all steps are completed".to_string()
            ));
        }
        
        // 构建求解提示
        let prompt = self.build_solving_prompt(state).await?;
        
        // 调用 AI 生成最终答案
        let answer = self.generate_answer(&prompt).await?;
        
        Ok(answer)
    }
    
    /// 构建求解提示 - 支持任务域路由
    async fn build_solving_prompt(&self, state: &ReWOOState) -> Result<String, ReWOOError> {
        // 首先推断任务域
        let used_tools: Vec<String> = state.steps.iter()
            .filter_map(|step| {
                // 从步骤中提取工具名称
                if let Some(start) = step.find(" = ") {
                    if let Some(end) = step[start + 3..].find('[') {
                        return Some(step[start + 3..start + 3 + end].to_string());
                    }
                }
                None
            })
            .collect();
            
        let task_domain = TaskDomain::infer_from_task(&state.task, &used_tools);
        log::info!("Inferred task domain: {:?} for task: {}", task_domain, state.task);
        
        // 根据任务域选择模板
        self.build_domain_specific_prompt(state, &task_domain).await
    }
    
    /// 构建领域特定的求解提示
    async fn build_domain_specific_prompt(&self, state: &ReWOOState, domain: &TaskDomain) -> Result<String, ReWOOError> {
        // 构建通用数据
        let execution_plan = state.steps.iter()
            .enumerate()
            .map(|(i, step)| format!("{}. {}", i + 1, step))
            .collect::<Vec<_>>()
            .join("\n");

        let execution_results = state.results.iter()
            .map(|(variable, result)| {
                let result_str = match result {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string())
                    }
                    _ => result.to_string(),
                };
                format!("**{}**: {}", variable, result_str)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let total_steps = state.steps.len();
        let completed_steps = state.results.len();
        let success_rate = if total_steps > 0 {
            (completed_steps as f32 / total_steps as f32 * 100.0).round()
        } else {
            0.0
        };
        let execution_stats = format!("总步骤数: {}, 已完成: {}, 成功率: {}%", total_steps, completed_steps, success_rate);

        // 根据任务域选择模板
        let domain_template = match domain {
            TaskDomain::CyberSecurity => self.build_security_template(state, &execution_plan, &execution_results, &execution_stats),
            TaskDomain::DataAnalysis => self.build_data_analysis_template(state, &execution_plan, &execution_results, &execution_stats),
            TaskDomain::QuestionAnswering => self.build_qa_template(state, &execution_plan, &execution_results, &execution_stats),
            TaskDomain::ApiIntegration => self.build_api_template(state, &execution_plan, &execution_results, &execution_stats),
            TaskDomain::InformationRetrieval => self.build_retrieval_template(state, &execution_plan, &execution_results, &execution_stats),
            TaskDomain::ContentGeneration => self.build_content_template(state, &execution_plan, &execution_results, &execution_stats),
            TaskDomain::General => self.build_general_template(state, &execution_plan, &execution_results, &execution_stats),
        };

        // 统一提示词解析器：优先覆盖并支持用户配置
        if let Some(repo) = &self.prompt_repo {
            let resolver = PromptResolver::new(repo.clone());
            let empty: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
            let params_ref = self.runtime_params.as_ref().unwrap_or(&empty);
            let agent_config = AgentPromptConfig::parse_agent_config(params_ref);

            let template = resolver
                .resolve_prompt(
                    &agent_config,
                    ArchitectureType::ReWOO,
                    CanonicalStage::Evaluator,
                    Some(&domain_template),
                )
                .await
                .unwrap_or(domain_template.clone());

            let replaced = Self::apply_placeholders(&template, vec![
                ("{{TASK}}", &state.task), ("{task}", &state.task),
                ("{{EXECUTION_PLAN}}", &execution_plan), ("{execution_plan}", &execution_plan),
                ("{{EXECUTION_RESULTS}}", &execution_results), ("{execution_results}", &execution_results),
                ("{{EXECUTION_STATS}}", &execution_stats), ("{execution_stats}", &execution_stats),
            ]);
            Ok(replaced)
        } else {
            Ok(domain_template)
        }
    }

    /// 应用占位符替换（参考planner的实现）
    fn apply_placeholders(template: &str, placeholders: Vec<(&str, &str)>) -> String {
        let mut result = template.to_string();
        for (placeholder, value) in placeholders {
            result = result.replace(placeholder, value);
        }
        result
    }
    
    /// 生成最终答案
    async fn generate_answer(&self, prompt: &str) -> Result<String, ReWOOError> {
        let request = ChatRequest {
            model: self.config.model_name.clone(),
            messages: vec![Message {
                role: MessageRole::System,
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
            .map_err(|e| ReWOOError::SolvingError(format!("Failed to generate answer: {}", e)))?;
        
        if let Some(choice) = response.choices.first() {
            return Ok(choice.message.content.clone());
        }
        
        Err(ReWOOError::SolvingError("No valid response from AI provider".to_string()))
    }
    
    /// 检查所有步骤是否已完成
    fn all_steps_completed(&self, state: &ReWOOState) -> bool {
        for step in &state.steps {
            // 从步骤中提取变量名
            if let Some(variable) = self.extract_variable_from_step(step) {
                if !state.results.contains_key(&variable) {
                    return false;
                }
            }
        }
        true
    }
    
    /// 从步骤中提取变量名
    fn extract_variable_from_step(&self, step: &str) -> Option<String> {
        // 使用正则表达式提取变量名 (#E1, #E2, etc.)
        use regex::Regex;
        let regex = Regex::new(r"#E(\d+)").ok()?;
        
        if let Some(captures) = regex.captures(step) {
            return Some(format!("#E{}", &captures[1]));
        }
        
        None
    }
    
    /// 生成结构化答案
    pub async fn solve_structured(&self, state: &ReWOOState) -> Result<StructuredAnswer, ReWOOError> {
        let answer = self.solve(state).await?;
        
        // 分析答案内容
        let summary = self.extract_summary(&answer);
        let key_findings = self.extract_key_findings(&answer);
        let recommendations = self.extract_recommendations(&answer);
        
        Ok(StructuredAnswer {
            summary,
            key_findings,
            recommendations,
            full_answer: answer,
            confidence_score: self.calculate_confidence_score(state),
            evidence_sources: self.collect_evidence_sources(state),
        })
    }
    
    /// 提取摘要
    fn extract_summary(&self, answer: &str) -> String {
        // 简化实现：取前200个字符作为摘要
        if answer.len() > 200 {
            format!("{}...", &answer[..200])
        } else {
            answer.to_string()
        }
    }
    
    /// 提取关键发现
    fn extract_key_findings(&self, answer: &str) -> Vec<String> {
        // 简化实现：按段落分割
        answer.split("\n\n")
            .filter(|s| !s.trim().is_empty())
            .take(5) // 最多5个关键发现
            .map(|s| s.trim().to_string())
            .collect()
    }
    
    /// 提取建议
    fn extract_recommendations(&self, answer: &str) -> Vec<String> {
        // 简化实现：查找包含建议关键词的句子
        let recommendation_keywords = ["建议", "推荐", "应该", "需要", "recommend", "suggest", "should"];
        
        answer.split('.')
            .filter(|sentence| {
                let sentence_lower = sentence.to_lowercase();
                recommendation_keywords.iter().any(|keyword| sentence_lower.contains(keyword))
            })
            .take(3) // 最多3个建议
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
    
    /// 计算置信度分数
    fn calculate_confidence_score(&self, state: &ReWOOState) -> f32 {
        let total_steps = state.steps.len() as f32;
        let successful_steps = state.results.len() as f32;
        
        if total_steps == 0.0 {
            return 0.0;
        }
        
        // 基础分数：完成步骤的比例
        let completion_score = successful_steps / total_steps;
        
        // 考虑执行质量（这里简化为固定值）
        let quality_score = 0.8;
        
        (completion_score * quality_score * 100.0).min(100.0)
    }
    
    /// 收集证据来源
    fn collect_evidence_sources(&self, state: &ReWOOState) -> Vec<String> {
        let mut sources = Vec::new();
        
        for (variable, _result) in &state.results {
            // 从变量名推断工具类型
            if let Some(step) = state.steps.iter().find(|s| s.contains(variable)) {
                if let Some(tool_name) = self.extract_tool_name_from_step(step) {
                    sources.push(format!("{} ({})", tool_name, variable));
                }
            }
        }
        
        sources
    }
    
    /// 从步骤中提取工具名称
    fn extract_tool_name_from_step(&self, step: &str) -> Option<String> {
        use regex::Regex;
        let regex = Regex::new(r"#E\d+\s*=\s*(\w+)\[").ok()?;
        
        if let Some(captures) = regex.captures(step) {
            return Some(captures[1].to_string());
        }
        
        None
    }
    
    /// 验证求解条件
    pub fn validate_solve_conditions(&self, state: &ReWOOState) -> Result<(), ReWOOError> {
        // 检查是否有步骤
        if state.steps.is_empty() {
            return Err(ReWOOError::SolvingError(
                "No steps to solve".to_string()
            ));
        }
        
        // 检查是否有结果
        if state.results.is_empty() {
            return Err(ReWOOError::SolvingError(
                "No execution results available".to_string()
            ));
        }
        
        // 检查任务是否为空
        if state.task.trim().is_empty() {
            return Err(ReWOOError::SolvingError(
                "Empty task description".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// 构建网络安全模板
    fn build_security_template(&self, state: &ReWOOState, execution_plan: &str, execution_results: &str, execution_stats: &str) -> String {
        format!(r#"你是一名资深的网络安全分析师和报告撰写专家，负责综合分析所有安全测试结果并生成专业的安全评估报告。

**分析职责**:
1. 整合所有工具扫描结果
2. 识别关联性攻击路径
3. 评估整体安全风险
4. 提供专业的修复建议
5. 生成符合行业标准的安全报告

**原始任务**: {task}
**执行计划**: {execution_plan}
**工具执行结果**: {execution_results}
**执行统计**: {execution_stats}

请基于以上测试结果生成综合安全评估报告。"#, 
            task = state.task,
            execution_plan = execution_plan,
            execution_results = execution_results,
            execution_stats = execution_stats
        )
    }
    
    /// 构建数据分析模板
    fn build_data_analysis_template(&self, state: &ReWOOState, execution_plan: &str, execution_results: &str, execution_stats: &str) -> String {
        format!(r#"你是一名专业的数据分析师，擅长从复杂数据中提取洞察并生成清晰的分析报告。

**分析任务**:
1. 汇总和整理收集到的数据
2. 识别数据模式和趋势
3. 计算关键指标和统计数据
4. 生成可视化建议
5. 提供数据驱动的结论

**原始任务**: {task}
**执行计划**: {execution_plan}
**数据收集结果**: {execution_results}
**执行统计**: {execution_stats}

请基于以上数据生成分析报告，包括：
- 数据摘要
- 关键发现
- 趋势分析
- 建议行动

请开始分析："#,
            task = state.task,
            execution_plan = execution_plan,
            execution_results = execution_results,
            execution_stats = execution_stats
        )
    }
    
    /// 构建问答模板
    fn build_qa_template(&self, state: &ReWOOState, execution_plan: &str, execution_results: &str, execution_stats: &str) -> String {
        format!(r#"你是一个知识渊博的助手，根据执行的查询步骤为用户提供准确、全面的答案。

**任务**: {task}

**我执行了以下步骤来获取信息**:
{execution_plan}

**获得的信息**:
{execution_results}

**执行情况**: {execution_stats}

请基于以上收集的信息，为原始问题提供一个清晰、准确、有用的答案。如果信息不足以完全回答问题，请明确指出限制并提供可能的建议。"#,
            task = state.task,
            execution_plan = execution_plan,
            execution_results = execution_results,
            execution_stats = execution_stats
        )
    }
    
    /// 构建 API 集成模板
    fn build_api_template(&self, state: &ReWOOState, execution_plan: &str, execution_results: &str, execution_stats: &str) -> String {
        format!(r#"你是一名 API 集成专家，帮助分析和总结 API 调用结果。

**集成任务**: {task}
**执行步骤**: {execution_plan}
**API 响应结果**: {execution_results}
**执行统计**: {execution_stats}

请生成 API 集成总结报告，包括：
- 成功的 API 调用概述
- 返回数据的格式和内容分析
- 错误处理情况
- 集成建议和最佳实践"#,
            task = state.task,
            execution_plan = execution_plan,
            execution_results = execution_results,
            execution_stats = execution_stats
        )
    }
    
    /// 构建信息检索模板
    fn build_retrieval_template(&self, state: &ReWOOState, execution_plan: &str, execution_results: &str, execution_stats: &str) -> String {
        format!(r#"你是一名信息检索专家，帮助整理和总结搜索到的信息。

**检索任务**: {task}
**搜索策略**: {execution_plan}
**检索结果**: {execution_results}
**检索统计**: {execution_stats}

请整理检索到的信息，生成结构化的总结报告：
- 信息来源概述
- 关键信息提取
- 相关性评估
- 推荐的后续查询方向"#,
            task = state.task,
            execution_plan = execution_plan,
            execution_results = execution_results,
            execution_stats = execution_stats
        )
    }
    
    /// 构建内容生成模板
    fn build_content_template(&self, state: &ReWOOState, execution_plan: &str, execution_results: &str, execution_stats: &str) -> String {
        format!(r#"你是一名内容创作专家，根据收集的素材生成高质量的内容。

**创作任务**: {task}
**素材收集过程**: {execution_plan}
**收集的素材**: {execution_results}
**收集统计**: {execution_stats}

请基于以上素材生成符合要求的内容，确保：
- 内容结构清晰
- 信息准确可靠
- 语言流畅自然
- 符合目标受众需求"#,
            task = state.task,
            execution_plan = execution_plan,
            execution_results = execution_results,
            execution_stats = execution_stats
        )
    }
    
    /// 构建通用模板
    fn build_general_template(&self, state: &ReWOOState, execution_plan: &str, execution_results: &str, execution_stats: &str) -> String {
        format!(r#"请根据执行的步骤和结果，为用户的任务提供综合性的分析和总结。

**任务**: {task}
**执行计划**: {execution_plan}
**执行结果**: {execution_results}
**执行统计**: {execution_stats}

请分析以上信息并生成：
1. 任务执行摘要
2. 关键发现和结果
3. 可能的改进建议
4. 总结性结论"#,
            task = state.task,
            execution_plan = execution_plan,
            execution_results = execution_results,
            execution_stats = execution_stats
        )
    }
}

/// 结构化答案
#[derive(Debug, Clone)]
pub struct StructuredAnswer {
    /// 答案摘要
    pub summary: String,
    /// 关键发现
    pub key_findings: Vec<String>,
    /// 建议
    pub recommendations: Vec<String>,
    /// 完整答案
    pub full_answer: String,
    /// 置信度分数 (0-100)
    pub confidence_score: f32,
    /// 证据来源
    pub evidence_sources: Vec<String>,
}

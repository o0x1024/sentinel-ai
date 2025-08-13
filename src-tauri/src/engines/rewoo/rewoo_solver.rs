//! ReWOO Solver 实现
//! 
//! 基于 LangGraph ReWOO 标准实现的 Solver 模块
//! 负责综合所有工具执行结果并生成最终答案
use super::*;
use crate::ai_adapter::types::{ChatRequest, Message, MessageRole};
use crate::ai_adapter::{AiProvider, ChatOptions};
use std::sync::Arc;
use crate::services::prompt_db::PromptRepository;
use crate::models::prompt::{ArchitectureType, StageType};

/// ReWOO Solver - 负责综合结果并生成最终答案
pub struct ReWOOSolver {
    /// AI 提供商
    ai_provider: Arc<dyn AiProvider>,
    /// 配置
    config: SolverConfig,
    prompt_repo: Option<PromptRepository>,
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
        }
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
    
    /// 构建求解提示
    async fn build_solving_prompt(&self, state: &ReWOOState) -> Result<String, ReWOOError> {
        // 构建执行计划字符串
        let execution_plan = state.steps.iter()
            .enumerate()
            .map(|(i, step)| format!("{}. {}", i + 1, step))
            .collect::<Vec<_>>()
            .join("\n");

        // 构建执行结果字符串
        let execution_results = state.results.iter()
            .map(|(variable, result)| {
                format!("**{}**: {}", variable, result)
            })
            .collect::<Vec<_>>()
            .join("\n");

        // 构建执行统计
        let total_steps = state.steps.len();
        let completed_steps = state.results.len();
        let success_rate = if total_steps > 0 {
            (completed_steps as f32 / total_steps as f32 * 100.0).round()
        } else {
            0.0
        };
        let execution_stats = format!("总步骤数: {}, 已完成: {}, 成功率: {}%", total_steps, completed_steps, success_rate);

        // 默认的网络安全solver模板
        let base_template = format!(r#"你是一名资深的网络安全分析师和报告撰写专家，负责综合分析所有安全测试结果并生成专业的安全评估报告。

**分析职责**:
1. 整合所有工具扫描结果
2. 识别关联性攻击路径
3. 评估整体安全风险
4. 提供专业的修复建议
5. 生成符合行业标准的安全报告

**分析框架**:
- **资产发现**: 总结发现的数字资产
- **攻击面分析**: 评估暴露的服务和端口
- **漏洞分析**: 按严重程度分类漏洞
- **风险评估**: 计算CVSS评分和业务影响
- **攻击路径**: 分析可能的攻击链
- **合规检查**: 对照安全标准和最佳实践

**原始任务**: {task}

**执行计划**:
{execution_plan}

**工具执行结果**:
{execution_results}

**执行统计**: {execution_stats}

请基于以上测试结果生成综合安全评估报告，格式如下：

## 安全评估报告

### 执行摘要
- 测试目标和范围
- 发现的关键风险
- 整体安全评级
- 优先修复建议

### 资产发现总结
- 发现的子域名: X个
- 开放端口服务: X个
- Web应用程序: X个
- 技术栈识别: 详细列表

### 漏洞发现详情
#### Critical级别漏洞 (数量: X)
- 漏洞名称
- 影响范围  
- 技术细节
- 利用复杂度
- 修复建议

#### High级别漏洞 (数量: X)
[类似格式]

#### Medium/Low级别漏洞
[汇总描述]

### 攻击路径分析
基于发现的漏洞，分析可能的攻击场景和路径

### 修复建议
按优先级排序的具体修复措施

### 合规性评估
对照OWASP Top 10、CIS Controls等标准的符合情况

**重要约束**:
1. 基于执行结果提供的信息进行分析
2. 如关键信息缺失，明确指出限制
3. 提供具体的技术细节和证据
4. 按风险等级对漏洞进行分类
5. 生成专业、准确的安全评估报告
6. 最大响应长度: {max_tokens} tokens

请开始生成安全评估报告:"#, 
            task = state.task,
            execution_plan = execution_plan,
            execution_results = execution_results,
            execution_stats = execution_stats,
            max_tokens = self.config.max_tokens
        );

        // 尝试获取动态prompt并应用占位符替换
        if let Some(repo) = &self.prompt_repo {
            match repo.get_active_prompt(ArchitectureType::ReWOO, StageType::Solver).await {
                Ok(Some(dynamic)) => {
                    let replaced = Self::apply_placeholders(&dynamic, vec![
                        ("{{TASK}}", &state.task),
                        ("{task}", &state.task),
                        ("{{ORIGINAL_TASK}}", &state.task),
                        ("{original_task}", &state.task),
                        ("{{EXECUTION_PLAN}}", &execution_plan),
                        ("{execution_plan}", &execution_plan),
                        ("{{EXECUTION_RESULTS}}", &execution_results),
                        ("{execution_results}", &execution_results),
                        ("{{EXECUTION_STATS}}", &execution_stats),
                        ("{execution_stats}", &execution_stats),
                        ("{{MAX_TOKENS}}", &self.config.max_tokens.to_string()),
                        ("{max_tokens}", &self.config.max_tokens.to_string()),
                        ("{{ALL_RESULTS}}", &execution_results),
                        ("{all_results}", &execution_results),
                        ("{{PLAN_STEPS}}", &execution_plan),
                        ("{plan_steps}", &execution_plan),
                    ]);
                    Ok(replaced)
                },
                _ => Ok(base_template),
            }
        } else {
            Ok(base_template)
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

//! ReWOO Solver 实现
//! 
//! 基于 LangGraph ReWOO 标准实现的 Solver 模块
//! 负责综合所有工具执行结果并生成最终答案
use super::*;
use crate::ai_adapter::types::{ChatRequest, Message, MessageContent, MessageRole};
use crate::ai_adapter::{AiProvider, ChatOptions};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

/// ReWOO Solver - 负责综合结果并生成最终答案
pub struct ReWOOSolver {
    /// AI 提供商
    ai_provider: Arc<dyn AiProvider>,
    /// 配置
    config: SolverConfig,
}

impl ReWOOSolver {
    /// 创建新的 Solver
    pub fn new(
        ai_provider: Arc<dyn AiProvider>,
        config: SolverConfig,
    ) -> Self {
        Self {
            ai_provider,
            config,
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
        let prompt = self.build_solving_prompt(state)?;
        
        // 调用 AI 生成最终答案
        let answer = self.generate_answer(&prompt).await?;
        
        Ok(answer)
    }
    
    /// 构建求解提示
    fn build_solving_prompt(&self, state: &ReWOOState) -> Result<String, ReWOOError> {
        let mut prompt = String::new();
        
        // 添加任务描述
        prompt.push_str(&format!("Original Task: {}\n\n", state.task));
        
        // 添加执行计划
        prompt.push_str("Execution Plan:\n");
        for (i, step) in state.steps.iter().enumerate() {
            prompt.push_str(&format!("{}. {}\n", i + 1, step));
        }
        prompt.push_str("\n");
        
        // 添加执行结果
        prompt.push_str("Execution Results:\n");
        for (variable, result) in &state.results {
            prompt.push_str(&format!("{}: {}\n", variable, result));
        }
        prompt.push_str("\n");
        
        // 添加求解指令
        prompt.push_str(&format!(r#"
You are a solver for the ReWOO (Reasoning without Observation) framework.
Your job is to synthesize the execution results above to provide a comprehensive answer to the original task.

IMPORTANT GUIDELINES:
1. Base your answer ONLY on the execution results provided above
2. Provide a clear, comprehensive, and well-structured response
3. If any critical information is missing, acknowledge the limitations
4. Organize your response logically with clear sections if needed
5. Include specific details and evidence from the execution results
6. Maximum response length: {} words

Generate your final answer:
"#, self.config.max_tokens));
        
        Ok(prompt)
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

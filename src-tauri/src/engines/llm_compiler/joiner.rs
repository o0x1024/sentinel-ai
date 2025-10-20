//! Intelligent Joiner - 智能决策器
//!
//! 负责分析执行结果并决定是否继续执行或完成任务
//! 核心特性：
//! - 智能分析：基于AI的结果分析
//! - 决策逻辑：Complete vs Continue
//! - 上下文感知：考虑历史执行结果
//! - 自适应学习：根据执行效果调整决策
//! - 结果聚合：整合多轮执行结果

use anyhow::Result;
use serde_json::{Value, json};
use std::collections::HashMap;
use chrono::Utc;
use tracing::{info, warn};

use crate::services::ai::AiService;
use crate::services::prompt_db::PromptRepository;
use crate::models::prompt::{ArchitectureType, StageType};
use crate::utils::ordered_message::ChunkType;
use super::types::*;
use super::types::EfficiencyMetrics;

/// Intelligent Joiner - 智能决策器
pub struct IntelligentJoiner {
    /// AI服务
    ai_service: AiService,
    /// 配置
    config: LlmCompilerConfig,
    /// 动态Prompt仓库
    prompt_repo: Option<PromptRepository>,
    /// 决策历史
    decision_history: Vec<JoinerDecisionRecord>,
    /// 执行上下文
    execution_context: ExecutionContext,
}

/// 决策记录
#[derive(Debug, Clone)]
struct JoinerDecisionRecord {
    /// 决策时间
    #[allow(unused)]
    timestamp: chrono::DateTime<Utc>,
    /// 决策类型
    decision: JoinerDecision,
    /// 决策原因
    reason: String,
    /// 执行轮次
    round: usize,
    /// 任务完成数
    #[allow(unused)]
    completed_tasks: usize,
    /// 任务失败数
    #[allow(unused)]
    failed_tasks: usize,
    /// 置信度
    confidence: f64,
}

/// 执行上下文
#[derive(Debug, Clone, Default)]
struct ExecutionContext {
    /// 原始查询
    original_query: String,
    /// 总执行轮次
    total_rounds: usize,
    /// 累计执行时间（毫秒）
    total_execution_time_ms: u64,
    /// 累计完成任务数
    total_completed_tasks: usize,
    /// 累计失败任务数
    total_failed_tasks: usize,
    /// 关键发现
    key_findings: Vec<String>,
    /// 执行摘要
    execution_summaries: Vec<String>,
}

/// 决策分析结果
#[derive(Debug, Clone)]
struct DecisionAnalysis {
    /// 推荐决策
    recommended_decision: JoinerDecision,
    /// 置信度 (0.0 - 1.0)
    confidence: f64,
    /// 分析原因
    reasoning: String,
    /// 关键指标
    #[allow(unused)]
    key_metrics: HashMap<String, Value>,
    /// 建议的下一步行动
    #[allow(unused)]
    suggested_actions: Vec<String>,
}

impl IntelligentJoiner {
    pub fn new(ai_service: AiService, config: LlmCompilerConfig, prompt_repo: Option<PromptRepository>) -> Self {
        Self {
            ai_service,
            config,
            prompt_repo,
            decision_history: Vec::new(),
            execution_context: ExecutionContext::default(),
        }
    }

    /// 分析执行结果并做出决策
    pub async fn analyze_and_decide(
        &mut self,
        original_query: &str,
        execution_plan: &DagExecutionPlan,
        execution_results: &[TaskExecutionResult],
        round: usize,
    ) -> Result<JoinerDecision> {
        info!("开始分析执行结果，轮次: {}", round);
        
        // 更新执行上下文
        self.update_execution_context(original_query, execution_results, round);
        
        // 执行多层次分析
        let analysis = self.perform_comprehensive_analysis(
            original_query,
            execution_plan,
            execution_results,
            round,
        ).await?;
        
        // 记录决策
        let decision_record = JoinerDecisionRecord {
            timestamp: Utc::now(),
            decision: analysis.recommended_decision.clone(),
            reason: analysis.reasoning.clone(),
            round,
            completed_tasks: execution_results.iter().filter(|r| r.status == TaskStatus::Completed).count(),
            failed_tasks: execution_results.iter().filter(|r| r.status == TaskStatus::Failed).count(),
            confidence: analysis.confidence,
        };
        
        self.decision_history.push(decision_record);
        
        info!(
            "决策完成: {:?} (置信度: {:.2}, 原因: {})",
            analysis.recommended_decision,
            analysis.confidence,
            analysis.reasoning
        );
        
        Ok(analysis.recommended_decision)
    }

    /// 执行综合分析
    async fn perform_comprehensive_analysis(
        &self,
        original_query: &str,
        execution_plan: &DagExecutionPlan,
        execution_results: &[TaskExecutionResult],
        round: usize,
    ) -> Result<DecisionAnalysis> {
        // 1. 基础指标分析
        let basic_metrics = self.analyze_basic_metrics(execution_results);
        
        // 2. 目标完成度分析
        let goal_completion = self.analyze_goal_completion(original_query, execution_results).await?;
        
        // 3. 执行效率分析
        let efficiency_analysis = self.analyze_execution_efficiency(execution_results, round);
        
        // 4. 风险评估
        let risk_assessment = self.assess_continuation_risks(round, execution_results);
        
        // 5. AI辅助决策
        let ai_decision = self.get_ai_decision(
            original_query,
            execution_plan,
            execution_results,
            round,
        ).await?;
        
        // 6. 综合决策
        let final_analysis = self.synthesize_decision(
            basic_metrics,
            goal_completion,
            efficiency_analysis,
            risk_assessment,
            ai_decision,
            round,
        );
        
        Ok(final_analysis)
    }

    /// 分析基础指标
    fn analyze_basic_metrics(&self, execution_results: &[TaskExecutionResult]) -> HashMap<String, Value> {
        let mut metrics = HashMap::new();
        
        let total_tasks = execution_results.len();
        let completed_tasks = execution_results.iter().filter(|r| r.status == TaskStatus::Completed).count();
        let failed_tasks = execution_results.iter().filter(|r| r.status == TaskStatus::Failed).count();
        let success_rate = if total_tasks > 0 { completed_tasks as f64 / total_tasks as f64 } else { 0.0 };
        
        let total_duration: u64 = execution_results.iter().map(|r| r.duration_ms).sum();
        let avg_duration = if total_tasks > 0 { total_duration as f64 / total_tasks as f64 } else { 0.0 };
        
        metrics.insert("total_tasks".to_string(), json!(total_tasks));
        metrics.insert("completed_tasks".to_string(), json!(completed_tasks));
        metrics.insert("failed_tasks".to_string(), json!(failed_tasks));
        metrics.insert("success_rate".to_string(), json!(success_rate));
        metrics.insert("total_duration_ms".to_string(), json!(total_duration));
        metrics.insert("avg_duration_ms".to_string(), json!(avg_duration));
        
        metrics
    }

    /// 分析目标完成度
    async fn analyze_goal_completion(
        &self,
        original_query: &str,
        execution_results: &[TaskExecutionResult],
    ) -> Result<f64> {
        // 提取所有成功任务的输出
        let successful_outputs: Vec<&HashMap<String, Value>> = execution_results
            .iter()
            .filter(|r| r.status == TaskStatus::Completed)
            .map(|r| &r.outputs)
            .collect();
        
        if successful_outputs.is_empty() {
            return Ok(0.0);
        }
        
        // 构建目标完成度分析提示
        let completion_prompt = self.build_goal_completion_prompt(
            original_query,
            &successful_outputs,
        ).await?;
        
        // 调用AI分析目标完成度
        match self.ai_service.send_message_stream(
            Some(&completion_prompt), 
            None, 
            None, 
            None,
            true, 
            false,
            Some(ChunkType::Content)
        ).await {
            Ok(response) => {
                // 解析AI响应中的完成度分数
                self.parse_completion_score(&response)
            }
            Err(e) => {
                warn!("AI目标完成度分析失败: {}", e);
                // 使用启发式方法估算
                Ok(self.heuristic_completion_estimate(&successful_outputs))
            }
        }
    }

    /// 分析执行效率
    fn analyze_execution_efficiency(
        &self,
        execution_results: &[TaskExecutionResult],
        round: usize,
    ) -> HashMap<String, Value> {
        let mut efficiency = HashMap::new();
        
        let total_time: u64 = execution_results.iter().map(|r| r.duration_ms).sum();
        let completed_count = execution_results.iter().filter(|r| r.status == TaskStatus::Completed).count();
        
        let efficiency_score = if round > 0 && total_time > 0 {
            (completed_count as f64) / (round as f64 * total_time as f64 / 1000.0)
        } else {
            0.0
        };
        
        efficiency.insert("efficiency_score".to_string(), json!(efficiency_score));
        efficiency.insert("time_per_task_ms".to_string(), json!(if completed_count > 0 { total_time / completed_count as u64 } else { 0 }));
        efficiency.insert("round_number".to_string(), json!(round));
        
        efficiency
    }

    /// 评估继续执行的风险
    fn assess_continuation_risks(
        &self,
        round: usize,
        execution_results: &[TaskExecutionResult],
    ) -> HashMap<String, Value> {
        let mut risks = HashMap::new();
        
        // 轮次风险
        let round_risk = if round >= self.config.max_iterations {
            1.0
        } else {
            round as f64 / self.config.max_iterations as f64
        };
        
        // 失败率风险
        let failed_count = execution_results.iter().filter(|r| r.status == TaskStatus::Failed).count();
        let total_count = execution_results.len();
        let failure_risk = if total_count > 0 {
            failed_count as f64 / total_count as f64
        } else {
            0.0
        };
        
        // 时间风险
        let total_time: u64 = self.execution_context.total_execution_time_ms;
        let time_risk = if total_time > 300000 { // 5分钟
            1.0
        } else {
            total_time as f64 / 300000.0
        };
        
        // 综合风险评分
        let overall_risk = (round_risk * 0.4 + failure_risk * 0.3 + time_risk * 0.3).min(1.0);
        
        risks.insert("round_risk".to_string(), json!(round_risk));
        risks.insert("failure_risk".to_string(), json!(failure_risk));
        risks.insert("time_risk".to_string(), json!(time_risk));
        risks.insert("overall_risk".to_string(), json!(overall_risk));
        
        risks
    }

    /// 获取AI决策建议
    async fn get_ai_decision(
        &self,
        original_query: &str,
        execution_plan: &DagExecutionPlan,
        execution_results: &[TaskExecutionResult],
        round: usize,
    ) -> Result<JoinerDecision> {
        let mut decision_prompt = self.build_ai_decision_prompt(
            original_query,
            execution_plan,
            execution_results,
            round,
        ).await?;
        // RAG augmentation for joiner decision
        if let Ok(rag_service) = crate::commands::rag_commands::get_global_rag_service().await {
            if rag_service.get_config().augmentation_enabled {
                use tokio::time::{timeout, Duration};
                let (primary, fallback) = crate::rag::query_utils::build_rag_query_pair(original_query);
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
                };
                if let Ok(Ok((knowledge_context, _))) = timeout(
                    Duration::from_millis(1200),
                    rag_service.query_for_assistant(&rag_request),
                )
                .await
                {
                    if !knowledge_context.trim().is_empty() {
                        decision_prompt.push_str("\n\n[KNOWLEDGE CONTEXT]\n");
                        decision_prompt.push_str(&knowledge_context);
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
                        };
                        if let Ok(Ok((kb2, _))) = timeout(Duration::from_millis(1200), rag_service.query_for_assistant(&fallback_req)).await {
                            if !kb2.trim().is_empty() {
                                decision_prompt.push_str("\n\n[KNOWLEDGE CONTEXT]\n");
                                decision_prompt.push_str(&kb2);
                            }
                        }
                    }
                }
            }
        }
        
        match self.ai_service.send_message_stream(
            Some(&decision_prompt), 
            None, 
            None, 
            None,
            true, 
            false,
            Some(ChunkType::Content)
        ).await {
            Ok(response) => {
                self.parse_ai_decision(&response)
            }
            Err(e) => {
                warn!("AI决策分析失败: {}", e);
                // 使用默认决策逻辑
                Ok(self.default_decision_logic(execution_results, round))
            }
        }
    }

    /// 综合决策
    fn synthesize_decision(
        &self,
        basic_metrics: HashMap<String, Value>,
        goal_completion: f64,
        _efficiency_analysis: HashMap<String, Value>,
        risk_assessment: HashMap<String, Value>,
        ai_decision: JoinerDecision,
        round: usize,
    ) -> DecisionAnalysis {
        let success_rate = basic_metrics.get("success_rate")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        
        let overall_risk = risk_assessment.get("overall_risk")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        
        // 决策逻辑
        let should_complete = goal_completion >= 0.8 || // 目标完成度高
                             success_rate < 0.3 || // 成功率太低
                             overall_risk > 0.8 || // 风险太高
                             round >= self.config.max_iterations; // 达到最大轮次
        
        let recommended_decision = if should_complete {
            JoinerDecision::Complete {
                response: "基于分析建议完成执行".to_string(),
                confidence: 0.8,
                summary: self.get_execution_stats(),
            }
        } else {
            JoinerDecision::Continue {
                feedback: "需要继续执行以获取更多信息".to_string(),
                suggested_tasks: Vec::new(),
                confidence: 0.8,
            }
        };
        
        // 计算置信度
        let confidence = self.calculate_decision_confidence(
            goal_completion,
            success_rate,
            overall_risk,
            &ai_decision,
            &recommended_decision,
        );
        
        // 生成决策原因
        let reasoning = self.generate_decision_reasoning(
            goal_completion,
            success_rate,
            overall_risk,
            round,
            &recommended_decision,
        );
        
        // 建议的下一步行动
        let suggested_actions = self.generate_suggested_actions(
            &recommended_decision,
            goal_completion,
            success_rate,
        );
        
        DecisionAnalysis {
            recommended_decision,
            confidence,
            reasoning,
            key_metrics: basic_metrics,
            suggested_actions,
        }
    }

    /// 构建目标完成度分析提示
    async fn build_goal_completion_prompt(
        &self,
        original_query: &str,
        successful_outputs: &[&HashMap<String, Value>],
    ) -> Result<String> {
        if let Some(repo) = &self.prompt_repo {
            if let Ok(Some(dynamic)) = repo.get_active_prompt(ArchitectureType::LLMCompiler, StageType::Execution).await {
                let replaced = Self::apply_placeholders(&dynamic, vec![
                    ("{{USER_QUERY}}", original_query),
                    ("{original_query}", original_query),
                    ("{{RESULTS}}", &self.format_outputs_for_analysis(successful_outputs)),
                    ("{results}", &self.format_outputs_for_analysis(successful_outputs)),
                ]);
                return Ok(replaced);
            }
        }
        Ok(format!(
            "请分析以下执行结果是否充分回答了原始查询。\n\n\
            原始查询: {}\n\n\
            执行结果:\n{}\n\n\
            请给出0-1之间的完成度分数，其中：\n\
            - 1.0: 完全回答了查询\n\
            - 0.8: 基本回答了查询\n\
            - 0.6: 部分回答了查询\n\
            - 0.4: 少量相关信息\n\
            - 0.2: 几乎没有相关信息\n\
            - 0.0: 完全没有回答查询\n\n\
            请只返回数字分数。",
            original_query,
            self.format_outputs_for_analysis(successful_outputs)
        ))
    }

    /// 构建AI决策提示
    async fn build_ai_decision_prompt(
        &self,
        original_query: &str,
        execution_plan: &DagExecutionPlan,
        execution_results: &[TaskExecutionResult],
        round: usize,
    ) -> Result<String> {
        let execution_summary = self.format_execution_summary(execution_results);
        let plan_summary = self.format_plan_summary(execution_plan);
        if let Some(repo) = &self.prompt_repo {
            if let Ok(Some(dynamic)) = repo.get_active_prompt(ArchitectureType::LLMCompiler, StageType::Replan).await {
                let replaced = Self::apply_placeholders(&dynamic, vec![
                    ("{{USER_QUERY}}", original_query),
                    ("{original_query}", original_query),
                    ("{{ROUND}}", &round.to_string()),
                    ("{round}", &round.to_string()),
                    ("{{MAX_ROUNDS}}", &self.config.max_iterations.to_string()),
                    ("{max_rounds}", &self.config.max_iterations.to_string()),
                    ("{{PLAN_SUMMARY}}", &plan_summary),
                    ("{plan_summary}", &plan_summary),
                    ("{{EXECUTION_SUMMARY}}", &execution_summary),
                    ("{execution_summary}", &execution_summary),
                    ("{{DECISION_HISTORY}}", &self.format_decision_history()),
                    ("{decision_history}", &self.format_decision_history()),
                ]);
                return Ok(replaced);
            }
        }
        Ok(format!(
            "作为LLMCompiler的智能决策器，请分析当前执行状态并决定下一步行动。\n\n\
            原始查询: {}\n\n\
            当前轮次: {}\n\
            最大轮次: {}\n\n\
            执行计划:\n{}\n\n\
            执行结果:\n{}\n\n\
            历史决策: {}\n\n\
            请基于以上信息决定：\n\
            - COMPLETE: 如果查询已被充分回答或继续执行风险过高\n\
            - CONTINUE: 如果需要更多信息或可以进一步优化结果\n\n\
            请返回JSON格式：\n\
            {{\n\
              \"decision\": \"COMPLETE\" 或 \"CONTINUE\",\n\
              \"reasoning\": \"决策原因\",\n\
              \"confidence\": 0.95\n\
            }}",
            original_query,
            round,
            self.config.max_iterations,
            plan_summary,
            execution_summary,
            self.format_decision_history()
        ))
    }

    fn apply_placeholders(template: &str, pairs: Vec<(&str, &str)>) -> String {
        let mut out = template.to_string();
        for (k, v) in pairs {
            out = out.replace(k, v);
        }
        out
    }

    /// 更新执行上下文
    fn update_execution_context(
        &mut self,
        original_query: &str,
        execution_results: &[TaskExecutionResult],
        round: usize,
    ) {
        self.execution_context.original_query = original_query.to_string();
        self.execution_context.total_rounds = round;
        
        let round_duration: u64 = execution_results.iter().map(|r| r.duration_ms).sum();
        self.execution_context.total_execution_time_ms += round_duration;
        
        let completed_count = execution_results.iter().filter(|r| r.status == TaskStatus::Completed).count();
        let failed_count = execution_results.iter().filter(|r| r.status == TaskStatus::Failed).count();
        
        self.execution_context.total_completed_tasks += completed_count;
        self.execution_context.total_failed_tasks += failed_count;
        
        // 提取关键发现
        let findings = self.extract_key_findings(execution_results);
        self.execution_context.key_findings.extend(findings);
        
        // 生成轮次摘要
        let summary = format!(
            "轮次{}: 完成{}个任务，失败{}个任务，耗时{}ms",
            round, completed_count, failed_count, round_duration
        );
        self.execution_context.execution_summaries.push(summary);
    }

    /// 提取关键发现
    fn extract_key_findings(&self, execution_results: &[TaskExecutionResult]) -> Vec<String> {
        let mut findings = Vec::new();
        
        for result in execution_results {
            if result.status == TaskStatus::Completed {
                // 从输出中提取关键信息
                if let Some(vulnerabilities) = result.outputs.get("vulnerabilities") {
                    if let Some(vulns) = vulnerabilities.as_array() {
                        if !vulns.is_empty() {
                            findings.push(format!("发现{}个漏洞", vulns.len()));
                        }
                    }
                }
                
                if let Some(open_ports) = result.outputs.get("open_ports") {
                    if let Some(ports) = open_ports.as_array() {
                        findings.push(format!("发现{}个开放端口", ports.len()));
                    }
                }
                
                if let Some(subdomains) = result.outputs.get("subdomains") {
                    if let Some(subs) = subdomains.as_array() {
                        findings.push(format!("发现{}个子域名", subs.len()));
                    }
                }
            }
        }
        
        findings
    }

    /// 计算决策置信度
    fn calculate_decision_confidence(
        &self,
        goal_completion: f64,
        success_rate: f64,
        overall_risk: f64,
        ai_decision: &JoinerDecision,
        recommended_decision: &JoinerDecision,
    ) -> f64 {
        let mut confidence = 0.5; // 基础置信度
        
        // 目标完成度影响
        if goal_completion > 0.8 {
            confidence += 0.3;
        } else if goal_completion < 0.3 {
            confidence += 0.2; // 明确知道未完成也是一种确定性
        }
        
        // 成功率影响
        if success_rate > 0.8 || success_rate < 0.2 {
            confidence += 0.2; // 极高或极低成功率都增加确定性
        }
        
        // 风险评估影响
        if overall_risk > 0.8 {
            confidence += 0.2; // 高风险时决策更确定
        }
        
        // AI决策一致性 - 比较决策类型
        let ai_is_complete = matches!(ai_decision, JoinerDecision::Complete { .. });
        let recommended_is_complete = matches!(recommended_decision, JoinerDecision::Complete { .. });
        if ai_is_complete == recommended_is_complete {
            confidence += 0.1;
        }
        
        (confidence as f64).min(1.0)
    }

    /// 生成决策原因
    fn generate_decision_reasoning(
        &self,
        goal_completion: f64,
        success_rate: f64,
        overall_risk: f64,
        round: usize,
        decision: &JoinerDecision,
    ) -> String {
        match decision {
            JoinerDecision::Complete { .. } => {
                let mut reasons = Vec::new();
                
                if goal_completion >= 0.8 {
                    reasons.push(format!("目标完成度高({:.1}%)", goal_completion * 100.0));
                }
                if success_rate < 0.3 {
                    reasons.push(format!("成功率过低({:.1}%)", success_rate * 100.0));
                }
                if overall_risk > 0.8 {
                    reasons.push(format!("继续执行风险过高({:.1}%)", overall_risk * 100.0));
                }
                if round >= self.config.max_iterations {
                    reasons.push("已达到最大执行轮次".to_string());
                }
                
                if reasons.is_empty() {
                    "基于综合分析决定完成执行".to_string()
                } else {
                    format!("决定完成执行: {}", reasons.join(", "))
                }
            }
            JoinerDecision::Continue { .. } => {
                format!(
                    "决定继续执行: 目标完成度({:.1}%), 成功率({:.1}%), 风险评估({:.1}%), 轮次({}/{})",
                    goal_completion * 100.0,
                    success_rate * 100.0,
                    overall_risk * 100.0,
                    round,
                    self.config.max_iterations
                )
            }
        }
    }

    /// 生成建议的下一步行动
    fn generate_suggested_actions(
        &self,
        decision: &JoinerDecision,
        goal_completion: f64,
        success_rate: f64,
    ) -> Vec<String> {
        let mut actions = Vec::new();
        
        match decision {
            JoinerDecision::Complete { .. } => {
                actions.push("整理和汇总执行结果".to_string());
                actions.push("生成最终报告".to_string());
                if goal_completion < 0.8 {
                    actions.push("标记未完全解决的问题".to_string());
                }
            }
            JoinerDecision::Continue { .. } => {
                if success_rate < 0.5 {
                    actions.push("检查失败任务并考虑重试".to_string());
                    actions.push("调整执行策略".to_string());
                }
                if goal_completion < 0.5 {
                    actions.push("重新规划以获取更多信息".to_string());
                    actions.push("考虑使用不同的工具或方法".to_string());
                }
                actions.push("继续执行下一轮任务".to_string());
            }
        }
        
        actions
    }

    // 辅助方法
    fn parse_completion_score(&self, response: &str) -> Result<f64> {
        // 尝试从响应中解析数字
        let score_str = response.trim();
        
        // 尝试直接解析数字
        if let Ok(score) = score_str.parse::<f64>() {
            return Ok(score.max(0.0).min(1.0));
        }
        
        // 尝试从文本中提取数字
        for line in response.lines() {
            if let Ok(score) = line.trim().parse::<f64>() {
                return Ok(score.max(0.0).min(1.0));
            }
        }
        
        // 如果无法解析，返回默认值
        warn!("无法解析完成度分数: {}", response);
        Ok(0.5)
    }

    fn parse_ai_decision(&self, response: &str) -> Result<JoinerDecision> {
        // 尝试解析JSON响应
        if let Ok(json_value) = serde_json::from_str::<Value>(response) {
            if let Some(decision_str) = json_value.get("decision").and_then(|v| v.as_str()) {
                return match decision_str.to_uppercase().as_str() {
                    "COMPLETE" => Ok(JoinerDecision::Complete {
                        response: "AI决策: 完成执行".to_string(),
                        confidence: 0.8,
                        summary: self.get_execution_stats(),
                    }),
                    "CONTINUE" => Ok(JoinerDecision::Continue {
                        feedback: "AI决策: 继续执行".to_string(),
                        suggested_tasks: Vec::new(),
                        confidence: 0.8,
                    }),
                    _ => Ok(JoinerDecision::Continue {
                        feedback: "默认决策: 继续执行".to_string(),
                        suggested_tasks: Vec::new(),
                        confidence: 0.5,
                    }),
                };
            }
        }
        
        // 如果JSON解析失败，尝试文本解析
        let response_upper = response.to_uppercase();
        if response_upper.contains("COMPLETE") {
            Ok(JoinerDecision::Complete {
                response: "文本解析: 完成执行".to_string(),
                confidence: 0.7,
                summary: self.get_execution_stats(),
            })
        } else if response_upper.contains("CONTINUE") {
            Ok(JoinerDecision::Continue {
                feedback: "文本解析: 继续执行".to_string(),
                suggested_tasks: Vec::new(),
                confidence: 0.7,
            })
        } else {
            warn!("无法解析AI决策: {}", response);
            Ok(JoinerDecision::Continue {
                feedback: "解析失败: 默认继续执行".to_string(),
                suggested_tasks: Vec::new(),
                confidence: 0.5,
            })
        }
    }

    fn default_decision_logic(
        &self,
        execution_results: &[TaskExecutionResult],
        round: usize,
    ) -> JoinerDecision {
        let completed_count = execution_results.iter().filter(|r| r.status == TaskStatus::Completed).count();
        let total_count = execution_results.len();
        let success_rate = if total_count > 0 { completed_count as f64 / total_count as f64 } else { 0.0 };
        
        // 简单的默认逻辑
        if round >= self.config.max_iterations || success_rate < 0.2 {
            JoinerDecision::Complete {
                response: format!("默认逻辑: 达到最大轮次({})或成功率过低({:.2})", round, success_rate),
                confidence: 0.6,
                summary: ExecutionSummary {
                    total_tasks: execution_results.len(),
                    successful_tasks: completed_count,
                    failed_tasks: execution_results.len() - completed_count,
                    total_duration_ms: execution_results.iter().map(|r| r.duration_ms).sum(),
                    replanning_count: 0,
                    key_findings: Vec::new(),
                    efficiency_metrics: EfficiencyMetrics {
                        average_parallelism: 0.0,
                        resource_utilization: 0.0,
                        task_success_rate: success_rate as f32,
                        average_task_duration_ms: 0.0,
                    },
                },
            }
        } else {
            JoinerDecision::Continue {
                feedback: format!("默认逻辑: 继续执行，当前成功率{:.2}", success_rate),
                suggested_tasks: Vec::new(),
                confidence: 0.6,
            }
        }
    }

    fn heuristic_completion_estimate(&self, successful_outputs: &[&HashMap<String, Value>]) -> f64 {
        if successful_outputs.is_empty() {
            return 0.0;
        }
        
        let mut score = 0.0;
        let mut factors = 0;
        
        for output in successful_outputs {
            // 检查是否有实质性结果
            if output.contains_key("vulnerabilities") ||
               output.contains_key("open_ports") ||
               output.contains_key("subdomains") ||
               output.contains_key("urls_found") {
                score += 0.3;
                factors += 1;
            }
            
            // 检查结果质量
            if output.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                score += 0.2;
                factors += 1;
            }
        }
        
        if factors > 0 {
            (score / factors as f64).min(1.0)
        } else {
            0.3 // 有输出但质量未知
        }
    }

    fn format_outputs_for_analysis(&self, outputs: &[&HashMap<String, Value>]) -> String {
        outputs.iter()
            .enumerate()
            .map(|(i, output)| format!("结果{}: {}", i + 1, serde_json::to_string_pretty(output).unwrap_or_else(|_| "无法序列化".to_string())))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn format_execution_summary(&self, execution_results: &[TaskExecutionResult]) -> String {
        execution_results.iter()
            .map(|result| {
                format!(
                    "任务{}: {} - {} ({}ms)",
                    result.task_id,
                    match result.status {
                        TaskStatus::Completed => "✓ 完成",
                        TaskStatus::Failed => "✗ 失败",
                        _ => "? 未知",
                    },
                    result.error.as_deref().unwrap_or("无错误"),
                    result.duration_ms
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_plan_summary(&self, plan: &DagExecutionPlan) -> String {
        format!(
            "计划ID: {}\n任务数: {}\n创建时间: {}",
            plan.id,
            plan.nodes.len(),
            plan.created_at.format("%Y-%m-%d %H:%M:%S")
        )
    }

    fn format_decision_history(&self) -> String {
        if self.decision_history.is_empty() {
            "无历史决策".to_string()
        } else {
            self.decision_history.iter()
                .take(3) // 只显示最近3个决策
                .map(|record| {
                    format!(
                        "轮次{}: {:?} (置信度: {:.2}) - {}",
                        record.round,
                        record.decision,
                        record.confidence,
                        record.reason
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    }

    /// 获取决策历史
    pub fn get_decision_history(&self) -> &[JoinerDecisionRecord] {
        &self.decision_history
    }

    /// 获取执行上下文
    pub fn get_execution_context(&self) -> &ExecutionContext {
        &self.execution_context
    }

    /// 重置决策器状态
    pub fn reset(&mut self) {
        self.decision_history.clear();
        self.execution_context = ExecutionContext::default();
    }

    /// 获取执行统计
    pub fn get_execution_stats(&self) -> ExecutionSummary {
        ExecutionSummary {
            total_tasks: self.execution_context.total_completed_tasks + self.execution_context.total_failed_tasks,
            successful_tasks: self.execution_context.total_completed_tasks,
            failed_tasks: self.execution_context.total_failed_tasks,
            total_duration_ms: self.execution_context.total_execution_time_ms,
            replanning_count: 0, // 可以从execution_context中获取
            key_findings: self.execution_context.key_findings.clone(),
            efficiency_metrics: EfficiencyMetrics {
                average_parallelism: 0.0,
                resource_utilization: 0.0,
                task_success_rate: if self.execution_context.total_completed_tasks + self.execution_context.total_failed_tasks > 0 {
                    self.execution_context.total_completed_tasks as f32 / (self.execution_context.total_completed_tasks + self.execution_context.total_failed_tasks) as f32
                } else {
                    0.0
                },
                average_task_duration_ms: 0.0,
            },
        }
    }
}
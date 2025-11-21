//! 任务复杂度分析器
//!
//! 实现混合判断机制:规则快速判断+LLM深度分析

use super::types::{TaskComplexity, ComplexityConfig};
use crate::services::ai::AiService;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;

/// 任务复杂度分析器
pub struct ComplexityAnalyzer {
    config: ComplexityConfig,
    ai_service: Option<Arc<AiService>>,
}

impl ComplexityAnalyzer {
    pub fn new(config: ComplexityConfig) -> Self {
        Self { 
            config,
            ai_service: None,
        }
    }

    /// 设置AI服务
    pub fn with_ai_service(mut self, ai_service: Arc<AiService>) -> Self {
        self.ai_service = Some(ai_service);
        self
    }

    /// 分析任务复杂度(混合判断)
    pub async fn analyze_task_complexity(
        &self,
        task_description: &str,
        task_parameters: Option<&HashMap<String, serde_json::Value>>,
    ) -> Result<TaskComplexity> {
        // 1. 尝试规则判断
        // if self.config.enable_rule_based {
        //     if let Some(complexity) = self.rule_based_analysis(task_description, task_parameters) {
        //         log::info!("Rule-based analysis result: {:?}", complexity);
        //         return Ok(complexity);
        //     }
        // }

        // 2. 规则不确定时,使用LLM判断
        if self.config.enable_llm_based {
            log::info!("Rule-based analysis inconclusive, using LLM analysis");
            return self.llm_based_analysis(task_description, task_parameters).await;
        }

        // 3. 默认返回中等复杂度
        log::warn!("Both rule-based and LLM analysis disabled, defaulting to Medium complexity");
        Ok(TaskComplexity::Medium)
    }

    /// 基于规则的快速判断
    fn rule_based_analysis(
        &self,
        task_description: &str,
        task_parameters: Option<&HashMap<String, serde_json::Value>>,
    ) -> Option<TaskComplexity> {
        let desc_lower = task_description.to_lowercase();

        // 检查简单任务关键词
        if let Some(simple_keywords) = self.config.rule_keywords.get(&TaskComplexity::Simple) {
            for keyword in simple_keywords {
                if desc_lower.contains(&keyword.to_lowercase()) {
                    // 进一步验证:简单任务通常只涉及单个操作
                    if self.is_single_operation(&desc_lower) {
                        return Some(TaskComplexity::Simple);
                    }
                }
            }
        }

        // 检查复杂任务关键词
        if let Some(complex_keywords) = self.config.rule_keywords.get(&TaskComplexity::Complex) {
            for keyword in complex_keywords {
                if desc_lower.contains(&keyword.to_lowercase()) {
                    return Some(TaskComplexity::Complex);
                }
            }
        }

        // 检查中等任务关键词
        if let Some(medium_keywords) = self.config.rule_keywords.get(&TaskComplexity::Medium) {
            for keyword in medium_keywords {
                if desc_lower.contains(&keyword.to_lowercase()) {
                    return Some(TaskComplexity::Medium);
                }
            }
        }

        // 基于任务参数判断
        if let Some(params) = task_parameters {
            // 检查是否指定了多个目标
            if let Some(targets) = params.get("targets").and_then(|v| v.as_array()) {
                if targets.len() > 5 {
                    return Some(TaskComplexity::Complex);
                } else if targets.len() > 1 {
                    return Some(TaskComplexity::Medium);
                }
            }

            // 检查是否指定了多个测试类型
            if let Some(test_types) = params.get("test_types").and_then(|v| v.as_array()) {
                if test_types.len() > 3 {
                    return Some(TaskComplexity::Complex);
                }
            }
        }

        // 基于句子复杂度判断
        let word_count = desc_lower.split_whitespace().count();
        let has_multiple_actions = desc_lower.matches("and").count() > 1
            || desc_lower.matches("then").count() > 0
            || desc_lower.matches("并且").count() > 1
            || desc_lower.matches("然后").count() > 0;

        if word_count > 30 || has_multiple_actions {
            return Some(TaskComplexity::Complex);
        }

        // 无法确定,返回None让LLM判断
        None
    }

    /// 判断是否为单个操作
    fn is_single_operation(&self, desc: &str) -> bool {
        // 单个操作的特征:
        // 1. 没有连接词(and, then, 并且, 然后)
        // 2. 只有一个动词
        let has_connectors = desc.contains("and ")
            || desc.contains("then ")
            || desc.contains("并且")
            || desc.contains("然后")
            || desc.contains("接着");

        !has_connectors
    }

    /// 基于LLM的深度分析
    async fn llm_based_analysis(
        &self,
        task_description: &str,
        task_parameters: Option<&HashMap<String, serde_json::Value>>,
    ) -> Result<TaskComplexity> {
        // 构建分析prompt
        let user_prompt = self.build_complexity_analysis_prompt(task_description, task_parameters);
        //中文
        let system_prompt = "你是一个任务复杂度分析专家，请分析给定的安全测试任务并将其分类为简单、中等或复杂。只用一个词回答。";

        log::info!("LLM complexity analysis for task: {}", task_description);

        // 如果有AI服务,使用LLM分析
        if let Some(service) = &self.ai_service {
            match service
                .send_message_stream(
                    Some(&user_prompt),
                    Some(system_prompt),
                    None, // conversation_id
                    None, // message_id
                    false, // stream
                    false, // is_final
                    None, // chunk_type
                )
                .await
            {
                Ok(response) => {
                    log::info!("LLM response: {}", response);
                    match self.parse_llm_response(&response) {
                        Ok(complexity) => {
                            log::info!("LLM analysis result: {:?}", complexity);
                            return Ok(complexity);
                        }
                        Err(e) => {
                            log::warn!("Failed to parse LLM response: {}, falling back to heuristic", e);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("LLM call failed: {}, falling back to heuristic", e);
                }
            }
        } else {
            log::info!("No AI service available, using heuristic analysis");
        }

        // 降级:基于启发式规则返回
        let complexity = if task_description.len() > 100 {
            TaskComplexity::Complex
        } else if task_description.len() > 30 {
            TaskComplexity::Medium
        } else {
            TaskComplexity::Simple
        };

        log::info!("Heuristic analysis result: {:?}", complexity);
        Ok(complexity)
    }

    /// 构建复杂度分析prompt
    fn build_complexity_analysis_prompt(
        &self,
        task_description: &str,
        task_parameters: Option<&HashMap<String, serde_json::Value>>,
    ) -> String {
        let params_str = if let Some(params) = task_parameters {
            serde_json::to_string_pretty(params).unwrap_or_else(|_| "{}".to_string())
        } else {
            "{}".to_string()
        };

        //中文
        format!(
            r#"分析以下任务的复杂度，并将其分类为简单、中等或复杂。只用一个词回答。

**任务描述**: {}

**任务参数**: {}

**复杂度分类**:
1. **简单**: 单个工具执行, 直接操作 (例如: "扫描端口80", "检查网站是否正常")
2. **中等**: 多个工具顺序调用, 中等分析 (例如: "扫描网站并识别技术", "测试常见漏洞")
3. **复杂**: 多步骤推理, 攻击链构造, 需要规划 (例如: "执行渗透测试", "利用漏洞链", "红队评估")

**指导**:
- 考虑所需步骤数量
- 评估是否需要推理/规划
- 评估是否需要多个工具协同
- 检查是否从一步结果中获取下一步信息

**响应格式**:
只用一个词回答: "Simple", "Medium", "Complex"

你的分析:"#,
            task_description, params_str
        )
    }

    /// 解析LLM响应
    fn parse_llm_response(&self, response: &str) -> Result<TaskComplexity> {
        let response_lower = response.trim().to_lowercase();

        // 优先匹配完整单词
        if response_lower == "simple" || response_lower.starts_with("simple") {
            Ok(TaskComplexity::Simple)
        } else if response_lower == "complex" || response_lower.starts_with("complex") {
            Ok(TaskComplexity::Complex)
        } else if response_lower == "medium" || response_lower.starts_with("medium") {
            Ok(TaskComplexity::Medium)
        } else if response_lower.contains("simple") {
            Ok(TaskComplexity::Simple)
        } else if response_lower.contains("complex") {
            Ok(TaskComplexity::Complex)
        } else if response_lower.contains("medium") {
            Ok(TaskComplexity::Medium)
        } else {
            Err(anyhow!(
                "Failed to parse LLM response for complexity: {}",
                response
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_based_simple_task() {
        let config = ComplexityConfig::default();
        let analyzer = ComplexityAnalyzer::new(config);

        let result = analyzer.rule_based_analysis("scan port 80", None);
        assert_eq!(result, Some(TaskComplexity::Simple));
    }

    #[test]
    fn test_rule_based_complex_task() {
        let config = ComplexityConfig::default();
        let analyzer = ComplexityAnalyzer::new(config);

        let result = analyzer.rule_based_analysis("perform penetration test on example.com", None);
        assert_eq!(result, Some(TaskComplexity::Complex));
    }

    #[test]
    fn test_is_single_operation() {
        let config = ComplexityConfig::default();
        let analyzer = ComplexityAnalyzer::new(config);

        assert!(analyzer.is_single_operation("scan port 80"));
        assert!(!analyzer.is_single_operation("scan port 80 and then check vulnerabilities"));
    }
}


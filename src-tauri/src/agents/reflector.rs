//! Agent 反思器
//!
//! 评估执行结果，决定是否完成、继续或重新规划

use super::config::ReflectorConfig;
use super::types::{Decision, ExecutionContext, Reflection, StepResult};
use anyhow::Result;

/// 反思器
pub struct Reflector {
    config: ReflectorConfig,
}

impl Reflector {
    pub fn new(config: ReflectorConfig) -> Self {
        Self { config }
    }

    /// 反思执行结果
    pub fn reflect(&self, context: &ExecutionContext) -> Result<Reflection> {
        let results: Vec<&StepResult> = context.results.values().collect();
        
        // 计算成功率
        let total = results.len();
        if total == 0 {
            return Ok(Reflection {
                decision: Decision::Continue,
                reasoning: "No steps executed yet".to_string(),
                improvements: vec![],
            });
        }

        let successful = results.iter().filter(|r| r.success).count();
        let failed = total - successful;
        let failure_rate = failed as f32 / total as f32;

        // 检查是否需要重规划
        if failure_rate >= self.config.replan_threshold && self.config.reflect_on_error {
            let failed_steps: Vec<String> = results
                .iter()
                .filter(|r| !r.success)
                .map(|r| r.step_id.clone())
                .collect();

            return Ok(Reflection {
                decision: Decision::Replan {
                    reason: format!(
                        "High failure rate ({:.0}%). Failed steps: {}",
                        failure_rate * 100.0,
                        failed_steps.join(", ")
                    ),
                },
                reasoning: format!(
                    "Analyzed {} steps: {} successful, {} failed",
                    total, successful, failed
                ),
                improvements: self.suggest_improvements(&results),
            });
        }

        // 检查是否所有步骤都完成
        let all_completed = results.iter().all(|r| r.success);
        if all_completed {
            // 生成总结答案
            let answer = self.generate_summary(&results);
            return Ok(Reflection {
                decision: Decision::Complete { answer },
                reasoning: format!("All {} steps completed successfully", total),
                improvements: vec![],
            });
        }

        // 继续执行
        Ok(Reflection {
            decision: Decision::Continue,
            reasoning: format!(
                "Progress: {}/{} steps completed. Continuing execution.",
                successful, total
            ),
            improvements: vec![],
        })
    }

    /// 基于 LLM 响应的反思（解析 LLM 输出）
    pub fn reflect_with_llm_response(
        &self,
        context: &ExecutionContext,
        llm_response: &str,
    ) -> Result<Reflection> {
        // 尝试解析 JSON 响应
        if let Some(json_start) = llm_response.find('{') {
            if let Some(json_end) = llm_response.rfind('}') {
                let json_str = &llm_response[json_start..=json_end];
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                    return self.parse_reflection_response(&parsed, context);
                }
            }
        }

        // 如果无法解析 JSON，使用基于规则的反思
        self.reflect(context)
    }

    /// 解析反思响应
    fn parse_reflection_response(
        &self,
        response: &serde_json::Value,
        _context: &ExecutionContext,
    ) -> Result<Reflection> {
        // 检查是否是最终答案
        if response.get("type").and_then(|v| v.as_str()) == Some("final_answer") {
            let answer = response
                .get("answer")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return Ok(Reflection {
                decision: Decision::Complete { answer },
                reasoning: "LLM provided final answer".to_string(),
                improvements: vec![],
            });
        }

        // 检查是否需要重规划
        if response.get("replan").and_then(|v| v.as_bool()).unwrap_or(false) {
            let reason = response
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("LLM requested replan")
                .to_string();
            return Ok(Reflection {
                decision: Decision::Replan { reason },
                reasoning: "LLM analysis suggests replanning".to_string(),
                improvements: vec![],
            });
        }

        // 默认继续执行
        Ok(Reflection {
            decision: Decision::Continue,
            reasoning: "LLM response indicates continuation".to_string(),
            improvements: vec![],
        })
    }

    /// 生成执行总结
    fn generate_summary(&self, results: &[&StepResult]) -> String {
        let mut summary = String::from("## Execution Summary\n\n");
        
        summary.push_str(&format!("**Total Steps:** {}\n", results.len()));
        summary.push_str(&format!(
            "**Successful:** {}\n",
            results.iter().filter(|r| r.success).count()
        ));
        summary.push_str(&format!(
            "**Total Duration:** {:.2}s\n\n",
            results.iter().map(|r| r.duration_ms).sum::<u64>() as f64 / 1000.0
        ));

        summary.push_str("### Results\n\n");
        for result in results {
            let status = if result.success { "✓" } else { "✗" };
            summary.push_str(&format!(
                "- {} Step {}: {:.2}s\n",
                status,
                result.step_id,
                result.duration_ms as f64 / 1000.0
            ));
        }

        summary
    }

    /// 建议改进措施
    fn suggest_improvements(&self, results: &[&StepResult]) -> Vec<String> {
        let mut improvements = Vec::new();

        for result in results {
            if !result.success {
                if let Some(error) = &result.error {
                    if error.contains("timeout") {
                        improvements.push(format!(
                            "Step {}: Consider increasing timeout or optimizing operation",
                            result.step_id
                        ));
                    } else if error.contains("not found") || error.contains("not available") {
                        improvements.push(format!(
                            "Step {}: Check tool availability or try alternative approach",
                            result.step_id
                        ));
                    } else {
                        improvements.push(format!(
                            "Step {}: Review error and adjust parameters",
                            result.step_id
                        ));
                    }
                }
            }
        }

        improvements
    }

    /// 检查是否应该进行反思
    pub fn should_reflect(&self, context: &ExecutionContext) -> bool {
        // 每 N 次迭代反思一次
        if context.iteration % (self.config.min_iterations_between_reflections + 1) == 0 {
            return true;
        }

        // 如果有错误且配置了错误后反思
        if self.config.reflect_on_error {
            let has_errors = context.results.values().any(|r| !r.success);
            if has_errors {
                return true;
            }
        }

        false
    }
}

impl Default for Reflector {
    fn default() -> Self {
        Self::new(ReflectorConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context() -> ExecutionContext {
        ExecutionContext::new("exec-1", "Test task", 10)
    }

    #[test]
    fn test_reflect_empty_results() {
        let reflector = Reflector::default();
        let ctx = create_test_context();
        
        let reflection = reflector.reflect(&ctx).unwrap();
        assert!(matches!(reflection.decision, Decision::Continue));
    }

    #[test]
    fn test_reflect_all_success() {
        let reflector = Reflector::default();
        let mut ctx = create_test_context();
        
        ctx.add_result(
            "1".to_string(),
            StepResult::success("1", serde_json::json!({}), 100),
        );
        ctx.add_result(
            "2".to_string(),
            StepResult::success("2", serde_json::json!({}), 100),
        );
        
        let reflection = reflector.reflect(&ctx).unwrap();
        assert!(matches!(reflection.decision, Decision::Complete { .. }));
    }

    #[test]
    fn test_reflect_high_failure_rate() {
        let reflector = Reflector::new(ReflectorConfig {
            replan_threshold: 0.3,
            reflect_on_error: true,
            ..Default::default()
        });
        let mut ctx = create_test_context();
        
        // 50% failure rate (> 30% threshold)
        ctx.add_result(
            "1".to_string(),
            StepResult::success("1", serde_json::json!({}), 100),
        );
        ctx.add_result(
            "2".to_string(),
            StepResult::failed("2", "Test error", 100),
        );
        
        let reflection = reflector.reflect(&ctx).unwrap();
        assert!(matches!(reflection.decision, Decision::Replan { .. }));
    }

    #[test]
    fn test_should_reflect() {
        let reflector = Reflector::new(ReflectorConfig {
            min_iterations_between_reflections: 2,
            ..Default::default()
        });
        
        let mut ctx = create_test_context();
        ctx.iteration = 0;
        assert!(reflector.should_reflect(&ctx));
        
        ctx.iteration = 1;
        assert!(!reflector.should_reflect(&ctx));
        
        ctx.iteration = 3;
        assert!(reflector.should_reflect(&ctx));
    }
}


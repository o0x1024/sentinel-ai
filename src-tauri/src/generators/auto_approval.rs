//! 插件自动批准配置和逻辑
//!
//! 实现智能的插件审核策略：
//! - 高质量插件：自动批准
//! - 中等质量插件：需要人工审核
//! - 低质量插件：自动拒绝

use serde::{Deserialize, Serialize};

/// 插件自动批准配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAutoApprovalConfig {
    /// 是否启用自动批准
    pub enabled: bool,
    
    /// 自动批准的质量阈值（>= 此分数自动批准）
    pub auto_approve_threshold: f32,
    
    /// 需要人工审核的最低阈值（>= 此分数进入审核队列）
    pub require_review_threshold: f32,
    
    /// 自动拒绝阈值（< 此分数自动拒绝）
    pub auto_reject_threshold: f32,
    
    /// 是否自动重新生成低质量插件
    pub auto_regenerate_on_low_quality: bool,
    
    /// 最大重新生成次数
    pub max_regeneration_attempts: u32,
    
    /// 是否检查危险代码模式
    pub check_dangerous_patterns: bool,
    
    /// 危险代码模式列表（如果检测到则强制人工审核）
    pub dangerous_patterns: Vec<String>,
}

impl Default for PluginAutoApprovalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_approve_threshold: 80.0,      // 80分以上自动批准
            require_review_threshold: 60.0,    // 60-80分需要审核
            auto_reject_threshold: 60.0,       // 60分以下自动拒绝
            auto_regenerate_on_low_quality: true,
            max_regeneration_attempts: 2,
            check_dangerous_patterns: true,
            dangerous_patterns: vec![
                "eval(".to_string(),
                "Function(".to_string(),
                "fetch(".to_string(),
                "XMLHttpRequest".to_string(),
                "require(".to_string(),
                "import(".to_string(),
                "Deno.readFile".to_string(),
                "Deno.writeFile".to_string(),
            ],
        }
    }
}

/// 批准决策
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApprovalDecision {
    /// 自动批准
    AutoApprove {
        reason: String,
    },
    /// 需要人工审核
    RequireHumanReview {
        reason: String,
    },
    /// 自动拒绝
    AutoReject {
        reason: String,
    },
    /// 重新生成
    Regenerate {
        reason: String,
        attempts_remaining: u32,
    },
}

/// 插件自动批准引擎
pub struct PluginAutoApprovalEngine {
    config: PluginAutoApprovalConfig,
}

impl PluginAutoApprovalEngine {
    /// 创建新的自动批准引擎
    pub fn new(config: PluginAutoApprovalConfig) -> Self {
        Self { config }
    }

    /// 评估插件并做出批准决策
    pub fn evaluate_plugin(
        &self,
        quality_score: f32,
        validation_status: &str,
        plugin_code: &str,
        current_attempt: u32,
    ) -> ApprovalDecision {
        // 检查是否启用自动批准
        if !self.config.enabled {
            return ApprovalDecision::RequireHumanReview {
                reason: "Auto-approval is disabled, all plugins require human review".to_string(),
            };
        }

        // 检查验证状态
        if validation_status != "Passed" {
            return ApprovalDecision::AutoReject {
                reason: format!("Validation failed: {}", validation_status),
            };
        }

        // 检查危险代码模式
        if self.config.check_dangerous_patterns {
            if let Some(pattern) = self.has_dangerous_patterns(plugin_code) {
                return ApprovalDecision::RequireHumanReview {
                    reason: format!(
                        "Contains potentially dangerous pattern: '{}'. Manual review required for security.",
                        pattern
                    ),
                };
            }
        }

        // 基于质量分数做决策
        if quality_score >= self.config.auto_approve_threshold {
            // 高质量：自动批准
            ApprovalDecision::AutoApprove {
                reason: format!(
                    "Quality score {:.1} meets auto-approve threshold of {:.1}",
                    quality_score, self.config.auto_approve_threshold
                ),
            }
        } else if quality_score >= self.config.require_review_threshold {
            // 中等质量：需要人工审核
            ApprovalDecision::RequireHumanReview {
                reason: format!(
                    "Quality score {:.1} requires human review (threshold: {:.1}-{:.1})",
                    quality_score,
                    self.config.require_review_threshold,
                    self.config.auto_approve_threshold
                ),
            }
        } else {
            // 低质量
            if self.config.auto_regenerate_on_low_quality
                && current_attempt < self.config.max_regeneration_attempts
            {
                // 尝试重新生成
                ApprovalDecision::Regenerate {
                    reason: format!(
                        "Quality score {:.1} is below threshold {:.1}. Attempting regeneration.",
                        quality_score, self.config.auto_reject_threshold
                    ),
                    attempts_remaining: self.config.max_regeneration_attempts - current_attempt,
                }
            } else {
                // 自动拒绝
                ApprovalDecision::AutoReject {
                    reason: format!(
                        "Quality score {:.1} is below acceptable threshold {:.1}{}",
                        quality_score,
                        self.config.auto_reject_threshold,
                        if current_attempt >= self.config.max_regeneration_attempts {
                            format!(" after {} regeneration attempts", current_attempt)
                        } else {
                            String::new()
                        }
                    ),
                }
            }
        }
    }

    /// 检查代码中是否包含危险模式
    fn has_dangerous_patterns(&self, code: &str) -> Option<String> {
        for pattern in &self.config.dangerous_patterns {
            if code.contains(pattern) {
                return Some(pattern.clone());
            }
        }
        None
    }

    /// 批量评估插件
    pub fn evaluate_batch(
        &self,
        plugins: &[(f32, String, String)], // (quality_score, validation_status, code)
    ) -> Vec<ApprovalDecision> {
        plugins
            .iter()
            .map(|(score, status, code)| self.evaluate_plugin(*score, status, code, 0))
            .collect()
    }

    /// 获取统计信息
    pub fn get_stats(&self, decisions: &[ApprovalDecision]) -> ApprovalStats {
        let mut stats = ApprovalStats::default();

        for decision in decisions {
            match decision {
                ApprovalDecision::AutoApprove { .. } => stats.auto_approved += 1,
                ApprovalDecision::RequireHumanReview { .. } => stats.require_review += 1,
                ApprovalDecision::AutoReject { .. } => stats.auto_rejected += 1,
                ApprovalDecision::Regenerate { .. } => stats.to_regenerate += 1,
            }
        }

        stats.total = decisions.len();
        stats
    }
}

/// 批准统计信息
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ApprovalStats {
    pub total: usize,
    pub auto_approved: usize,
    pub require_review: usize,
    pub auto_rejected: usize,
    pub to_regenerate: usize,
}

impl ApprovalStats {
    /// 计算自动化率
    pub fn automation_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.auto_approved + self.auto_rejected) as f64 / self.total as f64 * 100.0
    }

    /// 计算批准率
    pub fn approval_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.auto_approved as f64 / self.total as f64 * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_approve_high_quality() {
        let config = PluginAutoApprovalConfig::default();
        let engine = PluginAutoApprovalEngine::new(config);

        let decision = engine.evaluate_plugin(
            85.0,
            "Passed",
            "// Clean code without dangerous patterns",
            0,
        );

        assert!(matches!(decision, ApprovalDecision::AutoApprove { .. }));
    }

    #[test]
    fn test_require_review_medium_quality() {
        let config = PluginAutoApprovalConfig::default();
        let engine = PluginAutoApprovalEngine::new(config);

        let decision = engine.evaluate_plugin(70.0, "Passed", "// Medium quality code", 0);

        assert!(matches!(
            decision,
            ApprovalDecision::RequireHumanReview { .. }
        ));
    }

    #[test]
    fn test_auto_reject_low_quality() {
        let mut config = PluginAutoApprovalConfig::default();
        config.auto_regenerate_on_low_quality = false;
        let engine = PluginAutoApprovalEngine::new(config);

        let decision = engine.evaluate_plugin(40.0, "Passed", "// Low quality code", 0);

        assert!(matches!(decision, ApprovalDecision::AutoReject { .. }));
    }

    #[test]
    fn test_regenerate_on_low_quality() {
        let config = PluginAutoApprovalConfig::default();
        let engine = PluginAutoApprovalEngine::new(config);

        let decision = engine.evaluate_plugin(50.0, "Passed", "// Low quality code", 0);

        assert!(matches!(decision, ApprovalDecision::Regenerate { .. }));
    }

    #[test]
    fn test_dangerous_pattern_detection() {
        let config = PluginAutoApprovalConfig::default();
        let engine = PluginAutoApprovalEngine::new(config);

        let decision = engine.evaluate_plugin(
            90.0,
            "Passed",
            "const result = eval('dangerous code');",
            0,
        );

        assert!(matches!(
            decision,
            ApprovalDecision::RequireHumanReview { .. }
        ));
    }

    #[test]
    fn test_validation_failed() {
        let config = PluginAutoApprovalConfig::default();
        let engine = PluginAutoApprovalEngine::new(config);

        let decision = engine.evaluate_plugin(85.0, "Failed", "// Code", 0);

        assert!(matches!(decision, ApprovalDecision::AutoReject { .. }));
    }

    #[test]
    fn test_approval_stats() {
        let decisions = vec![
            ApprovalDecision::AutoApprove {
                reason: "High quality".to_string(),
            },
            ApprovalDecision::AutoApprove {
                reason: "High quality".to_string(),
            },
            ApprovalDecision::RequireHumanReview {
                reason: "Medium quality".to_string(),
            },
            ApprovalDecision::AutoReject {
                reason: "Low quality".to_string(),
            },
        ];

        let config = PluginAutoApprovalConfig::default();
        let engine = PluginAutoApprovalEngine::new(config);
        let stats = engine.get_stats(&decisions);

        assert_eq!(stats.total, 4);
        assert_eq!(stats.auto_approved, 2);
        assert_eq!(stats.require_review, 1);
        assert_eq!(stats.auto_rejected, 1);
        assert_eq!(stats.automation_rate(), 75.0); // (2+1)/4 * 100
    }
}


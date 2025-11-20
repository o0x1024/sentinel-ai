//! 安全护栏系统
//!
//! 实现四阶段安全检查:Observe/Orient/Decide/Act

use super::types::*;
use anyhow::Result;
use std::time::SystemTime;
use uuid::Uuid;

/// 护栏管理器
pub struct GuardrailManager {
    config: GuardrailConfig,
}

impl GuardrailManager {
    pub fn new(config: GuardrailConfig) -> Self {
        Self { config }
    }

    /// 执行Observe阶段护栏检查
    pub async fn check_observe_phase(
        &self,
        target_info: &serde_json::Value,
    ) -> Result<Vec<GuardrailCheckResult>> {
        let mut results = Vec::new();

        for rule in &self.config.observe_rules {
            if !rule.enabled {
                continue;
            }

            let result = match rule.rule_type {
                GuardrailRuleType::TargetLegality => {
                    self.check_target_legality(rule, target_info).await
                }
                GuardrailRuleType::Authorization => {
                    self.check_authorization(rule, target_info).await
                }
                _ => {
                    // 其他类型的规则在此阶段跳过
                    GuardrailCheckResult {
                        check_id: Uuid::new_v4().to_string(),
                        rule_id: rule.id.clone(),
                        rule_name: rule.name.clone(),
                        result: GuardrailCheckStatus::Skipped,
                        severity: rule.severity.clone(),
                        message: "Rule not applicable in Observe phase".to_string(),
                        details: None,
                        checked_at: SystemTime::now(),
                    }
                }
            };

            results.push(result);
        }

        // 严格模式下,检查是否有失败
        if self.config.strict_mode {
            self.enforce_strict_mode(&results)?;
        }

        Ok(results)
    }

    /// 执行Orient阶段护栏检查
    pub async fn check_orient_phase(
        &self,
        analysis: &ThreatAnalysis,
    ) -> Result<Vec<GuardrailCheckResult>> {
        let mut results = Vec::new();

        for rule in &self.config.orient_rules {
            if !rule.enabled {
                continue;
            }

            let result = match rule.rule_type {
                GuardrailRuleType::ExploitRisk => {
                    self.check_exploit_risk(rule, analysis).await
                }
                _ => GuardrailCheckResult {
                    check_id: Uuid::new_v4().to_string(),
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    result: GuardrailCheckStatus::Skipped,
                    severity: rule.severity.clone(),
                    message: "Rule not applicable in Orient phase".to_string(),
                    details: None,
                    checked_at: SystemTime::now(),
                },
            };

            results.push(result);
        }

        if self.config.strict_mode {
            self.enforce_strict_mode(&results)?;
        }

        Ok(results)
    }

    /// 执行Decide阶段护栏检查
    pub async fn check_decide_phase(
        &self,
        action_plan: &ActionPlan,
    ) -> Result<Vec<GuardrailCheckResult>> {
        let mut results = Vec::new();

        for rule in &self.config.decide_rules {
            if !rule.enabled {
                continue;
            }

            let result = match rule.rule_type {
                GuardrailRuleType::PayloadSafety => {
                    self.check_payload_safety(rule, action_plan).await
                }
                GuardrailRuleType::OperationRisk => {
                    self.check_operation_risk(rule, action_plan).await
                }
                _ => GuardrailCheckResult {
                    check_id: Uuid::new_v4().to_string(),
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    result: GuardrailCheckStatus::Skipped,
                    severity: rule.severity.clone(),
                    message: "Rule not applicable in Decide phase".to_string(),
                    details: None,
                    checked_at: SystemTime::now(),
                },
            };

            results.push(result);
        }

        if self.config.strict_mode {
            self.enforce_strict_mode(&results)?;
        }

        Ok(results)
    }

    /// 执行Act阶段护栏检查
    pub async fn check_act_phase(
        &self,
        action_plan: &ActionPlan,
        execution_context: &serde_json::Value,
    ) -> Result<Vec<GuardrailCheckResult>> {
        let mut results = Vec::new();

        for rule in &self.config.act_rules {
            if !rule.enabled {
                continue;
            }

            let result = match rule.rule_type {
                GuardrailRuleType::OperationRisk => {
                    self.check_final_operation_risk(rule, action_plan, execution_context)
                        .await
                }
                GuardrailRuleType::ResourceLimit => {
                    self.check_resource_limits(rule, execution_context).await
                }
                _ => GuardrailCheckResult {
                    check_id: Uuid::new_v4().to_string(),
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    result: GuardrailCheckStatus::Skipped,
                    severity: rule.severity.clone(),
                    message: "Rule not applicable in Act phase".to_string(),
                    details: None,
                    checked_at: SystemTime::now(),
                },
            };

            results.push(result);
        }

        if self.config.strict_mode {
            self.enforce_strict_mode(&results)?;
        }

        Ok(results)
    }

    /// 检查目标合法性
    async fn check_target_legality(
        &self,
        rule: &GuardrailRule,
        target_info: &serde_json::Value,
    ) -> GuardrailCheckResult {
        // 提取目标地址
        let target = target_info
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // 检查是否为本地地址或内网地址
        let is_local = target.starts_with("localhost")
            || target.starts_with("127.")
            || target.starts_with("192.168.")
            || target.starts_with("10.")
            || target.starts_with("172.16.");

        // 检查是否为生产环境
        let is_production = target.contains("prod")
            || target.contains("production")
            || target.contains(".com")
            || target.contains(".net");

        let (status, message) = if is_local {
            (
                GuardrailCheckStatus::Passed,
                "Target is local/internal, safe to test".to_string(),
            )
        } else if is_production {
            (
                GuardrailCheckStatus::Warning,
                format!("Target appears to be production environment: {}", target),
            )
        } else {
            (
                GuardrailCheckStatus::Passed,
                "Target legality check passed".to_string(),
            )
        };

        GuardrailCheckResult {
            check_id: Uuid::new_v4().to_string(),
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            result: status,
            severity: rule.severity.clone(),
            message,
            details: Some(serde_json::json!({
                "target": target,
                "is_local": is_local,
                "is_production": is_production,
            })),
            checked_at: SystemTime::now(),
        }
    }

    /// 检查授权
    async fn check_authorization(
        &self,
        rule: &GuardrailRule,
        target_info: &serde_json::Value,
    ) -> GuardrailCheckResult {
        // 检查是否提供了授权证明
        let has_authorization = target_info.get("authorization").is_some()
            || target_info.get("authorized").and_then(|v| v.as_bool()).unwrap_or(false);

        let (status, message) = if has_authorization {
            (
                GuardrailCheckStatus::Passed,
                "Authorization verified".to_string(),
            )
        } else {
            (
                GuardrailCheckStatus::Warning,
                "No explicit authorization provided, proceed with caution".to_string(),
            )
        };

        GuardrailCheckResult {
            check_id: Uuid::new_v4().to_string(),
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            result: status,
            severity: rule.severity.clone(),
            message,
            details: Some(serde_json::json!({
                "has_authorization": has_authorization,
            })),
            checked_at: SystemTime::now(),
        }
    }

    /// 检查漏洞利用风险
    async fn check_exploit_risk(
        &self,
        rule: &GuardrailRule,
        analysis: &ThreatAnalysis,
    ) -> GuardrailCheckResult {
        // 评估威胁等级
        let high_risk = analysis.threat_level >= ThreatLevel::High;
        let has_critical_vulns = analysis
            .vulnerabilities
            .iter()
            .any(|v| v.cvss_score.unwrap_or(0.0) >= 9.0);

        let (status, message) = if has_critical_vulns {
            (
                GuardrailCheckStatus::Warning,
                "Critical vulnerabilities detected, high exploit risk".to_string(),
            )
        } else if high_risk {
            (
                GuardrailCheckStatus::Warning,
                "High threat level detected".to_string(),
            )
        } else {
            (
                GuardrailCheckStatus::Passed,
                "Exploit risk within acceptable range".to_string(),
            )
        };

        GuardrailCheckResult {
            check_id: Uuid::new_v4().to_string(),
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            result: status,
            severity: rule.severity.clone(),
            message,
            details: Some(serde_json::json!({
                "threat_level": format!("{:?}", analysis.threat_level),
                "vulnerability_count": analysis.vulnerabilities.len(),
                "has_critical_vulns": has_critical_vulns,
            })),
            checked_at: SystemTime::now(),
        }
    }

    /// 检查Payload安全性
    async fn check_payload_safety(
        &self,
        rule: &GuardrailRule,
        action_plan: &ActionPlan,
    ) -> GuardrailCheckResult {
        // 检查是否包含危险操作
        let dangerous_keywords = vec![
            "rm -rf", "delete", "drop", "truncate", "format", "shutdown", "reboot",
        ];

        let mut dangerous_steps = Vec::new();
        for step in &action_plan.steps {
            let step_desc = step.description.to_lowercase();
            for keyword in &dangerous_keywords {
                if step_desc.contains(keyword) {
                    dangerous_steps.push(step.name.clone());
                    break;
                }
            }
        }

        let (status, message) = if !dangerous_steps.is_empty() {
            (
                GuardrailCheckStatus::Failed,
                format!(
                    "Dangerous operations detected in steps: {}",
                    dangerous_steps.join(", ")
                ),
            )
        } else {
            (
                GuardrailCheckStatus::Passed,
                "No dangerous payloads detected".to_string(),
            )
        };

        GuardrailCheckResult {
            check_id: Uuid::new_v4().to_string(),
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            result: status,
            severity: rule.severity.clone(),
            message,
            details: Some(serde_json::json!({
                "dangerous_steps": dangerous_steps,
                "total_steps": action_plan.steps.len(),
            })),
            checked_at: SystemTime::now(),
        }
    }

    /// 检查操作风险
    async fn check_operation_risk(
        &self,
        rule: &GuardrailRule,
        action_plan: &ActionPlan,
    ) -> GuardrailCheckResult {
        let high_risk = action_plan.risk_assessment.risk_level >= RiskLevel::High;
        let requires_approval = action_plan.risk_assessment.requires_manual_approval;

        let (status, message) = if requires_approval {
            (
                GuardrailCheckStatus::Warning,
                "Operation requires manual approval before execution".to_string(),
            )
        } else if high_risk {
            (
                GuardrailCheckStatus::Warning,
                "High risk operation, proceed with caution".to_string(),
            )
        } else {
            (
                GuardrailCheckStatus::Passed,
                "Operation risk within acceptable range".to_string(),
            )
        };

        GuardrailCheckResult {
            check_id: Uuid::new_v4().to_string(),
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            result: status,
            severity: rule.severity.clone(),
            message,
            details: Some(serde_json::json!({
                "risk_level": format!("{:?}", action_plan.risk_assessment.risk_level),
                "requires_approval": requires_approval,
                "risk_factors": action_plan.risk_assessment.risk_factors,
            })),
            checked_at: SystemTime::now(),
        }
    }

    /// 检查最终操作风险(Act阶段)
    async fn check_final_operation_risk(
        &self,
        rule: &GuardrailRule,
        action_plan: &ActionPlan,
        _execution_context: &serde_json::Value,
    ) -> GuardrailCheckResult {
        // Act阶段的最终检查,更严格
        let critical_risk = action_plan.risk_assessment.risk_level == RiskLevel::Critical;

        let (status, message) = if critical_risk {
            (
                GuardrailCheckStatus::Failed,
                "Critical risk operation blocked in Act phase".to_string(),
            )
        } else {
            (
                GuardrailCheckStatus::Passed,
                "Final operation risk check passed".to_string(),
            )
        };

        GuardrailCheckResult {
            check_id: Uuid::new_v4().to_string(),
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            result: status,
            severity: rule.severity.clone(),
            message,
            details: Some(serde_json::json!({
                "risk_level": format!("{:?}", action_plan.risk_assessment.risk_level),
            })),
            checked_at: SystemTime::now(),
        }
    }

    /// 检查资源限制
    async fn check_resource_limits(
        &self,
        rule: &GuardrailRule,
        execution_context: &serde_json::Value,
    ) -> GuardrailCheckResult {
        // 检查资源使用情况
        let timeout = execution_context
            .get("timeout")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let max_timeout = 3600.0; // 1小时

        let (status, message) = if timeout > max_timeout {
            (
                GuardrailCheckStatus::Failed,
                format!("Timeout exceeds maximum allowed: {} > {}", timeout, max_timeout),
            )
        } else {
            (
                GuardrailCheckStatus::Passed,
                "Resource limits check passed".to_string(),
            )
        };

        GuardrailCheckResult {
            check_id: Uuid::new_v4().to_string(),
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            result: status,
            severity: rule.severity.clone(),
            message,
            details: Some(serde_json::json!({
                "timeout": timeout,
                "max_timeout": max_timeout,
            })),
            checked_at: SystemTime::now(),
        }
    }

    /// 严格模式执行
    fn enforce_strict_mode(&self, results: &[GuardrailCheckResult]) -> Result<()> {
        for result in results {
            if result.result == GuardrailCheckStatus::Failed
                && result.severity >= GuardrailSeverity::Error
            {
                return Err(anyhow::anyhow!(
                    "Guardrail check failed in strict mode: {} - {}",
                    result.rule_name,
                    result.message
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_target_legality_check() {
        let config = GuardrailConfig::default();
        let manager = GuardrailManager::new(config);

        let target_info = serde_json::json!({
            "target": "localhost:8080"
        });

        let results = manager.check_observe_phase(&target_info).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].result, GuardrailCheckStatus::Passed);
    }

    #[tokio::test]
    async fn test_production_target_warning() {
        let config = GuardrailConfig::default();
        let manager = GuardrailManager::new(config);

        let target_info = serde_json::json!({
            "target": "production.example.com"
        });

        let results = manager.check_observe_phase(&target_info).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].result, GuardrailCheckStatus::Warning);
    }
}


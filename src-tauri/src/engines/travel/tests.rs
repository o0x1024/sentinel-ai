//! Travel架构集成测试

#[cfg(test)]
mod integration_tests {
    use crate::engines::travel::*;
    use std::collections::HashMap;

    #[test]
    fn test_travel_config_default() {
        let config = TravelConfig::default();
        assert_eq!(config.max_ooda_cycles, 10);
        assert!(config.enable_guardrails);
        assert!(config.enable_threat_intel);
    }

    #[test]
    fn test_ooda_cycle_creation() {
        let cycle = OodaCycle::new(1);
        assert_eq!(cycle.cycle_number, 1);
        assert_eq!(cycle.current_phase, OodaPhase::Observe);
        assert_eq!(cycle.status, OodaCycleStatus::Running);
    }

    #[test]
    fn test_travel_trace_creation() {
        let trace = TravelTrace::new(
            "Test security scan".to_string(),
            TaskComplexity::Simple,
        );
        assert_eq!(trace.task_complexity, TaskComplexity::Simple);
        assert_eq!(trace.status, TravelStatus::Running);
        assert_eq!(trace.ooda_cycles.len(), 0);
    }

    #[test]
    fn test_guardrail_config_default() {
        let config = GuardrailConfig::default();
        assert!(!config.observe_rules.is_empty());
        assert!(!config.orient_rules.is_empty());
        assert!(!config.decide_rules.is_empty());
        assert!(!config.act_rules.is_empty());
        assert!(config.strict_mode);
    }

    #[test]
    fn test_complexity_config_default() {
        let config = ComplexityConfig::default();
        assert!(config.enable_rule_based);
        assert!(config.enable_llm_based);
        assert!(config.rule_keywords.contains_key(&TaskComplexity::Simple));
        assert!(config.rule_keywords.contains_key(&TaskComplexity::Medium));
        assert!(config.rule_keywords.contains_key(&TaskComplexity::Complex));
    }

    #[tokio::test]
    async fn test_complexity_analyzer_simple_task() {
        let config = ComplexityConfig::default();
        let analyzer = ComplexityAnalyzer::new(config);

        let complexity = analyzer
            .analyze_task_complexity("scan port 80", None)
            .await
            .unwrap();

        // 应该识别为简单任务
        assert!(matches!(
            complexity,
            TaskComplexity::Simple | TaskComplexity::Medium
        ));
    }

    #[tokio::test]
    async fn test_complexity_analyzer_complex_task() {
        let config = ComplexityConfig::default();
        let analyzer = ComplexityAnalyzer::new(config);

        let complexity = analyzer
            .analyze_task_complexity("perform penetration test on example.com", None)
            .await
            .unwrap();

        // 应该识别为复杂任务
        assert_eq!(complexity, TaskComplexity::Complex);
    }

    #[tokio::test]
    async fn test_guardrail_manager_observe_phase() {
        let config = GuardrailConfig::default();
        let manager = GuardrailManager::new(config);

        let target_info = serde_json::json!({
            "target": "localhost:8080"
        });

        let results = manager.check_observe_phase(&target_info).await.unwrap();
        assert!(!results.is_empty());

        // 本地目标应该通过检查
        let target_check = results
            .iter()
            .find(|r| r.rule_id == "target_legality")
            .unwrap();
        assert_eq!(target_check.result, GuardrailCheckStatus::Passed);
    }

    #[tokio::test]
    async fn test_threat_intel_manager_query() {
        let config = ThreatIntelConfig::default();
        let manager = ThreatIntelManager::new(config);

        let mut context = HashMap::new();
        context.insert(
            "technology".to_string(),
            serde_json::Value::String("WordPress".to_string()),
        );

        let threats = manager
            .query_threat_intel("SQL injection", &context)
            .await
            .unwrap();

        // 应该返回一些威胁信息
        assert!(!threats.is_empty());
    }

    #[tokio::test]
    async fn test_engine_dispatcher_simple_task() {
        let dispatcher = EngineDispatcher::new();

        let action_plan = ActionPlan {
            id: "test-plan".to_string(),
            name: "Simple Scan".to_string(),
            description: "Scan a single port".to_string(),
            steps: vec![ActionStep {
                id: "step-1".to_string(),
                name: "Port Scan".to_string(),
                description: "Scan port 80".to_string(),
                step_type: ActionStepType::DirectToolCall,
                tool_name: Some("nmap".to_string()),
                tool_args: {
                    let mut args = HashMap::new();
                    args.insert("target".to_string(), serde_json::json!("localhost"));
                    args.insert("port".to_string(), serde_json::json!(80));
                    args
                },
                estimated_duration: 10,
            }],
            estimated_duration: 10,
            risk_assessment: RiskAssessment {
                risk_level: RiskLevel::Low,
                risk_factors: vec![],
                mitigations: vec![],
                requires_manual_approval: false,
            },
        };

        let context = HashMap::new();
        let result = dispatcher
            .dispatch(TaskComplexity::Simple, &action_plan, &context)
            .await
            .unwrap();

        assert_eq!(result["execution_type"], "simple");
    }

    #[tokio::test]
    async fn test_ooda_executor_cycle() {
        let config = TravelConfig::default();
        let executor = OodaExecutor::new(config);

        let mut context = HashMap::new();
        context.insert(
            "target".to_string(),
            serde_json::Value::String("localhost".to_string()),
        );

        let cycle = executor
            .execute_cycle(1, "Test security", TaskComplexity::Simple, &mut context)
            .await
            .unwrap();

        assert_eq!(cycle.cycle_number, 1);
        assert_eq!(cycle.status, OodaCycleStatus::Completed);
        assert!(!cycle.phase_history.is_empty());
    }

    // #[test]
    // fn test_travel_engine_creation() {
    //     let engine = TravelEngine::with_defaults();
    //     assert_eq!(engine.get_name(), "Travel");
    //     assert_eq!(engine.get_version(), "1.0.0");

    //     let scenarios = engine.get_supported_scenarios();
    //     assert!(scenarios.contains(&"penetration_testing".to_string()));
    //     assert!(scenarios.contains(&"vulnerability_assessment".to_string()));
    // }

    #[test]
    fn test_threat_level_ordering() {
        assert!(ThreatLevel::Critical > ThreatLevel::High);
        assert!(ThreatLevel::High > ThreatLevel::Medium);
        assert!(ThreatLevel::Medium > ThreatLevel::Low);
        assert!(ThreatLevel::Low > ThreatLevel::Info);
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Critical > RiskLevel::High);
        assert!(RiskLevel::High > RiskLevel::Medium);
        assert!(RiskLevel::Medium > RiskLevel::Low);
    }

    #[test]
    fn test_guardrail_severity_ordering() {
        assert!(GuardrailSeverity::Critical > GuardrailSeverity::Error);
        assert!(GuardrailSeverity::Error > GuardrailSeverity::Warning);
        assert!(GuardrailSeverity::Warning > GuardrailSeverity::Info);
    }
}


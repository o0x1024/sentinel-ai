//! Tests for Tenth Man with history input

#[cfg(test)]
mod tests {
    use crate::agents::tenth_man::{
        InterventionContext, InterventionMode, TenthManConfig, TenthManTriggerPolicy, TriggerReason,
    };
    use sentinel_tools::buildin_tools::tenth_man_tool::{ReviewMode, TenthManToolArgs};

    #[test]
    fn test_review_mode_serialization() {
        // Test FullHistory mode
        let mode = ReviewMode::FullHistory;
        let json = serde_json::to_string(&mode).unwrap();
        assert!(json.contains("full_history"));

        // Test RecentMessages mode
        let mode = ReviewMode::RecentMessages { count: 10 };
        let json = serde_json::to_string(&mode).unwrap();
        assert!(json.contains("recent_messages"));
        assert!(json.contains("10"));

        // Test SpecificContent mode
        let mode = ReviewMode::SpecificContent {
            content: "test content".to_string(),
        };
        let json = serde_json::to_string(&mode).unwrap();
        assert!(json.contains("specific_content"));
        assert!(json.contains("test content"));
    }

    #[test]
    fn test_review_mode_deserialization() {
        // Test FullHistory
        let json = r#"{"mode":"full_history"}"#;
        let mode: ReviewMode = serde_json::from_str(json).unwrap();
        assert!(matches!(mode, ReviewMode::FullHistory));

        // Test RecentMessages
        let json = r#"{"mode":"recent_messages","count":5}"#;
        let mode: ReviewMode = serde_json::from_str(json).unwrap();
        if let ReviewMode::RecentMessages { count } = mode {
            assert_eq!(count, 5);
        } else {
            panic!("Expected RecentMessages mode");
        }

        // Test SpecificContent
        let json = r#"{"mode":"specific_content","content":"test"}"#;
        let mode: ReviewMode = serde_json::from_str(json).unwrap();
        if let ReviewMode::SpecificContent { content } = mode {
            assert_eq!(content, "test");
        } else {
            panic!("Expected SpecificContent mode");
        }
    }

    #[test]
    fn test_tenth_man_tool_args_default() {
        let args = TenthManToolArgs {
            execution_id: "test-123".to_string(),
            review_mode: ReviewMode::default(),
            review_type: "quick".to_string(),
            focus_area: None,
        };

        assert_eq!(args.execution_id, "test-123");
        assert!(matches!(args.review_mode, ReviewMode::FullHistory));
        assert_eq!(args.review_type, "quick");
        assert!(args.focus_area.is_none());
    }

    #[test]
    fn test_tenth_man_tool_args_with_focus() {
        let args = TenthManToolArgs {
            execution_id: "test-456".to_string(),
            review_mode: ReviewMode::RecentMessages { count: 20 },
            review_type: "full".to_string(),
            focus_area: Some("security vulnerabilities".to_string()),
        };

        assert_eq!(args.execution_id, "test-456");
        if let ReviewMode::RecentMessages { count } = args.review_mode {
            assert_eq!(count, 20);
        } else {
            panic!("Expected RecentMessages mode");
        }
        assert_eq!(args.review_type, "full");
        assert_eq!(args.focus_area.unwrap(), "security vulnerabilities");
    }

    #[test]
    fn test_high_risk_trigger_reason_serialization() {
        let trigger = TriggerReason::HighRiskTool("shell".to_string());
        let json = serde_json::to_string(&trigger).unwrap();
        assert!(json.contains("HighRiskTool"));
        assert!(json.contains("shell"));
    }

    #[test]
    fn test_intervention_context_supports_new_trigger_fields() {
        let context = InterventionContext {
            execution_id: "exec-1".to_string(),
            task: "verify fix".to_string(),
            tool_call_count: 3,
            recent_failure_count: 2,
            last_tool_name: Some("shell".to_string()),
            has_recent_verification: false,
            has_side_effects: true,
            current_content: Some("about to run a mutating command".to_string()),
            trigger_reason: TriggerReason::RepeatedFailurePattern,
        };

        assert_eq!(context.recent_failure_count, 2);
        assert_eq!(context.last_tool_name.as_deref(), Some("shell"));
        assert!(context.has_side_effects);
        assert!(!context.has_recent_verification);
    }

    #[test]
    fn test_realtime_mode_still_exists_after_trigger_expansion() {
        let mode = InterventionMode::Realtime;
        let json = serde_json::to_string(&mode).unwrap();
        assert!(json.contains("Realtime"));
    }

    #[test]
    fn test_trigger_policy_defaults_are_safe_and_nonzero() {
        let policy = TenthManTriggerPolicy::default();
        assert!(policy.review_high_risk_tools);
        assert!(policy.review_repeated_failures);
        assert!(policy.review_loops);
        assert!(policy.review_final_response_without_verification);
        assert_eq!(policy.repeated_failure_streak(), 2);
        assert_eq!(policy.loop_repeat_threshold(), 2);
        assert_eq!(policy.recent_verification_window(), 3);
        assert_eq!(policy.minimum_evidence_tool_calls(), 2);
        assert_eq!(policy.minimum_evidence_score(), 3);
    }

    #[test]
    fn test_tenth_man_config_contains_trigger_policy_defaults() {
        let config = TenthManConfig::default();
        assert!(config.trigger_policy.review_high_risk_tools);
        assert_eq!(config.trigger_policy.repeated_failure_streak(), 2);
        assert!(config.trigger_policy.review_low_evidence_high_confidence);
        assert_eq!(config.trigger_policy.minimum_evidence_tool_calls(), 2);
        assert_eq!(config.trigger_policy.minimum_evidence_score(), 3);
    }
}

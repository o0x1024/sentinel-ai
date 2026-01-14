//! Tests for Tenth Man with history input

#[cfg(test)]
mod tests {
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
            content: "test content".to_string() 
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
}

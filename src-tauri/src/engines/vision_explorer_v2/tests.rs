//! Tests for Vision Explorer V2 components

use crate::engines::vision_explorer_v2::blackboard::{Blackboard, ExplorationConfig};
use crate::engines::vision_explorer_v2::brain::AuthAgent;
use crate::engines::vision_explorer_v2::core::{PageContext, SuggestedAction};
use crate::engines::vision_explorer_v2::graph::{
    ExplorationGraph, ExplorationStatus, PageStateNode,
};
use crate::engines::vision_explorer_v2::persistence::{ExplorationSnapshot, PersistenceManager};
use crate::engines::vision_explorer_v2::safety::{SafetyLayer, SafetyPolicy};

#[cfg(test)]
mod fingerprint_tests {
    use super::*;

    #[test]
    fn test_fingerprint_same_structure_same_hash() {
        let ctx1 = PageContext {
            url: "https://example.com/page".to_string(),
            title: "Test Page".to_string(),
            screenshot: None,
            dom_snapshot: "<html><body><div><p>Hello</p></div></body></html>".to_string(),
            accessibility_tree: None,
        };

        let ctx2 = PageContext {
            url: "https://example.com/page".to_string(),
            title: "Test Page".to_string(),
            screenshot: None,
            dom_snapshot: "<html><body><div><p>Goodbye</p></div></body></html>".to_string(), // Different text
            accessibility_tree: None,
        };

        // Same structure, different text -> should have same fingerprint
        assert_eq!(ctx1.fingerprint(), ctx2.fingerprint());
    }

    #[test]
    fn test_fingerprint_different_structure_different_hash() {
        let ctx1 = PageContext {
            url: "https://example.com/page".to_string(),
            title: "Test Page".to_string(),
            screenshot: None,
            dom_snapshot: "<html><body><div><p>Hello</p></div></body></html>".to_string(),
            accessibility_tree: None,
        };

        let ctx2 = PageContext {
            url: "https://example.com/page".to_string(),
            title: "Test Page".to_string(),
            screenshot: None,
            dom_snapshot: "<html><body><div><span>Hello</span></div></body></html>".to_string(), // Different tag
            accessibility_tree: None,
        };

        // Different structure -> should have different fingerprint
        assert_ne!(ctx1.fingerprint(), ctx2.fingerprint());
    }

    #[test]
    fn test_fingerprint_ignores_query_params() {
        let ctx1 = PageContext {
            url: "https://example.com/page?session=abc123".to_string(),
            title: "Test Page".to_string(),
            screenshot: None,
            dom_snapshot: "<html><body></body></html>".to_string(),
            accessibility_tree: None,
        };

        let ctx2 = PageContext {
            url: "https://example.com/page?session=xyz789".to_string(),
            title: "Test Page".to_string(),
            screenshot: None,
            dom_snapshot: "<html><body></body></html>".to_string(),
            accessibility_tree: None,
        };

        // Same base URL, different query params -> should have same fingerprint
        assert_eq!(ctx1.fingerprint(), ctx2.fingerprint());
    }

    #[test]
    fn test_fingerprint_considers_title() {
        let ctx1 = PageContext {
            url: "https://example.com/page".to_string(),
            title: "Dashboard".to_string(),
            screenshot: None,
            dom_snapshot: "<html><body></body></html>".to_string(),
            accessibility_tree: None,
        };

        let ctx2 = PageContext {
            url: "https://example.com/page".to_string(),
            title: "Settings".to_string(), // Different title
            screenshot: None,
            dom_snapshot: "<html><body></body></html>".to_string(),
            accessibility_tree: None,
        };

        // Different title -> different fingerprint (SPA tab detection)
        assert_ne!(ctx1.fingerprint(), ctx2.fingerprint());
    }
}

#[cfg(test)]
mod safety_tests {
    use super::*;

    #[test]
    fn test_safety_blocks_delete() {
        let safety = SafetyLayer::default();
        let action = SuggestedAction {
            description: "Delete all records".to_string(),
            selector: "#delete-btn".to_string(),
            action_type: "click".to_string(),
            value: None,
            confidence: 0.9,
        };

        let result = safety.check_action(&action);
        assert!(!result.allowed, "Delete action should be blocked");
        assert!(result.risk_level > 0);
    }

    #[test]
    fn test_safety_blocks_logout() {
        let safety = SafetyLayer::default();
        let action = SuggestedAction {
            description: "Sign out of account".to_string(),
            selector: "#logout".to_string(),
            action_type: "click".to_string(),
            value: None,
            confidence: 0.9,
        };

        let result = safety.check_action(&action);
        assert!(!result.allowed, "Logout action should be blocked");
    }

    #[test]
    fn test_safety_allows_navigation() {
        let safety = SafetyLayer::default();
        let action = SuggestedAction {
            description: "View user profile".to_string(),
            selector: "#profile-link".to_string(),
            action_type: "click".to_string(),
            value: None,
            confidence: 0.9,
        };

        let result = safety.check_action(&action);
        assert!(result.allowed, "Normal navigation should be allowed");
        assert_eq!(result.risk_level, 0);
    }

    #[test]
    fn test_safety_blocks_dangerous_url() {
        let safety = SafetyLayer::default();

        let result = safety.check_url("https://example.com/api/logout");
        assert!(!result.allowed, "Logout URL should be blocked");

        let result = safety.check_url("https://example.com/api/delete/user/123");
        assert!(!result.allowed, "Delete URL should be blocked");
    }

    #[test]
    fn test_safety_allows_normal_url() {
        let safety = SafetyLayer::default();

        let result = safety.check_url("https://example.com/dashboard");
        assert!(result.allowed, "Normal URL should be allowed");

        let result = safety.check_url("https://example.com/users");
        assert!(result.allowed, "Users URL should be allowed");
    }

    #[test]
    fn test_safety_filter_actions() {
        let safety = SafetyLayer::default();
        let actions = vec![
            SuggestedAction {
                description: "View dashboard".to_string(),
                selector: "#dashboard".to_string(),
                action_type: "click".to_string(),
                value: None,
                confidence: 0.9,
            },
            SuggestedAction {
                description: "Delete account".to_string(),
                selector: "#delete-account".to_string(),
                action_type: "click".to_string(),
                value: None,
                confidence: 0.8,
            },
            SuggestedAction {
                description: "View settings".to_string(),
                selector: "#settings".to_string(),
                action_type: "click".to_string(),
                value: None,
                confidence: 0.7,
            },
        ];

        let filtered = safety.filter_actions(actions);
        assert_eq!(filtered.len(), 2, "Should filter out delete action");
        assert!(filtered.iter().all(|a| !a.description.contains("Delete")));
    }

    #[test]
    fn test_safety_disabled() {
        let policy = SafetyPolicy {
            enabled: false,
            ..Default::default()
        };
        let safety = SafetyLayer::new(policy);

        let action = SuggestedAction {
            description: "Delete everything".to_string(),
            selector: "#destroy-all".to_string(),
            action_type: "click".to_string(),
            value: None,
            confidence: 0.9,
        };

        let result = safety.check_action(&action);
        assert!(result.allowed, "Should allow when safety is disabled");
    }
}

#[cfg(test)]
mod auth_agent_tests {
    use super::*;

    #[test]
    fn test_detects_login_page_by_url() {
        let ctx = PageContext {
            url: "https://example.com/login".to_string(),
            title: "Welcome".to_string(),
            screenshot: None,
            dom_snapshot: "<html><body></body></html>".to_string(),
            accessibility_tree: None,
        };

        assert!(AuthAgent::is_login_page(&ctx));
    }

    #[test]
    fn test_detects_login_page_by_form() {
        let ctx = PageContext {
            url: "https://example.com/auth".to_string(),
            title: "Welcome".to_string(),
            screenshot: None,
            dom_snapshot: r#"
                <html><body>
                    <form>
                        <input type="email" name="email" />
                        <input type="password" name="password" />
                        <button type="submit">Sign In</button>
                    </form>
                </body></html>
            "#
            .to_string(),
            accessibility_tree: None,
        };

        assert!(AuthAgent::is_login_page(&ctx));
    }

    #[test]
    fn test_not_login_page() {
        let ctx = PageContext {
            url: "https://example.com/dashboard".to_string(),
            title: "Dashboard".to_string(),
            screenshot: None,
            dom_snapshot: "<html><body><h1>Welcome to Dashboard</h1></body></html>".to_string(),
            accessibility_tree: None,
        };

        assert!(!AuthAgent::is_login_page(&ctx));
    }

    #[test]
    fn test_detect_login_failure() {
        let ctx = PageContext {
            url: "https://example.com/login".to_string(),
            title: "Login".to_string(),
            screenshot: None,
            dom_snapshot: r#"
                <html><body>
                    <div class="error">Invalid username or password</div>
                </body></html>
            "#
            .to_string(),
            accessibility_tree: None,
        };

        assert!(AuthAgent::detect_login_failure(&ctx));
    }
}

#[cfg(test)]
mod graph_tests {
    use super::*;

    #[test]
    fn test_add_node() {
        let mut graph = ExplorationGraph::new();

        let node = PageStateNode {
            fingerprint: "test123".to_string(),
            url: "https://example.com".to_string(),
            title: "Test".to_string(),
            status: ExplorationStatus::Unvisited,
            depth: 0,
            page_type: None,
            possible_actions: vec![],
        };

        let idx = graph.add_node(node.clone());
        assert!(graph.has_state("test123"));

        // Adding same node again returns same index
        let idx2 = graph.add_node(node);
        assert_eq!(idx, idx2);
    }

    #[test]
    fn test_get_node() {
        let mut graph = ExplorationGraph::new();

        let node = PageStateNode {
            fingerprint: "abc".to_string(),
            url: "https://example.com/page".to_string(),
            title: "Page".to_string(),
            status: ExplorationStatus::Unvisited,
            depth: 1,
            page_type: Some("content".to_string()),
            possible_actions: vec![],
        };

        graph.add_node(node);

        let retrieved = graph.get_node("abc");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().url, "https://example.com/page");

        let missing = graph.get_node("xyz");
        assert!(missing.is_none());
    }
}

#[cfg(test)]
mod blackboard_tests {
    use super::*;

    #[tokio::test]
    async fn test_authentication_state() {
        let bb = Blackboard::new();

        assert!(!bb.is_authenticated().await);

        bb.set_authenticated(true).await;
        assert!(bb.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_credentials() {
        let bb = Blackboard::new();

        assert!(bb.get_credentials().await.is_none());

        bb.set_credentials("user@test.com".to_string(), "secret123".to_string())
            .await;

        let creds = bb.get_credentials().await;
        assert!(creds.is_some());
        assert_eq!(creds.as_ref().unwrap().username, "user@test.com");
    }

    #[tokio::test]
    async fn test_skip_url() {
        let bb = Blackboard::new();

        assert!(!bb.should_skip_url("https://example.com/page").await);

        // Default exclude patterns include "logout"
        assert!(bb.should_skip_url("https://example.com/logout").await);

        bb.add_skip_url("https://example.com/skip-me".to_string())
            .await;
        assert!(bb.should_skip_url("https://example.com/skip-me").await);
    }

    #[tokio::test]
    async fn test_kv_store() {
        let bb = Blackboard::new();

        bb.set_kv(
            "last_action".to_string(),
            serde_json::json!({"type": "click"}),
        )
        .await;

        let val = bb.get_kv("last_action").await;
        assert!(val.is_some());
        assert_eq!(val.unwrap()["type"], "click");

        assert!(bb.get_kv("nonexistent").await.is_none());
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_snapshot_create() {
        let snapshot =
            ExplorationSnapshot::new("session-1".to_string(), "https://example.com".to_string());

        assert_eq!(snapshot.session_id, "session-1");
        assert_eq!(snapshot.target_url, "https://example.com");
        assert_eq!(snapshot.steps_taken, 0);
    }

    #[test]
    fn test_snapshot_json_roundtrip() {
        let mut snapshot =
            ExplorationSnapshot::new("session-2".to_string(), "https://test.com".to_string());
        snapshot.steps_taken = 42;

        let json = snapshot.to_json().unwrap();
        let restored = ExplorationSnapshot::from_json(&json).unwrap();

        assert_eq!(restored.session_id, "session-2");
        assert_eq!(restored.steps_taken, 42);
    }

    #[test]
    fn test_persistence_manager() {
        let dir = tempdir().unwrap();
        let manager = PersistenceManager::new(dir.path(), "test-session".to_string());

        assert!(!manager.has_snapshot());

        let snapshot = ExplorationSnapshot::new(
            "test-session".to_string(),
            "https://example.com".to_string(),
        );

        manager.save(&snapshot).unwrap();
        assert!(manager.has_snapshot());

        let loaded = manager.load().unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().session_id, "test-session");
    }
}

//! Perception Engine - Pure page understanding without decision making
//!
//! This module implements the perception logic, separating understanding (what's on the page)
//! from decision making (what actions to take).

use crate::engines::vision_explorer_v2::core::{
    AuthStatus, FormField, FormInfo, PageContext, PageElement, PageType, PerceptionCapabilities,
    PerceptionEngine, PerceptionResult,
};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

/// Mock perception engine for testing
pub struct MockPerceptionEngine;

#[async_trait]
impl PerceptionEngine for MockPerceptionEngine {
    async fn analyze(&self, context: &PageContext) -> Result<PerceptionResult> {
        // Simple mock analysis based on URL patterns
        let page_type = if context.url.contains("login") || context.url.contains("signin") {
            PageType::Login
        } else if context.url.contains("admin") || context.url.contains("dashboard") {
            PageType::Dashboard
        } else if context.url.contains("api") {
            PageType::Api
        } else {
            PageType::Unknown
        };

        let auth_status = if context.title.contains("Dashboard") || context.url.contains("admin") {
            AuthStatus::Authenticated { username: None }
        } else {
            AuthStatus::NotAuthenticated
        };

        Ok(PerceptionResult {
            page_type,
            auth_status,
            content_summary: format!("Mock analysis of {}", context.title),
            elements: vec![],
            forms: vec![],
            api_endpoints: vec![],
            errors: vec![],
            metadata: HashMap::new(),
            confidence: 0.5,
        })
    }

    async fn extract_data(
        &self,
        _context: &PageContext,
        _schema: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        Ok(serde_json::json!({}))
    }

    async fn detect_login_page(&self, context: &PageContext) -> Result<bool> {
        Ok(context.url.contains("login") || context.url.contains("signin"))
    }

    async fn extract_login_fields(&self, _context: &PageContext) -> Result<Vec<FormField>> {
        Ok(vec![
            FormField {
                name: "username".to_string(),
                field_type: "text".to_string(),
                label: Some("Username".to_string()),
                required: true,
                value: None,
                placeholder: Some("Enter username".to_string()),
            },
            FormField {
                name: "password".to_string(),
                field_type: "password".to_string(),
                label: Some("Password".to_string()),
                required: true,
                value: None,
                placeholder: Some("Enter password".to_string()),
            },
        ])
    }

    fn capabilities(&self) -> PerceptionCapabilities {
        PerceptionCapabilities {
            name: "MockPerceptionEngine".to_string(),
            version: "1.0.0".to_string(),
            supported_analysis: vec!["basic".to_string()],
            supports_vision: false,
            supports_dom: true,
            supports_accessibility: false,
        }
    }
}

/// Utility functions for perception analysis
pub mod utils {
    use super::*;

    /// Check if a page element is likely a button
    pub fn is_button_element(element: &PageElement) -> bool {
        element.element_type.to_lowercase() == "button"
            || element
                .attributes
                .get("type")
                .map(|t| t.to_lowercase())
                == Some("submit".to_string())
            || element
                .attributes
                .get("role")
                .map(|r| r.to_lowercase())
                == Some("button".to_string())
    }

    /// Check if a page element is likely an input field
    pub fn is_input_element(element: &PageElement) -> bool {
        element.element_type.to_lowercase() == "input"
            || element.element_type.to_lowercase() == "textarea"
            || element.element_type.to_lowercase() == "select"
    }

    /// Check if a page element is likely a link
    pub fn is_link_element(element: &PageElement) -> bool {
        element.element_type.to_lowercase() == "a"
            || element
                .attributes
                .get("role")
                .map(|r| r.to_lowercase())
                == Some("link".to_string())
    }

    /// Extract text content from an element, cleaning whitespace
    pub fn clean_element_text(text: Option<&str>) -> Option<String> {
        text.map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
    }

    /// Determine if a form is likely a login form based on field types
    pub fn is_login_form(form: &FormInfo) -> bool {
        let has_password = form.fields.iter().any(|f| f.field_type == "password");
        let has_username = form
            .fields
            .iter()
            .any(|f| f.field_type == "text" || f.field_type == "email");

        has_password && has_username && form.fields.len() <= 4
    }

    /// Calculate confidence score based on multiple factors
    pub fn calculate_confidence(
        page_type_confidence: f32,
        element_count: usize,
        form_count: usize,
    ) -> f32 {
        let mut confidence = page_type_confidence;

        // More elements generally means better analysis
        if element_count > 5 {
            confidence += 0.1;
        }

        // Forms provide structure
        if form_count > 0 {
            confidence += 0.1;
        }

        confidence.min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_perception_engine() {
        let engine = MockPerceptionEngine;

        let context = PageContext {
            url: "https://example.com/login".to_string(),
            title: "Login Page".to_string(),
            screenshot: vec![],
            dom_snapshot: "<html></html>".to_string(),
            accessibility_tree: None,
            viewport_size: Some((1920, 1080)),
            timestamp: 1234567890,
        };

        let result = engine.analyze(&context).await.unwrap();
        assert_eq!(result.page_type, PageType::Login);
        assert_eq!(result.auth_status, AuthStatus::NotAuthenticated);

        let is_login = engine.detect_login_page(&context).await.unwrap();
        assert!(is_login);

        let fields = engine.extract_login_fields(&context).await.unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "username");
        assert_eq!(fields[1].name, "password");
    }

    #[test]
    fn test_page_context_fingerprint() {
        let context = PageContext {
            url: "https://example.com/page".to_string(),
            title: "Test Page".to_string(),
            screenshot: vec![],
            dom_snapshot: "<html></html>".to_string(),
            accessibility_tree: None,
            viewport_size: None,
            timestamp: 1234567890,
        };

        let fingerprint1 = context.fingerprint(false);
        let fingerprint2 = context.fingerprint(true);

        // Different auth status should produce different fingerprints
        assert_ne!(fingerprint1, fingerprint2);

        // Same context should produce same fingerprint
        let fingerprint3 = context.fingerprint(false);
        assert_eq!(fingerprint1, fingerprint3);
    }

    #[test]
    fn test_utility_functions() {
        use utils::*;

        let button_element = PageElement {
            element_type: "button".to_string(),
            selector: "#submit".to_string(),
            text: Some("Submit".to_string()),
            attributes: HashMap::new(),
            coordinates: None,
            bounding_box: None,
            confidence: 1.0,
            is_visible: true,
            is_interactive: true,
        };

        assert!(is_button_element(&button_element));
        assert!(!is_input_element(&button_element));
        assert!(!is_link_element(&button_element));

        let cleaned = clean_element_text(Some("  Hello World  "));
        assert_eq!(cleaned, Some("Hello World".to_string()));

        let empty_cleaned = clean_element_text(Some("   "));
        assert_eq!(empty_cleaned, None);
    }
}
use crate::engines::vision_explorer_v2::core::{
    FormField, PageContext, PerceptionCapabilities, PerceptionEngine, PerceptionResult, SuggestedAction,
};
use crate::engines::{LlmClient, LlmConfig};
use anyhow::Result;
use async_trait::async_trait;
use base64::Engine;
use sentinel_llm::ImageAttachment;

/// Visual Analyst - Analyzes screenshots for UI understanding
pub struct VisualAnalyst {
    llm_client: LlmClient,
    model: String,
}

impl VisualAnalyst {
    pub fn new(config: LlmConfig, model: String) -> Self {
        // Setup env vars for the LLM client
        config.setup_env_vars();
        Self {
            llm_client: LlmClient::new(config),
            model,
        }
    }

    /// Build the system prompt with fixed rules
    fn build_system_prompt(&self) -> &'static str {
        r#"You are a visual analyst for web application security testing.

Analyze webpage screenshots and identify:
1. **Login/Authentication Forms**: Any login, signup, or password forms
2. **Navigation Elements**: Menus, sidebars, hamburger icons, tabs, breadcrumbs  
3. **Interactive Widgets**: Buttons, dropdowns, modals, dialogs, date pickers
4. **Error Messages**: Any error banners, validation messages, or alerts
5. **Data Tables**: Tables with potentially sensitive data
6. **API Indicators**: Elements suggesting API endpoints (fetch buttons, data loaders)

Return a JSON object with this exact structure:
{
    "summary": "Brief description of what the page contains",
    "page_type": "login|dashboard|list|form|settings|error|other",
    "has_login_form": true/false,
    "login_form_fields": ["field1", "field2"],
    "suggested_actions": [
        {
            "description": "action description",
            "selector": "CSS selector or element description (optional if x/y provided)",
            "action_type": "click|type|scroll|hover",
            "value": "value to type if applicable",
            "confidence": 0.9,
            "x": 123, // Estimated X coordinate in pixels
            "y": 456  // Estimated Y coordinate in pixels
        }
    ],
    "detected_errors": ["error message 1", "error message 2"],
    "navigation_items": ["nav item 1", "nav item 2"]
}

IMPORTANT: Return ONLY valid JSON, no markdown code blocks or explanations."#
    }

    /// Build the user prompt with page-specific context
    fn build_user_prompt(&self, context: &PageContext) -> String {
        format!(
            r#"Analyze the following webpage:

URL: {url}
Title: {title}"#,
            url = context.url,
            title = context.title
        )
    }

    /// Extract JSON from LLM response (handles various formats)
    fn extract_json(&self, response: &str) -> Option<serde_json::Value> {
        // Try direct parsing first
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(response) {
            return Some(json);
        }

        // Try extracting from code block
        let code_block_patterns = [
            (r"```json\s*\n?([\s\S]*?)\n?```", 1),
            (r"```\s*\n?([\s\S]*?)\n?```", 1),
            (r"\{[\s\S]*\}", 0),
        ];

        for (pattern, group) in code_block_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(response) {
                    let json_str = if group > 0 {
                        captures.get(group).map(|m| m.as_str())
                    } else {
                        captures.get(0).map(|m| m.as_str())
                    };

                    if let Some(json_str) = json_str {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str.trim())
                        {
                            return Some(json);
                        }
                    }
                }
            }
        }

        None
    }

    /// Parse suggested actions from JSON response
    fn parse_suggested_actions(&self, json: &serde_json::Value) -> Vec<SuggestedAction> {
        let mut actions = Vec::new();

        if let Some(arr) = json.get("suggested_actions").and_then(|v| v.as_array()) {
            for item in arr {
                let action = SuggestedAction {
                    description: item
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown action")
                        .to_string(),
                    selector: item
                        .get("selector")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    action_type: item
                        .get("action_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("click")
                        .to_string(),
                    value: item
                        .get("value")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    confidence: item
                        .get("confidence")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.5) as f32,
                    x: item.get("x").and_then(|v| v.as_i64()).map(|v| v as i32),
                    y: item.get("y").and_then(|v| v.as_i64()).map(|v| v as i32),
                };
                actions.push(action);
            }
        }

        actions
    }
}

#[async_trait]
impl PerceptionEngine for VisualAnalyst {
    async fn analyze(&self, context: &PageContext) -> Result<PerceptionResult> {
        let screenshot_base64 = base64::engine::general_purpose::STANDARD.encode(&context.screenshot);

        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(context);
        let image = ImageAttachment::new(screenshot_base64, "png".to_string());

        log::info!(
            "VisualAnalyst: Analyzing {} with model {}",
            context.url,
            self.model
        );

        let response = self
            .llm_client
            .completion_with_image(Some(system_prompt), &user_prompt, Some(&image))
            .await?;

        log::debug!(
            "VisualAnalyst response: {}",
            &response[..response.len().min(500)]
        );

        // Parse the JSON response
        if let Some(json) = self.extract_json(&response) {
            let summary = json
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("Visual analysis complete")
                .to_string();

            let suggested_actions = self.parse_suggested_actions(&json);

            // Check for login form
            let has_login = json
                .get("has_login_form")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let page_type = json
                .get("page_type")
                .and_then(|v| v.as_str())
                .unwrap_or("other")
                .to_string();

            // Add login detection to summary if found
            let enhanced_summary = if has_login {
                format!(
                    "{} [LOGIN FORM DETECTED - page_type: {}]",
                    summary, page_type
                )
            } else {
                summary
            };

            // Parse perception results from the structured data
            let page_type = if page_type == "login" {
                crate::engines::vision_explorer_v2::core::PageType::Login
            } else if page_type == "dashboard" {
                crate::engines::vision_explorer_v2::core::PageType::Dashboard
            } else if page_type == "form" {
                crate::engines::vision_explorer_v2::core::PageType::Form
            } else if page_type == "list" {
                crate::engines::vision_explorer_v2::core::PageType::List
            } else {
                crate::engines::vision_explorer_v2::core::PageType::Unknown
            };

            let auth_status = if has_login {
                crate::engines::vision_explorer_v2::core::AuthStatus::NotAuthenticated
            } else {
                crate::engines::vision_explorer_v2::core::AuthStatus::Unknown
            };

            Ok(PerceptionResult {
                page_type,
                auth_status,
                content_summary: enhanced_summary,
                elements: vec![], // TODO: Parse elements from LLM response
                forms: vec![],    // TODO: Parse forms from LLM response
                api_endpoints: vec![], // TODO: Parse API endpoints from LLM response
                errors: vec![],
                metadata: std::collections::HashMap::new(),
                confidence: 0.7,
            })
        } else {
            log::warn!("VisualAnalyst: Failed to parse JSON from response");
            Ok(PerceptionResult {
                page_type: crate::engines::vision_explorer_v2::core::PageType::Unknown,
                auth_status: crate::engines::vision_explorer_v2::core::AuthStatus::Unknown,
                content_summary: format!(
                    "Visual analysis completed but JSON parsing failed. Raw: {}",
                    &response[..response.len().min(200)]
                ),
                elements: vec![],
                forms: vec![],
                api_endpoints: vec![],
                errors: vec!["JSON parsing failed".to_string()],
                metadata: std::collections::HashMap::new(),
                confidence: 0.3,
            })
        }
    }

    async fn extract_data(
        &self,
        _context: &PageContext,
        _schema: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Implementation for visual data extraction
        Ok(serde_json::json!({}))
    }

    async fn detect_login_page(&self, context: &PageContext) -> Result<bool> {
        // Use visual analysis to detect login pages
        let screenshot_base64 = base64::engine::general_purpose::STANDARD.encode(&context.screenshot);

        let prompt = format!(
            "Analyze this screenshot and determine if it shows a login/sign-in page. \
             Look for: password fields, username/email fields, login buttons, sign-in forms. \
             Respond with only 'true' if it's a login page, 'false' otherwise."
        );

        let image = ImageAttachment::new(screenshot_base64, "png".to_string());

        match self
            .llm_client
            .completion_with_image(None, &prompt, Some(&image))
            .await
        {
            Ok(response) => {
                let response_lower = response.to_lowercase().trim().to_string();
                Ok(response_lower == "true" || response_lower.contains("yes"))
            }
            Err(_) => {
                // Fallback to URL-based detection
                let url_lower = context.url.to_lowercase();
                Ok(url_lower.contains("login") || url_lower.contains("signin"))
            }
        }
    }

    async fn extract_login_fields(&self, context: &PageContext) -> Result<Vec<FormField>> {
        // Use visual analysis to extract login form fields
        let screenshot_base64 = base64::engine::general_purpose::STANDARD.encode(&context.screenshot);

        let prompt = format!(
            "Analyze this screenshot and extract login form fields. \
             For each field, provide: name (username/password/email), type, label, required status. \
             Respond in JSON format: {{'fields': [{{'name': 'username', 'type': 'text', 'label': 'Username', 'required': true}}]}}"
        );

        let image = ImageAttachment::new(screenshot_base64, "png".to_string());

        match self
            .llm_client
            .completion_with_image(None, &prompt, Some(&image))
            .await
        {
            Ok(response) => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response) {
                    if let Some(fields) = json.get("fields").and_then(|v| v.as_array()) {
                        let form_fields: Vec<FormField> = fields
                            .iter()
                            .filter_map(|f| {
                                Some(FormField {
                                    name: f.get("name")?.as_str()?.to_string(),
                                    field_type: f.get("type")?.as_str()?.to_string(),
                                    label: f.get("label").and_then(|l| l.as_str().map(String::from)),
                                    required: f.get("required")?.as_bool().unwrap_or(true),
                                    value: None,
                                    placeholder: f.get("placeholder").and_then(|p| p.as_str().map(String::from)),
                                })
                            })
                            .collect();
                        return Ok(form_fields);
                    }
                }
                // Fallback to default fields
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
            Err(_) => {
                // Return default fields on error
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
        }
    }

    fn capabilities(&self) -> PerceptionCapabilities {
        PerceptionCapabilities {
            name: "VisualAnalyst".to_string(),
            version: "1.0.0".to_string(),
            supported_analysis: vec![
                "visual_analysis".to_string(),
                "element_detection".to_string(),
                "login_detection".to_string(),
                "form_analysis".to_string(),
            ],
            supports_vision: true,
            supports_dom: false,
            supports_accessibility: false,
        }
    }
}

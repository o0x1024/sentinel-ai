use crate::engines::vision_explorer_v2::core::{
    PageContext, PerceptionEngine, PerceptionResult, SuggestedAction,
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
            "selector": "CSS selector or element description",
            "action_type": "click|type|scroll|hover",
            "value": "value to type if applicable",
            "confidence": 0.9
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
        let screenshot_base64 = if let Some(bytes) = &context.screenshot {
            base64::engine::general_purpose::STANDARD.encode(bytes)
        } else {
            return Err(anyhow::anyhow!(
                "No screenshot available for visual analysis"
            ));
        };

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

            Ok(PerceptionResult {
                summary: enhanced_summary,
                suggested_actions,
            })
        } else {
            log::warn!("VisualAnalyst: Failed to parse JSON from response");
            Ok(PerceptionResult {
                summary: format!(
                    "Visual analysis completed but JSON parsing failed. Raw: {}",
                    &response[..response.len().min(200)]
                ),
                suggested_actions: vec![],
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
}

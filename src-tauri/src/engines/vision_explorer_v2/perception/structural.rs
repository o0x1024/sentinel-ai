use crate::engines::vision_explorer_v2::core::{
    PageContext, PerceptionEngine, PerceptionResult, SuggestedAction,
};
use crate::engines::{LlmClient, LlmConfig};
use anyhow::Result;
use async_trait::async_trait;

/// Structural Analyst - Analyzes DOM structure for navigation and forms
pub struct StructuralAnalyst {
    llm_client: LlmClient,
    model: String,
}

impl StructuralAnalyst {
    pub fn new(config: LlmConfig, model: String) -> Self {
        // Setup env vars for the LLM client
        config.setup_env_vars();
        Self {
            llm_client: LlmClient::new(config),
            model,
        }
    }

    fn build_prompt(&self, context: &PageContext) -> String {
        format!(
            r#"You are a structural analyst for web application security testing.

Analyze this webpage DOM structure and identify:
1. **Forms**: Login forms, search forms, data entry forms. For complete forms (like login), suggest a single "fill_form" action instead of multiple "type" actions if possible.
2. **Navigation**: Links, buttons, menus, tabs
3. **Interactive Elements**: Buttons, inputs, dropdowns, toggles
4. **Authentication Indicators**: Login/signup links, user menus, logout buttons
5. **Data Display**: Tables, lists, cards with data

Current Page:
- URL: {url}
- Title: {title}

DOM Structure (simplified):
{dom}

Return a JSON object with this exact structure:
{{
    "summary": "Brief description of the page structure",
    "page_type": "login|dashboard|list|form|settings|error|other",
    "has_login_form": true/false,
    "login_fields": [
        {{"field_id": "username", "field_type": "text", "label": "Username", "selector": "CSS selector"}},
        {{"field_id": "password", "field_type": "password", "label": "Password", "selector": "CSS selector"}}
    ],
    "navigation_links": [
        {{"text": "Link text", "href": "/path", "is_menu": false}}
    ],
    "suggested_actions": [
        {{
            "description": "action description",
            "selector": "CSS selector or form selector",
            "action_type": "click|type|scroll|hover|submit|fill_form",
            "value": "string value or JSON string for fill_form: {{\"field_selector\": \"value\", ...}}",
            "confidence": 0.9
        }}
    ],
    "api_endpoints": ["/api/endpoint1", "/api/endpoint2"]
}}

Note for "fill_form": Use it for login forms. The "selector" should be the form selector or a common parent. The "value" should be a JSON mapping selectors to values (use "[USERNAME]" and "[PASSWORD]" placeholders for credentials).

IMPORTANT: Return ONLY valid JSON, no markdown code blocks."#,
            url = context.url,
            title = context.title,
            dom = &context.dom_snapshot[..context.dom_snapshot.len().min(10000)]
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

        // Add high-priority login form actions if detected
        if json
            .get("has_login_form")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            if let Some(fields) = json.get("login_fields").and_then(|v| v.as_array()) {
                for field in fields {
                    let field_id = field
                        .get("field_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("input");
                    let field_type = field
                        .get("field_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("text");

                    actions.insert(
                        0,
                        SuggestedAction {
                            description: format!("Fill {} field", field_type),
                            selector: format!("#{}", field_id),
                            action_type: "type".to_string(),
                            value: Some(
                                if field_type == "password" {
                                    "[PASSWORD]"
                                } else {
                                    "[USERNAME]"
                                }
                                .to_string(),
                            ),
                            confidence: 0.95,
                        },
                    );
                }

                // Add submit action
                actions.push(SuggestedAction {
                    description: "Submit login form".to_string(),
                    selector: "button[type=submit], input[type=submit], button:contains(Login), button:contains(Sign in)".to_string(),
                    action_type: "click".to_string(),
                    value: None,
                    confidence: 0.9,
                });
            }
        }

        actions
    }
}

#[async_trait]
impl PerceptionEngine for StructuralAnalyst {
    async fn analyze(&self, context: &PageContext) -> Result<PerceptionResult> {
        let prompt = self.build_prompt(context);

        log::info!(
            "StructuralAnalyst: Analyzing {} with model {}",
            context.url,
            self.model
        );

        let response = self.llm_client.completion(None, &prompt).await?;

        log::debug!(
            "StructuralAnalyst response: {}",
            &response[..response.len().min(500)]
        );

        // Parse the JSON response
        if let Some(json) = self.extract_json(&response) {
            let summary = json
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("Structural analysis complete")
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

            // Enhanced summary with login detection
            let enhanced_summary = if has_login {
                let fields: Vec<String> = json
                    .get("login_fields")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|f| {
                                f.get("field_id")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                format!(
                    "{} [LOGIN FORM DETECTED - fields: {:?}, page_type: {}]",
                    summary, fields, page_type
                )
            } else {
                summary
            };

            Ok(PerceptionResult {
                summary: enhanced_summary,
                suggested_actions,
            })
        } else {
            log::warn!("StructuralAnalyst: Failed to parse JSON from response");
            Ok(PerceptionResult {
                summary: format!(
                    "Structural analysis completed but JSON parsing failed. Raw: {}",
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
        // Implementation for data extraction
        Ok(serde_json::json!({}))
    }
}

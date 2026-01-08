//! Perception Engine - Analyzes page context using Vision LLM
//!
//! This module is responsible for the "Observe" step in ReAct loop

use super::types::{
    AuthStatus, Element, FormField, FormInfo, Observation, PageContext, PageType,
};
use crate::engines::{LlmClient, LlmConfig};
use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::debug;

/// Perception engine for analyzing web pages
pub struct PerceptionEngine {
    llm_client: LlmClient,
}

impl PerceptionEngine {
    /// Create a new perception engine
    pub fn new(llm_config: LlmConfig) -> Self {
        Self {
            llm_client: LlmClient::new(llm_config),
        }
    }

    /// Analyze the current page and produce an observation
    pub async fn analyze(&self, context: &PageContext) -> Result<Observation> {
        debug!("Analyzing page: {}", context.url);

        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(context);

        // Prepare image attachment
        use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
        let image = crate::engines::ImageAttachment {
            base64_data: BASE64.encode(&context.screenshot),
            media_type: "image/png".to_string(),
        };

        // Call LLM with vision capability
        let response = self
            .llm_client
            .chat(Some(&system_prompt), &user_prompt, &[], Some(&image))
            .await
            .context("Failed to call LLM for perception")?;

        // Parse the response
        self.parse_llm_response(&response, context)
    }

    /// Build system prompt for perception
    fn build_system_prompt(&self) -> String {
        r#"You are a web page analyzer. Your task is to analyze a screenshot and HTML of a web page and provide structured information about it.

Analyze the page and return a JSON object with the following structure:

{
  "page_type": "login|dashboard|form|list|detail|api|error|static|unknown",
  "description": "A brief description of what this page is and what it does",
  "auth_status": {
    "type": "authenticated|not_authenticated|unknown",
    "username": "username if authenticated, null otherwise"
  },
  "elements": [
    {
      "element_id": "unique_id",
      "element_type": "button|link|input|select|textarea|etc",
      "selector": "CSS selector or XPath",
      "text": "visible text if any",
      "href": "URL if it's a link",
      "x": 100,
      "y": 200,
      "width": 150,
      "height": 40,
      "is_visible": true
    }
  ],
  "forms": [
    {
      "selector": "CSS selector for the form",
      "action": "form action URL",
      "method": "GET|POST",
      "fields": [
        {
          "name": "field name",
          "field_type": "text|password|email|etc",
          "label": "field label",
          "required": true,
          "value": "current value",
          "placeholder": "placeholder text"
        }
      ]
    }
  ],
  "links": ["url1", "url2", ...],
  "api_endpoints": [],
  "confidence": 0.95
}

Focus on:
1. Interactive elements (buttons, links, inputs)
2. Forms and their fields
3. Navigation elements
4. Authentication indicators
5. Important content

Return ONLY the JSON object, no additional text."#.to_string()
    }

    /// Build user prompt with context
    fn build_user_prompt(&self, context: &PageContext) -> String {
        format!(
            r#"Analyze this web page:

URL: {}
Title: {}
Viewport: {}x{}

HTML snippet (first 2000 chars):
{}

Please analyze the screenshot and HTML to provide structured information about this page."#,
            context.url,
            context.title,
            context.viewport_width,
            context.viewport_height,
            if context.html.len() > 2000 {
                &context.html[..2000]
            } else {
                &context.html
            }
        )
    }

    /// Parse LLM response into Observation
    fn parse_llm_response(&self, response: &str, _context: &PageContext) -> Result<Observation> {
        // Try to extract JSON from response
        let json_str = self.extract_json(response)?;
        
        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .context("Failed to parse LLM response as JSON")?;

        // Parse page type
        let page_type = match parsed["page_type"].as_str() {
            Some("login") => PageType::Login,
            Some("dashboard") => PageType::Dashboard,
            Some("form") => PageType::Form,
            Some("list") => PageType::List,
            Some("detail") => PageType::Detail,
            Some("api") => PageType::Api,
            Some("error") => PageType::Error,
            Some("static") => PageType::Static,
            _ => PageType::Unknown,
        };

        // Parse auth status
        let auth_status = match parsed["auth_status"]["type"].as_str() {
            Some("authenticated") => AuthStatus::Authenticated {
                username: parsed["auth_status"]["username"]
                    .as_str()
                    .map(|s| s.to_string()),
            },
            Some("not_authenticated") => AuthStatus::NotAuthenticated,
            _ => AuthStatus::Unknown,
        };

        // Parse elements
        let elements = if let Some(arr) = parsed["elements"].as_array() {
            arr.iter()
                .filter_map(|e| self.parse_element(e))
                .collect()
        } else {
            Vec::new()
        };

        // Parse forms
        let forms = if let Some(arr) = parsed["forms"].as_array() {
            arr.iter()
                .filter_map(|f| self.parse_form(f))
                .collect()
        } else {
            Vec::new()
        };

        // Parse links
        let links = if let Some(arr) = parsed["links"].as_array() {
            arr.iter()
                .filter_map(|l| l.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            Vec::new()
        };

        // Parse API endpoints
        let api_endpoints = if let Some(arr) = parsed["api_endpoints"].as_array() {
            arr.iter()
                .filter_map(|a| a.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            Vec::new()
        };

        let description = parsed["description"]
            .as_str()
            .unwrap_or("No description provided")
            .to_string();

        let confidence = parsed["confidence"].as_f64().unwrap_or(0.8) as f32;

        Ok(Observation {
            page_type,
            description,
            auth_status,
            elements,
            forms,
            links,
            api_endpoints,
            confidence,
            metadata: HashMap::new(),
        })
    }

    /// Extract JSON from LLM response (handles markdown code blocks)
    fn extract_json(&self, response: &str) -> Result<String> {
        let trimmed = response.trim();
        
        // Check if wrapped in markdown code block
        if trimmed.starts_with("```") {
            let lines: Vec<&str> = trimmed.lines().collect();
            if lines.len() > 2 {
                // Skip first and last line (code fence)
                let json_lines = &lines[1..lines.len() - 1];
                return Ok(json_lines.join("\n"));
            }
        }
        
        // Try to find JSON object
        if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                return Ok(trimmed[start..=end].to_string());
            }
        }
        
        // Return as-is and let JSON parser fail with proper error
        Ok(trimmed.to_string())
    }

    /// Parse an element from JSON
    fn parse_element(&self, json: &serde_json::Value) -> Option<Element> {
        Some(Element {
            element_id: json["element_id"].as_str()?.to_string(),
            element_type: json["element_type"].as_str()?.to_string(),
            selector: json["selector"].as_str()?.to_string(),
            text: json["text"].as_str().map(|s| s.to_string()),
            href: json["href"].as_str().map(|s| s.to_string()),
            x: json["x"].as_i64().map(|n| n as i32),
            y: json["y"].as_i64().map(|n| n as i32),
            width: json["width"].as_u64().map(|n| n as u32),
            height: json["height"].as_u64().map(|n| n as u32),
            is_visible: json["is_visible"].as_bool().unwrap_or(true),
        })
    }

    /// Parse a form from JSON
    fn parse_form(&self, json: &serde_json::Value) -> Option<FormInfo> {
        let fields = if let Some(arr) = json["fields"].as_array() {
            arr.iter()
                .filter_map(|f| self.parse_form_field(f))
                .collect()
        } else {
            Vec::new()
        };

        Some(FormInfo {
            selector: json["selector"].as_str()?.to_string(),
            action: json["action"].as_str().map(|s| s.to_string()),
            method: json["method"].as_str().unwrap_or("POST").to_string(),
            fields,
        })
    }

    /// Parse a form field from JSON
    fn parse_form_field(&self, json: &serde_json::Value) -> Option<FormField> {
        Some(FormField {
            name: json["name"].as_str()?.to_string(),
            field_type: json["field_type"].as_str()?.to_string(),
            label: json["label"].as_str().map(|s| s.to_string()),
            required: json["required"].as_bool().unwrap_or(false),
            value: json["value"].as_str().map(|s| s.to_string()),
            placeholder: json["placeholder"].as_str().map(|s| s.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json() {
        let engine = PerceptionEngine::new(LlmConfig::new("test", "test"));

        // Test with markdown code block
        let response = r#"```json
{"page_type": "login"}
```"#;
        let result = engine.extract_json(response).unwrap();
        assert!(result.contains("page_type"));

        // Test with plain JSON
        let response = r#"{"page_type": "login"}"#;
        let result = engine.extract_json(response).unwrap();
        assert_eq!(result, response);
    }
}

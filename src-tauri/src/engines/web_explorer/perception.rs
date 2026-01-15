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
    #[allow(dead_code)]
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
        debug!("Analyzing page: {} (using DOM-only analysis)", context.url);
        
        // Use DOM-only analysis by default
        // Vision LLM is disabled to avoid dependency on external vision services
        Ok(self.analyze_dom_only(context))
    }

    /// Build system prompt for perception
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
            snapshot_tree: None,
        })
    }

    /// Extract JSON from LLM response (handles markdown code blocks)
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

    /// DOM-only fallback when vision model is unavailable
    fn analyze_dom_only(&self, context: &PageContext) -> Observation {
        let html = context.html.as_str();

        let mut elements = Vec::new();
        let mut links = Vec::new();
        let mut inputs = Vec::new();

        // Extract links
        for href in Self::extract_attr_values(html, "a", "href").into_iter().take(50) {
            links.push(href.clone());
            elements.push(Element {
                element_id: format!("link-{}", elements.len() + 1),
                element_type: "link".to_string(),
                selector: format!("a[href=\"{}\"]", href),
                text: None,
                href: Some(href),
                x: None,
                y: None,
                width: None,
                height: None,
                is_visible: true,
            });
        }

        // Extract inputs
        let input_attrs = Self::extract_input_fields(html);
        for field in &input_attrs {
            inputs.push(FormField {
                name: field.name.clone().unwrap_or_else(|| "unknown".to_string()),
                field_type: field.field_type.clone().unwrap_or_else(|| "text".to_string()),
                label: None,
                required: false,
                value: None,
                placeholder: field.placeholder.clone(),
            });

            let selector = if let Some(ref name) = field.name {
                format!("input[name=\"{}\"]", name)
            } else if let Some(ref input_type) = field.field_type {
                format!("input[type=\"{}\"]", input_type)
            } else {
                "input".to_string()
            };

            elements.push(Element {
                element_id: format!("input-{}", elements.len() + 1),
                element_type: "input".to_string(),
                selector,
                text: None,
                href: None,
                x: None,
                y: None,
                width: None,
                height: None,
                is_visible: true,
            });
        }

        let forms = if inputs.is_empty() {
            Vec::new()
        } else {
            vec![FormInfo {
                selector: "form".to_string(),
                action: None,
                method: "POST".to_string(),
                fields: inputs,
            }]
        };

        let page_type = if html.contains("type=\"password\"") || html.contains("password") {
            PageType::Login
        } else if !forms.is_empty() {
            PageType::Form
        } else if html.contains("error") {
            PageType::Error
        } else {
            PageType::Unknown
        };

        let auth_status = if html.contains("logout") || html.contains("sign out") {
            AuthStatus::Authenticated { username: None }
        } else if page_type == PageType::Login {
            AuthStatus::NotAuthenticated
        } else {
            AuthStatus::Unknown
        };

        Observation {
            page_type,
            description: "DOM-only analysis (no vision model)".to_string(),
            auth_status,
            elements,
            forms,
            links,
            api_endpoints: Vec::new(),
            confidence: 0.3,
            metadata: HashMap::from([(
                "perception_mode".to_string(),
                serde_json::Value::String("dom_only".to_string()),
            )]),
            snapshot_tree: None, // Will be set by react_engine with actual snapshot
        }
    }

    fn extract_attr_values(html: &str, tag: &str, attr: &str) -> Vec<String> {
        let mut values = Vec::new();
        let mut remaining = html;
        let tag_open = format!("<{}", tag);
        let attr_pattern = format!("{}=", attr);

        while let Some(pos) = remaining.find(&tag_open) {
            let rest = &remaining[pos..];
            if let Some(end) = rest.find('>') {
                let tag_content = &rest[..end];
                if let Some(attr_pos) = tag_content.find(&attr_pattern) {
                    let value_start = attr_pos + attr_pattern.len();
                    let quote = tag_content.as_bytes().get(value_start).copied();
                    if let Some(q) = quote {
                        let q = q as char;
                        if q == '"' || q == '\'' {
                            let value_slice = &tag_content[(value_start + 1)..];
                            if let Some(value_end) = value_slice.find(q) {
                                values.push(value_slice[..value_end].to_string());
                            }
                        }
                    }
                }
                remaining = &rest[end..];
            } else {
                break;
            }
        }
        values
    }

    fn extract_input_fields(html: &str) -> Vec<InputField> {
        let mut fields = Vec::new();
        let mut remaining = html;
        let tag_open = "<input";

        while let Some(pos) = remaining.find(tag_open) {
            let rest = &remaining[pos..];
            if let Some(end) = rest.find('>') {
                let tag_content = &rest[..end];
                let field = InputField {
                    name: Self::extract_attr(tag_content, "name"),
                    field_type: Self::extract_attr(tag_content, "type"),
                    placeholder: Self::extract_attr(tag_content, "placeholder"),
                };
                fields.push(field);
                remaining = &rest[end..];
            } else {
                break;
            }
        }
        fields
    }

    fn extract_attr(tag_content: &str, attr: &str) -> Option<String> {
        let attr_pattern = format!("{}=", attr);
        let attr_pos = tag_content.find(&attr_pattern)?;
        let value_start = attr_pos + attr_pattern.len();
        let quote = tag_content.as_bytes().get(value_start).copied()? as char;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let value_slice = &tag_content[(value_start + 1)..];
        let value_end = value_slice.find(quote)?;
        Some(value_slice[..value_end].to_string())
    }
}

#[derive(Debug, Clone)]
struct InputField {
    name: Option<String>,
    field_type: Option<String>,
    placeholder: Option<String>,
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

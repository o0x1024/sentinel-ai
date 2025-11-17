//! Parameter extraction from HTTP requests

use serde::{Deserialize, Serialize};

/// Parameter type
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Unknown,
}

/// Parameter information
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub param_type: ParameterType,
    pub location: String, // "query", "body", "header", "path"
    pub example_values: Vec<String>,
}

/// Parameter extractor
pub struct ParamExtractor;

impl ParamExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extract parameters from query string
    pub fn extract_query_params(&self, query: &str) -> Vec<Parameter> {
        if query.is_empty() {
            return vec![];
        }

        let mut params = Vec::new();
        
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                let decoded_key = urlencoding::decode(key).unwrap_or_else(|_| key.into());
                let decoded_value = urlencoding::decode(value).unwrap_or_else(|_| value.into());
                
                params.push(Parameter {
                    name: decoded_key.to_string(),
                    param_type: self.infer_type(&decoded_value),
                    location: "query".to_string(),
                    example_values: vec![decoded_value.to_string()],
                });
            }
        }

        params
    }

    /// Extract parameters from request body
    pub fn extract_body_params(&self, body: &str, content_type: &Option<String>) -> Vec<Parameter> {
        let content_type = content_type.as_deref().unwrap_or("");

        // JSON body
        if content_type.contains("application/json") {
            return self.extract_from_json(body);
        }

        // Form data
        if content_type.contains("application/x-www-form-urlencoded") {
            return self.extract_from_form(body);
        }

        // Multipart form data
        if content_type.contains("multipart/form-data") {
            return self.extract_from_multipart(body);
        }

        // Try JSON as fallback
        if body.trim().starts_with('{') || body.trim().starts_with('[') {
            return self.extract_from_json(body);
        }

        vec![]
    }

    /// Extract from JSON body
    fn extract_from_json(&self, body: &str) -> Vec<Parameter> {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
            return vec![];
        };

        self.extract_from_json_value(&value, "body")
    }

    /// Recursively extract parameters from JSON value
    fn extract_from_json_value(&self, value: &serde_json::Value, location: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        match value {
            serde_json::Value::Object(obj) => {
                for (key, val) in obj {
                    let param_type = match val {
                        serde_json::Value::String(_) => ParameterType::String,
                        serde_json::Value::Number(_) => ParameterType::Number,
                        serde_json::Value::Bool(_) => ParameterType::Boolean,
                        serde_json::Value::Array(_) => ParameterType::Array,
                        serde_json::Value::Object(_) => ParameterType::Object,
                        serde_json::Value::Null => ParameterType::Unknown,
                    };

                    params.push(Parameter {
                        name: key.clone(),
                        param_type,
                        location: location.to_string(),
                        example_values: vec![self.value_to_string(val)],
                    });

                    // Recursively extract nested objects
                    if let serde_json::Value::Object(_) = val {
                        let nested = self.extract_from_json_value(val, location);
                        for nested_param in nested {
                            params.push(Parameter {
                                name: format!("{}.{}", key, nested_param.name),
                                ..nested_param
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        params
    }

    /// Extract from form data
    fn extract_from_form(&self, body: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        for pair in body.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                let decoded_key = urlencoding::decode(key).unwrap_or_else(|_| key.into());
                let decoded_value = urlencoding::decode(value).unwrap_or_else(|_| value.into());

                params.push(Parameter {
                    name: decoded_key.to_string(),
                    param_type: self.infer_type(&decoded_value),
                    location: "body".to_string(),
                    example_values: vec![decoded_value.to_string()],
                });
            }
        }

        params
    }

    /// Extract from multipart form data (basic)
    fn extract_from_multipart(&self, body: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        // Simple extraction: look for name="xxx" patterns
        for line in body.lines() {
            if line.contains("Content-Disposition") {
                if let Some(name_start) = line.find("name=\"") {
                    let name_str = &line[name_start + 6..];
                    if let Some(name_end) = name_str.find('"') {
                        let name = &name_str[..name_end];
                        params.push(Parameter {
                            name: name.to_string(),
                            param_type: ParameterType::String,
                            location: "body".to_string(),
                            example_values: vec![],
                        });
                    }
                }
            }
        }

        params
    }

    /// Infer parameter type from string value
    fn infer_type(&self, value: &str) -> ParameterType {
        // Number
        if value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok() {
            return ParameterType::Number;
        }

        // Boolean
        if matches!(value.to_lowercase().as_str(), "true" | "false") {
            return ParameterType::Boolean;
        }

        // Array (simple check)
        if value.starts_with('[') && value.ends_with(']') {
            return ParameterType::Array;
        }

        // Object (simple check)
        if value.starts_with('{') && value.ends_with('}') {
            return ParameterType::Object;
        }

        ParameterType::String
    }

    /// Convert JSON value to string representation
    fn value_to_string(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => "null".to_string(),
            other => serde_json::to_string(other).unwrap_or_else(|_| "".to_string()),
        }
    }
}

impl Default for ParamExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_query_params() {
        let extractor = ParamExtractor::new();
        let params = extractor.extract_query_params("id=123&name=test&active=true");

        assert_eq!(params.len(), 3);
        assert_eq!(params[0].name, "id");
        assert_eq!(params[0].param_type, ParameterType::Number);
        assert_eq!(params[1].name, "name");
        assert_eq!(params[1].param_type, ParameterType::String);
        assert_eq!(params[2].name, "active");
        assert_eq!(params[2].param_type, ParameterType::Boolean);
    }

    #[test]
    fn test_extract_from_json() {
        let extractor = ParamExtractor::new();
        let body = r#"{"username": "admin", "age": 25, "active": true}"#;
        let params = extractor.extract_body_params(body, &Some("application/json".to_string()));

        assert_eq!(params.len(), 3);
        assert!(params.iter().any(|p| p.name == "username" && p.param_type == ParameterType::String));
        assert!(params.iter().any(|p| p.name == "age" && p.param_type == ParameterType::Number));
        assert!(params.iter().any(|p| p.name == "active" && p.param_type == ParameterType::Boolean));
    }

    #[test]
    fn test_extract_from_form() {
        let extractor = ParamExtractor::new();
        let body = "username=admin&password=secret&remember=true";
        let params = extractor.extract_body_params(body, &Some("application/x-www-form-urlencoded".to_string()));

        assert_eq!(params.len(), 3);
        assert!(params.iter().any(|p| p.name == "username"));
        assert!(params.iter().any(|p| p.name == "password"));
        assert!(params.iter().any(|p| p.name == "remember"));
    }

    #[test]
    fn test_nested_json() {
        let extractor = ParamExtractor::new();
        let body = r#"{"user": {"name": "admin", "profile": {"age": 25}}}"#;
        let params = extractor.extract_body_params(body, &Some("application/json".to_string()));

        assert!(params.iter().any(|p| p.name == "user"));
        assert!(params.iter().any(|p| p.name == "user.name"));
        assert!(params.iter().any(|p| p.name == "user.profile"));
        assert!(params.iter().any(|p| p.name == "user.profile.age"));
    }
}


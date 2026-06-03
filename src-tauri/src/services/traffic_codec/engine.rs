use super::crypto::{execute_builtin_codec, CodecDirection};
use super::matcher;
use super::types::{
    CodecPipeline, CodecRequestMeta, CodecResult, CodecScope, CodecScopeTarget, CodecStep,
    CodecStepType, TrafficCodecRule,
};

pub struct TrafficCodecEngine {
    rules: Vec<TrafficCodecRule>,
}

impl TrafficCodecEngine {
    pub fn new(rules: Vec<TrafficCodecRule>) -> Self {
        Self { rules }
    }

    pub fn reload_rules(&mut self, rules: Vec<TrafficCodecRule>) {
        self.rules = rules;
    }

    pub fn find_matching_rules(&self, meta: &CodecRequestMeta) -> Vec<&TrafficCodecRule> {
        let mut rules = matcher::match_rules(&self.rules, meta);
        rules.sort_by_key(|rule| rule.order);
        rules
    }

    /// Decode content using matching rules. Steps execute in order.
    pub fn decode(&self, content: &str, meta: &CodecRequestMeta) -> CodecResult {
        self.apply_matching_rules(content, meta, CodecDirection::Decode, false)
    }

    /// Encode content using matching rules. Steps execute in REVERSE order.
    pub fn encode(&self, content: &str, meta: &CodecRequestMeta) -> CodecResult {
        self.apply_matching_rules(content, meta, CodecDirection::Encode, true)
    }

    /// Batch encode multiple contents (for Intruder)
    pub fn batch_encode(&self, contents: &[String], meta: &CodecRequestMeta) -> Vec<CodecResult> {
        contents
            .iter()
            .map(|content| self.encode(content, meta))
            .collect()
    }

    /// Test a pipeline with specific steps (for rule creation preview)
    pub fn test_pipeline(
        &self,
        content: &str,
        steps: &[CodecStep],
        direction: CodecDirection,
    ) -> CodecResult {
        let pipeline = CodecPipeline {
            steps: steps.to_vec(),
        };
        match self.apply_scoped_pipeline(
            content,
            &CodecScope {
                target: CodecScopeTarget::FullBody,
                fields: vec![],
                header_name: None,
                pattern: None,
            },
            &pipeline,
            direction,
        ) {
            Ok(result) => CodecResult {
                success: true,
                content: result,
                error: None,
                applied_rule_ids: vec![],
            },
            Err(error) => CodecResult {
                success: false,
                content: content.to_string(),
                error: Some(error),
                applied_rule_ids: vec![],
            },
        }
    }

    fn apply_matching_rules(
        &self,
        content: &str,
        meta: &CodecRequestMeta,
        direction: CodecDirection,
        reverse_rules: bool,
    ) -> CodecResult {
        let mut matching = self.find_matching_rules(meta);
        if reverse_rules {
            matching.reverse();
        }

        if matching.is_empty() {
            return CodecResult {
                success: true,
                content: content.to_string(),
                error: None,
                applied_rule_ids: vec![],
            };
        }

        let mut current = content.to_string();
        let mut applied_rule_ids = Vec::new();

        for rule in matching {
            match self.apply_scoped_pipeline(&current, &rule.scope, &rule.pipeline, direction) {
                Ok(result) => {
                    current = result;
                    applied_rule_ids.push(rule.id.clone());
                }
                Err(error) => {
                    return CodecResult {
                        success: false,
                        content: content.to_string(),
                        error: Some(error),
                        applied_rule_ids,
                    };
                }
            }
        }

        CodecResult {
            success: true,
            content: current,
            error: None,
            applied_rule_ids,
        }
    }

    fn apply_scoped_pipeline(
        &self,
        content: &str,
        scope: &CodecScope,
        pipeline: &CodecPipeline,
        direction: CodecDirection,
    ) -> Result<String, String> {
        match scope.target {
            CodecScopeTarget::FullBody => apply_full_body_codec(content, pipeline, direction),
            CodecScopeTarget::JsonField => {
                let body = extract_body(content);
                let processed = apply_json_field_codec(&body, &scope.fields, pipeline, direction)?;
                reassemble_with_body(content, &processed)
            }
            _ => Ok(content.to_string()),
        }
    }
}

fn apply_full_body_codec(
    content: &str,
    pipeline: &CodecPipeline,
    direction: CodecDirection,
) -> Result<String, String> {
    if is_http_message(content) {
        let (headers, body) = split_http_message(content);
        let processed = execute_pipeline(body.as_bytes(), pipeline, direction)?;
        let body_str = bytes_to_string(processed)?;
        Ok(format!("{headers}\n\n{body_str}"))
    } else {
        let processed = execute_pipeline(content.as_bytes(), pipeline, direction)?;
        bytes_to_string(processed)
    }
}

fn apply_json_field_codec(
    body: &str,
    fields: &[String],
    pipeline: &CodecPipeline,
    direction: CodecDirection,
) -> Result<String, String> {
    let mut json: serde_json::Value =
        serde_json::from_str(body).map_err(|error| format!("JSON parse failed: {error}"))?;

    for field_path in fields {
        apply_field_codec(&mut json, field_path, pipeline, direction)?;
    }

    serde_json::to_string(&json).map_err(|error| format!("JSON serialize failed: {error}"))
}

fn apply_field_codec(
    json: &mut serde_json::Value,
    field_path: &str,
    pipeline: &CodecPipeline,
    direction: CodecDirection,
) -> Result<(), String> {
    let parts: Vec<&str> = field_path.split('.').filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
        return Ok(());
    }

    let field = navigate_mut(json, &parts)
        .ok_or_else(|| format!("JSON field not found: {field_path}"))?;

    let serde_json::Value::String(value) = field else {
        return Err(format!("JSON field is not a string: {field_path}"));
    };

    let processed = execute_pipeline(value.as_bytes(), pipeline, direction)?;
    *field = serde_json::Value::String(bytes_to_string(processed)?);
    Ok(())
}

fn navigate_mut<'a>(
    value: &'a mut serde_json::Value,
    parts: &[&str],
) -> Option<&'a mut serde_json::Value> {
    if parts.is_empty() {
        return Some(value);
    }

    let (head, tail) = parts.split_first()?;
    match value {
        serde_json::Value::Object(map) => {
            let child = map.get_mut(*head)?;
            navigate_mut(child, tail)
        }
        _ => None,
    }
}

fn execute_pipeline(
    input: &[u8],
    pipeline: &CodecPipeline,
    direction: CodecDirection,
) -> Result<Vec<u8>, String> {
    let enabled_steps: Vec<&CodecStep> = pipeline
        .steps
        .iter()
        .filter(|step| step.enabled)
        .collect();

    let steps: Vec<&CodecStep> = match direction {
        CodecDirection::Decode => enabled_steps,
        CodecDirection::Encode => enabled_steps.into_iter().rev().collect(),
    };

    let mut current = input.to_vec();
    for step in steps {
        current = execute_step(step, &current, direction)?;
    }
    Ok(current)
}

fn execute_step(
    step: &CodecStep,
    input: &[u8],
    direction: CodecDirection,
) -> Result<Vec<u8>, String> {
    match step.step_type {
        CodecStepType::Builtin => {
            execute_builtin_codec(&step.codec, input, direction, &step.config)
        }
        CodecStepType::Plugin => Err(format!(
            "plugin codec '{}' is not yet implemented",
            step.plugin_id.as_deref().unwrap_or("unknown")
        )),
    }
}

fn is_http_message(content: &str) -> bool {
    let first_line = content.lines().next().unwrap_or("");
    let is_request = matches!(
        first_line.split_whitespace().next(),
        Some("GET") | Some("POST") | Some("PUT") | Some("DELETE") | Some("PATCH") | Some("HEAD")
            | Some("OPTIONS")
    );
    let is_response = first_line.starts_with("HTTP/");

    if !is_request && !is_response {
        return false;
    }

    content
        .split("\n\n")
        .next()
        .map(|header_section| header_section.contains(':'))
        .unwrap_or(false)
}

fn split_http_message(content: &str) -> (String, String) {
    if let Some(pos) = content.find("\n\n") {
        (
            content[..pos].to_string(),
            content[pos + 2..].to_string(),
        )
    } else {
        (String::new(), content.to_string())
    }
}

fn extract_body(content: &str) -> String {
    if is_http_message(content) {
        split_http_message(content).1
    } else {
        content.to_string()
    }
}

fn reassemble_with_body(content: &str, body: &str) -> Result<String, String> {
    if is_http_message(content) {
        let (headers, _) = split_http_message(content);
        Ok(format!("{headers}\n\n{body}"))
    } else {
        Ok(body.to_string())
    }
}

fn bytes_to_string(bytes: Vec<u8>) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|error| format!("invalid UTF-8 output: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_rule(id: &str, order: i32, host: &str) -> TrafficCodecRule {
        TrafficCodecRule {
            id: id.to_string(),
            name: id.to_string(),
            enabled: true,
            order,
            match_rule: super::super::types::CodecMatchRule {
                hosts: vec![host.to_string()],
                paths: vec![],
                methods: vec![],
                content_types: vec![],
            },
            scope: CodecScope {
                target: CodecScopeTarget::FullBody,
                fields: vec![],
                header_name: None,
                pattern: None,
            },
            pipeline: CodecPipeline {
                steps: vec![CodecStep {
                    id: "step-1".to_string(),
                    step_type: CodecStepType::Builtin,
                    codec: "base64".to_string(),
                    plugin_id: None,
                    config: HashMap::new(),
                    enabled: true,
                }],
            },
            reversible: true,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn decode_applies_matching_rules_in_order() {
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            b"hello",
        );
        let engine = TrafficCodecEngine::new(vec![sample_rule("rule-1", 1, "*.example.com")]);
        let meta = CodecRequestMeta {
            host: "api.example.com".to_string(),
            path: "/".to_string(),
            method: "POST".to_string(),
            content_type: String::new(),
        };

        let result = engine.decode(&encoded, &meta);
        assert!(result.success);
        assert_eq!(result.content, "hello");
        assert_eq!(result.applied_rule_ids, vec!["rule-1".to_string()]);
    }

    #[test]
    fn encode_reverses_pipeline_steps() {
        let engine = TrafficCodecEngine::new(vec![sample_rule("rule-1", 1, "*")]);
        let meta = CodecRequestMeta {
            host: "example.com".to_string(),
            path: "/".to_string(),
            method: "POST".to_string(),
            content_type: String::new(),
        };

        let result = engine.encode("hello", &meta);
        assert!(result.success);
        assert_eq!(
            result.content,
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"hello")
        );
    }

    #[test]
    fn json_field_scope_decodes_selected_field() {
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            b"secret",
        );
        let body = format!(r#"{{"data":"{encoded}","other":"plain"}}"#);
        let rule = TrafficCodecRule {
            id: "json-rule".to_string(),
            name: "json-rule".to_string(),
            enabled: true,
            order: 1,
            match_rule: super::super::types::CodecMatchRule {
                hosts: vec!["*".to_string()],
                paths: vec![],
                methods: vec![],
                content_types: vec![],
            },
            scope: CodecScope {
                target: CodecScopeTarget::JsonField,
                fields: vec!["data".to_string()],
                header_name: None,
                pattern: None,
            },
            pipeline: CodecPipeline {
                steps: vec![CodecStep {
                    id: "step-1".to_string(),
                    step_type: CodecStepType::Builtin,
                    codec: "base64".to_string(),
                    plugin_id: None,
                    config: HashMap::new(),
                    enabled: true,
                }],
            },
            reversible: true,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let engine = TrafficCodecEngine::new(vec![rule]);
        let meta = CodecRequestMeta {
            host: "example.com".to_string(),
            path: "/".to_string(),
            method: "POST".to_string(),
            content_type: "application/json".to_string(),
        };

        let result = engine.decode(&body, &meta);
        assert!(result.success);
        assert!(result.content.contains(r#""data":"secret""#));
        assert!(result.content.contains(r#""other":"plain""#));
    }
}

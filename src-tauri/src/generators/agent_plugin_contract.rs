use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const AGENT_TOOL_CONTRACT_VERSION: &str = "agent-tool/v2";

#[derive(Debug, Clone)]
pub struct SchemaExport {
    pub key: &'static str,
    pub function_name: &'static str,
    pub global_name: &'static str,
}

#[derive(Debug, Clone)]
pub struct AgentToolContract {
    pub version: &'static str,
    pub schema_exports: &'static [SchemaExport],
}

const AGENT_SCHEMA_EXPORTS: &[SchemaExport] = &[
    SchemaExport {
        key: "input",
        function_name: "get_input_schema",
        global_name: "get_input_schema",
    },
    SchemaExport {
        key: "output",
        function_name: "get_output_schema",
        global_name: "get_output_schema",
    },
];

pub const AGENT_TOOL_CONTRACT: AgentToolContract = AgentToolContract {
    version: AGENT_TOOL_CONTRACT_VERSION,
    schema_exports: AGENT_SCHEMA_EXPORTS,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPluginDefinition {
    pub contract_version: String,
    pub schemas: BTreeMap<String, Value>,
    #[serde(default)]
    pub helper_code: String,
    pub analyze_body: String,
}

#[derive(Debug, Clone)]
pub struct AgentPluginRenderContext {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub category: String,
    pub default_severity: String,
    pub tags: Vec<String>,
    pub description: String,
}

pub fn agent_contract_generation_instructions() -> String {
    let schema_keys = AGENT_TOOL_CONTRACT
        .schema_exports
        .iter()
        .map(|export| format!("\"{}\"", export.key))
        .collect::<Vec<_>>()
        .join(", ");
    let functions = AGENT_TOOL_CONTRACT
        .schema_exports
        .iter()
        .map(|export| export.function_name)
        .chain(std::iter::once("analyze"))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r#"## Agent Tool Contract

The current plugin contract is `{version}`.

Do not generate a complete TypeScript plugin file. Return exactly one JSON object with this camelCase shape:

```json
{{
  "contractVersion": "{version}",
  "schemas": {{
    {schema_keys}: "JSON Schema object for each required schema key"
  }},
  "helperCode": "Optional TypeScript helper code without exports and without globalThis assignments.",
  "analyzeBody": "The TypeScript body of async analyze(input: ToolInput): Promise<ToolOutput>."
}}
```

Contract rules:
1. `schemas` must contain exactly these required keys: {schema_keys}.
2. The renderer will generate these exports from the contract: {functions}.
3. Do not include `export`, `globalThis`, `get_input_schema`, `get_output_schema`, or `analyze` declarations in `helperCode`.
4. `analyzeBody` must return `ToolOutput` with `success`, optional `data`, and optional `error`.
5. `schemas.output` must describe the complete `ToolOutput` object returned by `analyzeBody`.
6. Every top-level property in every schema must include a non-empty `description`.
7. Return plain JSON only. Do not wrap it in markdown.
"#,
        version = AGENT_TOOL_CONTRACT.version,
        schema_keys = schema_keys,
        functions = functions
    )
}

pub fn render_agent_plugin_definition(
    definition: AgentPluginDefinition,
    context: &AgentPluginRenderContext,
) -> Result<String> {
    validate_definition(&definition)?;
    validate_helper_code(&definition.helper_code)?;

    let mut rendered = String::new();
    rendered.push_str("/**\n");
    rendered.push_str(" * Tool Plugin\n");
    rendered.push_str(&format!(" * @plugin {}\n", context.plugin_id));
    rendered.push_str(&format!(" * @name {}\n", context.name));
    rendered.push_str(&format!(" * @version {}\n", context.version));
    rendered.push_str(&format!(" * @author {}\n", context.author));
    rendered.push_str(&format!(" * @category {}\n", context.category));
    rendered.push_str(&format!(
        " * @default_severity {}\n",
        context.default_severity
    ));
    rendered.push_str(&format!(" * @tags {}\n", context.tags.join(", ")));
    rendered.push_str(&format!(" * @description {}\n", context.description));
    rendered.push_str(" */\n\n");
    rendered.push_str(&format!(
        "const PLUGIN_CONTRACT_VERSION = \"{}\";\n\n",
        AGENT_TOOL_CONTRACT.version
    ));

    let helper_code = definition.helper_code.trim();
    if !helper_code.is_empty() {
        rendered.push_str(helper_code);
        rendered.push_str("\n\n");
    }

    rendered.push_str("interface ToolInput {\n    [key: string]: any;\n}\n\n");
    rendered.push_str("interface ToolOutput {\n    success: boolean;\n    data?: any;\n    error?: string;\n}\n\n");

    for export in AGENT_TOOL_CONTRACT.schema_exports {
        let schema = definition
            .schemas
            .get(export.key)
            .ok_or_else(|| anyhow!("Missing schema key: {}", export.key))?;
        let schema_text = serde_json::to_string_pretty(schema)?;
        rendered.push_str(&format!(
            "export function {}() {{\n    return {};\n}}\n\n",
            export.function_name, schema_text
        ));
    }

    rendered.push_str("export async function analyze(input: ToolInput): Promise<ToolOutput> {\n");
    rendered.push_str(&indent_body(&definition.analyze_body));
    rendered.push_str("\n}\n\n");

    for export in AGENT_TOOL_CONTRACT.schema_exports {
        rendered.push_str(&format!(
            "globalThis.{} = {};\n",
            export.global_name, export.function_name
        ));
    }
    rendered.push_str("globalThis.analyze = analyze;\n");

    Ok(rendered)
}

pub fn parse_agent_plugin_definition(response: &str) -> Result<AgentPluginDefinition> {
    let json_text = extract_json_object(response)
        .ok_or_else(|| anyhow!("AI response did not contain a plugin definition JSON object"))?;
    serde_json::from_str::<AgentPluginDefinition>(&json_text)
        .map_err(|error| anyhow!("Failed to parse plugin definition JSON: {error}"))
}

pub fn validate_agent_plugin_source_contract(code: &str) -> Vec<String> {
    let mut errors = Vec::new();

    if !code.contains(&format!(
        "PLUGIN_CONTRACT_VERSION = \"{}\"",
        AGENT_TOOL_CONTRACT.version
    )) {
        errors.push(format!(
            "missing contract version declaration: {}",
            AGENT_TOOL_CONTRACT.version
        ));
    }

    for export in AGENT_TOOL_CONTRACT.schema_exports {
        if !code.contains(&format!("export function {}", export.function_name)) {
            errors.push(format!("missing export function {}", export.function_name));
        }
        if !code.contains(&format!("globalThis.{}", export.global_name)) {
            errors.push(format!("missing globalThis.{} binding", export.global_name));
        }
    }

    if !code.contains("export async function analyze") {
        errors.push("missing export async function analyze".to_string());
    }
    if !code.contains("globalThis.analyze") {
        errors.push("missing globalThis.analyze binding".to_string());
    }

    errors
}

pub fn validate_schema_object(schema: &Value, schema_key: &str) -> Vec<String> {
    let mut errors = Vec::new();

    if schema.get("type").and_then(Value::as_str) != Some("object") {
        errors.push(format!("schemas.{schema_key} must have type \"object\""));
    }

    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        errors.push(format!("schemas.{schema_key} must define object properties"));
        return errors;
    };

    for (property_name, property_schema) in properties {
        if property_schema
            .get("description")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|description| !description.is_empty())
            .is_none()
        {
            errors.push(format!(
                "schemas.{schema_key}.properties.{property_name} must include description"
            ));
        }
    }

    errors
}

fn validate_definition(definition: &AgentPluginDefinition) -> Result<()> {
    if definition.contract_version != AGENT_TOOL_CONTRACT.version {
        return Err(anyhow!(
            "Unsupported contractVersion: expected {}, got {}",
            AGENT_TOOL_CONTRACT.version,
            definition.contract_version
        ));
    }

    for export in AGENT_TOOL_CONTRACT.schema_exports {
        let schema = definition
            .schemas
            .get(export.key)
            .ok_or_else(|| anyhow!("Missing schemas.{}", export.key))?;
        let schema_errors = validate_schema_object(schema, export.key);
        if !schema_errors.is_empty() {
            return Err(anyhow!(schema_errors.join("; ")));
        }
    }

    let body = definition.analyze_body.trim();
    if body.is_empty() {
        return Err(anyhow!("analyzeBody is required"));
    }
    if !body.contains("return") {
        return Err(anyhow!("analyzeBody must return a ToolOutput object"));
    }

    Ok(())
}

fn validate_helper_code(helper_code: &str) -> Result<()> {
    let forbidden = [
        "export ",
        "globalThis",
        "get_input_schema",
        "get_output_schema",
        "function analyze",
        "const analyze",
        "let analyze",
        "var analyze",
    ];

    for token in forbidden {
        if helper_code.contains(token) {
            return Err(anyhow!("helperCode contains forbidden token: {token}"));
        }
    }

    Ok(())
}

fn indent_body(body: &str) -> String {
    body.lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                format!("    {}", line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_json_object(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
    }

    for fence in ["```json", "```"] {
        if let Some(start) = trimmed.find(fence) {
            let code_start = start + fence.len();
            if let Some(end) = trimmed[code_start..].find("```") {
                let candidate = trimmed[code_start..code_start + end].trim();
                if candidate.starts_with('{') && candidate.ends_with('}') {
                    return Some(candidate.to_string());
                }
            }
        }
    }

    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if start < end {
        Some(trimmed[start..=end].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn render_agent_plugin_definition_generates_contract_exports() {
        let definition = AgentPluginDefinition {
            contract_version: AGENT_TOOL_CONTRACT_VERSION.to_string(),
            schemas: BTreeMap::from([
                (
                    "input".to_string(),
                    json!({
                        "type": "object",
                        "required": ["query"],
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Search query"
                            }
                        }
                    }),
                ),
                (
                    "output".to_string(),
                    json!({
                        "type": "object",
                        "required": ["success"],
                        "properties": {
                            "success": {
                                "type": "boolean",
                                "description": "Whether execution succeeded"
                            },
                            "data": {
                                "type": "object",
                                "description": "Tool result data"
                            },
                            "error": {
                                "type": "string",
                                "description": "Error message when execution fails"
                            }
                        }
                    }),
                ),
            ]),
            helper_code: "const LIMIT = 10;".to_string(),
            analyze_body: "return { success: true, data: { limit: LIMIT, query: input.query } };"
                .to_string(),
        };
        let context = AgentPluginRenderContext {
            plugin_id: "test_tool".to_string(),
            name: "Test Tool".to_string(),
            version: "1.0.0".to_string(),
            author: "Sentinel AI".to_string(),
            category: "recon".to_string(),
            default_severity: "medium".to_string(),
            tags: vec!["test".to_string()],
            description: "Test tool".to_string(),
        };

        let code = render_agent_plugin_definition(definition, &context).unwrap();

        assert!(code.contains("PLUGIN_CONTRACT_VERSION = \"agent-tool/v2\""));
        assert!(code.contains("export function get_input_schema"));
        assert!(code.contains("export function get_output_schema"));
        assert!(code.contains("globalThis.get_input_schema = get_input_schema"));
        assert!(code.contains("globalThis.get_output_schema = get_output_schema"));
        assert!(code.contains("globalThis.analyze = analyze"));
    }
}

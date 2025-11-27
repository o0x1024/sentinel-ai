//! TypeScript 插件代码解析器
//! 
//! 从插件的 TypeScript 代码中提取参数签名、接口定义等信息

use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;

/// 插件参数信息
#[derive(Debug, Clone)]
pub struct PluginParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: Option<String>,
}

/// 从 TypeScript 代码中提取 analyze 函数的参数定义
pub fn extract_parameters_from_code(code: &str) -> Vec<PluginParameter> {
    let mut parameters = Vec::new();

    // 尝试提取自定义接口定义
    if let Some(custom_params) = extract_custom_interface(code) {
        return custom_params;
    }

    // 尝试从函数签名中提取参数
    if let Some(func_params) = extract_from_function_signature(code) {
        return func_params;
    }

    // 如果没有找到具体参数，返回默认的通用参数
    parameters.push(PluginParameter {
        name: "input".to_string(),
        param_type: "object".to_string(),
        required: false,
        description: Some("Plugin input parameters (flexible key-value pairs)".to_string()),
    });

    parameters
}

/// 从自定义接口中提取参数
fn extract_custom_interface(code: &str) -> Option<Vec<PluginParameter>> {
    // 查找 analyze 函数的参数类型
    let func_regex = Regex::new(r"export\s+async\s+function\s+analyze\s*\(\s*(\w+)\s*:\s*(\w+)").ok()?;
    let captures = func_regex.captures(code)?;
    let _param_name = captures.get(1)?.as_str();
    let param_type = captures.get(2)?.as_str();

    let interface_pattern = format!(
        r"(?s)(?:export\s+)?interface\s+{}\s*\{{([^}}]+)\}}",
        regex::escape(param_type)
    );
    let type_pattern = format!(
        r"(?s)(?:export\s+)?type\s+{}\s*=\s*\{{([^}}]+)\}}",
        regex::escape(param_type)
    );
    let interface_regex = Regex::new(&interface_pattern).ok()?;
    let type_regex = Regex::new(&type_pattern).ok()?;
    let interface_body = if let Some(caps) = interface_regex.captures(code) {
        caps.get(1)?.as_str()
    } else if let Some(caps) = type_regex.captures(code) {
        caps.get(1)?.as_str()
    } else {
        return None;
    };

    // 解析接口属性
    let mut parameters = Vec::new();
    let prop_regex = Regex::new(r"(\w+)(\?)?:\s*([^;,\n]+)").ok()?;

    for line in interface_body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }

        if let Some(prop_caps) = prop_regex.captures(trimmed) {
            let name = prop_caps.get(1)?.as_str().to_string();
            let optional = prop_caps.get(2).is_some();
            let raw_type = prop_caps.get(3)?.as_str().trim();
            
            let param_type = normalize_ts_type(raw_type);
            
            // 尝试提取注释作为描述
            let description = extract_comment_before_line(code, trimmed);

            parameters.push(PluginParameter {
                name,
                param_type,
                required: !optional,
                description,
            });
        }
    }

    if parameters.is_empty() {
        None
    } else {
        Some(parameters)
    }
}

/// 从函数签名中直接提取参数（如果有多个参数）
fn extract_from_function_signature(code: &str) -> Option<Vec<PluginParameter>> {
    // 匹配解构形式：analyze({ a, b }: { a: string; b?: number })
    let func_regex = Regex::new(
        r"export\s+async\s+function\s+analyze\s*\(\s*\{([^}]+)\}\s*:\s*\{([^}]+)\}"
    ).ok()?;

    if let Some(captures) = func_regex.captures(code) {
        let params_str = captures.get(1)?.as_str();
        let types_str = captures.get(2)?.as_str();
        return parse_destructured_params(params_str, types_str);
    }

    None
}

/// 解析解构参数
fn parse_destructured_params(params_str: &str, types_str: &str) -> Option<Vec<PluginParameter>> {
    let mut parameters = Vec::new();
    
    // 解析类型定义
    let type_map = parse_type_definitions(types_str);
    
    // 解析参数名称
    let param_regex = Regex::new(r"(\w+)(\s*=\s*[^,]+)?").ok()?;
    for param_match in param_regex.captures_iter(params_str) {
        let name = param_match.get(1)?.as_str().to_string();
        let has_default = param_match.get(2).is_some();
        
        let param_type = type_map.get(&name)
            .cloned()
            .unwrap_or_else(|| "any".to_string());
        
        parameters.push(PluginParameter {
            name: name.clone(),
            param_type: normalize_ts_type(&param_type),
            required: !has_default,
            description: None,
        });
    }
    
    if parameters.is_empty() {
        None
    } else {
        Some(parameters)
    }
}

/// 解析类型定义字符串
fn parse_type_definitions(types_str: &str) -> HashMap<String, String> {
    let mut type_map = HashMap::new();
    
    let type_regex = Regex::new(r"(\w+)(\?)?:\s*([^,;]+)").unwrap();
    for type_match in type_regex.captures_iter(types_str) {
        if let (Some(name), Some(type_str)) = (type_match.get(1), type_match.get(3)) {
            type_map.insert(
                name.as_str().to_string(),
                type_str.as_str().trim().to_string()
            );
        }
    }
    
    type_map
}

/// 标准化 TypeScript 类型到 JSON Schema 类型
fn normalize_ts_type(ts_type: &str) -> String {
    let ts_type = ts_type.trim();
    
    match ts_type {
        "string" => "string".to_string(),
        "number" => "number".to_string(),
        "boolean" => "boolean".to_string(),
        "any" => "any".to_string(),
        t if t.ends_with("[]") => "array".to_string(),
        t if t.starts_with("Array<") => "array".to_string(),
        t if t.starts_with("{") || t.contains("|") => "object".to_string(),
        _ => "object".to_string(),
    }
}

/// 提取行前的注释
fn extract_comment_before_line(code: &str, target_line: &str) -> Option<String> {
    let lines: Vec<&str> = code.lines().collect();
    
    for (i, line) in lines.iter().enumerate() {
        if line.trim() == target_line {
            // 查找上一行是否有注释
            if i > 0 {
                let prev_line = lines[i - 1].trim();
                if let Some(comment) = extract_single_line_comment(prev_line) {
                    return Some(comment);
                }
            }
            break;
        }
    }
    
    None
}

/// 提取单行注释
fn extract_single_line_comment(line: &str) -> Option<String> {
    let comment_regex = Regex::new(r"//\s*(.+)").ok()?;
    if let Some(captures) = comment_regex.captures(line) {
        return Some(captures.get(1)?.as_str().trim().to_string());
    }
    
    // JSDoc 风格
    let jsdoc_regex = Regex::new(r"\*\s*(.+)").ok()?;
    if let Some(captures) = jsdoc_regex.captures(line) {
        let comment = captures.get(1)?.as_str().trim();
        if !comment.starts_with("@") {
            return Some(comment.to_string());
        }
    }
    
    None
}

/// 将参数列表转换为 JSON Schema
pub fn parameters_to_json_schema(parameters: &[PluginParameter]) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    
    for param in parameters {
        let mut param_schema = serde_json::Map::new();
        param_schema.insert("type".to_string(), json!(param.param_type));
        
        if let Some(desc) = &param.description {
            param_schema.insert("description".to_string(), json!(desc));
        }
        
        properties.insert(param.name.clone(), Value::Object(param_schema));
        
        if param.required {
            required.push(param.name.clone());
        }
    }
    
    let mut schema = serde_json::Map::new();
    schema.insert("type".to_string(), json!("object"));
    schema.insert("properties".to_string(), Value::Object(properties));
    
    if !required.is_empty() {
        schema.insert("required".to_string(), json!(required));
    }
    
    Value::Object(schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_custom_interface() {
        let code = r#"
export interface ToolInput {
  url: string;
  method?: string;
  headers?: object;
  timeout?: number;
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
  // ...
}
"#;
        
        let params = extract_parameters_from_code(code);
        assert_eq!(params.len(), 4);
        assert_eq!(params[0].name, "url");
        assert!(params[0].required);
        assert_eq!(params[1].name, "method");
        assert!(!params[1].required);
    }

    #[test]
    fn test_extract_generic_tool_input() {
        let code = r#"
export interface ToolInput {
  [key: string]: any;
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
  // ...
}
"#;
        
        let params = extract_parameters_from_code(code);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "input");
        assert_eq!(params[0].param_type, "object");
    }

    #[test]
    fn test_normalize_ts_type() {
        assert_eq!(normalize_ts_type("string"), "string");
        assert_eq!(normalize_ts_type("number"), "number");
        assert_eq!(normalize_ts_type("string[]"), "array");
        assert_eq!(normalize_ts_type("Array<string>"), "array");
        assert_eq!(normalize_ts_type("Record<string, any>"), "object");
    }
}

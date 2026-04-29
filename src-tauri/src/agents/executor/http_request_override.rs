use std::collections::HashMap;
use std::sync::Arc;

use sentinel_tools::buildin_tools::HttpRequestTool;
use sentinel_tools::dynamic_tool::{DynamicToolDef, ToolExecutor, ToolSource};
use sentinel_tools::ToolServer;

#[derive(Debug, Clone)]
struct ReferencedTrafficRequest {
    id: Option<i64>,
    index: usize,
    url: String,
    method: String,
    request_headers: Option<String>,
    request_body: Option<String>,
}

pub(super) async fn build_http_override_def(
    tool_server: &ToolServer,
    referenced_traffic: &[serde_json::Value],
) -> Option<DynamicToolDef> {
    let http_info = tool_server.get_tool(HttpRequestTool::NAME).await?;
    let http_input_schema = http_info.input_schema.clone();
    let http_description = build_http_description(&http_info.description, referenced_traffic);
    let replay_context = referenced_traffic
        .iter()
        .enumerate()
        .filter_map(|(index, value)| ReferencedTrafficRequest::from_value(index + 1, value))
        .collect::<Vec<_>>();

    let http_executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let replay_context = replay_context.clone();
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::http_request::{HttpRequestArgs, HttpRequestTool};

            let mut patched_args = args;
            patch_http_args_from_referenced_traffic(&mut patched_args, &replay_context)?;

            if let Some(obj) = patched_args.as_object_mut() {
                obj.insert(
                    "enable_large_output_storage".to_string(),
                    serde_json::Value::Bool(true),
                );
            }

            let tool_args: HttpRequestArgs = serde_json::from_value(patched_args)
                .map_err(|e| format!("Invalid arguments: {}", e))?;

            let tool = HttpRequestTool::default();
            let result = tool
                .call(tool_args)
                .await
                .map_err(|e| format!("HTTP request failed: {}", e))?;

            serde_json::to_value(result)
                .map_err(|e| format!("Failed to serialize HTTP result: {}", e))
        })
    });

    Some(DynamicToolDef {
        name: HttpRequestTool::NAME.to_string(),
        description: http_description,
        input_schema: http_input_schema,
        output_schema: None,
        source: ToolSource::Builtin,
        category: "network".to_string(),
        tags: http_info.tags.clone(),
        search_hint: http_info.search_hint.clone(),
        exposure: http_info.exposure.clone(),
        execution_policy: http_info.execution_policy.clone(),
        executor: http_executor,
    })
}

impl ReferencedTrafficRequest {
    fn from_value(index: usize, value: &serde_json::Value) -> Option<Self> {
        let url = string_field(value, "url")?;
        let method = string_field(value, "method").unwrap_or_else(|| "GET".to_string());
        Some(Self {
            id: i64_field(value, "id"),
            index,
            url,
            method,
            request_headers: string_field(value, "request_headers"),
            request_body: string_field(value, "request_body"),
        })
    }
}

fn build_http_description(base: &str, referenced_traffic: &[serde_json::Value]) -> String {
    let references = referenced_traffic
        .iter()
        .enumerate()
        .filter_map(|(index, value)| ReferencedTrafficRequest::from_value(index + 1, value))
        .map(|entry| {
            let id = entry
                .id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            format!(
                "- referenced_traffic_index={} id={} {} {}",
                entry.index, id, entry.method, entry.url
            )
        })
        .collect::<Vec<_>>();

    if references.is_empty() {
        return base.to_string();
    }

    format!(
        "{base}\n\nCurrent user message includes referenced HTTP traffic. To replay a referenced request, pass referenced_traffic_index (1-based) or referenced_traffic_id. The runtime will populate the captured URL, method, request body, and all original request headers, including Cookie. Explicit url/method/body/headers fields override the referenced values.\n{}",
        references.join("\n")
    )
}

fn patch_http_args_from_referenced_traffic(
    args: &mut serde_json::Value,
    referenced_traffic: &[ReferencedTrafficRequest],
) -> Result<(), String> {
    let Some(obj) = args.as_object_mut() else {
        return Ok(());
    };

    let Some(reference) = resolve_referenced_traffic(obj, referenced_traffic)? else {
        return Ok(());
    };

    if is_missing_or_empty(obj.get("url")) {
        obj.insert(
            "url".to_string(),
            serde_json::Value::String(reference.url.clone()),
        );
    }
    if is_missing_or_empty(obj.get("method")) {
        obj.insert(
            "method".to_string(),
            serde_json::Value::String(reference.method.clone()),
        );
    }
    if is_missing_or_empty(obj.get("body")) {
        if let Some(body) = reference.request_body.as_ref() {
            obj.insert("body".to_string(), serde_json::Value::String(body.clone()));
        }
    }

    let mut merged_headers = reference
        .request_headers
        .as_deref()
        .map(parse_stored_headers)
        .unwrap_or_default();

    if let Some(explicit_headers) = obj.get("headers").and_then(|value| value.as_object()) {
        for (name, value) in explicit_headers {
            merged_headers.insert(name.clone(), value_to_header_string(value));
        }
    }

    if !merged_headers.is_empty() {
        obj.insert(
            "headers".to_string(),
            serde_json::Value::Object(
                merged_headers
                    .into_iter()
                    .map(|(name, value)| (name, serde_json::Value::String(value)))
                    .collect(),
            ),
        );
    }

    Ok(())
}

fn resolve_referenced_traffic<'a>(
    args: &serde_json::Map<String, serde_json::Value>,
    referenced_traffic: &'a [ReferencedTrafficRequest],
) -> Result<Option<&'a ReferencedTrafficRequest>, String> {
    if referenced_traffic.is_empty() {
        return Ok(None);
    }

    if let Some(id) = args.get("referenced_traffic_id").and_then(value_to_i64) {
        return referenced_traffic
            .iter()
            .find(|entry| entry.id == Some(id))
            .map(Some)
            .ok_or_else(|| format!("Referenced traffic id {} was not found", id));
    }

    if let Some(index) = args
        .get("referenced_traffic_index")
        .and_then(value_to_usize)
    {
        if index == 0 {
            return Err(
                "referenced_traffic_index is 1-based and must be greater than 0".to_string(),
            );
        }
        return referenced_traffic
            .iter()
            .find(|entry| entry.index == index)
            .map(Some)
            .ok_or_else(|| format!("Referenced traffic index {} was not found", index));
    }

    if let Some(url) = args
        .get("url")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let matches = referenced_traffic
            .iter()
            .filter(|entry| entry.url == url)
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            return Ok(Some(matches[0]));
        }
    }

    if referenced_traffic.len() == 1 && is_missing_or_empty(args.get("url")) {
        return Ok(referenced_traffic.first());
    }

    Ok(None)
}

fn parse_stored_headers(raw: &str) -> HashMap<String, String> {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(entries) = parsed.as_array() {
            return entries
                .iter()
                .filter_map(|entry| {
                    let name = string_field(entry, "name")?;
                    let value = string_field(entry, "value").unwrap_or_default();
                    Some((name, value))
                })
                .collect();
        }

        if let Some(map) = parsed.as_object() {
            return map
                .iter()
                .map(|(name, value)| (name.clone(), value_to_header_string(value)))
                .collect();
        }
    }

    parse_raw_header_lines(raw)
}

fn parse_raw_header_lines(raw: &str) -> HashMap<String, String> {
    let mut headers: HashMap<String, String> = HashMap::new();
    let mut previous_name: Option<String> = None;

    for line in raw
        .split(['\r', '\n'])
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Some(separator_index) = line.find(':') else {
            if let Some(name) = previous_name.as_ref() {
                if let Some(value) = headers.get_mut(name) {
                    value.push('\n');
                    value.push_str(line);
                }
            }
            continue;
        };

        let name = line[..separator_index].trim();
        if name.is_empty() {
            continue;
        }
        let value = line[separator_index + 1..].trim();
        headers.insert(name.to_string(), value.to_string());
        previous_name = Some(name.to_string());
    }

    headers
}

fn string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|field| field.as_str())
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .map(str::to_string)
}

fn i64_field(value: &serde_json::Value, key: &str) -> Option<i64> {
    value.get(key).and_then(value_to_i64)
}

fn value_to_i64(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str()?.trim().parse::<i64>().ok())
}

fn value_to_usize(value: &serde_json::Value) -> Option<usize> {
    value
        .as_u64()
        .and_then(|raw| usize::try_from(raw).ok())
        .or_else(|| value.as_str()?.trim().parse::<usize>().ok())
}

fn value_to_header_string(value: &serde_json::Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

fn is_missing_or_empty(value: Option<&serde_json::Value>) -> bool {
    match value {
        None | Some(serde_json::Value::Null) => true,
        Some(serde_json::Value::String(value)) => value.trim().is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        parse_stored_headers, patch_http_args_from_referenced_traffic, ReferencedTrafficRequest,
    };

    #[test]
    fn patches_http_args_from_referenced_traffic_index() {
        let references = vec![ReferencedTrafficRequest {
            id: Some(42),
            index: 1,
            url: "https://example.test/api".to_string(),
            method: "POST".to_string(),
            request_headers: Some(
                json!({
                    "Cookie": "sid=abc",
                    "X-CSRF-Token": "token"
                })
                .to_string(),
            ),
            request_body: Some("{\"a\":1}".to_string()),
        }];
        let mut args = json!({
            "referenced_traffic_index": 1,
            "headers": {
                "X-CSRF-Token": "override"
            }
        });

        patch_http_args_from_referenced_traffic(&mut args, &references).unwrap();

        assert_eq!(args["url"], "https://example.test/api");
        assert_eq!(args["method"], "POST");
        assert_eq!(args["body"], "{\"a\":1}");
        assert_eq!(args["headers"]["Cookie"], "sid=abc");
        assert_eq!(args["headers"]["X-CSRF-Token"], "override");
    }

    #[test]
    fn parses_raw_cookie_header() {
        let headers = parse_stored_headers("Host: example.test\r\nCookie: sid=abc; theme=dark");

        assert_eq!(
            headers.get("Host").map(String::as_str),
            Some("example.test")
        );
        assert_eq!(
            headers.get("Cookie").map(String::as_str),
            Some("sid=abc; theme=dark")
        );
    }
}

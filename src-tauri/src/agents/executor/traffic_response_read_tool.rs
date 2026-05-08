use std::sync::Arc;

use serde::Deserialize;
use serde_json::json;
use tauri::{AppHandle, Manager};

use sentinel_tools::dynamic_tool::{
    DynamicToolDef, ToolCategory, ToolExecutionPolicy, ToolExecutor, ToolSource,
};

use crate::commands::traffic::TrafficAnalysisState;

pub(super) const TRAFFIC_RESPONSE_READ_TOOL_NAME: &str = "traffic_response_read";

#[derive(Debug, Clone)]
struct ReferencedTrafficLocator {
    id: Option<i64>,
    db_request_id: Option<i64>,
    index: usize,
    url: String,
}

#[derive(Debug, Deserialize)]
struct TrafficResponseReadArgs {
    #[serde(default)]
    referenced_traffic_id: Option<i64>,
    #[serde(default)]
    referenced_traffic_index: Option<usize>,
    #[serde(default = "default_response_variant")]
    variant: String,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug)]
struct ResponseContent {
    id: Option<i64>,
    db_request_id: Option<i64>,
    status_code: i32,
    headers: Option<String>,
    body: String,
    response_size: i64,
}

pub(super) fn build_traffic_response_read_tool(
    app_handle: AppHandle,
    referenced_traffic: &[serde_json::Value],
) -> Option<DynamicToolDef> {
    let locators = referenced_traffic
        .iter()
        .enumerate()
        .filter_map(|(index, value)| ReferencedTrafficLocator::from_value(index + 1, value))
        .collect::<Vec<_>>();
    if locators.is_empty() {
        return None;
    }

    let description = format!(
        "Read the captured response body for HTTP traffic referenced in the current user message. Use referenced_traffic_index or referenced_traffic_id, plus offset/limit for large bodies. This reads history; it does not send a network request.\n{}",
        locators
            .iter()
            .map(|entry| {
                let id = entry
                    .id
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                format!(
                    "- referenced_traffic_index={} id={} {}",
                    entry.index, id, entry.url
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    );
    let input_schema = json!({
        "type": "object",
        "properties": {
            "referenced_traffic_id": {
                "type": "integer",
                "description": "Referenced traffic history id from the current user message."
            },
            "referenced_traffic_index": {
                "type": "integer",
                "description": "1-based referenced traffic index from the current user message."
            },
            "variant": {
                "type": "string",
                "enum": ["original", "edited"],
                "default": "original",
                "description": "Response variant to read."
            },
            "offset": {
                "type": "integer",
                "default": 0,
                "description": "Byte offset into the response body."
            },
            "limit": {
                "type": "integer",
                "default": 8192,
                "description": "Maximum bytes to return."
            }
        },
        "required": []
    });

    let executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let app_handle = app_handle.clone();
        let locators = locators.clone();
        Box::pin(async move {
            let args: TrafficResponseReadArgs = serde_json::from_value(args)
                .map_err(|error| format!("Invalid traffic_response_read arguments: {error}"))?;
            execute_traffic_response_read(&app_handle, &locators, args).await
        })
    });

    Some(DynamicToolDef {
        name: TRAFFIC_RESPONSE_READ_TOOL_NAME.to_string(),
        description,
        input_schema,
        output_schema: None,
        source: ToolSource::Builtin,
        category: ToolCategory::Traffic,
        tags: vec![
            "traffic".to_string(),
            "http".to_string(),
            "response".to_string(),
        ],
        search_hint: Some("Read captured response body for referenced traffic".to_string()),
        exposure: Default::default(),
        execution_policy: ToolExecutionPolicy {
            read_only: true,
            mutating: false,
            concurrency_safe: true,
            requires_permission: false,
            supports_background: false,
        },
        executor,
    })
}

impl ReferencedTrafficLocator {
    fn from_value(index: usize, value: &serde_json::Value) -> Option<Self> {
        Some(Self {
            id: i64_field(value, "id"),
            db_request_id: i64_field(value, "db_request_id"),
            index,
            url: string_field(value, "url")?,
        })
    }
}

async fn execute_traffic_response_read(
    app_handle: &AppHandle,
    locators: &[ReferencedTrafficLocator],
    args: TrafficResponseReadArgs,
) -> Result<serde_json::Value, String> {
    if args.limit == 0 {
        return Err("limit must be greater than 0".to_string());
    }
    let locator = resolve_locator(locators, &args)?;
    let state = app_handle
        .try_state::<TrafficAnalysisState>()
        .ok_or_else(|| "TrafficAnalysisState not initialized".to_string())?;
    let content = load_response_content(&state, locator, args.variant.as_str()).await?;
    let total_length = content.body.len();
    let safe_offset = args.offset.min(total_length);
    let next_offset = safe_offset.saturating_add(args.limit).min(total_length);
    let chunk = content
        .body
        .get(safe_offset..next_offset)
        .unwrap_or_default()
        .to_string();

    Ok(json!({
        "referenced_traffic_index": locator.index,
        "id": content.id,
        "db_request_id": content.db_request_id,
        "variant": args.variant,
        "status_code": content.status_code,
        "response_headers": content.headers,
        "response_size": content.response_size,
        "body": chunk,
        "offset": safe_offset,
        "next_offset": next_offset,
        "total_length": total_length,
        "complete": next_offset >= total_length,
        "truncated": next_offset < total_length
    }))
}

async fn load_response_content(
    state: &tauri::State<'_, TrafficAnalysisState>,
    locator: &ReferencedTrafficLocator,
    variant: &str,
) -> Result<ResponseContent, String> {
    let cache = state.get_history_cache();
    if let Some(id) = locator.id {
        if let Some(record) = cache.get_http_request_by_id(id).await {
            return response_from_history_record(record, variant);
        }
    }

    if let Some(db_request_id) = locator.db_request_id {
        let db = state.get_db_service();
        let record = db
            .get_proxy_request_by_id(db_request_id)
            .await
            .map_err(|error| format!("Failed to load proxy request from database: {error}"))?
            .ok_or_else(|| format!("Proxy request db id {db_request_id} was not found"))?;
        return response_from_db_record(record, variant);
    }

    Err(format!(
        "Referenced traffic index {} has no history id or database id",
        locator.index
    ))
}

fn response_from_history_record(
    record: sentinel_traffic::HttpRequestRecord,
    variant: &str,
) -> Result<ResponseContent, String> {
    match variant {
        "original" => Ok(ResponseContent {
            id: Some(record.id),
            db_request_id: record.db_request_id,
            status_code: record.status_code,
            headers: record.response_headers,
            body: record.response_body.unwrap_or_default(),
            response_size: record.response_size,
        }),
        "edited" => {
            if record.edited_response_headers.is_none()
                && record.edited_response_body.is_none()
                && record.edited_status_code.is_none()
            {
                return Err(format!(
                    "History request {} has no edited response",
                    record.id
                ));
            }
            Ok(ResponseContent {
                id: Some(record.id),
                db_request_id: record.db_request_id,
                status_code: record.edited_status_code.unwrap_or(record.status_code),
                headers: record.edited_response_headers,
                body: record.edited_response_body.unwrap_or_default(),
                response_size: record.response_size,
            })
        }
        _ => Err(format!("Unsupported response variant: {variant}")),
    }
}

fn response_from_db_record(
    record: sentinel_db::ProxyRequestRecord,
    variant: &str,
) -> Result<ResponseContent, String> {
    match variant {
        "original" => Ok(ResponseContent {
            id: None,
            db_request_id: record.id,
            status_code: record.status_code,
            headers: record.response_headers,
            body: record.response_body.unwrap_or_default(),
            response_size: record.response_size,
        }),
        "edited" => Err(format!(
            "Proxy request db id {} has no edited response storage",
            record
                .id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        )),
        _ => Err(format!("Unsupported response variant: {variant}")),
    }
}

fn resolve_locator<'a>(
    locators: &'a [ReferencedTrafficLocator],
    args: &TrafficResponseReadArgs,
) -> Result<&'a ReferencedTrafficLocator, String> {
    if let Some(id) = args.referenced_traffic_id {
        return locators
            .iter()
            .find(|entry| entry.id == Some(id))
            .ok_or_else(|| format!("Referenced traffic id {id} was not found"));
    }

    if let Some(index) = args.referenced_traffic_index {
        if index == 0 {
            return Err(
                "referenced_traffic_index is 1-based and must be greater than 0".to_string(),
            );
        }
        return locators
            .iter()
            .find(|entry| entry.index == index)
            .ok_or_else(|| format!("Referenced traffic index {index} was not found"));
    }

    if locators.len() == 1 {
        return locators
            .first()
            .ok_or_else(|| "Referenced traffic is empty".to_string());
    }

    Err("referenced_traffic_index or referenced_traffic_id is required when multiple traffic items are referenced".to_string())
}

fn default_response_variant() -> String {
    "original".to_string()
}

fn default_limit() -> usize {
    8192
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
    value.get(key).and_then(|field| {
        field
            .as_i64()
            .or_else(|| field.as_str()?.trim().parse().ok())
    })
}

#[cfg(test)]
mod tests {
    use super::{resolve_locator, ReferencedTrafficLocator, TrafficResponseReadArgs};

    fn locator(index: usize, id: i64) -> ReferencedTrafficLocator {
        ReferencedTrafficLocator {
            id: Some(id),
            db_request_id: Some(id + 100),
            index,
            url: format!("https://example.test/{index}"),
        }
    }

    #[test]
    fn resolves_locator_by_index() {
        let locators = vec![locator(1, 10), locator(2, 20)];
        let args = TrafficResponseReadArgs {
            referenced_traffic_id: None,
            referenced_traffic_index: Some(2),
            variant: "original".to_string(),
            offset: 0,
            limit: 8192,
        };

        let resolved = resolve_locator(&locators, &args).unwrap();

        assert_eq!(resolved.id, Some(20));
    }

    #[test]
    fn requires_reference_when_multiple_items_exist() {
        let locators = vec![locator(1, 10), locator(2, 20)];
        let args = TrafficResponseReadArgs {
            referenced_traffic_id: None,
            referenced_traffic_index: None,
            variant: "original".to_string(),
            offset: 0,
            limit: 8192,
        };

        assert!(resolve_locator(&locators, &args).is_err());
    }
}

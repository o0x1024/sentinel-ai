use crate::services::system_agents::TrafficContextExtractionSettings;
use serde_json::{json, Map, Value};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use url::Url;

use sentinel_traffic::HttpRequestRecord;

const MAX_TOP_LEVEL_KEYS: usize = 12;
const MAX_RECENT_SEQUENCE: usize = 6;
const MAX_BASELINE_BODY_LEN: usize = 16_384;

#[derive(Debug, Clone)]
pub struct TrafficContextSnapshot {
    pub cluster_key: String,
    pub sequence_key: String,
    pub action_kind: String,
    pub payload: Value,
}

#[derive(Debug, Clone)]
struct ExtractionMatch {
    configured_key: String,
    matched_key: String,
    source: String,
}

impl ExtractionMatch {
    fn to_value(&self) -> Value {
        json!({
            "configuredKey": self.configured_key,
            "matchedKey": self.matched_key,
            "source": self.source,
        })
    }
}

#[derive(Debug, Clone)]
struct ActionInferenceExplanation {
    kind: String,
    matched_alias: Option<String>,
    source: String,
}

impl ActionInferenceExplanation {
    fn to_value(&self) -> Value {
        json!({
            "kind": self.kind,
            "matchedAlias": self.matched_alias,
            "source": self.source,
        })
    }
}

pub fn build_traffic_context_snapshot(
    record: &HttpRequestRecord,
    recent_sequence: &[String],
    settings: &TrafficContextExtractionSettings,
) -> TrafficContextSnapshot {
    let method = effective_method(record);
    let raw_url = effective_url(record);
    let parsed_url = Url::parse(&raw_url).ok();
    let host = parsed_url
        .as_ref()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_else(|| record.host.clone());
    let path = parsed_url
        .as_ref()
        .map(|url| url.path().to_string())
        .unwrap_or_else(|| raw_url.clone());
    let path_template = derive_path_template(&path);

    let request_headers = parse_json_map(effective_request_headers(record).as_deref());
    let response_headers = parse_json_map(effective_response_headers(record).as_deref());
    let request_body = effective_request_body(record);
    let response_body = effective_response_body(record);
    let query_params = extract_query_params(parsed_url.as_ref());
    let body_params = extract_body_params(request_body.as_deref());
    let (auth_context, auth_matches) =
        extract_auth_context(&request_headers, &query_params, &body_params, settings);
    let (principal_context, principal_matches) =
        extract_principal_context(&request_headers, &query_params, &body_params, settings);
    let (resource_keys, resource_matches) =
        extract_resource_keys(&path, &query_params, &body_params, settings);
    let response_fingerprint = build_response_fingerprint(
        effective_status_code(record),
        &response_headers,
        response_body.as_deref(),
    );
    let body_schema = build_body_schema(request_body.as_deref());
    let action_inference = infer_action_kind(&method, &path, request_body.as_deref(), settings);
    let action_kind = action_inference.kind.clone();
    let sequence_key = format!("{}::{}", host, auth_context.fingerprint);
    let cluster_key =
        build_cluster_key(&host, &method, &path_template, &action_kind, &resource_keys);

    let payload = json!({
        "clusterKey": cluster_key,
        "host": host,
        "pathTemplate": path_template,
        "rawPath": path,
        "method": method,
        "statusCode": effective_status_code(record),
        // requestId in HttpRequestRecord is the in-memory history id, not proxy_requests.id.
        "historyRequestId": record.id,
        "dbRequestId": record.db_request_id,
        "url": raw_url,
        "scheme": record.scheme,
        "httpVersionObserved": record.http_version_observed,
        "timestamp": record.timestamp.to_rfc3339(),
        "wasEdited": record.was_edited,
        "hasRequestBody": request_body.as_ref().map(|body| !body.is_empty()).unwrap_or(false),
        "hasResponseBody": response_body.as_ref().map(|body| !body.is_empty()).unwrap_or(false),
        "responseSize": record.response_size,
        "responseTime": record.response_time,
        "authContext": auth_context.to_value(),
        "principalContext": principal_context,
        "resourceKeys": resource_keys,
        "actionKind": action_kind,
        "contextExtraction": {
            "principalMatches": principal_matches.iter().map(ExtractionMatch::to_value).collect::<Vec<_>>(),
            "resourceMatches": resource_matches.iter().map(ExtractionMatch::to_value).collect::<Vec<_>>(),
            "authHeaderMatches": auth_matches.header_matches.iter().map(ExtractionMatch::to_value).collect::<Vec<_>>(),
            "authTokenMatches": auth_matches.token_matches.iter().map(ExtractionMatch::to_value).collect::<Vec<_>>(),
            "cookieMatches": auth_matches.cookie_matches.iter().map(ExtractionMatch::to_value).collect::<Vec<_>>(),
            "actionInference": action_inference.to_value(),
        },
        "requestFingerprint": {
            "queryKeys": sorted_keys(&query_params),
            "bodySchema": body_schema,
        },
        "baselineRequest": {
            "requestHeaders": request_headers,
            "requestBody": truncate_payload_text(request_body.as_deref()),
            "responseStatus": effective_status_code(record),
            "responseHeaders": response_headers,
            "responseBody": truncate_payload_text(response_body.as_deref()),
        },
        "responseFingerprint": response_fingerprint,
        "recentSequence": recent_sequence.iter().take(MAX_RECENT_SEQUENCE).cloned().collect::<Vec<_>>(),
    });

    TrafficContextSnapshot {
        cluster_key,
        sequence_key,
        action_kind,
        payload,
    }
}

#[derive(Debug, Clone)]
struct AuthContext {
    fingerprint: String,
    header_sources: Vec<String>,
    cookie_keys: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct AuthExtractionMatches {
    header_matches: Vec<ExtractionMatch>,
    token_matches: Vec<ExtractionMatch>,
    cookie_matches: Vec<ExtractionMatch>,
}

impl AuthContext {
    fn to_value(&self) -> Value {
        json!({
            "fingerprint": self.fingerprint,
            "headerSources": self.header_sources,
            "cookieKeys": self.cookie_keys,
        })
    }
}

fn effective_method(record: &HttpRequestRecord) -> String {
    record
        .edited_method
        .clone()
        .unwrap_or_else(|| record.method.clone())
}

fn effective_url(record: &HttpRequestRecord) -> String {
    record
        .edited_url
        .clone()
        .unwrap_or_else(|| record.url.clone())
}

fn effective_status_code(record: &HttpRequestRecord) -> i32 {
    record.edited_status_code.unwrap_or(record.status_code)
}

fn effective_request_headers(record: &HttpRequestRecord) -> Option<String> {
    record
        .edited_request_headers
        .clone()
        .or_else(|| record.request_headers.clone())
}

fn effective_request_body(record: &HttpRequestRecord) -> Option<String> {
    record
        .edited_request_body
        .clone()
        .or_else(|| record.request_body.clone())
}

fn effective_response_headers(record: &HttpRequestRecord) -> Option<String> {
    record
        .edited_response_headers
        .clone()
        .or_else(|| record.response_headers.clone())
}

fn effective_response_body(record: &HttpRequestRecord) -> Option<String> {
    record
        .edited_response_body
        .clone()
        .or_else(|| record.response_body.clone())
}

fn parse_json_map(raw: Option<&str>) -> Map<String, Value> {
    raw.and_then(|text| serde_json::from_str::<Value>(text).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn truncate_payload_text(raw: Option<&str>) -> Option<String> {
    raw.map(|text| text.chars().take(MAX_BASELINE_BODY_LEN).collect())
}

fn extract_query_params(url: Option<&Url>) -> Map<String, Value> {
    let mut map = Map::new();
    let Some(url) = url else {
        return map;
    };
    for (key, value) in url.query_pairs() {
        map.insert(key.to_string(), Value::String(value.to_string()));
    }
    map
}

fn extract_body_params(body: Option<&str>) -> Map<String, Value> {
    let Some(body) = body else {
        return Map::new();
    };
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        return value.as_object().cloned().unwrap_or_default();
    }

    let mut map = Map::new();
    for (key, value) in url::form_urlencoded::parse(body.as_bytes()) {
        map.insert(key.to_string(), Value::String(value.to_string()));
    }
    map
}

fn extract_auth_context(
    headers: &Map<String, Value>,
    query: &Map<String, Value>,
    body: &Map<String, Value>,
    settings: &TrafficContextExtractionSettings,
) -> (AuthContext, AuthExtractionMatches) {
    let mut header_sources = Vec::new();
    let mut cookie_keys = Vec::new();
    let mut fingerprint_sources = Vec::new();
    let mut matches = AuthExtractionMatches::default();

    for header_name in settings.auth_header_keys() {
        if let Some((matched_name, value)) = get_case_insensitive_entry(headers, &header_name) {
            let rendered = render_value(value);
            if !rendered.is_empty() {
                header_sources.push(matched_name.to_string());
                matches.header_matches.push(ExtractionMatch {
                    configured_key: header_name,
                    matched_key: matched_name.to_string(),
                    source: "header".to_string(),
                });
                fingerprint_sources.push(format!(
                    "{}:{}",
                    matched_name.to_ascii_lowercase(),
                    stable_fingerprint(&rendered)
                ));
            }
        }
    }

    if let Some(cookie_value) = get_case_insensitive(headers, "cookie").map(render_value) {
        for pair in cookie_value.split(';') {
            let name = pair.split('=').next().unwrap_or("").trim();
            if name.is_empty() {
                continue;
            }
            let name_lc = name.to_ascii_lowercase();
            if let Some(hint) = settings
                .cookie_hint_keys()
                .iter()
                .find(|hint| name_lc.contains(hint.as_str()))
            {
                cookie_keys.push(name.to_string());
                matches.cookie_matches.push(ExtractionMatch {
                    configured_key: hint.clone(),
                    matched_key: name.to_string(),
                    source: "cookie".to_string(),
                });
                fingerprint_sources.push(format!(
                    "cookie:{}:{}",
                    name_lc,
                    stable_fingerprint(pair.trim())
                ));
            }
        }
    }

    for key in settings.auth_token_keys() {
        if let Some((matched_key, value)) =
            get_normalized_value(query, &key).or_else(|| get_normalized_value(body, &key))
        {
            let rendered = render_value(value);
            if !rendered.is_empty() {
                matches.token_matches.push(ExtractionMatch {
                    configured_key: key,
                    matched_key: matched_key.to_string(),
                    source: if query.contains_key(matched_key) {
                        "query".to_string()
                    } else {
                        "body".to_string()
                    },
                });
                fingerprint_sources.push(format!(
                    "{}:{}",
                    matched_key,
                    stable_fingerprint(&rendered)
                ));
            }
        }
    }

    if fingerprint_sources.is_empty() {
        fingerprint_sources.push("anonymous".to_string());
    }

    (
        AuthContext {
            fingerprint: stable_fingerprint(&fingerprint_sources.join("|")),
            header_sources,
            cookie_keys,
        },
        matches,
    )
}

fn extract_principal_context(
    headers: &Map<String, Value>,
    query: &Map<String, Value>,
    body: &Map<String, Value>,
    settings: &TrafficContextExtractionSettings,
) -> (Value, Vec<ExtractionMatch>) {
    let mut values = Map::new();
    let mut matches = Vec::new();
    for key in settings.principal_keys() {
        if let Some((matched_key, value, source)) = get_normalized_value(query, &key)
            .map(|(matched_key, value)| (matched_key, value, "query"))
            .or_else(|| {
                get_normalized_value(body, &key)
                    .map(|(matched_key, value)| (matched_key, value, "body"))
            })
            .or_else(|| {
                get_case_insensitive_entry(headers, &key)
                    .map(|(matched_key, value)| (matched_key, value, "header"))
            })
        {
            let rendered = render_value(value);
            if !rendered.is_empty() {
                if !values.contains_key(matched_key) {
                    matches.push(ExtractionMatch {
                        configured_key: key,
                        matched_key: matched_key.to_string(),
                        source: source.to_string(),
                    });
                }
                values.insert(matched_key.to_string(), Value::String(rendered));
            }
        }
    }
    (Value::Object(values), matches)
}

fn extract_resource_keys(
    path: &str,
    query: &Map<String, Value>,
    body: &Map<String, Value>,
    settings: &TrafficContextExtractionSettings,
) -> (Value, Vec<ExtractionMatch>) {
    let mut values = Map::new();
    let mut matches = Vec::new();

    let path_ids = path
        .split('/')
        .filter(|segment| is_dynamic_identifier(segment))
        .map(|segment| Value::String(segment.to_string()))
        .collect::<Vec<_>>();
    if !path_ids.is_empty() {
        values.insert("pathSegments".to_string(), Value::Array(path_ids));
        matches.push(ExtractionMatch {
            configured_key: "dynamic_path_segment".to_string(),
            matched_key: "pathSegments".to_string(),
            source: "path".to_string(),
        });
    }

    for key in settings.resource_key_hints() {
        if let Some((matched_key, value, source)) = get_normalized_value(query, &key)
            .map(|(matched_key, value)| (matched_key, value, "query"))
            .or_else(|| {
                get_normalized_value(body, &key)
                    .map(|(matched_key, value)| (matched_key, value, "body"))
            })
        {
            let rendered = render_value(value);
            if !rendered.is_empty() {
                if !values.contains_key(matched_key) {
                    matches.push(ExtractionMatch {
                        configured_key: key,
                        matched_key: matched_key.to_string(),
                        source: source.to_string(),
                    });
                }
                values.insert(matched_key.to_string(), Value::String(rendered));
            }
        }
    }

    (Value::Object(values), matches)
}

fn build_response_fingerprint(
    status_code: i32,
    headers: &Map<String, Value>,
    body: Option<&str>,
) -> Value {
    let content_type = get_case_insensitive(headers, "content-type")
        .map(render_value)
        .unwrap_or_default();
    let body_kind = infer_body_kind(body, &content_type);
    let top_level_keys = body
        .and_then(|body| serde_json::from_str::<Value>(body).ok())
        .and_then(|value| value.as_object().cloned())
        .map(|object| {
            object
                .keys()
                .take(MAX_TOP_LEVEL_KEYS)
                .cloned()
                .map(Value::String)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    json!({
        "statusCode": status_code,
        "contentType": content_type,
        "bodyKind": body_kind,
        "topLevelKeys": top_level_keys,
    })
}

fn build_body_schema(body: Option<&str>) -> Value {
    let Some(body) = body else {
        return Value::Null;
    };
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        return describe_json_shape(&value);
    }
    Value::String("raw_text".to_string())
}

fn describe_json_shape(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Array(
            map.keys()
                .take(MAX_TOP_LEVEL_KEYS)
                .cloned()
                .map(Value::String)
                .collect(),
        ),
        Value::Array(items) => Value::String(format!("array[{}]", items.len())),
        Value::String(_) => Value::String("string".to_string()),
        Value::Number(_) => Value::String("number".to_string()),
        Value::Bool(_) => Value::String("boolean".to_string()),
        Value::Null => Value::String("null".to_string()),
    }
}

fn infer_action_kind(
    method: &str,
    path: &str,
    body: Option<&str>,
    settings: &TrafficContextExtractionSettings,
) -> ActionInferenceExplanation {
    let path_lc = path.to_ascii_lowercase();
    let body_lc = body.unwrap_or_default().to_ascii_lowercase();
    for (action, aliases) in settings.ordered_action_aliases() {
        if contains_any(&path_lc, &aliases) {
            return ActionInferenceExplanation {
                kind: action,
                matched_alias: aliases.into_iter().find(|alias| path_lc.contains(alias)),
                source: "path".to_string(),
            };
        }
        if action == "search" && body_lc.contains("keyword") {
            return ActionInferenceExplanation {
                kind: action,
                matched_alias: Some("keyword".to_string()),
                source: "body".to_string(),
            };
        }
    }

    match method {
        "POST" => {
            if looks_like_create_path(&path_lc) {
                ActionInferenceExplanation {
                    kind: "create".to_string(),
                    matched_alias: None,
                    source: "method_fallback".to_string(),
                }
            } else {
                ActionInferenceExplanation {
                    kind: "invoke".to_string(),
                    matched_alias: None,
                    source: "method_fallback".to_string(),
                }
            }
        }
        "PUT" | "PATCH" => ActionInferenceExplanation {
            kind: "update".to_string(),
            matched_alias: None,
            source: "method_fallback".to_string(),
        },
        "DELETE" => ActionInferenceExplanation {
            kind: "delete".to_string(),
            matched_alias: None,
            source: "method_fallback".to_string(),
        },
        _ => ActionInferenceExplanation {
            kind: "read".to_string(),
            matched_alias: None,
            source: "method_fallback".to_string(),
        },
    }
}

fn contains_any(text: &str, needles: &[String]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn looks_like_create_path(path: &str) -> bool {
    if path.ends_with("/create") || path.ends_with("/draft") {
        return true;
    }

    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let Some(last_segment) = segments.last() else {
        return false;
    };

    if matches!(*last_segment, "api" | "v1" | "v2" | "v3") {
        return false;
    }

    if looks_like_identifier(last_segment) {
        return false;
    }

    is_plural_resource_name(last_segment)
}

fn looks_like_identifier(segment: &str) -> bool {
    let compact = segment.trim();
    if compact.is_empty() {
        return false;
    }
    compact.chars().all(|ch| ch.is_ascii_digit())
        || compact.contains('-')
        || compact.contains('_')
        || compact.len() > 12
}

fn is_plural_resource_name(segment: &str) -> bool {
    let normalized = segment.trim_matches('/');
    normalized.ends_with('s') && normalized.len() > 2
}

fn infer_body_kind(body: Option<&str>, content_type: &str) -> &'static str {
    if content_type.contains("json") {
        return "json";
    }
    if content_type.contains("html") {
        return "html";
    }
    if content_type.contains("xml") {
        return "xml";
    }
    let Some(body) = body else {
        return "empty";
    };
    let trimmed = body.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        "json"
    } else if trimmed.starts_with("<!DOCTYPE") || trimmed.starts_with("<html") {
        "html"
    } else {
        "text"
    }
}

fn derive_path_template(path: &str) -> String {
    let segments = path
        .split('/')
        .map(|segment| {
            if is_dynamic_identifier(segment) {
                "{id}".to_string()
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>();
    segments.join("/")
}

fn build_cluster_key(
    host: &str,
    method: &str,
    path_template: &str,
    action_kind: &str,
    resource_keys: &Value,
) -> String {
    let resource_shape = resource_keys
        .as_object()
        .map(|items| {
            let mut keys = items.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            if keys.is_empty() {
                "none".to_string()
            } else {
                keys.join("+")
            }
        })
        .unwrap_or_else(|| "none".to_string());
    format!(
        "{} {} {} action:{} resources:{}",
        host, method, path_template, action_kind, resource_shape
    )
}

fn is_dynamic_identifier(segment: &str) -> bool {
    if segment.is_empty() {
        return false;
    }
    if segment.chars().all(|ch| ch.is_ascii_digit()) {
        return true;
    }
    let normalized = segment.trim_matches(|ch: char| ch == '{' || ch == '}');
    let looks_like_uuid =
        normalized.len() >= 16 && normalized.chars().filter(|&ch| ch == '-').count() >= 2;
    let has_digits = normalized.chars().any(|ch| ch.is_ascii_digit());
    let long_token = normalized.len() >= 12 && has_digits;
    looks_like_uuid || long_token
}

fn get_case_insensitive<'a>(map: &'a Map<String, Value>, target: &str) -> Option<&'a Value> {
    get_case_insensitive_entry(map, target).map(|(_, value)| value)
}

fn get_case_insensitive_entry<'a>(
    map: &'a Map<String, Value>,
    target: &str,
) -> Option<(&'a str, &'a Value)> {
    map.iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(target))
        .map(|(key, value)| (key.as_str(), value))
}

fn get_normalized_value<'a>(
    map: &'a Map<String, Value>,
    target: &str,
) -> Option<(&'a str, &'a Value)> {
    if let Some((key, value)) = map.iter().find(|(key, _)| key.as_str() == target) {
        return Some((key.as_str(), value));
    }

    let target_key = normalize_lookup_key(target);
    if target_key.is_empty() {
        return None;
    }

    map.iter()
        .find(|(key, _)| normalize_lookup_key(key) == target_key)
        .map(|(key, value)| (key.as_str(), value))
}

fn normalize_lookup_key(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn render_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Array(items) => items.iter().map(render_value).collect::<Vec<_>>().join(","),
        Value::Object(_) => value.to_string(),
        Value::Null => String::new(),
    }
}

fn sorted_keys(map: &Map<String, Value>) -> Vec<String> {
    let mut keys = map.keys().cloned().collect::<Vec<_>>();
    keys.sort();
    keys
}

fn stable_fingerprint(input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::build_traffic_context_snapshot;
    use crate::services::system_agents::TrafficContextExtractionSettings;
    use chrono::Utc;
    use sentinel_traffic::HttpRequestRecord;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn build_record(url: &str, headers: &str, body: &str) -> HttpRequestRecord {
        HttpRequestRecord {
            id: 1,
            db_request_id: Some(101),
            traffic_request_id: None,
            origin_kind: None,
            origin_ref_id: None,
            parent_request_id: None,
            source_draft_revision_id: None,
            url: url.to_string(),
            host: "api.example.com".to_string(),
            scheme: "https".to_string(),
            http_version_observed: Some("HTTP/1.1".to_string()),
            method: "POST".to_string(),
            status_code: 200,
            request_headers: Some(headers.to_string()),
            request_body: Some(body.to_string()),
            response_headers: None,
            response_body: None,
            response_size: 0,
            response_time: 12,
            timestamp: Utc::now(),
            was_edited: false,
            edited_request_headers: None,
            edited_request_body: None,
            edited_method: None,
            edited_url: None,
            edited_response_headers: None,
            edited_response_body: None,
            edited_status_code: None,
        }
    }

    #[test]
    fn custom_settings_extract_custom_identity_resource_and_action_aliases() {
        let mut action_aliases = BTreeMap::new();
        action_aliases.insert("complete".to_string(), vec!["finalize".to_string()]);
        let settings = TrafficContextExtractionSettings {
            principal_keys: vec!["operatorCode".to_string()],
            resource_key_hints: vec!["caseRef".to_string()],
            auth_header_keys: vec!["x-tenant-token".to_string()],
            action_aliases,
            ..TrafficContextExtractionSettings::default()
        }
        .sanitized();
        let record = build_record(
            "https://api.example.com/workflows/finalize?case-ref=CASE-9",
            r#"{"X-Tenant-Token":"tenant-secret"}"#,
            r#"{"operator_code":"op-7"}"#,
        );

        let snapshot = build_traffic_context_snapshot(&record, &[], &settings);

        assert_eq!(snapshot.action_kind, "complete");
        assert_eq!(
            snapshot.payload.get("principalContext"),
            Some(&json!({"operator_code":"op-7"}))
        );
        assert_eq!(
            snapshot.payload.get("resourceKeys"),
            Some(&json!({"case-ref":"CASE-9"}))
        );
        assert_eq!(
            snapshot
                .payload
                .get("authContext")
                .and_then(|value| value.get("headerSources")),
            Some(&json!(["X-Tenant-Token"]))
        );
        assert_eq!(
            snapshot.payload.get("contextExtraction"),
            Some(&json!({
                "principalMatches": [
                    {
                        "configuredKey": "operatorCode",
                        "matchedKey": "operator_code",
                        "source": "body",
                    }
                ],
                "resourceMatches": [
                    {
                        "configuredKey": "caseRef",
                        "matchedKey": "case-ref",
                        "source": "query",
                    }
                ],
                "authHeaderMatches": [
                    {
                        "configuredKey": "x-tenant-token",
                        "matchedKey": "X-Tenant-Token",
                        "source": "header",
                    }
                ],
                "authTokenMatches": [],
                "cookieMatches": [],
                "actionInference": {
                    "kind": "complete",
                    "matchedAlias": "finalize",
                    "source": "path",
                },
            }))
        );
    }
}

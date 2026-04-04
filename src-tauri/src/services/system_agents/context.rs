use serde_json::{json, Map, Value};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use url::Url;

use sentinel_traffic::HttpRequestRecord;

const MAX_TOP_LEVEL_KEYS: usize = 12;
const MAX_RECENT_SEQUENCE: usize = 6;
const CANDIDATE_ID_KEYS: &[&str] = &[
    "id",
    "uid",
    "userId",
    "user_id",
    "accountId",
    "account_id",
    "memberId",
    "member_id",
    "orderId",
    "order_id",
    "projectId",
    "project_id",
    "tenantId",
    "tenant_id",
    "orgId",
    "org_id",
    "roleId",
    "role_id",
];
const PRINCIPAL_KEYS: &[&str] = &[
    "userId",
    "user_id",
    "uid",
    "username",
    "email",
    "sub",
    "subject",
    "tenantId",
    "tenant_id",
    "orgId",
    "org_id",
    "role",
    "roleId",
    "role_id",
];
const AUTH_HEADER_KEYS: &[&str] = &[
    "authorization",
    "x-api-key",
    "x-auth-token",
    "x-access-token",
    "x-session-id",
];
const COOKIE_HINT_KEYS: &[&str] = &[
    "session", "sess", "sid", "token", "jwt", "auth", "bearer", "phpessid",
];

#[derive(Debug, Clone)]
pub struct TrafficContextSnapshot {
    pub cluster_key: String,
    pub sequence_key: String,
    pub action_kind: String,
    pub payload: Value,
}

pub fn build_traffic_context_snapshot(
    record: &HttpRequestRecord,
    recent_sequence: &[String],
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
    let auth_context = extract_auth_context(&request_headers, &query_params, &body_params);
    let principal_context =
        extract_principal_context(&request_headers, &query_params, &body_params);
    let resource_keys = extract_resource_keys(&path, &query_params, &body_params);
    let response_fingerprint = build_response_fingerprint(
        effective_status_code(record),
        &response_headers,
        response_body.as_deref(),
    );
    let body_schema = build_body_schema(request_body.as_deref());
    let action_kind = infer_action_kind(&method, &path, request_body.as_deref());
    let sequence_key = format!("{}::{}", host, auth_context.fingerprint);
    let cluster_key = format!("{} {} {}", host, method, path_template);

    let payload = json!({
        "clusterKey": cluster_key,
        "host": host,
        "pathTemplate": path_template,
        "rawPath": path,
        "method": method,
        "statusCode": effective_status_code(record),
        "requestId": record.id,
        "url": raw_url,
        "protocol": record.protocol,
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
        "requestFingerprint": {
            "queryKeys": sorted_keys(&query_params),
            "bodySchema": body_schema,
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
) -> AuthContext {
    let mut header_sources = Vec::new();
    let mut cookie_keys = Vec::new();
    let mut fingerprint_sources = Vec::new();

    for header_name in AUTH_HEADER_KEYS {
        if let Some(value) = get_case_insensitive(headers, header_name) {
            let rendered = render_value(value);
            if !rendered.is_empty() {
                header_sources.push(header_name.to_string());
                fingerprint_sources.push(format!(
                    "{}:{}",
                    header_name,
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
            if COOKIE_HINT_KEYS.iter().any(|hint| name_lc.contains(hint)) {
                cookie_keys.push(name.to_string());
                fingerprint_sources.push(format!(
                    "cookie:{}:{}",
                    name_lc,
                    stable_fingerprint(pair.trim())
                ));
            }
        }
    }

    for key in ["access_token", "token", "session", "session_id"] {
        if let Some(value) = query.get(key).or_else(|| body.get(key)) {
            let rendered = render_value(value);
            if !rendered.is_empty() {
                fingerprint_sources.push(format!("{}:{}", key, stable_fingerprint(&rendered)));
            }
        }
    }

    if fingerprint_sources.is_empty() {
        fingerprint_sources.push("anonymous".to_string());
    }

    AuthContext {
        fingerprint: stable_fingerprint(&fingerprint_sources.join("|")),
        header_sources,
        cookie_keys,
    }
}

fn extract_principal_context(
    headers: &Map<String, Value>,
    query: &Map<String, Value>,
    body: &Map<String, Value>,
) -> Value {
    let mut values = Map::new();
    for key in PRINCIPAL_KEYS {
        if let Some(value) = query
            .get(*key)
            .or_else(|| body.get(*key))
            .or_else(|| get_case_insensitive(headers, key))
        {
            let rendered = render_value(value);
            if !rendered.is_empty() {
                values.insert((*key).to_string(), Value::String(rendered));
            }
        }
    }
    Value::Object(values)
}

fn extract_resource_keys(
    path: &str,
    query: &Map<String, Value>,
    body: &Map<String, Value>,
) -> Value {
    let mut values = Map::new();

    let path_ids = path
        .split('/')
        .filter(|segment| is_dynamic_identifier(segment))
        .map(|segment| Value::String(segment.to_string()))
        .collect::<Vec<_>>();
    if !path_ids.is_empty() {
        values.insert("pathSegments".to_string(), Value::Array(path_ids));
    }

    for key in CANDIDATE_ID_KEYS {
        if let Some(value) = query.get(*key).or_else(|| body.get(*key)) {
            let rendered = render_value(value);
            if !rendered.is_empty() {
                values.insert((*key).to_string(), Value::String(rendered));
            }
        }
    }

    Value::Object(values)
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

fn infer_action_kind(method: &str, path: &str, body: Option<&str>) -> String {
    let path_lc = path.to_ascii_lowercase();
    let body_lc = body.unwrap_or_default().to_ascii_lowercase();
    if path_lc.contains("login") || path_lc.contains("signin") || path_lc.contains("auth") {
        return "authenticate".to_string();
    }
    if path_lc.contains("approve") || path_lc.contains("review") {
        return "approve".to_string();
    }
    if path_lc.contains("pay") || path_lc.contains("payment") {
        return "pay".to_string();
    }
    if path_lc.contains("refund") {
        return "refund".to_string();
    }
    if path_lc.contains("export") || path_lc.contains("download") {
        return "export".to_string();
    }
    if path_lc.contains("search") || body_lc.contains("keyword") {
        return "search".to_string();
    }

    match method {
        "POST" => "create".to_string(),
        "PUT" | "PATCH" => "update".to_string(),
        "DELETE" => "delete".to_string(),
        _ => "read".to_string(),
    }
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
    map.iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(target))
        .map(|(_, value)| value)
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

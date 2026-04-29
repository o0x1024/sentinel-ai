use crate::history_cache::HttpRequestRecord;
use crate::{RequestContext, ResponseContext};

pub fn build_http_history_record(
    req_ctx: &RequestContext,
    resp_ctx: &ResponseContext,
) -> HttpRequestRecord {
    use url::Url;

    let start_time = req_ctx.timestamp;
    let end_time = resp_ctx.timestamp;
    let response_time = (end_time - start_time).num_milliseconds().max(0);

    let parsed_url = Url::parse(&req_ctx.url).ok();
    let host = parsed_url
        .as_ref()
        .and_then(|url| url.host_str())
        .unwrap_or("unknown")
        .to_string();
    let scheme = parsed_url
        .as_ref()
        .map(|url| url.scheme())
        .unwrap_or("http")
        .to_string();

    let request_headers = serde_json::to_string(&req_ctx.headers).ok();
    let response_headers = serde_json::to_string(&resp_ctx.headers).ok();
    let request_body = render_body(&req_ctx.body);
    let response_body = render_body(&resp_ctx.body);
    let response_size = resp_ctx.body.len() as i64;

    let was_edited = req_ctx.was_edited || resp_ctx.was_edited;
    let (edited_method, edited_url, edited_request_headers, edited_request_body) =
        if req_ctx.was_edited {
            (
                req_ctx.edited_method.clone(),
                req_ctx.edited_url.clone(),
                req_ctx.edited_headers.as_ref().map(render_edited_headers),
                req_ctx
                    .edited_body
                    .as_ref()
                    .and_then(|body| String::from_utf8(body.clone()).ok()),
            )
        } else {
            (None, None, None, None)
        };

    let (edited_response_headers, edited_response_body, edited_status_code) = if resp_ctx.was_edited
    {
        (
            resp_ctx.edited_headers.as_ref().map(render_edited_headers),
            resp_ctx
                .edited_body
                .as_ref()
                .and_then(|body| String::from_utf8(body.clone()).ok()),
            resp_ctx.edited_status.map(|status| status as i32),
        )
    } else {
        (None, None, None)
    };

    HttpRequestRecord {
        id: 0,
        db_request_id: None,
        traffic_request_id: Some(req_ctx.id.clone()),
        origin_kind: None,
        origin_ref_id: None,
        parent_request_id: None,
        source_draft_revision_id: None,
        url: req_ctx.url.clone(),
        host,
        scheme,
        http_version_observed: resp_ctx
            .http_version
            .clone()
            .or_else(|| req_ctx.http_version.clone()),
        method: req_ctx.method.clone(),
        status_code: resp_ctx.status as i32,
        request_headers,
        request_body,
        response_headers,
        response_body,
        response_size,
        response_time,
        timestamp: req_ctx.timestamp,
        was_edited,
        edited_request_headers,
        edited_request_body,
        edited_method,
        edited_url,
        edited_response_headers,
        edited_response_body,
        edited_status_code,
    }
}

fn render_body(body: &[u8]) -> Option<String> {
    if body.is_empty() {
        return None;
    }

    match String::from_utf8(body.to_vec()) {
        Ok(text) => Some(text),
        Err(_) => {
            use base64::{engine::general_purpose, Engine as _};
            Some(format!(
                "[BASE64]{}",
                general_purpose::STANDARD.encode(body)
            ))
        }
    }
}

fn render_edited_headers(headers: &std::collections::HashMap<String, String>) -> String {
    headers
        .iter()
        .flat_map(|(name, value)| {
            if !name.eq_ignore_ascii_case("set-cookie") {
                return vec![format!("{}: {}", name, value)];
            }

            value
                .split('\n')
                .map(str::trim)
                .filter(|segment| !segment.is_empty())
                .map(|segment| format!("{}: {}", name, segment))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
        .join("\r\n")
}

#[cfg(test)]
mod tests {
    use super::build_http_history_record;
    use crate::{RequestContext, ResponseContext};
    use chrono::{Duration, Utc};
    use std::collections::HashMap;

    fn request_context() -> RequestContext {
        RequestContext {
            id: "req-1".to_string(),
            method: "POST".to_string(),
            url: "https://example.com/api?debug=true".to_string(),
            http_version: Some("HTTP/1.1".to_string()),
            headers: HashMap::from([("content-type".to_string(), "text/plain".to_string())]),
            body: b"secret".to_vec(),
            content_type: Some("text/plain".to_string()),
            query_params: HashMap::from([("debug".to_string(), "true".to_string())]),
            is_https: true,
            timestamp: Utc::now(),
            was_edited: true,
            edited_method: Some("PUT".to_string()),
            edited_url: Some("https://example.com/v2/api".to_string()),
            edited_headers: Some(HashMap::from([(
                "x-test".to_string(),
                "rewritten".to_string(),
            )])),
            edited_body: Some(b"public".to_vec()),
        }
    }

    fn response_context(timestamp: chrono::DateTime<Utc>) -> ResponseContext {
        ResponseContext {
            request_id: "req-1".to_string(),
            status: 200,
            http_version: Some("HTTP/2".to_string()),
            headers: HashMap::from([("server".to_string(), "nginx".to_string())]),
            body: b"secret-response".to_vec(),
            content_type: Some("text/plain".to_string()),
            timestamp,
            was_edited: true,
            edited_status: Some(201),
            edited_headers: Some(HashMap::from([(
                "server".to_string(),
                "sentinel".to_string(),
            )])),
            edited_body: Some(b"public-response".to_vec()),
        }
    }

    #[test]
    fn builds_history_record_with_edited_fields() {
        let req = request_context();
        let resp = response_context(req.timestamp + Duration::milliseconds(42));

        let record = build_http_history_record(&req, &resp);

        assert!(record.was_edited);
        assert_eq!(record.edited_method.as_deref(), Some("PUT"));
        assert_eq!(
            record.edited_url.as_deref(),
            Some("https://example.com/v2/api")
        );
        assert_eq!(record.edited_request_body.as_deref(), Some("public"));
        assert_eq!(
            record.edited_response_body.as_deref(),
            Some("public-response")
        );
        assert_eq!(record.edited_status_code, Some(201));
        assert_eq!(record.response_time, 42);
        assert_eq!(record.http_version_observed.as_deref(), Some("HTTP/2"));
    }

    #[test]
    fn encodes_binary_bodies_for_history() {
        let mut req = request_context();
        req.was_edited = false;
        req.edited_method = None;
        req.edited_url = None;
        req.edited_headers = None;
        req.edited_body = None;
        req.body = vec![0xff, 0x00, 0x01];

        let mut resp = response_context(req.timestamp + Duration::milliseconds(5));
        resp.was_edited = false;
        resp.edited_status = None;
        resp.edited_headers = None;
        resp.edited_body = None;
        resp.body = vec![0xfe, 0x02, 0x03];

        let record = build_http_history_record(&req, &resp);

        assert!(record
            .request_body
            .as_deref()
            .is_some_and(|body| body.starts_with("[BASE64]")));
        assert!(record
            .response_body
            .as_deref()
            .is_some_and(|body| body.starts_with("[BASE64]")));
    }

    #[test]
    fn marks_history_record_as_edited_when_only_response_was_rewritten() {
        let mut req = request_context();
        req.was_edited = false;
        req.edited_method = None;
        req.edited_url = None;
        req.edited_headers = None;
        req.edited_body = None;

        let resp = response_context(req.timestamp + Duration::milliseconds(9));

        let record = build_http_history_record(&req, &resp);

        assert!(record.was_edited);
        assert!(record.edited_method.is_none());
        assert_eq!(
            record.edited_response_body.as_deref(),
            Some("public-response")
        );
        assert_eq!(record.edited_status_code, Some(201));
    }

    #[test]
    fn serializes_edited_set_cookie_headers_as_repeated_header_lines() {
        let mut req = request_context();
        req.was_edited = false;
        req.edited_method = None;
        req.edited_url = None;
        req.edited_headers = None;
        req.edited_body = None;

        let mut resp = response_context(req.timestamp + Duration::milliseconds(7));
        resp.edited_headers = Some(HashMap::from([(
            "set-cookie".to_string(),
            "session=abc; Path=/; HttpOnly\ntheme=dark; Path=/".to_string(),
        )]));

        let record = build_http_history_record(&req, &resp);

        assert_eq!(
            record.edited_response_headers.as_deref(),
            Some("set-cookie: session=abc; Path=/; HttpOnly\r\nset-cookie: theme=dark; Path=/")
        );
    }
}

use crate::proxy::InterceptFilterRule;
use crate::scope::{url_is_in_scope, ProxyScopeRule};
use crate::{RequestContext, ResponseContext};
use regex::RegexBuilder;
use std::net::IpAddr;
use url::Url;

pub fn should_intercept_response(
    rules: &[InterceptFilterRule],
    req_ctx: &RequestContext,
    resp_ctx: &ResponseContext,
    include_rules: &[ProxyScopeRule],
    exclude_rules: &[ProxyScopeRule],
    request_was_intercepted: bool,
) -> bool {
    let enabled_rules: Vec<&InterceptFilterRule> =
        rules.iter().filter(|rule| rule.enabled).collect();
    if enabled_rules.is_empty() {
        return true;
    }

    let mut result = None;

    for rule in enabled_rules {
        let matched = evaluate_response_rule(
            rule,
            req_ctx,
            resp_ctx,
            include_rules,
            exclude_rules,
            request_was_intercepted,
        );

        result = Some(match result {
            None => matched,
            Some(previous) if rule.operator.eq_ignore_ascii_case("or") => previous || matched,
            Some(previous) => previous && matched,
        });
    }

    result.unwrap_or(true)
}

fn evaluate_response_rule(
    rule: &InterceptFilterRule,
    req_ctx: &RequestContext,
    resp_ctx: &ResponseContext,
    include_rules: &[ProxyScopeRule],
    exclude_rules: &[ProxyScopeRule],
    request_was_intercepted: bool,
) -> bool {
    match rule.relationship.as_str() {
        "matches" => values_for_response_rule(rule.match_type.as_str(), req_ctx, resp_ctx)
            .iter()
            .any(|value| text_matches(&rule.condition, value)),
        "does_not_match" => !values_for_response_rule(rule.match_type.as_str(), req_ctx, resp_ctx)
            .iter()
            .any(|value| text_matches(&rule.condition, value)),
        "contains_parameters" => request_contains_parameters(req_ctx),
        "is_in_target_scope" => url_is_in_scope(&req_ctx.url, include_rules, exclude_rules),
        "was_modified" => match rule.match_type.as_str() {
            "request" => req_ctx.was_edited,
            _ => resp_ctx.was_edited,
        },
        "was_intercepted" => rule.match_type == "request" && request_was_intercepted,
        _ => false,
    }
}

fn values_for_response_rule(
    match_type: &str,
    req_ctx: &RequestContext,
    resp_ctx: &ResponseContext,
) -> Vec<String> {
    match match_type {
        "domain_name" => extract_host(&req_ctx.url).into_iter().collect(),
        "ip_address" => extract_host(&req_ctx.url)
            .filter(|host| host.parse::<IpAddr>().is_ok())
            .into_iter()
            .collect(),
        "protocol" => Some(if req_ctx.is_https { "https" } else { "http" }.to_string())
            .into_iter()
            .collect(),
        "http_method" => vec![req_ctx.method.clone()],
        "url" => vec![req_ctx.url.clone()],
        "file_extension" => extract_file_extension(&req_ctx.url).into_iter().collect(),
        "request" => vec![render_request(req_ctx)],
        "cookie_name" => extract_cookie_names(&resp_ctx.headers),
        "cookie_value" => extract_cookie_values(&resp_ctx.headers),
        "any_header" => render_headers(&resp_ctx.headers),
        "body" => vec![String::from_utf8_lossy(&resp_ctx.body).to_string()],
        "param_name" => req_ctx.query_params.keys().cloned().collect(),
        "param_value" => req_ctx.query_params.values().cloned().collect(),
        "status_code" => vec![resp_ctx.status.to_string()],
        "content_type_header" => resp_ctx
            .content_type
            .clone()
            .or_else(|| find_header(&resp_ctx.headers, "content-type"))
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

fn request_contains_parameters(req_ctx: &RequestContext) -> bool {
    if !req_ctx.query_params.is_empty() {
        return true;
    }

    let body = String::from_utf8_lossy(&req_ctx.body);
    body.contains('=')
}

fn render_request(req_ctx: &RequestContext) -> String {
    let mut lines = vec![format!("{} {}", req_ctx.method, req_ctx.url)];
    lines.extend(render_headers(&req_ctx.headers));

    let body = String::from_utf8_lossy(&req_ctx.body);
    if !body.is_empty() {
        lines.push(String::new());
        lines.push(body.to_string());
    }

    lines.join("\n")
}

fn render_headers(headers: &std::collections::HashMap<String, String>) -> Vec<String> {
    headers
        .iter()
        .flat_map(|(name, value)| [format!("{name}: {value}"), name.clone(), value.clone()])
        .collect()
}

fn extract_cookie_names(headers: &std::collections::HashMap<String, String>) -> Vec<String> {
    extract_cookie_pairs(headers)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

fn extract_cookie_values(headers: &std::collections::HashMap<String, String>) -> Vec<String> {
    extract_cookie_pairs(headers)
        .into_iter()
        .map(|(_, value)| value)
        .collect()
}

fn extract_cookie_pairs(
    headers: &std::collections::HashMap<String, String>,
) -> Vec<(String, String)> {
    let mut pairs = Vec::new();

    for (name, value) in headers {
        if !name.eq_ignore_ascii_case("set-cookie") && !name.eq_ignore_ascii_case("cookie") {
            continue;
        }

        for segment in value.split(';') {
            let Some((raw_name, raw_value)) = segment.split_once('=') else {
                continue;
            };

            let cookie_name = raw_name.trim().trim_start_matches(',').trim().to_string();
            let cookie_value = raw_value.trim().trim_matches(',').trim().to_string();
            if cookie_name.is_empty() || cookie_value.is_empty() {
                continue;
            }

            if is_cookie_attribute(&cookie_name) {
                continue;
            }

            pairs.push((cookie_name, cookie_value));
        }
    }

    pairs
}

fn is_cookie_attribute(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "path"
            | "expires"
            | "max-age"
            | "domain"
            | "secure"
            | "httponly"
            | "samesite"
            | "partitioned"
    )
}

fn find_header(
    headers: &std::collections::HashMap<String, String>,
    expected_name: &str,
) -> Option<String> {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(expected_name))
        .map(|(_, value)| value.clone())
}

fn extract_host(url: &str) -> Option<String> {
    Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(|value| value.to_string()))
}

fn extract_file_extension(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let path = parsed.path();
    let ext = path.rsplit('.').next()?;
    if ext == path {
        return None;
    }
    Some(ext.to_string())
}

fn text_matches(pattern: &str, actual: &str) -> bool {
    let normalized = pattern.trim();
    if normalized.is_empty() {
        return false;
    }

    if actual.eq_ignore_ascii_case(normalized) {
        return true;
    }

    RegexBuilder::new(normalized)
        .case_insensitive(true)
        .build()
        .map(|regex| regex.is_match(actual))
        .unwrap_or_else(|_| {
            actual
                .to_ascii_lowercase()
                .contains(&normalized.to_ascii_lowercase())
        })
}

#[cfg(test)]
mod tests {
    use super::should_intercept_response;
    use crate::proxy::InterceptFilterRule;
    use crate::{RequestContext, ResponseContext};
    use chrono::Utc;
    use std::collections::HashMap;

    fn request_context() -> RequestContext {
        RequestContext {
            id: "req-1".to_string(),
            method: "GET".to_string(),
            url: "https://example.com/api/orders?id=1".to_string(),
            http_version: Some("HTTP/1.1".to_string()),
            headers: HashMap::from([("host".to_string(), "example.com".to_string())]),
            body: Vec::new(),
            content_type: None,
            query_params: HashMap::from([("id".to_string(), "1".to_string())]),
            is_https: true,
            timestamp: Utc::now(),
            was_edited: false,
            edited_method: None,
            edited_url: None,
            edited_headers: None,
            edited_body: None,
        }
    }

    fn response_context() -> ResponseContext {
        ResponseContext {
            request_id: "req-1".to_string(),
            status: 200,
            http_version: Some("HTTP/1.1".to_string()),
            headers: HashMap::from([("content-type".to_string(), "text/html".to_string())]),
            body: b"<html>ok</html>".to_vec(),
            content_type: Some("text/html".to_string()),
            timestamp: Utc::now(),
            was_edited: false,
            edited_status: None,
            edited_headers: None,
            edited_body: None,
        }
    }

    #[test]
    fn response_rules_match_default_text_rule() {
        let req_ctx = request_context();
        let resp_ctx = response_context();
        let rules = vec![InterceptFilterRule {
            enabled: true,
            operator: String::new(),
            match_type: "content_type_header".to_string(),
            relationship: "matches".to_string(),
            condition: "text".to_string(),
        }];

        assert!(should_intercept_response(
            &rules,
            &req_ctx,
            &resp_ctx,
            &[],
            &[],
            false,
        ));
    }

    #[test]
    fn response_rules_respect_request_intercepted_relationship() {
        let req_ctx = request_context();
        let resp_ctx = response_context();
        let rules = vec![InterceptFilterRule {
            enabled: true,
            operator: String::new(),
            match_type: "request".to_string(),
            relationship: "was_intercepted".to_string(),
            condition: String::new(),
        }];

        assert!(!should_intercept_response(
            &rules,
            &req_ctx,
            &resp_ctx,
            &[],
            &[],
            false,
        ));
        assert!(should_intercept_response(
            &rules,
            &req_ctx,
            &resp_ctx,
            &[],
            &[],
            true,
        ));
    }

    #[test]
    fn response_rules_apply_boolean_operators() {
        let req_ctx = request_context();
        let mut resp_ctx = response_context();
        resp_ctx.status = 304;

        let rules = vec![
            InterceptFilterRule {
                enabled: true,
                operator: String::new(),
                match_type: "content_type_header".to_string(),
                relationship: "matches".to_string(),
                condition: "text".to_string(),
            },
            InterceptFilterRule {
                enabled: true,
                operator: "And".to_string(),
                match_type: "status_code".to_string(),
                relationship: "does_not_match".to_string(),
                condition: "^304$".to_string(),
            },
        ];

        assert!(!should_intercept_response(
            &rules,
            &req_ctx,
            &resp_ctx,
            &[],
            &[],
            false,
        ));
    }
}

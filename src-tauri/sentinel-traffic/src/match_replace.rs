use crate::header_utils::merge_header_value;
use crate::scope::{url_is_in_scope, ProxyScopeRule};
use http::Method;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::{form_urlencoded, Url};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatchReplaceRule {
    #[serde(default)]
    pub enabled: bool,
    #[serde(rename = "type", default)]
    pub rule_type: String,
    #[serde(rename = "match", default)]
    pub match_value: String,
    #[serde(rename = "replace", default)]
    pub replace_value: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub item: String,
    #[serde(default)]
    pub comment: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestMatchReplaceResult {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseMatchReplaceResult {
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub body_is_plain_text: bool,
}

struct EditableRequest {
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
    http_version: String,
}

struct EditableResponse {
    headers: HashMap<String, String>,
    body: Vec<u8>,
    body_is_plain_text: bool,
}

pub fn apply_request_match_replace_rules(
    rules: &[MatchReplaceRule],
    method: &str,
    url: &str,
    headers: &HashMap<String, String>,
    body: &[u8],
    http_version: Option<&str>,
    include_rules: &[ProxyScopeRule],
    exclude_rules: &[ProxyScopeRule],
) -> Option<RequestMatchReplaceResult> {
    let mut request = EditableRequest {
        method: method.to_string(),
        url: url.to_string(),
        headers: headers.clone(),
        body: body.to_vec(),
        http_version: http_version.unwrap_or("HTTP/1.1").to_string(),
    };
    let mut changed = false;

    for rule in rules.iter().filter(|rule| rule.enabled) {
        if !rule_applies_to_request(rule, &request.url, include_rules, exclude_rules) {
            continue;
        }

        let rule_changed = match rule.rule_type.as_str() {
            "Request header" => {
                apply_header_rule(&mut request.headers, &rule.match_value, &rule.replace_value)
            }
            "Request body" => {
                apply_text_body_rule(&mut request.body, &rule.match_value, &rule.replace_value)
            }
            "Request param name" => {
                apply_request_param_rule(&mut request, &rule.match_value, &rule.replace_value, true)
            }
            "Request param value" => apply_request_param_rule(
                &mut request,
                &rule.match_value,
                &rule.replace_value,
                false,
            ),
            "Request first line" => {
                apply_request_first_line_rule(&mut request, &rule.match_value, &rule.replace_value)
            }
            _ => false,
        };

        changed |= rule_changed;
    }

    if !changed {
        return None;
    }

    Some(RequestMatchReplaceResult {
        method: request.method,
        url: request.url,
        headers: request.headers,
        body: request.body,
    })
}

pub fn apply_response_match_replace_rules(
    rules: &[MatchReplaceRule],
    request_url: &str,
    headers: &HashMap<String, String>,
    body: &[u8],
    include_rules: &[ProxyScopeRule],
    exclude_rules: &[ProxyScopeRule],
) -> Option<ResponseMatchReplaceResult> {
    let mut response = EditableResponse {
        headers: headers.clone(),
        body: body.to_vec(),
        body_is_plain_text: false,
    };
    let mut changed = false;

    for rule in rules.iter().filter(|rule| rule.enabled) {
        if !rule_applies_to_request(rule, request_url, include_rules, exclude_rules) {
            continue;
        }

        let rule_changed = match rule.rule_type.as_str() {
            "Response header" => apply_header_rule(
                &mut response.headers,
                &rule.match_value,
                &rule.replace_value,
            ),
            "Response body" => {
                apply_text_body_rule(&mut response.body, &rule.match_value, &rule.replace_value)
            }
            _ => false,
        };

        if rule.rule_type == "Response body" && rule_changed {
            response.body_is_plain_text = true;
        }

        changed |= rule_changed;
    }

    if !changed {
        return None;
    }

    Some(ResponseMatchReplaceResult {
        headers: response.headers,
        body: response.body,
        body_is_plain_text: response.body_is_plain_text,
    })
}

fn rule_applies_to_request(
    rule: &MatchReplaceRule,
    url: &str,
    include_rules: &[ProxyScopeRule],
    exclude_rules: &[ProxyScopeRule],
) -> bool {
    if rule.scope.trim().eq_ignore_ascii_case("In scope") {
        return url_is_in_scope(url, include_rules, exclude_rules);
    }

    true
}

fn apply_header_rule(
    headers: &mut HashMap<String, String>,
    match_pattern: &str,
    replace_value: &str,
) -> bool {
    let Some(regex) = compile_regex(match_pattern) else {
        return false;
    };

    let mut changed = false;
    let mut rewritten = HashMap::new();

    for (name, value) in std::mem::take(headers) {
        let original_line = format!("{name}: {value}");
        let replaced_line = regex
            .replace_all(&original_line, replace_value)
            .into_owned();

        if replaced_line == original_line {
            merge_header_value(&mut rewritten, &name, &value);
            continue;
        }

        let trimmed = replaced_line.trim();
        if trimmed.is_empty() {
            changed = true;
            continue;
        }

        let Some((new_name, new_value)) = trimmed.split_once(':') else {
            merge_header_value(&mut rewritten, &name, &value);
            continue;
        };

        let new_name = new_name.trim();
        let new_value = new_value.trim();
        if new_name.is_empty() {
            merge_header_value(&mut rewritten, &name, &value);
            continue;
        }

        merge_header_value(&mut rewritten, new_name, new_value);
        changed = true;
    }

    *headers = rewritten;
    changed
}

fn apply_text_body_rule(body: &mut Vec<u8>, match_pattern: &str, replace_value: &str) -> bool {
    let Some(regex) = compile_regex(match_pattern) else {
        return false;
    };
    let Ok(body_text) = std::str::from_utf8(body) else {
        return false;
    };

    let replaced = regex.replace_all(body_text, replace_value).into_owned();
    if replaced == body_text {
        return false;
    }

    *body = replaced.into_bytes();
    true
}

fn apply_request_param_rule(
    request: &mut EditableRequest,
    match_pattern: &str,
    replace_value: &str,
    rename_key: bool,
) -> bool {
    let Some(regex) = compile_regex(match_pattern) else {
        return false;
    };

    let mut changed = false;

    if let Ok(mut parsed_url) = Url::parse(&request.url) {
        let original_pairs: Vec<(String, String)> = parsed_url.query_pairs().into_owned().collect();
        let rewritten_pairs =
            rewrite_form_pairs(&original_pairs, &regex, replace_value, rename_key);
        if rewritten_pairs != original_pairs {
            let query = serialize_form_pairs(&rewritten_pairs);
            parsed_url.set_query(query.as_deref());
            request.url = parsed_url.to_string();
            changed = true;
        }
    }

    if is_form_urlencoded(&request.headers) {
        let Ok(body_text) = std::str::from_utf8(&request.body) else {
            return changed;
        };

        let original_pairs: Vec<(String, String)> = form_urlencoded::parse(body_text.as_bytes())
            .into_owned()
            .collect();
        let rewritten_pairs =
            rewrite_form_pairs(&original_pairs, &regex, replace_value, rename_key);
        if rewritten_pairs != original_pairs {
            request.body = serialize_form_pairs(&rewritten_pairs)
                .unwrap_or_default()
                .into_bytes();
            changed = true;
        }
    }

    changed
}

fn apply_request_first_line_rule(
    request: &mut EditableRequest,
    match_pattern: &str,
    replace_value: &str,
) -> bool {
    let Some(regex) = compile_regex(match_pattern) else {
        return false;
    };

    let request_target = request_target_from_url(&request.url);
    let first_line = format!(
        "{} {} {}",
        request.method, request_target, request.http_version
    );
    let replaced = regex.replace_all(&first_line, replace_value).into_owned();
    if replaced == first_line {
        return false;
    }

    let mut parts = replaced.split_whitespace();
    let Some(new_method) = parts.next() else {
        return false;
    };
    let Some(new_target) = parts.next() else {
        return false;
    };

    if Method::from_bytes(new_method.as_bytes()).is_err() {
        return false;
    }

    let Some(new_url) = resolve_request_target(&request.url, new_target) else {
        return false;
    };

    request.method = new_method.to_string();
    request.url = new_url;
    true
}

fn rewrite_form_pairs(
    pairs: &[(String, String)],
    regex: &Regex,
    replace_value: &str,
    rename_key: bool,
) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(name, value)| {
            if rename_key {
                (
                    regex.replace_all(name, replace_value).into_owned(),
                    value.clone(),
                )
            } else {
                (
                    name.clone(),
                    regex.replace_all(value, replace_value).into_owned(),
                )
            }
        })
        .collect()
}

fn serialize_form_pairs(pairs: &[(String, String)]) -> Option<String> {
    if pairs.is_empty() {
        return None;
    }

    let mut serializer = form_urlencoded::Serializer::new(String::new());
    for (name, value) in pairs {
        serializer.append_pair(name, value);
    }
    Some(serializer.finish())
}

fn request_target_from_url(url: &str) -> String {
    Url::parse(url)
        .ok()
        .map(|parsed| {
            let path = if parsed.path().is_empty() {
                "/"
            } else {
                parsed.path()
            };

            match parsed.query() {
                Some(query) if !query.is_empty() => format!("{path}?{query}"),
                _ => path.to_string(),
            }
        })
        .unwrap_or_else(|| "/".to_string())
}

fn resolve_request_target(current_url: &str, next_target: &str) -> Option<String> {
    if next_target.starts_with("http://") || next_target.starts_with("https://") {
        return Url::parse(next_target).ok().map(|url| url.to_string());
    }

    let current = Url::parse(current_url).ok()?;
    current.join(next_target).ok().map(|url| url.to_string())
}

fn is_form_urlencoded(headers: &HashMap<String, String>) -> bool {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| {
            value
                .to_ascii_lowercase()
                .contains("application/x-www-form-urlencoded")
        })
        .unwrap_or(false)
}

fn compile_regex(pattern: &str) -> Option<Regex> {
    let normalized = pattern.trim();
    if normalized.is_empty() {
        return None;
    }

    Regex::new(normalized).ok()
}

#[cfg(test)]
mod tests {
    use super::{
        apply_request_match_replace_rules, apply_response_match_replace_rules, MatchReplaceRule,
    };
    use std::collections::HashMap;

    fn request_rule(rule_type: &str, match_value: &str, replace_value: &str) -> MatchReplaceRule {
        MatchReplaceRule {
            enabled: true,
            rule_type: rule_type.to_string(),
            match_value: match_value.to_string(),
            replace_value: replace_value.to_string(),
            scope: String::new(),
            item: String::new(),
            comment: String::new(),
        }
    }

    #[test]
    fn rewrites_request_header_and_body() {
        let headers = HashMap::from([
            ("Host".to_string(), "example.com".to_string()),
            ("User-Agent".to_string(), "curl/8.0".to_string()),
        ]);
        let rules = vec![
            request_rule(
                "Request header",
                r"User-Agent: .*",
                "User-Agent: Mozilla/5.0",
            ),
            request_rule("Request body", "debug=true", "debug=false"),
        ];

        let result = apply_request_match_replace_rules(
            &rules,
            "POST",
            "https://example.com/api",
            &headers,
            br#"debug=true"#,
            Some("HTTP/1.1"),
            &[],
            &[],
        )
        .expect("request should be rewritten");

        assert_eq!(
            result.headers.get("User-Agent"),
            Some(&"Mozilla/5.0".to_string())
        );
        assert_eq!(result.body, br#"debug=false"#);
    }

    #[test]
    fn rewrites_request_query_and_form_parameters() {
        let headers = HashMap::from([
            (
                "Content-Type".to_string(),
                "application/x-www-form-urlencoded".to_string(),
            ),
            ("Host".to_string(), "example.com".to_string()),
        ]);
        let rules = vec![
            request_rule("Request param name", "^token$", "session"),
            request_rule("Request param value", "^abc123$", "rotated"),
        ];

        let result = apply_request_match_replace_rules(
            &rules,
            "POST",
            "https://example.com/api?token=abc123",
            &headers,
            br#"token=abc123"#,
            Some("HTTP/1.1"),
            &[],
            &[],
        )
        .expect("request params should be rewritten");

        assert!(result.url.contains("session=rotated"));
        assert_eq!(result.body, br#"session=rotated"#);
    }

    #[test]
    fn rewrites_request_first_line_target() {
        let rules = vec![request_rule(
            "Request first line",
            r"GET /v1/users\?debug=true HTTP/1\.1",
            "POST /v2/admin HTTP/1.1",
        )];

        let result = apply_request_match_replace_rules(
            &rules,
            "GET",
            "https://example.com/v1/users?debug=true",
            &HashMap::new(),
            &[],
            Some("HTTP/1.1"),
            &[],
            &[],
        )
        .expect("request first line should be rewritten");

        assert_eq!(result.method, "POST");
        assert_eq!(result.url, "https://example.com/v2/admin");
    }

    #[test]
    fn rewrites_response_headers_and_body() {
        let headers = HashMap::from([
            ("Server".to_string(), "nginx".to_string()),
            ("Content-Type".to_string(), "text/plain".to_string()),
        ]);
        let rules = vec![
            request_rule("Response header", r"Server: nginx", "Server: sentinel"),
            request_rule("Response body", "secret", "public"),
        ];

        let result = apply_response_match_replace_rules(
            &rules,
            "https://example.com/api",
            &headers,
            br#"secret payload"#,
            &[],
            &[],
        )
        .expect("response should be rewritten");

        assert_eq!(result.headers.get("Server"), Some(&"sentinel".to_string()));
        assert_eq!(result.body, br#"public payload"#);
        assert!(result.body_is_plain_text);
    }

    #[test]
    fn in_scope_rule_skips_out_of_scope_request() {
        let rules = vec![MatchReplaceRule {
            enabled: true,
            rule_type: "Request body".to_string(),
            match_value: "secret".to_string(),
            replace_value: "public".to_string(),
            scope: "In scope".to_string(),
            item: String::new(),
            comment: String::new(),
        }];

        let include_rules = vec![crate::scope::ProxyScopeRule {
            enabled: true,
            protocol: "https".to_string(),
            host_or_ip_range: "api.example.com".to_string(),
            port: String::new(),
            file: String::new(),
        }];

        let result = apply_request_match_replace_rules(
            &rules,
            "POST",
            "https://other.example.com/api",
            &HashMap::new(),
            br#"secret"#,
            Some("HTTP/1.1"),
            &include_rules,
            &[],
        );

        assert!(result.is_none());
    }
}

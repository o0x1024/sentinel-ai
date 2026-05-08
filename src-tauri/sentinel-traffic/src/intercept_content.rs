use crate::header_utils::merge_header_value;
use crate::{Result, TrafficError};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedInterceptResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub fn parse_intercept_response_content(content: &str) -> Result<ParsedInterceptResponse> {
    let (head, body) = split_http_message_content(content);
    let mut lines = head.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| TrafficError::Proxy("Empty response content".to_string()))?;
    let parts: Vec<&str> = status_line.split_whitespace().collect();
    let status_code = if parts.len() >= 2 {
        parts[1].parse::<u16>().unwrap_or(200)
    } else {
        200
    };

    Ok(ParsedInterceptResponse {
        status_code,
        headers: parse_intercept_header_lines(lines),
        body: body.as_bytes().to_vec(),
    })
}

pub fn sanitize_edited_response_headers(headers: &mut HashMap<String, String>, body_len: usize) {
    remove_header_case_insensitive(headers, "content-length");
    remove_header_case_insensitive(headers, "content-encoding");
    remove_header_case_insensitive(headers, "transfer-encoding");
    headers.insert("content-length".to_string(), body_len.to_string());
}

fn split_http_message_content(content: &str) -> (&str, &str) {
    for delimiter in ["\r\n\r\n", "\n\n", "\r\r"] {
        if let Some(index) = content.find(delimiter) {
            return (&content[..index], &content[index + delimiter.len()..]);
        }
    }
    (content, "")
}

fn parse_intercept_header_lines<'a>(
    lines: impl Iterator<Item = &'a str>,
) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    let mut current_header: Option<String> = None;

    for raw_line in lines {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            break;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            if !key.is_empty() {
                merge_header_value(&mut headers, key, value);
                current_header = Some(key.to_string());
            }
            continue;
        }

        if let Some(header_name) = current_header.as_deref() {
            let value = line.trim();
            if !value.is_empty() {
                merge_header_value(&mut headers, header_name, value);
            }
        }
    }

    headers
}

fn remove_header_case_insensitive(headers: &mut HashMap<String, String>, target: &str) {
    let keys: Vec<String> = headers
        .keys()
        .filter(|key| key.eq_ignore_ascii_case(target))
        .cloned()
        .collect();
    for key in keys {
        headers.remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_intercept_response_content, sanitize_edited_response_headers};

    #[test]
    fn parses_lf_separated_edited_response_body() {
        let parsed = parse_intercept_response_content(
            "HTTP/2 200 OK\ncontent-type: application/json\n\n{\"TotalCount\":1}",
        )
        .expect("response should parse");

        assert_eq!(parsed.status_code, 200);
        assert_eq!(parsed.body, br#"{"TotalCount":1}"#);
    }

    #[test]
    fn preserves_set_cookie_continuation_lines_from_intercept_editor() {
        let parsed = parse_intercept_response_content(
            "HTTP/2 200 OK\nset-cookie: a=1; Path=/\nb=2; Path=/\ncontent-type: text/plain\n\nok",
        )
        .expect("response should parse");

        assert_eq!(
            parsed.headers.get("set-cookie").map(String::as_str),
            Some("a=1; Path=/\nb=2; Path=/"),
        );
    }

    #[test]
    fn removes_stale_transfer_headers_when_body_is_edited() {
        let mut parsed = parse_intercept_response_content(
            "HTTP/2 200 OK\ncontent-encoding: br\ntransfer-encoding: chunked\ncontent-length: 99\n\nhello",
        )
        .expect("response should parse");

        sanitize_edited_response_headers(&mut parsed.headers, parsed.body.len());

        assert_eq!(
            parsed.headers.get("content-length").map(String::as_str),
            Some("5")
        );
        assert!(!parsed
            .headers
            .keys()
            .any(|key| key.eq_ignore_ascii_case("content-encoding")));
        assert!(!parsed
            .headers
            .keys()
            .any(|key| key.eq_ignore_ascii_case("transfer-encoding")));
    }
}

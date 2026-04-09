use flate2::read::{DeflateDecoder, GzDecoder};
use serde::{Deserialize, Serialize};
use std::io::Read;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use url::Url;

const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawReplayRedirectHop {
    pub url: String,
    pub status_code: u16,
    pub location: Option<String>,
    pub set_cookie_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawReplayResult {
    pub raw_response: String,
    pub response_time_ms: u64,
    pub final_url: String,
    pub redirect_chain: Vec<RawReplayRedirectHop>,
}

#[derive(Debug, Clone)]
pub struct RawReplayConfig {
    pub host: String,
    pub port: u16,
    pub use_tls: bool,
    pub raw_request: String,
    pub timeout_secs: Option<u64>,
    pub follow_redirects: bool,
    pub max_redirects: usize,
    pub process_cookies_in_redirects: bool,
}

#[derive(Debug, Clone)]
struct ParsedRawRequest {
    method: String,
    target: String,
    protocol: String,
    headers: Vec<(String, String)>,
    body: String,
}

#[derive(Debug, Clone)]
struct ParsedRawResponse {
    status_code: u16,
    location: Option<String>,
    set_cookie_headers: Vec<String>,
}

#[derive(Debug, Clone)]
struct RedirectCookie {
    name: String,
    value: String,
    domain: String,
    path: String,
    secure: bool,
    host_only: bool,
}

pub async fn replay_raw_request(config: RawReplayConfig) -> Result<RawReplayResult, String> {
    let timeout = std::time::Duration::from_secs(config.timeout_secs.unwrap_or(30));
    let start = std::time::Instant::now();

    let mut current_request = parse_raw_request(&config.raw_request)?;
    let mut current_url = build_request_url(
        &current_request.target,
        &config.host,
        config.port,
        config.use_tls,
    )?;
    let mut cookie_jar = build_initial_cookie_jar(&current_request, &current_url);
    let mut redirect_chain = Vec::new();
    let mut last_raw_response = String::new();

    for redirect_index in 0..=config.max_redirects {
        let elapsed = start.elapsed();
        let remaining_timeout = timeout.saturating_sub(elapsed);
        if remaining_timeout.is_zero() {
            return Err("Total timeout exceeded".to_string());
        }

        let outbound_request = build_outbound_request(&current_request, &current_url, &cookie_jar);
        let response_buf = execute_single_raw_request(
            current_url.host_str().unwrap_or(&config.host),
            current_url.port_or_known_default().unwrap_or(config.port),
            current_url.scheme() == "https",
            &outbound_request,
            remaining_timeout,
        )
        .await?;

        let raw_response = decode_http_response(&response_buf);
        let response_head = parse_raw_response(&raw_response);
        last_raw_response = raw_response;

        let Some(response_head) = response_head else {
            break;
        };

        let should_follow = config.follow_redirects
            && redirect_index < config.max_redirects
            && matches!(response_head.status_code, 301 | 302 | 303 | 307 | 308);

        let Some(location_header) = response_head.location.clone() else {
            break;
        };

        if !should_follow {
            break;
        }

        let next_url = current_url
            .join(&location_header)
            .map_err(|error| format!("Failed to resolve redirect URL: {error}"))?;

        if config.process_cookies_in_redirects {
            update_cookie_jar(
                &mut cookie_jar,
                &response_head.set_cookie_headers,
                &current_url,
            );
        }

        redirect_chain.push(RawReplayRedirectHop {
            url: current_url.to_string(),
            status_code: response_head.status_code,
            location: Some(next_url.to_string()),
            set_cookie_count: response_head.set_cookie_headers.len(),
        });

        current_request = build_redirect_request(
            &current_request,
            &current_url,
            &next_url,
            response_head.status_code,
        );
        current_url = next_url;
    }

    Ok(RawReplayResult {
        raw_response: last_raw_response,
        response_time_ms: start.elapsed().as_millis() as u64,
        final_url: current_url.to_string(),
        redirect_chain,
    })
}

async fn execute_single_raw_request(
    host: &str,
    port: u16,
    use_tls: bool,
    raw_request: &str,
    timeout: std::time::Duration,
) -> Result<Vec<u8>, String> {
    let addr = format!("{host}:{port}");
    let stream = tokio::time::timeout(timeout, TcpStream::connect(&addr))
        .await
        .map_err(|_| format!("Connection timeout to {addr}"))?
        .map_err(|error| format!("Failed to connect to {addr}: {error}"))?;

    if use_tls {
        let connector = tokio_native_tls::TlsConnector::from(
            native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true)
                .build()
                .map_err(|error| format!("Failed to create TLS connector: {error}"))?,
        );

        let mut tls_stream = tokio::time::timeout(timeout, connector.connect(host, stream))
            .await
            .map_err(|_| "TLS handshake timeout".to_string())?
            .map_err(|error| format!("TLS handshake failed: {error}"))?;

        tls_stream
            .write_all(raw_request.as_bytes())
            .await
            .map_err(|error| format!("Failed to send request: {error}"))?;
        tls_stream
            .flush()
            .await
            .map_err(|error| format!("Failed to flush: {error}"))?;

        read_http_response(&mut tls_stream, timeout).await
    } else {
        let mut stream = stream;
        stream
            .write_all(raw_request.as_bytes())
            .await
            .map_err(|error| format!("Failed to send request: {error}"))?;
        stream
            .flush()
            .await
            .map_err(|error| format!("Failed to flush: {error}"))?;

        read_http_response(&mut stream, timeout).await
    }
}

fn parse_raw_request(raw_request: &str) -> Result<ParsedRawRequest, String> {
    let normalized = raw_request.replace("\r\n", "\n").replace('\r', "\n");
    let separator_index = normalized.find("\n\n").unwrap_or(normalized.len());
    let header_part = &normalized[..separator_index];
    let body = if separator_index < normalized.len() {
        normalized[separator_index + 2..].to_string()
    } else {
        String::new()
    };
    let mut lines = header_part.lines();

    let request_line = lines
        .next()
        .ok_or_else(|| "Raw request is missing the request line".to_string())?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| "Raw request is missing the HTTP method".to_string())?;
    let target = request_parts
        .next()
        .ok_or_else(|| "Raw request is missing the request target".to_string())?;
    let protocol = request_parts.next().unwrap_or("HTTP/1.1");

    let headers = lines
        .filter_map(|line| {
            let colon_index = line.find(':')?;
            Some((
                line[..colon_index].trim().to_string(),
                line[colon_index + 1..].trim().to_string(),
            ))
        })
        .collect();

    Ok(ParsedRawRequest {
        method: method.to_string(),
        target: target.to_string(),
        protocol: protocol.to_string(),
        headers,
        body,
    })
}

fn build_request_url(
    request_target: &str,
    host: &str,
    port: u16,
    use_tls: bool,
) -> Result<Url, String> {
    if request_target.starts_with("http://") || request_target.starts_with("https://") {
        return Url::parse(request_target)
            .map_err(|error| format!("Failed to parse request URL: {error}"));
    }

    let scheme = if use_tls { "https" } else { "http" };
    let default_port = if use_tls { 443 } else { 80 };
    let port_suffix = if port == default_port {
        String::new()
    } else {
        format!(":{port}")
    };
    let path = if request_target.starts_with('/') {
        request_target.to_string()
    } else {
        format!("/{request_target}")
    };

    Url::parse(&format!("{scheme}://{host}{port_suffix}{path}"))
        .map_err(|error| format!("Failed to build request URL: {error}"))
}

fn build_initial_cookie_jar(request: &ParsedRawRequest, request_url: &Url) -> Vec<RedirectCookie> {
    let mut jar = Vec::new();
    for (_, value) in request
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("cookie"))
    {
        for pair in value.split(';') {
            let trimmed = pair.trim();
            if trimmed.is_empty() {
                continue;
            }

            let Some((name, cookie_value)) = trimmed.split_once('=') else {
                continue;
            };

            upsert_cookie(
                &mut jar,
                RedirectCookie {
                    name: name.trim().to_string(),
                    value: cookie_value.trim().to_string(),
                    domain: request_url.host_str().unwrap_or_default().to_string(),
                    path: default_cookie_path(request_url),
                    secure: request_url.scheme() == "https",
                    host_only: true,
                },
            );
        }
    }
    jar
}

fn build_outbound_request(
    request: &ParsedRawRequest,
    target_url: &Url,
    cookie_jar: &[RedirectCookie],
) -> String {
    let mut headers = request.headers.clone();
    replace_or_insert_header(&mut headers, "Host", &build_host_header(target_url));

    let cookie_header = build_cookie_header(cookie_jar, target_url);
    match cookie_header {
        Some(value) => replace_or_insert_header(&mut headers, "Cookie", &value),
        None => remove_header(&mut headers, "Cookie"),
    }

    if request.body.is_empty() {
        remove_header(&mut headers, "Content-Length");
    } else {
        replace_or_insert_header(
            &mut headers,
            "Content-Length",
            &request.body.as_bytes().len().to_string(),
        );
    }

    let path = build_request_target(target_url);
    let mut raw_request = format!("{} {} {}\r\n", request.method, path, request.protocol);
    for (name, value) in headers {
        raw_request.push_str(&format!("{name}: {value}\r\n"));
    }
    raw_request.push_str("\r\n");
    raw_request.push_str(&request.body);
    if !raw_request.ends_with("\r\n") {
        raw_request.push_str("\r\n");
    }

    raw_request
}

fn build_redirect_request(
    previous_request: &ParsedRawRequest,
    _current_url: &Url,
    next_url: &Url,
    status_code: u16,
) -> ParsedRawRequest {
    let preserve_method = matches!(status_code, 307 | 308);
    let mut request = previous_request.clone();

    if !preserve_method && !request.method.eq_ignore_ascii_case("HEAD") {
        request.method = "GET".to_string();
        request.body.clear();
    }

    request.target = build_request_target(next_url);
    request
}

fn parse_raw_response(raw_response: &str) -> Option<ParsedRawResponse> {
    let header_end = raw_response
        .find("\r\n\r\n")
        .or_else(|| raw_response.find("\n\n"))?;
    let header_part = &raw_response[..header_end];
    let mut lines = header_part.lines();
    let status_line = lines.next()?;
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())?;

    let mut location = None;
    let mut set_cookie_headers = Vec::new();

    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let trimmed_value = value.trim().to_string();
        if name.eq_ignore_ascii_case("location") {
            location = Some(trimmed_value);
        } else if name.eq_ignore_ascii_case("set-cookie") {
            set_cookie_headers.push(trimmed_value);
        }
    }

    Some(ParsedRawResponse {
        status_code,
        location,
        set_cookie_headers,
    })
}

fn update_cookie_jar(
    cookie_jar: &mut Vec<RedirectCookie>,
    set_cookie_headers: &[String],
    request_url: &Url,
) {
    for header in set_cookie_headers {
        if let Some(cookie) = parse_set_cookie(header, request_url) {
            upsert_cookie(cookie_jar, cookie);
        }
    }
}

fn parse_set_cookie(header: &str, request_url: &Url) -> Option<RedirectCookie> {
    let mut parts = header.split(';');
    let name_value = parts.next()?.trim();
    let (name, value) = name_value.split_once('=')?;

    let mut cookie = RedirectCookie {
        name: name.trim().to_string(),
        value: value.trim().to_string(),
        domain: request_url.host_str()?.to_string(),
        path: default_cookie_path(request_url),
        secure: false,
        host_only: true,
    };

    for attribute in parts {
        let trimmed = attribute.trim();
        if trimmed.eq_ignore_ascii_case("secure") {
            cookie.secure = true;
            continue;
        }

        let Some((attr_name, attr_value)) = trimmed.split_once('=') else {
            continue;
        };

        if attr_name.eq_ignore_ascii_case("domain") {
            cookie.domain = attr_value.trim().trim_start_matches('.').to_string();
            cookie.host_only = false;
        } else if attr_name.eq_ignore_ascii_case("path") {
            cookie.path = normalize_cookie_path(attr_value.trim());
        }
    }

    Some(cookie)
}

fn upsert_cookie(cookie_jar: &mut Vec<RedirectCookie>, next_cookie: RedirectCookie) {
    cookie_jar.retain(|cookie| {
        !(cookie.name.eq_ignore_ascii_case(&next_cookie.name)
            && cookie.domain.eq_ignore_ascii_case(&next_cookie.domain)
            && cookie.path == next_cookie.path)
    });
    cookie_jar.push(next_cookie);
}

fn build_cookie_header(cookie_jar: &[RedirectCookie], target_url: &Url) -> Option<String> {
    let host = target_url.host_str().unwrap_or_default();
    let path = target_url.path();
    let is_secure = target_url.scheme() == "https";

    let values = cookie_jar
        .iter()
        .filter(|cookie| cookie_matches(cookie, host, path, is_secure))
        .map(|cookie| format!("{}={}", cookie.name, cookie.value))
        .collect::<Vec<_>>();

    if values.is_empty() {
        None
    } else {
        Some(values.join("; "))
    }
}

fn cookie_matches(cookie: &RedirectCookie, host: &str, path: &str, is_secure: bool) -> bool {
    if cookie.secure && !is_secure {
        return false;
    }

    let domain_matches = if cookie.host_only {
        cookie.domain.eq_ignore_ascii_case(host)
    } else {
        host.eq_ignore_ascii_case(&cookie.domain)
            || host
                .to_ascii_lowercase()
                .ends_with(&format!(".{}", cookie.domain.to_ascii_lowercase()))
    };

    if !domain_matches {
        return false;
    }

    path.starts_with(&cookie.path)
}

fn default_cookie_path(url: &Url) -> String {
    let path = url.path();
    if path.is_empty() || !path.starts_with('/') {
        return "/".to_string();
    }

    match path.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(index) => path[..index].to_string(),
    }
}

fn normalize_cookie_path(path: &str) -> String {
    if path.is_empty() {
        "/".to_string()
    } else if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

fn replace_or_insert_header(headers: &mut Vec<(String, String)>, name: &str, value: &str) {
    if let Some((_, existing_value)) = headers
        .iter_mut()
        .find(|(existing_name, _)| existing_name.eq_ignore_ascii_case(name))
    {
        *existing_value = value.to_string();
        return;
    }

    headers.push((name.to_string(), value.to_string()));
}

fn remove_header(headers: &mut Vec<(String, String)>, name: &str) {
    headers.retain(|(existing_name, _)| !existing_name.eq_ignore_ascii_case(name));
}

fn build_host_header(target_url: &Url) -> String {
    let host = target_url.host_str().unwrap_or_default();
    let default_port = match target_url.scheme() {
        "https" => 443,
        _ => 80,
    };

    match target_url.port() {
        Some(port) if port != default_port => format!("{host}:{port}"),
        _ => host.to_string(),
    }
}

fn build_request_target(target_url: &Url) -> String {
    let mut target = target_url.path().to_string();
    if target.is_empty() {
        target.push('/');
    }
    if let Some(query) = target_url.query() {
        target.push('?');
        target.push_str(query);
    }
    target
}

fn decode_chunked(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        let line_end = data[pos..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .map(|offset| pos + offset);

        let Some(line_end) = line_end else {
            break;
        };

        let size_str = String::from_utf8_lossy(&data[pos..line_end]);
        let size_str = size_str.split(';').next().unwrap_or("").trim();
        let chunk_size = match usize::from_str_radix(size_str, 16) {
            Ok(size) => size,
            Err(_) => break,
        };

        if chunk_size == 0 {
            break;
        }

        let chunk_start = line_end + 2;
        let chunk_end = chunk_start + chunk_size;
        if chunk_end > data.len() {
            break;
        }

        result.extend_from_slice(&data[chunk_start..chunk_end]);
        pos = chunk_end + 2;
    }

    if result.is_empty() {
        data.to_vec()
    } else {
        result
    }
}

fn decode_http_response(response_buf: &[u8]) -> String {
    let header_end = response_buf
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|offset| offset + 4);

    let Some(header_end) = header_end else {
        return String::from_utf8_lossy(response_buf).to_string();
    };

    let header_bytes = &response_buf[..header_end];
    let body_bytes = &response_buf[header_end..];
    let header_str = String::from_utf8_lossy(header_bytes);
    let header_lower = header_str.to_lowercase();

    let is_chunked = header_lower
        .lines()
        .any(|line| line.starts_with("transfer-encoding:") && line.contains("chunked"));
    let content_encoding = header_str
        .lines()
        .find(|line| line.to_lowercase().starts_with("content-encoding:"))
        .map(|line| line.split(':').nth(1).unwrap_or("").trim().to_lowercase());

    let body_bytes = if is_chunked {
        decode_chunked(body_bytes)
    } else {
        body_bytes.to_vec()
    };

    let decoded_body = match content_encoding.as_deref() {
        Some("gzip") => {
            let mut decoder = GzDecoder::new(body_bytes.as_slice());
            let mut decoded = Vec::new();
            decoder
                .read_to_end(&mut decoded)
                .map(|_| decoded)
                .unwrap_or(body_bytes)
        }
        Some("deflate") => {
            let mut decoder = DeflateDecoder::new(body_bytes.as_slice());
            let mut decoded = Vec::new();
            decoder
                .read_to_end(&mut decoded)
                .map(|_| decoded)
                .unwrap_or(body_bytes)
        }
        _ => body_bytes,
    };

    let mut result = String::from_utf8_lossy(header_bytes).to_string();
    result.push_str(&String::from_utf8_lossy(&decoded_body));
    result
}

async fn read_http_response<S: AsyncRead + Unpin>(
    stream: &mut S,
    total_timeout: std::time::Duration,
) -> Result<Vec<u8>, String> {
    let mut response_buf = Vec::new();
    let mut buf = [0u8; 8192];
    let mut headers_parsed = false;
    let mut content_length: Option<usize> = None;
    let mut is_chunked = false;
    let mut header_end_pos: Option<usize> = None;
    let read_timeout = std::time::Duration::from_millis(500);
    let start_time = std::time::Instant::now();

    loop {
        if start_time.elapsed() >= total_timeout {
            return Err("Total timeout exceeded".to_string());
        }

        if response_buf.len() > MAX_RESPONSE_SIZE {
            return Err(format!(
                "Response too large: {} bytes (max: {} bytes)",
                response_buf.len(),
                MAX_RESPONSE_SIZE
            ));
        }

        match tokio::time::timeout(read_timeout, stream.read(&mut buf)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => {
                response_buf.extend_from_slice(&buf[..n]);

                if !headers_parsed {
                    if let Some(pos) = response_buf
                        .windows(4)
                        .position(|window| window == b"\r\n\r\n")
                    {
                        header_end_pos = Some(pos + 4);
                        headers_parsed = true;

                        let header_str = String::from_utf8_lossy(&response_buf[..pos]);
                        for line in header_str.lines() {
                            let lower = line.to_lowercase();
                            if lower.starts_with("content-length:") {
                                if let Some(length) = line
                                    .split(':')
                                    .nth(1)
                                    .and_then(|value| value.trim().parse::<usize>().ok())
                                {
                                    content_length = Some(length);
                                }
                            } else if lower.starts_with("transfer-encoding:")
                                && lower.contains("chunked")
                            {
                                is_chunked = true;
                            }
                        }
                    }
                }

                if headers_parsed {
                    if let Some(header_end) = header_end_pos {
                        let body_len = response_buf.len().saturating_sub(header_end);
                        if let Some(expected_length) = content_length {
                            if body_len >= expected_length {
                                break;
                            }
                        }

                        if is_chunked && response_buf.len() >= 5 {
                            let tail = &response_buf[response_buf.len().saturating_sub(7)..];
                            if tail.windows(5).any(|window| window == b"0\r\n\r\n") {
                                break;
                            }
                        }
                    }
                }
            }
            Ok(Err(error)) => return Err(format!("Read error: {error}")),
            Err(_) => {
                if headers_parsed {
                    break;
                }
                if response_buf.is_empty() && start_time.elapsed() < total_timeout {
                    continue;
                }
                break;
            }
        }
    }

    Ok(response_buf)
}

#[cfg(test)]
mod tests {
    use super::{
        build_redirect_request, cookie_matches, parse_raw_request, parse_set_cookie, RedirectCookie,
    };
    use url::Url;

    #[test]
    fn parses_set_cookie_attributes() {
        let url = Url::parse("https://example.com/app/login").unwrap();
        let cookie =
            parse_set_cookie("session=abc; Path=/app; Secure; Domain=example.com", &url).unwrap();

        assert_eq!(cookie.name, "session");
        assert_eq!(cookie.path, "/app");
        assert!(cookie.secure);
        assert_eq!(cookie.domain, "example.com");
        assert!(!cookie.host_only);
    }

    #[test]
    fn cookie_matching_respects_domain_path_and_scheme() {
        let cookie = RedirectCookie {
            name: "session".to_string(),
            value: "abc".to_string(),
            domain: "example.com".to_string(),
            path: "/app".to_string(),
            secure: true,
            host_only: false,
        };

        assert!(cookie_matches(
            &cookie,
            "sub.example.com",
            "/app/dashboard",
            true
        ));
        assert!(!cookie_matches(&cookie, "sub.example.com", "/admin", true));
        assert!(!cookie_matches(
            &cookie,
            "sub.example.com",
            "/app/dashboard",
            false
        ));
    }

    #[test]
    fn redirect_rewrites_post_to_get_for_302() {
        let request = parse_raw_request(
            "POST /login HTTP/1.1\r\nHost: example.com\r\nContent-Length: 3\r\n\r\nx=1",
        )
        .unwrap();
        let next_url = Url::parse("https://example.com/home").unwrap();
        let redirected = build_redirect_request(&request, &next_url, &next_url, 302);

        assert_eq!(redirected.method, "GET");
        assert!(redirected.body.is_empty());
        assert_eq!(redirected.target, "/home");
    }
}

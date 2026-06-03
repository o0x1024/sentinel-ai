use crate::args::{HttpAction, HttpCommand, HttpRequestArgs};
use crate::output::print_json;
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Serialize)]
struct HttpResponseReport {
    url: String,
    status_code: u16,
    status_text: String,
    headers: HashMap<String, String>,
    body: String,
    body_length: usize,
    response_time_ms: u64,
    truncated: bool,
}

pub async fn run(command: HttpCommand) -> Result<()> {
    match command.action {
        HttpAction::Request(args) => request(args).await,
    }
}

async fn request(args: HttpRequestArgs) -> Result<()> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .redirect(if args.follow_redirects {
            reqwest::redirect::Policy::limited(10)
        } else {
            reqwest::redirect::Policy::none()
        })
        .timeout(Duration::from_secs(args.timeout_secs))
        .build()
        .context("failed to build HTTP client")?;

    let method = reqwest::Method::from_bytes(args.method.trim().to_uppercase().as_bytes())
        .map_err(|error| anyhow!("invalid HTTP method: {error}"))?;

    let mut request = client.request(method, &args.url);
    for header in args.headers {
        let (key, value) = header
            .split_once(':')
            .ok_or_else(|| anyhow!("invalid header `{header}`, expected Key:Value"))?;
        request = request.header(key.trim(), value.trim());
    }

    if let Some(body) = args.body {
        request = request.body(body);
    }

    let started = std::time::Instant::now();
    let response = request.send().await.context("request failed")?;
    let status_code = response.status().as_u16();
    let status_text = response.status().to_string();

    let mut headers = HashMap::new();
    for (key, value) in response.headers() {
        headers.insert(
            key.to_string(),
            value.to_str().unwrap_or("<non-utf8>").to_string(),
        );
    }

    let raw_body = response
        .text()
        .await
        .context("failed to read response body")?;
    let body_char_count = raw_body.chars().count();
    let truncated = body_char_count > args.max_body_chars;
    let body = if truncated {
        raw_body
            .chars()
            .take(args.max_body_chars)
            .collect::<String>()
    } else {
        raw_body
    };

    print_json(&HttpResponseReport {
        url: args.url,
        status_code,
        status_text,
        headers,
        body_length: body_char_count,
        body,
        response_time_ms: started.elapsed().as_millis() as u64,
        truncated,
    })
}

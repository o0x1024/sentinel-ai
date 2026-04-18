use futures_util::stream::{self, StreamExt};
use regex::Regex;
use reqwest::{Client, Url};
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::time::Instant;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct RouteDiscoveryArgs {
    /// Base URL to probe, for example http://10.0.0.10 or http://10.0.0.10/app
    pub base_url: String,
    /// Optional authenticated headers such as Authorization or Cookie.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Optional Cookie header value when not already provided in headers.
    #[serde(default)]
    pub cookies: Option<String>,
    /// Optional custom path list. When omitted, the tool uses a compact built-in list.
    #[serde(default)]
    pub paths: Vec<String>,
    /// Optional extensions appended to extensionless paths.
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Probe timeout in seconds.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    /// Maximum concurrent requests.
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    /// Maximum number of interesting results to keep.
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    /// Follow redirects while probing.
    #[serde(default = "default_follow_redirects")]
    pub follow_redirects: bool,
}

fn default_timeout_secs() -> u64 {
    12
}

fn default_concurrency() -> usize {
    12
}

fn default_max_results() -> usize {
    40
}

fn default_follow_redirects() -> bool {
    false
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteDiscoveryFinding {
    pub path: String,
    pub url: String,
    pub status_code: u16,
    pub content_length: usize,
    pub response_time_ms: u64,
    pub title: Option<String>,
    pub redirect_location: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteDiscoveryBaseline {
    pub status_code: u16,
    pub content_length: usize,
    pub body_fingerprint: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RouteDiscoveryOutput {
    pub base_url: String,
    pub requested_paths: usize,
    pub findings: Vec<RouteDiscoveryFinding>,
    pub filtered_as_wildcard: usize,
    pub baseline: Vec<RouteDiscoveryBaseline>,
    pub message: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RouteDiscoveryError {
    #[error("invalid base url: {0}")]
    InvalidBaseUrl(String),
    #[error("request failed: {0}")]
    RequestFailed(String),
}

#[derive(Debug, Clone, Default)]
pub struct RouteDiscoveryTool;

impl RouteDiscoveryTool {
    pub const NAME: &'static str = "route_discovery";
    pub const DESCRIPTION: &'static str = concat!(
        "Enumerate likely web routes with authenticated headers or cookies, using concurrent HTTP probes and wildcard-response filtering. ",
        "Use when you need directory or route discovery without handcrafting many repetitive requests. ",
        "Best for login-protected apps, hidden admin panels, exports, APIs, uploads, dashboards, or internal endpoints."
    );
}

impl Tool for RouteDiscoveryTool {
    const NAME: &'static str = Self::NAME;
    type Args = RouteDiscoveryArgs;
    type Output = RouteDiscoveryOutput;
    type Error = RouteDiscoveryError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(RouteDiscoveryArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let base_url = Url::parse(&args.base_url)
            .map_err(|error| RouteDiscoveryError::InvalidBaseUrl(error.to_string()))?;
        let client = build_client(args.follow_redirects)?;

        let baseline = collect_baseline(&client, &base_url, &args).await?;
        let candidates = build_candidate_paths(&args);
        let concurrency = args.concurrency.max(1).min(64);

        let probe_results = stream::iter(candidates.iter().cloned())
            .map(|path| {
                let client = client.clone();
                let base_url = base_url.clone();
                let headers = args.headers.clone();
                let cookies = args.cookies.clone();
                async move {
                    probe_path(
                        &client,
                        &base_url,
                        &headers,
                        cookies.as_deref(),
                        args.timeout_secs,
                        path,
                    )
                    .await
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;

        let mut findings = Vec::new();
        let mut filtered_as_wildcard = 0_usize;

        for result in probe_results {
            let Some(sample) = result? else {
                continue;
            };
            if baseline
                .iter()
                .any(|fingerprint| fingerprint.matches(sample.status_code, &sample.body))
            {
                filtered_as_wildcard += 1;
                continue;
            }
            if sample.status_code == 404 {
                continue;
            }
            findings.push(RouteDiscoveryFinding {
                path: sample.path,
                url: sample.url,
                status_code: sample.status_code,
                content_length: sample.body.len(),
                response_time_ms: sample.response_time_ms,
                title: extract_title(&sample.body),
                redirect_location: sample.redirect_location,
                content_type: sample.content_type,
            });
        }

        findings.sort_by(|left, right| {
            left.status_code
                .cmp(&right.status_code)
                .then(left.path.cmp(&right.path))
        });
        findings.truncate(args.max_results.max(1));

        Ok(RouteDiscoveryOutput {
            base_url: args.base_url,
            requested_paths: candidates.len(),
            findings,
            filtered_as_wildcard,
            baseline: baseline
                .into_iter()
                .map(|fingerprint| RouteDiscoveryBaseline {
                    status_code: fingerprint.status_code,
                    content_length: fingerprint.content_length,
                    body_fingerprint: fingerprint.body_fingerprint,
                })
                .collect(),
            message: None,
        })
    }
}

fn build_client(follow_redirects: bool) -> Result<Client, RouteDiscoveryError> {
    let client = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async move {
            let builder = reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .redirect(if follow_redirects {
                    reqwest::redirect::Policy::limited(5)
                } else {
                    reqwest::redirect::Policy::none()
                });
            let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
            builder.build()
        })
    })
    .map_err(|error| RouteDiscoveryError::RequestFailed(error.to_string()))?;
    Ok(client)
}

async fn collect_baseline(
    client: &Client,
    base_url: &Url,
    args: &RouteDiscoveryArgs,
) -> Result<Vec<ResponseFingerprint>, RouteDiscoveryError> {
    let probes = vec![
        format!("nonexistent-{}", Uuid::new_v4().simple()),
        format!("definitely-missing-{}", Uuid::new_v4().simple()),
    ];
    let mut fingerprints = Vec::new();
    for probe in probes {
        if let Some(sample) = probe_path(
            client,
            base_url,
            &args.headers,
            args.cookies.as_deref(),
            args.timeout_secs,
            probe,
        )
        .await?
        {
            fingerprints.push(ResponseFingerprint::from_sample(
                sample.status_code,
                &sample.body,
            ));
        }
    }
    Ok(fingerprints)
}

fn build_candidate_paths(args: &RouteDiscoveryArgs) -> Vec<String> {
    let source_paths = if args.paths.is_empty() {
        default_paths()
    } else {
        args.paths.clone()
    };
    let mut candidates = BTreeSet::new();
    for raw_path in source_paths {
        let normalized = normalize_path(&raw_path);
        if normalized.is_empty() {
            continue;
        }
        candidates.insert(normalized.clone());
        if !normalized.contains('.') {
            for ext in &args.extensions {
                let ext = ext.trim().trim_start_matches('.');
                if ext.is_empty() {
                    continue;
                }
                candidates.insert(format!("{}.{}", normalized, ext));
            }
        }
    }
    candidates.into_iter().collect()
}

fn normalize_path(value: &str) -> String {
    value.trim().trim_start_matches('/').to_string()
}

fn default_paths() -> Vec<String> {
    vec![
        "admin",
        "admin/login",
        "login",
        "dashboard",
        "index.php",
        "api",
        "api/v1",
        "graphql",
        "swagger",
        "docs",
        "debug",
        "health",
        "metrics",
        "config",
        "settings",
        "export",
        "exports",
        "download",
        "uploads",
        "files",
        "backup",
        "backups",
        "report",
        "reports",
        "billing",
        "invoice",
        "approval",
        "approvals",
        "workflow",
        "ticket",
        "tickets",
        "helpdesk",
        "user",
        "users",
        "profile",
        "search",
        "query",
        "internal",
        "private",
        "staff",
        "manage",
        "management",
        "system",
        "console",
        "panel",
        "status",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

async fn probe_path(
    client: &Client,
    base_url: &Url,
    headers: &HashMap<String, String>,
    cookies: Option<&str>,
    timeout_secs: u64,
    path: String,
) -> Result<Option<ResponseSample>, RouteDiscoveryError> {
    let url = base_url
        .join(&path)
        .map_err(|error| RouteDiscoveryError::InvalidBaseUrl(error.to_string()))?;
    let mut request = client
        .get(url.clone())
        .timeout(std::time::Duration::from_secs(timeout_secs));
    for (key, value) in headers {
        request = request.header(key, value);
    }
    if let Some(cookie_header) = cookies {
        if !headers.contains_key("Cookie") && !headers.contains_key("cookie") {
            request = request.header("Cookie", cookie_header);
        }
    }
    let started_at = Instant::now();
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) if error.is_timeout() => return Ok(None),
        Err(error) => return Err(RouteDiscoveryError::RequestFailed(error.to_string())),
    };
    let status_code = response.status().as_u16();
    let redirect_location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let body = response
        .text()
        .await
        .map_err(|error| RouteDiscoveryError::RequestFailed(error.to_string()))?;
    Ok(Some(ResponseSample {
        path,
        url: url.to_string(),
        status_code,
        redirect_location,
        content_type,
        body,
        response_time_ms: started_at.elapsed().as_millis() as u64,
    }))
}

#[derive(Debug, Clone)]
struct ResponseSample {
    path: String,
    url: String,
    status_code: u16,
    redirect_location: Option<String>,
    content_type: Option<String>,
    body: String,
    response_time_ms: u64,
}

#[derive(Debug, Clone)]
struct ResponseFingerprint {
    status_code: u16,
    content_length: usize,
    body_fingerprint: String,
}

impl ResponseFingerprint {
    fn from_sample(status_code: u16, body: &str) -> Self {
        Self {
            status_code,
            content_length: body.len(),
            body_fingerprint: hash_body(body),
        }
    }

    fn matches(&self, status_code: u16, body: &str) -> bool {
        self.status_code == status_code
            && self.content_length == body.len()
            && self.body_fingerprint == hash_body(body)
    }
}

fn hash_body(body: &str) -> String {
    let preview = body.chars().take(4096).collect::<String>();
    let mut hasher = Sha256::new();
    hasher.update(preview.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn extract_title(body: &str) -> Option<String> {
    static TITLE_RE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new("(?is)<title>(.*?)</title>").unwrap());
    TITLE_RE
        .captures(body)
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().trim().to_string())
        .filter(|value| !value.is_empty())
}

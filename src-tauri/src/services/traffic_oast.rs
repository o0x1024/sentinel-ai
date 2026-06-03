use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrafficOastConfig {
    pub enabled: bool,
    pub server_base_url: String,
    pub api_key: String,
    pub poll_interval_secs: u64,
    pub request_timeout_secs: u64,
}

impl Default for TrafficOastConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_base_url: String::new(),
            api_key: String::new(),
            poll_interval_secs: 15,
            request_timeout_secs: 10,
        }
    }
}

impl TrafficOastConfig {
    pub fn sanitized(&self) -> Self {
        Self {
            enabled: self.enabled,
            server_base_url: self
                .server_base_url
                .trim()
                .trim_end_matches('/')
                .to_string(),
            api_key: self.api_key.trim().to_string(),
            poll_interval_secs: self.poll_interval_secs.clamp(5, 300),
            request_timeout_secs: self.request_timeout_secs.clamp(3, 60),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrafficOastTestResult {
    pub reachable: bool,
    pub message: String,
    pub generated_token: Option<String>,
    pub generated_fqdn: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OastExampleUrls {
    pub http: String,
    pub https: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrafficOastEvent {
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub path: String,
    #[serde(default = "default_oast_query")]
    pub query: serde_json::Value,
    #[serde(default)]
    pub user_agent: String,
    #[serde(default)]
    pub referer: String,
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub ray: String,
    #[serde(default)]
    pub colo: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub asn: Option<u64>,
}

fn default_oast_query() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrafficOastRecord {
    pub token: String,
    pub fqdn: String,
    pub http_url: String,
    pub https_url: String,
    pub created_at: String,
    pub label: String,
    pub source_tool: String,
    pub source_request_id: Option<i64>,
    pub hit_count: u64,
    pub last_hit_at: Option<String>,
    pub last_sync_at: Option<String>,
    pub events: Vec<TrafficOastEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrafficOastEventKey {
    pub time: String,
    pub method: String,
    pub url: String,
    pub ip: String,
}

#[derive(Debug, Deserialize)]
pub struct OastGenerateResponse {
    pub ok: bool,
    pub token: Option<String>,
    pub fqdn: Option<String>,
    pub message: Option<String>,
    pub example_urls: Option<OastExampleUrls>,
}

#[derive(Debug, Deserialize)]
pub struct OastLookupResponse {
    pub ok: Option<bool>,
    pub token: Option<String>,
    pub fqdn: Option<String>,
    pub created_at: Option<String>,
    pub hit_count: Option<u64>,
    pub last_hit_at: Option<String>,
    pub events: Option<Vec<TrafficOastEvent>>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OastDeleteRequest {
    token: String,
    delete_all: bool,
    events: Vec<TrafficOastEventKey>,
}

#[derive(Debug, Deserialize)]
pub struct OastDeleteResponse {
    pub ok: Option<bool>,
    pub error: Option<String>,
    pub token: Option<String>,
    pub deleted_all: Option<bool>,
    pub deleted_count: Option<u64>,
    pub hit_count: Option<u64>,
    pub last_hit_at: Option<String>,
    pub events: Option<Vec<TrafficOastEvent>>,
}

fn build_client(config: &TrafficOastConfig) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(config.request_timeout_secs))
        .build()
        .map_err(|error| format!("Failed to create OAST client: {error}"))
}

fn build_generate_url(config: &TrafficOastConfig) -> Result<Url, String> {
    if config.server_base_url.is_empty() {
        return Err("OAST server base URL is required".to_string());
    }

    let mut url = Url::parse(&config.server_base_url)
        .map_err(|error| format!("Invalid OAST server base URL: {error}"))?;

    url = url
        .join("/gen")
        .map_err(|error| format!("Failed to build OAST generate URL: {error}"))?;

    if !config.api_key.is_empty() {
        url.query_pairs_mut().append_pair("key", &config.api_key);
    }

    Ok(url)
}

fn build_lookup_url(config: &TrafficOastConfig, token: &str) -> Result<Url, String> {
    if config.server_base_url.is_empty() {
        return Err("OAST server base URL is required".to_string());
    }

    if token.trim().is_empty() {
        return Err("OAST token is required".to_string());
    }

    let mut url = Url::parse(&config.server_base_url)
        .map_err(|error| format!("Invalid OAST server base URL: {error}"))?;

    url = url
        .join("/lookup")
        .map_err(|error| format!("Failed to build OAST lookup URL: {error}"))?;
    url.query_pairs_mut().append_pair("token", token.trim());

    if !config.api_key.is_empty() {
        url.query_pairs_mut().append_pair("key", &config.api_key);
    }

    Ok(url)
}

fn build_delete_url(config: &TrafficOastConfig) -> Result<Url, String> {
    if config.server_base_url.is_empty() {
        return Err("OAST server base URL is required".to_string());
    }

    let mut url = Url::parse(&config.server_base_url)
        .map_err(|error| format!("Invalid OAST server base URL: {error}"))?;

    url = url
        .join("/delete")
        .map_err(|error| format!("Failed to build OAST delete URL: {error}"))?;

    if !config.api_key.is_empty() {
        url.query_pairs_mut().append_pair("key", &config.api_key);
    }

    Ok(url)
}

pub async fn generate_traffic_oast_token(
    config: &TrafficOastConfig,
) -> Result<OastGenerateResponse, String> {
    let sanitized = config.sanitized();
    let url = build_generate_url(&sanitized)?;
    let client = build_client(&sanitized)?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Failed to reach OAST server: {error}"))?;
    let status = response.status();

    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("<unreadable response body>"));
        return Err(format!("OAST server returned {status}: {body}"));
    }

    let payload = response
        .json::<OastGenerateResponse>()
        .await
        .map_err(|error| format!("Failed to parse OAST response: {error}"))?;

    if !payload.ok {
        return Err(payload
            .message
            .clone()
            .unwrap_or_else(|| "OAST server returned ok=false".to_string()));
    }

    Ok(payload)
}

pub async fn lookup_traffic_oast_token(
    config: &TrafficOastConfig,
    token: &str,
) -> Result<OastLookupResponse, String> {
    let sanitized = config.sanitized();
    let url = build_lookup_url(&sanitized, token)?;
    let client = build_client(&sanitized)?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Failed to query OAST server: {error}"))?;
    let status = response.status();

    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("<unreadable response body>"));
        return Err(format!("OAST server returned {status}: {body}"));
    }

    let payload = response
        .json::<OastLookupResponse>()
        .await
        .map_err(|error| format!("Failed to parse OAST lookup response: {error}"))?;

    if matches!(payload.ok, Some(false)) {
        return Err(payload
            .error
            .clone()
            .unwrap_or_else(|| "OAST server returned ok=false".to_string()));
    }

    Ok(payload)
}

pub async fn delete_traffic_oast_events(
    config: &TrafficOastConfig,
    token: &str,
    events: &[TrafficOastEventKey],
) -> Result<OastDeleteResponse, String> {
    let sanitized = config.sanitized();
    if token.trim().is_empty() {
        return Err("OAST token is required".to_string());
    }
    if events.is_empty() {
        return Err("At least one OAST event must be selected".to_string());
    }

    let url = build_delete_url(&sanitized)?;
    let client = build_client(&sanitized)?;
    let response = client
        .post(url)
        .json(&OastDeleteRequest {
            token: token.trim().to_string(),
            delete_all: false,
            events: events.to_vec(),
        })
        .send()
        .await
        .map_err(|error| format!("Failed to delete OAST events: {error}"))?;
    let status = response.status();

    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("<unreadable response body>"));
        return Err(format!("OAST server returned {status}: {body}"));
    }

    let payload = response
        .json::<OastDeleteResponse>()
        .await
        .map_err(|error| format!("Failed to parse OAST delete response: {error}"))?;

    if matches!(payload.ok, Some(false)) {
        return Err(payload
            .error
            .clone()
            .unwrap_or_else(|| "OAST server returned ok=false".to_string()));
    }

    Ok(payload)
}

pub async fn delete_traffic_oast_token(
    config: &TrafficOastConfig,
    token: &str,
) -> Result<OastDeleteResponse, String> {
    let sanitized = config.sanitized();
    if token.trim().is_empty() {
        return Err("OAST token is required".to_string());
    }

    let url = build_delete_url(&sanitized)?;
    let client = build_client(&sanitized)?;
    let response = client
        .post(url)
        .json(&OastDeleteRequest {
            token: token.trim().to_string(),
            delete_all: true,
            events: Vec::new(),
        })
        .send()
        .await
        .map_err(|error| format!("Failed to delete OAST token: {error}"))?;
    let status = response.status();

    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("<unreadable response body>"));
        return Err(format!("OAST server returned {status}: {body}"));
    }

    let payload = response
        .json::<OastDeleteResponse>()
        .await
        .map_err(|error| format!("Failed to parse OAST delete response: {error}"))?;

    if matches!(payload.ok, Some(false)) {
        return Err(payload
            .error
            .clone()
            .unwrap_or_else(|| "OAST server returned ok=false".to_string()));
    }

    Ok(payload)
}

pub async fn test_traffic_oast_config(
    config: &TrafficOastConfig,
) -> Result<TrafficOastTestResult, String> {
    let payload = generate_traffic_oast_token(config).await?;

    Ok(TrafficOastTestResult {
        reachable: true,
        message: "OAST server is reachable and returned a test token".to_string(),
        generated_token: payload.token,
        generated_fqdn: payload.fqdn,
    })
}

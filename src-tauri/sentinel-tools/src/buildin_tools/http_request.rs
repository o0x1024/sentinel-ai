//! HTTP request tool using rig-core Tool trait

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// HTTP request arguments
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct HttpRequestArgs {
    /// Target URL
    pub url: String,
    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    #[serde(default = "default_method")]
    pub method: String,
    /// Request headers as key-value pairs
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request body (for POST, PUT, etc.)
    #[serde(default)]
    pub body: Option<String>,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Follow redirects
    #[serde(default = "default_follow_redirects")]
    pub follow_redirects: bool,
}

fn default_method() -> String { "GET".to_string() }
fn default_timeout() -> u64 { 30 }
fn default_follow_redirects() -> bool { true }

/// HTTP request result
#[derive(Debug, Clone, Serialize)]
pub struct HttpRequestOutput {
    pub url: String,
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub body_length: usize,
    pub response_time_ms: u64,
    pub truncated: bool,
    pub original_size: usize,
}

/// HTTP request errors
#[derive(Debug, thiserror::Error)]
pub enum HttpRequestError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Timeout: {0}")]
    Timeout(String),
}

/// HTTP request tool
#[derive(Debug, Clone)]
pub struct HttpRequestTool {
    client: reqwest::Client,
}

impl Default for HttpRequestTool {
    fn default() -> Self {
        // Create client with proxy support
        let client = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let builder = reqwest::Client::builder().danger_accept_invalid_certs(true);
                let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
                builder.build().unwrap_or_default()
            })
        });
        
        Self { client }
    }
}

impl HttpRequestTool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub const NAME: &'static str = "http_request";
    pub const DESCRIPTION: &'static str = "Make HTTP requests to any URL. Supports GET, POST, PUT, DELETE methods with custom headers and body.";
}

impl Tool for HttpRequestTool {
    const NAME: &'static str = Self::NAME;
    type Args = HttpRequestArgs;
    type Output = HttpRequestOutput;
    type Error = HttpRequestError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(HttpRequestArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let start_time = Instant::now();

        // Parse URL
        let url = reqwest::Url::parse(&args.url)
            .map_err(|e| HttpRequestError::InvalidUrl(e.to_string()))?;

        // Build request
        let method = args.method.to_uppercase();
        let mut request = match method.as_str() {
            "GET" => self.client.get(url.clone()),
            "POST" => self.client.post(url.clone()),
            "PUT" => self.client.put(url.clone()),
            "DELETE" => self.client.delete(url.clone()),
            "HEAD" => self.client.head(url.clone()),
            "PATCH" => self.client.patch(url.clone()),
            _ => return Err(HttpRequestError::RequestFailed(format!("Unsupported method: {}", method))),
        };

        // Add headers
        for (key, value) in &args.headers {
            request = request.header(key.as_str(), value.as_str());
        }

        // Add body
        if let Some(body) = &args.body {
            request = request.body(body.clone());
        }

        // Set timeout
        request = request.timeout(std::time::Duration::from_secs(args.timeout_secs));

        // Send request
        let response = request.send().await
            .map_err(|e| HttpRequestError::RequestFailed(e.to_string()))?;

        let status_code = response.status().as_u16();
        let status_text = response.status().to_string();

        // Collect headers
        let mut headers = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(key.to_string(), v.to_string());
            }
        }

        // Get body
        let body = response.text().await
            .map_err(|e| HttpRequestError::RequestFailed(e.to_string()))?;
        let original_size = body.len();
        
        // Check if body should be stored to container file
        let body = match crate::output_storage::store_output_unified(
            "http_response",
            &body,
            None,
        ).await {
            Ok(storage_result) => storage_result.get_agent_content(),
            Err(e) => {
                tracing::warn!("Failed to store HTTP response to container: {}", e);
                // Fallback: return original body (or truncate if too large)
                if body.len() > 100_000 {
                    let preview = body.chars().take(100_000).collect::<String>();
                    format!("{}\n\n[Response too large, showing first 100K chars. Total: {} KB]", 
                        preview, original_size / 1024)
                } else {
                    body
                }
            }
        };
        
        let truncated = body.contains("[Large Output Stored");
        
        let body_length = body.len();
        let response_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(HttpRequestOutput {
            url: args.url,
            status_code,
            status_text,
            headers,
            body,
            body_length,
            response_time_ms,
            truncated,
            original_size,
        })
    }
}


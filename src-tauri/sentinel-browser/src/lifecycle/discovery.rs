use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::adapter::traits::BrowserError;

static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("Failed to create HTTP client")
});

#[derive(Debug, Clone, Deserialize)]
pub struct DebugEndpoint {
    #[serde(rename = "webSocketDebuggerUrl")]
    pub web_socket_debugger_url: Option<String>,
    #[serde(rename = "devtoolsFrontendUrl")]
    pub devtools_frontend_url: Option<String>,
    #[serde(rename = "type")]
    pub target_type: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BrowserVersion {
    #[serde(rename = "Browser")]
    pub browser: String,
    #[serde(rename = "Protocol-Version")]
    pub protocol_version: String,
    #[serde(rename = "User-Agent")]
    pub user_agent: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub web_socket_debugger_url: String,
}

/// Discover running Chrome instances with remote debugging enabled
pub async fn discover_chrome_endpoints(port: u16) -> Result<Vec<DebugEndpoint>, BrowserError> {
    discover_chrome_endpoints_at("127.0.0.1", port).await
}

/// Discover Chrome endpoints at a specific host
pub async fn discover_chrome_endpoints_at(host: &str, port: u16) -> Result<Vec<DebugEndpoint>, BrowserError> {
    let url = format!("http://{}:{}/json/list", host, port);
    let resp = HTTP_CLIENT.get(&url)
        .send()
        .await
        .map_err(|e| BrowserError::ConnectionFailed(format!(
            "No Chrome instance found at {}:{}: {}", host, port, e
        )))?;

    let text = resp.text().await.map_err(|e| {
        BrowserError::ConnectionFailed(format!("Failed to read response: {}", e))
    })?;

    serde_json::from_str(&text).map_err(|e| {
        BrowserError::ConnectionFailed(format!("Invalid JSON from /json/list: {}", e))
    })
}

/// Get browser version info on localhost
pub async fn get_browser_version(port: u16) -> Result<BrowserVersion, BrowserError> {
    get_browser_version_at("127.0.0.1", port).await
}

/// Get browser version info at a specific host
pub async fn get_browser_version_at(host: &str, port: u16) -> Result<BrowserVersion, BrowserError> {
    let url = format!("http://{}:{}/json/version", host, port);
    let resp = HTTP_CLIENT.get(&url)
        .send()
        .await
        .map_err(|e| BrowserError::ConnectionFailed(format!(
            "Failed to get browser version at {}:{}: {}", host, port, e
        )))?;

    let text = resp.text().await.map_err(|e| {
        BrowserError::ConnectionFailed(format!("Failed to read response: {}", e))
    })?;

    let mut version: BrowserVersion = serde_json::from_str(&text).map_err(|e| {
        BrowserError::ConnectionFailed(format!(
            "Invalid JSON from /json/version: {} — body: {}",
            e,
            &text[..text.len().min(200)]
        ))
    })?;

    // When connecting to a remote host, the WebSocket URL returned by Chrome
    // uses the container's internal address. Replace it with the actual host.
    if host != "127.0.0.1" && host != "localhost" {
        version.web_socket_debugger_url = version
            .web_socket_debugger_url
            .replace("127.0.0.1", host)
            .replace("localhost", host);
    }

    Ok(version)
}

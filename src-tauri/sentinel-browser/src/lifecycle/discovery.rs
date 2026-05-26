use serde::Deserialize;

use crate::adapter::traits::BrowserError;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugEndpoint {
    pub web_socket_debugger_url: Option<String>,
    pub devtools_frontend_url: Option<String>,
    #[serde(rename = "type")]
    pub target_type: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserVersion {
    pub browser: String,
    pub protocol_version: String,
    pub user_agent: String,
    pub web_socket_debugger_url: String,
}

/// Discover running Chrome instances with remote debugging enabled
pub async fn discover_chrome_endpoints(port: u16) -> Result<Vec<DebugEndpoint>, BrowserError> {
    let url = format!("http://127.0.0.1:{}/json/list", port);
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| BrowserError::ConnectionFailed(format!(
            "No Chrome instance found on port {}: {}", port, e
        )))?;

    let endpoints: Vec<DebugEndpoint> = resp
        .json()
        .await
        .map_err(|e| BrowserError::ConnectionFailed(format!("Invalid response: {}", e)))?;

    Ok(endpoints)
}

/// Get browser version info
pub async fn get_browser_version(port: u16) -> Result<BrowserVersion, BrowserError> {
    let url = format!("http://127.0.0.1:{}/json/version", port);
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| BrowserError::ConnectionFailed(format!(
            "Failed to get browser version: {}", e
        )))?;

    resp.json()
        .await
        .map_err(|e| BrowserError::ConnectionFailed(format!("Invalid response: {}", e)))
}

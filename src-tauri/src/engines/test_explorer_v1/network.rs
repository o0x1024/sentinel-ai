//! Network listener module for Test Explorer V1
//!
//! This module provides network request capture functionality.
//! When using MCP backend, network capture is delegated to the Playwright MCP server.

use std::time::Duration;
use anyhow::Result;
use tracing::{debug, info};

use super::types::ApiRequest;
use super::driver::BrowserDriver;

/// Network listener for capturing HTTP requests/responses
/// 
/// This is a wrapper that delegates to BrowserDriver's MCP-based network capture.
#[derive(Clone)]
pub struct NetworkListener {
    // No internal state needed - uses BrowserDriver
}

impl NetworkListener {
    /// Create a new network listener
    pub fn new() -> Self {
        info!("Initializing NetworkListener (MCP backend)");
        Self {}
    }

    /// Start listening to network events via the browser driver
    pub async fn start(&self, driver: &BrowserDriver) -> Result<()> {
        driver.start_network_capture().await
    }

    /// Get all captured requests via the browser driver
    pub async fn get_requests(&self, driver: &BrowserDriver) -> Vec<ApiRequest> {
        driver.get_captured_requests().await
    }

    /// Wait for a request matching the given URL pattern
    pub async fn wait_for_request(
        &self,
        driver: &BrowserDriver,
        pattern: &str,
        timeout: Duration,
    ) -> Result<ApiRequest> {
        driver.wait_for_request(pattern, timeout).await
    }

    /// Clear all captured requests
    pub async fn clear(&self, _driver: &BrowserDriver) {
        // Not directly supported via MCP, but we can log the intent
        debug!("Clear captured requests (not supported in MCP mode)");
    }

    /// Export requests as HAR (HTTP Archive) format
    pub async fn export_har(&self, driver: &BrowserDriver) -> serde_json::Value {
        let requests = driver.get_captured_requests().await;

        let entries: Vec<serde_json::Value> = requests
            .iter()
            .map(|req| {
                serde_json::json!({
                    "startedDateTime": chrono::DateTime::<chrono::Utc>::from(req.timestamp).to_rfc3339(),
                    "time": 0,
                    "request": {
                        "method": req.method,
                        "url": req.url,
                        "httpVersion": "HTTP/1.1",
                        "headers": req.headers.iter().map(|(k, v)| {
                            serde_json::json!({"name": k, "value": v})
                        }).collect::<Vec<_>>(),
                        "queryString": [],
                        "postData": req.request_body.as_ref().map(|body| {
                            serde_json::json!({
                                "mimeType": "application/json",
                                "text": body
                            })
                        }),
                        "headersSize": -1,
                        "bodySize": req.request_body.as_ref().map(|b| b.len() as i64).unwrap_or(-1)
                    },
                    "response": {
                        "status": req.response_status.unwrap_or(0),
                        "statusText": "",
                        "httpVersion": "HTTP/1.1",
                        "headers": req.response_headers.as_ref().map(|headers| {
                            headers.iter().map(|(k, v)| {
                                serde_json::json!({"name": k, "value": v})
                            }).collect::<Vec<_>>()
                        }).unwrap_or_default(),
                        "content": {
                            "size": req.response_body.as_ref().map(|b| b.len() as i64).unwrap_or(-1),
                            "mimeType": "application/json",
                            "text": req.response_body.as_ref().unwrap_or(&String::new())
                        },
                        "redirectURL": "",
                        "headersSize": -1,
                        "bodySize": req.response_body.as_ref().map(|b| b.len() as i64).unwrap_or(-1)
                    },
                    "cache": {},
                    "timings": {
                        "send": 0,
                        "wait": 0,
                        "receive": 0
                    }
                })
            })
            .collect();

        serde_json::json!({
            "log": {
                "version": "1.2",
                "creator": {
                    "name": "Sentinel AI Test Explorer V1",
                    "version": "1.0.0"
                },
                "entries": entries
            }
        })
    }
}

impl Default for NetworkListener {
    fn default() -> Self {
        Self::new()
    }
}

//! Network listener for capturing API requests

use anyhow::{anyhow, Result};
use chromiumoxide::cdp::browser_protocol::network::{
    EnableParams, EventRequestWillBeSent, EventResponseReceived,
};
use chromiumoxide::page::Page;
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::types::ApiRequest;

/// Network listener for capturing HTTP requests/responses
#[derive(Clone)]
pub struct NetworkListener {
    page: Arc<Page>,
    requests: Arc<RwLock<Vec<ApiRequest>>>,
    pending_requests: Arc<RwLock<HashMap<String, PendingRequest>>>,
}

/// Pending request waiting for response
#[derive(Debug, Clone)]
struct PendingRequest {
    request_id: String,
    method: String,
    url: String,
    headers: HashMap<String, String>,
    request_body: Option<String>,
    resource_type: String,
    timestamp: SystemTime,
}

impl NetworkListener {
    /// Create a new network listener
    pub async fn new(page: Arc<Page>) -> Result<Self> {
        info!("Initializing NetworkListener");

        // Enable network tracking
        page.execute(EnableParams::default())
            .await
            .map_err(|e| anyhow!("Failed to enable network tracking: {}", e))?;

        let listener = Self {
            page: page.clone(),
            requests: Arc::new(RwLock::new(Vec::new())),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        };

        // Start listening to network events
        listener.start_listening().await?;

        Ok(listener)
    }

    /// Start listening to network events
    async fn start_listening(&self) -> Result<()> {
        let page = self.page.clone();
        let requests = self.requests.clone();
        let pending_requests = self.pending_requests.clone();

        // Listen for request events
        let mut request_stream = page
            .event_listener::<EventRequestWillBeSent>()
            .await
            .map_err(|e| anyhow!("Failed to create request listener: {}", e))?;

        let pending_clone = pending_requests.clone();
        tokio::spawn(async move {
            while let Some(event) = request_stream.next().await {
                let request_id = format!("{:?}", event.request_id);
                let request = &event.request;

                // Convert headers - chromiumoxide Headers is a HashMap-like structure
                let mut headers = HashMap::new();
                let headers_map = &request.headers;
                // Headers in chromiumoxide is a serde_json::Map
                if let serde_json::Value::Object(map) = serde_json::to_value(headers_map).unwrap_or(serde_json::json!({})) {
                    for (key, value) in map.iter() {
                        if let Some(v) = value.as_str() {
                            headers.insert(key.clone(), v.to_string());
                        }
                    }
                }

                let pending = PendingRequest {
                    request_id: request_id.clone(),
                    method: request.method.clone(),
                    url: request.url.clone(),
                    headers,
                    request_body: None, // TODO: chromiumoxide Binary type doesn't expose inner data
                    resource_type: "XHR".to_string(), // Simplified: chromiumoxide doesn't expose resource type directly
                    timestamp: SystemTime::now(),
                };

                pending_clone.write().await.insert(request_id, pending);
            }
        });

        // Listen for response events
        let mut response_stream = page
            .event_listener::<EventResponseReceived>()
            .await
            .map_err(|e| anyhow!("Failed to create response listener: {}", e))?;

        let page_clone = page.clone();
        tokio::spawn(async move {
            while let Some(event) = response_stream.next().await {
                let request_id = format!("{:?}", event.request_id);
                let response = &event.response;

                // Get pending request
                let pending = pending_requests.write().await.remove(&request_id);

                if let Some(pending) = pending {
                    // Convert response headers
                    let mut response_headers = HashMap::new();
                    if let serde_json::Value::Object(map) = serde_json::to_value(&response.headers).unwrap_or(serde_json::json!({})) {
                        for (key, value) in map.iter() {
                            if let Some(v) = value.as_str() {
                                response_headers.insert(key.clone(), v.to_string());
                            }
                        }
                    }

                    // Try to get response body (may fail for some requests)
                    // Note: We skip body retrieval for now as it requires proper RequestId handling
                    let response_body: Option<String> = None;

                    let api_request = ApiRequest {
                        request_id: pending.request_id,
                        method: pending.method,
                        url: pending.url,
                        headers: pending.headers,
                        request_body: pending.request_body,
                        response_status: Some(response.status as u16),
                        response_headers: Some(response_headers),
                        response_body,
                        timestamp: pending.timestamp,
                        resource_type: pending.resource_type,
                    };

                    // Filter: only capture XHR, Fetch, and Document requests
                    if matches!(
                        api_request.resource_type.as_str(),
                        "XHR" | "Fetch" | "Document"
                    ) {
                        debug!(
                            "Captured API request: {} {}",
                            api_request.method, api_request.url
                        );
                        requests.write().await.push(api_request);
                    }
                }
            }
        });

        info!("Network listener started");
        Ok(())
    }

    /// Get all captured requests
    pub async fn get_requests(&self) -> Vec<ApiRequest> {
        self.requests.read().await.clone()
    }

    /// Wait for a request matching the given URL pattern
    pub async fn wait_for_request(
        &self,
        pattern: &str,
        timeout: Duration,
    ) -> Result<ApiRequest> {
        let start = std::time::Instant::now();
        let pattern = pattern.to_lowercase();

        loop {
            if start.elapsed() > timeout {
                return Err(anyhow!(
                    "Timeout waiting for request matching pattern: {}",
                    pattern
                ));
            }

            let requests = self.requests.read().await;
            if let Some(request) = requests
                .iter()
                .find(|r| r.url.to_lowercase().contains(&pattern))
            {
                return Ok(request.clone());
            }

            drop(requests);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Clear all captured requests
    pub async fn clear(&self) {
        self.requests.write().await.clear();
        self.pending_requests.write().await.clear();
        info!("Cleared all captured requests");
    }

    /// Export requests as HAR (HTTP Archive) format
    pub async fn export_har(&self) -> serde_json::Value {
        let requests = self.requests.read().await;

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


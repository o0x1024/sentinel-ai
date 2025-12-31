//! Web search tool using Tavily API

use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Web search arguments
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WebSearchArgs {
    /// Search query
    pub query: String,
    /// Maximum number of results (default: 5)
    #[serde(default = "default_max_results")]
    pub max_results: u32,
    /// Search depth: "basic" or "advanced" (default: "basic")
    #[serde(default = "default_search_depth")]
    pub search_depth: String,
}

fn default_max_results() -> u32 { 5 }
fn default_search_depth() -> String { "basic".to_string() }

/// Web search result item
#[derive(Debug, Clone, Serialize)]
pub struct SearchResultItem {
    pub title: String,
    pub url: String,
    pub content: String,
}

/// Web search output
#[derive(Debug, Clone, Serialize)]
pub struct WebSearchOutput {
    pub query: String,
    pub results: Vec<SearchResultItem>,
    pub total_results: usize,
    pub source: String,
}

/// Web search errors
#[derive(Debug, thiserror::Error)]
pub enum WebSearchError {
    #[error("API key not configured: {0}")]
    ApiKeyNotConfigured(String),
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Web search tool using Tavily API
#[derive(Debug, Clone)]
pub struct WebSearchTool {
    api_key: Option<String>,
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new(None)
    }
}

impl WebSearchTool {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }

    pub fn with_api_key(api_key: String) -> Self {
        Self { api_key: Some(api_key) }
    }

    /// Get API key from environment or stored value
    fn get_api_key(&self) -> Result<String, WebSearchError> {
        self.api_key.clone()
            .or_else(|| std::env::var("TAVILY_API_KEY").ok())
            .ok_or_else(|| WebSearchError::ApiKeyNotConfigured(
                "TAVILY_API_KEY not configured. Set it in environment or AI settings.".to_string()
            ))
    }
}

impl Tool for WebSearchTool {
    const NAME: &'static str = "web_search";
    type Args = WebSearchArgs;
    type Output = WebSearchOutput;
    type Error = WebSearchError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search the web for real-time information using Tavily API. Returns relevant search results with titles, URLs, and content snippets. Useful for finding current information, documentation, CVEs, security advisories, and CTF writeups.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(WebSearchArgs))
                .unwrap_or_default(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let api_key = self.get_api_key()?;

        // Build HTTP client with proxy support
        let client = {
            let builder = reqwest::Client::builder()
                .timeout(Duration::from_secs(30));
            let builder = sentinel_core::global_proxy::apply_proxy_to_client(builder).await;
            builder.build()
                .map_err(|e| WebSearchError::RequestFailed(format!("Failed to build HTTP client: {}", e)))?
        };

        // Prepare request payload
        let payload = serde_json::json!({
            "query": args.query,
            "max_results": args.max_results,
            "include_answer": false,
            "include_raw_content": false,
            "search_depth": args.search_depth
        });

        // Make API request
        let resp = client
            .post("https://api.tavily.com/search")
            .bearer_auth(&api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| WebSearchError::RequestFailed(format!("Failed to call Tavily API: {}", e)))?;

        // Check response status
        if !resp.status().is_success() {
            let err_txt = resp.text().await.unwrap_or_default();
            return Err(WebSearchError::RequestFailed(format!("Tavily API error: {}", err_txt)));
        }

        // Parse response
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WebSearchError::ParseError(format!("Failed to parse Tavily response: {}", e)))?;

        // Extract results
        let mut results = Vec::new();
        if let Some(results_array) = json.get("results").and_then(|r| r.as_array()) {
            for item in results_array {
                let title = item.get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let url = item.get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let content = item.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                results.push(SearchResultItem {
                    title,
                    url,
                    content,
                });
            }
        }

        let total_results = results.len();

        Ok(WebSearchOutput {
            query: args.query,
            results,
            total_results,
            source: "Tavily".to_string(),
        })
    }
}

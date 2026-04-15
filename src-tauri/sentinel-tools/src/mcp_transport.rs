use std::collections::HashMap;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};
use rmcp::service::RunningService;
use rmcp::transport::{
    sse_client::SseClientConfig, streamable_http_client::StreamableHttpClientTransportConfig,
    SseClientTransport, StreamableHttpClientTransport, TokioChildProcess,
};
use rmcp::{RoleClient, ServiceExt};
use serde::{Deserialize, Serialize};
use tokio::process::Command as TokioCommand;

pub type McpClient = RunningService<RoleClient, ClientInfo>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpTransportConfig {
    pub transport_type: String,
    pub url: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

pub fn normalize_transport_type(transport_type: &str) -> String {
    match transport_type.trim() {
        "" => "stdio".to_string(),
        "stdio" => "stdio".to_string(),
        "sse" => "sse".to_string(),
        "http" | "streamableHttp" | "streamable_http" | "streamable-http" => {
            "streamableHttp".to_string()
        }
        other => other.to_string(),
    }
}

pub fn create_client_info() -> ClientInfo {
    ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "sentinel-ai".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        },
    }
}

pub fn build_reqwest_client(headers: &HashMap<String, String>) -> Result<reqwest::Client, String> {
    let mut header_map = HeaderMap::new();

    for (key, value) in headers {
        let header_name = HeaderName::from_bytes(key.as_bytes())
            .map_err(|e| format!("Invalid MCP header name '{}': {}", key, e))?;
        let header_value = HeaderValue::from_str(value)
            .map_err(|e| format!("Invalid MCP header value for '{}': {}", key, e))?;
        header_map.insert(header_name, header_value);
    }

    reqwest::Client::builder()
        .default_headers(header_map)
        .build()
        .map_err(|e| format!("Failed to build HTTP client for MCP transport: {}", e))
}

pub async fn connect_mcp_client(config: &McpTransportConfig) -> Result<McpClient, String> {
    let client_info = create_client_info();
    let transport_type = normalize_transport_type(&config.transport_type);

    match transport_type.as_str() {
        "stdio" => {
            if config.command.trim().is_empty() {
                return Err("MCP stdio transport requires a non-empty command".to_string());
            }

            let mut cmd = TokioCommand::new(&config.command);
            cmd.args(&config.args);

            let transport = TokioChildProcess::new(cmd)
                .map_err(|e| format!("Failed to create MCP stdio transport: {}", e))?;

            client_info
                .serve(transport)
                .await
                .map_err(|e| format!("Failed to connect to MCP stdio server: {}", e))
        }
        "sse" => {
            if config.url.trim().is_empty() {
                return Err("MCP SSE transport requires a non-empty URL".to_string());
            }

            let http_client = build_reqwest_client(&config.headers)?;
            let transport = SseClientTransport::start_with_client(
                http_client,
                SseClientConfig {
                    sse_endpoint: config.url.clone().into(),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| format!("Failed to create MCP SSE transport: {}", e))?;

            client_info
                .serve(transport)
                .await
                .map_err(|e| format!("Failed to connect to MCP SSE server: {}", e))
        }
        "streamableHttp" => {
            if config.url.trim().is_empty() {
                return Err("MCP streamable HTTP transport requires a non-empty URL".to_string());
            }

            let http_client = build_reqwest_client(&config.headers)?;
            let transport = StreamableHttpClientTransport::with_client(
                http_client,
                StreamableHttpClientTransportConfig::with_uri(config.url.clone()),
            );

            client_info
                .serve(transport)
                .await
                .map_err(|e| format!("Failed to connect to MCP streamable HTTP server: {}", e))
        }
        other => Err(format!("Unsupported MCP transport type: {}", other)),
    }
}

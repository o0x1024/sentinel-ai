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

/// Retrieve the user's full shell PATH, cached for the process lifetime.
///
/// On macOS, app bundles launched via Finder/Dock inherit a minimal PATH
/// (`/usr/bin:/bin:/usr/sbin:/sbin`). On Linux, apps launched from desktop
/// entries or systemd may also have an incomplete PATH.
/// On Windows, PATH is always inherited from the registry so this is a no-op.
///
/// This function runs the user's login shell to obtain the real PATH so that
/// MCP stdio child processes can locate binaries in non-system directories
/// (Homebrew, cargo, nvm, etc.).
fn get_user_shell_path() -> Option<String> {
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    static CACHED_PATH: Lazy<Mutex<Option<String>>> = Lazy::new(|| {
        let result = resolve_full_path();
        Mutex::new(result)
    });

    CACHED_PATH.lock().ok().and_then(|guard| guard.clone())
}

#[cfg(target_os = "windows")]
fn resolve_full_path() -> Option<String> {
    // Windows always inherits the full PATH from registry/system environment.
    None
}

#[cfg(not(target_os = "windows"))]
fn resolve_full_path() -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| {
        // SHELL may be absent in app bundle environments; probe common shells
        for candidate in &["/bin/zsh", "/bin/bash", "/bin/sh"] {
            if std::path::Path::new(candidate).exists() {
                return candidate.to_string();
            }
        }
        "/bin/sh".to_string()
    });

    let shell_name = std::path::Path::new(&shell)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("sh");

    // fish outputs PATH entries space-separated; use printf for consistency
    let print_cmd = if shell_name == "fish" {
        "printf '%s' $PATH"
    } else {
        "printf '%s' \"$PATH\""
    };

    std::process::Command::new(&shell)
        .args(["-l", "-c", print_cmd])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if path.is_empty() || path == std::env::var("PATH").unwrap_or_default() {
                    None
                } else {
                    Some(path)
                }
            } else {
                None
            }
        })
}

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

            if let Some(full_path) = get_user_shell_path() {
                cmd.env("PATH", full_path);
            }

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

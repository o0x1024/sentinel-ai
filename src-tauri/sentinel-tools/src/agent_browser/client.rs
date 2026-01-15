//! Socket client for communicating with agent-browser daemon

use super::daemon::get_socket_path;
use super::types::{BrowserCommand, BrowserResponse};
use anyhow::{Context, Result};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use tracing::debug;
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(windows)]
use std::net::TcpStream;

/// Client for communicating with agent-browser daemon
pub struct BrowserClient {
    session: String,
}

impl BrowserClient {
    pub fn new(session: &str) -> Self {
        Self {
            session: session.to_string(),
        }
    }

    /// Send a command to the daemon and get response
    pub fn send_command(&self, action: &str, params: Value) -> Result<BrowserResponse> {
        let command = BrowserCommand {
            id: Uuid::new_v4().to_string(),
            action: action.to_string(),
            params,
        };

        let command_json = serde_json::to_string(&command)?;
        debug!("Sending command: {}", command_json);

        let response_line = self.send_raw(&command_json)?;
        debug!("Received response: {}", response_line);

        let response: BrowserResponse =
            serde_json::from_str(&response_line).context("Failed to parse daemon response")?;

        Ok(response)
    }

    /// Send raw command string and get response
    fn send_raw(&self, command: &str) -> Result<String> {
        #[cfg(unix)]
        {
            self.send_unix(command)
        }
        #[cfg(windows)]
        {
            self.send_tcp(command)
        }
    }

    #[cfg(unix)]
    fn send_unix(&self, command: &str) -> Result<String> {
        let socket_path = get_socket_path(&self.session);
        let mut stream =
            UnixStream::connect(&socket_path).context("Failed to connect to daemon socket")?;

        // Use short timeouts to avoid blocking the UI
        // If daemon is unresponsive, fail fast rather than hang
        stream.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(3)))?;

        // Send command with newline
        writeln!(stream, "{}", command)?;
        stream.flush()?;

        // Read response
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;

        Ok(response.trim().to_string())
    }

    #[cfg(windows)]
    fn send_tcp(&self, command: &str) -> Result<String> {
        let port: u16 = get_socket_path(&self.session)
            .parse()
            .context("Invalid port")?;
        let mut stream = TcpStream::connect_timeout(
            &format!("127.0.0.1:{}", port).parse().unwrap(),
            std::time::Duration::from_secs(3)
        ).context("Failed to connect to daemon")?;

        // Use short timeouts to avoid blocking the UI
        stream.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(3)))?;

        // Send command with newline
        writeln!(stream, "{}", command)?;
        stream.flush()?;

        // Read response
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;

        Ok(response.trim().to_string())
    }

    /// Execute a command and extract data on success
    pub fn execute(&self, action: &str, params: Value) -> Result<Value> {
        let response = self.send_command(action, params)?;

        if response.success {
            Ok(response.data)
        } else {
            let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
            anyhow::bail!("{}", error_msg)
        }
    }

    /// Navigate to URL
    pub fn navigate(&self, url: &str, wait_until: Option<&str>) -> Result<Value> {
        let mut params = serde_json::json!({ "url": url });
        if let Some(wu) = wait_until {
            params["waitUntil"] = Value::String(wu.to_string());
        }
        self.execute("navigate", params)
    }

    /// Get snapshot
    pub fn snapshot(
        &self,
        interactive: bool,
        compact: bool,
        max_depth: Option<u32>,
        selector: Option<&str>,
    ) -> Result<Value> {
        let mut params = serde_json::json!({});
        if interactive {
            params["interactive"] = Value::Bool(true);
        }
        if compact {
            params["compact"] = Value::Bool(true);
        }
        if let Some(depth) = max_depth {
            params["maxDepth"] = Value::Number(depth.into());
        }
        if let Some(sel) = selector {
            params["selector"] = Value::String(sel.to_string());
        }
        self.execute("snapshot", params)
    }

    /// Click element
    pub fn click(&self, selector: &str) -> Result<Value> {
        self.execute("click", serde_json::json!({ "selector": selector }))
    }

    /// Fill input
    pub fn fill(&self, selector: &str, value: &str) -> Result<Value> {
        self.execute(
            "fill",
            serde_json::json!({ "selector": selector, "value": value }),
        )
    }

    /// Type text (character by character)
    pub fn type_text(&self, selector: &str, text: &str, delay: Option<u32>) -> Result<Value> {
        let mut params = serde_json::json!({ "selector": selector, "text": text });
        if let Some(d) = delay {
            params["delay"] = Value::Number(d.into());
        }
        self.execute("type", params)
    }

    /// Press key
    pub fn press(&self, key: &str, selector: Option<&str>) -> Result<Value> {
        let mut params = serde_json::json!({ "key": key });
        if let Some(sel) = selector {
            params["selector"] = Value::String(sel.to_string());
        }
        self.execute("press", params)
    }

    /// Select option
    pub fn select(&self, selector: &str, values: &[&str]) -> Result<Value> {
        self.execute(
            "select",
            serde_json::json!({ "selector": selector, "values": values }),
        )
    }

    /// Hover element
    pub fn hover(&self, selector: &str) -> Result<Value> {
        self.execute("hover", serde_json::json!({ "selector": selector }))
    }

    /// Scroll page
    pub fn scroll(&self, direction: &str, amount: Option<u32>) -> Result<Value> {
        let mut params = serde_json::json!({ "direction": direction });
        if let Some(amt) = amount {
            params["amount"] = Value::Number(amt.into());
        }
        self.execute("scroll", params)
    }

    /// Take screenshot
    pub fn screenshot(&self, full_page: bool, path: Option<&str>) -> Result<Value> {
        let mut params = serde_json::json!({ "fullPage": full_page });
        if let Some(p) = path {
            params["path"] = Value::String(p.to_string());
        }
        self.execute("screenshot", params)
    }

    /// Wait for selector/time
    pub fn wait(&self, selector: Option<&str>, timeout_ms: Option<u64>) -> Result<Value> {
        let mut params = serde_json::json!({});
        if let Some(sel) = selector {
            params["selector"] = Value::String(sel.to_string());
        }
        if let Some(timeout) = timeout_ms {
            params["timeout"] = Value::Number(timeout.into());
        }
        self.execute("wait", params)
    }

    /// Go back
    pub fn back(&self) -> Result<Value> {
        self.execute("back", serde_json::json!({}))
    }

    /// Go forward
    pub fn forward(&self) -> Result<Value> {
        self.execute("forward", serde_json::json!({}))
    }

    /// Reload page
    pub fn reload(&self) -> Result<Value> {
        self.execute("reload", serde_json::json!({}))
    }

    /// Get current URL
    pub fn get_url(&self) -> Result<Value> {
        self.execute("url", serde_json::json!({}))
    }

    /// Get page title
    pub fn get_title(&self) -> Result<Value> {
        self.execute("title", serde_json::json!({}))
    }

    /// Get text content
    pub fn get_text(&self, selector: &str) -> Result<Value> {
        self.execute("gettext", serde_json::json!({ "selector": selector }))
    }

    /// Get attribute
    pub fn get_attribute(&self, selector: &str, attribute: &str) -> Result<Value> {
        self.execute(
            "getattribute",
            serde_json::json!({ "selector": selector, "attribute": attribute }),
        )
    }

    /// Check if visible
    pub fn is_visible(&self, selector: &str) -> Result<Value> {
        self.execute("isvisible", serde_json::json!({ "selector": selector }))
    }

    /// Evaluate JavaScript
    pub fn evaluate(&self, script: &str) -> Result<Value> {
        self.execute("evaluate", serde_json::json!({ "script": script }))
    }

    /// Get page content (HTML)
    pub fn content(&self, selector: Option<&str>) -> Result<Value> {
        let mut params = serde_json::json!({});
        if let Some(sel) = selector {
            params["selector"] = Value::String(sel.to_string());
        }
        self.execute("content", params)
    }

    /// Get cookies
    pub fn get_cookies(&self, urls: Option<&[&str]>) -> Result<Value> {
        let mut params = serde_json::json!({});
        if let Some(u) = urls {
            params["urls"] = Value::Array(u.iter().map(|s| Value::String(s.to_string())).collect());
        }
        self.execute("cookies_get", params)
    }

    /// Set cookies
    pub fn set_cookies(&self, cookies: &[serde_json::Value]) -> Result<Value> {
        self.execute("cookies_set", serde_json::json!({ "cookies": cookies }))
    }

    /// Clear cookies
    pub fn clear_cookies(&self) -> Result<Value> {
        self.execute("cookies_clear", serde_json::json!({}))
    }

    /// Get localStorage
    pub fn get_local_storage(&self, key: Option<&str>) -> Result<Value> {
        let mut params = serde_json::json!({ "type": "local" });
        if let Some(k) = key {
            params["key"] = Value::String(k.to_string());
        }
        self.execute("storage_get", params)
    }

    /// Set localStorage
    pub fn set_local_storage(&self, key: &str, value: &str) -> Result<Value> {
        self.execute(
            "storage_set",
            serde_json::json!({ "type": "local", "key": key, "value": value }),
        )
    }

    /// List tabs
    pub fn list_tabs(&self) -> Result<Value> {
        self.execute("tab_list", serde_json::json!({}))
    }

    /// New tab
    pub fn new_tab(&self) -> Result<Value> {
        self.execute("tab_new", serde_json::json!({}))
    }

    /// Switch tab
    pub fn switch_tab(&self, index: u32) -> Result<Value> {
        self.execute("tab_switch", serde_json::json!({ "index": index }))
    }

    /// Close tab
    pub fn close_tab(&self, index: Option<u32>) -> Result<Value> {
        let mut params = serde_json::json!({});
        if let Some(i) = index {
            params["index"] = Value::Number(i.into());
        }
        self.execute("tab_close", params)
    }

    /// Set viewport
    pub fn set_viewport(&self, width: u32, height: u32) -> Result<Value> {
        self.execute(
            "viewport",
            serde_json::json!({ "width": width, "height": height }),
        )
    }

    /// Close browser
    pub fn close(&self) -> Result<Value> {
        self.execute("close", serde_json::json!({}))
    }

    /// Launch browser with options
    pub fn launch(&self, headless: bool, viewport: Option<(u32, u32)>) -> Result<Value> {
        let mut params = serde_json::json!({ "headless": headless });
        if let Some((w, h)) = viewport {
            params["viewport"] = serde_json::json!({ "width": w, "height": h });
        }
        self.execute("launch", params)
    }

    // ==================== Network Interception ====================

    /// Get tracked network requests (also starts tracking if not already)
    pub fn requests(&self, filter: Option<&str>, clear: bool) -> Result<Value> {
        let mut params = serde_json::json!({});
        if let Some(f) = filter {
            params["filter"] = serde_json::json!(f);
        }
        if clear {
            params["clear"] = serde_json::json!(true);
        }
        self.execute("requests", params)
    }

    /// Start tracking network requests
    pub fn start_request_tracking(&self) -> Result<Value> {
        // Calling requests without filter starts tracking
        self.requests(None, false)
    }

    /// Clear tracked network requests
    pub fn clear_requests(&self) -> Result<Value> {
        self.requests(None, true)
    }
}

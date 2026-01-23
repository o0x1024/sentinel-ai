//! AgentBrowserService - High-level browser automation service
//!
//! Provides a unified interface for browser operations, managing daemon lifecycle
//! and exposing operations for AI assistant tools.

use super::client::BrowserClient;
use super::daemon::{ensure_daemon, is_daemon_running, cleanup_daemon_files};
use super::types::*;
use anyhow::Result;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Global browser service instance
pub static BROWSER_SERVICE: Lazy<Arc<RwLock<AgentBrowserService>>> =
    Lazy::new(|| Arc::new(RwLock::new(AgentBrowserService::new())));

/// Get the global browser service
pub async fn get_browser_service() -> Arc<RwLock<AgentBrowserService>> {
    BROWSER_SERVICE.clone()
}

/// Browser automation service
pub struct AgentBrowserService {
    config: BrowserConfig,
    initialized: bool,
}

impl AgentBrowserService {
    pub fn new() -> Self {
        Self {
            config: BrowserConfig::default(),
            initialized: false,
        }
    }

    /// Initialize the browser service
    pub async fn init(&mut self, config: Option<BrowserConfig>) -> Result<()> {
        if let Some(cfg) = config {
            self.config = cfg;
        }

        // Ensure daemon is running
        ensure_daemon(&self.config.session)?;

        // Launch browser with config
        let client = self.client();
        let viewport = Some((self.config.viewport_width, self.config.viewport_height));
        client.launch(self.config.headless, viewport)?;

        self.initialized = true;
        info!("Browser service initialized with session: {}", self.config.session);
        Ok(())
    }

    /// Ensure service is initialized
    async fn ensure_init(&mut self) -> Result<()> {
        if !self.initialized {
            self.init(None).await?;
        } else if !is_daemon_running(&self.config.session) {
            // Daemon died, restart it
            warn!("Daemon not running, restarting...");
            ensure_daemon(&self.config.session)?;
            let client = self.client();
            let viewport = Some((self.config.viewport_width, self.config.viewport_height));
            client.launch(self.config.headless, viewport)?;
        }
        Ok(())
    }

    /// Get client instance
    fn client(&self) -> BrowserClient {
        BrowserClient::new(&self.config.session)
    }

    // ==================== Navigation ====================

    /// Open a URL
    pub async fn open(&mut self, url: &str, wait_until: Option<&str>, headless: Option<bool>) -> Result<NavigateResult> {
        self.ensure_init().await?;
        
        // If headless mode is specified and different from current, update it
        if let Some(headless_mode) = headless {
            if self.config.headless != headless_mode {
                self.set_headless(headless_mode).await?;
            }
        }
        
        let client = self.client();
        let result = client.navigate(url, wait_until)?;

        Ok(NavigateResult {
            url: result["url"].as_str().unwrap_or(url).to_string(),
            title: result["title"].as_str().unwrap_or("").to_string(),
        })
    }

    /// Go back
    pub async fn back(&mut self) -> Result<String> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.back()?;
        Ok(result["url"].as_str().unwrap_or("").to_string())
    }

    /// Go forward
    pub async fn forward(&mut self) -> Result<String> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.forward()?;
        Ok(result["url"].as_str().unwrap_or("").to_string())
    }

    /// Reload page
    pub async fn reload(&mut self) -> Result<String> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.reload()?;
        Ok(result["url"].as_str().unwrap_or("").to_string())
    }

    // ==================== Page Analysis ====================

    /// Get page snapshot (ARIA tree with refs)
    pub async fn snapshot(&mut self, options: SnapshotOptions) -> Result<Snapshot> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.snapshot(
            options.interactive,
            options.compact,
            options.max_depth,
            options.selector.as_deref(),
        )?;

        let tree = result["snapshot"].as_str().unwrap_or("").to_string();

        // Parse refs
        let mut refs = HashMap::new();
        if let Some(refs_obj) = result["refs"].as_object() {
            for (key, value) in refs_obj {
                refs.insert(
                    key.clone(),
                    RefData {
                        role: value["role"].as_str().unwrap_or("").to_string(),
                        name: value["name"].as_str().map(|s| s.to_string()),
                        nth: value["nth"].as_u64().map(|n| n as u32),
                    },
                );
            }
        }

        Ok(Snapshot { tree, refs })
    }

    /// Take screenshot
    pub async fn screenshot(&mut self, full_page: bool) -> Result<ScreenshotResult> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.screenshot(full_page, None)?;

        Ok(ScreenshotResult {
            base64: result["base64"].as_str().map(|s| s.to_string()),
            path: result["path"].as_str().map(|s| s.to_string()),
        })
    }

    /// Get current URL
    pub async fn get_url(&mut self) -> Result<String> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.get_url()?;
        Ok(result["url"].as_str().unwrap_or("").to_string())
    }

    /// Get page title
    pub async fn get_title(&mut self) -> Result<String> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.get_title()?;
        Ok(result["title"].as_str().unwrap_or("").to_string())
    }

    /// Get page HTML content
    pub async fn get_content(&mut self, selector: Option<&str>) -> Result<String> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.content(selector)?;
        Ok(result["html"].as_str().unwrap_or("").to_string())
    }

    // ==================== Element Interaction ====================

    /// Click element by ref (@e1) or selector
    pub async fn click(&mut self, target: &str) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        client.click(target)?;
        Ok(())
    }

    /// Fill input field
    pub async fn fill(&mut self, target: &str, value: &str) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        client.fill(target, value)?;
        Ok(())
    }

    /// Type text character by character
    pub async fn type_text(&mut self, target: &str, text: &str, delay_ms: Option<u32>) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        client.type_text(target, text, delay_ms)?;
        Ok(())
    }

    /// Press key
    pub async fn press(&mut self, key: &str, target: Option<&str>) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        client.press(key, target)?;
        Ok(())
    }

    /// Select option from dropdown
    pub async fn select(&mut self, target: &str, values: &[&str]) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        client.select(target, values)?;
        Ok(())
    }

    /// Hover over element
    pub async fn hover(&mut self, target: &str) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        client.hover(target)?;
        Ok(())
    }

    /// Scroll page
    pub async fn scroll(&mut self, direction: ScrollDirection, amount: Option<u32>) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        let dir = match direction {
            ScrollDirection::Up => "up",
            ScrollDirection::Down => "down",
            ScrollDirection::Left => "left",
            ScrollDirection::Right => "right",
        };
        client.scroll(dir, amount)?;
        Ok(())
    }

    // ==================== Wait Operations ====================

    /// Wait for selector or timeout
    pub async fn wait(&mut self, selector: Option<&str>, timeout_ms: Option<u64>) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        client.wait(selector, timeout_ms)?;
        Ok(())
    }

    // ==================== Element Info ====================

    /// Get text content of element
    pub async fn get_text(&mut self, target: &str) -> Result<String> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.get_text(target)?;
        Ok(result["text"].as_str().unwrap_or("").to_string())
    }

    /// Get attribute value
    pub async fn get_attribute(&mut self, target: &str, attribute: &str) -> Result<Option<String>> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.get_attribute(target, attribute)?;
        Ok(result["value"].as_str().map(|s| s.to_string()))
    }

    /// Check if element is visible
    pub async fn is_visible(&mut self, target: &str) -> Result<bool> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.is_visible(target)?;
        Ok(result["visible"].as_bool().unwrap_or(false))
    }

    // ==================== JavaScript ====================

    /// Evaluate JavaScript
    pub async fn evaluate(&mut self, script: &str) -> Result<Value> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.evaluate(script)?;
        Ok(result["result"].clone())
    }

    // ==================== Cookies & Storage ====================

    /// Get cookies
    pub async fn get_cookies(&mut self) -> Result<Vec<Cookie>> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.get_cookies(None)?;

        let cookies = result["cookies"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(cookies)
    }

    /// Set cookies
    pub async fn set_cookies(&mut self, cookies: &[Cookie]) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        let cookies_json: Vec<Value> = cookies
            .iter()
            .filter_map(|c| serde_json::to_value(c).ok())
            .collect();
        client.set_cookies(&cookies_json)?;
        Ok(())
    }

    /// Clear cookies
    pub async fn clear_cookies(&mut self) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        client.clear_cookies()?;
        Ok(())
    }

    /// Get localStorage
    pub async fn get_local_storage(&mut self, key: Option<&str>) -> Result<Value> {
        self.ensure_init().await?;
        let client = self.client();
        client.get_local_storage(key)
    }

    /// Set localStorage
    pub async fn set_local_storage(&mut self, key: &str, value: &str) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        client.set_local_storage(key, value)?;
        Ok(())
    }

    // ==================== Tabs ====================

    /// List tabs
    pub async fn list_tabs(&mut self) -> Result<Vec<TabInfo>> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.list_tabs()?;

        let tabs = result["tabs"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(tabs)
    }

    /// Create new tab
    pub async fn new_tab(&mut self) -> Result<u32> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.new_tab()?;
        Ok(result["index"].as_u64().unwrap_or(0) as u32)
    }

    /// Switch to tab
    pub async fn switch_tab(&mut self, index: u32) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        client.switch_tab(index)?;
        Ok(())
    }

    /// Close tab
    pub async fn close_tab(&mut self, index: Option<u32>) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        client.close_tab(index)?;
        Ok(())
    }

    // ==================== Viewport ====================

    /// Set viewport size
    pub async fn set_viewport(&mut self, width: u32, height: u32) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        client.set_viewport(width, height)?;
        self.config.viewport_width = width;
        self.config.viewport_height = height;
        Ok(())
    }

    // ==================== Lifecycle ====================

    /// Close browser and cleanup
    pub async fn close(&mut self) -> Result<()> {
        if self.initialized {
            let client = self.client();
            let _ = client.close();
            self.initialized = false;
        }
        Ok(())
    }

    /// Shutdown service completely (stop daemon)
    pub async fn shutdown(&mut self) -> Result<()> {
        self.close().await?;
        cleanup_daemon_files(&self.config.session);
        Ok(())
    }

    /// Check if browser is ready
    pub fn is_ready(&self) -> bool {
        self.initialized && is_daemon_running(&self.config.session)
    }

    /// Get current session name
    pub fn session(&self) -> &str {
        &self.config.session
    }

    /// Get current config
    pub fn config(&self) -> &BrowserConfig {
        &self.config
    }

    /// Set headless mode
    /// Note: This will close the current browser and reinitialize with new headless setting
    pub async fn set_headless(&mut self, headless: bool) -> Result<()> {
        info!("Setting headless mode to: {}", headless);
        
        // Only update if different from current setting
        if self.config.headless != headless {
            self.config.headless = headless;
            
            // If already initialized, close and reinitialize
            if self.initialized {
                info!("Closing browser to apply new headless setting");
                let _ = self.close().await;
                self.initialized = false;
                
                // Wait a bit for browser to fully close
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
        
        // Always ensure browser is initialized with correct headless setting
        self.ensure_init().await?;
        info!("Browser initialized with headless={}", headless);
        Ok(())
    }

    // ==================== Network Interception ====================

    /// Start tracking network requests
    pub async fn start_network_tracking(&mut self) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        client.start_request_tracking()?;
        info!("Network request tracking started");
        Ok(())
    }

    /// Get tracked network requests (optionally filtered)
    pub async fn get_network_requests(&mut self, filter: Option<&str>) -> Result<Vec<NetworkRequest>> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.requests(filter, false)?;
        
        let requests = result["requests"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        
        Ok(requests)
    }

    /// Clear tracked network requests
    pub async fn clear_network_requests(&mut self) -> Result<()> {
        self.ensure_init().await?;
        let client = self.client();
        client.clear_requests()?;
        Ok(())
    }

    /// Get current page URL
    pub async fn get_current_url(&mut self) -> Result<String> {
        self.ensure_init().await?;
        let client = self.client();
        let result = client.get_url()?;
        Ok(result["url"].as_str().unwrap_or("").to_string())
    }

    /// Extract domain from URL
    fn extract_domain(url: &str) -> Option<String> {
        let url = url.trim();
        
        // Parse URL to get domain
        if let Some(domain_start) = url.find("://") {
            let after_protocol = &url[domain_start + 3..];
            if let Some(path_start) = after_protocol.find('/') {
                let domain = &after_protocol[..path_start];
                // Remove port if exists
                if let Some(port_pos) = domain.find(':') {
                    return Some(domain[..port_pos].to_string());
                }
                return Some(domain.to_string());
            } else {
                // No path, entire string is domain
                let domain = after_protocol;
                if let Some(port_pos) = domain.find(':') {
                    return Some(domain[..port_pos].to_string());
                }
                return Some(domain.to_string());
            }
        }
        None
    }

    /// Get discovered API endpoints from network requests
    /// Returns all non-static requests from the target domain
    pub async fn get_discovered_apis(&mut self) -> Result<Vec<String>> {
        let requests = self.get_network_requests(None).await?;
        
        // Get current page URL to determine target domain
        let current_url = self.get_current_url().await.unwrap_or_default();
        let target_domain = Self::extract_domain(&current_url);
        
        let apis: Vec<String> = requests
            .iter()
            .filter(|r| {
                let url = &r.url;
                let resource_type = &r.resource_type;
                let url_lower = url.to_lowercase();
                
                // Only keep requests from target domain
                if let Some(ref domain) = target_domain {
                    if let Some(req_domain) = Self::extract_domain(url) {
                        if !req_domain.eq_ignore_ascii_case(domain) && 
                           !req_domain.ends_with(&format!(".{}", domain)) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                
                // Only exclude static resources
                let is_static = url_lower.ends_with(".js") 
                    || url_lower.ends_with(".css") 
                    || url_lower.ends_with(".png") 
                    || url_lower.ends_with(".jpg") 
                    || url_lower.ends_with(".jpeg")
                    || url_lower.ends_with(".gif") 
                    || url_lower.ends_with(".svg") 
                    || url_lower.ends_with(".woff") 
                    || url_lower.ends_with(".woff2")
                    || url_lower.ends_with(".ttf")
                    || url_lower.ends_with(".ico")
                    || url_lower.ends_with(".map")
                    || url_lower.ends_with(".eot")
                    || url_lower.ends_with(".webp")
                    || url_lower.ends_with(".mp3")
                    || url_lower.ends_with(".mp4")
                    || url_lower.ends_with(".webm")
                    || url_lower.ends_with(".ogg")
                    || url_lower.ends_with(".wav")
                    || url_lower.ends_with(".pdf")
                    || resource_type == "image"
                    || resource_type == "stylesheet"
                    || resource_type == "font"
                    || resource_type == "media"
                    || resource_type == "script";
                
                !is_static
            })
            .map(|r| {
                let method = if r.method.is_empty() { "GET" } else { &r.method };
                format!("{} {}", method, r.url)
            })
            .collect();
        
        Ok(apis)
    }
}

impl Default for AgentBrowserService {
    fn default() -> Self {
        Self::new()
    }
}

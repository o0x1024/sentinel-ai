//! Browser driver implementation using chromiumoxide

use anyhow::{anyhow, Result};
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::network::EventRequestWillBeSent;
use chromiumoxide::cdp::browser_protocol::network::EventResponseReceived;
use chromiumoxide::cdp::browser_protocol::page::NavigateParams;
use chromiumoxide::page::Page;
use futures::StreamExt;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::network::NetworkListener;
use super::types::{
    ApiRequest, ElementBounds, InteractiveElement, PageState, TestExplorerV1Config,
};

/// Browser driver for Test Explorer V1
pub struct BrowserDriver {
    browser: Arc<Browser>,
    page: Arc<Page>,
    network_listener: Arc<RwLock<Option<NetworkListener>>>,
    config: TestExplorerV1Config,
}

impl BrowserDriver {
    /// Create a new browser driver
    pub async fn new(config: TestExplorerV1Config) -> Result<Self> {
        info!("Initializing BrowserDriver with config: {:?}", config);

        // Configure browser
        let mut browser_config = BrowserConfig::builder()
            .window_size(config.viewport_width, config.viewport_height);

        if config.headless {
            browser_config = browser_config.with_head();
        }

        if let Some(ref user_agent) = config.user_agent {
            browser_config = browser_config.arg(format!("--user-agent={}", user_agent));
        }

        // Launch browser
        let (browser, mut handler) = Browser::launch(browser_config.build().map_err(|e| {
            anyhow!("Failed to build browser config: {}", e)
        })?)
        .await
        .map_err(|e| anyhow!("Failed to launch browser: {}", e))?;

        // Spawn handler task
        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(e) = event {
                    error!("Browser handler error: {}", e);
                }
            }
        });

        // Create new page
        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|e| anyhow!("Failed to create new page: {}", e))?;

        info!("Browser initialized successfully");

        Ok(Self {
            browser: Arc::new(browser),
            page: Arc::new(page),
            network_listener: Arc::new(RwLock::new(None)),
            config,
        })
    }

    /// Navigate to a URL and return page state
    pub async fn navigate(&self, url: &str) -> Result<PageState> {
        info!("Navigating to: {}", url);

        // Navigate
        self.page
            .goto(url)
            .await
            .map_err(|e| anyhow!("Navigation failed: {}", e))?;

        // Wait for page load
        tokio::time::sleep(Duration::from_millis(2000)).await;

        // Get page state
        self.get_page_state().await
    }

    /// Get current page state
    pub async fn get_page_state(&self) -> Result<PageState> {
        debug!("Getting page state");

        // Get URL
        let url = self
            .page
            .url()
            .await
            .map_err(|e| anyhow!("Failed to get URL: {}", e))?
            .unwrap_or_default();

        // Get title
        let title = self
            .page
            .evaluate("document.title")
            .await
            .map_err(|e| anyhow!("Failed to get title: {}", e))?
            .into_value::<String>()
            .unwrap_or_default();

        // Get visible text
        let visible_text = self.extract_visible_text().await?;

        // Get simplified HTML
        let simplified_html = self.extract_simplified_html().await?;

        // Get interactive elements
        let interactive_elements = self.annotate_elements().await?;

        // Get captured APIs
        let captured_apis = if let Some(listener) = self.network_listener.read().await.as_ref() {
            listener.get_requests().await
        } else {
            Vec::new()
        };

        Ok(PageState {
            url,
            title,
            visible_text,
            simplified_html,
            interactive_elements,
            captured_apis,
            timestamp: SystemTime::now(),
        })
    }

    /// Extract visible text from the page
    async fn extract_visible_text(&self) -> Result<String> {
        let script = r#"
            (() => {
                const walker = document.createTreeWalker(
                    document.body,
                    NodeFilter.SHOW_TEXT,
                    {
                        acceptNode: (node) => {
                            const parent = node.parentElement;
                            if (!parent) return NodeFilter.FILTER_REJECT;
                            
                            const style = window.getComputedStyle(parent);
                            if (style.display === 'none' || style.visibility === 'hidden') {
                                return NodeFilter.FILTER_REJECT;
                            }
                            
                            const tagName = parent.tagName.toLowerCase();
                            if (['script', 'style', 'noscript'].includes(tagName)) {
                                return NodeFilter.FILTER_REJECT;
                            }
                            
                            return NodeFilter.FILTER_ACCEPT;
                        }
                    }
                );
                
                let text = '';
                let node;
                while (node = walker.nextNode()) {
                    const content = node.textContent.trim();
                    if (content) {
                        text += content + ' ';
                    }
                }
                
                return text.trim();
            })()
        "#;

        let result = self
            .page
            .evaluate(script)
            .await
            .map_err(|e| anyhow!("Failed to extract visible text: {}", e))?;

        Ok(result.into_value::<String>().unwrap_or_default())
    }

    /// Extract simplified HTML (without scripts, styles, comments)
    async fn extract_simplified_html(&self) -> Result<String> {
        let script = r#"
            (() => {
                const clone = document.documentElement.cloneNode(true);
                
                // Remove scripts, styles, noscript
                ['script', 'style', 'noscript', 'iframe'].forEach(tag => {
                    clone.querySelectorAll(tag).forEach(el => el.remove());
                });
                
                // Remove comments
                const removeComments = (node) => {
                    for (let i = node.childNodes.length - 1; i >= 0; i--) {
                        const child = node.childNodes[i];
                        if (child.nodeType === Node.COMMENT_NODE) {
                            node.removeChild(child);
                        } else if (child.nodeType === Node.ELEMENT_NODE) {
                            removeComments(child);
                        }
                    }
                };
                removeComments(clone);
                
                // Get HTML
                let html = clone.outerHTML;
                
                // Limit size
                if (html.length > 50000) {
                    html = html.substring(0, 50000) + '... [truncated]';
                }
                
                return html;
            })()
        "#;

        let result = self
            .page
            .evaluate(script)
            .await
            .map_err(|e| anyhow!("Failed to extract simplified HTML: {}", e))?;

        Ok(result.into_value::<String>().unwrap_or_default())
    }

    /// Annotate interactive elements on the page
    pub async fn annotate_elements(&self) -> Result<Vec<InteractiveElement>> {
        let script = r#"
            (() => {
                const elements = [];
                const selectors = [
                    'a[href]',
                    'button',
                    'input',
                    'textarea',
                    'select',
                    '[onclick]',
                    '[role="button"]',
                    '[role="link"]'
                ];
                
                const found = new Set();
                selectors.forEach(selector => {
                    document.querySelectorAll(selector).forEach(el => {
                        if (found.has(el)) return;
                        found.add(el);
                        
                        const rect = el.getBoundingClientRect();
                        const style = window.getComputedStyle(el);
                        
                        // Skip hidden elements
                        if (style.display === 'none' || style.visibility === 'hidden' || 
                            rect.width === 0 || rect.height === 0) {
                            return;
                        }
                        
                        // Generate selector
                        let cssSelector = el.tagName.toLowerCase();
                        if (el.id) {
                            cssSelector = '#' + el.id;
                        } else if (el.className) {
                            const classes = el.className.split(' ').filter(c => c).slice(0, 2);
                            if (classes.length > 0) {
                                cssSelector += '.' + classes.join('.');
                            }
                        }
                        
                        // Get text
                        let text = el.textContent?.trim() || el.value || el.placeholder || '';
                        if (text.length > 100) {
                            text = text.substring(0, 100) + '...';
                        }
                        
                        // Get attributes
                        const attrs = {};
                        ['id', 'name', 'class', 'type', 'href', 'value', 'placeholder', 'aria-label'].forEach(attr => {
                            if (el.hasAttribute(attr)) {
                                attrs[attr] = el.getAttribute(attr);
                            }
                        });
                        
                        elements.push({
                            element_type: el.tagName.toLowerCase(),
                            selector: cssSelector,
                            text: text,
                            attributes: attrs,
                            bounds: {
                                x: rect.x,
                                y: rect.y,
                                width: rect.width,
                                height: rect.height
                            }
                        });
                    });
                });
                
                return elements;
            })()
        "#;

        let result = self
            .page
            .evaluate(script)
            .await
            .map_err(|e| anyhow!("Failed to annotate elements: {}", e))?;

        let elements_json = result
            .into_value::<serde_json::Value>()
            .unwrap_or(serde_json::json!([]));

        let mut elements: Vec<InteractiveElement> =
            serde_json::from_value(elements_json).unwrap_or_default();

        // Add indices
        for (i, elem) in elements.iter_mut().enumerate() {
            elem.index = i;
        }

        debug!("Found {} interactive elements", elements.len());
        Ok(elements)
    }

    /// Click an element by selector
    pub async fn click(&self, selector: &str) -> Result<()> {
        info!("Clicking element: {}", selector);

        self.page
            .find_element(selector)
            .await
            .map_err(|e| anyhow!("Element not found: {}", e))?
            .click()
            .await
            .map_err(|e| anyhow!("Click failed: {}", e))?;

        // Wait for potential navigation/changes
        tokio::time::sleep(Duration::from_millis(1000)).await;

        Ok(())
    }

    /// Click an element by index
    pub async fn click_by_index(&self, index: usize) -> Result<()> {
        let elements = self.annotate_elements().await?;
        let element = elements
            .get(index)
            .ok_or_else(|| anyhow!("Element index {} not found", index))?;

        self.click(&element.selector).await
    }

    /// Fill an input field
    pub async fn fill(&self, selector: &str, value: &str) -> Result<()> {
        info!("Filling element {} with value: {}", selector, value);

        let element = self
            .page
            .find_element(selector)
            .await
            .map_err(|e| anyhow!("Element not found: {}", e))?;

        element
            .click()
            .await
            .map_err(|e| anyhow!("Failed to focus element: {}", e))?;

        element
            .type_str(value)
            .await
            .map_err(|e| anyhow!("Failed to type value: {}", e))?;

        Ok(())
    }

    /// Go back in browser history
    pub async fn go_back(&self) -> Result<()> {
        info!("Going back in history");

        self.page
            .evaluate("window.history.back()")
            .await
            .map_err(|e| anyhow!("Failed to go back: {}", e))?;

        tokio::time::sleep(Duration::from_millis(1000)).await;

        Ok(())
    }

    /// Start network capture
    pub async fn start_network_capture(&self) -> Result<()> {
        info!("Starting network capture");

        let listener = NetworkListener::new(self.page.clone()).await?;
        *self.network_listener.write().await = Some(listener);

        Ok(())
    }

    /// Get captured API requests
    pub async fn get_captured_requests(&self) -> Vec<ApiRequest> {
        if let Some(listener) = self.network_listener.read().await.as_ref() {
            listener.get_requests().await
        } else {
            Vec::new()
        }
    }

    /// Wait for a specific API request matching pattern
    pub async fn wait_for_request(
        &self,
        pattern: &str,
        timeout: Duration,
    ) -> Result<ApiRequest> {
        let listener = self
            .network_listener
            .read()
            .await
            .as_ref()
            .ok_or_else(|| anyhow!("Network capture not started"))?
            .clone();

        listener.wait_for_request(pattern, timeout).await
    }

    /// Execute JavaScript and return result
    pub async fn evaluate(&self, script: &str) -> Result<serde_json::Value> {
        let result = self
            .page
            .evaluate(script)
            .await
            .map_err(|e| anyhow!("Script evaluation failed: {}", e))?;

        Ok(result
            .into_value::<serde_json::Value>()
            .unwrap_or(serde_json::json!(null)))
    }

    /// Close the browser
    pub async fn close(&self) -> Result<()> {
        info!("Closing browser");
        // Browser will be closed when dropped
        Ok(())
    }
}

impl Drop for BrowserDriver {
    fn drop(&mut self) {
        debug!("BrowserDriver dropped");
    }
}


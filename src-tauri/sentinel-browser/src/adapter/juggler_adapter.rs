use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::adapter::traits::*;
use crate::juggler::commands::methods;
use crate::juggler::pipe::JugglerPipe;
use crate::juggler::session::JugglerSession;
use crate::page::accessibility::AccessibilityTree;

/// Juggler-based backend for Firefox/Camoufox.
/// Communicates over pipe protocol, providing superior stealth properties
/// since automation commands are isolated from the page JS context.
pub struct JugglerBackend {
    session: Option<Mutex<JugglerSession>>,
}

impl JugglerBackend {
    pub fn new() -> Self {
        Self { session: None }
    }

    /// Initialize from a Camoufox child process's stdio pipes
    pub fn from_process(
        stdout: tokio::process::ChildStdout,
        stdin: tokio::process::ChildStdin,
    ) -> Self {
        let (event_tx, _event_rx) = tokio::sync::mpsc::unbounded_channel();
        let pipe = JugglerPipe::new(stdout, stdin, event_tx);
        let session = JugglerSession::new(pipe);
        Self {
            session: Some(Mutex::new(session)),
        }
    }

    fn get_session_ref(&self) -> Result<&Mutex<JugglerSession>> {
        self.session.as_ref().ok_or(BrowserError::NotConnected)
    }
}

#[async_trait]
impl BrowserBackend for JugglerBackend {
    async fn connect(&mut self, _endpoint: &str) -> Result<()> {
        if self.session.is_some() {
            Ok(())
        } else {
            Err(BrowserError::ConnectionFailed(
                "JugglerBackend requires pipe connection via from_process()".to_string(),
            ))
        }
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(ref session_mutex) = self.session {
            let session = session_mutex.lock().await;
            session.disconnect();
        }
        self.session = None;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.session.is_some()
    }

    async fn navigate(&self, tab: &TabId, url: &str) -> Result<NavigationResult> {
        let start = std::time::Instant::now();
        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;

        session
            .send_to_tab(&tab.0, methods::PAGE_NAVIGATE, Some(json!({ "url": url })))
            .await?;

        Ok(NavigationResult {
            url: url.to_string(),
            status: 200,
            load_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn reload(&self, tab: &TabId) -> Result<()> {
        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;
        session.send_to_tab(&tab.0, methods::PAGE_RELOAD, None).await?;
        Ok(())
    }

    async fn go_back(&self, tab: &TabId) -> Result<()> {
        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;
        session.send_to_tab(&tab.0, methods::PAGE_GO_BACK, None).await?;
        Ok(())
    }

    async fn go_forward(&self, tab: &TabId) -> Result<()> {
        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;
        session.send_to_tab(&tab.0, methods::PAGE_GO_FORWARD, None).await?;
        Ok(())
    }

    async fn current_url(&self, tab: &TabId) -> Result<String> {
        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;
        let result = session
            .send_to_tab(
                &tab.0,
                methods::PAGE_EVALUATE,
                Some(json!({ "expression": "window.location.href" })),
            )
            .await?;

        Ok(result
            .get("result")
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }

    async fn dispatch_mouse_event(&self, tab: &TabId, event: MouseEvent) -> Result<()> {
        let event_type = match event.kind {
            MouseEventKind::Moved => "mousemove",
            MouseEventKind::Pressed => "mousedown",
            MouseEventKind::Released => "mouseup",
        };

        let button_num = match event.button {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
        };

        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;
        session
            .send_to_tab(
                &tab.0,
                methods::INPUT_DISPATCH_MOUSE_EVENT,
                Some(json!({
                    "type": event_type,
                    "x": event.x,
                    "y": event.y,
                    "button": button_num,
                })),
            )
            .await?;
        Ok(())
    }

    async fn dispatch_key_event(&self, tab: &TabId, event: KeyEvent) -> Result<()> {
        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;

        match event.kind {
            KeyEventKind::Char => {
                session
                    .send_to_tab(
                        &tab.0,
                        methods::INPUT_INSERT_TEXT,
                        Some(json!({ "text": event.key })),
                    )
                    .await?;
            }
            KeyEventKind::Down | KeyEventKind::Up => {
                let event_type = if event.kind == KeyEventKind::Down {
                    "keydown"
                } else {
                    "keyup"
                };
                session
                    .send_to_tab(
                        &tab.0,
                        methods::INPUT_DISPATCH_KEY_EVENT,
                        Some(json!({
                            "type": event_type,
                            "key": event.key,
                            "code": event.code,
                            "ctrlKey": event.modifiers.ctrl,
                            "altKey": event.modifiers.alt,
                            "shiftKey": event.modifiers.shift,
                            "metaKey": event.modifiers.meta,
                        })),
                    )
                    .await?;
            }
        }
        Ok(())
    }

    async fn dispatch_scroll_event(&self, tab: &TabId, event: ScrollEvent) -> Result<()> {
        let js = format!("window.scrollBy({}, {})", event.delta_x, event.delta_y);
        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;
        session
            .send_to_tab(
                &tab.0,
                methods::PAGE_EVALUATE,
                Some(json!({ "expression": js })),
            )
            .await?;
        Ok(())
    }

    async fn get_accessibility_tree(&self, tab: &TabId) -> Result<AccessibilityTree> {
        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;
        let result = session
            .send_to_tab(&tab.0, methods::ACCESSIBILITY_GET_TREE, None)
            .await?;

        let nodes = result
            .get("tree")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut tree = AccessibilityTree::new();
        for (idx, node) in nodes.iter().enumerate() {
            let role = node.get("role").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            tree.add_node(
                format!("e{}", idx),
                role,
                name,
                node.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()),
            );
        }

        Ok(tree)
    }

    async fn capture_screenshot(&self, tab: &TabId, opts: ScreenshotOpts) -> Result<Vec<u8>> {
        let format_str = match opts.format {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpeg",
            ImageFormat::Webp => "png",
        };

        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;
        let result = session
            .send_to_tab(
                &tab.0,
                methods::PAGE_SCREENSHOT,
                Some(json!({
                    "mimeType": format!("image/{}", format_str),
                    "fullPage": opts.full_page,
                })),
            )
            .await?;

        let b64_data = result.get("data").and_then(|v| v.as_str()).unwrap_or("");

        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(b64_data)
            .map_err(|e| BrowserError::ProtocolError(format!("Failed to decode screenshot: {}", e)))
    }

    async fn evaluate_js(&self, tab: &TabId, expression: &str) -> Result<Value> {
        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;
        let result = session
            .send_to_tab(
                &tab.0,
                methods::PAGE_EVALUATE,
                Some(json!({ "expression": expression })),
            )
            .await?;

        if let Some(exception) = result.get("exceptionDetails") {
            let msg = exception
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown JS error");
            return Err(BrowserError::JsError(msg.to_string()));
        }

        Ok(result
            .get("result")
            .and_then(|v| v.get("value"))
            .cloned()
            .unwrap_or(Value::Null))
    }

    async fn create_tab(&self, url: Option<&str>) -> Result<TabId> {
        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;
        session.new_page(url).await
    }

    async fn close_tab(&self, tab: &TabId) -> Result<()> {
        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;
        session.close_page(&tab.0).await
    }

    async fn list_tabs(&self) -> Result<Vec<TabInfo>> {
        let session_mutex = self.get_session_ref()?;
        let session = session_mutex.lock().await;
        session.list_targets().await
    }

    async fn activate_tab(&self, tab: &TabId) -> Result<()> {
        let session_mutex = self.get_session_ref()?;
        let session = session_mutex.lock().await;
        session
            .pipe()
            .send_command(
                methods::TARGET_ACTIVATE,
                Some(json!({ "targetId": tab.0 })),
                None,
            )
            .await?;
        Ok(())
    }

    async fn intercept_requests(&self, tab: &TabId, patterns: &[String]) -> Result<()> {
        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;
        session
            .send_to_tab(
                &tab.0,
                methods::NETWORK_SET_REQUEST_INTERCEPTION,
                Some(json!({ "enabled": true, "patterns": patterns })),
            )
            .await?;
        Ok(())
    }

    async fn continue_request(
        &self,
        request_id: &str,
        modifications: Option<RequestMod>,
    ) -> Result<()> {
        let mut params = json!({ "requestId": request_id });
        if let Some(mods) = modifications {
            if let Some(url) = mods.url {
                params["url"] = json!(url);
            }
            if let Some(method) = mods.method {
                params["method"] = json!(method);
            }
            if let Some(headers) = mods.headers {
                let h: Value = headers
                    .into_iter()
                    .map(|(k, v)| json!({ "name": k, "value": v }))
                    .collect();
                params["headers"] = h;
            }
        }

        let session_mutex = self.get_session_ref()?;
        let session = session_mutex.lock().await;
        session
            .pipe()
            .send_command(methods::NETWORK_CONTINUE_REQUEST, Some(params), None)
            .await?;
        Ok(())
    }

    async fn get_cookies(&self, tab: &TabId) -> Result<Vec<Cookie>> {
        let session_mutex = self.get_session_ref()?;
        let mut session = session_mutex.lock().await;
        let result = session
            .send_to_tab(&tab.0, methods::NETWORK_GET_COOKIES, None)
            .await?;

        let cookies = result
            .get("cookies")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(cookies
            .iter()
            .filter_map(|c| serde_json::from_value(c.clone()).ok())
            .collect())
    }

    async fn set_cookies(&self, cookies: &[Cookie]) -> Result<()> {
        let cookie_values: Vec<Value> = cookies
            .iter()
            .map(|c| serde_json::to_value(c).unwrap_or(Value::Null))
            .collect();

        let session_mutex = self.get_session_ref()?;
        let session = session_mutex.lock().await;
        session
            .pipe()
            .send_command(
                methods::NETWORK_SET_COOKIES,
                Some(json!({ "cookies": cookie_values })),
                None,
            )
            .await?;
        Ok(())
    }
}

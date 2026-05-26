use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};

use super::traits::*;
use crate::cdp::commands::Cdp;
use crate::cdp::events::{methods, wait_for_event};
use crate::cdp::session::CdpSessionManager;
use crate::page::accessibility::{A11yNode, AccessibilityTree};

/// CDP implementation of BrowserBackend
pub struct CdpBackend {
    session_manager: Option<CdpSessionManager>,
}

impl CdpBackend {
    pub fn new() -> Self {
        Self {
            session_manager: None,
        }
    }

    fn manager(&self) -> Result<&CdpSessionManager> {
        self.session_manager
            .as_ref()
            .ok_or(BrowserError::NotConnected)
    }
}

#[async_trait]
impl BrowserBackend for CdpBackend {
    async fn connect(&mut self, endpoint: &str) -> Result<()> {
        let manager = CdpSessionManager::new(endpoint)
            .await
            .map_err(|e| BrowserError::ConnectionFailed(e))?;
        self.session_manager = Some(manager);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.session_manager = None;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.session_manager.is_some()
    }

    async fn navigate(&self, tab: &TabId, url: &str) -> Result<NavigationResult> {
        let mgr = self.manager()?;
        let start = std::time::Instant::now();

        let (method, params) = Cdp::page_navigate(url);
        let _result = mgr
            .send_to_target(&tab.0, method, Some(params))
            .await
            .map_err(|e| BrowserError::NavigationFailed(e))?;

        // Wait for page load
        let mut event_rx = mgr.subscribe_events();
        let session_id = mgr
            .get_session_id(&tab.0)
            .await
            .map_err(|e| BrowserError::NavigationFailed(e))?;

        let _ = wait_for_event(
            &mut event_rx,
            methods::PAGE_LOAD_EVENT_FIRED,
            Some(&session_id),
            Duration::from_secs(30),
        )
        .await;

        let load_time = start.elapsed().as_millis() as u64;

        Ok(NavigationResult {
            url: url.to_string(),
            status: 200, // CDP doesn't directly give us status in navigate
            load_time_ms: load_time,
        })
    }

    async fn reload(&self, tab: &TabId) -> Result<()> {
        let mgr = self.manager()?;
        let (method, params) = Cdp::page_reload(false);
        mgr.send_to_target(&tab.0, method, Some(params))
            .await
            .map_err(|e| BrowserError::NavigationFailed(e))?;
        Ok(())
    }

    async fn go_back(&self, tab: &TabId) -> Result<()> {
        self.evaluate_js(tab, "history.back()").await?;
        Ok(())
    }

    async fn go_forward(&self, tab: &TabId) -> Result<()> {
        self.evaluate_js(tab, "history.forward()").await?;
        Ok(())
    }

    async fn current_url(&self, tab: &TabId) -> Result<String> {
        let result = self.evaluate_js(tab, "window.location.href").await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    async fn dispatch_mouse_event(&self, tab: &TabId, event: MouseEvent) -> Result<()> {
        let mgr = self.manager()?;

        let event_type = match event.kind {
            MouseEventKind::Moved => "mouseMoved",
            MouseEventKind::Pressed => "mousePressed",
            MouseEventKind::Released => "mouseReleased",
        };

        let button = match event.button {
            MouseButton::Left => "left",
            MouseButton::Right => "right",
            MouseButton::Middle => "middle",
        };

        let click_count = match event.kind {
            MouseEventKind::Pressed | MouseEventKind::Released => 1,
            _ => 0,
        };

        let (method, params) =
            Cdp::input_dispatch_mouse_event(event_type, event.x, event.y, button, click_count);
        mgr.send_to_target(&tab.0, method, Some(params))
            .await
            .map_err(|e| BrowserError::ProtocolError(e))?;

        Ok(())
    }

    async fn dispatch_key_event(&self, tab: &TabId, event: KeyEvent) -> Result<()> {
        let mgr = self.manager()?;

        let event_type = match event.kind {
            KeyEventKind::Down => "keyDown",
            KeyEventKind::Up => "keyUp",
            KeyEventKind::Char => "char",
        };

        let mut modifiers_flags = 0u32;
        if event.modifiers.alt {
            modifiers_flags |= crate::cdp::commands::modifiers::ALT;
        }
        if event.modifiers.ctrl {
            modifiers_flags |= crate::cdp::commands::modifiers::CTRL;
        }
        if event.modifiers.meta {
            modifiers_flags |= crate::cdp::commands::modifiers::META;
        }
        if event.modifiers.shift {
            modifiers_flags |= crate::cdp::commands::modifiers::SHIFT;
        }

        let text = if event.kind == KeyEventKind::Char || event.kind == KeyEventKind::Down {
            if event.key.len() == 1 {
                Some(event.key.as_str())
            } else {
                None
            }
        } else {
            None
        };

        let (method, params) =
            Cdp::input_dispatch_key_event(event_type, &event.key, &event.code, text, modifiers_flags);
        mgr.send_to_target(&tab.0, method, Some(params))
            .await
            .map_err(|e| BrowserError::ProtocolError(e))?;

        Ok(())
    }

    async fn dispatch_scroll_event(&self, tab: &TabId, event: ScrollEvent) -> Result<()> {
        let mgr = self.manager()?;

        let (method, params) =
            Cdp::input_dispatch_mouse_wheel(event.x, event.y, event.delta_x, event.delta_y);
        mgr.send_to_target(&tab.0, method, Some(params))
            .await
            .map_err(|e| BrowserError::ProtocolError(e))?;

        Ok(())
    }

    async fn get_accessibility_tree(&self, tab: &TabId) -> Result<AccessibilityTree> {
        let mgr = self.manager()?;

        let (method, params) = Cdp::accessibility_get_full_tree();
        let result = mgr
            .send_to_target(&tab.0, method, Some(params))
            .await
            .map_err(|e| BrowserError::ProtocolError(e))?;

        let url = self.current_url(tab).await.unwrap_or_default();
        let title_val = self.evaluate_js(tab, "document.title").await.unwrap_or(Value::String(String::new()));
        let title = title_val.as_str().unwrap_or("").to_string();

        let root = parse_cdp_a11y_tree(&result);

        Ok(AccessibilityTree {
            url,
            title,
            root,
            snapshot_id: 0,
        })
    }

    async fn capture_screenshot(&self, tab: &TabId, opts: ScreenshotOpts) -> Result<Vec<u8>> {
        let mgr = self.manager()?;

        let format = match opts.format {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpeg",
            ImageFormat::Webp => "webp",
        };

        let clip = opts.clip.map(|c| {
            json!({
                "x": c.x,
                "y": c.y,
                "width": c.width,
                "height": c.height,
                "scale": 1.0
            })
        });

        let (method, params) = Cdp::page_capture_screenshot(format, opts.quality, clip);
        let result = mgr
            .send_to_target(&tab.0, method, Some(params))
            .await
            .map_err(|e| BrowserError::ProtocolError(e))?;

        let data_b64 = result
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or(BrowserError::ProtocolError("No screenshot data".to_string()))?;

        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(data_b64)
            .map_err(|e| BrowserError::ProtocolError(format!("Base64 decode error: {}", e)))
    }

    async fn evaluate_js(&self, tab: &TabId, expression: &str) -> Result<Value> {
        let mgr = self.manager()?;

        let (method, params) = Cdp::runtime_evaluate(expression, true);
        let result = mgr
            .send_to_target(&tab.0, method, Some(params))
            .await
            .map_err(|e| BrowserError::JsError(e))?;

        if let Some(exception) = result.get("exceptionDetails") {
            let msg = exception
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown JS error");
            return Err(BrowserError::JsError(msg.to_string()));
        }

        let value = result
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(Value::Null);

        Ok(value)
    }

    async fn create_tab(&self, url: Option<&str>) -> Result<TabId> {
        let mgr = self.manager()?;
        let target_url = url.unwrap_or("about:blank");
        let info = mgr
            .create_tab(target_url)
            .await
            .map_err(|e| BrowserError::ProtocolError(e))?;
        Ok(TabId(info.target_id))
    }

    async fn close_tab(&self, tab: &TabId) -> Result<()> {
        let mgr = self.manager()?;
        mgr.close_tab(&tab.0)
            .await
            .map_err(|e| BrowserError::ProtocolError(e))?;
        Ok(())
    }

    async fn list_tabs(&self) -> Result<Vec<TabInfo>> {
        let mgr = self.manager()?;
        mgr.refresh_targets()
            .await
            .map_err(|e| BrowserError::ProtocolError(e))?;

        let targets = mgr.list_tabs().await;
        Ok(targets
            .into_iter()
            .map(|t| TabInfo {
                id: TabId(t.target_id),
                url: t.url,
                title: t.title,
                is_active: false,
            })
            .collect())
    }

    async fn activate_tab(&self, tab: &TabId) -> Result<()> {
        let mgr = self.manager()?;
        mgr.activate_tab(&tab.0)
            .await
            .map_err(|e| BrowserError::ProtocolError(e))?;
        Ok(())
    }

    async fn intercept_requests(&self, tab: &TabId, patterns: &[String]) -> Result<()> {
        let mgr = self.manager()?;
        let pattern_values: Vec<Value> = patterns
            .iter()
            .map(|p| json!({ "urlPattern": p }))
            .collect();
        let (method, params) = Cdp::network_set_request_interception(&pattern_values);
        mgr.send_to_target(&tab.0, method, Some(params))
            .await
            .map_err(|e| BrowserError::ProtocolError(e))?;
        Ok(())
    }

    async fn continue_request(
        &self,
        request_id: &str,
        modifications: Option<RequestMod>,
    ) -> Result<()> {
        let mgr = self.manager()?;
        let mods = modifications.map(|m| {
            let mut obj = serde_json::Map::new();
            if let Some(url) = m.url {
                obj.insert("url".to_string(), json!(url));
            }
            if let Some(method) = m.method {
                obj.insert("method".to_string(), json!(method));
            }
            if let Some(headers) = m.headers {
                let header_entries: Vec<Value> = headers
                    .iter()
                    .map(|(k, v)| json!({ "name": k, "value": v }))
                    .collect();
                obj.insert("headers".to_string(), json!(header_entries));
            }
            Value::Object(obj)
        });

        let (method, params) = Cdp::fetch_continue_request(request_id, mods);
        // Send without specific target — fetch events carry their own session context
        mgr.send_to_target("", method, Some(params))
            .await
            .map_err(|e| BrowserError::ProtocolError(e))?;
        Ok(())
    }

    async fn get_cookies(&self, tab: &TabId) -> Result<Vec<Cookie>> {
        let mgr = self.manager()?;
        let (method, params) = Cdp::network_get_cookies(None);
        let result = mgr
            .send_to_target(&tab.0, method, Some(params))
            .await
            .map_err(|e| BrowserError::ProtocolError(e))?;

        let cookies: Vec<Cookie> = result
            .get("cookies")
            .and_then(|c| serde_json::from_value(c.clone()).ok())
            .unwrap_or_default();

        Ok(cookies)
    }

    async fn set_cookies(&self, cookies: &[Cookie]) -> Result<()> {
        let mgr = self.manager()?;
        let cookie_values: Vec<Value> = cookies
            .iter()
            .map(|c| {
                json!({
                    "name": c.name,
                    "value": c.value,
                    "domain": c.domain,
                    "path": c.path,
                    "httpOnly": c.http_only,
                    "secure": c.secure,
                })
            })
            .collect();
        let (method, params) = Cdp::network_set_cookies(&cookie_values);
        // Use browser-level command (no session needed)
        mgr.send_to_target("", method, Some(params))
            .await
            .map_err(|e| BrowserError::ProtocolError(e))?;
        Ok(())
    }
}

/// Parse CDP Accessibility.getFullAXTree response into our A11yNode structure
fn parse_cdp_a11y_tree(result: &Value) -> A11yNode {
    let nodes = result
        .get("nodes")
        .and_then(|n| n.as_array())
        .cloned()
        .unwrap_or_default();

    if nodes.is_empty() {
        return A11yNode {
            ref_id: "e0".to_string(),
            role: "document".to_string(),
            name: String::new(),
            value: None,
            state: vec![],
            bounds: None,
            level: None,
            children: vec![],
            is_new: false,
        };
    }

    // Build a map of node_id -> node
    let mut node_map: HashMap<String, &Value> = HashMap::new();
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();

    for node in &nodes {
        let node_id = node
            .get("nodeId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        node_map.insert(node_id.clone(), node);

        if let Some(child_ids) = node.get("childIds").and_then(|v| v.as_array()) {
            let ids: Vec<String> = child_ids
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            children_map.insert(node_id, ids);
        }
    }

    // Find root (first node or node with role "RootWebArea")
    let root_id = nodes
        .first()
        .and_then(|n| n.get("nodeId"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut ref_counter = 0u64;
    build_a11y_node(&root_id, &node_map, &children_map, &mut ref_counter)
}

fn build_a11y_node(
    node_id: &str,
    node_map: &HashMap<String, &Value>,
    children_map: &HashMap<String, Vec<String>>,
    ref_counter: &mut u64,
) -> A11yNode {
    let empty_val = &Value::Null;
    let node = node_map.get(node_id).copied().unwrap_or(empty_val);

    let role = node
        .get("role")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("none")
        .to_string();

    let name = node
        .get("name")
        .and_then(|n| n.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let value = node
        .get("value")
        .and_then(|n| n.get("value"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    *ref_counter += 1;
    let ref_id = format!("e{}", ref_counter);

    let children = children_map
        .get(node_id)
        .map(|ids| {
            ids.iter()
                .map(|child_id| build_a11y_node(child_id, node_map, children_map, ref_counter))
                .collect()
        })
        .unwrap_or_default();

    A11yNode {
        ref_id,
        role,
        name,
        value,
        state: vec![],
        bounds: None,
        level: None,
        children,
        is_new: false,
    }
}

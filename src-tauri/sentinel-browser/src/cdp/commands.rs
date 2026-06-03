use serde_json::{json, Value};

/// Helper to build CDP command params concisely
pub struct Cdp;

impl Cdp {
    // --- Page domain ---

    pub fn page_navigate(url: &str) -> (&'static str, Value) {
        ("Page.navigate", json!({ "url": url }))
    }

    pub fn page_reload(ignore_cache: bool) -> (&'static str, Value) {
        ("Page.reload", json!({ "ignoreCache": ignore_cache }))
    }

    pub fn page_enable() -> (&'static str, Value) {
        ("Page.enable", json!({}))
    }

    pub fn page_get_frame_tree() -> (&'static str, Value) {
        ("Page.getFrameTree", json!({}))
    }

    // --- DOM domain ---

    pub fn dom_enable() -> (&'static str, Value) {
        ("DOM.enable", json!({}))
    }

    pub fn dom_get_document() -> (&'static str, Value) {
        ("DOM.getDocument", json!({ "depth": -1, "pierce": true }))
    }

    pub fn dom_query_selector(node_id: u64, selector: &str) -> (&'static str, Value) {
        (
            "DOM.querySelector",
            json!({ "nodeId": node_id, "selector": selector }),
        )
    }

    pub fn dom_get_box_model(node_id: u64) -> (&'static str, Value) {
        ("DOM.getBoxModel", json!({ "nodeId": node_id }))
    }

    // --- Input domain ---

    pub fn input_dispatch_mouse_event(
        event_type: &str,
        x: f64,
        y: f64,
        button: &str,
        click_count: u32,
    ) -> (&'static str, Value) {
        (
            "Input.dispatchMouseEvent",
            json!({
                "type": event_type,
                "x": x,
                "y": y,
                "button": button,
                "clickCount": click_count
            }),
        )
    }

    pub fn input_dispatch_key_event(
        event_type: &str,
        key: &str,
        code: &str,
        text: Option<&str>,
        modifiers: u32,
    ) -> (&'static str, Value) {
        let mut params = json!({
            "type": event_type,
            "key": key,
            "code": code,
            "modifiers": modifiers
        });
        if let Some(t) = text {
            params["text"] = json!(t);
            params["unmodifiedText"] = json!(t);
        }
        ("Input.dispatchKeyEvent", params)
    }

    pub fn input_dispatch_mouse_wheel(x: f64, y: f64, delta_x: f64, delta_y: f64) -> (&'static str, Value) {
        (
            "Input.dispatchMouseEvent",
            json!({
                "type": "mouseWheel",
                "x": x,
                "y": y,
                "deltaX": delta_x,
                "deltaY": delta_y
            }),
        )
    }

    // --- Runtime domain ---

    pub fn runtime_enable() -> (&'static str, Value) {
        ("Runtime.enable", json!({}))
    }

    pub fn runtime_evaluate(expression: &str, await_promise: bool) -> (&'static str, Value) {
        (
            "Runtime.evaluate",
            json!({
                "expression": expression,
                "returnByValue": true,
                "awaitPromise": await_promise
            }),
        )
    }

    // --- Target domain ---

    pub fn target_get_targets() -> (&'static str, Value) {
        ("Target.getTargets", json!({}))
    }

    pub fn target_create_target(url: &str) -> (&'static str, Value) {
        ("Target.createTarget", json!({ "url": url }))
    }

    pub fn target_close_target(target_id: &str) -> (&'static str, Value) {
        ("Target.closeTarget", json!({ "targetId": target_id }))
    }

    pub fn target_activate_target(target_id: &str) -> (&'static str, Value) {
        ("Target.activateTarget", json!({ "targetId": target_id }))
    }

    pub fn target_attach_to_target(target_id: &str, flatten: bool) -> (&'static str, Value) {
        (
            "Target.attachToTarget",
            json!({ "targetId": target_id, "flatten": flatten }),
        )
    }

    pub fn target_set_discover_targets(discover: bool) -> (&'static str, Value) {
        ("Target.setDiscoverTargets", json!({ "discover": discover }))
    }

    // --- Network domain ---

    pub fn network_enable() -> (&'static str, Value) {
        ("Network.enable", json!({}))
    }

    pub fn network_set_request_interception(patterns: &[Value]) -> (&'static str, Value) {
        (
            "Fetch.enable",
            json!({ "patterns": patterns }),
        )
    }

    pub fn fetch_continue_request(request_id: &str, modifications: Option<Value>) -> (&'static str, Value) {
        let mut params = json!({ "requestId": request_id });
        if let Some(mods) = modifications {
            for (k, v) in mods.as_object().unwrap_or(&serde_json::Map::new()) {
                params[k] = v.clone();
            }
        }
        ("Fetch.continueRequest", params)
    }

    // --- Accessibility domain ---

    pub fn accessibility_get_full_tree() -> (&'static str, Value) {
        ("Accessibility.getFullAXTree", json!({}))
    }

    pub fn accessibility_enable() -> (&'static str, Value) {
        ("Accessibility.enable", json!({}))
    }

    // --- Page screenshot ---

    pub fn page_capture_screenshot(format: &str, quality: Option<u8>, clip: Option<Value>) -> (&'static str, Value) {
        let mut params = json!({ "format": format });
        if let Some(q) = quality {
            params["quality"] = json!(q);
        }
        if let Some(c) = clip {
            params["clip"] = c;
        }
        ("Page.captureScreenshot", params)
    }

    // --- Emulation ---

    pub fn emulation_set_device_metrics(width: u32, height: u32, scale: f64, mobile: bool) -> (&'static str, Value) {
        (
            "Emulation.setDeviceMetricsOverride",
            json!({
                "width": width,
                "height": height,
                "deviceScaleFactor": scale,
                "mobile": mobile
            }),
        )
    }

    // --- Browser domain ---

    pub fn browser_get_version() -> (&'static str, Value) {
        ("Browser.getVersion", json!({}))
    }

    // --- Network cookies ---

    pub fn network_get_cookies(urls: Option<&[&str]>) -> (&'static str, Value) {
        match urls {
            Some(u) => ("Network.getCookies", json!({ "urls": u })),
            None => ("Network.getCookies", json!({})),
        }
    }

    pub fn network_set_cookies(cookies: &[Value]) -> (&'static str, Value) {
        ("Network.setCookies", json!({ "cookies": cookies }))
    }

    pub fn network_clear_browser_cookies() -> (&'static str, Value) {
        ("Network.clearBrowserCookies", json!({}))
    }
}

/// Modifier key flags for Input.dispatchKeyEvent
pub mod modifiers {
    pub const NONE: u32 = 0;
    pub const ALT: u32 = 1;
    pub const CTRL: u32 = 2;
    pub const META: u32 = 4;
    pub const SHIFT: u32 = 8;
}

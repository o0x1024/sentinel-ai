use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Juggler protocol command envelope
#[derive(Debug, Clone, Serialize)]
pub struct JugglerCommand {
    pub id: u64,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

/// Juggler protocol response envelope
#[derive(Debug, Clone, Deserialize)]
pub struct JugglerResponse {
    pub id: Option<u64>,
    pub result: Option<Value>,
    pub error: Option<JugglerError>,
    pub method: Option<String>,
    pub params: Option<Value>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JugglerError {
    pub message: String,
    pub data: Option<String>,
}

impl JugglerResponse {
    pub fn is_event(&self) -> bool {
        self.method.is_some() && self.id.is_none()
    }
}

/// Juggler domains and methods mapping
pub mod methods {
    // Browser domain
    pub const BROWSER_GET_INFO: &str = "Browser.getInfo";
    pub const BROWSER_CLOSE: &str = "Browser.close";

    // Target domain
    pub const TARGET_GET_TARGETS: &str = "Target.getTargets";
    pub const TARGET_NEW_PAGE: &str = "Target.newPage";
    pub const TARGET_CLOSE_PAGE: &str = "Target.closePage";
    pub const TARGET_ACTIVATE: &str = "Target.activate";
    pub const TARGET_ATTACH: &str = "Target.attach";

    // Page domain
    pub const PAGE_NAVIGATE: &str = "Page.navigate";
    pub const PAGE_RELOAD: &str = "Page.reload";
    pub const PAGE_GO_BACK: &str = "Page.goBack";
    pub const PAGE_GO_FORWARD: &str = "Page.goForward";
    pub const PAGE_SCREENSHOT: &str = "Page.screenshot";
    pub const PAGE_GET_CONTENT: &str = "Page.getContent";
    pub const PAGE_EVALUATE: &str = "Page.evaluate";
    pub const PAGE_ADD_SCRIPT: &str = "Page.addScriptToEvaluateOnNewDocument";

    // Input domain (Juggler's key difference from CDP)
    pub const INPUT_DISPATCH_MOUSE_EVENT: &str = "Page.dispatchMouseEvent";
    pub const INPUT_DISPATCH_KEY_EVENT: &str = "Page.dispatchKeyEvent";
    pub const INPUT_SCROLL: &str = "Page.scrollIntoView";
    pub const INPUT_INSERT_TEXT: &str = "Page.insertText";

    // Network domain
    pub const NETWORK_SET_REQUEST_INTERCEPTION: &str = "Network.setRequestInterception";
    pub const NETWORK_CONTINUE_REQUEST: &str = "Network.continueInterceptedRequest";
    pub const NETWORK_GET_COOKIES: &str = "Network.getCookies";
    pub const NETWORK_SET_COOKIES: &str = "Network.setCookies";

    // Accessibility domain
    pub const ACCESSIBILITY_GET_TREE: &str = "Accessibility.getFullAXTree";
}

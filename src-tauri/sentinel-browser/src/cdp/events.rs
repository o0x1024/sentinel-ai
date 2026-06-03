use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A CDP event received from the browser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpEvent {
    pub method: String,
    pub params: Value,
    /// Session ID if event belongs to a specific target session
    pub session_id: Option<String>,
}

impl CdpEvent {
    /// Check if this event matches a specific method
    pub fn is(&self, method: &str) -> bool {
        self.method == method
    }

    /// Check if this event belongs to a specific session
    pub fn for_session(&self, session_id: &str) -> bool {
        self.session_id.as_deref() == Some(session_id)
    }
}

/// Well-known CDP event methods
pub mod methods {
    pub const PAGE_LOAD_EVENT_FIRED: &str = "Page.loadEventFired";
    pub const PAGE_DOM_CONTENT_LOADED: &str = "Page.domContentEventFired";
    pub const PAGE_FRAME_NAVIGATED: &str = "Page.frameNavigated";
    pub const PAGE_FRAME_STOPPED_LOADING: &str = "Page.frameStoppedLoading";

    pub const NETWORK_REQUEST_WILL_BE_SENT: &str = "Network.requestWillBeSent";
    pub const NETWORK_RESPONSE_RECEIVED: &str = "Network.responseReceived";
    pub const NETWORK_LOADING_FINISHED: &str = "Network.loadingFinished";
    pub const NETWORK_LOADING_FAILED: &str = "Network.loadingFailed";

    pub const FETCH_REQUEST_PAUSED: &str = "Fetch.requestPaused";

    pub const TARGET_TARGET_CREATED: &str = "Target.targetCreated";
    pub const TARGET_TARGET_DESTROYED: &str = "Target.targetDestroyed";
    pub const TARGET_ATTACHED_TO_TARGET: &str = "Target.attachedToTarget";
    pub const TARGET_DETACHED_FROM_TARGET: &str = "Target.detachedFromTarget";

    pub const RUNTIME_EXCEPTION_THROWN: &str = "Runtime.exceptionThrown";
    pub const RUNTIME_CONSOLE_API_CALLED: &str = "Runtime.consoleAPICalled";
}

/// Helper to wait for a specific event on an event receiver
pub async fn wait_for_event(
    rx: &mut tokio::sync::broadcast::Receiver<CdpEvent>,
    method: &str,
    session_id: Option<&str>,
    timeout: std::time::Duration,
) -> Result<CdpEvent, String> {
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(format!("Timeout waiting for event '{}'", method));
        }

        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Ok(event)) => {
                if event.method == method {
                    match session_id {
                        Some(sid) if event.session_id.as_deref() != Some(sid) => continue,
                        _ => return Ok(event),
                    }
                }
            }
            Ok(Err(e)) => return Err(format!("Event channel error: {}", e)),
            Err(_) => return Err(format!("Timeout waiting for event '{}'", method)),
        }
    }
}

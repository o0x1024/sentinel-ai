use std::time::Duration;

/// Condition to wait for before proceeding
#[derive(Debug, Clone)]
pub enum WaitCondition {
    /// Wait for an element matching the selector to appear
    Element { selector: String, timeout: Duration },
    /// Wait for navigation to complete
    Navigation { timeout: Duration },
    /// Wait for network to be idle (no pending requests for `idle_time`)
    NetworkIdle { idle_time: Duration, timeout: Duration },
    /// Wait for a fixed duration
    Time(Duration),
    /// Wait for a JS expression to return truthy
    JsExpression { expression: String, timeout: Duration },
    /// Wait for page to reach a specific lifecycle state
    Lifecycle { state: LifecycleState, timeout: Duration },
}

#[derive(Debug, Clone, Copy)]
pub enum LifecycleState {
    DomContentLoaded,
    Load,
    NetworkIdle,
    NetworkAlmostIdle,
}

impl Default for WaitCondition {
    fn default() -> Self {
        WaitCondition::NetworkIdle {
            idle_time: Duration::from_millis(500),
            timeout: Duration::from_secs(30),
        }
    }
}

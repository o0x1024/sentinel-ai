//! Core type definitions for Test Explorer V1

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Configuration for Test Explorer V1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestExplorerV1Config {
    /// Target URL to start exploration
    pub target_url: String,
    
    /// Maximum steps to execute
    pub max_steps: u32,
    
    /// Run browser in headless mode
    pub headless: bool,
    
    /// Enable network capture for API discovery
    pub capture_network: bool,
    
    /// Browser window size
    pub viewport_width: u32,
    pub viewport_height: u32,
    
    /// Timeout for page load (milliseconds)
    pub page_load_timeout_ms: u64,
    
    /// User agent string
    pub user_agent: Option<String>,
}

impl Default for TestExplorerV1Config {
    fn default() -> Self {
        Self {
            target_url: "about:blank".to_string(),
            max_steps: 50,
            headless: false,
            capture_network: true,
            viewport_width: 1280,
            viewport_height: 720,
            page_load_timeout_ms: 30000,
            user_agent: None,
        }
    }
}

/// Current page state (text-based representation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageState {
    /// Current URL
    pub url: String,
    
    /// Page title
    pub title: String,
    
    /// Visible text content (extracted from DOM)
    pub visible_text: String,
    
    /// Simplified HTML (cleaned, without scripts/styles)
    pub simplified_html: String,
    
    /// Interactive elements found on the page
    pub interactive_elements: Vec<InteractiveElement>,
    
    /// Captured API requests (if network capture enabled)
    pub captured_apis: Vec<ApiRequest>,
    
    /// Timestamp when state was captured
    pub timestamp: SystemTime,
}

impl Default for PageState {
    fn default() -> Self {
        Self {
            url: String::new(),
            title: String::new(),
            visible_text: String::new(),
            simplified_html: String::new(),
            interactive_elements: Vec::new(),
            captured_apis: Vec::new(),
            timestamp: SystemTime::now(),
        }
    }
}

/// Interactive element on the page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveElement {
    /// Element index (for reference in actions)
    pub index: usize,
    
    /// Element type (button, link, input, etc.)
    pub element_type: String,
    
    /// CSS selector
    pub selector: String,
    
    /// Visible text or label
    pub text: String,
    
    /// Element attributes (id, name, class, etc.)
    pub attributes: HashMap<String, String>,
    
    /// Bounding box coordinates
    pub bounds: Option<ElementBounds>,
}

/// Element bounding box
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Captured API request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    /// Request ID (from CDP)
    pub request_id: String,
    
    /// HTTP method
    pub method: String,
    
    /// Request URL
    pub url: String,
    
    /// Request headers
    pub headers: HashMap<String, String>,
    
    /// Request body (if available)
    pub request_body: Option<String>,
    
    /// Response status code
    pub response_status: Option<u16>,
    
    /// Response headers
    pub response_headers: Option<HashMap<String, String>>,
    
    /// Response body (if available)
    pub response_body: Option<String>,
    
    /// Request timestamp
    pub timestamp: SystemTime,
    
    /// Resource type (XHR, Fetch, Document, etc.)
    pub resource_type: String,
}

/// Action to perform on the page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Action type
    pub action_type: ActionType,
    
    /// CSS selector (for click, fill, etc.)
    pub selector: Option<String>,
    
    /// Element index (alternative to selector)
    pub index: Option<usize>,
    
    /// Value to fill (for input fields)
    pub value: Option<String>,
    
    /// URL to navigate to
    pub url: Option<String>,
    
    /// JSON schema for data extraction
    pub schema: Option<serde_json::Value>,
    
    /// URL pattern to wait for (API requests)
    pub url_pattern: Option<String>,
    
    /// Timeout in milliseconds
    pub timeout_ms: Option<u64>,
}

/// Action types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    /// Navigate to a URL
    Navigate,
    
    /// Click an element
    Click,
    
    /// Fill an input field
    Fill,
    
    /// Scroll the page
    Scroll,
    
    /// Wait for a duration
    Wait,
    
    /// Go back in history
    Back,
    
    /// Extract structured data
    Extract,
    
    /// Wait for an API request matching pattern
    WaitForApi,
    
    /// Finish execution
    Finish,
}

/// Execution step record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    /// Step number
    pub step_number: usize,
    
    /// Action performed
    pub action: Action,
    
    /// Result of the action
    pub success: bool,
    
    /// Error message (if failed)
    pub error: Option<String>,
    
    /// Page state after action
    pub resulting_state: Option<PageState>,
    
    /// Timestamp
    pub timestamp: SystemTime,
    
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Final execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether execution was successful
    pub success: bool,
    
    /// Final page state
    pub final_state: PageState,
    
    /// All captured API requests
    pub captured_apis: Vec<ApiRequest>,
    
    /// Extracted data (if any)
    pub extracted_data: Option<serde_json::Value>,
    
    /// All execution steps
    pub steps_taken: Vec<ExecutionStep>,
    
    /// Total execution time in milliseconds
    pub total_duration_ms: u64,
    
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Stream event for real-time updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    /// Step started
    StepStarted {
        step_number: usize,
        action: Action,
    },
    
    /// Step completed
    StepCompleted {
        step_number: usize,
        success: bool,
        error: Option<String>,
    },
    
    /// Page state updated
    PageStateUpdated {
        state: PageState,
    },
    
    /// API request captured
    ApiCaptured {
        request: ApiRequest,
    },
    
    /// Log message
    Log {
        level: String,
        message: String,
    },
    
    /// Execution finished
    Finished {
        result: ExecutionResult,
    },
}


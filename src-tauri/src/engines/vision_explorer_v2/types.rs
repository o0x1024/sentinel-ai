//! Data types for Vision Explorer V2 ReAct Architecture

use crate::engines::LlmConfig;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ==================== Configuration ====================

/// Configuration for Vision Explorer V2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionExplorerV2Config {
    /// Target URL to start exploration
    pub target_url: String,

    /// Maximum depth to explore
    pub max_depth: u32,

    /// Maximum total steps (budget)
    pub max_steps: u32,

    /// UserAgent string
    pub user_agent: Option<String>,

    /// Headless mode
    pub headless: bool,

    /// AI Configuration
    pub ai_config: AIConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    /// Fast LLM for text reasoning
    pub fast_model_id: String,

    /// Vision LLM for page analysis
    pub vision_model_id: String,

    /// Base provider for the fast model
    pub fast_provider: String,

    /// Base provider for the vision model
    pub vision_provider: String,

    /// API Key for fast model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_api_key: Option<String>,

    /// API Key for vision model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_api_key: Option<String>,

    /// Base URL for fast model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_base_url: Option<String>,

    /// Base URL for vision model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_base_url: Option<String>,
}

impl AIConfig {
    /// Create LlmConfig for fast model
    pub fn fast_llm_config(&self) -> LlmConfig {
        let mut config = LlmConfig::new(&self.fast_provider, &self.fast_model_id);
        if let Some(ref key) = self.fast_api_key {
            config = config.with_api_key(key.clone());
        }
        if let Some(ref url) = self.fast_base_url {
            config = config.with_base_url(url.clone());
        }
        config
    }

    /// Create LlmConfig for vision model
    pub fn vision_llm_config(&self) -> LlmConfig {
        let mut config = LlmConfig::new(&self.vision_provider, &self.vision_model_id);
        if let Some(ref key) = self.vision_api_key {
            config = config.with_api_key(key.clone());
        }
        if let Some(ref url) = self.vision_base_url {
            config = config.with_base_url(url.clone());
        }
        config
    }
}

impl Default for VisionExplorerV2Config {
    fn default() -> Self {
        Self {
            target_url: "about:blank".to_string(),
            max_depth: 5,
            max_steps: 100,
            user_agent: None,
            headless: false,
            ai_config: AIConfig {
                fast_model_id: "claude-3-haiku".to_string(),
                vision_model_id: "claude-3-sonnet".to_string(),
                fast_provider: "anthropic".to_string(),
                vision_provider: "anthropic".to_string(),
                fast_api_key: None,
                vision_api_key: None,
                fast_base_url: None,
                vision_base_url: None,
            },
        }
    }
}

// ==================== Page Context ====================

/// Current page context (input for perception)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContext {
    pub url: String,
    pub title: String,
    pub screenshot: Vec<u8>,
    pub html: String,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub timestamp: u64,
}

impl PageContext {
    /// Generate a fingerprint for this page state
    pub fn fingerprint(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        let url_base = self.url.split('?').next().unwrap_or(&self.url);
        url_base.hash(&mut hasher);
        self.title.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

// ==================== Observation (Perception Output) ====================

/// Observation from perception engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Type of the current page
    pub page_type: PageType,

    /// Natural language description of the page
    pub description: String,

    /// Authentication status
    pub auth_status: AuthStatus,

    /// Interactive elements found on the page (with @e1, @e2 refs)
    pub elements: Vec<Element>,

    /// Forms detected
    pub forms: Vec<FormInfo>,

    /// Links discovered
    pub links: Vec<String>,

    /// API endpoints discovered (from network requests)
    pub api_endpoints: Vec<String>,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Snapshot ARIA tree (for LLM context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_tree: Option<String>,
}

/// Type of page
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageType {
    Login,
    Dashboard,
    Form,
    List,
    Detail,
    Api,
    Error,
    Static,
    Unknown,
}

/// Authentication status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthStatus {
    Authenticated { username: Option<String> },
    NotAuthenticated,
    Unknown,
}

/// Interactive element on the page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    pub element_id: String,
    pub element_type: String, // button, link, input, etc.
    pub selector: String,
    pub text: Option<String>,
    pub href: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub is_visible: bool,
}

/// Form information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormInfo {
    pub selector: String,
    pub action: Option<String>,
    pub method: String,
    pub fields: Vec<FormField>,
}

/// Form field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub field_type: String,
    pub label: Option<String>,
    pub required: bool,
    pub value: Option<String>,
    pub placeholder: Option<String>,
}

// ==================== Action (What to execute) ====================

/// Action to be executed by the action executor
/// Supports hybrid mode: index (DOM annotation) > selector > coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Navigate to a URL
    Navigate { url: String },

    /// Click an element (priority: index > selector > coordinates)
    Click {
        /// Annotation index from get_annotated_elements (preferred)
        index: Option<u32>,
        /// CSS selector (fallback)
        selector: Option<String>,
        /// X coordinate (vision fallback)
        x: Option<i32>,
        /// Y coordinate (vision fallback)
        y: Option<i32>,
    },

    /// Fill a form field (priority: index > selector)
    Fill {
        /// Annotation index (preferred)
        index: Option<u32>,
        /// CSS selector (fallback)
        selector: Option<String>,
        /// Value to fill
        value: String,
    },

    /// Submit a form (press Enter)
    Submit { selector: String },

    /// Scroll the page
    Scroll {
        direction: ScrollDirection,
        amount: u32,
    },

    /// Wait for a duration
    Wait { duration_ms: u64 },

    /// Take a snapshot of current state
    TakeSnapshot,

    /// Go back in browser history
    GoBack,

    /// Stop exploration
    Stop { reason: String },
}

/// Scroll direction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

// ==================== Action Result ====================

/// Result of executing an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Whether the action succeeded
    pub success: bool,
    
    /// New URL after action (if changed)
    pub new_url: Option<String>,
    
    /// Error message if failed
    pub error: Option<String>,
    
    /// New observation after action
    pub observation: Option<Observation>,
}

// ==================== Exploration State ====================

/// State of the exploration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationState {
    /// Current URL
    pub current_url: String,
    
    /// Current depth in exploration tree
    pub current_depth: u32,
    
    /// Maximum allowed depth
    pub max_depth: u32,
    
    /// Number of steps taken
    pub steps_taken: u32,
    
    /// Maximum allowed steps
    pub max_steps: u32,
    
    /// URLs that have been visited
    pub visited_urls: HashSet<String>,
    
    /// API endpoints discovered
    pub discovered_apis: Vec<String>,
    
    /// History of steps
    pub history: Vec<Step>,
    
    /// Whether exploration is complete
    pub is_complete: bool,
    
    /// Reason for completion (if complete)
    pub completion_reason: Option<String>,
}

impl ExplorationState {
    pub fn new(start_url: String, max_depth: u32, max_steps: u32) -> Self {
        Self {
            current_url: start_url.clone(),
            current_depth: 0,
            max_depth,
            steps_taken: 0,
            max_steps,
            visited_urls: HashSet::from([start_url]),
            discovered_apis: Vec::new(),
            history: Vec::new(),
            is_complete: false,
            completion_reason: None,
        }
    }

    /// Check if should continue exploration
    pub fn should_continue(&self) -> bool {
        !self.is_complete 
            && self.steps_taken < self.max_steps 
            && self.current_depth <= self.max_depth
    }

    /// Record a step
    pub fn record_step(&mut self, step: Step) {
        self.steps_taken += 1;
        self.history.push(step);
    }

    /// Mark URL as visited
    pub fn mark_visited(&mut self, url: String) {
        self.visited_urls.insert(url);
    }

    /// Check if URL has been visited
    pub fn is_visited(&self, url: &str) -> bool {
        self.visited_urls.contains(url)
    }

    /// Add discovered API
    pub fn add_api(&mut self, api_url: String) {
        if !self.discovered_apis.contains(&api_url) {
            self.discovered_apis.push(api_url);
        }
    }

    /// Complete exploration
    pub fn complete(&mut self, reason: String) {
        self.is_complete = true;
        self.completion_reason = Some(reason);
    }
}

// ==================== Step (History Entry) ====================

/// A single step in the exploration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub step_number: u32,
    pub observation: Observation,
    pub thought: String,
    pub action: Action,
    pub result: ActionResult,
    pub timestamp: u64,
}

// ==================== ReAct Decision (LLM Output) ====================

/// Decision made by the ReAct LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReActDecision {
    /// Thought process
    pub thought: String,
    
    /// Chosen action
    pub action: Action,
    
    /// Reasoning for the action
    pub reason: String,
}

// ==================== Exploration Result ====================

/// Final result of exploration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationResult {
    pub success: bool,
    pub pages_visited: u32,
    pub apis_discovered: u32,
    pub actions_performed: u32,
    pub duration_seconds: u64,
    pub error: Option<String>,
    pub graph: serde_json::Value, // Simplified graph for export
}

// ==================== Message Types (for UI updates) ====================

/// Message types sent to the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VisionMessage {
    Started {
        session_id: String,
        target_url: String,
    },
    Step {
        step_number: u32,
        thought: String,
        action: String,
        current_url: String,
    },
    /// Screenshot captured
    Screenshot {
        step_number: u32,
        screenshot_base64: String,
        url: String,
        title: String,
    },
    /// Page analysis result
    Analysis {
        step_number: u32,
        page_type: PageType,
        description: String,
        elements_count: usize,
        forms_count: usize,
        links_count: usize,
    },
    /// Action being executed
    ActionExecuting {
        step_number: u32,
        action_type: String,
        action_details: serde_json::Value,
    },
    /// Action execution result
    ActionResult {
        step_number: u32,
        success: bool,
        error: Option<String>,
        new_url: Option<String>,
    },
    Observation {
        step_number: u32,
        page_type: PageType,
        description: String,
        elements_count: usize,
    },
    Progress {
        steps_taken: u32,
        max_steps: u32,
        pages_visited: u32,
        apis_discovered: u32,
    },
    ApiDiscovered {
        url: String,
        method: String,
    },
    Completed {
        success: bool,
        result: ExplorationResult,
    },
    Error {
        message: String,
    },
}

// ==================== Hybrid Exploration Types ====================

/// Site type detection result
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SiteProfile {
    /// Site type: SPA, MPA, or Hybrid
    #[serde(rename = "type")]
    pub site_type: String,
    /// Detected framework (Vue, React, Angular, etc.)
    pub framework: Option<String>,
    /// Whether authentication is detected
    #[serde(rename = "hasAuth")]
    pub has_auth: bool,
    /// Number of forms on the page
    #[serde(rename = "formCount")]
    pub form_count: u32,
    /// Whether virtual scroll is detected
    #[serde(rename = "hasVirtualScroll")]
    pub has_virtual_scroll: bool,
    /// Router type (hash, history, vue-router, react-router)
    #[serde(rename = "routerType")]
    pub router_type: Option<String>,
    /// Detection confidence (0.0 - 1.0)
    pub confidence: f32,
}

impl SiteProfile {
    pub fn is_spa(&self) -> bool {
        self.site_type == "SPA"
    }

    pub fn is_mpa(&self) -> bool {
        self.site_type == "MPA"
    }
}

/// Browser storage state (for session management)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageState {
    /// Cookies
    #[serde(default)]
    pub cookies: Vec<CookieData>,
    /// LocalStorage key-value pairs
    #[serde(default, rename = "localStorage")]
    pub local_storage: HashMap<String, String>,
    /// Auth indicators found
    #[serde(default, rename = "authIndicators")]
    pub auth_indicators: Vec<String>,
    /// Whether auth tokens were detected
    #[serde(default, rename = "hasAuth")]
    pub has_auth: bool,
}

/// Cookie data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieData {
    /// Cookie name
    #[serde(alias = "n")]
    pub name: String,
    /// Cookie value
    #[serde(alias = "v")]
    pub value: String,
    /// Domain
    #[serde(alias = "d")]
    pub domain: String,
    /// Path
    #[serde(alias = "p", default = "default_path")]
    pub path: String,
    /// HTTP only flag
    #[serde(alias = "h", default)]
    pub http_only: bool,
    /// Secure flag
    #[serde(alias = "s", default)]
    pub secure: bool,
}

fn default_path() -> String {
    "/".to_string()
}

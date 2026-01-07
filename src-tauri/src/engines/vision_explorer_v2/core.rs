use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

// ==================== Event System ====================

/// Event types for the Vision Explorer V2 Event Bus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// A new task has been assigned to an agent
    TaskAssigned {
        agent_id: String,
        task_id: String,
        target_node_id: String,
        #[serde(default)]
        payload: Option<serde_json::Value>,
    },
    /// An agent has completed a task
    TaskCompleted {
        agent_id: String,
        task_id: String,
        result: TaskResult,
    },
    /// A new node (page state) has been discovered
    NodeDiscovered {
        source_node_id: String,
        new_node_id: String,
        action: String, // The action that led to this node
    },
    /// User provided credentials for login
    CredentialsReceived {
        username: String,
        password: String,
        verification_code: Option<String>,
    },
    /// Request user to provide login credentials
    LoginTakeoverRequest {
        url: String,
        fields: Vec<LoginField>,
    },
    /// User chose to skip login
    SkipLogin,
    /// User manually completed login (via browser window)
    ManualLoginComplete,
    /// Login wait timeout expired - LLM should attempt auto-login
    LoginTimeout { url: String },
    /// System log/status update
    Log { level: String, message: String },
    /// Stop signal
    Stop,
}

/// Login form field information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginField {
    pub id: String,
    pub field_type: String, // "text", "password", "email", etc.
    pub label: String,
    pub required: bool,
}

/// Result of an agent's task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub message: String,
    pub new_nodes: Vec<String>,
    pub data: Option<serde_json::Value>,
}

// ==================== Agent Interface ====================

/// A standard agent in the system
#[async_trait]
pub trait Agent: Send + Sync + Debug {
    /// Get the agent's unique ID
    fn id(&self) -> String;

    /// Handle an incoming event
    async fn handle_event(&self, event: &Event) -> Result<()>;
}

// ==================== Perception Data Types ====================

/// Context passed to analysts and agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContext {
    pub url: String,
    pub title: String,
    pub screenshot: Vec<u8>,
    pub dom_snapshot: String,
    pub accessibility_tree: Option<serde_json::Value>,
    pub viewport_size: Option<(u32, u32)>,
    pub timestamp: u64, // Unix timestamp millis
}

impl PageContext {
    /// Generate a unique fingerprint for this page state.
    pub fn fingerprint(&self, is_authenticated: bool) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // 1. URL (without query params for stability)
        let url_base = self.url.split('?').next().unwrap_or(&self.url);
        url_base.hash(&mut hasher);

        // 2. Title (helps distinguish same-URL states like tabs)
        self.title.hash(&mut hasher);

        // 3. Auth Status (distinguishes logged in vs logged out versions of the same page)
        is_authenticated.hash(&mut hasher);

        format!("{:016x}", hasher.finish())
    }
}

/// Type of page detected
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

/// Authentication status detected on the page
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthStatus {
    Authenticated { username: Option<String> },
    NotAuthenticated,
    Unknown,
}

/// Interactive element found on the page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageElement {
    pub element_type: String,
    pub selector: String,
    pub text: Option<String>,
    pub attributes: HashMap<String, String>,
    pub coordinates: Option<(i32, i32)>,
    pub bounding_box: Option<(i32, i32, u32, u32)>,
    pub confidence: f32,
    pub is_visible: bool,
    pub is_interactive: bool,
}

/// Form information detected on the page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormInfo {
    pub selector: String,
    pub action: Option<String>,
    pub method: String,
    pub fields: Vec<FormField>,
    pub is_login_form: bool,
    pub is_search_form: bool,
}

/// Form field information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub field_type: String,
    pub label: Option<String>,
    pub required: bool,
    pub value: Option<String>,
    pub placeholder: Option<String>,
}

/// API endpoint information detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub url: String,
    pub method: String,
    pub parameters: Vec<String>,
    pub requires_auth: bool,
    pub response_format: Option<String>,
}

/// Pure perception result - only describes what was found
/// Note: Decision making (suggested actions) is moved to Planner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionResult {
    /// Type of page detected
    pub page_type: PageType,
    
    /// Authentication status
    pub auth_status: AuthStatus,
    
    /// Summary of page content
    pub content_summary: String,
    
    /// Interactive elements found
    pub elements: Vec<PageElement>,
    
    /// Forms detected
    pub forms: Vec<FormInfo>,
    
    /// API endpoints discovered
    pub api_endpoints: Vec<ApiEndpoint>,
    
    /// Errors or warnings detected
    pub errors: Vec<String>,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    
    /// Confidence in the analysis (0.0 to 1.0)
    pub confidence: f32,
}

/// Capabilities of a perception engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionCapabilities {
    pub name: String,
    pub version: String,
    pub supported_analysis: Vec<String>,
    pub supports_vision: bool,
    pub supports_dom: bool,
    pub supports_accessibility: bool,
}

// ==================== Perception Interface ====================

/// The Perception Engine Trait (Analyst)
/// Responsible for understanding the page content.
#[async_trait]
pub trait PerceptionEngine: Send + Sync {
    /// Analyze the current page context and return understanding
    async fn analyze(&self, context: &PageContext) -> Result<PerceptionResult>;

    /// Extract specific structured data from the page.
    async fn extract_data(
        &self,
        context: &PageContext,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value>;

    /// Detect if this is a login page
    async fn detect_login_page(&self, context: &PageContext) -> Result<bool>;

    /// Extract login form fields if present
    async fn extract_login_fields(&self, context: &PageContext) -> Result<Vec<FormField>>;

    /// Get engine capabilities/metadata
    fn capabilities(&self) -> PerceptionCapabilities;
}

// ==================== Planner/Graph Types ====================

/// Action suggested by the Planner (connected to Edges in ExplorationGraph)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub description: String,
    pub selector: String,
    pub action_type: String,
    pub value: Option<String>,
    pub confidence: f32,
    pub x: Option<i32>, // Pure vision: X coordinate
    pub y: Option<i32>, // Pure vision: Y coordinate
}

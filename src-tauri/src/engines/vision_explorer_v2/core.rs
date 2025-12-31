use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// The Perception Engine Trait (Analyst)
/// Responsible for understanding the page content.
#[async_trait]
pub trait PerceptionEngine: Send + Sync {
    /// Analyze the current page context and return a decision or analysis result.
    async fn analyze(&self, context: &PageContext) -> Result<PerceptionResult>;

    /// Extract specific structured data from the page.
    async fn extract_data(
        &self,
        context: &PageContext,
        schema: &serde_json::Value,
    ) -> Result<serde_json::Value>;
}

/// A standard agent in the system
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get the agent's unique ID
    fn id(&self) -> String;

    /// Handle an incoming event
    async fn handle_event(&self, event: &Event) -> Result<()>;
}

/// Context passed to analysts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContext {
    pub url: String,
    pub title: String,
    pub screenshot: Option<Vec<u8>>,
    pub dom_snapshot: String,
    pub accessibility_tree: Option<serde_json::Value>,
}

impl PageContext {
    /// Generate a unique fingerprint for this page state.
    /// Uses URL + normalized DOM structure hash for SPA support.
    pub fn fingerprint(&self, is_authenticated: bool) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // 1. URL (without query params for stability)
        let url_base = self.url.split('?').next().unwrap_or(&self.url);
        url_base.hash(&mut hasher);

        // 2. DOM Structure Hash (not content, just structure)
        let structure_hash = Self::compute_dom_structure_hash(&self.dom_snapshot);
        structure_hash.hash(&mut hasher);

        // 3. Title (helps distinguish same-URL states like tabs)
        self.title.hash(&mut hasher);

        // 4. Auth Status (distinguishes logged in vs logged out versions of the same page)
        is_authenticated.hash(&mut hasher);

        format!("{:016x}", hasher.finish())
    }

    /// Compute a hash of the DOM structure, ignoring dynamic content.
    /// This extracts tag names, hierarchy, and key functional attributes (id, name, type, etc.),
    /// filtering out text content, volatile classes, and other noise.
    fn compute_dom_structure_hash(dom: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Extract structural elements only (tag names and their nesting)
        let mut depth: u32 = 0;
        let mut in_tag = false;
        let mut tag_content = String::new();
        let mut is_closing = false;

        for ch in dom.chars() {
            match ch {
                '<' => {
                    in_tag = true;
                    tag_content.clear();
                    is_closing = false;
                }
                '/' if in_tag && tag_content.is_empty() => {
                    is_closing = true;
                }
                '>' => {
                    if in_tag && !tag_content.is_empty() {
                        // Split tag name and attributes
                        let parts: Vec<&str> = tag_content.split_whitespace().collect();
                        if let Some(tag_name_raw) = parts.first() {
                            let normalized = tag_name_raw.to_lowercase();

                            // Skip script/style content and dynamic elements
                            if !matches!(
                                normalized.as_str(),
                                "script" | "style" | "noscript" | "iframe"
                            ) {
                                // Hash: depth + tag name
                                depth.hash(&mut hasher);
                                normalized.hash(&mut hasher);

                                // For interactive elements, hash key attributes to distinguish state
                                if matches!(
                                    normalized.as_str(),
                                    "input" | "button" | "a" | "select" | "textarea" | "form"
                                ) {
                                    for attr in &parts[1..] {
                                        if let Some((key, val)) = attr.split_once('=') {
                                            let key_lc = key.to_lowercase();
                                            // Only hash stable and functional attributes
                                            if matches!(
                                                key_lc.as_str(),
                                                "id" | "name" | "type" | "href" | "value" | "placeholder" | "role"
                                            ) {
                                                key_lc.hash(&mut hasher);
                                                val.hash(&mut hasher);
                                            }
                                        }
                                    }
                                }
                            }

                            if is_closing {
                                depth = depth.saturating_sub(1);
                            } else if !normalized.ends_with('/') && !is_self_closing(&normalized) {
                                // Increase depth for opening tags that aren't self-closing
                                depth += 1;
                            }
                        }
                    }
                    in_tag = false;
                }
                _ if in_tag => {
                    if tag_content.len() < 500 {
                        // Limit tag content length
                        tag_content.push(ch);
                    }
                }
                _ => {}
            }
        }

        hasher.finish()
    }
}

/// Helper to identify common self-closing tags
fn is_self_closing(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Output of a perception analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionResult {
    pub summary: String,
    pub suggested_actions: Vec<SuggestedAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub description: String,
    pub selector: String,
    pub action_type: String,
    pub value: Option<String>,
    pub confidence: f32,
}

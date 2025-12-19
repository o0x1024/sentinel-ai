//! Blackboard - Shared data area for agents
//!
//! The Blackboard is a centralized state store that all agents can read from and write to.
//! It contains:
//! - Authentication state (cookies, tokens, logged-in user info)
//! - Global configuration (scope limits, excluded patterns)
//! - Discovered credentials and secrets
//! - Error patterns observed

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// The shared Blackboard accessible to all agents
#[derive(Debug, Clone)]
pub struct Blackboard {
    inner: Arc<RwLock<BlackboardData>>,
}

/// Internal data structure for the Blackboard
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlackboardData {
    /// Authentication state
    pub auth: AuthState,

    /// Global exploration configuration
    pub config: ExplorationConfig,

    /// Discovered secrets/credentials (for reporting)
    pub secrets: Vec<DiscoveredSecret>,

    /// Error patterns observed during exploration
    pub error_patterns: Vec<ErrorPattern>,

    /// Key-value store for arbitrary agent data sharing
    pub kv_store: HashMap<String, serde_json::Value>,

    /// URLs that should be skipped (learned during exploration)
    pub skip_urls: HashSet<String>,

    /// API endpoints discovered
    pub api_endpoints: Vec<ApiEndpoint>,
}

/// Authentication state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthState {
    /// Whether we are currently logged in
    pub is_authenticated: bool,

    /// Current session cookies (name -> value)
    pub cookies: HashMap<String, String>,

    /// Bearer/API tokens discovered
    pub tokens: Vec<String>,

    /// Current user info (if known)
    pub current_user: Option<UserInfo>,

    /// Login credentials (if takeover mode)
    pub credentials: Option<Credentials>,

    /// Login page URL (for recovery)
    pub login_url: Option<String>,

    /// Whether we are waiting for user to login manually
    pub is_waiting_for_login: bool,

    /// Timestamp when login wait started (Unix millis)
    #[serde(default)]
    pub login_wait_started: Option<u64>,
}

/// User info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub username: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
}

/// Login credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

/// Exploration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationConfig {
    /// Base URL scope (only explore URLs under this)
    pub scope_base_url: Option<String>,

    /// Maximum depth to explore
    pub max_depth: u32,

    /// Maximum total steps
    pub max_steps: u32,

    /// URL patterns to exclude (regex strings)
    pub exclude_patterns: Vec<String>,

    /// Whether to allow destructive actions
    pub allow_destructive: bool,

    /// Whether to auto-fill forms with test data
    pub auto_fill_forms: bool,
}

impl Default for ExplorationConfig {
    fn default() -> Self {
        Self {
            scope_base_url: None,
            max_depth: 5,
            max_steps: 100,
            exclude_patterns: vec![
                r"logout".to_string(),
                r"signout".to_string(),
                r"delete".to_string(),
            ],
            allow_destructive: false,
            auto_fill_forms: true,
        }
    }
}

/// A discovered secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredSecret {
    pub secret_type: String, // "api_key", "password", "token"
    pub value: String,
    pub source_url: String,
    pub context: String,
}

/// An error pattern observed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub pattern: String,
    pub url: String,
    pub count: u32,
    pub action_that_caused: String,
}

/// A discovered API endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub url: String,
    pub method: String,
    pub params: Vec<String>,
    pub auth_required: bool,
}

impl Blackboard {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(BlackboardData::default())),
        }
    }

    pub fn with_config(config: ExplorationConfig) -> Self {
        let data = BlackboardData {
            config,
            ..Default::default()
        };
        Self {
            inner: Arc::new(RwLock::new(data)),
        }
    }

    // ==================== Auth Operations ====================

    pub async fn set_authenticated(&self, authenticated: bool) {
        let mut data = self.inner.write().await;
        data.auth.is_authenticated = authenticated;
    }

    pub async fn is_authenticated(&self) -> bool {
        self.inner.read().await.auth.is_authenticated
    }

    pub async fn set_cookies(&self, cookies: HashMap<String, String>) {
        let mut data = self.inner.write().await;
        data.auth.cookies = cookies;
    }

    pub async fn get_cookies(&self) -> HashMap<String, String> {
        self.inner.read().await.auth.cookies.clone()
    }

    pub async fn add_token(&self, token: String) {
        let mut data = self.inner.write().await;
        if !data.auth.tokens.contains(&token) {
            data.auth.tokens.push(token);
        }
    }

    pub async fn set_credentials(&self, username: String, password: String) {
        let mut data = self.inner.write().await;
        data.auth.credentials = Some(Credentials { username, password });
    }

    pub async fn get_credentials(&self) -> Option<Credentials> {
        self.inner.read().await.auth.credentials.clone()
    }

    pub async fn set_login_url(&self, url: String) {
        let mut data = self.inner.write().await;
        data.auth.login_url = Some(url);
    }

    /// Start waiting for user login
    pub async fn start_login_wait(&self) {
        let mut data = self.inner.write().await;
        data.auth.is_waiting_for_login = true;
        data.auth.login_wait_started = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        );
    }

    /// Clear login wait state
    pub async fn clear_login_wait(&self) {
        let mut data = self.inner.write().await;
        data.auth.is_waiting_for_login = false;
        data.auth.login_wait_started = None;
    }

    /// Check if we are waiting for login
    pub async fn is_waiting_for_login(&self) -> bool {
        self.inner.read().await.auth.is_waiting_for_login
    }

    /// Get login wait started timestamp
    pub async fn get_login_wait_started(&self) -> Option<u64> {
        self.inner.read().await.auth.login_wait_started
    }

    // ==================== Config Operations ====================

    pub async fn get_config(&self) -> ExplorationConfig {
        self.inner.read().await.config.clone()
    }

    pub async fn is_url_in_scope(&self, url: &str) -> bool {
        let data = self.inner.read().await;
        if let Some(base) = &data.config.scope_base_url {
            url.starts_with(base)
        } else {
            true
        }
    }

    pub async fn should_skip_url(&self, url: &str) -> bool {
        let data = self.inner.read().await;

        // Check explicit skip list
        if data.skip_urls.contains(url) {
            return true;
        }

        // Check exclude patterns
        for pattern in &data.config.exclude_patterns {
            if url.to_lowercase().contains(&pattern.to_lowercase()) {
                return true;
            }
        }

        false
    }

    pub async fn add_skip_url(&self, url: String) {
        let mut data = self.inner.write().await;
        data.skip_urls.insert(url);
    }

    // ==================== Discovery Operations ====================

    pub async fn add_secret(&self, secret: DiscoveredSecret) {
        let mut data = self.inner.write().await;
        data.secrets.push(secret);
    }

    pub async fn add_api_endpoint(&self, endpoint: ApiEndpoint) {
        let mut data = self.inner.write().await;
        // Avoid duplicates
        if !data
            .api_endpoints
            .iter()
            .any(|e| e.url == endpoint.url && e.method == endpoint.method)
        {
            data.api_endpoints.push(endpoint);
        }
    }

    pub async fn add_error_pattern(&self, pattern: String, url: String, action: String) {
        let mut data = self.inner.write().await;
        if let Some(existing) = data
            .error_patterns
            .iter_mut()
            .find(|e| e.pattern == pattern)
        {
            existing.count += 1;
        } else {
            data.error_patterns.push(ErrorPattern {
                pattern,
                url,
                count: 1,
                action_that_caused: action,
            });
        }
    }

    // ==================== KV Store Operations ====================

    pub async fn set_kv(&self, key: String, value: serde_json::Value) {
        let mut data = self.inner.write().await;
        data.kv_store.insert(key, value);
    }

    pub async fn get_kv(&self, key: &str) -> Option<serde_json::Value> {
        self.inner.read().await.kv_store.get(key).cloned()
    }

    // ==================== Serialization ====================

    pub async fn to_json(&self) -> serde_json::Value {
        let data = self.inner.read().await;
        serde_json::to_value(&*data).unwrap_or(serde_json::json!({}))
    }

    pub async fn from_json(json: serde_json::Value) -> Self {
        let data: BlackboardData = serde_json::from_value(json).unwrap_or_default();
        Self {
            inner: Arc::new(RwLock::new(data)),
        }
    }
}

impl Default for Blackboard {
    fn default() -> Self {
        Self::new()
    }
}

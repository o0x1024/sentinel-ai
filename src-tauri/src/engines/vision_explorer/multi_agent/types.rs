//! Multi-Agent types for Vision Explorer

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Exploration scope assigned to a worker agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorationScope {
    /// Unique scope identifier
    pub id: String,
    /// Scope name (e.g., "Products", "Admin", "Blog")
    pub name: String,
    /// URL prefix patterns that belong to this scope
    pub url_patterns: Vec<String>,
    /// Entry point URL
    pub entry_url: String,
    /// Maximum depth for exploration within this scope
    pub max_depth: u32,
    /// Priority (higher = more important)
    pub priority: u32,
}

/// Navigation entry discovered by Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationEntry {
    /// Entry text/label
    pub text: String,
    /// Target URL or href
    pub href: String,
    /// Element index for interaction
    pub element_index: Option<u32>,
    /// Inferred scope based on URL pattern
    pub inferred_scope: Option<String>,
    /// Is this a dropdown/submenu parent
    pub has_submenu: bool,
}

/// Manager's analysis result of homepage navigation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationAnalysis {
    /// Main navigation entries
    pub main_nav: Vec<NavigationEntry>,
    /// Secondary navigation entries (sidebar, footer, etc.)
    pub secondary_nav: Vec<NavigationEntry>,
    /// Detected login/auth entry points
    pub auth_entries: Vec<NavigationEntry>,
    /// Suggested scope divisions
    pub suggested_scopes: Vec<ExplorationScope>,
    /// Page type detection (SPA/MPA)
    pub is_spa: bool,
    /// Detected login gate (if present)
    pub has_login_gate: bool,
}

/// Task assignment from Manager to Worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerTask {
    /// Task ID
    pub id: String,
    /// Assigned scope
    pub scope: ExplorationScope,
    /// Additional context from Manager
    pub context: String,
    /// Max iterations for this task
    pub max_iterations: u32,
}

/// Worker's exploration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    /// Worker/Task ID
    pub task_id: String,
    /// Scope explored
    pub scope_id: String,
    /// URLs visited within scope
    pub visited_urls: Vec<String>,
    /// APIs discovered
    pub discovered_apis: Vec<DiscoveredApi>,
    /// Cross-scope links found (for other workers)
    pub cross_scope_links: Vec<CrossScopeLink>,
    /// Exploration stats
    pub stats: WorkerStats,
    /// Completion reason
    pub completion_reason: String,
    /// Whether re-planning is needed (e.g., after login success)
    #[serde(default)]
    pub needs_replan: bool,
}

/// API discovered by worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredApi {
    pub method: String,
    pub path: String,
    pub full_url: String,
    pub parameters: HashMap<String, String>,
    pub status_code: Option<u16>,
}

/// Link pointing to another scope (for cross-worker coordination)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossScopeLink {
    /// Source URL where link was found
    pub source_url: String,
    /// Target URL (in different scope)
    pub target_url: String,
    /// Inferred target scope
    pub target_scope: Option<String>,
}

/// Worker exploration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStats {
    pub pages_visited: usize,
    pub elements_interacted: usize,
    pub apis_discovered: usize,
    pub iterations_used: u32,
    pub duration_ms: u64,
}

/// Multi-agent exploration mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExplorationMode {
    /// Sequential: One worker at a time
    Sequential,
    /// Parallel: Multiple workers simultaneously
    Parallel,
    /// Adaptive: Start sequential, switch to parallel if beneficial
    Adaptive,
}

impl Default for ExplorationMode {
    fn default() -> Self {
        Self::Sequential
    }
}

/// Multi-agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAgentConfig {
    /// Exploration mode
    pub mode: ExplorationMode,
    /// Maximum concurrent workers (for parallel mode)
    pub max_concurrent_workers: usize,
    /// Maximum depth per worker
    pub default_max_depth: u32,
    /// Whether to explore login-protected areas
    pub explore_authenticated: bool,
    /// URL patterns to ignore globally
    pub global_ignore_patterns: Vec<String>,
}

impl Default for MultiAgentConfig {
    fn default() -> Self {
        Self {
            mode: ExplorationMode::Adaptive,
            max_concurrent_workers: 3,
            default_max_depth: 5,
            explore_authenticated: true,
            global_ignore_patterns: vec![
                "/logout".to_string(),
                "/signout".to_string(),
                "#".to_string(),
                "javascript:".to_string(),
                "退出".to_string(),
            ],
        }
    }
}

/// Agent role in multi-agent system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRole {
    Manager,
    Worker,
}

/// Message between agents
#[derive(Debug, Clone)]
pub enum AgentMessage {
    /// Manager -> Workers: Start exploration
    StartExploration { task: WorkerTask },
    /// Worker -> Manager: Report progress
    Progress { task_id: String, progress: f32 },
    /// Worker -> Manager: Report completion
    Complete { result: WorkerResult },
    /// Worker -> GlobalState: Report discovered URL
    UrlDiscovered { url: String, scope_id: String },
    /// Worker -> GlobalState: Report discovered API
    ApiDiscovered { api: DiscoveredApi },
    /// Manager -> Worker: Stop exploration
    Stop { task_id: String },
}


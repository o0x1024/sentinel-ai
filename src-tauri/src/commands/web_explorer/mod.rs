//! Tauri Commands for Web Explorer
//!
//! NOTE: After ReAct refactoring, the engine is accessed through Rig Tool interface.
//! This module is kept for compatibility but most functions are disabled.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// State container for Web Explorer sessions (kept for compatibility)
pub struct WebExplorerState {
    /// Active engine sessions (kept for compatibility, currently unused)
    pub sessions: Arc<RwLock<HashMap<String, WebExplorerSession>>>,
}

/// A single Web Explorer session (kept for compatibility)
pub struct WebExplorerSession {
    pub session_id: String,
    pub target_url: String,
    pub has_credentials: bool,
    pub last_username: Option<String>,
    pub is_running: bool,
    pub created_at: u64,
    pub ended_at: Option<u64>,
}

impl Default for WebExplorerState {
    fn default() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// All commands are disabled after ReAct refactoring
// The new ReAct engine is accessed through Rig Tool interface
// See: src/engines/web_explorer/tool.rs

//! Tauri Commands for Vision Explorer V2
//!
//! NOTE: After ReAct refactoring, the V2 engine is accessed through Rig Tool interface.
//! This module is kept for compatibility but most functions are disabled.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// State container for V2 Engine sessions (kept for compatibility)
pub struct VisionExplorerV2State {
    /// Active V2 engine sessions (kept for compatibility, currently unused)
    pub sessions: Arc<RwLock<HashMap<String, V2Session>>>,
}

/// A single V2 session (kept for compatibility)
pub struct V2Session {
    pub session_id: String,
    pub target_url: String,
    pub has_credentials: bool,
    pub last_username: Option<String>,
    pub is_running: bool,
    pub created_at: u64,
    pub ended_at: Option<u64>,
}

impl Default for VisionExplorerV2State {
    fn default() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// All V2 commands are disabled after ReAct refactoring
// The new ReAct engine is accessed through Rig Tool interface
// See: src/engines/vision_explorer_v2/tool.rs

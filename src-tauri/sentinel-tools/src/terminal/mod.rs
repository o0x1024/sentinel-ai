//! Interactive terminal module with WebSocket support
//!
//! Provides persistent terminal sessions for interactive tools like msfconsole, sqlmap, etc.

pub mod command;
pub mod server;
pub mod session;
pub mod manager;

pub use command::{normalize_command, detect_shell_prompt, WaitStrategy};
pub use server::TerminalServer;
pub use session::{TerminalSession, TerminalSessionConfig, SessionState};
pub use manager::{TerminalSessionManager, SessionInfo, ContainerInfo};

use std::sync::Arc;
use once_cell::sync::Lazy;

/// Global terminal session manager for sharing between tools and UI
pub static TERMINAL_MANAGER: Lazy<Arc<TerminalSessionManager>> =
    Lazy::new(|| Arc::new(TerminalSessionManager::new()));

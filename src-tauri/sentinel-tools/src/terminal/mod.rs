//! Interactive terminal module with WebSocket support
//!
//! Provides persistent terminal sessions for interactive tools like msfconsole, sqlmap, etc.

pub mod command;
pub mod manager;
pub mod server;
pub mod session;

pub use command::{detect_shell_prompt, normalize_command, WaitStrategy};
pub use manager::{ContainerInfo, SessionInfo, TerminalSessionManager};
pub use server::TerminalServer;
pub use session::{ExecutionMode, SessionState, TerminalSession, TerminalSessionConfig};

use once_cell::sync::Lazy;
use std::sync::Arc;

/// Global terminal session manager for sharing between tools and UI
pub static TERMINAL_MANAGER: Lazy<Arc<TerminalSessionManager>> =
    Lazy::new(|| Arc::new(TerminalSessionManager::new()));

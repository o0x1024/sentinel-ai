//! Interactive terminal module with WebSocket support
//!
//! Provides persistent terminal sessions for interactive tools like msfconsole, sqlmap, etc.

pub mod command;
pub mod input;
pub mod manager;
pub mod server;
pub mod session;
pub mod unified_exec_tool;

pub use command::{detect_shell_prompt, is_shell_prompt_line, normalize_command, WaitStrategy};
pub use input::decode_transport_html_entities;
pub use manager::{ContainerInfo, SessionInfo, TerminalSessionManager};
use once_cell::sync::Lazy;
pub use server::TerminalServer;
pub use session::{
    default_shell_for_execution_mode, ExecutionMode, SessionState, TerminalSession,
    TerminalSessionConfig,
};
use std::sync::Arc;

/// Global terminal session manager for sharing between tools and UI
pub static TERMINAL_MANAGER: Lazy<Arc<TerminalSessionManager>> =
    Lazy::new(|| Arc::new(TerminalSessionManager::new()));

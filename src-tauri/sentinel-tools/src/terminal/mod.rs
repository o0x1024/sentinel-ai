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
pub use server::TerminalServer;
pub use session::{ExecutionMode, SessionState, TerminalSession, TerminalSessionConfig};
pub use unified_exec_tool::{EXEC_COMMAND_TOOL_NAME, WRITE_STDIN_TOOL_NAME};

use once_cell::sync::Lazy;
use std::sync::Arc;

/// Global terminal session manager for sharing between tools and UI
pub static TERMINAL_MANAGER: Lazy<Arc<TerminalSessionManager>> =
    Lazy::new(|| Arc::new(TerminalSessionManager::new()));

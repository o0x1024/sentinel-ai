//! Interactive terminal module with WebSocket support
//!
//! Provides persistent terminal sessions for interactive tools like msfconsole, sqlmap, etc.

pub mod server;
pub mod session;
pub mod manager;

pub use server::TerminalServer;
pub use session::{TerminalSession, TerminalSessionConfig};
pub use manager::TerminalSessionManager;

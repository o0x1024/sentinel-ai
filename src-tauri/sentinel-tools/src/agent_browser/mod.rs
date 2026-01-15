//! Agent Browser Module
//!
//! Provides browser automation capabilities using agent-browser daemon.
//! This module exposes browser operations as tools for AI assistant.

pub mod client;
pub mod daemon;
pub mod service;
pub mod setup;
pub mod types;

pub use daemon::{ensure_daemon, stop_daemon, stop_all_daemons};
pub use service::{get_browser_service, AgentBrowserService, BROWSER_SERVICE};
pub use setup::{check_playwright_installed, install_playwright_browsers, setup_environment};
pub use types::*;

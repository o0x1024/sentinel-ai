//! Test Explorer V1 - Text-based web exploration engine
//!
//! A hybrid architecture combining simple tool calling with optional agent planning.
//! Focused on API discovery and capture through network monitoring.

pub mod types;
pub mod driver;
pub mod network;
pub mod tools;
pub mod planner;
pub mod engine;

// Re-export key types
pub use types::{
    TestExplorerV1Config, PageState, Action, ActionType, ApiRequest, 
    InteractiveElement, ExecutionStep, ExecutionResult
};
pub use driver::BrowserDriver;
pub use network::NetworkListener;
pub use planner::TaskPlanner;
pub use engine::TestExplorerV1Engine;


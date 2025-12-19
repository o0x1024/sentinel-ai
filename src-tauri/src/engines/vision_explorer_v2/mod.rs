pub mod blackboard;
pub mod brain;
pub mod core;
pub mod driver;
pub mod emitter;
pub mod engine;
pub mod graph;
pub mod perception;
pub mod persistence;
pub mod safety;
pub mod tool;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export key items
pub use blackboard::Blackboard;
pub use brain::{AuthAgent, NavigationPatternSolver, PlannerAgent};
pub use core::{Agent, Event, LoginField, PerceptionEngine};
pub use driver::{BrowserDriver, NavigatorAgent};
pub use emitter::V2MessageEmitter;
pub use engine::V2Engine;
pub use graph::ExplorationGraph;
pub use perception::{StructuralAnalyst, VisualAnalyst};
pub use persistence::{ExplorationSnapshot, PersistenceManager};
pub use safety::{SafetyLayer, SafetyPolicy};
pub use tool::VisionExplorerV2Tool;
pub use types::VisionExplorerV2Config;

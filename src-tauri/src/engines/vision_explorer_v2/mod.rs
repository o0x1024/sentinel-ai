pub mod agent_framework;
pub mod blackboard;
pub mod brain;
pub mod core;
pub mod driver;
pub mod emitter;
pub mod engine;
pub mod error_recovery;
pub mod event_bus;
pub mod exploration_strategy;
pub mod graph;
pub mod login_state_machine;
pub mod perception;
pub mod perception_engine;
pub mod persistence;
pub mod safety;
pub mod tool;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export key items
pub use agent_framework::{Agent, AgentLifecycleManager, AgentMetadata, AgentStatus};
pub use blackboard::Blackboard;
pub use brain::{AuthAgent, NavigationPatternSolver, PlannerAgent};
pub use core::{Event, LoginField};
pub use driver::{BrowserDriver, NavigatorAgent};
pub use emitter::V2MessageEmitter;
pub use engine::V2Engine;
pub use error_recovery::{ErrorRecoveryContext, ErrorRecoveryPolicy, FallbackStrategy};
pub use event_bus::EventBus;
pub use exploration_strategy::{create_strategy, ExplorationStrategy, StrategyConfig};
pub use graph::ExplorationGraph;
pub use login_state_machine::{LoginState, LoginStateMachine};
pub use perception::VisualAnalyst;
pub use core::{PageContext, PerceptionEngine, PerceptionResult};
pub use persistence::{ExplorationSnapshot, PersistenceManager};
pub use safety::{SafetyLayer, SafetyPolicy};
pub use tool::VisionExplorerV2Tool;
pub use types::VisionExplorerV2Config;

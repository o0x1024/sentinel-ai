pub mod types;
pub mod planner;
pub mod task_fetcher;
pub mod executor;
pub mod joiner;
pub mod engine_adapter;
pub mod memory_integration;
pub mod message_emitter;

// Re-export main types and structures
pub use engine_adapter::LlmCompilerEngine;
pub use types::*;
pub use planner::LlmCompilerPlanner;
pub use task_fetcher::TaskFetchingUnit;
pub use executor::ParallelExecutorPool;
pub use joiner::IntelligentJoiner;
pub use memory_integration::{LlmCompilerMemoryIntegration, LlmCompilerMemoryConfig, LlmCompilerContextSummarizer};
pub use message_emitter::{LlmCompilerMessageEmitter, LlmCompilerMessageEmitterBuilder};
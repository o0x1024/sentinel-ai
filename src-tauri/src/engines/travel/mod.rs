//! Travel架构模块
//!
//! 基于OODA(Observe-Orient-Decide-Act)循环的智能安全测试Agent架构
//! 
//! ## 核心特性
//! - 智能任务复杂度分析(规则+LLM混合)
//! - OODA四阶段循环执行
//! - 多层安全护栏机制
//! - 威胁情报集成(RAG+工具)
//! - 智能引擎调度(工具/ReAct/其他)
//! - 错误回退策略

pub mod types;
pub mod complexity_analyzer;
pub mod guardrails;
pub mod threat_intel;
pub mod ooda_executor;
pub mod engine_dispatcher;
pub mod engine_adapter;
pub mod react_executor;
pub mod memory_integration;

#[cfg(test)]
mod tests;

// 重新导出核心类型
pub use types::*;
pub use complexity_analyzer::ComplexityAnalyzer;
pub use guardrails::GuardrailManager;
pub use threat_intel::ThreatIntelManager;
pub use ooda_executor::OodaExecutor;
pub use engine_dispatcher::EngineDispatcher;
pub use engine_adapter::TravelEngine;
pub use react_executor::TravelReactExecutor;
pub use memory_integration::TravelMemoryIntegration;


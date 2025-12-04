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
//!
//! ## Token优化特性 (v2.0)
//! - 精简DAG模式: 一次LLM调用生成完整计划
//! - 并行执行: 独立任务并行执行
//! - 上下文压缩: 智能压缩历史和结果
//! - 规划缓存: 相似任务复用计划
//! - 资源追踪: 自动清理浏览器/代理等资源

pub mod types;
pub mod complexity_analyzer;
pub mod guardrails;
pub mod threat_intel;
pub mod ooda_executor;
pub mod engine_dispatcher;
pub mod engine_adapter;
pub mod react_executor;
pub mod memory_integration;
pub mod message_emitter;

// Token优化模块 (v2.0)
pub mod dag_planner;
pub mod parallel_executor;
pub mod context_manager;
pub mod resource_integration;

// Vision Explorer 集成
pub mod vision_integration;

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

// LLM 客户端导出（TravelLlmClient 在 message_emitter 中）
pub use message_emitter::TravelLlmClient;

// 从 sentinel_llm 重新导出
pub use sentinel_llm::{LlmConfig, LlmClient, create_llm_config, create_client};

// Token优化组件导出
pub use dag_planner::DagPlanner;
pub use parallel_executor::ParallelExecutor;
pub use context_manager::ContextManager;
pub use resource_integration::ResourceTracker;

// Vision Explorer 集成导出
pub use vision_integration::{
    VisionIntegration, VisionIntegrationConfig,
    VisionExplorerToolAdapter, ObservePhaseEnhancer,
    ReconEnhancementResult, ApiEndpointInfo, AttackSurface,
};


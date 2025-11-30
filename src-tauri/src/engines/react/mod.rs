//! ReAct 架构模块
//!
//! 实现 Reasoning + Acting 的循环执行引擎，核心组件：
//! - types: 核心数据结构
//! - executor: 循环执行器
//! - parser: Action 指令解析器（JSON + 自然语言兜底）
//! - engine_adapter: 引擎适配器（对外接口）
//! - memory_integration: Memory 系统集成（经验学习、缓存）
//! - message_emitter: 专用消息发送器（结构化步骤消息）
//! - llm_client: 专用 LLM 客户端（流式调用）

pub mod types;
pub mod executor;
pub mod parser;
pub mod engine_adapter;
pub mod memory_integration;
pub mod message_emitter;
pub mod llm_client;

// 重新导出核心类型
pub use types::*;
pub use executor::{ReactExecutor, ReactExecutorConfig};
pub use parser::ActionParser;
pub use engine_adapter::ReactEngine;
pub use memory_integration::{ReactMemoryIntegration, ReactMemoryConfig, ContextSummarizer};
pub use message_emitter::{ReactMessageEmitter, ReactExecutionStats};
pub use llm_client::{ReactLlmClient, LlmConfig};

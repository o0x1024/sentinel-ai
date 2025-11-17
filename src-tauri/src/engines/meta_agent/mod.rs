//! Meta Agent 架构模块
//!
//! 元 Agent 架构，使用 ReAct 作为主控制器，协调调用其他三大架构：
//! - Plan-and-Execute: 适合需要动态重新规划的复杂任务
//! - ReWOO: 适合可以预先规划的独立任务
//! - LLMCompiler: 适合可以并行执行的任务
//!
//! ## 设计理念
//! 
//! ReAct Agent 作为"大脑"，可以：
//! 1. 分析用户任务的特点
//! 2. 决定使用哪种架构来执行子任务
//! 3. 将其他架构作为"高级工具"调用
//! 4. 根据执行结果动态调整策略
//!
//! ## 使用场景
//!
//! - 安全渗透测试：需要根据扫描结果动态调整测试策略
//! - 复杂多步骤任务：某些步骤可以并行，某些步骤需要顺序执行
//! - 混合任务：结合信息收集、分析、执行等多种类型

pub mod engine_adapter;
pub mod types;
pub mod tools;

pub use engine_adapter::MetaAgentEngine;
pub use types::*;


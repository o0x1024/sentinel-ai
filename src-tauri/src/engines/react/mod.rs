//! ReAct 架构模块
//!
//! 实现 Reasoning + Acting 的循环执行引擎，核心组件：
//! - types: 核心数据结构
//! - executor: 循环执行器
//! - parser: Action 指令解析器（JSON + 自然语言兜底）
//! - engine_adapter: 引擎适配器（对外接口）

pub mod types;
pub mod executor;
pub mod parser;
pub mod engine_adapter;

// 重新导出核心类型
pub use types::*;
pub use executor::{ReactExecutor, ReactExecutorConfig};
pub use parser::ActionParser;
pub use engine_adapter::ReactEngine;

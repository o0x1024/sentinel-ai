//! 消息块类型定义

use serde::{Deserialize, Serialize};

/// 消息块类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChunkType {
    /// 主要内容
    Content,
    /// AI思考过程
    Thinking,
    /// 工具执行结果
    ToolResult,
    /// 计划信息
    PlanInfo,
    /// 错误信息
    Error,
    /// 元数据信息
    Meta,
}


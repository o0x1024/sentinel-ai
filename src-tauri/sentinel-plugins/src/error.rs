//! 插件系统错误类型

use std::fmt;

/// 插件错误类型
#[derive(Debug)]
pub enum PluginError {
    /// 插件加载失败
    Load(String),
    /// 插件执行失败
    Execution(String),
    /// 插件未找到
    NotFound(String),
    /// 插件已存在
    AlreadyExists(String),
    /// IO 错误
    Io(std::io::Error),
    /// JSON 序列化/反序列化错误
    Json(serde_json::Error),
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginError::Load(msg) => write!(f, "Failed to load plugin: {}", msg),
            PluginError::Execution(msg) => write!(f, "Plugin execution failed: {}", msg),
            PluginError::NotFound(id) => write!(f, "Plugin not found: {}", id),
            PluginError::AlreadyExists(id) => write!(f, "Plugin already exists: {}", id),
            PluginError::Io(err) => write!(f, "IO error: {}", err),
            PluginError::Json(err) => write!(f, "JSON error: {}", err),
        }
    }
}

impl std::error::Error for PluginError {}

impl From<std::io::Error> for PluginError {
    fn from(err: std::io::Error) -> Self {
        PluginError::Io(err)
    }
}

impl From<serde_json::Error> for PluginError {
    fn from(err: serde_json::Error) -> Self {
        PluginError::Json(err)
    }
}

/// 插件系统 Result 类型
pub type Result<T> = std::result::Result<T, PluginError>;

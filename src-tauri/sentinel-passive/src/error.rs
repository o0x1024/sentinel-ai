use thiserror::Error;

/// 被动扫描系统错误类型
#[derive(Error, Debug)]
pub enum PassiveError {
    #[error("代理错误: {0}")]
    Proxy(String),

    #[error("证书错误: {0}")]
    Certificate(String),

    #[error("插件错误: {0}")]
    Plugin(String),

    #[error("扫描错误: {0}")]
    Scanner(String),

    #[error("数据库错误: {0}")]
    Database(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("其他错误: {0}")]
    Other(#[from] anyhow::Error),
}

/// 被动扫描系统 Result 类型
pub type Result<T> = std::result::Result<T, PassiveError>;

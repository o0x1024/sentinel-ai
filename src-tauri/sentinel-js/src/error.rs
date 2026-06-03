use thiserror::Error;

#[derive(Error, Debug)]
pub enum JsError {
    #[error("JavaScript execution error: {0}")]
    Execution(String),

    #[error("Module load error: {0}")]
    ModuleLoad(String),

    #[error("Type conversion error: {0}")]
    Conversion(String),

    #[error("Sandbox violation: {0}")]
    Sandbox(String),

    #[error("Timeout: execution exceeded {0}ms")]
    Timeout(u64),

    #[error("Memory limit exceeded: {0} bytes")]
    MemoryLimit(usize),

    #[error("Operation limit exceeded: {0} ops")]
    OperationLimit(u64),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<rquickjs::Error> for JsError {
    fn from(e: rquickjs::Error) -> Self {
        JsError::Execution(e.to_string())
    }
}

impl From<anyhow::Error> for JsError {
    fn from(e: anyhow::Error) -> Self {
        JsError::Internal(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, JsError>;

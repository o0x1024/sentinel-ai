//! Core error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SentinelError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Service error: {0}")]
    Service(String),
    
    #[error("Tool execution error: {0}")]
    ToolExecution(String),
    
    #[error("Engine error: {0}")]
    Engine(String),
    
    #[error("Agent error: {0}")]
    Agent(String),
    
    #[error("RAG error: {0}")]
    Rag(String),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, SentinelError>;

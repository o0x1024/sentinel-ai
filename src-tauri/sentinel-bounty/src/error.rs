//! Error types for the bounty module

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BountyError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Program not found: {0}")]
    ProgramNotFound(String),

    #[error("Finding not found: {0}")]
    FindingNotFound(String),

    #[error("Submission not found: {0}")]
    SubmissionNotFound(String),

    #[error("Invalid scope pattern: {0}")]
    InvalidScopePattern(String),

    #[error("Duplicate finding: {0}")]
    DuplicateFinding(String),

    #[error("Out of scope: {0}")]
    OutOfScope(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, BountyError>;

impl From<serde_json::Error> for BountyError {
    fn from(err: serde_json::Error) -> Self {
        BountyError::Serialization(err.to_string())
    }
}

impl From<anyhow::Error> for BountyError {
    fn from(err: anyhow::Error) -> Self {
        BountyError::Database(err.to_string())
    }
}

impl From<String> for BountyError {
    fn from(err: String) -> Self {
        BountyError::Internal(err)
    }
}

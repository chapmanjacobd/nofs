//! Error types for nofs

use thiserror::Error;

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum NofsError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Branch error: {0}")]
    Branch(String),

    #[error("Policy error: {0}")]
    Policy(String),

    #[error("Path not found: {0}")]
    PathNotFound(String),

    #[error("No suitable branch found for operation")]
    NoSuitableBranch,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Copy/Move error: {0}")]
    CopyMove(String),

    #[error("Conflict resolution error: {0}")]
    Conflict(String),
}

pub type Result<T> = std::result::Result<T, NofsError>;

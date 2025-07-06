use thiserror::Error;

/// Main error type for Codesworth operations
#[derive(Error, Debug)]
pub enum CodesworthError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Parser error: {0}")]
    Parser(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Template error: {0}")]
    Template(#[from] tera::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("File system error: {0}")]
    FileSystem(String),

    #[error("Documentation validation failed: {0}")]
    Validation(String),

    #[error("Content hash mismatch: expected {expected}, found {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Protected region parse error: {0}")]
    ProtectedRegion(String),
}

pub type Result<T> = std::result::Result<T, CodesworthError>;
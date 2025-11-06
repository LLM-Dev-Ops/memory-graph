//! Error types for LLM-Memory-Graph operations

/// Result type alias for LLM-Memory-Graph operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types that can occur during graph operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Storage backend error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Node not found error
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    /// Session not found error
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Invalid node type error
    #[error("Invalid node type: expected {expected}, got {actual}")]
    InvalidNodeType {
        /// Expected node type
        expected: String,
        /// Actual node type encountered
        actual: String,
    },

    /// Schema validation error
    #[error("Schema validation error: {0}")]
    ValidationError(String),

    /// Graph traversal error
    #[error("Graph traversal error: {0}")]
    TraversalError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl From<sled::Error> for Error {
    fn from(err: sled::Error) -> Self {
        Error::Storage(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<rmp_serde::encode::Error> for Error {
    fn from(err: rmp_serde::encode::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<rmp_serde::decode::Error> for Error {
    fn from(err: rmp_serde::decode::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

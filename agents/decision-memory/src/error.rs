//! Error types for Decision Memory Agent
//!
//! Error handling following the agentics-contracts AgentError specification.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Agent error codes as defined in agentics-contracts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentErrorCode {
    /// Input validation failed
    ValidationError,
    /// Cannot connect to ruvector-service
    RuvectorConnectionError,
    /// Failed to write to ruvector-service
    RuvectorWriteError,
    /// Internal agent error
    InternalError,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Input hash mismatch
    InputHashMismatch,
    /// Session not found
    SessionNotFound,
    /// Decision not found
    DecisionNotFound,
}

impl fmt::Display for AgentErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValidationError => write!(f, "VALIDATION_ERROR"),
            Self::RuvectorConnectionError => write!(f, "RUVECTOR_CONNECTION_ERROR"),
            Self::RuvectorWriteError => write!(f, "RUVECTOR_WRITE_ERROR"),
            Self::InternalError => write!(f, "INTERNAL_ERROR"),
            Self::RateLimitExceeded => write!(f, "RATE_LIMIT_EXCEEDED"),
            Self::InputHashMismatch => write!(f, "INPUT_HASH_MISMATCH"),
            Self::SessionNotFound => write!(f, "SESSION_NOT_FOUND"),
            Self::DecisionNotFound => write!(f, "DECISION_NOT_FOUND"),
        }
    }
}

/// Agent error structure matching agentics-contracts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentError {
    /// Error code
    pub error_code: AgentErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Execution reference for this error
    pub execution_ref: Uuid,
    /// When the error occurred
    pub timestamp: DateTime<Utc>,
}

impl AgentError {
    /// Create a new agent error
    pub fn new(code: AgentErrorCode, message: impl Into<String>) -> Self {
        Self {
            error_code: code,
            message: message.into(),
            details: None,
            execution_ref: Uuid::new_v4(),
            timestamp: Utc::now(),
        }
    }

    /// Create a new agent error with details
    pub fn with_details(
        code: AgentErrorCode,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            error_code: code,
            message: message.into(),
            details: Some(details),
            execution_ref: Uuid::new_v4(),
            timestamp: Utc::now(),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(AgentErrorCode::ValidationError, message)
    }

    /// Create a ruvector connection error
    pub fn ruvector_connection(message: impl Into<String>) -> Self {
        Self::new(AgentErrorCode::RuvectorConnectionError, message)
    }

    /// Create a ruvector write error
    pub fn ruvector_write(message: impl Into<String>) -> Self {
        Self::new(AgentErrorCode::RuvectorWriteError, message)
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(AgentErrorCode::InternalError, message)
    }

    /// Create a rate limit error
    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::new(AgentErrorCode::RateLimitExceeded, message)
    }
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.error_code, self.message)
    }
}

impl std::error::Error for AgentError {}

/// Result type for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

/// Internal error type for detailed error handling
#[derive(Debug, thiserror::Error)]
pub enum InternalError {
    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// RuVector service error
    #[error("RuVector service error: {0}")]
    RuVector(String),

    /// Graph error
    #[error("Graph error: {0}")]
    Graph(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<InternalError> for AgentError {
    fn from(err: InternalError) -> Self {
        match err {
            InternalError::Validation(msg) => AgentError::validation(msg),
            InternalError::Serialization(e) => {
                AgentError::internal(format!("Serialization error: {}", e))
            }
            InternalError::Http(e) => {
                if e.is_connect() {
                    AgentError::ruvector_connection(format!("Connection error: {}", e))
                } else if e.is_timeout() {
                    AgentError::ruvector_connection(format!("Timeout: {}", e))
                } else {
                    AgentError::internal(format!("HTTP error: {}", e))
                }
            }
            InternalError::Io(e) => AgentError::internal(format!("IO error: {}", e)),
            InternalError::RuVector(msg) => AgentError::ruvector_write(msg),
            InternalError::Graph(msg) => AgentError::internal(format!("Graph error: {}", msg)),
            InternalError::Config(msg) => AgentError::internal(format!("Config error: {}", msg)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_error_creation() {
        let err = AgentError::validation("Invalid input");
        assert_eq!(err.error_code, AgentErrorCode::ValidationError);
        assert_eq!(err.message, "Invalid input");
    }

    #[test]
    fn test_agent_error_serialization() {
        let err = AgentError::new(AgentErrorCode::InternalError, "Something went wrong");
        let json = serde_json::to_string(&err).unwrap();
        let deserialized: AgentError = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.error_code, AgentErrorCode::InternalError);
    }

    #[test]
    fn test_error_code_display() {
        assert_eq!(
            AgentErrorCode::ValidationError.to_string(),
            "VALIDATION_ERROR"
        );
        assert_eq!(
            AgentErrorCode::RuvectorConnectionError.to_string(),
            "RUVECTOR_CONNECTION_ERROR"
        );
    }
}

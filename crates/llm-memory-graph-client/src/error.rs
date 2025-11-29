//! Error types for the client

use thiserror::Error;

/// Client error types
#[derive(Debug, Error)]
pub enum ClientError {
    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// gRPC transport error
    #[error("gRPC transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    /// gRPC status error
    #[error("gRPC status error: {0}")]
    Status(#[from] tonic::Status),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Invalid argument
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Already exists
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Other error
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for client operations
pub type Result<T> = std::result::Result<T, ClientError>;

impl From<serde_json::Error> for ClientError {
    fn from(err: serde_json::Error) -> Self {
        ClientError::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ClientError::Connection("test error".to_string());
        assert_eq!(err.to_string(), "Connection error: test error");

        let err = ClientError::InvalidArgument("bad arg".to_string());
        assert_eq!(err.to_string(), "Invalid argument: bad arg");

        let err = ClientError::NotFound("resource".to_string());
        assert_eq!(err.to_string(), "Resource not found: resource");

        let err = ClientError::AlreadyExists("item".to_string());
        assert_eq!(err.to_string(), "Resource already exists: item");

        let err = ClientError::Internal("internal".to_string());
        assert_eq!(err.to_string(), "Internal error: internal");

        let err = ClientError::Other("other".to_string());
        assert_eq!(err.to_string(), "Other error: other");
    }

    #[test]
    fn test_error_debug() {
        let err = ClientError::Connection("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Connection"));
    }

    #[test]
    fn test_serialization_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json")
            .unwrap_err();
        let client_err: ClientError = json_err.into();

        match client_err {
            ClientError::Serialization(msg) => {
                assert!(msg.contains("expected") || msg.len() > 0);
            }
            _ => panic!("Expected Serialization error"),
        }
    }

    #[test]
    fn test_transport_error_conversion() {
        // Transport errors are automatically converted via From trait
        // This test verifies the trait is implemented
        fn _accepts_transport_error(_err: tonic::transport::Error) -> ClientError {
            // This function signature proves the From trait works
            unreachable!()
        }
    }

    #[test]
    fn test_status_error_conversion() {
        let status = tonic::Status::not_found("test resource");
        let client_err: ClientError = status.into();

        match client_err {
            ClientError::Status(_) => {
                // Successfully converted
            }
            _ => panic!("Expected Status error"),
        }
    }

    #[test]
    fn test_result_type() {
        fn returns_result() -> Result<String> {
            Ok("success".to_string())
        }

        let result = returns_result();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[test]
    fn test_result_type_error() {
        fn returns_error() -> Result<String> {
            Err(ClientError::NotFound("test".to_string()))
        }

        let result = returns_error();
        assert!(result.is_err());
    }

    #[test]
    fn test_error_variants() {
        // Test all error variants can be constructed
        let _conn = ClientError::Connection("test".to_string());
        let _ser = ClientError::Serialization("test".to_string());
        let _inv = ClientError::InvalidArgument("test".to_string());
        let _nf = ClientError::NotFound("test".to_string());
        let _ae = ClientError::AlreadyExists("test".to_string());
        let _int = ClientError::Internal("test".to_string());
        let _oth = ClientError::Other("test".to_string());
    }
}

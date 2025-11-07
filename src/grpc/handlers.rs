//! Request handlers for gRPC operations
//!
//! This module contains helper functions for handling specific types of
//! gRPC requests, including validation, transformation, and error handling.

use crate::error::Result;
use crate::grpc::proto;
use tonic::Status;

/// Validate a create session request
pub fn validate_create_session_request(
    _request: &proto::CreateSessionRequest,
) -> Result<(), Status> {
    // Add validation logic as needed
    Ok(())
}

/// Validate a prompt request
pub fn validate_add_prompt_request(request: &proto::AddPromptRequest) -> Result<(), Status> {
    if request.session_id.is_empty() {
        return Err(Status::invalid_argument("session_id cannot be empty"));
    }
    if request.content.is_empty() {
        return Err(Status::invalid_argument("content cannot be empty"));
    }
    if request.content.len() > 1_000_000 {
        return Err(Status::invalid_argument(
            "content exceeds maximum length of 1MB",
        ));
    }
    Ok(())
}

/// Validate a response request
pub fn validate_add_response_request(request: &proto::AddResponseRequest) -> Result<(), Status> {
    if request.prompt_id.is_empty() {
        return Err(Status::invalid_argument("prompt_id cannot be empty"));
    }
    if request.content.is_empty() {
        return Err(Status::invalid_argument("content cannot be empty"));
    }
    if request.token_usage.is_none() {
        return Err(Status::invalid_argument("token_usage is required"));
    }
    Ok(())
}

/// Validate a query request
pub fn validate_query_request(request: &proto::QueryRequest) -> Result<(), Status> {
    if let Some(limit) = request.limit.into() {
        if limit > 10000 {
            return Err(Status::invalid_argument(
                "limit cannot exceed 10000",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_add_prompt_request() {
        let valid_req = proto::AddPromptRequest {
            session_id: "test-session".to_string(),
            content: "Test prompt".to_string(),
            metadata: None,
        };
        assert!(validate_add_prompt_request(&valid_req).is_ok());

        let empty_session = proto::AddPromptRequest {
            session_id: "".to_string(),
            content: "Test".to_string(),
            metadata: None,
        };
        assert!(validate_add_prompt_request(&empty_session).is_err());

        let empty_content = proto::AddPromptRequest {
            session_id: "test".to_string(),
            content: "".to_string(),
            metadata: None,
        };
        assert!(validate_add_prompt_request(&empty_content).is_err());
    }
}

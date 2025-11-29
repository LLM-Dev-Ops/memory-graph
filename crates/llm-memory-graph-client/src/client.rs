//! Client implementation for the LLM Memory Graph service

use crate::error::{ClientError, Result};
use std::collections::HashMap;
use tonic::transport::Channel;

/// Generated protobuf definitions for the Memory Graph service
#[allow(missing_docs)]
pub mod proto {
    tonic::include_proto!("llm.memory.graph.v1");
}

use proto::memory_graph_service_client::MemoryGraphServiceClient;

/// High-level client for the LLM Memory Graph service
#[derive(Clone)]
pub struct MemoryGraphClient {
    client: MemoryGraphServiceClient<Channel>,
}

impl MemoryGraphClient {
    /// Connect to the Memory Graph service
    pub async fn connect<D>(addr: D) -> Result<Self>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: std::error::Error + Send + Sync + 'static,
    {
        let endpoint = addr
            .try_into()
            .map_err(|e| ClientError::Connection(format!("{:?}", e)))?;

        let client = MemoryGraphServiceClient::connect(endpoint).await?;

        Ok(Self { client })
    }

    /// Create a new session
    pub async fn create_session(&self, metadata: HashMap<String, String>) -> Result<String> {
        let request = proto::CreateSessionRequest { metadata };
        let response = self.client.clone().create_session(request).await?;
        Ok(response.into_inner().id)
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: String) -> Result<proto::Session> {
        let request = proto::GetSessionRequest {
            session_id,
        };
        let response = self.client.clone().get_session(request).await?;
        Ok(response.into_inner())
    }

    /// Delete a session
    pub async fn delete_session(&self, session_id: String) -> Result<()> {
        let request = proto::DeleteSessionRequest { session_id };
        self.client.clone().delete_session(request).await?;
        Ok(())
    }

    /// List sessions
    pub async fn list_sessions(&self, limit: i32, offset: i32) -> Result<Vec<proto::Session>> {
        let request = proto::ListSessionsRequest { limit, offset };
        let response = self.client.clone().list_sessions(request).await?;
        Ok(response.into_inner().sessions)
    }

    /// Add a prompt
    pub async fn add_prompt(
        &self,
        session_id: String,
        content: String,
        metadata: Option<proto::PromptMetadata>,
    ) -> Result<proto::PromptNode> {
        let request = proto::AddPromptRequest {
            session_id,
            content,
            metadata,
        };
        let response = self.client.clone().add_prompt(request).await?;
        Ok(response.into_inner())
    }

    /// Add a response
    pub async fn add_response(
        &self,
        prompt_id: String,
        content: String,
        token_usage: Option<proto::TokenUsage>,
        metadata: Option<proto::ResponseMetadata>,
    ) -> Result<proto::ResponseNode> {
        let request = proto::AddResponseRequest {
            prompt_id,
            content,
            token_usage,
            metadata,
        };
        let response = self.client.clone().add_response(request).await?;
        Ok(response.into_inner())
    }

    /// Query nodes
    pub async fn query(
        &self,
        session_id: Option<String>,
        node_type: Option<i32>,
        limit: i32,
        offset: i32,
    ) -> Result<proto::QueryResponse> {
        let request = proto::QueryRequest {
            session_id,
            node_type,
            limit,
            offset,
            filters: HashMap::new(),
            after: None,
            before: None,
        };
        let response = self.client.clone().query(request).await?;
        Ok(response.into_inner())
    }

    /// Get service health
    pub async fn health(&self) -> Result<proto::HealthResponse> {
        let request = tonic::Request::new(());
        let response = self.client.clone().health(request).await?;
        Ok(response.into_inner())
    }

    /// Get service metrics
    pub async fn metrics(&self) -> Result<proto::MetricsResponse> {
        let request = tonic::Request::new(());
        let response = self.client.clone().get_metrics(request).await?;
        Ok(response.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_construction() {
        // Verify types compile
        let _client: Option<MemoryGraphClient> = None;
    }

    #[test]
    fn test_client_clone() {
        // Verify client is clonable
        let client: Option<MemoryGraphClient> = None;
        let _cloned = client.clone();
    }

    #[tokio::test]
    async fn test_connect_invalid_address() {
        // Test connection to invalid address
        let result = MemoryGraphClient::connect("invalid://address").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connect_unreachable() {
        // Test connection to unreachable address
        let result = MemoryGraphClient::connect("http://localhost:99999").await;
        assert!(result.is_err());
    }
}

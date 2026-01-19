//! RuVector Service Client
//!
//! This module provides the client for persisting DecisionEvents to ruvector-service.
//!
//! # Architecture
//! - LLM-Memory-Graph is the LOGICAL memory layer
//! - PHYSICAL persistence is handled exclusively by ruvector-service
//! - ruvector-service is backed by Google SQL (Postgres)
//! - This agent NEVER connects directly to Google SQL
//! - This agent NEVER executes SQL
//! - All persistence occurs via ruvector-service client calls only

use crate::contracts::DecisionEvent;
use crate::error::{AgentError, AgentResult, InternalError};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// RuVector service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuVectorConfig {
    /// Base URL for the ruvector-service
    pub base_url: String,
    /// API key for authentication
    pub api_key: String,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Retry delay in milliseconds
    #[serde(default = "default_retry_delay")]
    pub retry_delay_ms: u64,
}

fn default_timeout() -> u64 {
    30
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_delay() -> u64 {
    100
}

impl Default for RuVectorConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("RUVECTOR_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            api_key: std::env::var("RUVECTOR_API_KEY").unwrap_or_default(),
            timeout_secs: default_timeout(),
            max_retries: default_max_retries(),
            retry_delay_ms: default_retry_delay(),
        }
    }
}

/// Response from ruvector-service for store operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreResponse {
    /// Storage reference ID
    pub ref_id: String,
    /// Operation success status
    pub success: bool,
    /// Storage location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// Response from ruvector-service for retrieve operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieveResponse<T> {
    /// Retrieved data
    pub data: T,
    /// Reference ID
    pub ref_id: String,
    /// Retrieved at timestamp
    pub retrieved_at: String,
}

/// Query parameters for decision event retrieval
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionEventQuery {
    /// Session ID filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<Uuid>,
    /// Decision ID filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_id: Option<Uuid>,
    /// Start timestamp filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_timestamp: Option<String>,
    /// End timestamp filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_timestamp: Option<String>,
    /// Limit results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
}

/// Trait for RuVector service operations
#[async_trait]
pub trait RuVectorService: Send + Sync {
    /// Store a decision event
    async fn store_decision_event(&self, event: &DecisionEvent) -> AgentResult<StoreResponse>;

    /// Retrieve a decision event by execution reference
    async fn retrieve_decision_event(
        &self,
        execution_ref: &Uuid,
    ) -> AgentResult<RetrieveResponse<DecisionEvent>>;

    /// Query decision events
    async fn query_decision_events(
        &self,
        query: &DecisionEventQuery,
    ) -> AgentResult<Vec<DecisionEvent>>;

    /// Store artifact content
    async fn store_artifact_content(
        &self,
        artifact_id: &Uuid,
        content: &[u8],
    ) -> AgentResult<StoreResponse>;

    /// Retrieve artifact content
    async fn retrieve_artifact_content(&self, content_ref: &str) -> AgentResult<Vec<u8>>;
}

/// RuVector service client implementation
#[derive(Debug, Clone)]
pub struct RuVectorClient {
    config: RuVectorConfig,
    client: Client,
}

impl RuVectorClient {
    /// Create a new RuVector client
    pub fn new(config: RuVectorConfig) -> Result<Self, InternalError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| InternalError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// Create a client with default configuration
    pub fn from_env() -> Result<Self, InternalError> {
        Self::new(RuVectorConfig::default())
    }

    /// Execute a request with retry logic
    async fn execute_with_retry<T, F, Fut>(&self, operation: F) -> AgentResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, InternalError>>,
    {
        let mut attempt = 0;
        let mut delay = Duration::from_millis(self.config.retry_delay_ms);

        loop {
            attempt += 1;
            match operation().await {
                Ok(value) => return Ok(value),
                Err(err) => {
                    let should_retry = matches!(
                        &err,
                        InternalError::Http(e) if e.is_connect() || e.is_timeout()
                    );

                    if !should_retry || attempt >= self.config.max_retries {
                        return Err(err.into());
                    }

                    warn!(
                        attempt = attempt,
                        max_retries = self.config.max_retries,
                        delay_ms = delay.as_millis(),
                        "Retrying ruvector-service request"
                    );

                    tokio::time::sleep(delay).await;
                    delay = delay.saturating_mul(2); // Exponential backoff
                }
            }
        }
    }
}

#[async_trait]
impl RuVectorService for RuVectorClient {
    #[instrument(skip(self, event), fields(execution_ref = %event.execution_ref))]
    async fn store_decision_event(&self, event: &DecisionEvent) -> AgentResult<StoreResponse> {
        info!("Storing decision event to ruvector-service");

        self.execute_with_retry(|| async {
            let url = format!(
                "{}/api/v1/decision-events",
                self.config.base_url.trim_end_matches('/')
            );

            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .header("Content-Type", "application/json")
                .header("X-Agent-Id", crate::constants::AGENT_ID)
                .header("X-Agent-Version", crate::constants::AGENT_VERSION)
                .json(event)
                .send()
                .await?;

            let status = response.status();
            if status.is_success() {
                let result: StoreResponse = response.json().await?;
                debug!(ref_id = %result.ref_id, "Decision event stored successfully");
                Ok(result)
            } else {
                let error_text = response.text().await.unwrap_or_default();
                error!(status = %status, error = %error_text, "Failed to store decision event");
                Err(InternalError::RuVector(format!(
                    "Store failed with status {}: {}",
                    status, error_text
                )))
            }
        })
        .await
    }

    #[instrument(skip(self), fields(execution_ref = %execution_ref))]
    async fn retrieve_decision_event(
        &self,
        execution_ref: &Uuid,
    ) -> AgentResult<RetrieveResponse<DecisionEvent>> {
        info!("Retrieving decision event from ruvector-service");

        self.execute_with_retry(|| async {
            let url = format!(
                "{}/api/v1/decision-events/{}",
                self.config.base_url.trim_end_matches('/'),
                execution_ref
            );

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .header("X-Agent-Id", crate::constants::AGENT_ID)
                .send()
                .await?;

            let status = response.status();
            if status.is_success() {
                let result: RetrieveResponse<DecisionEvent> = response.json().await?;
                debug!(ref_id = %result.ref_id, "Decision event retrieved successfully");
                Ok(result)
            } else if status == reqwest::StatusCode::NOT_FOUND {
                Err(InternalError::RuVector(format!(
                    "Decision event not found: {}",
                    execution_ref
                )))
            } else {
                let error_text = response.text().await.unwrap_or_default();
                Err(InternalError::RuVector(format!(
                    "Retrieve failed with status {}: {}",
                    status, error_text
                )))
            }
        })
        .await
    }

    #[instrument(skip(self, query))]
    async fn query_decision_events(
        &self,
        query: &DecisionEventQuery,
    ) -> AgentResult<Vec<DecisionEvent>> {
        info!(?query, "Querying decision events from ruvector-service");

        self.execute_with_retry(|| async {
            let url = format!(
                "{}/api/v1/decision-events/query",
                self.config.base_url.trim_end_matches('/')
            );

            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .header("Content-Type", "application/json")
                .header("X-Agent-Id", crate::constants::AGENT_ID)
                .json(query)
                .send()
                .await?;

            let status = response.status();
            if status.is_success() {
                let results: Vec<DecisionEvent> = response.json().await?;
                debug!(count = results.len(), "Decision events query completed");
                Ok(results)
            } else {
                let error_text = response.text().await.unwrap_or_default();
                Err(InternalError::RuVector(format!(
                    "Query failed with status {}: {}",
                    status, error_text
                )))
            }
        })
        .await
    }

    #[instrument(skip(self, content), fields(artifact_id = %artifact_id, content_size = content.len()))]
    async fn store_artifact_content(
        &self,
        artifact_id: &Uuid,
        content: &[u8],
    ) -> AgentResult<StoreResponse> {
        info!("Storing artifact content to ruvector-service");

        let content_clone = content.to_vec();
        self.execute_with_retry(|| async {
            let url = format!(
                "{}/api/v1/artifacts/{}",
                self.config.base_url.trim_end_matches('/'),
                artifact_id
            );

            let response = self
                .client
                .put(&url)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .header("Content-Type", "application/octet-stream")
                .header("X-Agent-Id", crate::constants::AGENT_ID)
                .body(content_clone.clone())
                .send()
                .await?;

            let status = response.status();
            if status.is_success() {
                let result: StoreResponse = response.json().await?;
                debug!(ref_id = %result.ref_id, "Artifact content stored successfully");
                Ok(result)
            } else {
                let error_text = response.text().await.unwrap_or_default();
                Err(InternalError::RuVector(format!(
                    "Store artifact failed with status {}: {}",
                    status, error_text
                )))
            }
        })
        .await
    }

    #[instrument(skip(self))]
    async fn retrieve_artifact_content(&self, content_ref: &str) -> AgentResult<Vec<u8>> {
        info!("Retrieving artifact content from ruvector-service");

        let content_ref = content_ref.to_string();
        self.execute_with_retry(|| async {
            let url = format!(
                "{}/api/v1/artifacts/content/{}",
                self.config.base_url.trim_end_matches('/'),
                content_ref
            );

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .header("X-Agent-Id", crate::constants::AGENT_ID)
                .send()
                .await?;

            let status = response.status();
            if status.is_success() {
                let content = response.bytes().await?.to_vec();
                debug!(content_size = content.len(), "Artifact content retrieved");
                Ok(content)
            } else if status == reqwest::StatusCode::NOT_FOUND {
                Err(InternalError::RuVector(format!(
                    "Artifact content not found: {}",
                    content_ref
                )))
            } else {
                let error_text = response.text().await.unwrap_or_default();
                Err(InternalError::RuVector(format!(
                    "Retrieve artifact failed with status {}: {}",
                    status, error_text
                )))
            }
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ruvector_config_default() {
        let config = RuVectorConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_ms, 100);
    }

    #[test]
    fn test_decision_event_query() {
        let query = DecisionEventQuery {
            session_id: Some(Uuid::new_v4()),
            decision_id: None,
            from_timestamp: Some("2024-01-01T00:00:00Z".to_string()),
            to_timestamp: None,
            limit: Some(100),
            offset: None,
        };

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("session_id"));
        assert!(json.contains("from_timestamp"));
        assert!(json.contains("limit"));
        assert!(!json.contains("decision_id"));
    }
}

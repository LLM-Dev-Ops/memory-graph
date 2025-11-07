//! LLM-Registry client implementation for metadata and version tracking

use super::types::{
    ModelListResponse, ModelMetadata, RegistryConfig, SessionInfo, SessionListResponse,
    SessionRegistration, SessionStatus, UsageReport, UsageStats,
};
use crate::integrations::{retry_request, IntegrationError, RetryPolicy};
use reqwest::{Client, StatusCode};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// LLM-Registry client
///
/// Provides integration with the LLM-Registry service for:
/// - Session registration and tracking
/// - Model metadata retrieval
/// - Usage statistics and monitoring
pub struct RegistryClient {
    config: RegistryConfig,
    client: Client,
    retry_policy: RetryPolicy,
}

impl RegistryClient {
    /// Create a new Registry client
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be created.
    pub fn new(config: RegistryConfig) -> Result<Self, IntegrationError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .build()
            .map_err(|e| IntegrationError::HttpError(e.to_string()))?;

        let retry_policy = RetryPolicy::new()
            .with_max_attempts(config.retry_count)
            .with_initial_delay(Duration::from_millis(100));

        Ok(Self {
            config,
            client,
            retry_policy,
        })
    }

    /// Create a client with custom retry policy
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Register a session with the registry
    ///
    /// # Errors
    /// Returns an error if the registration request fails.
    pub async fn register_session(
        &self,
        registration: SessionRegistration,
    ) -> Result<SessionInfo, IntegrationError> {
        let url = format!("{}/api/v1/sessions", self.config.base_url);

        if self.config.enable_logging {
            info!(
                "Registering session {} with model {}",
                registration.session_id, registration.model_id
            );
        }

        let operation = || async {
            let mut request = self.client.post(&url).json(&registration);

            if let Some(api_key) = &self.config.api_key {
                request = request.bearer_auth(api_key);
            }

            let response = request.send().await?;
            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                error!(
                    "Failed to register session: {} - {}",
                    status, error_body
                );
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let session_info: SessionInfo = response.json().await?;
            Ok(session_info)
        };

        let result = retry_request(&self.retry_policy, operation).await?;

        if self.config.enable_logging {
            info!("Session {} registered successfully", registration.session_id);
        }

        Ok(result)
    }

    /// Get model metadata from the registry
    ///
    /// # Errors
    /// Returns an error if the model is not found or the request fails.
    pub async fn get_model_metadata(
        &self,
        model_id: &str,
    ) -> Result<ModelMetadata, IntegrationError> {
        let url = format!("{}/api/v1/models/{}", self.config.base_url, model_id);

        if self.config.enable_logging {
            debug!("Fetching metadata for model: {}", model_id);
        }

        let operation = || async {
            let mut request = self.client.get(&url);

            if let Some(api_key) = &self.config.api_key {
                request = request.bearer_auth(api_key);
            }

            let response = request.send().await?;
            let status = response.status();

            if status == StatusCode::NOT_FOUND {
                return Err(IntegrationError::ApiError {
                    status: 404,
                    message: format!("Model not found: {}", model_id),
                });
            }

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                error!("Failed to get model metadata: {} - {}", status, error_body);
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let metadata: ModelMetadata = response.json().await?;
            Ok(metadata)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// List available models
    ///
    /// # Errors
    /// Returns an error if the request fails.
    pub async fn list_models(
        &self,
        page: Option<usize>,
        page_size: Option<usize>,
    ) -> Result<ModelListResponse, IntegrationError> {
        let mut url = format!("{}/api/v1/models", self.config.base_url);

        if let Some(page) = page {
            url.push_str(&format!("?page={}", page));
            if let Some(size) = page_size {
                url.push_str(&format!("&page_size={}", size));
            }
        } else if let Some(size) = page_size {
            url.push_str(&format!("?page_size={}", size));
        }

        let operation = || async {
            let mut request = self.client.get(&url);

            if let Some(api_key) = &self.config.api_key {
                request = request.bearer_auth(api_key);
            }

            let response = request.send().await?;
            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let models: ModelListResponse = response.json().await?;
            Ok(models)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Track token usage for a session
    ///
    /// # Errors
    /// Returns an error if the usage tracking request fails.
    pub async fn track_usage(&self, report: UsageReport) -> Result<(), IntegrationError> {
        let url = format!("{}/api/v1/usage", self.config.base_url);

        if self.config.enable_logging {
            debug!(
                "Tracking usage for session {}: {} tokens",
                report.session_id, report.total_tokens
            );
        }

        let operation = || async {
            let mut request = self.client.post(&url).json(&report);

            if let Some(api_key) = &self.config.api_key {
                request = request.bearer_auth(api_key);
            }

            let response = request.send().await?;
            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                warn!("Failed to track usage: {} - {}", status, error_body);
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            Ok(())
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Get usage statistics for a session
    ///
    /// # Errors
    /// Returns an error if the session is not found or the request fails.
    pub async fn get_session_usage(
        &self,
        session_id: &str,
    ) -> Result<UsageStats, IntegrationError> {
        let url = format!(
            "{}/api/v1/sessions/{}/usage",
            self.config.base_url, session_id
        );

        if self.config.enable_logging {
            debug!("Fetching usage stats for session: {}", session_id);
        }

        let operation = || async {
            let mut request = self.client.get(&url);

            if let Some(api_key) = &self.config.api_key {
                request = request.bearer_auth(api_key);
            }

            let response = request.send().await?;
            let status = response.status();

            if status == StatusCode::NOT_FOUND {
                return Err(IntegrationError::ApiError {
                    status: 404,
                    message: format!("Session not found: {}", session_id),
                });
            }

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let stats: UsageStats = response.json().await?;
            Ok(stats)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Get usage statistics for a model
    ///
    /// # Errors
    /// Returns an error if the model is not found or the request fails.
    pub async fn get_model_usage(
        &self,
        model_id: &str,
    ) -> Result<UsageStats, IntegrationError> {
        let url = format!(
            "{}/api/v1/models/{}/usage",
            self.config.base_url, model_id
        );

        let operation = || async {
            let mut request = self.client.get(&url);

            if let Some(api_key) = &self.config.api_key {
                request = request.bearer_auth(api_key);
            }

            let response = request.send().await?;
            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let stats: UsageStats = response.json().await?;
            Ok(stats)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Update session status
    ///
    /// # Errors
    /// Returns an error if the update request fails.
    pub async fn update_session_status(
        &self,
        session_id: &str,
        status: SessionStatus,
    ) -> Result<SessionInfo, IntegrationError> {
        let url = format!(
            "{}/api/v1/sessions/{}/status",
            self.config.base_url, session_id
        );

        let operation = || async {
            let mut request = self.client.put(&url).json(&serde_json::json!({
                "status": status
            }));

            if let Some(api_key) = &self.config.api_key {
                request = request.bearer_auth(api_key);
            }

            let response = request.send().await?;
            let status_code = response.status();

            if !status_code.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                return Err(IntegrationError::ApiError {
                    status: status_code.as_u16(),
                    message: error_body,
                });
            }

            let session_info: SessionInfo = response.json().await?;
            Ok(session_info)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// List sessions
    ///
    /// # Errors
    /// Returns an error if the request fails.
    pub async fn list_sessions(
        &self,
        page: Option<usize>,
        page_size: Option<usize>,
    ) -> Result<SessionListResponse, IntegrationError> {
        let mut url = format!("{}/api/v1/sessions", self.config.base_url);

        if let Some(page) = page {
            url.push_str(&format!("?page={}", page));
            if let Some(size) = page_size {
                url.push_str(&format!("&page_size={}", size));
            }
        } else if let Some(size) = page_size {
            url.push_str(&format!("?page_size={}", size));
        }

        let operation = || async {
            let mut request = self.client.get(&url);

            if let Some(api_key) = &self.config.api_key {
                request = request.bearer_auth(api_key);
            }

            let response = request.send().await?;
            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let sessions: SessionListResponse = response.json().await?;
            Ok(sessions)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Delete a session from the registry
    ///
    /// # Errors
    /// Returns an error if the deletion request fails.
    pub async fn delete_session(&self, session_id: &str) -> Result<(), IntegrationError> {
        let url = format!("{}/api/v1/sessions/{}", self.config.base_url, session_id);

        let operation = || async {
            let mut request = self.client.delete(&url);

            if let Some(api_key) = &self.config.api_key {
                request = request.bearer_auth(api_key);
            }

            let response = request.send().await?;
            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            Ok(())
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Health check for the registry service
    ///
    /// # Errors
    /// Returns an error if the health check fails.
    pub async fn health_check(&self) -> Result<bool, IntegrationError> {
        let url = format!("{}/health", self.config.base_url);

        let response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_registry_client_creation() {
        let config = RegistryConfig::new("http://localhost:8080");
        let client = RegistryClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_registry_client_with_retry_policy() {
        let config = RegistryConfig::new("http://localhost:8080");
        let client = RegistryClient::new(config).unwrap();
        let custom_policy = RetryPolicy::new().with_max_attempts(5);
        let client = client.with_retry_policy(custom_policy);
        assert_eq!(client.retry_policy.max_attempts, 5);
    }

    // Note: Integration tests would require a running registry service
    // and are better placed in tests/integration_test.rs
}

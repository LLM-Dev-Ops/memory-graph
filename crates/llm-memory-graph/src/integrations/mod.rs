//! External integrations for LLM-Memory-Graph ecosystem
//!
//! This module provides integration clients for external LLM DevOps ecosystem services:
//! - **LLM-Registry**: Model metadata, version tracking, and usage statistics
//! - **Data-Vault**: Secure archival, retention policies, and compliance
//! - **Schema-Registry**: Centralized schema management, validation, and versioning
//! - **Config-Manager**: Centralized configuration management with remote fetch and local override

pub mod config_manager;
pub mod registry;
pub mod schema_registry;
pub mod vault;

// Re-export main types
pub use config_manager::{
    CascadingConfigProvider, ConfigAdapter, ConfigManagerClient, ConfigManagerConfig,
    ConfigProvider, GraphLimitsInfo, LocalConfigProvider, MemoryGraphConfig, PruningInfo,
    RemoteConfigProvider, RetentionInfo,
};
pub use registry::{
    ModelMetadata, ModelParameters, RegistryClient, RegistryConfig, SessionRegistration,
    UsageStats,
};
pub use schema_registry::{
    CachingValidator, CompatibilityLevel, GracefulValidator, NoOpValidator, SchemaFormat,
    SchemaRegistryClient, SchemaRegistryConfig, SchemaValidator, ValidationError,
    ValidationResult,
};
pub use vault::{
    ArchivalScheduler, ArchiveEntry, ComplianceLevel, RetentionPolicy, SchedulerConfig,
    VaultClient, VaultConfig,
};

use crate::{Error, Result};
use std::time::Duration;

/// Integration error types
#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    /// HTTP request error
    #[error("HTTP request failed: {0}")]
    HttpError(String),

    /// Authentication error
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    /// API error response
    #[error("API error: {status} - {message}")]
    ApiError {
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
    },

    /// Connection error
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Timeout error
    #[error("Request timeout after {0}ms")]
    Timeout(u64),

    /// Circuit breaker open
    #[error("Circuit breaker open: {0}")]
    CircuitBreakerOpen(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<IntegrationError> for Error {
    fn from(err: IntegrationError) -> Self {
        Error::Other(err.to_string())
    }
}

impl From<reqwest::Error> for IntegrationError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            IntegrationError::Timeout(30000) // Default timeout
        } else if err.is_connect() {
            IntegrationError::ConnectionError(err.to_string())
        } else {
            IntegrationError::HttpError(err.to_string())
        }
    }
}

impl From<serde_json::Error> for IntegrationError {
    fn from(err: serde_json::Error) -> Self {
        IntegrationError::Serialization(err.to_string())
    }
}

/// Circuit breaker for external service calls
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    failure_threshold: usize,
    success_threshold: usize,
    timeout_duration: Duration,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(failure_threshold: usize, success_threshold: usize, timeout_duration: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout_duration,
        }
    }

    /// Default circuit breaker configuration
    pub fn default_config() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_duration: Duration::from_secs(60),
        }
    }
}

/// Retry configuration for integration calls
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: usize,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Whether to retry on timeout
    pub retry_on_timeout: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            retry_on_timeout: true,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum retry attempts
    pub fn with_max_attempts(mut self, max_attempts: usize) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Set initial delay
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Set backoff multiplier
    pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }
}

/// Execute a request with retry logic
pub async fn retry_request<F, Fut, T>(policy: &RetryPolicy, mut operation: F) -> Result<T, IntegrationError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, IntegrationError>>,
{
    let mut attempt = 0;
    let mut delay = policy.initial_delay;

    loop {
        attempt += 1;
        match operation().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                // Check if we should retry
                let should_retry = match &err {
                    IntegrationError::Timeout(_) => policy.retry_on_timeout,
                    IntegrationError::ConnectionError(_) => true,
                    IntegrationError::HttpError(_) => true,
                    IntegrationError::ApiError { status, .. } => {
                        // Retry on 5xx errors (server errors)
                        *status >= 500 && *status < 600
                    }
                    _ => false,
                };

                if !should_retry || attempt >= policy.max_attempts {
                    return Err(err);
                }

                // Wait before retrying
                tokio::time::sleep(delay).await;

                // Calculate next delay with exponential backoff
                delay = std::cmp::min(
                    Duration::from_millis(
                        (delay.as_millis() as f64 * policy.backoff_multiplier) as u64,
                    ),
                    policy.max_delay,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_creation() {
        let cb = CircuitBreaker::new(5, 2, Duration::from_secs(60));
        assert_eq!(cb.failure_threshold, 5);
        assert_eq!(cb.success_threshold, 2);
        assert_eq!(cb.timeout_duration, Duration::from_secs(60));
    }

    #[test]
    fn test_retry_policy_builder() {
        let policy = RetryPolicy::new()
            .with_max_attempts(5)
            .with_initial_delay(Duration::from_millis(200))
            .with_backoff_multiplier(3.0);

        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_delay, Duration::from_millis(200));
        assert_eq!(policy.backoff_multiplier, 3.0);
    }

    #[test]
    fn test_integration_error_from_reqwest() {
        // Test timeout error
        let err = IntegrationError::Timeout(5000);
        assert!(matches!(err, IntegrationError::Timeout(_)));
    }
}

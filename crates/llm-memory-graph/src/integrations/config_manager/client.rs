//! Config Manager HTTP client implementation

use super::types::{
    ConfigListResponse, ConfigManagerConfig, ConfigResponse, ConfigUpdateRequest,
    ConfigVersion, HealthCheckResponse, MemoryGraphConfig,
};
use crate::integrations::{retry_request, IntegrationError, RetryPolicy};
use reqwest::{Client, StatusCode};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Config Manager HTTP client
///
/// Provides integration with the Config Manager service for:
/// - Fetching remote configurations
/// - Updating configurations
/// - Version management
/// - Configuration validation
pub struct ConfigManagerClient {
    config: ConfigManagerConfig,
    client: Client,
    retry_policy: RetryPolicy,
    /// Optional cached configuration
    cached_config: Arc<parking_lot::RwLock<Option<(MemoryGraphConfig, std::time::Instant)>>>,
}

impl ConfigManagerClient {
    /// Create a new Config Manager client
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be created.
    pub fn new(config: ConfigManagerConfig) -> Result<Self, IntegrationError> {
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
            cached_config: Arc::new(parking_lot::RwLock::new(None)),
        })
    }

    /// Create a client with custom retry policy
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Get configuration for a specific application and environment
    ///
    /// # Errors
    /// Returns an error if the configuration is not found or the request fails.
    pub async fn get_config(
        &self,
        app_name: &str,
        environment: &str,
    ) -> Result<MemoryGraphConfig, IntegrationError> {
        // Check cache first
        if let Some(cached) = self.get_from_cache() {
            if self.config.enable_logging {
                debug!(
                    "Using cached config for app={}, env={}",
                    app_name, environment
                );
            }
            return Ok(cached);
        }

        let url = format!(
            "{}/api/v1/configs/{}/{}",
            self.config.base_url, app_name, environment
        );

        if self.config.enable_logging {
            info!(
                "Fetching config for app={}, environment={}",
                app_name, environment
            );
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
                    message: format!(
                        "Configuration not found for app={}, env={}",
                        app_name, environment
                    ),
                });
            }

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                error!("Failed to fetch config: {} - {}", status, error_body);
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let config_response: ConfigResponse = response.json().await?;
            Ok(config_response.config)
        };

        let config = retry_request(&self.retry_policy, operation).await?;

        // Update cache
        self.update_cache(config.clone());

        if self.config.enable_logging {
            info!(
                "Config fetched successfully: version={}",
                config.version
            );
        }

        Ok(config)
    }

    /// Get configuration by ID
    ///
    /// # Errors
    /// Returns an error if the configuration is not found or the request fails.
    pub async fn get_config_by_id(
        &self,
        config_id: &str,
    ) -> Result<MemoryGraphConfig, IntegrationError> {
        let url = format!("{}/api/v1/configs/id/{}", self.config.base_url, config_id);

        if self.config.enable_logging {
            debug!("Fetching config by ID: {}", config_id);
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
                    message: format!("Configuration not found: {}", config_id),
                });
            }

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                error!("Failed to fetch config by ID: {} - {}", status, error_body);
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let config_response: ConfigResponse = response.json().await?;
            Ok(config_response.config)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// List configurations with optional filtering
    ///
    /// # Errors
    /// Returns an error if the request fails.
    pub async fn list_configs(
        &self,
        app_name: Option<&str>,
        environment: Option<&str>,
        page: Option<usize>,
        page_size: Option<usize>,
    ) -> Result<ConfigListResponse, IntegrationError> {
        let mut url = format!("{}/api/v1/configs", self.config.base_url);
        let mut params = Vec::new();

        if let Some(app) = app_name {
            params.push(format!("app_name={}", app));
        }
        if let Some(env) = environment {
            params.push(format!("environment={}", env));
        }
        if let Some(p) = page {
            params.push(format!("page={}", p));
        }
        if let Some(size) = page_size {
            params.push(format!("page_size={}", size));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
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

            let configs: ConfigListResponse = response.json().await?;
            Ok(configs)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Update or create a configuration
    ///
    /// # Errors
    /// Returns an error if the update request fails.
    pub async fn update_config(
        &self,
        request: ConfigUpdateRequest,
    ) -> Result<MemoryGraphConfig, IntegrationError> {
        let url = format!("{}/api/v1/configs", self.config.base_url);

        if self.config.enable_logging {
            info!(
                "Updating config for app={}, env={}",
                request.config.app_name, request.config.environment
            );
        }

        let operation = || async {
            let mut http_request = self.client.put(&url).json(&request);

            if let Some(api_key) = &self.config.api_key {
                http_request = http_request.bearer_auth(api_key);
            }

            let response = http_request.send().await?;
            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                error!("Failed to update config: {} - {}", status, error_body);
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let config_response: ConfigResponse = response.json().await?;
            Ok(config_response.config)
        };

        let config = retry_request(&self.retry_policy, operation).await?;

        // Invalidate cache
        self.invalidate_cache();

        if self.config.enable_logging {
            info!("Config updated successfully: version={}", config.version);
        }

        Ok(config)
    }

    /// Delete a configuration
    ///
    /// # Errors
    /// Returns an error if the deletion request fails.
    pub async fn delete_config(
        &self,
        app_name: &str,
        environment: &str,
    ) -> Result<(), IntegrationError> {
        let url = format!(
            "{}/api/v1/configs/{}/{}",
            self.config.base_url, app_name, environment
        );

        if self.config.enable_logging {
            info!(
                "Deleting config for app={}, environment={}",
                app_name, environment
            );
        }

        let operation = || async {
            let mut request = self.client.delete(&url);

            if let Some(api_key) = &self.config.api_key {
                request = request.bearer_auth(api_key);
            }

            let response = request.send().await?;
            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                error!("Failed to delete config: {} - {}", status, error_body);
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            Ok(())
        };

        retry_request(&self.retry_policy, operation).await?;

        // Invalidate cache
        self.invalidate_cache();

        Ok(())
    }

    /// Get configuration version history
    ///
    /// # Errors
    /// Returns an error if the request fails.
    pub async fn get_version_history(
        &self,
        app_name: &str,
        environment: &str,
    ) -> Result<Vec<ConfigVersion>, IntegrationError> {
        let url = format!(
            "{}/api/v1/configs/{}/{}/versions",
            self.config.base_url, app_name, environment
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

            let versions: Vec<ConfigVersion> = response.json().await?;
            Ok(versions)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Validate a configuration without saving
    ///
    /// # Errors
    /// Returns an error if the validation fails or the request fails.
    pub async fn validate_config(
        &self,
        config: &MemoryGraphConfig,
    ) -> Result<bool, IntegrationError> {
        let url = format!("{}/api/v1/configs/validate", self.config.base_url);

        let operation = || async {
            let mut request = self.client.post(&url).json(config);

            if let Some(api_key) = &self.config.api_key {
                request = request.bearer_auth(api_key);
            }

            let response = request.send().await?;
            let status = response.status();

            if status == StatusCode::BAD_REQUEST {
                let error_body = response.text().await.unwrap_or_default();
                return Err(IntegrationError::ApiError {
                    status: 400,
                    message: format!("Validation failed: {}", error_body),
                });
            }

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            Ok(true)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Health check for the Config Manager service
    ///
    /// # Errors
    /// Returns an error if the health check fails.
    pub async fn health_check(&self) -> Result<HealthCheckResponse, IntegrationError> {
        let url = format!("{}/health", self.config.base_url);

        let response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(IntegrationError::ConnectionError(
                "Config Manager service is unavailable".to_string(),
            ));
        }

        let health: HealthCheckResponse = response
            .json()
            .await
            .unwrap_or_else(|_| HealthCheckResponse {
                status: "ok".to_string(),
                version: "unknown".to_string(),
                uptime_secs: 0,
            });

        Ok(health)
    }

    /// Get configuration from cache if available and not expired
    fn get_from_cache(&self) -> Option<MemoryGraphConfig> {
        let cache = self.cached_config.read();
        if let Some((config, timestamp)) = cache.as_ref() {
            let elapsed = timestamp.elapsed();
            if elapsed.as_secs() < self.config.cache_ttl_secs {
                return Some(config.clone());
            }
        }
        None
    }

    /// Update the configuration cache
    fn update_cache(&self, config: MemoryGraphConfig) {
        let mut cache = self.cached_config.write();
        *cache = Some((config, std::time::Instant::now()));
    }

    /// Invalidate the configuration cache
    fn invalidate_cache(&self) {
        let mut cache = self.cached_config.write();
        *cache = None;
    }

    /// Clear the configuration cache
    pub fn clear_cache(&self) {
        self.invalidate_cache();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_manager_client_creation() {
        let config = ConfigManagerConfig::new("http://localhost:7070");
        let client = ConfigManagerClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_config_manager_client_with_retry_policy() {
        let config = ConfigManagerConfig::new("http://localhost:7070");
        let client = ConfigManagerClient::new(config).unwrap();
        let custom_policy = RetryPolicy::new().with_max_attempts(5);
        let client = client.with_retry_policy(custom_policy);
        assert_eq!(client.retry_policy.max_attempts, 5);
    }

    #[test]
    fn test_cache_operations() {
        let config = ConfigManagerConfig::new("http://localhost:7070");
        let client = ConfigManagerClient::new(config).unwrap();

        // Initially no cache
        assert!(client.get_from_cache().is_none());

        // Clear cache should not panic
        client.clear_cache();
    }
}

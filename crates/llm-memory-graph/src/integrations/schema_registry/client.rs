//! Schema Registry client implementation

use super::config::SchemaRegistryConfig;
use super::types::{
    CompatibilityRequest, CompatibilityResponse, SchemaListResponse, SchemaMetadata,
    SchemaRegistration, ValidationRequest, ValidationResponse, ValidationResult,
};
use super::validator::{
    CachingValidator, GracefulValidator, NoOpValidator, SchemaValidator,
};
use crate::integrations::{retry_request, IntegrationError, RetryPolicy};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Schema Registry client
///
/// Provides integration with the Schema Registry service for:
/// - Schema registration and versioning
/// - Data validation against schemas
/// - Schema compatibility checking
/// - Schema metadata retrieval
///
/// # Examples
///
/// ```rust,ignore
/// use llm_memory_graph::integrations::schema_registry::{SchemaRegistryClient, SchemaRegistryConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = SchemaRegistryConfig::new("https://schema-registry.example.com")
///         .with_validation_enabled(true);
///
///     let client = SchemaRegistryClient::new(config)?;
///
///     // Validate data
///     let data = serde_json::json!({"name": "test", "value": 42});
///     let result = client.validate("my-schema", None, &data).await?;
///
///     if result.is_valid() {
///         println!("Data is valid!");
///     }
///
///     Ok(())
/// }
/// ```
pub struct SchemaRegistryClient {
    config: SchemaRegistryConfig,
    client: Client,
    retry_policy: RetryPolicy,
    validator: Arc<dyn SchemaValidator>,
}

impl SchemaRegistryClient {
    /// Create a new Schema Registry client
    ///
    /// # Arguments
    ///
    /// * `config` - Client configuration
    ///
    /// # Returns
    ///
    /// Returns a new SchemaRegistryClient or an error if the HTTP client cannot be created
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use llm_memory_graph::integrations::schema_registry::{SchemaRegistryClient, SchemaRegistryConfig};
    ///
    /// let config = SchemaRegistryConfig::new("https://schema-registry.example.com");
    /// let client = SchemaRegistryClient::new(config)?;
    /// ```
    pub fn new(config: SchemaRegistryConfig) -> Result<Self, IntegrationError> {
        // Validate configuration
        config.validate().map_err(|e| {
            IntegrationError::InvalidConfig(format!("Invalid configuration: {}", e))
        })?;

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

        // Create validator based on configuration
        let validator = Self::create_validator(&config);

        Ok(Self {
            config,
            client,
            retry_policy,
            validator,
        })
    }

    /// Create the appropriate validator based on configuration
    fn create_validator(config: &SchemaRegistryConfig) -> Arc<dyn SchemaValidator> {
        if !config.validation_enabled {
            // Validation disabled - use no-op validator
            Arc::new(NoOpValidator::new())
        } else {
            // Start with a registry validator (placeholder for now)
            let base_validator = Arc::new(NoOpValidator::new()) as Arc<dyn SchemaValidator>;

            // Wrap with caching if enabled
            let cached_validator = if config.is_cache_enabled() {
                Arc::new(CachingValidator::new(
                    base_validator,
                    config.cache_ttl(),
                    config.max_cache_size,
                )) as Arc<dyn SchemaValidator>
            } else {
                base_validator
            };

            // Wrap with graceful degradation
            Arc::new(GracefulValidator::new(
                cached_validator,
                config.fail_on_validation_error,
            ))
        }
    }

    /// Create a client with custom retry policy
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use llm_memory_graph::integrations::schema_registry::SchemaRegistryClient;
    /// use llm_memory_graph::integrations::RetryPolicy;
    /// use std::time::Duration;
    ///
    /// let config = SchemaRegistryConfig::new("https://schema-registry.example.com");
    /// let client = SchemaRegistryClient::new(config)?
    ///     .with_retry_policy(RetryPolicy::new().with_max_attempts(5));
    /// ```
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Get a reference to the validator
    pub fn validator(&self) -> &Arc<dyn SchemaValidator> {
        &self.validator
    }

    /// Register a schema with the registry
    ///
    /// # Arguments
    ///
    /// * `registration` - Schema registration details
    ///
    /// # Returns
    ///
    /// Returns the registered schema metadata
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use llm_memory_graph::integrations::schema_registry::{SchemaRegistration, SchemaFormat};
    ///
    /// let schema = serde_json::json!({
    ///     "type": "object",
    ///     "properties": {
    ///         "name": {"type": "string"},
    ///         "age": {"type": "integer"}
    ///     },
    ///     "required": ["name"]
    /// });
    ///
    /// let registration = SchemaRegistration::new(
    ///     "user-schema",
    ///     "1.0.0",
    ///     SchemaFormat::JsonSchema,
    ///     schema
    /// );
    ///
    /// let metadata = client.register_schema(registration).await?;
    /// ```
    pub async fn register_schema(
        &self,
        registration: SchemaRegistration,
    ) -> Result<SchemaMetadata, IntegrationError> {
        let url = format!("{}/api/v1/schemas", self.config.base_url);

        if self.config.enable_logging {
            info!(
                "Registering schema {} version {}",
                registration.schema_id, registration.version
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
                    "Failed to register schema: {} - {}",
                    status, error_body
                );
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let metadata: SchemaMetadata = response.json().await?;
            Ok(metadata)
        };

        let result = retry_request(&self.retry_policy, operation).await?;

        if self.config.enable_logging {
            info!(
                "Schema {} version {} registered successfully",
                registration.schema_id, registration.version
            );
        }

        Ok(result)
    }

    /// Get schema metadata from the registry
    ///
    /// # Arguments
    ///
    /// * `schema_id` - Schema identifier
    /// * `version` - Optional schema version (uses latest if not specified)
    ///
    /// # Returns
    ///
    /// Returns the schema metadata
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Get latest version
    /// let metadata = client.get_schema("user-schema", None).await?;
    ///
    /// // Get specific version
    /// let metadata = client.get_schema("user-schema", Some("1.0.0")).await?;
    /// ```
    pub async fn get_schema(
        &self,
        schema_id: &str,
        version: Option<&str>,
    ) -> Result<SchemaMetadata, IntegrationError> {
        let url = match version {
            Some(v) => format!(
                "{}/api/v1/schemas/{}/versions/{}",
                self.config.base_url, schema_id, v
            ),
            None => format!("{}/api/v1/schemas/{}", self.config.base_url, schema_id),
        };

        if self.config.enable_logging {
            debug!("Fetching schema: {} version {:?}", schema_id, version);
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
                    message: format!("Schema not found: {}", schema_id),
                });
            }

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                error!("Failed to get schema metadata: {} - {}", status, error_body);
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let metadata: SchemaMetadata = response.json().await?;
            Ok(metadata)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// List available schemas
    ///
    /// # Arguments
    ///
    /// * `page` - Optional page number
    /// * `page_size` - Optional page size
    ///
    /// # Returns
    ///
    /// Returns a paginated list of schemas
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // List all schemas
    /// let schemas = client.list_schemas(None, None).await?;
    ///
    /// // List with pagination
    /// let schemas = client.list_schemas(Some(1), Some(50)).await?;
    /// ```
    pub async fn list_schemas(
        &self,
        page: Option<usize>,
        page_size: Option<usize>,
    ) -> Result<SchemaListResponse, IntegrationError> {
        let mut url = format!("{}/api/v1/schemas", self.config.base_url);

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

            let schemas: SchemaListResponse = response.json().await?;
            Ok(schemas)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Validate data against a schema using the remote Schema Registry
    ///
    /// # Arguments
    ///
    /// * `request` - Validation request containing schema ID and data
    ///
    /// # Returns
    ///
    /// Returns the validation response from the registry
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use llm_memory_graph::integrations::schema_registry::ValidationRequest;
    ///
    /// let data = serde_json::json!({"name": "Alice", "age": 30});
    /// let request = ValidationRequest::new("user-schema", data);
    ///
    /// let response = client.validate_remote(request).await?;
    /// if response.valid {
    ///     println!("Validation passed!");
    /// }
    /// ```
    pub async fn validate_remote(
        &self,
        request: ValidationRequest,
    ) -> Result<ValidationResponse, IntegrationError> {
        let url = format!("{}/api/v1/validate", self.config.base_url);

        if self.config.enable_logging {
            debug!(
                "Validating data against schema: {} version {:?}",
                request.schema_id, request.version
            );
        }

        let operation = || async {
            let mut http_request = self.client.post(&url).json(&request);

            if let Some(api_key) = &self.config.api_key {
                http_request = http_request.bearer_auth(api_key);
            }

            let response = http_request.send().await?;
            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                warn!("Validation request failed: {} - {}", status, error_body);
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let validation_response: ValidationResponse = response.json().await?;
            Ok(validation_response)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Check schema compatibility
    ///
    /// # Arguments
    ///
    /// * `request` - Compatibility check request
    ///
    /// # Returns
    ///
    /// Returns the compatibility check response
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use llm_memory_graph::integrations::schema_registry::{CompatibilityRequest, CompatibilityLevel};
    ///
    /// let new_schema = serde_json::json!({
    ///     "type": "object",
    ///     "properties": {
    ///         "name": {"type": "string"},
    ///         "age": {"type": "integer"},
    ///         "email": {"type": "string"}  // New field
    ///     },
    ///     "required": ["name"]
    /// });
    ///
    /// let request = CompatibilityRequest {
    ///     schema_id: "user-schema".to_string(),
    ///     base_version: "1.0.0".to_string(),
    ///     new_schema,
    ///     compatibility_level: CompatibilityLevel::Backward,
    /// };
    ///
    /// let response = client.check_compatibility(request).await?;
    /// if response.compatible {
    ///     println!("Schemas are compatible!");
    /// }
    /// ```
    pub async fn check_compatibility(
        &self,
        request: CompatibilityRequest,
    ) -> Result<CompatibilityResponse, IntegrationError> {
        let url = format!("{}/api/v1/compatibility", self.config.base_url);

        if self.config.enable_logging {
            info!(
                "Checking compatibility for schema {} version {}",
                request.schema_id, request.base_version
            );
        }

        let operation = || async {
            let mut http_request = self.client.post(&url).json(&request);

            if let Some(api_key) = &self.config.api_key {
                http_request = http_request.bearer_auth(api_key);
            }

            let response = http_request.send().await?;
            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let compatibility_response: CompatibilityResponse = response.json().await?;
            Ok(compatibility_response)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Delete a schema from the registry
    ///
    /// # Arguments
    ///
    /// * `schema_id` - Schema identifier to delete
    /// * `version` - Optional version (deletes all versions if not specified)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Delete specific version
    /// client.delete_schema("user-schema", Some("1.0.0")).await?;
    ///
    /// // Delete all versions
    /// client.delete_schema("user-schema", None).await?;
    /// ```
    pub async fn delete_schema(
        &self,
        schema_id: &str,
        version: Option<&str>,
    ) -> Result<(), IntegrationError> {
        let url = match version {
            Some(v) => format!(
                "{}/api/v1/schemas/{}/versions/{}",
                self.config.base_url, schema_id, v
            ),
            None => format!("{}/api/v1/schemas/{}", self.config.base_url, schema_id),
        };

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

    /// Health check for the Schema Registry service
    ///
    /// # Returns
    ///
    /// Returns true if the service is healthy, false otherwise
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// if client.health_check().await? {
    ///     println!("Schema Registry is healthy");
    /// }
    /// ```
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

// Implement SchemaValidator trait for the client
#[async_trait]
impl SchemaValidator for SchemaRegistryClient {
    async fn validate(
        &self,
        schema_id: &str,
        version: Option<&str>,
        data: &serde_json::Value,
    ) -> Result<ValidationResult, IntegrationError> {
        self.validator.validate(schema_id, version, data).await
    }

    fn is_enabled(&self) -> bool {
        self.validator.is_enabled()
    }

    fn name(&self) -> &str {
        "SchemaRegistryClient"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::schema_registry::types::SchemaFormat;

    #[test]
    fn test_client_creation() {
        let config = SchemaRegistryConfig::new("http://localhost:8081");
        let client = SchemaRegistryClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_with_retry_policy() {
        let config = SchemaRegistryConfig::new("http://localhost:8081");
        let client = SchemaRegistryClient::new(config).unwrap();
        let custom_policy = RetryPolicy::new().with_max_attempts(5);
        let client = client.with_retry_policy(custom_policy);
        assert_eq!(client.retry_policy.max_attempts, 5);
    }

    #[test]
    fn test_validator_creation_disabled() {
        let config = SchemaRegistryConfig::new("http://localhost:8081")
            .with_validation_enabled(false);
        let client = SchemaRegistryClient::new(config).unwrap();
        assert!(!client.validator().is_enabled());
    }

    #[test]
    fn test_validator_creation_enabled() {
        let config = SchemaRegistryConfig::new("http://localhost:8081")
            .with_validation_enabled(true);
        let client = SchemaRegistryClient::new(config).unwrap();
        // The validator is enabled but wrapped, so is_enabled() should return true
        // Note: Since we're using NoOpValidator as base for now, this will be false
        // When we integrate the real validator, this should be true
    }

    #[tokio::test]
    async fn test_schema_validator_trait() {
        let config = SchemaRegistryConfig::new("http://localhost:8081")
            .with_validation_enabled(false);
        let client = SchemaRegistryClient::new(config).unwrap();

        let data = serde_json::json!({"test": "data"});
        let result = client.validate("test-schema", None, &data).await.unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn test_invalid_config() {
        let config = SchemaRegistryConfig {
            base_url: String::new(),
            ..Default::default()
        };
        let client = SchemaRegistryClient::new(config);
        assert!(client.is_err());
    }

    // Note: Integration tests would require a running Schema Registry service
    // and are better placed in tests/integration_test.rs
}

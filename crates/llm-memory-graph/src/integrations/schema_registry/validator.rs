//! Schema validation trait and implementations

use super::types::{ValidationError, ValidationResult};
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Schema validation trait
///
/// This trait defines the interface for validating data against schemas.
/// Implementations can validate against local schemas, remote Schema Registry, or both.
///
/// # Examples
///
/// ```rust,ignore
/// use llm_memory_graph::integrations::schema_registry::{SchemaValidator, NoOpValidator};
///
/// #[tokio::main]
/// async fn main() {
///     let validator = NoOpValidator::new();
///     let data = serde_json::json!({"name": "test"});
///
///     let result = validator.validate("test-schema", None, &data).await.unwrap();
///     assert!(result.is_valid());
/// }
/// ```
#[async_trait]
pub trait SchemaValidator: Send + Sync {
    /// Validate data against a schema
    ///
    /// # Arguments
    ///
    /// * `schema_id` - Schema identifier
    /// * `version` - Optional schema version (uses latest if not specified)
    /// * `data` - Data to validate
    ///
    /// # Returns
    ///
    /// Returns a ValidationResult indicating success or containing validation errors
    async fn validate(
        &self,
        schema_id: &str,
        version: Option<&str>,
        data: &JsonValue,
    ) -> Result<ValidationResult, crate::integrations::IntegrationError>;

    /// Validate data against a schema and fail fast on first error
    ///
    /// # Arguments
    ///
    /// * `schema_id` - Schema identifier
    /// * `version` - Optional schema version (uses latest if not specified)
    /// * `data` - Data to validate
    ///
    /// # Returns
    ///
    /// Returns Ok(()) if valid, or an error if validation fails
    async fn validate_strict(
        &self,
        schema_id: &str,
        version: Option<&str>,
        data: &JsonValue,
    ) -> Result<(), crate::integrations::IntegrationError> {
        let result = self.validate(schema_id, version, data).await?;
        match result {
            ValidationResult::Valid => Ok(()),
            ValidationResult::Invalid(errors) => {
                Err(crate::integrations::IntegrationError::ApiError {
                    status: 400,
                    message: format!(
                        "Schema validation failed: {}",
                        errors
                            .iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                })
            }
        }
    }

    /// Check if validation is enabled
    ///
    /// Some validators (like NoOpValidator) always return true without checking
    fn is_enabled(&self) -> bool;

    /// Get validator name for logging and debugging
    fn name(&self) -> &str;
}

/// No-op validator that always passes validation
///
/// This is the default validator used when schema validation is disabled.
/// It allows for graceful degradation when the Schema Registry is unavailable
/// or when validation is explicitly disabled.
///
/// # Examples
///
/// ```rust,ignore
/// use llm_memory_graph::integrations::schema_registry::NoOpValidator;
///
/// let validator = NoOpValidator::new();
/// assert!(!validator.is_enabled());
/// ```
#[derive(Debug, Clone, Default)]
pub struct NoOpValidator;

impl NoOpValidator {
    /// Create a new no-op validator
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SchemaValidator for NoOpValidator {
    async fn validate(
        &self,
        _schema_id: &str,
        _version: Option<&str>,
        _data: &JsonValue,
    ) -> Result<ValidationResult, crate::integrations::IntegrationError> {
        // Always return valid without checking
        Ok(ValidationResult::Valid)
    }

    fn is_enabled(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "NoOpValidator"
    }
}

/// Wrapper validator with graceful degradation
///
/// This validator wraps another validator and provides graceful degradation:
/// - If the inner validator is unavailable, validation passes
/// - If validation errors occur but fail_on_error is false, warnings are logged but validation passes
/// - If validation errors occur and fail_on_error is true, validation fails
///
/// # Examples
///
/// ```rust,ignore
/// use llm_memory_graph::integrations::schema_registry::{GracefulValidator, NoOpValidator};
/// use std::sync::Arc;
///
/// let inner = Arc::new(NoOpValidator::new());
/// let validator = GracefulValidator::new(inner, false);
/// assert!(validator.is_enabled());
/// ```
#[derive(Clone)]
pub struct GracefulValidator {
    inner: Arc<dyn SchemaValidator>,
    fail_on_error: bool,
}

impl GracefulValidator {
    /// Create a new graceful validator
    ///
    /// # Arguments
    ///
    /// * `inner` - The inner validator to wrap
    /// * `fail_on_error` - Whether to fail on validation errors or just log warnings
    pub fn new(inner: Arc<dyn SchemaValidator>, fail_on_error: bool) -> Self {
        Self {
            inner,
            fail_on_error,
        }
    }

    /// Get a reference to the inner validator
    pub fn inner(&self) -> &Arc<dyn SchemaValidator> {
        &self.inner
    }
}

#[async_trait]
impl SchemaValidator for GracefulValidator {
    async fn validate(
        &self,
        schema_id: &str,
        version: Option<&str>,
        data: &JsonValue,
    ) -> Result<ValidationResult, crate::integrations::IntegrationError> {
        match self.inner.validate(schema_id, version, data).await {
            Ok(result) => {
                if !self.fail_on_error && result.is_invalid() {
                    // Log warnings but return Valid for graceful degradation
                    if let Some(errors) = result.errors() {
                        tracing::warn!(
                            schema_id = schema_id,
                            version = version,
                            error_count = errors.len(),
                            "Schema validation failed but continuing due to graceful degradation"
                        );
                        for error in errors {
                            tracing::debug!(
                                field = error.field,
                                code = error.code,
                                message = error.message,
                                "Validation error details"
                            );
                        }
                    }
                    Ok(ValidationResult::Valid)
                } else {
                    Ok(result)
                }
            }
            Err(err) => {
                // If validation service is unavailable, degrade gracefully
                if !self.fail_on_error {
                    tracing::warn!(
                        schema_id = schema_id,
                        version = version,
                        error = %err,
                        "Schema validation unavailable, continuing without validation"
                    );
                    Ok(ValidationResult::Valid)
                } else {
                    Err(err)
                }
            }
        }
    }

    fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }

    fn name(&self) -> &str {
        "GracefulValidator"
    }
}

/// Caching validator wrapper
///
/// Wraps another validator and caches validation results to reduce load on the Schema Registry.
/// Cache entries expire after a configurable TTL.
///
/// # Examples
///
/// ```rust,ignore
/// use llm_memory_graph::integrations::schema_registry::{CachingValidator, NoOpValidator};
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// let inner = Arc::new(NoOpValidator::new());
/// let validator = CachingValidator::new(inner, Duration::from_secs(300), 100);
/// ```
#[derive(Clone)]
pub struct CachingValidator {
    inner: Arc<dyn SchemaValidator>,
    cache: Arc<moka::future::Cache<String, ValidationResult>>,
}

impl CachingValidator {
    /// Create a new caching validator
    ///
    /// # Arguments
    ///
    /// * `inner` - The inner validator to wrap
    /// * `ttl` - Time-to-live for cache entries
    /// * `max_capacity` - Maximum number of entries to cache
    pub fn new(
        inner: Arc<dyn SchemaValidator>,
        ttl: std::time::Duration,
        max_capacity: usize,
    ) -> Self {
        let cache = moka::future::Cache::builder()
            .time_to_live(ttl)
            .max_capacity(max_capacity as u64)
            .build();

        Self {
            inner,
            cache: Arc::new(cache),
        }
    }

    /// Generate cache key from schema_id, version, and data hash
    fn cache_key(schema_id: &str, version: Option<&str>, data: &JsonValue) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.to_string().hash(&mut hasher);
        let data_hash = hasher.finish();

        match version {
            Some(v) => format!("{}:{}:{}", schema_id, v, data_hash),
            None => format!("{}:latest:{}", schema_id, data_hash),
        }
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (u64, u64) {
        (self.cache.entry_count(), self.cache.weighted_size())
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        self.cache.invalidate_all();
    }

    /// Get a reference to the inner validator
    pub fn inner(&self) -> &Arc<dyn SchemaValidator> {
        &self.inner
    }
}

#[async_trait]
impl SchemaValidator for CachingValidator {
    async fn validate(
        &self,
        schema_id: &str,
        version: Option<&str>,
        data: &JsonValue,
    ) -> Result<ValidationResult, crate::integrations::IntegrationError> {
        let cache_key = Self::cache_key(schema_id, version, data);

        // Try to get from cache
        if let Some(cached_result) = self.cache.get(&cache_key).await {
            tracing::debug!(
                schema_id = schema_id,
                version = version,
                "Using cached validation result"
            );
            return Ok(cached_result);
        }

        // Not in cache, validate and cache the result
        let result = self.inner.validate(schema_id, version, data).await?;

        // Only cache successful validations or deterministic errors
        // Don't cache transient errors (service unavailable, etc.)
        self.cache.insert(cache_key, result.clone()).await;

        Ok(result)
    }

    fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }

    fn name(&self) -> &str {
        "CachingValidator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_validator() {
        let validator = NoOpValidator::new();
        assert!(!validator.is_enabled());
        assert_eq!(validator.name(), "NoOpValidator");

        let data = serde_json::json!({"test": "data"});
        let result = validator.validate("test-schema", None, &data).await.unwrap();
        assert!(result.is_valid());
    }

    #[tokio::test]
    async fn test_noop_validator_strict() {
        let validator = NoOpValidator::new();
        let data = serde_json::json!({"test": "data"});

        // Should always pass
        validator
            .validate_strict("test-schema", None, &data)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_graceful_validator_no_fail() {
        let inner = Arc::new(NoOpValidator::new()) as Arc<dyn SchemaValidator>;
        let validator = GracefulValidator::new(inner, false);

        assert!(validator.is_enabled());
        assert_eq!(validator.name(), "GracefulValidator");

        let data = serde_json::json!({"test": "data"});
        let result = validator.validate("test-schema", None, &data).await.unwrap();
        assert!(result.is_valid());
    }

    #[tokio::test]
    async fn test_caching_validator() {
        let inner = Arc::new(NoOpValidator::new()) as Arc<dyn SchemaValidator>;
        let validator = CachingValidator::new(
            inner,
            std::time::Duration::from_secs(60),
            100,
        );

        assert_eq!(validator.name(), "CachingValidator");

        let data = serde_json::json!({"test": "data"});

        // First validation - not cached
        let result1 = validator.validate("test-schema", None, &data).await.unwrap();
        assert!(result1.is_valid());

        // Second validation - should be cached
        let result2 = validator.validate("test-schema", None, &data).await.unwrap();
        assert!(result2.is_valid());

        // Check cache stats
        let (entry_count, _) = validator.cache_stats();
        assert_eq!(entry_count, 1);
    }

    #[tokio::test]
    async fn test_caching_validator_cache_key() {
        let data1 = serde_json::json!({"test": "data1"});
        let data2 = serde_json::json!({"test": "data2"});

        let key1 = CachingValidator::cache_key("schema1", Some("v1"), &data1);
        let key2 = CachingValidator::cache_key("schema1", Some("v1"), &data1);
        let key3 = CachingValidator::cache_key("schema1", Some("v1"), &data2);
        let key4 = CachingValidator::cache_key("schema1", None, &data1);

        // Same schema, version, and data should produce same key
        assert_eq!(key1, key2);

        // Different data should produce different key
        assert_ne!(key1, key3);

        // Different version should produce different key
        assert_ne!(key1, key4);
    }

    #[tokio::test]
    async fn test_caching_validator_clear() {
        let inner = Arc::new(NoOpValidator::new()) as Arc<dyn SchemaValidator>;
        let validator = CachingValidator::new(
            inner,
            std::time::Duration::from_secs(60),
            100,
        );

        let data = serde_json::json!({"test": "data"});

        // Populate cache
        validator.validate("test-schema", None, &data).await.unwrap();
        let (entry_count, _) = validator.cache_stats();
        assert_eq!(entry_count, 1);

        // Clear cache
        validator.clear_cache().await;
        let (entry_count, _) = validator.cache_stats();
        assert_eq!(entry_count, 0);
    }
}

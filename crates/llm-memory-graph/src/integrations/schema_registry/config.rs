//! Configuration for Schema Registry integration

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Schema Registry client configuration
///
/// # Examples
///
/// ```rust
/// use llm_memory_graph::integrations::schema_registry::SchemaRegistryConfig;
///
/// // Create with default settings
/// let config = SchemaRegistryConfig::default();
///
/// // Create with custom settings
/// let config = SchemaRegistryConfig::new("https://schema-registry.example.com")
///     .with_api_key("my-api-key")
///     .with_timeout(Duration::from_secs(60))
///     .with_validation_enabled(true);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRegistryConfig {
    /// Base URL of the Schema Registry service
    pub base_url: String,

    /// API key for authentication
    pub api_key: Option<String>,

    /// Request timeout in seconds
    pub timeout_secs: u64,

    /// Maximum number of retry attempts
    pub retry_count: usize,

    /// Enable request/response logging
    pub enable_logging: bool,

    /// Enable schema validation (OPT-IN)
    ///
    /// When false, all validation calls will pass through without checking
    /// This allows graceful degradation when Schema Registry is unavailable
    pub validation_enabled: bool,

    /// Cache TTL for schema metadata in seconds
    ///
    /// Schemas are cached locally to reduce registry load
    /// Set to 0 to disable caching
    pub cache_ttl_secs: u64,

    /// Maximum cache size (number of schemas)
    pub max_cache_size: usize,

    /// Fail on validation errors vs. warn and continue
    ///
    /// When false, validation errors are logged but don't block operations
    pub fail_on_validation_error: bool,
}

impl Default for SchemaRegistryConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("SCHEMA_REGISTRY_URL")
                .unwrap_or_else(|_| "http://localhost:8081".to_string()),
            api_key: std::env::var("SCHEMA_REGISTRY_API_KEY").ok(),
            timeout_secs: 30,
            retry_count: 3,
            enable_logging: true,
            validation_enabled: false, // OPT-IN by default
            cache_ttl_secs: 300, // 5 minutes
            max_cache_size: 100,
            fail_on_validation_error: false, // Graceful degradation by default
        }
    }
}

impl SchemaRegistryConfig {
    /// Create a new Schema Registry configuration
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL of the Schema Registry service
    ///
    /// # Examples
    ///
    /// ```rust
    /// use llm_memory_graph::integrations::schema_registry::SchemaRegistryConfig;
    ///
    /// let config = SchemaRegistryConfig::new("https://schema-registry.example.com");
    /// ```
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    /// Set the API key for authentication
    ///
    /// # Examples
    ///
    /// ```rust
    /// use llm_memory_graph::integrations::schema_registry::SchemaRegistryConfig;
    ///
    /// let config = SchemaRegistryConfig::new("https://schema-registry.example.com")
    ///     .with_api_key("my-secret-key");
    /// ```
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set the request timeout
    ///
    /// # Examples
    ///
    /// ```rust
    /// use llm_memory_graph::integrations::schema_registry::SchemaRegistryConfig;
    /// use std::time::Duration;
    ///
    /// let config = SchemaRegistryConfig::new("https://schema-registry.example.com")
    ///     .with_timeout(Duration::from_secs(60));
    /// ```
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_secs = timeout.as_secs();
        self
    }

    /// Set the retry count
    ///
    /// # Examples
    ///
    /// ```rust
    /// use llm_memory_graph::integrations::schema_registry::SchemaRegistryConfig;
    ///
    /// let config = SchemaRegistryConfig::new("https://schema-registry.example.com")
    ///     .with_retry_count(5);
    /// ```
    pub fn with_retry_count(mut self, retry_count: usize) -> Self {
        self.retry_count = retry_count;
        self
    }

    /// Enable or disable request/response logging
    ///
    /// # Examples
    ///
    /// ```rust
    /// use llm_memory_graph::integrations::schema_registry::SchemaRegistryConfig;
    ///
    /// let config = SchemaRegistryConfig::new("https://schema-registry.example.com")
    ///     .with_logging(false);
    /// ```
    pub fn with_logging(mut self, enable: bool) -> Self {
        self.enable_logging = enable;
        self
    }

    /// Enable or disable schema validation (OPT-IN)
    ///
    /// When enabled, all data will be validated against registered schemas.
    /// When disabled, validation calls pass through without checking.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use llm_memory_graph::integrations::schema_registry::SchemaRegistryConfig;
    ///
    /// // Enable validation
    /// let config = SchemaRegistryConfig::new("https://schema-registry.example.com")
    ///     .with_validation_enabled(true);
    /// ```
    pub fn with_validation_enabled(mut self, enabled: bool) -> Self {
        self.validation_enabled = enabled;
        self
    }

    /// Set the cache TTL for schema metadata
    ///
    /// # Examples
    ///
    /// ```rust
    /// use llm_memory_graph::integrations::schema_registry::SchemaRegistryConfig;
    /// use std::time::Duration;
    ///
    /// let config = SchemaRegistryConfig::new("https://schema-registry.example.com")
    ///     .with_cache_ttl(Duration::from_secs(600)); // 10 minutes
    /// ```
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl_secs = ttl.as_secs();
        self
    }

    /// Set the maximum cache size
    ///
    /// # Examples
    ///
    /// ```rust
    /// use llm_memory_graph::integrations::schema_registry::SchemaRegistryConfig;
    ///
    /// let config = SchemaRegistryConfig::new("https://schema-registry.example.com")
    ///     .with_max_cache_size(200);
    /// ```
    pub fn with_max_cache_size(mut self, size: usize) -> Self {
        self.max_cache_size = size;
        self
    }

    /// Set whether to fail on validation errors
    ///
    /// When true, validation errors will cause operations to fail.
    /// When false, validation errors are logged but don't block operations (graceful degradation).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use llm_memory_graph::integrations::schema_registry::SchemaRegistryConfig;
    ///
    /// let config = SchemaRegistryConfig::new("https://schema-registry.example.com")
    ///     .with_fail_on_validation_error(true);
    /// ```
    pub fn with_fail_on_validation_error(mut self, fail: bool) -> Self {
        self.fail_on_validation_error = fail;
        self
    }

    /// Get the timeout as a Duration
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }

    /// Get the cache TTL as a Duration
    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache_ttl_secs)
    }

    /// Check if caching is enabled
    pub fn is_cache_enabled(&self) -> bool {
        self.cache_ttl_secs > 0 && self.max_cache_size > 0
    }

    /// Validate configuration settings
    ///
    /// Returns an error if the configuration is invalid
    pub fn validate(&self) -> Result<(), String> {
        if self.base_url.is_empty() {
            return Err("base_url cannot be empty".to_string());
        }

        if self.timeout_secs == 0 {
            return Err("timeout_secs must be greater than 0".to_string());
        }

        if self.cache_ttl_secs > 0 && self.max_cache_size == 0 {
            return Err("max_cache_size must be greater than 0 when caching is enabled".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = SchemaRegistryConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.retry_count, 3);
        assert!(config.enable_logging);
        assert!(!config.validation_enabled); // OPT-IN by default
        assert_eq!(config.cache_ttl_secs, 300);
        assert_eq!(config.max_cache_size, 100);
        assert!(!config.fail_on_validation_error); // Graceful degradation by default
    }

    #[test]
    fn test_config_builder() {
        let config = SchemaRegistryConfig::new("https://registry.example.com")
            .with_api_key("test-key")
            .with_timeout(Duration::from_secs(60))
            .with_retry_count(5)
            .with_logging(false)
            .with_validation_enabled(true)
            .with_cache_ttl(Duration::from_secs(600))
            .with_max_cache_size(200)
            .with_fail_on_validation_error(true);

        assert_eq!(config.base_url, "https://registry.example.com");
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.retry_count, 5);
        assert!(!config.enable_logging);
        assert!(config.validation_enabled);
        assert_eq!(config.cache_ttl_secs, 600);
        assert_eq!(config.max_cache_size, 200);
        assert!(config.fail_on_validation_error);
    }

    #[test]
    fn test_config_helpers() {
        let config = SchemaRegistryConfig::new("https://registry.example.com")
            .with_timeout(Duration::from_secs(45))
            .with_cache_ttl(Duration::from_secs(300));

        assert_eq!(config.timeout(), Duration::from_secs(45));
        assert_eq!(config.cache_ttl(), Duration::from_secs(300));
        assert!(config.is_cache_enabled());
    }

    #[test]
    fn test_config_cache_disabled() {
        let config = SchemaRegistryConfig::new("https://registry.example.com")
            .with_cache_ttl(Duration::from_secs(0));

        assert!(!config.is_cache_enabled());

        let config = SchemaRegistryConfig::new("https://registry.example.com")
            .with_max_cache_size(0);

        assert!(!config.is_cache_enabled());
    }

    #[test]
    fn test_config_validation() {
        // Valid config
        let config = SchemaRegistryConfig::new("https://registry.example.com");
        assert!(config.validate().is_ok());

        // Invalid: empty base_url
        let config = SchemaRegistryConfig {
            base_url: String::new(),
            ..Default::default()
        };
        assert!(config.validate().is_err());

        // Invalid: zero timeout
        let config = SchemaRegistryConfig::new("https://registry.example.com")
            .with_timeout(Duration::from_secs(0));
        assert!(config.validate().is_err());

        // Invalid: cache enabled but max_cache_size is 0
        let config = SchemaRegistryConfig::new("https://registry.example.com")
            .with_cache_ttl(Duration::from_secs(300))
            .with_max_cache_size(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_from_env() {
        // Note: This test assumes environment variables are not set
        // In a real scenario, you'd use a test harness that sets env vars
        let config = SchemaRegistryConfig::default();

        // Should fall back to default values when env vars are not set
        assert!(config.base_url.contains("localhost") || config.base_url.starts_with("http"));
    }
}

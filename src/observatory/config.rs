//! Configuration for Observatory integration

use serde::{Deserialize, Serialize};

/// Configuration for Observatory integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservatoryConfig {
    /// Enable event publishing
    pub enabled: bool,

    /// Event batching size
    pub batch_size: usize,

    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,

    /// Enable metrics collection
    pub enable_metrics: bool,

    /// Additional configuration (for custom publishers)
    #[serde(default)]
    pub custom_config: std::collections::HashMap<String, String>,
}

impl Default for ObservatoryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            batch_size: 100,
            flush_interval_ms: 1000,
            enable_metrics: true,
            custom_config: std::collections::HashMap::new(),
        }
    }
}

impl ObservatoryConfig {
    /// Create a new observatory configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable Observatory integration
    pub fn enabled(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Set batch size for event publishing
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Set flush interval in milliseconds
    pub fn with_flush_interval(mut self, interval_ms: u64) -> Self {
        self.flush_interval_ms = interval_ms;
        self
    }

    /// Enable or disable metrics collection
    pub fn with_metrics(mut self, enabled: bool) -> Self {
        self.enable_metrics = enabled;
        self
    }

    /// Add custom configuration parameter
    pub fn with_custom(mut self, key: String, value: String) -> Self {
        self.custom_config.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ObservatoryConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.flush_interval_ms, 1000);
        assert!(config.enable_metrics);
    }

    #[test]
    fn test_builder_pattern() {
        let config = ObservatoryConfig::new()
            .enabled()
            .with_batch_size(50)
            .with_flush_interval(500)
            .with_metrics(false);

        assert!(config.enabled);
        assert_eq!(config.batch_size, 50);
        assert_eq!(config.flush_interval_ms, 500);
        assert!(!config.enable_metrics);
    }

    #[test]
    fn test_custom_config() {
        let config = ObservatoryConfig::new()
            .with_custom("kafka_brokers".to_string(), "localhost:9092".to_string())
            .with_custom("topic".to_string(), "events".to_string());

        assert_eq!(config.custom_config.len(), 2);
        assert_eq!(
            config.custom_config.get("kafka_brokers"),
            Some(&"localhost:9092".to_string())
        );
    }
}

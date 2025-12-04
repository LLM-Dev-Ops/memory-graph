//! Configuration provider trait and implementations

use super::client::ConfigManagerClient;
use super::types::{ConfigManagerConfig, MemoryGraphConfig};
use crate::integrations::IntegrationError;
use crate::Config;
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Configuration provider trait
///
/// Defines the interface for fetching and managing configurations
/// from different sources (remote, local, environment variables).
#[async_trait]
pub trait ConfigProvider: Send + Sync {
    /// Fetch configuration from the provider
    ///
    /// # Errors
    /// Returns an error if the configuration cannot be fetched.
    async fn fetch_config(&self) -> Result<MemoryGraphConfig, IntegrationError>;

    /// Check if the provider is available
    ///
    /// # Errors
    /// Returns an error if the availability check fails.
    async fn is_available(&self) -> Result<bool, IntegrationError>;

    /// Get the provider name for logging
    fn provider_name(&self) -> &str;

    /// Refresh the configuration (re-fetch from source)
    ///
    /// # Errors
    /// Returns an error if the configuration cannot be refreshed.
    async fn refresh(&self) -> Result<MemoryGraphConfig, IntegrationError> {
        self.fetch_config().await
    }
}

/// Remote configuration provider using Config Manager HTTP API
pub struct RemoteConfigProvider {
    client: ConfigManagerClient,
    app_name: String,
    environment: String,
}

impl RemoteConfigProvider {
    /// Create a new remote configuration provider
    ///
    /// # Errors
    /// Returns an error if the client cannot be created.
    pub fn new(
        config: ConfigManagerConfig,
        app_name: impl Into<String>,
        environment: impl Into<String>,
    ) -> Result<Self, IntegrationError> {
        let client = ConfigManagerClient::new(config)?;
        Ok(Self {
            client,
            app_name: app_name.into(),
            environment: environment.into(),
        })
    }

    /// Get the underlying client
    pub fn client(&self) -> &ConfigManagerClient {
        &self.client
    }
}

#[async_trait]
impl ConfigProvider for RemoteConfigProvider {
    async fn fetch_config(&self) -> Result<MemoryGraphConfig, IntegrationError> {
        debug!(
            "Fetching config from remote: app={}, env={}",
            self.app_name, self.environment
        );
        self.client
            .get_config(&self.app_name, &self.environment)
            .await
    }

    async fn is_available(&self) -> Result<bool, IntegrationError> {
        match self.client.health_check().await {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("Remote config provider unavailable: {}", e);
                Ok(false)
            }
        }
    }

    fn provider_name(&self) -> &str {
        "remote"
    }
}

/// Local configuration provider using file-based config
pub struct LocalConfigProvider {
    config: MemoryGraphConfig,
}

impl LocalConfigProvider {
    /// Create a new local configuration provider
    pub fn new(config: MemoryGraphConfig) -> Self {
        Self { config }
    }

    /// Create from a JSON file
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file(path: impl Into<PathBuf>) -> Result<Self, IntegrationError> {
        let path = path.into();
        let content = std::fs::read_to_string(&path).map_err(|e| {
            IntegrationError::InvalidConfig(format!("Failed to read config file: {}", e))
        })?;

        let config: MemoryGraphConfig = serde_json::from_str(&content).map_err(|e| {
            IntegrationError::Serialization(format!("Failed to parse config: {}", e))
        })?;

        Ok(Self { config })
    }

    /// Create from the default config structure
    pub fn from_default_config(config: &Config) -> Self {
        let now = chrono::Utc::now();
        let memory_graph_config = MemoryGraphConfig {
            config_id: uuid::Uuid::new_v4().to_string(),
            version: "1.0.0".to_string(),
            app_name: "llm-memory-graph".to_string(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string()),
            storage: super::types::StorageConfig {
                path: config
                    .path
                    .to_str()
                    .unwrap_or("./data/graph.db")
                    .to_string(),
                cache_size_mb: config.cache_size_mb,
                enable_wal: config.enable_wal,
                compression_level: config.compression_level,
                flush_interval_ms: config.flush_interval_ms,
                enable_mmap: false,
                page_size_bytes: None,
            },
            retention: super::types::RetentionConfig::default(),
            pruning: super::types::PruningConfig::default(),
            limits: super::types::GraphLimitsConfig::default(),
            performance: super::types::PerformanceConfig::default(),
            created_at: now,
            updated_at: now,
            metadata: std::collections::HashMap::new(),
        };

        Self {
            config: memory_graph_config,
        }
    }
}

#[async_trait]
impl ConfigProvider for LocalConfigProvider {
    async fn fetch_config(&self) -> Result<MemoryGraphConfig, IntegrationError> {
        debug!("Using local configuration");
        Ok(self.config.clone())
    }

    async fn is_available(&self) -> Result<bool, IntegrationError> {
        Ok(true)
    }

    fn provider_name(&self) -> &str {
        "local"
    }
}

/// Environment variable configuration provider
pub struct EnvConfigProvider {
    base_config: MemoryGraphConfig,
}

impl EnvConfigProvider {
    /// Create a new environment configuration provider
    pub fn new(base_config: MemoryGraphConfig) -> Self {
        Self { base_config }
    }

    /// Override configuration with environment variables
    fn apply_env_overrides(&self, mut config: MemoryGraphConfig) -> MemoryGraphConfig {
        // Storage overrides
        if let Ok(path) = std::env::var("MEMORY_GRAPH_DB_PATH") {
            config.storage.path = path;
        }
        if let Ok(cache_size) = std::env::var("MEMORY_GRAPH_CACHE_SIZE_MB") {
            if let Ok(size) = cache_size.parse::<usize>() {
                config.storage.cache_size_mb = size;
            }
        }
        if let Ok(wal) = std::env::var("MEMORY_GRAPH_ENABLE_WAL") {
            config.storage.enable_wal = wal.to_lowercase() == "true";
        }
        if let Ok(compression) = std::env::var("MEMORY_GRAPH_COMPRESSION_LEVEL") {
            if let Ok(level) = compression.parse::<u8>() {
                config.storage.compression_level = level.min(9);
            }
        }

        // Retention overrides
        if let Ok(days) = std::env::var("MEMORY_GRAPH_SESSION_RETENTION_DAYS") {
            if let Ok(retention) = days.parse::<u32>() {
                config.retention.session_retention_days = retention;
            }
        }
        if let Ok(auto_archive) = std::env::var("MEMORY_GRAPH_AUTO_ARCHIVE") {
            config.retention.auto_archive = auto_archive.to_lowercase() == "true";
        }

        // Pruning overrides
        if let Ok(enabled) = std::env::var("MEMORY_GRAPH_PRUNING_ENABLED") {
            config.pruning.enabled = enabled.to_lowercase() == "true";
        }
        if let Ok(days) = std::env::var("MEMORY_GRAPH_PRUNE_AFTER_DAYS") {
            if let Ok(prune_days) = days.parse::<u32>() {
                config.pruning.prune_sessions_after_days = prune_days;
            }
        }

        // Limits overrides
        if let Ok(max_nodes) = std::env::var("MEMORY_GRAPH_MAX_NODES") {
            if let Ok(nodes) = max_nodes.parse::<usize>() {
                config.limits.max_nodes = Some(nodes);
            }
        }
        if let Ok(max_edges) = std::env::var("MEMORY_GRAPH_MAX_EDGES") {
            if let Ok(edges) = max_edges.parse::<usize>() {
                config.limits.max_edges = Some(edges);
            }
        }

        // Performance overrides
        if let Ok(cache_enabled) = std::env::var("MEMORY_GRAPH_QUERY_CACHE_ENABLED") {
            config.performance.enable_query_cache = cache_enabled.to_lowercase() == "true";
        }
        if let Ok(cache_size) = std::env::var("MEMORY_GRAPH_QUERY_CACHE_SIZE") {
            if let Ok(size) = cache_size.parse::<usize>() {
                config.performance.query_cache_size = size;
            }
        }

        config
    }
}

#[async_trait]
impl ConfigProvider for EnvConfigProvider {
    async fn fetch_config(&self) -> Result<MemoryGraphConfig, IntegrationError> {
        debug!("Applying environment variable overrides");
        let config = self.apply_env_overrides(self.base_config.clone());
        Ok(config)
    }

    async fn is_available(&self) -> Result<bool, IntegrationError> {
        Ok(true)
    }

    fn provider_name(&self) -> &str {
        "env"
    }
}

/// Cascading configuration provider
///
/// Attempts to fetch configuration from multiple providers in priority order:
/// 1. Local file configuration
/// 2. Environment variable overrides
/// 3. Remote Config Manager
/// 4. Default fallback
pub struct CascadingConfigProvider {
    providers: Vec<Box<dyn ConfigProvider>>,
}

impl CascadingConfigProvider {
    /// Create a new cascading configuration provider
    pub fn new(providers: Vec<Box<dyn ConfigProvider>>) -> Self {
        Self { providers }
    }

    /// Create with default provider chain
    ///
    /// Priority: Local > Env > Remote > Default
    ///
    /// # Errors
    /// Returns an error if any provider fails to initialize.
    pub fn with_defaults(
        base_config: &Config,
        remote_config: Option<ConfigManagerConfig>,
    ) -> Result<Self, IntegrationError> {
        let mut providers: Vec<Box<dyn ConfigProvider>> = Vec::new();

        // 1. Local file provider (highest priority)
        let local_provider = LocalConfigProvider::from_default_config(base_config);
        providers.push(Box::new(local_provider));

        // 2. Environment variable provider
        let env_base = LocalConfigProvider::from_default_config(base_config);
        let env_provider = EnvConfigProvider::new(env_base.config);
        providers.push(Box::new(env_provider));

        // 3. Remote provider (if configured)
        if let Some(remote_cfg) = remote_config {
            let app_name = std::env::var("APP_NAME")
                .unwrap_or_else(|_| "llm-memory-graph".to_string());
            let environment =
                std::env::var("ENVIRONMENT").unwrap_or_else(|_| "dev".to_string());

            match RemoteConfigProvider::new(remote_cfg, app_name, environment) {
                Ok(remote_provider) => {
                    providers.push(Box::new(remote_provider));
                }
                Err(e) => {
                    warn!("Failed to initialize remote config provider: {}", e);
                }
            }
        }

        Ok(Self { providers })
    }

    /// Fetch configuration with cascading fallback
    ///
    /// Tries each provider in order until one succeeds.
    ///
    /// # Errors
    /// Returns an error if all providers fail.
    pub async fn fetch_with_fallback(&self) -> Result<MemoryGraphConfig, IntegrationError> {
        let mut last_error = None;

        for provider in &self.providers {
            debug!("Trying config provider: {}", provider.provider_name());

            match provider.fetch_config().await {
                Ok(config) => {
                    info!(
                        "Successfully fetched config from provider: {}",
                        provider.provider_name()
                    );
                    return Ok(config);
                }
                Err(e) => {
                    warn!(
                        "Provider {} failed: {}",
                        provider.provider_name(),
                        e
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            IntegrationError::InvalidConfig("All config providers failed".to_string())
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_config_provider_creation() {
        let config = Config::default();
        let provider = LocalConfigProvider::from_default_config(&config);
        assert_eq!(provider.provider_name(), "local");
    }

    #[tokio::test]
    async fn test_local_config_provider_fetch() {
        let config = Config::default();
        let provider = LocalConfigProvider::from_default_config(&config);
        let result = provider.fetch_config().await;
        assert!(result.is_ok());

        let memory_config = result.unwrap();
        assert_eq!(memory_config.app_name, "llm-memory-graph");
    }

    #[tokio::test]
    async fn test_local_config_provider_availability() {
        let config = Config::default();
        let provider = LocalConfigProvider::from_default_config(&config);
        let available = provider.is_available().await.unwrap();
        assert!(available);
    }

    #[tokio::test]
    async fn test_env_config_provider() {
        let base_config = Config::default();
        let local_provider = LocalConfigProvider::from_default_config(&base_config);
        let env_provider = EnvConfigProvider::new(local_provider.config);

        let result = env_provider.fetch_config().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cascading_provider_with_defaults() {
        let base_config = Config::default();
        let provider = CascadingConfigProvider::with_defaults(&base_config, None);
        assert!(provider.is_ok());

        let cascading = provider.unwrap();
        let result = cascading.fetch_with_fallback().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_env_overrides() {
        std::env::set_var("MEMORY_GRAPH_CACHE_SIZE_MB", "256");
        std::env::set_var("MEMORY_GRAPH_ENABLE_WAL", "false");

        let base_config = Config::default();
        let local_provider = LocalConfigProvider::from_default_config(&base_config);
        let env_provider = EnvConfigProvider::new(local_provider.config);

        let config = env_provider.apply_env_overrides(env_provider.base_config.clone());
        assert_eq!(config.storage.cache_size_mb, 256);
        assert!(!config.storage.enable_wal);

        // Cleanup
        std::env::remove_var("MEMORY_GRAPH_CACHE_SIZE_MB");
        std::env::remove_var("MEMORY_GRAPH_ENABLE_WAL");
    }
}

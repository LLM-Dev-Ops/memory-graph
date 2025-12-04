//! Type definitions for Config Manager integration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration as StdDuration;

/// Config Manager client configuration
#[derive(Debug, Clone)]
pub struct ConfigManagerConfig {
    /// Base URL of the Config Manager service
    pub base_url: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum number of retry attempts
    pub retry_count: usize,
    /// Enable request/response logging
    pub enable_logging: bool,
    /// Cache TTL for configurations in seconds
    pub cache_ttl_secs: u64,
    /// Enable automatic config refresh
    pub auto_refresh: bool,
}

impl Default for ConfigManagerConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("CONFIG_MANAGER_URL")
                .unwrap_or_else(|_| "http://localhost:7070".to_string()),
            api_key: std::env::var("CONFIG_MANAGER_API_KEY").ok(),
            timeout_secs: 30,
            retry_count: 3,
            enable_logging: true,
            cache_ttl_secs: 300, // 5 minutes
            auto_refresh: true,
        }
    }
}

impl ConfigManagerConfig {
    /// Create a new config manager configuration
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    /// Set the API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set the timeout in seconds
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Set the retry count
    pub fn with_retry_count(mut self, retry_count: usize) -> Self {
        self.retry_count = retry_count;
        self
    }

    /// Enable or disable logging
    pub fn with_logging(mut self, enable: bool) -> Self {
        self.enable_logging = enable;
        self
    }

    /// Set cache TTL in seconds
    pub fn with_cache_ttl(mut self, ttl_secs: u64) -> Self {
        self.cache_ttl_secs = ttl_secs;
        self
    }

    /// Enable or disable auto-refresh
    pub fn with_auto_refresh(mut self, enable: bool) -> Self {
        self.auto_refresh = enable;
        self
    }
}

/// Memory Graph configuration schema from Config Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGraphConfig {
    /// Configuration ID
    pub config_id: String,
    /// Configuration version
    pub version: String,
    /// Application or service name
    pub app_name: String,
    /// Environment (dev, staging, prod)
    pub environment: String,
    /// Storage configuration
    pub storage: StorageConfig,
    /// Retention policies
    pub retention: RetentionConfig,
    /// Pruning configuration
    pub pruning: PruningConfig,
    /// Graph limits and constraints
    pub limits: GraphLimitsConfig,
    /// Performance tuning
    #[serde(default)]
    pub performance: PerformanceConfig,
    /// When this configuration was created
    pub created_at: DateTime<Utc>,
    /// When this configuration was last updated
    pub updated_at: DateTime<Utc>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Storage backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Database path (can be overridden by local config)
    pub path: String,
    /// Cache size in megabytes
    pub cache_size_mb: usize,
    /// Enable write-ahead logging
    pub enable_wal: bool,
    /// Compression level (0-9)
    pub compression_level: u8,
    /// Flush interval in milliseconds
    pub flush_interval_ms: u64,
    /// Enable memory mapping
    #[serde(default)]
    pub enable_mmap: bool,
    /// Page size in bytes
    #[serde(default)]
    pub page_size_bytes: Option<usize>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            path: "./data/graph.db".to_string(),
            cache_size_mb: 100,
            enable_wal: true,
            compression_level: 3,
            flush_interval_ms: 1000,
            enable_mmap: false,
            page_size_bytes: None,
        }
    }
}

/// Retention policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// How long to keep session data (in days)
    pub session_retention_days: u32,
    /// How long to keep prompt data (in days)
    pub prompt_retention_days: u32,
    /// How long to keep response data (in days)
    pub response_retention_days: u32,
    /// How long to keep agent data (in days)
    pub agent_retention_days: u32,
    /// Enable automatic archival to Data Vault
    pub auto_archive: bool,
    /// Archive after specified days
    pub archive_after_days: u32,
    /// Enable automatic deletion
    pub auto_delete: bool,
    /// Compliance level
    #[serde(default)]
    pub compliance_level: String,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            session_retention_days: 90,
            prompt_retention_days: 90,
            response_retention_days: 90,
            agent_retention_days: 365,
            auto_archive: false,
            archive_after_days: 90,
            auto_delete: false,
            compliance_level: "standard".to_string(),
        }
    }
}

/// Pruning threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruningConfig {
    /// Enable automatic pruning
    pub enabled: bool,
    /// Prune sessions older than specified days
    pub prune_sessions_after_days: u32,
    /// Prune orphaned nodes (nodes not connected to any session)
    pub prune_orphaned_nodes: bool,
    /// Prune interval in hours
    pub prune_interval_hours: u32,
    /// Maximum number of nodes to prune per batch
    pub prune_batch_size: usize,
    /// Prune during off-peak hours only
    pub off_peak_only: bool,
    /// Off-peak hours start (24-hour format)
    #[serde(default)]
    pub off_peak_start_hour: Option<u8>,
    /// Off-peak hours end (24-hour format)
    #[serde(default)]
    pub off_peak_end_hour: Option<u8>,
}

impl Default for PruningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            prune_sessions_after_days: 180,
            prune_orphaned_nodes: true,
            prune_interval_hours: 24,
            prune_batch_size: 1000,
            off_peak_only: false,
            off_peak_start_hour: None,
            off_peak_end_hour: None,
        }
    }
}

/// Graph size limits and constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphLimitsConfig {
    /// Maximum number of nodes in the graph
    pub max_nodes: Option<usize>,
    /// Maximum number of edges in the graph
    pub max_edges: Option<usize>,
    /// Maximum number of active sessions
    pub max_active_sessions: Option<usize>,
    /// Maximum nodes per session
    pub max_nodes_per_session: Option<usize>,
    /// Maximum edges per session
    pub max_edges_per_session: Option<usize>,
    /// Maximum prompt size in bytes
    pub max_prompt_size_bytes: Option<usize>,
    /// Maximum response size in bytes
    pub max_response_size_bytes: Option<usize>,
    /// Warn when approaching limits (percentage threshold)
    #[serde(default)]
    pub warn_threshold_percent: u8,
}

impl Default for GraphLimitsConfig {
    fn default() -> Self {
        Self {
            max_nodes: None,
            max_edges: None,
            max_active_sessions: None,
            max_nodes_per_session: None,
            max_edges_per_session: None,
            max_prompt_size_bytes: Some(1_048_576), // 1 MB
            max_response_size_bytes: Some(10_485_760), // 10 MB
            warn_threshold_percent: 80,
        }
    }
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable query result caching
    pub enable_query_cache: bool,
    /// Query cache size (number of entries)
    pub query_cache_size: usize,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Metrics collection interval in seconds
    pub metrics_interval_secs: u32,
    /// Enable parallel query execution
    pub enable_parallel_queries: bool,
    /// Maximum parallel query threads
    pub max_parallel_threads: Option<usize>,
    /// Enable connection pooling
    pub enable_connection_pool: bool,
    /// Connection pool size
    pub connection_pool_size: usize,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_query_cache: true,
            query_cache_size: 1000,
            enable_metrics: true,
            metrics_interval_secs: 60,
            enable_parallel_queries: false,
            max_parallel_threads: None,
            enable_connection_pool: true,
            connection_pool_size: 10,
        }
    }
}

/// Configuration update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigUpdateRequest {
    /// Configuration to update
    pub config: MemoryGraphConfig,
    /// Update description/reason
    #[serde(default)]
    pub description: String,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Configuration fetch response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    /// The configuration
    pub config: MemoryGraphConfig,
    /// Response metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Configuration list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigListResponse {
    /// List of configurations
    pub configs: Vec<MemoryGraphConfig>,
    /// Total count
    pub total: usize,
    /// Current page
    #[serde(default)]
    pub page: usize,
    /// Page size
    #[serde(default)]
    pub page_size: usize,
}

/// Configuration version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigVersion {
    /// Version string (semver)
    pub version: String,
    /// When this version was created
    pub created_at: DateTime<Utc>,
    /// Version description
    #[serde(default)]
    pub description: String,
    /// Who created this version
    #[serde(default)]
    pub created_by: String,
}

/// Configuration health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// Service status
    pub status: String,
    /// Service version
    pub version: String,
    /// Uptime in seconds
    #[serde(default)]
    pub uptime_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_manager_config_builder() {
        let config = ConfigManagerConfig::new("http://config.example.com")
            .with_api_key("test-key")
            .with_timeout(60)
            .with_retry_count(5)
            .with_logging(false)
            .with_cache_ttl(600)
            .with_auto_refresh(false);

        assert_eq!(config.base_url, "http://config.example.com");
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.retry_count, 5);
        assert!(!config.enable_logging);
        assert_eq!(config.cache_ttl_secs, 600);
        assert!(!config.auto_refresh);
    }

    #[test]
    fn test_storage_config_defaults() {
        let config = StorageConfig::default();
        assert_eq!(config.cache_size_mb, 100);
        assert!(config.enable_wal);
        assert_eq!(config.compression_level, 3);
    }

    #[test]
    fn test_retention_config_defaults() {
        let config = RetentionConfig::default();
        assert_eq!(config.session_retention_days, 90);
        assert!(!config.auto_archive);
        assert!(!config.auto_delete);
    }

    #[test]
    fn test_pruning_config_defaults() {
        let config = PruningConfig::default();
        assert!(!config.enabled);
        assert!(config.prune_orphaned_nodes);
        assert_eq!(config.prune_batch_size, 1000);
    }

    #[test]
    fn test_graph_limits_config_defaults() {
        let config = GraphLimitsConfig::default();
        assert!(config.max_nodes.is_none());
        assert_eq!(config.max_prompt_size_bytes, Some(1_048_576));
        assert_eq!(config.warn_threshold_percent, 80);
    }

    #[test]
    fn test_performance_config_defaults() {
        let config = PerformanceConfig::default();
        assert!(config.enable_query_cache);
        assert_eq!(config.query_cache_size, 1000);
        assert!(config.enable_metrics);
    }

    #[test]
    fn test_memory_graph_config_serialization() {
        let config = MemoryGraphConfig {
            config_id: "cfg-123".to_string(),
            version: "1.0.0".to_string(),
            app_name: "test-app".to_string(),
            environment: "dev".to_string(),
            storage: StorageConfig::default(),
            retention: RetentionConfig::default(),
            pruning: PruningConfig::default(),
            limits: GraphLimitsConfig::default(),
            performance: PerformanceConfig::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: MemoryGraphConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.config_id, "cfg-123");
        assert_eq!(deserialized.version, "1.0.0");
    }
}

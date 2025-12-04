//! Configuration adapter for transforming between Config Manager schemas and local Config

use super::types::MemoryGraphConfig;
use crate::Config;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Configuration adapter for transforming configurations
///
/// Handles bidirectional transformation between:
/// - Remote Config Manager schema (MemoryGraphConfig)
/// - Local Config structure used by the memory graph
pub struct ConfigAdapter;

impl ConfigAdapter {
    /// Transform Config Manager configuration to local Config
    ///
    /// Applies the remote configuration while preserving local overrides
    /// and ensuring backward compatibility.
    pub fn to_local_config(remote_config: &MemoryGraphConfig) -> Config {
        debug!("Transforming remote config to local Config");

        // Start with storage configuration from remote
        let mut config = Config {
            path: PathBuf::from(&remote_config.storage.path),
            cache_size_mb: remote_config.storage.cache_size_mb,
            enable_wal: remote_config.storage.enable_wal,
            compression_level: remote_config.storage.compression_level,
            flush_interval_ms: remote_config.storage.flush_interval_ms,
        };

        // Apply local environment variable overrides (highest priority)
        config = Self::apply_local_overrides(config);

        info!(
            "Transformed config: path={}, cache={} MB, wal={}, compression={}",
            config.path.display(),
            config.cache_size_mb,
            config.enable_wal,
            config.compression_level
        );

        config
    }

    /// Transform local Config to Config Manager schema
    ///
    /// Creates a MemoryGraphConfig from the local configuration,
    /// preserving additional metadata and settings.
    pub fn to_remote_config(
        local_config: &Config,
        app_name: impl Into<String>,
        environment: impl Into<String>,
    ) -> MemoryGraphConfig {
        debug!("Transforming local Config to remote config");

        let now = chrono::Utc::now();

        MemoryGraphConfig {
            config_id: uuid::Uuid::new_v4().to_string(),
            version: "1.0.0".to_string(),
            app_name: app_name.into(),
            environment: environment.into(),
            storage: super::types::StorageConfig {
                path: local_config
                    .path
                    .to_str()
                    .unwrap_or("./data/graph.db")
                    .to_string(),
                cache_size_mb: local_config.cache_size_mb,
                enable_wal: local_config.enable_wal,
                compression_level: local_config.compression_level,
                flush_interval_ms: local_config.flush_interval_ms,
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
        }
    }

    /// Merge remote configuration with local overrides
    ///
    /// Priority: Local file > Environment variables > Remote config
    pub fn merge_configs(
        remote_config: &MemoryGraphConfig,
        local_config: &Config,
    ) -> Config {
        debug!("Merging remote and local configurations");

        // Start with remote config
        let mut merged = Self::to_local_config(remote_config);

        // Override with local config if explicitly set (non-default values)
        // This ensures local configuration takes precedence

        // If local path is not default, use it
        if local_config.path != PathBuf::from("./data/graph.db") {
            merged.path = local_config.path.clone();
        }

        // If local cache size is not default (100 MB), use it
        if local_config.cache_size_mb != 100 {
            merged.cache_size_mb = local_config.cache_size_mb;
        }

        // If local compression level is not default (3), use it
        if local_config.compression_level != 3 {
            merged.compression_level = local_config.compression_level;
        }

        // If local flush interval is not default (1000 ms), use it
        if local_config.flush_interval_ms != 1000 {
            merged.flush_interval_ms = local_config.flush_interval_ms;
        }

        info!("Merged configuration with local overrides");
        merged
    }

    /// Apply local environment variable overrides to configuration
    ///
    /// These take the highest priority and override both remote and local file configs.
    fn apply_local_overrides(mut config: Config) -> Config {
        // Database path override
        if let Ok(path) = std::env::var("MEMORY_GRAPH_DB_PATH") {
            debug!("Applying DB path override from env: {}", path);
            config.path = PathBuf::from(path);
        }

        // Cache size override
        if let Ok(cache_size) = std::env::var("MEMORY_GRAPH_CACHE_SIZE_MB") {
            if let Ok(size) = cache_size.parse::<usize>() {
                debug!("Applying cache size override from env: {} MB", size);
                config.cache_size_mb = size;
            }
        }

        // WAL override
        if let Ok(wal) = std::env::var("MEMORY_GRAPH_ENABLE_WAL") {
            let enable = wal.to_lowercase() == "true";
            debug!("Applying WAL override from env: {}", enable);
            config.enable_wal = enable;
        }

        // Compression level override
        if let Ok(compression) = std::env::var("MEMORY_GRAPH_COMPRESSION_LEVEL") {
            if let Ok(level) = compression.parse::<u8>() {
                let clamped = level.min(9);
                debug!("Applying compression override from env: {}", clamped);
                config.compression_level = clamped;
            }
        }

        // Flush interval override
        if let Ok(interval) = std::env::var("MEMORY_GRAPH_FLUSH_INTERVAL_MS") {
            if let Ok(ms) = interval.parse::<u64>() {
                debug!("Applying flush interval override from env: {} ms", ms);
                config.flush_interval_ms = ms;
            }
        }

        config
    }

    /// Validate configuration consistency
    ///
    /// Ensures that the configuration values are valid and consistent.
    pub fn validate_config(config: &Config) -> Result<(), String> {
        // Validate cache size (must be at least 1 MB)
        if config.cache_size_mb < 1 {
            return Err("Cache size must be at least 1 MB".to_string());
        }

        // Validate cache size (warn if too large, > 10 GB)
        if config.cache_size_mb > 10240 {
            warn!(
                "Cache size is very large: {} MB (> 10 GB)",
                config.cache_size_mb
            );
        }

        // Validate compression level (0-9)
        if config.compression_level > 9 {
            return Err("Compression level must be between 0 and 9".to_string());
        }

        // Validate flush interval (must be less than 1 hour)
        if config.flush_interval_ms > 3_600_000 {
            warn!(
                "Flush interval is very large: {} ms (> 1 hour)",
                config.flush_interval_ms
            );
        }

        // Validate path is not empty
        if config.path.as_os_str().is_empty() {
            return Err("Database path cannot be empty".to_string());
        }

        Ok(())
    }

    /// Extract retention policy information from remote config
    ///
    /// Returns retention settings that can be used for pruning and archival decisions.
    pub fn extract_retention_info(
        remote_config: &MemoryGraphConfig,
    ) -> RetentionInfo {
        RetentionInfo {
            session_retention_days: remote_config.retention.session_retention_days,
            prompt_retention_days: remote_config.retention.prompt_retention_days,
            response_retention_days: remote_config.retention.response_retention_days,
            agent_retention_days: remote_config.retention.agent_retention_days,
            auto_archive: remote_config.retention.auto_archive,
            archive_after_days: remote_config.retention.archive_after_days,
            auto_delete: remote_config.retention.auto_delete,
            compliance_level: remote_config.retention.compliance_level.clone(),
        }
    }

    /// Extract pruning configuration from remote config
    pub fn extract_pruning_info(
        remote_config: &MemoryGraphConfig,
    ) -> PruningInfo {
        PruningInfo {
            enabled: remote_config.pruning.enabled,
            prune_sessions_after_days: remote_config.pruning.prune_sessions_after_days,
            prune_orphaned_nodes: remote_config.pruning.prune_orphaned_nodes,
            prune_interval_hours: remote_config.pruning.prune_interval_hours,
            prune_batch_size: remote_config.pruning.prune_batch_size,
            off_peak_only: remote_config.pruning.off_peak_only,
            off_peak_start_hour: remote_config.pruning.off_peak_start_hour,
            off_peak_end_hour: remote_config.pruning.off_peak_end_hour,
        }
    }

    /// Extract graph limits from remote config
    pub fn extract_limits_info(
        remote_config: &MemoryGraphConfig,
    ) -> GraphLimitsInfo {
        GraphLimitsInfo {
            max_nodes: remote_config.limits.max_nodes,
            max_edges: remote_config.limits.max_edges,
            max_active_sessions: remote_config.limits.max_active_sessions,
            max_nodes_per_session: remote_config.limits.max_nodes_per_session,
            max_edges_per_session: remote_config.limits.max_edges_per_session,
            max_prompt_size_bytes: remote_config.limits.max_prompt_size_bytes,
            max_response_size_bytes: remote_config.limits.max_response_size_bytes,
            warn_threshold_percent: remote_config.limits.warn_threshold_percent,
        }
    }
}

/// Extracted retention policy information
#[derive(Debug, Clone)]
pub struct RetentionInfo {
    /// Session retention in days
    pub session_retention_days: u32,
    /// Prompt retention in days
    pub prompt_retention_days: u32,
    /// Response retention in days
    pub response_retention_days: u32,
    /// Agent retention in days
    pub agent_retention_days: u32,
    /// Enable automatic archival
    pub auto_archive: bool,
    /// Archive after specified days
    pub archive_after_days: u32,
    /// Enable automatic deletion
    pub auto_delete: bool,
    /// Compliance level
    pub compliance_level: String,
}

/// Extracted pruning configuration
#[derive(Debug, Clone)]
pub struct PruningInfo {
    /// Pruning enabled
    pub enabled: bool,
    /// Prune sessions after specified days
    pub prune_sessions_after_days: u32,
    /// Prune orphaned nodes
    pub prune_orphaned_nodes: bool,
    /// Pruning interval in hours
    pub prune_interval_hours: u32,
    /// Batch size for pruning
    pub prune_batch_size: usize,
    /// Only prune during off-peak hours
    pub off_peak_only: bool,
    /// Off-peak start hour (24-hour format)
    pub off_peak_start_hour: Option<u8>,
    /// Off-peak end hour (24-hour format)
    pub off_peak_end_hour: Option<u8>,
}

/// Extracted graph limits information
#[derive(Debug, Clone)]
pub struct GraphLimitsInfo {
    /// Maximum nodes in graph
    pub max_nodes: Option<usize>,
    /// Maximum edges in graph
    pub max_edges: Option<usize>,
    /// Maximum active sessions
    pub max_active_sessions: Option<usize>,
    /// Maximum nodes per session
    pub max_nodes_per_session: Option<usize>,
    /// Maximum edges per session
    pub max_edges_per_session: Option<usize>,
    /// Maximum prompt size in bytes
    pub max_prompt_size_bytes: Option<usize>,
    /// Maximum response size in bytes
    pub max_response_size_bytes: Option<usize>,
    /// Warning threshold percentage
    pub warn_threshold_percent: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_remote_config() -> MemoryGraphConfig {
        let now = chrono::Utc::now();
        MemoryGraphConfig {
            config_id: "test-123".to_string(),
            version: "1.0.0".to_string(),
            app_name: "test-app".to_string(),
            environment: "test".to_string(),
            storage: super::super::types::StorageConfig {
                path: "/tmp/test.db".to_string(),
                cache_size_mb: 200,
                enable_wal: false,
                compression_level: 5,
                flush_interval_ms: 2000,
                enable_mmap: true,
                page_size_bytes: Some(4096),
            },
            retention: super::super::types::RetentionConfig::default(),
            pruning: super::super::types::PruningConfig::default(),
            limits: super::super::types::GraphLimitsConfig::default(),
            performance: super::super::types::PerformanceConfig::default(),
            created_at: now,
            updated_at: now,
            metadata: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_to_local_config() {
        let remote_config = create_test_remote_config();
        let local_config = ConfigAdapter::to_local_config(&remote_config);

        assert_eq!(local_config.cache_size_mb, 200);
        assert!(!local_config.enable_wal);
        assert_eq!(local_config.compression_level, 5);
        assert_eq!(local_config.flush_interval_ms, 2000);
    }

    #[test]
    fn test_to_remote_config() {
        let local_config = Config::default();
        let remote_config = ConfigAdapter::to_remote_config(
            &local_config,
            "test-app",
            "dev",
        );

        assert_eq!(remote_config.app_name, "test-app");
        assert_eq!(remote_config.environment, "dev");
        assert_eq!(remote_config.storage.cache_size_mb, 100);
        assert!(remote_config.storage.enable_wal);
    }

    #[test]
    fn test_merge_configs() {
        let remote_config = create_test_remote_config();
        let mut local_config = Config::default();
        local_config.cache_size_mb = 256; // Override

        let merged = ConfigAdapter::merge_configs(&remote_config, &local_config);

        // Local override should take precedence
        assert_eq!(merged.cache_size_mb, 256);
    }

    #[test]
    fn test_validate_config() {
        let config = Config::default();
        assert!(ConfigAdapter::validate_config(&config).is_ok());

        let mut invalid_config = Config::default();
        invalid_config.cache_size_mb = 0;
        assert!(ConfigAdapter::validate_config(&invalid_config).is_err());

        let mut invalid_compression = Config::default();
        invalid_compression.compression_level = 15;
        assert!(ConfigAdapter::validate_config(&invalid_compression).is_err());
    }

    #[test]
    fn test_extract_retention_info() {
        let remote_config = create_test_remote_config();
        let retention = ConfigAdapter::extract_retention_info(&remote_config);

        assert_eq!(retention.session_retention_days, 90);
        assert!(!retention.auto_archive);
    }

    #[test]
    fn test_extract_pruning_info() {
        let remote_config = create_test_remote_config();
        let pruning = ConfigAdapter::extract_pruning_info(&remote_config);

        assert!(!pruning.enabled);
        assert!(pruning.prune_orphaned_nodes);
        assert_eq!(pruning.prune_batch_size, 1000);
    }

    #[test]
    fn test_extract_limits_info() {
        let remote_config = create_test_remote_config();
        let limits = ConfigAdapter::extract_limits_info(&remote_config);

        assert!(limits.max_nodes.is_none());
        assert_eq!(limits.warn_threshold_percent, 80);
    }

    #[test]
    fn test_apply_local_overrides() {
        std::env::set_var("MEMORY_GRAPH_CACHE_SIZE_MB", "512");
        std::env::set_var("MEMORY_GRAPH_COMPRESSION_LEVEL", "7");

        let config = Config::default();
        let overridden = ConfigAdapter::apply_local_overrides(config);

        assert_eq!(overridden.cache_size_mb, 512);
        assert_eq!(overridden.compression_level, 7);

        // Cleanup
        std::env::remove_var("MEMORY_GRAPH_CACHE_SIZE_MB");
        std::env::remove_var("MEMORY_GRAPH_COMPRESSION_LEVEL");
    }
}

//! Config Manager integration module
//!
//! Provides client functionality for integrating with the Config Manager service,
//! including remote configuration fetching, local config adaptation, and cascading
//! configuration providers with fallback support.
//!
//! # Features
//!
//! - **Remote Configuration**: Fetch configurations from Config Manager service
//! - **Local Configuration**: File-based configuration with environment overrides
//! - **Cascading Providers**: Multiple configuration sources with priority ordering
//! - **Backward Compatible**: Existing `Config` structure continues to work
//! - **Graceful Fallback**: Continues operation when Config Manager is unavailable
//!
//! # Configuration Priority
//!
//! Configurations are applied in the following priority order (highest to lowest):
//!
//! 1. **Local File Config** - Explicitly configured local settings
//! 2. **Environment Variables** - Runtime environment overrides
//! 3. **Remote Config Manager** - Centrally managed configurations
//! 4. **Default Values** - Built-in fallback defaults
//!
//! # Usage Example
//!
//! ```no_run
//! use llm_memory_graph::integrations::config_manager::{
//!     ConfigAdapter, ConfigManagerConfig, CascadingConfigProvider,
//! };
//! use llm_memory_graph::Config;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create base configuration
//! let base_config = Config::default();
//!
//! // Optional: Configure remote Config Manager
//! let remote_config = Some(ConfigManagerConfig::new("http://config-manager:7070")
//!     .with_api_key("your-api-key"));
//!
//! // Create cascading provider with automatic fallback
//! let provider = CascadingConfigProvider::with_defaults(&base_config, remote_config)?;
//!
//! // Fetch configuration with automatic fallback
//! let memory_graph_config = provider.fetch_with_fallback().await?;
//!
//! // Transform to local Config structure
//! let local_config = ConfigAdapter::to_local_config(&memory_graph_config);
//!
//! // Use the configuration
//! // let graph = MemoryGraph::open(local_config)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Environment Variables
//!
//! The following environment variables can override configuration settings:
//!
//! - `CONFIG_MANAGER_URL` - Config Manager service URL
//! - `CONFIG_MANAGER_API_KEY` - API key for authentication
//! - `APP_NAME` - Application name for config lookup
//! - `ENVIRONMENT` - Environment name (dev, staging, prod)
//! - `MEMORY_GRAPH_DB_PATH` - Database path override
//! - `MEMORY_GRAPH_CACHE_SIZE_MB` - Cache size override
//! - `MEMORY_GRAPH_ENABLE_WAL` - Enable/disable WAL
//! - `MEMORY_GRAPH_COMPRESSION_LEVEL` - Compression level (0-9)
//! - `MEMORY_GRAPH_FLUSH_INTERVAL_MS` - Flush interval override
//! - `MEMORY_GRAPH_SESSION_RETENTION_DAYS` - Session retention override
//! - `MEMORY_GRAPH_AUTO_ARCHIVE` - Enable automatic archival
//! - `MEMORY_GRAPH_PRUNING_ENABLED` - Enable pruning
//! - `MEMORY_GRAPH_PRUNE_AFTER_DAYS` - Pruning threshold
//! - `MEMORY_GRAPH_MAX_NODES` - Maximum nodes limit
//! - `MEMORY_GRAPH_MAX_EDGES` - Maximum edges limit
//! - `MEMORY_GRAPH_QUERY_CACHE_ENABLED` - Enable query cache
//! - `MEMORY_GRAPH_QUERY_CACHE_SIZE` - Query cache size
//!
//! # Configuration Schema
//!
//! The Config Manager uses a comprehensive schema that includes:
//!
//! - **Storage**: Database path, cache size, WAL, compression
//! - **Retention**: Data retention policies by node type
//! - **Pruning**: Automatic pruning thresholds and schedules
//! - **Limits**: Graph size constraints and warnings
//! - **Performance**: Caching, metrics, parallelization settings

pub mod adapter;
pub mod client;
pub mod provider;
pub mod types;

// Re-export main types
pub use adapter::{ConfigAdapter, GraphLimitsInfo, PruningInfo, RetentionInfo};
pub use client::ConfigManagerClient;
pub use provider::{
    CascadingConfigProvider, ConfigProvider, EnvConfigProvider, LocalConfigProvider,
    RemoteConfigProvider,
};
pub use types::{
    ConfigListResponse, ConfigManagerConfig, ConfigResponse, ConfigUpdateRequest, ConfigVersion,
    GraphLimitsConfig, HealthCheckResponse, MemoryGraphConfig, PerformanceConfig, PruningConfig,
    RetentionConfig, StorageConfig,
};

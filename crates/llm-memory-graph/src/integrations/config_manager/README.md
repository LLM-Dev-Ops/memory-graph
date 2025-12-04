# Config Manager Integration

This module provides a comprehensive configuration management integration for the LLM Memory Graph, enabling centralized configuration management with support for remote fetching, local overrides, and graceful fallback.

## Overview

The Config Manager integration implements a consumption layer that:

1. **Fetches configurations** from a remote Config Manager service
2. **Supports local overrides** via files and environment variables
3. **Provides graceful fallback** when remote service is unavailable
4. **Maintains backward compatibility** with existing `Config` structure
5. **Enables cascading configuration** with priority-based resolution

## Architecture

### Components

#### 1. **types.rs** - Configuration Schema Types
Defines the data structures for Config Manager integration:

- `ConfigManagerConfig` - Client configuration
- `MemoryGraphConfig` - Complete remote configuration schema
- `StorageConfig` - Storage backend settings
- `RetentionConfig` - Data retention policies
- `PruningConfig` - Automatic pruning configuration
- `GraphLimitsConfig` - Graph size constraints
- `PerformanceConfig` - Performance tuning settings

#### 2. **client.rs** - HTTP Client
Implements the HTTP client for Config Manager API:

- `ConfigManagerClient` - Main client with retry logic
- Configuration fetching by app/environment
- Configuration CRUD operations
- Version history management
- Health checking
- Local caching with TTL

#### 3. **provider.rs** - Configuration Providers
Defines the `ConfigProvider` trait and implementations:

- `RemoteConfigProvider` - Fetches from Config Manager service
- `LocalConfigProvider` - Uses local file-based configuration
- `EnvConfigProvider` - Applies environment variable overrides
- `CascadingConfigProvider` - Combines multiple providers with fallback

#### 4. **adapter.rs** - Configuration Adapter
Transforms between Config Manager and local schemas:

- `ConfigAdapter` - Bidirectional transformation utilities
- `RetentionInfo` - Extracted retention policy information
- `PruningInfo` - Extracted pruning configuration
- `GraphLimitsInfo` - Extracted graph limits
- Configuration validation and merging

#### 5. **mod.rs** - Module Exports
Exports all public types and provides module documentation.

## Configuration Priority

Configurations are resolved in the following order (highest to lowest priority):

1. **Local File Config** - Explicitly configured local settings
2. **Environment Variables** - Runtime environment overrides
3. **Remote Config Manager** - Centrally managed configurations
4. **Default Values** - Built-in fallback defaults

## Usage Examples

### Basic Usage

```rust
use llm_memory_graph::{Config, MemoryGraph};
use llm_memory_graph::integrations::config_manager::{
    ConfigAdapter, ConfigManagerConfig, CascadingConfigProvider,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create base configuration
    let base_config = Config::default();

    // Configure remote Config Manager (optional)
    let remote_config = Some(
        ConfigManagerConfig::new("http://config-manager:7070")
            .with_api_key("your-api-key")
            .with_cache_ttl(300)
    );

    // Create cascading provider with automatic fallback
    let provider = CascadingConfigProvider::with_defaults(&base_config, remote_config)?;

    // Fetch configuration with automatic fallback
    let memory_graph_config = provider.fetch_with_fallback().await?;

    // Transform to local Config structure
    let local_config = ConfigAdapter::to_local_config(&memory_graph_config);

    // Open memory graph with the configuration
    let graph = MemoryGraph::open(local_config)?;

    Ok(())
}
```

### Using Remote Provider Directly

```rust
use llm_memory_graph::integrations::config_manager::{
    ConfigManagerConfig, RemoteConfigProvider, ConfigProvider,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigManagerConfig::new("http://config-manager:7070")
        .with_api_key("your-api-key");

    let provider = RemoteConfigProvider::new(
        config,
        "llm-memory-graph",
        "production"
    )?;

    // Check if service is available
    if provider.is_available().await? {
        let config = provider.fetch_config().await?;
        println!("Fetched config version: {}", config.version);
    }

    Ok(())
}
```

### Using Local Provider with Environment Overrides

```rust
use llm_memory_graph::{Config};
use llm_memory_graph::integrations::config_manager::{
    LocalConfigProvider, EnvConfigProvider, ConfigProvider,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_config = Config::default();

    // Create local provider
    let local_provider = LocalConfigProvider::from_default_config(&base_config);

    // Create env provider with overrides
    let env_provider = EnvConfigProvider::new(local_provider.config);

    // Fetch with environment overrides applied
    let config = env_provider.fetch_config().await?;

    Ok(())
}
```

### Extracting Configuration Information

```rust
use llm_memory_graph::integrations::config_manager::ConfigAdapter;

// Extract retention policy information
let retention_info = ConfigAdapter::extract_retention_info(&remote_config);
println!("Session retention: {} days", retention_info.session_retention_days);
println!("Auto-archive: {}", retention_info.auto_archive);

// Extract pruning configuration
let pruning_info = ConfigAdapter::extract_pruning_info(&remote_config);
println!("Pruning enabled: {}", pruning_info.enabled);
println!("Prune after: {} days", pruning_info.prune_sessions_after_days);

// Extract graph limits
let limits_info = ConfigAdapter::extract_limits_info(&remote_config);
if let Some(max_nodes) = limits_info.max_nodes {
    println!("Maximum nodes: {}", max_nodes);
}
```

## Environment Variables

### Config Manager Connection

- `CONFIG_MANAGER_URL` - Config Manager service URL (default: `http://localhost:7070`)
- `CONFIG_MANAGER_API_KEY` - API key for authentication
- `APP_NAME` - Application name for config lookup (default: `llm-memory-graph`)
- `ENVIRONMENT` - Environment name (default: `dev`)

### Storage Configuration

- `MEMORY_GRAPH_DB_PATH` - Database path override
- `MEMORY_GRAPH_CACHE_SIZE_MB` - Cache size in megabytes
- `MEMORY_GRAPH_ENABLE_WAL` - Enable write-ahead logging (true/false)
- `MEMORY_GRAPH_COMPRESSION_LEVEL` - Compression level (0-9)
- `MEMORY_GRAPH_FLUSH_INTERVAL_MS` - Flush interval in milliseconds

### Retention Configuration

- `MEMORY_GRAPH_SESSION_RETENTION_DAYS` - Session retention period
- `MEMORY_GRAPH_AUTO_ARCHIVE` - Enable automatic archival (true/false)

### Pruning Configuration

- `MEMORY_GRAPH_PRUNING_ENABLED` - Enable automatic pruning (true/false)
- `MEMORY_GRAPH_PRUNE_AFTER_DAYS` - Prune sessions after specified days

### Limits Configuration

- `MEMORY_GRAPH_MAX_NODES` - Maximum nodes in graph
- `MEMORY_GRAPH_MAX_EDGES` - Maximum edges in graph

### Performance Configuration

- `MEMORY_GRAPH_QUERY_CACHE_ENABLED` - Enable query cache (true/false)
- `MEMORY_GRAPH_QUERY_CACHE_SIZE` - Query cache size (number of entries)

## API Reference

### ConfigManagerClient

```rust
impl ConfigManagerClient {
    pub fn new(config: ConfigManagerConfig) -> Result<Self, IntegrationError>;
    pub async fn get_config(&self, app_name: &str, environment: &str) -> Result<MemoryGraphConfig, IntegrationError>;
    pub async fn update_config(&self, request: ConfigUpdateRequest) -> Result<MemoryGraphConfig, IntegrationError>;
    pub async fn delete_config(&self, app_name: &str, environment: &str) -> Result<(), IntegrationError>;
    pub async fn health_check(&self) -> Result<HealthCheckResponse, IntegrationError>;
    pub fn clear_cache(&self);
}
```

### ConfigProvider Trait

```rust
#[async_trait]
pub trait ConfigProvider: Send + Sync {
    async fn fetch_config(&self) -> Result<MemoryGraphConfig, IntegrationError>;
    async fn is_available(&self) -> Result<bool, IntegrationError>;
    fn provider_name(&self) -> &str;
    async fn refresh(&self) -> Result<MemoryGraphConfig, IntegrationError>;
}
```

### ConfigAdapter

```rust
impl ConfigAdapter {
    pub fn to_local_config(remote_config: &MemoryGraphConfig) -> Config;
    pub fn to_remote_config(local_config: &Config, app_name: impl Into<String>, environment: impl Into<String>) -> MemoryGraphConfig;
    pub fn merge_configs(remote_config: &MemoryGraphConfig, local_config: &Config) -> Config;
    pub fn validate_config(config: &Config) -> Result<(), String>;
    pub fn extract_retention_info(remote_config: &MemoryGraphConfig) -> RetentionInfo;
    pub fn extract_pruning_info(remote_config: &MemoryGraphConfig) -> PruningInfo;
    pub fn extract_limits_info(remote_config: &MemoryGraphConfig) -> GraphLimitsInfo;
}
```

## Configuration Schema

### MemoryGraphConfig Structure

```rust
pub struct MemoryGraphConfig {
    pub config_id: String,
    pub version: String,
    pub app_name: String,
    pub environment: String,
    pub storage: StorageConfig,
    pub retention: RetentionConfig,
    pub pruning: PruningConfig,
    pub limits: GraphLimitsConfig,
    pub performance: PerformanceConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

### StorageConfig

- `path` - Database file path
- `cache_size_mb` - Cache size in megabytes
- `enable_wal` - Write-ahead logging enabled
- `compression_level` - Compression level (0-9)
- `flush_interval_ms` - Flush interval in milliseconds
- `enable_mmap` - Memory mapping enabled
- `page_size_bytes` - Page size in bytes (optional)

### RetentionConfig

- `session_retention_days` - Session retention period
- `prompt_retention_days` - Prompt retention period
- `response_retention_days` - Response retention period
- `agent_retention_days` - Agent retention period
- `auto_archive` - Enable automatic archival
- `archive_after_days` - Archive after specified days
- `auto_delete` - Enable automatic deletion
- `compliance_level` - Compliance level string

### PruningConfig

- `enabled` - Pruning enabled
- `prune_sessions_after_days` - Prune threshold in days
- `prune_orphaned_nodes` - Prune orphaned nodes
- `prune_interval_hours` - Pruning interval
- `prune_batch_size` - Batch size for pruning
- `off_peak_only` - Only prune during off-peak hours
- `off_peak_start_hour` - Off-peak start hour (optional)
- `off_peak_end_hour` - Off-peak end hour (optional)

### GraphLimitsConfig

- `max_nodes` - Maximum nodes in graph (optional)
- `max_edges` - Maximum edges in graph (optional)
- `max_active_sessions` - Maximum active sessions (optional)
- `max_nodes_per_session` - Maximum nodes per session (optional)
- `max_edges_per_session` - Maximum edges per session (optional)
- `max_prompt_size_bytes` - Maximum prompt size (optional)
- `max_response_size_bytes` - Maximum response size (optional)
- `warn_threshold_percent` - Warning threshold percentage

### PerformanceConfig

- `enable_query_cache` - Query cache enabled
- `query_cache_size` - Query cache size
- `enable_metrics` - Metrics collection enabled
- `metrics_interval_secs` - Metrics interval
- `enable_parallel_queries` - Parallel queries enabled
- `max_parallel_threads` - Maximum parallel threads (optional)
- `enable_connection_pool` - Connection pooling enabled
- `connection_pool_size` - Connection pool size

## Error Handling

All operations return `Result<T, IntegrationError>` where `IntegrationError` can be:

- `HttpError(String)` - HTTP request failed
- `AuthenticationError(String)` - Authentication failed
- `ApiError { status, message }` - API returned error response
- `ConnectionError(String)` - Connection failed
- `Timeout(u64)` - Request timed out
- `InvalidConfig(String)` - Invalid configuration
- `Serialization(String)` - Serialization/deserialization error

## Testing

The module includes comprehensive unit tests for:

- Configuration type serialization/deserialization
- Client creation and configuration
- Provider implementations and fallback behavior
- Adapter transformations and validation
- Environment variable overrides

Run tests with:

```bash
cargo test --package llm-memory-graph --lib integrations::config_manager
```

## Best Practices

1. **Use CascadingConfigProvider** for production deployments to ensure graceful fallback
2. **Enable caching** with appropriate TTL to reduce API calls
3. **Set environment variables** for deployment-specific overrides
4. **Monitor health checks** to detect Config Manager availability
5. **Validate configurations** before applying to catch errors early
6. **Use retry policies** to handle transient network failures
7. **Log configuration sources** for debugging and audit trails

## Integration with Other Modules

The Config Manager integration is designed to work seamlessly with:

- **Data Vault** - Retention policies inform archival decisions
- **Schema Registry** - Configuration versioning aligns with schema versions
- **Observatory** - Configuration changes can be tracked as events
- **LLM Registry** - Model-specific configurations can be managed centrally

## Future Enhancements

Potential future enhancements include:

1. **Configuration hot-reload** - Automatic configuration updates without restart
2. **Configuration diffs** - Track and display configuration changes
3. **Configuration templates** - Reusable configuration templates
4. **Configuration encryption** - End-to-end encryption for sensitive values
5. **Configuration validation** - Schema-based validation at API level
6. **Configuration rollback** - Ability to rollback to previous versions
7. **Multi-environment support** - Manage multiple environments from single config

## Support

For issues or questions regarding the Config Manager integration:

1. Check the module documentation and examples
2. Review the test cases for usage patterns
3. Consult the Config Manager service documentation
4. File an issue in the repository

## License

This module is part of the LLM Memory Graph project and is licensed under MIT OR Apache-2.0.

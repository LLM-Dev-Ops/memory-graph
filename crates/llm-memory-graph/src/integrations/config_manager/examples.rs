//! Example usage patterns for Config Manager integration
//!
//! This file contains comprehensive examples demonstrating how to use
//! the Config Manager integration in various scenarios.

#![allow(dead_code)]
#![allow(unused_variables)]

use super::{
    CascadingConfigProvider, ConfigAdapter, ConfigManagerClient, ConfigManagerConfig,
    ConfigProvider, LocalConfigProvider, RemoteConfigProvider,
};
use crate::Config;

/// Example 1: Basic usage with cascading providers and automatic fallback
///
/// This is the recommended approach for production use as it provides
/// graceful fallback when the Config Manager service is unavailable.
pub async fn example_cascading_provider() -> Result<(), Box<dyn std::error::Error>> {
    // Create base configuration (uses defaults)
    let base_config = Config::default();

    // Configure remote Config Manager (optional)
    let remote_config = Some(
        ConfigManagerConfig::new("http://config-manager:7070")
            .with_api_key("your-api-key")
            .with_cache_ttl(300) // 5 minutes cache
            .with_retry_count(3)
            .with_timeout(30),
    );

    // Create cascading provider with automatic fallback
    // Priority: Local > Env > Remote > Default
    let provider = CascadingConfigProvider::with_defaults(&base_config, remote_config)?;

    // Fetch configuration with automatic fallback
    let memory_graph_config = provider.fetch_with_fallback().await?;

    // Transform to local Config structure
    let local_config = ConfigAdapter::to_local_config(&memory_graph_config);

    println!("Configuration loaded successfully!");
    println!("Database path: {}", local_config.path.display());
    println!("Cache size: {} MB", local_config.cache_size_mb);

    // Use the configuration to open the memory graph
    // let graph = MemoryGraph::open(local_config)?;

    Ok(())
}

/// Example 2: Using remote provider directly
///
/// Useful when you want direct control over the Config Manager client
/// and want to handle errors explicitly.
pub async fn example_remote_provider() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigManagerConfig::new("http://config-manager:7070")
        .with_api_key("your-api-key")
        .with_logging(true);

    let provider = RemoteConfigProvider::new(config, "llm-memory-graph", "production")?;

    // Check if service is available
    if provider.is_available().await? {
        println!("Config Manager service is available");

        // Fetch configuration
        let config = provider.fetch_config().await?;
        println!("Fetched config version: {}", config.version);
        println!("Environment: {}", config.environment);
    } else {
        println!("Config Manager service is unavailable, using fallback");
        // Handle fallback logic
    }

    Ok(())
}

/// Example 3: Using local provider with environment overrides
///
/// Useful for development or when running without Config Manager service.
pub async fn example_local_provider() -> Result<(), Box<dyn std::error::Error>> {
    let base_config = Config::default();

    // Create local provider from default config
    let provider = LocalConfigProvider::from_default_config(&base_config);

    // Fetch configuration
    let config = provider.fetch_config().await?;

    println!("Using local configuration");
    println!("App name: {}", config.app_name);
    println!("Storage path: {}", config.storage.path);

    Ok(())
}

/// Example 4: Direct client usage for CRUD operations
///
/// Demonstrates how to use the Config Manager client directly
/// for creating, reading, updating, and deleting configurations.
pub async fn example_client_crud() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigManagerConfig::new("http://config-manager:7070")
        .with_api_key("your-api-key");

    let client = ConfigManagerClient::new(config)?;

    // Health check
    match client.health_check().await {
        Ok(health) => {
            println!("Service status: {}", health.status);
            println!("Service version: {}", health.version);
        }
        Err(e) => {
            eprintln!("Health check failed: {}", e);
            return Ok(());
        }
    }

    // Fetch configuration
    let config = client.get_config("llm-memory-graph", "production").await?;
    println!("Current config version: {}", config.version);

    // List all configurations
    let configs = client.list_configs(Some("llm-memory-graph"), None, None, None).await?;
    println!("Found {} configurations", configs.total);

    // Get version history
    let versions = client.get_version_history("llm-memory-graph", "production").await?;
    println!("Configuration has {} versions", versions.len());

    Ok(())
}

/// Example 5: Configuration validation
///
/// Shows how to validate configurations before applying them.
pub async fn example_validation() -> Result<(), Box<dyn std::error::Error>> {
    let base_config = Config::default();
    let local_provider = LocalConfigProvider::from_default_config(&base_config);
    let memory_graph_config = local_provider.fetch_config().await?;

    // Transform to local config
    let local_config = ConfigAdapter::to_local_config(&memory_graph_config);

    // Validate the configuration
    match ConfigAdapter::validate_config(&local_config) {
        Ok(()) => println!("Configuration is valid"),
        Err(e) => {
            eprintln!("Configuration validation failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Example 6: Extracting configuration information
///
/// Demonstrates how to extract specific configuration sections
/// for use in different parts of the application.
pub async fn example_extract_info() -> Result<(), Box<dyn std::error::Error>> {
    let base_config = Config::default();
    let local_provider = LocalConfigProvider::from_default_config(&base_config);
    let memory_graph_config = local_provider.fetch_config().await?;

    // Extract retention policy information
    let retention_info = ConfigAdapter::extract_retention_info(&memory_graph_config);
    println!("Retention Configuration:");
    println!("  Session retention: {} days", retention_info.session_retention_days);
    println!("  Prompt retention: {} days", retention_info.prompt_retention_days);
    println!("  Auto-archive: {}", retention_info.auto_archive);
    println!("  Archive after: {} days", retention_info.archive_after_days);

    // Extract pruning configuration
    let pruning_info = ConfigAdapter::extract_pruning_info(&memory_graph_config);
    println!("\nPruning Configuration:");
    println!("  Enabled: {}", pruning_info.enabled);
    println!("  Prune after: {} days", pruning_info.prune_sessions_after_days);
    println!("  Prune orphaned nodes: {}", pruning_info.prune_orphaned_nodes);
    println!("  Batch size: {}", pruning_info.prune_batch_size);

    // Extract graph limits
    let limits_info = ConfigAdapter::extract_limits_info(&memory_graph_config);
    println!("\nGraph Limits:");
    if let Some(max_nodes) = limits_info.max_nodes {
        println!("  Maximum nodes: {}", max_nodes);
    } else {
        println!("  Maximum nodes: unlimited");
    }
    if let Some(max_edges) = limits_info.max_edges {
        println!("  Maximum edges: {}", max_edges);
    } else {
        println!("  Maximum edges: unlimited");
    }
    println!("  Warn threshold: {}%", limits_info.warn_threshold_percent);

    Ok(())
}

/// Example 7: Merging configurations
///
/// Shows how to merge remote and local configurations with proper priority.
pub async fn example_merge_configs() -> Result<(), Box<dyn std::error::Error>> {
    // Local configuration with explicit settings
    let local_config = Config::new("./data/prod.db")
        .with_cache_size(256)
        .with_compression(5);

    // Fetch remote configuration
    let base_config = Config::default();
    let local_provider = LocalConfigProvider::from_default_config(&base_config);
    let remote_config = local_provider.fetch_config().await?;

    // Merge configurations (local takes precedence)
    let merged_config = ConfigAdapter::merge_configs(&remote_config, &local_config);

    println!("Merged configuration:");
    println!("  Path: {}", merged_config.path.display());
    println!("  Cache size: {} MB", merged_config.cache_size_mb);
    println!("  Compression: {}", merged_config.compression_level);

    Ok(())
}

/// Example 8: Using environment variables
///
/// Demonstrates how environment variables override configuration values.
pub async fn example_env_overrides() -> Result<(), Box<dyn std::error::Error>> {
    // Set environment variables (in practice, these would be set externally)
    // std::env::set_var("MEMORY_GRAPH_CACHE_SIZE_MB", "512");
    // std::env::set_var("MEMORY_GRAPH_COMPRESSION_LEVEL", "7");

    let base_config = Config::default();

    // Environment variables are automatically applied by EnvConfigProvider
    // when using CascadingConfigProvider
    let provider = CascadingConfigProvider::with_defaults(&base_config, None)?;
    let config = provider.fetch_with_fallback().await?;

    let local_config = ConfigAdapter::to_local_config(&config);

    println!("Configuration with environment overrides:");
    println!("  Cache size: {} MB", local_config.cache_size_mb);
    println!("  Compression: {}", local_config.compression_level);

    Ok(())
}

/// Example 9: Transforming local to remote config
///
/// Shows how to convert local configuration to Config Manager schema
/// for uploading to the service.
pub async fn example_local_to_remote() -> Result<(), Box<dyn std::error::Error>> {
    let local_config = Config::new("./data/graph.db")
        .with_cache_size(200)
        .with_wal(true)
        .with_compression(4)
        .with_flush_interval(2000);

    // Transform to remote config schema
    let remote_config = ConfigAdapter::to_remote_config(
        &local_config,
        "llm-memory-graph",
        "production",
    );

    println!("Remote configuration:");
    println!("  Config ID: {}", remote_config.config_id);
    println!("  Version: {}", remote_config.version);
    println!("  App name: {}", remote_config.app_name);
    println!("  Environment: {}", remote_config.environment);

    // This could be uploaded to Config Manager
    // let client = ConfigManagerClient::new(config)?;
    // client.update_config(ConfigUpdateRequest { config: remote_config, ... }).await?;

    Ok(())
}

/// Example 10: Error handling and retry
///
/// Demonstrates proper error handling and retry behavior.
pub async fn example_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigManagerConfig::new("http://config-manager:7070")
        .with_api_key("your-api-key")
        .with_retry_count(5) // Increased retries
        .with_timeout(30);

    let client = ConfigManagerClient::new(config)?;

    // Attempt to fetch configuration with automatic retries
    match client.get_config("llm-memory-graph", "production").await {
        Ok(config) => {
            println!("Successfully fetched config: version {}", config.version);
        }
        Err(e) => {
            eprintln!("Failed to fetch config after retries: {}", e);

            // Handle specific error types
            match e {
                crate::integrations::IntegrationError::ConnectionError(_) => {
                    println!("Network issue detected, using local fallback");
                }
                crate::integrations::IntegrationError::AuthenticationError(_) => {
                    println!("Authentication failed, check API key");
                }
                crate::integrations::IntegrationError::Timeout(_) => {
                    println!("Request timed out, service may be overloaded");
                }
                _ => {
                    println!("Other error occurred, using default config");
                }
            }
        }
    }

    Ok(())
}

/// Example 11: Complete production setup
///
/// Comprehensive example showing recommended production configuration.
pub async fn example_production_setup() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create base configuration
    let base_config = Config::new("./data/production.db")
        .with_cache_size(512) // 512 MB cache
        .with_wal(true) // Enable WAL for durability
        .with_compression(6) // Balanced compression
        .with_flush_interval(1000); // 1 second flush

    // 2. Configure remote Config Manager with production settings
    let remote_config = if let Ok(url) = std::env::var("CONFIG_MANAGER_URL") {
        let api_key = std::env::var("CONFIG_MANAGER_API_KEY")
            .expect("CONFIG_MANAGER_API_KEY must be set");

        Some(
            ConfigManagerConfig::new(url)
                .with_api_key(api_key)
                .with_cache_ttl(300) // 5 minute cache
                .with_retry_count(5) // More retries for production
                .with_timeout(30)
                .with_logging(true) // Enable logging
                .with_auto_refresh(true), // Auto-refresh on cache expiry
        )
    } else {
        None
    };

    // 3. Create cascading provider with fallback
    let provider = CascadingConfigProvider::with_defaults(&base_config, remote_config)?;

    // 4. Fetch configuration with automatic fallback
    let memory_graph_config = provider.fetch_with_fallback().await?;

    // 5. Validate configuration
    let local_config = ConfigAdapter::to_local_config(&memory_graph_config);
    ConfigAdapter::validate_config(&local_config)?;

    // 6. Log configuration details
    println!("Production configuration loaded:");
    println!("  Database: {}", local_config.path.display());
    println!("  Cache: {} MB", local_config.cache_size_mb);
    println!("  WAL: {}", local_config.enable_wal);
    println!("  Compression: {}", local_config.compression_level);

    // 7. Extract operational settings
    let retention_info = ConfigAdapter::extract_retention_info(&memory_graph_config);
    let pruning_info = ConfigAdapter::extract_pruning_info(&memory_graph_config);

    println!("\nOperational settings:");
    println!("  Session retention: {} days", retention_info.session_retention_days);
    println!("  Pruning enabled: {}", pruning_info.enabled);

    // 8. Open memory graph with validated config
    // let graph = MemoryGraph::open(local_config)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_provider_example() {
        let result = example_local_provider().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validation_example() {
        let result = example_validation().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extract_info_example() {
        let result = example_extract_info().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_local_to_remote_example() {
        let result = example_local_to_remote().await;
        assert!(result.is_ok());
    }
}

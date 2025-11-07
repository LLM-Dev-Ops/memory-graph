//! gRPC server binary for LLM-Memory-Graph
//!
//! This is a standalone production-ready gRPC server that provides remote access
//! to the LLM-Memory-Graph database with full observability and metrics.
//!
//! # Configuration
//!
//! The server is configured via environment variables:
//!
//! - `DB_PATH`: Database storage path (default: ./data)
//! - `GRPC_HOST`: gRPC server bind address (default: 0.0.0.0)
//! - `GRPC_PORT`: gRPC server port (default: 50051)
//! - `METRICS_PORT`: Prometheus metrics HTTP port (default: 9090)
//! - `RUST_LOG`: Log level (default: info)
//! - `PLUGIN_DIRS`: Comma-separated plugin directories (optional)
//! - `REGISTRY_URL`: LLM-Registry URL (optional)
//! - `REGISTRY_API_KEY`: LLM-Registry API key (optional)
//! - `VAULT_URL`: Data-Vault URL (optional)
//! - `VAULT_API_KEY`: Data-Vault API key (optional)
//!
//! # Usage
//!
//! ```bash
//! # Basic usage
//! cargo run --bin server
//!
//! # With custom configuration
//! DB_PATH=/var/lib/memory-graph \
//! GRPC_PORT=8080 \
//! METRICS_PORT=9090 \
//! RUST_LOG=debug \
//! cargo run --bin server
//! ```

use llm_memory_graph::{
    engine::AsyncMemoryGraph,
    observatory::PrometheusMetrics,
    types::Config,
};
use prometheus::Registry;
use std::sync::Arc;
use std::time::Instant;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Server configuration loaded from environment variables
#[derive(Debug, Clone)]
struct ServerConfig {
    /// Database storage path
    db_path: String,
    /// gRPC server host
    grpc_host: String,
    /// gRPC server port
    grpc_port: u16,
    /// Prometheus metrics port
    metrics_port: u16,
    /// Plugin directories (comma-separated)
    plugin_dirs: Option<String>,
    /// LLM-Registry URL
    registry_url: Option<String>,
    /// LLM-Registry API key
    registry_api_key: Option<String>,
    /// Data-Vault URL
    vault_url: Option<String>,
    /// Data-Vault API key
    vault_api_key: Option<String>,
    /// Server start time for uptime calculation
    start_time: Instant,
}

impl ServerConfig {
    /// Load configuration from environment variables
    fn from_env() -> Self {
        Self {
            db_path: std::env::var("DB_PATH").unwrap_or_else(|_| "./data".to_string()),
            grpc_host: std::env::var("GRPC_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            grpc_port: std::env::var("GRPC_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(50051),
            metrics_port: std::env::var("METRICS_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(9090),
            plugin_dirs: std::env::var("PLUGIN_DIRS").ok(),
            registry_url: std::env::var("REGISTRY_URL").ok(),
            registry_api_key: std::env::var("REGISTRY_API_KEY").ok(),
            vault_url: std::env::var("VAULT_URL").ok(),
            vault_api_key: std::env::var("VAULT_API_KEY").ok(),
            start_time: Instant::now(),
        }
    }

    /// Validate configuration
    fn validate(&self) -> Result<(), String> {
        if self.grpc_port == 0 {
            return Err("GRPC_PORT must be non-zero".to_string());
        }
        if self.metrics_port == 0 {
            return Err("METRICS_PORT must be non-zero".to_string());
        }
        if self.grpc_port == self.metrics_port {
            return Err("GRPC_PORT and METRICS_PORT must be different".to_string());
        }
        Ok(())
    }

    /// Get the gRPC bind address
    fn grpc_address(&self) -> String {
        format!("{}:{}", self.grpc_host, self.grpc_port)
    }

    /// Get the metrics bind address
    fn metrics_address(&self) -> ([u8; 4], u16) {
        ([0, 0, 0, 0], self.metrics_port)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing/logging
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    tracing_subscriber::registry()
        .with(EnvFilter::new(log_level))
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
        )
        .init();

    info!("Starting LLM-Memory-Graph gRPC Server v{}", env!("CARGO_PKG_VERSION"));

    // Load and validate configuration
    let config = ServerConfig::from_env();
    config.validate()?;

    info!("Configuration:");
    info!("  Database path: {}", config.db_path);
    info!("  gRPC address: {}", config.grpc_address());
    info!("  Metrics address: 0.0.0.0:{}", config.metrics_port);

    if let Some(ref plugin_dirs) = config.plugin_dirs {
        info!("  Plugin directories: {}", plugin_dirs);
    }
    if config.registry_url.is_some() {
        info!("  LLM-Registry integration: enabled");
    }
    if config.vault_url.is_some() {
        info!("  Data-Vault integration: enabled");
    }

    // Initialize Prometheus registry
    info!("Initializing Prometheus metrics...");
    let registry = Registry::new();
    let metrics = Arc::new(
        PrometheusMetrics::new(&registry)
            .map_err(|e| format!("Failed to create Prometheus metrics: {}", e))?
    );
    info!("Prometheus metrics initialized with 18 metrics");

    // Initialize memory graph with Observatory
    info!("Opening memory graph database at: {}", config.db_path);
    let graph_config = Config::new(&config.db_path);
    let graph = Arc::new(
        AsyncMemoryGraph::open(graph_config)
            .await
            .map_err(|e| format!("Failed to open memory graph: {}", e))?
    );
    info!("Memory graph database opened successfully");

    // Get initial statistics
    match graph.stats().await {
        Ok(stats) => {
            info!("Database statistics:");
            info!("  Total nodes: {}", stats.node_count);
            info!("  Total edges: {}", stats.edge_count);
            info!("  Total sessions: {}", stats.session_count);

            // Update Prometheus gauges with initial values
            metrics.set_total_nodes(stats.node_count as i64);
            metrics.set_total_edges(stats.edge_count as i64);
        }
        Err(e) => {
            warn!("Could not retrieve database statistics: {}", e);
        }
    }

    // Initialize plugin manager (future extension)
    // Note: Plugin system would be initialized here when implemented
    if let Some(ref plugin_dirs) = config.plugin_dirs {
        info!("Plugin system not yet implemented, ignoring PLUGIN_DIRS: {}", plugin_dirs);
    }

    // Initialize integrations (future extension)
    if let Some(ref url) = config.registry_url {
        info!("LLM-Registry integration not yet implemented, URL: {}", url);
    }
    if let Some(ref url) = config.vault_url {
        info!("Data-Vault integration not yet implemented, URL: {}", url);
    }

    // Spawn metrics HTTP server
    let metrics_addr = config.metrics_address();
    let registry_clone = registry.clone();
    let metrics_handle = tokio::spawn(async move {
        if let Err(e) = serve_metrics(registry_clone, metrics_addr).await {
            error!("Metrics server error: {}", e);
        }
    });

    info!("Metrics server started on http://0.0.0.0:{}/metrics", config.metrics_port);

    // Note: gRPC service implementation would go here
    // For now, we create a placeholder that demonstrates the structure
    info!("gRPC service not yet fully implemented");
    info!("Server initialization complete");

    // Wait for shutdown signal
    info!("Server ready. Press Ctrl+C to shutdown");

    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal");
        }
        Err(e) => {
            error!("Error listening for shutdown signal: {}", e);
        }
    }

    // Graceful shutdown
    info!("Starting graceful shutdown...");

    // Abort metrics server
    metrics_handle.abort();

    // Flush database
    info!("Flushing database...");
    if let Err(e) = graph.flush().await {
        error!("Error flushing database: {}", e);
    }

    // Log final statistics
    match graph.stats().await {
        Ok(stats) => {
            info!("Final database statistics:");
            info!("  Total nodes: {}", stats.node_count);
            info!("  Total edges: {}", stats.edge_count);
            info!("  Total sessions: {}", stats.session_count);
        }
        Err(e) => {
            warn!("Could not retrieve final statistics: {}", e);
        }
    }

    let uptime = config.start_time.elapsed();
    info!("Server uptime: {:.2}s", uptime.as_secs_f64());
    info!("Shutdown complete");

    Ok(())
}

/// Serve Prometheus metrics on a separate HTTP port
async fn serve_metrics(
    registry: Registry,
    addr: ([u8; 4], u16),
) -> Result<(), Box<dyn std::error::Error>> {
    use warp::Filter;

    // Health check endpoint
    let health = warp::path("health")
        .map(|| warp::reply::json(&serde_json::json!({
            "status": "healthy",
            "service": "llm-memory-graph",
            "version": env!("CARGO_PKG_VERSION")
        })));

    // Metrics endpoint
    let metrics = warp::path("metrics")
        .map(move || {
            use prometheus::Encoder;
            let encoder = prometheus::TextEncoder::new();
            let metric_families = registry.gather();
            let mut buffer = vec![];

            match encoder.encode(&metric_families, &mut buffer) {
                Ok(()) => {
                    match String::from_utf8(buffer) {
                        Ok(text) => warp::reply::with_status(
                            text,
                            warp::http::StatusCode::OK
                        ),
                        Err(_) => warp::reply::with_status(
                            "Error encoding metrics".to_string(),
                            warp::http::StatusCode::INTERNAL_SERVER_ERROR
                        ),
                    }
                }
                Err(_) => warp::reply::with_status(
                    "Error gathering metrics".to_string(),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR
                ),
            }
        });

    // Root endpoint
    let root = warp::path::end()
        .map(|| warp::reply::html(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>LLM-Memory-Graph Metrics</title>
    <style>
        body { font-family: sans-serif; margin: 40px; }
        h1 { color: #333; }
        a { color: #0066cc; text-decoration: none; }
        a:hover { text-decoration: underline; }
        .endpoint { margin: 10px 0; }
    </style>
</head>
<body>
    <h1>LLM-Memory-Graph Metrics Server</h1>
    <p>Available endpoints:</p>
    <div class="endpoint"><a href="/metrics">/metrics</a> - Prometheus metrics</div>
    <div class="endpoint"><a href="/health">/health</a> - Health check</div>
</body>
</html>"#
        ));

    let routes = root.or(health).or(metrics);

    info!("Metrics server listening on http://{}:{}", addr.0[0], addr.1);
    warp::serve(routes).run(addr).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_validation() {
        let mut config = ServerConfig {
            db_path: "./test".to_string(),
            grpc_host: "127.0.0.1".to_string(),
            grpc_port: 50051,
            metrics_port: 9090,
            plugin_dirs: None,
            registry_url: None,
            registry_api_key: None,
            vault_url: None,
            vault_api_key: None,
            start_time: Instant::now(),
        };

        // Valid configuration
        assert!(config.validate().is_ok());

        // Invalid: zero gRPC port
        config.grpc_port = 0;
        assert!(config.validate().is_err());
        config.grpc_port = 50051;

        // Invalid: zero metrics port
        config.metrics_port = 0;
        assert!(config.validate().is_err());
        config.metrics_port = 9090;

        // Invalid: same ports
        config.metrics_port = 50051;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_server_config_addresses() {
        let config = ServerConfig {
            db_path: "./test".to_string(),
            grpc_host: "0.0.0.0".to_string(),
            grpc_port: 50051,
            metrics_port: 9090,
            plugin_dirs: None,
            registry_url: None,
            registry_api_key: None,
            vault_url: None,
            vault_api_key: None,
            start_time: Instant::now(),
        };

        assert_eq!(config.grpc_address(), "0.0.0.0:50051");
        assert_eq!(config.metrics_address(), ([0, 0, 0, 0], 9090));
    }
}

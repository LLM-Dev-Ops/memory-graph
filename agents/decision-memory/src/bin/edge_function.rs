//! Decision Memory Agent - Google Cloud Edge Function Entry Point
//!
//! This binary provides the entry point for deploying the Decision Memory Agent
//! as a Google Cloud Edge Function.
//!
//! # Deployment
//! - Deploy as part of the LLM-Memory-Graph unified GCP service
//! - Stateless execution
//! - No direct SQL access - all persistence via ruvector-service
//!
//! # Environment Variables
//! - `RUVECTOR_BASE_URL`: Base URL for ruvector-service
//! - `RUVECTOR_API_KEY`: API key for ruvector-service authentication
//! - `PORT`: Port to listen on (default: 8080)
//! - `RUST_LOG`: Logging level (default: info)

use decision_memory_agent::{
    handler::EdgeFunctionHandler,
    ruvector::RuVectorConfig,
    agent::AgentConfig,
};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(
            env::var("RUST_LOG")
                .map(|v| v.parse().unwrap_or(Level::INFO))
                .unwrap_or(Level::INFO),
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .json()
        .init();

    info!(
        agent_id = decision_memory_agent::constants::AGENT_ID,
        agent_version = decision_memory_agent::constants::AGENT_VERSION,
        classification = decision_memory_agent::constants::CLASSIFICATION,
        "Starting Decision Memory Agent Edge Function"
    );

    // Load configuration from environment
    let ruvector_config = RuVectorConfig {
        base_url: env::var("RUVECTOR_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string()),
        api_key: env::var("RUVECTOR_API_KEY").unwrap_or_default(),
        timeout_secs: env::var("RUVECTOR_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),
        max_retries: env::var("RUVECTOR_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3),
        retry_delay_ms: env::var("RUVECTOR_RETRY_DELAY_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100),
    };

    let agent_config = AgentConfig {
        strict_validation: env::var("STRICT_VALIDATION")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true),
        compute_embeddings: env::var("COMPUTE_EMBEDDINGS")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false),
        apply_pii_redaction: env::var("APPLY_PII_REDACTION")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false),
        max_artifacts: env::var("MAX_ARTIFACTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100),
        max_artifact_content_size: env::var("MAX_ARTIFACT_CONTENT_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1024 * 1024),
    };

    // Create handler
    let handler = match EdgeFunctionHandler::new(ruvector_config, agent_config) {
        Ok(h) => Arc::new(h),
        Err(e) => {
            eprintln!("Failed to create handler: {}", e);
            std::process::exit(1);
        }
    };

    // Set up routes
    let routes = EdgeFunctionHandler::routes(handler);

    // Get port from environment (Cloud Run sets PORT)
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);

    let addr: SocketAddr = ([0, 0, 0, 0], port).into();

    info!(
        port = port,
        "Decision Memory Agent listening"
    );

    // Start server
    warp::serve(routes).run(addr).await;
}

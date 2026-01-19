//! Prompt Lineage Agent - CLI Entry Point
//!
//! This binary provides command-line access to the Prompt Lineage Agent,
//! enabling inspection, retrieval, and replay of prompt lineage data.
//!
//! # Usage
//!
//! ```bash
//! # Inspect lineage of a prompt
//! prompt-lineage-agent inspect 550e8400-e29b-41d4-a716-446655440000
//!
//! # Retrieve full lineage subgraph as JSON
//! prompt-lineage-agent retrieve 550e8400-e29b-41d4-a716-446655440000 --format json
//!
//! # Replay lineage creation for debugging
//! prompt-lineage-agent replay 550e8400-e29b-41d4-a716-446655440000 --verbose
//!
//! # Compare two prompts
//! prompt-lineage-agent compare <id1> <id2> --suggest-type
//!
//! # Find similar prompts
//! prompt-lineage-agent similar <id> --threshold 0.7
//! ```

use clap::Parser;
use prompt_lineage_agent::cli::{execute, LineageCli};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize logging with environment-based filtering
fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,prompt_lineage_agent=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_logging();

    // Parse CLI arguments
    let cli = LineageCli::parse();

    // Execute the command
    match execute(cli).await {
        Ok(()) => {
            tracing::debug!("Command completed successfully");
            Ok(())
        }
        Err(e) => {
            tracing::error!("Command failed: {}", e);
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

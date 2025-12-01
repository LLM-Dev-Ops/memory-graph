//! Canonical benchmark runner for llm-memory-graph
//!
//! This binary runs all registered benchmarks and outputs results in the
//! canonical format for cross-repository aggregation and analysis.

use anyhow::Result;
use llm_memory_graph::benchmarks;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║        LLM Memory Graph - Canonical Benchmark Suite          ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    // Run all benchmarks
    let results = benchmarks::run_all_benchmarks().await?;

    // Print summary
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║                      Summary Statistics                        ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    let mut total_mean = 0.0;
    let mut count = 0;

    for result in &results {
        if let Some(mean) = result.mean_ns() {
            total_mean += mean;
            count += 1;
        }
    }

    if count > 0 {
        println!("Total benchmarks: {}", results.len());
        println!("Average mean time: {:.2} ns", total_mean / count as f64);
        println!("Average mean time: {:.2} ms", total_mean / count as f64 / 1_000_000.0);
    }

    println!("\n✓ All benchmarks completed successfully!");
    println!("✓ Results written to benchmarks/output/");

    Ok(())
}

//! Canonical benchmark interface for llm-memory-graph
//!
//! This module provides a standardized interface for running benchmarks across
//! the memory-graph ecosystem. It follows the canonical cross-repository format
//! for benchmark results, enabling aggregation and comparison across different
//! implementations.
//!
//! # Architecture
//!
//! The benchmark system is organized into several modules:
//!
//! - `result`: Canonical `BenchmarkResult` struct with standardized fields
//! - `adapters`: `BenchTarget` trait and registry for all benchmark targets
//! - `targets`: Concrete benchmark implementations wrapping existing criterion tests
//! - `io`: I/O utilities for reading/writing results in JSON and CSV formats
//! - `markdown`: Report generation for human-readable summaries
//!
//! # Usage
//!
//! Run all benchmarks and collect results:
//!
//! ```no_run
//! use llm_memory_graph::benchmarks::run_all_benchmarks;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let results = run_all_benchmarks().await?;
//!     println!("Completed {} benchmarks", results.len());
//!     Ok(())
//! }
//! ```
//!
//! # Output Format
//!
//! Results are written to the canonical directory structure:
//!
//! ```text
//! benchmarks/
//! └── output/
//!     ├── raw/
//!     │   ├── results_TIMESTAMP.json
//!     │   └── results_TIMESTAMP.csv
//!     └── summary.md
//! ```

pub mod adapters;
pub mod io;
pub mod markdown;
pub mod result;
pub mod targets;

use anyhow::{Context, Result};
use result::BenchmarkResult;

/// Run all registered benchmarks and return results
///
/// This function executes all benchmark targets registered in `adapters::all_targets()`,
/// collects the results, and writes them to the canonical output directory.
///
/// # Output
///
/// Results are written to:
/// - `benchmarks/output/raw/results_TIMESTAMP.json` - Raw JSON data
/// - `benchmarks/output/raw/results_TIMESTAMP.csv` - CSV format for spreadsheets
/// - `benchmarks/output/summary.md` - Human-readable markdown report
///
/// # Errors
///
/// Returns an error if:
/// - Any benchmark fails to execute
/// - Output directories cannot be created
/// - Results cannot be written to disk
///
/// # Example
///
/// ```no_run
/// use llm_memory_graph::benchmarks::run_all_benchmarks;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     println!("Running benchmarks...");
///     let results = run_all_benchmarks().await?;
///     println!("Completed {} benchmarks", results.len());
///
///     for result in &results {
///         if let Some(mean) = result.mean_ns() {
///             println!("{}: {:.2} ns", result.target_id, mean);
///         }
///     }
///
///     Ok(())
/// }
/// ```
pub async fn run_all_benchmarks() -> Result<Vec<BenchmarkResult>> {
    use chrono::Utc;

    println!("Starting benchmark suite...");

    // Ensure output directories exist
    io::ensure_output_dirs()
        .context("Failed to create output directories")?;

    let targets = adapters::all_targets();
    let total = targets.len();
    println!("Found {} benchmark targets", total);

    let mut results = Vec::new();

    // Run benchmarks sequentially to avoid resource contention
    // Each benchmark gets a clean temporary environment
    for (idx, target) in targets.into_iter().enumerate() {
        let target_id = target.id();
        let category = target.category().unwrap_or_else(|| "uncategorized".to_string());

        println!(
            "[{}/{}] Running: {} (category: {})",
            idx + 1,
            total,
            target_id,
            category
        );

        match target.run().await {
            Ok(result) => {
                println!("  ✓ Completed successfully");
                if let Some(mean) = result.mean_ns() {
                    println!("    Mean: {:.2} ns", mean);
                }
                if let Some(throughput) = result.throughput() {
                    println!("    Throughput: {:.2} ops/s", throughput);
                }
                results.push(result);
            }
            Err(e) => {
                eprintln!("  ✗ Failed: {}", e);
                return Err(e).context(format!("Benchmark '{}' failed", target_id));
            }
        }
    }

    // Generate timestamp for this benchmark run
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();

    // Write results in multiple formats
    let json_file = format!("results_{}.json", timestamp);
    let csv_file = format!("results_{}.csv", timestamp);

    println!("\nWriting results...");

    let json_path = io::write_results_json(&results, &json_file)
        .context("Failed to write JSON results")?;
    println!("  JSON: {}", json_path.display());

    let csv_path = io::write_results_csv(&results, &csv_file)
        .context("Failed to write CSV results")?;
    println!("  CSV: {}", csv_path.display());

    let summary_path = markdown::write_summary(&results, io::output_dir())
        .context("Failed to write markdown summary")?;
    println!("  Summary: {}", summary_path.display());

    println!("\nBenchmark suite completed successfully!");
    println!("Total benchmarks: {}", results.len());

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_targets_registered() {
        let targets = adapters::all_targets();
        assert!(!targets.is_empty(), "No benchmark targets registered");

        // Verify all targets have unique IDs
        let mut ids = std::collections::HashSet::new();
        for target in targets {
            let id = target.id();
            assert!(!ids.contains(&id), "Duplicate target ID: {}", id);
            ids.insert(id);
        }
    }

    #[tokio::test]
    async fn test_run_single_benchmark() {
        // Test that we can run at least one benchmark
        let targets = adapters::all_targets();
        if let Some(target) = targets.first() {
            let result = target.run().await;
            assert!(result.is_ok(), "First benchmark should succeed");
        }
    }
}

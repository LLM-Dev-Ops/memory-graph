//! I/O utilities for benchmark results
//!
//! This module provides functions for reading and writing benchmark results
//! in various formats (JSON, CSV) to the canonical output directory structure.

use super::result::BenchmarkResult;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Default output directory for benchmark results
pub const DEFAULT_OUTPUT_DIR: &str = "benchmarks/output";

/// Raw benchmark data subdirectory
pub const RAW_OUTPUT_SUBDIR: &str = "raw";

/// Get the canonical output directory path
pub fn output_dir() -> PathBuf {
    PathBuf::from(DEFAULT_OUTPUT_DIR)
}

/// Get the raw output subdirectory path
pub fn raw_output_dir() -> PathBuf {
    output_dir().join(RAW_OUTPUT_SUBDIR)
}

/// Ensure output directories exist
pub fn ensure_output_dirs() -> Result<()> {
    let output = output_dir();
    let raw = raw_output_dir();

    fs::create_dir_all(&output)
        .with_context(|| format!("Failed to create output directory: {}", output.display()))?;

    fs::create_dir_all(&raw)
        .with_context(|| format!("Failed to create raw output directory: {}", raw.display()))?;

    Ok(())
}

/// Write benchmark results to a JSON file in the raw output directory
pub fn write_results_json(
    results: &[BenchmarkResult],
    filename: impl AsRef<Path>,
) -> Result<PathBuf> {
    ensure_output_dirs()?;

    let path = raw_output_dir().join(filename.as_ref());
    let json = serde_json::to_string_pretty(results)
        .with_context(|| "Failed to serialize benchmark results")?;

    fs::write(&path, json)
        .with_context(|| format!("Failed to write results to {}", path.display()))?;

    Ok(path)
}

/// Read benchmark results from a JSON file
pub fn read_results_json(path: impl AsRef<Path>) -> Result<Vec<BenchmarkResult>> {
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read results from {}", path.as_ref().display()))?;

    let results = serde_json::from_str(&content)
        .with_context(|| "Failed to deserialize benchmark results")?;

    Ok(results)
}

/// Write benchmark results to a CSV file in the raw output directory
pub fn write_results_csv(
    results: &[BenchmarkResult],
    filename: impl AsRef<Path>,
) -> Result<PathBuf> {
    ensure_output_dirs()?;

    let path = raw_output_dir().join(filename.as_ref());
    let mut csv_content = String::new();

    // Header
    csv_content.push_str("target_id,timestamp,mean_ns,std_dev_ns,median_ns,throughput,metrics_json\n");

    // Data rows
    for result in results {
        csv_content.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            result.target_id,
            result.timestamp.to_rfc3339(),
            result.mean_ns().map_or(String::from(""), |v| v.to_string()),
            result.std_dev_ns().map_or(String::from(""), |v| v.to_string()),
            result.median_ns().map_or(String::from(""), |v| v.to_string()),
            result.throughput().map_or(String::from(""), |v| v.to_string()),
            serde_json::to_string(&result.metrics).unwrap_or_default()
        ));
    }

    fs::write(&path, csv_content)
        .with_context(|| format!("Failed to write CSV to {}", path.display()))?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_ensure_output_dirs() {
        let temp = tempdir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        ensure_output_dirs().unwrap();

        assert!(output_dir().exists());
        assert!(raw_output_dir().exists());
    }

    #[test]
    fn test_write_and_read_json() {
        let temp = tempdir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let results = vec![
            BenchmarkResult::new("test1", json!({"mean_ns": 1000.0})),
            BenchmarkResult::new("test2", json!({"mean_ns": 2000.0})),
        ];

        let path = write_results_json(&results, "test_results.json").unwrap();
        assert!(path.exists());

        let loaded = read_results_json(&path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].target_id, "test1");
        assert_eq!(loaded[1].target_id, "test2");
    }

    #[test]
    fn test_write_csv() {
        let temp = tempdir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let results = vec![
            BenchmarkResult::new("test1", json!({"mean_ns": 1000.0, "throughput": 1000000.0})),
        ];

        let path = write_results_csv(&results, "test_results.csv").unwrap();
        assert!(path.exists());

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("target_id"));
        assert!(content.contains("test1"));
        assert!(content.contains("1000"));
    }
}

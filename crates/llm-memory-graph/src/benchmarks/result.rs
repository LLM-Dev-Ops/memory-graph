//! Benchmark result types following the canonical cross-repository interface
//!
//! This module defines the standard `BenchmarkResult` structure used across
//! all repositories for consistent benchmark reporting and analysis.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Canonical benchmark result structure
///
/// This struct provides a standardized format for benchmark results across
/// all repositories. It contains the essential information needed for
/// aggregation, comparison, and analysis of benchmark data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Unique identifier for the benchmark target (e.g., "batch_prompts_sequential_10")
    pub target_id: String,

    /// Flexible metrics storage as JSON
    ///
    /// This allows each benchmark to store custom metrics while maintaining
    /// compatibility. Common fields include:
    /// - "mean_ns": Mean execution time in nanoseconds
    /// - "std_dev_ns": Standard deviation
    /// - "min_ns": Minimum execution time
    /// - "max_ns": Maximum execution time
    /// - "throughput": Operations per second
    /// - "median_ns": Median execution time
    pub metrics: serde_json::Value,

    /// Timestamp when the benchmark was executed
    pub timestamp: DateTime<Utc>,
}

impl BenchmarkResult {
    /// Create a new benchmark result
    pub fn new(target_id: impl Into<String>, metrics: serde_json::Value) -> Self {
        Self {
            target_id: target_id.into(),
            metrics,
            timestamp: Utc::now(),
        }
    }

    /// Create a benchmark result with a specific timestamp
    pub fn with_timestamp(
        target_id: impl Into<String>,
        metrics: serde_json::Value,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            target_id: target_id.into(),
            metrics,
            timestamp,
        }
    }

    /// Get the mean execution time in nanoseconds, if available
    pub fn mean_ns(&self) -> Option<f64> {
        self.metrics
            .get("mean_ns")
            .and_then(|v| v.as_f64())
    }

    /// Get the standard deviation in nanoseconds, if available
    pub fn std_dev_ns(&self) -> Option<f64> {
        self.metrics
            .get("std_dev_ns")
            .and_then(|v| v.as_f64())
    }

    /// Get the throughput (operations per second), if available
    pub fn throughput(&self) -> Option<f64> {
        self.metrics
            .get("throughput")
            .and_then(|v| v.as_f64())
    }

    /// Get the median execution time in nanoseconds, if available
    pub fn median_ns(&self) -> Option<f64> {
        self.metrics
            .get("median_ns")
            .and_then(|v| v.as_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_benchmark_result_creation() {
        let metrics = json!({
            "mean_ns": 1000.0,
            "std_dev_ns": 100.0,
            "throughput": 1000000.0
        });

        let result = BenchmarkResult::new("test_benchmark", metrics);

        assert_eq!(result.target_id, "test_benchmark");
        assert_eq!(result.mean_ns(), Some(1000.0));
        assert_eq!(result.std_dev_ns(), Some(100.0));
        assert_eq!(result.throughput(), Some(1000000.0));
    }

    #[test]
    fn test_benchmark_result_serialization() {
        let metrics = json!({
            "mean_ns": 1000.0,
            "median_ns": 950.0
        });

        let result = BenchmarkResult::new("test", metrics);
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: BenchmarkResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.target_id, deserialized.target_id);
        assert_eq!(result.mean_ns(), deserialized.mean_ns());
    }
}

//! Adapter trait for benchmark targets
//!
//! This module defines the canonical `BenchTarget` trait that all benchmarks
//! must implement, along with the registry function `all_targets()`.

use super::result::BenchmarkResult;
use anyhow::Result;
use async_trait::async_trait;

/// Trait for all benchmark targets
///
/// This trait provides a common interface for running benchmarks and
/// collecting results. Each benchmark implementation should provide a
/// unique ID and a run method that executes the benchmark.
#[async_trait]
pub trait BenchTarget: Send + Sync {
    /// Get the unique identifier for this benchmark target
    ///
    /// The ID should be descriptive and follow the pattern:
    /// `{category}_{operation}_{variant}_{size}`
    ///
    /// Examples:
    /// - "batch_prompts_sequential_10"
    /// - "cache_hit_single_node"
    /// - "pool_overhead_write_default"
    fn id(&self) -> String;

    /// Run the benchmark and return results
    ///
    /// This method should execute the benchmark, collect timing and
    /// performance data, and return it as a `BenchmarkResult`.
    async fn run(&self) -> Result<BenchmarkResult>;

    /// Optional: Get a human-readable description of this benchmark
    fn description(&self) -> Option<String> {
        None
    }

    /// Optional: Get the category/group for this benchmark
    fn category(&self) -> Option<String> {
        None
    }
}

/// Registry of all available benchmark targets
///
/// This function returns all registered benchmark targets. Add new
/// benchmark implementations here to include them in the suite.
pub fn all_targets() -> Vec<Box<dyn BenchTarget>> {
    vec![
        // Batch operation benchmarks
        Box::new(crate::benchmarks::targets::BatchPromptsSequential10),
        Box::new(crate::benchmarks::targets::BatchPromptsBatch10),
        Box::new(crate::benchmarks::targets::BatchPromptsSequential50),
        Box::new(crate::benchmarks::targets::BatchPromptsBatch50),

        // Cache performance benchmarks
        Box::new(crate::benchmarks::targets::CacheHitSingle),
        Box::new(crate::benchmarks::targets::CacheMissSingle),

        // Pool performance benchmarks
        Box::new(crate::benchmarks::targets::PoolOverheadNonPooled),
        Box::new(crate::benchmarks::targets::PoolOverheadPooled),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct MockBenchmark;

    #[async_trait]
    impl BenchTarget for MockBenchmark {
        fn id(&self) -> String {
            "mock_benchmark".to_string()
        }

        async fn run(&self) -> Result<BenchmarkResult> {
            Ok(BenchmarkResult::new(
                self.id(),
                json!({"mean_ns": 1000.0}),
            ))
        }

        fn description(&self) -> Option<String> {
            Some("A mock benchmark for testing".to_string())
        }

        fn category(&self) -> Option<String> {
            Some("mock".to_string())
        }
    }

    #[tokio::test]
    async fn test_bench_target_trait() {
        let benchmark = MockBenchmark;

        assert_eq!(benchmark.id(), "mock_benchmark");
        assert_eq!(
            benchmark.description(),
            Some("A mock benchmark for testing".to_string())
        );
        assert_eq!(benchmark.category(), Some("mock".to_string()));

        let result = benchmark.run().await.unwrap();
        assert_eq!(result.target_id, "mock_benchmark");
        assert_eq!(result.mean_ns(), Some(1000.0));
    }
}

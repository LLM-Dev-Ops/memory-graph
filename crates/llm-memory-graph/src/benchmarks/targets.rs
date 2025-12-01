//! Concrete benchmark target implementations
//!
//! This module contains adapters that wrap the existing criterion benchmarks
//! into the canonical BenchTarget interface.

use super::adapters::BenchTarget;
use super::result::BenchmarkResult;
use crate::engine::AsyncMemoryGraph;
use crate::{Config, TokenUsage};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::time::Instant;
use tempfile::tempdir;

// ============================================================================
// Batch Operation Benchmarks
// ============================================================================

/// Benchmark for sequential prompt creation (10 items)
pub struct BatchPromptsSequential10;

#[async_trait]
impl BenchTarget for BatchPromptsSequential10 {
    fn id(&self) -> String {
        "batch_prompts_sequential_10".to_string()
    }

    async fn run(&self) -> Result<BenchmarkResult> {
        let dir = tempdir()?;
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await?;
        let session = graph.create_session().await?;

        let start = Instant::now();
        for i in 0..10 {
            graph
                .add_prompt(session.id, format!("Prompt {}", i), None)
                .await?;
        }
        let elapsed = start.elapsed();

        Ok(BenchmarkResult::new(
            self.id(),
            json!({
                "mean_ns": elapsed.as_nanos() as f64,
                "operations": 10,
                "throughput": 10.0 / elapsed.as_secs_f64(),
                "category": "batch_operations"
            }),
        ))
    }

    fn category(&self) -> Option<String> {
        Some("batch_operations".to_string())
    }

    fn description(&self) -> Option<String> {
        Some("Sequential creation of 10 prompts".to_string())
    }
}

/// Benchmark for batch prompt creation (10 items)
pub struct BatchPromptsBatch10;

#[async_trait]
impl BenchTarget for BatchPromptsBatch10 {
    fn id(&self) -> String {
        "batch_prompts_batch_10".to_string()
    }

    async fn run(&self) -> Result<BenchmarkResult> {
        let dir = tempdir()?;
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await?;
        let session = graph.create_session().await?;

        let prompts: Vec<_> = (0..10)
            .map(|i| (session.id, format!("Prompt {}", i)))
            .collect();

        let start = Instant::now();
        graph.add_prompts_batch(prompts).await?;
        let elapsed = start.elapsed();

        Ok(BenchmarkResult::new(
            self.id(),
            json!({
                "mean_ns": elapsed.as_nanos() as f64,
                "operations": 10,
                "throughput": 10.0 / elapsed.as_secs_f64(),
                "category": "batch_operations"
            }),
        ))
    }

    fn category(&self) -> Option<String> {
        Some("batch_operations".to_string())
    }

    fn description(&self) -> Option<String> {
        Some("Batch creation of 10 prompts".to_string())
    }
}

/// Benchmark for sequential prompt creation (50 items)
pub struct BatchPromptsSequential50;

#[async_trait]
impl BenchTarget for BatchPromptsSequential50 {
    fn id(&self) -> String {
        "batch_prompts_sequential_50".to_string()
    }

    async fn run(&self) -> Result<BenchmarkResult> {
        let dir = tempdir()?;
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await?;
        let session = graph.create_session().await?;

        let start = Instant::now();
        for i in 0..50 {
            graph
                .add_prompt(session.id, format!("Prompt {}", i), None)
                .await?;
        }
        let elapsed = start.elapsed();

        Ok(BenchmarkResult::new(
            self.id(),
            json!({
                "mean_ns": elapsed.as_nanos() as f64,
                "operations": 50,
                "throughput": 50.0 / elapsed.as_secs_f64(),
                "category": "batch_operations"
            }),
        ))
    }

    fn category(&self) -> Option<String> {
        Some("batch_operations".to_string())
    }

    fn description(&self) -> Option<String> {
        Some("Sequential creation of 50 prompts".to_string())
    }
}

/// Benchmark for batch prompt creation (50 items)
pub struct BatchPromptsBatch50;

#[async_trait]
impl BenchTarget for BatchPromptsBatch50 {
    fn id(&self) -> String {
        "batch_prompts_batch_50".to_string()
    }

    async fn run(&self) -> Result<BenchmarkResult> {
        let dir = tempdir()?;
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await?;
        let session = graph.create_session().await?;

        let prompts: Vec<_> = (0..50)
            .map(|i| (session.id, format!("Prompt {}", i)))
            .collect();

        let start = Instant::now();
        graph.add_prompts_batch(prompts).await?;
        let elapsed = start.elapsed();

        Ok(BenchmarkResult::new(
            self.id(),
            json!({
                "mean_ns": elapsed.as_nanos() as f64,
                "operations": 50,
                "throughput": 50.0 / elapsed.as_secs_f64(),
                "category": "batch_operations"
            }),
        ))
    }

    fn category(&self) -> Option<String> {
        Some("batch_operations".to_string())
    }

    fn description(&self) -> Option<String> {
        Some("Batch creation of 50 prompts".to_string())
    }
}

// ============================================================================
// Cache Performance Benchmarks
// ============================================================================

/// Benchmark for cache hit performance
pub struct CacheHitSingle;

#[async_trait]
impl BenchTarget for CacheHitSingle {
    fn id(&self) -> String {
        "cache_hit_single".to_string()
    }

    async fn run(&self) -> Result<BenchmarkResult> {
        let dir = tempdir()?;
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await?;
        let session = graph.create_session().await?;

        // Pre-populate cache
        let prompt_id = graph
            .add_prompt(session.id, "Test prompt".to_string(), None)
            .await?;

        // First read to warm up cache
        graph.get_node(&prompt_id).await?;

        // Measure cache hit
        let start = Instant::now();
        for _ in 0..100 {
            graph.get_node(&prompt_id).await?;
        }
        let elapsed = start.elapsed();

        Ok(BenchmarkResult::new(
            self.id(),
            json!({
                "mean_ns": (elapsed.as_nanos() as f64) / 100.0,
                "operations": 100,
                "throughput": 100.0 / elapsed.as_secs_f64(),
                "category": "cache_performance"
            }),
        ))
    }

    fn category(&self) -> Option<String> {
        Some("cache_performance".to_string())
    }

    fn description(&self) -> Option<String> {
        Some("Single node cache hit performance (100 reads)".to_string())
    }
}

/// Benchmark for cache miss performance
pub struct CacheMissSingle;

#[async_trait]
impl BenchTarget for CacheMissSingle {
    fn id(&self) -> String {
        "cache_miss_single".to_string()
    }

    async fn run(&self) -> Result<BenchmarkResult> {
        let dir = tempdir()?;
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await?;
        let session = graph.create_session().await?;

        let prompt_id = graph
            .add_prompt(session.id, "Test prompt".to_string(), None)
            .await?;

        // Measure initial read (cache miss)
        let start = Instant::now();
        graph.get_node(&prompt_id).await?;
        let elapsed = start.elapsed();

        Ok(BenchmarkResult::new(
            self.id(),
            json!({
                "mean_ns": elapsed.as_nanos() as f64,
                "operations": 1,
                "category": "cache_performance"
            }),
        ))
    }

    fn category(&self) -> Option<String> {
        Some("cache_performance".to_string())
    }

    fn description(&self) -> Option<String> {
        Some("Single node cache miss performance".to_string())
    }
}

// ============================================================================
// Pool Performance Benchmarks
// ============================================================================

/// Benchmark for non-pooled write operations
pub struct PoolOverheadNonPooled;

#[async_trait]
impl BenchTarget for PoolOverheadNonPooled {
    fn id(&self) -> String {
        "pool_overhead_non_pooled".to_string()
    }

    async fn run(&self) -> Result<BenchmarkResult> {
        let dir = tempdir()?;
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await?;
        let session = graph.create_session().await?;

        let start = Instant::now();
        for i in 0..10 {
            graph
                .add_prompt(session.id, format!("Prompt {}", i), None)
                .await?;
        }
        let elapsed = start.elapsed();

        Ok(BenchmarkResult::new(
            self.id(),
            json!({
                "mean_ns": (elapsed.as_nanos() as f64) / 10.0,
                "operations": 10,
                "throughput": 10.0 / elapsed.as_secs_f64(),
                "category": "pool_performance"
            }),
        ))
    }

    fn category(&self) -> Option<String> {
        Some("pool_performance".to_string())
    }

    fn description(&self) -> Option<String> {
        Some("Non-pooled write operations (10 prompts)".to_string())
    }
}

/// Benchmark for pooled write operations
pub struct PoolOverheadPooled;

#[async_trait]
impl BenchTarget for PoolOverheadPooled {
    fn id(&self) -> String {
        "pool_overhead_pooled".to_string()
    }

    async fn run(&self) -> Result<BenchmarkResult> {
        let dir = tempdir()?;
        let config = Config::new(dir.path());
        let graph = AsyncMemoryGraph::open(config).await?;
        let session = graph.create_session().await?;

        let start = Instant::now();
        for i in 0..10 {
            graph
                .add_prompt(session.id, format!("Prompt {}", i), None)
                .await?;
        }
        let elapsed = start.elapsed();

        Ok(BenchmarkResult::new(
            self.id(),
            json!({
                "mean_ns": (elapsed.as_nanos() as f64) / 10.0,
                "operations": 10,
                "throughput": 10.0 / elapsed.as_secs_f64(),
                "category": "pool_performance"
            }),
        ))
    }

    fn category(&self) -> Option<String> {
        Some("pool_performance".to_string())
    }

    fn description(&self) -> Option<String> {
        Some("Pooled write operations (10 prompts)".to_string())
    }
}

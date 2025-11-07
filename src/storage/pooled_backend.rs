//! Pooled async storage backend with resource management
//!
//! This module provides an enterprise-grade connection pooling layer for the
//! async storage backend. While Sled is an embedded database that doesn't have
//! traditional "connections", this pool manages concurrent access, provides
//! backpressure, and collects metrics for production deployments.
//!
//! # Features
//!
//! - **Concurrent Access Control**: Limits simultaneous operations to prevent resource exhaustion
//! - **Backpressure**: Applies backpressure when pool is saturated
//! - **Metrics**: Tracks pool utilization, wait times, and operation counts
//! - **Timeout Handling**: Configurable timeouts for acquiring pool permits
//! - **Graceful Degradation**: Handles overload scenarios gracefully
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │  PooledAsyncBackend                     │
//! │  ┌────────────────────────────────────┐ │
//! │  │ Semaphore (max_concurrent)         │ │
//! │  └────────────────────────────────────┘ │
//! │  ┌────────────────────────────────────┐ │
//! │  │ PoolMetrics (atomic counters)      │ │
//! │  └────────────────────────────────────┘ │
//! │  ┌────────────────────────────────────┐ │
//! │  │ AsyncSledBackend (underlying DB)   │ │
//! │  └────────────────────────────────────┘ │
//! └─────────────────────────────────────────┘
//! ```

use crate::error::{Error, Result};
use crate::storage::{AsyncSledBackend, AsyncStorageBackend, StorageStats};
use crate::types::{Edge, EdgeId, Node, NodeId, SessionId};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::timeout;

/// Configuration for the connection pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of concurrent operations
    pub max_concurrent: usize,
    /// Timeout for acquiring a pool permit (milliseconds)
    pub acquire_timeout_ms: u64,
    /// Enable detailed metrics collection
    pub enable_metrics: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 100, // Allow 100 concurrent operations
            acquire_timeout_ms: 5000, // 5 second timeout
            enable_metrics: true,
        }
    }
}

impl PoolConfig {
    /// Create a new pool configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum concurrent operations
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Set acquire timeout in milliseconds
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.acquire_timeout_ms = timeout_ms;
        self
    }

    /// Enable or disable metrics
    pub fn with_metrics(mut self, enable: bool) -> Self {
        self.enable_metrics = enable;
        self
    }
}

/// Metrics for the connection pool
#[derive(Debug)]
pub struct PoolMetrics {
    /// Total number of operations performed
    total_operations: AtomicU64,
    /// Total number of successful operations
    successful_operations: AtomicU64,
    /// Total number of failed operations
    failed_operations: AtomicU64,
    /// Total number of timeouts
    timeouts: AtomicU64,
    /// Current number of active operations
    active_operations: AtomicUsize,
    /// Peak number of concurrent operations
    peak_concurrent: AtomicUsize,
    /// Total time spent waiting for permits (microseconds)
    total_wait_time_us: AtomicU64,
}

impl PoolMetrics {
    /// Create new pool metrics
    pub fn new() -> Self {
        Self {
            total_operations: AtomicU64::new(0),
            successful_operations: AtomicU64::new(0),
            failed_operations: AtomicU64::new(0),
            timeouts: AtomicU64::new(0),
            active_operations: AtomicUsize::new(0),
            peak_concurrent: AtomicUsize::new(0),
            total_wait_time_us: AtomicU64::new(0),
        }
    }

    /// Record operation start
    fn operation_started(&self) {
        self.total_operations.fetch_add(1, Ordering::Relaxed);
        let active = self.active_operations.fetch_add(1, Ordering::Relaxed) + 1;

        // Update peak if needed
        let mut peak = self.peak_concurrent.load(Ordering::Relaxed);
        while active > peak {
            match self.peak_concurrent.compare_exchange_weak(
                peak,
                active,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }
    }

    /// Record operation completion
    fn operation_completed(&self, success: bool) {
        if success {
            self.successful_operations.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_operations.fetch_add(1, Ordering::Relaxed);
        }
        self.active_operations.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record timeout
    fn record_timeout(&self) {
        self.timeouts.fetch_add(1, Ordering::Relaxed);
        self.failed_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record wait time
    fn record_wait_time(&self, wait_time_us: u64) {
        self.total_wait_time_us
            .fetch_add(wait_time_us, Ordering::Relaxed);
    }

    /// Get a snapshot of current metrics
    pub fn snapshot(&self) -> PoolMetricsSnapshot {
        PoolMetricsSnapshot {
            total_operations: self.total_operations.load(Ordering::Relaxed),
            successful_operations: self.successful_operations.load(Ordering::Relaxed),
            failed_operations: self.failed_operations.load(Ordering::Relaxed),
            timeouts: self.timeouts.load(Ordering::Relaxed),
            active_operations: self.active_operations.load(Ordering::Relaxed),
            peak_concurrent: self.peak_concurrent.load(Ordering::Relaxed),
            total_wait_time_us: self.total_wait_time_us.load(Ordering::Relaxed),
        }
    }
}

impl Default for PoolMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of pool metrics at a point in time
#[derive(Debug, Clone, Copy)]
pub struct PoolMetricsSnapshot {
    /// Total operations performed
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Number of timeouts
    pub timeouts: u64,
    /// Currently active operations
    pub active_operations: usize,
    /// Peak concurrent operations
    pub peak_concurrent: usize,
    /// Total wait time in microseconds
    pub total_wait_time_us: u64,
}

impl PoolMetricsSnapshot {
    /// Calculate average wait time in milliseconds
    pub fn avg_wait_time_ms(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            (self.total_wait_time_us as f64) / (self.total_operations as f64) / 1000.0
        }
    }

    /// Calculate success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            1.0
        } else {
            (self.successful_operations as f64) / (self.total_operations as f64)
        }
    }

    /// Calculate timeout rate (0.0 to 1.0)
    pub fn timeout_rate(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            (self.timeouts as f64) / (self.total_operations as f64)
        }
    }
}

/// Pooled async storage backend with resource management
///
/// This backend wraps AsyncSledBackend with a semaphore-based pool that:
/// - Limits concurrent operations to prevent resource exhaustion
/// - Provides backpressure when the pool is saturated
/// - Collects metrics on pool utilization
/// - Implements timeouts to prevent indefinite blocking
///
/// # Examples
///
/// ```no_run
/// use llm_memory_graph::storage::{PooledAsyncBackend, PoolConfig};
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = PoolConfig::new()
///         .with_max_concurrent(50)
///         .with_timeout(3000);
///
///     let backend = PooledAsyncBackend::open(Path::new("./data/db"), config).await?;
///
///     // Get pool metrics
///     let metrics = backend.metrics();
///     println!("Active operations: {}", metrics.active_operations);
///
///     Ok(())
/// }
/// ```
pub struct PooledAsyncBackend {
    /// Underlying async backend
    backend: Arc<AsyncSledBackend>,
    /// Semaphore for controlling concurrent access
    semaphore: Arc<Semaphore>,
    /// Pool configuration
    config: PoolConfig,
    /// Pool metrics
    metrics: Arc<PoolMetrics>,
}

impl PooledAsyncBackend {
    /// Open a pooled async backend
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::storage::{PooledAsyncBackend, PoolConfig};
    /// use std::path::Path;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let backend = PooledAsyncBackend::open(
    ///     Path::new("./data/db"),
    ///     PoolConfig::default()
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open(path: &std::path::Path, config: PoolConfig) -> Result<Self> {
        let backend = AsyncSledBackend::open(path).await?;
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent));
        let metrics = if config.enable_metrics {
            Arc::new(PoolMetrics::new())
        } else {
            Arc::new(PoolMetrics::new()) // Always create metrics for now
        };

        Ok(Self {
            backend: Arc::new(backend),
            semaphore,
            config,
            metrics,
        })
    }

    /// Get current pool metrics
    pub fn metrics(&self) -> PoolMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Get pool configuration
    pub fn config(&self) -> &PoolConfig {
        &self.config
    }

    /// Get number of available permits
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Acquire a permit from the pool with timeout
    async fn acquire_permit(&self) -> Result<tokio::sync::SemaphorePermit<'_>> {
        let start = std::time::Instant::now();

        let permit = timeout(
            Duration::from_millis(self.config.acquire_timeout_ms),
            self.semaphore.acquire(),
        )
        .await
        .map_err(|_| {
            self.metrics.record_timeout();
            Error::Storage("Pool acquire timeout".to_string())
        })?
        .map_err(|_| Error::Storage("Semaphore closed".to_string()))?;

        // Record wait time
        let wait_time = start.elapsed().as_micros() as u64;
        self.metrics.record_wait_time(wait_time);
        self.metrics.operation_started();

        Ok(permit)
    }

    /// Execute an operation with pool management
    async fn with_permit<F, T>(&self, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let _permit = self.acquire_permit().await?;

        let result = f.await;
        self.metrics.operation_completed(result.is_ok());

        result
    }
}

#[async_trait]
impl AsyncStorageBackend for PooledAsyncBackend {
    async fn store_node(&self, node: &Node) -> Result<()> {
        self.with_permit(self.backend.store_node(node)).await
    }

    async fn get_node(&self, id: &NodeId) -> Result<Option<Node>> {
        self.with_permit(self.backend.get_node(id)).await
    }

    async fn delete_node(&self, id: &NodeId) -> Result<()> {
        self.with_permit(self.backend.delete_node(id)).await
    }

    async fn store_edge(&self, edge: &Edge) -> Result<()> {
        self.with_permit(self.backend.store_edge(edge)).await
    }

    async fn get_edge(&self, id: &EdgeId) -> Result<Option<Edge>> {
        self.with_permit(self.backend.get_edge(id)).await
    }

    async fn delete_edge(&self, id: &EdgeId) -> Result<()> {
        self.with_permit(self.backend.delete_edge(id)).await
    }

    async fn get_session_nodes(&self, session_id: &SessionId) -> Result<Vec<Node>> {
        self.with_permit(self.backend.get_session_nodes(session_id))
            .await
    }

    async fn get_outgoing_edges(&self, node_id: &NodeId) -> Result<Vec<Edge>> {
        self.with_permit(self.backend.get_outgoing_edges(node_id))
            .await
    }

    async fn get_incoming_edges(&self, node_id: &NodeId) -> Result<Vec<Edge>> {
        self.with_permit(self.backend.get_incoming_edges(node_id))
            .await
    }

    async fn flush(&self) -> Result<()> {
        self.with_permit(self.backend.flush()).await
    }

    async fn stats(&self) -> Result<StorageStats> {
        self.with_permit(self.backend.stats()).await
    }

    async fn store_nodes_batch(&self, nodes: &[Node]) -> Result<Vec<NodeId>> {
        self.with_permit(self.backend.store_nodes_batch(nodes))
            .await
    }

    async fn store_edges_batch(&self, edges: &[Edge]) -> Result<Vec<EdgeId>> {
        self.with_permit(self.backend.store_edges_batch(edges))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ConversationSession;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_pool_creation() {
        let dir = tempdir().unwrap();
        let config = PoolConfig::new().with_max_concurrent(10);

        let pool = PooledAsyncBackend::open(dir.path(), config).await.unwrap();

        assert_eq!(pool.available_permits(), 10);
        assert_eq!(pool.config().max_concurrent, 10);
    }

    #[tokio::test]
    async fn test_pool_operations() {
        let dir = tempdir().unwrap();
        let config = PoolConfig::new();

        let pool = PooledAsyncBackend::open(dir.path(), config).await.unwrap();

        // Create session
        let session = ConversationSession::new();
        let node = Node::Session(session.clone());

        // Store node
        pool.store_node(&node).await.unwrap();

        // Retrieve node
        let retrieved = pool.get_node(&session.node_id).await.unwrap();
        assert!(retrieved.is_some());

        // Check metrics
        let metrics = pool.metrics();
        assert!(metrics.total_operations >= 2); // At least store + get
        assert!(metrics.successful_operations >= 2);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let dir = tempdir().unwrap();
        let config = PoolConfig::new().with_max_concurrent(20);

        let pool = Arc::new(PooledAsyncBackend::open(dir.path(), config).await.unwrap());

        // Create 50 concurrent operations
        let mut handles = vec![];
        for _ in 0..50 {
            let pool_clone = Arc::clone(&pool);
            let handle = tokio::spawn(async move {
                let session = ConversationSession::new();
                let node = Node::Session(session);
                pool_clone.store_node(&node).await
            });
            handles.push(handle);
        }

        // Wait for all operations
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Check metrics
        let metrics = pool.metrics();
        assert_eq!(metrics.total_operations, 50);
        assert_eq!(metrics.successful_operations, 50);
        assert!(metrics.peak_concurrent <= 20); // Shouldn't exceed pool size
    }

    #[tokio::test]
    async fn test_pool_backpressure() {
        let dir = tempdir().unwrap();
        let config = PoolConfig::new()
            .with_max_concurrent(2) // Very small pool
            .with_timeout(1000); // 1 second timeout

        let pool = Arc::new(PooledAsyncBackend::open(dir.path(), config).await.unwrap());

        // Start 2 long-running operations to fill the pool
        let pool1 = Arc::clone(&pool);
        let handle1 = tokio::spawn(async move {
            let _permit = pool1.acquire_permit().await.unwrap();
            tokio::time::sleep(Duration::from_millis(500)).await;
        });

        let pool2 = Arc::clone(&pool);
        let handle2 = tokio::spawn(async move {
            let _permit = pool2.acquire_permit().await.unwrap();
            tokio::time::sleep(Duration::from_millis(500)).await;
        });

        // Give time for permits to be acquired
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Pool should be full
        assert_eq!(pool.available_permits(), 0);

        // Wait for operations to complete
        handle1.await.unwrap();
        handle2.await.unwrap();

        // Permits should be returned
        assert_eq!(pool.available_permits(), 2);
    }

    #[tokio::test]
    async fn test_metrics_tracking() {
        let dir = tempdir().unwrap();
        let config = PoolConfig::new().with_metrics(true);

        let pool = PooledAsyncBackend::open(dir.path(), config).await.unwrap();

        // Perform operations
        for _ in 0..10 {
            let session = ConversationSession::new();
            pool.store_node(&Node::Session(session)).await.unwrap();
        }

        let metrics = pool.metrics();
        assert_eq!(metrics.total_operations, 10);
        assert_eq!(metrics.successful_operations, 10);
        assert_eq!(metrics.failed_operations, 0);
        assert!(metrics.avg_wait_time_ms() >= 0.0);
        assert_eq!(metrics.success_rate(), 1.0);
    }

    #[tokio::test]
    async fn test_batch_operations_with_pool() {
        let dir = tempdir().unwrap();
        let config = PoolConfig::new();

        let pool = PooledAsyncBackend::open(dir.path(), config).await.unwrap();

        // Create batch of nodes
        let nodes: Vec<Node> = (0..10)
            .map(|_| Node::Session(ConversationSession::new()))
            .collect();

        let ids = pool.store_nodes_batch(&nodes).await.unwrap();
        assert_eq!(ids.len(), 10);

        // Check metrics - batch should count as 1 operation
        let metrics = pool.metrics();
        assert_eq!(metrics.total_operations, 1);
    }
}

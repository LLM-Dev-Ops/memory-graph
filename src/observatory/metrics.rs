//! Metrics collection for memory graph operations

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

/// Metrics collector for memory graph operations
#[derive(Clone)]
pub struct MemoryGraphMetrics {
    // Counters
    nodes_created: Arc<AtomicUsize>,
    edges_created: Arc<AtomicUsize>,
    prompts_submitted: Arc<AtomicUsize>,
    responses_generated: Arc<AtomicUsize>,
    tools_invoked: Arc<AtomicUsize>,
    queries_executed: Arc<AtomicUsize>,

    // Latency tracking (in microseconds for precision)
    total_write_latency_us: Arc<AtomicU64>,
    write_count: Arc<AtomicUsize>,
    total_read_latency_us: Arc<AtomicU64>,
    read_count: Arc<AtomicUsize>,
}

impl MemoryGraphMetrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            nodes_created: Arc::new(AtomicUsize::new(0)),
            edges_created: Arc::new(AtomicUsize::new(0)),
            prompts_submitted: Arc::new(AtomicUsize::new(0)),
            responses_generated: Arc::new(AtomicUsize::new(0)),
            tools_invoked: Arc::new(AtomicUsize::new(0)),
            queries_executed: Arc::new(AtomicUsize::new(0)),
            total_write_latency_us: Arc::new(AtomicU64::new(0)),
            write_count: Arc::new(AtomicUsize::new(0)),
            total_read_latency_us: Arc::new(AtomicU64::new(0)),
            read_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Record a node creation
    pub fn record_node_created(&self) {
        self.nodes_created.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an edge creation
    pub fn record_edge_created(&self) {
        self.edges_created.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a prompt submission
    pub fn record_prompt_submitted(&self) {
        self.prompts_submitted.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a response generation
    pub fn record_response_generated(&self) {
        self.responses_generated.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a tool invocation
    pub fn record_tool_invoked(&self) {
        self.tools_invoked.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a query execution
    pub fn record_query_executed(&self) {
        self.queries_executed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record write latency in microseconds
    pub fn record_write_latency_us(&self, latency_us: u64) {
        self.total_write_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
        self.write_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record read latency in microseconds
    pub fn record_read_latency_us(&self, latency_us: u64) {
        self.total_read_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
        self.read_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get a snapshot of current metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        let write_count = self.write_count.load(Ordering::Relaxed);
        let read_count = self.read_count.load(Ordering::Relaxed);

        let avg_write_latency_us = if write_count > 0 {
            self.total_write_latency_us.load(Ordering::Relaxed) as f64 / write_count as f64
        } else {
            0.0
        };

        let avg_read_latency_us = if read_count > 0 {
            self.total_read_latency_us.load(Ordering::Relaxed) as f64 / read_count as f64
        } else {
            0.0
        };

        MetricsSnapshot {
            nodes_created: self.nodes_created.load(Ordering::Relaxed),
            edges_created: self.edges_created.load(Ordering::Relaxed),
            prompts_submitted: self.prompts_submitted.load(Ordering::Relaxed),
            responses_generated: self.responses_generated.load(Ordering::Relaxed),
            tools_invoked: self.tools_invoked.load(Ordering::Relaxed),
            queries_executed: self.queries_executed.load(Ordering::Relaxed),
            avg_write_latency_ms: avg_write_latency_us / 1000.0,
            avg_read_latency_ms: avg_read_latency_us / 1000.0,
        }
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.nodes_created.store(0, Ordering::Relaxed);
        self.edges_created.store(0, Ordering::Relaxed);
        self.prompts_submitted.store(0, Ordering::Relaxed);
        self.responses_generated.store(0, Ordering::Relaxed);
        self.tools_invoked.store(0, Ordering::Relaxed);
        self.queries_executed.store(0, Ordering::Relaxed);
        self.total_write_latency_us.store(0, Ordering::Relaxed);
        self.write_count.store(0, Ordering::Relaxed);
        self.total_read_latency_us.store(0, Ordering::Relaxed);
        self.read_count.store(0, Ordering::Relaxed);
    }
}

impl Default for MemoryGraphMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    /// Total nodes created
    pub nodes_created: usize,
    /// Total edges created
    pub edges_created: usize,
    /// Total prompts submitted
    pub prompts_submitted: usize,
    /// Total responses generated
    pub responses_generated: usize,
    /// Total tools invoked
    pub tools_invoked: usize,
    /// Total queries executed
    pub queries_executed: usize,
    /// Average write latency in milliseconds
    pub avg_write_latency_ms: f64,
    /// Average read latency in milliseconds
    pub avg_read_latency_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = MemoryGraphMetrics::new();
        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.nodes_created, 0);
        assert_eq!(snapshot.edges_created, 0);
        assert_eq!(snapshot.prompts_submitted, 0);
    }

    #[test]
    fn test_metrics_recording() {
        let metrics = MemoryGraphMetrics::new();

        metrics.record_node_created();
        metrics.record_node_created();
        metrics.record_edge_created();
        metrics.record_prompt_submitted();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.nodes_created, 2);
        assert_eq!(snapshot.edges_created, 1);
        assert_eq!(snapshot.prompts_submitted, 1);
    }

    #[test]
    fn test_latency_tracking() {
        let metrics = MemoryGraphMetrics::new();

        // Record some write latencies (in microseconds)
        metrics.record_write_latency_us(1000); // 1ms
        metrics.record_write_latency_us(2000); // 2ms
        metrics.record_write_latency_us(3000); // 3ms

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.avg_write_latency_ms, 2.0); // Average of 1, 2, 3 ms
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = MemoryGraphMetrics::new();

        metrics.record_node_created();
        metrics.record_edge_created();
        metrics.record_prompt_submitted();

        let snapshot_before = metrics.snapshot();
        assert_eq!(snapshot_before.nodes_created, 1);

        metrics.reset();

        let snapshot_after = metrics.snapshot();
        assert_eq!(snapshot_after.nodes_created, 0);
        assert_eq!(snapshot_after.edges_created, 0);
        assert_eq!(snapshot_after.prompts_submitted, 0);
    }

    #[test]
    fn test_concurrent_metrics_update() {
        use std::sync::Arc;
        use std::thread;

        let metrics = Arc::new(MemoryGraphMetrics::new());
        let mut handles = vec![];

        // Spawn 10 threads, each incrementing counters 100 times
        for _ in 0..10 {
            let metrics_clone = Arc::clone(&metrics);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    metrics_clone.record_node_created();
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.nodes_created, 1000);
    }
}

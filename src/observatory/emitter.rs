//! Async event emitter for non-blocking event emission
//!
//! This module provides async event emission that doesn't block the main operation flow.
//! Events are emitted in background tasks using `tokio::spawn`, ensuring that event
//! publishing never delays critical operations.
//!
//! # Features
//!
//! - **Non-blocking**: Events are sent in background tasks
//! - **Error resilience**: Emission errors don't affect main operations
//! - **Statistics**: Track emission success/failure rates
//! - **Graceful degradation**: Continues operating even if event system fails
//!
//! # Examples
//!
//! ```no_run
//! use llm_memory_graph::observatory::{AsyncEventEmitter, InMemoryPublisher, MemoryGraphEvent};
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let publisher = Arc::new(InMemoryPublisher::new());
//! let emitter = AsyncEventEmitter::new(publisher);
//!
//! // Emit event without blocking
//! let event = MemoryGraphEvent::QueryExecuted {
//!     query_type: "test".to_string(),
//!     results_count: 10,
//!     duration_ms: 50,
//!     timestamp: chrono::Utc::now(),
//! };
//!
//! emitter.emit(event);
//!
//! // Get statistics
//! let stats = emitter.stats().await;
//! println!("Emitted: {}, Failed: {}", stats.events_emitted, stats.events_failed);
//! # Ok(())
//! # }
//! ```

use super::events::MemoryGraphEvent;
use super::publisher::EventPublisher;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Async event emitter for non-blocking event emission
///
/// This struct wraps an EventPublisher and provides fire-and-forget
/// emission semantics. Events are sent in background tasks using
/// `tokio::spawn`, ensuring they never block the caller.
#[derive(Clone)]
pub struct AsyncEventEmitter<P: EventPublisher + 'static> {
    /// The underlying event publisher
    publisher: Arc<P>,
    /// Statistics tracking
    stats: Arc<EmissionStats>,
    /// Whether to log errors
    log_errors: bool,
}

impl<P: EventPublisher + 'static> AsyncEventEmitter<P> {
    /// Create a new async event emitter
    ///
    /// # Arguments
    ///
    /// * `publisher` - The event publisher to use for sending events
    ///
    /// # Examples
    ///
    /// ```
    /// use llm_memory_graph::observatory::{AsyncEventEmitter, InMemoryPublisher};
    /// use std::sync::Arc;
    ///
    /// let publisher = Arc::new(InMemoryPublisher::new());
    /// let emitter = AsyncEventEmitter::new(publisher);
    /// ```
    pub fn new(publisher: Arc<P>) -> Self {
        Self {
            publisher,
            stats: Arc::new(EmissionStats::new()),
            log_errors: true,
        }
    }

    /// Create a new async event emitter without error logging
    pub fn new_silent(publisher: Arc<P>) -> Self {
        Self {
            publisher,
            stats: Arc::new(EmissionStats::new()),
            log_errors: false,
        }
    }

    /// Emit an event without blocking
    ///
    /// This method spawns a background task to publish the event and returns
    /// immediately. Errors during emission are logged but don't affect the caller.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to emit
    ///
    /// # Examples
    ///
    /// ```
    /// # use llm_memory_graph::observatory::{AsyncEventEmitter, InMemoryPublisher, MemoryGraphEvent};
    /// # use std::sync::Arc;
    /// # async fn example() {
    /// let publisher = Arc::new(InMemoryPublisher::new());
    /// let emitter = AsyncEventEmitter::new(publisher);
    ///
    /// let event = MemoryGraphEvent::QueryExecuted {
    ///     query_type: "test".to_string(),
    ///     results_count: 10,
    ///     duration_ms: 50,
    ///     timestamp: chrono::Utc::now(),
    /// };
    ///
    /// emitter.emit(event); // Returns immediately
    /// # }
    /// ```
    pub fn emit(&self, event: MemoryGraphEvent) {
        let publisher = Arc::clone(&self.publisher);
        let stats = Arc::clone(&self.stats);
        let log_errors = self.log_errors;

        tokio::spawn(async move {
            stats.inc_submitted();

            match publisher.publish(event).await {
                Ok(()) => {
                    stats.inc_emitted();
                }
                Err(e) => {
                    stats.inc_failed();
                    if log_errors {
                        tracing::warn!("Failed to emit event: {}", e);
                    }
                }
            }
        });
    }

    /// Emit multiple events without blocking
    ///
    /// # Arguments
    ///
    /// * `events` - Vector of events to emit
    pub fn emit_batch(&self, events: Vec<MemoryGraphEvent>) {
        let publisher = Arc::clone(&self.publisher);
        let stats = Arc::clone(&self.stats);
        let log_errors = self.log_errors;
        let count = events.len() as u64;

        tokio::spawn(async move {
            stats.inc_submitted_by(count);

            match publisher.publish_batch(events).await {
                Ok(()) => {
                    stats.inc_emitted_by(count);
                }
                Err(e) => {
                    stats.inc_failed_by(count);
                    if log_errors {
                        tracing::warn!("Failed to emit event batch: {}", e);
                    }
                }
            }
        });
    }

    /// Emit an event and wait for completion
    ///
    /// Unlike `emit()`, this method waits for the event to be published
    /// and returns any errors. Useful for testing and critical events.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to emit
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the event was successfully published
    pub async fn emit_sync(&self, event: MemoryGraphEvent) -> crate::error::Result<()> {
        self.stats.inc_submitted();

        match self.publisher.publish(event).await {
            Ok(()) => {
                self.stats.inc_emitted();
                Ok(())
            }
            Err(e) => {
                self.stats.inc_failed();
                if self.log_errors {
                    tracing::warn!("Failed to emit event: {}", e);
                }
                Err(e)
            }
        }
    }

    /// Get emission statistics
    ///
    /// # Returns
    ///
    /// Returns a snapshot of current emission statistics
    pub async fn stats(&self) -> EmissionStatsSnapshot {
        self.stats.snapshot().await
    }

    /// Reset all statistics to zero
    pub async fn reset_stats(&self) {
        self.stats.reset().await;
    }

    /// Get the underlying publisher
    pub fn publisher(&self) -> &Arc<P> {
        &self.publisher
    }
}

/// Statistics for event emission
struct EmissionStats {
    /// Total events submitted for emission
    events_submitted: AtomicU64,
    /// Total events successfully emitted
    events_emitted: AtomicU64,
    /// Total events that failed to emit
    events_failed: AtomicU64,
    /// Peak concurrent emissions (for monitoring)
    peak_concurrent: RwLock<u64>,
}

impl EmissionStats {
    fn new() -> Self {
        Self {
            events_submitted: AtomicU64::new(0),
            events_emitted: AtomicU64::new(0),
            events_failed: AtomicU64::new(0),
            peak_concurrent: RwLock::new(0),
        }
    }

    fn inc_submitted(&self) {
        self.events_submitted.fetch_add(1, Ordering::Relaxed);
    }

    fn inc_submitted_by(&self, count: u64) {
        self.events_submitted.fetch_add(count, Ordering::Relaxed);
    }

    fn inc_emitted(&self) {
        self.events_emitted.fetch_add(1, Ordering::Relaxed);
    }

    fn inc_emitted_by(&self, count: u64) {
        self.events_emitted.fetch_add(count, Ordering::Relaxed);
    }

    fn inc_failed(&self) {
        self.events_failed.fetch_add(1, Ordering::Relaxed);
    }

    fn inc_failed_by(&self, count: u64) {
        self.events_failed.fetch_add(count, Ordering::Relaxed);
    }

    async fn snapshot(&self) -> EmissionStatsSnapshot {
        EmissionStatsSnapshot {
            events_submitted: self.events_submitted.load(Ordering::Relaxed),
            events_emitted: self.events_emitted.load(Ordering::Relaxed),
            events_failed: self.events_failed.load(Ordering::Relaxed),
            peak_concurrent: *self.peak_concurrent.read().await,
        }
    }

    async fn reset(&self) {
        self.events_submitted.store(0, Ordering::Relaxed);
        self.events_emitted.store(0, Ordering::Relaxed);
        self.events_failed.store(0, Ordering::Relaxed);
        *self.peak_concurrent.write().await = 0;
    }
}

/// Snapshot of emission statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmissionStatsSnapshot {
    /// Total events submitted for emission
    pub events_submitted: u64,
    /// Total events successfully emitted
    pub events_emitted: u64,
    /// Total events that failed to emit
    pub events_failed: u64,
    /// Peak concurrent emissions
    pub peak_concurrent: u64,
}

impl EmissionStatsSnapshot {
    /// Calculate success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.events_submitted == 0 {
            100.0
        } else {
            (self.events_emitted as f64 / self.events_submitted as f64) * 100.0
        }
    }

    /// Calculate failure rate as a percentage
    pub fn failure_rate(&self) -> f64 {
        if self.events_submitted == 0 {
            0.0
        } else {
            (self.events_failed as f64 / self.events_submitted as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observatory::publisher::InMemoryPublisher;
    use crate::types::{NodeId, NodeType, SessionId};
    use chrono::Utc;
    use std::collections::HashMap;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_emitter_creation() {
        let publisher = Arc::new(InMemoryPublisher::new());
        let emitter = AsyncEventEmitter::new(publisher.clone());

        let stats = emitter.stats().await;
        assert_eq!(stats.events_submitted, 0);
        assert_eq!(stats.events_emitted, 0);
        assert_eq!(stats.events_failed, 0);
    }

    #[tokio::test]
    async fn test_emit_single_event() {
        let publisher = Arc::new(InMemoryPublisher::new());
        let emitter = AsyncEventEmitter::new(publisher.clone());

        let event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: Some(SessionId::new()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        emitter.emit(event);

        // Wait for async task to complete
        sleep(Duration::from_millis(50)).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.events_submitted, 1);
        assert_eq!(stats.events_emitted, 1);
        assert_eq!(stats.events_failed, 0);

        // Verify event was published
        let published = publisher.get_events().await;
        assert_eq!(published.len(), 1);
    }

    #[tokio::test]
    async fn test_emit_multiple_events() {
        let publisher = Arc::new(InMemoryPublisher::new());
        let emitter = AsyncEventEmitter::new(publisher.clone());

        for _ in 0..10 {
            let event = MemoryGraphEvent::QueryExecuted {
                query_type: "test".to_string(),
                results_count: 5,
                duration_ms: 10,
                timestamp: Utc::now(),
            };
            emitter.emit(event);
        }

        // Wait for async tasks to complete
        sleep(Duration::from_millis(100)).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.events_submitted, 10);
        assert_eq!(stats.events_emitted, 10);
        assert_eq!(stats.events_failed, 0);

        let published = publisher.get_events().await;
        assert_eq!(published.len(), 10);
    }

    #[tokio::test]
    async fn test_emit_batch() {
        let publisher = Arc::new(InMemoryPublisher::new());
        let emitter = AsyncEventEmitter::new(publisher.clone());

        let events = vec![
            MemoryGraphEvent::NodeCreated {
                node_id: NodeId::new(),
                node_type: NodeType::Prompt,
                session_id: None,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            },
            MemoryGraphEvent::NodeCreated {
                node_id: NodeId::new(),
                node_type: NodeType::Response,
                session_id: None,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            },
            MemoryGraphEvent::QueryExecuted {
                query_type: "batch".to_string(),
                results_count: 2,
                duration_ms: 15,
                timestamp: Utc::now(),
            },
        ];

        emitter.emit_batch(events);

        // Wait for async task to complete
        sleep(Duration::from_millis(50)).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.events_submitted, 3);
        assert_eq!(stats.events_emitted, 3);

        let published = publisher.get_events().await;
        assert_eq!(published.len(), 3);
    }

    #[tokio::test]
    async fn test_emit_sync() {
        let publisher = Arc::new(InMemoryPublisher::new());
        let emitter = AsyncEventEmitter::new(publisher.clone());

        let event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        // This should complete synchronously
        emitter.emit_sync(event).await.unwrap();

        let stats = emitter.stats().await;
        assert_eq!(stats.events_submitted, 1);
        assert_eq!(stats.events_emitted, 1);

        let published = publisher.get_events().await;
        assert_eq!(published.len(), 1);
    }

    #[tokio::test]
    async fn test_concurrent_emission() {
        let publisher = Arc::new(InMemoryPublisher::new());
        let emitter = AsyncEventEmitter::new(publisher.clone());

        let mut handles = vec![];

        for i in 0..50 {
            let emitter_clone = emitter.clone();
            let handle = tokio::spawn(async move {
                let event = MemoryGraphEvent::QueryExecuted {
                    query_type: format!("query_{}", i),
                    results_count: i,
                    duration_ms: 10,
                    timestamp: Utc::now(),
                };
                emitter_clone.emit(event);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        // Wait for all async emissions to complete
        sleep(Duration::from_millis(200)).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.events_submitted, 50);
        assert_eq!(stats.events_emitted, 50);

        let published = publisher.get_events().await;
        assert_eq!(published.len(), 50);
    }

    #[tokio::test]
    async fn test_stats_snapshot() {
        let publisher = Arc::new(InMemoryPublisher::new());
        let emitter = AsyncEventEmitter::new(publisher);

        let event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        emitter.emit(event);
        sleep(Duration::from_millis(50)).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.success_rate(), 100.0);
        assert_eq!(stats.failure_rate(), 0.0);
    }

    #[tokio::test]
    async fn test_reset_stats() {
        let publisher = Arc::new(InMemoryPublisher::new());
        let emitter = AsyncEventEmitter::new(publisher);

        let event = MemoryGraphEvent::QueryExecuted {
            query_type: "test".to_string(),
            results_count: 1,
            duration_ms: 10,
            timestamp: Utc::now(),
        };

        emitter.emit(event);
        sleep(Duration::from_millis(50)).await;

        let stats_before = emitter.stats().await;
        assert_eq!(stats_before.events_emitted, 1);

        emitter.reset_stats().await;

        let stats_after = emitter.stats().await;
        assert_eq!(stats_after.events_submitted, 0);
        assert_eq!(stats_after.events_emitted, 0);
        assert_eq!(stats_after.events_failed, 0);
    }

    #[tokio::test]
    async fn test_silent_emitter() {
        let publisher = Arc::new(InMemoryPublisher::new());
        let emitter = AsyncEventEmitter::new_silent(publisher.clone());

        let event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        // Should emit without logging errors
        emitter.emit(event);
        sleep(Duration::from_millis(50)).await;

        let published = publisher.get_events().await;
        assert_eq!(published.len(), 1);
    }

    #[tokio::test]
    async fn test_mixed_emit_modes() {
        let publisher = Arc::new(InMemoryPublisher::new());
        let emitter = AsyncEventEmitter::new(publisher.clone());

        // Mix async and sync emissions
        let event1 = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let event2 = MemoryGraphEvent::QueryExecuted {
            query_type: "test".to_string(),
            results_count: 1,
            duration_ms: 10,
            timestamp: Utc::now(),
        };

        emitter.emit(event1);
        emitter.emit_sync(event2).await.unwrap();

        sleep(Duration::from_millis(50)).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.events_submitted, 2);
        assert_eq!(stats.events_emitted, 2);

        let published = publisher.get_events().await;
        assert_eq!(published.len(), 2);
    }

    #[tokio::test]
    async fn test_high_throughput() {
        let publisher = Arc::new(InMemoryPublisher::new());
        let emitter = AsyncEventEmitter::new(publisher.clone());

        // Emit 1000 events rapidly
        for i in 0..1000 {
            let event = MemoryGraphEvent::QueryExecuted {
                query_type: format!("query_{}", i),
                results_count: i,
                duration_ms: 1,
                timestamp: Utc::now(),
            };
            emitter.emit(event);
        }

        // Wait for all emissions to complete
        sleep(Duration::from_millis(500)).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.events_submitted, 1000);
        assert_eq!(stats.events_emitted, 1000);
        assert_eq!(stats.events_failed, 0);

        let published = publisher.get_events().await;
        assert_eq!(published.len(), 1000);
    }

    #[tokio::test]
    async fn test_success_failure_rates() {
        let publisher = Arc::new(InMemoryPublisher::new());
        let emitter = AsyncEventEmitter::new(publisher);

        // All events should succeed with InMemoryPublisher
        for _ in 0..10 {
            let event = MemoryGraphEvent::NodeCreated {
                node_id: NodeId::new(),
                node_type: NodeType::Prompt,
                session_id: None,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            };
            emitter.emit(event);
        }

        sleep(Duration::from_millis(100)).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.success_rate(), 100.0);
        assert_eq!(stats.failure_rate(), 0.0);
    }
}

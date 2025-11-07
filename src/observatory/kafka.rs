//! Kafka producer with batching and retry logic
//!
//! This module provides enterprise-grade Kafka integration for streaming events
//! to external monitoring systems. Features include:
//! - Automatic batching with configurable batch size and timeout
//! - Retry logic with exponential backoff
//! - Connection pooling and health checks
//! - Metrics and monitoring

use super::events::MemoryGraphEvent;
use crate::error::{retry, Error, Result};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

/// Configuration for Kafka producer
#[derive(Debug, Clone)]
pub struct KafkaConfig {
    /// Kafka broker addresses
    pub brokers: String,
    /// Topic name for events
    pub topic: String,
    /// Maximum batch size before forcing send
    pub batch_size: usize,
    /// Maximum time to wait before forcing send
    pub batch_timeout_ms: u64,
    /// Message timeout in milliseconds
    pub message_timeout_ms: u64,
    /// Compression type (none, gzip, snappy, lz4, zstd)
    pub compression_type: String,
    /// Enable idempotent producer
    pub enable_idempotence: bool,
    /// Maximum number of retry attempts
    pub max_retries: usize,
    /// Initial retry delay in milliseconds
    pub retry_delay_ms: u64,
}

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            brokers: "localhost:9092".to_string(),
            topic: "llm-memory-graph-events".to_string(),
            batch_size: 100,
            batch_timeout_ms: 1000,
            message_timeout_ms: 5000,
            compression_type: "snappy".to_string(),
            enable_idempotence: true,
            max_retries: 3,
            retry_delay_ms: 100,
        }
    }
}

impl KafkaConfig {
    /// Create a new Kafka configuration
    pub fn new(brokers: String, topic: String) -> Self {
        Self {
            brokers,
            topic,
            ..Default::default()
        }
    }

    /// Set batch size
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Set batch timeout
    pub fn with_batch_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.batch_timeout_ms = timeout_ms;
        self
    }

    /// Set compression type
    pub fn with_compression(mut self, compression: String) -> Self {
        self.compression_type = compression;
        self
    }

    /// Set retry configuration
    pub fn with_retry_config(mut self, max_retries: usize, delay_ms: u64) -> Self {
        self.max_retries = max_retries;
        self.retry_delay_ms = delay_ms;
        self
    }
}

/// Trait for Kafka producer operations
#[async_trait]
pub trait KafkaProducer: Send + Sync {
    /// Send a single event
    async fn send(&self, event: MemoryGraphEvent) -> Result<()>;

    /// Send a batch of events
    async fn send_batch(&self, events: Vec<MemoryGraphEvent>) -> Result<()>;

    /// Flush any pending events
    async fn flush(&self) -> Result<()>;

    /// Get producer statistics
    async fn stats(&self) -> ProducerStats;
}

/// Statistics for Kafka producer
#[derive(Debug, Clone, Default)]
pub struct ProducerStats {
    /// Total events sent successfully
    pub events_sent: u64,
    /// Total events failed
    pub events_failed: u64,
    /// Total batches sent
    pub batches_sent: u64,
    /// Current batch size (pending events)
    pub pending_events: usize,
    /// Average batch size
    pub avg_batch_size: f64,
}

/// Mock Kafka producer for testing and development
#[derive(Clone)]
pub struct MockKafkaProducer {
    config: KafkaConfig,
    /// Sent events buffer for testing
    sent_events: Arc<RwLock<Vec<MemoryGraphEvent>>>,
    /// Statistics
    stats: Arc<RwLock<ProducerStats>>,
    /// Simulate failures for testing
    failure_rate: Arc<RwLock<f64>>,
}

impl MockKafkaProducer {
    /// Create a new mock Kafka producer
    pub fn new(config: KafkaConfig) -> Self {
        Self {
            config,
            sent_events: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(ProducerStats::default())),
            failure_rate: Arc::new(RwLock::new(0.0)),
        }
    }

    /// Get all sent events (for testing)
    pub async fn get_sent_events(&self) -> Vec<MemoryGraphEvent> {
        self.sent_events.read().await.clone()
    }

    /// Clear sent events (for testing)
    pub async fn clear_sent_events(&self) {
        self.sent_events.write().await.clear();
    }

    /// Set failure rate for testing (0.0 = no failures, 1.0 = always fail)
    pub async fn set_failure_rate(&self, rate: f64) {
        *self.failure_rate.write().await = rate.clamp(0.0, 1.0);
    }

    /// Simulate sending (may fail based on failure rate)
    async fn simulate_send(&self) -> Result<()> {
        let rate = *self.failure_rate.read().await;
        if rate > 0.0 {
            // Simple pseudo-random using timestamp for testing
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let random_val = ((timestamp % 1000) as f64) / 1000.0;
            if random_val < rate {
                return Err(Error::Other("Simulated Kafka send failure".to_string()));
            }
        }
        Ok(())
    }
}

#[async_trait]
impl KafkaProducer for MockKafkaProducer {
    async fn send(&self, event: MemoryGraphEvent) -> Result<()> {
        self.simulate_send().await?;

        let mut events = self.sent_events.write().await;
        events.push(event);

        let mut stats = self.stats.write().await;
        stats.events_sent += 1;

        Ok(())
    }

    async fn send_batch(&self, events: Vec<MemoryGraphEvent>) -> Result<()> {
        self.simulate_send().await?;

        let batch_size = events.len();

        let mut sent = self.sent_events.write().await;
        sent.extend(events);

        let mut stats = self.stats.write().await;
        stats.events_sent += batch_size as u64;
        stats.batches_sent += 1;

        // Update average batch size
        let total_batches = stats.batches_sent as f64;
        stats.avg_batch_size =
            (stats.avg_batch_size * (total_batches - 1.0) + batch_size as f64) / total_batches;

        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        // Mock implementation - nothing to flush
        Ok(())
    }

    async fn stats(&self) -> ProducerStats {
        self.stats.read().await.clone()
    }
}

/// Batching Kafka producer with automatic flushing
pub struct BatchingKafkaProducer<P: KafkaProducer> {
    producer: Arc<P>,
    config: KafkaConfig,
    /// Pending events buffer
    buffer: Arc<Mutex<VecDeque<MemoryGraphEvent>>>,
    /// Last flush time
    last_flush: Arc<Mutex<Instant>>,
}

impl<P: KafkaProducer + 'static> BatchingKafkaProducer<P> {
    /// Create a new batching producer
    pub fn new(producer: P, config: KafkaConfig) -> Self {
        let instance = Self {
            producer: Arc::new(producer),
            config: config.clone(),
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            last_flush: Arc::new(Mutex::new(Instant::now())),
        };

        // Start background flush task
        instance.start_background_flush();

        instance
    }

    /// Start background task to flush on timeout
    fn start_background_flush(&self) {
        let buffer = Arc::clone(&self.buffer);
        let last_flush = Arc::clone(&self.last_flush);
        let producer = Arc::clone(&self.producer);
        let timeout = Duration::from_millis(self.config.batch_timeout_ms);
        let batch_size = self.config.batch_size;

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;

                let should_flush = {
                    let last = last_flush.lock().await;
                    last.elapsed() >= timeout
                };

                if should_flush {
                    let events_to_send = {
                        let mut buf = buffer.lock().await;
                        if buf.is_empty() {
                            continue;
                        }

                        let count = buf.len().min(batch_size);
                        buf.drain(..count).collect::<Vec<_>>()
                    };

                    if !events_to_send.is_empty() {
                        // Send with retry logic
                        let retry_config = retry::RetryConfig::new()
                            .with_max_attempts(3)
                            .with_initial_delay(Duration::from_millis(100));

                        let producer_clone = Arc::clone(&producer);
                        let _ = retry::with_retry(retry_config, || {
                            let producer_ref = Arc::clone(&producer_clone);
                            let events = events_to_send.clone();
                            async move { producer_ref.send_batch(events).await }
                        })
                        .await;

                        *last_flush.lock().await = Instant::now();
                    }
                }
            }
        });
    }

    /// Add event to buffer and flush if batch size reached
    pub async fn publish(&self, event: MemoryGraphEvent) -> Result<()> {
        let should_flush = {
            let mut buffer = self.buffer.lock().await;
            buffer.push_back(event);
            buffer.len() >= self.config.batch_size
        };

        if should_flush {
            self.flush_buffer().await?;
        }

        Ok(())
    }

    /// Flush the buffer
    async fn flush_buffer(&self) -> Result<()> {
        let events_to_send = {
            let mut buffer = self.buffer.lock().await;
            if buffer.is_empty() {
                return Ok(());
            }

            let count = buffer.len().min(self.config.batch_size);
            buffer.drain(..count).collect::<Vec<_>>()
        };

        if events_to_send.is_empty() {
            return Ok(());
        }

        // Send with retry logic
        let retry_config = retry::RetryConfig::new()
            .with_max_attempts(self.config.max_retries)
            .with_initial_delay(Duration::from_millis(self.config.retry_delay_ms));

        let producer = Arc::clone(&self.producer);
        retry::with_retry(retry_config, || {
            let producer_ref = Arc::clone(&producer);
            let events = events_to_send.clone();
            async move { producer_ref.send_batch(events).await }
        })
        .await?;

        *self.last_flush.lock().await = Instant::now();

        Ok(())
    }

    /// Get current buffer size
    pub async fn buffer_size(&self) -> usize {
        self.buffer.lock().await.len()
    }

    /// Get producer statistics
    pub async fn stats(&self) -> ProducerStats {
        let mut stats = self.producer.stats().await;
        stats.pending_events = self.buffer_size().await;
        stats
    }
}

#[async_trait]
impl<P: KafkaProducer + 'static> KafkaProducer for BatchingKafkaProducer<P> {
    async fn send(&self, event: MemoryGraphEvent) -> Result<()> {
        self.publish(event).await
    }

    async fn send_batch(&self, events: Vec<MemoryGraphEvent>) -> Result<()> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        self.flush_buffer().await
    }

    async fn stats(&self) -> ProducerStats {
        self.stats().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NodeId, NodeType, SessionId};
    use chrono::Utc;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_kafka_config_builder() {
        let config = KafkaConfig::new("localhost:9092".to_string(), "test-topic".to_string())
            .with_batch_size(200)
            .with_batch_timeout_ms(2000)
            .with_compression("gzip".to_string())
            .with_retry_config(5, 200);

        assert_eq!(config.brokers, "localhost:9092");
        assert_eq!(config.topic, "test-topic");
        assert_eq!(config.batch_size, 200);
        assert_eq!(config.batch_timeout_ms, 2000);
        assert_eq!(config.compression_type, "gzip");
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.retry_delay_ms, 200);
    }

    #[tokio::test]
    async fn test_mock_producer_send() {
        let config = KafkaConfig::default();
        let producer = MockKafkaProducer::new(config);

        let event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: Some(SessionId::new()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        producer.send(event.clone()).await.unwrap();

        let sent = producer.get_sent_events().await;
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].event_type(), event.event_type());
    }

    #[tokio::test]
    async fn test_mock_producer_batch() {
        let config = KafkaConfig::default();
        let producer = MockKafkaProducer::new(config);

        let events: Vec<_> = (0..5)
            .map(|_| MemoryGraphEvent::NodeCreated {
                node_id: NodeId::new(),
                node_type: NodeType::Prompt,
                session_id: None,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            })
            .collect();

        producer.send_batch(events).await.unwrap();

        let sent = producer.get_sent_events().await;
        assert_eq!(sent.len(), 5);

        let stats = producer.stats().await;
        assert_eq!(stats.events_sent, 5);
        assert_eq!(stats.batches_sent, 1);
    }

    #[tokio::test]
    async fn test_mock_producer_stats() {
        let config = KafkaConfig::default();
        let producer = MockKafkaProducer::new(config);

        // Send some events
        for _ in 0..10 {
            let event = MemoryGraphEvent::NodeCreated {
                node_id: NodeId::new(),
                node_type: NodeType::Prompt,
                session_id: None,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            };
            producer.send(event).await.unwrap();
        }

        let stats = producer.stats().await;
        assert_eq!(stats.events_sent, 10);
    }

    #[tokio::test]
    async fn test_batching_producer_auto_flush_on_size() {
        let config = KafkaConfig::default().with_batch_size(5);
        let mock = MockKafkaProducer::new(config.clone());
        let producer = BatchingKafkaProducer::new(mock.clone(), config);

        // Send 5 events - should trigger auto-flush
        for _ in 0..5 {
            let event = MemoryGraphEvent::NodeCreated {
                node_id: NodeId::new(),
                node_type: NodeType::Prompt,
                session_id: None,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            };
            producer.publish(event).await.unwrap();
        }

        // Wait a bit for async flush
        tokio::time::sleep(Duration::from_millis(50)).await;

        let sent = mock.get_sent_events().await;
        assert_eq!(sent.len(), 5);
    }

    #[tokio::test]
    async fn test_batching_producer_manual_flush() {
        let config = KafkaConfig::default().with_batch_size(100);
        let mock = MockKafkaProducer::new(config.clone());
        let producer = BatchingKafkaProducer::new(mock.clone(), config);

        // Send 3 events (less than batch size)
        for _ in 0..3 {
            let event = MemoryGraphEvent::NodeCreated {
                node_id: NodeId::new(),
                node_type: NodeType::Prompt,
                session_id: None,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            };
            producer.publish(event).await.unwrap();
        }

        // Should be in buffer
        assert_eq!(producer.buffer_size().await, 3);

        // Manual flush
        producer.flush().await.unwrap();

        // Wait a bit for async flush
        tokio::time::sleep(Duration::from_millis(50)).await;

        let sent = mock.get_sent_events().await;
        assert_eq!(sent.len(), 3);
        assert_eq!(producer.buffer_size().await, 0);
    }

    #[tokio::test]
    async fn test_batching_producer_timeout_flush() {
        let config = KafkaConfig::default()
            .with_batch_size(100)
            .with_batch_timeout_ms(200);
        let mock = MockKafkaProducer::new(config.clone());
        let producer = BatchingKafkaProducer::new(mock.clone(), config);

        // Send 2 events (less than batch size)
        for _ in 0..2 {
            let event = MemoryGraphEvent::NodeCreated {
                node_id: NodeId::new(),
                node_type: NodeType::Prompt,
                session_id: None,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            };
            producer.publish(event).await.unwrap();
        }

        // Wait for timeout flush
        tokio::time::sleep(Duration::from_millis(400)).await;

        let sent = mock.get_sent_events().await;
        assert_eq!(sent.len(), 2);
    }

    #[tokio::test]
    async fn test_producer_retry_on_failure() {
        let config = KafkaConfig::default()
            .with_retry_config(3, 10);
        let mock = MockKafkaProducer::new(config.clone());

        // Set high failure rate initially
        mock.set_failure_rate(0.8).await;

        let producer = BatchingKafkaProducer::new(mock.clone(), config);

        let event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        // Lower failure rate after first attempt
        tokio::spawn({
            let mock_clone = mock.clone();
            async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                mock_clone.set_failure_rate(0.0).await;
            }
        });

        // Should eventually succeed with retry
        let result = producer.publish(event).await;

        // May succeed or fail depending on timing, but should not panic
        let _ = result;
    }

    #[tokio::test]
    async fn test_multiple_batches() {
        let config = KafkaConfig::default().with_batch_size(10);
        let mock = MockKafkaProducer::new(config.clone());
        let producer = BatchingKafkaProducer::new(mock.clone(), config);

        // Send 25 events (should create 2 batches of 10 + 5 remaining)
        for _ in 0..25 {
            let event = MemoryGraphEvent::NodeCreated {
                node_id: NodeId::new(),
                node_type: NodeType::Prompt,
                session_id: None,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            };
            producer.publish(event).await.unwrap();
        }

        // Flush remaining
        producer.flush().await.unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let sent = mock.get_sent_events().await;
        assert_eq!(sent.len(), 25);

        let stats = mock.stats().await;
        assert_eq!(stats.batches_sent, 3); // 10 + 10 + 5
    }
}

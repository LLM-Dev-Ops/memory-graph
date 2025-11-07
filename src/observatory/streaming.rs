//! Real-time event streaming infrastructure
//!
//! This module provides streaming capabilities for memory graph events,
//! enabling real-time monitoring and analysis of graph operations.

use super::events::MemoryGraphEvent;
use crate::error::Result;
use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Trait for event streaming
#[async_trait]
pub trait EventStream: Send + Sync {
    /// Publish an event to the stream
    async fn publish(&self, event: MemoryGraphEvent) -> Result<()>;

    /// Publish multiple events in batch
    async fn publish_batch(&self, events: Vec<MemoryGraphEvent>) -> Result<()> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }

    /// Subscribe to the event stream
    fn subscribe(&self) -> Pin<Box<dyn Stream<Item = MemoryGraphEvent> + Send + '_>>;
}

/// In-memory event stream for testing and development
#[derive(Clone)]
pub struct InMemoryEventStream {
    sender: broadcast::Sender<MemoryGraphEvent>,
    /// Buffer of recent events for replay
    buffer: Arc<RwLock<Vec<MemoryGraphEvent>>>,
    /// Maximum buffer size
    buffer_size: usize,
}

impl InMemoryEventStream {
    /// Create a new in-memory event stream
    ///
    /// # Arguments
    ///
    /// * `capacity` - Channel capacity for concurrent subscribers
    /// * `buffer_size` - Maximum number of events to buffer for replay
    ///
    /// # Examples
    ///
    /// ```
    /// use llm_memory_graph::observatory::streaming::InMemoryEventStream;
    ///
    /// let stream = InMemoryEventStream::new(100, 1000);
    /// ```
    pub fn new(capacity: usize, buffer_size: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            buffer: Arc::new(RwLock::new(Vec::new())),
            buffer_size,
        }
    }

    /// Get all buffered events
    pub async fn get_buffered_events(&self) -> Vec<MemoryGraphEvent> {
        self.buffer.read().await.clone()
    }

    /// Clear the event buffer
    pub async fn clear_buffer(&self) {
        self.buffer.write().await.clear();
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

#[async_trait]
impl EventStream for InMemoryEventStream {
    async fn publish(&self, event: MemoryGraphEvent) -> Result<()> {
        // Add to buffer
        let mut buffer = self.buffer.write().await;
        buffer.push(event.clone());

        // Trim buffer if needed
        if buffer.len() > self.buffer_size {
            let excess = buffer.len() - self.buffer_size;
            buffer.drain(0..excess);
        }
        drop(buffer);

        // Send to subscribers (ignore if no subscribers)
        let _ = self.sender.send(event);

        Ok(())
    }

    async fn publish_batch(&self, events: Vec<MemoryGraphEvent>) -> Result<()> {
        // Add all events to buffer
        let mut buffer = self.buffer.write().await;
        buffer.extend(events.iter().cloned());

        // Trim buffer if needed
        if buffer.len() > self.buffer_size {
            let excess = buffer.len() - self.buffer_size;
            buffer.drain(0..excess);
        }
        drop(buffer);

        // Send each event to subscribers
        for event in events {
            let _ = self.sender.send(event);
        }

        Ok(())
    }

    fn subscribe(&self) -> Pin<Box<dyn Stream<Item = MemoryGraphEvent> + Send + '_>> {
        let receiver = self.sender.subscribe();
        Box::pin(async_stream::stream! {
            let mut rx = receiver;
            while let Ok(event) = rx.recv().await {
                yield event;
            }
        })
    }
}

/// Event stream combinator that broadcasts to multiple streams
pub struct MultiEventStream {
    streams: Vec<Arc<dyn EventStream>>,
}

impl MultiEventStream {
    /// Create a new multi-event stream
    pub fn new(streams: Vec<Arc<dyn EventStream>>) -> Self {
        Self { streams }
    }

    /// Add a stream to the combinator
    pub fn add_stream(&mut self, stream: Arc<dyn EventStream>) {
        self.streams.push(stream);
    }
}

#[async_trait]
impl EventStream for MultiEventStream {
    async fn publish(&self, event: MemoryGraphEvent) -> Result<()> {
        let futures: Vec<_> = self
            .streams
            .iter()
            .map(|stream| stream.publish(event.clone()))
            .collect();

        futures::future::try_join_all(futures).await?;
        Ok(())
    }

    async fn publish_batch(&self, events: Vec<MemoryGraphEvent>) -> Result<()> {
        let futures: Vec<_> = self
            .streams
            .iter()
            .map(|stream| stream.publish_batch(events.clone()))
            .collect();

        futures::future::try_join_all(futures).await?;
        Ok(())
    }

    fn subscribe(&self) -> Pin<Box<dyn Stream<Item = MemoryGraphEvent> + Send + '_>> {
        // Subscribe to the first stream only
        if let Some(first) = self.streams.first() {
            first.subscribe()
        } else {
            Box::pin(futures::stream::empty())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NodeId, NodeType, SessionId};
    use chrono::Utc;
    use futures::StreamExt;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_in_memory_stream_creation() {
        let stream = InMemoryEventStream::new(100, 1000);
        assert_eq!(stream.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_publish_and_subscribe() {
        let stream = InMemoryEventStream::new(100, 1000);
        let mut subscription = stream.subscribe();

        let event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: Some(SessionId::new()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        // Publish event
        stream.publish(event.clone()).await.unwrap();

        // Receive event
        let received = subscription.next().await.unwrap();
        assert_eq!(received.event_type(), event.event_type());
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let stream = InMemoryEventStream::new(100, 1000);
        let mut sub1 = stream.subscribe();
        let mut sub2 = stream.subscribe();
        let mut sub3 = stream.subscribe();

        assert_eq!(stream.subscriber_count(), 3);

        let event = MemoryGraphEvent::QueryExecuted {
            query_type: "test".to_string(),
            results_count: 42,
            duration_ms: 100,
            timestamp: Utc::now(),
        };

        stream.publish(event.clone()).await.unwrap();

        // All subscribers should receive the event
        let r1 = sub1.next().await.unwrap();
        let r2 = sub2.next().await.unwrap();
        let r3 = sub3.next().await.unwrap();

        assert_eq!(r1.event_type(), "query_executed");
        assert_eq!(r2.event_type(), "query_executed");
        assert_eq!(r3.event_type(), "query_executed");
    }

    #[tokio::test]
    async fn test_event_buffer() {
        let stream = InMemoryEventStream::new(100, 10);

        // Publish 5 events
        for i in 0..5 {
            let event = MemoryGraphEvent::NodeCreated {
                node_id: NodeId::new(),
                node_type: NodeType::Prompt,
                session_id: None,
                timestamp: Utc::now(),
                metadata: HashMap::from([("index".to_string(), i.to_string())]),
            };
            stream.publish(event).await.unwrap();
        }

        let buffered = stream.get_buffered_events().await;
        assert_eq!(buffered.len(), 5);
    }

    #[tokio::test]
    async fn test_buffer_trimming() {
        let stream = InMemoryEventStream::new(100, 5);

        // Publish 10 events (buffer size is 5)
        for i in 0..10 {
            let event = MemoryGraphEvent::QueryExecuted {
                query_type: format!("query_{}", i),
                results_count: i,
                duration_ms: 100,
                timestamp: Utc::now(),
            };
            stream.publish(event).await.unwrap();
        }

        let buffered = stream.get_buffered_events().await;
        assert_eq!(buffered.len(), 5);

        // Should have the last 5 events (5-9)
        if let MemoryGraphEvent::QueryExecuted { results_count, .. } = &buffered[0] {
            assert_eq!(*results_count, 5);
        } else {
            panic!("Wrong event type");
        }
    }

    #[tokio::test]
    async fn test_clear_buffer() {
        let stream = InMemoryEventStream::new(100, 100);

        for _ in 0..5 {
            let event = MemoryGraphEvent::NodeCreated {
                node_id: NodeId::new(),
                node_type: NodeType::Prompt,
                session_id: None,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            };
            stream.publish(event).await.unwrap();
        }

        assert_eq!(stream.get_buffered_events().await.len(), 5);

        stream.clear_buffer().await;
        assert_eq!(stream.get_buffered_events().await.len(), 0);
    }

    #[tokio::test]
    async fn test_publish_batch() {
        let stream = InMemoryEventStream::new(100, 100);
        let mut subscription = stream.subscribe();

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
        ];

        stream.publish_batch(events.clone()).await.unwrap();

        // Should receive both events
        let e1 = subscription.next().await.unwrap();
        let e2 = subscription.next().await.unwrap();

        assert_eq!(e1.event_type(), "node_created");
        assert_eq!(e2.event_type(), "node_created");

        // Buffer should have both events
        let buffered = stream.get_buffered_events().await;
        assert_eq!(buffered.len(), 2);
    }

    #[tokio::test]
    async fn test_multi_event_stream() {
        let stream1 = Arc::new(InMemoryEventStream::new(100, 100));
        let stream2 = Arc::new(InMemoryEventStream::new(100, 100));

        let multi = MultiEventStream::new(vec![stream1.clone(), stream2.clone()]);

        let event = MemoryGraphEvent::QueryExecuted {
            query_type: "test".to_string(),
            results_count: 10,
            duration_ms: 50,
            timestamp: Utc::now(),
        };

        multi.publish(event).await.unwrap();

        // Both streams should have the event
        let buf1 = stream1.get_buffered_events().await;
        let buf2 = stream2.get_buffered_events().await;

        assert_eq!(buf1.len(), 1);
        assert_eq!(buf2.len(), 1);
    }

    #[tokio::test]
    async fn test_multi_stream_batch() {
        let stream1 = Arc::new(InMemoryEventStream::new(100, 100));
        let stream2 = Arc::new(InMemoryEventStream::new(100, 100));

        let multi = MultiEventStream::new(vec![stream1.clone(), stream2.clone()]);

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
        ];

        multi.publish_batch(events).await.unwrap();

        // Both streams should have both events
        assert_eq!(stream1.get_buffered_events().await.len(), 2);
        assert_eq!(stream2.get_buffered_events().await.len(), 2);
    }

    #[tokio::test]
    async fn test_concurrent_publishing() {
        let stream = Arc::new(InMemoryEventStream::new(1000, 1000));

        let mut handles = vec![];

        // Spawn 10 concurrent publishers
        for i in 0..10 {
            let stream_clone = Arc::clone(&stream);
            let handle = tokio::spawn(async move {
                for j in 0..10 {
                    let event = MemoryGraphEvent::QueryExecuted {
                        query_type: format!("query_{}_{}", i, j),
                        results_count: j,
                        duration_ms: 100,
                        timestamp: Utc::now(),
                    };
                    stream_clone.publish(event).await.unwrap();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let buffered = stream.get_buffered_events().await;
        assert_eq!(buffered.len(), 100); // 10 publishers * 10 events each
    }
}

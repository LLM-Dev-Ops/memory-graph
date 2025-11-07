//! Event publisher traits and implementations

use super::events::MemoryGraphEvent;
use crate::error::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for publishing Observatory events
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish a single event
    async fn publish(&self, event: MemoryGraphEvent) -> Result<()>;

    /// Publish a batch of events
    async fn publish_batch(&self, events: Vec<MemoryGraphEvent>) -> Result<()> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }

    /// Flush any pending events
    async fn flush(&self) -> Result<()> {
        Ok(())
    }
}

/// In-memory event publisher for testing and development
#[derive(Clone)]
pub struct InMemoryPublisher {
    events: Arc<RwLock<Vec<MemoryGraphEvent>>>,
}

impl InMemoryPublisher {
    /// Create a new in-memory publisher
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get all published events
    pub async fn get_events(&self) -> Vec<MemoryGraphEvent> {
        self.events.read().await.clone()
    }

    /// Get the number of published events
    pub async fn count(&self) -> usize {
        self.events.read().await.len()
    }

    /// Clear all events
    pub async fn clear(&self) {
        self.events.write().await.clear();
    }

    /// Get events of a specific type
    pub async fn get_events_by_type(&self, event_type: &str) -> Vec<MemoryGraphEvent> {
        self.events
            .read()
            .await
            .iter()
            .filter(|e| e.event_type() == event_type)
            .cloned()
            .collect()
    }
}

impl Default for InMemoryPublisher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventPublisher for InMemoryPublisher {
    async fn publish(&self, event: MemoryGraphEvent) -> Result<()> {
        self.events.write().await.push(event);
        Ok(())
    }

    async fn publish_batch(&self, events: Vec<MemoryGraphEvent>) -> Result<()> {
        self.events.write().await.extend(events);
        Ok(())
    }
}

/// No-op publisher that discards all events
#[derive(Clone, Copy)]
pub struct NoOpPublisher;

#[async_trait]
impl EventPublisher for NoOpPublisher {
    async fn publish(&self, _event: MemoryGraphEvent) -> Result<()> {
        Ok(())
    }

    async fn publish_batch(&self, _events: Vec<MemoryGraphEvent>) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NodeId, NodeType, SessionId};
    use chrono::Utc;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_in_memory_publisher() {
        let publisher = InMemoryPublisher::new();

        let event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: Some(SessionId::new()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        publisher.publish(event.clone()).await.unwrap();

        let events = publisher.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), event.event_type());

        let count = publisher.count().await;
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_in_memory_publisher_batch() {
        let publisher = InMemoryPublisher::new();

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

        publisher.publish_batch(events).await.unwrap();

        assert_eq!(publisher.count().await, 2);
    }

    #[tokio::test]
    async fn test_in_memory_publisher_clear() {
        let publisher = InMemoryPublisher::new();

        let event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        publisher.publish(event).await.unwrap();
        assert_eq!(publisher.count().await, 1);

        publisher.clear().await;
        assert_eq!(publisher.count().await, 0);
    }

    #[tokio::test]
    async fn test_in_memory_publisher_filter_by_type() {
        let publisher = InMemoryPublisher::new();

        let node_event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let query_event = MemoryGraphEvent::QueryExecuted {
            query_type: "test".to_string(),
            results_count: 10,
            duration_ms: 50,
            timestamp: Utc::now(),
        };

        publisher.publish(node_event).await.unwrap();
        publisher.publish(query_event).await.unwrap();

        let node_events = publisher.get_events_by_type("node_created").await;
        assert_eq!(node_events.len(), 1);

        let query_events = publisher.get_events_by_type("query_executed").await;
        assert_eq!(query_events.len(), 1);
    }

    #[tokio::test]
    async fn test_noop_publisher() {
        let publisher = NoOpPublisher;

        let event = MemoryGraphEvent::NodeCreated {
            node_id: NodeId::new(),
            node_type: NodeType::Prompt,
            session_id: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        // Should not panic or error
        publisher.publish(event).await.unwrap();
        publisher.publish_batch(vec![]).await.unwrap();
        publisher.flush().await.unwrap();
    }
}

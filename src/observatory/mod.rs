//! Observatory integration for real-time event streaming and metrics
//!
//! This module provides event streaming capabilities for monitoring and analyzing
//! memory graph operations in real-time. Events can be published to external systems
//! like Kafka, and metrics can be collected for performance monitoring.
//!
//! # Features
//!
//! - **Event Streaming**: Publish events for all graph operations
//! - **Metrics Collection**: Track performance and usage metrics
//! - **Pluggable Publishers**: Implement custom event publishers
//! - **In-Memory Testing**: Built-in publisher for development and testing
//!
//! # Examples
//!
//! ```no_run
//! use llm_memory_graph::observatory::{ObservatoryConfig, InMemoryPublisher};
//! use llm_memory_graph::engine::AsyncMemoryGraph;
//! use llm_memory_graph::Config;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create an in-memory event publisher
//!     let publisher = Arc::new(InMemoryPublisher::new());
//!
//!     // Configure observatory
//!     let obs_config = ObservatoryConfig::new()
//!         .enabled()
//!         .with_batch_size(50);
//!
//!     // Create graph with observatory
//!     let config = Config::default();
//!     let graph = AsyncMemoryGraph::with_observatory(
//!         config,
//!         Some(publisher.clone()),
//!         obs_config
//!     ).await?;
//!
//!     // Operations will now emit events
//!     let session = graph.create_session().await?;
//!
//!     // Check published events
//!     let events = publisher.get_events().await;
//!     println!("Published {} events", events.len());
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod emitter;
pub mod events;
pub mod kafka;
pub mod metrics;
pub mod prometheus;
pub mod publisher;
pub mod streaming;

pub use config::ObservatoryConfig;
pub use emitter::{AsyncEventEmitter, EmissionStatsSnapshot};
pub use events::MemoryGraphEvent;
pub use kafka::{
    BatchingKafkaProducer, KafkaConfig, KafkaProducer, MockKafkaProducer, ProducerStats,
};
pub use metrics::{MemoryGraphMetrics, MetricsSnapshot};
pub use prometheus::{
    GrpcMetricsSnapshot, MetricsCounterSnapshot, MetricsGaugeSnapshot, PrometheusMetrics,
    VaultMetricsSnapshot,
};
pub use publisher::{EventPublisher, InMemoryPublisher, NoOpPublisher};
pub use streaming::{EventStream, InMemoryEventStream, MultiEventStream};

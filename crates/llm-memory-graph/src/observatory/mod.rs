//! Observatory integration for bidirectional telemetry and event streaming
//!
//! This module provides comprehensive Observatory integration with both publishing
//! and consumption capabilities for real-time event streaming, metrics collection,
//! and distributed tracing.
//!
//! # Features
//!
//! ## Publishing (Outbound)
//! - **Event Streaming**: Publish events for all graph operations
//! - **Metrics Collection**: Track performance and usage metrics
//! - **Pluggable Publishers**: Implement custom event publishers
//! - **In-Memory Testing**: Built-in publisher for development and testing
//!
//! ## Consumption (Inbound)
//! - **Telemetry Ingestion**: Consume spans, metrics, and logs from external systems
//! - **Lineage Building**: Construct lineage chains from distributed traces
//! - **Temporal Graphs**: Build temporal correlation graphs from metrics
//! - **Span Mapping**: Map Observatory events to memory graph entities
//!
//! # Examples
//!
//! ## Publishing Events
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
//!
//! ## Consuming Telemetry
//!
//! ```no_run
//! use llm_memory_graph::observatory::{IngestionPipeline, TelemetryData, SpanStatus};
//! use std::collections::HashMap;
//! use chrono::Utc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create ingestion pipeline
//!     let pipeline = IngestionPipeline::new();
//!
//!     // Ingest a trace span
//!     let span = TelemetryData::Span {
//!         span_id: "span-123".to_string(),
//!         trace_id: "trace-456".to_string(),
//!         parent_span_id: None,
//!         operation_name: "llm.generate".to_string(),
//!         start_time: Utc::now(),
//!         end_time: Utc::now(),
//!         attributes: HashMap::new(),
//!         status: SpanStatus::Ok,
//!     };
//!
//!     pipeline.ingest(span).await?;
//!
//!     // Get lineage chain
//!     if let Some(chain) = pipeline.get_lineage_chain("trace-456").await {
//!         println!("Built lineage chain with {} nodes", chain.nodes.len());
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod consumer;
pub mod emitter;
pub mod events;
pub mod ingest;
pub mod kafka;
pub mod lineage;
pub mod mapping;
pub mod metrics;
pub mod prometheus;
pub mod publisher;
pub mod streaming;
pub mod temporal;

// Configuration
pub use config::ObservatoryConfig;

// Publishing (Outbound)
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

// Consumption (Inbound)
pub use consumer::{
    ConsumptionStats, InMemoryConsumer, LogLevel, MetricType, NoOpConsumer, SpanStatus,
    TelemetryConsumer, TelemetryData, TraceContext,
};
pub use ingest::{IngestionConfig, IngestionPipeline, IngestionStats, ProcessingResult};
pub use lineage::{LineageBuilder, LineageChain, LineageEdge, LineageEdgeType, LineageNode};
pub use mapping::{MappedEdge, MappedEntity, MappingConfig, MappingResult, SpanMapper};
pub use temporal::{
    Correlation, DataPoint, TemporalGraph, TemporalGraphBuilder, TemporalNode, TimeSeries,
};

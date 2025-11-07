//! gRPC service implementation for LLM-Memory-Graph
//!
//! This module provides a production-grade gRPC service for distributed access
//! to the memory graph. It includes:
//!
//! - Full CRUD operations for sessions, nodes, and edges
//! - Streaming query support for large result sets
//! - Real-time event subscriptions
//! - Health checks and metrics endpoints
//! - Plugin hook integration points
//! - Comprehensive error handling and observability
//!
//! # Architecture
//!
//! The gRPC service is designed as a standalone server that wraps the
//! `AsyncMemoryGraph` core engine. It provides:
//!
//! - **Service Layer**: Main gRPC service implementation with Tonic
//! - **Handlers**: Request/response handling with validation
//! - **Streaming**: Bi-directional streaming support
//! - **Converters**: Type conversion between protobuf and internal types
//! - **Metrics**: Prometheus metrics integration
//!
//! # Examples
//!
//! ## Starting the gRPC Server
//!
//! ```no_run
//! use llm_memory_graph::grpc::MemoryGraphServiceImpl;
//! use llm_memory_graph::engine::AsyncMemoryGraph;
//! use llm_memory_graph::Config;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::default();
//!     let graph = Arc::new(AsyncMemoryGraph::open(config).await?);
//!
//!     // Service will be created with plugin manager and metrics
//!     // See src/bin/server.rs for full example
//!
//!     Ok(())
//! }
//! ```

// Include generated protobuf code
#[allow(clippy::all)]
#[allow(missing_docs)]
#[path = "llm.memory.graph.v1.rs"]
pub mod proto;

pub mod converters;
pub mod handlers;
pub mod service;
pub mod streaming;

// Re-export main types
pub use service::{MemoryGraphServiceImpl, ServiceConfig};

/// Default gRPC server port
pub const DEFAULT_GRPC_PORT: u16 = 50051;

/// Default metrics server port
pub const DEFAULT_METRICS_PORT: u16 = 9090;

/// Maximum concurrent gRPC streams per connection
pub const MAX_CONCURRENT_STREAMS: u32 = 100;

/// Request timeout in milliseconds
pub const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 30_000;

/// Maximum message size (100MB)
pub const MAX_MESSAGE_SIZE: usize = 100 * 1024 * 1024;

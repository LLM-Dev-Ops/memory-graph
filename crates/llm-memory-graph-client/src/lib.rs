//! Rust client for the LLM Memory Graph gRPC service
//!
//! This crate provides a high-level, ergonomic async client for interacting with
//! the LLM Memory Graph service over gRPC.
//!
//! # Features
//!
//! - **Async/await**: Full async support with tokio
//! - **Type-safe**: Strongly typed API using llm-memory-graph-types
//! - **Streaming**: Support for streaming queries and events
//! - **Connection pooling**: Efficient connection management
//! - **Error handling**: Comprehensive error types
//!
//! # Example
//!
//! ```no_run
//! use llm_memory_graph_client::MemoryGraphClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = MemoryGraphClient::connect("http://localhost:50051").await?;
//!
//!     // Create a session
//!     let session_id = client.create_session(Default::default()).await?;
//!
//!     // Add a prompt
//!     let prompt = client.add_prompt(
//!         session_id,
//!         "What is the capital of France?".to_string(),
//!         None
//!     ).await?;
//!
//!     Ok(())
//! }
//! ```

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod client;
pub mod error;

// Re-export main types
pub use client::MemoryGraphClient;
pub use error::{ClientError, Result};

// Re-export types from llm-memory-graph-types
pub use llm_memory_graph_types::*;

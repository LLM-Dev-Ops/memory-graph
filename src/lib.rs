//! LLM-Memory-Graph: Graph-based context-tracking and prompt-lineage database
//!
//! This crate provides a high-performance, embeddable graph database specifically designed
//! for tracking LLM conversation context, prompt lineage, and multi-agent coordination.
//!
//! # Features
//!
//! - **Context Persistence**: Maintain conversation history across sessions
//! - **Prompt Lineage**: Track prompt evolution and template inheritance
//! - **Graph-Native**: Efficient relationship queries using graph algorithms
//! - **Embedded Storage**: Low-latency, file-based storage using Sled
//! - **Type-Safe**: Strongly typed nodes and edges with schema validation
//!
//! # Quick Start
//!
//! ```no_run
//! use llm_memory_graph::{MemoryGraph, Config};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = Config::default();
//! let graph = MemoryGraph::open(config)?;
//!
//! let session = graph.create_session()?;
//! let prompt_id = graph.add_prompt(
//!     session.id,
//!     "Explain quantum computing".to_string(),
//!     None
//! )?;
//!
//! # Ok(())
//! # }
//! ```

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod engine;
pub mod error;
pub mod query;
pub mod storage;
pub mod types;

// Re-export main types
pub use engine::MemoryGraph;
pub use error::{Error, Result};
pub use types::{
    AgentConfig, AgentId, AgentMetrics, AgentNode, AgentStatus, Config, ConversationSession,
    EdgeType, NodeId, NodeType, PromptMetadata, PromptNode, ResponseMetadata, ResponseNode,
    SessionId, ToolInvocation, TokenUsage,
};

/// Current version of the LLM-Memory-Graph library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

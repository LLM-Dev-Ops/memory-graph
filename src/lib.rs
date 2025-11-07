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
// Allow pedantic lints that are overly strict for this codebase
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::unused_self)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::explicit_iter_loop)]

pub mod engine;
pub mod error;
// pub mod grpc; // TODO: Complete gRPC implementation
// pub mod integrations; // TODO: Fix type mismatches in retry logic
pub mod migration;
pub mod observatory;
pub mod plugin;
pub mod query;
pub mod storage;
pub mod types;

// Re-export main types
pub use engine::{AsyncMemoryGraph, MemoryGraph};
pub use error::{Error, Result};
pub use types::{
    AgentConfig, AgentId, AgentMetrics, AgentNode, AgentStatus, Config, ContextType,
    ConversationSession, EdgeType, InheritsProperties, InstantiatesProperties, InvokesProperties,
    NodeId, NodeType, Priority, PromptMetadata, PromptNode, PromptTemplate, ReferencesProperties,
    ResponseMetadata, ResponseNode, SessionId, TemplateId, TokenUsage, ToolInvocation,
    TransfersToProperties, VariableSpec, Version, VersionLevel,
};

/// Current version of the LLM-Memory-Graph library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

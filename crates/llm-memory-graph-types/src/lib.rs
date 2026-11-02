//! Core types for the LLM Memory Graph system
//!
//! This crate provides the foundational data structures used throughout
//! the LLM Memory Graph ecosystem, including node types, edge types,
//! identifiers, and error handling.
//!
//! # Features
//!
//! - **Zero heavy dependencies**: Only serde, uuid, chrono, and thiserror
//! - **Type safety**: Strong typing for all graph entities
//! - **Serialization**: Full serde support for all types
//! - **Documentation**: Comprehensive docs for all public APIs
//!
//! # Example
//!
//! ```rust
//! use llm_memory_graph_types::{PromptNode, SessionId};
//!
//! let session_id = SessionId::new();
//! let prompt = PromptNode::new(
//!     session_id,
//!     "What is the capital of France?".to_string(),
//! );
//! assert_eq!(prompt.session_id, session_id);
//! ```

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]
// These APIs intentionally take owned property values to preserve their public
// builder shape, and metrics/backoff calculations explicitly cross numeric
// domains with bounded values.
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]

pub mod config;
pub mod edges;
pub mod error;
pub mod ids;
pub mod nodes;
pub mod utils;

// Re-export main types
pub use config::Config;
pub use edges::{
    ContextType, Edge, EdgeType, InheritsProperties, InstantiatesProperties, InvokesProperties,
    Priority, ReferencesProperties, TransfersToProperties,
};
pub use error::{Error, Result};
pub use ids::{AgentId, EdgeId, NodeId, SessionId, TemplateId};
pub use nodes::{
    AgentConfig, AgentMetrics, AgentNode, AgentStatus, ConversationSession, Node, NodeType,
    PromptMetadata, PromptNode, PromptTemplate, ResponseMetadata, ResponseNode, TokenUsage,
    ToolInvocation, VariableSpec, Version, VersionLevel,
};
pub use utils::*;

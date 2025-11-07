//! Core data types for LLM-Memory-Graph

mod config;
mod edges;
mod ids;
mod nodes;

pub use config::Config;
pub use edges::{
    ContextType, Edge, EdgeType, InheritsProperties, InstantiatesProperties, InvokesProperties,
    Priority, ReferencesProperties, TransfersToProperties,
};
pub use ids::{AgentId, EdgeId, NodeId, SessionId, TemplateId};
pub use nodes::{
    AgentConfig, AgentMetrics, AgentNode, AgentStatus, ConversationSession, Node, NodeType,
    PromptMetadata, PromptNode, PromptTemplate, ResponseMetadata, ResponseNode, TokenUsage,
    ToolInvocation, VariableSpec, Version, VersionLevel,
};

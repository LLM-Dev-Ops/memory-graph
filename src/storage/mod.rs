//! Storage backend for persisting graph data

mod sled_backend;
mod serialization;

pub use sled_backend::SledBackend;
pub use serialization::{Serializer, SerializationFormat};

use crate::error::Result;
use crate::types::{Edge, EdgeId, Node, NodeId, SessionId};

/// Trait defining storage backend operations
pub trait StorageBackend: Send + Sync {
    /// Store a node in the backend
    fn store_node(&self, node: &Node) -> Result<()>;

    /// Retrieve a node by ID
    fn get_node(&self, id: &NodeId) -> Result<Option<Node>>;

    /// Delete a node
    fn delete_node(&self, id: &NodeId) -> Result<()>;

    /// Store an edge
    fn store_edge(&self, edge: &Edge) -> Result<()>;

    /// Retrieve an edge by ID
    fn get_edge(&self, id: &EdgeId) -> Result<Option<Edge>>;

    /// Delete an edge
    fn delete_edge(&self, id: &EdgeId) -> Result<()>;

    /// Get all nodes in a session
    fn get_session_nodes(&self, session_id: &SessionId) -> Result<Vec<Node>>;

    /// Get all edges from a node
    fn get_outgoing_edges(&self, node_id: &NodeId) -> Result<Vec<Edge>>;

    /// Get all edges to a node
    fn get_incoming_edges(&self, node_id: &NodeId) -> Result<Vec<Edge>>;

    /// Flush any pending writes
    fn flush(&self) -> Result<()>;

    /// Get storage statistics
    fn stats(&self) -> Result<StorageStats>;
}

/// Statistics about storage usage
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// Total number of nodes
    pub node_count: u64,
    /// Total number of edges
    pub edge_count: u64,
    /// Total storage size in bytes
    pub storage_bytes: u64,
    /// Number of sessions
    pub session_count: u64,
}

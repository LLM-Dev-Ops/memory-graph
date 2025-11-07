//! Storage backend for persisting graph data

mod sled_backend;
mod async_sled_backend;
mod serialization;
mod cache;
mod pooled_backend;

pub use sled_backend::SledBackend;
pub use async_sled_backend::AsyncSledBackend;
pub use serialization::{Serializer, SerializationFormat};
pub use cache::{StorageCache, CacheStats};
pub use pooled_backend::{PooledAsyncBackend, PoolConfig, PoolMetrics, PoolMetricsSnapshot};

use crate::error::Result;
use crate::types::{Edge, EdgeId, Node, NodeId, SessionId};
use async_trait::async_trait;

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

/// Async trait defining storage backend operations
///
/// This trait provides async versions of all storage operations for use with Tokio runtime.
/// It enables high-performance concurrent operations and non-blocking I/O.
#[async_trait]
pub trait AsyncStorageBackend: Send + Sync {
    /// Store a node in the backend asynchronously
    async fn store_node(&self, node: &Node) -> Result<()>;

    /// Retrieve a node by ID asynchronously
    async fn get_node(&self, id: &NodeId) -> Result<Option<Node>>;

    /// Delete a node asynchronously
    async fn delete_node(&self, id: &NodeId) -> Result<()>;

    /// Store an edge asynchronously
    async fn store_edge(&self, edge: &Edge) -> Result<()>;

    /// Retrieve an edge by ID asynchronously
    async fn get_edge(&self, id: &EdgeId) -> Result<Option<Edge>>;

    /// Delete an edge asynchronously
    async fn delete_edge(&self, id: &EdgeId) -> Result<()>;

    /// Get all nodes in a session asynchronously
    async fn get_session_nodes(&self, session_id: &SessionId) -> Result<Vec<Node>>;

    /// Get all edges from a node asynchronously
    async fn get_outgoing_edges(&self, node_id: &NodeId) -> Result<Vec<Edge>>;

    /// Get all edges to a node asynchronously
    async fn get_incoming_edges(&self, node_id: &NodeId) -> Result<Vec<Edge>>;

    /// Flush any pending writes asynchronously
    async fn flush(&self) -> Result<()>;

    /// Get storage statistics asynchronously
    async fn stats(&self) -> Result<StorageStats>;

    /// Batch store multiple nodes asynchronously for improved performance
    async fn store_nodes_batch(&self, nodes: &[Node]) -> Result<Vec<NodeId>> {
        let mut ids = Vec::with_capacity(nodes.len());
        for node in nodes {
            self.store_node(node).await?;
            ids.push(node.id());
        }
        Ok(ids)
    }

    /// Batch store multiple edges asynchronously for improved performance
    async fn store_edges_batch(&self, edges: &[Edge]) -> Result<Vec<EdgeId>> {
        let mut ids = Vec::with_capacity(edges.len());
        for edge in edges {
            self.store_edge(edge).await?;
            ids.push(edge.id);
        }
        Ok(ids)
    }

    /// Stream nodes from a session for memory-efficient iteration over large result sets
    ///
    /// This method returns a stream that yields nodes one at a time, avoiding the need
    /// to load all nodes into memory at once. This is particularly useful for sessions
    /// with thousands of nodes.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::storage::AsyncSledBackend;
    /// use llm_memory_graph::storage::AsyncStorageBackend;
    /// use llm_memory_graph::types::SessionId;
    /// use futures::stream::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let backend = AsyncSledBackend::open("./data/graph.db").await?;
    /// let session_id = SessionId::new();
    ///
    /// let mut stream = backend.get_session_nodes_stream(&session_id);
    /// let mut count = 0;
    /// while let Some(node) = stream.next().await {
    ///     let node = node?;
    ///     // Process node without loading all into memory
    ///     count += 1;
    /// }
    /// println!("Processed {} nodes", count);
    /// # Ok(())
    /// # }
    /// ```
    fn get_session_nodes_stream(
        &self,
        session_id: &SessionId,
    ) -> std::pin::Pin<Box<dyn futures::stream::Stream<Item = Result<Node>> + Send + '_>> {
        // Default implementation: load all and convert to stream
        // Backends should override for true streaming from storage
        let session_id = *session_id;
        Box::pin(async_stream::stream! {
            match self.get_session_nodes(&session_id).await {
                Ok(nodes) => {
                    for node in nodes {
                        yield Ok(node);
                    }
                }
                Err(e) => yield Err(e),
            }
        })
    }

    /// Count nodes in a session without loading them into memory
    ///
    /// This is more efficient than `get_session_nodes().await?.len()` for large sessions
    /// as it only counts without deserializing node data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::storage::AsyncSledBackend;
    /// use llm_memory_graph::storage::AsyncStorageBackend;
    /// use llm_memory_graph::types::SessionId;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let backend = AsyncSledBackend::open("./data/graph.db").await?;
    /// let session_id = SessionId::new();
    ///
    /// let count = backend.count_session_nodes(&session_id).await?;
    /// println!("Session has {} nodes", count);
    /// # Ok(())
    /// # }
    /// ```
    async fn count_session_nodes(&self, session_id: &SessionId) -> Result<usize> {
        // Default implementation: load and count
        // Backends should override for O(1) counting if possible
        let nodes = self.get_session_nodes(session_id).await?;
        Ok(nodes.len())
    }
}

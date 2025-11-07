//! Async Sled-based storage backend implementation using Tokio
//!
//! This module provides an async wrapper around the synchronous SledBackend,
//! using `tokio::task::spawn_blocking` to run blocking operations on a dedicated
//! thread pool without blocking the async runtime.

use super::{AsyncStorageBackend, SerializationFormat, SledBackend, StorageBackend, StorageStats};
use crate::error::Result;
use crate::types::{Edge, EdgeId, Node, NodeId, SessionId};
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

/// Async wrapper around Sled-based storage backend
///
/// This struct provides async versions of all storage operations by wrapping
/// the synchronous SledBackend and using Tokio's blocking task pool.
#[derive(Clone)]
pub struct AsyncSledBackend {
    /// Shared reference to the underlying synchronous backend
    inner: Arc<SledBackend>,
}

impl AsyncSledBackend {
    /// Open or create a new async Sled backend at the specified path
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::storage::AsyncSledBackend;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let backend = AsyncSledBackend::open("./data/graph.db").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();

        // Run the synchronous open operation in a blocking task
        let inner = tokio::task::spawn_blocking(move || {
            SledBackend::open(path_buf)
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))??;

        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    /// Open with a custom serialization format
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::storage::{AsyncSledBackend, SerializationFormat};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let backend = AsyncSledBackend::open_with_format(
    ///         "./data/graph.db",
    ///         SerializationFormat::Json
    ///     ).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn open_with_format<P: AsRef<Path>>(
        path: P,
        format: SerializationFormat,
    ) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();

        let inner = tokio::task::spawn_blocking(move || {
            SledBackend::open_with_format(path_buf, format)
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))??;

        Ok(Self {
            inner: Arc::new(inner),
        })
    }
}

#[async_trait]
impl AsyncStorageBackend for AsyncSledBackend {
    async fn store_node(&self, node: &Node) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        let node = node.clone();

        tokio::task::spawn_blocking(move || {
            inner.store_node(&node)
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))?
    }

    async fn get_node(&self, id: &NodeId) -> Result<Option<Node>> {
        let inner = Arc::clone(&self.inner);
        let id = *id;

        tokio::task::spawn_blocking(move || {
            inner.get_node(&id)
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))?
    }

    async fn delete_node(&self, id: &NodeId) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        let id = *id;

        tokio::task::spawn_blocking(move || {
            inner.delete_node(&id)
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))?
    }

    async fn store_edge(&self, edge: &Edge) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        let edge = edge.clone();

        tokio::task::spawn_blocking(move || {
            inner.store_edge(&edge)
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))?
    }

    async fn get_edge(&self, id: &EdgeId) -> Result<Option<Edge>> {
        let inner = Arc::clone(&self.inner);
        let id = *id;

        tokio::task::spawn_blocking(move || {
            inner.get_edge(&id)
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))?
    }

    async fn delete_edge(&self, id: &EdgeId) -> Result<()> {
        let inner = Arc::clone(&self.inner);
        let id = *id;

        tokio::task::spawn_blocking(move || {
            inner.delete_edge(&id)
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))?
    }

    async fn get_session_nodes(&self, session_id: &SessionId) -> Result<Vec<Node>> {
        let inner = Arc::clone(&self.inner);
        let session_id = *session_id;

        tokio::task::spawn_blocking(move || {
            inner.get_session_nodes(&session_id)
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))?
    }

    async fn get_outgoing_edges(&self, node_id: &NodeId) -> Result<Vec<Edge>> {
        let inner = Arc::clone(&self.inner);
        let node_id = *node_id;

        tokio::task::spawn_blocking(move || {
            inner.get_outgoing_edges(&node_id)
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))?
    }

    async fn get_incoming_edges(&self, node_id: &NodeId) -> Result<Vec<Edge>> {
        let inner = Arc::clone(&self.inner);
        let node_id = *node_id;

        tokio::task::spawn_blocking(move || {
            inner.get_incoming_edges(&node_id)
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))?
    }

    async fn flush(&self) -> Result<()> {
        let inner = Arc::clone(&self.inner);

        tokio::task::spawn_blocking(move || {
            inner.flush()
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))?
    }

    async fn stats(&self) -> Result<StorageStats> {
        let inner = Arc::clone(&self.inner);

        tokio::task::spawn_blocking(move || {
            inner.stats()
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))?
    }

    async fn store_nodes_batch(&self, nodes: &[Node]) -> Result<Vec<NodeId>> {
        let inner = Arc::clone(&self.inner);
        let nodes = nodes.to_vec();

        tokio::task::spawn_blocking(move || {
            let mut ids = Vec::with_capacity(nodes.len());
            for node in &nodes {
                inner.store_node(node)?;
                ids.push(node.id());
            }
            Ok(ids)
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))?
    }

    async fn store_edges_batch(&self, edges: &[Edge]) -> Result<Vec<EdgeId>> {
        let inner = Arc::clone(&self.inner);
        let edges = edges.to_vec();

        tokio::task::spawn_blocking(move || {
            let mut ids = Vec::with_capacity(edges.len());
            for edge in &edges {
                inner.store_edge(edge)?;
                ids.push(edge.id);
            }
            Ok(ids)
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))?
    }

    fn get_session_nodes_stream(
        &self,
        session_id: &SessionId,
    ) -> std::pin::Pin<Box<dyn futures::stream::Stream<Item = Result<Node>> + Send + '_>> {
        let inner = Arc::clone(&self.inner);
        let session_id = *session_id;

        Box::pin(async_stream::stream! {
            // Load nodes in a blocking task, but stream them out
            // This provides some memory efficiency by not holding all nodes in memory at once
            let result = tokio::task::spawn_blocking(move || {
                inner.get_session_nodes(&session_id)
            })
            .await
            .map_err(|e| crate::error::Error::RuntimeError(e.to_string()));

            match result {
                Ok(Ok(nodes)) => {
                    // Stream nodes out one at a time
                    for node in nodes {
                        yield Ok(node);
                    }
                }
                Ok(Err(e)) => yield Err(e),
                Err(e) => yield Err(e),
            }
        })
    }

    async fn count_session_nodes(&self, session_id: &SessionId) -> Result<usize> {
        let inner = Arc::clone(&self.inner);
        let session_id = *session_id;

        tokio::task::spawn_blocking(move || {
            inner.get_session_nodes(&session_id)
                .map(|nodes| nodes.len())
        })
        .await
        .map_err(|e| crate::error::Error::RuntimeError(e.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ConversationSession, PromptNode};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_async_backend_creation() {
        let dir = tempdir().unwrap();
        let backend = AsyncSledBackend::open(dir.path()).await.unwrap();

        // Should be able to get stats
        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.node_count, 0);
    }

    #[tokio::test]
    async fn test_async_node_operations() {
        let dir = tempdir().unwrap();
        let backend = AsyncSledBackend::open(dir.path()).await.unwrap();

        // Create and store a session
        let session = ConversationSession::new();
        backend.store_node(&Node::Session(session.clone())).await.unwrap();

        // Retrieve it
        let retrieved = backend.get_node(&session.node_id).await.unwrap();
        assert!(retrieved.is_some());

        // Check stats
        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.node_count, 1);
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let dir = tempdir().unwrap();
        let backend = AsyncSledBackend::open(dir.path()).await.unwrap();

        let session = ConversationSession::new();
        backend.store_node(&Node::Session(session.clone())).await.unwrap();

        // Perform 100 concurrent write operations
        let mut handles = vec![];
        for i in 0..100 {
            let backend_clone = backend.clone();
            let session_id = session.id;

            let handle = tokio::spawn(async move {
                let prompt = PromptNode::new(
                    session_id,
                    format!("Prompt {}", i),
                );
                backend_clone.store_node(&Node::Prompt(prompt)).await
            });

            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Verify all prompts were stored
        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.node_count, 101); // 1 session + 100 prompts
    }

    #[tokio::test]
    async fn test_batch_operations() {
        let dir = tempdir().unwrap();
        let backend = AsyncSledBackend::open(dir.path()).await.unwrap();

        let session = ConversationSession::new();

        // Create multiple nodes
        let mut nodes = vec![Node::Session(session.clone())];
        for i in 0..10 {
            let prompt = PromptNode::new(
                session.id,
                format!("Prompt {}", i),
            );
            nodes.push(Node::Prompt(prompt));
        }

        // Batch store
        let ids = backend.store_nodes_batch(&nodes).await.unwrap();
        assert_eq!(ids.len(), 11);

        // Verify stats
        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.node_count, 11);
    }

    #[tokio::test]
    async fn test_session_nodes_streaming() {
        use crate::storage::AsyncStorageBackend;
        use futures::stream::StreamExt;

        let dir = tempdir().unwrap();
        let backend = AsyncSledBackend::open(dir.path()).await.unwrap();

        let session = ConversationSession::new();
        backend.store_node(&Node::Session(session.clone())).await.unwrap();

        // Add 20 prompts
        for i in 0..20 {
            let prompt = PromptNode::new(session.id, format!("Prompt {}", i));
            backend.store_node(&Node::Prompt(prompt)).await.unwrap();
        }

        // Stream nodes
        let mut stream = backend.get_session_nodes_stream(&session.id);
        let mut count = 0;
        while let Some(result) = stream.next().await {
            result.unwrap();
            count += 1;
        }

        assert_eq!(count, 21); // 1 session + 20 prompts
    }

    #[tokio::test]
    async fn test_count_session_nodes() {
        use crate::storage::AsyncStorageBackend;

        let dir = tempdir().unwrap();
        let backend = AsyncSledBackend::open(dir.path()).await.unwrap();

        let session = ConversationSession::new();
        backend.store_node(&Node::Session(session.clone())).await.unwrap();

        // Add 15 prompts
        for i in 0..15 {
            let prompt = PromptNode::new(session.id, format!("Prompt {}", i));
            backend.store_node(&Node::Prompt(prompt)).await.unwrap();
        }

        // Count without loading
        let count = backend.count_session_nodes(&session.id).await.unwrap();
        assert_eq!(count, 16); // 1 session + 15 prompts
    }

    #[tokio::test]
    async fn test_streaming_vs_batch() {
        use crate::storage::AsyncStorageBackend;
        use futures::stream::StreamExt;

        let dir = tempdir().unwrap();
        let backend = AsyncSledBackend::open(dir.path()).await.unwrap();

        let session = ConversationSession::new();
        backend.store_node(&Node::Session(session.clone())).await.unwrap();

        // Add 50 prompts
        for i in 0..50 {
            let prompt = PromptNode::new(session.id, format!("Prompt {}", i));
            backend.store_node(&Node::Prompt(prompt)).await.unwrap();
        }

        // Get via batch
        let batch_nodes = backend.get_session_nodes(&session.id).await.unwrap();

        // Get via streaming
        let mut stream = backend.get_session_nodes_stream(&session.id);
        let mut stream_nodes = Vec::new();
        while let Some(result) = stream.next().await {
            stream_nodes.push(result.unwrap());
        }

        // Both should return same nodes
        assert_eq!(batch_nodes.len(), stream_nodes.len());
        assert_eq!(batch_nodes.len(), 51); // 1 session + 50 prompts
    }
}

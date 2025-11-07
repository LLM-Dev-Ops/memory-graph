//! Async query builder with streaming support for memory-efficient queries
//!
//! This module provides a fluent API for building and executing async queries
//! over the graph data with support for streaming large result sets.

use crate::error::Result;
use crate::storage::AsyncStorageBackend;
use crate::types::{Node, NodeType, SessionId};
use chrono::{DateTime, Utc};
use futures::stream::Stream;
use std::pin::Pin;
use std::sync::Arc;

/// Builder for constructing async queries over the graph
///
/// Provides a fluent API for filtering and executing queries asynchronously.
/// Supports both batch loading and streaming for memory-efficient processing.
///
/// # Examples
///
/// ```no_run
/// use llm_memory_graph::query::AsyncQueryBuilder;
/// use llm_memory_graph::types::NodeType;
/// use futures::stream::StreamExt;
///
/// # async fn example(builder: AsyncQueryBuilder) -> Result<(), Box<dyn std::error::Error>> {
/// // Query with filters
/// let nodes = builder
///     .node_type(NodeType::Prompt)
///     .limit(100)
///     .execute()
///     .await?;
/// # Ok(())
/// # }
///
/// # async fn example2(builder: AsyncQueryBuilder) -> Result<(), Box<dyn std::error::Error>> {
/// // Stream large result sets
/// let mut stream = builder.execute_stream();
/// while let Some(node) = stream.next().await {
///     // Process node...
/// }
/// # Ok(())
/// # }
/// ```
pub struct AsyncQueryBuilder {
    storage: Arc<dyn AsyncStorageBackend>,
    session_filter: Option<SessionId>,
    node_type_filter: Option<NodeType>,
    time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    limit: Option<usize>,
    offset: usize,
}

impl AsyncQueryBuilder {
    /// Create a new async query builder
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::query::AsyncQueryBuilder;
    /// use llm_memory_graph::storage::AsyncSledBackend;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let backend = AsyncSledBackend::open("./data/graph.db").await?;
    /// let builder = AsyncQueryBuilder::new(Arc::new(backend));
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(storage: Arc<dyn AsyncStorageBackend>) -> Self {
        Self {
            storage,
            session_filter: None,
            node_type_filter: None,
            time_range: None,
            limit: None,
            offset: 0,
        }
    }

    /// Filter by session ID
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::query::AsyncQueryBuilder;
    /// # use llm_memory_graph::types::SessionId;
    /// # async fn example(builder: AsyncQueryBuilder, session_id: SessionId) -> Result<(), Box<dyn std::error::Error>> {
    /// let nodes = builder
    ///     .session(session_id)
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn session(mut self, session_id: SessionId) -> Self {
        self.session_filter = Some(session_id);
        self
    }

    /// Filter by node type
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::query::AsyncQueryBuilder;
    /// # use llm_memory_graph::types::NodeType;
    /// # async fn example(builder: AsyncQueryBuilder) -> Result<(), Box<dyn std::error::Error>> {
    /// let prompts = builder
    ///     .node_type(NodeType::Prompt)
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn node_type(mut self, node_type: NodeType) -> Self {
        self.node_type_filter = Some(node_type);
        self
    }

    /// Filter by time range (inclusive)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::query::AsyncQueryBuilder;
    /// # use chrono::Utc;
    /// # async fn example(builder: AsyncQueryBuilder) -> Result<(), Box<dyn std::error::Error>> {
    /// let start = Utc::now() - chrono::Duration::hours(24);
    /// let end = Utc::now();
    ///
    /// let recent_nodes = builder
    ///     .time_range(start, end)
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.time_range = Some((start, end));
        self
    }

    /// Limit the number of results
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::query::AsyncQueryBuilder;
    /// # async fn example(builder: AsyncQueryBuilder) -> Result<(), Box<dyn std::error::Error>> {
    /// let first_10 = builder
    ///     .limit(10)
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Skip the first N results
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::query::AsyncQueryBuilder;
    /// # async fn example(builder: AsyncQueryBuilder) -> Result<(), Box<dyn std::error::Error>> {
    /// // Get results 11-20 (skip first 10, take next 10)
    /// let page2 = builder
    ///     .offset(10)
    ///     .limit(10)
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Execute the query and return all matching nodes
    ///
    /// This loads all results into memory. For large result sets, consider using
    /// `execute_stream()` instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::query::AsyncQueryBuilder;
    /// # async fn example(builder: AsyncQueryBuilder) -> Result<(), Box<dyn std::error::Error>> {
    /// let nodes = builder.execute().await?;
    /// println!("Found {} nodes", nodes.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute(&self) -> Result<Vec<Node>> {
        // Get base nodes from session or all nodes
        let mut nodes = if let Some(session_id) = &self.session_filter {
            self.storage.get_session_nodes(session_id).await?
        } else {
            // For now, we'll need to iterate through sessions
            // In production, you'd want a more efficient approach
            vec![]
        };

        // Apply node type filter
        if let Some(node_type) = &self.node_type_filter {
            nodes.retain(|node| node.node_type() == *node_type);
        }

        // Apply time range filter
        if let Some((start, end)) = &self.time_range {
            nodes.retain(|node| {
                let timestamp = match node {
                    Node::Prompt(p) => p.timestamp,
                    Node::Response(r) => r.timestamp,
                    Node::Session(s) => s.created_at,
                    Node::ToolInvocation(t) => t.timestamp,
                    Node::Agent(a) => a.created_at,
                    Node::Template(t) => t.created_at,
                };
                timestamp >= *start && timestamp <= *end
            });
        }

        // Sort by timestamp (newest first)
        nodes.sort_by(|a, b| {
            let ts_a = match a {
                Node::Prompt(p) => p.timestamp,
                Node::Response(r) => r.timestamp,
                Node::Session(s) => s.created_at,
                Node::ToolInvocation(t) => t.timestamp,
                Node::Agent(a) => a.created_at,
                Node::Template(t) => t.created_at,
            };
            let ts_b = match b {
                Node::Prompt(p) => p.timestamp,
                Node::Response(r) => r.timestamp,
                Node::Session(s) => s.created_at,
                Node::ToolInvocation(t) => t.timestamp,
                Node::Agent(a) => a.created_at,
                Node::Template(t) => t.created_at,
            };
            ts_b.cmp(&ts_a)
        });

        // Apply offset
        let nodes: Vec<_> = nodes.into_iter().skip(self.offset).collect();

        // Apply limit
        let nodes = if let Some(limit) = self.limit {
            nodes.into_iter().take(limit).collect()
        } else {
            nodes
        };

        Ok(nodes)
    }

    /// Execute the query and return a stream of results
    ///
    /// This is memory-efficient for large result sets as it processes nodes
    /// one at a time without loading everything into memory. The stream uses
    /// storage-level streaming to avoid loading all nodes at once.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::query::AsyncQueryBuilder;
    /// # use futures::stream::StreamExt;
    /// # async fn example(builder: AsyncQueryBuilder) -> Result<(), Box<dyn std::error::Error>> {
    /// let mut stream = builder.execute_stream();
    ///
    /// let mut count = 0;
    /// while let Some(result) = stream.next().await {
    ///     match result {
    ///         Ok(node) => {
    ///             // Process node without loading all into memory
    ///             count += 1;
    ///         }
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    ///
    /// println!("Processed {} nodes", count);
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute_stream(&self) -> Pin<Box<dyn Stream<Item = Result<Node>> + Send + '_>> {
        use futures::StreamExt;

        let session_filter = self.session_filter;
        let node_type_filter = self.node_type_filter.clone();
        let time_range = self.time_range;
        let limit = self.limit;
        let offset = self.offset;

        Box::pin(async_stream::stream! {
            // Use storage-level streaming for better memory efficiency
            let mut stream = if let Some(session_id) = session_filter {
                self.storage.get_session_nodes_stream(&session_id)
            } else {
                // Empty stream if no session filter
                Box::pin(futures::stream::empty()) as Pin<Box<dyn Stream<Item = Result<Node>> + Send + '_>>
            };

            // Apply filters and stream results
            let mut skipped = 0;
            let mut emitted = 0;

            while let Some(result) = stream.next().await {
                let node = match result {
                    Ok(n) => n,
                    Err(e) => {
                        yield Err(e);
                        continue;
                    }
                };

                // Apply node type filter
                if let Some(ref nt) = node_type_filter {
                    if node.node_type() != *nt {
                        continue;
                    }
                }

                // Apply time range filter
                if let Some((start, end)) = time_range {
                    let timestamp = match &node {
                        Node::Prompt(p) => p.timestamp,
                        Node::Response(r) => r.timestamp,
                        Node::Session(s) => s.created_at,
                        Node::ToolInvocation(t) => t.timestamp,
                        Node::Agent(a) => a.created_at,
                        Node::Template(t) => t.created_at,
                    };

                    if timestamp < start || timestamp > end {
                        continue;
                    }
                }

                // Apply offset
                if skipped < offset {
                    skipped += 1;
                    continue;
                }

                // Apply limit
                if let Some(lim) = limit {
                    if emitted >= lim {
                        break;
                    }
                }

                emitted += 1;
                yield Ok(node);
            }
        })
    }

    /// Count the number of matching nodes without loading them
    ///
    /// This is more efficient than `execute().await?.len()` for large result sets
    /// as it uses storage-level counting when possible.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::query::AsyncQueryBuilder;
    /// # use llm_memory_graph::types::NodeType;
    /// # async fn example(builder: AsyncQueryBuilder) -> Result<(), Box<dyn std::error::Error>> {
    /// let prompt_count = builder
    ///     .node_type(NodeType::Prompt)
    ///     .count()
    ///     .await?;
    ///
    /// println!("Total prompts: {}", prompt_count);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn count(&self) -> Result<usize> {
        use futures::StreamExt;

        // If we only have a session filter and no other filters, use efficient count
        if self.session_filter.is_some()
            && self.node_type_filter.is_none()
            && self.time_range.is_none()
            && self.offset == 0
            && self.limit.is_none()
        {
            return self.storage.count_session_nodes(&self.session_filter.unwrap()).await;
        }

        // Otherwise, stream and count to avoid loading all into memory
        let mut stream = self.execute_stream();
        let mut count = 0;
        while let Some(result) = stream.next().await {
            result?;
            count += 1;
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::AsyncSledBackend;
    use crate::types::{ConversationSession, PromptNode};
    use futures::stream::StreamExt;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_query_builder_creation() {
        let dir = tempdir().unwrap();
        let backend = AsyncSledBackend::open(dir.path()).await.unwrap();
        let builder = AsyncQueryBuilder::new(Arc::new(backend) as Arc<dyn crate::storage::AsyncStorageBackend>);

        let results = builder.execute().await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_query_with_session_filter() {
        let dir = tempdir().unwrap();
        let backend = Arc::new(AsyncSledBackend::open(dir.path()).await.unwrap()) as Arc<dyn crate::storage::AsyncStorageBackend>;

        // Create test data
        let session = ConversationSession::new();
        backend
            .store_node(&Node::Session(session.clone()))
            .await
            .unwrap();

        for i in 0..5 {
            let prompt = PromptNode::new(session.id, format!("Prompt {}", i));
            backend.store_node(&Node::Prompt(prompt)).await.unwrap();
        }

        // Query with session filter
        let builder = AsyncQueryBuilder::new(backend);
        let results = builder.session(session.id).execute().await.unwrap();

        assert_eq!(results.len(), 6); // 1 session + 5 prompts
    }

    #[tokio::test]
    async fn test_query_with_node_type_filter() {
        let dir = tempdir().unwrap();
        let backend = Arc::new(AsyncSledBackend::open(dir.path()).await.unwrap()) as Arc<dyn crate::storage::AsyncStorageBackend>;

        let session = ConversationSession::new();
        backend
            .store_node(&Node::Session(session.clone()))
            .await
            .unwrap();

        for i in 0..3 {
            let prompt = PromptNode::new(session.id, format!("Prompt {}", i));
            backend.store_node(&Node::Prompt(prompt)).await.unwrap();
        }

        // Query only prompts
        let builder = AsyncQueryBuilder::new(backend);
        let results = builder
            .session(session.id)
            .node_type(NodeType::Prompt)
            .execute()
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
        for node in results {
            assert!(matches!(node, Node::Prompt(_)));
        }
    }

    #[tokio::test]
    async fn test_query_with_limit_and_offset() {
        let dir = tempdir().unwrap();
        let backend = Arc::new(AsyncSledBackend::open(dir.path()).await.unwrap()) as Arc<dyn crate::storage::AsyncStorageBackend>;

        let session = ConversationSession::new();
        backend
            .store_node(&Node::Session(session.clone()))
            .await
            .unwrap();

        for i in 0..10 {
            let prompt = PromptNode::new(session.id, format!("Prompt {}", i));
            backend.store_node(&Node::Prompt(prompt)).await.unwrap();
        }

        // Test limit
        let builder = AsyncQueryBuilder::new(Arc::clone(&backend));
        let results = builder
            .session(session.id)
            .node_type(NodeType::Prompt)
            .limit(5)
            .execute()
            .await
            .unwrap();
        assert_eq!(results.len(), 5);

        // Test offset + limit (pagination)
        let builder = AsyncQueryBuilder::new(backend);
        let results = builder
            .session(session.id)
            .node_type(NodeType::Prompt)
            .offset(5)
            .limit(3)
            .execute()
            .await
            .unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_query_streaming() {
        let dir = tempdir().unwrap();
        let backend = Arc::new(AsyncSledBackend::open(dir.path()).await.unwrap()) as Arc<dyn crate::storage::AsyncStorageBackend>;

        let session = ConversationSession::new();
        backend
            .store_node(&Node::Session(session.clone()))
            .await
            .unwrap();

        for i in 0..10 {
            let prompt = PromptNode::new(session.id, format!("Prompt {}", i));
            backend.store_node(&Node::Prompt(prompt)).await.unwrap();
        }

        // Stream results
        let query = AsyncQueryBuilder::new(backend)
            .session(session.id)
            .node_type(NodeType::Prompt);
        let mut stream = query.execute_stream();

        let mut count = 0;
        while let Some(result) = stream.next().await {
            result.unwrap();
            count += 1;
        }

        assert_eq!(count, 10);
    }

    #[tokio::test]
    async fn test_query_count() {
        let dir = tempdir().unwrap();
        let backend = Arc::new(AsyncSledBackend::open(dir.path()).await.unwrap()) as Arc<dyn crate::storage::AsyncStorageBackend>;

        let session = ConversationSession::new();
        backend
            .store_node(&Node::Session(session.clone()))
            .await
            .unwrap();

        for i in 0..7 {
            let prompt = PromptNode::new(session.id, format!("Prompt {}", i));
            backend.store_node(&Node::Prompt(prompt)).await.unwrap();
        }

        // Count prompts
        let builder = AsyncQueryBuilder::new(backend);
        let count = builder
            .session(session.id)
            .node_type(NodeType::Prompt)
            .count()
            .await
            .unwrap();

        assert_eq!(count, 7);
    }

    #[tokio::test]
    async fn test_streaming_with_limit() {
        let dir = tempdir().unwrap();
        let backend = Arc::new(AsyncSledBackend::open(dir.path()).await.unwrap()) as Arc<dyn crate::storage::AsyncStorageBackend>;

        let session = ConversationSession::new();
        backend
            .store_node(&Node::Session(session.clone()))
            .await
            .unwrap();

        for i in 0..20 {
            let prompt = PromptNode::new(session.id, format!("Prompt {}", i));
            backend.store_node(&Node::Prompt(prompt)).await.unwrap();
        }

        // Stream with limit
        let query = AsyncQueryBuilder::new(backend)
            .session(session.id)
            .node_type(NodeType::Prompt)
            .limit(5);
        let mut stream = query.execute_stream();

        let mut count = 0;
        while let Some(result) = stream.next().await {
            result.unwrap();
            count += 1;
        }

        assert_eq!(count, 5);
    }
}

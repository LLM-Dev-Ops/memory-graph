//! Query interface for graph traversal and filtering

pub mod async_query;

pub use async_query::AsyncQueryBuilder;

use crate::error::{Error, Result};
use crate::types::{EdgeType, Node, NodeId, NodeType, SessionId};
use chrono::{DateTime, Utc};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{Bfs, Dfs};
use std::collections::HashMap;

/// Builder for constructing graph queries
///
/// Provides a fluent interface for filtering and traversing the memory graph.
///
/// # Examples
///
/// ```no_run
/// use llm_memory_graph::{MemoryGraph, Config, query::QueryBuilder, NodeType};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let graph = MemoryGraph::open(Config::default())?;
/// # let session = graph.create_session()?;
/// let nodes = QueryBuilder::new(&graph)
///     .session(session.id)
///     .node_type(NodeType::Prompt)
///     .limit(10)
///     .execute()?;
/// # Ok(())
/// # }
/// ```
pub struct QueryBuilder<'a> {
    graph: &'a crate::engine::MemoryGraph,
    session_filter: Option<SessionId>,
    node_type_filter: Option<NodeType>,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    limit: Option<usize>,
    offset: usize,
}

impl<'a> QueryBuilder<'a> {
    /// Create a new query builder
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, query::QueryBuilder};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// let query = QueryBuilder::new(&graph);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub const fn new(graph: &'a crate::engine::MemoryGraph) -> Self {
        Self {
            graph,
            session_filter: None,
            node_type_filter: None,
            start_time: None,
            end_time: None,
            limit: None,
            offset: 0,
        }
    }

    /// Filter by session ID
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, query::QueryBuilder};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// let query = QueryBuilder::new(&graph).session(session.id);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub const fn session(mut self, session_id: SessionId) -> Self {
        self.session_filter = Some(session_id);
        self
    }

    /// Filter by node type
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, query::QueryBuilder, NodeType};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// let query = QueryBuilder::new(&graph).node_type(NodeType::Prompt);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub const fn node_type(mut self, node_type: NodeType) -> Self {
        self.node_type_filter = Some(node_type);
        self
    }

    /// Filter by start time (inclusive)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, query::QueryBuilder};
    /// # use chrono::Utc;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// let query = QueryBuilder::new(&graph).after(Utc::now());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub const fn after(mut self, time: DateTime<Utc>) -> Self {
        self.start_time = Some(time);
        self
    }

    /// Filter by end time (inclusive)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, query::QueryBuilder};
    /// # use chrono::Utc;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// let query = QueryBuilder::new(&graph).before(Utc::now());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub const fn before(mut self, time: DateTime<Utc>) -> Self {
        self.end_time = Some(time);
        self
    }

    /// Limit the number of results
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, query::QueryBuilder};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// let query = QueryBuilder::new(&graph).limit(10);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub const fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Skip the first N results (for pagination)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, query::QueryBuilder};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// let query = QueryBuilder::new(&graph).offset(20).limit(10);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub const fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Execute the query and return matching nodes
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Storage retrieval fails
    /// - The specified session doesn't exist
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, query::QueryBuilder};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// let nodes = QueryBuilder::new(&graph)
    ///     .session(session.id)
    ///     .execute()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute(&self) -> Result<Vec<Node>> {
        let mut nodes = if let Some(session_id) = self.session_filter {
            self.graph.get_session_nodes(session_id)?
        } else {
            // If no session filter, we'd need to scan all nodes
            // For now, require a session filter for efficiency
            return Err(Error::ValidationError(
                "Query must specify a session filter".to_string(),
            ));
        };

        // Apply node type filter
        if let Some(ref node_type) = self.node_type_filter {
            nodes.retain(|n| n.node_type() == *node_type);
        }

        // Apply time filters
        if let Some(start_time) = self.start_time {
            nodes.retain(|n| {
                let timestamp = match n {
                    Node::Prompt(p) => p.timestamp,
                    Node::Response(r) => r.timestamp,
                    Node::Session(s) => s.created_at,
                    Node::ToolInvocation(t) => t.timestamp,
                    Node::Agent(a) => a.created_at,
                    Node::Template(t) => t.created_at,
                };
                timestamp >= start_time
            });
        }

        if let Some(end_time) = self.end_time {
            nodes.retain(|n| {
                let timestamp = match n {
                    Node::Prompt(p) => p.timestamp,
                    Node::Response(r) => r.timestamp,
                    Node::Session(s) => s.created_at,
                    Node::ToolInvocation(t) => t.timestamp,
                    Node::Agent(a) => a.created_at,
                    Node::Template(t) => t.created_at,
                };
                timestamp <= end_time
            });
        }

        // Sort by timestamp (newest first)
        nodes.sort_by(|a, b| {
            let time_a = match a {
                Node::Prompt(p) => p.timestamp,
                Node::Response(r) => r.timestamp,
                Node::Session(s) => s.created_at,
                Node::ToolInvocation(t) => t.timestamp,
                Node::Agent(a) => a.created_at,
                Node::Template(t) => t.created_at,
            };
            let time_b = match b {
                Node::Prompt(p) => p.timestamp,
                Node::Response(r) => r.timestamp,
                Node::Session(s) => s.created_at,
                Node::ToolInvocation(t) => t.timestamp,
                Node::Agent(a) => a.created_at,
                Node::Template(t) => t.created_at,
            };
            time_b.cmp(&time_a)
        });

        // Apply offset and limit
        let start = self.offset;
        let end = if let Some(limit) = self.limit {
            (start + limit).min(nodes.len())
        } else {
            nodes.len()
        };

        Ok(nodes[start..end].to_vec())
    }
}

/// Graph traversal utilities
pub struct GraphTraversal<'a> {
    graph: &'a crate::engine::MemoryGraph,
}

impl<'a> GraphTraversal<'a> {
    /// Create a new graph traversal helper
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, query::GraphTraversal};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// let traversal = GraphTraversal::new(&graph);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub const fn new(graph: &'a crate::engine::MemoryGraph) -> Self {
        Self { graph }
    }

    /// Build a petgraph representation of the subgraph starting from a node
    ///
    /// # Errors
    ///
    /// Returns an error if node or edge retrieval fails.
    fn build_subgraph(&self, start: NodeId) -> Result<(DiGraph<NodeId, EdgeType>, NodeIndex)> {
        let mut graph = DiGraph::new();
        let mut node_map: HashMap<NodeId, NodeIndex> = HashMap::new();

        // Add start node
        let start_idx = graph.add_node(start);
        node_map.insert(start, start_idx);

        // BFS to build the graph
        let mut queue = vec![start];
        let mut visited = std::collections::HashSet::new();
        visited.insert(start);

        while let Some(current) = queue.pop() {
            let current_idx = node_map[&current];

            // Get outgoing edges
            if let Ok(edges) = self.graph.get_outgoing_edges(current) {
                for edge in edges {
                    // Add target node if not exists
                    let target_idx = *node_map.entry(edge.to).or_insert_with(|| {
                        graph.add_node(edge.to)
                    });

                    // Add edge
                    graph.add_edge(current_idx, target_idx, edge.edge_type.clone());

                    // Queue target for processing
                    if visited.insert(edge.to) {
                        queue.push(edge.to);
                    }
                }
            }

            // Get incoming edges
            if let Ok(edges) = self.graph.get_incoming_edges(current) {
                for edge in edges {
                    // Add source node if not exists
                    let source_idx = *node_map.entry(edge.from).or_insert_with(|| {
                        graph.add_node(edge.from)
                    });

                    // Add edge
                    graph.add_edge(source_idx, current_idx, edge.edge_type.clone());

                    // Queue source for processing
                    if visited.insert(edge.from) {
                        queue.push(edge.from);
                    }
                }
            }
        }

        Ok((graph, start_idx))
    }

    /// Perform breadth-first search from a starting node
    ///
    /// Returns nodes in BFS order.
    ///
    /// # Errors
    ///
    /// Returns an error if graph traversal fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, query::GraphTraversal};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// let traversal = GraphTraversal::new(&graph);
    /// let nodes = traversal.bfs(prompt_id)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn bfs(&self, start: NodeId) -> Result<Vec<NodeId>> {
        let (pg_graph, start_idx) = self.build_subgraph(start)?;
        let mut bfs = Bfs::new(&pg_graph, start_idx);
        let mut result = Vec::new();

        while let Some(idx) = bfs.next(&pg_graph) {
            if let Some(node_id) = pg_graph.node_weight(idx) {
                result.push(*node_id);
            }
        }

        Ok(result)
    }

    /// Perform depth-first search from a starting node
    ///
    /// Returns nodes in DFS order.
    ///
    /// # Errors
    ///
    /// Returns an error if graph traversal fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, query::GraphTraversal};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// let traversal = GraphTraversal::new(&graph);
    /// let nodes = traversal.dfs(prompt_id)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn dfs(&self, start: NodeId) -> Result<Vec<NodeId>> {
        let (pg_graph, start_idx) = self.build_subgraph(start)?;
        let mut dfs = Dfs::new(&pg_graph, start_idx);
        let mut result = Vec::new();

        while let Some(idx) = dfs.next(&pg_graph) {
            if let Some(node_id) = pg_graph.node_weight(idx) {
                result.push(*node_id);
            }
        }

        Ok(result)
    }

    /// Get the conversation thread for a prompt or response
    ///
    /// Returns nodes in chronological order (oldest to newest).
    ///
    /// # Errors
    ///
    /// Returns an error if node retrieval fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, query::GraphTraversal};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// let traversal = GraphTraversal::new(&graph);
    /// let thread = traversal.get_conversation_thread(prompt_id)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_conversation_thread(&self, start: NodeId) -> Result<Vec<Node>> {
        let node = self.graph.get_node(start)?;

        // Get session ID from the node
        let session_id = match &node {
            Node::Prompt(p) => p.session_id,
            Node::Response(r) => {
                // Get the prompt to find session
                let prompt_node = self.graph.get_node(r.prompt_id)?;
                if let Node::Prompt(p) = prompt_node {
                    p.session_id
                } else {
                    return Err(Error::TraversalError(
                        "Response does not point to a prompt".to_string(),
                    ));
                }
            }
            Node::Session(s) => s.id,
            Node::ToolInvocation(t) => {
                // Get the response to find the session
                let response_node = self.graph.get_node(t.response_id)?;
                if let Node::Response(r) = response_node {
                    let prompt_node = self.graph.get_node(r.prompt_id)?;
                    if let Node::Prompt(p) = prompt_node {
                        p.session_id
                    } else {
                        return Err(Error::TraversalError(
                            "Response does not point to a prompt".to_string(),
                        ));
                    }
                } else {
                    return Err(Error::TraversalError(
                        "ToolInvocation does not point to a response".to_string(),
                    ));
                }
            }
            Node::Agent(_a) => {
                // Agents are global entities, find sessions they're involved in
                // via HandledBy edges
                return Err(Error::TraversalError(
                    "Cannot get conversation thread for agent nodes".to_string(),
                ));
            }
            Node::Template(_t) => {
                // Templates are global entities, not part of conversations
                return Err(Error::TraversalError(
                    "Cannot get conversation thread for template nodes".to_string(),
                ));
            }
        };

        // Get all nodes in the session
        let mut nodes = self.graph.get_session_nodes(session_id)?;

        // Filter to only prompts and responses
        nodes.retain(|n| matches!(n, Node::Prompt(_) | Node::Response(_)));

        // Sort chronologically
        nodes.sort_by(|a, b| {
            let time_a = match a {
                Node::Prompt(p) => p.timestamp,
                Node::Response(r) => r.timestamp,
                Node::Session(s) => s.created_at,
                Node::ToolInvocation(t) => t.timestamp,
                Node::Agent(ag) => ag.created_at,
                Node::Template(t) => t.created_at,
            };
            let time_b = match b {
                Node::Prompt(p) => p.timestamp,
                Node::Response(r) => r.timestamp,
                Node::Session(s) => s.created_at,
                Node::ToolInvocation(t) => t.timestamp,
                Node::Agent(ag) => ag.created_at,
                Node::Template(t) => t.created_at,
            };
            time_a.cmp(&time_b)
        });

        Ok(nodes)
    }

    /// Find all responses to a prompt
    ///
    /// # Errors
    ///
    /// Returns an error if edge or node retrieval fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use llm_memory_graph::{MemoryGraph, Config, query::GraphTraversal};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let graph = MemoryGraph::open(Config::default())?;
    /// # let session = graph.create_session()?;
    /// # let prompt_id = graph.add_prompt(session.id, "Test".to_string(), None)?;
    /// let traversal = GraphTraversal::new(&graph);
    /// let responses = traversal.find_responses(prompt_id)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn find_responses(&self, prompt_id: NodeId) -> Result<Vec<Node>> {
        let incoming = self.graph.get_incoming_edges(prompt_id)?;
        let mut responses = Vec::new();

        for edge in incoming {
            if edge.edge_type == EdgeType::RespondsTo {
                if let Ok(node) = self.graph.get_node(edge.from) {
                    if matches!(node, Node::Response(_)) {
                        responses.push(node);
                    }
                }
            }
        }

        Ok(responses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::MemoryGraph;
    use crate::types::{Config, TokenUsage};
    use tempfile::tempdir;

    #[test]
    fn test_query_builder() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = MemoryGraph::open(config).unwrap();

        let session = graph.create_session().unwrap();
        graph
            .add_prompt(session.id, "Test 1".to_string(), None)
            .unwrap();
        graph
            .add_prompt(session.id, "Test 2".to_string(), None)
            .unwrap();

        let nodes = QueryBuilder::new(&graph)
            .session(session.id)
            .node_type(NodeType::Prompt)
            .execute()
            .unwrap();

        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_query_limit_offset() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = MemoryGraph::open(config).unwrap();

        let session = graph.create_session().unwrap();
        for i in 0..5 {
            graph
                .add_prompt(session.id, format!("Test {i}"), None)
                .unwrap();
        }

        let nodes = QueryBuilder::new(&graph)
            .session(session.id)
            .node_type(NodeType::Prompt)
            .limit(2)
            .offset(1)
            .execute()
            .unwrap();

        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_bfs_traversal() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = MemoryGraph::open(config).unwrap();

        let session = graph.create_session().unwrap();
        let prompt_id = graph
            .add_prompt(session.id, "Test".to_string(), None)
            .unwrap();

        let traversal = GraphTraversal::new(&graph);
        let nodes = traversal.bfs(prompt_id).unwrap();

        assert!(!nodes.is_empty());
        assert_eq!(nodes[0], prompt_id);
    }

    #[test]
    fn test_conversation_thread() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = MemoryGraph::open(config).unwrap();

        let session = graph.create_session().unwrap();
        let prompt1 = graph
            .add_prompt(session.id, "First".to_string(), None)
            .unwrap();
        let usage = TokenUsage::new(10, 20);
        let _response1 = graph
            .add_response(prompt1, "Response 1".to_string(), usage, None)
            .unwrap();

        let traversal = GraphTraversal::new(&graph);
        let thread = traversal.get_conversation_thread(prompt1).unwrap();

        assert_eq!(thread.len(), 2); // 1 prompt + 1 response
    }

    #[test]
    fn test_find_responses() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = MemoryGraph::open(config).unwrap();

        let session = graph.create_session().unwrap();
        let prompt_id = graph
            .add_prompt(session.id, "Test".to_string(), None)
            .unwrap();
        let usage = TokenUsage::new(10, 20);
        let _response_id = graph
            .add_response(prompt_id, "Response".to_string(), usage, None)
            .unwrap();

        let traversal = GraphTraversal::new(&graph);
        let responses = traversal.find_responses(prompt_id).unwrap();

        assert_eq!(responses.len(), 1);
    }

    #[test]
    fn test_query_without_session_fails() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());
        let graph = MemoryGraph::open(config).unwrap();

        let result = QueryBuilder::new(&graph).execute();

        assert!(result.is_err());
    }
}

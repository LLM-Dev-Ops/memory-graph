//! Lineage builder for constructing graph relationships from distributed traces
//!
//! This module provides functionality to build lineage chains from trace spans,
//! establishing parent-child relationships and causal dependencies in the memory graph.

use super::consumer::{SpanStatus, TelemetryData};
use crate::{EdgeType, NodeId, NodeType, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A lineage node represents a single operation in a trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    /// Unique identifier for this lineage node
    pub id: String,
    /// Operation name
    pub operation: String,
    /// Start timestamp
    pub start_time: DateTime<Utc>,
    /// End timestamp
    pub end_time: DateTime<Utc>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Execution status
    pub status: SpanStatus,
    /// Attributes from the span
    pub attributes: HashMap<String, String>,
    /// Memory graph node ID (if mapped)
    pub graph_node_id: Option<NodeId>,
}

/// A lineage edge represents a relationship between operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    /// Source lineage node ID
    pub from: String,
    /// Target lineage node ID
    pub to: String,
    /// Edge type (parent-child, follows, etc.)
    pub edge_type: LineageEdgeType,
}

/// Types of lineage relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageEdgeType {
    /// Parent-child relationship (span hierarchy)
    ParentChild,
    /// Sequential follows relationship
    Follows,
    /// Causal dependency
    CausedBy,
    /// Data flow relationship
    DataFlow,
}

/// A complete lineage chain representing a distributed trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageChain {
    /// Trace ID
    pub trace_id: String,
    /// Nodes in the lineage
    pub nodes: Vec<LineageNode>,
    /// Edges between nodes
    pub edges: Vec<LineageEdge>,
    /// Root node IDs (entry points)
    pub roots: Vec<String>,
    /// Metadata about the trace
    pub metadata: HashMap<String, String>,
}

impl LineageChain {
    /// Create a new empty lineage chain
    pub fn new(trace_id: String) -> Self {
        Self {
            trace_id,
            nodes: Vec::new(),
            edges: Vec::new(),
            roots: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a node to the lineage chain
    pub fn add_node(&mut self, node: LineageNode) {
        self.nodes.push(node);
    }

    /// Add an edge to the lineage chain
    pub fn add_edge(&mut self, edge: LineageEdge) {
        self.edges.push(edge);
    }

    /// Find a node by ID
    pub fn find_node(&self, id: &str) -> Option<&LineageNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get all child nodes of a given node
    pub fn get_children(&self, node_id: &str) -> Vec<&LineageNode> {
        let child_ids: Vec<&str> = self
            .edges
            .iter()
            .filter(|e| e.from == node_id && e.edge_type == LineageEdgeType::ParentChild)
            .map(|e| e.to.as_str())
            .collect();

        self.nodes
            .iter()
            .filter(|n| child_ids.contains(&n.id.as_str()))
            .collect()
    }

    /// Get the total duration of the trace
    pub fn total_duration_ms(&self) -> u64 {
        if self.nodes.is_empty() {
            return 0;
        }

        let min_start = self
            .nodes
            .iter()
            .map(|n| n.start_time)
            .min()
            .unwrap_or_else(Utc::now);

        let max_end = self
            .nodes
            .iter()
            .map(|n| n.end_time)
            .max()
            .unwrap_or_else(Utc::now);

        (max_end - min_start).num_milliseconds().max(0) as u64
    }

    /// Count nodes by status
    pub fn count_by_status(&self) -> HashMap<SpanStatus, usize> {
        let mut counts = HashMap::new();
        for node in &self.nodes {
            *counts.entry(node.status).or_insert(0) += 1;
        }
        counts
    }
}

/// Builder for constructing lineage chains from trace spans
pub struct LineageBuilder {
    /// In-progress lineage chains indexed by trace ID
    chains: Arc<RwLock<HashMap<String, LineageChain>>>,
}

impl LineageBuilder {
    /// Create a new lineage builder
    pub fn new() -> Self {
        Self {
            chains: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Process a span and add it to the appropriate lineage chain
    pub async fn process_span(&self, data: &TelemetryData) -> Result<()> {
        if let TelemetryData::Span {
            span_id,
            trace_id,
            parent_span_id,
            operation_name,
            start_time,
            end_time,
            attributes,
            status,
        } = data
        {
            let duration_ms = (*end_time - *start_time).num_milliseconds().max(0) as u64;

            let node = LineageNode {
                id: span_id.clone(),
                operation: operation_name.clone(),
                start_time: *start_time,
                end_time: *end_time,
                duration_ms,
                status: *status,
                attributes: attributes.clone(),
                graph_node_id: None,
            };

            let mut chains = self.chains.write().await;
            let chain = chains
                .entry(trace_id.clone())
                .or_insert_with(|| LineageChain::new(trace_id.clone()));

            // Add the node
            chain.add_node(node);

            // Add parent-child edge if parent exists
            if let Some(parent_id) = parent_span_id {
                let edge = LineageEdge {
                    from: parent_id.clone(),
                    to: span_id.clone(),
                    edge_type: LineageEdgeType::ParentChild,
                };
                chain.add_edge(edge);
            } else {
                // This is a root span
                if !chain.roots.contains(span_id) {
                    chain.roots.push(span_id.clone());
                }
            }
        }

        Ok(())
    }

    /// Get a lineage chain by trace ID
    pub async fn get_chain(&self, trace_id: &str) -> Option<LineageChain> {
        self.chains.read().await.get(trace_id).cloned()
    }

    /// Get all lineage chains
    pub async fn get_all_chains(&self) -> Vec<LineageChain> {
        self.chains.read().await.values().cloned().collect()
    }

    /// Remove a completed lineage chain
    pub async fn remove_chain(&self, trace_id: &str) -> Option<LineageChain> {
        self.chains.write().await.remove(trace_id)
    }

    /// Clear all lineage chains
    pub async fn clear(&self) {
        self.chains.write().await.clear();
    }

    /// Get the number of active lineage chains
    pub async fn chain_count(&self) -> usize {
        self.chains.read().await.len()
    }

    /// Convert a lineage node to a memory graph node type
    pub fn infer_node_type(operation: &str) -> NodeType {
        // Infer node type from operation name conventions
        if operation.contains("prompt") || operation.contains("llm.generate") {
            NodeType::Prompt
        } else if operation.contains("response") || operation.contains("llm.completion") {
            NodeType::Response
        } else if operation.contains("tool") || operation.contains("function") {
            NodeType::Tool
        } else {
            NodeType::Context
        }
    }

    /// Convert a lineage edge type to a memory graph edge type
    pub fn map_edge_type(lineage_type: LineageEdgeType) -> EdgeType {
        match lineage_type {
            LineageEdgeType::ParentChild => EdgeType::ParentChild,
            LineageEdgeType::Follows => EdgeType::Follows,
            LineageEdgeType::CausedBy => EdgeType::References,
            LineageEdgeType::DataFlow => EdgeType::Contains,
        }
    }
}

impl Default for LineageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_span(
        span_id: &str,
        trace_id: &str,
        parent: Option<&str>,
        operation: &str,
    ) -> TelemetryData {
        let start_time = Utc::now();
        let end_time = start_time + chrono::Duration::milliseconds(100);

        TelemetryData::Span {
            span_id: span_id.to_string(),
            trace_id: trace_id.to_string(),
            parent_span_id: parent.map(|s| s.to_string()),
            operation_name: operation.to_string(),
            start_time,
            end_time,
            attributes: HashMap::new(),
            status: SpanStatus::Ok,
        }
    }

    #[tokio::test]
    async fn test_lineage_builder_single_span() {
        let builder = LineageBuilder::new();

        let span = create_test_span("span-1", "trace-1", None, "llm.generate");
        builder.process_span(&span).await.unwrap();

        let chain = builder.get_chain("trace-1").await.unwrap();
        assert_eq!(chain.nodes.len(), 1);
        assert_eq!(chain.edges.len(), 0);
        assert_eq!(chain.roots.len(), 1);
        assert_eq!(chain.roots[0], "span-1");
    }

    #[tokio::test]
    async fn test_lineage_builder_parent_child() {
        let builder = LineageBuilder::new();

        let parent = create_test_span("span-1", "trace-1", None, "llm.generate");
        let child = create_test_span("span-2", "trace-1", Some("span-1"), "tool.call");

        builder.process_span(&parent).await.unwrap();
        builder.process_span(&child).await.unwrap();

        let chain = builder.get_chain("trace-1").await.unwrap();
        assert_eq!(chain.nodes.len(), 2);
        assert_eq!(chain.edges.len(), 1);
        assert_eq!(chain.roots.len(), 1);

        let edge = &chain.edges[0];
        assert_eq!(edge.from, "span-1");
        assert_eq!(edge.to, "span-2");
        assert_eq!(edge.edge_type, LineageEdgeType::ParentChild);
    }

    #[tokio::test]
    async fn test_lineage_builder_multiple_traces() {
        let builder = LineageBuilder::new();

        let span1 = create_test_span("span-1", "trace-1", None, "op1");
        let span2 = create_test_span("span-2", "trace-2", None, "op2");

        builder.process_span(&span1).await.unwrap();
        builder.process_span(&span2).await.unwrap();

        assert_eq!(builder.chain_count().await, 2);

        let chain1 = builder.get_chain("trace-1").await.unwrap();
        let chain2 = builder.get_chain("trace-2").await.unwrap();

        assert_eq!(chain1.nodes.len(), 1);
        assert_eq!(chain2.nodes.len(), 1);
    }

    #[tokio::test]
    async fn test_lineage_chain_get_children() {
        let mut chain = LineageChain::new("trace-1".to_string());

        let parent = LineageNode {
            id: "span-1".to_string(),
            operation: "parent".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_ms: 100,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            graph_node_id: None,
        };

        let child1 = LineageNode {
            id: "span-2".to_string(),
            operation: "child1".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_ms: 50,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            graph_node_id: None,
        };

        let child2 = LineageNode {
            id: "span-3".to_string(),
            operation: "child2".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_ms: 50,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            graph_node_id: None,
        };

        chain.add_node(parent);
        chain.add_node(child1);
        chain.add_node(child2);

        chain.add_edge(LineageEdge {
            from: "span-1".to_string(),
            to: "span-2".to_string(),
            edge_type: LineageEdgeType::ParentChild,
        });

        chain.add_edge(LineageEdge {
            from: "span-1".to_string(),
            to: "span-3".to_string(),
            edge_type: LineageEdgeType::ParentChild,
        });

        let children = chain.get_children("span-1");
        assert_eq!(children.len(), 2);
    }

    #[tokio::test]
    async fn test_lineage_builder_clear() {
        let builder = LineageBuilder::new();

        let span = create_test_span("span-1", "trace-1", None, "op1");
        builder.process_span(&span).await.unwrap();

        assert_eq!(builder.chain_count().await, 1);

        builder.clear().await;
        assert_eq!(builder.chain_count().await, 0);
    }

    #[tokio::test]
    async fn test_lineage_builder_remove_chain() {
        let builder = LineageBuilder::new();

        let span = create_test_span("span-1", "trace-1", None, "op1");
        builder.process_span(&span).await.unwrap();

        assert_eq!(builder.chain_count().await, 1);

        let chain = builder.remove_chain("trace-1").await;
        assert!(chain.is_some());
        assert_eq!(builder.chain_count().await, 0);
    }

    #[test]
    fn test_infer_node_type() {
        assert_eq!(
            LineageBuilder::infer_node_type("llm.generate"),
            NodeType::Prompt
        );
        assert_eq!(
            LineageBuilder::infer_node_type("llm.completion"),
            NodeType::Response
        );
        assert_eq!(
            LineageBuilder::infer_node_type("tool.execute"),
            NodeType::Tool
        );
        assert_eq!(
            LineageBuilder::infer_node_type("other.operation"),
            NodeType::Context
        );
    }

    #[test]
    fn test_map_edge_type() {
        assert_eq!(
            LineageBuilder::map_edge_type(LineageEdgeType::ParentChild),
            EdgeType::ParentChild
        );
        assert_eq!(
            LineageBuilder::map_edge_type(LineageEdgeType::Follows),
            EdgeType::Follows
        );
        assert_eq!(
            LineageBuilder::map_edge_type(LineageEdgeType::CausedBy),
            EdgeType::References
        );
        assert_eq!(
            LineageBuilder::map_edge_type(LineageEdgeType::DataFlow),
            EdgeType::Contains
        );
    }

    #[test]
    fn test_lineage_chain_total_duration() {
        let mut chain = LineageChain::new("trace-1".to_string());

        let start = Utc::now();
        let mid = start + chrono::Duration::milliseconds(100);
        let end = start + chrono::Duration::milliseconds(200);

        chain.add_node(LineageNode {
            id: "span-1".to_string(),
            operation: "op1".to_string(),
            start_time: start,
            end_time: mid,
            duration_ms: 100,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            graph_node_id: None,
        });

        chain.add_node(LineageNode {
            id: "span-2".to_string(),
            operation: "op2".to_string(),
            start_time: mid,
            end_time: end,
            duration_ms: 100,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            graph_node_id: None,
        });

        assert_eq!(chain.total_duration_ms(), 200);
    }

    #[test]
    fn test_lineage_chain_count_by_status() {
        let mut chain = LineageChain::new("trace-1".to_string());

        chain.add_node(LineageNode {
            id: "span-1".to_string(),
            operation: "op1".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_ms: 100,
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            graph_node_id: None,
        });

        chain.add_node(LineageNode {
            id: "span-2".to_string(),
            operation: "op2".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_ms: 100,
            status: SpanStatus::Error,
            attributes: HashMap::new(),
            graph_node_id: None,
        });

        let counts = chain.count_by_status();
        assert_eq!(counts.get(&SpanStatus::Ok), Some(&1));
        assert_eq!(counts.get(&SpanStatus::Error), Some(&1));
    }
}

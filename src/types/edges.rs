//! Edge types for connecting nodes in the memory graph

use super::{EdgeId, NodeId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of edges that can connect nodes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    /// Connects sequential prompts in a conversation (Prompt → Prompt)
    Follows,
    /// Links a response to its originating prompt (Response → Prompt)
    RespondsTo,
    /// Tracks which agent handled a prompt (Prompt → Agent)
    HandledBy,
    /// Links a prompt to the session it belongs to (Prompt → Session)
    PartOf,
}

/// An edge connecting two nodes in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Unique edge identifier
    pub id: EdgeId,
    /// Source node ID
    pub from: NodeId,
    /// Target node ID
    pub to: NodeId,
    /// Type of relationship
    pub edge_type: EdgeType,
    /// When the edge was created
    pub created_at: DateTime<Utc>,
    /// Additional properties for this edge
    pub properties: HashMap<String, String>,
}

impl Edge {
    /// Create a new edge between two nodes
    #[must_use]
    pub fn new(from: NodeId, to: NodeId, edge_type: EdgeType) -> Self {
        Self {
            id: EdgeId::new(),
            from,
            to,
            edge_type,
            created_at: Utc::now(),
            properties: HashMap::new(),
        }
    }

    /// Create an edge with custom properties
    #[must_use]
    pub fn with_properties(
        from: NodeId,
        to: NodeId,
        edge_type: EdgeType,
        properties: HashMap<String, String>,
    ) -> Self {
        Self {
            id: EdgeId::new(),
            from,
            to,
            edge_type,
            created_at: Utc::now(),
            properties,
        }
    }

    /// Add a property to the edge
    pub fn add_property(&mut self, key: String, value: String) {
        self.properties.insert(key, value);
    }

    /// Get a property value
    #[must_use]
    pub fn get_property(&self, key: &str) -> Option<&String> {
        self.properties.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_creation() {
        let from = NodeId::new();
        let to = NodeId::new();
        let edge = Edge::new(from, to, EdgeType::Follows);
        assert_eq!(edge.from, from);
        assert_eq!(edge.to, to);
        assert_eq!(edge.edge_type, EdgeType::Follows);
    }

    #[test]
    fn test_edge_properties() {
        let from = NodeId::new();
        let to = NodeId::new();
        let mut edge = Edge::new(from, to, EdgeType::RespondsTo);
        edge.add_property("latency_ms".to_string(), "150".to_string());
        assert_eq!(edge.get_property("latency_ms"), Some(&"150".to_string()));
    }

    #[test]
    fn test_edge_with_properties() {
        let from = NodeId::new();
        let to = NodeId::new();
        let mut props = HashMap::new();
        props.insert("test".to_string(), "value".to_string());
        let edge = Edge::with_properties(from, to, EdgeType::HandledBy, props);
        assert_eq!(edge.properties.len(), 1);
    }
}

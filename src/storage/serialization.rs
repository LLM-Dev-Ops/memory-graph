//! Serialization utilities for storage

use crate::error::{Error, Result};
use crate::types::{Edge, Node};

/// Serialization format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationFormat {
    /// JSON format (human-readable, slower)
    Json,
    /// MessagePack format (binary, faster)
    MessagePack,
    /// Bincode format (binary, fastest)
    Bincode,
}

/// Handles serialization and deserialization of graph entities
pub struct Serializer {
    format: SerializationFormat,
}

impl Serializer {
    /// Create a new serializer with the specified format
    #[must_use]
    pub const fn new(format: SerializationFormat) -> Self {
        Self { format }
    }

    /// Serialize a node to bytes
    pub fn serialize_node(&self, node: &Node) -> Result<Vec<u8>> {
        match self.format {
            SerializationFormat::Json => {
                serde_json::to_vec(node).map_err(|e| Error::Serialization(e.to_string()))
            }
            SerializationFormat::MessagePack => {
                rmp_serde::to_vec(node).map_err(|e| Error::Serialization(e.to_string()))
            }
            SerializationFormat::Bincode => {
                bincode::serialize(node).map_err(|e| Error::Serialization(e.to_string()))
            }
        }
    }

    /// Deserialize a node from bytes
    pub fn deserialize_node(&self, bytes: &[u8]) -> Result<Node> {
        match self.format {
            SerializationFormat::Json => {
                serde_json::from_slice(bytes).map_err(|e| Error::Serialization(e.to_string()))
            }
            SerializationFormat::MessagePack => {
                rmp_serde::from_slice(bytes).map_err(|e| Error::Serialization(e.to_string()))
            }
            SerializationFormat::Bincode => {
                bincode::deserialize(bytes).map_err(|e| Error::Serialization(e.to_string()))
            }
        }
    }

    /// Serialize an edge to bytes
    pub fn serialize_edge(&self, edge: &Edge) -> Result<Vec<u8>> {
        match self.format {
            SerializationFormat::Json => {
                serde_json::to_vec(edge).map_err(|e| Error::Serialization(e.to_string()))
            }
            SerializationFormat::MessagePack => {
                rmp_serde::to_vec(edge).map_err(|e| Error::Serialization(e.to_string()))
            }
            SerializationFormat::Bincode => {
                bincode::serialize(edge).map_err(|e| Error::Serialization(e.to_string()))
            }
        }
    }

    /// Deserialize an edge from bytes
    pub fn deserialize_edge(&self, bytes: &[u8]) -> Result<Edge> {
        match self.format {
            SerializationFormat::Json => {
                serde_json::from_slice(bytes).map_err(|e| Error::Serialization(e.to_string()))
            }
            SerializationFormat::MessagePack => {
                rmp_serde::from_slice(bytes).map_err(|e| Error::Serialization(e.to_string()))
            }
            SerializationFormat::Bincode => {
                bincode::deserialize(bytes).map_err(|e| Error::Serialization(e.to_string()))
            }
        }
    }
}

impl Default for Serializer {
    fn default() -> Self {
        Self::new(SerializationFormat::MessagePack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NodeId, PromptNode, SessionId};

    #[test]
    fn test_node_json_serialization() {
        let session_id = SessionId::new();
        let prompt = PromptNode::new(session_id, "Test".to_string());
        let node = Node::Prompt(prompt);

        let serializer = Serializer::new(SerializationFormat::Json);
        let bytes = serializer.serialize_node(&node).unwrap();
        let deserialized = serializer.deserialize_node(&bytes).unwrap();

        assert_eq!(node.id(), deserialized.id());
    }

    #[test]
    fn test_node_messagepack_serialization() {
        let session_id = SessionId::new();
        let prompt = PromptNode::new(session_id, "Test".to_string());
        let node = Node::Prompt(prompt);

        let serializer = Serializer::new(SerializationFormat::MessagePack);
        let bytes = serializer.serialize_node(&node).unwrap();
        let deserialized = serializer.deserialize_node(&bytes).unwrap();

        assert_eq!(node.id(), deserialized.id());
    }

    #[test]
    fn test_edge_serialization() {
        use crate::types::{Edge, EdgeType};

        let from = NodeId::new();
        let to = NodeId::new();
        let edge = Edge::new(from, to, EdgeType::Follows);

        let serializer = Serializer::default();
        let bytes = serializer.serialize_edge(&edge).unwrap();
        let deserialized = serializer.deserialize_edge(&bytes).unwrap();

        assert_eq!(edge.id, deserialized.id);
        assert_eq!(edge.edge_type, deserialized.edge_type);
    }
}

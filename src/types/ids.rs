//! Unique identifier types for graph entities

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a node in the graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(Uuid);

impl NodeId {
    /// Create a new random node ID
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a node ID from a UUID
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Convert to bytes for storage
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 16] {
        *self.0.as_bytes()
    }

    /// Create from bytes
    #[must_use]
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for NodeId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// Unique identifier for a conversation session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);

impl SessionId {
    /// Create a new random session ID
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a session ID from a UUID
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Convert to bytes for storage
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 16] {
        *self.0.as_bytes()
    }

    /// Create from bytes
    #[must_use]
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an edge in the graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(Uuid);

impl EdgeId {
    /// Create a new random edge ID
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Convert to bytes for storage
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 16] {
        *self.0.as_bytes()
    }

    /// Create from bytes
    #[must_use]
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }
}

impl Default for EdgeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a prompt template
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemplateId(Uuid);

impl TemplateId {
    /// Create a new random template ID
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a template ID from a UUID
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for TemplateId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TemplateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an autonomous agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(Uuid);

impl AgentId {
    /// Create a new random agent ID
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create an agent ID from a UUID
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Convert to bytes for storage
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 16] {
        *self.0.as_bytes()
    }

    /// Create from bytes
    #[must_use]
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(Uuid::from_bytes(bytes))
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for AgentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_creation() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_node_id_bytes_roundtrip() {
        let id = NodeId::new();
        let bytes = id.to_bytes();
        let restored = NodeId::from_bytes(bytes);
        assert_eq!(id, restored);
    }

    #[test]
    fn test_session_id_creation() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_session_id_display() {
        let id = SessionId::new();
        let display = format!("{id}");
        assert!(!display.is_empty());
    }

    #[test]
    fn test_agent_id_creation() {
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_agent_id_bytes_roundtrip() {
        let id = AgentId::new();
        let bytes = id.to_bytes();
        let restored = AgentId::from_bytes(bytes);
        assert_eq!(id, restored);
    }

    #[test]
    fn test_agent_id_display() {
        let id = AgentId::new();
        let display = format!("{id}");
        assert!(!display.is_empty());
    }
}

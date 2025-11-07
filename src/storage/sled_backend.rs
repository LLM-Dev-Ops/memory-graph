//! Sled-based storage backend implementation

use super::{Serializer, SerializationFormat, StorageBackend, StorageStats};
use crate::error::{Error, Result};
use crate::types::{Edge, EdgeId, Node, NodeId, SessionId};
use sled::{Db, Tree};
use std::path::Path;

/// Sled-based storage backend
pub struct SledBackend {
    db: Db,
    nodes: Tree,
    edges: Tree,
    session_index: Tree,
    outgoing_edges_index: Tree,
    incoming_edges_index: Tree,
    serializer: Serializer,
}

impl SledBackend {
    /// Open or create a new Sled backend at the specified path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)?;

        let nodes = db.open_tree(b"nodes")?;
        let edges = db.open_tree(b"edges")?;
        let session_index = db.open_tree(b"session_index")?;
        let outgoing_edges_index = db.open_tree(b"outgoing_edges")?;
        let incoming_edges_index = db.open_tree(b"incoming_edges")?;

        Ok(Self {
            db,
            nodes,
            edges,
            session_index,
            outgoing_edges_index,
            incoming_edges_index,
            serializer: Serializer::new(SerializationFormat::MessagePack),
        })
    }

    /// Open with a custom serialization format
    pub fn open_with_format<P: AsRef<Path>>(
        path: P,
        format: SerializationFormat,
    ) -> Result<Self> {
        let mut backend = Self::open(path)?;
        backend.serializer = Serializer::new(format);
        Ok(backend)
    }

    /// Build a composite key for indexing
    fn build_index_key(prefix: &[u8], id: &[u8]) -> Vec<u8> {
        let mut key = Vec::with_capacity(prefix.len() + id.len());
        key.extend_from_slice(prefix);
        key.extend_from_slice(id);
        key
    }
}

impl StorageBackend for SledBackend {
    fn store_node(&self, node: &Node) -> Result<()> {
        let id = node.id();
        let bytes = self.serializer.serialize_node(node)?;

        // Store the node
        self.nodes.insert(id.to_bytes(), bytes)?;

        // Update session index for prompts and responses
        match node {
            Node::Prompt(p) => {
                let key = Self::build_index_key(&p.session_id.to_bytes(), &id.to_bytes());
                self.session_index.insert(key, &[])?;
            }
            Node::Response(r) => {
                // Find the prompt to get session_id
                if let Some(prompt_bytes) = self.nodes.get(r.prompt_id.to_bytes())? {
                    if let Ok(prompt_node) = self.serializer.deserialize_node(&prompt_bytes) {
                        if let Node::Prompt(p) = prompt_node {
                            let key = Self::build_index_key(&p.session_id.to_bytes(), &id.to_bytes());
                            self.session_index.insert(key, &[])?;
                        }
                    }
                }
            }
            Node::Session(s) => {
                let key = Self::build_index_key(&s.id.to_bytes(), &id.to_bytes());
                self.session_index.insert(key, &[])?;
            }
            Node::ToolInvocation(_t) => {
                // Tool invocations are not directly indexed by session
                // They're accessed via response nodes through edges
            }
            Node::Agent(_a) => {
                // Agents are global entities, not tied to specific sessions
                // They're accessed via agent ID or HandledBy/TransfersTo edges
            }
            Node::Template(_t) => {
                // Templates are global entities, not tied to specific sessions
                // They're accessed via template ID or Instantiates/Inherits edges
            }
        }

        self.db.flush()?;
        Ok(())
    }

    fn get_node(&self, id: &NodeId) -> Result<Option<Node>> {
        match self.nodes.get(id.to_bytes())? {
            Some(bytes) => {
                let node = self.serializer.deserialize_node(&bytes)?;
                Ok(Some(node))
            }
            None => Ok(None),
        }
    }

    fn delete_node(&self, id: &NodeId) -> Result<()> {
        self.nodes.remove(id.to_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    fn store_edge(&self, edge: &Edge) -> Result<()> {
        let bytes = self.serializer.serialize_edge(edge)?;

        // Store the edge
        self.edges.insert(edge.id.to_bytes(), bytes)?;

        // Update outgoing edges index
        let outgoing_key = Self::build_index_key(&edge.from.to_bytes(), &edge.id.to_bytes());
        self.outgoing_edges_index.insert(outgoing_key, &[])?;

        // Update incoming edges index
        let incoming_key = Self::build_index_key(&edge.to.to_bytes(), &edge.id.to_bytes());
        self.incoming_edges_index.insert(incoming_key, &[])?;

        self.db.flush()?;
        Ok(())
    }

    fn get_edge(&self, id: &EdgeId) -> Result<Option<Edge>> {
        match self.edges.get(id.to_bytes())? {
            Some(bytes) => {
                let edge = self.serializer.deserialize_edge(&bytes)?;
                Ok(Some(edge))
            }
            None => Ok(None),
        }
    }

    fn delete_edge(&self, id: &EdgeId) -> Result<()> {
        self.edges.remove(id.to_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    fn get_session_nodes(&self, session_id: &SessionId) -> Result<Vec<Node>> {
        let prefix = session_id.to_bytes();
        let mut nodes = Vec::new();

        for result in self.session_index.scan_prefix(prefix) {
            let (key, _) = result?;
            // Extract node ID from composite key (skip session_id bytes)
            if key.len() >= 32 {
                let node_id_bytes: [u8; 16] = key[16..32].try_into()
                    .map_err(|_| Error::Storage("Invalid node ID in index".to_string()))?;
                let node_id = NodeId::from_bytes(node_id_bytes);

                if let Some(node) = self.get_node(&node_id)? {
                    nodes.push(node);
                }
            }
        }

        Ok(nodes)
    }

    fn get_outgoing_edges(&self, node_id: &NodeId) -> Result<Vec<Edge>> {
        let prefix = node_id.to_bytes();
        let mut edges = Vec::new();

        for result in self.outgoing_edges_index.scan_prefix(prefix) {
            let (key, _) = result?;
            // Extract edge ID from composite key
            if key.len() >= 32 {
                let edge_id_bytes: [u8; 16] = key[16..32].try_into()
                    .map_err(|_| Error::Storage("Invalid edge ID in index".to_string()))?;
                let edge_id = EdgeId::from_bytes(edge_id_bytes);

                if let Some(edge) = self.get_edge(&edge_id)? {
                    edges.push(edge);
                }
            }
        }

        Ok(edges)
    }

    fn get_incoming_edges(&self, node_id: &NodeId) -> Result<Vec<Edge>> {
        let prefix = node_id.to_bytes();
        let mut edges = Vec::new();

        for result in self.incoming_edges_index.scan_prefix(prefix) {
            let (key, _) = result?;
            // Extract edge ID from composite key
            if key.len() >= 32 {
                let edge_id_bytes: [u8; 16] = key[16..32].try_into()
                    .map_err(|_| Error::Storage("Invalid edge ID in index".to_string()))?;
                let edge_id = EdgeId::from_bytes(edge_id_bytes);

                if let Some(edge) = self.get_edge(&edge_id)? {
                    edges.push(edge);
                }
            }
        }

        Ok(edges)
    }

    fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }

    fn stats(&self) -> Result<StorageStats> {
        let node_count = self.nodes.len() as u64;
        let edge_count = self.edges.len() as u64;
        let storage_bytes = self.db.size_on_disk()?;

        // Count unique sessions
        let mut session_count = 0u64;
        let mut last_session: Option<[u8; 16]> = None;

        for result in self.session_index.iter() {
            let (key, _) = result?;
            if key.len() >= 16 {
                let session_bytes: [u8; 16] = key[0..16].try_into().unwrap_or([0; 16]);
                if Some(session_bytes) != last_session {
                    session_count += 1;
                    last_session = Some(session_bytes);
                }
            }
        }

        Ok(StorageStats {
            node_count,
            edge_count,
            storage_bytes,
            session_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ConversationSession, EdgeType, PromptNode};
    use tempfile::tempdir;

    #[test]
    fn test_store_and_retrieve_node() {
        let dir = tempdir().unwrap();
        let backend = SledBackend::open(dir.path()).unwrap();

        let session = ConversationSession::new();
        let node = Node::Session(session.clone());

        backend.store_node(&node).unwrap();
        let retrieved = backend.get_node(&session.node_id).unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), session.node_id);
    }

    #[test]
    fn test_store_and_retrieve_edge() {
        let dir = tempdir().unwrap();
        let backend = SledBackend::open(dir.path()).unwrap();

        let from = NodeId::new();
        let to = NodeId::new();
        let edge = Edge::new(from, to, EdgeType::Follows);

        backend.store_edge(&edge).unwrap();
        let retrieved = backend.get_edge(&edge.id).unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, edge.id);
    }

    #[test]
    fn test_session_index() {
        let dir = tempdir().unwrap();
        let backend = SledBackend::open(dir.path()).unwrap();

        let session = ConversationSession::new();
        let session_node = Node::Session(session.clone());
        backend.store_node(&session_node).unwrap();

        let prompt = PromptNode::new(session.id, "Test".to_string());
        let prompt_node = Node::Prompt(prompt);
        backend.store_node(&prompt_node).unwrap();

        let nodes = backend.get_session_nodes(&session.id).unwrap();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_edge_indices() {
        let dir = tempdir().unwrap();
        let backend = SledBackend::open(dir.path()).unwrap();

        let from = NodeId::new();
        let to = NodeId::new();
        let edge = Edge::new(from, to, EdgeType::Follows);
        backend.store_edge(&edge).unwrap();

        let outgoing = backend.get_outgoing_edges(&from).unwrap();
        assert_eq!(outgoing.len(), 1);

        let incoming = backend.get_incoming_edges(&to).unwrap();
        assert_eq!(incoming.len(), 1);
    }

    #[test]
    fn test_stats() {
        let dir = tempdir().unwrap();
        let backend = SledBackend::open(dir.path()).unwrap();

        let session = ConversationSession::new();
        backend.store_node(&Node::Session(session)).unwrap();

        let stats = backend.stats().unwrap();
        assert_eq!(stats.node_count, 1);
        assert!(stats.storage_bytes > 0);
    }
}

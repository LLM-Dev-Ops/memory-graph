//! Vault storage adapter for dual-storage pattern
//!
//! Provides a storage adapter that writes to both Sled (primary) and Vault (secondary)
//! storage backends, enabling durable archival, compliance, and encryption while
//! maintaining fast local access through Sled.

use super::anonymizer::{DataAnonymizer, AnonymizationResult};
use super::archiver::{ArchiveEntry, VaultClient};
use super::config::{ArchivalMode, StorageMode, VaultStorageConfig};
use crate::integrations::IntegrationError;
use crate::storage::{AsyncStorageBackend, StorageStats};
use crate::{Edge, EdgeId, Node, NodeId, SessionId};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Dual-storage adapter combining Sled (primary) with Vault (secondary)
pub struct VaultStorageAdapter<B: AsyncStorageBackend> {
    /// Primary storage backend (Sled)
    primary: Arc<B>,

    /// Vault client for secondary storage
    vault_client: Option<Arc<VaultClient>>,

    /// Configuration
    config: VaultStorageConfig,

    /// Data anonymizer
    anonymizer: Option<Arc<DataAnonymizer>>,

    /// Pending archival queue (session_id -> nodes/edges)
    pending_archival: Arc<RwLock<HashMap<SessionId, PendingArchival>>>,

    /// Statistics
    stats: Arc<RwLock<AdapterStats>>,
}

/// Pending archival data for a session
#[derive(Debug, Clone, Default)]
struct PendingArchival {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    metadata: HashMap<String, String>,
}

/// Statistics for the storage adapter
#[derive(Debug, Clone, Default)]
pub struct AdapterStats {
    /// Total writes to Sled
    pub sled_writes: u64,

    /// Total writes to Vault
    pub vault_writes: u64,

    /// Vault write failures
    pub vault_failures: u64,

    /// PII instances anonymized
    pub pii_anonymized: u64,

    /// Sessions archived
    pub sessions_archived: u64,

    /// Bytes archived to vault
    pub bytes_archived: u64,
}

impl<B: AsyncStorageBackend> VaultStorageAdapter<B> {
    /// Create a new vault storage adapter
    ///
    /// # Errors
    /// Returns an error if vault client or anonymizer initialization fails
    pub async fn new(
        primary: Arc<B>,
        config: VaultStorageConfig,
    ) -> Result<Self, IntegrationError> {
        // Initialize vault client if enabled
        let vault_client = if config.enabled {
            let vault_config = crate::integrations::vault::VaultConfig::new(
                config.vault_url.clone(),
                config.api_key.clone(),
            )
            .with_encryption(config.encryption.enabled)
            .with_compression(config.encryption.compression_enabled)
            .with_timeout(config.performance.timeout_secs);

            let client = VaultClient::new(vault_config)?;

            // Health check
            match client.health_check().await {
                Ok(true) => {
                    info!("Vault storage adapter: Vault is healthy");
                    Some(Arc::new(client))
                }
                Ok(false) => {
                    if config.performance.graceful_degradation {
                        warn!("Vault storage adapter: Vault health check failed, continuing in degraded mode");
                        None
                    } else {
                        return Err(IntegrationError::ConnectionError(
                            "Vault health check failed".to_string(),
                        ));
                    }
                }
                Err(e) => {
                    if config.performance.graceful_degradation {
                        warn!("Vault storage adapter: Cannot reach vault ({}), continuing in degraded mode", e);
                        None
                    } else {
                        return Err(e);
                    }
                }
            }
        } else {
            debug!("Vault storage adapter: Vault storage disabled");
            None
        };

        // Initialize anonymizer if enabled
        let anonymizer = if config.anonymization.enabled {
            Some(Arc::new(DataAnonymizer::from_vault_config(&config)?))
        } else {
            None
        };

        Ok(Self {
            primary,
            vault_client,
            config,
            anonymizer,
            pending_archival: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(AdapterStats::default())),
        })
    }

    /// Create adapter with Sled-only mode (vault disabled)
    pub async fn sled_only(primary: Arc<B>) -> Result<Self, IntegrationError> {
        let config = VaultStorageConfig::default().with_vault_disabled();
        Self::new(primary, config).await
    }

    /// Get current adapter statistics
    pub async fn get_stats(&self) -> AdapterStats {
        self.stats.read().await.clone()
    }

    /// Archive a session to vault
    ///
    /// This is called explicitly or triggered by policy
    ///
    /// # Errors
    /// Returns an error if archival fails and graceful degradation is disabled
    pub async fn archive_session(&self, session_id: &SessionId) -> Result<String, IntegrationError> {
        // Get all data for the session
        let nodes = self.primary.get_session_nodes(session_id).await?;

        if nodes.is_empty() {
            debug!("Session {} has no nodes to archive", session_id);
            return Err(IntegrationError::NotFound(format!(
                "Session {} not found or empty",
                session_id
            )));
        }

        // Collect edges for all nodes in session
        let mut all_edges = Vec::new();
        for node in &nodes {
            let outgoing = self.primary.get_outgoing_edges(&node.id()).await?;
            all_edges.extend(outgoing);
        }

        // Serialize to JSON
        let session_data = serde_json::json!({
            "session_id": session_id.to_string(),
            "nodes": nodes,
            "edges": all_edges,
            "archived_at": Utc::now(),
        });

        // Anonymize if enabled
        let (final_data, pii_count) = if let Some(anonymizer) = &self.anonymizer {
            let result = anonymizer.anonymize_with_stats(&session_data)?;
            debug!("Anonymized {} PII instances in session {}", result.pii_count, session_id);

            // Update stats
            {
                let mut stats = self.stats.write().await;
                stats.pii_anonymized += result.pii_count as u64;
            }

            (result.data, result.pii_count)
        } else {
            (session_data, 0)
        };

        // Archive to vault
        if let Some(vault) = &self.vault_client {
            let entry = ArchiveEntry::new(
                session_id.to_string(),
                final_data.clone(),
                self.config.archival_policy.retention_days,
            )
            .with_tag("memory-graph")
            .with_metadata("node_count", serde_json::json!(nodes.len()))
            .with_metadata("edge_count", serde_json::json!(all_edges.len()))
            .with_metadata("pii_anonymized", serde_json::json!(pii_count));

            match vault.archive_session(entry).await {
                Ok(response) => {
                    info!(
                        "Archived session {} to vault with ID {}",
                        session_id, response.archive_id
                    );

                    // Update stats
                    {
                        let mut stats = self.stats.write().await;
                        stats.vault_writes += 1;
                        stats.sessions_archived += 1;
                        stats.bytes_archived += serde_json::to_string(&final_data)
                            .map(|s| s.len() as u64)
                            .unwrap_or(0);
                    }

                    Ok(response.archive_id)
                }
                Err(e) => {
                    error!("Failed to archive session {} to vault: {}", session_id, e);

                    // Update stats
                    {
                        let mut stats = self.stats.write().await;
                        stats.vault_failures += 1;
                    }

                    if self.config.performance.graceful_degradation {
                        warn!("Continuing despite vault archival failure");
                        Ok("degraded".to_string())
                    } else {
                        Err(e)
                    }
                }
            }
        } else {
            debug!("Vault not available, skipping archival");
            Ok("vault-disabled".to_string())
        }
    }

    /// Retrieve archived session from vault
    ///
    /// # Errors
    /// Returns an error if retrieval fails
    pub async fn retrieve_archived_session(
        &self,
        archive_id: &str,
    ) -> Result<ArchiveEntry, IntegrationError> {
        if let Some(vault) = &self.vault_client {
            vault.retrieve_session(archive_id).await
        } else {
            Err(IntegrationError::NotFound(
                "Vault not available".to_string(),
            ))
        }
    }

    /// Add node to pending archival queue
    async fn queue_for_archival(&self, node: &Node) {
        if matches!(self.config.archival_policy.mode, ArchivalMode::Immediate) {
            return; // Immediate mode doesn't use queue
        }

        // Extract session ID from node
        let session_id = match node {
            Node::Prompt(p) => p.session_id,
            Node::Response(r) => r.session_id,
            Node::Session(s) => Some(s.id),
            Node::ToolInvocation(t) => t.session_id,
            Node::Agent(_) => None,
            Node::Template(_) => None,
        };

        if let Some(sid) = session_id {
            let mut pending = self.pending_archival.write().await;
            let entry = pending.entry(sid).or_insert_with(PendingArchival::default);
            entry.nodes.push(node.clone());
        }
    }

    /// Write to vault based on storage mode
    async fn write_to_vault(&self, session_id: &SessionId, node: &Node) -> Result<(), IntegrationError> {
        match self.config.storage_mode {
            StorageMode::SledOnly => {
                // No vault write
                Ok(())
            }
            StorageMode::DualSync => {
                // Queue for archival (or archive immediately based on policy)
                self.queue_for_archival(node).await;
                Ok(())
            }
            StorageMode::DualAsync => {
                // Spawn async task for vault write
                let adapter = self.clone_for_async();
                let sid = *session_id;
                let node_clone = node.clone();

                tokio::spawn(async move {
                    adapter.queue_for_archival(&node_clone).await;
                });

                Ok(())
            }
            StorageMode::ArchiveOnPolicy => {
                // Queue based on policy
                match self.config.archival_policy.mode {
                    ArchivalMode::Immediate => {
                        // Archive immediately
                        self.archive_session(session_id).await?;
                    }
                    _ => {
                        // Queue for later archival
                        self.queue_for_archival(node).await;
                    }
                }
                Ok(())
            }
        }
    }

    /// Clone adapter for async operations
    fn clone_for_async(&self) -> Self {
        Self {
            primary: Arc::clone(&self.primary),
            vault_client: self.vault_client.as_ref().map(Arc::clone),
            config: self.config.clone(),
            anonymizer: self.anonymizer.as_ref().map(Arc::clone),
            pending_archival: Arc::clone(&self.pending_archival),
            stats: Arc::clone(&self.stats),
        }
    }

    /// Flush pending archival queue
    ///
    /// Archives all queued sessions to vault
    pub async fn flush_archival_queue(&self) -> Result<Vec<String>, IntegrationError> {
        let mut archived_ids = Vec::new();

        let session_ids: Vec<SessionId> = {
            let pending = self.pending_archival.read().await;
            pending.keys().copied().collect()
        };

        for session_id in session_ids {
            match self.archive_session(&session_id).await {
                Ok(archive_id) => {
                    archived_ids.push(archive_id);

                    // Remove from queue
                    self.pending_archival.write().await.remove(&session_id);
                }
                Err(e) => {
                    error!("Failed to archive queued session {}: {}", session_id, e);
                    if !self.config.performance.graceful_degradation {
                        return Err(e);
                    }
                }
            }
        }

        Ok(archived_ids)
    }
}

#[async_trait]
impl<B: AsyncStorageBackend> AsyncStorageBackend for VaultStorageAdapter<B> {
    async fn store_node(&self, node: &Node) -> crate::Result<()> {
        // Always write to primary storage first
        self.primary.store_node(node).await?;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.sled_writes += 1;
        }

        // Write to vault based on configuration
        // Extract session ID for vault write
        let session_id = match node {
            Node::Prompt(p) => p.session_id,
            Node::Response(r) => r.session_id,
            Node::Session(s) => Some(s.id),
            Node::ToolInvocation(t) => t.session_id,
            Node::Agent(_) => None,
            Node::Template(_) => None,
        };

        if let Some(sid) = session_id {
            if let Err(e) = self.write_to_vault(&sid, node).await {
                error!("Vault write failed: {}", e);
                if !self.config.performance.graceful_degradation {
                    return Err(crate::Error::Storage(format!("Vault write failed: {}", e)));
                }
            }
        }

        Ok(())
    }

    async fn get_node(&self, id: &NodeId) -> crate::Result<Option<Node>> {
        // Read from primary storage
        self.primary.get_node(id).await
    }

    async fn delete_node(&self, id: &NodeId) -> crate::Result<()> {
        // Delete from primary storage
        // Note: Vault archives are immutable and not deleted here
        self.primary.delete_node(id).await
    }

    async fn store_edge(&self, edge: &Edge) -> crate::Result<()> {
        // Write to primary storage
        self.primary.store_edge(edge).await?;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.sled_writes += 1;
        }

        Ok(())
    }

    async fn get_edge(&self, id: &EdgeId) -> crate::Result<Option<Edge>> {
        self.primary.get_edge(id).await
    }

    async fn delete_edge(&self, id: &EdgeId) -> crate::Result<()> {
        self.primary.delete_edge(id).await
    }

    async fn get_session_nodes(&self, session_id: &SessionId) -> crate::Result<Vec<Node>> {
        self.primary.get_session_nodes(session_id).await
    }

    async fn get_outgoing_edges(&self, node_id: &NodeId) -> crate::Result<Vec<Edge>> {
        self.primary.get_outgoing_edges(node_id).await
    }

    async fn get_incoming_edges(&self, node_id: &NodeId) -> crate::Result<Vec<Edge>> {
        self.primary.get_incoming_edges(node_id).await
    }

    async fn flush(&self) -> crate::Result<()> {
        // Flush primary storage
        self.primary.flush().await?;

        // Flush archival queue if configured
        if self.config.performance.graceful_degradation {
            if let Err(e) = self.flush_archival_queue().await {
                warn!("Failed to flush archival queue: {}", e);
            }
        } else {
            self.flush_archival_queue()
                .await
                .map_err(|e| crate::Error::Storage(format!("Archival queue flush failed: {}", e)))?;
        }

        Ok(())
    }

    async fn stats(&self) -> crate::Result<StorageStats> {
        // Return primary storage stats
        self.primary.stats().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::AsyncSledBackend;
    use tempfile::TempDir;

    async fn create_test_adapter() -> (VaultStorageAdapter<AsyncSledBackend>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.db");
        let backend = AsyncSledBackend::open(path.to_str().unwrap())
            .await
            .unwrap();

        let config = VaultStorageConfig::default().with_vault_disabled();
        let adapter = VaultStorageAdapter::new(Arc::new(backend), config)
            .await
            .unwrap();

        (adapter, temp_dir)
    }

    #[tokio::test]
    async fn test_sled_only_mode() {
        let (adapter, _temp) = create_test_adapter().await;

        let stats = adapter.get_stats().await;
        assert_eq!(stats.sled_writes, 0);
        assert_eq!(stats.vault_writes, 0);
    }

    #[tokio::test]
    async fn test_adapter_stats() {
        let (adapter, _temp) = create_test_adapter().await;

        let stats = adapter.get_stats().await;
        assert_eq!(stats.pii_anonymized, 0);
        assert_eq!(stats.sessions_archived, 0);
    }

    #[test]
    fn test_config_builder() {
        let config = VaultStorageConfig::new("http://vault:9000", "key")
            .with_storage_mode(StorageMode::DualAsync);

        assert_eq!(config.storage_mode, StorageMode::DualAsync);
        assert!(config.enabled);
    }
}

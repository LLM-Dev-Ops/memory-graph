//! Migration utilities for transitioning from sync to async APIs
//!
//! This module provides tools and helpers for migrating from the synchronous
//! `MemoryGraph` API to the asynchronous `AsyncMemoryGraph` API.
//!
//! # Migration Strategies
//!
//! ## 1. Gradual Migration (Recommended)
//!
//! Start by identifying async boundaries in your application and migrate
//! one component at a time:
//!
//! ```no_run
//! // Old sync code
//! use llm_memory_graph::MemoryGraph;
//!
//! fn process_sync() -> Result<(), Box<dyn std::error::Error>> {
//!     let graph = MemoryGraph::open(Default::default())?;
//!     let session = graph.create_session()?;
//!     Ok(())
//! }
//!
//! // New async code
//! use llm_memory_graph::AsyncMemoryGraph;
//!
//! async fn process_async() -> Result<(), Box<dyn std::error::Error>> {
//!     let graph = AsyncMemoryGraph::open(Default::default()).await?;
//!     let session = graph.create_session().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## 2. Parallel APIs
//!
//! Both APIs can coexist during migration. The sync and async versions
//! use the same storage format and are fully compatible.
//!
//! # Data Compatibility
//!
//! - Same storage format (Sled-based)
//! - No data migration required
//! - Can switch between sync and async at any time
//! - Node IDs and edge IDs are compatible

use crate::error::Result;
use crate::types::Config;
use crate::{AsyncMemoryGraph, MemoryGraph};

/// Migration helper providing utilities for transitioning between sync and async APIs
pub struct MigrationHelper;

impl MigrationHelper {
    /// Verify that a database is compatible with both sync and async APIs
    ///
    /// This function checks that:
    /// - The database can be opened with sync API
    /// - The database can be opened with async API
    /// - Both APIs can read the same data
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::{Config, migration::MigrationHelper};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = Config::new("./data/graph.db");
    ///     MigrationHelper::verify_compatibility(&config).await?;
    ///     println!("Database is compatible!");
    ///     Ok(())
    /// }
    /// ```
    pub async fn verify_compatibility(config: &Config) -> Result<CompatibilityReport> {
        let mut report = CompatibilityReport {
            sync_accessible: false,
            async_accessible: false,
            sync_node_count: 0,
            async_node_count: 0,
            sync_edge_count: 0,
            async_edge_count: 0,
            compatible: false,
        };

        // Test sync API
        match MemoryGraph::open(config.clone()) {
            Ok(graph) => {
                report.sync_accessible = true;
                if let Ok(stats) = graph.stats() {
                    report.sync_node_count = stats.node_count;
                    report.sync_edge_count = stats.edge_count;
                }
            }
            Err(_) => return Ok(report),
        }

        // Test async API
        match AsyncMemoryGraph::open(config.clone()).await {
            Ok(graph) => {
                report.async_accessible = true;
                if let Ok(stats) = graph.stats().await {
                    report.async_node_count = stats.node_count;
                    report.async_edge_count = stats.edge_count;
                }
            }
            Err(_) => return Ok(report),
        }

        // Verify counts match
        report.compatible = report.sync_accessible
            && report.async_accessible
            && report.sync_node_count == report.async_node_count
            && report.sync_edge_count == report.async_edge_count;

        Ok(report)
    }

    /// Create a migration checkpoint that can be used to rollback if needed
    ///
    /// This function creates a snapshot of database statistics that can be
    /// compared later to detect any issues during migration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::{Config, migration::MigrationHelper};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = Config::new("./data/graph.db");
    ///     let checkpoint = MigrationHelper::create_checkpoint(&config).await?;
    ///     println!("Checkpoint: {} nodes, {} edges",
    ///         checkpoint.node_count, checkpoint.edge_count);
    ///     Ok(())
    /// }
    /// ```
    pub async fn create_checkpoint(config: &Config) -> Result<MigrationCheckpoint> {
        let graph = AsyncMemoryGraph::open(config.clone()).await?;
        let stats = graph.stats().await?;

        Ok(MigrationCheckpoint {
            timestamp: chrono::Utc::now(),
            node_count: stats.node_count,
            edge_count: stats.edge_count,
            session_count: stats.session_count,
            storage_bytes: stats.storage_bytes,
        })
    }

    /// Verify that a database hasn't been corrupted after migration
    ///
    /// Compares current state against a checkpoint created before migration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::{Config, migration::MigrationHelper};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = Config::new("./data/graph.db");
    ///     let before = MigrationHelper::create_checkpoint(&config).await?;
    ///
    ///     // ... perform migration ...
    ///
    ///     let result = MigrationHelper::verify_checkpoint(&config, &before).await?;
    ///     assert!(result.valid, "Migration corrupted data!");
    ///     Ok(())
    /// }
    /// ```
    pub async fn verify_checkpoint(
        config: &Config,
        checkpoint: &MigrationCheckpoint,
    ) -> Result<CheckpointVerification> {
        let graph = AsyncMemoryGraph::open(config.clone()).await?;
        let stats = graph.stats().await?;

        let verification = CheckpointVerification {
            valid: stats.node_count >= checkpoint.node_count
                && stats.edge_count >= checkpoint.edge_count,
            node_count_match: stats.node_count == checkpoint.node_count,
            edge_count_match: stats.edge_count == checkpoint.edge_count,
            nodes_added: stats.node_count.saturating_sub(checkpoint.node_count),
            edges_added: stats.edge_count.saturating_sub(checkpoint.edge_count),
        };

        Ok(verification)
    }

    /// Run a test migration workflow to validate the migration process
    ///
    /// This performs a complete migration test:
    /// 1. Creates a checkpoint
    /// 2. Tests sync API access
    /// 3. Tests async API access
    /// 4. Verifies data integrity
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use llm_memory_graph::{Config, migration::MigrationHelper};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = Config::new("./data/graph.db");
    ///     let report = MigrationHelper::run_migration_test(&config).await?;
    ///     println!("Migration test: {}", if report.success { "PASSED" } else { "FAILED" });
    ///     Ok(())
    /// }
    /// ```
    pub async fn run_migration_test(config: &Config) -> Result<MigrationTestReport> {
        let mut report = MigrationTestReport {
            success: false,
            steps_completed: Vec::new(),
            errors: Vec::new(),
        };

        // Step 1: Create checkpoint
        match Self::create_checkpoint(config).await {
            Ok(_) => report.steps_completed.push("Checkpoint created".to_string()),
            Err(e) => {
                report
                    .errors
                    .push(format!("Failed to create checkpoint: {}", e));
                return Ok(report);
            }
        }

        // Step 2: Test sync API
        match MemoryGraph::open(config.clone()) {
            Ok(_) => report.steps_completed.push("Sync API accessible".to_string()),
            Err(e) => {
                report
                    .errors
                    .push(format!("Sync API failed: {}", e));
                return Ok(report);
            }
        }

        // Step 3: Test async API
        match AsyncMemoryGraph::open(config.clone()).await {
            Ok(_) => report.steps_completed.push("Async API accessible".to_string()),
            Err(e) => {
                report
                    .errors
                    .push(format!("Async API failed: {}", e));
                return Ok(report);
            }
        }

        // Step 4: Verify compatibility
        match Self::verify_compatibility(config).await {
            Ok(compat) if compat.compatible => {
                report.steps_completed.push("APIs compatible".to_string())
            }
            Ok(_) => {
                report
                    .errors
                    .push("APIs not compatible".to_string());
                return Ok(report);
            }
            Err(e) => {
                report
                    .errors
                    .push(format!("Compatibility check failed: {}", e));
                return Ok(report);
            }
        }

        report.success = true;
        Ok(report)
    }
}

/// Report on database compatibility between sync and async APIs
#[derive(Debug, Clone)]
pub struct CompatibilityReport {
    /// Whether the sync API can access the database
    pub sync_accessible: bool,
    /// Whether the async API can access the database
    pub async_accessible: bool,
    /// Node count from sync API
    pub sync_node_count: u64,
    /// Node count from async API
    pub async_node_count: u64,
    /// Edge count from sync API
    pub sync_edge_count: u64,
    /// Edge count from async API
    pub async_edge_count: u64,
    /// Whether the APIs are compatible (counts match)
    pub compatible: bool,
}

/// Snapshot of database state for migration checkpoints
#[derive(Debug, Clone)]
pub struct MigrationCheckpoint {
    /// When the checkpoint was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Number of nodes at checkpoint
    pub node_count: u64,
    /// Number of edges at checkpoint
    pub edge_count: u64,
    /// Number of sessions at checkpoint
    pub session_count: u64,
    /// Storage size in bytes at checkpoint
    pub storage_bytes: u64,
}

/// Result of verifying a migration checkpoint
#[derive(Debug, Clone)]
pub struct CheckpointVerification {
    /// Whether the checkpoint is valid (data not lost)
    pub valid: bool,
    /// Whether node count matches exactly
    pub node_count_match: bool,
    /// Whether edge count matches exactly
    pub edge_count_match: bool,
    /// Number of nodes added since checkpoint
    pub nodes_added: u64,
    /// Number of edges added since checkpoint
    pub edges_added: u64,
}

/// Report from running a complete migration test
#[derive(Debug, Clone)]
pub struct MigrationTestReport {
    /// Whether the migration test succeeded
    pub success: bool,
    /// Steps that were completed successfully
    pub steps_completed: Vec<String>,
    /// Errors encountered during the test
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_verify_compatibility_empty_db() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());

        let report = MigrationHelper::verify_compatibility(&config).await.unwrap();

        assert!(report.sync_accessible);
        assert!(report.async_accessible);
        assert!(report.compatible);
        assert_eq!(report.sync_node_count, 0);
        assert_eq!(report.async_node_count, 0);
    }

    #[tokio::test]
    async fn test_verify_compatibility_with_data() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());

        // Create data with sync API
        {
            let graph = MemoryGraph::open(config.clone()).unwrap();
            graph.create_session().unwrap();
        }

        let report = MigrationHelper::verify_compatibility(&config).await.unwrap();

        assert!(report.sync_accessible);
        assert!(report.async_accessible);
        assert!(report.compatible);
        assert_eq!(report.sync_node_count, 1);
        assert_eq!(report.async_node_count, 1);
    }

    #[tokio::test]
    async fn test_create_checkpoint() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());

        // Create some data
        {
            let graph = AsyncMemoryGraph::open(config.clone()).await.unwrap();
            graph.create_session().await.unwrap();
        }

        let checkpoint = MigrationHelper::create_checkpoint(&config).await.unwrap();

        assert_eq!(checkpoint.node_count, 1);
        assert_eq!(checkpoint.edge_count, 0);
    }

    #[tokio::test]
    async fn test_verify_checkpoint() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());

        // Create checkpoint with initial data
        {
            let graph = AsyncMemoryGraph::open(config.clone()).await.unwrap();
            graph.create_session().await.unwrap();
        }

        let checkpoint = MigrationHelper::create_checkpoint(&config).await.unwrap();

        // Add more data
        {
            let graph = AsyncMemoryGraph::open(config.clone()).await.unwrap();
            graph.create_session().await.unwrap();
        }

        let verification = MigrationHelper::verify_checkpoint(&config, &checkpoint)
            .await
            .unwrap();

        assert!(verification.valid);
        assert!(!verification.node_count_match); // We added more nodes
        assert_eq!(verification.nodes_added, 1);
    }

    #[tokio::test]
    async fn test_run_migration_test() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());

        // Create initial data
        {
            let graph = MemoryGraph::open(config.clone()).unwrap();
            graph.create_session().unwrap();
        }

        let report = MigrationHelper::run_migration_test(&config).await.unwrap();

        assert!(report.success, "Migration test should succeed");
        assert!(report.errors.is_empty(), "Should have no errors");
        assert!(report.steps_completed.len() >= 4, "Should complete all steps");
    }

    #[tokio::test]
    async fn test_sync_async_interop() {
        let dir = tempdir().unwrap();
        let config = Config::new(dir.path());

        // Create session with sync API
        let session_id = {
            let graph = MemoryGraph::open(config.clone()).unwrap();
            let session = graph.create_session().unwrap();
            session.id
        };

        // Read session with async API
        {
            let graph = AsyncMemoryGraph::open(config.clone()).await.unwrap();
            let session = graph.get_session(session_id).await.unwrap();
            assert_eq!(session.id, session_id);
        }

        // Add prompt with async API
        let prompt_id = {
            let graph = AsyncMemoryGraph::open(config.clone()).await.unwrap();
            graph
                .add_prompt(session_id, "Test prompt".to_string(), None)
                .await
                .unwrap()
        };

        // Read with sync API
        {
            let graph = MemoryGraph::open(config.clone()).unwrap();
            let node = graph.get_node(prompt_id).unwrap();
            assert_eq!(node.id(), prompt_id);
        }
    }
}

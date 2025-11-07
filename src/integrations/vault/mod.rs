//! Data-Vault integration module
//!
//! Provides client functionality for integrating with the Data-Vault service,
//! including archival operations, retention policies, and automatic scheduling.

pub mod archiver;
pub mod retention;

pub use archiver::{
    ArchiveEntry, ArchiveFailure, ArchiveResponse, BatchArchiveResponse, ComplianceLevel,
    RetentionPolicy, VaultClient, VaultConfig,
};
pub use retention::{ArchivalScheduler, ArchivalStats, RetentionPolicyManager, SchedulerConfig};

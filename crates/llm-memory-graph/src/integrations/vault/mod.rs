//! Data-Vault integration module
//!
//! Provides client functionality for integrating with the Data-Vault service,
//! including archival operations, retention policies, automatic scheduling,
//! dual-storage capabilities, and PII anonymization.

pub mod anonymizer;
pub mod archiver;
pub mod config;
pub mod retention;
pub mod storage_adapter;

pub use anonymizer::{AnonymizationResult, DataAnonymizer, PiiDetection};
pub use archiver::{
    ArchiveEntry, ArchiveFailure, ArchiveResponse, BatchArchiveResponse, ComplianceLevel,
    RetentionPolicy, VaultClient, VaultConfig,
};
pub use config::{
    AnonymizationConfig, AnonymizationStrategy, ArchivalMode, ArchivalPolicy, CustomPattern,
    EncryptionAlgorithm, EncryptionConfig, PerformanceConfig, PiiType, StorageMode,
    VaultStorageConfig,
};
pub use retention::{ArchivalScheduler, ArchivalStats, RetentionPolicyManager, SchedulerConfig};
pub use storage_adapter::{AdapterStats, VaultStorageAdapter};

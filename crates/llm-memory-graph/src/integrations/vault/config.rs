//! Vault storage configuration for dual-storage integration
//!
//! Provides configuration options for vault storage adapter, including
//! archival policies, anonymization settings, and dual-storage behavior.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for vault storage adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStorageConfig {
    /// Enable vault storage (if false, only Sled is used)
    pub enabled: bool,

    /// Base URL of the vault service
    pub vault_url: String,

    /// API key for vault authentication
    pub api_key: String,

    /// Storage mode controlling dual-write behavior
    pub storage_mode: StorageMode,

    /// Archival policy configuration
    pub archival_policy: ArchivalPolicy,

    /// Anonymization settings
    pub anonymization: AnonymizationConfig,

    /// Performance and reliability settings
    pub performance: PerformanceConfig,

    /// Encryption settings
    pub encryption: EncryptionConfig,
}

impl VaultStorageConfig {
    /// Create a new vault storage configuration
    pub fn new(vault_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            enabled: true,
            vault_url: vault_url.into(),
            api_key: api_key.into(),
            storage_mode: StorageMode::default(),
            archival_policy: ArchivalPolicy::default(),
            anonymization: AnonymizationConfig::default(),
            performance: PerformanceConfig::default(),
            encryption: EncryptionConfig::default(),
        }
    }

    /// Disable vault storage (Sled-only mode)
    pub fn with_vault_disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Set storage mode
    pub fn with_storage_mode(mut self, mode: StorageMode) -> Self {
        self.storage_mode = mode;
        self
    }

    /// Set archival policy
    pub fn with_archival_policy(mut self, policy: ArchivalPolicy) -> Self {
        self.archival_policy = policy;
        self
    }

    /// Set anonymization config
    pub fn with_anonymization(mut self, config: AnonymizationConfig) -> Self {
        self.anonymization = config;
        self
    }

    /// Set performance config
    pub fn with_performance(mut self, config: PerformanceConfig) -> Self {
        self.performance = config;
        self
    }
}

impl Default for VaultStorageConfig {
    fn default() -> Self {
        Self {
            enabled: std::env::var("VAULT_STORAGE_ENABLED")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
            vault_url: std::env::var("VAULT_URL")
                .unwrap_or_else(|_| "http://localhost:9000".to_string()),
            api_key: std::env::var("VAULT_API_KEY").unwrap_or_default(),
            storage_mode: StorageMode::default(),
            archival_policy: ArchivalPolicy::default(),
            anonymization: AnonymizationConfig::default(),
            performance: PerformanceConfig::default(),
            encryption: EncryptionConfig::default(),
        }
    }
}

/// Storage mode controlling dual-write behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageMode {
    /// Write to Sled only (Vault disabled)
    SledOnly,

    /// Write to both Sled and Vault synchronously
    /// Writes succeed only if both succeed
    DualSync,

    /// Write to Sled first, then Vault asynchronously
    /// Primary write succeeds immediately, vault write is fire-and-forget
    DualAsync,

    /// Write to Sled, archive to Vault based on policy
    /// Archives happen on schedule or trigger (e.g., session end)
    ArchiveOnPolicy,
}

impl Default for StorageMode {
    fn default() -> Self {
        StorageMode::ArchiveOnPolicy
    }
}

/// Archival policy determining when data is archived to vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalPolicy {
    /// Archive mode determining when archival occurs
    pub mode: ArchivalMode,

    /// Retention period in days
    pub retention_days: i64,

    /// Auto-delete from Sled after archival to vault
    pub auto_delete_from_sled: bool,

    /// Grace period before deleting from Sled (in days)
    pub sled_retention_days: Option<i64>,

    /// Batch size for batch archival operations
    pub batch_size: usize,

    /// Tags to apply to archived data
    pub archive_tags: Vec<String>,
}

impl Default for ArchivalPolicy {
    fn default() -> Self {
        Self {
            mode: ArchivalMode::OnSessionEnd,
            retention_days: 365, // 1 year default
            auto_delete_from_sled: false,
            sled_retention_days: Some(30), // Keep in Sled for 30 days
            batch_size: 100,
            archive_tags: vec!["memory-graph".to_string()],
        }
    }
}

/// Archival mode determining when archival occurs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchivalMode {
    /// Archive immediately on write
    Immediate,

    /// Archive when session ends or is marked complete
    OnSessionEnd,

    /// Archive on a scheduled basis (e.g., nightly)
    Scheduled,

    /// Archive based on age threshold (e.g., after 7 days)
    AgeThreshold,

    /// Manual archival only (via explicit API call)
    Manual,
}

/// Anonymization configuration for PII handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizationConfig {
    /// Enable PII detection and anonymization
    pub enabled: bool,

    /// PII types to detect and anonymize
    pub pii_types: Vec<PiiType>,

    /// Anonymization strategy
    pub strategy: AnonymizationStrategy,

    /// Preserve format when anonymizing (e.g., keep email structure)
    pub preserve_format: bool,

    /// Custom regex patterns for additional PII detection
    pub custom_patterns: Vec<CustomPattern>,
}

impl Default for AnonymizationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pii_types: vec![
                PiiType::Email,
                PiiType::PhoneNumber,
                PiiType::CreditCard,
                PiiType::SocialSecurity,
                PiiType::IpAddress,
            ],
            strategy: AnonymizationStrategy::Hash,
            preserve_format: true,
            custom_patterns: Vec::new(),
        }
    }
}

/// Types of personally identifiable information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiiType {
    /// Email addresses
    Email,
    /// Phone numbers
    PhoneNumber,
    /// Credit card numbers
    CreditCard,
    /// Social Security Numbers
    SocialSecurity,
    /// IP addresses
    IpAddress,
    /// Physical addresses
    PhysicalAddress,
    /// Person names
    PersonName,
    /// Date of birth
    DateOfBirth,
    /// API keys and tokens
    ApiKey,
}

/// Strategy for anonymizing PII
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnonymizationStrategy {
    /// Replace with fixed placeholder
    Redact,

    /// Hash using SHA-256 (deterministic)
    Hash,

    /// Replace with random value preserving format
    Randomize,

    /// Encrypt (reversible with key)
    Encrypt,

    /// Tokenize with mapping table
    Tokenize,
}

/// Custom regex pattern for PII detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPattern {
    /// Pattern name/description
    pub name: String,

    /// Regex pattern to match
    pub pattern: String,

    /// Replacement strategy
    pub strategy: AnonymizationStrategy,
}

/// Performance and reliability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Timeout for vault operations (in seconds)
    pub timeout_secs: u64,

    /// Enable retry on vault failures
    pub retry_enabled: bool,

    /// Maximum retry attempts
    pub max_retries: u32,

    /// Retry backoff multiplier
    pub retry_backoff_ms: u64,

    /// Graceful degradation: continue on vault failures
    pub graceful_degradation: bool,

    /// Queue failed writes for later retry
    pub queue_failed_writes: bool,

    /// Maximum queue size for failed writes
    pub max_queue_size: usize,

    /// Enable connection pooling
    pub connection_pooling: bool,

    /// Maximum concurrent vault operations
    pub max_concurrent_ops: usize,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            retry_enabled: true,
            max_retries: 3,
            retry_backoff_ms: 100,
            graceful_degradation: true,
            queue_failed_writes: true,
            max_queue_size: 10000,
            connection_pooling: true,
            max_concurrent_ops: 10,
        }
    }
}

/// Encryption configuration for vault storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable encryption for data at rest
    pub enabled: bool,

    /// Enable compression before encryption
    pub compression_enabled: bool,

    /// Encryption algorithm
    pub algorithm: EncryptionAlgorithm,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            compression_enabled: true,
            algorithm: EncryptionAlgorithm::Aes256Gcm,
        }
    }
}

/// Encryption algorithms supported by vault
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionAlgorithm {
    /// AES-256 in GCM mode (recommended)
    Aes256Gcm,

    /// ChaCha20-Poly1305
    ChaCha20Poly1305,

    /// No encryption (not recommended for production)
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_config_builder() {
        let config = VaultStorageConfig::new("http://vault:9000", "test-key")
            .with_storage_mode(StorageMode::DualAsync)
            .with_anonymization(AnonymizationConfig {
                enabled: true,
                pii_types: vec![PiiType::Email],
                ..Default::default()
            });

        assert_eq!(config.vault_url, "http://vault:9000");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.storage_mode, StorageMode::DualAsync);
        assert!(config.anonymization.enabled);
    }

    #[test]
    fn test_archival_policy_defaults() {
        let policy = ArchivalPolicy::default();
        assert_eq!(policy.retention_days, 365);
        assert_eq!(policy.mode, ArchivalMode::OnSessionEnd);
        assert!(!policy.auto_delete_from_sled);
    }

    #[test]
    fn test_storage_mode_serialization() {
        let mode = StorageMode::DualSync;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"dual_sync\"");

        let deserialized: StorageMode = serde_json::from_str("\"archive_on_policy\"").unwrap();
        assert_eq!(deserialized, StorageMode::ArchiveOnPolicy);
    }

    #[test]
    fn test_anonymization_config_defaults() {
        let config = AnonymizationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.strategy, AnonymizationStrategy::Hash);
        assert!(config.preserve_format);
        assert!(config.pii_types.contains(&PiiType::Email));
    }

    #[test]
    fn test_performance_config() {
        let config = PerformanceConfig::default();
        assert!(config.graceful_degradation);
        assert!(config.retry_enabled);
        assert_eq!(config.max_retries, 3);
    }
}

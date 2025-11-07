//! Data-Vault integration for secure archival and compliance

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use crate::integrations::{retry_request, IntegrationError, RetryPolicy};

/// Vault client configuration
#[derive(Debug, Clone)]
pub struct VaultConfig {
    /// Base URL of the Data-Vault service
    pub base_url: String,
    /// API key for authentication (required)
    pub api_key: String,
    /// Enable encryption for archived data
    pub encryption_enabled: bool,
    /// Enable compression for archived data
    pub compression_enabled: bool,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Batch size for bulk operations
    pub batch_size: usize,
    /// Enable request/response logging
    pub enable_logging: bool,
}

impl VaultConfig {
    /// Create a new vault configuration
    ///
    /// # Panics
    /// Panics if `VAULT_API_KEY` environment variable is not set when using default.
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            encryption_enabled: true,
            compression_enabled: true,
            timeout_secs: 60,
            batch_size: 100,
            enable_logging: true,
        }
    }

    /// Set encryption enabled
    pub fn with_encryption(mut self, enabled: bool) -> Self {
        self.encryption_enabled = enabled;
        self
    }

    /// Set compression enabled
    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.compression_enabled = enabled;
        self
    }

    /// Set timeout in seconds
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Set batch size
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("VAULT_URL")
                .unwrap_or_else(|_| "http://localhost:9000".to_string()),
            api_key: std::env::var("VAULT_API_KEY")
                .expect("VAULT_API_KEY environment variable must be set"),
            encryption_enabled: true,
            compression_enabled: true,
            timeout_secs: 60,
            batch_size: 100,
            enable_logging: true,
        }
    }
}

/// Archive entry for storing in the vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    /// Unique archive ID
    pub id: String,
    /// Session ID from memory graph
    pub session_id: String,
    /// Archived data payload
    pub data: serde_json::Value,
    /// When the data was archived
    pub archived_at: DateTime<Utc>,
    /// Retention period in days
    pub retention_days: i64,
    /// Tags for categorization and search
    #[serde(default)]
    pub tags: Vec<String>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ArchiveEntry {
    /// Create a new archive entry
    pub fn new(
        session_id: impl Into<String>,
        data: serde_json::Value,
        retention_days: i64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            data,
            archived_at: Utc::now(),
            retention_days,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a tag to the archive entry
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add metadata to the archive entry
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Compliance level for data retention
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComplianceLevel {
    /// Standard compliance (no special requirements)
    Standard,
    /// HIPAA compliance for healthcare data
    Hipaa,
    /// GDPR compliance for EU personal data
    Gdpr,
    /// PCI-DSS compliance for payment card data
    Pci,
    /// SOC 2 compliance
    Soc2,
}

/// Retention policy for archived data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Unique policy ID
    pub policy_id: String,
    /// Policy name
    pub name: String,
    /// Retention period in days
    pub retention_days: i64,
    /// Automatically delete data after retention period
    pub auto_delete: bool,
    /// Compliance level
    pub compliance_level: ComplianceLevel,
    /// Policy description
    #[serde(default)]
    pub description: String,
    /// Policy creation time
    pub created_at: DateTime<Utc>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

impl RetentionPolicy {
    /// Create a new retention policy
    pub fn new(
        name: impl Into<String>,
        retention_days: i64,
        compliance_level: ComplianceLevel,
    ) -> Self {
        Self {
            policy_id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            retention_days,
            auto_delete: false,
            compliance_level,
            description: String::new(),
            created_at: Utc::now(),
            tags: Vec::new(),
        }
    }

    /// Enable auto-deletion
    pub fn with_auto_delete(mut self, auto_delete: bool) -> Self {
        self.auto_delete = auto_delete;
        self
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Archive response from vault
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveResponse {
    /// Archive ID
    pub archive_id: String,
    /// Session ID
    pub session_id: String,
    /// Archive status
    pub status: String,
    /// Archive timestamp
    pub archived_at: DateTime<Utc>,
}

/// Batch archive response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchArchiveResponse {
    /// Successfully archived IDs
    pub archived: Vec<String>,
    /// Failed archives with errors
    #[serde(default)]
    pub failed: Vec<ArchiveFailure>,
    /// Total count
    pub total: usize,
    /// Success count
    pub success_count: usize,
}

/// Archive failure information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveFailure {
    /// Session ID that failed
    pub session_id: String,
    /// Error message
    pub error: String,
}

/// Data-Vault client for archival operations
pub struct VaultClient {
    config: VaultConfig,
    client: Client,
    retry_policy: RetryPolicy,
}

impl VaultClient {
    /// Create a new Vault client
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be created.
    pub fn new(config: VaultConfig) -> Result<Self, IntegrationError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .build()
            .map_err(|e| IntegrationError::HttpError(e.to_string()))?;

        let retry_policy = RetryPolicy::new()
            .with_max_attempts(3)
            .with_initial_delay(Duration::from_millis(200));

        Ok(Self {
            config,
            client,
            retry_policy,
        })
    }

    /// Create a client with custom retry policy
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Archive a session to the vault
    ///
    /// # Errors
    /// Returns an error if the archival request fails.
    pub async fn archive_session(
        &self,
        entry: ArchiveEntry,
    ) -> Result<ArchiveResponse, IntegrationError> {
        let url = format!("{}/api/v1/archive", self.config.base_url);

        if self.config.enable_logging {
            info!("Archiving session {} to vault", entry.session_id);
        }

        let operation = || async {
            let mut request = self
                .client
                .post(&url)
                .bearer_auth(&self.config.api_key)
                .header(
                    "X-Encryption-Enabled",
                    self.config.encryption_enabled.to_string(),
                )
                .header(
                    "X-Compression-Enabled",
                    self.config.compression_enabled.to_string(),
                )
                .json(&entry);

            let response = request.send().await?;
            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                error!("Failed to archive session: {} - {}", status, error_body);
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let archive_response: ArchiveResponse = response.json().await?;
            Ok(archive_response)
        };

        let result = retry_request(&self.retry_policy, operation).await?;

        if self.config.enable_logging {
            info!(
                "Session {} archived successfully with ID {}",
                entry.session_id, result.archive_id
            );
        }

        Ok(result)
    }

    /// Batch archive multiple sessions
    ///
    /// # Errors
    /// Returns an error if the batch archival request fails.
    pub async fn batch_archive(
        &self,
        entries: Vec<ArchiveEntry>,
    ) -> Result<BatchArchiveResponse, IntegrationError> {
        let url = format!("{}/api/v1/archive/batch", self.config.base_url);

        if self.config.enable_logging {
            info!("Batch archiving {} sessions", entries.len());
        }

        let operation = || async {
            let response = self
                .client
                .post(&url)
                .bearer_auth(&self.config.api_key)
                .header(
                    "X-Encryption-Enabled",
                    self.config.encryption_enabled.to_string(),
                )
                .header(
                    "X-Compression-Enabled",
                    self.config.compression_enabled.to_string(),
                )
                .json(&entries)
                .send()
                .await?;

            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                error!("Failed to batch archive: {} - {}", status, error_body);
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let batch_response: BatchArchiveResponse = response.json().await?;
            Ok(batch_response)
        };

        let result = retry_request(&self.retry_policy, operation).await?;

        if self.config.enable_logging {
            info!(
                "Batch archived {} of {} sessions successfully",
                result.success_count, result.total
            );
        }

        Ok(result)
    }

    /// Retrieve an archived session
    ///
    /// # Errors
    /// Returns an error if the archive is not found or the request fails.
    pub async fn retrieve_session(
        &self,
        archive_id: &str,
    ) -> Result<ArchiveEntry, IntegrationError> {
        let url = format!("{}/api/v1/archive/{}", self.config.base_url, archive_id);

        if self.config.enable_logging {
            debug!("Retrieving archive: {}", archive_id);
        }

        let operation = || async {
            let response = self
                .client
                .get(&url)
                .bearer_auth(&self.config.api_key)
                .send()
                .await?;

            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                error!("Failed to retrieve archive: {} - {}", status, error_body);
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let entry: ArchiveEntry = response.json().await?;
            Ok(entry)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Delete an archived session
    ///
    /// # Errors
    /// Returns an error if the deletion request fails.
    pub async fn delete_archive(&self, archive_id: &str) -> Result<(), IntegrationError> {
        let url = format!("{}/api/v1/archive/{}", self.config.base_url, archive_id);

        if self.config.enable_logging {
            info!("Deleting archive: {}", archive_id);
        }

        let operation = || async {
            let response = self
                .client
                .delete(&url)
                .bearer_auth(&self.config.api_key)
                .send()
                .await?;

            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                error!("Failed to delete archive: {} - {}", status, error_body);
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            Ok(())
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Create a retention policy
    ///
    /// # Errors
    /// Returns an error if the policy creation request fails.
    pub async fn create_retention_policy(
        &self,
        policy: RetentionPolicy,
    ) -> Result<String, IntegrationError> {
        let url = format!("{}/api/v1/policies", self.config.base_url);

        if self.config.enable_logging {
            info!("Creating retention policy: {}", policy.name);
        }

        let operation = || async {
            let response = self
                .client
                .post(&url)
                .bearer_auth(&self.config.api_key)
                .json(&policy)
                .send()
                .await?;

            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                error!("Failed to create retention policy: {} - {}", status, error_body);
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            let response_json: serde_json::Value = response.json().await?;
            let policy_id = response_json
                .get("policy_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| IntegrationError::Serialization("Missing policy_id".to_string()))?
                .to_string();

            Ok(policy_id)
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Apply a retention policy to an archive
    ///
    /// # Errors
    /// Returns an error if the policy application fails.
    pub async fn apply_retention_policy(
        &self,
        archive_id: &str,
        policy_id: &str,
    ) -> Result<(), IntegrationError> {
        let url = format!(
            "{}/api/v1/archive/{}/policy",
            self.config.base_url, archive_id
        );

        if self.config.enable_logging {
            debug!("Applying policy {} to archive {}", policy_id, archive_id);
        }

        let operation = || async {
            let response = self
                .client
                .put(&url)
                .bearer_auth(&self.config.api_key)
                .json(&serde_json::json!({
                    "policy_id": policy_id
                }))
                .send()
                .await?;

            let status = response.status();

            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                error!("Failed to apply retention policy: {} - {}", status, error_body);
                return Err(IntegrationError::ApiError {
                    status: status.as_u16(),
                    message: error_body,
                });
            }

            Ok(())
        };

        retry_request(&self.retry_policy, operation).await
    }

    /// Health check for the vault service
    ///
    /// # Errors
    /// Returns an error if the health check fails.
    pub async fn health_check(&self) -> Result<bool, IntegrationError> {
        let url = format!("{}/health", self.config.base_url);

        let response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_config_builder() {
        let config = VaultConfig::new("http://localhost:9000", "test-key")
            .with_encryption(false)
            .with_compression(true)
            .with_timeout(120)
            .with_batch_size(50);

        assert_eq!(config.base_url, "http://localhost:9000");
        assert_eq!(config.api_key, "test-key");
        assert!(!config.encryption_enabled);
        assert!(config.compression_enabled);
        assert_eq!(config.timeout_secs, 120);
        assert_eq!(config.batch_size, 50);
    }

    #[test]
    fn test_archive_entry_builder() {
        let data = serde_json::json!({"key": "value"});
        let entry = ArchiveEntry::new("session-123", data, 365)
            .with_tag("production")
            .with_metadata("user_id", serde_json::json!("user-456"));

        assert_eq!(entry.session_id, "session-123");
        assert_eq!(entry.retention_days, 365);
        assert_eq!(entry.tags.len(), 1);
        assert!(entry.metadata.contains_key("user_id"));
    }

    #[test]
    fn test_retention_policy_builder() {
        let policy = RetentionPolicy::new("HIPAA Compliance", 2555, ComplianceLevel::Hipaa)
            .with_auto_delete(true)
            .with_description("7-year retention for HIPAA compliance")
            .with_tag("healthcare");

        assert_eq!(policy.name, "HIPAA Compliance");
        assert_eq!(policy.retention_days, 2555);
        assert_eq!(policy.compliance_level, ComplianceLevel::Hipaa);
        assert!(policy.auto_delete);
        assert_eq!(policy.tags.len(), 1);
    }

    #[test]
    fn test_compliance_level_serialization() {
        let level = ComplianceLevel::Gdpr;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"gdpr\"");

        let level: ComplianceLevel = serde_json::from_str("\"hipaa\"").unwrap();
        assert_eq!(level, ComplianceLevel::Hipaa);
    }
}

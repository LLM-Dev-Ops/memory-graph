//! Retention policy and automatic archival scheduler

use super::archiver::{ArchiveEntry, ComplianceLevel, RetentionPolicy, VaultClient};
use crate::engine::AsyncMemoryGraph;
use crate::integrations::IntegrationError;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Scheduler configuration for automatic archival
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Interval between archival runs (in hours)
    pub interval_hours: u64,
    /// Archive sessions older than this many days
    pub archive_after_days: i64,
    /// Default retention period for archived sessions (in days)
    pub retention_days: i64,
    /// Maximum number of sessions to archive per batch
    pub batch_size: usize,
    /// Default compliance level
    pub default_compliance_level: ComplianceLevel,
    /// Enable automatic archival
    pub enabled: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            interval_hours: 24,
            archive_after_days: 30,
            retention_days: 365,
            batch_size: 100,
            default_compliance_level: ComplianceLevel::Standard,
            enabled: true,
        }
    }
}

impl SchedulerConfig {
    /// Create a new scheduler configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the archival interval in hours
    pub fn with_interval_hours(mut self, hours: u64) -> Self {
        self.interval_hours = hours;
        self
    }

    /// Set the age threshold for archival in days
    pub fn with_archive_after_days(mut self, days: i64) -> Self {
        self.archive_after_days = days;
        self
    }

    /// Set the retention period in days
    pub fn with_retention_days(mut self, days: i64) -> Self {
        self.retention_days = days;
        self
    }

    /// Set the batch size
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Set the default compliance level
    pub fn with_compliance_level(mut self, level: ComplianceLevel) -> Self {
        self.default_compliance_level = level;
        self
    }

    /// Enable or disable the scheduler
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Archival statistics
#[derive(Debug, Clone, Default)]
pub struct ArchivalStats {
    /// Total sessions processed
    pub total_processed: usize,
    /// Successfully archived
    pub archived_count: usize,
    /// Failed to archive
    pub failed_count: usize,
    /// Skipped sessions
    pub skipped_count: usize,
    /// Total time taken in milliseconds
    pub duration_ms: u64,
}

impl ArchivalStats {
    /// Create a new stats instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_processed == 0 {
            0.0
        } else {
            (self.archived_count as f64 / self.total_processed as f64) * 100.0
        }
    }
}

/// Automatic archival scheduler
///
/// Periodically archives old sessions based on configured retention policies.
pub struct ArchivalScheduler {
    vault_client: Arc<VaultClient>,
    graph: Arc<AsyncMemoryGraph>,
    config: SchedulerConfig,
    running: Arc<tokio::sync::RwLock<bool>>,
}

impl ArchivalScheduler {
    /// Create a new archival scheduler
    pub fn new(
        vault_client: VaultClient,
        graph: Arc<AsyncMemoryGraph>,
        config: SchedulerConfig,
    ) -> Self {
        Self {
            vault_client: Arc::new(vault_client),
            graph,
            config,
            running: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }

    /// Check if the scheduler is currently running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Start the archival scheduler
    ///
    /// Returns a join handle that can be used to stop the scheduler.
    pub async fn start(&self) -> JoinHandle<()> {
        if !self.config.enabled {
            info!("Archival scheduler is disabled");
            return tokio::spawn(async {});
        }

        let vault = Arc::clone(&self.vault_client);
        let graph = Arc::clone(&self.graph);
        let config = self.config.clone();
        let running = Arc::clone(&self.running);

        info!(
            "Starting archival scheduler (interval: {}h, archive after: {} days)",
            config.interval_hours, config.archive_after_days
        );

        tokio::spawn(async move {
            *running.write().await = true;
            let mut interval =
                tokio::time::interval(Duration::from_secs(config.interval_hours * 3600));

            loop {
                interval.tick().await;

                if !*running.read().await {
                    info!("Archival scheduler stopped");
                    break;
                }

                info!("Running archival scheduler iteration");
                let start = std::time::Instant::now();

                match Self::run_archival(&vault, &graph, &config).await {
                    Ok(stats) => {
                        let duration = start.elapsed();
                        info!(
                            "Archival iteration complete: {} processed, {} archived, {} failed, {} skipped (took {}ms, {:.2}% success)",
                            stats.total_processed,
                            stats.archived_count,
                            stats.failed_count,
                            stats.skipped_count,
                            duration.as_millis(),
                            stats.success_rate()
                        );
                    }
                    Err(e) => {
                        error!("Archival iteration failed: {}", e);
                    }
                }
            }
        })
    }

    /// Stop the scheduler
    pub async fn stop(&self) {
        info!("Stopping archival scheduler");
        *self.running.write().await = false;
    }

    /// Run a single archival iteration
    async fn run_archival(
        vault: &VaultClient,
        graph: &AsyncMemoryGraph,
        config: &SchedulerConfig,
    ) -> Result<ArchivalStats, IntegrationError> {
        let mut stats = ArchivalStats::new();
        let start_time = std::time::Instant::now();

        // Calculate cutoff date for archival
        let cutoff_date = Utc::now() - ChronoDuration::days(config.archive_after_days);
        debug!("Archiving sessions older than {}", cutoff_date);

        // Query for old sessions
        // Note: This is a placeholder. The actual implementation would need
        // to query the graph for sessions older than the cutoff date.
        // For now, we'll return empty stats.

        // TODO: Implement session querying by age when query API supports it
        // Example:
        // let old_sessions = graph
        //     .query()
        //     .session_older_than(cutoff_date)
        //     .limit(config.batch_size)
        //     .execute()
        //     .await?;

        warn!("Session age-based querying not yet implemented in graph query API");

        stats.duration_ms = start_time.elapsed().as_millis() as u64;
        Ok(stats)
    }

    /// Archive a specific session manually
    ///
    /// # Errors
    /// Returns an error if the archival fails.
    pub async fn archive_session_now(
        &self,
        session_id: &str,
        session_data: serde_json::Value,
    ) -> Result<String, IntegrationError> {
        info!("Manually archiving session: {}", session_id);

        let entry = ArchiveEntry::new(
            session_id,
            session_data,
            self.config.retention_days,
        )
        .with_tag("manual")
        .with_metadata(
            "archived_by",
            serde_json::json!("archival_scheduler"),
        );

        let response = self.vault_client.archive_session(entry).await?;
        Ok(response.archive_id)
    }

    /// Create a retention policy for a specific compliance level
    ///
    /// # Errors
    /// Returns an error if policy creation fails.
    pub async fn create_compliance_policy(
        &self,
        name: impl Into<String>,
        compliance_level: ComplianceLevel,
    ) -> Result<String, IntegrationError> {
        let retention_days = match compliance_level {
            ComplianceLevel::Standard => 365,
            ComplianceLevel::Hipaa => 2555, // 7 years
            ComplianceLevel::Gdpr => 2555,  // 7 years
            ComplianceLevel::Pci => 1095,   // 3 years
            ComplianceLevel::Soc2 => 2555,  // 7 years
        };

        let policy = RetentionPolicy::new(name, retention_days, compliance_level)
            .with_auto_delete(false)
            .with_tag(format!("{:?}", compliance_level).to_lowercase());

        self.vault_client.create_retention_policy(policy).await
    }

    /// Batch archive multiple sessions
    ///
    /// # Errors
    /// Returns an error if the batch archival fails.
    pub async fn batch_archive_sessions(
        &self,
        sessions: Vec<(String, serde_json::Value)>,
    ) -> Result<ArchivalStats, IntegrationError> {
        let mut stats = ArchivalStats::new();
        stats.total_processed = sessions.len();

        let entries: Vec<ArchiveEntry> = sessions
            .into_iter()
            .map(|(session_id, data)| {
                ArchiveEntry::new(session_id, data, self.config.retention_days)
                    .with_tag("batch")
            })
            .collect();

        let response = self.vault_client.batch_archive(entries).await?;

        stats.archived_count = response.success_count;
        stats.failed_count = response.failed.len();

        Ok(stats)
    }
}

/// Retention policy manager for creating and managing policies
pub struct RetentionPolicyManager {
    vault_client: Arc<VaultClient>,
}

impl RetentionPolicyManager {
    /// Create a new policy manager
    pub fn new(vault_client: VaultClient) -> Self {
        Self {
            vault_client: Arc::new(vault_client),
        }
    }

    /// Create a standard retention policy
    pub async fn create_standard_policy(
        &self,
        name: impl Into<String>,
        retention_days: i64,
    ) -> Result<String, IntegrationError> {
        let policy = RetentionPolicy::new(name, retention_days, ComplianceLevel::Standard);
        self.vault_client.create_retention_policy(policy).await
    }

    /// Create a HIPAA compliance policy (7 years)
    pub async fn create_hipaa_policy(
        &self,
        name: impl Into<String>,
    ) -> Result<String, IntegrationError> {
        let policy = RetentionPolicy::new(name, 2555, ComplianceLevel::Hipaa)
            .with_description("HIPAA 7-year retention requirement")
            .with_tag("healthcare")
            .with_tag("compliance");

        self.vault_client.create_retention_policy(policy).await
    }

    /// Create a GDPR compliance policy (7 years)
    pub async fn create_gdpr_policy(
        &self,
        name: impl Into<String>,
    ) -> Result<String, IntegrationError> {
        let policy = RetentionPolicy::new(name, 2555, ComplianceLevel::Gdpr)
            .with_description("GDPR 7-year retention for financial records")
            .with_tag("gdpr")
            .with_tag("compliance");

        self.vault_client.create_retention_policy(policy).await
    }

    /// Create a PCI-DSS compliance policy (3 years)
    pub async fn create_pci_policy(
        &self,
        name: impl Into<String>,
    ) -> Result<String, IntegrationError> {
        let policy = RetentionPolicy::new(name, 1095, ComplianceLevel::Pci)
            .with_description("PCI-DSS 3-year retention requirement")
            .with_tag("payment")
            .with_tag("compliance");

        self.vault_client.create_retention_policy(policy).await
    }

    /// Apply a policy to an archive
    pub async fn apply_policy(
        &self,
        archive_id: &str,
        policy_id: &str,
    ) -> Result<(), IntegrationError> {
        self.vault_client
            .apply_retention_policy(archive_id, policy_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_config_builder() {
        let config = SchedulerConfig::new()
            .with_interval_hours(12)
            .with_archive_after_days(60)
            .with_retention_days(730)
            .with_batch_size(50)
            .with_compliance_level(ComplianceLevel::Hipaa)
            .with_enabled(false);

        assert_eq!(config.interval_hours, 12);
        assert_eq!(config.archive_after_days, 60);
        assert_eq!(config.retention_days, 730);
        assert_eq!(config.batch_size, 50);
        assert_eq!(config.default_compliance_level, ComplianceLevel::Hipaa);
        assert!(!config.enabled);
    }

    #[test]
    fn test_archival_stats_success_rate() {
        let mut stats = ArchivalStats::new();
        stats.total_processed = 100;
        stats.archived_count = 95;
        stats.failed_count = 5;

        assert_eq!(stats.success_rate(), 95.0);
    }

    #[test]
    fn test_archival_stats_zero_processed() {
        let stats = ArchivalStats::new();
        assert_eq!(stats.success_rate(), 0.0);
    }
}

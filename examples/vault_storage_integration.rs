//! Example: Vault Storage Integration
//!
//! Demonstrates how to use the VaultStorageAdapter for dual-storage
//! with PII anonymization and flexible archival policies.

use llm_memory_graph::integrations::vault::{
    AnonymizationConfig, AnonymizationStrategy, ArchivalMode, ArchivalPolicy, DataAnonymizer,
    PerformanceConfig, PiiType, StorageMode, VaultStorageAdapter, VaultStorageConfig,
};
use llm_memory_graph::storage::AsyncSledBackend;
use llm_memory_graph::types::{ConversationSession, Node, PromptMetadata, PromptNode};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Vault Storage Integration Example ===\n");

    // Example 1: Sled-Only Mode (Development)
    example_sled_only().await?;

    // Example 2: Production Configuration with PII Anonymization
    example_production_config().await?;

    // Example 3: Manual Archival
    example_manual_archival().await?;

    // Example 4: PII Detection and Anonymization
    example_pii_anonymization().await?;

    // Example 5: Statistics and Monitoring
    example_statistics().await?;

    Ok(())
}

/// Example 1: Sled-Only Mode (Development)
async fn example_sled_only() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Sled-Only Mode\n");

    // Create temporary directory for this example
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("example1.db");

    // Create Sled backend
    let sled = AsyncSledBackend::open(db_path.to_str().unwrap()).await?;

    // Configure vault storage in Sled-only mode (vault disabled)
    let config = VaultStorageConfig::default().with_vault_disabled();

    // Create adapter
    let adapter = VaultStorageAdapter::new(Arc::new(sled), config).await?;

    println!("✓ Created VaultStorageAdapter in Sled-only mode");
    println!("  - Vault integration disabled");
    println!("  - All operations use Sled only");
    println!("  - Perfect for development and testing\n");

    Ok(())
}

/// Example 2: Production Configuration with PII Anonymization
async fn example_production_config() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Production Configuration\n");

    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("example2.db");
    let sled = AsyncSledBackend::open(db_path.to_str().unwrap()).await?;

    // Configure for production with compliance
    let config = VaultStorageConfig::new(
        "http://vault.example.com:9000",
        "production-api-key"
    )
    .with_storage_mode(StorageMode::ArchiveOnPolicy)
    .with_archival_policy(ArchivalPolicy {
        mode: ArchivalMode::OnSessionEnd,
        retention_days: 2555, // 7 years for HIPAA compliance
        auto_delete_from_sled: true,
        sled_retention_days: Some(90), // Keep hot data for 90 days
        batch_size: 100,
        archive_tags: vec!["production".to_string(), "hipaa".to_string()],
    })
    .with_anonymization(AnonymizationConfig {
        enabled: true,
        pii_types: vec![
            PiiType::Email,
            PiiType::PhoneNumber,
            PiiType::CreditCard,
            PiiType::SocialSecurity,
            PiiType::IpAddress,
            PiiType::ApiKey,
        ],
        strategy: AnonymizationStrategy::Hash,
        preserve_format: true,
        custom_patterns: vec![],
    })
    .with_performance(PerformanceConfig {
        timeout_secs: 60,
        retry_enabled: true,
        max_retries: 5,
        graceful_degradation: true,
        queue_failed_writes: true,
        max_queue_size: 50000,
        connection_pooling: true,
        max_concurrent_ops: 20,
        ..Default::default()
    });

    println!("Production Configuration:");
    println!("  Storage Mode: Archive-on-Policy");
    println!("  Archival Mode: On Session End");
    println!("  Retention: 7 years (HIPAA)");
    println!("  Sled Hot Storage: 90 days");
    println!("  PII Anonymization: Enabled (6 types)");
    println!("  Graceful Degradation: Enabled");
    println!("  Max Retries: 5");
    println!("  Queue Size: 50,000\n");

    // Note: This would fail to connect to vault in this example
    // In production, ensure vault is running and accessible

    Ok(())
}

/// Example 3: Manual Archival
async fn example_manual_archival() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: Manual Archival\n");

    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("example3.db");
    let sled = AsyncSledBackend::open(db_path.to_str().unwrap()).await?;

    // Configure with manual archival mode
    let config = VaultStorageConfig::new(
        "http://localhost:9000",
        "test-key"
    )
    .with_vault_disabled() // Disabled for this example
    .with_archival_policy(ArchivalPolicy {
        mode: ArchivalMode::Manual,
        retention_days: 365,
        ..Default::default()
    });

    let adapter = VaultStorageAdapter::new(Arc::new(sled), config).await?;

    // Create a test session
    let session = ConversationSession::new();
    let session_node = Node::Session(session.clone());

    // Create a prompt node
    let prompt = PromptNode {
        id: llm_memory_graph::types::NodeId::new(),
        content: "What is the weather today?".to_string(),
        metadata: PromptMetadata::default(),
        session_id: Some(session.id),
        created_at: chrono::Utc::now(),
    };
    let prompt_node = Node::Prompt(prompt);

    // Store nodes (only goes to Sled in manual mode)
    use llm_memory_graph::storage::AsyncStorageBackend;
    adapter.store_node(&session_node).await?;
    adapter.store_node(&prompt_node).await?;

    println!("✓ Stored nodes to Sled");
    println!("  Session ID: {}", session.id);
    println!("  Nodes: 2 (session + prompt)");

    // Manual archival (would archive to vault if enabled)
    match adapter.archive_session(&session.id).await {
        Ok(archive_id) => {
            println!("✓ Archived session to vault");
            println!("  Archive ID: {}", archive_id);
        }
        Err(e) => {
            println!("  Note: Archival skipped (vault disabled): {}", e);
        }
    }

    println!();

    Ok(())
}

/// Example 4: PII Detection and Anonymization
async fn example_pii_anonymization() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 4: PII Detection and Anonymization\n");

    // Create anonymizer with default config
    let config = AnonymizationConfig {
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
        custom_patterns: vec![],
    };

    let anonymizer = DataAnonymizer::new(config)?;

    // Test data with PII
    let test_cases = vec![
        "Contact me at alice@example.com or call 555-123-4567",
        "Credit card: 4532-1234-5678-9010",
        "SSN: 123-45-6789",
        "Server IP: 192.168.1.100",
        "No PII in this text",
    ];

    for (i, text) in test_cases.iter().enumerate() {
        println!("Test Case {}:", i + 1);
        println!("  Original: {}", text);

        // Detect PII
        let detections = anonymizer.detect_pii(text);
        println!("  Detected: {} PII instance(s)", detections.len());

        for detection in &detections {
            println!(
                "    - {:?} at position {}-{}: '{}'",
                detection.pii_type,
                detection.start,
                detection.end,
                detection.matched_text
            );
        }

        // Anonymize
        let anonymized = anonymizer.anonymize_string(text)?;
        println!("  Anonymized: {}", anonymized);
        println!();
    }

    // Test JSON anonymization
    let json_data = serde_json::json!({
        "user": {
            "email": "bob@example.com",
            "phone": "555-987-6543",
            "address": "123 Main St"
        },
        "transaction": {
            "card": "4111-1111-1111-1111",
            "amount": 99.99
        }
    });

    println!("JSON Anonymization:");
    println!("  Original: {}", serde_json::to_string_pretty(&json_data)?);

    let anonymized_json = anonymizer.anonymize(&json_data)?;
    println!("  Anonymized: {}", serde_json::to_string_pretty(&anonymized_json)?);
    println!();

    Ok(())
}

/// Example 5: Statistics and Monitoring
async fn example_statistics() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 5: Statistics and Monitoring\n");

    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("example5.db");
    let sled = AsyncSledBackend::open(db_path.to_str().unwrap()).await?;

    let config = VaultStorageConfig::default().with_vault_disabled();
    let adapter = VaultStorageAdapter::new(Arc::new(sled), config).await?;

    // Perform some operations
    let session = ConversationSession::new();
    let session_node = Node::Session(session.clone());

    use llm_memory_graph::storage::AsyncStorageBackend;
    adapter.store_node(&session_node).await?;

    // Get statistics
    let stats = adapter.get_stats().await;

    println!("Adapter Statistics:");
    println!("  Performance Metrics:");
    println!("    Sled writes: {}", stats.sled_writes);
    println!("    Vault writes: {}", stats.vault_writes);
    println!("    Vault failures: {}", stats.vault_failures);

    if stats.vault_writes > 0 {
        let failure_rate = (stats.vault_failures as f64 / stats.vault_writes as f64) * 100.0;
        println!("    Failure rate: {:.2}%", failure_rate);
    }

    println!("\n  Data Metrics:");
    println!("    Sessions archived: {}", stats.sessions_archived);
    println!("    Bytes archived: {}", stats.bytes_archived);
    println!("    PII instances anonymized: {}", stats.pii_anonymized);

    println!();

    Ok(())
}

# Vault Storage Integration Documentation

## Overview

The Vault Storage Integration extends the LLM Memory Graph with dual-storage capabilities, combining fast local Sled storage with secure, encrypted Data Vault archival. This implementation provides:

- **Dual-storage pattern** - Sled (primary/hot) + Vault (secondary/archive)
- **PII anonymization** - Automated detection and sanitization of sensitive data
- **Flexible archival policies** - Immediate, scheduled, or policy-based archival
- **Graceful degradation** - Continues operation if Vault unavailable
- **Compliance support** - HIPAA, GDPR, PCI-DSS, SOC2 compliance modes

## Architecture

### Components

```
┌─────────────────────────────────────────────────────┐
│          VaultStorageAdapter                        │
│  ┌───────────────────────────────────────────────┐  │
│  │  Primary: Sled (AsyncStorageBackend)         │  │
│  │  - Fast local queries                        │  │
│  │  - Hot data storage                          │  │
│  │  - Real-time access                          │  │
│  └───────────────────────────────────────────────┘  │
│                      │                              │
│                      ├──────────┐                   │
│                      ▼          ▼                   │
│  ┌──────────────────────┐  ┌──────────────────┐    │
│  │  DataAnonymizer      │  │  VaultClient     │    │
│  │  - PII detection     │  │  - Encryption    │    │
│  │  - Sanitization      │  │  - Compression   │    │
│  │  - Pattern matching  │  │  - Archival      │    │
│  └──────────────────────┘  └──────────────────┘    │
│                                    │                │
│                                    ▼                │
│                            ┌────────────────┐       │
│                            │  Data Vault    │       │
│                            │  - Long-term   │       │
│                            │  - Compliance  │       │
│                            │  - Encrypted   │       │
│                            └────────────────┘       │
└─────────────────────────────────────────────────────┘
```

### File Structure

```
crates/llm-memory-graph/src/integrations/vault/
├── mod.rs               # Module exports
├── config.rs            # VaultStorageConfig (NEW)
├── anonymizer.rs        # PII detection/anonymization (NEW)
├── storage_adapter.rs   # Dual-storage implementation (NEW)
├── archiver.rs          # Existing vault client
└── retention.rs         # Existing retention policies
```

## Usage

### Basic Setup

```rust
use llm_memory_graph::storage::AsyncSledBackend;
use llm_memory_graph::integrations::vault::{
    VaultStorageAdapter, VaultStorageConfig, StorageMode
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create primary Sled backend
    let sled = AsyncSledBackend::open("./data/graph.db").await?;

    // Configure vault storage
    let config = VaultStorageConfig::new(
        "http://vault:9000",
        "vault-api-key"
    )
    .with_storage_mode(StorageMode::ArchiveOnPolicy);

    // Create dual-storage adapter
    let adapter = VaultStorageAdapter::new(
        Arc::new(sled),
        config
    ).await?;

    // Use adapter as AsyncStorageBackend
    // adapter.store_node(...).await?;

    Ok(())
}
```

### Storage Modes

#### 1. Sled-Only (Vault Disabled)

```rust
let config = VaultStorageConfig::default()
    .with_vault_disabled();

let adapter = VaultStorageAdapter::new(sled, config).await?;
// Only writes to Sled, no vault operations
```

#### 2. Dual-Sync (Synchronous Writes)

```rust
let config = VaultStorageConfig::new(vault_url, api_key)
    .with_storage_mode(StorageMode::DualSync);

// Writes succeed only if BOTH Sled and Vault succeed
```

#### 3. Dual-Async (Asynchronous Archival)

```rust
let config = VaultStorageConfig::new(vault_url, api_key)
    .with_storage_mode(StorageMode::DualAsync);

// Writes to Sled immediately, vault write happens in background
```

#### 4. Archive-on-Policy (Recommended)

```rust
use llm_memory_graph::integrations::vault::{
    ArchivalPolicy, ArchivalMode
};

let policy = ArchivalPolicy {
    mode: ArchivalMode::OnSessionEnd,
    retention_days: 365,
    auto_delete_from_sled: false,
    sled_retention_days: Some(30),
    ..Default::default()
};

let config = VaultStorageConfig::new(vault_url, api_key)
    .with_storage_mode(StorageMode::ArchiveOnPolicy)
    .with_archival_policy(policy);

// Archives to vault when session ends
```

### PII Anonymization

#### Basic Anonymization

```rust
use llm_memory_graph::integrations::vault::{
    AnonymizationConfig, PiiType, AnonymizationStrategy
};

let anon_config = AnonymizationConfig {
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
    ..Default::default()
};

let config = VaultStorageConfig::new(vault_url, api_key)
    .with_anonymization(anon_config);
```

#### Custom PII Patterns

```rust
use llm_memory_graph::integrations::vault::CustomPattern;

let custom = CustomPattern {
    name: "Internal User ID".to_string(),
    pattern: r"USR-\d{8}".to_string(),
    strategy: AnonymizationStrategy::Hash,
};

let mut anon_config = AnonymizationConfig::default();
anon_config.custom_patterns.push(custom);
```

#### Anonymization Strategies

```rust
// 1. Redact - Replace with placeholder
AnonymizationStrategy::Redact
// "alice@example.com" → "[EMAIL_REDACTED]"

// 2. Hash - Deterministic hash
AnonymizationStrategy::Hash
// "alice@example.com" → "user_12345@redacted.com"

// 3. Randomize - Random value preserving format
AnonymizationStrategy::Randomize
// "555-123-4567" → "555-987-6543"

// 4. Encrypt - Reversible encryption
AnonymizationStrategy::Encrypt
// "alice@example.com" → "[ENCRYPTED:hash]"

// 5. Tokenize - Token mapping
AnonymizationStrategy::Tokenize
// "alice@example.com" → "[TOKEN:hash]"
```

### Archival Modes

```rust
use llm_memory_graph::integrations::vault::ArchivalMode;

// Archive immediately on write
ArchivalMode::Immediate

// Archive when session ends
ArchivalMode::OnSessionEnd

// Archive on schedule (e.g., nightly)
ArchivalMode::Scheduled

// Archive after age threshold (e.g., 7 days)
ArchivalMode::AgeThreshold

// Manual archival only
ArchivalMode::Manual
```

### Manual Archival

```rust
use llm_memory_graph::types::SessionId;

let session_id = SessionId::new();

// Archive specific session
let archive_id = adapter.archive_session(&session_id).await?;
println!("Archived to vault: {}", archive_id);

// Retrieve archived session
let archive = adapter.retrieve_archived_session(&archive_id).await?;
```

### Graceful Degradation

```rust
use llm_memory_graph::integrations::vault::PerformanceConfig;

let perf_config = PerformanceConfig {
    graceful_degradation: true,  // Continue on vault failures
    retry_enabled: true,
    max_retries: 3,
    queue_failed_writes: true,
    max_queue_size: 10000,
    ..Default::default()
};

let config = VaultStorageConfig::new(vault_url, api_key)
    .with_performance(perf_config);

// If vault is unavailable, operations continue with Sled
// Failed writes are queued for retry
```

### Statistics and Monitoring

```rust
// Get adapter statistics
let stats = adapter.get_stats().await;

println!("Sled writes: {}", stats.sled_writes);
println!("Vault writes: {}", stats.vault_writes);
println!("Vault failures: {}", stats.vault_failures);
println!("PII anonymized: {}", stats.pii_anonymized);
println!("Sessions archived: {}", stats.sessions_archived);
println!("Bytes archived: {}", stats.bytes_archived);
```

### Flushing Archival Queue

```rust
// Flush all pending archival operations
let archived_ids = adapter.flush_archival_queue().await?;
println!("Archived {} sessions", archived_ids.len());

// Flush on shutdown
adapter.flush().await?;  // Flushes both Sled and archival queue
```

## Configuration Reference

### VaultStorageConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `false` | Enable vault storage |
| `vault_url` | `String` | `http://localhost:9000` | Vault service URL |
| `api_key` | `String` | - | Vault API key |
| `storage_mode` | `StorageMode` | `ArchiveOnPolicy` | Dual-write behavior |
| `archival_policy` | `ArchivalPolicy` | - | When to archive |
| `anonymization` | `AnonymizationConfig` | - | PII handling |
| `performance` | `PerformanceConfig` | - | Timeouts, retries |
| `encryption` | `EncryptionConfig` | - | Encryption settings |

### ArchivalPolicy

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | `ArchivalMode` | `OnSessionEnd` | When to archive |
| `retention_days` | `i64` | `365` | Vault retention period |
| `auto_delete_from_sled` | `bool` | `false` | Delete from Sled after archive |
| `sled_retention_days` | `Option<i64>` | `Some(30)` | Sled retention period |
| `batch_size` | `usize` | `100` | Batch operation size |
| `archive_tags` | `Vec<String>` | `["memory-graph"]` | Tags for archives |

### AnonymizationConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Enable anonymization |
| `pii_types` | `Vec<PiiType>` | Email, Phone, CC, SSN, IP | PII to detect |
| `strategy` | `AnonymizationStrategy` | `Hash` | Anonymization method |
| `preserve_format` | `bool` | `true` | Keep format structure |
| `custom_patterns` | `Vec<CustomPattern>` | `[]` | Custom regex patterns |

### PerformanceConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `timeout_secs` | `u64` | `30` | Operation timeout |
| `retry_enabled` | `bool` | `true` | Enable retries |
| `max_retries` | `u32` | `3` | Max retry attempts |
| `retry_backoff_ms` | `u64` | `100` | Backoff multiplier |
| `graceful_degradation` | `bool` | `true` | Continue on failures |
| `queue_failed_writes` | `bool` | `true` | Queue failed writes |
| `max_queue_size` | `usize` | `10000` | Max queue entries |
| `connection_pooling` | `bool` | `true` | Enable pooling |
| `max_concurrent_ops` | `usize` | `10` | Max concurrent ops |

## Environment Variables

```bash
# Enable vault storage (default: false)
export VAULT_STORAGE_ENABLED=true

# Vault service URL
export VAULT_URL=http://vault.example.com:9000

# Vault API key
export VAULT_API_KEY=your-api-key-here
```

## Compliance Modes

### HIPAA Compliance (Healthcare)

```rust
use llm_memory_graph::integrations::vault::ComplianceLevel;

let policy = RetentionPolicy::new(
    "HIPAA Healthcare Data",
    2555,  // 7 years
    ComplianceLevel::Hipaa
)
.with_auto_delete(true);
```

### GDPR Compliance (EU Personal Data)

```rust
let policy = RetentionPolicy::new(
    "GDPR User Data",
    365,  // 1 year
    ComplianceLevel::Gdpr
)
.with_description("EU personal data retention");
```

### PCI-DSS Compliance (Payment Card Data)

```rust
let policy = RetentionPolicy::new(
    "PCI Payment Data",
    365,  // 1 year minimum
    ComplianceLevel::Pci
);
```

## Best Practices

### 1. Production Configuration

```rust
let config = VaultStorageConfig::new(vault_url, api_key)
    .with_storage_mode(StorageMode::ArchiveOnPolicy)
    .with_archival_policy(ArchivalPolicy {
        mode: ArchivalMode::OnSessionEnd,
        retention_days: 2555,  // 7 years for compliance
        auto_delete_from_sled: true,
        sled_retention_days: Some(90),  // 90 days hot storage
        ..Default::default()
    })
    .with_anonymization(AnonymizationConfig {
        enabled: true,
        strategy: AnonymizationStrategy::Hash,
        pii_types: vec![
            PiiType::Email,
            PiiType::PhoneNumber,
            PiiType::CreditCard,
            PiiType::SocialSecurity,
            PiiType::IpAddress,
            PiiType::ApiKey,
        ],
        ..Default::default()
    })
    .with_performance(PerformanceConfig {
        graceful_degradation: true,
        retry_enabled: true,
        max_retries: 5,
        timeout_secs: 60,
        ..Default::default()
    });
```

### 2. Development Configuration

```rust
let config = VaultStorageConfig::new(
    "http://localhost:9000",
    "dev-key"
)
.with_vault_disabled();  // Sled-only for development

// Or with vault enabled but lenient
let config = VaultStorageConfig::new(vault_url, api_key)
    .with_anonymization(AnonymizationConfig {
        enabled: false,  // Disable for dev/testing
        ..Default::default()
    })
    .with_performance(PerformanceConfig {
        graceful_degradation: true,
        timeout_secs: 10,
        ..Default::default()
    });
```

### 3. High-Availability Configuration

```rust
let config = VaultStorageConfig::new(vault_url, api_key)
    .with_storage_mode(StorageMode::DualAsync)
    .with_performance(PerformanceConfig {
        graceful_degradation: true,
        connection_pooling: true,
        max_concurrent_ops: 50,
        queue_failed_writes: true,
        max_queue_size: 100000,
        ..Default::default()
    });
```

### 4. Testing PII Anonymization

```rust
use llm_memory_graph::integrations::vault::DataAnonymizer;

let anonymizer = DataAnonymizer::from_vault_config(&config)?;

// Detect PII
let text = "Contact me at alice@example.com or 555-123-4567";
let detections = anonymizer.detect_pii(text);

for detection in detections {
    println!("Found {:?} at {}-{}: {}",
        detection.pii_type,
        detection.start,
        detection.end,
        detection.matched_text
    );
}

// Anonymize
let anonymized = anonymizer.anonymize_string(text)?;
println!("Anonymized: {}", anonymized);
```

## Error Handling

```rust
use llm_memory_graph::integrations::IntegrationError;

match adapter.archive_session(&session_id).await {
    Ok(archive_id) => {
        println!("Archived: {}", archive_id);
    }
    Err(IntegrationError::ConnectionError(msg)) => {
        eprintln!("Vault connection failed: {}", msg);
        // Handle gracefully
    }
    Err(IntegrationError::ApiError { status, message }) => {
        eprintln!("Vault API error {}: {}", status, message);
    }
    Err(e) => {
        eprintln!("Archival failed: {}", e);
    }
}
```

## Performance Considerations

### Memory Usage

- **Sled**: Hot data in memory-mapped files (~1GB per 1M nodes)
- **Vault queue**: Up to `max_queue_size` sessions in memory
- **Anonymizer**: Regex compilation on startup (~10KB per pattern)

### Throughput

- **Sled writes**: ~100K ops/sec (local)
- **Vault writes**: ~1K ops/sec (network bound)
- **Archival queue**: Async processing, non-blocking

### Optimization Tips

1. **Use ArchiveOnPolicy** for better performance than dual-sync
2. **Enable connection pooling** for high concurrency
3. **Batch archival operations** with appropriate `batch_size`
4. **Monitor queue depth** to prevent memory exhaustion
5. **Tune retry parameters** based on network reliability

## Migration Guide

### From Sled-only to Dual-storage

```rust
// Before: Direct Sled usage
let backend = AsyncSledBackend::open("./data/graph.db").await?;

// After: Wrapped in VaultStorageAdapter
let sled = AsyncSledBackend::open("./data/graph.db").await?;
let config = VaultStorageConfig::new(vault_url, api_key);
let adapter = VaultStorageAdapter::new(Arc::new(sled), config).await?;

// Use adapter as drop-in replacement
// All AsyncStorageBackend methods work the same
```

### Backward Compatibility

The vault integration is **100% backward compatible**:

- ✅ Existing Sled storage continues to work
- ✅ No schema changes required
- ✅ Vault is opt-in via configuration
- ✅ Can disable vault anytime with `.with_vault_disabled()`
- ✅ All existing code works unchanged

## Troubleshooting

### Vault Connection Issues

```rust
// Check vault health
if let Some(vault) = &adapter.vault_client {
    match vault.health_check().await {
        Ok(true) => println!("Vault is healthy"),
        Ok(false) => eprintln!("Vault health check failed"),
        Err(e) => eprintln!("Cannot reach vault: {}", e),
    }
}
```

### High Queue Depth

```rust
let stats = adapter.get_stats().await;
if stats.vault_failures > stats.vault_writes * 10 / 100 {
    eprintln!("Warning: >10% vault failure rate");
    // Consider:
    // 1. Check vault connectivity
    // 2. Increase timeout
    // 3. Reduce concurrent ops
    // 4. Enable graceful degradation
}
```

### PII Detection False Positives

```rust
// Customize PII detection to reduce false positives
let anon_config = AnonymizationConfig {
    enabled: true,
    pii_types: vec![
        // Only detect high-confidence PII
        PiiType::Email,
        PiiType::CreditCard,
        PiiType::SocialSecurity,
    ],
    ..Default::default()
};
```

## Examples

See the comprehensive examples in:
- `/examples/vault_storage_basic.rs` - Basic setup
- `/examples/vault_storage_advanced.rs` - Advanced configurations
- `/examples/pii_anonymization.rs` - PII handling

## API Reference

Full API documentation: `cargo doc --open`

## Security Considerations

1. **API Keys**: Store vault API keys in environment variables or secrets manager
2. **Encryption**: Always enable encryption in production (`encryption.enabled = true`)
3. **PII Compliance**: Enable anonymization for sensitive data
4. **Access Control**: Vault handles encryption and access control
5. **Audit Logging**: Vault provides comprehensive audit logs

## Future Enhancements

- [ ] Compression optimization for large sessions
- [ ] Incremental archival (delta encoding)
- [ ] Multi-region vault replication
- [ ] Advanced PII detection (ML-based)
- [ ] Automatic retention policy selection
- [ ] Real-time archival metrics dashboard
- [ ] Hot-to-cold data tier migration
- [ ] Archival search and query capabilities

## Support

For issues or questions:
- GitHub Issues: https://github.com/globalbusinessadvisors/llm-memory-graph/issues
- Documentation: https://docs.rs/llm-memory-graph

## License

MIT OR Apache-2.0

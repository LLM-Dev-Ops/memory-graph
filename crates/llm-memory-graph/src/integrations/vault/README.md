# Data Vault Integration

Secure, encrypted storage integration for LLM Memory Graph with dual-storage capabilities, PII anonymization, and compliance support.

## Features

- **Dual Storage**: Sled (fast/local) + Vault (secure/archived)
- **PII Anonymization**: Automated detection and sanitization of sensitive data
- **Compliance**: HIPAA, GDPR, PCI-DSS, SOC2 support
- **Graceful Degradation**: Continues operation if Vault unavailable
- **Flexible Policies**: Immediate, scheduled, or event-based archival

## Quick Start

```rust
use llm_memory_graph::storage::AsyncSledBackend;
use llm_memory_graph::integrations::vault::{
    VaultStorageAdapter, VaultStorageConfig, StorageMode
};
use std::sync::Arc;

// Create dual-storage adapter
let sled = AsyncSledBackend::open("./data/graph.db").await?;
let config = VaultStorageConfig::new("http://vault:9000", "api-key")
    .with_storage_mode(StorageMode::ArchiveOnPolicy);
let adapter = VaultStorageAdapter::new(Arc::new(sled), config).await?;

// Use as AsyncStorageBackend
adapter.store_node(&node).await?;

// Archive session to vault
let archive_id = adapter.archive_session(&session_id).await?;
```

## Modules

### `config.rs` - Configuration
- `VaultStorageConfig` - Main configuration
- `StorageMode` - Dual-write behavior
- `ArchivalPolicy` - When/how to archive
- `AnonymizationConfig` - PII handling
- `PerformanceConfig` - Timeouts, retries, degradation

### `anonymizer.rs` - PII Detection & Anonymization
- `DataAnonymizer` - PII detection and sanitization
- `PiiType` - Types of PII (email, phone, SSN, etc.)
- `AnonymizationStrategy` - Redact, hash, encrypt, tokenize
- `PiiDetection` - Detection results

### `storage_adapter.rs` - Dual-Storage Implementation
- `VaultStorageAdapter<B>` - Main adapter implementing AsyncStorageBackend
- `AdapterStats` - Statistics (writes, failures, PII counts)
- Dual-write logic with graceful degradation

### `archiver.rs` - Vault Client (Existing)
- `VaultClient` - HTTP client for Data Vault service
- `ArchiveEntry` - Archive data structure
- `RetentionPolicy` - Retention configuration
- `ComplianceLevel` - HIPAA, GDPR, PCI, SOC2

### `retention.rs` - Retention Management (Existing)
- `RetentionPolicyManager` - Policy management
- `ArchivalScheduler` - Scheduled archival
- `ArchivalStats` - Archival statistics

## Storage Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| `SledOnly` | Sled only, no vault | Development, testing |
| `DualSync` | Both must succeed | Critical data, strong consistency |
| `DualAsync` | Sled first, vault async | High throughput, eventual consistency |
| `ArchiveOnPolicy` | Archive based on policy | Production (recommended) |

## Archival Modes

- `Immediate` - Archive on every write
- `OnSessionEnd` - Archive when session completes
- `Scheduled` - Nightly/periodic archival
- `AgeThreshold` - Archive after N days
- `Manual` - Explicit API calls only

## PII Anonymization

Supports detection and anonymization of:
- Email addresses
- Phone numbers
- Credit card numbers
- Social Security Numbers
- IP addresses
- Physical addresses
- API keys
- Custom patterns (regex)

Strategies:
- **Redact**: Replace with `[REDACTED]`
- **Hash**: Deterministic hash (format-preserving)
- **Randomize**: Random value preserving format
- **Encrypt**: Reversible encryption
- **Tokenize**: Token mapping

## Configuration Examples

### Production (Compliance)

```rust
VaultStorageConfig::new(vault_url, api_key)
    .with_storage_mode(StorageMode::ArchiveOnPolicy)
    .with_archival_policy(ArchivalPolicy {
        mode: ArchivalMode::OnSessionEnd,
        retention_days: 2555,  // 7 years HIPAA
        auto_delete_from_sled: true,
        sled_retention_days: Some(90),
        ..Default::default()
    })
    .with_anonymization(AnonymizationConfig {
        enabled: true,
        strategy: AnonymizationStrategy::Hash,
        ..Default::default()
    })
```

### Development (Sled-Only)

```rust
VaultStorageConfig::default()
    .with_vault_disabled()
```

### High-Availability

```rust
VaultStorageConfig::new(vault_url, api_key)
    .with_storage_mode(StorageMode::DualAsync)
    .with_performance(PerformanceConfig {
        graceful_degradation: true,
        max_concurrent_ops: 50,
        queue_failed_writes: true,
        ..Default::default()
    })
```

## Environment Variables

```bash
export VAULT_STORAGE_ENABLED=true
export VAULT_URL=http://vault.example.com:9000
export VAULT_API_KEY=your-api-key
```

## Statistics & Monitoring

```rust
let stats = adapter.get_stats().await;
println!("Sled writes: {}", stats.sled_writes);
println!("Vault writes: {}", stats.vault_writes);
println!("Vault failures: {}", stats.vault_failures);
println!("PII anonymized: {}", stats.pii_anonymized);
println!("Sessions archived: {}", stats.sessions_archived);
```

## Key Constraints

- ✅ **Backward Compatible** - Existing Sled storage continues to work
- ✅ **No Schema Changes** - Uses existing data structures
- ✅ **Vault Optional** - Can disable anytime
- ✅ **Graceful Degradation** - Continues on vault failures
- ✅ **Non-Invasive** - Doesn't modify existing archiver/retention modules

## Testing

```bash
cargo test -p llm-memory-graph --lib integrations::vault
```

Tests cover:
- Config serialization/deserialization
- PII detection patterns
- Anonymization strategies
- Sled-only mode
- Statistics tracking

## Documentation

Full documentation: `/docs/VAULT_STORAGE_INTEGRATION.md`

API docs: `cargo doc --open`

## Dependencies

- `llm-data-vault` - Data Vault client SDK
- `regex` - PII pattern matching
- `serde_json` - Data serialization
- `tokio` - Async runtime
- `tracing` - Logging

## Performance

- **Sled writes**: ~100K ops/sec (local)
- **Vault writes**: ~1K ops/sec (network)
- **Anonymization**: ~10-50 μs per string
- **Memory**: ~1MB per 10K queued sessions

## License

MIT OR Apache-2.0

# LLM-Memory-Graph Integrations

This document describes the external ecosystem integrations for LLM-Memory-Graph, including LLM-Registry and Data-Vault clients.

## Table of Contents

1. [Overview](#overview)
2. [LLM-Registry Integration](#llm-registry-integration)
3. [Data-Vault Integration](#data-vault-integration)
4. [Configuration](#configuration)
5. [Error Handling](#error-handling)
6. [Examples](#examples)

---

## Overview

The integrations module provides seamless connectivity with the LLM DevOps ecosystem:

- **LLM-Registry**: Model metadata, version tracking, and usage statistics
- **Data-Vault**: Secure archival, retention policies, and compliance management

### Key Features

- Async HTTP clients using `reqwest`
- Automatic retry logic with exponential backoff
- Circuit breaker support for fault tolerance
- Comprehensive error handling and logging
- API key authentication
- Configurable timeouts and batch sizes

---

## LLM-Registry Integration

The LLM-Registry integration provides functionality for:

- Session registration and tracking
- Model metadata retrieval
- Usage statistics and monitoring
- Session lifecycle management

### Usage

```rust
use llm_memory_graph::integrations::registry::{
    RegistryClient, RegistryConfig, SessionRegistration, UsageReport
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create registry client
    let config = RegistryConfig::new("http://registry.example.com")
        .with_api_key("your-api-key")
        .with_timeout(30);

    let client = RegistryClient::new(config)?;

    // Register a session
    let registration = SessionRegistration::new("session-123", "gpt-4")
        .with_tag("production")
        .with_metadata("user_id", serde_json::json!("user-456"));

    let session_info = client.register_session(registration).await?;
    println!("Session registered: {:?}", session_info);

    // Track usage
    let usage = UsageReport::new("session-123", 100, 200)
        .with_model_id("gpt-4")
        .with_latency_ms(1500);

    client.track_usage(usage).await?;

    // Get model metadata
    let metadata = client.get_model_metadata("gpt-4").await?;
    println!("Model: {} v{}", metadata.model_id, metadata.version);

    // Get usage statistics
    let stats = client.get_session_usage("session-123").await?;
    println!("Total tokens used: {}", stats.total_tokens);

    Ok(())
}
```

### Registry Client API

#### Session Operations

```rust
// Register a new session
async fn register_session(
    &self,
    registration: SessionRegistration,
) -> Result<SessionInfo, IntegrationError>

// Update session status
async fn update_session_status(
    &self,
    session_id: &str,
    status: SessionStatus,
) -> Result<SessionInfo, IntegrationError>

// List sessions (with pagination)
async fn list_sessions(
    &self,
    page: Option<usize>,
    page_size: Option<usize>,
) -> Result<SessionListResponse, IntegrationError>

// Delete a session
async fn delete_session(
    &self,
    session_id: &str,
) -> Result<(), IntegrationError>
```

#### Model Operations

```rust
// Get model metadata
async fn get_model_metadata(
    &self,
    model_id: &str,
) -> Result<ModelMetadata, IntegrationError>

// List available models
async fn list_models(
    &self,
    page: Option<usize>,
    page_size: Option<usize>,
) -> Result<ModelListResponse, IntegrationError>
```

#### Usage Tracking

```rust
// Track token usage
async fn track_usage(
    &self,
    report: UsageReport,
) -> Result<(), IntegrationError>

// Get session usage statistics
async fn get_session_usage(
    &self,
    session_id: &str,
) -> Result<UsageStats, IntegrationError>

// Get model usage statistics
async fn get_model_usage(
    &self,
    model_id: &str,
) -> Result<UsageStats, IntegrationError>
```

### Session States

```rust
pub enum SessionStatus {
    Active,      // Session is currently active
    Completed,   // Session ended normally
    Failed,      // Session terminated with error
    Archived,    // Session has been archived
}
```

---

## Data-Vault Integration

The Data-Vault integration provides:

- Session archival and retrieval
- Retention policy management
- Compliance-level data handling
- Automatic archival scheduling

### Usage

```rust
use llm_memory_graph::integrations::vault::{
    VaultClient, VaultConfig, ArchiveEntry, RetentionPolicy, ComplianceLevel
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create vault client
    let config = VaultConfig::new("http://vault.example.com", "vault-api-key")
        .with_encryption(true)
        .with_compression(true);

    let client = VaultClient::new(config)?;

    // Archive a session
    let session_data = serde_json::json!({
        "session_id": "session-123",
        "prompts": ["prompt1", "prompt2"],
        "responses": ["response1", "response2"]
    });

    let entry = ArchiveEntry::new("session-123", session_data, 365)
        .with_tag("production")
        .with_metadata("user_id", serde_json::json!("user-456"));

    let response = client.archive_session(entry).await?;
    println!("Archived with ID: {}", response.archive_id);

    // Create retention policy
    let policy = RetentionPolicy::new("HIPAA", 2555, ComplianceLevel::Hipaa)
        .with_auto_delete(false)
        .with_description("7-year retention for healthcare data");

    let policy_id = client.create_retention_policy(policy).await?;

    // Apply policy to archive
    client.apply_retention_policy(&response.archive_id, &policy_id).await?;

    // Retrieve archive
    let retrieved = client.retrieve_session(&response.archive_id).await?;
    println!("Retrieved data: {:?}", retrieved.data);

    Ok(())
}
```

### Vault Client API

#### Archive Operations

```rust
// Archive a session
async fn archive_session(
    &self,
    entry: ArchiveEntry,
) -> Result<ArchiveResponse, IntegrationError>

// Batch archive multiple sessions
async fn batch_archive(
    &self,
    entries: Vec<ArchiveEntry>,
) -> Result<BatchArchiveResponse, IntegrationError>

// Retrieve an archived session
async fn retrieve_session(
    &self,
    archive_id: &str,
) -> Result<ArchiveEntry, IntegrationError>

// Delete an archive
async fn delete_archive(
    &self,
    archive_id: &str,
) -> Result<(), IntegrationError>
```

#### Retention Policy Operations

```rust
// Create a retention policy
async fn create_retention_policy(
    &self,
    policy: RetentionPolicy,
) -> Result<String, IntegrationError>

// Apply a policy to an archive
async fn apply_retention_policy(
    &self,
    archive_id: &str,
    policy_id: &str,
) -> Result<(), IntegrationError>
```

### Compliance Levels

```rust
pub enum ComplianceLevel {
    Standard,  // Standard retention (1 year default)
    Hipaa,     // HIPAA compliance (7 years)
    Gdpr,      // GDPR compliance (7 years)
    Pci,       // PCI-DSS compliance (3 years)
    Soc2,      // SOC 2 compliance (7 years)
}
```

### Automatic Archival Scheduler

The archival scheduler provides automated session archival based on age and retention policies:

```rust
use llm_memory_graph::integrations::vault::{
    ArchivalScheduler, SchedulerConfig, ComplianceLevel
};
use llm_memory_graph::engine::AsyncMemoryGraph;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vault_client = VaultClient::new(vault_config)?;
    let graph = Arc::new(AsyncMemoryGraph::open(config).await?);

    // Configure scheduler
    let scheduler_config = SchedulerConfig::new()
        .with_interval_hours(24)           // Run every 24 hours
        .with_archive_after_days(30)       // Archive sessions older than 30 days
        .with_retention_days(365)          // Keep archives for 1 year
        .with_batch_size(100)              // Process 100 sessions per batch
        .with_compliance_level(ComplianceLevel::Standard);

    // Create and start scheduler
    let scheduler = ArchivalScheduler::new(vault_client, graph, scheduler_config);
    let handle = scheduler.start().await;

    // Scheduler runs in background
    // ...

    // Stop scheduler when needed
    scheduler.stop().await;

    Ok(())
}
```

---

## Configuration

### Environment Variables

Both integrations support configuration via environment variables:

#### LLM-Registry

```bash
REGISTRY_URL=http://registry.example.com
REGISTRY_API_KEY=your-registry-api-key
```

#### Data-Vault

```bash
VAULT_URL=http://vault.example.com
VAULT_API_KEY=your-vault-api-key
```

### Configuration Objects

#### RegistryConfig

```rust
RegistryConfig {
    base_url: String,           // Registry service URL
    api_key: Option<String>,    // API key for authentication
    timeout_secs: u64,          // Request timeout (default: 30s)
    retry_count: usize,         // Max retry attempts (default: 3)
    enable_logging: bool,       // Enable debug logging (default: true)
}
```

#### VaultConfig

```rust
VaultConfig {
    base_url: String,           // Vault service URL
    api_key: String,            // API key (required)
    encryption_enabled: bool,   // Enable encryption (default: true)
    compression_enabled: bool,  // Enable compression (default: true)
    timeout_secs: u64,          // Request timeout (default: 60s)
    batch_size: usize,          // Batch operation size (default: 100)
    enable_logging: bool,       // Enable debug logging (default: true)
}
```

---

## Error Handling

All integration operations return `Result<T, IntegrationError>`:

```rust
pub enum IntegrationError {
    HttpError(String),              // HTTP request failed
    AuthenticationError(String),    // Authentication failed
    ApiError {                      // API returned error
        status: u16,
        message: String,
    },
    ConnectionError(String),        // Connection failed
    Timeout(u64),                   // Request timed out
    CircuitBreakerOpen(String),     // Circuit breaker active
    InvalidConfig(String),          // Invalid configuration
    Serialization(String),          // Serialization error
}
```

### Retry Logic

The integration clients include automatic retry logic for transient errors:

```rust
use llm_memory_graph::integrations::RetryPolicy;
use std::time::Duration;

let retry_policy = RetryPolicy::new()
    .with_max_attempts(5)
    .with_initial_delay(Duration::from_millis(100))
    .with_backoff_multiplier(2.0);

let client = RegistryClient::new(config)?
    .with_retry_policy(retry_policy);
```

Retryable errors:
- HTTP 5xx errors (server errors)
- Connection timeouts
- Connection errors

Non-retryable errors:
- HTTP 4xx errors (client errors)
- Authentication failures
- Invalid configuration

---

## Examples

### Complete Integration Example

```rust
use llm_memory_graph::{
    AsyncMemoryGraph, Config,
    integrations::{
        registry::{RegistryClient, RegistryConfig, SessionRegistration, UsageReport},
        vault::{VaultClient, VaultConfig, ArchiveEntry, ArchivalScheduler, SchedulerConfig},
    },
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize memory graph
    let graph_config = Config::new("./data");
    let graph = Arc::new(AsyncMemoryGraph::open(graph_config).await?);

    // Initialize registry client
    let registry_config = RegistryConfig::new("http://registry.example.com")
        .with_api_key("registry-api-key");
    let registry = RegistryClient::new(registry_config)?;

    // Initialize vault client
    let vault_config = VaultConfig::new("http://vault.example.com", "vault-api-key");
    let vault = VaultClient::new(vault_config)?;

    // Create a session
    let session = graph.create_session().await?;

    // Register with registry
    let registration = SessionRegistration::new(
        session.id.to_string(),
        "gpt-4"
    ).with_tag("production");

    registry.register_session(registration).await?;

    // Add prompts and responses
    let prompt_id = graph.add_prompt(
        session.id,
        "Explain quantum computing".to_string(),
        None
    ).await?;

    let response_id = graph.add_response(
        &prompt_id,
        "Quantum computing uses...".to_string(),
        Default::default(),
        None
    ).await?;

    // Track usage
    let usage = UsageReport::new(session.id.to_string(), 10, 100)
        .with_model_id("gpt-4");
    registry.track_usage(usage).await?;

    // Archive session after completion
    let session_data = serde_json::json!({
        "session_id": session.id.to_string(),
        "prompts": 1,
        "responses": 1
    });

    let archive_entry = ArchiveEntry::new(
        session.id.to_string(),
        session_data,
        365
    );

    let archive_response = vault.archive_session(archive_entry).await?;
    println!("Session archived: {}", archive_response.archive_id);

    Ok(())
}
```

### Scheduler Example

```rust
use llm_memory_graph::integrations::vault::{
    VaultClient, VaultConfig, ArchivalScheduler, SchedulerConfig, ComplianceLevel
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vault_config = VaultConfig::new("http://vault.example.com", "api-key");
    let vault = VaultClient::new(vault_config)?;

    let graph = Arc::new(AsyncMemoryGraph::open(Config::default()).await?);

    let scheduler_config = SchedulerConfig::new()
        .with_interval_hours(24)
        .with_archive_after_days(30)
        .with_retention_days(2555)  // 7 years for HIPAA
        .with_compliance_level(ComplianceLevel::Hipaa);

    let scheduler = ArchivalScheduler::new(vault, graph, scheduler_config);

    // Start background archival
    let handle = scheduler.start().await;

    // Run application...

    // Cleanup
    scheduler.stop().await;
    handle.await?;

    Ok(())
}
```

---

## API Endpoints

### LLM-Registry Endpoints

- `POST /api/v1/sessions` - Register a session
- `GET /api/v1/sessions` - List sessions
- `GET /api/v1/sessions/{id}` - Get session details
- `PUT /api/v1/sessions/{id}/status` - Update session status
- `DELETE /api/v1/sessions/{id}` - Delete session
- `GET /api/v1/models` - List models
- `GET /api/v1/models/{id}` - Get model metadata
- `POST /api/v1/usage` - Track usage
- `GET /api/v1/sessions/{id}/usage` - Get session usage
- `GET /api/v1/models/{id}/usage` - Get model usage
- `GET /health` - Health check

### Data-Vault Endpoints

- `POST /api/v1/archive` - Archive a session
- `POST /api/v1/archive/batch` - Batch archive sessions
- `GET /api/v1/archive/{id}` - Retrieve archive
- `DELETE /api/v1/archive/{id}` - Delete archive
- `POST /api/v1/policies` - Create retention policy
- `PUT /api/v1/archive/{id}/policy` - Apply retention policy
- `GET /health` - Health check

---

## Testing

Run integration tests:

```bash
# Unit tests
cargo test --test integrations_test

# Integration tests (requires running services)
REGISTRY_URL=http://localhost:8080 \
REGISTRY_API_KEY=test-key \
VAULT_URL=http://localhost:9000 \
VAULT_API_KEY=test-key \
cargo test --test integrations_test --features integration-tests
```

---

## Performance Considerations

1. **Connection Pooling**: Clients reuse HTTP connections
2. **Retry Logic**: Exponential backoff prevents thundering herd
3. **Batch Operations**: Use batch APIs for bulk operations
4. **Timeouts**: Configure appropriate timeouts for your network
5. **Logging**: Disable verbose logging in production

---

## Security

1. **API Keys**: Store API keys in environment variables, never in code
2. **TLS**: Use HTTPS for production deployments
3. **Encryption**: Enable encryption for sensitive data in vault
4. **Authentication**: All requests require valid API keys
5. **Rate Limiting**: Clients implement retry backoff to respect rate limits

---

## Troubleshooting

### Connection Errors

```rust
// Increase timeout for slow networks
let config = RegistryConfig::new("http://registry.example.com")
    .with_timeout(60);
```

### Authentication Failures

```bash
# Verify API key is set
echo $REGISTRY_API_KEY
echo $VAULT_API_KEY
```

### Retry Exhausted

```rust
// Increase retry attempts
let policy = RetryPolicy::new().with_max_attempts(5);
let client = RegistryClient::new(config)?.with_retry_policy(policy);
```

---

## License

Same as LLM-Memory-Graph: MIT OR Apache-2.0

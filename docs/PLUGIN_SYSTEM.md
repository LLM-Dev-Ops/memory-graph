# LLM-Memory-Graph Plugin System

**Version**: 1.0.0
**Status**: Production Ready
**Date**: 2025-11-07

## Overview

The LLM-Memory-Graph plugin system provides a flexible, async-first architecture for extending the core functionality with custom behavior. Plugins can intercept operations at specific hook points to provide validation, enrichment, transformation, auditing, and integration with external systems.

## Architecture

### Core Components

1. **Plugin Trait** (`src/plugin/mod.rs`)
   - Main interface that all plugins must implement
   - Defines hook methods for different operations
   - Thread-safe with `Send + Sync` bounds
   - Uses interior mutability pattern for state management

2. **Plugin Manager** (`src/plugin/manager.rs`)
   - Manages plugin lifecycle (registration, initialization, enable/disable, shutdown)
   - Executes hooks in order
   - Handles errors and state tracking
   - Thread-safe wrapper (Arc<RwLock<PluginManager>>)

3. **Plugin Registry** (`src/plugin/registry.rs`)
   - Catalogs available plugins with metadata
   - Supports capability-based discovery
   - Tag-based organization
   - Separate from runtime management

4. **Hook Execution Framework** (`src/plugin/hooks.rs`)
   - Defines hook points in the system
   - Before/after hook semantics
   - Fail-fast vs. continue-on-error modes
   - Performance metrics collection

## Hook Points

Plugins can intercept operations at these points:

| Hook Point | When Executed | Can Fail Operation |
|-----------|---------------|-------------------|
| `before_create_node` | Before node creation | Yes |
| `after_create_node` | After node creation | No |
| `before_create_session` | Before session creation | Yes |
| `after_create_session` | After session creation | No |
| `before_query` | Before query execution | Yes |
| `after_query` | After query execution | No |
| `before_create_edge` | Before edge creation | Yes |
| `after_create_edge` | After edge creation | No |

### Hook Semantics

- **Before Hooks**: Can prevent operations by returning errors (fail-fast)
- **After Hooks**: Cannot fail operations (logging/notification only)

## Creating a Plugin

### 1. Basic Plugin Structure

```rust
use llm_memory_graph::plugin::{Plugin, PluginBuilder, PluginContext, PluginError, PluginMetadata};
use async_trait::async_trait;

pub struct MyPlugin {
    metadata: PluginMetadata,
    // Use interior mutability for state (e.g., Arc<AtomicUsize>, Mutex<T>)
}

impl MyPlugin {
    pub fn new() -> Self {
        let metadata = PluginBuilder::new("my_plugin", "1.0.0")
            .author("Your Name")
            .description("My custom plugin")
            .capability("validation")
            .build();

        Self { metadata }
    }
}

#[async_trait]
impl Plugin for MyPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn init(&self) -> Result<(), PluginError> {
        // Initialize resources (optional)
        Ok(())
    }

    async fn before_create_node(&self, context: &PluginContext) -> Result<(), PluginError> {
        // Implement your validation logic
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), PluginError> {
        // Cleanup resources (optional)
        Ok(())
    }
}
```

### 2. Using the Plugin

```rust
use llm_memory_graph::plugin::PluginManager;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create plugin manager
    let mut manager = PluginManager::new();

    // Register plugin
    let plugin: Arc<dyn Plugin> = Arc::new(MyPlugin::new());
    manager.register(plugin).await?;

    // Initialize and enable
    manager.initialize("my_plugin").await?;
    manager.enable("my_plugin")?;

    // Wrap for concurrent access
    let manager = Arc::new(RwLock::new(manager));

    // Use in your application...

    Ok(())
}
```

## Example Plugins

### 1. Validation Plugin

Location: `/workspaces/llm-memory-graph/plugins/example_validator`

**Features**:
- Content length validation
- Character set validation
- Profanity filtering
- Custom blocked words

**Usage**:
```rust
use example_validator::{ValidationPlugin, ValidationRulesBuilder};

let validator = ValidationRulesBuilder::new()
    .max_length(5000)
    .min_length(10)
    .check_profanity(true)
    .block_word("spam")
    .build();
```

### 2. Enrichment Plugin

Location: `/workspaces/llm-memory-graph/plugins/example_enricher`

**Features**:
- Automatic content analysis (word count, character count)
- Timestamp enrichment
- Correlation IDs
- Session statistics

**Usage**:
```rust
use example_enricher::EnrichmentPlugin;

let enricher = EnrichmentPlugin::new();
// Get statistics
let stats = enricher.get_stats();
```

## Plugin Lifecycle

```
┌─────────────┐
│ Registered  │ ← register()
└──────┬──────┘
       │ initialize()
       ▼
┌─────────────┐
│ Initialized │
└──────┬──────┘
       │ enable()
       ▼
┌─────────────┐
│  Enabled    │ ◄─┐
└──────┬──────┘   │
       │          │ enable()
       │ disable()│
       ▼          │
┌─────────────┐   │
│  Disabled   │ ──┘
└──────┬──────┘
       │ shutdown()
       ▼
   (removed)
```

## Plugin Context

The `PluginContext` provides plugins with information about the operation:

```rust
pub struct PluginContext {
    pub operation: String,              // Operation type
    pub data: Value,                    // Operation data (JSON)
    pub metadata: HashMap<String, String>, // Additional metadata
    pub timestamp: DateTime<Utc>,       // When context was created
}
```

### Accessing Context Data

```rust
async fn before_create_node(&self, context: &PluginContext) -> Result<(), PluginError> {
    // Get content from data
    if let Some(content) = context.data().get("content").and_then(|v| v.as_str()) {
        // Process content...
    }

    // Get metadata
    if let Some(user) = context.get_metadata("user") {
        // Process user info...
    }

    Ok(())
}
```

## Error Handling

Plugins should return specific error types:

```rust
pub enum PluginError {
    InitFailed(String),        // Initialization failure
    HookFailed(String),         // Hook execution failure
    NotFound(String),           // Plugin not found
    VersionMismatch(String),    // API version incompatible
    ConfigError(String),        // Configuration error
    AlreadyRegistered(String),  // Duplicate registration
    Disabled(String),           // Plugin disabled
    General(String),            // General error
}
```

## Best Practices

### 1. Use Interior Mutability

Since plugins are shared via `Arc<dyn Plugin>`, use interior mutability for state:

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct MyPlugin {
    metadata: PluginMetadata,
    counter: Arc<AtomicUsize>,  // Interior mutability
}

impl Plugin for MyPlugin {
    async fn before_create_node(&self, _context: &PluginContext) -> Result<(), PluginError> {
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}
```

### 2. Keep Hooks Fast

Hooks are executed synchronously in the request path:

- Avoid heavy computation
- Use async operations for I/O
- Consider background tasks for expensive operations

### 3. Handle Errors Gracefully

Before hooks can fail operations, so validate carefully:

```rust
async fn before_create_node(&self, context: &PluginContext) -> Result<(), PluginError> {
    if let Some(content) = extract_content(context) {
        if content.len() > MAX_LENGTH {
            return Err(PluginError::HookFailed(
                format!("Content too long: {}", content.len())
            ));
        }
    }
    Ok(())
}
```

### 4. Provide Clear Metadata

Help users discover and understand your plugin:

```rust
let metadata = PluginBuilder::new("my_plugin", "1.0.0")
    .author("Your Name <your.email@example.com>")
    .description("Clear description of what the plugin does")
    .capability("validation")
    .capability("enrichment")
    .api_version("1.0.0")  // Must match system API version
    .build();
```

## Performance Considerations

### Hook Execution Overhead

- Before hooks: Fail-fast execution (stops on first error)
- After hooks: Continue-on-error (all plugins execute)
- Typical overhead: < 10ms per hook

### Concurrency

The plugin manager is designed for concurrent access:

```rust
let manager = Arc::new(RwLock::new(PluginManager::new()));

// Multiple threads can read concurrently
let guard = manager.read().await;
guard.execute_before_hooks("before_create_node", &context).await?;
```

## Testing

### Unit Tests

Test individual plugin behavior:

```rust
#[tokio::test]
async fn test_my_plugin() {
    let plugin = MyPlugin::new();
    let context = PluginContext::new("test", json!({"content": "test"}));

    assert!(plugin.before_create_node(&context).await.is_ok());
}
```

### Integration Tests

Test plugin lifecycle and interaction:

```rust
#[tokio::test]
async fn test_plugin_lifecycle() {
    let mut manager = PluginManager::new();
    let plugin: Arc<dyn Plugin> = Arc::new(MyPlugin::new());

    manager.register(plugin).await.unwrap();
    manager.initialize("my_plugin").await.unwrap();
    manager.enable("my_plugin").unwrap();

    assert!(manager.is_enabled("my_plugin"));
}
```

## API Reference

### Plugin Trait

```rust
#[async_trait]
pub trait Plugin: Send + Sync {
    fn metadata(&self) -> &PluginMetadata;
    async fn init(&self) -> Result<(), PluginError>;
    async fn shutdown(&self) -> Result<(), PluginError>;
    async fn before_create_node(&self, context: &PluginContext) -> Result<(), PluginError>;
    async fn after_create_node(&self, context: &PluginContext) -> Result<(), PluginError>;
    async fn before_create_session(&self, context: &PluginContext) -> Result<(), PluginError>;
    async fn after_create_session(&self, context: &PluginContext) -> Result<(), PluginError>;
    async fn before_query(&self, context: &PluginContext) -> Result<(), PluginError>;
    async fn after_query(&self, context: &PluginContext) -> Result<(), PluginError>;
    async fn before_create_edge(&self, context: &PluginContext) -> Result<(), PluginError>;
    async fn after_create_edge(&self, context: &PluginContext) -> Result<(), PluginError>;
}
```

### PluginManager

```rust
impl PluginManager {
    pub fn new() -> Self;
    pub async fn register(&mut self, plugin: Arc<dyn Plugin>) -> Result<(), PluginError>;
    pub async fn unregister(&mut self, name: &str) -> Result<(), PluginError>;
    pub async fn initialize(&mut self, name: &str) -> Result<(), PluginError>;
    pub fn enable(&mut self, name: &str) -> Result<(), PluginError>;
    pub fn disable(&mut self, name: &str) -> Result<(), PluginError>;
    pub fn active_plugins(&self) -> Vec<Arc<dyn Plugin>>;
    pub async fn init_all(&mut self) -> Result<(), PluginError>;
    pub fn enable_all(&mut self) -> Result<(), PluginError>;
    pub fn disable_all(&mut self) -> Result<(), PluginError>;
    pub async fn shutdown_all(&mut self) -> Result<(), PluginError>;
    pub async fn execute_before_hooks(&self, hook_name: &str, context: &PluginContext) -> Result<(), PluginError>;
    pub async fn execute_after_hooks(&self, hook_name: &str, context: &PluginContext) -> Result<(), PluginError>;
}
```

### PluginRegistry

```rust
impl PluginRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, metadata: PluginMetadata, source: Option<PathBuf>) -> Result<(), PluginError>;
    pub fn unregister(&mut self, name: &str) -> Result<(), PluginError>;
    pub fn get(&self, name: &str) -> Option<&PluginRegistryEntry>;
    pub fn contains(&self, name: &str) -> bool;
    pub fn list_all(&self) -> Vec<&PluginRegistryEntry>;
    pub fn find_by_capability(&self, capability: &str) -> Vec<&PluginRegistryEntry>;
    pub fn find_by_tag(&self, tag: &str) -> Vec<&PluginRegistryEntry>;
    pub fn add_tag(&mut self, name: &str, tag: impl Into<String>) -> Result<(), PluginError>;
    pub fn stats(&self) -> PluginRegistryStats;
}
```

## Future Enhancements

### Dynamic Plugin Loading

Future versions will support dynamic plugin loading via shared libraries:

```rust
// Future API
manager.load_from_directory("/path/to/plugins").await?;
```

### Plugin Configuration

Enhanced configuration schema support:

```rust
let metadata = PluginBuilder::new("my_plugin", "1.0.0")
    .config_schema(json!({
        "type": "object",
        "properties": {
            "max_length": {"type": "integer"},
            "enabled": {"type": "boolean"}
        }
    }))
    .build();
```

### Plugin Marketplace

A centralized registry for discovering and sharing plugins.

## Troubleshooting

### Plugin Not Executing

1. Check plugin is registered: `manager.get_state("plugin_name")`
2. Verify plugin is enabled: `manager.is_enabled("plugin_name")`
3. Check hook name matches: Use constants from `HookPoint`

### Version Mismatch Errors

Ensure plugin API version matches system version:

```rust
PluginBuilder::new("my_plugin", "1.0.0")
    .api_version("1.0.0")  // Must match system API version
    .build();
```

### Performance Issues

1. Profile hook execution with metrics
2. Consider moving expensive operations to background tasks
3. Use caching for repeated calculations

## Support and Contributing

- **Documentation**: `/workspaces/llm-memory-graph/docs/`
- **Examples**: `/workspaces/llm-memory-graph/plugins/`
- **Tests**: `/workspaces/llm-memory-graph/tests/plugin_integration_test.rs`
- **Issues**: GitHub Issues
- **Discussions**: GitHub Discussions

## Version History

- **1.0.0** (2025-11-07): Initial production release
  - Complete plugin system implementation
  - Hook execution framework
  - Plugin registry and discovery
  - Example plugins (validator, enricher)
  - Comprehensive test suite (17 tests, 100% pass rate)

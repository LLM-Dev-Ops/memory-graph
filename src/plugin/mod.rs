//! Plugin system for extending LLM-Memory-Graph functionality
//!
//! This module provides a flexible plugin architecture that allows users to extend
//! the core functionality of LLM-Memory-Graph with custom behavior. Plugins can hook
//! into various operations to provide:
//!
//! - **Validation**: Content validation and rule enforcement
//! - **Enrichment**: Automatic metadata enhancement
//! - **Transformation**: Data transformation and normalization
//! - **Auditing**: Custom audit logging and compliance tracking
//! - **Integration**: External system integration
//!
//! # Architecture
//!
//! The plugin system is designed around these core concepts:
//!
//! - **Plugin Trait**: The main interface all plugins must implement
//! - **Plugin Context**: Provides plugins with access to operation data
//! - **Hook Points**: Specific points in the execution flow where plugins are called
//! - **Plugin Manager**: Manages plugin lifecycle and execution
//!
//! # Example
//!
//! ```rust
//! use llm_memory_graph::plugin::{Plugin, PluginBuilder, PluginContext, PluginError, PluginMetadata};
//! use async_trait::async_trait;
//!
//! struct MyValidationPlugin {
//!     metadata: PluginMetadata,
//! }
//!
//! impl MyValidationPlugin {
//!     pub fn new() -> Self {
//!         let metadata = PluginBuilder::new("my_validator", "1.0.0")
//!             .author("Your Name")
//!             .description("Custom validation plugin")
//!             .capability("validation")
//!             .build();
//!
//!         Self { metadata }
//!     }
//! }
//!
//! #[async_trait]
//! impl Plugin for MyValidationPlugin {
//!     fn metadata(&self) -> &PluginMetadata {
//!         &self.metadata
//!     }
//!
//!     async fn before_create_node(&self, context: &PluginContext) -> Result<(), PluginError> {
//!         // Custom validation logic
//!         Ok(())
//!     }
//! }
//! ```

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

pub mod hooks;
pub mod manager;
pub mod registry;

pub use hooks::{HookExecutor, HookPoint, HookRegistry};
pub use manager::PluginManager;
pub use registry::{PluginDiscovery, PluginRegistry};

/// Plugin error type
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// Plugin initialization failed
    #[error("Plugin initialization failed: {0}")]
    InitFailed(String),

    /// Plugin hook execution failed
    #[error("Plugin hook execution failed: {0}")]
    HookFailed(String),

    /// Plugin not found
    #[error("Plugin not found: {0}")]
    NotFound(String),

    /// Plugin version incompatible
    #[error("Plugin version incompatible: {0}")]
    VersionMismatch(String),

    /// Plugin configuration error
    #[error("Plugin configuration error: {0}")]
    ConfigError(String),

    /// Plugin already registered
    #[error("Plugin already registered: {0}")]
    AlreadyRegistered(String),

    /// Plugin disabled
    #[error("Plugin disabled: {0}")]
    Disabled(String),

    /// General plugin error
    #[error("Plugin error: {0}")]
    General(String),
}

/// Plugin metadata
///
/// Contains information about a plugin, including its name, version,
/// author, description, and capabilities.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginMetadata {
    /// Plugin name (unique identifier)
    pub name: String,

    /// Plugin version (semantic versioning)
    pub version: String,

    /// Plugin author
    pub author: String,

    /// Plugin description
    pub description: String,

    /// API version this plugin is compatible with
    pub api_version: String,

    /// List of capabilities this plugin provides
    pub capabilities: Vec<String>,

    /// Optional plugin configuration schema
    pub config_schema: Option<Value>,
}

impl fmt::Display for PluginMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} v{} by {} - {} (API: {})",
            self.name, self.version, self.author, self.description, self.api_version
        )
    }
}

/// Plugin context for hooks
///
/// Provides plugins with information about the current operation,
/// including the operation type, data being processed, and metadata.
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// Operation being performed (e.g., "create_node", "create_session")
    pub operation: String,

    /// Data associated with the operation (JSON format)
    pub data: Value,

    /// Additional metadata (key-value pairs)
    pub metadata: HashMap<String, String>,

    /// Timestamp when the context was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl PluginContext {
    /// Create a new plugin context
    pub fn new(operation: impl Into<String>, data: Value) -> Self {
        Self {
            operation: operation.into(),
            data,
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Add metadata to the context
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get a metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Set a metadata value
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get the operation type
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// Get the data
    pub fn data(&self) -> &Value {
        &self.data
    }
}

/// Plugin trait - all plugins must implement this
///
/// Plugins can selectively implement hooks they're interested in.
/// All hooks are async and return a Result to allow for error handling.
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize plugin
    ///
    /// Called once when the plugin is registered and enabled.
    /// Use this to set up any resources, connections, or state.
    /// Plugins should use interior mutability (e.g., Mutex, RwLock) for any state changes.
    async fn init(&self) -> Result<(), PluginError> {
        Ok(())
    }

    /// Shutdown plugin
    ///
    /// Called when the plugin is being disabled or the system is shutting down.
    /// Use this to clean up resources, close connections, etc.
    /// Plugins should use interior mutability (e.g., Mutex, RwLock) for any state changes.
    async fn shutdown(&self) -> Result<(), PluginError> {
        Ok(())
    }

    /// Hook: Before node creation
    ///
    /// Called before a node is created in the graph.
    /// Can be used for validation, transformation, or enrichment.
    async fn before_create_node(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Hook: After node creation
    ///
    /// Called after a node is successfully created.
    /// Can be used for logging, notifications, or follow-up actions.
    async fn after_create_node(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Hook: Before session creation
    ///
    /// Called before a session is created.
    /// Can be used for validation, quota checking, or initialization.
    async fn before_create_session(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Hook: After session creation
    ///
    /// Called after a session is successfully created.
    /// Can be used for registration, logging, or setup.
    async fn after_create_session(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Hook: Before query execution
    ///
    /// Called before a query is executed.
    /// Can be used for query validation, transformation, or access control.
    async fn before_query(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Hook: After query execution
    ///
    /// Called after a query is successfully executed.
    /// Can be used for result transformation, caching, or logging.
    async fn after_query(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Hook: Before edge creation
    ///
    /// Called before an edge is created in the graph.
    /// Can be used for relationship validation or enforcement.
    async fn before_create_edge(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Hook: After edge creation
    ///
    /// Called after an edge is successfully created.
    /// Can be used for graph analysis or notifications.
    async fn after_create_edge(&self, _context: &PluginContext) -> Result<(), PluginError> {
        Ok(())
    }

    /// Generic hook execution (before)
    ///
    /// Routes to the appropriate before hook based on the hook name.
    async fn before_hook(&self, hook_name: &str, context: &PluginContext) -> Result<(), PluginError> {
        match hook_name {
            "before_create_node" => self.before_create_node(context).await,
            "before_create_session" => self.before_create_session(context).await,
            "before_query" => self.before_query(context).await,
            "before_create_edge" => self.before_create_edge(context).await,
            _ => Ok(()),
        }
    }

    /// Generic hook execution (after)
    ///
    /// Routes to the appropriate after hook based on the hook name.
    async fn after_hook(&self, hook_name: &str, context: &PluginContext) -> Result<(), PluginError> {
        match hook_name {
            "after_create_node" => self.after_create_node(context).await,
            "after_create_session" => self.after_create_session(context).await,
            "after_query" => self.after_query(context).await,
            "after_create_edge" => self.after_create_edge(context).await,
            _ => Ok(()),
        }
    }
}

/// Plugin builder for configuration
///
/// Provides a fluent API for building plugin metadata.
///
/// # Example
///
/// ```rust
/// use llm_memory_graph::plugin::PluginBuilder;
///
/// let metadata = PluginBuilder::new("my_plugin", "1.0.0")
///     .author("John Doe")
///     .description("My custom plugin")
///     .capability("validation")
///     .capability("enrichment")
///     .build();
/// ```
pub struct PluginBuilder {
    metadata: PluginMetadata,
}

impl PluginBuilder {
    /// Create a new plugin builder
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            metadata: PluginMetadata {
                name: name.into(),
                version: version.into(),
                author: String::new(),
                description: String::new(),
                api_version: "1.0.0".to_string(),
                capabilities: Vec::new(),
                config_schema: None,
            },
        }
    }

    /// Set the plugin author
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.metadata.author = author.into();
        self
    }

    /// Set the plugin description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.metadata.description = description.into();
        self
    }

    /// Set the API version
    pub fn api_version(mut self, version: impl Into<String>) -> Self {
        self.metadata.api_version = version.into();
        self
    }

    /// Add a capability
    pub fn capability(mut self, capability: impl Into<String>) -> Self {
        self.metadata.capabilities.push(capability.into());
        self
    }

    /// Set the configuration schema
    pub fn config_schema(mut self, schema: Value) -> Self {
        self.metadata.config_schema = Some(schema);
        self
    }

    /// Build the plugin metadata
    pub fn build(self) -> PluginMetadata {
        self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_builder() {
        let metadata = PluginBuilder::new("test_plugin", "1.0.0")
            .author("Test Author")
            .description("Test plugin")
            .capability("validation")
            .capability("enrichment")
            .build();

        assert_eq!(metadata.name, "test_plugin");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.author, "Test Author");
        assert_eq!(metadata.capabilities.len(), 2);
    }

    #[test]
    fn test_plugin_context() {
        let context = PluginContext::new("test_operation", serde_json::json!({"key": "value"}))
            .with_metadata("test_key", "test_value");

        assert_eq!(context.operation(), "test_operation");
        assert_eq!(context.get_metadata("test_key"), Some(&"test_value".to_string()));
    }
}

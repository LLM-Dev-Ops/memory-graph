//! Schema Registry integration module
//!
//! This module provides integration with the Schema Registry service for centralized
//! schema management, validation, and versioning. It enables:
//!
//! - **Schema Registration**: Register and version schemas for prompts, responses, and metadata
//! - **Data Validation**: Validate data against registered schemas with caching
//! - **Schema Evolution**: Check schema compatibility and manage schema versions
//! - **Graceful Degradation**: OPT-IN validation with fallback when registry is unavailable
//!
//! # Architecture
//!
//! The integration follows a layered architecture:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   AsyncMemoryGraph                          │
//! │                  (Consumer Layer)                           │
//! └─────────────────────────────────────────────────────────────┘
//!                            │
//!                            ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │              SchemaRegistryClient                           │
//! │         (Adapter/Client Layer - THIS MODULE)                │
//! │                                                              │
//! │  ┌────────────────────────────────────────────────────┐    │
//! │  │           SchemaValidator (Trait)                  │    │
//! │  └────────────────────────────────────────────────────┘    │
//! │           │                  │                │             │
//! │           ▼                  ▼                ▼             │
//! │    NoOpValidator    GracefulValidator  CachingValidator    │
//! │                                                              │
//! └─────────────────────────────────────────────────────────────┘
//!                            │
//!                            ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │         llm-schema-registry (External Service)              │
//! │              via llm-schema-registry-client                 │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage Patterns
//!
//! ## 1. OPT-IN Validation (Default)
//!
//! By default, schema validation is disabled to ensure backward compatibility:
//!
//! ```rust,ignore
//! use llm_memory_graph::integrations::schema_registry::{
//!     SchemaRegistryClient, SchemaRegistryConfig
//! };
//!
//! // Validation is disabled by default
//! let config = SchemaRegistryConfig::default();
//! let client = SchemaRegistryClient::new(config)?;
//!
//! // All validation calls pass through without checking
//! let data = serde_json::json!({"any": "data"});
//! let result = client.validate("schema-id", None, &data).await?;
//! assert!(result.is_valid()); // Always true when disabled
//! ```
//!
//! ## 2. Enable Validation with Graceful Degradation
//!
//! Enable validation but continue operation if registry is unavailable:
//!
//! ```rust,ignore
//! use llm_memory_graph::integrations::schema_registry::SchemaRegistryConfig;
//!
//! let config = SchemaRegistryConfig::new("https://schema-registry.example.com")
//!     .with_validation_enabled(true)
//!     .with_fail_on_validation_error(false); // Graceful degradation
//!
//! let client = SchemaRegistryClient::new(config)?;
//!
//! // If registry is unavailable, validation passes with warnings logged
//! let result = client.validate("schema-id", None, &data).await?;
//! ```
//!
//! ## 3. Strict Validation Mode
//!
//! Enable validation and fail operations on validation errors:
//!
//! ```rust,ignore
//! let config = SchemaRegistryConfig::new("https://schema-registry.example.com")
//!     .with_validation_enabled(true)
//!     .with_fail_on_validation_error(true); // Strict mode
//!
//! let client = SchemaRegistryClient::new(config)?;
//!
//! // Validation errors will cause operations to fail
//! match client.validate("schema-id", None, &data).await {
//!     Ok(result) if result.is_valid() => println!("Valid!"),
//!     Ok(result) => println!("Invalid: {:?}", result.errors()),
//!     Err(e) => println!("Registry error: {}", e),
//! }
//! ```
//!
//! ## 4. Schema Registration
//!
//! Register new schemas or schema versions:
//!
//! ```rust,ignore
//! use llm_memory_graph::integrations::schema_registry::{
//!     SchemaRegistration, SchemaFormat
//! };
//!
//! let schema = serde_json::json!({
//!     "type": "object",
//!     "properties": {
//!         "prompt": {"type": "string"},
//!         "model": {"type": "string"},
//!         "temperature": {"type": "number", "minimum": 0, "maximum": 2}
//!     },
//!     "required": ["prompt", "model"]
//! });
//!
//! let registration = SchemaRegistration::new(
//!     "llm-prompt-v1",
//!     "1.0.0",
//!     SchemaFormat::JsonSchema,
//!     schema
//! ).with_description("LLM prompt schema");
//!
//! let metadata = client.register_schema(registration).await?;
//! ```
//!
//! ## 5. Caching for Performance
//!
//! Enable caching to reduce load on the Schema Registry:
//!
//! ```rust,ignore
//! use std::time::Duration;
//!
//! let config = SchemaRegistryConfig::new("https://schema-registry.example.com")
//!     .with_validation_enabled(true)
//!     .with_cache_ttl(Duration::from_secs(300)) // 5 minutes
//!     .with_max_cache_size(100);
//!
//! let client = SchemaRegistryClient::new(config)?;
//!
//! // First validation - fetches from registry
//! client.validate("schema-id", None, &data).await?;
//!
//! // Second validation - uses cached result
//! client.validate("schema-id", None, &data).await?;
//! ```
//!
//! # Integration with AsyncMemoryGraph
//!
//! While this module is standalone, it's designed to integrate with AsyncMemoryGraph:
//!
//! ```rust,ignore
//! // Future integration (not yet implemented)
//! use llm_memory_graph::{AsyncMemoryGraph, Config};
//! use llm_memory_graph::integrations::schema_registry::SchemaRegistryConfig;
//!
//! let mut graph_config = Config::default();
//!
//! // Configure Schema Registry integration
//! let schema_config = SchemaRegistryConfig::new("https://schema-registry.example.com")
//!     .with_validation_enabled(true);
//!
//! // Future: graph_config.with_schema_registry(schema_config);
//!
//! let graph = AsyncMemoryGraph::open(graph_config).await?;
//!
//! // Prompts and responses will be automatically validated
//! let prompt_id = graph.add_prompt(session_id, "text", None).await?;
//! ```
//!
//! # Design Principles
//!
//! 1. **Backward Compatibility**: No breaking changes to existing code
//! 2. **OPT-IN**: Validation is disabled by default
//! 3. **Graceful Degradation**: Continue operation when registry is unavailable
//! 4. **Performance**: Caching to minimize registry load
//! 5. **Observability**: Comprehensive logging and metrics
//! 6. **Testability**: All components are mockable and testable
//!
//! # Error Handling
//!
//! The module provides several layers of error handling:
//!
//! - **NoOpValidator**: Always succeeds (default)
//! - **GracefulValidator**: Logs errors but allows operations to continue
//! - **Strict Mode**: Fails operations on validation errors
//! - **Retry Logic**: Automatic retry with exponential backoff
//!
//! # Performance Considerations
//!
//! - **Caching**: Results are cached to reduce registry load
//! - **Connection Pooling**: HTTP client reuses connections
//! - **Async I/O**: Non-blocking operations throughout
//! - **Batch Operations**: Support for validating multiple items (future)

pub mod client;
pub mod config;
pub mod types;
pub mod validator;

// Re-export main types
pub use client::SchemaRegistryClient;
pub use config::SchemaRegistryConfig;
pub use types::{
    CompatibilityLevel, CompatibilityRequest, CompatibilityResponse, SchemaFormat,
    SchemaListResponse, SchemaMetadata, SchemaRegistration, ValidationError, ValidationOptions,
    ValidationRequest, ValidationResponse, ValidationResult,
};
pub use validator::{
    CachingValidator, GracefulValidator, NoOpValidator, SchemaValidator,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify that all main types are exported and accessible
        let _config = SchemaRegistryConfig::default();
        let _validator = NoOpValidator::new();
    }

    #[test]
    fn test_validation_result_usage() {
        let result = ValidationResult::Valid;
        assert!(result.is_valid());

        let errors = vec![ValidationError::new("field", "message", "code")];
        let result = ValidationResult::Invalid(errors);
        assert!(result.is_invalid());
    }

    #[test]
    fn test_schema_format_usage() {
        let format = SchemaFormat::JsonSchema;
        assert_eq!(format.to_string(), "json-schema");
    }

    #[test]
    fn test_compatibility_level_usage() {
        let level = CompatibilityLevel::Backward;
        assert_eq!(level.to_string(), "BACKWARD");
    }
}

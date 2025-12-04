//! Type definitions for Schema Registry integration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Schema validation passed
    Valid,
    /// Schema validation failed with errors
    Invalid(Vec<ValidationError>),
}

impl ValidationResult {
    /// Check if the validation result is valid
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }

    /// Check if the validation result is invalid
    pub fn is_invalid(&self) -> bool {
        matches!(self, ValidationResult::Invalid(_))
    }

    /// Get validation errors if invalid
    pub fn errors(&self) -> Option<&[ValidationError]> {
        match self {
            ValidationResult::Invalid(errors) => Some(errors),
            ValidationResult::Valid => None,
        }
    }

    /// Convert to Result type
    pub fn into_result(self) -> Result<(), Vec<ValidationError>> {
        match self {
            ValidationResult::Valid => Ok(()),
            ValidationResult::Invalid(errors) => Err(errors),
        }
    }
}

/// Schema validation error
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationError {
    /// Field path where error occurred (e.g., "metadata.model_id")
    pub field: String,
    /// Error message describing the validation failure
    pub message: String,
    /// Error code for programmatic handling
    pub code: String,
    /// Additional context about the error
    #[serde(default)]
    pub context: HashMap<String, String>,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(field: impl Into<String>, message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            code: code.into(),
            context: HashMap::new(),
        }
    }

    /// Add context to the validation error
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} (code: {})", self.field, self.message, self.code)
    }
}

impl std::error::Error for ValidationError {}

/// Schema metadata from the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    /// Schema identifier
    pub schema_id: String,
    /// Schema version
    pub version: String,
    /// Schema format (e.g., "json-schema", "avro", "protobuf")
    pub format: SchemaFormat,
    /// Schema definition (JSON-encoded)
    pub schema: serde_json::Value,
    /// When the schema was created
    pub created_at: DateTime<Utc>,
    /// When the schema was last updated
    pub updated_at: DateTime<Utc>,
    /// Schema description
    #[serde(default)]
    pub description: String,
    /// Schema tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Additional custom metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Schema format types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SchemaFormat {
    /// JSON Schema (draft 7 or later)
    JsonSchema,
    /// Apache Avro schema
    Avro,
    /// Protocol Buffers
    Protobuf,
    /// Custom schema format
    Custom,
}

impl std::fmt::Display for SchemaFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaFormat::JsonSchema => write!(f, "json-schema"),
            SchemaFormat::Avro => write!(f, "avro"),
            SchemaFormat::Protobuf => write!(f, "protobuf"),
            SchemaFormat::Custom => write!(f, "custom"),
        }
    }
}

/// Schema registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRegistration {
    /// Schema identifier
    pub schema_id: String,
    /// Schema version
    pub version: String,
    /// Schema format
    pub format: SchemaFormat,
    /// Schema definition (JSON-encoded)
    pub schema: serde_json::Value,
    /// Schema description
    #[serde(default)]
    pub description: String,
    /// Schema tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SchemaRegistration {
    /// Create a new schema registration
    pub fn new(
        schema_id: impl Into<String>,
        version: impl Into<String>,
        format: SchemaFormat,
        schema: serde_json::Value,
    ) -> Self {
        Self {
            schema_id: schema_id.into(),
            version: version.into(),
            format,
            schema,
            description: String::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Schema validation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRequest {
    /// Schema identifier to validate against
    pub schema_id: String,
    /// Schema version (optional, uses latest if not specified)
    pub version: Option<String>,
    /// Data to validate (JSON-encoded)
    pub data: serde_json::Value,
    /// Validation options
    #[serde(default)]
    pub options: ValidationOptions,
}

impl ValidationRequest {
    /// Create a new validation request
    pub fn new(schema_id: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            schema_id: schema_id.into(),
            version: None,
            data,
            options: ValidationOptions::default(),
        }
    }

    /// Set the schema version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set validation options
    pub fn with_options(mut self, options: ValidationOptions) -> Self {
        self.options = options;
        self
    }
}

/// Validation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationOptions {
    /// Strict mode - fail on additional properties not defined in schema
    #[serde(default)]
    pub strict: bool,
    /// Collect all errors (vs. fail fast on first error)
    #[serde(default = "default_true")]
    pub collect_all_errors: bool,
    /// Maximum validation depth for nested structures
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    /// Custom validation rules
    #[serde(default)]
    pub custom_rules: HashMap<String, serde_json::Value>,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            strict: false,
            collect_all_errors: true,
            max_depth: 100,
            custom_rules: HashMap::new(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_depth() -> usize {
    100
}

/// Schema validation response from the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResponse {
    /// Whether validation passed
    pub valid: bool,
    /// Validation errors (if any)
    #[serde(default)]
    pub errors: Vec<ValidationError>,
    /// Schema ID that was used for validation
    pub schema_id: String,
    /// Schema version that was used
    pub version: String,
    /// Validation timestamp
    pub validated_at: DateTime<Utc>,
}

impl ValidationResponse {
    /// Convert to ValidationResult
    pub fn into_result(self) -> ValidationResult {
        if self.valid {
            ValidationResult::Valid
        } else {
            ValidationResult::Invalid(self.errors)
        }
    }
}

/// Schema list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaListResponse {
    /// List of schemas
    pub schemas: Vec<SchemaMetadata>,
    /// Total count
    pub total: usize,
    /// Current page
    #[serde(default)]
    pub page: usize,
    /// Page size
    #[serde(default)]
    pub page_size: usize,
}

/// Schema compatibility check request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityRequest {
    /// Base schema ID
    pub schema_id: String,
    /// Base schema version
    pub base_version: String,
    /// New schema to check compatibility with
    pub new_schema: serde_json::Value,
    /// Compatibility level to enforce
    pub compatibility_level: CompatibilityLevel,
}

/// Schema compatibility levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompatibilityLevel {
    /// No compatibility checking
    None,
    /// New schema is backward compatible (can read old data)
    Backward,
    /// New schema is forward compatible (old schema can read new data)
    Forward,
    /// New schema is both backward and forward compatible
    Full,
    /// New schema is backward compatible with all previous versions
    BackwardTransitive,
    /// New schema is forward compatible with all previous versions
    ForwardTransitive,
    /// New schema is fully compatible with all previous versions
    FullTransitive,
}

impl std::fmt::Display for CompatibilityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompatibilityLevel::None => write!(f, "NONE"),
            CompatibilityLevel::Backward => write!(f, "BACKWARD"),
            CompatibilityLevel::Forward => write!(f, "FORWARD"),
            CompatibilityLevel::Full => write!(f, "FULL"),
            CompatibilityLevel::BackwardTransitive => write!(f, "BACKWARD_TRANSITIVE"),
            CompatibilityLevel::ForwardTransitive => write!(f, "FORWARD_TRANSITIVE"),
            CompatibilityLevel::FullTransitive => write!(f, "FULL_TRANSITIVE"),
        }
    }
}

/// Schema compatibility check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityResponse {
    /// Whether schemas are compatible
    pub compatible: bool,
    /// Compatibility issues (if any)
    #[serde(default)]
    pub issues: Vec<String>,
    /// Compatibility level that was checked
    pub level: CompatibilityLevel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_is_valid() {
        let result = ValidationResult::Valid;
        assert!(result.is_valid());
        assert!(!result.is_invalid());
        assert!(result.errors().is_none());
    }

    #[test]
    fn test_validation_result_is_invalid() {
        let errors = vec![ValidationError::new("field", "error", "ERR001")];
        let result = ValidationResult::Invalid(errors.clone());
        assert!(!result.is_valid());
        assert!(result.is_invalid());
        assert_eq!(result.errors(), Some(errors.as_slice()));
    }

    #[test]
    fn test_validation_error_with_context() {
        let error = ValidationError::new("metadata.model_id", "Missing required field", "REQUIRED")
            .with_context("expected_type", "string")
            .with_context("actual_type", "null");

        assert_eq!(error.field, "metadata.model_id");
        assert_eq!(error.code, "REQUIRED");
        assert_eq!(error.context.get("expected_type"), Some(&"string".to_string()));
        assert_eq!(error.context.get("actual_type"), Some(&"null".to_string()));
    }

    #[test]
    fn test_schema_format_display() {
        assert_eq!(SchemaFormat::JsonSchema.to_string(), "json-schema");
        assert_eq!(SchemaFormat::Avro.to_string(), "avro");
        assert_eq!(SchemaFormat::Protobuf.to_string(), "protobuf");
    }

    #[test]
    fn test_schema_registration_builder() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let registration = SchemaRegistration::new("test-schema", "1.0.0", SchemaFormat::JsonSchema, schema)
            .with_description("Test schema")
            .with_tag("production")
            .with_metadata("author", serde_json::json!("test-user"));

        assert_eq!(registration.schema_id, "test-schema");
        assert_eq!(registration.version, "1.0.0");
        assert_eq!(registration.format, SchemaFormat::JsonSchema);
        assert_eq!(registration.description, "Test schema");
        assert_eq!(registration.tags.len(), 1);
        assert!(registration.metadata.contains_key("author"));
    }

    #[test]
    fn test_validation_request_builder() {
        let data = serde_json::json!({"name": "test"});
        let request = ValidationRequest::new("test-schema", data)
            .with_version("1.0.0")
            .with_options(ValidationOptions {
                strict: true,
                collect_all_errors: false,
                max_depth: 50,
                custom_rules: HashMap::new(),
            });

        assert_eq!(request.schema_id, "test-schema");
        assert_eq!(request.version, Some("1.0.0".to_string()));
        assert!(request.options.strict);
        assert!(!request.options.collect_all_errors);
        assert_eq!(request.options.max_depth, 50);
    }

    #[test]
    fn test_validation_options_default() {
        let options = ValidationOptions::default();
        assert!(!options.strict);
        assert!(options.collect_all_errors);
        assert_eq!(options.max_depth, 100);
    }

    #[test]
    fn test_compatibility_level_display() {
        assert_eq!(CompatibilityLevel::Backward.to_string(), "BACKWARD");
        assert_eq!(CompatibilityLevel::Forward.to_string(), "FORWARD");
        assert_eq!(CompatibilityLevel::Full.to_string(), "FULL");
        assert_eq!(CompatibilityLevel::BackwardTransitive.to_string(), "BACKWARD_TRANSITIVE");
    }

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError::new("test_field", "Test error message", "TEST001");
        let display = format!("{}", error);
        assert!(display.contains("test_field"));
        assert!(display.contains("Test error message"));
        assert!(display.contains("TEST001"));
    }

    #[test]
    fn test_validation_response_into_result() {
        let response = ValidationResponse {
            valid: true,
            errors: vec![],
            schema_id: "test".to_string(),
            version: "1.0.0".to_string(),
            validated_at: Utc::now(),
        };
        assert!(response.into_result().is_valid());

        let response = ValidationResponse {
            valid: false,
            errors: vec![ValidationError::new("field", "error", "ERR")],
            schema_id: "test".to_string(),
            version: "1.0.0".to_string(),
            validated_at: Utc::now(),
        };
        assert!(response.into_result().is_invalid());
    }
}

//! Example content validation plugin for LLM-Memory-Graph
//!
//! This plugin demonstrates how to implement content validation hooks
//! to enforce business rules and data quality standards.
//!
//! # Features
//!
//! - Content length validation
//! - Character set validation
//! - Profanity filtering (basic example)
//! - Custom validation rules
//!
//! # Usage
//!
//! ```rust
//! use example_validator::ValidationPlugin;
//! use llm_memory_graph::plugin::PluginManager;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut manager = PluginManager::new();
//! let validator = Arc::new(ValidationPlugin::new());
//! manager.register(validator).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use llm_memory_graph::plugin::{Plugin, PluginBuilder, PluginContext, PluginError, PluginMetadata};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{debug, warn};

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Maximum content length
    pub max_content_length: usize,
    /// Minimum content length
    pub min_content_length: usize,
    /// Whether to check for profanity
    pub check_profanity: bool,
    /// Whether to validate character sets
    pub validate_charset: bool,
    /// Custom blocked words
    pub blocked_words: Vec<String>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_content_length: 10000,
            min_content_length: 1,
            check_profanity: true,
            validate_charset: true,
            blocked_words: Vec::new(),
        }
    }
}

/// Content validation plugin
///
/// Validates prompts, responses, and other content against configurable rules.
pub struct ValidationPlugin {
    metadata: PluginMetadata,
    config: ValidationConfig,
    content_regex: Regex,
    profanity_list: HashSet<String>,
}

impl ValidationPlugin {
    /// Create a new validation plugin with default configuration
    pub fn new() -> Self {
        Self::with_config(ValidationConfig::default())
    }

    /// Create a validation plugin with custom configuration
    pub fn with_config(config: ValidationConfig) -> Self {
        let metadata = PluginBuilder::new("content_validator", "1.0.0")
            .author("LLM DevOps Team")
            .description("Validates prompt and response content against business rules")
            .capability("validation")
            .capability("content_filtering")
            .capability("quality_assurance")
            .build();

        // Basic character set validation (alphanumeric + common punctuation + whitespace)
        let content_regex = Regex::new(r"^[\w\s\.,!?;:\-'\"()\[\]{}@#$%^&*+=<>/\\|`~\n\r\t]+$")
            .expect("Failed to compile content regex");

        // Example profanity list (very basic - in production, use a comprehensive list)
        let profanity_list: HashSet<String> = vec![
            "badword1".to_string(),
            "badword2".to_string(),
            "spam".to_string(),
        ]
        .into_iter()
        .collect();

        Self {
            metadata,
            config,
            content_regex,
            profanity_list,
        }
    }

    /// Validate content length
    fn validate_length(&self, content: &str) -> Result<(), PluginError> {
        let len = content.len();

        if len < self.config.min_content_length {
            return Err(PluginError::HookFailed(format!(
                "Content too short: {} characters (minimum: {})",
                len, self.config.min_content_length
            )));
        }

        if len > self.config.max_content_length {
            return Err(PluginError::HookFailed(format!(
                "Content too long: {} characters (maximum: {})",
                len, self.config.max_content_length
            )));
        }

        Ok(())
    }

    /// Validate character set
    fn validate_charset(&self, content: &str) -> Result<(), PluginError> {
        if !self.config.validate_charset {
            return Ok(());
        }

        if !self.content_regex.is_match(content) {
            return Err(PluginError::HookFailed(
                "Content contains invalid characters".to_string(),
            ));
        }

        Ok(())
    }

    /// Check for profanity
    fn check_profanity(&self, content: &str) -> Result<(), PluginError> {
        if !self.config.check_profanity {
            return Ok(());
        }

        let content_lower = content.to_lowercase();
        let words: Vec<&str> = content_lower.split_whitespace().collect();

        for word in words {
            if self.profanity_list.contains(word) {
                warn!("Profanity detected in content: {}", word);
                return Err(PluginError::HookFailed(
                    "Content contains inappropriate language".to_string(),
                ));
            }
        }

        // Check custom blocked words
        for blocked_word in &self.config.blocked_words {
            if content_lower.contains(&blocked_word.to_lowercase()) {
                warn!("Blocked word detected: {}", blocked_word);
                return Err(PluginError::HookFailed(format!(
                    "Content contains blocked word: {}",
                    blocked_word
                )));
            }
        }

        Ok(())
    }

    /// Validate content
    fn validate_content(&self, content: &str) -> Result<(), PluginError> {
        debug!("Validating content (length: {})", content.len());

        self.validate_length(content)?;
        self.validate_charset(content)?;
        self.check_profanity(content)?;

        debug!("Content validation passed");
        Ok(())
    }

    /// Extract content from context
    fn extract_content(&self, context: &PluginContext) -> Option<String> {
        // Try different fields where content might be
        context
            .data()
            .get("content")
            .or_else(|| context.data().get("text"))
            .or_else(|| context.data().get("body"))
            .and_then(|v| v.as_str())
            .map(String::from)
    }
}

impl Default for ValidationPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for ValidationPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn init(&self) -> Result<(), PluginError> {
        tracing::info!(
            "ValidationPlugin initialized with max_length={}, profanity_check={}",
            self.config.max_content_length,
            self.config.check_profanity
        );
        Ok(())
    }

    async fn before_create_node(&self, context: &PluginContext) -> Result<(), PluginError> {
        debug!("ValidationPlugin: before_create_node hook");

        // Extract and validate content if present
        if let Some(content) = self.extract_content(context) {
            self.validate_content(&content)?;
        }

        Ok(())
    }

    async fn before_create_session(&self, context: &PluginContext) -> Result<(), PluginError> {
        debug!("ValidationPlugin: before_create_session hook");

        // Validate session metadata if needed
        if let Some(metadata) = context.data().get("metadata") {
            if let Some(metadata_obj) = metadata.as_object() {
                // Check for required fields or validate metadata content
                debug!("Session metadata validated: {} fields", metadata_obj.len());
            }
        }

        Ok(())
    }

    async fn before_query(&self, context: &PluginContext) -> Result<(), PluginError> {
        debug!("ValidationPlugin: before_query hook");

        // Validate query parameters
        if let Some(filters) = context.data().get("filters") {
            if let Some(filters_obj) = filters.as_object() {
                for (key, value) in filters_obj {
                    if let Some(value_str) = value.as_str() {
                        // Prevent injection attacks
                        if value_str.contains("';") || value_str.contains("--") {
                            return Err(PluginError::HookFailed(format!(
                                "Suspicious query filter detected: {}",
                                key
                            )));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Validation rules builder
///
/// Provides a fluent API for building custom validation rules.
pub struct ValidationRulesBuilder {
    config: ValidationConfig,
}

impl ValidationRulesBuilder {
    /// Create a new rules builder
    pub fn new() -> Self {
        Self {
            config: ValidationConfig::default(),
        }
    }

    /// Set maximum content length
    pub fn max_length(mut self, max: usize) -> Self {
        self.config.max_content_length = max;
        self
    }

    /// Set minimum content length
    pub fn min_length(mut self, min: usize) -> Self {
        self.config.min_content_length = min;
        self
    }

    /// Enable or disable profanity checking
    pub fn check_profanity(mut self, enabled: bool) -> Self {
        self.config.check_profanity = enabled;
        self
    }

    /// Enable or disable character set validation
    pub fn validate_charset(mut self, enabled: bool) -> Self {
        self.config.validate_charset = enabled;
        self
    }

    /// Add a blocked word
    pub fn block_word(mut self, word: impl Into<String>) -> Self {
        self.config.blocked_words.push(word.into());
        self
    }

    /// Build the validation plugin
    pub fn build(self) -> ValidationPlugin {
        ValidationPlugin::with_config(self.config)
    }
}

impl Default for ValidationRulesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_content_validation_success() {
        let plugin = ValidationPlugin::new();
        let context = PluginContext::new(
            "test",
            json!({
                "content": "This is valid content."
            }),
        );

        assert!(plugin.before_create_node(&context).await.is_ok());
    }

    #[tokio::test]
    async fn test_content_too_long() {
        let plugin = ValidationPlugin::with_config(ValidationConfig {
            max_content_length: 10,
            ..Default::default()
        });

        let context = PluginContext::new(
            "test",
            json!({
                "content": "This is way too long content that exceeds the limit"
            }),
        );

        assert!(plugin.before_create_node(&context).await.is_err());
    }

    #[tokio::test]
    async fn test_profanity_detection() {
        let plugin = ValidationPlugin::new();
        let context = PluginContext::new(
            "test",
            json!({
                "content": "This contains spam content"
            }),
        );

        assert!(plugin.before_create_node(&context).await.is_err());
    }

    #[test]
    fn test_validation_rules_builder() {
        let plugin = ValidationRulesBuilder::new()
            .max_length(5000)
            .min_length(10)
            .check_profanity(false)
            .block_word("custom_blocked")
            .build();

        assert_eq!(plugin.config.max_content_length, 5000);
        assert_eq!(plugin.config.min_content_length, 10);
        assert!(!plugin.config.check_profanity);
        assert_eq!(plugin.config.blocked_words.len(), 1);
    }
}

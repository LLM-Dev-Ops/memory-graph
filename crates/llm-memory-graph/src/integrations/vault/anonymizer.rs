//! Data anonymization pipeline for PII detection and sanitization
//!
//! Provides automated detection and anonymization of personally identifiable
//! information (PII) before archival to the vault, ensuring compliance with
//! data protection regulations.

use super::config::{
    AnonymizationConfig, AnonymizationStrategy, CustomPattern, PiiType, VaultStorageConfig,
};
use crate::integrations::IntegrationError;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, warn};

/// Anonymizer for detecting and sanitizing PII in data
pub struct DataAnonymizer {
    config: AnonymizationConfig,
    patterns: HashMap<PiiType, Regex>,
    custom_patterns: Vec<CompiledCustomPattern>,
}

/// Compiled custom pattern with regex
struct CompiledCustomPattern {
    name: String,
    regex: Regex,
    strategy: AnonymizationStrategy,
}

impl DataAnonymizer {
    /// Create a new data anonymizer
    ///
    /// # Errors
    /// Returns an error if regex patterns fail to compile
    pub fn new(config: AnonymizationConfig) -> Result<Self, IntegrationError> {
        let mut patterns = HashMap::new();

        // Compile built-in PII patterns
        for pii_type in &config.pii_types {
            if let Some(pattern) = Self::get_pattern_for_type(*pii_type) {
                let regex = Regex::new(&pattern).map_err(|e| {
                    IntegrationError::ConfigError(format!(
                        "Failed to compile regex for {:?}: {}",
                        pii_type, e
                    ))
                })?;
                patterns.insert(*pii_type, regex);
            }
        }

        // Compile custom patterns
        let mut custom_patterns = Vec::new();
        for custom in &config.custom_patterns {
            let regex = Regex::new(&custom.pattern).map_err(|e| {
                IntegrationError::ConfigError(format!(
                    "Failed to compile custom pattern '{}': {}",
                    custom.name, e
                ))
            })?;
            custom_patterns.push(CompiledCustomPattern {
                name: custom.name.clone(),
                regex,
                strategy: custom.strategy,
            });
        }

        Ok(Self {
            config,
            patterns,
            custom_patterns,
        })
    }

    /// Create anonymizer from vault storage config
    pub fn from_vault_config(config: &VaultStorageConfig) -> Result<Self, IntegrationError> {
        Self::new(config.anonymization.clone())
    }

    /// Anonymize a JSON value, replacing PII with anonymized versions
    ///
    /// # Errors
    /// Returns an error if anonymization fails
    pub fn anonymize(&self, value: &Value) -> Result<Value, IntegrationError> {
        if !self.config.enabled {
            return Ok(value.clone());
        }

        self.anonymize_value(value)
    }

    /// Recursively anonymize a JSON value
    fn anonymize_value(&self, value: &Value) -> Result<Value, IntegrationError> {
        match value {
            Value::String(s) => Ok(Value::String(self.anonymize_string(s)?)),
            Value::Array(arr) => {
                let anonymized: Result<Vec<_>, _> =
                    arr.iter().map(|v| self.anonymize_value(v)).collect();
                Ok(Value::Array(anonymized?))
            }
            Value::Object(obj) => {
                let mut anonymized = serde_json::Map::new();
                for (key, val) in obj {
                    // Anonymize both keys and values
                    let anon_key = self.anonymize_string(key)?;
                    let anon_val = self.anonymize_value(val)?;
                    anonymized.insert(anon_key, anon_val);
                }
                Ok(Value::Object(anonymized))
            }
            // Numbers, booleans, and null pass through unchanged
            _ => Ok(value.clone()),
        }
    }

    /// Anonymize a string, detecting and replacing PII
    fn anonymize_string(&self, text: &str) -> Result<String, IntegrationError> {
        let mut result = text.to_string();

        // Apply built-in PII patterns
        for (pii_type, regex) in &self.patterns {
            result = self.apply_pattern(&result, *pii_type, regex, self.config.strategy)?;
        }

        // Apply custom patterns
        for custom in &self.custom_patterns {
            result = self.apply_pattern(&result, PiiType::Email, &custom.regex, custom.strategy)?;
        }

        Ok(result)
    }

    /// Apply a pattern and anonymize matches
    fn apply_pattern(
        &self,
        text: &str,
        pii_type: PiiType,
        regex: &Regex,
        strategy: AnonymizationStrategy,
    ) -> Result<String, IntegrationError> {
        let result = regex.replace_all(text, |caps: &regex::Captures| {
            let matched = caps.get(0).map_or("", |m| m.as_str());
            self.anonymize_match(matched, pii_type, strategy)
                .unwrap_or_else(|_| "[REDACTED]".to_string())
        });

        Ok(result.into_owned())
    }

    /// Anonymize a single matched PII instance
    fn anonymize_match(
        &self,
        matched: &str,
        pii_type: PiiType,
        strategy: AnonymizationStrategy,
    ) -> Result<String, IntegrationError> {
        match strategy {
            AnonymizationStrategy::Redact => Ok(Self::redact(matched, pii_type)),
            AnonymizationStrategy::Hash => Ok(Self::hash(matched, pii_type)),
            AnonymizationStrategy::Randomize => Ok(Self::randomize(matched, pii_type)),
            AnonymizationStrategy::Encrypt => {
                // Simplified encryption placeholder
                Ok(format!("[ENCRYPTED:{}]", Self::hash(matched, pii_type)))
            }
            AnonymizationStrategy::Tokenize => {
                // Simplified tokenization
                Ok(format!("[TOKEN:{}]", Self::hash(matched, pii_type)))
            }
        }
    }

    /// Redact PII with placeholder
    fn redact(matched: &str, pii_type: PiiType) -> String {
        match pii_type {
            PiiType::Email => "[EMAIL_REDACTED]".to_string(),
            PiiType::PhoneNumber => "[PHONE_REDACTED]".to_string(),
            PiiType::CreditCard => "[CARD_REDACTED]".to_string(),
            PiiType::SocialSecurity => "[SSN_REDACTED]".to_string(),
            PiiType::IpAddress => "[IP_REDACTED]".to_string(),
            PiiType::PhysicalAddress => "[ADDRESS_REDACTED]".to_string(),
            PiiType::PersonName => "[NAME_REDACTED]".to_string(),
            PiiType::DateOfBirth => "[DOB_REDACTED]".to_string(),
            PiiType::ApiKey => "[KEY_REDACTED]".to_string(),
        }
    }

    /// Hash PII deterministically
    fn hash(matched: &str, pii_type: PiiType) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        matched.hash(&mut hasher);
        let hash = hasher.finish();

        match pii_type {
            PiiType::Email => format!("user_{}@redacted.com", hash),
            PiiType::PhoneNumber => format!("+1-555-{:08}", hash % 100000000),
            PiiType::CreditCard => format!("****-****-****-{:04}", hash % 10000),
            PiiType::SocialSecurity => format!("***-**-{:04}", hash % 10000),
            PiiType::IpAddress => {
                let a = (hash % 256) as u8;
                let b = ((hash >> 8) % 256) as u8;
                format!("10.{}.{}.***", a, b)
            }
            PiiType::PhysicalAddress => format!("[ADDRESS_HASH_{}]", hash),
            PiiType::PersonName => format!("[NAME_{}]", hash),
            PiiType::DateOfBirth => format!("[DOB_{}]", hash),
            PiiType::ApiKey => format!("sk_hash_{}", hash),
        }
    }

    /// Randomize PII while preserving format
    fn randomize(matched: &str, pii_type: PiiType) -> String {
        // For now, use hash-based randomization
        // In production, could use cryptographically secure random values
        Self::hash(matched, pii_type)
    }

    /// Get regex pattern for a PII type
    fn get_pattern_for_type(pii_type: PiiType) -> Option<String> {
        match pii_type {
            PiiType::Email => Some(
                r#"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"#.to_string(),
            ),
            PiiType::PhoneNumber => Some(
                r#"(\+\d{1,3}[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}"#.to_string(),
            ),
            PiiType::CreditCard => Some(
                r#"\b(?:\d{4}[-\s]?){3}\d{4}\b"#.to_string(),
            ),
            PiiType::SocialSecurity => Some(
                r#"\b\d{3}-\d{2}-\d{4}\b"#.to_string(),
            ),
            PiiType::IpAddress => Some(
                r#"\b(?:\d{1,3}\.){3}\d{1,3}\b"#.to_string(),
            ),
            PiiType::ApiKey => Some(
                r#"\b(sk|pk|api)[-_][a-zA-Z0-9]{32,}\b"#.to_string(),
            ),
            // These require more sophisticated detection
            PiiType::PhysicalAddress | PiiType::PersonName | PiiType::DateOfBirth => None,
        }
    }

    /// Detect PII in text without anonymizing (for reporting)
    pub fn detect_pii(&self, text: &str) -> Vec<PiiDetection> {
        let mut detections = Vec::new();

        for (pii_type, regex) in &self.patterns {
            for capture in regex.captures_iter(text) {
                if let Some(matched) = capture.get(0) {
                    detections.push(PiiDetection {
                        pii_type: *pii_type,
                        matched_text: matched.as_str().to_string(),
                        start: matched.start(),
                        end: matched.end(),
                    });
                }
            }
        }

        detections
    }

    /// Check if data contains any PII
    pub fn contains_pii(&self, value: &Value) -> bool {
        self.detect_pii_in_value(value)
    }

    /// Recursively check for PII in JSON value
    fn detect_pii_in_value(&self, value: &Value) -> bool {
        match value {
            Value::String(s) => !self.detect_pii(s).is_empty(),
            Value::Array(arr) => arr.iter().any(|v| self.detect_pii_in_value(v)),
            Value::Object(obj) => obj.values().any(|v| self.detect_pii_in_value(v)),
            _ => false,
        }
    }
}

/// PII detection result
#[derive(Debug, Clone)]
pub struct PiiDetection {
    /// Type of PII detected
    pub pii_type: PiiType,
    /// Matched text
    pub matched_text: String,
    /// Start position in text
    pub start: usize,
    /// End position in text
    pub end: usize,
}

/// Anonymization result with statistics
#[derive(Debug, Clone)]
pub struct AnonymizationResult {
    /// Anonymized data
    pub data: Value,
    /// Number of PII instances detected and anonymized
    pub pii_count: usize,
    /// Types of PII found
    pub pii_types_found: Vec<PiiType>,
}

impl DataAnonymizer {
    /// Anonymize with detailed result
    pub fn anonymize_with_stats(&self, value: &Value) -> Result<AnonymizationResult, IntegrationError> {
        let detections = if let Value::String(s) = value {
            self.detect_pii(s)
        } else {
            Vec::new()
        };

        let anonymized = self.anonymize(value)?;

        let mut pii_types_found: Vec<_> = detections
            .iter()
            .map(|d| d.pii_type)
            .collect();
        pii_types_found.sort_by_key(|t| format!("{:?}", t));
        pii_types_found.dedup();

        Ok(AnonymizationResult {
            data: anonymized,
            pii_count: detections.len(),
            pii_types_found,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_anonymizer() -> DataAnonymizer {
        DataAnonymizer::new(AnonymizationConfig::default()).unwrap()
    }

    #[test]
    fn test_email_detection() {
        let anonymizer = create_test_anonymizer();
        let text = "Contact me at john.doe@example.com";
        let detections = anonymizer.detect_pii(text);

        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].pii_type, PiiType::Email);
        assert_eq!(detections[0].matched_text, "john.doe@example.com");
    }

    #[test]
    fn test_phone_detection() {
        let anonymizer = create_test_anonymizer();
        let text = "Call me at (555) 123-4567";
        let detections = anonymizer.detect_pii(text);

        assert!(!detections.is_empty());
        assert!(detections.iter().any(|d| d.pii_type == PiiType::PhoneNumber));
    }

    #[test]
    fn test_email_anonymization() {
        let anonymizer = create_test_anonymizer();
        let text = "Send email to alice@example.com";
        let result = anonymizer.anonymize_string(text).unwrap();

        assert!(!result.contains("alice@example.com"));
        assert!(result.contains("@redacted.com") || result.contains("[EMAIL"));
    }

    #[test]
    fn test_json_anonymization() {
        let anonymizer = create_test_anonymizer();
        let data = serde_json::json!({
            "user": {
                "email": "test@example.com",
                "phone": "555-123-4567"
            },
            "count": 42
        });

        let result = anonymizer.anonymize(&data).unwrap();
        assert!(result.is_object());

        let result_str = serde_json::to_string(&result).unwrap();
        assert!(!result_str.contains("test@example.com"));
    }

    #[test]
    fn test_disabled_anonymization() {
        let config = AnonymizationConfig {
            enabled: false,
            ..Default::default()
        };
        let anonymizer = DataAnonymizer::new(config).unwrap();

        let text = "test@example.com";
        let result = anonymizer.anonymize_string(text).unwrap();
        assert_eq!(result, text);
    }

    #[test]
    fn test_credit_card_detection() {
        let anonymizer = create_test_anonymizer();
        let text = "Card: 4532-1234-5678-9010";
        let detections = anonymizer.detect_pii(text);

        assert!(!detections.is_empty());
    }

    #[test]
    fn test_contains_pii() {
        let anonymizer = create_test_anonymizer();

        let with_pii = serde_json::json!({
            "message": "Email me at test@example.com"
        });
        assert!(anonymizer.contains_pii(&with_pii));

        let without_pii = serde_json::json!({
            "message": "Hello world"
        });
        assert!(!anonymizer.contains_pii(&without_pii));
    }

    #[test]
    fn test_anonymize_with_stats() {
        let anonymizer = create_test_anonymizer();
        let value = Value::String("Contact: alice@example.com or bob@test.org".to_string());

        let result = anonymizer.anonymize_with_stats(&value).unwrap();
        assert_eq!(result.pii_count, 2);
        assert!(result.pii_types_found.contains(&PiiType::Email));
    }

    #[test]
    fn test_custom_pattern() {
        let custom = CustomPattern {
            name: "Custom ID".to_string(),
            pattern: r"ID-\d{6}".to_string(),
            strategy: AnonymizationStrategy::Hash,
        };

        let config = AnonymizationConfig {
            enabled: true,
            custom_patterns: vec![custom],
            ..Default::default()
        };

        let anonymizer = DataAnonymizer::new(config).unwrap();
        let text = "User ID-123456 logged in";
        let result = anonymizer.anonymize_string(text).unwrap();

        assert!(!result.contains("ID-123456"));
    }
}

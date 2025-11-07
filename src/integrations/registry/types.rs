//! Type definitions for LLM-Registry integration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Registry client configuration
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Base URL of the LLM-Registry service
    pub base_url: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum number of retry attempts
    pub retry_count: usize,
    /// Enable request/response logging
    pub enable_logging: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("REGISTRY_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            api_key: std::env::var("REGISTRY_API_KEY").ok(),
            timeout_secs: 30,
            retry_count: 3,
            enable_logging: true,
        }
    }
}

impl RegistryConfig {
    /// Create a new registry configuration
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    /// Set the API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set the timeout in seconds
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Set the retry count
    pub fn with_retry_count(mut self, retry_count: usize) -> Self {
        self.retry_count = retry_count;
        self
    }

    /// Enable or disable logging
    pub fn with_logging(mut self, enable: bool) -> Self {
        self.enable_logging = enable;
        self
    }
}

/// Model metadata from the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Unique model identifier
    pub model_id: String,
    /// Model version
    pub version: String,
    /// Model provider (e.g., "openai", "anthropic", "cohere")
    pub provider: String,
    /// Model parameters/configuration
    pub parameters: ModelParameters,
    /// When the model was registered
    pub created_at: DateTime<Utc>,
    /// Model description
    #[serde(default)]
    pub description: String,
    /// Model capabilities
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Model tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Additional custom metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Model parameter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    /// Temperature setting (0.0 - 2.0)
    pub temperature: f64,
    /// Maximum tokens to generate
    pub max_tokens: Option<usize>,
    /// Top-p sampling parameter
    pub top_p: Option<f64>,
    /// Frequency penalty
    pub frequency_penalty: Option<f64>,
    /// Presence penalty
    pub presence_penalty: Option<f64>,
    /// Stop sequences
    #[serde(default)]
    pub stop_sequences: Vec<String>,
    /// Additional custom parameters
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            temperature: 1.0,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequences: Vec::new(),
            custom: HashMap::new(),
        }
    }
}

/// Session registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRegistration {
    /// Session ID from memory graph
    pub session_id: String,
    /// Model being used in this session
    pub model_id: String,
    /// Model version
    #[serde(default)]
    pub model_version: String,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Session metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

impl SessionRegistration {
    /// Create a new session registration
    pub fn new(session_id: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            model_id: model_id.into(),
            model_version: String::new(),
            started_at: Utc::now(),
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Add metadata to the session registration
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Add a tag to the session registration
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Usage statistics for a session or model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    /// Session or model ID
    pub id: String,
    /// Total prompts submitted
    pub total_prompts: u64,
    /// Total responses generated
    pub total_responses: u64,
    /// Total prompt tokens used
    pub total_prompt_tokens: i64,
    /// Total completion tokens used
    pub total_completion_tokens: i64,
    /// Total tokens used (prompt + completion)
    pub total_tokens: i64,
    /// Average response latency in milliseconds
    pub avg_latency_ms: f64,
    /// Statistics time range start
    pub from: DateTime<Utc>,
    /// Statistics time range end
    pub to: DateTime<Utc>,
    /// Additional metrics
    #[serde(default)]
    pub custom_metrics: HashMap<String, serde_json::Value>,
}

/// Usage tracking report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageReport {
    /// Session ID
    pub session_id: String,
    /// Model ID
    #[serde(default)]
    pub model_id: String,
    /// Number of prompt tokens
    pub prompt_tokens: i64,
    /// Number of completion tokens
    pub completion_tokens: i64,
    /// Total tokens
    pub total_tokens: i64,
    /// Response latency in milliseconds
    #[serde(default)]
    pub latency_ms: Option<i64>,
    /// Timestamp of usage
    pub timestamp: DateTime<Utc>,
    /// Additional context
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl UsageReport {
    /// Create a new usage report
    pub fn new(session_id: impl Into<String>, prompt_tokens: i64, completion_tokens: i64) -> Self {
        Self {
            session_id: session_id.into(),
            model_id: String::new(),
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            latency_ms: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Set the model ID
    pub fn with_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = model_id.into();
        self
    }

    /// Set the latency
    pub fn with_latency_ms(mut self, latency_ms: i64) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Model list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelListResponse {
    /// List of models
    pub models: Vec<ModelMetadata>,
    /// Total count
    pub total: usize,
    /// Current page
    #[serde(default)]
    pub page: usize,
    /// Page size
    #[serde(default)]
    pub page_size: usize,
}

/// Session list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionListResponse {
    /// List of sessions
    pub sessions: Vec<SessionInfo>,
    /// Total count
    pub total: usize,
    /// Current page
    #[serde(default)]
    pub page: usize,
    /// Page size
    #[serde(default)]
    pub page_size: usize,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID
    pub session_id: String,
    /// Model ID
    pub model_id: String,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Session end time (if ended)
    pub ended_at: Option<DateTime<Utc>>,
    /// Session status
    pub status: SessionStatus,
    /// Total prompts in session
    pub prompt_count: u64,
    /// Total tokens used
    pub total_tokens: i64,
    /// Session metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Session status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Session is active
    Active,
    /// Session has ended normally
    Completed,
    /// Session was terminated with an error
    Failed,
    /// Session was archived
    Archived,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_config_builder() {
        let config = RegistryConfig::new("http://registry.example.com")
            .with_api_key("test-key")
            .with_timeout(60)
            .with_retry_count(5)
            .with_logging(false);

        assert_eq!(config.base_url, "http://registry.example.com");
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.retry_count, 5);
        assert!(!config.enable_logging);
    }

    #[test]
    fn test_session_registration_builder() {
        let registration = SessionRegistration::new("session-123", "gpt-4")
            .with_tag("production")
            .with_tag("high-priority")
            .with_metadata("user_id", serde_json::json!("user-456"));

        assert_eq!(registration.session_id, "session-123");
        assert_eq!(registration.model_id, "gpt-4");
        assert_eq!(registration.tags.len(), 2);
        assert!(registration.metadata.contains_key("user_id"));
    }

    #[test]
    fn test_usage_report_builder() {
        let report = UsageReport::new("session-123", 100, 200)
            .with_model_id("gpt-4")
            .with_latency_ms(1500)
            .with_metadata("endpoint", serde_json::json!("/v1/chat/completions"));

        assert_eq!(report.session_id, "session-123");
        assert_eq!(report.model_id, "gpt-4");
        assert_eq!(report.prompt_tokens, 100);
        assert_eq!(report.completion_tokens, 200);
        assert_eq!(report.total_tokens, 300);
        assert_eq!(report.latency_ms, Some(1500));
        assert!(report.metadata.contains_key("endpoint"));
    }

    #[test]
    fn test_model_parameters_default() {
        let params = ModelParameters::default();
        assert_eq!(params.temperature, 1.0);
        assert!(params.max_tokens.is_none());
        assert!(params.top_p.is_none());
    }

    #[test]
    fn test_session_status_serialization() {
        let status = SessionStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"active\"");

        let status: SessionStatus = serde_json::from_str("\"completed\"").unwrap();
        assert_eq!(status, SessionStatus::Completed);
    }
}

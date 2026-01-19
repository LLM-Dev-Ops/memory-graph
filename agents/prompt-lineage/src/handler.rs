//! Prompt Lineage Agent - Edge Function Handler
//!
//! Classification: MEMORY WRITE
//! Decision Type: prompt_lineage_tracking
//!
//! This agent tracks prompt evolution and lineage across iterations and agents.
//! It creates LineageNode entries and evolution edges (Evolves, Refines, Derives)
//! with computed confidence scores for relationships.
//!
//! This agent:
//! - Tracks prompt evolution and lineage
//! - Creates LineageNode entries in the memory graph
//! - Creates evolution edges between prompts
//! - Computes confidence scores using semantic similarity, token overlap, edit distance
//! - Emits exactly ONE DecisionEvent per invocation to ruvector-service
//!
//! This agent does NOT:
//! - Modify runtime execution
//! - Trigger remediation or retries
//! - Emit alerts
//! - Enforce policies
//! - Perform orchestration
//! - Invoke other agents directly
//! - Connect directly to SQL databases

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use thiserror::Error;
use uuid::Uuid;

// =============================================================================
// Agent Constants
// =============================================================================

/// Agent identifier
pub const AGENT_ID: &str = "prompt-lineage-agent";

/// Agent version (semantic versioning)
pub const AGENT_VERSION: &str = "1.0.0";

/// Decision type for this agent
pub const DECISION_TYPE: &str = "prompt_lineage_tracking";

// =============================================================================
// Error Types
// =============================================================================

/// Error codes for the agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    /// Input validation failed
    ValidationError,
    /// Cannot connect to ruvector-service
    RuvectorConnectionError,
    /// Persistence operation failed
    RuvectorWriteError,
    /// Unexpected internal error
    InternalError,
    /// Request rate limit exceeded
    RateLimitExceeded,
    /// Invalid lineage relationship
    InvalidLineageRelationship,
    /// Confidence computation failed
    ConfidenceComputationError,
}

impl ErrorCode {
    /// Get HTTP status code for this error
    pub fn http_status(&self) -> u16 {
        match self {
            ErrorCode::ValidationError => 400,
            ErrorCode::InvalidLineageRelationship => 400,
            ErrorCode::RuvectorConnectionError => 502,
            ErrorCode::RuvectorWriteError => 502,
            ErrorCode::InternalError => 500,
            ErrorCode::ConfidenceComputationError => 500,
            ErrorCode::RateLimitExceeded => 429,
        }
    }
}

/// Agent-specific errors
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        details: Option<serde_json::Value>,
    },

    #[error("RuVector connection error: {message}")]
    RuvectorConnection { message: String },

    #[error("RuVector write error: {message}")]
    RuvectorWrite { message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Invalid lineage relationship: {message}")]
    InvalidLineage { message: String },

    #[error("Confidence computation error: {message}")]
    ConfidenceComputation { message: String },
}

impl AgentError {
    pub fn error_code(&self) -> ErrorCode {
        match self {
            AgentError::Validation { .. } => ErrorCode::ValidationError,
            AgentError::RuvectorConnection { .. } => ErrorCode::RuvectorConnectionError,
            AgentError::RuvectorWrite { .. } => ErrorCode::RuvectorWriteError,
            AgentError::Internal { .. } => ErrorCode::InternalError,
            AgentError::RateLimitExceeded => ErrorCode::RateLimitExceeded,
            AgentError::InvalidLineage { .. } => ErrorCode::InvalidLineageRelationship,
            AgentError::ConfidenceComputation { .. } => ErrorCode::ConfidenceComputationError,
        }
    }
}

/// Structured error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error_code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    pub execution_ref: String,
    pub timestamp: DateTime<Utc>,
}

// =============================================================================
// Input Types
// =============================================================================

/// Type of lineage relationship between prompts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageRelationType {
    /// Prompt evolved from another (iterative refinement)
    Evolves,
    /// Prompt refines another (targeted improvement)
    Refines,
    /// Prompt derives from another (new variant)
    Derives,
    /// Prompt is a fork of another (parallel branch)
    Forks,
    /// Prompt merges multiple sources
    Merges,
}

impl LineageRelationType {
    /// Get the base confidence multiplier for this relation type
    pub fn base_confidence_multiplier(&self) -> f64 {
        match self {
            LineageRelationType::Evolves => 0.9,
            LineageRelationType::Refines => 0.85,
            LineageRelationType::Derives => 0.7,
            LineageRelationType::Forks => 0.6,
            LineageRelationType::Merges => 0.5,
        }
    }
}

/// Prompt metadata for lineage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMetadata {
    /// Unique identifier for this prompt
    pub prompt_id: Uuid,
    /// Content hash for deduplication
    pub content_hash: String,
    /// Prompt content (may be truncated for storage)
    pub content: String,
    /// Template ID if prompt was generated from template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<Uuid>,
    /// Template version used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_version: Option<String>,
    /// Agent that created/modified this prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Model used with this prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Token count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<u32>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Lineage relationship specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageRelation {
    /// Source prompt ID
    pub from_prompt_id: Uuid,
    /// Target prompt ID
    pub to_prompt_id: Uuid,
    /// Type of relationship
    pub relation_type: LineageRelationType,
    /// Optional confidence override (computed if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_override: Option<f64>,
    /// Reason for the lineage relationship
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Additional context for the relationship
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub context: HashMap<String, serde_json::Value>,
}

/// Options for lineage tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LineageOptions {
    /// Compute semantic similarity for confidence
    #[serde(default = "default_true")]
    pub compute_semantic_similarity: bool,
    /// Compute token overlap for confidence
    #[serde(default = "default_true")]
    pub compute_token_overlap: bool,
    /// Compute edit distance for confidence
    #[serde(default = "default_true")]
    pub compute_edit_distance: bool,
    /// Minimum confidence threshold (relations below this are rejected)
    #[serde(default = "default_min_confidence")]
    pub min_confidence_threshold: f64,
    /// Track template lineage
    #[serde(default)]
    pub track_template_lineage: bool,
    /// Track agent lineage
    #[serde(default)]
    pub track_agent_lineage: bool,
}

fn default_true() -> bool {
    true
}

fn default_min_confidence() -> f64 {
    0.1
}

/// Input for prompt lineage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptLineageInput {
    /// Session ID for this lineage tracking
    pub session_id: Uuid,
    /// Prompts to track (must include both source and target prompts)
    pub prompts: Vec<PromptMetadata>,
    /// Lineage relations to create
    pub relations: Vec<LineageRelation>,
    /// Tracking options
    #[serde(default)]
    pub options: LineageOptions,
}

// =============================================================================
// Output Types
// =============================================================================

/// Reference to a created lineage node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNodeReference {
    pub node_id: Uuid,
    pub prompt_id: Uuid,
    pub content_hash: String,
}

/// Reference to a created lineage edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdgeReference {
    pub edge_id: Uuid,
    pub edge_type: LineageRelationType,
    pub from_node_id: Uuid,
    pub to_node_id: Uuid,
    pub confidence: f64,
}

/// Confidence computation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceDetails {
    /// Final computed confidence score (0.0 to 1.0)
    pub final_score: f64,
    /// Semantic similarity component (if computed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_similarity: Option<f64>,
    /// Token overlap component (if computed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_overlap: Option<f64>,
    /// Edit distance similarity component (if computed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit_distance_similarity: Option<f64>,
    /// Relation type multiplier applied
    pub relation_multiplier: f64,
}

/// Output from prompt lineage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptLineageOutput {
    /// Session ID
    pub session_id: Uuid,
    /// Created lineage nodes
    pub nodes_created: Vec<LineageNodeReference>,
    /// Created lineage edges
    pub edges_created: Vec<LineageEdgeReference>,
    /// Confidence details for each edge
    pub confidence_details: HashMap<Uuid, ConfidenceDetails>,
    /// Number of relations processed
    pub relations_processed: usize,
    /// Number of relations rejected (below threshold)
    pub relations_rejected: usize,
    /// Processing timestamp
    pub timestamp: DateTime<Utc>,
}

// =============================================================================
// DecisionEvent Types
// =============================================================================

/// Telemetry data for the decision event
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionTelemetry {
    /// Total execution duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Memory usage in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,
    /// RuVector service latency in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ruvector_latency_ms: Option<u64>,
    /// Confidence computation time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_computation_ms: Option<u64>,
}

/// Decision event emitted to ruvector-service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEvent {
    /// Agent identifier
    pub agent_id: String,
    /// Agent version
    pub agent_version: String,
    /// Type of decision made
    pub decision_type: String,
    /// SHA-256 hash of inputs
    pub inputs_hash: String,
    /// Agent outputs
    pub outputs: PromptLineageOutput,
    /// Confidence in the decision (0.0 to 1.0)
    pub confidence: f64,
    /// Constraints that were applied
    pub constraints_applied: Vec<String>,
    /// Unique execution reference
    pub execution_ref: Uuid,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Telemetry data
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telemetry: Option<DecisionTelemetry>,
}

// =============================================================================
// HTTP Handler Types
// =============================================================================

/// HTTP request for the edge function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<serde_json::Value>,
}

/// HTTP response from the edge function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: serde_json::Value,
}

impl HttpResponse {
    /// Create a successful response
    pub fn success<T: Serialize>(data: T) -> Self {
        Self {
            status_code: 200,
            headers: Self::json_headers(),
            body: serde_json::to_value(data).unwrap_or(serde_json::Value::Null),
        }
    }

    /// Create an error response
    pub fn error(error: &ErrorResponse) -> Self {
        Self {
            status_code: error.error_code.http_status(),
            headers: Self::json_headers(),
            body: serde_json::to_value(error).unwrap_or(serde_json::Value::Null),
        }
    }

    /// Create a health check response
    pub fn health() -> Self {
        Self {
            status_code: 200,
            headers: Self::json_headers(),
            body: serde_json::json!({
                "status": "healthy",
                "agent_id": AGENT_ID,
                "agent_version": AGENT_VERSION,
                "timestamp": Utc::now().to_rfc3339()
            }),
        }
    }

    fn json_headers() -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("X-Agent-ID".to_string(), AGENT_ID.to_string());
        headers.insert("X-Agent-Version".to_string(), AGENT_VERSION.to_string());
        headers
    }
}

// =============================================================================
// RuVector Client
// =============================================================================

/// Configuration for RuVector client
#[derive(Debug, Clone)]
pub struct RuvectorConfig {
    pub base_url: String,
    pub api_key: String,
    pub timeout_secs: u64,
}

impl Default for RuvectorConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("RUVECTOR_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            api_key: std::env::var("RUVECTOR_API_KEY").unwrap_or_default(),
            timeout_secs: 30,
        }
    }
}

/// Result from RuVector persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuvectorPersistResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Client for ruvector-service
pub struct RuvectorClient {
    config: RuvectorConfig,
    client: Client,
}

impl RuvectorClient {
    /// Create a new RuVector client
    pub fn new(config: RuvectorConfig) -> Result<Self, AgentError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| AgentError::Internal {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(Self { config, client })
    }

    /// Create a client with default configuration
    pub fn with_defaults() -> Result<Self, AgentError> {
        Self::new(RuvectorConfig::default())
    }

    /// Persist a decision event to ruvector-service
    pub async fn persist_decision_event(
        &self,
        event: &DecisionEvent,
    ) -> Result<RuvectorPersistResult, AgentError> {
        let url = format!("{}/api/v1/decisions", self.config.base_url);
        let start = Instant::now();

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("X-Agent-ID", AGENT_ID)
            .header("X-Agent-Version", AGENT_VERSION)
            .json(event)
            .send()
            .await
            .map_err(|e| AgentError::RuvectorConnection {
                message: e.to_string(),
            })?;

        let latency_ms = start.elapsed().as_millis() as u64;

        if response.status().is_success() {
            let event_id = response
                .headers()
                .get("X-Event-ID")
                .and_then(|v| v.to_str().ok())
                .map(String::from);

            Ok(RuvectorPersistResult {
                success: true,
                event_id,
                latency_ms: Some(latency_ms),
                error: None,
            })
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Ok(RuvectorPersistResult {
                success: false,
                event_id: None,
                latency_ms: Some(latency_ms),
                error: Some(format!("HTTP {}: {}", status, error_text)),
            })
        }
    }
}

// =============================================================================
// Confidence Computation
// =============================================================================

/// Compute confidence score for a lineage relationship
pub struct ConfidenceComputer;

impl ConfidenceComputer {
    /// Compute overall confidence for a lineage relation
    pub fn compute(
        from_content: &str,
        to_content: &str,
        relation_type: LineageRelationType,
        options: &LineageOptions,
    ) -> ConfidenceDetails {
        let mut components: Vec<f64> = Vec::new();

        // Semantic similarity (simplified - in production would use embeddings)
        let semantic_similarity = if options.compute_semantic_similarity {
            let sim = Self::compute_semantic_similarity(from_content, to_content);
            components.push(sim);
            Some(sim)
        } else {
            None
        };

        // Token overlap
        let token_overlap = if options.compute_token_overlap {
            let overlap = Self::compute_token_overlap(from_content, to_content);
            components.push(overlap);
            Some(overlap)
        } else {
            None
        };

        // Edit distance similarity
        let edit_distance_similarity = if options.compute_edit_distance {
            let sim = Self::compute_edit_distance_similarity(from_content, to_content);
            components.push(sim);
            Some(sim)
        } else {
            None
        };

        // Calculate base score (average of components)
        let base_score = if components.is_empty() {
            0.5 // Default if no components computed
        } else {
            components.iter().sum::<f64>() / components.len() as f64
        };

        // Apply relation type multiplier
        let relation_multiplier = relation_type.base_confidence_multiplier();
        let final_score = (base_score * relation_multiplier).clamp(0.0, 1.0);

        ConfidenceDetails {
            final_score,
            semantic_similarity,
            token_overlap,
            edit_distance_similarity,
            relation_multiplier,
        }
    }

    /// Compute semantic similarity between two texts
    /// In production, this would use embedding vectors from a model
    fn compute_semantic_similarity(text1: &str, text2: &str) -> f64 {
        // Simplified: Use character-level n-gram similarity as proxy
        let ngrams1 = Self::extract_ngrams(text1, 3);
        let ngrams2 = Self::extract_ngrams(text2, 3);

        if ngrams1.is_empty() || ngrams2.is_empty() {
            return 0.0;
        }

        let intersection: HashSet<_> = ngrams1.intersection(&ngrams2).collect();
        let union: HashSet<_> = ngrams1.union(&ngrams2).collect();

        if union.is_empty() {
            0.0
        } else {
            intersection.len() as f64 / union.len() as f64
        }
    }

    /// Extract character n-grams from text
    fn extract_ngrams(text: &str, n: usize) -> HashSet<String> {
        let normalized: String = text.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
        if normalized.len() < n {
            return HashSet::new();
        }

        (0..=normalized.len() - n)
            .map(|i| normalized[i..i + n].to_string())
            .collect()
    }

    /// Compute token overlap (Jaccard similarity on words)
    fn compute_token_overlap(text1: &str, text2: &str) -> f64 {
        let tokens1: HashSet<_> = Self::tokenize(text1).into_iter().collect();
        let tokens2: HashSet<_> = Self::tokenize(text2).into_iter().collect();

        if tokens1.is_empty() || tokens2.is_empty() {
            return 0.0;
        }

        let intersection = tokens1.intersection(&tokens2).count();
        let union = tokens1.union(&tokens2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Tokenize text into normalized words
    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty() && s.len() > 1)
            .map(String::from)
            .collect()
    }

    /// Compute edit distance similarity (1 - normalized Levenshtein distance)
    fn compute_edit_distance_similarity(text1: &str, text2: &str) -> f64 {
        let max_len = text1.len().max(text2.len());
        if max_len == 0 {
            return 1.0;
        }

        let distance = Self::levenshtein_distance(text1, text2);
        1.0 - (distance as f64 / max_len as f64)
    }

    /// Compute Levenshtein edit distance between two strings
    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let len1 = s1.chars().count();
        let len2 = s2.chars().count();

        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }

        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();

        let mut prev_row: Vec<usize> = (0..=len2).collect();
        let mut curr_row: Vec<usize> = vec![0; len2 + 1];

        for i in 1..=len1 {
            curr_row[0] = i;

            for j in 1..=len2 {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };
                curr_row[j] = (prev_row[j] + 1)
                    .min(curr_row[j - 1] + 1)
                    .min(prev_row[j - 1] + cost);
            }

            std::mem::swap(&mut prev_row, &mut curr_row);
        }

        prev_row[len2]
    }
}

// =============================================================================
// Input Validation
// =============================================================================

/// Validate the input for prompt lineage tracking
fn validate_input(input: &PromptLineageInput) -> Result<(), AgentError> {
    // Validate session ID is not nil
    if input.session_id.is_nil() {
        return Err(AgentError::Validation {
            message: "session_id cannot be nil UUID".to_string(),
            details: None,
        });
    }

    // Validate at least some prompts are provided
    if input.prompts.is_empty() {
        return Err(AgentError::Validation {
            message: "prompts array cannot be empty".to_string(),
            details: None,
        });
    }

    // Validate at least one relation is provided
    if input.relations.is_empty() {
        return Err(AgentError::Validation {
            message: "relations array cannot be empty".to_string(),
            details: None,
        });
    }

    // Build set of prompt IDs
    let prompt_ids: HashSet<Uuid> = input.prompts.iter().map(|p| p.prompt_id).collect();

    // Validate all relation endpoints exist in prompts
    for (idx, relation) in input.relations.iter().enumerate() {
        if !prompt_ids.contains(&relation.from_prompt_id) {
            return Err(AgentError::Validation {
                message: format!(
                    "relation[{}].from_prompt_id {} not found in prompts",
                    idx, relation.from_prompt_id
                ),
                details: None,
            });
        }

        if !prompt_ids.contains(&relation.to_prompt_id) {
            return Err(AgentError::Validation {
                message: format!(
                    "relation[{}].to_prompt_id {} not found in prompts",
                    idx, relation.to_prompt_id
                ),
                details: None,
            });
        }

        // Validate self-reference
        if relation.from_prompt_id == relation.to_prompt_id {
            return Err(AgentError::Validation {
                message: format!(
                    "relation[{}] cannot reference same prompt as source and target",
                    idx
                ),
                details: None,
            });
        }

        // Validate confidence override if provided
        if let Some(confidence) = relation.confidence_override {
            if !(0.0..=1.0).contains(&confidence) {
                return Err(AgentError::Validation {
                    message: format!(
                        "relation[{}].confidence_override {} must be between 0.0 and 1.0",
                        idx, confidence
                    ),
                    details: None,
                });
            }
        }
    }

    // Validate prompt metadata
    for (idx, prompt) in input.prompts.iter().enumerate() {
        if prompt.prompt_id.is_nil() {
            return Err(AgentError::Validation {
                message: format!("prompts[{}].prompt_id cannot be nil UUID", idx),
                details: None,
            });
        }

        if prompt.content.is_empty() {
            return Err(AgentError::Validation {
                message: format!("prompts[{}].content cannot be empty", idx),
                details: None,
            });
        }

        if prompt.content_hash.is_empty() {
            return Err(AgentError::Validation {
                message: format!("prompts[{}].content_hash cannot be empty", idx),
                details: None,
            });
        }
    }

    // Validate options
    if !(0.0..=1.0).contains(&input.options.min_confidence_threshold) {
        return Err(AgentError::Validation {
            message: format!(
                "options.min_confidence_threshold {} must be between 0.0 and 1.0",
                input.options.min_confidence_threshold
            ),
            details: None,
        });
    }

    Ok(())
}

// =============================================================================
// Agent Handler
// =============================================================================

/// Execution context for the agent
struct ExecutionContext {
    execution_ref: Uuid,
    start_time: Instant,
}

impl ExecutionContext {
    fn new() -> Self {
        Self {
            execution_ref: Uuid::new_v4(),
            start_time: Instant::now(),
        }
    }

    fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}

/// Result from agent execution
pub struct AgentResult {
    pub output: PromptLineageOutput,
    pub decision_event: DecisionEvent,
}

/// Prompt Lineage Agent Handler
///
/// Stateless, deterministic handler for tracking prompt lineage.
/// Designed for deployment as a Google Cloud Edge Function.
pub struct PromptLineageHandler {
    ruvector_client: RuvectorClient,
}

impl PromptLineageHandler {
    /// Create a new handler with the given RuVector client
    pub fn new(ruvector_client: RuvectorClient) -> Self {
        Self { ruvector_client }
    }

    /// Create a handler with default configuration
    pub fn with_defaults() -> Result<Self, AgentError> {
        let client = RuvectorClient::with_defaults()?;
        Ok(Self::new(client))
    }

    /// Handle an HTTP request (Edge Function entry point)
    pub async fn handle_http(&self, request: HttpRequest) -> HttpResponse {
        // Health check endpoint
        if request.path == "/health" || request.path == "/_health" {
            return HttpResponse::health();
        }

        // Only accept POST for main endpoint
        if request.method.to_uppercase() != "POST" {
            return HttpResponse::error(&ErrorResponse {
                error_code: ErrorCode::ValidationError,
                message: "Method not allowed. Use POST.".to_string(),
                details: None,
                execution_ref: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
            });
        }

        // Parse input
        let input: PromptLineageInput = match request.body {
            Some(body) => match serde_json::from_value(body) {
                Ok(input) => input,
                Err(e) => {
                    return HttpResponse::error(&ErrorResponse {
                        error_code: ErrorCode::ValidationError,
                        message: format!("Invalid input JSON: {}", e),
                        details: None,
                        execution_ref: Uuid::new_v4().to_string(),
                        timestamp: Utc::now(),
                    });
                }
            },
            None => {
                return HttpResponse::error(&ErrorResponse {
                    error_code: ErrorCode::ValidationError,
                    message: "Request body is required".to_string(),
                    details: None,
                    execution_ref: Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                });
            }
        };

        // Execute agent
        match self.execute(input).await {
            Ok(result) => {
                // Return success response with decision event
                HttpResponse::success(serde_json::json!({
                    "success": true,
                    "output": result.output,
                    "decision_event": result.decision_event
                }))
            }
            Err(error) => HttpResponse::error(&ErrorResponse {
                error_code: error.error_code(),
                message: error.to_string(),
                details: None,
                execution_ref: Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
            }),
        }
    }

    /// Execute the agent to track prompt lineage
    ///
    /// This is the main entry point for the agent.
    /// Emits exactly ONE DecisionEvent to ruvector-service.
    pub async fn execute(&self, input: PromptLineageInput) -> Result<AgentResult, AgentError> {
        let context = ExecutionContext::new();

        // Validate input
        validate_input(&input)?;

        // Process lineage tracking
        let confidence_start = Instant::now();
        let (output, avg_confidence) = self.process_lineage(&input, &context)?;
        let confidence_ms = confidence_start.elapsed().as_millis() as u64;

        // Create decision event
        let decision_event = self.create_decision_event(&input, &output, avg_confidence, &context);

        // Persist to ruvector-service
        let persist_result = self
            .ruvector_client
            .persist_decision_event(&decision_event)
            .await?;

        if !persist_result.success {
            return Err(AgentError::RuvectorWrite {
                message: persist_result
                    .error
                    .unwrap_or_else(|| "Unknown persistence error".to_string()),
            });
        }

        // Update telemetry in decision event
        let mut final_event = decision_event;
        final_event.telemetry = Some(DecisionTelemetry {
            duration_ms: Some(context.elapsed_ms()),
            memory_bytes: None,
            ruvector_latency_ms: persist_result.latency_ms,
            confidence_computation_ms: Some(confidence_ms),
        });

        Ok(AgentResult {
            output,
            decision_event: final_event,
        })
    }

    /// Process lineage relations and create graph representation
    fn process_lineage(
        &self,
        input: &PromptLineageInput,
        context: &ExecutionContext,
    ) -> Result<(PromptLineageOutput, f64), AgentError> {
        let mut nodes_created: Vec<LineageNodeReference> = Vec::new();
        let mut edges_created: Vec<LineageEdgeReference> = Vec::new();
        let mut confidence_details: HashMap<Uuid, ConfidenceDetails> = HashMap::new();
        let mut relations_rejected = 0;

        // Build prompt lookup map
        let prompt_map: HashMap<Uuid, &PromptMetadata> =
            input.prompts.iter().map(|p| (p.prompt_id, p)).collect();

        // Create nodes for all prompts
        for prompt in &input.prompts {
            nodes_created.push(LineageNodeReference {
                node_id: Uuid::new_v4(),
                prompt_id: prompt.prompt_id,
                content_hash: prompt.content_hash.clone(),
            });
        }

        // Build node ID lookup for edge creation
        let node_map: HashMap<Uuid, Uuid> = nodes_created
            .iter()
            .map(|n| (n.prompt_id, n.node_id))
            .collect();

        // Process each relation
        for relation in &input.relations {
            let from_prompt = prompt_map.get(&relation.from_prompt_id).ok_or_else(|| {
                AgentError::Internal {
                    message: format!("Prompt {} not found in map", relation.from_prompt_id),
                }
            })?;

            let to_prompt = prompt_map.get(&relation.to_prompt_id).ok_or_else(|| {
                AgentError::Internal {
                    message: format!("Prompt {} not found in map", relation.to_prompt_id),
                }
            })?;

            // Compute confidence
            let details = if let Some(override_confidence) = relation.confidence_override {
                ConfidenceDetails {
                    final_score: override_confidence,
                    semantic_similarity: None,
                    token_overlap: None,
                    edit_distance_similarity: None,
                    relation_multiplier: relation.relation_type.base_confidence_multiplier(),
                }
            } else {
                ConfidenceComputer::compute(
                    &from_prompt.content,
                    &to_prompt.content,
                    relation.relation_type,
                    &input.options,
                )
            };

            // Check confidence threshold
            if details.final_score < input.options.min_confidence_threshold {
                relations_rejected += 1;
                continue;
            }

            // Get node IDs
            let from_node_id = *node_map.get(&relation.from_prompt_id).ok_or_else(|| {
                AgentError::Internal {
                    message: format!("Node for prompt {} not found", relation.from_prompt_id),
                }
            })?;

            let to_node_id = *node_map.get(&relation.to_prompt_id).ok_or_else(|| {
                AgentError::Internal {
                    message: format!("Node for prompt {} not found", relation.to_prompt_id),
                }
            })?;

            // Create edge
            let edge_id = Uuid::new_v4();
            edges_created.push(LineageEdgeReference {
                edge_id,
                edge_type: relation.relation_type,
                from_node_id,
                to_node_id,
                confidence: details.final_score,
            });

            confidence_details.insert(edge_id, details);
        }

        // Calculate average confidence for decision event
        let avg_confidence = if edges_created.is_empty() {
            1.0 // No edges means we processed everything deterministically
        } else {
            edges_created.iter().map(|e| e.confidence).sum::<f64>() / edges_created.len() as f64
        };

        let output = PromptLineageOutput {
            session_id: input.session_id,
            nodes_created,
            edges_created,
            confidence_details,
            relations_processed: input.relations.len() - relations_rejected,
            relations_rejected,
            timestamp: Utc::now(),
        };

        Ok((output, avg_confidence))
    }

    /// Create decision event for this execution
    fn create_decision_event(
        &self,
        input: &PromptLineageInput,
        output: &PromptLineageOutput,
        confidence: f64,
        context: &ExecutionContext,
    ) -> DecisionEvent {
        let inputs_hash = self.hash_input(input);
        let constraints = self.get_constraints_applied(input);

        DecisionEvent {
            agent_id: AGENT_ID.to_string(),
            agent_version: AGENT_VERSION.to_string(),
            decision_type: DECISION_TYPE.to_string(),
            inputs_hash,
            outputs: output.clone(),
            confidence,
            constraints_applied: constraints,
            execution_ref: context.execution_ref,
            timestamp: Utc::now(),
            telemetry: None, // Will be updated after persistence
        }
    }

    /// Hash input for idempotency tracking
    fn hash_input(&self, input: &PromptLineageInput) -> String {
        let data = serde_json::to_string(input).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Get list of constraints that were applied
    fn get_constraints_applied(&self, input: &PromptLineageInput) -> Vec<String> {
        let mut constraints = Vec::new();

        if input.options.compute_semantic_similarity {
            constraints.push("semantic_similarity_enabled".to_string());
        }

        if input.options.compute_token_overlap {
            constraints.push("token_overlap_enabled".to_string());
        }

        if input.options.compute_edit_distance {
            constraints.push("edit_distance_enabled".to_string());
        }

        if input.options.track_template_lineage {
            constraints.push("template_lineage_tracking".to_string());
        }

        if input.options.track_agent_lineage {
            constraints.push("agent_lineage_tracking".to_string());
        }

        constraints.push(format!(
            "min_confidence_threshold={}",
            input.options.min_confidence_threshold
        ));

        constraints
    }
}

// =============================================================================
// Observatory Integration
// =============================================================================

/// Telemetry event for Observatory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservatoryEvent {
    pub event_type: String,
    pub agent_id: String,
    pub agent_version: String,
    pub execution_ref: Uuid,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

/// Emit telemetry event to Observatory (fire-and-forget)
pub async fn emit_observatory_event(event: ObservatoryEvent) {
    let observatory_url = std::env::var("LLM_OBSERVATORY_URL").ok();

    if let Some(url) = observatory_url {
        let client = match Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
        {
            Ok(c) => c,
            Err(_) => return,
        };

        let _ = client
            .post(format!("{}/api/v1/events", url))
            .header("Content-Type", "application/json")
            .json(&event)
            .send()
            .await;
    }
}

/// Create an observatory event for lineage tracking
pub fn create_lineage_observatory_event(
    execution_ref: Uuid,
    output: &PromptLineageOutput,
) -> ObservatoryEvent {
    ObservatoryEvent {
        event_type: "prompt_lineage_tracked".to_string(),
        agent_id: AGENT_ID.to_string(),
        agent_version: AGENT_VERSION.to_string(),
        execution_ref,
        timestamp: Utc::now(),
        data: serde_json::json!({
            "session_id": output.session_id,
            "nodes_created": output.nodes_created.len(),
            "edges_created": output.edges_created.len(),
            "relations_processed": output.relations_processed,
            "relations_rejected": output.relations_rejected
        }),
    }
}

// =============================================================================
// Module Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_prompt(id: Uuid, content: &str) -> PromptMetadata {
        PromptMetadata {
            prompt_id: id,
            content_hash: format!("{:x}", Sha256::digest(content.as_bytes())),
            content: content.to_string(),
            template_id: None,
            template_version: None,
            agent_id: None,
            model: None,
            token_count: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    fn create_test_input() -> PromptLineageInput {
        let prompt1_id = Uuid::new_v4();
        let prompt2_id = Uuid::new_v4();

        PromptLineageInput {
            session_id: Uuid::new_v4(),
            prompts: vec![
                create_test_prompt(prompt1_id, "What is Rust programming?"),
                create_test_prompt(prompt2_id, "Explain Rust programming in detail with examples."),
            ],
            relations: vec![LineageRelation {
                from_prompt_id: prompt1_id,
                to_prompt_id: prompt2_id,
                relation_type: LineageRelationType::Evolves,
                confidence_override: None,
                reason: Some("Prompt was refined to be more specific".to_string()),
                context: HashMap::new(),
            }],
            options: LineageOptions::default(),
        }
    }

    #[test]
    fn test_validation_success() {
        let input = create_test_input();
        assert!(validate_input(&input).is_ok());
    }

    #[test]
    fn test_validation_empty_prompts() {
        let mut input = create_test_input();
        input.prompts.clear();
        assert!(matches!(
            validate_input(&input),
            Err(AgentError::Validation { .. })
        ));
    }

    #[test]
    fn test_validation_empty_relations() {
        let mut input = create_test_input();
        input.relations.clear();
        assert!(matches!(
            validate_input(&input),
            Err(AgentError::Validation { .. })
        ));
    }

    #[test]
    fn test_validation_missing_prompt_reference() {
        let mut input = create_test_input();
        input.relations[0].from_prompt_id = Uuid::new_v4(); // Non-existent
        assert!(matches!(
            validate_input(&input),
            Err(AgentError::Validation { .. })
        ));
    }

    #[test]
    fn test_validation_self_reference() {
        let mut input = create_test_input();
        let id = input.prompts[0].prompt_id;
        input.relations[0].from_prompt_id = id;
        input.relations[0].to_prompt_id = id;
        assert!(matches!(
            validate_input(&input),
            Err(AgentError::Validation { .. })
        ));
    }

    #[test]
    fn test_confidence_computation() {
        let text1 = "Explain Rust programming";
        let text2 = "Explain Rust programming language in detail";

        let details = ConfidenceComputer::compute(
            text1,
            text2,
            LineageRelationType::Evolves,
            &LineageOptions::default(),
        );

        assert!(details.final_score > 0.0);
        assert!(details.final_score <= 1.0);
        assert!(details.semantic_similarity.is_some());
        assert!(details.token_overlap.is_some());
        assert!(details.edit_distance_similarity.is_some());
    }

    #[test]
    fn test_token_overlap_identical() {
        let text = "hello world test";
        let overlap = ConfidenceComputer::compute_token_overlap(text, text);
        assert!((overlap - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_token_overlap_different() {
        let text1 = "hello world";
        let text2 = "foo bar baz";
        let overlap = ConfidenceComputer::compute_token_overlap(text1, text2);
        assert!((overlap - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_edit_distance_identical() {
        let text = "hello world";
        let sim = ConfidenceComputer::compute_edit_distance_similarity(text, text);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_edit_distance_different() {
        let text1 = "abc";
        let text2 = "xyz";
        let sim = ConfidenceComputer::compute_edit_distance_similarity(text1, text2);
        assert!(sim < 0.5);
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(ConfidenceComputer::levenshtein_distance("", ""), 0);
        assert_eq!(ConfidenceComputer::levenshtein_distance("abc", ""), 3);
        assert_eq!(ConfidenceComputer::levenshtein_distance("", "abc"), 3);
        assert_eq!(ConfidenceComputer::levenshtein_distance("abc", "abc"), 0);
        assert_eq!(ConfidenceComputer::levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_relation_type_multipliers() {
        assert!(LineageRelationType::Evolves.base_confidence_multiplier() > 0.0);
        assert!(LineageRelationType::Refines.base_confidence_multiplier() > 0.0);
        assert!(LineageRelationType::Derives.base_confidence_multiplier() > 0.0);
        assert!(LineageRelationType::Forks.base_confidence_multiplier() > 0.0);
        assert!(LineageRelationType::Merges.base_confidence_multiplier() > 0.0);

        // Evolves should have highest multiplier
        assert!(
            LineageRelationType::Evolves.base_confidence_multiplier()
                > LineageRelationType::Merges.base_confidence_multiplier()
        );
    }

    #[test]
    fn test_http_response_success() {
        let data = serde_json::json!({"test": "value"});
        let response = HttpResponse::success(data);
        assert_eq!(response.status_code, 200);
        assert!(response.headers.contains_key("Content-Type"));
    }

    #[test]
    fn test_http_response_error() {
        let error = ErrorResponse {
            error_code: ErrorCode::ValidationError,
            message: "Test error".to_string(),
            details: None,
            execution_ref: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
        };
        let response = HttpResponse::error(&error);
        assert_eq!(response.status_code, 400);
    }

    #[test]
    fn test_http_response_health() {
        let response = HttpResponse::health();
        assert_eq!(response.status_code, 200);
        assert!(response.body["agent_id"].as_str() == Some(AGENT_ID));
    }

    #[test]
    fn test_error_codes_http_status() {
        assert_eq!(ErrorCode::ValidationError.http_status(), 400);
        assert_eq!(ErrorCode::InvalidLineageRelationship.http_status(), 400);
        assert_eq!(ErrorCode::RuvectorConnectionError.http_status(), 502);
        assert_eq!(ErrorCode::RuvectorWriteError.http_status(), 502);
        assert_eq!(ErrorCode::InternalError.http_status(), 500);
        assert_eq!(ErrorCode::RateLimitExceeded.http_status(), 429);
    }

    #[test]
    fn test_hash_consistency() {
        let input = create_test_input();
        let data = serde_json::to_string(&input).unwrap();
        let mut hasher1 = Sha256::new();
        hasher1.update(data.as_bytes());
        let hash1 = hex::encode(hasher1.finalize());

        let mut hasher2 = Sha256::new();
        hasher2.update(data.as_bytes());
        let hash2 = hex::encode(hasher2.finalize());

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_observatory_event_creation() {
        let output = PromptLineageOutput {
            session_id: Uuid::new_v4(),
            nodes_created: vec![],
            edges_created: vec![],
            confidence_details: HashMap::new(),
            relations_processed: 5,
            relations_rejected: 1,
            timestamp: Utc::now(),
        };

        let event = create_lineage_observatory_event(Uuid::new_v4(), &output);
        assert_eq!(event.event_type, "prompt_lineage_tracked");
        assert_eq!(event.agent_id, AGENT_ID);
    }
}

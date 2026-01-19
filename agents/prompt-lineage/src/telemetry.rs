//! Telemetry module for the Prompt Lineage Agent
//!
//! This module provides telemetry emission for Observatory integration,
//! enabling real-time monitoring and analysis of prompt lineage operations.
//!
//! # Features
//!
//! - **Event Emission**: Publish lineage events for tracking and analysis
//! - **Prometheus Metrics**: Track performance metrics for lineage operations
//! - **Kafka Publishing**: Stream lineage events to external systems
//! - **Non-blocking**: Async emission that doesn't block main operations
//!
//! # Events
//!
//! The module defines four key lineage event types:
//! - `LineageTracked`: When prompt lineage is established
//! - `EvolutionDetected`: When prompt evolution is detected
//! - `LineageQueried`: When lineage is inspected/retrieved
//! - `SimilarityComputed`: When confidence scores are calculated
//!
//! # Examples
//!
//! ```no_run
//! use prompt_lineage::telemetry::{LineageTelemetry, LineageTelemetryEmitter, LineageEvent};
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let emitter = LineageTelemetryEmitter::new();
//!
//! // Emit lineage tracked event
//! emitter.emit_lineage_tracked(
//!     "prompt-001",
//!     "prompt-002",
//!     EvolutionType::Refinement,
//!     0.95,
//! );
//!
//! // Get emission statistics
//! let stats = emitter.stats().await;
//! println!("Events emitted: {}", stats.events_emitted);
//! # Ok(())
//! # }
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Type Aliases for Prompt Lineage IDs
// ============================================================================

/// Unique identifier for a prompt in the lineage system
pub type PromptId = String;

/// Unique identifier for a lineage relationship
pub type LineageId = String;

// ============================================================================
// Evolution Types
// ============================================================================

/// Types of prompt evolution detected in lineage tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvolutionType {
    /// Prompt was refined/improved from parent
    Refinement,
    /// Prompt is a variation/alternative of parent
    Variation,
    /// Prompt was extended with additional content
    Extension,
    /// Prompt was compressed/simplified from parent
    Compression,
    /// Prompt was derived via template instantiation
    TemplateInstantiation,
    /// Prompt is a complete rewrite inspired by parent
    Rewrite,
    /// Prompt is a merge of multiple parent prompts
    Merge,
    /// Prompt was split from a larger parent prompt
    Split,
    /// Unknown or custom evolution type
    Unknown,
}

impl std::fmt::Display for EvolutionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refinement => write!(f, "refinement"),
            Self::Variation => write!(f, "variation"),
            Self::Extension => write!(f, "extension"),
            Self::Compression => write!(f, "compression"),
            Self::TemplateInstantiation => write!(f, "template_instantiation"),
            Self::Rewrite => write!(f, "rewrite"),
            Self::Merge => write!(f, "merge"),
            Self::Split => write!(f, "split"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

// ============================================================================
// Query Types
// ============================================================================

/// Types of lineage queries supported
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageQueryType {
    /// Query ancestors of a prompt
    Ancestors,
    /// Query descendants of a prompt
    Descendants,
    /// Query full lineage tree
    FullTree,
    /// Query direct parent only
    DirectParent,
    /// Query direct children only
    DirectChildren,
    /// Query siblings (prompts with same parent)
    Siblings,
    /// Query by evolution type
    ByEvolutionType,
    /// Query by similarity threshold
    BySimilarity,
    /// Query for common ancestors
    CommonAncestor,
}

impl std::fmt::Display for LineageQueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ancestors => write!(f, "ancestors"),
            Self::Descendants => write!(f, "descendants"),
            Self::FullTree => write!(f, "full_tree"),
            Self::DirectParent => write!(f, "direct_parent"),
            Self::DirectChildren => write!(f, "direct_children"),
            Self::Siblings => write!(f, "siblings"),
            Self::ByEvolutionType => write!(f, "by_evolution_type"),
            Self::BySimilarity => write!(f, "by_similarity"),
            Self::CommonAncestor => write!(f, "common_ancestor"),
        }
    }
}

// ============================================================================
// Similarity Methods
// ============================================================================

/// Methods used for computing prompt similarity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimilarityMethod {
    /// Cosine similarity on embeddings
    Cosine,
    /// Euclidean distance on embeddings
    Euclidean,
    /// Jaccard similarity on tokens
    Jaccard,
    /// Levenshtein edit distance
    Levenshtein,
    /// Semantic similarity via transformer model
    Semantic,
    /// Hybrid method combining multiple approaches
    Hybrid,
    /// Custom similarity function
    Custom,
}

impl std::fmt::Display for SimilarityMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cosine => write!(f, "cosine"),
            Self::Euclidean => write!(f, "euclidean"),
            Self::Jaccard => write!(f, "jaccard"),
            Self::Levenshtein => write!(f, "levenshtein"),
            Self::Semantic => write!(f, "semantic"),
            Self::Hybrid => write!(f, "hybrid"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

// ============================================================================
// Lineage Events
// ============================================================================

/// Events emitted by the Prompt Lineage Agent for Observatory integration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LineageEvent {
    /// Emitted when lineage relationship is established between prompts
    LineageTracked {
        /// Unique identifier for this lineage relationship
        lineage_id: LineageId,
        /// ID of the source/parent prompt
        from_prompt_id: PromptId,
        /// ID of the derived/child prompt
        to_prompt_id: PromptId,
        /// Type of evolution detected
        evolution_type: EvolutionType,
        /// Confidence score for the lineage relationship (0.0 - 1.0)
        confidence: f64,
        /// Event timestamp
        timestamp: DateTime<Utc>,
        /// Additional metadata
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        metadata: HashMap<String, String>,
    },

    /// Emitted when prompt evolution is detected
    EvolutionDetected {
        /// ID of the prompt that evolved
        prompt_id: PromptId,
        /// Type of evolution detected
        evolution_type: EvolutionType,
        /// Number of evolution steps from original
        generation: u32,
        /// Confidence in evolution detection
        confidence: f64,
        /// Event timestamp
        timestamp: DateTime<Utc>,
        /// Additional metadata about the evolution
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        metadata: HashMap<String, String>,
    },

    /// Emitted when lineage is queried/inspected
    LineageQueried {
        /// ID of the prompt being queried
        prompt_id: PromptId,
        /// Type of query performed
        query_type: LineageQueryType,
        /// Number of results returned
        results_count: usize,
        /// Query execution duration in milliseconds
        duration_ms: u64,
        /// Whether query results were cached
        cached: bool,
        /// Event timestamp
        timestamp: DateTime<Utc>,
        /// Query parameters
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        query_params: HashMap<String, String>,
    },

    /// Emitted when similarity between prompts is computed
    SimilarityComputed {
        /// ID of the first prompt
        prompt_a_id: PromptId,
        /// ID of the second prompt
        prompt_b_id: PromptId,
        /// Computed similarity score (0.0 - 1.0)
        score: f64,
        /// Method used for similarity computation
        method: SimilarityMethod,
        /// Computation duration in milliseconds
        duration_ms: u64,
        /// Event timestamp
        timestamp: DateTime<Utc>,
        /// Additional computation details
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        details: HashMap<String, String>,
    },
}

impl LineageEvent {
    /// Get a unique key for this event (for Kafka partitioning)
    pub fn key(&self) -> String {
        match self {
            Self::LineageTracked { lineage_id, .. } => format!("lineage:{}", lineage_id),
            Self::EvolutionDetected { prompt_id, .. } => format!("evolution:{}", prompt_id),
            Self::LineageQueried { prompt_id, .. } => format!("query:{}", prompt_id),
            Self::SimilarityComputed {
                prompt_a_id,
                prompt_b_id,
                ..
            } => format!("similarity:{}:{}", prompt_a_id, prompt_b_id),
        }
    }

    /// Get the event type name
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::LineageTracked { .. } => "lineage_tracked",
            Self::EvolutionDetected { .. } => "evolution_detected",
            Self::LineageQueried { .. } => "lineage_queried",
            Self::SimilarityComputed { .. } => "similarity_computed",
        }
    }

    /// Get the timestamp of this event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::LineageTracked { timestamp, .. }
            | Self::EvolutionDetected { timestamp, .. }
            | Self::LineageQueried { timestamp, .. }
            | Self::SimilarityComputed { timestamp, .. } => *timestamp,
        }
    }

    /// Get the primary prompt ID associated with this event
    pub fn primary_prompt_id(&self) -> &str {
        match self {
            Self::LineageTracked { from_prompt_id, .. } => from_prompt_id,
            Self::EvolutionDetected { prompt_id, .. } => prompt_id,
            Self::LineageQueried { prompt_id, .. } => prompt_id,
            Self::SimilarityComputed { prompt_a_id, .. } => prompt_a_id,
        }
    }
}

// ============================================================================
// Lineage Telemetry Trait
// ============================================================================

/// Trait defining telemetry emission capabilities for lineage operations
pub trait LineageTelemetry: Send + Sync {
    /// Emit an event when lineage is tracked between prompts
    ///
    /// # Arguments
    ///
    /// * `from_id` - ID of the source/parent prompt
    /// * `to_id` - ID of the derived/child prompt
    /// * `evolution_type` - Type of evolution detected
    /// * `confidence` - Confidence score for the relationship (0.0 - 1.0)
    fn emit_lineage_tracked(
        &self,
        from_id: &str,
        to_id: &str,
        evolution_type: EvolutionType,
        confidence: f64,
    );

    /// Emit an event when prompt evolution is detected
    ///
    /// # Arguments
    ///
    /// * `prompt_id` - ID of the evolved prompt
    /// * `evolution_type` - Type of evolution
    /// * `metadata` - Additional metadata about the evolution
    fn emit_evolution_detected(
        &self,
        prompt_id: &str,
        evolution_type: EvolutionType,
        metadata: HashMap<String, String>,
    );

    /// Emit an event when lineage is queried
    ///
    /// # Arguments
    ///
    /// * `prompt_id` - ID of the prompt being queried
    /// * `query_type` - Type of lineage query
    /// * `results_count` - Number of results returned
    /// * `duration_ms` - Query execution time in milliseconds
    fn emit_lineage_queried(
        &self,
        prompt_id: &str,
        query_type: LineageQueryType,
        results_count: usize,
        duration_ms: u64,
    );

    /// Emit an event when similarity is computed between prompts
    ///
    /// # Arguments
    ///
    /// * `prompt_a_id` - ID of the first prompt
    /// * `prompt_b_id` - ID of the second prompt
    /// * `score` - Computed similarity score (0.0 - 1.0)
    /// * `method` - Method used for computation
    fn emit_similarity_computed(
        &self,
        prompt_a_id: &str,
        prompt_b_id: &str,
        score: f64,
        method: SimilarityMethod,
    );
}

// ============================================================================
// Event Publisher Trait
// ============================================================================

/// Trait for publishing lineage events (Kafka, HTTP, etc.)
#[async_trait::async_trait]
pub trait LineageEventPublisher: Send + Sync {
    /// Publish a single lineage event
    async fn publish(&self, event: LineageEvent) -> Result<(), LineageTelemetryError>;

    /// Publish a batch of lineage events
    async fn publish_batch(
        &self,
        events: Vec<LineageEvent>,
    ) -> Result<(), LineageTelemetryError> {
        for event in events {
            self.publish(event).await?;
        }
        Ok(())
    }

    /// Flush any pending events
    async fn flush(&self) -> Result<(), LineageTelemetryError> {
        Ok(())
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during telemetry operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LineageTelemetryError {
    /// Failed to serialize event
    SerializationError(String),
    /// Failed to publish event
    PublishError(String),
    /// Connection error to telemetry backend
    ConnectionError(String),
    /// Timeout during operation
    Timeout(String),
    /// Invalid event data
    InvalidEventData(String),
}

impl std::fmt::Display for LineageTelemetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::PublishError(msg) => write!(f, "Publish error: {}", msg),
            Self::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            Self::Timeout(msg) => write!(f, "Timeout: {}", msg),
            Self::InvalidEventData(msg) => write!(f, "Invalid event data: {}", msg),
        }
    }
}

impl std::error::Error for LineageTelemetryError {}

// ============================================================================
// In-Memory Publisher (for testing)
// ============================================================================

/// In-memory event publisher for testing and development
#[derive(Clone, Default)]
pub struct InMemoryLineagePublisher {
    events: Arc<RwLock<Vec<LineageEvent>>>,
}

impl InMemoryLineagePublisher {
    /// Create a new in-memory publisher
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get all published events
    pub async fn get_events(&self) -> Vec<LineageEvent> {
        self.events.read().await.clone()
    }

    /// Get the number of published events
    pub async fn count(&self) -> usize {
        self.events.read().await.len()
    }

    /// Clear all events
    pub async fn clear(&self) {
        self.events.write().await.clear();
    }

    /// Get events by type
    pub async fn get_events_by_type(&self, event_type: &str) -> Vec<LineageEvent> {
        self.events
            .read()
            .await
            .iter()
            .filter(|e| e.event_type() == event_type)
            .cloned()
            .collect()
    }
}

#[async_trait::async_trait]
impl LineageEventPublisher for InMemoryLineagePublisher {
    async fn publish(&self, event: LineageEvent) -> Result<(), LineageTelemetryError> {
        self.events.write().await.push(event);
        Ok(())
    }

    async fn publish_batch(
        &self,
        events: Vec<LineageEvent>,
    ) -> Result<(), LineageTelemetryError> {
        self.events.write().await.extend(events);
        Ok(())
    }
}

// ============================================================================
// No-Op Publisher
// ============================================================================

/// No-op publisher that discards all events (useful for disabled telemetry)
#[derive(Clone, Copy, Default)]
pub struct NoOpLineagePublisher;

#[async_trait::async_trait]
impl LineageEventPublisher for NoOpLineagePublisher {
    async fn publish(&self, _event: LineageEvent) -> Result<(), LineageTelemetryError> {
        Ok(())
    }
}

// ============================================================================
// Emission Statistics
// ============================================================================

/// Statistics for telemetry emission
struct EmissionStats {
    /// Total events submitted for emission
    events_submitted: AtomicU64,
    /// Total events successfully emitted
    events_emitted: AtomicU64,
    /// Total events that failed to emit
    events_failed: AtomicU64,
    /// Count by event type
    lineage_tracked_count: AtomicU64,
    evolution_detected_count: AtomicU64,
    lineage_queried_count: AtomicU64,
    similarity_computed_count: AtomicU64,
}

impl EmissionStats {
    fn new() -> Self {
        Self {
            events_submitted: AtomicU64::new(0),
            events_emitted: AtomicU64::new(0),
            events_failed: AtomicU64::new(0),
            lineage_tracked_count: AtomicU64::new(0),
            evolution_detected_count: AtomicU64::new(0),
            lineage_queried_count: AtomicU64::new(0),
            similarity_computed_count: AtomicU64::new(0),
        }
    }

    fn inc_submitted(&self) {
        self.events_submitted.fetch_add(1, Ordering::Relaxed);
    }

    fn inc_emitted(&self) {
        self.events_emitted.fetch_add(1, Ordering::Relaxed);
    }

    fn inc_failed(&self) {
        self.events_failed.fetch_add(1, Ordering::Relaxed);
    }

    fn inc_by_event_type(&self, event: &LineageEvent) {
        match event {
            LineageEvent::LineageTracked { .. } => {
                self.lineage_tracked_count.fetch_add(1, Ordering::Relaxed);
            }
            LineageEvent::EvolutionDetected { .. } => {
                self.evolution_detected_count.fetch_add(1, Ordering::Relaxed);
            }
            LineageEvent::LineageQueried { .. } => {
                self.lineage_queried_count.fetch_add(1, Ordering::Relaxed);
            }
            LineageEvent::SimilarityComputed { .. } => {
                self.similarity_computed_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    async fn snapshot(&self) -> EmissionStatsSnapshot {
        EmissionStatsSnapshot {
            events_submitted: self.events_submitted.load(Ordering::Relaxed),
            events_emitted: self.events_emitted.load(Ordering::Relaxed),
            events_failed: self.events_failed.load(Ordering::Relaxed),
            lineage_tracked_count: self.lineage_tracked_count.load(Ordering::Relaxed),
            evolution_detected_count: self.evolution_detected_count.load(Ordering::Relaxed),
            lineage_queried_count: self.lineage_queried_count.load(Ordering::Relaxed),
            similarity_computed_count: self.similarity_computed_count.load(Ordering::Relaxed),
        }
    }

    async fn reset(&self) {
        self.events_submitted.store(0, Ordering::Relaxed);
        self.events_emitted.store(0, Ordering::Relaxed);
        self.events_failed.store(0, Ordering::Relaxed);
        self.lineage_tracked_count.store(0, Ordering::Relaxed);
        self.evolution_detected_count.store(0, Ordering::Relaxed);
        self.lineage_queried_count.store(0, Ordering::Relaxed);
        self.similarity_computed_count.store(0, Ordering::Relaxed);
    }
}

/// Snapshot of emission statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmissionStatsSnapshot {
    /// Total events submitted for emission
    pub events_submitted: u64,
    /// Total events successfully emitted
    pub events_emitted: u64,
    /// Total events that failed to emit
    pub events_failed: u64,
    /// Count of lineage tracked events
    pub lineage_tracked_count: u64,
    /// Count of evolution detected events
    pub evolution_detected_count: u64,
    /// Count of lineage queried events
    pub lineage_queried_count: u64,
    /// Count of similarity computed events
    pub similarity_computed_count: u64,
}

impl EmissionStatsSnapshot {
    /// Calculate success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.events_submitted == 0 {
            100.0
        } else {
            (self.events_emitted as f64 / self.events_submitted as f64) * 100.0
        }
    }

    /// Calculate failure rate as a percentage
    pub fn failure_rate(&self) -> f64 {
        if self.events_submitted == 0 {
            0.0
        } else {
            (self.events_failed as f64 / self.events_submitted as f64) * 100.0
        }
    }
}

// ============================================================================
// Lineage Telemetry Emitter
// ============================================================================

/// Async telemetry emitter for lineage events
///
/// Provides non-blocking event emission that doesn't delay main operations.
/// Events are sent in background tasks using `tokio::spawn`.
#[derive(Clone)]
pub struct LineageTelemetryEmitter<P: LineageEventPublisher + Clone + 'static> {
    /// The underlying event publisher
    publisher: Arc<P>,
    /// Statistics tracking
    stats: Arc<EmissionStats>,
    /// Whether to log errors
    log_errors: bool,
    /// Agent identifier for telemetry context
    agent_id: String,
}

impl<P: LineageEventPublisher + Clone + 'static> LineageTelemetryEmitter<P> {
    /// Create a new telemetry emitter
    ///
    /// # Arguments
    ///
    /// * `publisher` - The event publisher to use
    pub fn new(publisher: Arc<P>) -> Self {
        Self {
            publisher,
            stats: Arc::new(EmissionStats::new()),
            log_errors: true,
            agent_id: "prompt-lineage-agent".to_string(),
        }
    }

    /// Create a new telemetry emitter with custom agent ID
    pub fn with_agent_id(publisher: Arc<P>, agent_id: impl Into<String>) -> Self {
        Self {
            publisher,
            stats: Arc::new(EmissionStats::new()),
            log_errors: true,
            agent_id: agent_id.into(),
        }
    }

    /// Create a silent emitter that doesn't log errors
    pub fn new_silent(publisher: Arc<P>) -> Self {
        Self {
            publisher,
            stats: Arc::new(EmissionStats::new()),
            log_errors: false,
            agent_id: "prompt-lineage-agent".to_string(),
        }
    }

    /// Emit a lineage event without blocking
    pub fn emit(&self, event: LineageEvent) {
        let publisher = Arc::clone(&self.publisher);
        let stats = Arc::clone(&self.stats);
        let log_errors = self.log_errors;

        stats.inc_submitted();
        stats.inc_by_event_type(&event);

        tokio::spawn(async move {
            match publisher.publish(event).await {
                Ok(()) => {
                    stats.inc_emitted();
                }
                Err(e) => {
                    stats.inc_failed();
                    if log_errors {
                        tracing::warn!("Failed to emit lineage event: {}", e);
                    }
                }
            }
        });
    }

    /// Emit multiple events without blocking
    pub fn emit_batch(&self, events: Vec<LineageEvent>) {
        let publisher = Arc::clone(&self.publisher);
        let stats = Arc::clone(&self.stats);
        let log_errors = self.log_errors;
        let count = events.len() as u64;

        for event in &events {
            stats.inc_by_event_type(event);
        }

        tokio::spawn(async move {
            stats.events_submitted.fetch_add(count, Ordering::Relaxed);

            match publisher.publish_batch(events).await {
                Ok(()) => {
                    stats.events_emitted.fetch_add(count, Ordering::Relaxed);
                }
                Err(e) => {
                    stats.events_failed.fetch_add(count, Ordering::Relaxed);
                    if log_errors {
                        tracing::warn!("Failed to emit lineage event batch: {}", e);
                    }
                }
            }
        });
    }

    /// Emit an event and wait for completion (sync version)
    pub async fn emit_sync(&self, event: LineageEvent) -> Result<(), LineageTelemetryError> {
        self.stats.inc_submitted();
        self.stats.inc_by_event_type(&event);

        match self.publisher.publish(event).await {
            Ok(()) => {
                self.stats.inc_emitted();
                Ok(())
            }
            Err(e) => {
                self.stats.inc_failed();
                if self.log_errors {
                    tracing::warn!("Failed to emit lineage event: {}", e);
                }
                Err(e)
            }
        }
    }

    /// Get emission statistics
    pub async fn stats(&self) -> EmissionStatsSnapshot {
        self.stats.snapshot().await
    }

    /// Reset all statistics
    pub async fn reset_stats(&self) {
        self.stats.reset().await;
    }

    /// Get the underlying publisher
    pub fn publisher(&self) -> &Arc<P> {
        &self.publisher
    }

    /// Get the agent ID
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    // Helper method to generate a lineage ID
    fn generate_lineage_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

impl<P: LineageEventPublisher + Clone + 'static> LineageTelemetry for LineageTelemetryEmitter<P> {
    fn emit_lineage_tracked(
        &self,
        from_id: &str,
        to_id: &str,
        evolution_type: EvolutionType,
        confidence: f64,
    ) {
        let event = LineageEvent::LineageTracked {
            lineage_id: self.generate_lineage_id(),
            from_prompt_id: from_id.to_string(),
            to_prompt_id: to_id.to_string(),
            evolution_type,
            confidence: confidence.clamp(0.0, 1.0),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        self.emit(event);
    }

    fn emit_evolution_detected(
        &self,
        prompt_id: &str,
        evolution_type: EvolutionType,
        metadata: HashMap<String, String>,
    ) {
        let generation = metadata
            .get("generation")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let confidence = metadata
            .get("confidence")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.0);

        let event = LineageEvent::EvolutionDetected {
            prompt_id: prompt_id.to_string(),
            evolution_type,
            generation,
            confidence,
            timestamp: Utc::now(),
            metadata,
        };
        self.emit(event);
    }

    fn emit_lineage_queried(
        &self,
        prompt_id: &str,
        query_type: LineageQueryType,
        results_count: usize,
        duration_ms: u64,
    ) {
        let event = LineageEvent::LineageQueried {
            prompt_id: prompt_id.to_string(),
            query_type,
            results_count,
            duration_ms,
            cached: false,
            timestamp: Utc::now(),
            query_params: HashMap::new(),
        };
        self.emit(event);
    }

    fn emit_similarity_computed(
        &self,
        prompt_a_id: &str,
        prompt_b_id: &str,
        score: f64,
        method: SimilarityMethod,
    ) {
        let event = LineageEvent::SimilarityComputed {
            prompt_a_id: prompt_a_id.to_string(),
            prompt_b_id: prompt_b_id.to_string(),
            score: score.clamp(0.0, 1.0),
            method,
            duration_ms: 0,
            timestamp: Utc::now(),
            details: HashMap::new(),
        };
        self.emit(event);
    }
}

// ============================================================================
// Prometheus Metrics for Lineage Operations
// ============================================================================

/// Prometheus metrics for lineage telemetry
///
/// Provides production-grade metrics for monitoring lineage operations.
#[cfg(feature = "prometheus")]
pub mod prometheus_metrics {
    use prometheus::{
        Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, Opts, Registry,
    };

    /// Lineage-specific Prometheus metrics
    #[derive(Clone)]
    pub struct LineageMetrics {
        /// Total lineage relationships tracked
        pub lineage_tracked_total: IntCounter,
        /// Total evolutions detected by type
        pub evolutions_detected_total: IntCounterVec,
        /// Total lineage queries by type
        pub queries_total: IntCounterVec,
        /// Total similarity computations by method
        pub similarity_computations_total: IntCounterVec,
        /// Lineage query duration histogram
        pub query_duration: HistogramVec,
        /// Similarity computation duration histogram
        pub similarity_duration: Histogram,
        /// Confidence score distribution
        pub confidence_distribution: Histogram,
    }

    impl LineageMetrics {
        /// Create and register lineage metrics
        pub fn new(registry: &Registry) -> Result<Self, prometheus::Error> {
            let lineage_tracked_total = IntCounter::with_opts(Opts::new(
                "prompt_lineage_tracked_total",
                "Total number of lineage relationships tracked",
            ))?;
            registry.register(Box::new(lineage_tracked_total.clone()))?;

            let evolutions_detected_total = IntCounterVec::new(
                Opts::new(
                    "prompt_lineage_evolutions_detected_total",
                    "Total evolutions detected by type",
                ),
                &["evolution_type"],
            )?;
            registry.register(Box::new(evolutions_detected_total.clone()))?;

            let queries_total = IntCounterVec::new(
                Opts::new(
                    "prompt_lineage_queries_total",
                    "Total lineage queries by type",
                ),
                &["query_type"],
            )?;
            registry.register(Box::new(queries_total.clone()))?;

            let similarity_computations_total = IntCounterVec::new(
                Opts::new(
                    "prompt_lineage_similarity_computations_total",
                    "Total similarity computations by method",
                ),
                &["method"],
            )?;
            registry.register(Box::new(similarity_computations_total.clone()))?;

            let query_duration = HistogramVec::new(
                HistogramOpts::new(
                    "prompt_lineage_query_duration_seconds",
                    "Lineage query duration in seconds",
                )
                .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
                &["query_type"],
            )?;
            registry.register(Box::new(query_duration.clone()))?;

            let similarity_duration = Histogram::with_opts(
                HistogramOpts::new(
                    "prompt_lineage_similarity_duration_seconds",
                    "Similarity computation duration in seconds",
                )
                .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]),
            )?;
            registry.register(Box::new(similarity_duration.clone()))?;

            let confidence_distribution = Histogram::with_opts(
                HistogramOpts::new(
                    "prompt_lineage_confidence_distribution",
                    "Distribution of lineage confidence scores",
                )
                .buckets(vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95, 1.0]),
            )?;
            registry.register(Box::new(confidence_distribution.clone()))?;

            Ok(Self {
                lineage_tracked_total,
                evolutions_detected_total,
                queries_total,
                similarity_computations_total,
                query_duration,
                similarity_duration,
                confidence_distribution,
            })
        }

        /// Record a lineage tracked event
        pub fn record_lineage_tracked(&self, confidence: f64) {
            self.lineage_tracked_total.inc();
            self.confidence_distribution.observe(confidence);
        }

        /// Record an evolution detected event
        pub fn record_evolution_detected(&self, evolution_type: &str) {
            self.evolutions_detected_total
                .with_label_values(&[evolution_type])
                .inc();
        }

        /// Record a lineage query
        pub fn record_query(&self, query_type: &str, duration_secs: f64) {
            self.queries_total.with_label_values(&[query_type]).inc();
            self.query_duration
                .with_label_values(&[query_type])
                .observe(duration_secs);
        }

        /// Record a similarity computation
        pub fn record_similarity(&self, method: &str, duration_secs: f64) {
            self.similarity_computations_total
                .with_label_values(&[method])
                .inc();
            self.similarity_duration.observe(duration_secs);
        }
    }
}

// ============================================================================
// Kafka Publisher
// ============================================================================

/// Kafka publisher configuration for lineage events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaPublisherConfig {
    /// Kafka broker addresses
    pub brokers: Vec<String>,
    /// Topic for lineage events
    pub topic: String,
    /// Optional client ID
    pub client_id: Option<String>,
    /// Batch size for publishing
    pub batch_size: usize,
    /// Linger time in milliseconds
    pub linger_ms: u64,
}

impl Default for KafkaPublisherConfig {
    fn default() -> Self {
        Self {
            brokers: vec!["localhost:9092".to_string()],
            topic: "prompt-lineage-events".to_string(),
            client_id: Some("prompt-lineage-agent".to_string()),
            batch_size: 100,
            linger_ms: 5,
        }
    }
}

// ============================================================================
// Factory Functions
// ============================================================================

/// Create a telemetry emitter with an in-memory publisher (for testing)
pub fn create_in_memory_emitter() -> LineageTelemetryEmitter<InMemoryLineagePublisher> {
    LineageTelemetryEmitter::new(Arc::new(InMemoryLineagePublisher::new()))
}

/// Create a telemetry emitter with a no-op publisher (for disabled telemetry)
pub fn create_noop_emitter() -> LineageTelemetryEmitter<NoOpLineagePublisher> {
    LineageTelemetryEmitter::new(Arc::new(NoOpLineagePublisher))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[test]
    fn test_evolution_type_display() {
        assert_eq!(EvolutionType::Refinement.to_string(), "refinement");
        assert_eq!(EvolutionType::Variation.to_string(), "variation");
        assert_eq!(
            EvolutionType::TemplateInstantiation.to_string(),
            "template_instantiation"
        );
    }

    #[test]
    fn test_query_type_display() {
        assert_eq!(LineageQueryType::Ancestors.to_string(), "ancestors");
        assert_eq!(LineageQueryType::FullTree.to_string(), "full_tree");
    }

    #[test]
    fn test_similarity_method_display() {
        assert_eq!(SimilarityMethod::Cosine.to_string(), "cosine");
        assert_eq!(SimilarityMethod::Semantic.to_string(), "semantic");
    }

    #[test]
    fn test_lineage_event_serialization() {
        let event = LineageEvent::LineageTracked {
            lineage_id: "lineage-001".to_string(),
            from_prompt_id: "prompt-001".to_string(),
            to_prompt_id: "prompt-002".to_string(),
            evolution_type: EvolutionType::Refinement,
            confidence: 0.95,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: LineageEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.event_type(), deserialized.event_type());
    }

    #[test]
    fn test_event_key_generation() {
        let event = LineageEvent::LineageTracked {
            lineage_id: "lineage-001".to_string(),
            from_prompt_id: "prompt-001".to_string(),
            to_prompt_id: "prompt-002".to_string(),
            evolution_type: EvolutionType::Refinement,
            confidence: 0.95,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        let key = event.key();
        assert!(key.starts_with("lineage:"));
        assert!(key.contains("lineage-001"));
    }

    #[test]
    fn test_event_types() {
        let events = vec![
            LineageEvent::LineageTracked {
                lineage_id: "l1".to_string(),
                from_prompt_id: "p1".to_string(),
                to_prompt_id: "p2".to_string(),
                evolution_type: EvolutionType::Refinement,
                confidence: 0.9,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            },
            LineageEvent::EvolutionDetected {
                prompt_id: "p1".to_string(),
                evolution_type: EvolutionType::Extension,
                generation: 2,
                confidence: 0.85,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            },
            LineageEvent::LineageQueried {
                prompt_id: "p1".to_string(),
                query_type: LineageQueryType::Ancestors,
                results_count: 5,
                duration_ms: 25,
                cached: false,
                timestamp: Utc::now(),
                query_params: HashMap::new(),
            },
            LineageEvent::SimilarityComputed {
                prompt_a_id: "p1".to_string(),
                prompt_b_id: "p2".to_string(),
                score: 0.92,
                method: SimilarityMethod::Cosine,
                duration_ms: 10,
                timestamp: Utc::now(),
                details: HashMap::new(),
            },
        ];

        assert_eq!(events[0].event_type(), "lineage_tracked");
        assert_eq!(events[1].event_type(), "evolution_detected");
        assert_eq!(events[2].event_type(), "lineage_queried");
        assert_eq!(events[3].event_type(), "similarity_computed");
    }

    #[tokio::test]
    async fn test_in_memory_publisher() {
        let publisher = InMemoryLineagePublisher::new();

        let event = LineageEvent::LineageTracked {
            lineage_id: "l1".to_string(),
            from_prompt_id: "p1".to_string(),
            to_prompt_id: "p2".to_string(),
            evolution_type: EvolutionType::Refinement,
            confidence: 0.95,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        publisher.publish(event.clone()).await.unwrap();

        let events = publisher.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "lineage_tracked");

        let count = publisher.count().await;
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_emitter_creation() {
        let publisher = Arc::new(InMemoryLineagePublisher::new());
        let emitter = LineageTelemetryEmitter::new(publisher.clone());

        let stats = emitter.stats().await;
        assert_eq!(stats.events_submitted, 0);
        assert_eq!(stats.events_emitted, 0);
    }

    #[tokio::test]
    async fn test_emit_lineage_tracked() {
        let publisher = Arc::new(InMemoryLineagePublisher::new());
        let emitter = LineageTelemetryEmitter::new(publisher.clone());

        emitter.emit_lineage_tracked("prompt-001", "prompt-002", EvolutionType::Refinement, 0.95);

        // Wait for async task
        sleep(Duration::from_millis(50)).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.events_submitted, 1);
        assert_eq!(stats.events_emitted, 1);
        assert_eq!(stats.lineage_tracked_count, 1);

        let events = publisher.get_events().await;
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_emit_evolution_detected() {
        let publisher = Arc::new(InMemoryLineagePublisher::new());
        let emitter = LineageTelemetryEmitter::new(publisher.clone());

        let mut metadata = HashMap::new();
        metadata.insert("generation".to_string(), "3".to_string());
        metadata.insert("confidence".to_string(), "0.88".to_string());

        emitter.emit_evolution_detected("prompt-001", EvolutionType::Extension, metadata);

        sleep(Duration::from_millis(50)).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.evolution_detected_count, 1);

        let events = publisher.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "evolution_detected");
    }

    #[tokio::test]
    async fn test_emit_lineage_queried() {
        let publisher = Arc::new(InMemoryLineagePublisher::new());
        let emitter = LineageTelemetryEmitter::new(publisher.clone());

        emitter.emit_lineage_queried("prompt-001", LineageQueryType::Ancestors, 5, 25);

        sleep(Duration::from_millis(50)).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.lineage_queried_count, 1);
    }

    #[tokio::test]
    async fn test_emit_similarity_computed() {
        let publisher = Arc::new(InMemoryLineagePublisher::new());
        let emitter = LineageTelemetryEmitter::new(publisher.clone());

        emitter.emit_similarity_computed("prompt-001", "prompt-002", 0.92, SimilarityMethod::Cosine);

        sleep(Duration::from_millis(50)).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.similarity_computed_count, 1);
    }

    #[tokio::test]
    async fn test_emit_batch() {
        let publisher = Arc::new(InMemoryLineagePublisher::new());
        let emitter = LineageTelemetryEmitter::new(publisher.clone());

        let events = vec![
            LineageEvent::LineageTracked {
                lineage_id: "l1".to_string(),
                from_prompt_id: "p1".to_string(),
                to_prompt_id: "p2".to_string(),
                evolution_type: EvolutionType::Refinement,
                confidence: 0.9,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            },
            LineageEvent::EvolutionDetected {
                prompt_id: "p1".to_string(),
                evolution_type: EvolutionType::Extension,
                generation: 2,
                confidence: 0.85,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            },
        ];

        emitter.emit_batch(events);

        sleep(Duration::from_millis(50)).await;

        let published = publisher.get_events().await;
        assert_eq!(published.len(), 2);
    }

    #[tokio::test]
    async fn test_emit_sync() {
        let publisher = Arc::new(InMemoryLineagePublisher::new());
        let emitter = LineageTelemetryEmitter::new(publisher.clone());

        let event = LineageEvent::LineageTracked {
            lineage_id: "l1".to_string(),
            from_prompt_id: "p1".to_string(),
            to_prompt_id: "p2".to_string(),
            evolution_type: EvolutionType::Refinement,
            confidence: 0.95,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        emitter.emit_sync(event).await.unwrap();

        let stats = emitter.stats().await;
        assert_eq!(stats.events_submitted, 1);
        assert_eq!(stats.events_emitted, 1);
    }

    #[tokio::test]
    async fn test_stats_snapshot() {
        let publisher = Arc::new(InMemoryLineagePublisher::new());
        let emitter = LineageTelemetryEmitter::new(publisher);

        let event = LineageEvent::LineageTracked {
            lineage_id: "l1".to_string(),
            from_prompt_id: "p1".to_string(),
            to_prompt_id: "p2".to_string(),
            evolution_type: EvolutionType::Refinement,
            confidence: 0.95,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        emitter.emit(event);
        sleep(Duration::from_millis(50)).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.success_rate(), 100.0);
        assert_eq!(stats.failure_rate(), 0.0);
    }

    #[tokio::test]
    async fn test_reset_stats() {
        let publisher = Arc::new(InMemoryLineagePublisher::new());
        let emitter = LineageTelemetryEmitter::new(publisher);

        emitter.emit_lineage_tracked("p1", "p2", EvolutionType::Refinement, 0.9);
        sleep(Duration::from_millis(50)).await;

        let stats_before = emitter.stats().await;
        assert_eq!(stats_before.events_emitted, 1);

        emitter.reset_stats().await;

        let stats_after = emitter.stats().await;
        assert_eq!(stats_after.events_submitted, 0);
        assert_eq!(stats_after.events_emitted, 0);
    }

    #[tokio::test]
    async fn test_noop_publisher() {
        let publisher = NoOpLineagePublisher;

        let event = LineageEvent::LineageTracked {
            lineage_id: "l1".to_string(),
            from_prompt_id: "p1".to_string(),
            to_prompt_id: "p2".to_string(),
            evolution_type: EvolutionType::Refinement,
            confidence: 0.95,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        // Should not error
        publisher.publish(event).await.unwrap();
        publisher.flush().await.unwrap();
    }

    #[tokio::test]
    async fn test_factory_functions() {
        let in_memory_emitter = create_in_memory_emitter();
        assert_eq!(in_memory_emitter.agent_id(), "prompt-lineage-agent");

        let noop_emitter = create_noop_emitter();
        assert_eq!(noop_emitter.agent_id(), "prompt-lineage-agent");
    }

    #[tokio::test]
    async fn test_get_events_by_type() {
        let publisher = InMemoryLineagePublisher::new();

        publisher
            .publish(LineageEvent::LineageTracked {
                lineage_id: "l1".to_string(),
                from_prompt_id: "p1".to_string(),
                to_prompt_id: "p2".to_string(),
                evolution_type: EvolutionType::Refinement,
                confidence: 0.9,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            })
            .await
            .unwrap();

        publisher
            .publish(LineageEvent::EvolutionDetected {
                prompt_id: "p1".to_string(),
                evolution_type: EvolutionType::Extension,
                generation: 1,
                confidence: 0.85,
                timestamp: Utc::now(),
                metadata: HashMap::new(),
            })
            .await
            .unwrap();

        let lineage_events = publisher.get_events_by_type("lineage_tracked").await;
        assert_eq!(lineage_events.len(), 1);

        let evolution_events = publisher.get_events_by_type("evolution_detected").await;
        assert_eq!(evolution_events.len(), 1);
    }

    #[test]
    fn test_telemetry_error_display() {
        let error = LineageTelemetryError::SerializationError("test error".to_string());
        assert!(error.to_string().contains("Serialization error"));

        let error = LineageTelemetryError::PublishError("publish failed".to_string());
        assert!(error.to_string().contains("Publish error"));
    }

    #[test]
    fn test_kafka_config_default() {
        let config = KafkaPublisherConfig::default();
        assert_eq!(config.topic, "prompt-lineage-events");
        assert_eq!(config.batch_size, 100);
    }

    #[test]
    fn test_confidence_clamping() {
        // Confidence should be clamped to 0.0-1.0 range
        let event = LineageEvent::LineageTracked {
            lineage_id: "l1".to_string(),
            from_prompt_id: "p1".to_string(),
            to_prompt_id: "p2".to_string(),
            evolution_type: EvolutionType::Refinement,
            confidence: 1.5, // Over 1.0
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        // Serialize and deserialize
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: LineageEvent = serde_json::from_str(&json).unwrap();

        if let LineageEvent::LineageTracked { confidence, .. } = deserialized {
            // Note: serde doesn't clamp, but emit_lineage_tracked does
            assert_eq!(confidence, 1.5);
        }
    }

    #[test]
    fn test_primary_prompt_id() {
        let event = LineageEvent::LineageTracked {
            lineage_id: "l1".to_string(),
            from_prompt_id: "p1".to_string(),
            to_prompt_id: "p2".to_string(),
            evolution_type: EvolutionType::Refinement,
            confidence: 0.9,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        assert_eq!(event.primary_prompt_id(), "p1");
    }

    #[tokio::test]
    async fn test_concurrent_emission() {
        let publisher = Arc::new(InMemoryLineagePublisher::new());
        let emitter = LineageTelemetryEmitter::new(publisher.clone());

        let mut handles = vec![];

        for i in 0..50 {
            let emitter_clone = emitter.clone();
            let handle = tokio::spawn(async move {
                emitter_clone.emit_lineage_tracked(
                    &format!("p{}", i),
                    &format!("p{}", i + 1),
                    EvolutionType::Refinement,
                    0.9,
                );
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        sleep(Duration::from_millis(200)).await;

        let stats = emitter.stats().await;
        assert_eq!(stats.events_submitted, 50);
        assert_eq!(stats.events_emitted, 50);

        let published = publisher.get_events().await;
        assert_eq!(published.len(), 50);
    }
}

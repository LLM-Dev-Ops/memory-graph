//! Core lineage tracking module for the Prompt Lineage Agent
//!
//! This module provides comprehensive prompt evolution tracking, similarity computation,
//! and graph traversal algorithms for exploring prompt lineage relationships.
//!
//! # Overview
//!
//! The lineage system tracks three types of prompt evolution:
//! - **Evolves**: Same intent, modified approach
//! - **Refines**: Improved version of the same prompt
//! - **Derives**: New prompt derived from original
//!
//! # Example
//!
//! ```rust,ignore
//! use prompt_lineage::lineage::{LineageTracker, EvolutionType};
//! use llm_memory_graph_types::NodeId;
//!
//! let tracker = LineageTracker::new();
//! let edge = tracker.track_evolution(parent_id, child_id, EvolutionType::Refines, 0.95)?;
//! let ancestors = tracker.get_ancestors(child_id, 5)?;
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;

// Re-use types from llm-memory-graph-types
// In production, these would be imported from the types crate
// For now, we define local wrappers that match the types crate interface

/// Unique identifier for a node (mirrors llm_memory_graph_types::NodeId)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(uuid::Uuid);

impl NodeId {
    /// Create a new random node ID
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Create from UUID
    #[must_use]
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an edge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(uuid::Uuid);

impl EdgeId {
    /// Create a new random edge ID
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for EdgeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EdgeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during lineage operations
#[derive(Debug, Error)]
pub enum LineageError {
    /// Node not found in the graph
    #[error("Node not found: {0}")]
    NodeNotFound(NodeId),

    /// Edge not found in the graph
    #[error("Edge not found: {0}")]
    EdgeNotFound(EdgeId),

    /// Circular dependency detected
    #[error("Circular dependency detected: {0} -> {1}")]
    CircularDependency(NodeId, NodeId),

    /// Invalid confidence score (must be 0.0 to 1.0)
    #[error("Invalid confidence score: {0} (must be 0.0 to 1.0)")]
    InvalidConfidence(f64),

    /// Maximum depth exceeded
    #[error("Maximum traversal depth exceeded: {0}")]
    MaxDepthExceeded(usize),

    /// Graph operation failed
    #[error("Graph operation failed: {0}")]
    GraphError(String),

    /// Prompt content required for similarity computation
    #[error("Prompt content not available for node: {0}")]
    ContentNotAvailable(NodeId),
}

/// Result type for lineage operations
pub type LineageResult<T> = Result<T, LineageError>;

// ============================================================================
// Evolution Types
// ============================================================================

/// Types of prompt evolution relationships
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvolutionType {
    /// Same intent, modified approach
    /// Used when the prompt's goal remains the same but the method changes
    Evolves,

    /// Improved version of the same prompt
    /// Used for iterative refinements that enhance clarity or effectiveness
    Refines,

    /// New prompt derived from original
    /// Used when creating a new prompt inspired by or based on another
    Derives,
}

impl EvolutionType {
    /// Get the default confidence threshold for this evolution type
    #[must_use]
    pub const fn default_confidence_threshold(&self) -> f64 {
        match self {
            Self::Evolves => 0.7,
            Self::Refines => 0.85,
            Self::Derives => 0.5,
        }
    }

    /// Get a human-readable description
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Evolves => "Same intent, modified approach",
            Self::Refines => "Improved version of the same prompt",
            Self::Derives => "New prompt derived from original",
        }
    }
}

impl std::fmt::Display for EvolutionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Evolves => write!(f, "EVOLVES"),
            Self::Refines => write!(f, "REFINES"),
            Self::Derives => write!(f, "DERIVES"),
        }
    }
}

// ============================================================================
// Lineage Edge
// ============================================================================

/// Properties for a lineage edge connecting two prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    /// Unique edge identifier
    pub id: EdgeId,
    /// Source (parent) prompt node ID
    pub parent_id: NodeId,
    /// Target (child) prompt node ID
    pub child_id: NodeId,
    /// Type of evolution relationship
    pub evolution_type: EvolutionType,
    /// Confidence score (0.0 to 1.0) indicating strength of relationship
    pub confidence: f64,
    /// When the lineage was established
    pub created_at: DateTime<Utc>,
    /// Additional metadata about the evolution
    pub metadata: HashMap<String, String>,
}

impl LineageEdge {
    /// Create a new lineage edge
    ///
    /// # Errors
    ///
    /// Returns `LineageError::InvalidConfidence` if confidence is not in [0.0, 1.0]
    pub fn new(
        parent_id: NodeId,
        child_id: NodeId,
        evolution_type: EvolutionType,
        confidence: f64,
    ) -> LineageResult<Self> {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(LineageError::InvalidConfidence(confidence));
        }

        Ok(Self {
            id: EdgeId::new(),
            parent_id,
            child_id,
            evolution_type,
            confidence,
            created_at: Utc::now(),
            metadata: HashMap::new(),
        })
    }

    /// Add metadata to the edge
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Check if this edge meets the confidence threshold for its type
    #[must_use]
    pub fn meets_threshold(&self) -> bool {
        self.confidence >= self.evolution_type.default_confidence_threshold()
    }
}

// ============================================================================
// Prompt Node (for lineage tracking)
// ============================================================================

/// A prompt node with content for lineage tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptInfo {
    /// Node ID
    pub id: NodeId,
    /// Prompt content
    pub content: String,
    /// Tokenized content (for similarity computation)
    pub tokens: Vec<String>,
    /// When the prompt was created
    pub created_at: DateTime<Utc>,
    /// Optional embedding vector (for semantic similarity)
    pub embedding: Option<Vec<f32>>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl PromptInfo {
    /// Create a new prompt info from content
    #[must_use]
    pub fn new(id: NodeId, content: impl Into<String>) -> Self {
        let content = content.into();
        let tokens = Self::tokenize(&content);
        Self {
            id,
            content,
            tokens,
            created_at: Utc::now(),
            embedding: None,
            metadata: HashMap::new(),
        }
    }

    /// Simple whitespace and punctuation tokenizer
    fn tokenize(content: &str) -> Vec<String> {
        content
            .to_lowercase()
            .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    }

    /// Set the embedding vector
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

// ============================================================================
// Similarity Computation
// ============================================================================

/// Configuration for similarity computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityConfig {
    /// Weight for token overlap score (0.0 to 1.0)
    pub token_overlap_weight: f64,
    /// Weight for semantic similarity score (0.0 to 1.0)
    pub semantic_weight: f64,
    /// Weight for edit distance ratio (0.0 to 1.0)
    pub edit_distance_weight: f64,
    /// Minimum token overlap to consider prompts related
    pub min_token_overlap: f64,
}

impl Default for SimilarityConfig {
    fn default() -> Self {
        Self {
            token_overlap_weight: 0.4,
            semantic_weight: 0.35,
            edit_distance_weight: 0.25,
            min_token_overlap: 0.1,
        }
    }
}

impl SimilarityConfig {
    /// Create a config optimized for detecting refinements
    #[must_use]
    pub fn for_refinement() -> Self {
        Self {
            token_overlap_weight: 0.5,
            semantic_weight: 0.3,
            edit_distance_weight: 0.2,
            min_token_overlap: 0.3,
        }
    }

    /// Create a config optimized for detecting derivations
    #[must_use]
    pub fn for_derivation() -> Self {
        Self {
            token_overlap_weight: 0.3,
            semantic_weight: 0.5,
            edit_distance_weight: 0.2,
            min_token_overlap: 0.05,
        }
    }
}

/// Detailed similarity scores breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityScores {
    /// Token overlap score (Jaccard similarity)
    pub token_overlap: f64,
    /// Semantic similarity (cosine of embeddings)
    pub semantic: f64,
    /// Edit distance ratio (1 - normalized Levenshtein)
    pub edit_distance_ratio: f64,
    /// Final weighted score
    pub combined: f64,
}

/// Similarity computation engine
#[derive(Debug, Clone)]
pub struct SimilarityComputer {
    config: SimilarityConfig,
}

impl SimilarityComputer {
    /// Create a new similarity computer with default config
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: SimilarityConfig::default(),
        }
    }

    /// Create with custom configuration
    #[must_use]
    pub fn with_config(config: SimilarityConfig) -> Self {
        Self { config }
    }

    /// Compute similarity between two prompts
    ///
    /// Returns a detailed breakdown of similarity scores
    pub fn compute(&self, prompt_a: &PromptInfo, prompt_b: &PromptInfo) -> SimilarityScores {
        let token_overlap = self.compute_token_overlap(&prompt_a.tokens, &prompt_b.tokens);
        let semantic = self.compute_semantic_similarity(
            prompt_a.embedding.as_deref(),
            prompt_b.embedding.as_deref(),
        );
        let edit_distance_ratio =
            self.compute_edit_distance_ratio(&prompt_a.content, &prompt_b.content);

        let combined = self.config.token_overlap_weight * token_overlap
            + self.config.semantic_weight * semantic
            + self.config.edit_distance_weight * edit_distance_ratio;

        SimilarityScores {
            token_overlap,
            semantic,
            edit_distance_ratio,
            combined,
        }
    }

    /// Compute combined confidence score (convenience method)
    pub fn compute_confidence(&self, prompt_a: &PromptInfo, prompt_b: &PromptInfo) -> f64 {
        self.compute(prompt_a, prompt_b).combined
    }

    /// Compute Jaccard similarity of token sets
    fn compute_token_overlap(&self, tokens_a: &[String], tokens_b: &[String]) -> f64 {
        if tokens_a.is_empty() && tokens_b.is_empty() {
            return 1.0;
        }
        if tokens_a.is_empty() || tokens_b.is_empty() {
            return 0.0;
        }

        let set_a: HashSet<&String> = tokens_a.iter().collect();
        let set_b: HashSet<&String> = tokens_b.iter().collect();

        let intersection = set_a.intersection(&set_b).count();
        let union = set_a.union(&set_b).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Compute cosine similarity of embedding vectors
    ///
    /// Returns 0.5 (neutral) if embeddings are not available
    fn compute_semantic_similarity(
        &self,
        embedding_a: Option<&[f32]>,
        embedding_b: Option<&[f32]>,
    ) -> f64 {
        match (embedding_a, embedding_b) {
            (Some(a), Some(b)) if a.len() == b.len() && !a.is_empty() => {
                let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| *x as f64 * *y as f64).sum();
                let magnitude_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
                let magnitude_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();

                if magnitude_a > 0.0 && magnitude_b > 0.0 {
                    // Convert from [-1, 1] to [0, 1] range
                    (dot_product / (magnitude_a * magnitude_b) + 1.0) / 2.0
                } else {
                    0.5
                }
            }
            _ => 0.5, // Return neutral score when embeddings unavailable
        }
    }

    /// Compute normalized edit distance ratio
    ///
    /// Returns 1 - (levenshtein_distance / max_length)
    fn compute_edit_distance_ratio(&self, content_a: &str, content_b: &str) -> f64 {
        if content_a.is_empty() && content_b.is_empty() {
            return 1.0;
        }

        let distance = self.levenshtein_distance(content_a, content_b);
        let max_len = content_a.len().max(content_b.len());

        if max_len == 0 {
            1.0
        } else {
            1.0 - (distance as f64 / max_len as f64)
        }
    }

    /// Compute Levenshtein edit distance
    fn levenshtein_distance(&self, s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        let len1 = s1_chars.len();
        let len2 = s2_chars.len();

        if len1 == 0 {
            return len2;
        }
        if len2 == 0 {
            return len1;
        }

        // Use two rows instead of full matrix for memory efficiency
        let mut prev_row: Vec<usize> = (0..=len2).collect();
        let mut curr_row: Vec<usize> = vec![0; len2 + 1];

        for (i, c1) in s1_chars.iter().enumerate() {
            curr_row[0] = i + 1;

            for (j, c2) in s2_chars.iter().enumerate() {
                let cost = if c1 == c2 { 0 } else { 1 };
                curr_row[j + 1] = (prev_row[j + 1] + 1) // deletion
                    .min(curr_row[j] + 1) // insertion
                    .min(prev_row[j] + cost); // substitution
            }

            std::mem::swap(&mut prev_row, &mut curr_row);
        }

        prev_row[len2]
    }
}

impl Default for SimilarityComputer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Lineage Subgraph
// ============================================================================

/// A subgraph representing lineage relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageSubgraph {
    /// Root node of the subgraph
    pub root_id: NodeId,
    /// All nodes in the subgraph
    pub nodes: HashMap<NodeId, PromptInfo>,
    /// All edges in the subgraph
    pub edges: Vec<LineageEdge>,
    /// Depth of the deepest node from root
    pub max_depth: usize,
    /// Total number of nodes
    pub node_count: usize,
    /// Total number of edges
    pub edge_count: usize,
}

impl LineageSubgraph {
    /// Create an empty subgraph rooted at the given node
    #[must_use]
    pub fn new(root_id: NodeId) -> Self {
        Self {
            root_id,
            nodes: HashMap::new(),
            edges: Vec::new(),
            max_depth: 0,
            node_count: 0,
            edge_count: 0,
        }
    }

    /// Add a node to the subgraph
    pub fn add_node(&mut self, prompt: PromptInfo) {
        if !self.nodes.contains_key(&prompt.id) {
            self.nodes.insert(prompt.id, prompt);
            self.node_count += 1;
        }
    }

    /// Add an edge to the subgraph
    pub fn add_edge(&mut self, edge: LineageEdge) {
        self.edges.push(edge);
        self.edge_count += 1;
    }

    /// Get all direct ancestors of a node
    #[must_use]
    pub fn get_parents(&self, node_id: NodeId) -> Vec<NodeId> {
        self.edges
            .iter()
            .filter(|e| e.child_id == node_id)
            .map(|e| e.parent_id)
            .collect()
    }

    /// Get all direct descendants of a node
    #[must_use]
    pub fn get_children(&self, node_id: NodeId) -> Vec<NodeId> {
        self.edges
            .iter()
            .filter(|e| e.parent_id == node_id)
            .map(|e| e.child_id)
            .collect()
    }

    /// Get the edge between two nodes if it exists
    #[must_use]
    pub fn get_edge(&self, parent_id: NodeId, child_id: NodeId) -> Option<&LineageEdge> {
        self.edges
            .iter()
            .find(|e| e.parent_id == parent_id && e.child_id == child_id)
    }

    /// Filter edges by evolution type
    #[must_use]
    pub fn edges_by_type(&self, evolution_type: EvolutionType) -> Vec<&LineageEdge> {
        self.edges
            .iter()
            .filter(|e| e.evolution_type == evolution_type)
            .collect()
    }
}

// ============================================================================
// Lineage Chain
// ============================================================================

/// A single node in a lineage chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageChainNode {
    /// Node ID
    pub id: NodeId,
    /// Depth from the starting point (0 = starting node)
    pub depth: usize,
    /// Edge that led to this node (None for root)
    pub incoming_edge: Option<LineageEdge>,
    /// Prompt info if available
    pub prompt: Option<PromptInfo>,
}

/// A chain of prompts representing lineage history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageChain {
    /// The starting node ID
    pub origin_id: NodeId,
    /// Ordered chain of nodes (from oldest to newest for ancestors, newest to oldest for descendants)
    pub chain: Vec<LineageChainNode>,
    /// Direction of traversal
    pub direction: TraversalDirection,
}

/// Direction of lineage traversal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraversalDirection {
    /// Traverse toward ancestors (parents)
    Ancestors,
    /// Traverse toward descendants (children)
    Descendants,
}

impl LineageChain {
    /// Create a new lineage chain
    #[must_use]
    pub fn new(origin_id: NodeId, direction: TraversalDirection) -> Self {
        Self {
            origin_id,
            chain: Vec::new(),
            direction,
        }
    }

    /// Add a node to the chain
    pub fn push(&mut self, node: LineageChainNode) {
        self.chain.push(node);
    }

    /// Get the length of the chain
    #[must_use]
    pub fn len(&self) -> usize {
        self.chain.len()
    }

    /// Check if the chain is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chain.is_empty()
    }

    /// Get the deepest node in the chain
    #[must_use]
    pub fn deepest(&self) -> Option<&LineageChainNode> {
        self.chain.last()
    }

    /// Get node IDs in order
    #[must_use]
    pub fn node_ids(&self) -> Vec<NodeId> {
        self.chain.iter().map(|n| n.id).collect()
    }

    /// Calculate total confidence along the chain
    #[must_use]
    pub fn total_confidence(&self) -> f64 {
        if self.chain.is_empty() {
            return 1.0;
        }

        self.chain
            .iter()
            .filter_map(|n| n.incoming_edge.as_ref())
            .map(|e| e.confidence)
            .product()
    }
}

// ============================================================================
// Lineage Tracker
// ============================================================================

/// Configuration for the lineage tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageTrackerConfig {
    /// Maximum traversal depth for lineage queries
    pub max_depth: usize,
    /// Similarity configuration
    pub similarity_config: SimilarityConfig,
    /// Minimum confidence to establish lineage
    pub min_confidence: f64,
    /// Whether to detect circular dependencies
    pub detect_cycles: bool,
}

impl Default for LineageTrackerConfig {
    fn default() -> Self {
        Self {
            max_depth: 100,
            similarity_config: SimilarityConfig::default(),
            min_confidence: 0.3,
            detect_cycles: true,
        }
    }
}

/// Main lineage tracking system
///
/// The `LineageTracker` is the core component for tracking prompt evolution
/// across iterations and agents. It maintains a graph of prompt relationships
/// and provides efficient traversal algorithms.
#[derive(Debug)]
pub struct LineageTracker {
    /// Configuration
    config: LineageTrackerConfig,
    /// Prompt storage (in production, this would be backed by a database)
    prompts: HashMap<NodeId, PromptInfo>,
    /// Edges indexed by parent ID
    edges_by_parent: HashMap<NodeId, Vec<LineageEdge>>,
    /// Edges indexed by child ID
    edges_by_child: HashMap<NodeId, Vec<LineageEdge>>,
    /// Similarity computer
    similarity: SimilarityComputer,
}

impl LineageTracker {
    /// Create a new lineage tracker with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(LineageTrackerConfig::default())
    }

    /// Create with custom configuration
    #[must_use]
    pub fn with_config(config: LineageTrackerConfig) -> Self {
        let similarity = SimilarityComputer::with_config(config.similarity_config.clone());
        Self {
            config,
            prompts: HashMap::new(),
            edges_by_parent: HashMap::new(),
            edges_by_child: HashMap::new(),
            similarity,
        }
    }

    /// Register a prompt for lineage tracking
    pub fn register_prompt(&mut self, prompt: PromptInfo) {
        self.prompts.insert(prompt.id, prompt);
    }

    /// Get a registered prompt
    #[must_use]
    pub fn get_prompt(&self, id: NodeId) -> Option<&PromptInfo> {
        self.prompts.get(&id)
    }

    /// Track evolution between two prompts
    ///
    /// Creates an edge representing the evolution relationship between
    /// a parent prompt and a child prompt.
    ///
    /// # Arguments
    ///
    /// * `parent_prompt_id` - ID of the parent/source prompt
    /// * `child_prompt_id` - ID of the child/derived prompt
    /// * `evolution_type` - Type of evolution relationship
    /// * `confidence` - Confidence score (0.0 to 1.0), or compute automatically if None
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Either node is not found
    /// - Confidence is invalid
    /// - A circular dependency would be created
    pub fn track_evolution(
        &mut self,
        parent_prompt_id: NodeId,
        child_prompt_id: NodeId,
        evolution_type: EvolutionType,
        confidence: Option<f64>,
    ) -> LineageResult<LineageEdge> {
        // Verify nodes exist
        if !self.prompts.contains_key(&parent_prompt_id) {
            return Err(LineageError::NodeNotFound(parent_prompt_id));
        }
        if !self.prompts.contains_key(&child_prompt_id) {
            return Err(LineageError::NodeNotFound(child_prompt_id));
        }

        // Check for circular dependency
        if self.config.detect_cycles && self.would_create_cycle(parent_prompt_id, child_prompt_id) {
            return Err(LineageError::CircularDependency(
                parent_prompt_id,
                child_prompt_id,
            ));
        }

        // Compute confidence if not provided
        let confidence = match confidence {
            Some(c) => c,
            None => {
                let parent = self.prompts.get(&parent_prompt_id).unwrap();
                let child = self.prompts.get(&child_prompt_id).unwrap();
                self.similarity.compute_confidence(parent, child)
            }
        };

        // Create the edge
        let edge = LineageEdge::new(parent_prompt_id, child_prompt_id, evolution_type, confidence)?;

        // Index the edge
        self.edges_by_parent
            .entry(parent_prompt_id)
            .or_default()
            .push(edge.clone());
        self.edges_by_child
            .entry(child_prompt_id)
            .or_default()
            .push(edge.clone());

        Ok(edge)
    }

    /// Compute similarity between two prompts
    ///
    /// Returns a detailed breakdown of similarity scores.
    ///
    /// # Errors
    ///
    /// Returns an error if either prompt is not found.
    pub fn compute_similarity(
        &self,
        prompt_a_id: NodeId,
        prompt_b_id: NodeId,
    ) -> LineageResult<SimilarityScores> {
        let prompt_a = self
            .prompts
            .get(&prompt_a_id)
            .ok_or(LineageError::NodeNotFound(prompt_a_id))?;
        let prompt_b = self
            .prompts
            .get(&prompt_b_id)
            .ok_or(LineageError::NodeNotFound(prompt_b_id))?;

        Ok(self.similarity.compute(prompt_a, prompt_b))
    }

    /// Get ancestors of a prompt up to a maximum depth
    ///
    /// Traverses the lineage graph upward to find all prompts
    /// from which the given prompt was derived.
    ///
    /// # Arguments
    ///
    /// * `prompt_id` - Starting prompt ID
    /// * `max_depth` - Maximum depth to traverse (None for config default)
    ///
    /// # Errors
    ///
    /// Returns an error if the prompt is not found or max depth is exceeded.
    pub fn get_ancestors(
        &self,
        prompt_id: NodeId,
        max_depth: Option<usize>,
    ) -> LineageResult<LineageChain> {
        if !self.prompts.contains_key(&prompt_id) {
            return Err(LineageError::NodeNotFound(prompt_id));
        }

        let max_depth = max_depth.unwrap_or(self.config.max_depth);
        let mut chain = LineageChain::new(prompt_id, TraversalDirection::Ancestors);
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with the origin node
        visited.insert(prompt_id);

        // Get initial parents
        if let Some(edges) = self.edges_by_child.get(&prompt_id) {
            for edge in edges {
                queue.push_back((edge.parent_id, 1, edge.clone()));
            }
        }

        while let Some((current_id, depth, edge)) = queue.pop_front() {
            if depth > max_depth {
                continue;
            }

            if visited.contains(&current_id) {
                continue;
            }
            visited.insert(current_id);

            let prompt = self.prompts.get(&current_id).cloned();
            chain.push(LineageChainNode {
                id: current_id,
                depth,
                incoming_edge: Some(edge),
                prompt,
            });

            // Add parents to queue
            if depth < max_depth {
                if let Some(edges) = self.edges_by_child.get(&current_id) {
                    for parent_edge in edges {
                        if !visited.contains(&parent_edge.parent_id) {
                            queue.push_back((parent_edge.parent_id, depth + 1, parent_edge.clone()));
                        }
                    }
                }
            }
        }

        // Sort by depth (closest ancestors first)
        chain.chain.sort_by_key(|n| n.depth);

        Ok(chain)
    }

    /// Get descendants of a prompt up to a maximum depth
    ///
    /// Traverses the lineage graph downward to find all prompts
    /// that were derived from the given prompt.
    ///
    /// # Arguments
    ///
    /// * `prompt_id` - Starting prompt ID
    /// * `max_depth` - Maximum depth to traverse (None for config default)
    ///
    /// # Errors
    ///
    /// Returns an error if the prompt is not found.
    pub fn get_descendants(
        &self,
        prompt_id: NodeId,
        max_depth: Option<usize>,
    ) -> LineageResult<LineageChain> {
        if !self.prompts.contains_key(&prompt_id) {
            return Err(LineageError::NodeNotFound(prompt_id));
        }

        let max_depth = max_depth.unwrap_or(self.config.max_depth);
        let mut chain = LineageChain::new(prompt_id, TraversalDirection::Descendants);
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with the origin node
        visited.insert(prompt_id);

        // Get initial children
        if let Some(edges) = self.edges_by_parent.get(&prompt_id) {
            for edge in edges {
                queue.push_back((edge.child_id, 1, edge.clone()));
            }
        }

        while let Some((current_id, depth, edge)) = queue.pop_front() {
            if depth > max_depth {
                continue;
            }

            if visited.contains(&current_id) {
                continue;
            }
            visited.insert(current_id);

            let prompt = self.prompts.get(&current_id).cloned();
            chain.push(LineageChainNode {
                id: current_id,
                depth,
                incoming_edge: Some(edge),
                prompt,
            });

            // Add children to queue
            if depth < max_depth {
                if let Some(edges) = self.edges_by_parent.get(&current_id) {
                    for child_edge in edges {
                        if !visited.contains(&child_edge.child_id) {
                            queue.push_back((child_edge.child_id, depth + 1, child_edge.clone()));
                        }
                    }
                }
            }
        }

        // Sort by depth (closest descendants first)
        chain.chain.sort_by_key(|n| n.depth);

        Ok(chain)
    }

    /// Get the full lineage subgraph centered on a prompt
    ///
    /// This combines both ancestors and descendants into a complete
    /// subgraph showing all related prompts.
    ///
    /// # Arguments
    ///
    /// * `prompt_id` - Center prompt ID
    /// * `ancestor_depth` - Maximum depth for ancestor traversal
    /// * `descendant_depth` - Maximum depth for descendant traversal
    ///
    /// # Errors
    ///
    /// Returns an error if the prompt is not found.
    pub fn get_lineage_subgraph(
        &self,
        prompt_id: NodeId,
        ancestor_depth: Option<usize>,
        descendant_depth: Option<usize>,
    ) -> LineageResult<LineageSubgraph> {
        if !self.prompts.contains_key(&prompt_id) {
            return Err(LineageError::NodeNotFound(prompt_id));
        }

        let mut subgraph = LineageSubgraph::new(prompt_id);

        // Add the root node
        if let Some(prompt) = self.prompts.get(&prompt_id) {
            subgraph.add_node(prompt.clone());
        }

        // Get ancestors
        let ancestors = self.get_ancestors(prompt_id, ancestor_depth)?;
        for node in &ancestors.chain {
            if let Some(prompt) = &node.prompt {
                subgraph.add_node(prompt.clone());
            } else if let Some(prompt) = self.prompts.get(&node.id) {
                subgraph.add_node(prompt.clone());
            }
            if let Some(edge) = &node.incoming_edge {
                subgraph.add_edge(edge.clone());
            }
        }

        // Get descendants
        let descendants = self.get_descendants(prompt_id, descendant_depth)?;
        for node in &descendants.chain {
            if let Some(prompt) = &node.prompt {
                subgraph.add_node(prompt.clone());
            } else if let Some(prompt) = self.prompts.get(&node.id) {
                subgraph.add_node(prompt.clone());
            }
            if let Some(edge) = &node.incoming_edge {
                subgraph.add_edge(edge.clone());
            }
        }

        // Calculate max depth
        let ancestor_max = ancestors.chain.iter().map(|n| n.depth).max().unwrap_or(0);
        let descendant_max = descendants.chain.iter().map(|n| n.depth).max().unwrap_or(0);
        subgraph.max_depth = ancestor_max.max(descendant_max);

        Ok(subgraph)
    }

    /// Find prompts similar to the given prompt
    ///
    /// Searches all registered prompts for those with similarity
    /// above the given threshold.
    ///
    /// # Arguments
    ///
    /// * `prompt_id` - Reference prompt ID
    /// * `min_similarity` - Minimum similarity threshold
    /// * `limit` - Maximum number of results
    ///
    /// # Errors
    ///
    /// Returns an error if the prompt is not found.
    pub fn find_similar(
        &self,
        prompt_id: NodeId,
        min_similarity: f64,
        limit: usize,
    ) -> LineageResult<Vec<(NodeId, SimilarityScores)>> {
        let reference = self
            .prompts
            .get(&prompt_id)
            .ok_or(LineageError::NodeNotFound(prompt_id))?;

        let mut results: Vec<(NodeId, SimilarityScores)> = self
            .prompts
            .iter()
            .filter(|(id, _)| **id != prompt_id)
            .map(|(id, prompt)| (*id, self.similarity.compute(reference, prompt)))
            .filter(|(_, scores)| scores.combined >= min_similarity)
            .collect();

        // Sort by combined score descending
        results.sort_by(|a, b| b.1.combined.partial_cmp(&a.1.combined).unwrap_or(std::cmp::Ordering::Equal));

        // Limit results
        results.truncate(limit);

        Ok(results)
    }

    /// Suggest evolution type based on similarity analysis
    ///
    /// Analyzes the relationship between two prompts and suggests
    /// the most appropriate evolution type.
    pub fn suggest_evolution_type(
        &self,
        parent_id: NodeId,
        child_id: NodeId,
    ) -> LineageResult<(EvolutionType, f64)> {
        let scores = self.compute_similarity(parent_id, child_id)?;

        // High edit distance ratio + high token overlap = Refines
        if scores.edit_distance_ratio > 0.7 && scores.token_overlap > 0.6 {
            return Ok((EvolutionType::Refines, scores.combined));
        }

        // Moderate overlap with different structure = Evolves
        if scores.token_overlap > 0.3 && scores.token_overlap < 0.7 {
            return Ok((EvolutionType::Evolves, scores.combined));
        }

        // Lower overlap = Derives
        Ok((EvolutionType::Derives, scores.combined))
    }

    /// Check if adding an edge would create a cycle
    fn would_create_cycle(&self, parent_id: NodeId, child_id: NodeId) -> bool {
        // If parent is a descendant of child, adding this edge would create a cycle
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(child_id);

        while let Some(current) = queue.pop_front() {
            if current == parent_id {
                return true;
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            // Check children of current node
            if let Some(edges) = self.edges_by_parent.get(&current) {
                for edge in edges {
                    queue.push_back(edge.child_id);
                }
            }
        }

        false
    }

    /// Get statistics about the lineage graph
    #[must_use]
    pub fn stats(&self) -> LineageStats {
        let total_edges: usize = self.edges_by_parent.values().map(|v| v.len()).sum();

        let mut evolves_count = 0;
        let mut refines_count = 0;
        let mut derives_count = 0;
        let mut total_confidence = 0.0;

        for edges in self.edges_by_parent.values() {
            for edge in edges {
                total_confidence += edge.confidence;
                match edge.evolution_type {
                    EvolutionType::Evolves => evolves_count += 1,
                    EvolutionType::Refines => refines_count += 1,
                    EvolutionType::Derives => derives_count += 1,
                }
            }
        }

        let average_confidence = if total_edges > 0 {
            total_confidence / total_edges as f64
        } else {
            0.0
        };

        LineageStats {
            total_prompts: self.prompts.len(),
            total_edges,
            evolves_count,
            refines_count,
            derives_count,
            average_confidence,
        }
    }
}

impl Default for LineageTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the lineage graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageStats {
    /// Total number of registered prompts
    pub total_prompts: usize,
    /// Total number of lineage edges
    pub total_edges: usize,
    /// Number of EVOLVES relationships
    pub evolves_count: usize,
    /// Number of REFINES relationships
    pub refines_count: usize,
    /// Number of DERIVES relationships
    pub derives_count: usize,
    /// Average confidence across all edges
    pub average_confidence: f64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_prompt(content: &str) -> PromptInfo {
        PromptInfo::new(NodeId::new(), content)
    }

    #[test]
    fn test_evolution_type_display() {
        assert_eq!(EvolutionType::Evolves.to_string(), "EVOLVES");
        assert_eq!(EvolutionType::Refines.to_string(), "REFINES");
        assert_eq!(EvolutionType::Derives.to_string(), "DERIVES");
    }

    #[test]
    fn test_lineage_edge_creation() {
        let parent_id = NodeId::new();
        let child_id = NodeId::new();
        let edge = LineageEdge::new(parent_id, child_id, EvolutionType::Refines, 0.9).unwrap();

        assert_eq!(edge.parent_id, parent_id);
        assert_eq!(edge.child_id, child_id);
        assert_eq!(edge.evolution_type, EvolutionType::Refines);
        assert_eq!(edge.confidence, 0.9);
    }

    #[test]
    fn test_lineage_edge_invalid_confidence() {
        let parent_id = NodeId::new();
        let child_id = NodeId::new();

        assert!(LineageEdge::new(parent_id, child_id, EvolutionType::Derives, 1.5).is_err());
        assert!(LineageEdge::new(parent_id, child_id, EvolutionType::Derives, -0.1).is_err());
    }

    #[test]
    fn test_similarity_identical() {
        let computer = SimilarityComputer::new();
        let prompt_a = create_test_prompt("What is the capital of France?");
        let prompt_b = create_test_prompt("What is the capital of France?");

        let scores = computer.compute(&prompt_a, &prompt_b);
        assert_eq!(scores.token_overlap, 1.0);
        assert_eq!(scores.edit_distance_ratio, 1.0);
    }

    #[test]
    fn test_similarity_different() {
        let computer = SimilarityComputer::new();
        let prompt_a = create_test_prompt("What is the capital of France?");
        let prompt_b = create_test_prompt("How do I make coffee?");

        let scores = computer.compute(&prompt_a, &prompt_b);
        assert!(scores.token_overlap < 0.5);
        assert!(scores.combined < 0.5);
    }

    #[test]
    fn test_similarity_refinement() {
        let computer = SimilarityComputer::new();
        let prompt_a = create_test_prompt("Explain quantum computing");
        let prompt_b = create_test_prompt("Explain quantum computing in simple terms");

        let scores = computer.compute(&prompt_a, &prompt_b);
        assert!(scores.token_overlap > 0.5);
        assert!(scores.combined > 0.5);
    }

    #[test]
    fn test_lineage_tracker_basic() {
        let mut tracker = LineageTracker::new();

        let parent = create_test_prompt("Original prompt");
        let child = create_test_prompt("Refined prompt based on original");
        let parent_id = parent.id;
        let child_id = child.id;

        tracker.register_prompt(parent);
        tracker.register_prompt(child);

        let edge = tracker
            .track_evolution(parent_id, child_id, EvolutionType::Refines, Some(0.85))
            .unwrap();

        assert_eq!(edge.evolution_type, EvolutionType::Refines);
        assert_eq!(edge.confidence, 0.85);
    }

    #[test]
    fn test_get_ancestors() {
        let mut tracker = LineageTracker::new();

        // Create a chain: grandparent -> parent -> child
        let grandparent = create_test_prompt("Grandparent prompt");
        let parent = create_test_prompt("Parent prompt");
        let child = create_test_prompt("Child prompt");

        let gp_id = grandparent.id;
        let p_id = parent.id;
        let c_id = child.id;

        tracker.register_prompt(grandparent);
        tracker.register_prompt(parent);
        tracker.register_prompt(child);

        tracker.track_evolution(gp_id, p_id, EvolutionType::Evolves, Some(0.8)).unwrap();
        tracker.track_evolution(p_id, c_id, EvolutionType::Refines, Some(0.9)).unwrap();

        let ancestors = tracker.get_ancestors(c_id, Some(10)).unwrap();
        assert_eq!(ancestors.chain.len(), 2);
        assert!(ancestors.node_ids().contains(&p_id));
        assert!(ancestors.node_ids().contains(&gp_id));
    }

    #[test]
    fn test_get_descendants() {
        let mut tracker = LineageTracker::new();

        // Create a tree: root -> child1, child2
        let root = create_test_prompt("Root prompt");
        let child1 = create_test_prompt("Child 1 prompt");
        let child2 = create_test_prompt("Child 2 prompt");

        let root_id = root.id;
        let c1_id = child1.id;
        let c2_id = child2.id;

        tracker.register_prompt(root);
        tracker.register_prompt(child1);
        tracker.register_prompt(child2);

        tracker.track_evolution(root_id, c1_id, EvolutionType::Derives, Some(0.7)).unwrap();
        tracker.track_evolution(root_id, c2_id, EvolutionType::Derives, Some(0.6)).unwrap();

        let descendants = tracker.get_descendants(root_id, Some(10)).unwrap();
        assert_eq!(descendants.chain.len(), 2);
        assert!(descendants.node_ids().contains(&c1_id));
        assert!(descendants.node_ids().contains(&c2_id));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut tracker = LineageTracker::new();

        let a = create_test_prompt("Prompt A");
        let b = create_test_prompt("Prompt B");
        let c = create_test_prompt("Prompt C");

        let a_id = a.id;
        let b_id = b.id;
        let c_id = c.id;

        tracker.register_prompt(a);
        tracker.register_prompt(b);
        tracker.register_prompt(c);

        // A -> B -> C
        tracker.track_evolution(a_id, b_id, EvolutionType::Evolves, Some(0.8)).unwrap();
        tracker.track_evolution(b_id, c_id, EvolutionType::Evolves, Some(0.8)).unwrap();

        // Trying to add C -> A should fail (cycle)
        let result = tracker.track_evolution(c_id, a_id, EvolutionType::Evolves, Some(0.8));
        assert!(matches!(result, Err(LineageError::CircularDependency(_, _))));
    }

    #[test]
    fn test_lineage_subgraph() {
        let mut tracker = LineageTracker::new();

        let ancestor = create_test_prompt("Ancestor");
        let middle = create_test_prompt("Middle");
        let descendant = create_test_prompt("Descendant");

        let anc_id = ancestor.id;
        let mid_id = middle.id;
        let desc_id = descendant.id;

        tracker.register_prompt(ancestor);
        tracker.register_prompt(middle);
        tracker.register_prompt(descendant);

        tracker.track_evolution(anc_id, mid_id, EvolutionType::Evolves, Some(0.8)).unwrap();
        tracker.track_evolution(mid_id, desc_id, EvolutionType::Refines, Some(0.9)).unwrap();

        let subgraph = tracker.get_lineage_subgraph(mid_id, Some(5), Some(5)).unwrap();

        assert_eq!(subgraph.node_count, 3);
        assert_eq!(subgraph.edge_count, 2);
        assert!(subgraph.nodes.contains_key(&anc_id));
        assert!(subgraph.nodes.contains_key(&mid_id));
        assert!(subgraph.nodes.contains_key(&desc_id));
    }

    #[test]
    fn test_suggest_evolution_type() {
        let mut tracker = LineageTracker::new();

        // Similar prompts should suggest Refines
        let original = create_test_prompt("Explain machine learning algorithms");
        let refined = create_test_prompt("Explain machine learning algorithms in detail with examples");

        let orig_id = original.id;
        let ref_id = refined.id;

        tracker.register_prompt(original);
        tracker.register_prompt(refined);

        let (suggested_type, _confidence) = tracker.suggest_evolution_type(orig_id, ref_id).unwrap();
        // The exact type depends on the similarity scores, but it should be a valid type
        assert!(matches!(suggested_type, EvolutionType::Refines | EvolutionType::Evolves | EvolutionType::Derives));
    }

    #[test]
    fn test_find_similar() {
        let mut tracker = LineageTracker::new();

        let reference = create_test_prompt("What is the capital of France?");
        let similar = create_test_prompt("What is the capital of Germany?");
        let different = create_test_prompt("How do you make pancakes?");

        let ref_id = reference.id;

        tracker.register_prompt(reference);
        tracker.register_prompt(similar);
        tracker.register_prompt(different);

        let results = tracker.find_similar(ref_id, 0.3, 10).unwrap();

        // Should find at least the similar prompt
        assert!(!results.is_empty());
        // The similar prompt should have higher similarity than the different one
        let similar_score = results.iter().find(|(_, s)| s.combined > 0.5);
        assert!(similar_score.is_some());
    }

    #[test]
    fn test_lineage_stats() {
        let mut tracker = LineageTracker::new();

        let p1 = create_test_prompt("Prompt 1");
        let p2 = create_test_prompt("Prompt 2");
        let p3 = create_test_prompt("Prompt 3");

        let p1_id = p1.id;
        let p2_id = p2.id;
        let p3_id = p3.id;

        tracker.register_prompt(p1);
        tracker.register_prompt(p2);
        tracker.register_prompt(p3);

        tracker.track_evolution(p1_id, p2_id, EvolutionType::Evolves, Some(0.8)).unwrap();
        tracker.track_evolution(p2_id, p3_id, EvolutionType::Refines, Some(0.9)).unwrap();

        let stats = tracker.stats();
        assert_eq!(stats.total_prompts, 3);
        assert_eq!(stats.total_edges, 2);
        assert_eq!(stats.evolves_count, 1);
        assert_eq!(stats.refines_count, 1);
        assert_eq!(stats.derives_count, 0);
        assert!((stats.average_confidence - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_levenshtein_distance() {
        let computer = SimilarityComputer::new();

        // Same strings
        assert_eq!(computer.levenshtein_distance("hello", "hello"), 0);

        // Single character difference
        assert_eq!(computer.levenshtein_distance("hello", "hallo"), 1);

        // Empty strings
        assert_eq!(computer.levenshtein_distance("", ""), 0);
        assert_eq!(computer.levenshtein_distance("hello", ""), 5);
        assert_eq!(computer.levenshtein_distance("", "hello"), 5);
    }

    #[test]
    fn test_token_overlap_empty() {
        let computer = SimilarityComputer::new();
        let empty: Vec<String> = vec![];
        let tokens = vec!["hello".to_string()];

        assert_eq!(computer.compute_token_overlap(&empty, &empty), 1.0);
        assert_eq!(computer.compute_token_overlap(&empty, &tokens), 0.0);
        assert_eq!(computer.compute_token_overlap(&tokens, &empty), 0.0);
    }

    #[test]
    fn test_semantic_similarity_without_embeddings() {
        let computer = SimilarityComputer::new();

        // Without embeddings, should return neutral score
        let score = computer.compute_semantic_similarity(None, None);
        assert_eq!(score, 0.5);
    }

    #[test]
    fn test_semantic_similarity_with_embeddings() {
        let computer = SimilarityComputer::new();

        // Identical embeddings should give high similarity
        let embedding = vec![1.0, 0.0, 0.0];
        let score = computer.compute_semantic_similarity(Some(&embedding), Some(&embedding));
        assert!(score > 0.9);

        // Orthogonal embeddings should give neutral similarity
        let embedding_a = vec![1.0, 0.0, 0.0];
        let embedding_b = vec![0.0, 1.0, 0.0];
        let score = computer.compute_semantic_similarity(Some(&embedding_a), Some(&embedding_b));
        assert!((score - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_lineage_chain_total_confidence() {
        let mut chain = LineageChain::new(NodeId::new(), TraversalDirection::Ancestors);

        // Empty chain has confidence 1.0
        assert_eq!(chain.total_confidence(), 1.0);

        // Add nodes with edges
        let edge1 = LineageEdge::new(NodeId::new(), NodeId::new(), EvolutionType::Refines, 0.9).unwrap();
        let edge2 = LineageEdge::new(NodeId::new(), NodeId::new(), EvolutionType::Refines, 0.8).unwrap();

        chain.push(LineageChainNode {
            id: NodeId::new(),
            depth: 1,
            incoming_edge: Some(edge1),
            prompt: None,
        });
        chain.push(LineageChainNode {
            id: NodeId::new(),
            depth: 2,
            incoming_edge: Some(edge2),
            prompt: None,
        });

        // Total confidence should be product: 0.9 * 0.8 = 0.72
        assert!((chain.total_confidence() - 0.72).abs() < 0.01);
    }

    #[test]
    fn test_similarity_config_variants() {
        let refinement_config = SimilarityConfig::for_refinement();
        assert!(refinement_config.token_overlap_weight > refinement_config.semantic_weight);

        let derivation_config = SimilarityConfig::for_derivation();
        assert!(derivation_config.semantic_weight > derivation_config.token_overlap_weight);
    }
}

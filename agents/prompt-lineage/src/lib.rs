//! Prompt Lineage Agent for LLM Memory Graph
//!
//! This crate provides comprehensive prompt evolution tracking and lineage
//! analysis for the LLM Memory Graph system. It enables tracking how prompts
//! evolve, refine, and derive from each other across iterations and agents.
//!
//! # Overview
//!
//! The prompt lineage system tracks three types of evolution relationships:
//!
//! - **Evolves**: Same intent, modified approach - used when the prompt's
//!   goal remains the same but the method changes
//! - **Refines**: Improved version of the same prompt - used for iterative
//!   refinements that enhance clarity or effectiveness
//! - **Derives**: New prompt derived from original - used when creating
//!   a new prompt inspired by or based on another
//!
//! # Features
//!
//! - **Evolution Tracking**: Record and query prompt evolution relationships
//! - **Similarity Computation**: Multi-factor similarity analysis including
//!   token overlap, semantic similarity, and edit distance
//! - **Graph Traversal**: Efficient algorithms for ancestor/descendant queries
//! - **Subgraph Extraction**: Extract relevant portions of the lineage graph
//! - **Cycle Detection**: Prevent circular dependencies in the lineage graph
//!
//! # Example
//!
//! ```rust
//! use prompt_lineage::{LineageTracker, PromptInfo, EvolutionType, NodeId};
//!
//! // Create a lineage tracker
//! let mut tracker = LineageTracker::new();
//!
//! // Register prompts
//! let original = PromptInfo::new(NodeId::new(), "Explain quantum computing");
//! let refined = PromptInfo::new(NodeId::new(), "Explain quantum computing in simple terms for beginners");
//! let original_id = original.id;
//! let refined_id = refined.id;
//!
//! tracker.register_prompt(original);
//! tracker.register_prompt(refined);
//!
//! // Track the evolution
//! let edge = tracker.track_evolution(
//!     original_id,
//!     refined_id,
//!     EvolutionType::Refines,
//!     Some(0.85)
//! ).unwrap();
//!
//! // Query ancestors
//! let ancestors = tracker.get_ancestors(refined_id, Some(5)).unwrap();
//! assert_eq!(ancestors.len(), 1);
//!
//! // Get the full lineage subgraph
//! let subgraph = tracker.get_lineage_subgraph(refined_id, Some(5), Some(5)).unwrap();
//! assert_eq!(subgraph.node_count, 2);
//! ```
//!
//! # Architecture
//!
//! The lineage system is built on the following key components:
//!
//! - [`LineageTracker`]: Main entry point for all lineage operations
//! - [`LineageEdge`]: Represents a directed evolution relationship
//! - [`PromptInfo`]: Prompt data structure optimized for similarity computation
//! - [`SimilarityComputer`]: Multi-factor similarity analysis engine
//! - [`LineageSubgraph`]: Extracted subgraph for visualization/analysis
//!
//! # Performance
//!
//! The lineage tracker uses efficient indexing structures for fast traversal:
//!
//! - O(1) prompt lookup by ID
//! - O(k) ancestor/descendant traversal where k is the number of edges
//! - O(n) similarity search where n is the number of prompts
//!
//! For large-scale deployments, consider using the vector search features
//! of the main memory graph for semantic similarity queries.

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]

/// Contract definitions for the Prompt Lineage Agent
///
/// This module defines the schemas and contracts for tracking prompt evolution
/// and lineage across iterations and agents. It integrates with ruvector-service
/// for decision event tracking.
///
/// # Agent Classification
/// - **Type**: MEMORY WRITE
/// - **Decision Type**: `prompt_lineage_tracking`
///
/// # Key Types
///
/// - [`LineageInput`](contracts::LineageInput) - Input schema for lineage tracking requests
/// - [`LineageOutput`](contracts::LineageOutput) - Output schema for lineage tracking results
/// - [`DecisionEvent`](contracts::DecisionEvent) - Decision event for ruvector-service
/// - [`LineageNode`](contracts::LineageNode) - Node type extending PromptNode for lineage
/// - [`LineageEdge`](contracts::LineageEdge) - Edge properties for evolution tracking
pub mod contracts;

pub mod lineage;

/// CLI commands for the Prompt Lineage Agent
///
/// This module provides command-line interface functionality for inspecting,
/// retrieving, and replaying prompt lineage data from the memory graph.
///
/// # Commands
///
/// - `lineage inspect <prompt_id>` - Inspect lineage of a prompt
/// - `lineage retrieve <prompt_id>` - Retrieve full lineage subgraph
/// - `lineage replay <prompt_id>` - Replay lineage creation step-by-step
///
/// # Output Formats
///
/// All commands support multiple output formats:
/// - `text` - Human-readable colored output (default)
/// - `json` - JSON format for programmatic consumption
/// - `yaml` - YAML format for configuration files
/// - `table` - Tabular format for structured display
pub mod cli;

/// Edge Function HTTP Handler for Google Cloud deployment
///
/// This module provides a stateless, deterministic HTTP handler for the
/// Prompt Lineage Agent, designed for deployment as a Google Cloud Edge Function.
///
/// # Classification
///
/// - **Agent ID**: `prompt-lineage-agent`
/// - **Version**: `1.0.0`
/// - **Classification**: `MEMORY WRITE`
/// - **Decision Type**: `prompt_lineage_tracking`
///
/// # Features
///
/// - HTTP request/response handling
/// - Input validation using contracts
/// - Lineage node and edge creation
/// - Confidence score computation (semantic similarity, token overlap, edit distance)
/// - DecisionEvent emission to ruvector-service
/// - Telemetry events for Observatory
///
/// # Usage
///
/// ```rust,no_run
/// use prompt_lineage_agent::handler::{PromptLineageHandler, HttpRequest};
/// use std::collections::HashMap;
///
/// #[tokio::main]
/// async fn main() {
///     let handler = PromptLineageHandler::with_defaults().unwrap();
///
///     let request = HttpRequest {
///         method: "POST".to_string(),
///         path: "/".to_string(),
///         headers: HashMap::new(),
///         body: Some(serde_json::json!({
///             "session_id": "550e8400-e29b-41d4-a716-446655440000",
///             "prompts": [],
///             "relations": []
///         })),
///     };
///
///     let response = handler.handle_http(request).await;
///     println!("Status: {}", response.status_code);
/// }
/// ```
pub mod handler;

// Re-export main types for convenience
pub use lineage::{
    // Core types
    EdgeId,
    EvolutionType,
    LineageChain,
    LineageChainNode,
    LineageEdge,
    LineageError,
    LineageResult,
    LineageStats,
    LineageSubgraph,
    LineageTracker,
    LineageTrackerConfig,
    NodeId,
    PromptInfo,
    // Similarity
    SimilarityComputer,
    SimilarityConfig,
    SimilarityScores,
    // Traversal
    TraversalDirection,
};

// Re-export contract types for ruvector-service integration
pub use contracts::{
    // Decision event types
    DecisionEvent,
    DecisionOutputs,
    DecisionType,
    ExecutionRef,
    // Input/Output schemas
    LineageInput,
    LineageOutput,
    LineageOutputStats,
    // Lineage node and edge types
    LineageNode as ContractLineageNode,
    LineageEdge as ContractLineageEdge,
    LineageEdgeType,
    LineageMetrics,
    DiffSummary,
    // Evolution types
    EvolutionType as ContractEvolutionType,
    EvolutionReason,
    EvolutionCategory,
    TriggerSource,
    PerformanceDelta,
    // Graph constraints and validation
    GraphConstraint,
    ValidationError,
    LineageConfig,
    // Session types
    SessionId,
};

/// Prelude module for convenient imports
pub mod prelude {
    //! Prelude module providing the most commonly used types
    //!
    //! # Usage
    //!
    //! ```rust
    //! use prompt_lineage::prelude::*;
    //! ```

    pub use crate::{
        // Core lineage types
        EvolutionType, LineageChain, LineageEdge, LineageError, LineageResult, LineageSubgraph,
        LineageTracker, LineageTrackerConfig, NodeId, PromptInfo, SimilarityComputer,
        SimilarityConfig, SimilarityScores, TraversalDirection,
        // Contract types for ruvector integration
        DecisionEvent, DecisionOutputs, DecisionType, ExecutionRef,
        LineageInput, LineageOutput, LineageOutputStats,
        ContractLineageNode, ContractLineageEdge, LineageEdgeType,
        GraphConstraint, ValidationError, LineageConfig, SessionId,
    };
}

#[cfg(test)]
mod integration_tests {
    use super::prelude::*;

    #[test]
    fn test_full_workflow() {
        // Create tracker
        let mut tracker = LineageTracker::new();

        // Create a prompt evolution chain:
        // v1 -> v2 (refines) -> v3 (evolves)
        //    \-> v2_alt (derives)

        let v1 = PromptInfo::new(NodeId::new(), "Write a function to sort an array");
        let v2 = PromptInfo::new(
            NodeId::new(),
            "Write a function to sort an array using quicksort algorithm",
        );
        let v3 = PromptInfo::new(
            NodeId::new(),
            "Implement quicksort with in-place partitioning for memory efficiency",
        );
        let v2_alt = PromptInfo::new(
            NodeId::new(),
            "Create a generic sorting function that works with any comparable type",
        );

        let v1_id = v1.id;
        let v2_id = v2.id;
        let v3_id = v3.id;
        let v2_alt_id = v2_alt.id;

        // Register all prompts
        tracker.register_prompt(v1);
        tracker.register_prompt(v2);
        tracker.register_prompt(v3);
        tracker.register_prompt(v2_alt);

        // Track evolution
        tracker
            .track_evolution(v1_id, v2_id, EvolutionType::Refines, Some(0.85))
            .unwrap();
        tracker
            .track_evolution(v2_id, v3_id, EvolutionType::Evolves, Some(0.75))
            .unwrap();
        tracker
            .track_evolution(v1_id, v2_alt_id, EvolutionType::Derives, Some(0.6))
            .unwrap();

        // Verify ancestors of v3
        let ancestors = tracker.get_ancestors(v3_id, None).unwrap();
        assert_eq!(ancestors.len(), 2); // v2 and v1

        // Verify descendants of v1
        let descendants = tracker.get_descendants(v1_id, None).unwrap();
        assert_eq!(descendants.len(), 3); // v2, v3, v2_alt

        // Get subgraph centered on v2
        let subgraph = tracker.get_lineage_subgraph(v2_id, None, None).unwrap();
        assert_eq!(subgraph.node_count, 3); // v1, v2, v3 (not v2_alt as it's not connected through v2)

        // Check stats
        let stats = tracker.stats();
        assert_eq!(stats.total_prompts, 4);
        assert_eq!(stats.total_edges, 3);
        assert_eq!(stats.refines_count, 1);
        assert_eq!(stats.evolves_count, 1);
        assert_eq!(stats.derives_count, 1);
    }

    #[test]
    fn test_similarity_based_suggestion() {
        let mut tracker = LineageTracker::new();

        let original = PromptInfo::new(
            NodeId::new(),
            "Explain the concept of machine learning to a beginner",
        );
        let minor_edit = PromptInfo::new(
            NodeId::new(),
            "Explain the concept of machine learning to a complete beginner",
        );
        let major_change = PromptInfo::new(
            NodeId::new(),
            "Compare supervised and unsupervised machine learning approaches",
        );

        let orig_id = original.id;
        let minor_id = minor_edit.id;
        let major_id = major_change.id;

        tracker.register_prompt(original);
        tracker.register_prompt(minor_edit);
        tracker.register_prompt(major_change);

        // Minor edit should suggest Refines (high similarity)
        let (suggested, confidence) = tracker.suggest_evolution_type(orig_id, minor_id).unwrap();
        assert!(confidence > 0.7);
        // The exact type depends on thresholds, but should be Refines or Evolves for high similarity
        assert!(matches!(
            suggested,
            EvolutionType::Refines | EvolutionType::Evolves
        ));

        // Major change should suggest lower confidence
        let (_, confidence) = tracker.suggest_evolution_type(orig_id, major_id).unwrap();
        // Still related but less similar
        assert!(confidence < 0.9);
    }
}

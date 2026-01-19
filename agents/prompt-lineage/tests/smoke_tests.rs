//! Smoke Tests for Prompt Lineage Agent
//!
//! These tests verify basic functionality and ensure the agent
//! is properly deployed and accessible.

use prompt_lineage_agent::{
    EvolutionType, LineageTracker, LineageTrackerConfig, NodeId, PromptInfo, SimilarityComputer,
    SimilarityConfig,
};

// =============================================================================
// Smoke Tests - Basic Functionality
// =============================================================================

#[test]
fn smoke_test_lineage_tracker_creation() {
    let tracker = LineageTracker::new();
    let stats = tracker.stats();
    assert_eq!(stats.total_prompts, 0);
    assert_eq!(stats.total_edges, 0);
}

#[test]
fn smoke_test_lineage_tracker_with_config() {
    let config = LineageTrackerConfig {
        max_depth: 50,
        similarity_config: SimilarityConfig::default(),
        min_confidence: 0.5,
        detect_cycles: true,
    };
    let tracker = LineageTracker::with_config(config);
    let stats = tracker.stats();
    assert_eq!(stats.total_prompts, 0);
}

#[test]
fn smoke_test_prompt_registration() {
    let mut tracker = LineageTracker::new();

    let prompt = PromptInfo::new(NodeId::new(), "Test prompt content");
    let prompt_id = prompt.id;

    tracker.register_prompt(prompt);

    assert!(tracker.get_prompt(prompt_id).is_some());
}

#[test]
fn smoke_test_evolution_tracking() {
    let mut tracker = LineageTracker::new();

    let parent = PromptInfo::new(NodeId::new(), "Parent prompt");
    let child = PromptInfo::new(NodeId::new(), "Child prompt derived from parent");
    let parent_id = parent.id;
    let child_id = child.id;

    tracker.register_prompt(parent);
    tracker.register_prompt(child);

    let edge = tracker
        .track_evolution(parent_id, child_id, EvolutionType::Derives, Some(0.7))
        .expect("Should create evolution edge");

    assert_eq!(edge.parent_id, parent_id);
    assert_eq!(edge.child_id, child_id);
    assert_eq!(edge.confidence, 0.7);
}

#[test]
fn smoke_test_ancestor_retrieval() {
    let mut tracker = LineageTracker::new();

    let root = PromptInfo::new(NodeId::new(), "Root prompt");
    let child = PromptInfo::new(NodeId::new(), "Child prompt");
    let root_id = root.id;
    let child_id = child.id;

    tracker.register_prompt(root);
    tracker.register_prompt(child);
    tracker
        .track_evolution(root_id, child_id, EvolutionType::Evolves, Some(0.8))
        .unwrap();

    let ancestors = tracker.get_ancestors(child_id, Some(5)).unwrap();
    assert_eq!(ancestors.len(), 1);
}

#[test]
fn smoke_test_descendant_retrieval() {
    let mut tracker = LineageTracker::new();

    let root = PromptInfo::new(NodeId::new(), "Root prompt");
    let child1 = PromptInfo::new(NodeId::new(), "First child");
    let child2 = PromptInfo::new(NodeId::new(), "Second child");
    let root_id = root.id;
    let child1_id = child1.id;
    let child2_id = child2.id;

    tracker.register_prompt(root);
    tracker.register_prompt(child1);
    tracker.register_prompt(child2);
    tracker
        .track_evolution(root_id, child1_id, EvolutionType::Derives, Some(0.7))
        .unwrap();
    tracker
        .track_evolution(root_id, child2_id, EvolutionType::Derives, Some(0.6))
        .unwrap();

    let descendants = tracker.get_descendants(root_id, Some(5)).unwrap();
    assert_eq!(descendants.len(), 2);
}

#[test]
fn smoke_test_subgraph_extraction() {
    let mut tracker = LineageTracker::new();

    let p1 = PromptInfo::new(NodeId::new(), "Prompt 1");
    let p2 = PromptInfo::new(NodeId::new(), "Prompt 2");
    let p3 = PromptInfo::new(NodeId::new(), "Prompt 3");
    let p1_id = p1.id;
    let p2_id = p2.id;
    let p3_id = p3.id;

    tracker.register_prompt(p1);
    tracker.register_prompt(p2);
    tracker.register_prompt(p3);
    tracker
        .track_evolution(p1_id, p2_id, EvolutionType::Evolves, Some(0.8))
        .unwrap();
    tracker
        .track_evolution(p2_id, p3_id, EvolutionType::Refines, Some(0.9))
        .unwrap();

    let subgraph = tracker.get_lineage_subgraph(p2_id, Some(5), Some(5)).unwrap();
    assert_eq!(subgraph.node_count, 3);
    assert_eq!(subgraph.edge_count, 2);
}

#[test]
fn smoke_test_similarity_computation() {
    let computer = SimilarityComputer::new();

    let prompt_a = PromptInfo::new(NodeId::new(), "Explain machine learning to a beginner");
    let prompt_b =
        PromptInfo::new(NodeId::new(), "Explain machine learning to a complete beginner");

    let scores = computer.compute(&prompt_a, &prompt_b);

    assert!(scores.combined > 0.5, "Similar prompts should have high score");
    assert!(scores.token_overlap > 0.5);
}

#[test]
fn smoke_test_cycle_detection() {
    let mut tracker = LineageTracker::new();

    let p1 = PromptInfo::new(NodeId::new(), "Prompt 1");
    let p2 = PromptInfo::new(NodeId::new(), "Prompt 2");
    let p3 = PromptInfo::new(NodeId::new(), "Prompt 3");
    let p1_id = p1.id;
    let p2_id = p2.id;
    let p3_id = p3.id;

    tracker.register_prompt(p1);
    tracker.register_prompt(p2);
    tracker.register_prompt(p3);

    // Create chain: p1 -> p2 -> p3
    tracker
        .track_evolution(p1_id, p2_id, EvolutionType::Evolves, Some(0.8))
        .unwrap();
    tracker
        .track_evolution(p2_id, p3_id, EvolutionType::Evolves, Some(0.8))
        .unwrap();

    // Attempting to create p3 -> p1 should fail (cycle)
    let result = tracker.track_evolution(p3_id, p1_id, EvolutionType::Evolves, Some(0.8));
    assert!(result.is_err());
}

#[test]
fn smoke_test_evolution_type_suggestion() {
    let mut tracker = LineageTracker::new();

    let original = PromptInfo::new(NodeId::new(), "Write a function to calculate factorial");
    let refined = PromptInfo::new(
        NodeId::new(),
        "Write a recursive function to calculate factorial with memoization",
    );
    let original_id = original.id;
    let refined_id = refined.id;

    tracker.register_prompt(original);
    tracker.register_prompt(refined);

    let (suggested_type, confidence) = tracker
        .suggest_evolution_type(original_id, refined_id)
        .unwrap();

    // Should suggest some valid type with reasonable confidence
    assert!(matches!(
        suggested_type,
        EvolutionType::Evolves | EvolutionType::Refines | EvolutionType::Derives
    ));
    assert!(confidence > 0.0);
}

#[test]
fn smoke_test_find_similar() {
    let mut tracker = LineageTracker::new();

    let reference = PromptInfo::new(NodeId::new(), "Explain quantum computing basics");
    let similar = PromptInfo::new(NodeId::new(), "Describe quantum computing fundamentals");
    let different = PromptInfo::new(NodeId::new(), "How to make pancakes");
    let reference_id = reference.id;

    tracker.register_prompt(reference);
    tracker.register_prompt(similar);
    tracker.register_prompt(different);

    let results = tracker.find_similar(reference_id, 0.3, 10).unwrap();

    // Should find at least the similar prompt
    assert!(!results.is_empty());
}

#[test]
fn smoke_test_stats() {
    let mut tracker = LineageTracker::new();

    let p1 = PromptInfo::new(NodeId::new(), "Prompt 1");
    let p2 = PromptInfo::new(NodeId::new(), "Prompt 2");
    let p1_id = p1.id;
    let p2_id = p2.id;

    tracker.register_prompt(p1);
    tracker.register_prompt(p2);
    tracker
        .track_evolution(p1_id, p2_id, EvolutionType::Refines, Some(0.9))
        .unwrap();

    let stats = tracker.stats();

    assert_eq!(stats.total_prompts, 2);
    assert_eq!(stats.total_edges, 1);
    assert_eq!(stats.refines_count, 1);
    assert_eq!(stats.evolves_count, 0);
    assert_eq!(stats.derives_count, 0);
    assert!((stats.average_confidence - 0.9).abs() < 0.01);
}

// =============================================================================
// Smoke Tests - Error Cases
// =============================================================================

#[test]
fn smoke_test_invalid_node_error() {
    let tracker = LineageTracker::new();
    let nonexistent_id = NodeId::new();

    let result = tracker.get_ancestors(nonexistent_id, Some(5));
    assert!(result.is_err());
}

#[test]
fn smoke_test_invalid_confidence() {
    use prompt_lineage_agent::LineageEdge;

    let result = LineageEdge::new(NodeId::new(), NodeId::new(), EvolutionType::Evolves, 1.5);
    assert!(result.is_err());

    let result = LineageEdge::new(NodeId::new(), NodeId::new(), EvolutionType::Evolves, -0.1);
    assert!(result.is_err());
}

// =============================================================================
// Smoke Tests - Edge Cases
// =============================================================================

#[test]
fn smoke_test_empty_prompt() {
    let prompt = PromptInfo::new(NodeId::new(), "");
    assert!(prompt.tokens.is_empty());
}

#[test]
fn smoke_test_identical_prompts() {
    let computer = SimilarityComputer::new();

    let content = "This is a test prompt";
    let prompt_a = PromptInfo::new(NodeId::new(), content);
    let prompt_b = PromptInfo::new(NodeId::new(), content);

    let scores = computer.compute(&prompt_a, &prompt_b);

    assert_eq!(scores.token_overlap, 1.0);
    assert_eq!(scores.edit_distance_ratio, 1.0);
}

#[test]
fn smoke_test_completely_different_prompts() {
    let computer = SimilarityComputer::new();

    let prompt_a = PromptInfo::new(NodeId::new(), "Alpha beta gamma delta");
    let prompt_b = PromptInfo::new(NodeId::new(), "One two three four");

    let scores = computer.compute(&prompt_a, &prompt_b);

    assert!(scores.token_overlap < 0.1);
    assert!(scores.combined < 0.5);
}

// =============================================================================
// Smoke Tests - CLI Structures
// =============================================================================

#[test]
fn smoke_test_cli_output_format() {
    use prompt_lineage_agent::cli::OutputFormat;

    assert_eq!(OutputFormat::Text.to_string(), "text");
    assert_eq!(OutputFormat::Json.to_string(), "json");
    assert_eq!(OutputFormat::Yaml.to_string(), "yaml");
    assert_eq!(OutputFormat::Table.to_string(), "table");
}

#[test]
fn smoke_test_cli_result_serialization() {
    use prompt_lineage_agent::cli::{InspectResult, LineageStats};
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    let result = InspectResult {
        prompt_id: Uuid::new_v4(),
        content_preview: "Test content".to_string(),
        created_at: Utc::now(),
        ancestors: vec![],
        children: vec![],
        confidence_scores: vec![],
        related: None,
        stats: LineageStats {
            ancestor_count: 0,
            descendant_count: 0,
            depth_to_root: 0,
            avg_confidence: 0.0,
            evolution_types: HashMap::new(),
        },
    };

    let json = serde_json::to_string(&result);
    assert!(json.is_ok());
}

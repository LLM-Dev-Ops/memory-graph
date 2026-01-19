//! Integration tests for Decision Memory Agent
//!
//! These tests verify the agent contract compliance:
//! - Input validation
//! - Graph node/edge creation
//! - DecisionEvent emission
//! - CLI endpoint functionality
//! - Deterministic output

use chrono::Utc;
use decision_memory_agent::{
    agent::{AgentConfig, DecisionMemoryAgent},
    contracts::{
        DecisionConstraint, DecisionContext, DecisionEdgeType, DecisionMemoryInput,
        DecisionNodeType, DecisionOutcome, DecisionResultType, ReasoningArtifact,
        ReasoningArtifactType,
    },
    error::{AgentError, AgentErrorCode},
    ruvector::{DecisionEventQuery, RetrieveResponse, RuVectorService, StoreResponse},
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Mock RuVector service for testing
struct MockRuVectorService {
    events: Mutex<Vec<decision_memory_agent::contracts::DecisionEvent>>,
    should_fail: Mutex<bool>,
}

impl MockRuVectorService {
    fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            should_fail: Mutex::new(false),
        }
    }

    fn set_should_fail(&self, fail: bool) {
        *self.should_fail.lock().unwrap() = fail;
    }

    fn get_events(&self) -> Vec<decision_memory_agent::contracts::DecisionEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl RuVectorService for MockRuVectorService {
    async fn store_decision_event(
        &self,
        event: &decision_memory_agent::contracts::DecisionEvent,
    ) -> decision_memory_agent::error::AgentResult<StoreResponse> {
        if *self.should_fail.lock().unwrap() {
            return Err(AgentError::ruvector_write("Mock failure"));
        }

        self.events.lock().unwrap().push(event.clone());
        Ok(StoreResponse {
            ref_id: event.execution_ref.to_string(),
            success: true,
            location: Some("mock://test".to_string()),
        })
    }

    async fn retrieve_decision_event(
        &self,
        execution_ref: &Uuid,
    ) -> decision_memory_agent::error::AgentResult<
        RetrieveResponse<decision_memory_agent::contracts::DecisionEvent>,
    > {
        let events = self.events.lock().unwrap();
        for event in events.iter() {
            if event.execution_ref == *execution_ref {
                return Ok(RetrieveResponse {
                    data: event.clone(),
                    ref_id: event.execution_ref.to_string(),
                    retrieved_at: Utc::now().to_rfc3339(),
                });
            }
        }
        Err(AgentError::new(
            AgentErrorCode::DecisionNotFound,
            "Not found",
        ))
    }

    async fn query_decision_events(
        &self,
        query: &DecisionEventQuery,
    ) -> decision_memory_agent::error::AgentResult<
        Vec<decision_memory_agent::contracts::DecisionEvent>,
    > {
        let events = self.events.lock().unwrap();
        let mut results = Vec::new();

        for event in events.iter() {
            let mut matches = true;

            if let Some(session_id) = query.session_id {
                if event.input.context.session_id != session_id {
                    matches = false;
                }
            }

            if let Some(decision_id) = query.decision_id {
                if event.input.decision_id != decision_id {
                    matches = false;
                }
            }

            if matches {
                results.push(event.clone());
            }
        }

        if let Some(limit) = query.limit {
            results.truncate(limit as usize);
        }

        Ok(results)
    }

    async fn store_artifact_content(
        &self,
        artifact_id: &Uuid,
        _content: &[u8],
    ) -> decision_memory_agent::error::AgentResult<StoreResponse> {
        Ok(StoreResponse {
            ref_id: artifact_id.to_string(),
            success: true,
            location: Some("mock://artifacts".to_string()),
        })
    }

    async fn retrieve_artifact_content(
        &self,
        _content_ref: &str,
    ) -> decision_memory_agent::error::AgentResult<Vec<u8>> {
        Ok(vec![])
    }
}

fn create_test_input() -> DecisionMemoryInput {
    DecisionMemoryInput {
        decision_id: Uuid::new_v4(),
        decision_type: "test_decision".to_string(),
        context: DecisionContext {
            session_id: Uuid::new_v4(),
            agent_id: Some("test-agent".to_string()),
            predecessor_decision_id: None,
            conversation_turn: Some(1),
            model_id: Some("gpt-4".to_string()),
            temperature: Some(0.7),
            user_id: None,
            environment: None,
        },
        reasoning_artifacts: vec![],
        outcome: None,
        tags: vec!["test".to_string()],
    }
}

// =============================================================================
// CONTRACT COMPLIANCE TESTS
// =============================================================================

#[tokio::test]
async fn test_agent_id_is_correct() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    let result = agent.capture(input).await.unwrap();

    assert_eq!(result.agent_id, "decision-memory-agent");
}

#[tokio::test]
async fn test_agent_version_is_semver() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    let result = agent.capture(input).await.unwrap();

    // Verify semver format
    let version_parts: Vec<&str> = result.agent_version.split('.').collect();
    assert_eq!(version_parts.len(), 3);
    for part in version_parts {
        assert!(part.parse::<u32>().is_ok());
    }
}

#[tokio::test]
async fn test_decision_type_is_decision_memory_capture() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    let result = agent.capture(input).await.unwrap();

    assert_eq!(result.decision_type, "decision_memory_capture");
}

#[tokio::test]
async fn test_inputs_hash_is_sha256() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    let result = agent.capture(input).await.unwrap();

    // SHA-256 produces 64 hex characters
    assert_eq!(result.inputs_hash.len(), 64);
    assert!(result.inputs_hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn test_execution_ref_is_uuid() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    let result = agent.capture(input).await.unwrap();

    // execution_ref should be a valid UUID
    assert!(!result.execution_ref.is_nil());
}

#[tokio::test]
async fn test_timestamp_is_utc() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let before = Utc::now();
    let input = create_test_input();
    let result = agent.capture(input).await.unwrap();
    let after = Utc::now();

    assert!(result.timestamp >= before);
    assert!(result.timestamp <= after);
}

#[tokio::test]
async fn test_confidence_is_bounded() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    let result = agent.capture(input).await.unwrap();

    assert!(result.confidence >= 0.0);
    assert!(result.confidence <= 1.0);
}

// =============================================================================
// GRAPH NODE/EDGE TESTS
// =============================================================================

#[tokio::test]
async fn test_decision_node_created() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    let decision_id = input.decision_id;
    let result = agent.capture(input).await.unwrap();

    let decision_nodes: Vec<_> = result
        .outputs
        .nodes_created
        .iter()
        .filter(|n| n.node_type == DecisionNodeType::Decision)
        .collect();

    assert_eq!(decision_nodes.len(), 1);
    assert_eq!(decision_nodes[0].node_id, decision_id);
}

#[tokio::test]
async fn test_session_node_created() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    let session_id = input.context.session_id;
    let result = agent.capture(input).await.unwrap();

    let session_nodes: Vec<_> = result
        .outputs
        .nodes_created
        .iter()
        .filter(|n| n.node_type == DecisionNodeType::Session)
        .collect();

    assert_eq!(session_nodes.len(), 1);
    assert_eq!(session_nodes[0].node_id, session_id);
}

#[tokio::test]
async fn test_outcome_node_created_when_present() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let mut input = create_test_input();
    let outcome_id = Uuid::new_v4();
    input.outcome = Some(DecisionOutcome {
        outcome_id,
        decision_ref: input.decision_id,
        result_type: DecisionResultType::Success,
        result_data: None,
        metrics: None,
        recorded_at: Utc::now(),
    });

    let result = agent.capture(input).await.unwrap();

    let outcome_nodes: Vec<_> = result
        .outputs
        .nodes_created
        .iter()
        .filter(|n| n.node_type == DecisionNodeType::Outcome)
        .collect();

    assert_eq!(outcome_nodes.len(), 1);
    assert_eq!(outcome_nodes[0].node_id, outcome_id);
}

#[tokio::test]
async fn test_artifact_nodes_created() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let mut input = create_test_input();
    let artifact_id = Uuid::new_v4();
    input.reasoning_artifacts = vec![ReasoningArtifact {
        artifact_id,
        artifact_type: ReasoningArtifactType::ChainOfThought,
        content_hash: "a".repeat(64),
        content_ref: Some("ref-1".to_string()),
        parent_artifact_id: None,
        created_at: Utc::now(),
        metadata: Default::default(),
    }];

    let result = agent.capture(input).await.unwrap();

    let artifact_nodes: Vec<_> = result
        .outputs
        .nodes_created
        .iter()
        .filter(|n| n.node_type == DecisionNodeType::Artifact)
        .collect();

    assert_eq!(artifact_nodes.len(), 1);
    assert_eq!(artifact_nodes[0].node_id, artifact_id);
}

#[tokio::test]
async fn test_part_of_edge_created() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    let decision_id = input.decision_id;
    let session_id = input.context.session_id;
    let result = agent.capture(input).await.unwrap();

    let part_of_edges: Vec<_> = result
        .outputs
        .edges_created
        .iter()
        .filter(|e| e.edge_type == DecisionEdgeType::PartOf)
        .collect();

    assert_eq!(part_of_edges.len(), 1);
    assert_eq!(part_of_edges[0].from_node_id, decision_id);
    assert_eq!(part_of_edges[0].to_node_id, session_id);
}

#[tokio::test]
async fn test_has_outcome_edge_created() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let mut input = create_test_input();
    let outcome_id = Uuid::new_v4();
    input.outcome = Some(DecisionOutcome {
        outcome_id,
        decision_ref: input.decision_id,
        result_type: DecisionResultType::Success,
        result_data: None,
        metrics: None,
        recorded_at: Utc::now(),
    });

    let decision_id = input.decision_id;
    let result = agent.capture(input).await.unwrap();

    let has_outcome_edges: Vec<_> = result
        .outputs
        .edges_created
        .iter()
        .filter(|e| e.edge_type == DecisionEdgeType::HasOutcome)
        .collect();

    assert_eq!(has_outcome_edges.len(), 1);
    assert_eq!(has_outcome_edges[0].from_node_id, decision_id);
    assert_eq!(has_outcome_edges[0].to_node_id, outcome_id);
}

#[tokio::test]
async fn test_follows_edge_created_for_chain() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let mut input = create_test_input();
    let predecessor_id = Uuid::new_v4();
    input.context.predecessor_decision_id = Some(predecessor_id);

    let decision_id = input.decision_id;
    let result = agent.capture(input).await.unwrap();

    let follows_edges: Vec<_> = result
        .outputs
        .edges_created
        .iter()
        .filter(|e| e.edge_type == DecisionEdgeType::Follows)
        .collect();

    assert_eq!(follows_edges.len(), 1);
    assert_eq!(follows_edges[0].from_node_id, decision_id);
    assert_eq!(follows_edges[0].to_node_id, predecessor_id);
}

// =============================================================================
// DECISION EVENT PERSISTENCE TESTS
// =============================================================================

#[tokio::test]
async fn test_exactly_one_decision_event_emitted() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    agent.capture(input).await.unwrap();

    let events = mock.get_events();
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn test_decision_event_can_be_retrieved() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    let result = agent.capture(input).await.unwrap();

    let retrieved = mock
        .retrieve_decision_event(&result.execution_ref)
        .await
        .unwrap();

    assert_eq!(retrieved.data.execution_ref, result.execution_ref);
}

#[tokio::test]
async fn test_decision_events_can_be_queried() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    let session_id = input.context.session_id;
    agent.capture(input).await.unwrap();

    let query = DecisionEventQuery {
        session_id: Some(session_id),
        ..Default::default()
    };

    let results = mock.query_decision_events(&query).await.unwrap();
    assert_eq!(results.len(), 1);
}

// =============================================================================
// VALIDATION TESTS
// =============================================================================

#[tokio::test]
async fn test_invalid_artifact_hash_rejected() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let mut input = create_test_input();
    input.reasoning_artifacts = vec![ReasoningArtifact {
        artifact_id: Uuid::new_v4(),
        artifact_type: ReasoningArtifactType::ChainOfThought,
        content_hash: "invalid".to_string(), // Invalid: not 64 chars
        content_ref: None,
        parent_artifact_id: None,
        created_at: Utc::now(),
        metadata: Default::default(),
    }];

    let result = agent.capture(input).await;
    assert!(result.is_err());
}

// =============================================================================
// FAILURE MODE TESTS
// =============================================================================

#[tokio::test]
async fn test_ruvector_failure_returns_error() {
    let mock = Arc::new(MockRuVectorService::new());
    mock.set_should_fail(true);

    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    let result = agent.capture(input).await;

    assert!(result.is_err());
}

// =============================================================================
// CONFIDENCE CALCULATION TESTS
// =============================================================================

#[tokio::test]
async fn test_confidence_increases_with_outcome() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    // Without outcome
    let input_without = create_test_input();
    let result_without = agent.capture(input_without).await.unwrap();

    // With outcome
    let mut input_with = create_test_input();
    input_with.outcome = Some(DecisionOutcome {
        outcome_id: Uuid::new_v4(),
        decision_ref: input_with.decision_id,
        result_type: DecisionResultType::Success,
        result_data: None,
        metrics: None,
        recorded_at: Utc::now(),
    });
    let result_with = agent.capture(input_with).await.unwrap();

    assert!(result_with.confidence > result_without.confidence);
}

#[tokio::test]
async fn test_confidence_increases_with_artifacts() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    // Without artifacts
    let input_without = create_test_input();
    let result_without = agent.capture(input_without).await.unwrap();

    // With artifacts
    let mut input_with = create_test_input();
    input_with.reasoning_artifacts = (0..5)
        .map(|i| ReasoningArtifact {
            artifact_id: Uuid::new_v4(),
            artifact_type: ReasoningArtifactType::ContextSnapshot,
            content_hash: format!("{:0>64}", i),
            content_ref: Some(format!("ref-{}", i)),
            parent_artifact_id: None,
            created_at: Utc::now(),
            metadata: Default::default(),
        })
        .collect();
    let result_with = agent.capture(input_with).await.unwrap();

    assert!(result_with.confidence > result_without.confidence);
}

// =============================================================================
// CONSTRAINTS TESTS
// =============================================================================

#[tokio::test]
async fn test_session_boundary_constraint_always_applied() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    let result = agent.capture(input).await.unwrap();

    assert!(result
        .constraints_applied
        .contains(&DecisionConstraint::SessionBoundary));
}

#[tokio::test]
async fn test_retention_policy_constraint_always_applied() {
    let mock = Arc::new(MockRuVectorService::new());
    let agent = DecisionMemoryAgent::new(mock.clone(), AgentConfig::default());

    let input = create_test_input();
    let result = agent.capture(input).await.unwrap();

    assert!(result
        .constraints_applied
        .contains(&DecisionConstraint::RetentionPolicy));
}

// =============================================================================
// DETERMINISM TESTS
// =============================================================================

#[tokio::test]
async fn test_same_input_produces_same_hash() {
    let mock1 = Arc::new(MockRuVectorService::new());
    let mock2 = Arc::new(MockRuVectorService::new());
    let agent1 = DecisionMemoryAgent::new(mock1.clone(), AgentConfig::default());
    let agent2 = DecisionMemoryAgent::new(mock2.clone(), AgentConfig::default());

    let decision_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();

    let input1 = DecisionMemoryInput {
        decision_id,
        decision_type: "test".to_string(),
        context: DecisionContext {
            session_id,
            agent_id: None,
            predecessor_decision_id: None,
            conversation_turn: None,
            model_id: None,
            temperature: None,
            user_id: None,
            environment: None,
        },
        reasoning_artifacts: vec![],
        outcome: None,
        tags: vec![],
    };

    let input2 = input1.clone();

    let result1 = agent1.capture(input1).await.unwrap();
    let result2 = agent2.capture(input2).await.unwrap();

    assert_eq!(result1.inputs_hash, result2.inputs_hash);
}

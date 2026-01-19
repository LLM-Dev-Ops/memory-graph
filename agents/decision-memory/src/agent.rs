//! Decision Memory Agent Implementation
//!
//! Core agent logic for capturing decisions, outcomes, and reasoning artifacts.
//!
//! # Classification
//! - **Type**: MEMORY_WRITE
//! - **decision_type**: decision_memory_capture
//!
//! # What This Agent Does
//! - Captures memory events
//! - Creates or updates graph nodes
//! - Creates or updates graph edges
//! - Stores artifact content to ruvector-service
//!
//! # What This Agent MUST NOT Do
//! - Modify system behavior
//! - Trigger remediation
//! - Trigger retries
//! - Emit alerts
//! - Enforce policies
//! - Perform orchestration
//! - Connect directly to Google SQL
//! - Execute SQL

use crate::contracts::{
    DecisionConstraint, DecisionContext, DecisionEdgeType, DecisionEvent, DecisionEventTelemetry,
    DecisionMemoryInput, DecisionMemoryOutput, DecisionNodeType, GraphEdgeCreated, GraphNodeCreated,
    ReasoningArtifact,
};
use crate::error::{AgentError, AgentResult};
use crate::ruvector::{RuVectorClient, RuVectorService, StoreResponse};
use crate::telemetry::TelemetryCollector;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;
use validator::Validate;

/// Maximum number of artifacts per decision
const MAX_ARTIFACTS: usize = 100;

/// Maximum content size for artifacts (1MB)
const MAX_ARTIFACT_CONTENT_SIZE: usize = 1024 * 1024;

/// Configuration for the Decision Memory Agent
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Whether to validate inputs strictly
    pub strict_validation: bool,
    /// Whether to compute embeddings for artifacts
    pub compute_embeddings: bool,
    /// Whether to apply PII redaction
    pub apply_pii_redaction: bool,
    /// Maximum artifacts per decision
    pub max_artifacts: usize,
    /// Maximum artifact content size
    pub max_artifact_content_size: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            strict_validation: true,
            compute_embeddings: false,
            apply_pii_redaction: false,
            max_artifacts: MAX_ARTIFACTS,
            max_artifact_content_size: MAX_ARTIFACT_CONTENT_SIZE,
        }
    }
}

/// Decision Memory Agent
///
/// Persists decisions, outcomes, and reasoning artifacts for audit and learning.
pub struct DecisionMemoryAgent {
    ruvector: Arc<dyn RuVectorService>,
    config: AgentConfig,
}

impl DecisionMemoryAgent {
    /// Create a new Decision Memory Agent
    pub fn new(ruvector: Arc<dyn RuVectorService>, config: AgentConfig) -> Self {
        Self { ruvector, config }
    }

    /// Create an agent from environment configuration
    pub fn from_env() -> AgentResult<Self> {
        let ruvector = RuVectorClient::from_env().map_err(|e| {
            AgentError::internal(format!("Failed to create RuVector client: {}", e))
        })?;

        Ok(Self::new(Arc::new(ruvector), AgentConfig::default()))
    }

    /// Process a decision memory capture request
    ///
    /// This is the main entry point for the agent. It:
    /// 1. Validates the input
    /// 2. Creates graph nodes for the decision and its components
    /// 3. Creates graph edges to establish relationships
    /// 4. Stores artifact content to ruvector-service
    /// 5. Emits exactly ONE DecisionEvent to ruvector-service
    #[instrument(skip(self, input), fields(decision_id = %input.decision_id))]
    pub async fn capture(&self, input: DecisionMemoryInput) -> AgentResult<DecisionEvent> {
        let execution_ref = Uuid::new_v4();
        let mut telemetry = TelemetryCollector::new(execution_ref);

        info!("Starting decision memory capture");

        // Step 1: Validate input
        self.validate_input(&input)?;

        // Step 2: Compute input hash for determinism
        let inputs_hash = self.compute_input_hash(&input)?;

        // Step 3: Determine constraints to apply
        let constraints = self.determine_constraints(&input);

        // Step 4: Create graph nodes
        let nodes_created = self.create_graph_nodes(&input).await?;

        // Step 5: Create graph edges
        let edges_created = self.create_graph_edges(&input, &nodes_created).await?;

        // Step 6: Store artifact content
        let ruvector_refs = self
            .store_artifacts(&input.reasoning_artifacts, &mut telemetry)
            .await?;

        // Step 7: Calculate confidence
        let confidence = self.calculate_confidence(&input, &nodes_created, &edges_created);

        // Record decision capture in telemetry
        telemetry.record_decision_capture(
            input.decision_id,
            input.context.session_id,
            confidence,
        );

        // Step 8: Build output
        let output = DecisionMemoryOutput {
            decision_id: input.decision_id,
            nodes_created,
            edges_created,
            artifacts_stored: ruvector_refs.len(),
            capture_timestamp: Utc::now(),
            ruvector_refs,
        };

        // Step 9: Build and emit DecisionEvent
        let telemetry_data = telemetry.complete_success(
            output.nodes_created.len(),
            output.edges_created.len(),
            output.artifacts_stored,
        );

        let event = DecisionEvent::builder()
            .input(input)
            .outputs(output)
            .confidence(confidence)
            .telemetry(telemetry_data)
            .build(inputs_hash)
            .map_err(|e| AgentError::internal(format!("Failed to build event: {}", e)))?;

        // Add constraints
        let mut event = event;
        event.constraints_applied = constraints;
        event.execution_ref = execution_ref;

        // Step 10: Persist to ruvector-service (EXACTLY ONE DecisionEvent)
        let start = Instant::now();
        let store_result = self.ruvector.store_decision_event(&event).await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        debug!(
            ref_id = %store_result.ref_id,
            latency_ms = %latency_ms,
            "DecisionEvent persisted to ruvector-service"
        );

        info!(
            execution_ref = %execution_ref,
            decision_id = %event.outputs.decision_id,
            nodes_created = %event.outputs.nodes_created.len(),
            edges_created = %event.outputs.edges_created.len(),
            artifacts_stored = %event.outputs.artifacts_stored,
            "Decision memory capture completed"
        );

        Ok(event)
    }

    /// Validate the input against contracts
    fn validate_input(&self, input: &DecisionMemoryInput) -> AgentResult<()> {
        // Validate using validator crate
        if let Err(errors) = input.validate() {
            return Err(AgentError::validation(format!(
                "Input validation failed: {}",
                errors
            )));
        }

        // Additional validations
        if input.reasoning_artifacts.len() > self.config.max_artifacts {
            return Err(AgentError::validation(format!(
                "Too many artifacts: {} > {}",
                input.reasoning_artifacts.len(),
                self.config.max_artifacts
            )));
        }

        // Validate artifact content hashes
        for artifact in &input.reasoning_artifacts {
            if artifact.content_hash.len() != 64 {
                return Err(AgentError::validation(format!(
                    "Invalid content hash length for artifact {}: expected 64, got {}",
                    artifact.artifact_id,
                    artifact.content_hash.len()
                )));
            }
        }

        Ok(())
    }

    /// Compute SHA-256 hash of the input for determinism
    fn compute_input_hash(&self, input: &DecisionMemoryInput) -> AgentResult<String> {
        let json = serde_json::to_string(input)
            .map_err(|e| AgentError::internal(format!("Failed to serialize input: {}", e)))?;

        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        let result = hasher.finalize();

        Ok(hex::encode(result))
    }

    /// Determine which constraints to apply
    fn determine_constraints(&self, input: &DecisionMemoryInput) -> Vec<DecisionConstraint> {
        let mut constraints = vec![DecisionConstraint::SessionBoundary];

        if input.reasoning_artifacts.len() > self.config.max_artifacts / 2 {
            warn!(
                artifacts = input.reasoning_artifacts.len(),
                max = self.config.max_artifacts,
                "Approaching artifact limit"
            );
            constraints.push(DecisionConstraint::MaxArtifacts);
        }

        if self.config.apply_pii_redaction {
            constraints.push(DecisionConstraint::PiiRedaction);
        }

        constraints.push(DecisionConstraint::RetentionPolicy);

        constraints
    }

    /// Create graph nodes for the decision and its components
    async fn create_graph_nodes(
        &self,
        input: &DecisionMemoryInput,
    ) -> AgentResult<Vec<GraphNodeCreated>> {
        let mut nodes = Vec::new();

        // Decision node
        nodes.push(GraphNodeCreated {
            node_id: input.decision_id,
            node_type: DecisionNodeType::Decision,
        });

        // Session node (if this is a new session)
        nodes.push(GraphNodeCreated {
            node_id: input.context.session_id,
            node_type: DecisionNodeType::Session,
        });

        // Outcome node (if present)
        if let Some(ref outcome) = input.outcome {
            nodes.push(GraphNodeCreated {
                node_id: outcome.outcome_id,
                node_type: DecisionNodeType::Outcome,
            });
        }

        // Artifact nodes
        for artifact in &input.reasoning_artifacts {
            nodes.push(GraphNodeCreated {
                node_id: artifact.artifact_id,
                node_type: DecisionNodeType::Artifact,
            });
        }

        debug!(
            node_count = nodes.len(),
            "Created graph nodes"
        );

        Ok(nodes)
    }

    /// Create graph edges to establish relationships
    async fn create_graph_edges(
        &self,
        input: &DecisionMemoryInput,
        _nodes: &[GraphNodeCreated],
    ) -> AgentResult<Vec<GraphEdgeCreated>> {
        let mut edges = Vec::new();

        // Decision -> Session (part_of)
        edges.push(GraphEdgeCreated {
            edge_id: Uuid::new_v4(),
            edge_type: DecisionEdgeType::PartOf,
            from_node_id: input.decision_id,
            to_node_id: input.context.session_id,
        });

        // Decision -> Outcome (has_outcome)
        if let Some(ref outcome) = input.outcome {
            edges.push(GraphEdgeCreated {
                edge_id: Uuid::new_v4(),
                edge_type: DecisionEdgeType::HasOutcome,
                from_node_id: input.decision_id,
                to_node_id: outcome.outcome_id,
            });
        }

        // Decision -> Artifacts (has_artifact)
        for artifact in &input.reasoning_artifacts {
            edges.push(GraphEdgeCreated {
                edge_id: Uuid::new_v4(),
                edge_type: DecisionEdgeType::HasArtifact,
                from_node_id: input.decision_id,
                to_node_id: artifact.artifact_id,
            });

            // Artifact lineage (derived_from)
            if let Some(parent_id) = artifact.parent_artifact_id {
                edges.push(GraphEdgeCreated {
                    edge_id: Uuid::new_v4(),
                    edge_type: DecisionEdgeType::DerivedFrom,
                    from_node_id: artifact.artifact_id,
                    to_node_id: parent_id,
                });
            }
        }

        // Decision chain (follows)
        if let Some(predecessor_id) = input.context.predecessor_decision_id {
            edges.push(GraphEdgeCreated {
                edge_id: Uuid::new_v4(),
                edge_type: DecisionEdgeType::Follows,
                from_node_id: input.decision_id,
                to_node_id: predecessor_id,
            });
        }

        debug!(
            edge_count = edges.len(),
            "Created graph edges"
        );

        Ok(edges)
    }

    /// Store artifact content to ruvector-service
    async fn store_artifacts(
        &self,
        artifacts: &[ReasoningArtifact],
        telemetry: &mut TelemetryCollector,
    ) -> AgentResult<Vec<String>> {
        let mut refs = Vec::new();

        for artifact in artifacts {
            // Only store if content_ref is provided (artifact content is external)
            if artifact.content_ref.is_some() {
                // Content is already stored, just record the reference
                refs.push(artifact.content_ref.clone().unwrap());

                telemetry.record_artifact_stored(
                    artifact.artifact_id,
                    &format!("{:?}", artifact.artifact_type),
                    0, // Content size unknown for pre-stored artifacts
                );
            }
        }

        debug!(
            artifact_count = refs.len(),
            "Processed artifacts"
        );

        Ok(refs)
    }

    /// Calculate confidence score for the decision capture
    ///
    /// Confidence represents the association strength between the decision
    /// and its artifacts/outcomes. Factors:
    /// - Presence of outcome (adds confidence)
    /// - Number of artifacts (more context = higher confidence)
    /// - Lineage information (predecessor decisions add context)
    /// - Completeness of context metadata
    fn calculate_confidence(
        &self,
        input: &DecisionMemoryInput,
        nodes: &[GraphNodeCreated],
        edges: &[GraphEdgeCreated],
    ) -> f64 {
        let mut confidence = 0.5; // Base confidence

        // Outcome presence
        if input.outcome.is_some() {
            confidence += 0.15;
        }

        // Artifacts (up to +0.2 for having artifacts)
        let artifact_bonus = (input.reasoning_artifacts.len() as f64 / 10.0).min(0.2);
        confidence += artifact_bonus;

        // Lineage information
        if input.context.predecessor_decision_id.is_some() {
            confidence += 0.05;
        }

        // Context completeness
        if input.context.model_id.is_some() {
            confidence += 0.03;
        }
        if input.context.agent_id.is_some() {
            confidence += 0.02;
        }

        // Graph completeness
        if !nodes.is_empty() && !edges.is_empty() {
            confidence += 0.05;
        }

        // Cap at 1.0
        confidence.min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{DecisionOutcome, DecisionResultType, ReasoningArtifactType};
    use crate::ruvector::{DecisionEventQuery, RetrieveResponse};
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// Mock RuVector service for testing
    struct MockRuVectorService {
        stored_events: Mutex<Vec<DecisionEvent>>,
    }

    impl MockRuVectorService {
        fn new() -> Self {
            Self {
                stored_events: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl RuVectorService for MockRuVectorService {
        async fn store_decision_event(&self, event: &DecisionEvent) -> AgentResult<StoreResponse> {
            self.stored_events.lock().unwrap().push(event.clone());
            Ok(StoreResponse {
                ref_id: event.execution_ref.to_string(),
                success: true,
                location: Some("mock://storage".to_string()),
            })
        }

        async fn retrieve_decision_event(
            &self,
            execution_ref: &Uuid,
        ) -> AgentResult<RetrieveResponse<DecisionEvent>> {
            let events = self.stored_events.lock().unwrap();
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
                crate::error::AgentErrorCode::DecisionNotFound,
                "Event not found",
            ))
        }

        async fn query_decision_events(
            &self,
            _query: &DecisionEventQuery,
        ) -> AgentResult<Vec<DecisionEvent>> {
            Ok(self.stored_events.lock().unwrap().clone())
        }

        async fn store_artifact_content(
            &self,
            artifact_id: &Uuid,
            _content: &[u8],
        ) -> AgentResult<StoreResponse> {
            Ok(StoreResponse {
                ref_id: artifact_id.to_string(),
                success: true,
                location: Some("mock://artifacts".to_string()),
            })
        }

        async fn retrieve_artifact_content(&self, _content_ref: &str) -> AgentResult<Vec<u8>> {
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

    #[tokio::test]
    async fn test_capture_basic_decision() {
        let mock_ruvector = Arc::new(MockRuVectorService::new());
        let agent = DecisionMemoryAgent::new(mock_ruvector.clone(), AgentConfig::default());

        let input = create_test_input();
        let result = agent.capture(input.clone()).await;

        assert!(result.is_ok());
        let event = result.unwrap();
        assert_eq!(event.agent_id, "decision-memory-agent");
        assert_eq!(event.decision_type, "decision_memory_capture");
        assert_eq!(event.outputs.decision_id, input.decision_id);
        assert!(!event.outputs.nodes_created.is_empty());
        assert!(!event.outputs.edges_created.is_empty());
    }

    #[tokio::test]
    async fn test_capture_with_outcome() {
        let mock_ruvector = Arc::new(MockRuVectorService::new());
        let agent = DecisionMemoryAgent::new(mock_ruvector.clone(), AgentConfig::default());

        let mut input = create_test_input();
        input.outcome = Some(DecisionOutcome {
            outcome_id: Uuid::new_v4(),
            decision_ref: input.decision_id,
            result_type: DecisionResultType::Success,
            result_data: None,
            metrics: None,
            recorded_at: Utc::now(),
        });

        let result = agent.capture(input).await;
        assert!(result.is_ok());

        let event = result.unwrap();
        // Higher confidence with outcome
        assert!(event.confidence > 0.6);
    }

    #[tokio::test]
    async fn test_capture_with_artifacts() {
        let mock_ruvector = Arc::new(MockRuVectorService::new());
        let agent = DecisionMemoryAgent::new(mock_ruvector.clone(), AgentConfig::default());

        let mut input = create_test_input();
        input.reasoning_artifacts = vec![
            ReasoningArtifact {
                artifact_id: Uuid::new_v4(),
                artifact_type: ReasoningArtifactType::ChainOfThought,
                content_hash: "a".repeat(64),
                content_ref: Some("ref-1".to_string()),
                parent_artifact_id: None,
                created_at: Utc::now(),
                metadata: Default::default(),
            },
            ReasoningArtifact {
                artifact_id: Uuid::new_v4(),
                artifact_type: ReasoningArtifactType::PromptTemplate,
                content_hash: "b".repeat(64),
                content_ref: Some("ref-2".to_string()),
                parent_artifact_id: None,
                created_at: Utc::now(),
                metadata: Default::default(),
            },
        ];

        let result = agent.capture(input).await;
        assert!(result.is_ok());

        let event = result.unwrap();
        assert_eq!(event.outputs.artifacts_stored, 2);
    }

    #[tokio::test]
    async fn test_validation_error() {
        let mock_ruvector = Arc::new(MockRuVectorService::new());
        let agent = DecisionMemoryAgent::new(mock_ruvector.clone(), AgentConfig::default());

        let mut input = create_test_input();
        input.reasoning_artifacts = vec![ReasoningArtifact {
            artifact_id: Uuid::new_v4(),
            artifact_type: ReasoningArtifactType::ChainOfThought,
            content_hash: "invalid".to_string(), // Invalid hash length
            content_ref: None,
            parent_artifact_id: None,
            created_at: Utc::now(),
            metadata: Default::default(),
        }];

        let result = agent.capture(input).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_confidence_calculation() {
        let mock_ruvector = Arc::new(MockRuVectorService::new());
        let agent = DecisionMemoryAgent::new(mock_ruvector, AgentConfig::default());

        let input = create_test_input();
        let nodes = vec![GraphNodeCreated {
            node_id: Uuid::new_v4(),
            node_type: DecisionNodeType::Decision,
        }];
        let edges = vec![GraphEdgeCreated {
            edge_id: Uuid::new_v4(),
            edge_type: DecisionEdgeType::PartOf,
            from_node_id: Uuid::new_v4(),
            to_node_id: Uuid::new_v4(),
        }];

        let confidence = agent.calculate_confidence(&input, &nodes, &edges);
        assert!(confidence >= 0.5);
        assert!(confidence <= 1.0);
    }
}

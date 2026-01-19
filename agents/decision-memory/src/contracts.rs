//! Contract types for Decision Memory Agent
//!
//! These types are derived from agentics-contracts and MUST be kept in sync
//! with the JSON schema definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

/// Decision result types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionResultType {
    /// Decision executed successfully
    Success,
    /// Decision failed
    Failure,
    /// Partial execution
    Partial,
    /// Execution deferred
    Deferred,
}

/// Artifact types for reasoning capture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningArtifactType {
    /// Prompt template used
    PromptTemplate,
    /// Chain of thought reasoning
    ChainOfThought,
    /// Evaluation criteria applied
    EvaluationCriteria,
    /// Constraints that were considered
    Constraints,
    /// Snapshot of context at decision time
    ContextSnapshot,
    /// Tool execution trace
    ToolTrace,
}

/// Graph node types for decision memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionNodeType {
    /// Decision node
    Decision,
    /// Outcome node
    Outcome,
    /// Artifact node
    Artifact,
    /// Session node
    Session,
}

/// Graph edge types for decision memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionEdgeType {
    /// Decision has an outcome
    HasOutcome,
    /// Decision has an artifact
    HasArtifact,
    /// Decision follows another decision
    Follows,
    /// Node is part of a session
    PartOf,
    /// Artifact derived from another
    DerivedFrom,
}

/// Constraints applied during decision capture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionConstraint {
    /// Session boundary constraint
    SessionBoundary,
    /// Maximum artifacts limit
    MaxArtifacts,
    /// Content size limit
    ContentSizeLimit,
    /// PII redaction applied
    PiiRedaction,
    /// Retention policy constraint
    RetentionPolicy,
}

/// Environment types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentType {
    /// Development environment
    Development,
    /// Staging environment
    Staging,
    /// Production environment
    Production,
}

/// Outcome metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutcomeMetrics {
    /// Execution latency in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Total tokens consumed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_consumed: Option<u64>,
    /// Number of retry attempts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<u32>,
    /// Estimated cost in USD
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_estimate_usd: Option<f64>,
}

/// Decision outcome
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DecisionOutcome {
    /// Unique identifier for this outcome
    pub outcome_id: Uuid,
    /// Reference to the decision this outcome relates to
    pub decision_ref: Uuid,
    /// Type of outcome result
    pub result_type: DecisionResultType,
    /// Outcome-specific result payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_data: Option<serde_json::Value>,
    /// Outcome metrics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<OutcomeMetrics>,
    /// When the outcome was recorded
    pub recorded_at: DateTime<Utc>,
}

/// Reasoning artifact
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ReasoningArtifact {
    /// Unique identifier for this artifact
    pub artifact_id: Uuid,
    /// Type of reasoning artifact
    pub artifact_type: ReasoningArtifactType,
    /// SHA-256 hash of the artifact content
    #[validate(length(equal = 64))]
    pub content_hash: String,
    /// Reference to artifact content in ruvector storage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_ref: Option<String>,
    /// Reference to parent artifact for lineage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_artifact_id: Option<Uuid>,
    /// When the artifact was created
    pub created_at: DateTime<Utc>,
    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Decision context
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DecisionContext {
    /// Session this decision belongs to
    pub session_id: Uuid,
    /// Agent that made the decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Previous decision in the chain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predecessor_decision_id: Option<Uuid>,
    /// Turn number in the conversation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_turn: Option<u32>,
    /// LLM model used for the decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    /// Temperature setting
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 0.0, max = 2.0))]
    pub temperature: Option<f64>,
    /// User context (anonymized/hashed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Deployment environment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<EnvironmentType>,
}

/// Decision memory input
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DecisionMemoryInput {
    /// Unique identifier for the decision being captured
    pub decision_id: Uuid,
    /// Type/category of decision
    #[validate(length(min = 1))]
    pub decision_type: String,
    /// Decision context
    #[validate]
    pub context: DecisionContext,
    /// Artifacts capturing the reasoning process
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasoning_artifacts: Vec<ReasoningArtifact>,
    /// Decision outcome
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<DecisionOutcome>,
    /// Tags for categorization and retrieval
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Graph node created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodeCreated {
    /// Node identifier
    pub node_id: Uuid,
    /// Node type
    pub node_type: DecisionNodeType,
}

/// Graph edge created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdgeCreated {
    /// Edge identifier
    pub edge_id: Uuid,
    /// Edge type
    pub edge_type: DecisionEdgeType,
    /// Source node
    pub from_node_id: Uuid,
    /// Target node
    pub to_node_id: Uuid,
}

/// Decision memory output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionMemoryOutput {
    /// Decision identifier
    pub decision_id: Uuid,
    /// Nodes created in the graph
    pub nodes_created: Vec<GraphNodeCreated>,
    /// Edges created in the graph
    pub edges_created: Vec<GraphEdgeCreated>,
    /// Number of artifacts stored
    pub artifacts_stored: usize,
    /// When the capture occurred
    pub capture_timestamp: DateTime<Utc>,
    /// References to ruvector-stored data
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ruvector_refs: Vec<String>,
}

/// Decision event telemetry
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionEventTelemetry {
    /// Processing duration in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Memory bytes used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,
    /// RuVector service latency in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ruvector_latency_ms: Option<u64>,
}

/// Decision event - the main output persisted to ruvector-service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEvent {
    /// Agent identifier (always "decision-memory-agent")
    pub agent_id: String,
    /// Agent version (semver)
    pub agent_version: String,
    /// Decision type (always "decision_memory_capture")
    pub decision_type: String,
    /// SHA-256 hash of the input payload
    pub inputs_hash: String,
    /// Original input
    pub input: DecisionMemoryInput,
    /// Output from processing
    pub outputs: DecisionMemoryOutput,
    /// Confidence score (association strength)
    pub confidence: f64,
    /// Constraints applied during processing
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints_applied: Vec<DecisionConstraint>,
    /// Unique execution reference
    pub execution_ref: Uuid,
    /// UTC timestamp
    pub timestamp: DateTime<Utc>,
    /// Telemetry data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telemetry: Option<DecisionEventTelemetry>,
}

impl DecisionEvent {
    /// Create a new DecisionEvent builder
    pub fn builder() -> DecisionEventBuilder {
        DecisionEventBuilder::default()
    }
}

/// Builder for DecisionEvent
#[derive(Debug, Default)]
pub struct DecisionEventBuilder {
    input: Option<DecisionMemoryInput>,
    outputs: Option<DecisionMemoryOutput>,
    confidence: Option<f64>,
    constraints_applied: Vec<DecisionConstraint>,
    telemetry: Option<DecisionEventTelemetry>,
}

impl DecisionEventBuilder {
    /// Set the input
    pub fn input(mut self, input: DecisionMemoryInput) -> Self {
        self.input = Some(input);
        self
    }

    /// Set the outputs
    pub fn outputs(mut self, outputs: DecisionMemoryOutput) -> Self {
        self.outputs = Some(outputs);
        self
    }

    /// Set confidence score
    pub fn confidence(mut self, confidence: f64) -> Self {
        self.confidence = Some(confidence);
        self
    }

    /// Add a constraint
    pub fn constraint(mut self, constraint: DecisionConstraint) -> Self {
        self.constraints_applied.push(constraint);
        self
    }

    /// Set telemetry
    pub fn telemetry(mut self, telemetry: DecisionEventTelemetry) -> Self {
        self.telemetry = Some(telemetry);
        self
    }

    /// Build the DecisionEvent
    pub fn build(self, inputs_hash: String) -> Result<DecisionEvent, &'static str> {
        let input = self.input.ok_or("input is required")?;
        let outputs = self.outputs.ok_or("outputs is required")?;
        let confidence = self.confidence.ok_or("confidence is required")?;

        Ok(DecisionEvent {
            agent_id: crate::constants::AGENT_ID.to_string(),
            agent_version: crate::constants::AGENT_VERSION.to_string(),
            decision_type: crate::constants::DECISION_TYPE.to_string(),
            inputs_hash,
            input,
            outputs,
            confidence,
            constraints_applied: self.constraints_applied,
            execution_ref: Uuid::new_v4(),
            timestamp: Utc::now(),
            telemetry: self.telemetry,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_result_type_serialization() {
        let success = DecisionResultType::Success;
        let json = serde_json::to_string(&success).unwrap();
        assert_eq!(json, "\"success\"");

        let deserialized: DecisionResultType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, DecisionResultType::Success);
    }

    #[test]
    fn test_decision_context_validation() {
        let context = DecisionContext {
            session_id: Uuid::new_v4(),
            agent_id: None,
            predecessor_decision_id: None,
            conversation_turn: Some(1),
            model_id: Some("gpt-4".to_string()),
            temperature: Some(0.7),
            user_id: None,
            environment: Some(EnvironmentType::Production),
        };

        assert!(context.validate().is_ok());
    }

    #[test]
    fn test_decision_event_builder() {
        let input = DecisionMemoryInput {
            decision_id: Uuid::new_v4(),
            decision_type: "test_decision".to_string(),
            context: DecisionContext {
                session_id: Uuid::new_v4(),
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

        let output = DecisionMemoryOutput {
            decision_id: input.decision_id,
            nodes_created: vec![],
            edges_created: vec![],
            artifacts_stored: 0,
            capture_timestamp: Utc::now(),
            ruvector_refs: vec![],
        };

        let event = DecisionEvent::builder()
            .input(input)
            .outputs(output)
            .confidence(0.95)
            .constraint(DecisionConstraint::SessionBoundary)
            .build("a".repeat(64))
            .unwrap();

        assert_eq!(event.agent_id, "decision-memory-agent");
        assert_eq!(event.decision_type, "decision_memory_capture");
        assert_eq!(event.confidence, 0.95);
        assert_eq!(event.constraints_applied.len(), 1);
    }
}

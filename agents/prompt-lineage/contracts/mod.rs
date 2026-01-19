//! Contract definitions for the Prompt Lineage Agent
//!
//! This module defines the schemas and contracts for tracking prompt evolution
//! and lineage across iterations and agents. It is designed to integrate with
//! the ruvector-service for decision event tracking.
//!
//! # Agent Classification
//! - **Type**: MEMORY WRITE
//! - **Decision Type**: `prompt_lineage_tracking`
//!
//! # Architecture
//!
//! ```text
//! +-----------------+     +------------------+     +-----------------+
//! |  LineageInput   |---->| Prompt Lineage   |---->|  LineageOutput  |
//! |                 |     |     Agent        |     |                 |
//! +-----------------+     +--------+---------+     +-----------------+
//!                                  |
//!                                  v
//!                         +------------------+
//!                         |  DecisionEvent   |
//!                         |  (ruvector)      |
//!                         +------------------+
//! ```
//!
//! # Integration with llm-memory-graph-types
//!
//! When the `integration` feature is enabled, this module uses types from
//! `llm_memory_graph_types`. Otherwise, it uses compatible standalone definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

// ============================================================================
// Standalone Type Definitions (Compatible with llm-memory-graph-types)
// ============================================================================
// These types mirror the upstream definitions and can be replaced with imports
// when the `integration` feature is enabled.

/// Unique identifier for a node in the graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(uuid::Uuid);

impl NodeId {
    /// Create a new random node ID
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Create a node ID from a UUID
    #[must_use]
    pub const fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID
    #[must_use]
    pub const fn as_uuid(&self) -> &uuid::Uuid {
        &self.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a conversation session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(uuid::Uuid);

impl SessionId {
    /// Create a new random session ID
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Create a session ID from a UUID
    #[must_use]
    pub const fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an edge in the graph
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

impl fmt::Display for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a prompt template
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemplateId(uuid::Uuid);

impl TemplateId {
    /// Create a new random template ID
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for TemplateId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TemplateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an autonomous agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(uuid::Uuid);

impl AgentId {
    /// Create a new random agent ID
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Create an agent ID from a UUID
    #[must_use]
    pub const fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Types of edges that can connect nodes (subset used by lineage)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// Links a template to its parent template (Template -> Template)
    Inherits,
    /// Links a prompt to the template it was created from (Prompt -> Template)
    Instantiates,
}

/// Semantic version for template versioning
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version {
    /// Major version (breaking changes)
    pub major: u16,
    /// Minor version (new features, backwards compatible)
    pub minor: u16,
    /// Patch version (bug fixes)
    pub patch: u16,
}

impl Version {
    /// Create a new version
    #[must_use]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Bump major version (resets minor and patch to 0)
    pub fn bump_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
    }

    /// Bump minor version (resets patch to 0)
    pub fn bump_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
    }

    /// Bump patch version
    pub fn bump_patch(&mut self) {
        self.patch += 1;
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::new(1, 0, 0)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Metadata associated with a prompt
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptMetadata {
    /// The LLM model name (e.g., "gpt-4", "claude-3-opus")
    #[serde(default)]
    pub model: String,
    /// Temperature parameter for generation
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Maximum tokens to generate
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    /// List of tools/functions available to the model
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools_available: Vec<String>,
    /// Additional custom metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, String>,
}

fn default_temperature() -> f32 {
    0.7
}

/// A prompt node representing input to an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptNode {
    /// Unique node identifier
    pub id: NodeId,
    /// Session this prompt belongs to
    pub session_id: SessionId,
    /// When the prompt was created
    pub timestamp: DateTime<Utc>,
    /// Optional template this prompt was instantiated from
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<TemplateId>,
    /// The actual prompt content
    pub content: String,
    /// Variables used if instantiated from a template
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub variables: HashMap<String, String>,
    /// Metadata about the prompt
    #[serde(default)]
    pub metadata: PromptMetadata,
}

impl PromptNode {
    /// Create a new prompt node
    #[must_use]
    pub fn new(session_id: SessionId, content: String) -> Self {
        Self {
            id: NodeId::new(),
            session_id,
            timestamp: Utc::now(),
            template_id: None,
            content,
            variables: HashMap::new(),
            metadata: PromptMetadata::default(),
        }
    }

    /// Create a prompt with custom metadata
    #[must_use]
    pub fn with_metadata(session_id: SessionId, content: String, metadata: PromptMetadata) -> Self {
        Self {
            id: NodeId::new(),
            session_id,
            timestamp: Utc::now(),
            template_id: None,
            content,
            variables: HashMap::new(),
            metadata,
        }
    }

    /// Create a prompt from a template
    #[must_use]
    pub fn from_template(
        session_id: SessionId,
        template_id: TemplateId,
        content: String,
        variables: HashMap<String, String>,
    ) -> Self {
        Self {
            id: NodeId::new(),
            session_id,
            timestamp: Utc::now(),
            template_id: Some(template_id),
            content,
            variables,
            metadata: PromptMetadata::default(),
        }
    }
}

// ============================================================================
// Lineage Identifiers
// ============================================================================

/// Unique identifier for a lineage chain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LineageId(uuid::Uuid);

impl LineageId {
    /// Create a new random lineage ID
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Create a lineage ID from a UUID
    #[must_use]
    pub const fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID
    #[must_use]
    pub const fn as_uuid(&self) -> &uuid::Uuid {
        &self.0
    }
}

impl Default for LineageId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LineageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Lineage Evolution Types
// ============================================================================

/// The type of evolution between prompt versions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvolutionType {
    /// Direct evolution from a previous version (minor improvements)
    Evolves,
    /// Refinement based on feedback or performance data
    Refines,
    /// Derivation creating a new branch or variant
    Derives,
    /// Merge of multiple prompt lineages
    Merges,
    /// Fork creating an experimental branch
    Forks,
    /// Reversion to a previous known-good version
    Reverts,
}

impl fmt::Display for EvolutionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolves => write!(f, "evolves"),
            Self::Refines => write!(f, "refines"),
            Self::Derives => write!(f, "derives"),
            Self::Merges => write!(f, "merges"),
            Self::Forks => write!(f, "forks"),
            Self::Reverts => write!(f, "reverts"),
        }
    }
}

impl std::str::FromStr for EvolutionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "evolves" => Ok(Self::Evolves),
            "refines" => Ok(Self::Refines),
            "derives" => Ok(Self::Derives),
            "merges" => Ok(Self::Merges),
            "forks" => Ok(Self::Forks),
            "reverts" => Ok(Self::Reverts),
            _ => Err(format!("Invalid evolution type: {}", s)),
        }
    }
}

/// Reason for the evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionReason {
    /// Category of the change
    pub category: EvolutionCategory,
    /// Human-readable description of why the evolution occurred
    pub description: String,
    /// Source of the evolution trigger
    pub trigger_source: TriggerSource,
    /// Associated metrics that influenced the decision (if any)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub performance_delta: Option<PerformanceDelta>,
}

/// Category of evolution change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvolutionCategory {
    /// Performance optimization
    Performance,
    /// Quality improvement
    Quality,
    /// Safety/alignment enhancement
    Safety,
    /// Cost optimization
    Cost,
    /// Capability expansion
    Capability,
    /// Bug fix or error correction
    Correction,
    /// Structural refactoring
    Refactoring,
    /// Experimentation
    Experiment,
}

/// Source that triggered the evolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSource {
    /// Human operator initiated the change
    Human,
    /// Automated system (e.g., A/B testing)
    Automated,
    /// AI agent self-improvement
    Agent,
    /// External feedback system
    Feedback,
    /// Scheduled review process
    Review,
}

/// Performance change between versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceDelta {
    /// Metric name
    pub metric: String,
    /// Previous value
    pub previous_value: f64,
    /// New value
    pub new_value: f64,
    /// Percentage change
    pub percent_change: f64,
}

impl PerformanceDelta {
    /// Create a new performance delta
    #[must_use]
    pub fn new(metric: String, previous_value: f64, new_value: f64) -> Self {
        let percent_change = if previous_value != 0.0 {
            ((new_value - previous_value) / previous_value) * 100.0
        } else {
            0.0
        };
        Self {
            metric,
            previous_value,
            new_value,
            percent_change,
        }
    }
}

// ============================================================================
// Input Schema
// ============================================================================

/// Input schema for lineage tracking requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageInput {
    /// The prompt content to track
    pub prompt_content: String,

    /// Session context for this prompt
    pub session_id: SessionId,

    /// Optional template this prompt derives from
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<TemplateId>,

    /// Parent prompt ID (if this is an evolution)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_prompt_id: Option<NodeId>,

    /// Type of evolution (if parent exists)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evolution_type: Option<EvolutionType>,

    /// Reason for evolution (if applicable)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evolution_reason: Option<EvolutionReason>,

    /// Agent that created this prompt
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creating_agent_id: Option<AgentId>,

    /// Existing lineage chain ID (if joining an existing chain)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage_id: Option<LineageId>,

    /// Prompt metadata
    #[serde(default)]
    pub metadata: PromptMetadata,

    /// Variables used in template instantiation
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub variables: HashMap<String, String>,

    /// Tags for categorization
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Request timestamp (defaults to now if not provided)
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
}

impl LineageInput {
    /// Create a new lineage input for an initial prompt (no parent)
    #[must_use]
    pub fn new_initial(session_id: SessionId, prompt_content: String) -> Self {
        Self {
            prompt_content,
            session_id,
            template_id: None,
            parent_prompt_id: None,
            evolution_type: None,
            evolution_reason: None,
            creating_agent_id: None,
            lineage_id: None,
            metadata: PromptMetadata::default(),
            variables: HashMap::new(),
            tags: Vec::new(),
            timestamp: Utc::now(),
        }
    }

    /// Create a lineage input for an evolved prompt
    #[must_use]
    pub fn new_evolution(
        session_id: SessionId,
        prompt_content: String,
        parent_prompt_id: NodeId,
        evolution_type: EvolutionType,
        reason: EvolutionReason,
    ) -> Self {
        Self {
            prompt_content,
            session_id,
            template_id: None,
            parent_prompt_id: Some(parent_prompt_id),
            evolution_type: Some(evolution_type),
            evolution_reason: Some(reason),
            creating_agent_id: None,
            lineage_id: None,
            metadata: PromptMetadata::default(),
            variables: HashMap::new(),
            tags: Vec::new(),
            timestamp: Utc::now(),
        }
    }

    /// Validate the input for consistency
    pub fn validate(&self) -> Result<(), ValidationError> {
        // If parent exists, evolution type must be specified
        if self.parent_prompt_id.is_some() && self.evolution_type.is_none() {
            return Err(ValidationError::MissingEvolutionType);
        }

        // If evolution type exists, parent must exist (unless it's a merge)
        if let Some(evolution) = &self.evolution_type {
            if self.parent_prompt_id.is_none() && *evolution != EvolutionType::Merges {
                return Err(ValidationError::MissingParentPrompt);
            }
        }

        // Content must not be empty
        if self.prompt_content.trim().is_empty() {
            return Err(ValidationError::EmptyContent);
        }

        // Content length check (max 1MB)
        if self.prompt_content.len() > 1_000_000 {
            return Err(ValidationError::ContentTooLarge {
                size: self.prompt_content.len(),
                max: 1_000_000,
            });
        }

        Ok(())
    }

    /// Compute a hash of the inputs for decision event tracking
    #[must_use]
    pub fn compute_inputs_hash(&self) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.prompt_content.hash(&mut hasher);
        self.session_id.to_string().hash(&mut hasher);
        if let Some(parent) = &self.parent_prompt_id {
            parent.to_string().hash(&mut hasher);
        }
        format!("{:016x}", hasher.finish())
    }
}

// ============================================================================
// Output Schema
// ============================================================================

/// Output schema for lineage tracking results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageOutput {
    /// The created or updated lineage node
    pub lineage_node: LineageNode,

    /// The lineage chain this node belongs to
    pub lineage_id: LineageId,

    /// Edge created (if this was an evolution)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evolution_edge: Option<LineageEdge>,

    /// Position in the lineage chain (1-indexed)
    pub chain_position: u32,

    /// Total depth of the lineage chain
    pub chain_depth: u32,

    /// Association strength (confidence) of the lineage relationship
    pub confidence: f64,

    /// Graph constraints that were applied
    pub constraints_applied: Vec<GraphConstraint>,

    /// Warnings generated during processing
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,

    /// Processing timestamp
    pub timestamp: DateTime<Utc>,
}

impl LineageOutput {
    /// Create a successful output for an initial prompt
    #[must_use]
    pub fn new_initial(lineage_node: LineageNode, lineage_id: LineageId) -> Self {
        Self {
            lineage_node,
            lineage_id,
            evolution_edge: None,
            chain_position: 1,
            chain_depth: 1,
            confidence: 1.0,
            constraints_applied: vec![GraphConstraint::SingleRootPerChain],
            warnings: Vec::new(),
            timestamp: Utc::now(),
        }
    }

    /// Create a successful output for an evolved prompt
    #[must_use]
    pub fn new_evolution(
        lineage_node: LineageNode,
        lineage_id: LineageId,
        evolution_edge: LineageEdge,
        chain_position: u32,
        chain_depth: u32,
        confidence: f64,
    ) -> Self {
        Self {
            lineage_node,
            lineage_id,
            evolution_edge: Some(evolution_edge),
            chain_position,
            chain_depth,
            confidence,
            constraints_applied: vec![
                GraphConstraint::EvolutionMustHaveParent,
                GraphConstraint::NoCircularLineage,
            ],
            warnings: Vec::new(),
            timestamp: Utc::now(),
        }
    }

    /// Add a warning to the output
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

// ============================================================================
// Decision Event (for ruvector-service)
// ============================================================================

/// Decision event for ruvector-service integration
///
/// This struct captures the complete decision context for lineage tracking
/// operations, enabling audit trails and machine learning on agent decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEvent {
    /// Unique identifier for this decision event
    pub event_id: uuid::Uuid,

    /// Agent that made this decision
    pub agent_id: String,

    /// Version of the agent
    pub agent_version: String,

    /// Type of decision made
    pub decision_type: DecisionType,

    /// Hash of the inputs used for this decision
    pub inputs_hash: String,

    /// Outputs produced by this decision
    pub outputs: DecisionOutputs,

    /// Confidence score for the decision (0.0 to 1.0)
    pub confidence: f64,

    /// Graph constraints that were applied
    pub constraints_applied: Vec<GraphConstraint>,

    /// Reference to the execution context
    pub execution_ref: ExecutionRef,

    /// Timestamp of the decision (UTC)
    pub timestamp: DateTime<Utc>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl DecisionEvent {
    /// Create a new decision event
    #[must_use]
    pub fn new(
        agent_id: String,
        agent_version: String,
        inputs_hash: String,
        outputs: DecisionOutputs,
        confidence: f64,
        constraints_applied: Vec<GraphConstraint>,
        execution_ref: ExecutionRef,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4(),
            agent_id,
            agent_version,
            decision_type: DecisionType::PromptLineageTracking,
            inputs_hash,
            outputs,
            confidence: confidence.clamp(0.0, 1.0),
            constraints_applied,
            execution_ref,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create from lineage input and output
    #[must_use]
    pub fn from_lineage(
        agent_id: String,
        agent_version: String,
        input: &LineageInput,
        output: &LineageOutput,
        execution_ref: ExecutionRef,
    ) -> Self {
        let outputs = DecisionOutputs {
            node_id: output.lineage_node.base.id,
            lineage_id: output.lineage_id,
            edge_id: output.evolution_edge.as_ref().map(|e| e.id),
            chain_position: output.chain_position,
            chain_depth: output.chain_depth,
        };

        Self::new(
            agent_id,
            agent_version,
            input.compute_inputs_hash(),
            outputs,
            output.confidence,
            output.constraints_applied.clone(),
            execution_ref,
        )
    }

    /// Add metadata to the event
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
    }

    /// Get event key for partitioning (e.g., Kafka)
    #[must_use]
    pub fn partition_key(&self) -> String {
        format!("lineage:{}", self.outputs.lineage_id)
    }

    /// Serialize to JSON for transmission
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Type of decision made by the agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionType {
    /// Prompt lineage tracking decision
    PromptLineageTracking,
}

impl fmt::Display for DecisionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PromptLineageTracking => write!(f, "prompt_lineage_tracking"),
        }
    }
}

/// Outputs from a decision event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOutputs {
    /// Created or updated node ID
    pub node_id: NodeId,
    /// Lineage chain ID
    pub lineage_id: LineageId,
    /// Created edge ID (if evolution)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_id: Option<EdgeId>,
    /// Position in chain
    pub chain_position: u32,
    /// Depth of chain
    pub chain_depth: u32,
}

/// Reference to the execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRef {
    /// Trace ID for distributed tracing
    pub trace_id: String,
    /// Span ID within the trace
    pub span_id: String,
    /// Session context
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Request ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl ExecutionRef {
    /// Create a new execution reference
    #[must_use]
    pub fn new(trace_id: String, span_id: String) -> Self {
        Self {
            trace_id,
            span_id,
            session_id: None,
            request_id: None,
        }
    }

    /// Create with full context
    #[must_use]
    pub fn with_context(
        trace_id: String,
        span_id: String,
        session_id: Option<String>,
        request_id: Option<String>,
    ) -> Self {
        Self {
            trace_id,
            span_id,
            session_id,
            request_id,
        }
    }
}

// ============================================================================
// Lineage Node
// ============================================================================

/// Extended prompt node for lineage tracking
///
/// This wraps the base `PromptNode` and adds lineage-specific metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    /// Base prompt node data
    #[serde(flatten)]
    pub base: PromptNode,

    /// Lineage chain this node belongs to
    pub lineage_id: LineageId,

    /// Version within the lineage chain
    pub lineage_version: Version,

    /// Position in the lineage chain (1-indexed)
    pub chain_position: u32,

    /// Whether this is the root of the lineage chain
    pub is_root: bool,

    /// Whether this is the current head of the lineage chain
    pub is_head: bool,

    /// Branch name (for forked lineages)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Agent that created this node
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creating_agent: Option<AgentId>,

    /// Performance metrics for this version
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub performance_metrics: Option<LineageMetrics>,

    /// Timestamp when lineage tracking was applied
    pub lineage_tracked_at: DateTime<Utc>,
}

impl LineageNode {
    /// Create a new root lineage node
    #[must_use]
    pub fn new_root(base: PromptNode, lineage_id: LineageId) -> Self {
        Self {
            base,
            lineage_id,
            lineage_version: Version::new(1, 0, 0),
            chain_position: 1,
            is_root: true,
            is_head: true,
            branch: None,
            creating_agent: None,
            performance_metrics: None,
            lineage_tracked_at: Utc::now(),
        }
    }

    /// Create a new evolved lineage node
    #[must_use]
    pub fn new_evolution(
        base: PromptNode,
        lineage_id: LineageId,
        parent_version: Version,
        chain_position: u32,
        evolution_type: EvolutionType,
    ) -> Self {
        let lineage_version = Self::compute_next_version(parent_version, evolution_type);
        Self {
            base,
            lineage_id,
            lineage_version,
            chain_position,
            is_root: false,
            is_head: true, // New evolutions become the head
            branch: None,
            creating_agent: None,
            performance_metrics: None,
            lineage_tracked_at: Utc::now(),
        }
    }

    /// Compute the next version based on evolution type
    #[must_use]
    fn compute_next_version(mut parent_version: Version, evolution_type: EvolutionType) -> Version {
        match evolution_type {
            EvolutionType::Evolves | EvolutionType::Refines => {
                parent_version.bump_patch();
            }
            EvolutionType::Derives | EvolutionType::Forks => {
                parent_version.bump_minor();
            }
            EvolutionType::Merges => {
                parent_version.bump_major();
            }
            EvolutionType::Reverts => {
                // Reverts keep the version but mark it differently
                parent_version.bump_patch();
            }
        }
        parent_version
    }

    /// Set performance metrics for this node
    pub fn set_metrics(&mut self, metrics: LineageMetrics) {
        self.performance_metrics = Some(metrics);
    }

    /// Mark this node as no longer the head (when a new evolution is created)
    pub fn mark_not_head(&mut self) {
        self.is_head = false;
    }

    /// Set the branch name for forked lineages
    pub fn set_branch(&mut self, branch: String) {
        self.branch = Some(branch);
    }
}

/// Performance metrics for a lineage node
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LineageMetrics {
    /// Number of times this version was used
    pub usage_count: u64,
    /// Average response quality score (0.0 to 1.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg_quality_score: Option<f64>,
    /// Average latency in milliseconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg_latency_ms: Option<f64>,
    /// Success rate (0.0 to 1.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_rate: Option<f64>,
    /// Cost per invocation (if applicable)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_per_invocation: Option<f64>,
    /// First used timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_used: Option<DateTime<Utc>>,
    /// Last used timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used: Option<DateTime<Utc>>,
}

impl LineageMetrics {
    /// Create new metrics with initial usage
    #[must_use]
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            usage_count: 1,
            avg_quality_score: None,
            avg_latency_ms: None,
            success_rate: None,
            cost_per_invocation: None,
            first_used: Some(now),
            last_used: Some(now),
        }
    }

    /// Record a new usage
    pub fn record_usage(&mut self, quality: Option<f64>, latency_ms: Option<f64>, success: bool) {
        self.usage_count += 1;
        self.last_used = Some(Utc::now());

        // Update average quality
        if let Some(q) = quality {
            self.avg_quality_score = Some(match self.avg_quality_score {
                Some(avg) => (avg * (self.usage_count - 1) as f64 + q) / self.usage_count as f64,
                None => q,
            });
        }

        // Update average latency
        if let Some(l) = latency_ms {
            self.avg_latency_ms = Some(match self.avg_latency_ms {
                Some(avg) => (avg * (self.usage_count - 1) as f64 + l) / self.usage_count as f64,
                None => l,
            });
        }

        // Update success rate
        let success_val = if success { 1.0 } else { 0.0 };
        self.success_rate = Some(match self.success_rate {
            Some(rate) => {
                (rate * (self.usage_count - 1) as f64 + success_val) / self.usage_count as f64
            }
            None => success_val,
        });
    }
}

// ============================================================================
// Lineage Edge
// ============================================================================

/// Edge properties for evolution tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    /// Unique edge identifier
    pub id: EdgeId,

    /// Source node (parent prompt)
    pub from: NodeId,

    /// Target node (evolved prompt)
    pub to: NodeId,

    /// Type of edge (mapped to EdgeType)
    pub edge_type: LineageEdgeType,

    /// Type of evolution
    pub evolution_type: EvolutionType,

    /// Reason for the evolution
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evolution_reason: Option<EvolutionReason>,

    /// Semantic similarity between parent and child (0.0 to 1.0)
    pub similarity_score: f64,

    /// Content diff summary
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_summary: Option<DiffSummary>,

    /// When the evolution occurred
    pub created_at: DateTime<Utc>,

    /// Additional properties
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, String>,
}

/// Type of lineage edge, maps to core EdgeType
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageEdgeType {
    /// Evolves edge - direct evolution
    Evolves,
    /// Refines edge - refinement based on feedback
    Refines,
    /// Derives edge - derivation creating new variant
    Derives,
}

impl LineageEdgeType {
    /// Convert to core EdgeType
    ///
    /// Note: This maps to the closest equivalent in the core edge types.
    /// For lineage-specific semantics, use LineageEdgeType directly.
    #[must_use]
    pub fn to_core_edge_type(&self) -> EdgeType {
        // All lineage edges represent inheritance/derivation relationships
        EdgeType::Inherits
    }
}

impl From<EvolutionType> for LineageEdgeType {
    fn from(evolution: EvolutionType) -> Self {
        match evolution {
            EvolutionType::Evolves | EvolutionType::Reverts => Self::Evolves,
            EvolutionType::Refines => Self::Refines,
            EvolutionType::Derives | EvolutionType::Forks | EvolutionType::Merges => Self::Derives,
        }
    }
}

impl LineageEdge {
    /// Create a new lineage edge
    #[must_use]
    pub fn new(
        from: NodeId,
        to: NodeId,
        evolution_type: EvolutionType,
        similarity_score: f64,
    ) -> Self {
        Self {
            id: EdgeId::new(),
            from,
            to,
            edge_type: LineageEdgeType::from(evolution_type),
            evolution_type,
            evolution_reason: None,
            similarity_score: similarity_score.clamp(0.0, 1.0),
            diff_summary: None,
            created_at: Utc::now(),
            properties: HashMap::new(),
        }
    }

    /// Create with full details
    #[must_use]
    pub fn with_details(
        from: NodeId,
        to: NodeId,
        evolution_type: EvolutionType,
        evolution_reason: EvolutionReason,
        similarity_score: f64,
        diff_summary: DiffSummary,
    ) -> Self {
        Self {
            id: EdgeId::new(),
            from,
            to,
            edge_type: LineageEdgeType::from(evolution_type),
            evolution_type,
            evolution_reason: Some(evolution_reason),
            similarity_score: similarity_score.clamp(0.0, 1.0),
            diff_summary: Some(diff_summary),
            created_at: Utc::now(),
            properties: HashMap::new(),
        }
    }

    /// Add a property to the edge
    pub fn add_property(&mut self, key: String, value: String) {
        self.properties.insert(key, value);
    }

    /// Convert to property map for storage
    #[must_use]
    pub fn to_properties(&self) -> HashMap<String, String> {
        let mut props = self.properties.clone();
        props.insert(
            "evolution_type".to_string(),
            self.evolution_type.to_string(),
        );
        props.insert(
            "similarity_score".to_string(),
            self.similarity_score.to_string(),
        );
        props.insert("created_at".to_string(), self.created_at.to_rfc3339());
        if let Some(ref reason) = self.evolution_reason {
            props.insert(
                "evolution_reason".to_string(),
                serde_json::to_string(reason).unwrap_or_default(),
            );
        }
        if let Some(ref diff) = self.diff_summary {
            props.insert(
                "diff_summary".to_string(),
                serde_json::to_string(diff).unwrap_or_default(),
            );
        }
        props
    }
}

/// Summary of differences between parent and child prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Number of characters added
    pub chars_added: usize,
    /// Number of characters removed
    pub chars_removed: usize,
    /// Number of lines changed
    pub lines_changed: usize,
    /// Sections that were modified
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modified_sections: Vec<String>,
    /// Brief textual summary of changes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl DiffSummary {
    /// Create a new diff summary
    #[must_use]
    pub fn new(chars_added: usize, chars_removed: usize, lines_changed: usize) -> Self {
        Self {
            chars_added,
            chars_removed,
            lines_changed,
            modified_sections: Vec::new(),
            summary: None,
        }
    }

    /// Compute diff between two strings
    #[must_use]
    pub fn compute(parent: &str, child: &str) -> Self {
        let parent_len = parent.len();
        let child_len = child.len();

        let (chars_added, chars_removed) = if child_len >= parent_len {
            (child_len - parent_len, 0)
        } else {
            (0, parent_len - child_len)
        };

        let parent_lines = parent.lines().count();
        let child_lines = child.lines().count();
        let lines_changed = parent_lines.abs_diff(child_lines);

        Self {
            chars_added,
            chars_removed,
            lines_changed,
            modified_sections: Vec::new(),
            summary: None,
        }
    }
}

// ============================================================================
// Graph Constraints
// ============================================================================

/// Constraints applied to maintain graph integrity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphConstraint {
    /// Each lineage chain must have exactly one root
    SingleRootPerChain,
    /// Evolution edges must have a parent prompt
    EvolutionMustHaveParent,
    /// No circular references in lineage chains
    NoCircularLineage,
    /// Maximum chain depth limit (prevents runaway evolution)
    MaxChainDepth,
    /// Branch names must be unique within a lineage
    UniqueBranchNames,
    /// Template references must exist
    ValidTemplateReference,
    /// Session must exist for prompt
    ValidSessionReference,
}

impl fmt::Display for GraphConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SingleRootPerChain => write!(f, "single_root_per_chain"),
            Self::EvolutionMustHaveParent => write!(f, "evolution_must_have_parent"),
            Self::NoCircularLineage => write!(f, "no_circular_lineage"),
            Self::MaxChainDepth => write!(f, "max_chain_depth"),
            Self::UniqueBranchNames => write!(f, "unique_branch_names"),
            Self::ValidTemplateReference => write!(f, "valid_template_reference"),
            Self::ValidSessionReference => write!(f, "valid_session_reference"),
        }
    }
}

// ============================================================================
// Validation Errors
// ============================================================================

/// Validation errors for lineage inputs
#[derive(Debug, Clone, Error)]
pub enum ValidationError {
    /// Evolution type is required when parent prompt exists
    #[error("Evolution type is required when parent prompt is specified")]
    MissingEvolutionType,

    /// Parent prompt is required for evolution
    #[error("Parent prompt is required for evolution type (except merges)")]
    MissingParentPrompt,

    /// Content cannot be empty
    #[error("Prompt content cannot be empty")]
    EmptyContent,

    /// Content exceeds maximum size
    #[error("Prompt content too large: {size} bytes exceeds maximum of {max} bytes")]
    ContentTooLarge {
        /// Actual size
        size: usize,
        /// Maximum allowed
        max: usize,
    },

    /// Invalid lineage chain reference
    #[error("Invalid lineage chain reference: {0}")]
    InvalidLineageReference(String),

    /// Circular lineage detected
    #[error("Circular lineage detected: {0}")]
    CircularLineage(String),

    /// Maximum chain depth exceeded
    #[error("Maximum chain depth of {max} exceeded (current: {current})")]
    MaxDepthExceeded {
        /// Current depth
        current: u32,
        /// Maximum allowed
        max: u32,
    },

    /// Invalid graph constraint
    #[error("Graph constraint violation: {0}")]
    ConstraintViolation(GraphConstraint),
}

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for the lineage tracking agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageConfig {
    /// Maximum allowed chain depth
    pub max_chain_depth: u32,
    /// Maximum content size in bytes
    pub max_content_size: usize,
    /// Default confidence threshold for similarity
    pub similarity_threshold: f64,
    /// Enable automatic diff computation
    pub compute_diffs: bool,
    /// Enable performance metrics tracking
    pub track_metrics: bool,
    /// Agent identifier
    pub agent_id: String,
    /// Agent version
    pub agent_version: String,
}

impl Default for LineageConfig {
    fn default() -> Self {
        Self {
            max_chain_depth: 100,
            max_content_size: 1_000_000,
            similarity_threshold: 0.7,
            compute_diffs: true,
            track_metrics: true,
            agent_id: "prompt-lineage-agent".to_string(),
            agent_version: "1.0.0".to_string(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lineage_id_creation() {
        let id1 = LineageId::new();
        let id2 = LineageId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_evolution_type_display() {
        assert_eq!(EvolutionType::Evolves.to_string(), "evolves");
        assert_eq!(EvolutionType::Refines.to_string(), "refines");
        assert_eq!(EvolutionType::Derives.to_string(), "derives");
    }

    #[test]
    fn test_evolution_type_from_str() {
        assert_eq!(
            "evolves".parse::<EvolutionType>().unwrap(),
            EvolutionType::Evolves
        );
        assert_eq!(
            "REFINES".parse::<EvolutionType>().unwrap(),
            EvolutionType::Refines
        );
        assert!("invalid".parse::<EvolutionType>().is_err());
    }

    #[test]
    fn test_lineage_input_validation_empty_content() {
        let input = LineageInput::new_initial(SessionId::new(), "".to_string());
        assert!(matches!(
            input.validate(),
            Err(ValidationError::EmptyContent)
        ));
    }

    #[test]
    fn test_lineage_input_validation_missing_evolution_type() {
        let mut input =
            LineageInput::new_initial(SessionId::new(), "Test prompt content".to_string());
        input.parent_prompt_id = Some(NodeId::new());
        assert!(matches!(
            input.validate(),
            Err(ValidationError::MissingEvolutionType)
        ));
    }

    #[test]
    fn test_lineage_input_valid() {
        let input =
            LineageInput::new_initial(SessionId::new(), "Test prompt content".to_string());
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_lineage_input_evolution_valid() {
        let reason = EvolutionReason {
            category: EvolutionCategory::Quality,
            description: "Improved clarity".to_string(),
            trigger_source: TriggerSource::Human,
            performance_delta: None,
        };

        let input = LineageInput::new_evolution(
            SessionId::new(),
            "Evolved prompt content".to_string(),
            NodeId::new(),
            EvolutionType::Evolves,
            reason,
        );

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_lineage_node_version_computation() {
        let parent_version = Version::new(1, 0, 0);

        // Test patch bump for evolves
        let node = LineageNode::new_evolution(
            PromptNode::new(SessionId::new(), "Test".to_string()),
            LineageId::new(),
            parent_version.clone(),
            2,
            EvolutionType::Evolves,
        );
        assert_eq!(node.lineage_version, Version::new(1, 0, 1));

        // Test minor bump for derives
        let node2 = LineageNode::new_evolution(
            PromptNode::new(SessionId::new(), "Test".to_string()),
            LineageId::new(),
            parent_version.clone(),
            2,
            EvolutionType::Derives,
        );
        assert_eq!(node2.lineage_version, Version::new(1, 1, 0));

        // Test major bump for merges
        let node3 = LineageNode::new_evolution(
            PromptNode::new(SessionId::new(), "Test".to_string()),
            LineageId::new(),
            parent_version,
            2,
            EvolutionType::Merges,
        );
        assert_eq!(node3.lineage_version, Version::new(2, 0, 0));
    }

    #[test]
    fn test_lineage_edge_creation() {
        let edge = LineageEdge::new(
            NodeId::new(),
            NodeId::new(),
            EvolutionType::Refines,
            0.85,
        );

        assert_eq!(edge.edge_type, LineageEdgeType::Refines);
        assert_eq!(edge.evolution_type, EvolutionType::Refines);
        assert_eq!(edge.similarity_score, 0.85);
    }

    #[test]
    fn test_lineage_edge_similarity_clamping() {
        let edge1 = LineageEdge::new(NodeId::new(), NodeId::new(), EvolutionType::Evolves, 1.5);
        assert_eq!(edge1.similarity_score, 1.0);

        let edge2 = LineageEdge::new(NodeId::new(), NodeId::new(), EvolutionType::Evolves, -0.5);
        assert_eq!(edge2.similarity_score, 0.0);
    }

    #[test]
    fn test_diff_summary_compute() {
        let parent = "Hello world";
        let child = "Hello beautiful world!";

        let diff = DiffSummary::compute(parent, child);
        assert!(diff.chars_added > 0);
        assert_eq!(diff.chars_removed, 0);
    }

    #[test]
    fn test_decision_event_creation() {
        let outputs = DecisionOutputs {
            node_id: NodeId::new(),
            lineage_id: LineageId::new(),
            edge_id: None,
            chain_position: 1,
            chain_depth: 1,
        };

        let exec_ref = ExecutionRef::new("trace-123".to_string(), "span-456".to_string());

        let event = DecisionEvent::new(
            "prompt-lineage-agent".to_string(),
            "1.0.0".to_string(),
            "abc123".to_string(),
            outputs,
            0.95,
            vec![GraphConstraint::SingleRootPerChain],
            exec_ref,
        );

        assert_eq!(event.decision_type, DecisionType::PromptLineageTracking);
        assert_eq!(event.confidence, 0.95);
        assert_eq!(event.constraints_applied.len(), 1);
    }

    #[test]
    fn test_decision_event_confidence_clamping() {
        let outputs = DecisionOutputs {
            node_id: NodeId::new(),
            lineage_id: LineageId::new(),
            edge_id: None,
            chain_position: 1,
            chain_depth: 1,
        };

        let exec_ref = ExecutionRef::new("trace-123".to_string(), "span-456".to_string());

        let event = DecisionEvent::new(
            "agent".to_string(),
            "1.0.0".to_string(),
            "hash".to_string(),
            outputs,
            1.5, // Should be clamped to 1.0
            vec![],
            exec_ref,
        );

        assert_eq!(event.confidence, 1.0);
    }

    #[test]
    fn test_lineage_metrics_record_usage() {
        let mut metrics = LineageMetrics::new();
        assert_eq!(metrics.usage_count, 1);

        metrics.record_usage(Some(0.9), Some(100.0), true);
        assert_eq!(metrics.usage_count, 2);
        assert!(metrics.avg_quality_score.is_some());
        assert!(metrics.avg_latency_ms.is_some());
        assert!(metrics.success_rate.is_some());
    }

    #[test]
    fn test_performance_delta() {
        let delta = PerformanceDelta::new("latency_ms".to_string(), 100.0, 80.0);
        assert_eq!(delta.previous_value, 100.0);
        assert_eq!(delta.new_value, 80.0);
        assert_eq!(delta.percent_change, -20.0);
    }

    #[test]
    fn test_graph_constraint_display() {
        assert_eq!(
            GraphConstraint::SingleRootPerChain.to_string(),
            "single_root_per_chain"
        );
        assert_eq!(
            GraphConstraint::NoCircularLineage.to_string(),
            "no_circular_lineage"
        );
    }

    #[test]
    fn test_lineage_config_default() {
        let config = LineageConfig::default();
        assert_eq!(config.max_chain_depth, 100);
        assert_eq!(config.max_content_size, 1_000_000);
        assert_eq!(config.similarity_threshold, 0.7);
        assert!(config.compute_diffs);
        assert!(config.track_metrics);
    }

    #[test]
    fn test_lineage_output_initial() {
        let node = LineageNode::new_root(
            PromptNode::new(SessionId::new(), "Test".to_string()),
            LineageId::new(),
        );
        let lineage_id = node.lineage_id;

        let output = LineageOutput::new_initial(node, lineage_id);

        assert_eq!(output.chain_position, 1);
        assert_eq!(output.chain_depth, 1);
        assert_eq!(output.confidence, 1.0);
        assert!(output.evolution_edge.is_none());
    }

    #[test]
    fn test_inputs_hash_deterministic() {
        let input1 =
            LineageInput::new_initial(SessionId::new(), "Test prompt content".to_string());

        // Create identical input
        let input2 = LineageInput {
            prompt_content: input1.prompt_content.clone(),
            session_id: input1.session_id,
            ..input1.clone()
        };

        assert_eq!(input1.compute_inputs_hash(), input2.compute_inputs_hash());
    }

    #[test]
    fn test_decision_event_serialization() {
        let outputs = DecisionOutputs {
            node_id: NodeId::new(),
            lineage_id: LineageId::new(),
            edge_id: Some(EdgeId::new()),
            chain_position: 2,
            chain_depth: 3,
        };

        let exec_ref = ExecutionRef::with_context(
            "trace-123".to_string(),
            "span-456".to_string(),
            Some("session-789".to_string()),
            Some("req-abc".to_string()),
        );

        let event = DecisionEvent::new(
            "prompt-lineage-agent".to_string(),
            "1.0.0".to_string(),
            "hash123".to_string(),
            outputs,
            0.92,
            vec![
                GraphConstraint::EvolutionMustHaveParent,
                GraphConstraint::NoCircularLineage,
            ],
            exec_ref,
        );

        let json = event.to_json().unwrap();
        assert!(json.contains("prompt_lineage_tracking"));
        assert!(json.contains("0.92"));

        // Verify round-trip
        let deserialized: DecisionEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.agent_id, "prompt-lineage-agent");
        assert_eq!(deserialized.confidence, 0.92);
    }

    #[test]
    fn test_version_operations() {
        let mut v = Version::new(1, 0, 0);
        assert_eq!(v.to_string(), "1.0.0");

        v.bump_patch();
        assert_eq!(v, Version::new(1, 0, 1));

        v.bump_minor();
        assert_eq!(v, Version::new(1, 1, 0));

        v.bump_major();
        assert_eq!(v, Version::new(2, 0, 0));
    }

    #[test]
    fn test_prompt_node_creation() {
        let session_id = SessionId::new();
        let prompt = PromptNode::new(session_id, "Test content".to_string());

        assert_eq!(prompt.session_id, session_id);
        assert_eq!(prompt.content, "Test content");
        assert!(prompt.template_id.is_none());
    }

    #[test]
    fn test_lineage_edge_type_conversion() {
        assert_eq!(
            LineageEdgeType::from(EvolutionType::Evolves),
            LineageEdgeType::Evolves
        );
        assert_eq!(
            LineageEdgeType::from(EvolutionType::Refines),
            LineageEdgeType::Refines
        );
        assert_eq!(
            LineageEdgeType::from(EvolutionType::Derives),
            LineageEdgeType::Derives
        );
        assert_eq!(
            LineageEdgeType::from(EvolutionType::Forks),
            LineageEdgeType::Derives
        );
        assert_eq!(
            LineageEdgeType::from(EvolutionType::Merges),
            LineageEdgeType::Derives
        );
        assert_eq!(
            LineageEdgeType::from(EvolutionType::Reverts),
            LineageEdgeType::Evolves
        );
    }
}

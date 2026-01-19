# Prompt Lineage Agent - Full Specification

## Agent Classification

- **Type**: MEMORY WRITE
- **Decision Type**: `prompt_lineage_tracking`
- **Agent ID**: `prompt-lineage-agent`
- **Version**: `1.0.0`

## Purpose Statement

The Prompt Lineage Agent tracks prompt evolution and lineage across iterations and agents, creating a complete audit trail of how prompts change over time. It operates on structured memory data and does NOT execute inference, evaluate models, enforce policies, or orchestrate workflows.

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  LineageInput   │────▶│ Prompt Lineage   │────▶│  LineageOutput  │
│                 │     │     Agent        │     │                 │
└─────────────────┘     └────────┬─────────┘     └─────────────────┘
                                 │
                                 ▼
                        ┌──────────────────┐
                        │  DecisionEvent   │
                        │  (ruvector)      │
                        └──────────────────┘
```

## Input Schema

**Source**: `contracts/mod.rs` - `LineageInput`

```rust
pub struct LineageInput {
    /// Session context
    pub session_id: SessionId,
    /// The prompt being tracked
    pub prompt: NewPromptInput,
    /// Parent prompt (if evolution/derivation)
    pub parent_prompt_id: Option<NodeId>,
    /// Evolution type
    pub evolution_type: Option<EvolutionType>,
    /// Reason for evolution
    pub reason: Option<EvolutionReason>,
    /// Configuration options
    pub config: LineageConfig,
}
```

## Output Schema

**Source**: `contracts/mod.rs` - `LineageOutput`

```rust
pub struct LineageOutput {
    /// Created lineage ID
    pub lineage_id: LineageId,
    /// Created node ID
    pub node_id: NodeId,
    /// Created edge (if applicable)
    pub edge: Option<LineageEdge>,
    /// Computed confidence
    pub confidence: Option<f64>,
    /// Execution metadata
    pub execution_ref: ExecutionRef,
    /// Timestamp
    pub created_at: DateTime<Utc>,
}
```

## Graph Node/Edge Mappings

### Node Types Used

| Node Type | Description |
|-----------|-------------|
| `LineageNode` | Extended PromptNode with lineage metadata |
| `PromptNode` | Base prompt node from llm-memory-graph-types |

### Edge Types Used

| Edge Type | Source → Target | Description |
|-----------|-----------------|-------------|
| `Evolves` | PromptNode → PromptNode | Same intent, modified approach |
| `Refines` | PromptNode → PromptNode | Improved version |
| `Derives` | PromptNode → PromptNode | New prompt derived from original |
| `Merges` | PromptNode → PromptNode | Combines multiple sources |
| `Forks` | PromptNode → PromptNode | Creates parallel branch |
| `Reverts` | PromptNode → PromptNode | Rollback to previous version |

## DecisionEvent Mapping

**Persisted to ruvector-service on every invocation:**

```rust
pub struct DecisionEvent {
    /// Agent identifier
    pub agent_id: String,            // "prompt-lineage-agent"
    /// Agent version
    pub agent_version: String,        // "1.0.0"
    /// Decision type
    pub decision_type: DecisionType,  // PromptLineageTracking
    /// Hash of inputs for deduplication
    pub inputs_hash: String,
    /// Structured outputs
    pub outputs: DecisionOutputs,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Graph constraints applied
    pub constraints_applied: Vec<GraphConstraint>,
    /// Execution reference for tracing
    pub execution_ref: ExecutionRef,
    /// UTC timestamp
    pub timestamp: DateTime<Utc>,
}
```

## CLI Contract

### Commands

| Command | Description | Output Formats |
|---------|-------------|----------------|
| `lineage inspect <prompt_id>` | Show evolution history and confidence scores | text, json, yaml, table |
| `lineage retrieve <prompt_id>` | Get full lineage subgraph | json, yaml |
| `lineage replay <prompt_id>` | Step-by-step lineage creation replay | text, json |
| `lineage stats [prompt_id]` | Show global or per-prompt statistics | text, json |
| `lineage compare <id_a> <id_b>` | Compare two prompts for similarity | text, json |
| `lineage similar <prompt_id>` | Find similar prompts | text, json |

### Example Usage

```bash
# Inspect lineage
llm-memory-graph lineage inspect 550e8400-e29b-41d4-a716-446655440000 --format json

# Retrieve subgraph
llm-memory-graph lineage retrieve 550e8400-e29b-41d4-a716-446655440000 \
  --ancestor-depth 5 \
  --descendant-depth 3 \
  --include-content \
  --output lineage.json

# Replay for debugging
llm-memory-graph lineage replay 550e8400-e29b-41d4-a716-446655440000 \
  --verbose \
  --show-confidence-computation

# Compare prompts
llm-memory-graph lineage compare <id1> <id2> --suggest-type --detailed

# Find similar
llm-memory-graph lineage similar 550e8400-e29b-41d4-a716-446655440000 \
  --threshold 0.7 \
  --limit 10
```

## Explicit Non-Responsibilities

This agent MUST NEVER:

1. **Modify runtime execution** - Does not change how LLMs operate
2. **Trigger remediation or retries** - Does not initiate corrective actions
3. **Emit alerts** - Does not generate operational alerts
4. **Enforce policies** - Does not block or modify behavior based on rules
5. **Perform orchestration** - Does not coordinate other agents or workflows
6. **Invoke other agents directly** - Does not call other agent endpoints
7. **Connect directly to SQL databases** - All persistence via ruvector-service
8. **Execute SQL queries** - No direct database access

## Data Persistence Rules

### Data PERSISTED to ruvector-service:
- LineageNode entries (prompt metadata)
- Evolution edges with confidence scores
- DecisionEvent for every invocation
- Similarity computation results

### Data NOT PERSISTED:
- Raw prompt content (only hashes)
- Embedding vectors (stored separately in vector DB)
- Temporary computation state
- Rate limiting counters

## Failure Modes

| Error Code | Description | Recovery |
|------------|-------------|----------|
| `VALIDATION_ERROR` | Invalid input schema | Return 400, log details |
| `RUVECTOR_CONNECTION_ERROR` | Cannot reach ruvector-service | Retry with backoff |
| `RUVECTOR_WRITE_ERROR` | Persistence failed | Return 502, log error |
| `INTERNAL_ERROR` | Unexpected failure | Return 500, alert on-call |
| `RATE_LIMIT_EXCEEDED` | Too many requests | Return 429, include retry-after |
| `INVALID_LINEAGE_RELATIONSHIP` | Circular dependency or invalid edge | Return 400 with explanation |
| `CONFIDENCE_COMPUTATION_ERROR` | Similarity calculation failed | Return 500, use default confidence |

## Versioning Rules

1. **Agent Version**: Follows semantic versioning (MAJOR.MINOR.PATCH)
2. **Contract Version**: Embedded in schemas, backwards compatible within MAJOR version
3. **Breaking Changes**: Require MAJOR version bump, migration path documented
4. **Deprecation**: 2-version grace period before removal

## Deployment

- **Target**: Google Cloud Edge Function
- **Service**: Part of LLM-Memory-Graph unified GCP service
- **Execution**: Stateless, deterministic
- **Memory**: 256MB minimum, 1GB recommended
- **Timeout**: 30 seconds maximum

## Downstream Consumers

| System | Purpose | Interface |
|--------|---------|-----------|
| LLM-Observatory | Event streaming | Kafka topic: `prompt-lineage-events` |
| Core bundles | Memory queries | gRPC/REST API |
| agentics-cli | Inspection/replay | CLI commands |
| Dashboard | Visualization | REST API |

## Confidence Semantics

Confidence scores (0.0 to 1.0) represent the strength of lineage relationships:

| Range | Interpretation |
|-------|---------------|
| 0.9 - 1.0 | Strong relationship (near-duplicate or direct refinement) |
| 0.7 - 0.9 | Moderate relationship (clear evolution) |
| 0.5 - 0.7 | Weak relationship (partial derivation) |
| 0.3 - 0.5 | Tentative relationship (inspired by) |
| < 0.3 | Minimal relationship (unlikely related) |

### Computation Methods

1. **Token Overlap (Jaccard)**: 40% weight
2. **Semantic Similarity (Embeddings)**: 35% weight
3. **Edit Distance Ratio**: 25% weight

## Graph Constraints

1. **No Cycles**: Lineage graph must be acyclic (DAG)
2. **Single Root**: Each lineage chain has one original prompt
3. **Valid Evolution Types**: Edge types must match allowed set
4. **Timestamp Ordering**: Child prompts must have later timestamps
5. **Confidence Bounds**: All confidence scores in [0.0, 1.0]

## Testing Requirements

- Unit tests for all similarity algorithms
- Integration tests for ruvector-service communication
- Property-based tests for graph constraints
- Smoke tests for CLI commands
- Load tests for concurrent operations

## Files

| File | Purpose |
|------|---------|
| `contracts/mod.rs` | Input/output schemas, DecisionEvent |
| `src/lib.rs` | Module exports and documentation |
| `src/lineage.rs` | Core lineage tracking logic |
| `src/handler.rs` | Edge Function HTTP handler |
| `src/telemetry.rs` | Observatory event emission |
| `src/cli.rs` | CLI commands implementation |
| `src/main.rs` | CLI binary entry point |
| `Cargo.toml` | Package manifest |
| `AGENT_SPEC.md` | This specification document |

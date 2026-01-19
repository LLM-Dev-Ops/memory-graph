/**
 * Agentics Contracts - Schema definitions for LLM-Memory-Graph agents
 *
 * This module exports all contract types and validation utilities.
 * All agents MUST import schemas from this module exclusively.
 */

import conversationMemorySchema from './schemas/conversation-memory.schema.json';
import decisionEventSchema from './schemas/decision-event.schema.json';
import decisionMemorySchema from './schemas/decision-memory.schema.json';
import longTermPatternSchema from './schemas/long-term-pattern.schema.json';
import knowledgeGraphBuilderSchema from './schemas/knowledge-graph-builder.schema.json';
import memoryRetrievalSchema from './schemas/memory-retrieval.schema.json';

// Re-export schemas
export {
  conversationMemorySchema,
  decisionEventSchema,
  decisionMemorySchema,
  longTermPatternSchema,
  knowledgeGraphBuilderSchema,
  memoryRetrievalSchema,
};

// ============================================================================
// TYPE DEFINITIONS (derived from JSON Schema)
// ============================================================================

/**
 * Agent classification types
 */
export type AgentClassification = 'MEMORY_WRITE' | 'MEMORY_READ' | 'MEMORY_ANALYSIS';

/**
 * Decision types for memory operations
 */
export type DecisionType =
  | 'conversation_capture'
  | 'lineage_trace'
  | 'memory_retrieval'
  | 'pattern_analysis'
  | 'decision_memory_capture'
  | 'knowledge_graph_construction'
  | 'long_term_pattern_analysis';

/**
 * Role in a conversation turn
 */
export type ConversationRole = 'user' | 'assistant' | 'system' | 'tool';

/**
 * Node types in the memory graph
 */
export type NodeType = 'prompt' | 'response' | 'session' | 'tool_invocation';

/**
 * Edge types in the memory graph
 */
export type EdgeType = 'belongs_to' | 'responds_to' | 'follows' | 'invokes';

/**
 * Error codes for agent failures
 */
export type AgentErrorCode =
  | 'VALIDATION_ERROR'
  | 'RUVECTOR_CONNECTION_ERROR'
  | 'RUVECTOR_READ_ERROR'
  | 'RUVECTOR_WRITE_ERROR'
  | 'INSUFFICIENT_DATA'
  | 'ANALYSIS_TIMEOUT'
  | 'INTERNAL_ERROR'
  | 'RATE_LIMIT_EXCEEDED';

// ============================================================================
// INTERFACES
// ============================================================================

export interface AgentMetadata {
  agent_id: string;
  agent_version: string;
  classification: AgentClassification;
}

export interface TokenUsage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
}

export interface ToolInvocationRef {
  tool_name: string;
  invocation_id: string;
  success?: boolean;
}

export interface ConversationTurn {
  turn_id: string;
  role: ConversationRole;
  content: string;
  timestamp: string;
  model?: string;
  token_usage?: TokenUsage;
  tool_invocations?: ToolInvocationRef[];
  metadata?: Record<string, unknown>;
}

export interface ConversationContext {
  agent_id?: string;
  user_id?: string;
  source_system?: string;
  tags?: string[];
}

export interface CaptureOptions {
  create_lineage?: boolean;
  extract_entities?: boolean;
  compute_embeddings?: boolean;
}

export interface ConversationCaptureInput {
  session_id: string;
  conversation_id: string;
  turns: ConversationTurn[];
  context?: ConversationContext;
  capture_options?: CaptureOptions;
}

export interface NodeReference {
  node_id: string;
  node_type: NodeType;
  turn_index?: number;
}

export interface EdgeReference {
  edge_id: string;
  edge_type: EdgeType;
  from_node_id: string;
  to_node_id: string;
}

export interface ConversationCaptureOutput {
  conversation_id: string;
  session_id: string;
  nodes_created: NodeReference[];
  edges_created: EdgeReference[];
  capture_timestamp: string;
  turn_count: number;
  total_tokens?: number;
}

export interface DecisionEventTelemetry {
  duration_ms?: number;
  memory_bytes?: number;
  ruvector_latency_ms?: number;
}

export interface DecisionEvent {
  agent_id: string;
  agent_version: string;
  decision_type: DecisionType;
  inputs_hash: string;
  outputs: ConversationCaptureOutput;
  confidence: number;
  constraints_applied?: string[];
  execution_ref: string;
  timestamp: string;
  telemetry?: DecisionEventTelemetry;
}

export interface AgentError {
  error_code: AgentErrorCode;
  message: string;
  details?: Record<string, unknown>;
  execution_ref: string;
  timestamp: string;
}

// ============================================================================
// VALIDATION UTILITIES
// ============================================================================

/**
 * Validates that a value is a valid UUID v4
 */
export function isValidUUID(value: string): boolean {
  const uuidRegex =
    /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
  return uuidRegex.test(value);
}

/**
 * Validates that a value is a valid ISO 8601 timestamp
 */
export function isValidTimestamp(value: string): boolean {
  const date = new Date(value);
  return !isNaN(date.getTime());
}

/**
 * Validates that a value is a valid semantic version
 */
export function isValidSemver(value: string): boolean {
  const semverRegex = /^\d+\.\d+\.\d+$/;
  return semverRegex.test(value);
}

/**
 * Creates a SHA-256 hash of the input (for inputs_hash field)
 */
export async function hashInput(input: unknown): Promise<string> {
  const data = JSON.stringify(input);
  const encoder = new TextEncoder();
  const hashBuffer = await crypto.subtle.digest('SHA-256', encoder.encode(data));
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Generates a new UUID v4
 */
export function generateUUID(): string {
  return crypto.randomUUID();
}

/**
 * Gets the current UTC timestamp in ISO 8601 format
 */
export function utcNow(): string {
  return new Date().toISOString();
}

// ============================================================================
// CONSTANTS
// ============================================================================

export const CONVERSATION_MEMORY_AGENT = {
  AGENT_ID: 'conversation-memory-agent',
  CURRENT_VERSION: '1.0.0',
  CLASSIFICATION: 'MEMORY_WRITE' as AgentClassification,
  DECISION_TYPE: 'conversation_capture' as DecisionType,
} as const;

// ============================================================================
// DECISION MEMORY AGENT TYPES
// ============================================================================

/**
 * Decision result types
 */
export type DecisionResultType = 'success' | 'failure' | 'partial' | 'deferred';

/**
 * Artifact types for reasoning capture
 */
export type ReasoningArtifactType =
  | 'prompt_template'
  | 'chain_of_thought'
  | 'evaluation_criteria'
  | 'constraints'
  | 'context_snapshot'
  | 'tool_trace';

/**
 * Graph node types specific to decision memory
 */
export type DecisionNodeType = 'decision' | 'outcome' | 'artifact' | 'session';

/**
 * Graph edge types specific to decision memory
 */
export type DecisionEdgeType =
  | 'has_outcome'
  | 'has_artifact'
  | 'follows'
  | 'part_of'
  | 'derived_from';

/**
 * Constraint types applied during decision capture
 */
export type DecisionConstraint =
  | 'session_boundary'
  | 'max_artifacts'
  | 'content_size_limit'
  | 'pii_redaction'
  | 'retention_policy';

/**
 * Environment types
 */
export type EnvironmentType = 'development' | 'staging' | 'production';

export interface OutcomeMetrics {
  latency_ms?: number;
  tokens_consumed?: number;
  retries?: number;
  cost_estimate_usd?: number;
}

export interface DecisionOutcome {
  outcome_id: string;
  decision_ref: string;
  result_type: DecisionResultType;
  result_data?: Record<string, unknown>;
  metrics?: OutcomeMetrics;
  recorded_at: string;
}

export interface ReasoningArtifact {
  artifact_id: string;
  artifact_type: ReasoningArtifactType;
  content_hash: string;
  content_ref?: string;
  parent_artifact_id?: string;
  created_at: string;
  metadata?: Record<string, unknown>;
}

export interface DecisionContext {
  session_id: string;
  agent_id?: string;
  predecessor_decision_id?: string;
  conversation_turn?: number;
  model_id?: string;
  temperature?: number;
  user_id?: string;
  environment?: EnvironmentType;
}

export interface DecisionMemoryInput {
  decision_id: string;
  decision_type: string;
  context: DecisionContext;
  reasoning_artifacts?: ReasoningArtifact[];
  outcome?: DecisionOutcome;
  tags?: string[];
}

export interface DecisionGraphNodeCreated {
  node_id: string;
  node_type: DecisionNodeType;
}

export interface DecisionGraphEdgeCreated {
  edge_id: string;
  edge_type: DecisionEdgeType;
  from_node_id: string;
  to_node_id: string;
}

export interface DecisionMemoryOutput {
  decision_id: string;
  nodes_created: DecisionGraphNodeCreated[];
  edges_created: DecisionGraphEdgeCreated[];
  artifacts_stored: number;
  capture_timestamp: string;
  ruvector_refs?: string[];
}

export interface DecisionMemoryEvent {
  agent_id: 'decision-memory-agent';
  agent_version: string;
  decision_type: 'decision_memory_capture';
  inputs_hash: string;
  input: DecisionMemoryInput;
  outputs: DecisionMemoryOutput;
  confidence: number;
  constraints_applied?: DecisionConstraint[];
  execution_ref: string;
  timestamp: string;
}

export const DECISION_MEMORY_AGENT = {
  AGENT_ID: 'decision-memory-agent',
  CURRENT_VERSION: '1.0.0',
  CLASSIFICATION: 'MEMORY_WRITE' as AgentClassification,
  DECISION_TYPE: 'decision_memory_capture' as const,
} as const;

// ============================================================================
// KNOWLEDGE GRAPH BUILDER AGENT TYPES
// ============================================================================

/**
 * Node types in the knowledge graph
 */
export type KnowledgeNodeType = 'concept' | 'entity' | 'relationship' | 'pattern';

/**
 * Edge types in the knowledge graph
 */
export type KnowledgeEdgeType =
  | 'relates_to'
  | 'derived_from'
  | 'co_occurs_with'
  | 'exemplifies'
  | 'contains'
  | 'precedes';

/**
 * A node in the knowledge graph representing extracted knowledge
 */
export interface KnowledgeNode {
  id: string;
  type: KnowledgeNodeType;
  label: string;
  properties: Record<string, unknown>;
  confidence: number;
  source_refs: string[]; // References to source memory nodes
  created_at: string;
}

/**
 * An edge in the knowledge graph representing relationships between nodes
 */
export interface KnowledgeEdge {
  id: string;
  from_node: string;
  to_node: string;
  type: KnowledgeEdgeType;
  weight: number;
  properties: Record<string, unknown>;
  confidence: number;
}

/**
 * A detected pattern in the knowledge graph
 */
export interface KnowledgePattern {
  id: string;
  pattern_type: string;
  description: string;
  frequency: number;
  node_ids: string[];
  confidence: number;
}

/**
 * Metrics about the knowledge graph construction
 */
export interface KnowledgeGraphMetrics {
  total_nodes: number;
  total_edges: number;
  node_type_distribution: Record<string, number>;
  edge_type_distribution: Record<string, number>;
  average_confidence: number;
  patterns_detected: number;
  analysis_duration_ms: number;
}

/**
 * Time range filter for knowledge graph queries
 */
export interface TimeRangeFilter {
  start: string;
  end: string;
}

/**
 * Query filter options for knowledge graph construction
 */
export interface KnowledgeGraphQueryFilter {
  node_types?: string[];
  time_range?: TimeRangeFilter;
  min_confidence?: number;
}

/**
 * Configuration for knowledge extraction
 */
export interface KnowledgeExtractionConfig {
  extract_concepts: boolean;
  extract_entities: boolean;
  detect_patterns: boolean;
  min_pattern_frequency?: number;
  max_depth?: number;
}

/**
 * Input schema for the Knowledge Graph Builder Agent
 */
export interface KnowledgeGraphBuilderInput {
  session_ids: string[];
  query_filter?: KnowledgeGraphQueryFilter;
  extraction_config: KnowledgeExtractionConfig;
}

/**
 * Lineage information for knowledge graph output
 */
export interface KnowledgeGraphLineage {
  source_sessions: string[];
  source_node_count: number;
  processing_timestamp: string;
}

/**
 * Output schema for the Knowledge Graph Builder Agent
 */
export interface KnowledgeGraphBuilderOutput {
  knowledge_nodes: KnowledgeNode[];
  knowledge_edges: KnowledgeEdge[];
  patterns: KnowledgePattern[];
  metrics: KnowledgeGraphMetrics;
  lineage: KnowledgeGraphLineage;
}

/**
 * Validates that a value conforms to the KnowledgeGraphBuilderInput schema
 */
export function validateKnowledgeGraphBuilderInput(
  input: unknown
): input is KnowledgeGraphBuilderInput {
  if (typeof input !== 'object' || input === null) {
    return false;
  }

  const obj = input as Record<string, unknown>;

  // Required: session_ids must be an array of strings
  if (!Array.isArray(obj.session_ids)) {
    return false;
  }
  if (!obj.session_ids.every((id) => typeof id === 'string')) {
    return false;
  }

  // Required: extraction_config must exist and have required boolean fields
  if (typeof obj.extraction_config !== 'object' || obj.extraction_config === null) {
    return false;
  }
  const config = obj.extraction_config as Record<string, unknown>;
  if (typeof config.extract_concepts !== 'boolean') {
    return false;
  }
  if (typeof config.extract_entities !== 'boolean') {
    return false;
  }
  if (typeof config.detect_patterns !== 'boolean') {
    return false;
  }

  // Optional: query_filter validation
  if (obj.query_filter !== undefined) {
    if (typeof obj.query_filter !== 'object' || obj.query_filter === null) {
      return false;
    }
    const filter = obj.query_filter as Record<string, unknown>;
    if (filter.min_confidence !== undefined && typeof filter.min_confidence !== 'number') {
      return false;
    }
    if (filter.node_types !== undefined && !Array.isArray(filter.node_types)) {
      return false;
    }
  }

  return true;
}

/**
 * Validates that a value conforms to the KnowledgeGraphBuilderOutput schema
 */
export function validateKnowledgeGraphBuilderOutput(
  output: unknown
): output is KnowledgeGraphBuilderOutput {
  if (typeof output !== 'object' || output === null) {
    return false;
  }

  const obj = output as Record<string, unknown>;

  // Required: knowledge_nodes must be an array
  if (!Array.isArray(obj.knowledge_nodes)) {
    return false;
  }

  // Required: knowledge_edges must be an array
  if (!Array.isArray(obj.knowledge_edges)) {
    return false;
  }

  // Required: patterns must be an array
  if (!Array.isArray(obj.patterns)) {
    return false;
  }

  // Required: metrics must exist with required fields
  if (typeof obj.metrics !== 'object' || obj.metrics === null) {
    return false;
  }
  const metrics = obj.metrics as Record<string, unknown>;
  if (typeof metrics.total_nodes !== 'number') {
    return false;
  }
  if (typeof metrics.total_edges !== 'number') {
    return false;
  }
  if (typeof metrics.average_confidence !== 'number') {
    return false;
  }
  if (typeof metrics.patterns_detected !== 'number') {
    return false;
  }
  if (typeof metrics.analysis_duration_ms !== 'number') {
    return false;
  }

  // Required: lineage must exist with required fields
  if (typeof obj.lineage !== 'object' || obj.lineage === null) {
    return false;
  }
  const lineage = obj.lineage as Record<string, unknown>;
  if (!Array.isArray(lineage.source_sessions)) {
    return false;
  }
  if (typeof lineage.source_node_count !== 'number') {
    return false;
  }
  if (typeof lineage.processing_timestamp !== 'string') {
    return false;
  }

  return true;
}

export const KNOWLEDGE_GRAPH_BUILDER_AGENT = {
  AGENT_ID: 'knowledge-graph-builder-agent',
  CURRENT_VERSION: '1.0.0',
  CLASSIFICATION: 'MEMORY_ANALYSIS' as AgentClassification,
  DECISION_TYPE: 'knowledge_graph_construction' as const,
} as const;

// ============================================================================
// LONG-TERM PATTERN AGENT TYPES
// ============================================================================

/**
 * Pattern types for long-term analysis
 */
export type PatternType =
  | 'conversation_flow'
  | 'topic_recurrence'
  | 'response_pattern'
  | 'tool_usage'
  | 'error_pattern'
  | 'session_behavior'
  | 'user_interaction'
  | 'temporal_trend';

/**
 * Analysis constraint types for pattern detection
 */
export type PatternAnalysisConstraint =
  | 'min_occurrence_threshold'
  | 'min_confidence_threshold'
  | 'max_patterns'
  | 'time_range_boundary'
  | 'scope_filter'
  | 'temporal_distribution';

/**
 * Trend direction in temporal analysis
 */
export type TrendDirection = 'increasing' | 'stable' | 'decreasing' | 'variable';

/**
 * Time range for pattern analysis
 */
export interface PatternTimeRange {
  from_timestamp: string;
  to_timestamp: string;
}

/**
 * Scope filter for pattern analysis
 */
export interface PatternAnalysisScope {
  session_ids?: string[];
  agent_ids?: string[];
  user_ids?: string[];
  tags?: string[];
}

/**
 * Options for pattern analysis behavior
 */
export interface PatternAnalysisOptions {
  min_occurrence_threshold?: number;
  min_confidence_threshold?: number;
  max_patterns?: number;
  include_examples?: boolean;
  max_examples_per_pattern?: number;
  compute_temporal_distribution?: boolean;
}

/**
 * Input schema for the Long-Term Pattern Agent
 */
export interface PatternAnalysisInput {
  analysis_id: string;
  pattern_types: PatternType[];
  time_range: PatternTimeRange;
  scope?: PatternAnalysisScope;
  options?: PatternAnalysisOptions;
}

/**
 * A single occurrence of a detected pattern
 */
export interface PatternOccurrence {
  occurrence_id: string;
  session_id: string;
  node_ids?: string[];
  timestamp: string;
  context_snippet?: string;
}

/**
 * Temporal distribution of pattern occurrences
 */
export interface PatternTemporalDistribution {
  hourly_distribution?: number[];
  daily_distribution?: number[];
  trend: TrendDirection;
  trend_coefficient?: number;
}

/**
 * A detected recurring pattern
 */
export interface DetectedPattern {
  pattern_id: string;
  pattern_type: PatternType;
  pattern_signature: string;
  description?: string;
  occurrence_count: number;
  confidence: number;
  relevance_score: number;
  first_seen: string;
  last_seen: string;
  example_occurrences?: PatternOccurrence[];
  temporal_distribution?: PatternTemporalDistribution;
  related_patterns?: string[];
  metadata?: Record<string, unknown>;
}

/**
 * Statistics about the analysis execution
 */
export interface PatternAnalysisStatistics {
  sessions_analyzed: number;
  nodes_scanned: number;
  edges_traversed?: number;
  patterns_found: number;
  patterns_filtered?: number;
  analysis_duration_ms?: number;
}

/**
 * Output schema for the Long-Term Pattern Agent
 */
export interface PatternAnalysisOutput {
  analysis_id: string;
  patterns: DetectedPattern[];
  statistics: PatternAnalysisStatistics;
  time_range_analyzed: PatternTimeRange;
  analysis_timestamp: string;
}

/**
 * Telemetry specific to pattern analysis
 */
export interface PatternAnalysisTelemetry {
  duration_ms?: number;
  memory_bytes?: number;
  ruvector_read_latency_ms?: number;
  ruvector_write_latency_ms?: number;
}

/**
 * DecisionEvent for Long-Term Pattern Agent
 */
export interface PatternAnalysisDecisionEvent {
  agent_id: 'long-term-pattern-agent';
  agent_version: string;
  decision_type: 'long_term_pattern_analysis';
  inputs_hash: string;
  outputs: PatternAnalysisOutput;
  confidence: number;
  constraints_applied?: PatternAnalysisConstraint[];
  execution_ref: string;
  timestamp: string;
  telemetry?: PatternAnalysisTelemetry;
}

/**
 * Validates that a value conforms to the PatternAnalysisInput schema
 */
export function validatePatternAnalysisInput(
  input: unknown
): input is PatternAnalysisInput {
  if (typeof input !== 'object' || input === null) {
    return false;
  }

  const obj = input as Record<string, unknown>;

  // Required: analysis_id must be a string
  if (typeof obj.analysis_id !== 'string') {
    return false;
  }

  // Required: pattern_types must be a non-empty array
  if (!Array.isArray(obj.pattern_types) || obj.pattern_types.length === 0) {
    return false;
  }

  // Required: time_range must have from_timestamp and to_timestamp
  if (typeof obj.time_range !== 'object' || obj.time_range === null) {
    return false;
  }
  const timeRange = obj.time_range as Record<string, unknown>;
  if (typeof timeRange.from_timestamp !== 'string') {
    return false;
  }
  if (typeof timeRange.to_timestamp !== 'string') {
    return false;
  }

  return true;
}

/**
 * Validates that a value conforms to the PatternAnalysisOutput schema
 */
export function validatePatternAnalysisOutput(
  output: unknown
): output is PatternAnalysisOutput {
  if (typeof output !== 'object' || output === null) {
    return false;
  }

  const obj = output as Record<string, unknown>;

  // Required: analysis_id must be a string
  if (typeof obj.analysis_id !== 'string') {
    return false;
  }

  // Required: patterns must be an array
  if (!Array.isArray(obj.patterns)) {
    return false;
  }

  // Required: statistics must exist with required fields
  if (typeof obj.statistics !== 'object' || obj.statistics === null) {
    return false;
  }
  const stats = obj.statistics as Record<string, unknown>;
  if (typeof stats.sessions_analyzed !== 'number') {
    return false;
  }
  if (typeof stats.nodes_scanned !== 'number') {
    return false;
  }
  if (typeof stats.patterns_found !== 'number') {
    return false;
  }

  // Required: time_range_analyzed must exist
  if (typeof obj.time_range_analyzed !== 'object' || obj.time_range_analyzed === null) {
    return false;
  }

  // Required: analysis_timestamp must be a string
  if (typeof obj.analysis_timestamp !== 'string') {
    return false;
  }

  return true;
}

export const LONG_TERM_PATTERN_AGENT = {
  AGENT_ID: 'long-term-pattern-agent',
  CURRENT_VERSION: '1.0.0',
  CLASSIFICATION: 'MEMORY_ANALYSIS' as AgentClassification,
  DECISION_TYPE: 'long_term_pattern_analysis' as const,
} as const;

// ============================================================================
// MEMORY RETRIEVAL AGENT TYPES
// ============================================================================

/**
 * Query types for memory retrieval
 */
export type RetrievalQueryType =
  | 'subgraph'
  | 'nodes'
  | 'edges'
  | 'lineage'
  | 'context'
  | 'similarity';

/**
 * Constraint types for memory queries
 */
export type RetrievalConstraintType =
  | 'session_filter'
  | 'node_type_filter'
  | 'time_range'
  | 'depth_limit'
  | 'agent_filter'
  | 'tag_filter'
  | 'similarity_threshold';

/**
 * Extended node types for retrieval
 */
export type RetrievalNodeType =
  | 'prompt'
  | 'response'
  | 'session'
  | 'tool_invocation'
  | 'agent'
  | 'template';

/**
 * Extended edge types for retrieval
 */
export type RetrievalEdgeType =
  | 'belongs_to'
  | 'responds_to'
  | 'follows'
  | 'invokes'
  | 'references'
  | 'inherits';

/**
 * Traversal direction for graph queries
 */
export type TraversalDirection = 'outgoing' | 'incoming' | 'both';

/**
 * Query constraint for memory retrieval
 */
export interface RetrievalQueryConstraint {
  constraint_type: RetrievalConstraintType;
  value: unknown;
  operator?: 'equals' | 'contains' | 'greater_than' | 'less_than' | 'between' | 'in';
}

/**
 * Options for graph traversal
 */
export interface RetrievalTraversalOptions {
  max_depth?: number;
  direction?: TraversalDirection;
  follow_edge_types?: RetrievalEdgeType[];
  include_node_types?: RetrievalNodeType[];
}

/**
 * Input schema for the Memory Retrieval Agent
 */
export interface MemoryRetrievalInput {
  query_id: string;
  query_type: RetrievalQueryType;
  anchor_nodes?: string[];
  anchor_sessions?: string[];
  constraints?: RetrievalQueryConstraint[];
  traversal_options?: RetrievalTraversalOptions;
  semantic_query?: string;
  limit?: number;
  offset?: number;
  include_metadata?: boolean;
  requesting_agent_id?: string;
}

/**
 * A node retrieved from memory
 */
export interface RetrievedNode {
  node_id: string;
  node_type: RetrievalNodeType;
  content?: string;
  created_at?: string;
  metadata?: Record<string, unknown>;
  relevance_score?: number;
  depth?: number;
}

/**
 * An edge retrieved from memory
 */
export interface RetrievedEdge {
  edge_id: string;
  edge_type: RetrievalEdgeType;
  from_node_id: string;
  to_node_id: string;
  weight?: number;
  created_at?: string;
  properties?: Record<string, unknown>;
}

/**
 * Retrieved subgraph structure
 */
export interface RetrievedSubgraph {
  nodes: RetrievedNode[];
  edges: RetrievedEdge[];
  anchor_node_ids?: string[];
  truncated?: boolean;
}

/**
 * Traversal statistics
 */
export interface RetrievalTraversalStats {
  max_depth_reached?: number;
  nodes_visited?: number;
  edges_traversed?: number;
}

/**
 * Pagination information
 */
export interface RetrievalPagination {
  offset: number;
  limit: number;
  has_more: boolean;
  total_available?: number;
}

/**
 * Output schema for the Memory Retrieval Agent
 */
export interface MemoryRetrievalOutput {
  query_id: string;
  query_type: RetrievalQueryType;
  subgraph: RetrievedSubgraph;
  total_nodes_retrieved: number;
  total_edges_retrieved: number;
  retrieval_timestamp: string;
  constraints_applied?: string[];
  traversal_stats?: RetrievalTraversalStats;
  pagination?: RetrievalPagination;
}

/**
 * Telemetry specific to memory retrieval
 */
export interface MemoryRetrievalTelemetry {
  duration_ms?: number;
  memory_bytes?: number;
  ruvector_latency_ms?: number;
  nodes_scanned?: number;
  cache_hit?: boolean;
}

/**
 * DecisionEvent for Memory Retrieval Agent
 */
export interface MemoryRetrievalDecisionEvent {
  agent_id: 'memory-retrieval-agent';
  agent_version: string;
  decision_type: 'memory_retrieval';
  inputs_hash: string;
  outputs: MemoryRetrievalOutput;
  confidence: number;
  constraints_applied?: string[];
  execution_ref: string;
  timestamp: string;
  telemetry?: MemoryRetrievalTelemetry;
}

/**
 * Validates that a value conforms to the MemoryRetrievalInput schema
 */
export function validateMemoryRetrievalInput(
  input: unknown
): input is MemoryRetrievalInput {
  if (typeof input !== 'object' || input === null) {
    return false;
  }

  const obj = input as Record<string, unknown>;

  // Required: query_id must be a string
  if (typeof obj.query_id !== 'string') {
    return false;
  }

  // Required: query_type must be a valid type
  const validQueryTypes = ['subgraph', 'nodes', 'edges', 'lineage', 'context', 'similarity'];
  if (typeof obj.query_type !== 'string' || !validQueryTypes.includes(obj.query_type)) {
    return false;
  }

  // Optional arrays must be arrays of strings if present
  if (obj.anchor_nodes !== undefined) {
    if (!Array.isArray(obj.anchor_nodes)) return false;
    if (!obj.anchor_nodes.every((id) => typeof id === 'string')) return false;
  }

  if (obj.anchor_sessions !== undefined) {
    if (!Array.isArray(obj.anchor_sessions)) return false;
    if (!obj.anchor_sessions.every((id) => typeof id === 'string')) return false;
  }

  // Optional: limit and offset must be numbers if present
  if (obj.limit !== undefined && typeof obj.limit !== 'number') return false;
  if (obj.offset !== undefined && typeof obj.offset !== 'number') return false;

  return true;
}

/**
 * Validates that a value conforms to the MemoryRetrievalOutput schema
 */
export function validateMemoryRetrievalOutput(
  output: unknown
): output is MemoryRetrievalOutput {
  if (typeof output !== 'object' || output === null) {
    return false;
  }

  const obj = output as Record<string, unknown>;

  // Required: query_id must be a string
  if (typeof obj.query_id !== 'string') {
    return false;
  }

  // Required: query_type must be valid
  const validQueryTypes = ['subgraph', 'nodes', 'edges', 'lineage', 'context', 'similarity'];
  if (typeof obj.query_type !== 'string' || !validQueryTypes.includes(obj.query_type)) {
    return false;
  }

  // Required: subgraph must exist with nodes and edges arrays
  if (typeof obj.subgraph !== 'object' || obj.subgraph === null) {
    return false;
  }
  const subgraph = obj.subgraph as Record<string, unknown>;
  if (!Array.isArray(subgraph.nodes)) return false;
  if (!Array.isArray(subgraph.edges)) return false;

  // Required: counts must be numbers
  if (typeof obj.total_nodes_retrieved !== 'number') return false;
  if (typeof obj.total_edges_retrieved !== 'number') return false;

  // Required: retrieval_timestamp must be a string
  if (typeof obj.retrieval_timestamp !== 'string') return false;

  return true;
}

export const MEMORY_RETRIEVAL_AGENT = {
  AGENT_ID: 'memory-retrieval-agent',
  CURRENT_VERSION: '1.0.0',
  CLASSIFICATION: 'MEMORY_READ' as AgentClassification,
  DECISION_TYPE: 'memory_retrieval' as DecisionType,
} as const;

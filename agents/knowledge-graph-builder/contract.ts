/**
 * Knowledge Graph Builder Agent - Contract Definition
 *
 * @module agents/knowledge-graph-builder/contract
 * @version 1.0.0
 *
 * Classification: MEMORY_ANALYSIS
 * Decision Type: knowledge_graph_construction
 *
 * This agent constructs and maintains higher-order knowledge graphs from
 * memory nodes and relationships. It analyzes existing memory data to extract
 * concepts, entities, relationships, and patterns.
 *
 * EXPLICIT NON-RESPONSIBILITIES (from PROMPT 0 constraints):
 * - MUST NOT modify system behavior
 * - MUST NOT trigger remediation or automated actions
 * - MUST NOT emit alerts or notifications
 * - MUST NOT enforce policies
 * - MUST NOT orchestrate workflows
 * - MUST NOT connect directly to SQL databases
 * - MUST NOT make decisions that affect runtime behavior
 * - MUST NOT generate or execute code
 * - MUST NOT access external APIs beyond ruvector-service
 */

import { z } from 'zod';

// ============================================================================
// SECTION 1: AGENT METADATA & CONSTANTS
// ============================================================================

/**
 * Agent identifier - unique across all agents in the system
 */
export const AGENT_ID = 'knowledge-graph-builder-agent' as const;

/**
 * Current agent version following semantic versioning
 */
export const AGENT_VERSION = '1.0.0' as const;

/**
 * Agent classification category
 * MEMORY_ANALYSIS: Reads and analyzes memory data, produces derived insights
 */
export const AGENT_CLASSIFICATION = 'MEMORY_ANALYSIS' as const;

/**
 * Decision type for DecisionEvent emissions
 */
export const DECISION_TYPE = 'knowledge_graph_construction' as const;

/**
 * Agent metadata constant object
 */
export const KNOWLEDGE_GRAPH_BUILDER_AGENT = {
  AGENT_ID,
  AGENT_VERSION,
  CLASSIFICATION: AGENT_CLASSIFICATION,
  DECISION_TYPE,
} as const;

// ============================================================================
// SECTION 2: NODE TYPE DEFINITIONS
// ============================================================================

/**
 * Knowledge graph node types
 *
 * These are the node types created by this agent, distinct from
 * conversation memory nodes (prompt, response, session, tool_invocation).
 */
export type KnowledgeNodeType =
  | 'concept'           // Abstract concept extracted from content
  | 'entity'            // Named entity (person, organization, location, etc.)
  | 'relationship'      // Reified relationship node for complex associations
  | 'pattern'           // Recurring pattern detected across sessions
  | 'cluster'           // Group of semantically related nodes
  | 'topic'             // High-level topic classification
  | 'fact'              // Extracted factual assertion
  | 'reference';        // External reference or citation

/**
 * Entity subtypes for more granular classification
 */
export type EntitySubtype =
  | 'person'
  | 'organization'
  | 'location'
  | 'product'
  | 'technology'
  | 'event'
  | 'date'
  | 'quantity'
  | 'code_identifier'
  | 'file_path'
  | 'url'
  | 'other';

/**
 * Pattern types detected by analysis
 */
export type PatternType =
  | 'behavioral'        // User behavior patterns
  | 'conversational'    // Conversation flow patterns
  | 'temporal'          // Time-based patterns
  | 'semantic'          // Meaning/content patterns
  | 'structural'        // Graph structure patterns
  | 'preference'        // User preference patterns
  | 'error'             // Error/failure patterns
  | 'workflow';         // Task workflow patterns

// ============================================================================
// SECTION 3: EDGE TYPE DEFINITIONS
// ============================================================================

/**
 * Knowledge graph edge types
 *
 * These are the edge types created by this agent to connect
 * knowledge nodes to each other and to source memory nodes.
 */
export type KnowledgeEdgeType =
  | 'relates_to'        // General semantic relationship
  | 'derived_from'      // Source lineage (knowledge <- memory)
  | 'co_occurs_with'    // Co-occurrence in same context
  | 'exemplifies'       // Instance-of relationship
  | 'generalizes'       // Abstraction relationship (inverse of exemplifies)
  | 'part_of'           // Composition relationship
  | 'contains'          // Containment (inverse of part_of)
  | 'causes'            // Causal relationship
  | 'precedes'          // Temporal ordering
  | 'contradicts'       // Conflicting information
  | 'supports'          // Supporting evidence
  | 'similar_to'        // Semantic similarity (with weight)
  | 'references';       // Citation/reference relationship

/**
 * Edge weight semantics
 * - For similarity edges: cosine similarity (0-1)
 * - For co-occurrence: normalized frequency (0-1)
 * - For causal/temporal: confidence score (0-1)
 */
export interface EdgeWeight {
  value: number;        // Weight value between 0 and 1
  computation: string;  // How the weight was computed
}

// ============================================================================
// SECTION 4: INPUT SCHEMA DEFINITIONS
// ============================================================================

/**
 * Query filter for selecting memory nodes to process
 */
export const QueryFilterSchema = z.object({
  /** Filter by node types */
  node_types: z.array(z.enum(['prompt', 'response', 'session', 'tool_invocation'])).optional(),
  /** Filter by time range - start */
  time_range_start: z.string().datetime().optional(),
  /** Filter by time range - end */
  time_range_end: z.string().datetime().optional(),
  /** Filter by tags */
  tags: z.array(z.string()).optional(),
  /** Filter by agent_id in context */
  agent_ids: z.array(z.string()).optional(),
  /** Filter by user_id in context */
  user_ids: z.array(z.string()).optional(),
  /** Minimum confidence threshold for source nodes */
  min_confidence: z.number().min(0).max(1).optional(),
  /** Custom metadata filter (key-value pairs) */
  metadata_filter: z.record(z.unknown()).optional(),
});

/**
 * Configuration for knowledge extraction algorithms
 */
export const ExtractionConfigSchema = z.object({
  /** Enable concept extraction */
  extract_concepts: z.boolean().default(true),
  /** Enable named entity recognition */
  extract_entities: z.boolean().default(true),
  /** Enable relationship extraction */
  extract_relationships: z.boolean().default(true),
  /** Enable pattern detection */
  detect_patterns: z.boolean().default(true),
  /** Enable topic modeling */
  model_topics: z.boolean().default(false),
  /** Enable fact extraction */
  extract_facts: z.boolean().default(false),
  /** Minimum entity mention count for inclusion */
  min_entity_mentions: z.number().int().positive().default(1),
  /** Minimum pattern occurrence count */
  min_pattern_occurrences: z.number().int().positive().default(2),
  /** Confidence threshold for extracted knowledge (0-1) */
  confidence_threshold: z.number().min(0).max(1).default(0.7),
  /** Maximum entities to extract per session */
  max_entities_per_session: z.number().int().positive().default(100),
  /** Maximum concepts to extract per session */
  max_concepts_per_session: z.number().int().positive().default(50),
  /** Enable semantic clustering of similar nodes */
  enable_clustering: z.boolean().default(false),
  /** Similarity threshold for clustering (0-1) */
  clustering_threshold: z.number().min(0).max(1).default(0.8),
  /** Language hint for NLP processing */
  language_hint: z.string().default('en'),
});

/**
 * Main input schema for the Knowledge Graph Builder Agent
 */
export const KnowledgeGraphBuilderInputSchema = z.object({
  /**
   * Session IDs to analyze
   * At least one session_id is required
   */
  session_ids: z.array(z.string().uuid()).min(1),

  /**
   * Optional filter for selecting specific nodes within sessions
   */
  query_filter: QueryFilterSchema.optional(),

  /**
   * Configuration for knowledge extraction algorithms
   */
  extraction_config: ExtractionConfigSchema,

  /**
   * Maximum depth for graph traversal when building relationships
   * Default: 3 (direct relationships + 2 hops)
   */
  max_depth: z.number().int().min(1).max(10).default(3),

  /**
   * Whether to merge with existing knowledge graph
   * If false, creates isolated subgraph
   */
  merge_existing: z.boolean().default(true),

  /**
   * Optional execution reference for idempotency
   * If provided, allows resuming interrupted processing
   */
  idempotency_key: z.string().uuid().optional(),

  /**
   * Request metadata for audit trail
   */
  request_metadata: z.object({
    requester_id: z.string().optional(),
    request_source: z.string().optional(),
    correlation_id: z.string().uuid().optional(),
  }).optional(),
});

// ============================================================================
// SECTION 5: OUTPUT SCHEMA DEFINITIONS
// ============================================================================

/**
 * Knowledge node created by the agent
 */
export const KnowledgeNodeSchema = z.object({
  /** Unique node identifier */
  node_id: z.string().uuid(),
  /** Type of knowledge node */
  node_type: z.enum([
    'concept', 'entity', 'relationship', 'pattern',
    'cluster', 'topic', 'fact', 'reference'
  ]),
  /** Human-readable label */
  label: z.string().min(1).max(500),
  /** Canonical normalized form (lowercase, trimmed) */
  canonical_form: z.string().optional(),
  /** Entity subtype (for entity nodes) */
  entity_subtype: z.enum([
    'person', 'organization', 'location', 'product', 'technology',
    'event', 'date', 'quantity', 'code_identifier', 'file_path', 'url', 'other'
  ]).optional(),
  /** Pattern type (for pattern nodes) */
  pattern_type: z.enum([
    'behavioral', 'conversational', 'temporal', 'semantic',
    'structural', 'preference', 'error', 'workflow'
  ]).optional(),
  /** Extraction confidence score (0-1) */
  confidence: z.number().min(0).max(1),
  /** Number of source mentions/occurrences */
  mention_count: z.number().int().nonnegative(),
  /** Source session IDs where this knowledge was extracted */
  source_sessions: z.array(z.string().uuid()),
  /** Embedding vector reference (stored separately) */
  embedding_ref: z.string().optional(),
  /** Creation timestamp */
  created_at: z.string().datetime(),
  /** Additional metadata */
  metadata: z.record(z.unknown()).optional(),
});

/**
 * Knowledge edge created by the agent
 */
export const KnowledgeEdgeSchema = z.object({
  /** Unique edge identifier */
  edge_id: z.string().uuid(),
  /** Type of relationship */
  edge_type: z.enum([
    'relates_to', 'derived_from', 'co_occurs_with', 'exemplifies',
    'generalizes', 'part_of', 'contains', 'causes', 'precedes',
    'contradicts', 'supports', 'similar_to', 'references'
  ]),
  /** Source node ID */
  from_node_id: z.string().uuid(),
  /** Target node ID */
  to_node_id: z.string().uuid(),
  /** Edge weight/strength (0-1) */
  weight: z.number().min(0).max(1),
  /** How the weight was computed */
  weight_computation: z.string().optional(),
  /** Relationship confidence score (0-1) */
  confidence: z.number().min(0).max(1),
  /** Source evidence for this relationship */
  evidence_refs: z.array(z.string().uuid()).optional(),
  /** Creation timestamp */
  created_at: z.string().datetime(),
  /** Additional metadata */
  metadata: z.record(z.unknown()).optional(),
});

/**
 * Detected pattern in the knowledge graph
 */
export const PatternSchema = z.object({
  /** Unique pattern identifier */
  pattern_id: z.string().uuid(),
  /** Pattern type classification */
  pattern_type: z.enum([
    'behavioral', 'conversational', 'temporal', 'semantic',
    'structural', 'preference', 'error', 'workflow'
  ]),
  /** Human-readable pattern description */
  description: z.string(),
  /** Pattern detection confidence (0-1) */
  confidence: z.number().min(0).max(1),
  /** Number of occurrences detected */
  occurrence_count: z.number().int().positive(),
  /** Node IDs involved in this pattern */
  involved_nodes: z.array(z.string().uuid()),
  /** Sessions where pattern was observed */
  observed_in_sessions: z.array(z.string().uuid()),
  /** First occurrence timestamp */
  first_observed: z.string().datetime(),
  /** Last occurrence timestamp */
  last_observed: z.string().datetime(),
  /** Statistical significance score */
  significance_score: z.number().min(0).max(1).optional(),
  /** Additional pattern metadata */
  metadata: z.record(z.unknown()).optional(),
});

/**
 * Graph metrics computed during analysis
 */
export const GraphMetricsSchema = z.object({
  /** Total nodes in the constructed subgraph */
  total_nodes: z.number().int().nonnegative(),
  /** Total edges in the constructed subgraph */
  total_edges: z.number().int().nonnegative(),
  /** Nodes by type breakdown */
  nodes_by_type: z.record(z.number().int().nonnegative()),
  /** Edges by type breakdown */
  edges_by_type: z.record(z.number().int().nonnegative()),
  /** Number of connected components */
  connected_components: z.number().int().positive(),
  /** Average node degree */
  average_degree: z.number().nonnegative(),
  /** Graph density (0-1) */
  density: z.number().min(0).max(1),
  /** Maximum depth reached in traversal */
  max_depth_reached: z.number().int().nonnegative(),
  /** Number of source memory nodes processed */
  source_nodes_processed: z.number().int().nonnegative(),
  /** Processing duration in milliseconds */
  processing_duration_ms: z.number().int().nonnegative(),
  /** Number of patterns detected */
  patterns_detected: z.number().int().nonnegative(),
  /** Average confidence of extracted knowledge */
  average_confidence: z.number().min(0).max(1),
});

/**
 * Main output schema for the Knowledge Graph Builder Agent
 */
export const KnowledgeGraphBuilderOutputSchema = z.object({
  /** Extracted knowledge nodes */
  knowledge_nodes: z.array(KnowledgeNodeSchema),
  /** Extracted knowledge edges */
  knowledge_edges: z.array(KnowledgeEdgeSchema),
  /** Detected patterns */
  patterns_detected: z.array(PatternSchema),
  /** Graph analysis metrics */
  graph_metrics: GraphMetricsSchema,
  /** Sessions that were processed */
  sessions_processed: z.array(z.string().uuid()),
  /** Timestamp of graph construction */
  construction_timestamp: z.string().datetime(),
  /** Reference to the constructed subgraph in ruvector */
  subgraph_ref: z.string().uuid(),
  /** Whether the graph was merged with existing knowledge */
  merged_with_existing: z.boolean(),
  /** Number of existing nodes updated (if merged) */
  existing_nodes_updated: z.number().int().nonnegative().optional(),
  /** Warnings or non-fatal issues encountered */
  warnings: z.array(z.string()).optional(),
});

// ============================================================================
// SECTION 6: DECISION EVENT SCHEMA
// ============================================================================

/**
 * Telemetry data for the DecisionEvent
 */
export const KnowledgeGraphTelemetrySchema = z.object({
  /** Total execution duration in milliseconds */
  duration_ms: z.number().int().nonnegative().optional(),
  /** Memory usage in bytes */
  memory_bytes: z.number().int().nonnegative().optional(),
  /** RuVector query latency in milliseconds */
  ruvector_read_latency_ms: z.number().int().nonnegative().optional(),
  /** RuVector write latency in milliseconds */
  ruvector_write_latency_ms: z.number().int().nonnegative().optional(),
  /** Number of ruvector queries made */
  ruvector_query_count: z.number().int().nonnegative().optional(),
  /** NLP processing time in milliseconds */
  nlp_processing_ms: z.number().int().nonnegative().optional(),
  /** Pattern detection time in milliseconds */
  pattern_detection_ms: z.number().int().nonnegative().optional(),
});

/**
 * Constraints that can be applied during knowledge graph construction
 */
export type KnowledgeGraphConstraint =
  | 'session_boundary'          // Respect session boundaries
  | 'max_nodes'                 // Maximum nodes limit applied
  | 'max_edges'                 // Maximum edges limit applied
  | 'confidence_threshold'      // Confidence filtering applied
  | 'depth_limit'               // Depth traversal limit applied
  | 'clustering_disabled'       // Clustering was disabled
  | 'pattern_detection_limited' // Pattern detection was limited
  | 'merge_conflict_resolution' // Conflicts resolved during merge
  | 'deduplication_applied'     // Duplicate entities merged
  | 'pii_redaction'             // PII was detected and handled
  | 'content_size_limit';       // Content size limits applied

/**
 * DecisionEvent schema for knowledge graph construction
 *
 * Confidence semantics:
 * - 0.9-1.0: High-quality extraction with strong evidence
 * - 0.7-0.9: Good extraction with moderate evidence
 * - 0.5-0.7: Acceptable extraction, may need review
 * - <0.5: Low confidence, included due to relaxed thresholds
 */
export const KnowledgeGraphDecisionEventSchema = z.object({
  /** Agent identifier */
  agent_id: z.literal(AGENT_ID),
  /** Agent version */
  agent_version: z.string().regex(/^\d+\.\d+\.\d+$/),
  /** Decision type */
  decision_type: z.literal(DECISION_TYPE),
  /** SHA-256 hash of the input for idempotency */
  inputs_hash: z.string().length(64),
  /** Input that was processed */
  input: KnowledgeGraphBuilderInputSchema,
  /** Output produced */
  outputs: KnowledgeGraphBuilderOutputSchema,
  /**
   * Overall confidence in the constructed knowledge graph
   *
   * Computed as weighted average of:
   * - Average node extraction confidence (40%)
   * - Average edge confidence (30%)
   * - Pattern detection confidence (20%)
   * - Source data quality (10%)
   */
  confidence: z.number().min(0).max(1),
  /** Constraints that were applied during processing */
  constraints_applied: z.array(z.enum([
    'session_boundary', 'max_nodes', 'max_edges', 'confidence_threshold',
    'depth_limit', 'clustering_disabled', 'pattern_detection_limited',
    'merge_conflict_resolution', 'deduplication_applied', 'pii_redaction',
    'content_size_limit'
  ])).optional(),
  /** Unique execution reference for tracing */
  execution_ref: z.string().uuid(),
  /** Timestamp of the decision */
  timestamp: z.string().datetime(),
  /** Execution telemetry */
  telemetry: KnowledgeGraphTelemetrySchema.optional(),
});

// ============================================================================
// SECTION 7: ERROR SCHEMA DEFINITIONS
// ============================================================================

/**
 * Error codes specific to the Knowledge Graph Builder Agent
 */
export type KnowledgeGraphErrorCode =
  | 'VALIDATION_ERROR'           // Input validation failed
  | 'RUVECTOR_CONNECTION_ERROR'  // Cannot connect to ruvector-service
  | 'RUVECTOR_READ_ERROR'        // Error reading from ruvector
  | 'RUVECTOR_WRITE_ERROR'       // Error writing to ruvector
  | 'SESSION_NOT_FOUND'          // Requested session does not exist
  | 'EMPTY_RESULT'               // No knowledge could be extracted
  | 'CONFIDENCE_THRESHOLD_FAIL'  // All results below confidence threshold
  | 'DEPTH_LIMIT_EXCEEDED'       // Graph traversal hit depth limit
  | 'RESOURCE_EXHAUSTED'         // Memory or time limit exceeded
  | 'MERGE_CONFLICT'             // Unresolvable merge conflict
  | 'INTERNAL_ERROR'             // Unexpected internal error
  | 'RATE_LIMIT_EXCEEDED';       // Too many requests

/**
 * Agent error schema
 */
export const KnowledgeGraphErrorSchema = z.object({
  /** Error code */
  error_code: z.enum([
    'VALIDATION_ERROR', 'RUVECTOR_CONNECTION_ERROR', 'RUVECTOR_READ_ERROR',
    'RUVECTOR_WRITE_ERROR', 'SESSION_NOT_FOUND', 'EMPTY_RESULT',
    'CONFIDENCE_THRESHOLD_FAIL', 'DEPTH_LIMIT_EXCEEDED', 'RESOURCE_EXHAUSTED',
    'MERGE_CONFLICT', 'INTERNAL_ERROR', 'RATE_LIMIT_EXCEEDED'
  ]),
  /** Human-readable error message */
  message: z.string(),
  /** Additional error details */
  details: z.record(z.unknown()).optional(),
  /** Execution reference for correlation */
  execution_ref: z.string().uuid(),
  /** Error timestamp */
  timestamp: z.string().datetime(),
  /** Sessions that were partially processed before error */
  partial_sessions: z.array(z.string().uuid()).optional(),
  /** Whether the error is retryable */
  retryable: z.boolean().default(false),
});

// ============================================================================
// SECTION 8: TYPE EXPORTS (inferred from Zod schemas)
// ============================================================================

export type QueryFilter = z.infer<typeof QueryFilterSchema>;
export type ExtractionConfig = z.infer<typeof ExtractionConfigSchema>;
export type KnowledgeGraphBuilderInput = z.infer<typeof KnowledgeGraphBuilderInputSchema>;
export type KnowledgeNode = z.infer<typeof KnowledgeNodeSchema>;
export type KnowledgeEdge = z.infer<typeof KnowledgeEdgeSchema>;
export type Pattern = z.infer<typeof PatternSchema>;
export type GraphMetrics = z.infer<typeof GraphMetricsSchema>;
export type KnowledgeGraphBuilderOutput = z.infer<typeof KnowledgeGraphBuilderOutputSchema>;
export type KnowledgeGraphTelemetry = z.infer<typeof KnowledgeGraphTelemetrySchema>;
export type KnowledgeGraphDecisionEvent = z.infer<typeof KnowledgeGraphDecisionEventSchema>;
export type KnowledgeGraphError = z.infer<typeof KnowledgeGraphErrorSchema>;

// ============================================================================
// SECTION 9: CLI CONTRACT
// ============================================================================

/**
 * CLI Command Contract
 *
 * The Knowledge Graph Builder Agent exposes the following CLI commands
 * for inspection, retrieval, and replay operations.
 */
export interface CLIContract {
  /**
   * Inspect command - View knowledge graph state
   *
   * Usage: knowledge-graph inspect [options]
   *
   * Options:
   *   --session-id <uuid>     Inspect knowledge from specific session
   *   --subgraph-ref <uuid>   Inspect specific subgraph
   *   --node-id <uuid>        Inspect specific node
   *   --format <format>       Output format (json|table|graph)
   *   --depth <number>        Traversal depth for graph view
   */
  inspect: {
    sessionId?: string;
    subgraphRef?: string;
    nodeId?: string;
    format: 'json' | 'table' | 'graph';
    depth?: number;
  };

  /**
   * Retrieve command - Query knowledge entities
   *
   * Usage: knowledge-graph retrieve [options]
   *
   * Options:
   *   --query <string>        Semantic search query
   *   --node-type <type>      Filter by node type
   *   --entity-type <type>    Filter by entity subtype
   *   --pattern-type <type>   Filter by pattern type
   *   --min-confidence <n>    Minimum confidence threshold
   *   --limit <number>        Maximum results to return
   *   --include-edges         Include related edges
   *   --format <format>       Output format (json|table)
   */
  retrieve: {
    query?: string;
    nodeType?: KnowledgeNodeType;
    entityType?: EntitySubtype;
    patternType?: PatternType;
    minConfidence?: number;
    limit?: number;
    includeEdges?: boolean;
    format: 'json' | 'table';
  };

  /**
   * Replay command - Replay analysis on historical data
   *
   * Usage: knowledge-graph replay [options]
   *
   * Options:
   *   --session-ids <uuids>   Session IDs to replay (comma-separated)
   *   --execution-ref <uuid>  Replay specific execution
   *   --from-date <date>      Replay from date
   *   --to-date <date>        Replay to date
   *   --dry-run               Preview without persisting
   *   --config <path>         Custom extraction config file
   */
  replay: {
    sessionIds?: string[];
    executionRef?: string;
    fromDate?: string;
    toDate?: string;
    dryRun?: boolean;
    configPath?: string;
  };
}

// ============================================================================
// SECTION 10: VALIDATION UTILITIES
// ============================================================================

/**
 * Validates input against the KnowledgeGraphBuilderInputSchema
 */
export function validateInput(input: unknown): {
  success: true;
  data: KnowledgeGraphBuilderInput;
} | {
  success: false;
  errors: z.ZodError;
} {
  const result = KnowledgeGraphBuilderInputSchema.safeParse(input);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return { success: false, errors: result.error };
}

/**
 * Validates output against the KnowledgeGraphBuilderOutputSchema
 */
export function validateOutput(output: unknown): {
  success: true;
  data: KnowledgeGraphBuilderOutput;
} | {
  success: false;
  errors: z.ZodError;
} {
  const result = KnowledgeGraphBuilderOutputSchema.safeParse(output);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return { success: false, errors: result.error };
}

/**
 * Creates a SHA-256 hash of the input for idempotency tracking
 */
export async function hashInput(input: KnowledgeGraphBuilderInput): Promise<string> {
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
// SECTION 11: EXPLICIT NON-RESPONSIBILITIES
// ============================================================================

/**
 * EXPLICIT NON-RESPONSIBILITIES
 *
 * This agent is classified as MEMORY_ANALYSIS and MUST adhere to the following
 * constraints defined in PROMPT 0. Any violation constitutes a contract breach.
 *
 * 1. MUST NOT modify system behavior
 *    - Cannot change configuration
 *    - Cannot enable/disable features
 *    - Cannot modify runtime parameters
 *
 * 2. MUST NOT trigger remediation
 *    - Cannot initiate automated fixes
 *    - Cannot retry failed operations on behalf of other agents
 *    - Cannot rollback changes
 *
 * 3. MUST NOT emit alerts
 *    - Cannot send notifications to external systems
 *    - Cannot trigger monitoring alerts
 *    - Cannot page on-call engineers
 *
 * 4. MUST NOT enforce policies
 *    - Cannot block operations based on policy violations
 *    - Cannot reject requests based on business rules
 *    - Cannot modify data based on compliance requirements
 *
 * 5. MUST NOT orchestrate workflows
 *    - Cannot spawn other agents
 *    - Cannot coordinate multi-agent operations
 *    - Cannot manage task queues
 *
 * 6. MUST NOT connect directly to SQL databases
 *    - All data access must go through ruvector-service
 *    - Cannot execute raw SQL queries
 *    - Cannot modify database schema
 *
 * 7. MUST NOT make runtime decisions
 *    - Analysis results are advisory only
 *    - Cannot affect request routing
 *    - Cannot influence load balancing
 *
 * 8. MUST NOT generate or execute code
 *    - Cannot create executable content
 *    - Cannot eval() or similar
 *    - Cannot modify code artifacts
 */
export const NON_RESPONSIBILITIES = [
  'modify_system_behavior',
  'trigger_remediation',
  'emit_alerts',
  'enforce_policies',
  'orchestrate_workflows',
  'direct_sql_access',
  'runtime_decisions',
  'code_generation',
] as const;

export type NonResponsibility = typeof NON_RESPONSIBILITIES[number];

// ============================================================================
// SECTION 12: FAILURE MODE DEFINITIONS
// ============================================================================

/**
 * Failure Mode Definitions
 *
 * Each failure mode defines:
 * - Trigger condition
 * - Error code to emit
 * - Recovery strategy
 * - Retryability
 */
export const FAILURE_MODES = {
  /**
   * Invalid session_id handling
   * Trigger: session_id does not exist in ruvector
   */
  SESSION_NOT_FOUND: {
    error_code: 'SESSION_NOT_FOUND' as const,
    message_template: 'Session {session_id} not found in memory store',
    retryable: false,
    recovery: 'Caller should verify session_id exists before requesting analysis',
  },

  /**
   * Empty result handling
   * Trigger: No knowledge could be extracted from provided sessions
   */
  EMPTY_RESULT: {
    error_code: 'EMPTY_RESULT' as const,
    message_template: 'No knowledge extracted from {session_count} sessions',
    retryable: false,
    recovery: 'Sessions may be empty or contain no extractable content. Verify session data.',
  },

  /**
   * Confidence threshold failures
   * Trigger: All extracted knowledge falls below confidence threshold
   */
  CONFIDENCE_THRESHOLD_FAIL: {
    error_code: 'CONFIDENCE_THRESHOLD_FAIL' as const,
    message_template: 'All {candidate_count} candidates below confidence threshold {threshold}',
    retryable: true,
    recovery: 'Consider lowering confidence_threshold in extraction_config',
  },

  /**
   * RuVector service unavailability
   * Trigger: Cannot establish connection to ruvector-service
   */
  RUVECTOR_UNAVAILABLE: {
    error_code: 'RUVECTOR_CONNECTION_ERROR' as const,
    message_template: 'Failed to connect to ruvector-service: {error}',
    retryable: true,
    recovery: 'Retry with exponential backoff. Check service health.',
  },

  /**
   * RuVector read error
   * Trigger: Error while reading memory nodes from ruvector
   */
  RUVECTOR_READ_FAILURE: {
    error_code: 'RUVECTOR_READ_ERROR' as const,
    message_template: 'Failed to read from ruvector: {error}',
    retryable: true,
    recovery: 'Retry operation. If persistent, check ruvector logs.',
  },

  /**
   * RuVector write error
   * Trigger: Error while persisting knowledge graph to ruvector
   */
  RUVECTOR_WRITE_FAILURE: {
    error_code: 'RUVECTOR_WRITE_ERROR' as const,
    message_template: 'Failed to write to ruvector: {error}',
    retryable: true,
    recovery: 'Retry operation. Check for capacity limits.',
  },

  /**
   * Resource exhaustion
   * Trigger: Processing exceeds memory or time limits
   */
  RESOURCE_EXHAUSTED: {
    error_code: 'RESOURCE_EXHAUSTED' as const,
    message_template: 'Resource limit exceeded: {resource} at {usage}',
    retryable: true,
    recovery: 'Reduce batch size or session count. Consider chunked processing.',
  },

  /**
   * Merge conflict
   * Trigger: Cannot resolve conflicting knowledge during merge
   */
  MERGE_CONFLICT: {
    error_code: 'MERGE_CONFLICT' as const,
    message_template: 'Unresolvable merge conflict for {entity_type}: {conflict_detail}',
    retryable: false,
    recovery: 'Set merge_existing to false or manually resolve conflicts.',
  },
} as const;

export type FailureMode = keyof typeof FAILURE_MODES;

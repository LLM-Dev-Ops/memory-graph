/**
 * Memory Retrieval Agent - Type Definitions
 *
 * All types are derived from agentics-contracts schemas.
 * This agent is classified as MEMORY READ.
 */

import { z } from 'zod';

// ============================================================================
// AGENT METADATA
// ============================================================================

export const AGENT_ID = 'memory-retrieval-agent' as const;
export const AGENT_VERSION = '1.0.0' as const;
export const AGENT_CLASSIFICATION = 'MEMORY_READ' as const;
export const DECISION_TYPE = 'memory_retrieval' as const;

// ============================================================================
// ZOD SCHEMAS (for runtime validation)
// ============================================================================

export const QueryConstraintSchema = z.object({
  constraint_type: z.enum([
    'session_filter',
    'node_type_filter',
    'time_range',
    'depth_limit',
    'agent_filter',
    'tag_filter',
    'similarity_threshold',
  ]),
  value: z.unknown(),
  operator: z
    .enum(['equals', 'contains', 'greater_than', 'less_than', 'between', 'in'])
    .default('equals'),
});

export const TraversalOptionsSchema = z.object({
  max_depth: z.number().int().min(1).max(10).default(3),
  direction: z.enum(['outgoing', 'incoming', 'both']).default('both'),
  follow_edge_types: z
    .array(z.enum(['belongs_to', 'responds_to', 'follows', 'invokes', 'references', 'inherits']))
    .optional(),
  include_node_types: z
    .array(z.enum(['prompt', 'response', 'session', 'tool_invocation', 'agent', 'template']))
    .optional(),
});

export const MemoryRetrievalInputSchema = z.object({
  query_id: z.string().uuid(),
  query_type: z.enum(['subgraph', 'nodes', 'edges', 'lineage', 'context', 'similarity']),
  anchor_nodes: z.array(z.string().uuid()).optional(),
  anchor_sessions: z.array(z.string().uuid()).optional(),
  constraints: z.array(QueryConstraintSchema).optional(),
  traversal_options: TraversalOptionsSchema.optional(),
  semantic_query: z.string().max(4096).optional(),
  limit: z.number().int().min(1).max(1000).default(100),
  offset: z.number().int().min(0).default(0),
  include_metadata: z.boolean().default(true),
  requesting_agent_id: z.string().optional(),
});

export const RetrievedNodeSchema = z.object({
  node_id: z.string().uuid(),
  node_type: z.enum(['prompt', 'response', 'session', 'tool_invocation', 'agent', 'template']),
  content: z.string().optional(),
  created_at: z.string().datetime().optional(),
  metadata: z.record(z.unknown()).optional(),
  relevance_score: z.number().min(0).max(1).optional(),
  depth: z.number().int().min(0).optional(),
});

export const RetrievedEdgeSchema = z.object({
  edge_id: z.string().uuid(),
  edge_type: z.enum(['belongs_to', 'responds_to', 'follows', 'invokes', 'references', 'inherits']),
  from_node_id: z.string().uuid(),
  to_node_id: z.string().uuid(),
  weight: z.number().min(0).max(1).optional(),
  created_at: z.string().datetime().optional(),
  properties: z.record(z.unknown()).optional(),
});

export const RetrievedSubgraphSchema = z.object({
  nodes: z.array(RetrievedNodeSchema),
  edges: z.array(RetrievedEdgeSchema),
  anchor_node_ids: z.array(z.string().uuid()).optional(),
  truncated: z.boolean().optional(),
});

export const TraversalStatsSchema = z.object({
  max_depth_reached: z.number().int().optional(),
  nodes_visited: z.number().int().optional(),
  edges_traversed: z.number().int().optional(),
});

export const PaginationSchema = z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  has_more: z.boolean(),
  total_available: z.number().int().optional(),
});

export const MemoryRetrievalOutputSchema = z.object({
  query_id: z.string().uuid(),
  query_type: z.enum(['subgraph', 'nodes', 'edges', 'lineage', 'context', 'similarity']),
  subgraph: RetrievedSubgraphSchema,
  total_nodes_retrieved: z.number().int().min(0),
  total_edges_retrieved: z.number().int().min(0),
  retrieval_timestamp: z.string().datetime(),
  constraints_applied: z.array(z.string()).optional(),
  traversal_stats: TraversalStatsSchema.optional(),
  pagination: PaginationSchema.optional(),
});

export const DecisionEventTelemetrySchema = z.object({
  duration_ms: z.number().int().nonnegative().optional(),
  memory_bytes: z.number().int().nonnegative().optional(),
  ruvector_latency_ms: z.number().int().nonnegative().optional(),
  nodes_scanned: z.number().int().nonnegative().optional(),
  cache_hit: z.boolean().optional(),
});

export const DecisionEventSchema = z.object({
  agent_id: z.literal(AGENT_ID),
  agent_version: z.string().regex(/^\d+\.\d+\.\d+$/),
  decision_type: z.literal(DECISION_TYPE),
  inputs_hash: z.string().length(64),
  outputs: MemoryRetrievalOutputSchema,
  confidence: z.number().min(0).max(1),
  constraints_applied: z.array(z.string()).optional(),
  execution_ref: z.string().uuid(),
  timestamp: z.string().datetime(),
  telemetry: DecisionEventTelemetrySchema.optional(),
});

export const AgentErrorSchema = z.object({
  error_code: z.enum([
    'VALIDATION_ERROR',
    'RUVECTOR_CONNECTION_ERROR',
    'RUVECTOR_READ_ERROR',
    'QUERY_TIMEOUT',
    'INVALID_ANCHOR_NODE',
    'INTERNAL_ERROR',
    'RATE_LIMIT_EXCEEDED',
  ]),
  message: z.string(),
  details: z.record(z.unknown()).optional(),
  execution_ref: z.string().uuid(),
  timestamp: z.string().datetime(),
});

// ============================================================================
// TYPE EXPORTS (inferred from Zod schemas)
// ============================================================================

export type QueryConstraint = z.infer<typeof QueryConstraintSchema>;
export type TraversalOptions = z.infer<typeof TraversalOptionsSchema>;
export type MemoryRetrievalInput = z.infer<typeof MemoryRetrievalInputSchema>;
export type RetrievedNode = z.infer<typeof RetrievedNodeSchema>;
export type RetrievedEdge = z.infer<typeof RetrievedEdgeSchema>;
export type RetrievedSubgraph = z.infer<typeof RetrievedSubgraphSchema>;
export type TraversalStats = z.infer<typeof TraversalStatsSchema>;
export type Pagination = z.infer<typeof PaginationSchema>;
export type MemoryRetrievalOutput = z.infer<typeof MemoryRetrievalOutputSchema>;
export type DecisionEventTelemetry = z.infer<typeof DecisionEventTelemetrySchema>;
export type DecisionEvent = z.infer<typeof DecisionEventSchema>;
export type AgentError = z.infer<typeof AgentErrorSchema>;
export type AgentErrorCode = AgentError['error_code'];

// ============================================================================
// NODE & EDGE TYPE MAPPINGS
// ============================================================================

export type NodeType = 'prompt' | 'response' | 'session' | 'tool_invocation' | 'agent' | 'template';
export type EdgeType =
  | 'belongs_to'
  | 'responds_to'
  | 'follows'
  | 'invokes'
  | 'references'
  | 'inherits';
export type QueryType = 'subgraph' | 'nodes' | 'edges' | 'lineage' | 'context' | 'similarity';
export type TraversalDirection = 'outgoing' | 'incoming' | 'both';
export type ConstraintType =
  | 'session_filter'
  | 'node_type_filter'
  | 'time_range'
  | 'depth_limit'
  | 'agent_filter'
  | 'tag_filter'
  | 'similarity_threshold';

/**
 * Maps query types to expected traversal behaviors
 */
export const QUERY_TYPE_DEFAULTS: Record<
  QueryType,
  { direction: TraversalDirection; max_depth: number }
> = {
  subgraph: { direction: 'both', max_depth: 3 },
  nodes: { direction: 'both', max_depth: 1 },
  edges: { direction: 'both', max_depth: 1 },
  lineage: { direction: 'incoming', max_depth: 10 },
  context: { direction: 'both', max_depth: 2 },
  similarity: { direction: 'both', max_depth: 1 },
};

/**
 * Edge types that represent temporal/causal relationships
 */
export const LINEAGE_EDGE_TYPES: EdgeType[] = ['follows', 'responds_to', 'invokes'];

/**
 * Edge types that represent structural relationships
 */
export const STRUCTURAL_EDGE_TYPES: EdgeType[] = ['belongs_to', 'references', 'inherits'];

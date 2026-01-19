/**
 * Knowledge Graph Builder Agent - Type Definitions
 *
 * Classification: MEMORY_ANALYSIS
 * Decision Type: knowledge_graph_construction
 *
 * All types are derived from agentics-contracts schemas.
 * This agent analyzes conversations to build knowledge graphs.
 */

import { z } from 'zod';

// ============================================================================
// AGENT METADATA
// ============================================================================

export const AGENT_ID = 'knowledge-graph-builder-agent' as const;
export const AGENT_VERSION = '1.0.0' as const;
export const AGENT_CLASSIFICATION = 'MEMORY_ANALYSIS' as const;
export const DECISION_TYPE = 'knowledge_graph_construction' as const;

// ============================================================================
// KNOWLEDGE GRAPH SPECIFIC TYPES
// ============================================================================

/**
 * Types of concepts that can be extracted
 */
export type ConceptType =
  | 'entity'
  | 'action'
  | 'attribute'
  | 'topic'
  | 'keyword'
  | 'pattern';

/**
 * Types of relationships in the knowledge graph
 */
export type RelationshipType =
  | 'is_a'
  | 'has_attribute'
  | 'related_to'
  | 'part_of'
  | 'causes'
  | 'precedes'
  | 'co_occurs'
  | 'depends_on'
  | 'references';

/**
 * Entity types for named entity recognition
 */
export type EntityType =
  | 'person'
  | 'organization'
  | 'location'
  | 'datetime'
  | 'technology'
  | 'concept'
  | 'file_path'
  | 'url'
  | 'code_element'
  | 'other';

// ============================================================================
// ZOD SCHEMAS (for runtime validation)
// ============================================================================

/**
 * Schema for text content to analyze
 */
export const TextContentSchema = z.object({
  content_id: z.string().uuid(),
  text: z.string().min(1),
  role: z.enum(['user', 'assistant', 'system', 'tool']).optional(),
  timestamp: z.string().datetime().optional(),
  metadata: z.record(z.unknown()).optional(),
});

/**
 * Schema for extraction options
 */
export const ExtractionOptionsSchema = z.object({
  extract_entities: z.boolean().default(true),
  extract_concepts: z.boolean().default(true),
  extract_relationships: z.boolean().default(true),
  detect_patterns: z.boolean().default(true),
  min_confidence: z.number().min(0).max(1).default(0.5),
  max_concepts_per_text: z.number().int().positive().default(50),
  entity_types: z.array(z.enum([
    'person', 'organization', 'location', 'datetime',
    'technology', 'concept', 'file_path', 'url', 'code_element', 'other'
  ])).optional(),
});

/**
 * Schema for graph construction options
 */
export const GraphOptionsSchema = z.object({
  merge_similar_concepts: z.boolean().default(true),
  similarity_threshold: z.number().min(0).max(1).default(0.8),
  create_temporal_edges: z.boolean().default(true),
  compute_centrality: z.boolean().default(false),
  max_relationship_depth: z.number().int().positive().default(3),
});

/**
 * Main input schema for the Knowledge Graph Builder
 */
export const KnowledgeGraphBuilderInputSchema = z.object({
  request_id: z.string().uuid(),
  session_id: z.string().uuid().optional(),
  conversation_id: z.string().uuid().optional(),
  texts: z.array(TextContentSchema).min(1),
  extraction_options: ExtractionOptionsSchema.optional(),
  graph_options: GraphOptionsSchema.optional(),
  context: z.object({
    domain: z.string().optional(),
    language: z.string().default('en'),
    source_system: z.string().optional(),
    tags: z.array(z.string()).optional(),
  }).optional(),
});

/**
 * Schema for extracted concept
 */
export const ExtractedConceptSchema = z.object({
  concept_id: z.string().uuid(),
  name: z.string().min(1),
  normalized_name: z.string().min(1),
  type: z.enum(['entity', 'action', 'attribute', 'topic', 'keyword', 'pattern']),
  entity_type: z.enum([
    'person', 'organization', 'location', 'datetime',
    'technology', 'concept', 'file_path', 'url', 'code_element', 'other'
  ]).optional(),
  confidence: z.number().min(0).max(1),
  frequency: z.number().int().positive(),
  source_content_ids: z.array(z.string().uuid()),
  context_snippets: z.array(z.string()).optional(),
  metadata: z.record(z.unknown()).optional(),
});

/**
 * Schema for extracted relationship
 */
export const ExtractedRelationshipSchema = z.object({
  relationship_id: z.string().uuid(),
  source_concept_id: z.string().uuid(),
  target_concept_id: z.string().uuid(),
  relationship_type: z.enum([
    'is_a', 'has_attribute', 'related_to', 'part_of',
    'causes', 'precedes', 'co_occurs', 'depends_on', 'references'
  ]),
  confidence: z.number().min(0).max(1),
  weight: z.number().min(0).default(1),
  evidence: z.array(z.object({
    content_id: z.string().uuid(),
    snippet: z.string(),
  })).optional(),
  metadata: z.record(z.unknown()).optional(),
});

/**
 * Schema for detected pattern
 */
export const DetectedPatternSchema = z.object({
  pattern_id: z.string().uuid(),
  pattern_type: z.enum(['recurring_theme', 'temporal_sequence', 'co_occurrence', 'structural']),
  description: z.string(),
  involved_concepts: z.array(z.string().uuid()),
  occurrences: z.number().int().positive(),
  confidence: z.number().min(0).max(1),
  first_seen: z.string().datetime().optional(),
  last_seen: z.string().datetime().optional(),
});

/**
 * Schema for graph statistics
 */
export const GraphStatisticsSchema = z.object({
  total_concepts: z.number().int().nonnegative(),
  total_relationships: z.number().int().nonnegative(),
  total_patterns: z.number().int().nonnegative(),
  concept_type_distribution: z.record(z.number().int().nonnegative()),
  relationship_type_distribution: z.record(z.number().int().nonnegative()),
  avg_concept_confidence: z.number().min(0).max(1),
  avg_relationship_confidence: z.number().min(0).max(1),
  graph_density: z.number().min(0).max(1).optional(),
  connected_components: z.number().int().nonnegative().optional(),
});

/**
 * Output schema for the Knowledge Graph Builder
 */
export const KnowledgeGraphBuilderOutputSchema = z.object({
  request_id: z.string().uuid(),
  session_id: z.string().uuid().optional(),
  conversation_id: z.string().uuid().optional(),
  concepts: z.array(ExtractedConceptSchema),
  relationships: z.array(ExtractedRelationshipSchema),
  patterns: z.array(DetectedPatternSchema),
  statistics: GraphStatisticsSchema,
  build_timestamp: z.string().datetime(),
  processing_metadata: z.object({
    texts_processed: z.number().int().positive(),
    total_characters: z.number().int().nonnegative(),
    extraction_duration_ms: z.number().int().nonnegative(),
    graph_build_duration_ms: z.number().int().nonnegative(),
  }),
});

/**
 * Telemetry schema
 */
export const DecisionEventTelemetrySchema = z.object({
  duration_ms: z.number().int().nonnegative().optional(),
  memory_bytes: z.number().int().nonnegative().optional(),
  ruvector_latency_ms: z.number().int().nonnegative().optional(),
  extraction_duration_ms: z.number().int().nonnegative().optional(),
  graph_build_duration_ms: z.number().int().nonnegative().optional(),
});

/**
 * DecisionEvent schema for this agent
 */
export const DecisionEventSchema = z.object({
  agent_id: z.literal(AGENT_ID),
  agent_version: z.string().regex(/^\d+\.\d+\.\d+$/),
  decision_type: z.literal(DECISION_TYPE),
  inputs_hash: z.string().length(64),
  outputs: KnowledgeGraphBuilderOutputSchema,
  confidence: z.number().min(0).max(1),
  constraints_applied: z.array(z.string()).optional(),
  execution_ref: z.string().uuid(),
  timestamp: z.string().datetime(),
  telemetry: DecisionEventTelemetrySchema.optional(),
});

/**
 * Agent error schema
 */
export const AgentErrorSchema = z.object({
  error_code: z.enum([
    'VALIDATION_ERROR',
    'RUVECTOR_CONNECTION_ERROR',
    'RUVECTOR_WRITE_ERROR',
    'EXTRACTION_ERROR',
    'GRAPH_BUILD_ERROR',
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

export type TextContent = z.infer<typeof TextContentSchema>;
export type ExtractionOptions = z.infer<typeof ExtractionOptionsSchema>;
export type GraphOptions = z.infer<typeof GraphOptionsSchema>;
export type KnowledgeGraphBuilderInput = z.infer<typeof KnowledgeGraphBuilderInputSchema>;
export type ExtractedConcept = z.infer<typeof ExtractedConceptSchema>;
export type ExtractedRelationship = z.infer<typeof ExtractedRelationshipSchema>;
export type DetectedPattern = z.infer<typeof DetectedPatternSchema>;
export type GraphStatistics = z.infer<typeof GraphStatisticsSchema>;
export type KnowledgeGraphBuilderOutput = z.infer<typeof KnowledgeGraphBuilderOutputSchema>;
export type DecisionEventTelemetry = z.infer<typeof DecisionEventTelemetrySchema>;
export type DecisionEvent = z.infer<typeof DecisionEventSchema>;
export type AgentError = z.infer<typeof AgentErrorSchema>;
export type AgentErrorCode = AgentError['error_code'];

// ============================================================================
// EXTRACTION CONSTANTS
// ============================================================================

/**
 * Common stop words to filter during extraction
 */
export const STOP_WORDS = new Set([
  'the', 'a', 'an', 'and', 'or', 'but', 'in', 'on', 'at', 'to', 'for',
  'of', 'with', 'by', 'from', 'is', 'are', 'was', 'were', 'be', 'been',
  'being', 'have', 'has', 'had', 'do', 'does', 'did', 'will', 'would',
  'could', 'should', 'may', 'might', 'must', 'shall', 'can', 'need',
  'this', 'that', 'these', 'those', 'it', 'its', 'as', 'if', 'then',
  'than', 'so', 'such', 'not', 'no', 'yes', 'all', 'any', 'both', 'each',
  'few', 'more', 'most', 'other', 'some', 'very', 'just', 'also', 'now',
  'here', 'there', 'when', 'where', 'why', 'how', 'what', 'which', 'who',
]);

/**
 * Entity type detection patterns
 */
export const ENTITY_PATTERNS: Record<EntityType, RegExp[]> = {
  person: [
    /\b[A-Z][a-z]+\s+[A-Z][a-z]+\b/g, // Capitalized names
  ],
  organization: [
    /\b(?:Inc|Corp|LLC|Ltd|Company|Organization|Foundation|Institute)\b/gi,
    /\b[A-Z][A-Za-z]*(?:Tech|Soft|Corp|Labs?)\b/g,
  ],
  location: [
    /\b(?:in|at|from|to)\s+[A-Z][a-z]+(?:\s+[A-Z][a-z]+)?\b/g,
  ],
  datetime: [
    /\b\d{4}-\d{2}-\d{2}(?:T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?)?\b/g,
    /\b(?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{1,2}(?:,?\s+\d{4})?\b/gi,
    /\b\d{1,2}\/\d{1,2}\/\d{2,4}\b/g,
  ],
  technology: [
    /\b(?:JavaScript|TypeScript|Python|Java|Rust|Go|C\+\+|Ruby|PHP|Swift|Kotlin)\b/gi,
    /\b(?:React|Angular|Vue|Node\.?js|Express|Django|Flask|Spring|Rails)\b/gi,
    /\b(?:SQL|NoSQL|MongoDB|PostgreSQL|MySQL|Redis|Elasticsearch)\b/gi,
    /\b(?:AWS|Azure|GCP|Docker|Kubernetes|Terraform)\b/gi,
    /\b(?:REST|GraphQL|gRPC|WebSocket|HTTP|HTTPS)\b/gi,
  ],
  concept: [
    /\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)*\b/g, // Proper nouns
  ],
  file_path: [
    /(?:\/[\w.-]+)+(?:\.\w+)?/g,
    /\b[\w.-]+\.[a-z]{2,4}\b/gi,
  ],
  url: [
    /https?:\/\/[^\s<>"{}|\\^`[\]]+/gi,
  ],
  code_element: [
    /\b(?:function|class|interface|type|const|let|var|import|export|async|await)\s+\w+/g,
    /\b\w+(?:\.\w+)+\(\)/g, // Method calls
    /`[^`]+`/g, // Code in backticks
  ],
  other: [],
};

/**
 * Relationship inference patterns
 */
export const RELATIONSHIP_PATTERNS: Array<{
  pattern: RegExp;
  type: RelationshipType;
  sourceGroup: number;
  targetGroup: number;
}> = [
  {
    pattern: /(\w+)\s+is\s+(?:a|an)\s+(\w+)/gi,
    type: 'is_a',
    sourceGroup: 1,
    targetGroup: 2,
  },
  {
    pattern: /(\w+)\s+has\s+(\w+)/gi,
    type: 'has_attribute',
    sourceGroup: 1,
    targetGroup: 2,
  },
  {
    pattern: /(\w+)\s+(?:is\s+)?related\s+to\s+(\w+)/gi,
    type: 'related_to',
    sourceGroup: 1,
    targetGroup: 2,
  },
  {
    pattern: /(\w+)\s+(?:is\s+)?part\s+of\s+(\w+)/gi,
    type: 'part_of',
    sourceGroup: 1,
    targetGroup: 2,
  },
  {
    pattern: /(\w+)\s+causes?\s+(\w+)/gi,
    type: 'causes',
    sourceGroup: 1,
    targetGroup: 2,
  },
  {
    pattern: /(\w+)\s+(?:before|then)\s+(\w+)/gi,
    type: 'precedes',
    sourceGroup: 1,
    targetGroup: 2,
  },
  {
    pattern: /(\w+)\s+depends?\s+on\s+(\w+)/gi,
    type: 'depends_on',
    sourceGroup: 1,
    targetGroup: 2,
  },
  {
    pattern: /(\w+)\s+references?\s+(\w+)/gi,
    type: 'references',
    sourceGroup: 1,
    targetGroup: 2,
  },
];
